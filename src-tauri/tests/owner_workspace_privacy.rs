//! Real sanitizer and durable admission checks with temporary SQLite fixtures.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::owner_workspace::*;
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn path() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("what-changed-test-owner-privacy-{}", id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("owner-workspace.sqlite")
}
fn input() -> SaveOwnerAppInput {
    SaveOwnerAppInput {
        operation_id: id(),
        id: id(),
        expected_version: 0,
        name: "Fixture".into(),
        description: "token sk-abcdefghijklmnopqrstuvwx".into(),
        legacy_project_id: None,
        archived: false,
    }
}

#[test]
fn sanitizer_precedes_payload_and_idempotency_digest_persistence() {
    let path = path();
    let store = OwnerStore::open(&path).unwrap();
    let first = input();
    let saved = store.save_app(first.clone()).unwrap();
    assert!(saved.description.contains("[REDACTED]"));
    let mut retry = first;
    retry.description = "token sk-zyxwvutsrqponmlkjihgfedc".into();
    assert_eq!(store.save_app(retry).unwrap(), saved);
    store.flush().unwrap();
    store.shutdown().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    for secret in ["sk-abcdefghijklmnopqrstuvwx", "sk-zyxwvutsrqponmlkjihgfedc"] {
        assert!(!bytes.windows(secret.len()).any(|w| w == secret.as_bytes()));
    }
}

#[test]
fn stale_deferred_producer_cannot_commit_after_purge_or_restart() {
    let path = path();
    let store = OwnerStore::open(&path).unwrap();
    store.save_app(input()).unwrap();
    let before = store.capture_epoch().unwrap();
    let epoch = store.close_admission().unwrap();
    assert!(epoch > before.value());
    assert!(store.capture_epoch().is_err());
    store.reset_after_purge(epoch).unwrap();
    assert!(store.capture_epoch().is_err());
    store.reopen_admission().unwrap();
    let called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let inside = called.clone();
    assert!(store
        .write_at_epoch(before, move |_| {
            inside.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
        .is_err());
    assert!(!called.load(std::sync::atomic::Ordering::SeqCst));
    assert!(store.list_apps(true).unwrap().is_empty());
    store.shutdown().unwrap();
    let reopened = OwnerStore::open(&path).unwrap();
    assert_eq!(reopened.capture_epoch().unwrap().value(), epoch);
    assert!(reopened.write_at_epoch(before, |_| Ok(())).is_err());
}

#[test]
fn purge_removes_only_owned_inventory_and_preserves_external_files() {
    let path = path();
    let store = OwnerStore::open(&path).unwrap();
    store.save_app(input()).unwrap();
    let parent = path.parent().unwrap();
    let backup = parent.join(format!("owner-workspace.pre-migration-{}.sqlite", id()));
    let unrelated = parent.join("user-export.sqlite");
    std::fs::write(&backup, b"private backup canary").unwrap();
    std::fs::write(&unrelated, b"external export").unwrap();
    let epoch = store.close_admission().unwrap();
    store.reset_after_purge(epoch).unwrap();
    assert!(!backup.exists());
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"external export");
    store.reopen_admission().unwrap();
    assert!(store.list_apps(true).unwrap().is_empty());
}

#[test]
fn closed_admission_survives_restart_until_global_coordinator_reopens() {
    let path = path();
    let store = OwnerStore::open(&path).unwrap();
    let epoch = store.close_admission().unwrap();
    store.shutdown().unwrap();
    let reopened = OwnerStore::open(&path).unwrap();
    assert!(reopened.capture_epoch().is_err());
    reopened.synchronize_epoch(epoch + 10).unwrap();
    assert!(reopened.capture_epoch().is_err());
    reopened.reopen_admission().unwrap();
    assert_eq!(reopened.capture_epoch().unwrap().value(), epoch + 10);
}
