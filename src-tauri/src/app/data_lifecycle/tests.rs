#![cfg(test)]

use super::*;
use std::fs;
use std::sync::mpsc::sync_channel;

#[path = "inventory_tests.rs"]
mod inventory_tests;
#[path = "journal_tests.rs"]
mod journal_tests;

fn open(paths: &AppPaths) -> (Arc<DataLifecycle>, Arc<Db>, Arc<OwnerStore>) {
    let lifecycle = Arc::new(DataLifecycle::prepare(paths).unwrap());
    let owner = Arc::new(OwnerStore::open(&paths.owner_workspace()).unwrap());
    let legacy = Arc::new(Db::open(&paths.db).unwrap());
    lifecycle.finish_startup(&legacy, &owner).unwrap();
    (lifecycle, legacy, owner)
}

fn seed(legacy: &Db, owner: &OwnerStore) {
    legacy
        .write(|conn| settings_repository::put(conn, "private_fixture", &"legacy-secret"))
        .unwrap();
    owner
        .save_app(crate::owner_workspace::SaveOwnerAppInput {
            operation_id: uuid::Uuid::new_v4().to_string(),
            id: uuid::Uuid::new_v4().to_string(),
            expected_version: 0,
            name: "Owner fixture".into(),
            description: "owner-secret".into(),
            legacy_project_id: None,
            archived: false,
        })
        .unwrap();
}

fn purge(
    lifecycle: &DataLifecycle,
    paths: &AppPaths,
    legacy: &Db,
    owner: &OwnerStore,
) -> AppResult<()> {
    let _operation = lifecycle.lock_operation();
    purge_stores(
        lifecycle,
        paths,
        legacy,
        owner,
        || Ok(()),
        "\"dark\"",
        "\"2d\"",
    )?;
    lifecycle.reopen_global()
}

#[test]
fn admission_lease_is_send_and_closed_at_startup() {
    fn assert_send<T: Send>() {}
    assert_send::<MutationLease>();
    let dir = crate::testing::temp_dir("data-lifecycle-admission");
    let lifecycle = Arc::new(DataLifecycle::prepare(&AppPaths::under(&dir)).unwrap());
    assert!(lifecycle.admit().is_err());
}

#[test]
fn interrupted_purge_removes_both_generations_before_any_store_opens() {
    let dir = crate::testing::temp_dir("data-lifecycle-pending");
    let paths = AppPaths::under(&dir);
    fs::write(&paths.db.database, b"legacy invalid bytes").unwrap();
    fs::write(paths.owner_workspace(), b"owner invalid bytes").unwrap();
    fs::write(
        inventory::companion(&paths.owner_workspace(), "-journal"),
        b"old rollback",
    )
    .unwrap();
    fs::write(dir.join("events.corrupt-123.db"), b"old copy").unwrap();
    fs::create_dir_all(dir.join("owner-artifacts")).unwrap();
    fs::write(dir.join("owner-artifacts/preview.html"), b"old preview").unwrap();
    journal::save(
        &paths.purge_journal(),
        &PurgeJournal {
            epoch: 4,
            pending: true,
            theme: Theme::Dark,
            visualization: VisualizationMode::TwoD,
            ..PurgeJournal::default()
        },
    )
    .unwrap();
    let lifecycle = DataLifecycle::prepare(&paths).unwrap();
    assert!(!paths.db.database.exists());
    assert!(!paths.owner_workspace().exists());
    assert!(!dir.join("events.corrupt-123.db").exists());
    assert!(!dir.join("owner-artifacts").exists());
    assert!(
        journal::load(&paths.purge_journal())
            .unwrap()
            .unwrap()
            .pending
    );
    let owner = OwnerStore::open(&paths.owner_workspace()).unwrap();
    let legacy = Db::open(&paths.db).unwrap();
    lifecycle.finish_startup(&legacy, &owner).unwrap();
    let settings = legacy.read(settings_repository::load).unwrap();
    assert_eq!(settings.theme, Theme::Dark);
    assert_eq!(settings.visualization, VisualizationMode::TwoD);
    assert!(!settings.onboarding_completed);
    assert_eq!(owner.capture_epoch().unwrap().value(), 4);
    assert!(
        !journal::load(&paths.purge_journal())
            .unwrap()
            .unwrap()
            .pending
    );
}

#[test]
fn completed_epoch_survives_restart_and_stale_store_tokens_never_revive() {
    let dir = crate::testing::temp_dir("data-lifecycle-epochs");
    let paths = AppPaths::under(&dir);
    let (lifecycle, legacy, owner) = open(&paths);
    seed(&legacy, &owner);
    let old_owner = owner.capture_epoch().unwrap();
    let old_legacy = legacy.capture_write_epoch().unwrap();
    purge(&lifecycle, &paths, &legacy, &owner).unwrap();
    assert_eq!(lifecycle.epoch(), 1);
    assert!(owner.write_at_epoch(old_owner, |_| Ok(())).is_err());
    assert!(legacy.write_at_epoch(old_legacy, |_| Ok(())).is_err());
    assert!(owner.list_apps(true).unwrap().is_empty());
    assert!(legacy
        .read(|conn| settings_repository::get::<String>(conn, "private_fixture"))
        .unwrap()
        .is_none());
    seed(&legacy, &owner);
    owner.shutdown().unwrap();
    legacy.close();
    drop((lifecycle, legacy, owner));
    let (lifecycle, legacy, owner) = open(&paths);
    assert_eq!(lifecycle.epoch(), 1);
    assert_eq!(
        owner.list_apps(true).unwrap().len(),
        1,
        "ordinary restart preserves the fresh generation"
    );
    purge(&lifecycle, &paths, &legacy, &owner).unwrap();
    assert_eq!(lifecycle.epoch(), 2);
    assert_eq!(owner.capture_epoch().unwrap().value(), 2);
    assert!(owner.write_at_epoch(old_owner, |_| Ok(())).is_err());
    assert!(legacy.write_at_epoch(old_legacy, |_| Ok(())).is_err());
}

