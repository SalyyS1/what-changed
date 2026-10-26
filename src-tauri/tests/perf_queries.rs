//! Explicit (ignored by default) Phase 20 query budget gate.
//!
//! The normal test suite must stay quick and deterministic, so this gate is
//! opt-in. Use `cargo test --features devtools --test perf_queries
//! -- --include-ignored --nocapture`; set `WHAT_CHANGED_PERF_ROWS` to
//! `10000`, `100000`, or `1000000` to select a corpus. All eight §61 service
//! paths are measured over 30 warm samples at the selected scale.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use what_changed_lib::commands::privacy::privacy_summary;
use what_changed_lib::domain::{EventQuery, InvestigationWindow, TimeRange};
use what_changed_lib::services::investigation_service::InvestigationService;
use what_changed_lib::services::{query_service, search_service, summary_service};
use what_changed_lib::storage::{
    checkpoint_repository, event_repository, Db, Fts5Index, SearchIndex,
};

#[path = "support/perf_fixture.rs"]
mod perf_fixture;
use perf_fixture::PerfFixture;

const ITERATIONS: usize = 30;

fn rows() -> usize {
    std::env::var("WHAT_CHANGED_PERF_ROWS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(10_000)
}

fn percentile(samples: &mut [Duration], rank: usize) -> Duration {
    samples.sort_unstable();
    let index = samples.len().saturating_mul(rank).div_ceil(100).max(1) - 1;
    samples[index]
}

fn measure(mut operation: impl FnMut()) -> Vec<Duration> {
    (0..ITERATIONS)
        .map(|_| {
            let started = Instant::now();
            operation();
            started.elapsed()
        })
        .collect()
}

struct Gate {
    name: &'static str,
    budget: Duration,
    samples: Vec<Duration>,
}

impl Gate {
    fn new(name: &'static str, budget_ms: u64, operation: impl FnMut()) -> Self {
        Self {
            name,
            budget: Duration::from_millis(budget_ms),
            samples: measure(operation),
        }
    }
}

fn assert_gates(rows: usize, gates: &mut [Gate]) {
    let mut failures = Vec::new();
    for gate in gates {
        let p95 = percentile(&mut gate.samples, 95);
        println!(
            "phase20 rows={rows} shape={} samples={} p95_ms={:.3} budget_ms={}",
            gate.name,
            gate.samples.len(),
            p95.as_secs_f64() * 1_000.0,
            gate.budget.as_millis()
        );
        if p95 > gate.budget {
            failures.push(format!(
                "{} p95 {:.3} ms > {} ms",
                gate.name,
                p95.as_secs_f64() * 1_000.0,
                gate.budget.as_millis()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("; "));
}

fn page_ten_query(db: &Db, first_query: &EventQuery) -> EventQuery {
    let mut query = first_query.clone();
    // Follow the actual service's keyset cursor through nine 200-row pages;
    // an arbitrary middle-of-corpus cursor is not the specified page 10.
    for page_number in 1..=9 {
        let page = query_service::page(db, query.clone()).expect("advance keyset cursor");
        query.cursor =
            Some(page.next.unwrap_or_else(|| {
                panic!("page {page_number} ended before the requested tenth page")
            }));
    }
    query
}

/// One diagnostic pass through the SQL stages used by the service calls. This
/// is not a substitute for the end-to-end p95 budget measured above.
fn print_stage_diagnostics(db: &Db, from: i64, to: i64) {
    db.read(|conn| {
        let started = Instant::now();
        let checkpoints = checkpoint_repository::search_labels(conn, "Synthetic Service", 20)?;
        let checkpoint_search_ms = started.elapsed().as_secs_f64() * 1_000.0;

        let started = Instant::now();
        let event_limit = 50_u16.saturating_sub(u16::try_from(checkpoints.len()).unwrap_or(50));
        let ids = Fts5Index.search(conn, "Synthetic Service", event_limit)?;
        let fts_search_ms = started.elapsed().as_secs_f64() * 1_000.0;

        let started = Instant::now();
        let events = event_repository::summaries_by_ids(conn, &ids)?;
        let event_hydrate_ms = started.elapsed().as_secs_f64() * 1_000.0;
        println!(
            "phase20 search_stages checkpoint_ms={checkpoint_search_ms:.3} fts_join_sort_ms={fts_search_ms:.3} hydrate_ms={event_hydrate_ms:.3} checkpoint_hits={} event_hits={}",
            checkpoints.len(),
            events.len()
        );

        let started = Instant::now();
        let counts = event_repository::counts_for_range(conn, from, to)?;
        let four_counts_ms = started.elapsed().as_secs_f64() * 1_000.0;

        let started = Instant::now();
        let important = event_repository::top_important(conn, from, to, 3)?;
        let top_important_ms = started.elapsed().as_secs_f64() * 1_000.0;

        let started = Instant::now();
        let checkpoint = checkpoint_repository::latest(conn)?;
        let latest_checkpoint_ms = started.elapsed().as_secs_f64() * 1_000.0;
        println!(
            "phase20 summary_stages four_counts_ms={four_counts_ms:.3} top_important_ms={top_important_ms:.3} latest_checkpoint_ms={latest_checkpoint_ms:.3} total_count={} important_hits={} checkpoint_present={}",
            counts.total(),
            important.len(),
            checkpoint.is_some()
        );
        Ok(())
    })
    .expect("measure query stages");
}

#[test]
#[ignore = "Phase 20 performance gate; run explicitly on a release-like machine"]
fn all_eight_query_shapes_meet_the_selected_budget() {
    let row_count = rows();
    let (fixture, corpus) = PerfFixture::new_seeded_app_state(row_count);
    let (from, to, details_id, investigation_from) = (
        corpus.from,
        corpus.to,
        corpus.details_id,
        corpus.investigation_from,
    );
    let db = fixture.db();
    let first_query = EventQuery {
        range: TimeRange::Custom { from, to },
        limit: 200,
        ..EventQuery::default()
    };
    let page_ten_query = page_ten_query(db, &first_query);
    let investigation_window = InvestigationWindow::Custom {
        from_ms: investigation_from,
        to_ms: to,
    };
    let investigation = InvestigationService::new(Arc::clone(&fixture.state().db));
    let [left_checkpoint, right_checkpoint] = corpus.checkpoint_ids;

    assert!(query_service::page(db, first_query.clone())
        .expect("warm first page")
        .next
        .is_some());
    assert!(query_service::page(db, page_ten_query.clone())
        .expect("warm tenth page")
        .next
        .is_some());
    std::hint::black_box(summary_service::summary(db, from, to).expect("warm summary"));
    assert!(!search_service::search(db, "Synthetic Service", 50)
        .expect("warm two-token search")
        .is_empty());
    assert!(
        query_service::details(db, details_id)
            .expect("warm event details")
            .prior_occurrences
            > 0,
        "details fixture must exercise nonzero recurrence"
    );
    let warm_investigation = investigation
        .start(
            "Synthetic Service stopped working".to_owned(),
            Some(investigation_window.clone()),
        )
        .expect("warm persisted investigation");
    assert_eq!(warm_investigation.total_events_in_window, 5_000);
    assert_eq!(warm_investigation.analyzed_events, 5_000);
    assert!(
        warm_investigation.no_cause_reason.is_none(),
        "the canonical investigation must execute scoring, not the no-cause shortcut"
    );
    assert!(
        !warm_investigation.suspects.is_empty(),
        "the canonical investigation must rank real seeded suspects"
    );
    assert!(!fixture
        .state()
        .checkpoints
        .compare(left_checkpoint, Some(right_checkpoint))
        .expect("warm checkpoint comparison")
        .changed
        .is_empty());
    assert_eq!(
        privacy_summary(fixture.state())
            .expect("warm privacy summary")
            .categories
            .iter()
            .map(|category| category.count)
            .sum::<u64>(),
        u64::try_from(row_count).expect("row count fits")
    );

    let budget = match row_count {
        10_000 => [10, 10, 15, 40, 10, 400, 150, 30],
        100_000 => [20, 20, 30, 90, 15, 600, 200, 60],
        _ => [45, 45, 80, 250, 30, 1_500, 300, 200],
    };
    let mut gates = vec![
        Gate::new("get_events_first_page_200", budget[0], || {
            std::hint::black_box(query_service::page(db, first_query.clone()).expect("first page"));
        }),
        Gate::new("get_events_page_10_keyset", budget[1], || {
            std::hint::black_box(query_service::page(db, page_ten_query.clone()).expect("page 10"));
        }),
        Gate::new("get_dashboard_summary", budget[2], || {
            std::hint::black_box(summary_service::summary(db, from, to).expect("summary"));
        }),
        Gate::new("search_events_two_token", budget[3], || {
            std::hint::black_box(
                search_service::search(db, "Synthetic Service", 50).expect("two-token search"),
            );
        }),
        Gate::new("get_event_details_and_prior", budget[4], || {
            std::hint::black_box(query_service::details(db, details_id).expect("event details"));
        }),
        Gate::new("start_investigation_5k", budget[5], || {
            std::hint::black_box(
                investigation
                    .start(
                        "Synthetic Service stopped working".to_owned(),
                        Some(investigation_window.clone()),
                    )
                    .expect("persisted 5k investigation"),
            );
        }),
        Gate::new("compare_checkpoint_3k", budget[6], || {
            std::hint::black_box(
                fixture
                    .state()
                    .checkpoints
                    .compare(left_checkpoint, Some(right_checkpoint))
                    .expect("3k-item checkpoint comparison"),
            );
        }),
        Gate::new("get_privacy_summary", budget[7], || {
            std::hint::black_box(privacy_summary(fixture.state()).expect("privacy summary"));
        }),
    ];
    print_stage_diagnostics(db, from, to);
    assert_gates(row_count, &mut gates);
}
