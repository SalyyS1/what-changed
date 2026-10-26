//! Opt-in real-OS timing gate for Windows Update Agent history.

#![cfg(windows)]
#![allow(clippy::expect_used)]

use std::time::{Duration, Instant};

use what_changed_lib::collectors::updates::UpdateHistoryProvider;
use what_changed_lib::platform::windows::wua::WindowsUpdateProvider;

const POLLS: usize = 20;

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[(samples.len() * 95).div_ceil(100).saturating_sub(1)]
}

#[test]
#[ignore = "real-OS Phase 12 timing gate; run explicitly on a release workstation"]
fn wua_history_poll_meets_phase_12_p95_budget() {
    let provider = WindowsUpdateProvider::new().expect("start WUA apartment thread");
    let warm_rows = provider.history().expect("warm WUA history poll");
    assert!(!warm_rows.is_empty(), "WUA history should not be empty");

    let samples = (0..POLLS)
        .map(|_| {
            let started = Instant::now();
            let rows = provider.history().expect("WUA history poll");
            assert!(!rows.is_empty(), "WUA history should stay available");
            started.elapsed()
        })
        .collect();
    let history_p95 = p95(samples);
    eprintln!("WUA history p95={history_p95:?}; rows={}", warm_rows.len());
    assert!(
        history_p95 <= Duration::from_millis(200),
        "WUA history p95 {history_p95:?} exceeded 200 ms"
    );
}
