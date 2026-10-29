#![cfg(test)]

use super::*;
use crate::domain::{Theme, VisualizationMode};
use switch::Boundary;

#[test]
fn absent_fresh_data_root_has_nothing_to_recover_or_create() {
    let root = crate::testing::temp_dir("generation-fresh-root");
    let paths = AppPaths::under(&root.join("not-created-yet"));
    assert_eq!(recover_before_open(&paths).unwrap(), RecoveryOutcome::Clean);
    assert!(!paths.data.exists());
    let lifecycle = DataLifecycle::prepare(&paths).unwrap();
    assert!(lifecycle.is_closed());
    assert!(!paths.data.exists());
}

fn fixture() -> AppPaths {
    let root = crate::testing::temp_dir("closed-generation-\u{0111}");
    let paths = AppPaths::under(&root);
    let db = Db::open(&paths.db).unwrap();
    let owner = OwnerStore::open(&paths.owner_workspace()).unwrap();
    owner.shutdown().unwrap();
    db.close();
    set_generation(&paths, "old");
    std::fs::write(root.join("external-source.txt"), b"never managed").unwrap();
    paths
}

fn set_generation(paths: &AppPaths, label: &str) {
    for name in ["events.db", "owner-workspace.sqlite"] {
        let conn = rusqlite::Connection::open(paths.data.join(name)).unwrap();
        conn.execute_batch("CREATE TABLE IF NOT EXISTS generation_fixture(value TEXT); DELETE FROM generation_fixture;").unwrap();
        conn.execute("INSERT INTO generation_fixture(value) VALUES(?1)", [label])
            .unwrap();
        conn.close().unwrap();
    }
    let root = paths.data.join("owner-artifacts");
    std::fs::create_dir_all(root.join("objects")).unwrap();
    std::fs::create_dir_all(root.join("objects/nested-branch/empty")).unwrap();
    std::fs::write(root.join("objects/record.txt"), label).unwrap();
    std::fs::write(root.join("objects/nested-branch/nested-record.txt"), label).unwrap();
    std::fs::write(root.join("root-record.txt"), label).unwrap();
}

fn assert_generation(paths: &AppPaths, label: &str) {
    for name in ["events.db", "owner-workspace.sqlite"] {
        let conn = rusqlite::Connection::open(paths.data.join(name)).unwrap();
        let found: String = conn
            .query_row("SELECT value FROM generation_fixture", [], |r| r.get(0))
            .unwrap();
        assert_eq!(found, label);
    }
    assert_eq!(
        std::fs::read_to_string(paths.data.join("owner-artifacts/objects/record.txt")).unwrap(),
        label
    );
    assert_eq!(
        std::fs::read(paths.data.join("external-source.txt")).unwrap(),
        b"never managed"
    );
    for name in ["root-record.txt", "objects/nested-branch/nested-record.txt"] {
        assert_eq!(
            std::fs::read_to_string(paths.data.join("owner-artifacts").join(name)).unwrap(),
            label
        );
    }
    assert!(paths
        .data
        .join("owner-artifacts/objects/nested-branch/empty")
        .is_dir());
}

fn backed_up_old(paths: &AppPaths) -> BackupId {
    let closed = ClosedStores::acquire(paths, None).unwrap();
    create_backup(&closed).unwrap()
}

