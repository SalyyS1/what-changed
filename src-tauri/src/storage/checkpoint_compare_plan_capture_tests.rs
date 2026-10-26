#![allow(clippy::panic, clippy::unwrap_used)]

use super::*;
use crate::storage::checkpoint_repository::{by_id, event_ids_in_window};
use crate::storage::page_plan_capture::test_fixture::connection;
use rusqlite::Connection;

pub(super) fn assert_clean(conn: &Connection) {
    let captured = capture_checkpoint_compare_plan(|| by_id(conn, -1)).unwrap();
    assert!(captured.value.unwrap().is_none());
    assert_eq!(captured.evidence.invocations, 1);
    assert_eq!(captured.evidence.failure, None);
    assert!(captured.evidence.reads[0].fully_decoded);
    assert_eq!(captured.evidence.reads[0].decoded_rows, Some(0));
}

#[test]
fn inactive_and_missing_envelopes_do_not_claim_reads() {
    assert!(begin(CheckpointCompareOperation::Metadata).is_none());
    let captured = capture_checkpoint_compare_plan(|| 7).unwrap();
    assert_eq!(captured.value, 7);
    assert_eq!(captured.evidence.invocations, 0);
    assert!(captured.evidence.reads.is_empty());
    assert_eq!(
        captured.evidence.failure,
        Some(CheckpointCompareCaptureError::Missing)
    );
    assert_clean(&connection());
}

#[test]
fn both_scope_nesting_directions_reject_before_closure_and_preserve_outer() {
    let conn = connection();
    let mut ran = false;
    let captured = capture_checkpoint_compare_plan(|| {
        assert!(matches!(
            capture_checkpoint_compare_plan(|| ran = true),
            Err(CheckpointCompareCaptureError::Nested)
        ));
        assert!(matches!(
            capture_checkpoint_window_plan(|| ran = true),
            Err(CheckpointWindowCaptureError::Nested)
        ));
        by_id(&conn, -1)
    })
    .unwrap();
    assert!(!ran);
    assert!(captured.value.unwrap().is_none());
    assert!(captured.evidence.reads[0].fully_decoded);
    assert_clean(&conn);
    let window = capture_checkpoint_window_plan(|| {
        assert!(matches!(
            capture_checkpoint_compare_plan(|| ran = true),
            Err(CheckpointCompareCaptureError::Nested)
        ));
        event_ids_in_window(&conn, 0, 100)
    })
    .unwrap();
    assert!(!ran);
    assert!(window.value.unwrap().is_empty());
    assert!(window.evidence.unwrap().fully_decoded);
    assert_clean(&conn);
}

#[test]
fn multiple_reads_are_counted_but_retained_evidence_is_bounded() {
    let conn = connection();
    let captured = capture_checkpoint_compare_plan(|| {
        (0..7)
            .map(|_| by_id(&conn, -1).unwrap())
            .collect::<Vec<_>>()
    })
    .unwrap();
    assert!(captured.value.iter().all(Option::is_none));
    assert_eq!(captured.evidence.invocations, 7);
    assert_eq!(captured.evidence.metadata_invocations, 7);
    assert_eq!(captured.evidence.reads.len(), 5);
    assert_eq!(
        captured.evidence.failure,
        Some(CheckpointCompareCaptureError::Multiple)
    );
    assert_clean(&conn);
    let captured = capture_checkpoint_compare_plan(|| {
        (
            event_ids_in_window(&conn, 0, 100),
            event_ids_in_window(&conn, 100, 0),
        )
    })
    .unwrap();
    assert_eq!(captured.value.0.unwrap(), captured.value.1.unwrap());
    assert_eq!(captured.evidence.window_invocations, 2);
    assert_eq!(captured.evidence.reads.len(), 2);
    assert_eq!(
        captured.evidence.failure,
        Some(CheckpointCompareCaptureError::Window(
            CheckpointWindowCaptureError::Multiple
        ))
    );
    assert_clean(&conn);
}

#[test]
fn primitive_binding_and_explain_failures_preserve_actual_operation_value() {
    let conn = connection();
    let captured = capture_checkpoint_compare_plan(|| {
        let token = begin(CheckpointCompareOperation::Metadata).unwrap();
        token.prepared(&conn, "SELECT ?1+?2", &[1], 2);
        conn.query_row("SELECT 19", [], |row| row.get::<_, i64>(0))
            .unwrap()
    })
    .unwrap();
    assert_eq!(captured.value, 19);
    assert_eq!(
        captured.evidence.failure,
        Some(CheckpointCompareCaptureError::BindingMismatch)
    );
    assert!(!captured.evidence.reads[0].fully_decoded);
    assert_clean(&conn);
    // Primitive isolation, not a claim of forcing an actual owner EQP failure.
    let captured = capture_checkpoint_compare_plan(|| {
        let token = begin(CheckpointCompareOperation::Metadata).unwrap();
        token.prepared(&conn, "private invalid SQLite", &[1], 1);
        token.started();
        let value = conn
            .query_row("SELECT 23", [], |row| row.get::<_, i64>(0))
            .unwrap();
        token.completed(1);
        value
    })
    .unwrap();
    assert_eq!(captured.value, 23);
    let read = &captured.evidence.reads[0];
    assert!(read.prepared && read.query_started && read.fully_decoded);
    assert_eq!(
        read.plan,
        CheckpointComparePlan::Checkpoint(CheckpointExplainOutcome::Failed(
            crate::storage::page_plan_capture::PageExplainError::Sqlite
        ))
    );
    assert!(!format!("{:?}", captured.evidence).contains("private invalid SQLite"));
    assert_clean(&conn);
}

#[test]
fn scopes_are_thread_bound_and_both_guards_clear_after_unwind() {
    let conn = connection();
    let captured = capture_checkpoint_compare_plan(|| {
        std::thread::spawn(|| assert_clean(&connection()))
            .join()
            .unwrap();
        by_id(&conn, -1)
    })
    .unwrap();
    assert!(captured.value.unwrap().is_none());
    assert_eq!(captured.evidence.invocations, 1);
    let missing = capture_checkpoint_compare_plan(|| {
        std::thread::spawn(|| by_id(&connection(), -1).unwrap())
            .join()
            .unwrap()
    })
    .unwrap();
    assert_eq!(
        missing.evidence.failure,
        Some(CheckpointCompareCaptureError::Missing)
    );
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = capture_checkpoint_compare_plan(|| {
            by_id(&conn, -1).unwrap();
            event_ids_in_window(&conn, 0, 100).unwrap();
            panic!("owned compare scope unwind");
        });
    }))
    .is_err());
    assert_clean(&conn);
    let window = capture_checkpoint_window_plan(|| event_ids_in_window(&conn, 0, 100)).unwrap();
    assert!(window.value.is_ok() && window.evidence.unwrap().fully_decoded);
}
