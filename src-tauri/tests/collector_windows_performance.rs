//! Opt-in real-OS timing gate for the Phase 11 Windows providers.
//!
//! This stays ignored in the normal suite because the thresholds describe a
//! release workstation acceptance run, not a deterministic CI invariant.

#![cfg(windows)]
#![allow(clippy::expect_used)]

use std::time::{Duration, Instant};

use what_changed_lib::collectors::applications::RegistryUninstallProvider;
use what_changed_lib::collectors::services::ServiceProvider;
use what_changed_lib::platform::windows::registry::WindowsRegistryUninstallProvider;
use what_changed_lib::platform::windows::services::WindowsServiceProvider;

const SWEEPS: usize = 20;

fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[(samples.len() * 95).div_ceil(100).saturating_sub(1)]
}

#[test]
#[ignore = "real-OS Phase 11 timing gate; run explicitly on a release workstation"]
fn service_and_registry_sweeps_meet_phase_11_p95_budgets() {
    let services = WindowsServiceProvider::default();
    let registry = WindowsRegistryUninstallProvider;

    // The first call also performs the owned 15-minute full configuration
    // reconciliation. Report it separately from the steady five-second sweep.
    let full_refresh_started = Instant::now();
    services
        .services()
        .expect("full service configuration sweep");
    let full_refresh = full_refresh_started.elapsed();
    registry.enumerate().expect("warm registry sweep");

    let service_samples = (0..SWEEPS)
        .map(|_| {
            let started = Instant::now();
            services.services().expect("service sweep");
            started.elapsed()
        })
        .collect();
    let registry_samples = (0..SWEEPS)
        .map(|_| {
            let started = Instant::now();
            registry.enumerate().expect("registry sweep");
            started.elapsed()
        })
        .collect();

    let service_p95 = p95(service_samples);
    let registry_p95 = p95(registry_samples);
    eprintln!(
        "service p95={service_p95:?}; service full-config refresh={full_refresh:?}; registry p95={registry_p95:?}"
    );
    assert!(
        service_p95 <= Duration::from_millis(12),
        "service p95 {service_p95:?} exceeded 12 ms"
    );
    assert!(
        registry_p95 <= Duration::from_millis(150),
        "registry p95 {registry_p95:?} exceeded 150 ms"
    );
    assert!(
        full_refresh <= Duration::from_millis(500),
        "service full-config refresh {full_refresh:?} exceeded 500 ms"
    );
}