#[test]
fn sqlite_read_sidecars_do_not_hide_committed_or_artifact_changes() {
    let paths = fixture();
    let before = locking::live(&paths).unwrap().fingerprint;
    let id = backed_up_old(&paths);
    assert_eq!(locking::live(&paths).unwrap().fingerprint, before);
    assert!(paths.data.join("events.db-wal").exists());
    assert_eq!(
        paths.data.join("events.db-wal").metadata().unwrap().len(),
        0
    );

    let artifact = paths.data.join("owner-artifacts/record-shm");
    std::fs::write(&artifact, b"artifact data is never a SQLite sidecar").unwrap();
    let with_artifact = locking::live(&paths).unwrap().fingerprint;
    assert!(with_artifact.contains_key("owner-artifacts/record-shm"));
    std::fs::write(&artifact, b"changed artifact").unwrap();
    assert_ne!(locking::live(&paths).unwrap().fingerprint, with_artifact);

    // A nonempty WAL is conservatively compared, even before SQLite validates
    // its contents. Empty-sidecar normalization cannot hide committed pages.
    std::fs::write(paths.data.join("events.db-wal"), b"changed WAL bytes").unwrap();
    let changed = locking::live(&paths).unwrap().fingerprint;
    assert!(changed.contains_key("events.db-wal"));
    assert_ne!(changed, before);
    let closed = ClosedStores::acquire(&paths, None).unwrap();
    assert!(manifest::validate(&closed, &id).is_ok());
}

#[test]
fn complete_restore_and_explicit_rollback_keep_both_databases_and_artifacts_together() {
    let paths = fixture();
    let old = backed_up_old(&paths);
    set_generation(&paths, "new");
    let mut closed = ClosedStores::acquire(&paths, None).unwrap();
    let receipt = restore(&mut closed, &old).unwrap();
    drop(closed);
    assert_generation(&paths, "old");
    let mut closed = ClosedStores::acquire(&paths, None).unwrap();
    rollback(&mut closed, &receipt).unwrap();
    drop(closed);
    assert_generation(&paths, "new");
    assert_eq!(recover_before_open(&paths).unwrap(), RecoveryOutcome::Clean);
}

#[test]
fn every_switch_boundary_recovers_one_complete_generation_before_open() {
    let mut boundaries = vec![Boundary::Prepared];
    boundaries.extend((0..ITEMS.len()).map(Boundary::Displaced));
    boundaries.extend((0..3).map(Boundary::Promoted));
    boundaries.extend((0..3).map(Boundary::ArtifactDisplaced));
    boundaries.extend((0..3).map(Boundary::ArtifactPromoted));
    boundaries.push(Boundary::DisplacedArtifactsBeforeCleanup);
    boundaries.push(Boundary::PromotedArtifactsBeforeCleanup);
    boundaries.push(Boundary::Committed);
    for stop in boundaries {
        let paths = fixture();
        let old = backed_up_old(&paths);
        set_generation(&paths, "new");
        let mut closed = ClosedStores::acquire(&paths, None).unwrap();
        assert!(
            switch::restore_with_hook(&mut closed, &old, |point| if point == stop {
                Err(error("fixture interruption"))
            } else {
                Ok(())
            })
            .is_err(),
            "{stop:?}"
        );
        drop(closed);
        let recovered = recover_before_open(&paths).unwrap();
        if stop == Boundary::Committed {
            assert_eq!(recovered, RecoveryOutcome::Completed);
            assert_generation(&paths, "old");
        } else {
            assert_eq!(recovered, RecoveryOutcome::RolledBack);
            assert_generation(&paths, "new");
        }
        assert_eq!(recover_before_open(&paths).unwrap(), RecoveryOutcome::Clean);
    }
}

#[test]
fn external_sqlite_handles_block_backup_and_recovery_before_any_live_move() {
    let paths = fixture();
    let conn = rusqlite::Connection::open(&paths.db.database).unwrap();
    let closed = ClosedStores::acquire(&paths, None).unwrap();
    assert!(create_backup(&closed).is_err());
    drop(closed);
    drop(conn);
    assert_generation(&paths, "old");
}

#[test]
fn checksum_and_partial_snapshot_fail_before_any_live_change() {
    let paths = fixture();
    let id = backed_up_old(&paths);
    set_generation(&paths, "new");
    let mut closed = ClosedStores::acquire(&paths, None).unwrap();
    let artifact = closed
        .backup_path(&id)
        .unwrap()
        .join("owner-artifacts/objects/record.txt");
    std::fs::write(artifact, b"tampered").unwrap();
    assert!(restore(&mut closed, &id).is_err());
    let partial = BackupId::new();
    files::directory(&closed.backup_path(&partial).unwrap()).unwrap();
    assert!(restore(&mut closed, &partial).is_err());
    drop(closed);
    assert_generation(&paths, "new");
}

