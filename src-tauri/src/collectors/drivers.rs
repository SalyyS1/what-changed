//! Present-device driver collection with durable, atomic diffing.

mod diff;
mod observations;
mod state;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::collectors::stateful;
use crate::collectors::{
    AckedState, CancelToken, Collector, CollectorContext, CollectorDescriptor, CollectorSnapshot,
    CommitToken, ObservationClock, RawObservation, SnapshotContext, SystemObservationClock,
    WakeHandle,
};
use crate::domain::{CollectorStatusDetails, CollectorWakeTier};
use crate::error::{AppError, AppResult};
use crate::storage::CollectorCommitReceipt;

pub use diff::{compare_versions, VersionDirection};
pub use state::{DriverRecord, DriverSnapshot};

pub const ID: &str = "drivers";
pub const BASELINE_KIND: &str = "present_drivers";
pub const SWEEP_TIMEOUT: Duration = Duration::from_secs(10);

/// Version is the only driver mutation surfaced by live observations.
pub(crate) fn checkpoint_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    diff::checkpoint_fields(before, after)
}

/// Serializes SetupAPI queries across collector instance replacement.
///
/// The runtime deliberately discards a collector after a provider error. A
/// timed-out query, however, remains alive until its detached worker returns.
/// Sharing this coordinator through the factory prevents the replacement
/// instance from starting a second SetupAPI enumeration in that window.
#[derive(Clone, Default)]
pub struct DriverQueryCoordinator {
    busy: Arc<AtomicBool>,
}

impl DriverQueryCoordinator {
    fn try_begin(&self) -> bool {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn finish(&self) {
        self.busy.store(false, Ordering::Release);
    }
}

/// Wake source actually obtained by the platform provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverWakeTier {
    ConfigurationManager,
    Registry,
    TimerOnly,
}

impl DriverWakeTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConfigurationManager => "configuration_manager",
            Self::Registry => "registry",
            Self::TimerOnly => "timer_only",
        }
    }
}

/// Read-only, provider-injected driver enumeration boundary.
pub trait DriverProvider: Send + Sync + 'static {
    /// Enumerate present devices, cooperatively abandoning work at `deadline`.
    fn drivers(&self, cancel: &CancelToken, deadline: Instant) -> AppResult<Vec<DriverRecord>>;

    /// Try CM notification, then Class-tree registry notification, then timer.
    fn register_wake(&self, _wake: WakeHandle, _cancel: CancelToken) -> DriverWakeTier {
        DriverWakeTier::TimerOnly
    }

    /// Release any platform notification registration before provider drop.
    fn unregister_wake(&self) {}
}

pub fn catalog_descriptor() -> AppResult<CollectorDescriptor> {
    crate::collectors::registry::CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == ID)
        .map(|entry| entry.descriptor.clone())
        .ok_or_else(|| AppError::InvalidInput {
            what: "collector catalog",
            reason: "the drivers collector is not in the canonical catalog".to_owned(),
        })
}

pub struct DriverCollector<P, C = SystemObservationClock> {
    provider: Arc<P>,
    clock: C,
    descriptor: CollectorDescriptor,
    state: AckedState<DriverSnapshot>,
    context: Option<CollectorContext>,
    downtime: bool,
    downtime_clear_token: Option<CommitToken>,
    wake_tier: DriverWakeTier,
    query_coordinator: DriverQueryCoordinator,
}

impl<P: DriverProvider> DriverCollector<P, SystemObservationClock> {
    pub fn new(provider: P, descriptor: CollectorDescriptor) -> Self {
        Self::with_clock(provider, descriptor, SystemObservationClock)
    }

    pub fn with_query_coordinator(
        provider: P,
        descriptor: CollectorDescriptor,
        query_coordinator: DriverQueryCoordinator,
    ) -> Self {
        Self::with_clock_and_query_coordinator(
            provider,
            descriptor,
            SystemObservationClock,
            query_coordinator,
        )
    }
}

impl<P: DriverProvider, C: ObservationClock> DriverCollector<P, C> {
    pub fn with_clock(provider: P, descriptor: CollectorDescriptor, clock: C) -> Self {
        Self::with_clock_and_query_coordinator(
            provider,
            descriptor,
            clock,
            DriverQueryCoordinator::default(),
        )
    }

    pub fn with_clock_and_query_coordinator(
        provider: P,
        descriptor: CollectorDescriptor,
        clock: C,
        query_coordinator: DriverQueryCoordinator,
    ) -> Self {
        Self {
            provider: Arc::new(provider),
            clock,
            descriptor,
            state: AckedState::new(DriverSnapshot::default()),
            context: None,
            downtime: false,
            downtime_clear_token: None,
            wake_tier: DriverWakeTier::TimerOnly,
            query_coordinator,
        }
    }

    pub const fn wake_tier(&self) -> DriverWakeTier {
        self.wake_tier
    }

