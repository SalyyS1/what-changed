#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::HashSet;

use rusqlite::Connection;

use super::super::{
    by_id, insert_batch, insert_batch_preserving_entities,
    insert_batch_preserving_entities_with_timings,
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

fn assert_success(timings: BatchInsertionTimings, rows: usize) {
    assert_eq!(timings.batches, 1);
    assert_eq!(timings.attempted_rows, rows);
    assert_eq!(timings.committed_batches, 1);
    assert_eq!(timings.committed_rows, rows);
    assert_eq!(timings.failed_batches, 0);
    assert_eq!(timings.empty_batches, 0);
    assert_eq!(timings.completed_phases, 4);
    assert!(
        timings.repository_total
            >= timings.begin_prepare
                + timings.event_entity_loop
                + timings.reconciliation
                + timings.commit
    );
}

#[test]
fn disabled_insert_paths_read_no_attribution_clocks() {
    let conn = connection();
    CLOCK_READS.with(|reads| reads.set(0));
    insert_batch(&conn, &events()).unwrap();
    insert_batch_preserving_entities(&conn, &events(), HashSet::new()).unwrap();
    insert_batch(&conn, &[]).unwrap();
    assert_eq!(CLOCK_READS.with(|reads| reads.get()), 0);
}

#[test]
fn captured_insert_keeps_actual_rows_ids_and_entity_ownership() {
    let old = connection();
    let captured_conn = connection();
    let batch = events();
    // Both histories already contain one real entity the synthetic policy must
    // preserve, including display name, timestamps and metadata.
    for conn in [&old, &captured_conn] {
        insert_batch(conn, &batch[..1]).unwrap();
        conn.execute(
            "UPDATE entities SET display_name='Real evidence', metadata_json='{}'",
            [],
        )
        .unwrap();
    }
    let expected = insert_batch_preserving_entities(&old, &batch, HashSet::new()).unwrap();
    CLOCK_READS.with(|reads| reads.set(0));
    let captured =
        insert_batch_preserving_entities_with_timings(&captured_conn, &batch, HashSet::new());
    assert_eq!(CLOCK_READS.with(|reads| reads.get()), 6);
    assert_success(captured.timings, batch.len());
    let actual = captured.value.unwrap();
    assert_eq!(actual, expected);
    assert!(!actual.1.contains(&batch[0].get().entity_id));
    for id in actual.0 {
        assert_eq!(
            serde_json::to_value(by_id(&captured_conn, id).unwrap()).unwrap(),
            serde_json::to_value(by_id(&old, id).unwrap()).unwrap()
        );
    }
    for event in batch {
        assert_eq!(
            serde_json::to_value(
                entity_repository::by_id(&captured_conn, &event.get().entity_id).unwrap()
            )
            .unwrap(),
            serde_json::to_value(entity_repository::by_id(&old, &event.get().entity_id).unwrap())
                .unwrap()
        );
    }
}

#[test]
fn real_mid_batch_failure_rolls_back_and_next_capture_is_fresh() {
    let old = connection();
    let captured_conn = connection();
    let batch = events();
    for conn in [&old, &captured_conn] {
        conn.execute_batch("CREATE TRIGGER fail_second_insert BEFORE INSERT ON events \
            WHEN (SELECT COUNT(*) FROM events) = 1 BEGIN SELECT RAISE(ABORT, 'fixture rollback'); END;").unwrap();
    }
    let old_error = insert_batch_preserving_entities(&old, &batch, HashSet::new()).unwrap_err();
    let captured =
        insert_batch_preserving_entities_with_timings(&captured_conn, &batch, HashSet::new());
    assert_eq!(
        captured.value.unwrap_err().to_string(),
        old_error.to_string()
    );
    assert_eq!(captured.timings.batches, 1);
    assert_eq!(captured.timings.attempted_rows, 4);
    assert_eq!(captured.timings.failed_batches, 1);
    assert_eq!(captured.timings.committed_batches, 0);
    assert_eq!(captured.timings.committed_rows, 0);
    assert_eq!(captured.timings.completed_phases, 1);
    for conn in [&old, &captured_conn] {
        assert!(conn.is_autocommit());
        for table in ["events", "entities"] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0);
        }
    }
    captured_conn
        .execute_batch("DROP TRIGGER fail_second_insert")
        .unwrap();
    let next =
        insert_batch_preserving_entities_with_timings(&captured_conn, &batch, HashSet::new());
    assert_eq!(next.value.unwrap().0.len(), 4);
    assert_success(next.timings, 4);
}