#[test]
fn pending_purge_deletes_owned_backups_and_journal_without_replaying_them() {
    let paths = fixture();
    let old = backed_up_old(&paths);
    set_generation(&paths, "new");
    let mut closed = ClosedStores::acquire(&paths, None).unwrap();
    assert!(
        switch::restore_with_hook(&mut closed, &old, |point| if point == Boundary::Prepared {
            Err(error("fixture interruption"))
        } else {
            Ok(())
        })
        .is_err()
    );
    drop(closed);
    let fence = serde_json::json!({"version":1,"epoch":1,"pending":true,"theme":Theme::System,"visualization":VisualizationMode::Auto});
    std::fs::write(paths.purge_journal(), serde_json::to_vec(&fence).unwrap()).unwrap();
    assert_eq!(
        recover_before_open(&paths).unwrap(),
        RecoveryOutcome::PurgeRequired
    );
    assert!(!paths.data.join(BACKUPS).exists());
    assert!(!paths.data.join(RESTORES).exists());
    assert!(!paths.data.join(JOURNAL).exists());
    // The global purge owner must still delete live stores. This helper has
    // removed restoration routes and must never report global purge success.
    assert_generation(&paths, "new");
}

#[test]
fn completed_purge_epoch_rejects_a_previous_complete_backup() {
    let paths = fixture();
    let old = backed_up_old(&paths);
    set_generation(&paths, "new");
    let fence = serde_json::json!({"version":1,"epoch":1,"pending":false,"theme":Theme::System,"visualization":VisualizationMode::Auto});
    std::fs::write(paths.purge_journal(), serde_json::to_vec(&fence).unwrap()).unwrap();
    let mut closed = ClosedStores::acquire(&paths, None).unwrap();
    assert!(restore(&mut closed, &old).is_err());
    drop(closed);
    assert_generation(&paths, "new");
}

#[test]
fn hardlinked_artifacts_and_windows_path_aliases_are_rejected() {
    let paths = fixture();
    let source = paths.data.join("external-source.txt");
    let linked = paths.data.join("owner-artifacts/alias.txt");
    std::fs::hard_link(&source, &linked).unwrap();
    let closed = ClosedStores::acquire(&paths, None).unwrap();
    assert!(create_backup(&closed).is_err());
    drop(closed);
    for path in [
        "../escape",
        "owner-artifacts/../escape",
        "owner-artifacts/CON",
        "owner-artifacts/a:stream",
        "owner-artifacts/a.",
        "C:/escape",
        "owner-artifacts\\escape",
    ] {
        assert!(files::relative(path).is_err(), "{path}");
    }
    assert_eq!(std::fs::read(source).unwrap(), b"never managed");
}

#[test]
fn a_newer_schema_even_with_updated_checksum_is_not_restorable() {
    let paths = fixture();
    let id = backed_up_old(&paths);
    let closed = ClosedStores::acquire(&paths, None).unwrap();
    let root = closed.backup_path(&id).unwrap();
    let owner = root.join("owner-workspace.sqlite");
    let conn = rusqlite::Connection::open(&owner).unwrap();
    conn.pragma_update(None, "user_version", 999).unwrap();
    conn.close().unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join(files::MANIFEST)).unwrap()).unwrap();
    let file = native::for_read(&owner).unwrap();
    let (size, digest) = files::hash(&file).unwrap();
    drop(file);
    for entry in value["files"].as_array_mut().unwrap() {
        if entry["path"] == "owner-workspace.sqlite" {
            entry["bytes"] = size.into();
            entry["sha256"] = digest.clone().into();
        }
    }
    std::fs::write(
        root.join(files::MANIFEST),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    assert!(manifest::validate(&closed, &id).is_err());
    drop(closed);
    assert_generation(&paths, "old");
}