    fn query_with_deadline(&self, cancel: &CancelToken) -> AppResult<Option<Vec<DriverRecord>>> {
        if !self.query_coordinator.try_begin() {
            return Ok(None);
        }
        let deadline = Instant::now() + SWEEP_TIMEOUT;
        let provider = Arc::clone(&self.provider);
        let query_coordinator = self.query_coordinator.clone();
        let cycle_cancel = CancelToken::default();
        let worker_cancel = cycle_cancel.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        if let Err(source) = std::thread::Builder::new()
            .name("driver-setupapi-query".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    provider.drivers(&worker_cancel, deadline)
                }))
                .unwrap_or_else(|_| {
                    Err(AppError::Platform {
                        operation: "enumerate present drivers",
                        reason: "the driver query provider panicked".to_owned(),
                    })
                });
                query_coordinator.finish();
                let _ = sender.send(result);
            })
        {
            self.query_coordinator.finish();
            return Err(AppError::Platform {
                operation: "start a driver enumeration",
                reason: source.kind().to_string(),
            });
        }

        loop {
            if cancel.is_cancelled() {
                cycle_cancel.cancel();
                return Ok(None);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                cycle_cancel.cancel();
                return Err(timeout_error());
            }
            match receiver.recv_timeout(remaining.min(Duration::from_millis(50))) {
                Ok(result) => return result.map(Some),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(AppError::Platform {
                        operation: "enumerate present drivers",
                        reason: "the driver query worker stopped without a result".to_owned(),
                    });
                }
            }
        }
    }

    fn stage(
        &mut self,
        token: CommitToken,
        next: DriverSnapshot,
        now_ms: i64,
        observations: Vec<RawObservation>,
        clears_downtime: bool,
    ) -> AppResult<CollectorSnapshot> {
        let baseline = stateful::baseline_for(BASELINE_KIND, &next, now_ms)?;
        self.state.stage(token, next)?;
        self.downtime_clear_token = clears_downtime.then_some(token);
        Ok(CollectorSnapshot::Atomic {
            token,
            observations,
            relationships: Vec::new(),
            baseline: Some(baseline),
            config_snapshots: Vec::new(),
        })
    }
}

impl<P: DriverProvider, C: ObservationClock> Collector for DriverCollector<P, C> {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let (state, downtime) =
            DriverSnapshot::hydrate(context.state.baseline(BASELINE_KIND)?, self.clock.now_ms());
        self.state = AckedState::new(state);
        self.downtime = downtime;
        self.downtime_clear_token = None;
        self.wake_tier = self
            .provider
            .register_wake(context.wake.clone(), context.cancel.clone());
        let wake_tier = match self.wake_tier {
            DriverWakeTier::ConfigurationManager => CollectorWakeTier::ConfigurationManager,
            DriverWakeTier::Registry => CollectorWakeTier::Registry,
            DriverWakeTier::TimerOnly => CollectorWakeTier::TimerOnly,
        };
        if let Err(error) = context.status_details.replace(Some(CollectorStatusDetails {
            wake_tier: Some(wake_tier),
            docker_state: None,
            docker_api_version: None,
        })) {
            self.provider.unregister_wake();
            return Err(error);
        }
        self.context = Some(context);
        Ok(())
    }

    fn stop(&mut self) {
        self.provider.unregister_wake();
        self.context = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Err(AppError::InvalidInput {
            what: "driver collector",
            reason: "a stateful collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        let Some(cancel) = self.context.as_ref().map(|held| held.cancel.clone()) else {
            return Ok(CollectorSnapshot::Noop);
        };
        let now_ms = self.clock.now_ms();
        let Some(rows) = self.query_with_deadline(&cancel)? else {
            return Ok(CollectorSnapshot::Noop);
        };
        if cancel.is_cancelled() {
            return Ok(CollectorSnapshot::Noop);
        }
        let current = state::index(rows);
        if !self.state.committed().established {
            return self.stage(
                context.token(),
                DriverSnapshot {
                    acknowledged: current,
                    pending: None,
                    established: true,
                },
                now_ms,
                Vec::new(),
                true,
            );
        }

        let pending = self.state.committed().pending.clone();
        let (target, observed_at, uncertain) = pending
            .as_ref()
            .map(|pending| {
                (
                    pending.target.clone(),
                    pending.observed_at,
                    pending.occurred_at_is_uncertain,
                )
            })
            .unwrap_or((current, now_ms, self.downtime));
        let changes = diff::changes(&self.state.committed().acknowledged, &target);
        if changes.is_empty() {
            self.downtime = false;
            return Ok(CollectorSnapshot::Noop);
        }

        let mut capacity = stateful::CommitCapacity::full();
        let mut acknowledged = self.state.committed().acknowledged.clone();
        let mut observations = Vec::new();
        let mut drained = 0usize;
        for change in &changes {
            let observation = observations::for_change(change, observed_at, uncertain);
            if observation.is_some() && !capacity.spend(1, 0, 0) {
                break;
            }
            diff::apply(&mut acknowledged, change);
            observations.extend(observation);
            drained += 1;
        }
        let fully_drained = drained == changes.len();
        self.stage(
            context.token(),
            DriverSnapshot {
                acknowledged,
                pending: (!fully_drained).then_some(stateful::PendingTarget {
                    target,
                    observed_at,
                    occurred_at_is_uncertain: uncertain,
                }),
                established: true,
            },
            now_ms,
            observations,
            pending.is_none(),
        )
    }

    fn commit_succeeded(
        &mut self,
        token: CommitToken,
        _receipt: &CollectorCommitReceipt,
    ) -> AppResult<()> {
        self.state.ack(token)?;
        if self.downtime_clear_token == Some(token) {
            self.downtime = false;
        }
        self.downtime_clear_token = None;
        Ok(())
    }

    fn commit_failed(&mut self, token: CommitToken) -> AppResult<()> {
        let result = self.state.reject(token);
        if self.downtime_clear_token == Some(token) {
            self.downtime_clear_token = None;
        }
        result
    }
}

fn timeout_error() -> AppError {
    AppError::Platform {
        operation: "enumerate present drivers",
        reason: "the ten-second sweep deadline elapsed".to_owned(),
    }
}
