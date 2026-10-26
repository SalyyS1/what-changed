//! Release-oriented query evidence for the Phase 06 10k corpus.
//!
//! Run with:
//! `cargo test --manifest-path src-tauri/Cargo.toml --release --features devtools --test perf_scale -- --nocapture`
//!
//! The corpus is inserted through the production writer and sanitizer path;
//! this test does not write directly to SQLite.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use what_changed_lib::domain::{Cursor, EventQuery};
use what_changed_lib::services::{query_service, search_service, summary_service};
use what_changed_lib::storage::Db;
use what_changed_lib::testing::{synthetic, temp_dir};

const CORPUS_ROWS: usize = 10_000;
const BATCH_ROWS: usize = 200;
const ITERATIONS: usize = 11;
const PAGE_BUDGET: Duration = Duration::from_millis(10);
const SUMMARY_BUDGET: Duration = Duration::from_millis(15);
const SEARCH_BUDGET: Duration = Duration::from_millis(40);
const END_AT: i64 = 1_700_000_000_000;

#[test]
fn ten_thousand_events_meet_release_query_budgets() {
    let db = Db::open_in(&temp_dir("perf-scale")).expect("open performance database");
    let events = synthetic::profile(
        synthetic::SyntheticProfile::TenThousand,
        0x0610_0000,
        END_AT,
    );
    assert_eq!(events.len(), CORPUS_ROWS);
    assert!(events
        .iter()
        .all(|event| event.get().source_collector == "synthetic"));

    // Insert in bounded batches through the same writer and transaction path
    // used by ingestion. The generator has already passed every event through
    // Sanitizer before this call.
    for batch in events.chunks(BATCH_ROWS) {
        db.insert_events(batch.to_vec())
            .expect("insert sanitized event batch");
    }
    db.flush().expect("flush the writer before querying");

    let middle = CORPUS_ROWS / 2;
    let page_query = EventQuery {
        cursor: Some(Cursor {
            timestamp: events[middle].get().timestamp,
            // A fresh database allocates ids in insertion order, starting at 1.
            id: i64::try_from(middle + 1).expect("cursor id fits in i64"),
        }),
        limit: 200,
        ..EventQuery::default()
    };
    let from = events[0].get().timestamp;
    let to = events[CORPUS_ROWS - 1].get().timestamp;
    let search_text = "Synthetic Service";

    // Warm prepared statements, the reader pool, SQLite's page cache, and FTS5
    // before collecting samples. Warm-up timings are never included below.
    black_box(query_service::page(&db, page_query.clone()).expect("warm page"));
    black_box(summary_service::summary(&db, from, to).expect("warm summary"));
    black_box(search_service::search(&db, search_text, 100).expect("warm search"));

    let page_samples =
        measure(|| query_service::page(&db, page_query.clone()).expect("page query"));
    let summary_samples =
        measure(|| summary_service::summary(&db, from, to).expect("summary query"));
    let search_samples =
        measure(|| search_service::search(&db, search_text, 100).expect("search query"));

    report("page", &page_samples);
    report("summary", &summary_samples);
    report("search", &search_samples);

    assert_budget("page", &page_samples, PAGE_BUDGET);
    assert_budget("summary", &summary_samples, SUMMARY_BUDGET);
    assert_budget("search", &search_samples, SEARCH_BUDGET);
}

fn measure<T>(mut operation: impl FnMut() -> T) -> Vec<Duration> {
    let mut samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let started = Instant::now();
        black_box(operation());
        samples.push(started.elapsed());
    }
    samples
}

fn percentile(samples: &[Duration], percentile: usize) -> Duration {
    assert!(!samples.is_empty(), "percentile requires samples");
    let mut ordered = samples.to_vec();
    ordered.sort_unstable();
    // Nearest-rank p95: with 11 samples this intentionally selects the slowest.
    let rank = (ordered.len() * percentile).div_ceil(100).max(1);
    ordered[rank - 1]
}

fn report(name: &str, samples: &[Duration]) {
    let p50 = percentile(samples, 50);
    let p95 = percentile(samples, 95);
    println!(
        "perf {name}: iterations={} p50={:.3} ms p95={:.3} ms",
        samples.len(),
        p50.as_secs_f64() * 1_000.0,
        p95.as_secs_f64() * 1_000.0,
    );
}

fn assert_budget(name: &str, samples: &[Duration], budget: Duration) {
    let p95 = percentile(samples, 95);
    assert!(
        p95 <= budget,
        "{name} p95 {p95:?} exceeds release budget {budget:?}"
    );
}
