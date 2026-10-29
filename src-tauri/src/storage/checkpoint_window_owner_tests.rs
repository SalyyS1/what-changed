#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::tests::assert_clean;
use super::*;
use crate::error::{AppError, DbError};
use crate::storage::checkpoint_repository::{event_ids_in_window, MAX_EVENT_IDS_IN_WINDOW};
use crate::storage::page_plan_capture::test_fixture::{connection, insert};
use crate::storage::page_plan_capture::{PageAccessKind, PageExplainError, PagePlanNodeKind};
use crate::testing::synthetic_event;

fn actual(conn: &Connection, from: i64, to: i64) -> Vec<i64> {
    let original = event_ids_in_window(conn, from, to).unwrap();
    let captured = capture_checkpoint_window_plan(|| event_ids_in_window(conn, from, to)).unwrap();
    let value = captured.value.unwrap();
    assert_eq!(value, original);
    let evidence = captured.evidence.unwrap();
    assert_eq!(evidence.invocations, 1);
    assert_eq!(evidence.bindings, Some(3));
    assert_eq!(evidence.statement_parameters, Some(3));
    assert!(evidence.prepared && evidence.query_started && evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, Some(value.len()));
    let PageExplainOutcome::Complete(nodes) = evidence.explain else {
        panic!("actual window plan must be complete");
    };
    assert!(nodes.iter().any(|node| matches!(&node.kind, PagePlanNodeKind::Events(access) if access.operation==PageAccessKind::Search && access.index.is_some())));
    assert!(nodes.iter().all(|node| !matches!(&node.kind, PagePlanNodeKind::Events(access) if access.operation==PageAccessKind::Scan && access.index.is_none())));
    value
}

#[test]
fn normalized_inclusive_bounds_preserve_timestamp_id_ties_and_empty_windows() {
    let conn = connection();
    // Deliberately insert out of timestamp order so ID-only ordering cannot pass.
    let ids = insert(
        &conn,
        [20, 9, 10, 21, 20, 10]
            .into_iter()
            .enumerate()
            .map(|(index, timestamp)| synthetic_event(index as u64, timestamp))
            .collect(),
    );
    let ordered = [ids[2], ids[5], ids[0], ids[4]];
    assert_eq!(actual(&conn, 10, 20), ordered);
    assert_eq!(actual(&conn, 20, 10), ordered);
    assert_eq!(actual(&conn, 10, 10), [ids[2], ids[5]]);
    assert!(actual(&conn, 11, 19).is_empty());
    assert_clean(&conn);
}

#[test]
fn actual_window_cap_preserves_first_ten_thousand_tied_ids() {
    let conn = connection();
    let ids = insert(
        &conn,
        (0..MAX_EVENT_IDS_IN_WINDOW + 2)
            .map(|index| synthetic_event(index as u64, 10))
            .collect(),
    );
    assert_eq!(MAX_EVENT_IDS_IN_WINDOW, 10_000);
    assert_eq!(actual(&conn, 10, 10), ids[..MAX_EVENT_IDS_IN_WINDOW]);
    assert_clean(&conn);
}

#[test]
fn actual_prepare_and_incompatible_schema_decode_errors_remain_incomplete() {
    let missing = Connection::open_in_memory().unwrap();
    let original = event_ids_in_window(&missing, 0, 100).unwrap_err();
    let captured =
        capture_checkpoint_window_plan(|| event_ids_in_window(&missing, 0, 100)).unwrap();
    let error = captured.value.unwrap_err();
    assert_eq!(error.to_string(), original.to_string());
    assert!(matches!(
        &error,
        AppError::Db(DbError::Sqlite {
            operation: "prepare checkpoint event window",
            source: rusqlite::Error::SqliteFailure(..)
        })
    ));
    let evidence = captured.evidence.unwrap();
    assert!(!evidence.prepared && !evidence.query_started && !evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, None);
    assert_eq!(evidence.explain, PageExplainOutcome::NotAttempted);
    missing.execute_batch("CREATE TABLE events(id INTEGER PRIMARY KEY,timestamp INTEGER); CREATE INDEX idx_events_timestamp ON events(timestamp)").unwrap();
    assert_clean(&missing);

    // Real SQLite incompatible-schema decoding boundary, NOT a reachable
    // migrated INTEGER PRIMARY KEY corruption path or mocked row conversion.
    let incompatible = Connection::open_in_memory().unwrap();
    incompatible.execute_batch("CREATE TABLE events(id TEXT,timestamp INTEGER); CREATE INDEX idx_events_timestamp ON events(timestamp); INSERT INTO events VALUES('private-invalid-id',10)").unwrap();
    let original = event_ids_in_window(&incompatible, 0, 100).unwrap_err();
    let captured =
        capture_checkpoint_window_plan(|| event_ids_in_window(&incompatible, 0, 100)).unwrap();
    let error = captured.value.unwrap_err();
    assert_eq!(error.to_string(), original.to_string());
    assert!(matches!(
        &error,
        AppError::Db(DbError::Sqlite {
            operation: "read checkpoint event window",
            source: rusqlite::Error::InvalidColumnType(..)
        })
    ));
    let evidence = captured.evidence.unwrap();
    assert!(evidence.prepared && evidence.query_started && !evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, None);
    assert!(matches!(evidence.explain, PageExplainOutcome::Complete(_)));
    assert!(!format!("{evidence:?}").contains("private-invalid-id"));
    incompatible.execute("DELETE FROM events", []).unwrap();
    assert_clean(&incompatible);
}

#[test]
fn actual_unknown_schema_index_changes_only_plan_evidence() {
    let conn = connection();
    insert(&conn, vec![synthetic_event(1, 10)]);
    let indexes = {
        let mut statement = conn
            .prepare("SELECT name FROM sqlite_schema WHERE type='index' AND tbl_name='events'")
            .unwrap();
        let indexes = statement
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        indexes
    };
    for index in indexes {
        conn.execute_batch(&format!("DROP INDEX {index}")).unwrap();
    }
    conn.execute_batch("CREATE INDEX private_window_index ON events(timestamp)")
        .unwrap();
    let original = event_ids_in_window(&conn, 0, 100).unwrap();
    let captured = capture_checkpoint_window_plan(|| event_ids_in_window(&conn, 0, 100)).unwrap();
    assert_eq!(captured.value.unwrap(), original);
    let evidence = captured.evidence.unwrap();
    assert_eq!(
        evidence.explain,
        PageExplainOutcome::Failed(PageExplainError::UnsupportedVocabulary)
    );
    assert!(evidence.query_started && evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, Some(1));
    assert!(!format!("{evidence:?}").contains("private_window_index"));
    conn.execute_batch(
        "DROP INDEX private_window_index; CREATE INDEX idx_events_timestamp ON events(timestamp)",
    )
    .unwrap();
    assert_clean(&conn);
}
