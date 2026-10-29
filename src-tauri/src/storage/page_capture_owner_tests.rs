#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::*;
use crate::storage::page_plan_capture::test_fixture::{assert_clean, connection, insert};
use crate::storage::page_plan_capture::{
    capture_page_plan, PageExplainError, PageExplainOutcome, PagePlanVariant,
};
use crate::testing::synthetic_event;

#[test]
fn concrete_page_bindings_preserve_exact_sql_order_and_variable_arities() {
    let query = EventQuery {
        from: 11,
        to: 19,
        cursor: Some(Cursor {
            timestamp: 13,
            id: 7,
        }),
        categories: vec![Category::Service, Category::Docker, Category::Service],
        importance_min: Some(Importance::Informational),
        entity_id: Some("private-bind-canary".into()),
        limit: 2,
        ..EventQuery::default()
    };
    let (sql, values, limit) = page_statement(&query);
    assert_eq!(sql, format!("SELECT {SUMMARY_COLUMNS} FROM events WHERE timestamp >= ?1 AND timestamp <= ?2 AND (timestamp < ?3 OR (timestamp = ?3 AND id < ?4)) AND category IN (?5, ?6, ?7) AND importance IN (?8, ?9, ?10, ?11) AND entity_id = ?12 ORDER BY timestamp DESC, id DESC LIMIT ?13"));
    assert_eq!(
        values,
        vec![
            SqlValue::Integer(11),
            SqlValue::Integer(19),
            SqlValue::Integer(13),
            SqlValue::Integer(7),
            SqlValue::Text("service".into()),
            SqlValue::Text("docker".into()),
            SqlValue::Text("service".into()),
            SqlValue::Text("informational".into()),
            SqlValue::Text("notable".into()),
            SqlValue::Text("important".into()),
            SqlValue::Text("critical".into()),
            SqlValue::Text("private-bind-canary".into()),
            SqlValue::Integer(3)
        ]
    );
    assert_eq!(limit, 2);
    let conn = connection();
    for query in [
        query,
        EventQuery {
            categories: vec![Category::Docker, Category::Service],
            importance_min: Some(Importance::Critical),
            ..EventQuery::default()
        },
    ] {
        let variant = PagePlanVariant::for_query(&query);
        let captured = capture_page_plan(variant, || page(&conn, &query)).unwrap();
        assert!(captured.value.is_ok());
        let evidence = captured.evidence.unwrap();
        assert_eq!(evidence.variant, variant);
        assert_eq!(evidence.bindings, evidence.statement_parameters);
        assert!(evidence.fully_decoded);
        assert!(matches!(evidence.explain, PageExplainOutcome::Complete(_)));
        assert!(!format!("{evidence:?}").contains("private-bind-canary"));
    }
}

#[test]
fn tied_keyset_pages_decode_lookahead_and_complete_empty_pages() {
    let conn = connection();
    let ids = insert(
        &conn,
        (0..5).map(|index| synthetic_event(index, 10_000)).collect(),
    );
    let mut query = EventQuery {
        limit: 2,
        ..EventQuery::default()
    };
    let mut actual_ids = Vec::new();
    for expected_decodes in [3, 3, 1] {
        let captured =
            capture_page_plan(PagePlanVariant::for_query(&query), || page(&conn, &query)).unwrap();
        let result = captured.value.unwrap();
        let evidence = captured.evidence.unwrap();
        assert!(evidence.fully_decoded && evidence.query_started);
        assert_eq!(evidence.decoded_rows, Some(expected_decodes));
        if let Some(cursor) = result.next {
            assert_eq!(cursor.id, result.items.last().unwrap().id);
        }
        actual_ids.extend(result.items.iter().map(|event| event.id));
        query.cursor = result.next;
    }
    assert!(query.cursor.is_none());
    assert_eq!(actual_ids, ids.into_iter().rev().collect::<Vec<_>>());
    query.from = 20_000;
    query.to = 30_000;
    let captured =
        capture_page_plan(PagePlanVariant::for_query(&query), || page(&conn, &query)).unwrap();
    assert!(captured.value.unwrap().items.is_empty());
    let evidence = captured.evidence.unwrap();
    assert!(evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, Some(0));
}

#[test]
fn prepare_and_lookahead_decode_errors_remain_incomplete_and_clear_scope() {
    let query = EventQuery {
        limit: 1,
        ..EventQuery::default()
    };
    let missing = Connection::open_in_memory().unwrap();
    let original_error = page(&missing, &query).unwrap_err().to_string();
    let captured = capture_page_plan(PagePlanVariant::for_query(&query), || {
        page(&missing, &query)
    })
    .unwrap();
    assert_eq!(captured.value.unwrap_err().to_string(), original_error);
    let evidence = captured.evidence.unwrap();
    assert!(!evidence.prepared && !evidence.query_started && !evidence.fully_decoded);
    assert_eq!(evidence.explain, PageExplainOutcome::NotAttempted);
    let conn = connection();
    assert_clean(&conn);
    let ids = insert(
        &conn,
        vec![synthetic_event(1, 10_000), synthetic_event(2, 20_000)],
    );
    conn.execute(
        "UPDATE events SET after_json='private-invalid-json' WHERE id=?1",
        [ids[0]],
    )
    .unwrap();
    let original_error = page(&conn, &query).unwrap_err().to_string();
    let captured =
        capture_page_plan(PagePlanVariant::for_query(&query), || page(&conn, &query)).unwrap();
    assert_eq!(captured.value.unwrap_err().to_string(), original_error);
    let evidence = captured.evidence.unwrap();
    assert!(evidence.prepared && evidence.query_started);
    assert!(matches!(evidence.explain, PageExplainOutcome::Complete(_)));
    assert!(!evidence.fully_decoded);
    assert_eq!(
        evidence.decoded_rows, None,
        "lookahead decode failure is not a complete one-row page"
    );
    conn.execute("UPDATE events SET after_json=NULL WHERE id=?1", [ids[0]])
        .unwrap();
    assert_clean(&conn);
}

#[test]
fn unknown_actual_schema_index_fails_only_plan_evidence_not_page_execution() {
    let conn = connection();
    insert(&conn, vec![synthetic_event(1, 10_000)]);
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
    conn.execute_batch("CREATE INDEX private_schema_canary ON events(timestamp)")
        .unwrap();
    let query = EventQuery {
        from: 0,
        to: 30_000,
        ..EventQuery::default()
    };
    let original = page(&conn, &query).unwrap();
    let captured =
        capture_page_plan(PagePlanVariant::for_query(&query), || page(&conn, &query)).unwrap();
    assert_eq!(captured.value.unwrap(), original);
    let evidence = captured.evidence.unwrap();
    assert_eq!(
        evidence.explain,
        PageExplainOutcome::Failed(PageExplainError::UnsupportedVocabulary)
    );
    assert!(evidence.query_started && evidence.fully_decoded);
    assert!(!format!("{evidence:?}").contains("private_schema_canary"));
    conn.execute_batch(
        "DROP INDEX private_schema_canary; CREATE INDEX idx_events_timestamp ON events(timestamp)",
    )
    .unwrap();
    assert_clean(&conn);
}
