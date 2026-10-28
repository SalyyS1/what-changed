#![cfg(test)]

use super::*;

#[test]
fn a_flushed_temporary_record_recovers_a_missing_main_journal() {
    let dir = crate::testing::temp_dir("data-journal-staged");
    let path = AppPaths::under(&dir).purge_journal();
    let staged = inventory::companion(&path, ".tmp");
    let record = PurgeJournal {
        epoch: 2,
        pending: true,
        ..PurgeJournal::default()
    };
    fs::write(&staged, serde_json::to_vec(&record).unwrap()).unwrap();
    assert_eq!(journal::load(&path).unwrap(), Some(record));
    assert!(path.exists());
    assert!(!staged.exists());
}

#[test]
fn malformed_or_backwards_temporary_records_fail_closed() {
    let dir = crate::testing::temp_dir("data-journal-invalid");
    let paths = AppPaths::under(&dir);
    let path = paths.purge_journal();
    let staged = inventory::companion(&path, ".tmp");
    journal::save(
        &path,
        &PurgeJournal {
            epoch: 3,
            ..PurgeJournal::default()
        },
    )
    .unwrap();
    fs::write(&staged, b"{\"version\":1,").unwrap();
    assert!(DataLifecycle::prepare(&paths).is_err());
    fs::write(
        &staged,
        serde_json::to_vec(&PurgeJournal {
            epoch: 2,
            pending: true,
            ..PurgeJournal::default()
        })
        .unwrap(),
    )
    .unwrap();
    assert!(DataLifecycle::prepare(&paths).is_err());
}

#[test]
fn normal_shutdown_never_creates_a_deletion_intent() {
    let dir = crate::testing::temp_dir("data-journal-shutdown");
    let paths = AppPaths::under(&dir);
    let (lifecycle, legacy, owner) = open(&paths);
    seed(&legacy, &owner);
    lifecycle.begin_shutdown();
    lifecycle.wait_for_idle().unwrap();
    owner.shutdown().unwrap();
    legacy.close();
    assert!(!paths.purge_journal().exists());
    let (restarted, legacy, owner) = open(&paths);
    assert!(!restarted.is_closed());
    assert_eq!(owner.list_apps(true).unwrap().len(), 1);
    assert_eq!(
        legacy
            .read(|conn| settings_repository::get::<String>(conn, "private_fixture"))
            .unwrap()
            .as_deref(),
        Some("legacy-secret")
    );
}

#[test]
fn journal_rejects_unknown_text_fields_and_unsupported_epochs() {
    let dir = crate::testing::temp_dir("data-journal-shape");
    let paths = AppPaths::under(&dir);
    for invalid in [
        r#"{"version":1,"epoch":0,"pending":true,"theme":"system","visualization":"auto"}"#,
        r#"{"version":1,"epoch":1,"pending":true,"theme":"path-from-user","visualization":"auto"}"#,
        r#"{"version":1,"epoch":1,"pending":true,"theme":"system","visualization":"auto","path":"outside"}"#,
    ] {
        fs::write(paths.purge_journal(), invalid).unwrap();
        assert!(DataLifecycle::prepare(&paths).is_err());
    }
}
