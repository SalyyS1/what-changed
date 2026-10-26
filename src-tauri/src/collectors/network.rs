//! Network configuration collection with durable, per-field diffing.

pub(crate) mod diff;
mod observations;
mod state;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::collectors::stateful;
use crate::collectors::{
    AckedState, CancelToken, Collector, CollectorContext, CollectorDescriptor, CollectorSnapshot,
    CommitToken, ObservationClock, SnapshotContext, SystemObservationClock, WakeHandle,
};
use crate::error::{AppError, AppResult};
use crate::privacy::Sanitizer;
use crate::storage::CollectorCommitReceipt;

pub use state::{NetworkAdapter, NetworkSnapshot, NetworkState, ProxyConfiguration};

pub const ID: &str = "network";
pub const BASELINE_KIND: &str = "network_configuration";
pub const SWEEP_TIMEOUT: Duration = Duration::from_secs(15);

/// Platform notification coverage obtained for the current collector instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkWakeTier {
    IpHelperAndRegistry,
    IpHelper,
    Registry,
    TimerOnly,
}

/// Read-only, provider-injected network configuration boundary.
pub trait NetworkProvider: Send + Sync + 'static {
    fn network_state(&self, cancel: &CancelToken, deadline: Instant) -> AppResult<NetworkState>;

    fn register_wake(&self, _wake: WakeHandle, _cancel: CancelToken) -> NetworkWakeTier {
        NetworkWakeTier::TimerOnly
    }

    fn unregister_wake(&self) {}
}

/// Serializes expensive network enumerations across runtime replacement.
#[derive(Clone, Default)]
pub struct NetworkQueryCoordinator {
    busy: Arc<AtomicBool>,
}

impl NetworkQueryCoordinator {
    fn try_begin(&self) -> bool {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn finish(&self) {
        self.busy.store(false, Ordering::Release);
    }
}

pub struct NetworkCollector<P, C = SystemObservationClock> {
    provider: Arc<P>,
    clock: C,
    descriptor: CollectorDescriptor,
    state: AckedState<NetworkSnapshot>,
    context: Option<CollectorContext>,
    downtime: bool,
    downtime_clear_token: Option<CommitToken>,
    wake_tier: NetworkWakeTier,
    query_coordinator: NetworkQueryCoordinator,
}

impl<P: NetworkProvider> NetworkCollector<P, SystemObservationClock> {
    pub fn new(provider: P, descriptor: CollectorDescriptor) -> Self {
        Self::with_clock(provider, descriptor, SystemObservationClock)
    }

    pub fn with_query_coordinator(
        provider: P,
        descriptor: CollectorDescriptor,
        query_coordinator: NetworkQueryCoordinator,
    ) -> Self {
        Self::with_clock_and_query_coordinator(
            provider,
            descriptor,
            SystemObservationClock,
            query_coordinator,
        )
    }
}

impl<P: NetworkProvider, C: ObservationClock> NetworkCollector<P, C> {
    pub fn with_clock(provider: P, descriptor: CollectorDescriptor, clock: C) -> Self {
        Self::with_clock_and_query_coordinator(
            provider,
            descriptor,
            clock,
            NetworkQueryCoordinator::default(),
        )
    }

    pub fn with_clock_and_query_coordinator(
        provider: P,
        descriptor: CollectorDescriptor,
        clock: C,
        query_coordinator: NetworkQueryCoordinator,
    ) -> Self {
        Self {
            provider: Arc::new(provider),
            clock,
            descriptor,
            state: AckedState::new(NetworkSnapshot::default()),
            context: None,
            downtime: false,
            downtime_clear_token: None,
            wake_tier: NetworkWakeTier::TimerOnly,
            query_coordinator,
        }
    }

    pub const fn wake_tier(&self) -> NetworkWakeTier {
        self.wake_tier
    }

    fn query_with_deadline(&self, cancel: &CancelToken) -> AppResult<Option<NetworkState>> {
        if !self.query_coordinator.try_begin() {
            return Ok(None);
        }
        let deadline = Instant::now() + SWEEP_TIMEOUT;
        let provider = Arc::clone(&self.provider);
        let coordinator = self.query_coordinator.clone();
        let cycle_cancel = CancelToken::default();
        let worker_cancel = cycle_cancel.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        if let Err(source) = std::thread::Builder::new()
            .name("network-config-query".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    provider.network_state(&worker_cancel, deadline)
                }))
                .unwrap_or_else(|_| {
                    Err(AppError::Platform {
                        operation: "enumerate network configuration",
                        reason: "the network provider panicked".to_owned(),
                    })
                })
                // Diff the same canonical representation that is persisted.
                // Otherwise a credential-shaped proxy value becomes redacted
                // only on disk and looks changed again after every restart.
                .map(|state| Sanitizer::new().sanitize(state).0.into_inner());
                coordinator.finish();
                let _ = sender.send(result);
            })
        {
            self.query_coordinator.finish();
            return Err(AppError::Platform {
                operation: "start a network enumeration",
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
                        operation: "enumerate network configuration",
                        reason: "the network query worker stopped without a result".to_owned(),
                    });
                }
            }
        }
    }

    fn stage(
        &mut self,
        token: CommitToken,
        next: NetworkSnapshot,
        now_ms: i64,
        observations: Vec<crate::collectors::RawObservation>,
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

impl<P: NetworkProvider, C: ObservationClock> Collector for NetworkCollector<P, C> {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let (state, downtime) =
            NetworkSnapshot::hydrate(context.state.baseline(BASELINE_KIND)?, self.clock.now_ms());
        self.state = AckedState::new(state);
        self.downtime = downtime;
        self.downtime_clear_token = None;
        self.wake_tier = self
            .provider
            .register_wake(context.wake.clone(), context.cancel.clone());
        self.context = Some(context);
        Ok(())
    }

    fn stop(&mut self) {
        self.provider.unregister_wake();
        self.context = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<crate::collectors::RawObservation>> {
        Err(AppError::InvalidInput {
            what: "network collector",
            reason: "a stateful collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        let Some(cancel) = self.context.as_ref().map(|held| held.cancel.clone()) else {
            return Ok(CollectorSnapshot::Noop);
        };
        let now_ms = self.clock.now_ms();
        let Some(current) = self.query_with_deadline(&cancel)? else {
            return Ok(CollectorSnapshot::Noop);
        };
        if cancel.is_cancelled() {
            return Ok(CollectorSnapshot::Noop);
        }
        if !self.state.committed().established {
            return self.stage(
                context.token(),
                NetworkSnapshot {
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
            let next = observations::for_change(change, observed_at, uncertain);
            if !capacity.spend(next.len(), 0, 0) {
                break;
            }
            diff::apply(&mut acknowledged, change);
            observations.extend(next);
            drained += 1;
        }
        let fully_drained = drained == changes.len();
        self.stage(
            context.token(),
            NetworkSnapshot {
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

pub fn catalog_descriptor() -> AppResult<CollectorDescriptor> {
    crate::collectors::registry::CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == ID)
        .map(|entry| entry.descriptor.clone())
        .ok_or_else(|| AppError::InvalidInput {
            what: "collector catalog",
            reason: "the network collector is not in the canonical catalog".to_owned(),
        })
}

fn timeout_error() -> AppError {
    AppError::Platform {
        operation: "enumerate network configuration",
        reason: "the fifteen-second sweep deadline elapsed".to_owned(),
    }
}
