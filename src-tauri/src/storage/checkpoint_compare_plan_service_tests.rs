#![allow(clippy::panic, clippy::unwrap_used)]

use super::test_fixture::Fixture;
use super::*;
use crate::storage::page_plan_capture::{PageAccessKind, PageExplainOutcome, PagePlanNodeKind};
use crate::{
    domain::CheckpointItem,
    error::{AppError, DbError},
    storage::checkpoint_repository::by_id,
};

fn service_item(identity: &str, running: bool) -> CheckpointItem {
    CheckpointItem {
        collector_id: "services".to_owned(),
        entity_kind: "service".to_owned(),
        entity_external_id: identity.to_owned(),
        entity_name: Some(identity.to_owned()),
        state: serde_json::json!({ "name":identity,"display_name":identity,
            "state":if running { "running" } else { "stopped" },"start_type":"automatic",
            "config_available":true,"process_id":1 }),
    }
}
fn pair(fixture: &Fixture) -> (i64, i64) {
    let left = fixture.checkpoint(
        10,
        vec![
            service_item("kept", true),
            service_item("changed", true),
            service_item("removed", true),
        ],
    );
    let right = fixture.checkpoint(
        20,
        vec![
            service_item("added", true),
            service_item("changed", false),
            service_item("kept", true),
        ],
    );
    (left, right)
}
fn clean(fixture: &Fixture) {
    let captured =
        capture_checkpoint_compare_plan(|| fixture.db().read(|conn| by_id(conn, -1))).unwrap();
    assert!(captured.value.unwrap().is_none());
    assert_eq!(captured.evidence.failure, None);
    assert!(captured.evidence.reads[0].fully_decoded);
}

#[test]
fn actual_some_comparison_preserves_full_semantic_return_and_five_read_order() {
    let fixture = Fixture::new();
    let (left, right) = pair(&fixture);
    let ids = fixture.events(&[25, 20, 9, 15, 10, 15]);
    let original = fixture.service().compare(left, Some(right)).unwrap();
    let captured =
        capture_checkpoint_compare_plan(|| fixture.service().compare(left, Some(right))).unwrap();
    let actual = captured.value.unwrap();
    assert!(
        actual == original,
        "full actual Some comparison differs; no raw diff is logged"
    );
    assert_eq!(
        (
            actual.added.len(),
            actual.removed.len(),
            actual.changed.len()
        ),
        (1, 1, 1)
    );
    assert!(
        actual.event_ids_in_window == [ids[4], ids[3], ids[5], ids[1]],
        "inclusive ordered window differs"
    );
    assert_eq!((actual.window_from, actual.window_to), (10, 20));
    let evidence = captured.evidence;
    assert_eq!(
        (
            evidence.invocations,
            evidence.metadata_invocations,
            evidence.items_invocations,
            evidence.window_invocations
        ),
        (5, 2, 2, 1)
    );
    assert_eq!(evidence.failure, None);
    assert_eq!(
        evidence
            .reads
            .iter()
            .map(|read| read.operation)
            .collect::<Vec<_>>(),
        [
            CheckpointCompareOperation::Metadata,
            CheckpointCompareOperation::Items,
            CheckpointCompareOperation::Metadata,
            CheckpointCompareOperation::Items,
            CheckpointCompareOperation::EventWindow
        ]
    );
    for (index, read) in evidence.reads.iter().enumerate() {
        assert!(read.prepared && read.query_started && read.fully_decoded);
        assert_eq!(read.bindings, Some(if index == 4 { 3 } else { 1 }));
        assert_eq!(read.statement_parameters, read.bindings);
        assert_eq!(read.decoded_rows, Some([1, 3, 1, 3, 4][index]));
        match &read.plan {
            CheckpointComparePlan::Checkpoint(CheckpointExplainOutcome::Complete(nodes)) => {
                assert!(nodes.iter().any(|node| match &node.kind {
                    CheckpointPlanNodeKind::Metadata(access)
                    | CheckpointPlanNodeKind::Items(access) =>
                        access.operation == PageAccessKind::Search && access.index.is_some(),
                    _ => false,
                }));
            }
            CheckpointComparePlan::EventWindow(PageExplainOutcome::Complete(nodes)) => {
                assert!(nodes
                    .iter()
                    .any(|node| matches!(&node.kind, PagePlanNodeKind::Events(access)
                    if access.operation==PageAccessKind::Search && access.index.is_some())));
                assert!(nodes.iter().all(
                    |node| !matches!(&node.kind, PagePlanNodeKind::Events(access)
                    if access.operation==PageAccessKind::Scan && access.index.is_none())
                ));
            }
            _ => panic!("actual read has incomplete plan evidence"),
        }
    }
    clean(&fixture);
}

