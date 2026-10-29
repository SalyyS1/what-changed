#![allow(clippy::panic, clippy::unwrap_used)]

use super::tests::assert_clean;
use super::*;
use crate::{
    checkpoint::prepare_capture,
    domain::{CheckpointItem, CheckpointKind},
    error::{AppError, DbError},
    privacy::Sanitizer,
    storage::{
        checkpoint_repository::{by_id, create, items},
        page_plan_capture::{test_fixture::connection, PageAccessKind, PageExplainError},
    },
};
use rusqlite::Connection;

pub(super) fn item(collector: &str, kind: &str, identity: &str, state: u32) -> CheckpointItem {
    CheckpointItem {
        collector_id: collector.to_owned(),
        entity_kind: kind.to_owned(),
        entity_external_id: identity.to_owned(),
        entity_name: None,
        state: serde_json::json!({"value":state}),
    }
}
fn persist(conn: &Connection, values: Vec<CheckpointItem>) -> i64 {
    let collectors = values
        .iter()
        .map(|item| item.collector_id.clone())
        .collect();
    let (capture, _) = prepare_capture(
        Some("Owned owner fixture".to_owned()),
        CheckpointKind::Manual,
        None,
        10,
        0,
        collectors,
        Vec::new(),
        values,
        &Sanitizer::new(),
    );
    create(conn, &capture).unwrap().id
}

#[test]
fn direct_metadata_zero_one_and_item_empty_three_column_order_are_complete() {
    let conn = connection();
    let values = vec![
        item("z", "a", "a", 1),
        item("a", "b", "a", 2),
        item("a", "a", "z", 3),
        item("a", "a", "a", 4),
    ];
    let id = persist(&conn, values);
    let original = by_id(&conn, id).unwrap();
    let captured = capture_checkpoint_compare_plan(|| by_id(&conn, id)).unwrap();
    assert!(
        captured.value.unwrap() == original,
        "complete metadata differs"
    );
    let evidence = &captured.evidence.reads[0];
    assert!(evidence.prepared && evidence.query_started && evidence.fully_decoded);
    assert_eq!(evidence.bindings, Some(1));
    assert_eq!(evidence.statement_parameters, Some(1));
    assert_eq!(evidence.decoded_rows, Some(1));
    let CheckpointComparePlan::Checkpoint(CheckpointExplainOutcome::Complete(nodes)) =
        &evidence.plan
    else {
        panic!("actual metadata plan incomplete");
    };
    assert!(nodes.iter().any(
        |node| matches!(&node.kind, CheckpointPlanNodeKind::Metadata(access)
        if access.operation==PageAccessKind::Search && access.index==Some("INTEGER PRIMARY KEY"))
    ));
    assert_clean(&conn); // Actual absent metadata must fully decode zero rows.
    let original = items(&conn, id).unwrap();
    let captured = capture_checkpoint_compare_plan(|| items(&conn, id)).unwrap();
    let actual = captured.value.unwrap();
    assert!(actual == original, "complete ordered items differ");
    let expected = vec![
        item("a", "a", "a", 4),
        item("a", "a", "z", 3),
        item("a", "b", "a", 2),
        item("z", "a", "a", 1),
    ];
    assert!(actual == expected, "three-column item order differs");
    let evidence = &captured.evidence.reads[0];
    assert!(evidence.query_started && evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, Some(4));
    let CheckpointComparePlan::Checkpoint(CheckpointExplainOutcome::Complete(nodes)) =
        &evidence.plan
    else {
        panic!("actual item plan incomplete");
    };
    assert!(nodes.iter().any(
        |node| matches!(&node.kind, CheckpointPlanNodeKind::Items(access)
        if access.operation==PageAccessKind::Search && access.index.is_some())
    ));
    assert!(nodes.iter().any(|node| matches!(
        &node.kind,
        CheckpointPlanNodeKind::OrderByTempBTree { .. }
            | CheckpointPlanNodeKind::OrderByRightPartTempBTree
    )));
    let captured = capture_checkpoint_compare_plan(|| items(&conn, -1)).unwrap();
    assert!(captured.value.unwrap().is_empty());
    assert!(captured.evidence.reads[0].fully_decoded);
    assert_eq!(captured.evidence.reads[0].decoded_rows, Some(0));
    assert_clean(&conn);
}

fn incomplete_metadata(conn: &Connection, id: i64, raw: bool) {
    let original = by_id(conn, id).unwrap_err();
    let captured = capture_checkpoint_compare_plan(|| by_id(conn, id)).unwrap();
    let error = captured.value.unwrap_err();
    assert!(
        error.to_string() == original.to_string(),
        "original metadata error changed"
    );
    if raw {
        assert!(matches!(
            error,
            AppError::Db(DbError::Sqlite {
                operation: "read checkpoint",
                source: rusqlite::Error::InvalidColumnType(..)
            })
        ));
    } else {
        assert!(matches!(error, AppError::Db(DbError::Decoding { .. })));
    }
    let evidence = &captured.evidence.reads[0];
    assert!(evidence.prepared && evidence.query_started && !evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, None);
    assert!(!format!("{:?}", captured.evidence).contains("private-invalid"));
}

