#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::HashSet;

use rusqlite::{types::Value, Connection};

use super::super::{
    insert_batch, insert_batch_preserving_entities, insert_batch_preserving_entities_in_transaction,
};
use super::*;
use crate::privacy::Sanitizer;
use crate::storage::{entity_repository, migrations};

fn connection() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrations::run(&mut conn, migrations::MIGRATIONS, std::path::Path::new(".")).unwrap();
    conn
}

fn events() -> Vec<crate::privacy::Sanitized<crate::domain::NormalizedEvent>> {
    (0..4)
        .map(|seed| {
            Sanitizer::new()
                .sanitize(crate::testing::synthetic_event(
                    seed,
                    1_700_000_000_000 + seed as i64,
                ))
                .0
        })
        .collect()
}

// Compare every persisted column without semantic timestamp normalization.
fn rows(conn: &Connection, table: &str) -> Vec<Vec<Value>> {
    let mut statement = conn
        .prepare(&format!("SELECT * FROM {table} ORDER BY id"))
        .unwrap();
    let persisted = statement
        .query_map([], |row| {
            (0..row.as_ref().column_count())
                .map(|column| row.get(column))
                .collect()
        })
        .unwrap();
    persisted.collect::<rusqlite::Result<_>>().unwrap()
}

fn assert_completed(timings: BodyInsertionTimings, inputs: usize) {
    assert_eq!(timings.bodies, 1);
    assert_eq!(timings.attempted_rows, inputs);
    assert_eq!(timings.completed_bodies, 1);
    assert_eq!(timings.completed_rows, inputs);
    assert_eq!(timings.failed_bodies, 0);
    assert_eq!(timings.empty_bodies, 0);
    assert_eq!(timings.completed_phases, 3);
    assert!(
        timings.body_total
            >= timings.preparation + timings.event_entity_loop + timings.reconciliation
    );
}

#[test]
fn body_success_is_uncommitted_and_outer_drop_rolls_back() {
    let conn = connection();
    let tx = conn.unchecked_transaction().unwrap();
    CLOCK_READS.with(|reads| reads.set(0));
    let outcome =
        insert_batch_preserving_entities_in_transaction(&tx, &events(), HashSet::new(), true);
    assert_eq!(CLOCK_READS.with(|reads| reads.get()), 4);
    assert_completed(outcome.timings.unwrap(), 4);
    let (ids, owned) = outcome.value.unwrap();
    assert_eq!(ids.len(), 4);
    assert_eq!(owned.len(), 4);
    assert!(!tx.is_autocommit());
    assert_eq!(rows(&tx, "events").len(), 4);
    drop(tx);
    assert!(conn.is_autocommit());
    assert!(rows(&conn, "events").is_empty());
    assert!(rows(&conn, "entities").is_empty());
}

#[test]
fn outer_commit_matches_legacy_rows_and_preserves_real_entities() {
    let old = connection();
    let candidate = connection();
    let batch = events();
    for conn in [&old, &candidate] {
        insert_batch(conn, &batch[..1]).unwrap();
        conn.execute(
            "UPDATE entities SET display_name='Real evidence', metadata_json='{}'",
            [],
        )
        .unwrap();
    }
    let real_before = rows(&candidate, "entities");
    let expected = insert_batch_preserving_entities(&old, &batch, HashSet::new()).unwrap();
    let tx = candidate.unchecked_transaction().unwrap();
    let actual = insert_batch_preserving_entities_in_transaction(&tx, &batch, HashSet::new(), true);
    assert_completed(actual.timings.unwrap(), 4);
    let actual = actual.value.unwrap();
    assert_eq!(actual, expected);
    assert!(!actual.1.contains(&batch[0].get().entity_id));
    tx.commit().unwrap();
    for table in ["events", "entities", "entity_relationships"] {
        assert_eq!(rows(&candidate, table), rows(&old, table));
    }
    let real_after = rows(&candidate, "entities");
    assert!(real_after.contains(&real_before[0]));
}