#[test]
fn detached_lease_blocks_deletion_and_new_mutation_is_rejected() {
    let dir = crate::testing::temp_dir("data-lifecycle-drain");
    let paths = AppPaths::under(&dir);
    let (lifecycle, legacy, owner) = open(&paths);
    seed(&legacy, &owner);
    let lease = lifecycle.admit().unwrap();
    let (release_tx, release_rx) = sync_channel(1);
    let worker = std::thread::spawn(move || {
        release_rx.recv().unwrap();
        assert!(lease.validate().is_err());
    });
    let (closed_tx, closed_rx) = sync_channel(1);
    let purging_lifecycle = Arc::clone(&lifecycle);
    let purging_db = Arc::clone(&legacy);
    let purging_owner = Arc::clone(&owner);
    let purger = std::thread::spawn(move || {
        let _operation = purging_lifecycle.lock_operation();
        purge_stores(
            &purging_lifecycle,
            &paths,
            &purging_db,
            &purging_owner,
            || {
                closed_tx.send(()).unwrap();
                Ok(())
            },
            "\"dark\"",
            "\"2d\"",
        )
    });
    closed_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(lifecycle.admit().is_err());
    assert!(owner.capture_epoch().is_err());
    assert!(legacy.write(|_| Ok(())).is_err());
    assert_eq!(
        owner.list_apps(true).unwrap().len(),
        1,
        "deletion waits for the detached worker"
    );
    release_tx.send(()).unwrap();
    worker.join().unwrap();
    purger.join().unwrap().unwrap();
    lifecycle.reopen_global().unwrap();
    assert!(owner.list_apps(true).unwrap().is_empty());
}

#[test]
fn failed_inventory_keeps_every_admission_closed_and_retry_reuses_epoch() {
    let dir = crate::testing::temp_dir("data-lifecycle-retry");
    let paths = AppPaths::under(&dir);
    let (lifecycle, legacy, owner) = open(&paths);
    seed(&legacy, &owner);
    let invalid_root = dir.join("owner-artifacts");
    fs::write(&invalid_root, b"a directory was expected").unwrap();
    assert!(purge(&lifecycle, &paths, &legacy, &owner).is_err());
    assert!(lifecycle.is_closed());
    assert!(lifecycle.admit().is_err());
    assert!(legacy.capture_write_epoch().is_err());
    assert!(owner.capture_epoch().is_err());
    assert!(
        journal::load(&paths.purge_journal())
            .unwrap()
            .unwrap()
            .pending
    );
    assert_eq!(owner.list_apps(true).unwrap().len(), 1);
    fs::remove_file(&invalid_root).unwrap();
    purge(&lifecycle, &paths, &legacy, &owner).unwrap();
    assert_eq!(lifecycle.epoch(), 1);
    assert!(!lifecycle.is_closed());
    assert!(owner.list_apps(true).unwrap().is_empty());
}

#[test]
fn mutation_timeout_does_not_delete_or_complete_the_pending_journal() {
    let dir = crate::testing::temp_dir("data-lifecycle-timeout");
    let paths = AppPaths::under(&dir);
    let (lifecycle, legacy, owner) = open(&paths);
    seed(&legacy, &owner);
    let lease = lifecycle.admit().unwrap();
    lifecycle
        .begin_purge(Theme::Dark, VisualizationMode::TwoD)
        .unwrap();
    assert!(lifecycle
        .wait_for_idle_with_timeout(Duration::from_millis(1))
        .is_err());
    assert!(
        journal::load(&paths.purge_journal())
            .unwrap()
            .unwrap()
            .pending
    );
    assert_eq!(owner.list_apps(true).unwrap().len(), 1);
    drop(lease);
    purge(&lifecycle, &paths, &legacy, &owner).unwrap();
    assert_eq!(lifecycle.epoch(), 1);
}

#[test]
fn an_unusable_store_does_not_prevent_the_other_store_from_opening() {
    let owner_dir = crate::testing::temp_dir("data-lifecycle-owner-corrupt");
    let owner_paths = AppPaths::under(&owner_dir);
    fs::write(owner_paths.owner_workspace(), b"not sqlite").unwrap();
    let (lifecycle, legacy, owner) = open(&owner_paths);
    assert!(!lifecycle.is_closed());
    assert!(matches!(owner.state(), OwnerStoreState::Unavailable { .. }));
    legacy
        .write(|conn| settings_repository::put(conn, "still_usable", &true))
        .unwrap();
    assert_eq!(
        fs::read(owner_paths.owner_workspace()).unwrap(),
        b"not sqlite"
    );

    let legacy_dir = crate::testing::temp_dir("data-lifecycle-legacy-corrupt");
    let legacy_paths = AppPaths::under(&legacy_dir);
    fs::write(&legacy_paths.db.database, b"not sqlite").unwrap();
    let (lifecycle, legacy, owner) = open(&legacy_paths);
    assert!(!lifecycle.is_closed());
    assert!(!legacy.state().is_ready());
    assert!(matches!(owner.state(), OwnerStoreState::Ready));
    owner
        .write_at_epoch(owner.capture_epoch().unwrap(), |_| Ok(()))
        .unwrap();
    assert_eq!(fs::read(&legacy_paths.db.database).unwrap(), b"not sqlite");
}