#[test]
fn preparation_error_and_empty_capture_do_not_claim_commits() {
    let missing = Connection::open_in_memory().unwrap();
    let failed = insert_batch_preserving_entities_with_timings(&missing, &events(), HashSet::new());
    assert!(failed.value.is_err());
    assert!(missing.is_autocommit());
    assert_eq!(failed.timings.completed_phases, 0);
    assert_eq!(failed.timings.failed_batches, 1);
    assert_eq!(failed.timings.committed_rows, 0);
    let empty = insert_batch_preserving_entities_with_timings(&missing, &[], HashSet::new());
    assert!(empty.value.unwrap().0.is_empty());
    assert_eq!(empty.timings.empty_batches, 1);
    assert_eq!(empty.timings.committed_batches, 0);
    assert_eq!(empty.timings.failed_batches, 0);
    assert_eq!(empty.timings.completed_phases, 0);
}

#[test]
fn deferred_commit_failure_includes_rollback_but_no_committed_rows() {
    let conn = connection();
    conn.execute_batch("PRAGMA foreign_keys=ON; \
        CREATE TABLE fixture_parent(id INTEGER PRIMARY KEY); \
        CREATE TABLE fixture_child(parent_id INTEGER REFERENCES fixture_parent(id) DEFERRABLE INITIALLY DEFERRED); \
        CREATE TRIGGER fail_commit AFTER INSERT ON events BEGIN INSERT INTO fixture_child VALUES(1); END;").unwrap();
    let captured = insert_batch_preserving_entities_with_timings(&conn, &events(), HashSet::new());
    assert!(captured.value.is_err());
    assert!(conn.is_autocommit());
    assert_eq!(captured.timings.completed_phases, 3);
    assert_eq!(captured.timings.failed_batches, 1);
    assert_eq!(captured.timings.committed_rows, 0);
    assert!(
        captured.timings.repository_total
            >= captured.timings.begin_prepare
                + captured.timings.event_entity_loop
                + captured.timings.reconciliation
                + captured.timings.commit
    );
    for table in ["events", "entities", "fixture_child"] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}

#[test]
fn caller_unwinding_and_other_threads_cannot_leave_capture_state() {
    let conn = connection();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _unfinished = Recorder::start(4);
        insert_batch(&conn, &events()).unwrap();
        panic!("owned unfinished insertion recorder panic");
    }))
    .is_err());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let captured =
            insert_batch_preserving_entities_with_timings(&conn, &events(), HashSet::new());
        assert_success(captured.timings, 4);
        captured.value.unwrap();
        panic!("owned insertion diagnostic caller panic");
    }))
    .is_err());
    CLOCK_READS.with(|reads| reads.set(0));
    insert_batch(&conn, &events()).unwrap();
    assert_eq!(CLOCK_READS.with(|reads| reads.get()), 0);
    let sibling = std::thread::spawn(|| {
        let conn = connection();
        let captured =
            insert_batch_preserving_entities_with_timings(&conn, &events(), HashSet::new());
        captured.value.unwrap();
        assert_success(captured.timings, 4);
    });
    sibling.join().unwrap();
    let next = insert_batch_preserving_entities_with_timings(&conn, &events(), HashSet::new());
    next.value.unwrap();
    assert_success(next.timings, 4);
}

#[test]
fn aggregation_adds_counts_and_durations_without_retaining_history() {
    let conn = connection();
    let mut total = BatchInsertionTimings::default();
    for _ in 0..2 {
        let captured =
            insert_batch_preserving_entities_with_timings(&conn, &events(), HashSet::new());
        captured.value.unwrap();
        total.accumulate(captured.timings);
    }
    assert_eq!(total.batches, 2);
    assert_eq!(total.attempted_rows, 8);
    assert_eq!(total.committed_rows, 8);
    assert_eq!(total.completed_phases, 8);
}
