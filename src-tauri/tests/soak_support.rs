//! Pure checks shared by the PowerShell soak evidence.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

#[derive(Debug, Clone, Copy, PartialEq)]
struct GrowthVerdict {
    working_set_percent: f64,
    handle_percent: f64,
    threads_delta: i64,
}

fn verdict(
    first_working: u64,
    last_working: u64,
    first_handles: u64,
    last_handles: u64,
    first_threads: i64,
    last_threads: i64,
) -> GrowthVerdict {
    let working_set_percent = if first_working == 0 {
        0.0
    } else {
        (last_working as f64 - first_working as f64) * 100.0 / first_working as f64
    };
    let handle_percent = if first_handles == 0 {
        0.0
    } else {
        (last_handles as f64 - first_handles as f64) * 100.0 / first_handles as f64
    };
    GrowthVerdict {
        working_set_percent,
        handle_percent,
        threads_delta: last_threads - first_threads,
    }
}

#[test]
fn growth_rules_are_explicit_and_conservative() {
    let pass = verdict(100, 114, 100, 104, 10, 10);
    assert!(pass.working_set_percent <= 15.0);
    assert!(pass.handle_percent.abs() <= 5.0);
    assert_eq!(pass.threads_delta, 0);
    let fail = verdict(100, 116, 100, 106, 10, 11);
    assert!(
        fail.working_set_percent > 15.0
            || fail.handle_percent.abs() > 5.0
            || fail.threads_delta != 0
    );
}
