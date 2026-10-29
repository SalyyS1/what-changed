#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::test_fixture::{assert_clean, connection};
use super::*;
use crate::storage::event_repository::page;

#[test]
fn inactive_and_missing_scopes_do_not_claim_an_invocation() {
    let query = EventQuery::default();
    assert!(begin(&query).is_none());
    let captured = capture_page_plan(PagePlanVariant::for_query(&query), || 7).unwrap();
    assert_eq!(captured.value, 7);
    assert_eq!(captured.evidence.unwrap_err(), PageCaptureError::Missing);
    assert_clean(&connection());
}

#[test]
fn nested_scope_preserves_outer_capture_and_does_not_run_inner_operation() {
    let conn = connection();
    let query = EventQuery::default();
    let variant = PagePlanVariant::for_query(&query);
    let captured = capture_page_plan(variant, || {
        let mut ran = false;
        let nested = capture_page_plan(variant, || ran = true);
        assert!(matches!(nested, Err(PageCaptureError::Nested)));
        assert!(!ran);
        page(&conn, &query)
    })
    .unwrap();
    assert!(captured.value.is_ok());
    assert!(captured.evidence.unwrap().fully_decoded);
    assert_clean(&conn);
}

#[test]
fn multiple_actual_calls_and_variant_mismatch_preserve_operation_results() {
    let conn = connection();
    let query = EventQuery::default();
    let variant = PagePlanVariant::for_query(&query);
    let captured =
        capture_page_plan(variant, || (page(&conn, &query), page(&conn, &query))).unwrap();
    assert!(captured.value.0.is_ok() && captured.value.1.is_ok());
    assert_eq!(captured.evidence.unwrap_err(), PageCaptureError::Multiple);
    assert_clean(&conn);
    let expected = PagePlanVariant {
        category_arity: 2,
        ..variant
    };
    let captured = capture_page_plan(expected, || page(&conn, &query)).unwrap();
    assert!(captured.value.is_ok());
    assert_eq!(
        captured.evidence.unwrap_err(),
        PageCaptureError::VariantMismatch
    );
    assert_clean(&conn);
}

#[test]
fn primitive_explain_sqlite_failure_is_independent_of_successful_real_operation() {
    let conn = connection();
    let query = EventQuery::default();
    let captured = capture_page_plan(PagePlanVariant::for_query(&query), || {
        let token = begin(&query).unwrap();
        let values = [Value::Integer(1), Value::Integer(2), Value::Integer(3)];
        let mut original = conn.prepare("SELECT ?1+?2+?3").unwrap();
        // Primitive isolation only: deliberately invalid EXPLAIN input does
        // NOT pretend to force a failure of the real page owner's exact SQL.
        token.prepared(
            &conn,
            "invalid SQLite statement",
            &values,
            original.parameter_count(),
        );
        let result: i64 = original
            .query_row(rusqlite::params_from_iter(values.iter()), |row| row.get(0))
            .unwrap();
        token.started();
        token.completed(1);
        result
    })
    .unwrap();
    assert_eq!(captured.value, 6);
    let evidence = captured.evidence.unwrap();
    assert_eq!(
        evidence.explain,
        PageExplainOutcome::Failed(PageExplainError::Sqlite)
    );
    assert!(evidence.query_started && evidence.fully_decoded);
    assert_clean(&conn);
}

#[test]
fn primitive_binding_mismatch_does_not_override_real_operation() {
    let conn = connection();
    let query = EventQuery::default();
    let captured = capture_page_plan(PagePlanVariant::for_query(&query), || {
        let token = begin(&query).unwrap();
        token.prepared(&conn, "SELECT ?1", &[Value::Integer(1)], 2);
        conn.query_row("SELECT 19", [], |row| row.get::<_, i64>(0))
            .unwrap()
    })
    .unwrap();
    assert_eq!(captured.value, 19);
    assert_eq!(
        captured.evidence.unwrap_err(),
        PageCaptureError::BindingMismatch
    );
    assert_clean(&conn);
}

#[test]
fn capture_is_thread_local_and_clears_after_unwind() {
    let conn = connection();
    let query = EventQuery::default();
    let variant = PagePlanVariant::for_query(&query);
    let captured = capture_page_plan(variant, || {
        std::thread::spawn(|| assert_clean(&connection()))
            .join()
            .unwrap();
        page(&conn, &query)
    })
    .unwrap();
    assert!(captured.value.is_ok());
    assert_eq!(captured.evidence.unwrap().invocations, 1);
    let missing = capture_page_plan(variant, || {
        std::thread::spawn(|| page(&connection(), &EventQuery::default()).unwrap())
            .join()
            .unwrap()
    })
    .unwrap();
    assert_eq!(missing.evidence.unwrap_err(), PageCaptureError::Missing);
    assert_clean(&conn);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = capture_page_plan(variant, || {
            page(&conn, &query).unwrap();
            panic!("page capture unwind");
        });
    }))
    .is_err());
    assert_clean(&conn);
}
