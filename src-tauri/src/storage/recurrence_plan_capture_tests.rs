#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::*;
use crate::domain::{Category, EventSignature, EventType};
use crate::storage::event_repository::prior_occurrences_batch;
use crate::storage::event_repository::{
    prior_occurrences_sorted_full_counts, SORTED_RECURRENCE_SQL,
};

fn query() -> PriorOccurrenceQuery {
    PriorOccurrenceQuery {
        event_id: 7,
        signature: EventSignature {
            entity_id: "fixture-do-not-log-entity".to_owned(),
            category: Category::Service,
            event_type: EventType::Stopped,
        },
        since: 0,
        before: 4,
    }
}

fn connection() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE events(entity_id TEXT, category TEXT, event_type TEXT, timestamp INTEGER); \
         CREATE INDEX idx_recurrence_fixture ON events(entity_id, category, event_type, timestamp);",
    )
    .unwrap();
    for timestamp in [-5, 0, 2, 4] {
        let query = query();
        conn.execute(
            "INSERT INTO events VALUES (?1, ?2, ?3, ?4)",
            (
                query.signature.entity_id,
                query.signature.category.as_str(),
                query.signature.event_type.as_str(),
                timestamp,
            ),
        )
        .unwrap();
    }
    conn
}

fn assert_scope_cleared() {
    let captured = capture_sorted_recurrence_plan(|| 7).unwrap();
    assert_eq!(captured.value, 7);
    assert_eq!(
        captured.plan.unwrap_err(),
        SortedRecurrenceCaptureError::Missing
    );
}

fn padded_pair(second: PriorOccurrenceQuery) -> Vec<PriorOccurrenceQuery> {
    let mut queries = vec![query(), second];
    for index in 2..2_000 {
        let mut padding = query();
        padding.event_id = index + 10_000;
        padding.signature.entity_id = format!("z-capture-padding-{index:05}");
        queries.push(padding);
    }
    queries
}

fn assert_plan_coverage(
    plan: &SortedRecurrencePlan,
    full: usize,
    delta: usize,
    rolled: usize,
    reused: usize,
) {
    assert_eq!(plan.completed_input_queries, plan.input_queries);
    assert_eq!(plan.window_statistics.input_queries, plan.input_queries);
    assert_eq!(plan.completed_full_executions, full);
    assert_eq!(plan.completed_delta_executions, delta);
    assert_eq!(plan.completed_rolling_queries, rolled);
    assert_eq!(plan.identical_reuses, reused);
    assert_eq!(plan.completed_executions, full + delta);
    assert_eq!(plan.completed_input_queries, full + rolled + reused);
    assert!(rolled <= delta && delta <= rolled * 2);
    assert_eq!(plan.full_plan_kind, SortedRecurrenceCountKind::Full);
    assert_eq!(plan.delta_plan.is_some(), delta > 0);
    for details in
        std::iter::once(&plan.details).chain(plan.delta_plan.iter().map(|delta| &delta.details))
    {
        assert!(details.iter().any(|detail| {
            let normalized = detail.to_ascii_uppercase();
            normalized.contains("SEARCH EVENTS") && normalized.contains("INDEX")
        }));
        assert!(details.iter().all(|detail| {
            let normalized = detail.to_ascii_uppercase();
            !normalized.contains("SCAN EVENTS")
                && !normalized.contains("SCAN TABLE EVENTS")
                && !detail.contains("fixture-do-not-log-entity")
                && !detail.contains("z-capture-padding")
        }));
    }
    if let Some(delta) = &plan.delta_plan {
        assert_eq!(delta.kind, SortedRecurrenceCountKind::Delta);
    }
}

#[test]
fn inactive_capture_does_not_explain() {
    // Invalid indices prove the inactive path never walks or indexes inputs.
    assert!(begin(&connection(), "invalid SQL", &[], &[usize::MAX]).is_none());
    assert_scope_cleared();
}

#[test]
fn actual_sorted_invocation_reports_index_and_completed_counts_without_bind_values() {
    let conn = connection();
    let queries = vec![query(); 2_000];
    let captured =
        capture_sorted_recurrence_plan(|| prior_occurrences_batch(&conn, &queries)).unwrap();
    // Duplicate IDs sum within each 150-query chunk; the last 50 overwrite.
    assert_eq!(captured.value.unwrap().get(&7), Some(&100));
    let plan = captured.plan.unwrap();
    assert_eq!(plan.input_queries, 2_000);
    assert_eq!(plan.completed_executions, 2_000);
    assert_plan_coverage(&plan, 2_000, 0, 0, 0);
    assert_eq!(
        plan.window_statistics,
        SortedRecurrenceWindowStatistics {
            input_queries: 2_000,
            distinct_signatures: 1,
            distinct_windows: 1,
            identical_window_repeats: 1_999,
            empty_or_reversed_windows: 0,
            max_queries_per_signature: 2_000,
            adjacent_overlapping_windows: 1_999,
            monotone_before_overlapping_windows: 1_999,
        }
    );
    assert!(plan.details.iter().any(|detail| {
        let normalized = detail.to_ascii_uppercase();
        normalized.contains("SEARCH EVENTS") && normalized.contains("INDEX")
    }));
    assert!(plan.details.iter().all(|detail| {
        !detail.contains("fixture-do-not-log-entity")
            && !detail.to_ascii_uppercase().contains("SCAN EVENTS")
    }));
    assert_scope_cleared();
}

