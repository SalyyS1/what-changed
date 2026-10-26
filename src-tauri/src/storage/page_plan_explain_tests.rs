#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::*;

#[test]
fn pinned_access_sort_and_or_vocabulary_preserves_structural_fields() {
    let PagePlanNodeKind::Events(access) = parse("SEARCH events USING COVERING INDEX idx_events_recurrence (entity_id=? AND category=? AND ANY(event_type) AND timestamp>? AND timestamp<?)").unwrap() else { panic!("expected actual access"); };
    assert_eq!(access.operation, PageAccessKind::Search);
    assert_eq!(access.index, Some("idx_events_recurrence"));
    assert!(access.covering);
    assert_eq!(access.constraints.len(), 5);
    assert_eq!(access.constraints[2].kind, PageConstraintKind::Any);
    assert_eq!(
        parse("MULTI-INDEX OR"),
        Some(PagePlanNodeKind::MultiIndexOr)
    );
    assert_eq!(
        parse("INDEX 2"),
        Some(PagePlanNodeKind::IndexBranch { ordinal: 2 })
    );
    assert_eq!(
        parse("USE TEMP B-TREE FOR ORDER BY"),
        Some(PagePlanNodeKind::OrderByTempBTree { last_terms: None })
    );
    assert_eq!(
        parse("USE TEMP B-TREE FOR LAST TERM OF ORDER BY"),
        Some(PagePlanNodeKind::OrderByTempBTree {
            last_terms: Some(1)
        })
    );
    assert_eq!(
        parse("USE TEMP B-TREE FOR LAST 2 TERMS OF ORDER BY"),
        Some(PagePlanNodeKind::OrderByTempBTree {
            last_terms: Some(2)
        })
    );
    assert!(parse("SEARCH events USING INTEGER PRIMARY KEY (rowid<?)").is_some());
    assert!(parse("SCAN events").is_some());
    assert!(parse("SCAN events USING INDEX idx_events_timestamp").is_some());
}

#[test]
fn unknown_names_literals_and_vocabulary_are_not_retained_or_classified() {
    for raw in [
        "SEARCH events USING INDEX do-not-retain-secret (timestamp>?)",
        "SEARCH other_table USING INDEX idx_events_timestamp (timestamp>?)",
        "SEARCH events USING INDEX idx_events_timestamp (timestamp>private-bound)",
        "SEARCH events USING INDEX idx_events_timestamp (private_column=?)",
        "USE TEMP B-TREE FOR DISTINCT",
        "INDEX private",
        "BLOOM FILTER ON events (entity_id=?)",
    ] {
        assert!(parse(raw).is_none());
    }
}

#[test]
fn actual_sqlite_projection_preserves_all_node_ids_parents_and_order() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE events(id INTEGER PRIMARY KEY,timestamp INTEGER); CREATE INDEX idx_events_timestamp ON events(timestamp);").unwrap();
    let sql = "SELECT id FROM events WHERE timestamp>?1 OR id<?2 ORDER BY timestamp DESC,id DESC";
    let params = [Value::Integer(5), Value::Integer(2)];
    let actual = explain_rows(&conn, sql, &params).unwrap();
    let mut statement = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
    let identities = statement
        .query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(actual.len(), identities.len());
    for (node, (id, parent)) in actual.iter().zip(identities) {
        assert_eq!((node.id, node.parent), (id, parent));
    }
}