#[test]
fn metadata_completion_waits_for_raw_kind_and_both_manifest_decodes() {
    let conn = connection();
    let id = persist(&conn, Vec::new());
    // Isolated synthetic persisted decoding boundaries, not normal generated data.
    for (mutation, restore, raw) in [
        (
            "UPDATE checkpoints SET created_at='private-invalid' WHERE id=?1",
            "UPDATE checkpoints SET created_at=10 WHERE id=?1",
            true,
        ),
        (
            "UPDATE checkpoints SET kind='private-invalid' WHERE id=?1",
            "UPDATE checkpoints SET kind='manual' WHERE id=?1",
            false,
        ),
        (
            "UPDATE checkpoints SET missing_collectors='private-invalid' WHERE id=?1",
            "UPDATE checkpoints SET missing_collectors='[]' WHERE id=?1",
            false,
        ),
        (
            "UPDATE checkpoints SET captured_collectors='{}' WHERE id=?1",
            "UPDATE checkpoints SET captured_collectors='[]' WHERE id=?1",
            false,
        ),
    ] {
        conn.execute(mutation, [id]).unwrap();
        incomplete_metadata(&conn, id, raw);
        conn.execute(restore, [id]).unwrap();
        assert_clean(&conn);
    }
}

#[test]
fn item_raw_and_json_errors_never_claim_full_consumption() {
    let conn = connection();
    let id = persist(&conn, vec![item("a", "a", "a", 1), item("a", "a", "b", 2)]);
    for (mutation, restore, raw) in [
        ("UPDATE snapshots SET entity_external_id=NULL WHERE checkpoint_id=?1 AND entity_external_id='b'",
         "UPDATE snapshots SET entity_external_id='b' WHERE checkpoint_id=?1 AND entity_external_id IS NULL", true),
        ("UPDATE snapshots SET state_json='private-invalid' WHERE checkpoint_id=?1 AND entity_external_id='b'",
         "UPDATE snapshots SET state_json='{}' WHERE checkpoint_id=?1 AND entity_external_id='b'", false),
    ] {
        conn.execute(mutation, [id]).unwrap();
        let original = items(&conn, id).unwrap_err();
        let captured = capture_checkpoint_compare_plan(|| items(&conn, id)).unwrap();
        let error = captured.value.unwrap_err();
        assert!(error.to_string() == original.to_string(), "original item error changed");
        if raw { assert!(matches!(error, AppError::Db(DbError::Sqlite {
            operation: "read checkpoint items", source: rusqlite::Error::InvalidColumnType(..) }))); }
        else { assert!(matches!(error, AppError::Db(DbError::Decoding { .. }))); }
        let evidence = &captured.evidence.reads[0];
        assert!(evidence.prepared && evidence.query_started && !evidence.fully_decoded);
        assert_eq!(evidence.decoded_rows, None);
        conn.execute(restore, [id]).unwrap();
        assert_clean(&conn);
    }
}

#[test]
fn real_prepare_failures_and_unknown_private_indexes_do_not_replace_results() {
    let missing = Connection::open_in_memory().unwrap();
    for metadata in [true, false] {
        let original = if metadata {
            by_id(&missing, -1).map(|value| usize::from(value.is_some()))
        } else {
            items(&missing, -1).map(|values| values.len())
        }
        .unwrap_err();
        let captured = capture_checkpoint_compare_plan(|| {
            if metadata {
                by_id(&missing, -1).map(|value| usize::from(value.is_some()))
            } else {
                items(&missing, -1).map(|values| values.len())
            }
        })
        .unwrap();
        let error = captured.value.unwrap_err();
        assert!(
            error.to_string() == original.to_string(),
            "original prepare error changed"
        );
        assert!(matches!(
            error,
            AppError::Db(DbError::Sqlite {
                source: rusqlite::Error::SqliteFailure(..),
                ..
            })
        ));
        let evidence = &captured.evidence.reads[0];
        assert!(!evidence.prepared && !evidence.query_started && !evidence.fully_decoded);
        assert_eq!(
            evidence.plan,
            CheckpointComparePlan::Checkpoint(CheckpointExplainOutcome::NotAttempted)
        );
        assert_clean(&connection());
    }
    let conn = connection();
    let id = persist(&conn, vec![item("a", "a", "a", 1)]);
    conn.execute_batch("DROP INDEX idx_snapshots_checkpoint; DROP INDEX idx_snapshots_checkpoint_collector; CREATE INDEX private_items_index ON snapshots(checkpoint_id)").unwrap();
    let original = items(&conn, id).unwrap();
    let captured = capture_checkpoint_compare_plan(|| items(&conn, id)).unwrap();
    assert!(
        captured.value.unwrap() == original,
        "unknown index altered original items"
    );
    let evidence = &captured.evidence.reads[0];
    assert_eq!(
        evidence.plan,
        CheckpointComparePlan::Checkpoint(CheckpointExplainOutcome::Failed(
            PageExplainError::UnsupportedVocabulary
        ))
    );
    assert!(evidence.query_started && evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, Some(1));
    assert!(!format!("{:?}", captured.evidence).contains("private_items_index"));
    assert_clean(&conn);
}