#[test]
fn actual_five_thousand_counts_match_captured_input_statistics() {
    let conn = connection();
    let queries = vec![query(); 5_000];
    let captured =
        capture_sorted_recurrence_plan(|| prior_occurrences_batch(&conn, &queries)).unwrap();
    assert_eq!(captured.value.unwrap().get(&7), Some(&100));
    let plan = captured.plan.unwrap();
    assert_eq!(plan.input_queries, 5_000);
    assert_eq!(plan.completed_executions, 5_000);
    assert_plan_coverage(&plan, 5_000, 0, 0, 0);
    assert_eq!(plan.window_statistics.input_queries, 5_000);
    assert_eq!(plan.window_statistics.distinct_signatures, 1);
    assert_eq!(plan.window_statistics.distinct_windows, 1);
    assert_eq!(plan.window_statistics.identical_window_repeats, 4_999);
    assert_eq!(plan.window_statistics.empty_or_reversed_windows, 0);
    assert_eq!(plan.window_statistics.max_queries_per_signature, 5_000);
    assert_eq!(plan.window_statistics.adjacent_overlapping_windows, 4_999);
    assert_eq!(
        plan.window_statistics.monotone_before_overlapping_windows,
        4_999
    );
    assert_scope_cleared();
}

#[test]
fn nested_scope_rejection_preserves_outer_capture() {
    let conn = connection();
    let captured = capture_sorted_recurrence_plan(|| {
        let nested = capture_sorted_recurrence_plan(|| panic!("nested operation must not run"));
        assert!(matches!(nested, Err(SortedRecurrenceCaptureError::Nested)));
        prior_occurrences_batch(&conn, &vec![query(); 2_000])
    })
    .unwrap();
    assert_eq!(captured.value.unwrap().get(&7), Some(&100));
    assert_eq!(captured.plan.unwrap().completed_executions, 2_000);
    assert_scope_cleared();
}

#[test]
fn explain_error_is_static_and_does_not_replace_operation_result() {
    let conn = connection();
    let captured = capture_sorted_recurrence_plan(|| {
        let capture = begin(&conn, "invalid private SQL", &[query()], &[0]);
        assert!(capture.is_some());
        7
    })
    .unwrap();
    assert_eq!(captured.value, 7);
    let error = captured.plan.unwrap_err();
    assert_eq!(error, SortedRecurrenceCaptureError::Explain);
    assert_eq!(format!("{error:?}"), "Explain");
    assert_scope_cleared();
}

#[test]
fn multiple_invocations_are_rejected_without_changing_repository_results() {
    let conn = connection();
    let queries = vec![query(); 2_000];
    let captured = capture_sorted_recurrence_plan(|| {
        let first = prior_occurrences_batch(&conn, &queries).unwrap();
        let second = prior_occurrences_batch(&conn, &queries).unwrap();
        assert_eq!(first, second);
        second
    })
    .unwrap();
    assert_eq!(captured.value.get(&7), Some(&100));
    assert_eq!(
        captured.plan.unwrap_err(),
        SortedRecurrenceCaptureError::Multiple
    );
    assert_scope_cleared();
}

#[test]
fn returned_error_and_unwinding_clear_thread_local_scope() {
    let captured = capture_sorted_recurrence_plan(|| Err::<(), _>("operation failed")).unwrap();
    assert_eq!(captured.value, Err("operation failed"));
    assert_scope_cleared();
    assert!(std::panic::catch_unwind(|| {
        let _ = capture_sorted_recurrence_plan(|| panic!("owned diagnostic fixture panic"));
    })
    .is_err());
    assert_scope_cleared();
}

