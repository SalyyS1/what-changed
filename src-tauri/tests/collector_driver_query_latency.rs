//! Opt-in Phase 12 proof that a real SetupAPI sweep never holds the query path.

#![cfg(windows)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use what_changed_lib::collectors::drivers::DriverProvider;
use what_changed_lib::collectors::CancelToken;
use what_changed_lib::domain::EventQuery;
use what_changed_lib::platform::windows::devices::WindowsDriverProvider;
use what_changed_lib::privacy::{Sanitized, Sanitizer};
use what_changed_lib::services::query_service;
use what_changed_lib::storage::Db;

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[(samples.len() * 95).div_ceil(100).saturating_sub(1)]
}

#[test]
#[ignore = "real-OS Phase 12 SetupAPI/query concurrency gate"]
fn event_queries_stay_responsive_during_a_driver_sweep() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir("driver-query-latency")).unwrap(),
    );
    let sanitizer = Sanitizer::new();
    let events: Vec<Sanitized<_>> = (0..1_000)
        .map(|index| {
            sanitizer
                .sanitize(what_changed_lib::testing::synthetic_event(
                    index,
                    what_changed_lib::domain::now_ms().saturating_sub(index as i64),
                ))
                .0
        })
        .collect();
    db.insert_events(events).unwrap();

    let started = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let worker_started = Arc::clone(&started);
    let worker_finished = Arc::clone(&finished);
    let sweep = std::thread::spawn(move || {
        worker_started.store(true, Ordering::Release);
        let rows = WindowsDriverProvider::default()
            .drivers(
                &CancelToken::default(),
                Instant::now() + Duration::from_secs(10),
            )
            .expect("real SetupAPI sweep");
        worker_finished.store(true, Ordering::Release);
        rows.len()
    });
    while !started.load(Ordering::Acquire) {
        std::thread::yield_now();
    }

    let mut samples = Vec::new();
    while !finished.load(Ordering::Acquire) || samples.len() < 20 {
        let query_started = Instant::now();
        query_service::page(&db, EventQuery::default()).expect("timeline query during sweep");
        samples.push(query_started.elapsed());
        if samples.len() >= 200 {
            break;
        }
    }
    let rows = sweep.join().expect("driver sweep thread");
    let query_p95 = p95(samples);
    eprintln!("driver rows={rows}; concurrent query p95={query_p95:?}");
    assert!(
        query_p95 <= Duration::from_millis(20),
        "concurrent event query p95 {query_p95:?} exceeded 20 ms"
    );
}
