#![cfg(test)]

use super::*;

#[test]
fn native_move_guard_refuses_an_open_sqlite_database() {
    let root = crate::testing::temp_dir("generation-sqlite-open");
    let path = root.join("fixture.db");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE fixture(value); INSERT INTO fixture VALUES(42);")
        .unwrap();
    assert!(crate::platform::managed_file_lock::for_move(&path).is_err());
    drop(conn);
    let guard = crate::platform::managed_file_lock::for_move(&path).unwrap();
    assert!(Connection::open(&path).is_err());
    drop(guard);
}

#[test]
fn reserved_destination_allows_sqlite_self_open_and_refuses_name_substitution() {
    let root = crate::testing::temp_dir("generation-sqlite-reservation");
    let path = root.join("reserved.db");
    let alternate = root.join("substitute.db");
    let reservation = crate::platform::managed_file_lock::reserve_sqlite(&path).unwrap();
    assert!(std::fs::rename(&path, &alternate).is_err());
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE fixture(value); INSERT INTO fixture VALUES(42);")
        .unwrap();
    conn.close().unwrap();
    assert!(reservation.metadata().unwrap().len() > 0);
    assert!(std::fs::rename(&path, &alternate).is_err());
    drop(reservation);
    let conn = Connection::open(&path).unwrap();
    let value: i64 = conn
        .query_row("SELECT value FROM fixture", [], |r| r.get(0))
        .unwrap();
    assert_eq!(value, 42);
}

#[test]
fn online_snapshot_includes_committed_wal_rows_in_both_self_contained_databases() {
    let source = crate::testing::temp_dir("generation-wal-source");
    let destination = crate::testing::temp_dir("generation-wal-target");
    let db = crate::storage::Db::open_in(&source).unwrap();
    let owner = OwnerStore::open(&source.join("owner-workspace.sqlite")).unwrap();
    owner.shutdown().unwrap();
    db.close();
    let mut writers = Vec::new();
    for name in ["events.db", "owner-workspace.sqlite"] {
        let writer = Connection::open(source.join(name)).unwrap();
        writer.pragma_update(None, "journal_mode", "WAL").unwrap();
        writer.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
        writer.execute_batch("CREATE TABLE wal_fixture(value TEXT); INSERT INTO wal_fixture VALUES('committed-wal');").unwrap();
        assert!(source.join(format!("{name}-wal")).metadata().unwrap().len() > 0);
        std::fs::write(destination.join(name), b"").unwrap();
        writers.push(writer);
    }
    let schemas = snapshot_pair(&source, &destination).unwrap();
    assert!(schemas.legacy > 0 && schemas.owner > 0);
    for name in ["events.db", "owner-workspace.sqlite"] {
        assert!(!destination.join(format!("{name}-wal")).exists());
        assert!(!destination.join(format!("{name}-shm")).exists());
        let copy = reader(&destination.join(name)).unwrap();
        let observed: String = copy
            .query_row("SELECT value FROM wal_fixture", [], |r| r.get(0))
            .unwrap();
        assert_eq!(observed, "committed-wal");
    }
    drop(writers);
}

#[test]
fn schema_drift_and_newer_versions_are_preserved_and_rejected() {
    let source = crate::testing::temp_dir("generation-schema-source");
    let db = crate::storage::Db::open_in(&source).unwrap();
    let owner = OwnerStore::open(&source.join("owner-workspace.sqlite")).unwrap();
    owner.shutdown().unwrap();
    db.close();
    let conn = Connection::open(source.join("events.db")).unwrap();
    conn.execute(
        "UPDATE schema_migrations SET sha256='different' WHERE version=1",
        [],
    )
    .unwrap();
    drop(conn);
    assert!(validate_pair(&source).is_err());
    let preserved = std::fs::read(source.join("events.db")).unwrap();
    assert!(validate_pair(&source).is_err());
    assert_eq!(std::fs::read(source.join("events.db")).unwrap(), preserved);
}