#[test]
fn transaction_capture_classifies_actual_full_and_two_delta_probes() {
    let conn = connection();
    let tx = conn.unchecked_transaction().unwrap();
    let mut second = query();
    second.event_id = 8;
    second.since = 1;
    second.before = 5;
    let queries = padded_pair(second);
    let old = prior_occurrences_sorted_full_counts(&tx, &queries).unwrap();
    let captured =
        capture_sorted_recurrence_plan(|| prior_occurrences_batch(&tx, &queries)).unwrap();
    assert_eq!(captured.value.unwrap(), old);
    assert_plan_coverage(&captured.plan.unwrap(), 1_999, 2, 1, 0);
    assert_scope_cleared();
}

#[test]
fn transaction_capture_reuses_five_thousand_identical_valid_windows() {
    let conn = connection();
    let tx = conn.unchecked_transaction().unwrap();
    let queries = vec![query(); 5_000];
    let old = prior_occurrences_sorted_full_counts(&tx, &queries).unwrap();
    let captured =
        capture_sorted_recurrence_plan(|| prior_occurrences_batch(&tx, &queries)).unwrap();
    assert_eq!(captured.value.unwrap(), old);
    assert_eq!(old.get(&7), Some(&100));
    let plan = captured.plan.unwrap();
    assert_plan_coverage(&plan, 1, 0, 0, 4_999);
    assert_eq!(plan.window_statistics.identical_window_repeats, 4_999);
    assert_scope_cleared();
}

#[test]
fn transaction_capture_omits_zero_width_deltas_and_rejects_wide_shifts() {
    let conn = connection();
    let tx = conn.unchecked_transaction().unwrap();
    for (since, before, delta) in [(0, 5, 1), (1, 4, 1), (3, 7, 0)] {
        let mut second = query();
        second.event_id = 8;
        second.since = since;
        second.before = before;
        let queries = padded_pair(second);
        let old = prior_occurrences_sorted_full_counts(&tx, &queries).unwrap();
        let captured =
            capture_sorted_recurrence_plan(|| prior_occurrences_batch(&tx, &queries)).unwrap();
        assert_eq!(captured.value.unwrap(), old);
        assert_plan_coverage(
            &captured.plan.unwrap(),
            if delta == 0 { 2_000 } else { 1_999 },
            delta,
            usize::from(delta > 0),
            0,
        );
    }
    assert_scope_cleared();
}

#[test]
fn transaction_nested_and_multiple_scopes_preserve_repository_results() {
    let conn = connection();
    let tx = conn.unchecked_transaction().unwrap();
    let queries = vec![query(); 2_000];
    let old = prior_occurrences_sorted_full_counts(&tx, &queries).unwrap();
    let captured = capture_sorted_recurrence_plan(|| {
        assert!(matches!(
            capture_sorted_recurrence_plan(|| panic!("nested must not run")),
            Err(SortedRecurrenceCaptureError::Nested)
        ));
        prior_occurrences_batch(&tx, &queries)
    })
    .unwrap();
    assert_eq!(captured.value.unwrap(), old);
    assert_plan_coverage(&captured.plan.unwrap(), 1, 0, 0, 1_999);
    let multiple = capture_sorted_recurrence_plan(|| {
        assert_eq!(prior_occurrences_batch(&tx, &queries).unwrap(), old);
        prior_occurrences_batch(&tx, &queries)
    })
    .unwrap();
    assert_eq!(multiple.value.unwrap(), old);
    assert_eq!(
        multiple.plan.unwrap_err(),
        SortedRecurrenceCaptureError::Multiple
    );
    assert_scope_cleared();
}

#[test]
fn delta_explain_error_is_static_and_preserves_operation_value() {
    let conn = connection();
    let captured = capture_sorted_recurrence_plan(|| {
        let mut capture = begin(&conn, SORTED_RECURRENCE_SQL, &[query()], &[0]).unwrap();
        capture.explain_delta_once(&conn, "invalid private delta SQL", &query(), 1, 2);
        7
    })
    .unwrap();
    assert_eq!(captured.value, 7);
    assert_eq!(
        captured.plan.unwrap_err(),
        SortedRecurrenceCaptureError::Explain
    );
    assert_scope_cleared();
}

#[test]
fn transaction_error_and_unwinding_clear_capture_after_real_queries() {
    let conn = connection();
    let tx = conn.unchecked_transaction().unwrap();
    let queries = vec![query(); 2_000];
    let captured = capture_sorted_recurrence_plan(|| {
        prior_occurrences_batch(&tx, &queries).unwrap();
        Err::<(), _>("operation failed")
    })
    .unwrap();
    assert_eq!(captured.value, Err("operation failed"));
    assert_plan_coverage(&captured.plan.unwrap(), 1, 0, 0, 1_999);
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = capture_sorted_recurrence_plan(|| {
            prior_occurrences_batch(&tx, &queries).unwrap();
            panic!("owned transaction capture fixture panic");
        });
    }))
    .is_err());
    assert_scope_cleared();
}
