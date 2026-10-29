#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::*;
use crate::storage::checkpoint_repository::event_ids_in_window;
use crate::storage::page_plan_capture::test_fixture::connection;

pub(super) fn assert_clean(conn: &Connection) {
    let captured = capture_checkpoint_window_plan(|| event_ids_in_window(conn, 0, 100)).unwrap();
    let value = captured.value.unwrap();
    let evidence = captured.evidence.unwrap();
    assert!(evidence.prepared && evidence.query_started && evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, Some(value.len()));
    assert!(matches!(evidence.explain, PageExplainOutcome::Complete(_)));
}

#[test]
fn inactive_and_missing_scopes_do_not_claim_an_invocation() {
    assert!(begin().is_none());
    let captured = capture_checkpoint_window_plan(|| 7).unwrap();
    assert_eq!(captured.value, 7);
    assert_eq!(
        captured.evidence.unwrap_err(),
        CheckpointWindowCaptureError::Missing
    );
    assert_clean(&connection());
}

#[test]
fn nested_scope_preserves_outer_and_rejects_before_inner_closure() {
    let conn = connection();
    let mut inner_ran = false;
    let captured = capture_checkpoint_window_plan(|| {
        assert!(matches!(
            capture_checkpoint_window_plan(|| inner_ran = true),
            Err(CheckpointWindowCaptureError::Nested)
        ));
        event_ids_in_window(&conn, 0, 100)
    })
    .unwrap();
    assert!(!inner_ran);
    assert!(captured.value.is_ok());
    assert!(captured.evidence.unwrap().fully_decoded);
    assert_clean(&conn);
}

#[test]
fn multiple_actual_calls_preserve_both_original_results() {
    let conn = connection();
    let captured = capture_checkpoint_window_plan(|| {
        (
            event_ids_in_window(&conn, 0, 100),
            event_ids_in_window(&conn, 100, 0),
        )
    })
    .unwrap();
    assert_eq!(captured.value.0.unwrap(), captured.value.1.unwrap());
    assert_eq!(
        captured.evidence.unwrap_err(),
        CheckpointWindowCaptureError::Multiple
    );
    assert_clean(&conn);
}

#[test]
fn primitive_binding_mismatch_does_not_replace_operation_result() {
    let conn = connection();
    let captured = capture_checkpoint_window_plan(|| {
        let token = begin().unwrap();
        token.prepared(&conn, "SELECT ?1", (1, 2, 3), 1);
        conn.query_row("SELECT 19", [], |row| row.get::<_, i64>(0))
            .unwrap()
    })
    .unwrap();
    assert_eq!(captured.value, 19);
    assert_eq!(
        captured.evidence.unwrap_err(),
        CheckpointWindowCaptureError::BindingMismatch
    );
    assert_clean(&conn);
}

#[test]
fn primitive_explain_failure_and_execution_transitions_are_independent() {
    let conn = connection();
    // Primitive isolation ONLY: invalid EXPLAIN does not pretend to induce
    // an EQP SQLite error for the owner's real fixed event-window statement.
    let captured = capture_checkpoint_window_plan(|| {
        let token = begin().unwrap();
        CAPTURE.with(|slot| {
            let slot = slot.borrow();
            let evidence = slot.as_ref().unwrap().evidence.as_ref().unwrap();
            assert!(!evidence.prepared && !evidence.query_started && !evidence.fully_decoded);
        });
        let mut statement = conn.prepare("SELECT ?1+?2+?3").unwrap();
        let parameters = (1, 2, 3);
        token.prepared(
            &conn,
            "invalid SQLite statement",
            parameters,
            statement.parameter_count(),
        );
        CAPTURE.with(|slot| {
            let slot = slot.borrow();
            let evidence = slot.as_ref().unwrap().evidence.as_ref().unwrap();
            assert!(evidence.prepared && !evidence.query_started && !evidence.fully_decoded);
        });
        let rows = statement
            .query_map(parameters, |row| row.get::<_, i64>(0))
            .unwrap();
        token.started();
        CAPTURE.with(|slot| {
            let slot = slot.borrow();
            let evidence = slot.as_ref().unwrap().evidence.as_ref().unwrap();
            assert!(evidence.query_started && !evidence.fully_decoded);
            assert_eq!(evidence.decoded_rows, None);
        });
        let values = rows.collect::<rusqlite::Result<Vec<_>>>().unwrap();
        token.completed(values.len());
        values
    })
    .unwrap();
    assert_eq!(captured.value, [6]);
    let evidence = captured.evidence.unwrap();
    assert_eq!(
        evidence.explain,
        PageExplainOutcome::Failed(crate::storage::page_plan_capture::PageExplainError::Sqlite)
    );
    assert!(evidence.query_started && evidence.fully_decoded);
    assert_eq!(evidence.decoded_rows, Some(1));
    assert_clean(&conn);
}

#[test]
fn scopes_are_thread_local_and_clear_after_unwind() {
    let conn = connection();
    let captured = capture_checkpoint_window_plan(|| {
        std::thread::spawn(|| assert_clean(&connection()))
            .join()
            .unwrap();
        event_ids_in_window(&conn, 0, 100)
    })
    .unwrap();
    assert!(captured.value.is_ok());
    assert_eq!(captured.evidence.unwrap().invocations, 1);
    let missing = capture_checkpoint_window_plan(|| {
        std::thread::spawn(|| event_ids_in_window(&connection(), 0, 100).unwrap())
            .join()
            .unwrap()
    })
    .unwrap();
    assert_eq!(
        missing.evidence.unwrap_err(),
        CheckpointWindowCaptureError::Missing
    );
    assert_clean(&conn);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = capture_checkpoint_window_plan(|| {
            event_ids_in_window(&conn, 0, 100).unwrap();
            panic!("owned checkpoint window capture unwind");
        });
    }))
    .is_err());
    assert_clean(&conn);
}
