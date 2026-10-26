//! Explicit actual-service diagnostics, separate from the acceptance workload.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use what_changed_lib::domain::{Investigation, InvestigationWindow};
use what_changed_lib::services::investigation_service::InvestigationService;
use what_changed_lib::storage::event_repository::{
    capture_sorted_recurrence_plan, SortedRecurrenceCountKind,
};

#[path = "support/perf_fixture.rs"]
mod perf_fixture;
use perf_fixture::PerfFixture;

fn assert_scored_persisted(result: &Investigation) {
    assert!(
        result.id > 0,
        "actual persisted investigation must receive an id"
    );
    assert_eq!(result.total_events_in_window, 5_000);
    assert_eq!(result.analyzed_events, 5_000);
    assert!(
        result.no_cause_reason.is_none(),
        "the canonical window must execute scoring"
    );
    assert!(
        !result.suspects.is_empty(),
        "the canonical window must rank seeded suspects"
    );
}

fn assert_indexed_plan(details: &[String]) {
    assert!(details.iter().any(|detail| {
        let normalized = detail.to_ascii_uppercase();
        normalized.contains("SEARCH EVENTS") && normalized.contains("INDEX")
    }));
    assert!(details.iter().all(|detail| {
        let normalized = detail.to_ascii_uppercase();
        !normalized.contains("SCAN EVENTS") && !normalized.contains("SCAN TABLE EVENTS")
    }));
}

#[test]
#[ignore = "Diagnostic profile; requires explicit stage timing opt-in and a fresh canonical corpus"]
fn five_persisted_investigations_report_actual_stage_timings() {
    run_profile(5);
}

#[test]
#[ignore = "Diagnostic tail attribution; requires explicit stage timing opt-in and a fresh canonical corpus"]
fn thirty_persisted_investigations_report_paired_stage_timings() {
    run_profile(30);
}

fn run_profile(sample_count: usize) {
    assert_eq!(
        std::env::var("WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS").as_deref(),
        Ok("1"),
        "set WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS=1 for this diagnostic target"
    );
    let rows = match std::env::var("WHAT_CHANGED_PERF_ROWS") {
        Err(std::env::VarError::NotPresent) => 10_000,
        Ok(value) => value
            .parse::<usize>()
            .expect("WHAT_CHANGED_PERF_ROWS must be an integer"),
        Err(error) => panic!("invalid WHAT_CHANGED_PERF_ROWS: {error}"),
    };
    let (fixture, corpus) = PerfFixture::new_seeded_app_state(rows);
    let service = InvestigationService::new(Arc::clone(&fixture.state().db));
    let window = InvestigationWindow::Custom {
        from_ms: corpus.investigation_from,
        to_ms: corpus.to,
    };
    // EXPLAIN uses the first actual full and delta ranges on their transaction
    // connection. The capture is cleared before any measured request.
    eprintln!("DIAGNOSTIC profile_call sample=warm status=begin");
    let captured = capture_sorted_recurrence_plan(|| {
        service.start(
            "Synthetic Service stopped working".to_owned(),
            Some(window.clone()),
        )
    })
    .expect("one nonnested diagnostic capture scope");
    eprintln!("DIAGNOSTIC profile_call sample=warm status=end");
    let warm = captured.value.expect("warm actual persisted investigation");
    assert_scored_persisted(&warm);
    let plan = captured
        .plan
        .expect("executed sorted recurrence bound plan");
    assert_eq!(plan.input_queries, 5_000);
    assert_eq!(plan.completed_input_queries, plan.input_queries);
    assert_eq!(
        plan.completed_executions,
        plan.completed_full_executions + plan.completed_delta_executions
    );
    assert_eq!(
        plan.completed_input_queries,
        plan.completed_full_executions + plan.completed_rolling_queries + plan.identical_reuses
    );
    assert!(
        plan.completed_rolling_queries <= plan.completed_delta_executions
            && plan.completed_delta_executions <= plan.completed_rolling_queries * 2
    );
    assert_eq!(plan.full_plan_kind, SortedRecurrenceCountKind::Full);
    assert_eq!(
        plan.delta_plan.is_some(),
        plan.completed_delta_executions > 0
    );
    assert_eq!(plan.window_statistics.input_queries, plan.input_queries);
    let statistics = plan.window_statistics;
    eprintln!(
        "DIAGNOSTIC recurrence_window_statistics input_queries={} distinct_signatures={} distinct_windows={} identical_window_repeats={} empty_or_reversed_windows={} max_queries_per_signature={} adjacent_overlapping_windows={} monotone_before_overlapping_windows={}",
        statistics.input_queries,
        statistics.distinct_signatures,
        statistics.distinct_windows,
        statistics.identical_window_repeats,
        statistics.empty_or_reversed_windows,
        statistics.max_queries_per_signature,
        statistics.adjacent_overlapping_windows,
        statistics.monotone_before_overlapping_windows,
    );
    assert_indexed_plan(&plan.details);
    if let Some(delta) = &plan.delta_plan {
        assert_eq!(delta.kind, SortedRecurrenceCountKind::Delta);
        assert_indexed_plan(&delta.details);
    }
    eprintln!(
        "DIAGNOSTIC recurrence_execution_counts input_queries={} completed_input_queries={} completed_full_executions={} completed_delta_executions={} completed_rolling_queries={} identical_reuses={} completed_executions={}",
        plan.input_queries, plan.completed_input_queries, plan.completed_full_executions,
        plan.completed_delta_executions, plan.completed_rolling_queries,
        plan.identical_reuses, plan.completed_executions,
    );
    for detail in plan.details {
        eprintln!(
            "DIAGNOSTIC recurrence_bound_plan kind={} input_queries={} completed_executions={} detail={detail}",
            plan.full_plan_kind.as_str(), plan.input_queries, plan.completed_executions,
        );
    }
    if let Some(delta) = plan.delta_plan {
        for detail in delta.details {
            eprintln!(
                "DIAGNOSTIC recurrence_bound_plan kind={} input_queries={} completed_executions={} detail={detail}",
                delta.kind.as_str(), plan.input_queries, plan.completed_executions,
            );
        }
    }

    let samples: Vec<Duration> = (1..=sample_count)
        .map(|sample| {
            // Markers are outside the e2e clock. Existing stage output occurs
            // inside the call, so these durations remain diagnostic, not acceptance.
            eprintln!("DIAGNOSTIC profile_call sample={sample} status=begin");
            let started = Instant::now();
            let result = service
                .start(
                    "Synthetic Service stopped working".to_owned(),
                    Some(window.clone()),
                )
                .expect("actual persisted diagnostic investigation");
            let elapsed = started.elapsed();
            eprintln!(
                "DIAGNOSTIC profile_call sample={sample} status=end e2e_ms={:.3}",
                elapsed.as_secs_f64() * 1000.0,
            );
            assert_scored_persisted(&result);
            elapsed
        })
        .collect();
    let total: Duration = samples.iter().sum();
    eprintln!(
        "DIAGNOSTIC investigation_profile rows={rows} samples={} e2e_total_ms={:.3} e2e_mean_ms={:.3} e2e_min_ms={:.3} e2e_max_ms={:.3}",
        samples.len(), total.as_secs_f64() * 1000.0,
        total.as_secs_f64() * 1000.0 / samples.len() as f64,
        samples.iter().min().expect("profile samples").as_secs_f64() * 1000.0,
        samples.iter().max().expect("profile samples").as_secs_f64() * 1000.0,
    );
}
