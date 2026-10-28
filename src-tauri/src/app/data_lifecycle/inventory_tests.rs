#![cfg(test)]

use super::*;

#[test]
fn exact_inventory_covers_companions_and_preserves_unknown_files() {
    let dir = crate::testing::temp_dir("data-inventory-exact");
    let paths = AppPaths::under(&dir);
    fs::create_dir_all(&paths.db.backups).unwrap();
    fs::create_dir_all(&paths.logs).unwrap();
    let owner_copy = format!(
        "owner-workspace.pre-migration-{}.sqlite",
        uuid::Uuid::new_v4()
    );
    let mut owned = vec![
        "events.db".to_owned(),
        "owner-workspace.sqlite".to_owned(),
        "events.repair.db".to_owned(),
        "events.corrupt-123.db".to_owned(),
        "events.purge-old-123.db".to_owned(),
        "events.purge-fresh-123.db".to_owned(),
        owner_copy,
    ];
    let base = owned.clone();
    for file in base {
        for suffix in ["-wal", "-shm", "-journal"] {
            owned.push(format!("{file}{suffix}"));
        }
    }
    owned.extend([
        "backups/events-pre-v14.db".into(),
        "backups/events-pre-v14.db-wal".into(),
        "diagnostics-123.json".into(),
    ]);
    for name in &owned {
        fs::write(dir.join(name), b"private").unwrap();
    }
    let unrelated = [
        "events.corrupt-note.db",
        "events.purge-old-.db",
        "events.repair.db.txt",
        "owner-workspace.pre-migration-not-a-uuid.sqlite",
        "backups/user-export.db",
        "diagnostics-note.json",
    ];
    for name in unrelated {
        fs::write(dir.join(name), b"keep").unwrap();
    }
    fs::write(paths.logs.join("app.log"), b"private log").unwrap();
    for root in [
        "owner-artifacts",
        "owner-temp-profiles",
        "owner-cached-exports",
    ] {
        fs::create_dir_all(dir.join(root).join("nested")).unwrap();
        fs::write(dir.join(root).join("nested/item"), b"private").unwrap();
    }
    inventory::remove_owned(&paths, true).unwrap();
    for name in owned {
        assert!(!dir.join(name).exists());
    }
    for name in unrelated {
        assert_eq!(fs::read(dir.join(name)).unwrap(), b"keep");
    }
    assert!(fs::read(paths.logs.join("app.log")).unwrap().is_empty());
    assert!(!dir.join("owner-artifacts").exists());
}

#[test]
fn a_directory_with_an_owned_file_name_is_never_recursively_deleted() {
    let dir = crate::testing::temp_dir("data-inventory-file-kind");
    let paths = AppPaths::under(&dir);
    let unexpected = dir.join("events.corrupt-123.db");
    fs::create_dir_all(&unexpected).unwrap();
    fs::write(unexpected.join("keep.txt"), b"keep").unwrap();
    assert!(inventory::remove_owned(&paths, true).is_err());
    assert_eq!(fs::read(unexpected.join("keep.txt")).unwrap(), b"keep");
}

#[cfg(unix)]
#[test]
fn artifact_symlink_cannot_reach_an_external_directory() {
    let dir = crate::testing::temp_dir("data-inventory-link");
    let outside = crate::testing::temp_dir("data-inventory-outside");
    fs::write(outside.join("keep.txt"), b"keep").unwrap();
    std::os::unix::fs::symlink(&outside, dir.join("owner-artifacts")).unwrap();
    assert!(inventory::remove_owned(&AppPaths::under(&dir), true).is_err());
    assert_eq!(fs::read(outside.join("keep.txt")).unwrap(), b"keep");
}

#[cfg(windows)]
#[test]
fn a_locked_owned_artifact_keeps_the_purge_pending_until_retry() {
    use std::os::windows::fs::OpenOptionsExt;
    let dir = crate::testing::temp_dir("data-inventory-locked");
    let paths = AppPaths::under(&dir);
    let (lifecycle, legacy, owner) = open(&paths);
    fs::create_dir_all(dir.join("owner-artifacts")).unwrap();
    let artifact = dir.join("owner-artifacts/locked.txt");
    fs::write(&artifact, b"private").unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&artifact)
        .unwrap();
    assert!(purge(&lifecycle, &paths, &legacy, &owner).is_err());
    assert!(lifecycle.is_closed());
    assert!(
        journal::load(&paths.purge_journal())
            .unwrap()
            .unwrap()
            .pending
    );
    drop(held);
    purge(&lifecycle, &paths, &legacy, &owner).unwrap();
    assert!(!artifact.exists());
}
