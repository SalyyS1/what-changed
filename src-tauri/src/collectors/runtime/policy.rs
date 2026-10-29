//! Injectable runtime policies and platform-neutral notification contracts.
//!
//! These definitions form the supervisor's construction seam. Supervisor,
//! monitoring-fence and shutdown mechanics remain in `runtime.rs`; the serial
//! provider and acknowledgement cycle lives in `worker.rs`.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::app::Monitoring;
use crate::collectors::schedule::QueryBusy;
use crate::domain::{CollectorCadence, CollectorCost, MonitoringProfile};
use crate::dto::MonitoringDto;
use crate::error::AppResult;
use crate::storage::Db;

use super::{read_lock, signal, EntryState, Shared};

/// Injectable time source; test clocks need not wait for wall time.
pub trait RuntimeClock: Send + Sync + 'static {
    fn now(&self) -> Instant;
    fn now_ms(&self) -> i64;
}

#[derive(Debug, Default)]
pub struct SystemClock;

impl RuntimeClock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn now_ms(&self) -> i64 {
        crate::domain::now_ms()
    }
}

/// Injectable retry schedule, keeping five-minute waits out of tests.
pub trait BackoffPolicy: Send + Sync + 'static {
    fn delay(&self, consecutive_failures: u64) -> Duration;
}

#[derive(Debug, Default)]
pub struct ExponentialBackoff;

impl BackoffPolicy for ExponentialBackoff {
    fn delay(&self, failures: u64) -> Duration {
        match failures {
            0 => Duration::ZERO,
            1 => Duration::from_secs(5),
            2 => Duration::from_secs(30),
            _ => Duration::from_secs(5 * 60),
        }
    }
}

/// Platform-neutral delivery for runtime state changes.
///
/// The Tauri adapter lives in `lib.rs`; tests use an in-memory sink. Keeping
/// `AppHandle` out of this module preserves the cross-platform collector core.
pub trait RuntimeStatusSink: Send + Sync + 'static {
    fn collector_changed(&self, collector_id: &str);
    fn monitoring_changed(&self, monitoring: MonitoringDto);

    /// Observe that an atomic collector cycle entered the durable writer.
    ///
    /// Admission is irrevocable but does not mean SQLite committed it yet.
    /// This callback must remain observer-only; the worker contains panics so
    /// instrumentation cannot alter acknowledgement semantics.
    fn collector_commit_admitted(&self, _collector_id: &str) {}

    /// Observe successful worker snapshot outcomes or failure notifications.
    /// Success includes Noop/legacy drains, not only atomic durable commits.
    /// Failures can describe startup or asynchronous delivery, so they are not
    /// an exact failed-cycle count. Observers must not re-enter the runtime.
    /// Panics are contained; implementations must remain short and nonblocking.
    fn collector_outcome_observed(&self, _collector_id: &str, _success: bool) {}
}

#[derive(Debug, Default)]
pub(super) struct SilentStatusSink;

impl RuntimeStatusSink for SilentStatusSink {
    fn collector_changed(&self, _collector_id: &str) {}

    fn monitoring_changed(&self, _monitoring: MonitoringDto) {}
}

/// Dependencies and initial settings for a collector runtime.
///
/// Grouping these values keeps test construction explicit without an
/// error-prone positional argument list.
pub struct CollectorRuntimeConfig {
    pub db: Arc<Db>,
    pub monitor: Arc<Monitoring>,
    pub query_busy: QueryBusy,
    pub enabled: BTreeMap<String, bool>,
    pub profile: MonitoringProfile,
    pub clock: Arc<dyn RuntimeClock>,
    pub backoff: Arc<dyn BackoffPolicy>,
}

pub(super) const fn cadence_supports_targeted_wake(cadence: CollectorCadence) -> bool {
    matches!(
        cadence,
        CollectorCadence::Notify | CollectorCadence::NotifyPlusSweep(_)
    )
}

pub(super) fn request_targeted_snapshot(
    shared: &Shared,
    state: &EntryState,
    generation: u64,
    monitoring_generation: u64,
    instance_generation: u64,
) -> AppResult<()> {
    let _gate = read_lock(&shared.emission_gate);
    if shared.stop.load(Ordering::Acquire)
        || !state.is_enabled()
        || shared.monitor.is_paused()
        || state.generation.load(Ordering::Acquire) != generation
        || shared.monitoring_generation.load(Ordering::Acquire) != monitoring_generation
        || state.instance_generation.load(Ordering::Acquire) != instance_generation
    {
        return Err(inactive_wake_error());
    }
    if !cadence_supports_targeted_wake(state.descriptor.cadence) {
        return Ok(());
    }
    if !state.targeted_wake_pending.swap(true, Ordering::AcqRel) {
        signal(shared);
    }
    Ok(())
}

fn inactive_wake_error() -> crate::error::AppError {
    crate::error::AppError::InvalidInput {
        what: "collector wake",
        reason: "collector context is no longer active".to_owned(),
    }
}

/// A runtime-wide, nonblocking lease for resource-heavy snapshots.
///
/// The flag is released even when collector code panics because the unwind is
/// caught outside the RAII guard. Waiting workers retry rather than blocking a
/// lifecycle or shutdown thread on a mutex.
pub(super) struct ExpensiveSnapshotLease<'a> {
    shared: Option<&'a Shared>,
}

impl<'a> ExpensiveSnapshotLease<'a> {
    pub(super) fn try_acquire(shared: &'a Shared, cost: CollectorCost) -> Option<Self> {
        if cost != CollectorCost::Expensive {
            return Some(Self { shared: None });
        }
        shared
            .expensive_snapshot_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self {
                shared: Some(shared),
            })
    }
}

impl Drop for ExpensiveSnapshotLease<'_> {
    fn drop(&mut self) {
        if let Some(shared) = self.shared {
            shared
                .expensive_snapshot_active
                .store(false, Ordering::Release);
            signal(shared);
        }
    }
}