#[test]
fn actual_missing_checkpoint_short_circuits_keep_original_error_and_prefix() {
    let fixture = Fixture::new();
    let (left, right) = pair(&fixture);
    for (left, right, expected_rows, what) in [
        (-1, right, vec![0], "checkpoint id"),
        (left, -1, vec![1, 3, 0], "comparison checkpoint id"),
    ] {
        let original = fixture.service().compare(left, Some(right)).unwrap_err();
        let captured =
            capture_checkpoint_compare_plan(|| fixture.service().compare(left, Some(right)))
                .unwrap();
        let error = captured.value.unwrap_err();
        assert!(
            error.to_string() == original.to_string(),
            "original missing checkpoint error changed"
        );
        assert!(matches!(error, AppError::InvalidInput { what: actual, .. } if actual == what));
        assert_eq!(captured.evidence.invocations, expected_rows.len());
        assert_eq!(captured.evidence.window_invocations, 0);
        assert_eq!(captured.evidence.failure, None);
        for (read, rows) in captured.evidence.reads.iter().zip(expected_rows) {
            assert!(read.fully_decoded);
            assert_eq!(read.decoded_rows, Some(rows));
        }
        clean(&fixture);
    }
}

#[test]
fn actual_domain_item_and_sql_failures_keep_partial_prefix_without_window() {
    let fixture = Fixture::new();
    let (left, right) = pair(&fixture);
    fixture
        .db()
        .write(move |conn| {
            conn.execute(
                "UPDATE checkpoints SET captured_collectors='{}' WHERE id=?1",
                [left],
            )
            .map_err(|source| DbError::Sqlite {
                operation: "mutate owned checkpoint fixture",
                source,
            })?;
            Ok(())
        })
        .unwrap();
    let original = fixture.service().compare(left, Some(right)).unwrap_err();
    let captured =
        capture_checkpoint_compare_plan(|| fixture.service().compare(left, Some(right))).unwrap();
    assert!(
        captured.value.unwrap_err().to_string() == original.to_string(),
        "original domain error changed"
    );
    assert_eq!(captured.evidence.invocations, 1);
    assert!(!captured.evidence.reads[0].fully_decoded);
    clean(&fixture);
    fixture
        .db()
        .write(move |conn| {
            conn.execute(
                "UPDATE checkpoints SET captured_collectors='[\"services\"]' WHERE id=?1",
                [left],
            )
            .map_err(|source| DbError::Sqlite {
                operation: "restore owned checkpoint fixture",
                source,
            })?;
            conn.execute(
                "UPDATE snapshots SET state_json='invalid' WHERE checkpoint_id=?1",
                [right],
            )
            .map_err(|source| DbError::Sqlite {
                operation: "mutate owned item fixture",
                source,
            })?;
            Ok(())
        })
        .unwrap();
    let original = fixture.service().compare(left, Some(right)).unwrap_err();
    let captured =
        capture_checkpoint_compare_plan(|| fixture.service().compare(left, Some(right))).unwrap();
    assert!(
        captured.value.unwrap_err().to_string() == original.to_string(),
        "original state JSON error changed"
    );
    assert_eq!(captured.evidence.invocations, 4);
    assert_eq!(captured.evidence.window_invocations, 0);
    assert!(captured.evidence.reads[..3]
        .iter()
        .all(|read| read.fully_decoded));
    assert!(!captured.evidence.reads[3].fully_decoded);
    assert_eq!(captured.evidence.reads[3].decoded_rows, None);
    clean(&fixture);

    // Isolated missing-table SQL boundary, not a migrated product-state claim.
    let fixture = Fixture::new();
    let (left, right) = pair(&fixture);
    fixture
        .db()
        .write(|conn| {
            conn.execute_batch("DROP TABLE snapshots")
                .map_err(|source| DbError::Sqlite {
                    operation: "mutate owned schema fixture",
                    source,
                })?;
            Ok(())
        })
        .unwrap();
    let original = fixture.service().compare(left, Some(right)).unwrap_err();
    let captured =
        capture_checkpoint_compare_plan(|| fixture.service().compare(left, Some(right))).unwrap();
    let error = captured.value.unwrap_err();
    assert!(
        error.to_string() == original.to_string(),
        "original SQL error changed"
    );
    assert!(matches!(
        error,
        AppError::Db(DbError::Sqlite {
            operation: "prepare checkpoint item query",
            ..
        })
    ));
    assert_eq!(captured.evidence.invocations, 2);
    assert!(captured.evidence.reads[0].fully_decoded);
    assert!(!captured.evidence.reads[1].prepared);
    assert_eq!(captured.evidence.window_invocations, 0);
    clean(&fixture);
}