#[test]
fn body_error_returns_no_ownership_and_requires_outer_abandonment() {
    let old = connection();
    let candidate = connection();
    for conn in [&old, &candidate] {
        conn.execute_batch("CREATE TRIGGER fail_second_insert BEFORE INSERT ON events \
            WHEN (SELECT COUNT(*) FROM events) = 1 BEGIN SELECT RAISE(ABORT, 'fixture rollback'); END;").unwrap();
    }
    let expected = insert_batch_preserving_entities(&old, &events(), HashSet::new()).unwrap_err();
    let tx = candidate.unchecked_transaction().unwrap();
    CLOCK_READS.with(|reads| reads.set(0));
    let outcome =
        insert_batch_preserving_entities_in_transaction(&tx, &events(), HashSet::new(), true);
    assert_eq!(CLOCK_READS.with(|reads| reads.get()), 3);
    assert_eq!(outcome.value.unwrap_err().to_string(), expected.to_string());
    let timings = outcome.timings.unwrap();
    assert_eq!(timings.attempted_rows, 4);
    assert_eq!(timings.failed_bodies, 1);
    assert_eq!(timings.completed_bodies, 0);
    assert_eq!(timings.completed_rows, 0);
    assert_eq!(timings.completed_phases, 1);
    // ABORT undoes the failing statement, not the already processed inputs.
    assert!(!tx.is_autocommit());
    assert_eq!(rows(&tx, "events").len(), 1);
    assert_eq!(rows(&tx, "entities").len(), 1);
    drop(tx);
    assert!(rows(&candidate, "events").is_empty());
    assert!(rows(&candidate, "entities").is_empty());
    candidate
        .execute_batch("DROP TRIGGER fail_second_insert")
        .unwrap();
    let tx = candidate.unchecked_transaction().unwrap();
    let next =
        insert_batch_preserving_entities_in_transaction(&tx, &events(), HashSet::new(), true);
    assert_completed(next.timings.unwrap(), 4);
    assert_eq!(next.value.unwrap().1.len(), 4);
    tx.rollback().unwrap();
}

#[test]
fn preparation_error_and_empty_body_keep_transaction_ownership() {
    let conn = Connection::open_in_memory().unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    CLOCK_READS.with(|reads| reads.set(0));
    let failed =
        insert_batch_preserving_entities_in_transaction(&tx, &events(), HashSet::new(), true);
    assert_eq!(CLOCK_READS.with(|reads| reads.get()), 2);
    assert!(failed.value.is_err());
    let timings = failed.timings.unwrap();
    assert_eq!(timings.failed_bodies, 1);
    assert_eq!(timings.completed_rows, 0);
    assert_eq!(timings.completed_phases, 0);
    assert!(!tx.is_autocommit());
    drop(tx);
    let tx = conn.unchecked_transaction().unwrap();
    let owned = HashSet::from(["already-owned".to_owned()]);
    CLOCK_READS.with(|reads| reads.set(0));
    // No schema exists: success therefore also proves no insertion SQL ran.
    let empty = insert_batch_preserving_entities_in_transaction(&tx, &[], owned.clone(), true);
    assert_eq!(empty.value.unwrap(), (Vec::new(), owned));
    let timings = empty.timings.unwrap();
    assert_eq!(timings.empty_bodies, 1);
    assert_eq!(timings.completed_bodies, 0);
    assert_eq!(timings.attempted_rows, 0);
    assert_eq!(timings.completed_rows, 0);
    assert_eq!(timings.failed_bodies, 0);
    assert_eq!(timings.completed_phases, 0);
    assert_eq!(timings.body_total, Duration::ZERO);
    assert_eq!(
        timings.preparation + timings.event_entity_loop + timings.reconciliation,
        Duration::ZERO
    );
    assert_eq!(CLOCK_READS.with(|reads| reads.get()), 0);
    assert!(!tx.is_autocommit());
}

