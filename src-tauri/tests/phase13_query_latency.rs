//! Opt-in Phase 13 proof that real network and Task Scheduler sweeps never
//! hold the timeline query path.

#![cfg(windows)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use what_changed_lib::collectors::network::NetworkProvider;
use what_changed_lib::collectors::startup::StartupProvider;
use what_changed_lib::collectors::CancelToken;
use what_changed_lib::domain::EventQuery;
use what_changed_lib::error::AppResult;
use what_changed_lib::platform::windows::network::WindowsNetworkProvider;
use what_changed_lib::platform::windows::tasks::WindowsStartupProvider;
use what_changed_lib::privacy::{Sanitized, Sanitizer};
use what_changed_lib::services::query_service;
use what_changed_lib::storage::Db;

const REQUIRED_OVERLAPPING_QUERIES: usize = 100;

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[(samples.len() * 95).div_ceil(100).saturating_sub(1)]
}

#[test]
#[ignore = "real-OS Phase 13 network/task-query concurrency gate"]
fn event_queries_stay_responsive_during_network_and_task_sweeps() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "phase13-query-latency",
        ))
        .unwrap(),
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

    let start = Arc::new(Barrier::new(3));
    let active_sweeps = Arc::new(AtomicUsize::new(0));
    let finished_workers = Arc::new(AtomicUsize::new(0));
    let stop = Arc::new(AtomicBool::new(false));

    let network_worker = {
        let start = Arc::clone(&start);
        let active_sweeps = Arc::clone(&active_sweeps);
        let finished_workers = Arc::clone(&finished_workers);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || -> AppResult<(usize, usize)> {
            start.wait();
            let result = (|| {
                let provider = WindowsNetworkProvider::default();
                let cancel = CancelToken::default();
                let mut sweeps = 0usize;
                let mut rows = 0usize;
                while !stop.load(Ordering::Acquire) {
                    active_sweeps.fetch_add(1, Ordering::AcqRel);
                    let result =
                        provider.network_state(&cancel, Instant::now() + Duration::from_secs(15));
                    active_sweeps.fetch_sub(1, Ordering::AcqRel);
                    rows = result?.adapters.len();
                    sweeps += 1;
                }
                Ok((sweeps, rows))
            })();
            finished_workers.fetch_add(1, Ordering::Release);
            result
        })
    };

    let startup_worker = {
        let start = Arc::clone(&start);
        let active_sweeps = Arc::clone(&active_sweeps);
        let finished_workers = Arc::clone(&finished_workers);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || -> AppResult<(usize, usize)> {
            start.wait();
            let result = (|| {
                let provider = WindowsStartupProvider::new()?;
                let cancel = CancelToken::default();
                let mut sweeps = 0usize;
                let mut rows = 0usize;
                while !stop.load(Ordering::Acquire) {
                    active_sweeps.fetch_add(1, Ordering::AcqRel);
                    let result =
                        provider.inventory(&cancel, Instant::now() + Duration::from_secs(15));
                    active_sweeps.fetch_sub(1, Ordering::AcqRel);
                    rows = result?.tasks.len();
                    sweeps += 1;
                }
                Ok((sweeps, rows))
            })();
            finished_workers.fetch_add(1, Ordering::Release);
            result
        })
    };

    start.wait();
    let gate_deadline = Instant::now() + Duration::from_secs(30);
    let mut samples = Vec::with_capacity(REQUIRED_OVERLAPPING_QUERIES);
    while samples.len() < REQUIRED_OVERLAPPING_QUERIES
        && finished_workers.load(Ordering::Acquire) < 2
        && Instant::now() < gate_deadline
    {
        if active_sweeps.load(Ordering::Acquire) == 0 {
            std::thread::yield_now();
            continue;
        }
        let query_started = Instant::now();
        query_service::page(&db, EventQuery::default())
            .expect("timeline query while Phase 13 sweeps are active");
        let elapsed = query_started.elapsed();
        if active_sweeps.load(Ordering::Acquire) > 0 {
            samples.push(elapsed);
        }
    }
    stop.store(true, Ordering::Release);

    let (network_sweeps, adapter_count) = network_worker
        .join()
        .expect("network sweep worker")
        .expect("real network sweep");
    let (startup_sweeps, task_count) = startup_worker
        .join()
        .expect("Task Scheduler sweep worker")
        .expect("real Task Scheduler sweep");

    assert_eq!(
        samples.len(),
        REQUIRED_OVERLAPPING_QUERIES,
        "the real sweeps did not expose enough overlapping query windows"
    );
    let query_p95 = p95(samples);
    eprintln!(
        "network sweeps={network_sweeps}, adapters={adapter_count}; task sweeps={startup_sweeps}, tasks={task_count}; overlapping query p95={query_p95:?}"
    );
    assert!(
        query_p95 <= Duration::from_millis(20),
        "concurrent event query p95 {query_p95:?} exceeded 20 ms"
    );
}
