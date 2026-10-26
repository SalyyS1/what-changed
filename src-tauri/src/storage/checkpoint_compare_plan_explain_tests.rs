#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn pinned_checkpoint_access_and_temporary_ordering_are_not_conflated() {
    let metadata = parse("SEARCH checkpoints USING INTEGER PRIMARY KEY (rowid=?)").unwrap();
    assert!(matches!(
        metadata,
        CheckpointPlanNodeKind::Metadata(CheckpointPlanAccess {
            operation: PageAccessKind::Search,
            index: Some("INTEGER PRIMARY KEY"),
            constraint: Some(CheckpointPlanConstraint::RowIdEqual),
            covering: false,
        })
    ));
    for name in [
        "idx_snapshots_checkpoint",
        "idx_snapshots_checkpoint_collector",
    ] {
        let items = parse(&format!(
            "SEARCH snapshots USING INDEX {name} (checkpoint_id=?)"
        ))
        .unwrap();
        assert!(matches!(
            items,
            CheckpointPlanNodeKind::Items(CheckpointPlanAccess {
                operation: PageAccessKind::Search,
                constraint: Some(CheckpointPlanConstraint::CheckpointIdEqual),
                ..
            })
        ));
        assert!(parse(&format!(
            "SEARCH snapshots USING COVERING INDEX {name} (checkpoint_id=?)"
        ))
        .is_some());
    }
    assert!(parse("SCAN snapshots").is_some());
    for (raw, expected) in [
        (
            "USE TEMP B-TREE FOR ORDER BY",
            CheckpointPlanNodeKind::OrderByTempBTree { last_terms: None },
        ),
        (
            "USE TEMP B-TREE FOR LAST TERM OF ORDER BY",
            CheckpointPlanNodeKind::OrderByTempBTree {
                last_terms: Some(1),
            },
        ),
        (
            "USE TEMP B-TREE FOR LAST 2 TERMS OF ORDER BY",
            CheckpointPlanNodeKind::OrderByTempBTree {
                last_terms: Some(2),
            },
        ),
        (
            "USE TEMP B-TREE FOR RIGHT PART OF ORDER BY",
            CheckpointPlanNodeKind::OrderByRightPartTempBTree,
        ),
    ] {
        assert_eq!(parse(raw), Some(expected));
    }
}

#[test]
fn private_schema_names_and_unrecognized_vocabulary_are_rejected_whole() {
    for raw in [
        "SEARCH snapshots USING INDEX private_index (checkpoint_id=?)",
        "SEARCH checkpoints USING INDEX private_index (id=?)",
        "SEARCH snapshots_private USING INDEX idx_snapshots_checkpoint (checkpoint_id=?)",
        "SEARCH snapshots USING INDEX idx_snapshots_checkpoint (private_column=?)",
        "USE TEMP B-TREE FOR LAST 4 TERMS OF ORDER BY",
        "BLOOM FILTER ON snapshots (checkpoint_id=?)",
    ] {
        assert!(parse(raw).is_none());
    }
}

#[test]
fn actual_projection_keeps_node_ids_and_eqp_errors_do_not_retain_sql() {
    let conn = crate::storage::page_plan_capture::test_fixture::connection();
    let sql = "SELECT collector_id FROM snapshots WHERE checkpoint_id=?1 ORDER BY collector_id,entity_kind,entity_external_id";
    let parameters = [Value::Integer(0)];
    let original = conn
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .unwrap()
        .query_map([0], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let outcome = explain(&conn, sql, &parameters);
    assert!(
        matches!(&outcome, CheckpointExplainOutcome::Complete(_)),
        "migrated fixed query must have supported projection"
    );
    if let CheckpointExplainOutcome::Complete(nodes) = outcome {
        assert_eq!(
            nodes
                .iter()
                .map(|node| (node.id, node.parent))
                .collect::<Vec<_>>(),
            original
        );
    }
    assert_eq!(
        explain(&conn, "private invalid SQL", &parameters),
        CheckpointExplainOutcome::Failed(PageExplainError::Sqlite)
    );
}