#[test]
fn successive_bodies_refresh_owned_entities_without_committing() {
    let conn = connection();
    let tx = conn.unchecked_transaction().unwrap();
    let batch = events();
    let first =
        insert_batch_preserving_entities_in_transaction(&tx, &batch[..1], HashSet::new(), false);
    let (_, owned) = first.value.unwrap();
    let mut changed = batch[0].get().clone();
    changed.timestamp += 100;
    changed.entity_display_name = "Refreshed owned entity".to_owned();
    let changed = Sanitizer::new().sanitize(changed).0;
    let second = insert_batch_preserving_entities_in_transaction(&tx, &[changed], owned, true);
    assert_completed(second.timings.unwrap(), 1);
    assert_eq!(second.value.unwrap().1.len(), 1);
    let entity = entity_repository::by_id(&tx, &batch[0].get().entity_id)
        .unwrap()
        .unwrap();
    assert_eq!(entity.first_seen, batch[0].get().timestamp);
    assert_eq!(entity.last_seen, batch[0].get().timestamp + 100);
    assert_eq!(entity.display_name, "Refreshed owned entity");
    assert_eq!(rows(&tx, "events").len(), 2);
    tx.rollback().unwrap();
    assert!(rows(&conn, "events").is_empty());
    assert!(rows(&conn, "entities").is_empty());
}

#[test]
fn disabled_body_capture_reads_no_clocks_on_success_error_or_empty() {
    let conn = connection();
    CLOCK_READS.with(|reads| reads.set(0));
    let tx = conn.unchecked_transaction().unwrap();
    let inserted =
        insert_batch_preserving_entities_in_transaction(&tx, &events(), HashSet::new(), false);
    assert!(inserted.timings.is_none());
    let (_, owned) = inserted.value.unwrap();
    let empty = insert_batch_preserving_entities_in_transaction(&tx, &[], owned, false);
    assert!(empty.timings.is_none());
    assert_eq!(empty.value.unwrap().1.len(), 4);
    tx.rollback().unwrap();
    conn.execute_batch(
        "CREATE TRIGGER fail_second_insert BEFORE INSERT ON events \
        WHEN (SELECT COUNT(*) FROM events) = 1 BEGIN SELECT RAISE(ABORT, 'fixture rollback'); END;",
    )
    .unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    let mid_body =
        insert_batch_preserving_entities_in_transaction(&tx, &events(), HashSet::new(), false);
    assert!(mid_body.value.is_err());
    assert!(mid_body.timings.is_none());
    tx.rollback().unwrap();
    let missing = Connection::open_in_memory().unwrap();
    let tx = missing.unchecked_transaction().unwrap();
    let failed =
        insert_batch_preserving_entities_in_transaction(&tx, &events(), HashSet::new(), false);
    assert!(failed.value.is_err());
    assert!(failed.timings.is_none());
    assert_eq!(CLOCK_READS.with(|reads| reads.get()), 0);
}

#[test]
fn caller_unwind_rolls_back_completed_body_and_leaves_no_capture_state() {
    let conn = connection();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let tx = conn.unchecked_transaction().unwrap();
        let outcome =
            insert_batch_preserving_entities_in_transaction(&tx, &events(), HashSet::new(), true);
        assert_completed(outcome.timings.unwrap(), 4);
        outcome.value.unwrap();
        panic!("transaction owner unwinds before commit");
    }))
    .is_err());
    assert!(conn.is_autocommit());
    assert!(rows(&conn, "events").is_empty());
    assert!(rows(&conn, "entities").is_empty());
    CLOCK_READS.with(|reads| reads.set(0));
    let tx = conn.unchecked_transaction().unwrap();
    let next =
        insert_batch_preserving_entities_in_transaction(&tx, &events(), HashSet::new(), true);
    assert_completed(next.timings.unwrap(), 4);
    assert_eq!(CLOCK_READS.with(|reads| reads.get()), 4);
    assert_eq!(next.value.unwrap().0.len(), 4);
}
