//! Isolated live-machine performance gates for the two diff-polled collectors.
//!
//! These measurements live in their own integration binary and share a lock so
//! Rust's parallel test harness cannot turn unrelated test load into collector
//! latency. The timed region calls the real collector snapshot boundary, so it
//! includes provider enumeration, current-state ownership/destruction, shared
//! map publication, state merging and diffing.

#![cfg(all(windows, feature = "devtools"))]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::Mutex;
use std::time::{Duration, Instant};

use what_changed_lib::collectors::listeners::ListenerCollector;
use what_changed_lib::collectors::process::ProcessCollector;
use what_changed_lib::collectors::process_map::ProcessMap;
use what_changed_lib::collectors::testing::live_context;
use what_changed_lib::collectors::{listeners, process};
use what_changed_lib::collectors::{Collector, CollectorSnapshot, SnapshotContext};
use what_changed_lib::platform::windows::process::WindowsProcessProvider;
use what_changed_lib::platform::windows::sockets::WindowsSocketProvider;
use what_changed_lib::storage::CollectorCommitReceipt;

#[path = "support/process_cost_diagnostic.rs"]
mod process_cost_diagnostic;

const CYCLES: usize = 50;
const BUDGET: Duration = Duration::from_millis(15);
static PERF_LOCK: Mutex<()> = Mutex::new(());

fn lock_perf() -> std::sync::MutexGuard<'static, ()> {
    PERF_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    let index = ((samples.len() as f64) * 0.95).ceil() as usize;
    samples[index.clamp(1, samples.len()) - 1]
}

#[test]
fn a_real_process_collector_cycle_stays_inside_the_budget() {
    let _serial = lock_perf();
    let live = live_context(process::ID).unwrap();
    let mut collector = ProcessCollector::new(
        WindowsProcessProvider,
        ProcessMap::new(),
        process::catalog_descriptor().unwrap(),
    );
    collector.start(live.context.clone()).unwrap();
    let initial_context = SnapshotContext::for_devtools(1);
    let initial = collector.snapshot_batch(initial_context).unwrap();
    assert!(matches!(initial, CollectorSnapshot::Atomic { .. }));
    collector
        .commit_succeeded(
            initial_context.token(),
            &CollectorCommitReceipt { event_ids: vec![] },
        )
        .unwrap();

    let samples = (0..CYCLES)
        .map(|index| {
            let token = u64::try_from(index).unwrap() + 2;
            let context = SnapshotContext::for_devtools(token);
            let began = Instant::now();
            let snapshot = collector.snapshot_batch(context).unwrap();
            if matches!(snapshot, CollectorSnapshot::Atomic { .. }) {
                collector
                    .commit_succeeded(
                        context.token(),
                        &CollectorCommitReceipt { event_ids: vec![] },
                    )
                    .unwrap();
            }
            std::hint::black_box(&snapshot);
            drop(snapshot);
            began.elapsed()
        })
        .collect();
    let p95 = p95(samples);
    eprintln!("process collector p95: {p95:?}");
    assert!(
        p95 <= BUDGET,
        "p95 full process snapshot cycle was {p95:?} against a {BUDGET:?} budget"
    );
}

#[test]
fn a_real_listener_collector_cycle_stays_inside_the_budget() {
    let _serial = lock_perf();
    let live = live_context(listeners::ID).unwrap();
    let mut collector = ListenerCollector::new(
        WindowsSocketProvider,
        ProcessMap::new(),
        listeners::catalog_descriptor().unwrap(),
    );
    collector.start(live.context.clone()).unwrap();
    let initial_context = SnapshotContext::for_devtools(1);
    let initial = collector.snapshot_batch(initial_context).unwrap();
    assert!(matches!(initial, CollectorSnapshot::Atomic { .. }));
    collector
        .commit_succeeded(
            initial_context.token(),
            &CollectorCommitReceipt { event_ids: vec![] },
        )
        .unwrap();

    let samples = (0..CYCLES)
        .map(|index| {
            let token = u64::try_from(index).unwrap() + 2;
            let context = SnapshotContext::for_devtools(token);
            let began = Instant::now();
            let snapshot = collector.snapshot_batch(context).unwrap();
            if matches!(snapshot, CollectorSnapshot::Atomic { .. }) {
                collector
                    .commit_succeeded(
                        context.token(),
                        &CollectorCommitReceipt { event_ids: vec![] },
                    )
                    .unwrap();
            }
            std::hint::black_box(&snapshot);
            drop(snapshot);
            began.elapsed()
        })
        .collect();
    let p95 = p95(samples);
    eprintln!("listener collector p95: {p95:?}");
    assert!(
        p95 <= BUDGET,
        "p95 full listener snapshot cycle was {p95:?} against a {BUDGET:?} budget"
    );
}
