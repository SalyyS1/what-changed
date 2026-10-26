//! Scheduled-task and startup-entry collection with an atomic durable baseline.

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
use crate::error::{AppError, AppResult};
use crate::storage::CollectorCommitReceipt;

pub use state::{
    RunEntry, ScheduledTask, StartupFile, StartupInventory, StartupLocation, TaskTriggerKind,
};

pub const ID: &str = "startup";
pub const BASELINE_KIND: &str = "startup_entries_and_tasks";
pub const SWEEP_TIMEOUT: Duration = Duration::from_secs(15);

/// Compare durable startup entities with the live collector's precedence.
/// Task execution timestamps/results are intentionally not configuration.
pub(crate) fn checkpoint_fields(
    entity_kind: &str,
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    diff::checkpoint_fields(entity_kind, before, after)
}

/// Keeps registry and Startup-folder watchers alive until collector stop.
pub trait StartupNotificationGuard: Send {}

/// Provider seam encompassing Task Scheduler, four Run keys, StartupApproved,
/// and the two Startup folders.
pub trait StartupProvider: Send + Sync + 'static {
    fn inventory(&self, cancel: &CancelToken, deadline: Instant) -> AppResult<StartupInventory>;

    fn register_notifications(
        &self,
        _wake: WakeHandle,
        _cancel: CancelToken,
    ) -> AppResult<Box<dyn StartupNotificationGuard>> {
        Err(AppError::Platform {
            operation: "watch startup entries",
            reason: "startup notifications are unavailable".to_owned(),
        })
    }
}

/// Shared across factory-created instances so a timed-out Task Scheduler COM
/// call cannot overlap with its replacement.
#[derive(Clone, Default)]
pub struct StartupQueryCoordinator {
    busy: Arc<AtomicBool>,
}

impl StartupQueryCoordinator {
    fn try_begin(&self) -> bool {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn finish(&self) {
        self.busy.store(false, Ordering::Release);
    }
}

pub struct StartupCollector<P, C = SystemObservationClock> {
    provider: Arc<P>,
    clock: C,
    descriptor: CollectorDescriptor,
    state: AckedState<state::StartupState>,
    context: Option<CollectorContext>,
    notifications: Option<Box<dyn StartupNotificationGuard>>,
    downtime: bool,
    downtime_clear_token: Option<CommitToken>,
    query_coordinator: StartupQueryCoordinator,
}

impl<P: StartupProvider> StartupCollector<P, SystemObservationClock> {
    pub fn new(provider: P, descriptor: CollectorDescriptor) -> Self {
        Self::with_clock(provider, descriptor, SystemObservationClock)
    }

    pub fn with_query_coordinator(
        provider: P,
        descriptor: CollectorDescriptor,
        query_coordinator: StartupQueryCoordinator,
    ) -> Self {
        Self::build(
            provider,
            descriptor,
            SystemObservationClock,
            query_coordinator,
        )
    }
}

impl<P: StartupProvider, C: ObservationClock> StartupCollector<P, C> {
    pub fn with_clock(provider: P, descriptor: CollectorDescriptor, clock: C) -> Self {
        Self::build(
            provider,
            descriptor,
            clock,
            StartupQueryCoordinator::default(),
        )
    }

    pub fn with_clock_and_query_coordinator(
        provider: P,
        descriptor: CollectorDescriptor,
        clock: C,
        query_coordinator: StartupQueryCoordinator,
    ) -> Self {
        Self::build(provider, descriptor, clock, query_coordinator)
    }

    fn build(
        provider: P,
        descriptor: CollectorDescriptor,
        clock: C,
        query_coordinator: StartupQueryCoordinator,
    ) -> Self {
        Self {
            provider: Arc::new(provider),
            clock,
            descriptor,
            state: AckedState::new(state::StartupState::default()),
            context: None,
            notifications: None,
            downtime: false,
            downtime_clear_token: None,
            query_coordinator,
        }
    }

    fn query(&self, cancel: &CancelToken) -> AppResult<Option<StartupInventory>> {
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
            .name("startup-task-query".to_owned())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    provider.inventory(&worker_cancel, deadline)
                }))
                .unwrap_or_else(|_| {
                    Err(AppError::Platform {
                        operation: "enumerate startup entries",
                        reason: "the startup provider panicked".to_owned(),
                    })
                });
                coordinator.finish();
                let _ = sender.send(result);
            })
        {
            self.query_coordinator.finish();
            return Err(AppError::Platform {
                operation: "start a startup enumeration",
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
                        operation: "enumerate startup entries",
                        reason: "the startup query worker stopped without a result".to_owned(),
                    });
                }
            }
        }
    }

    fn stage(
        &mut self,
        token: CommitToken,
        next: state::StartupState,
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

impl<P: StartupProvider, C: ObservationClock> Collector for StartupCollector<P, C> {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let (state, downtime) = state::StartupState::hydrate(
            context.state.baseline(BASELINE_KIND)?,
            self.clock.now_ms(),
        );
        self.state = AckedState::new(state);
        self.downtime = downtime;
        self.downtime_clear_token = None;
        self.notifications = self
            .provider
            .register_notifications(context.wake.clone(), context.cancel.clone())
            .map_err(|error| {
                tracing::debug!(reason = %error, "startup notifications unavailable; using hourly sweep");
                error
            })
            .ok();
        self.context = Some(context);
        Ok(())
    }

    fn stop(&mut self) {
        self.notifications = None;
        self.context = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Err(AppError::InvalidInput {
            what: "startup collector",
            reason: "a stateful collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        let Some(cancel) = self.context.as_ref().map(|held| held.cancel.clone()) else {
            return Ok(CollectorSnapshot::Noop);
        };
        let now_ms = self.clock.now_ms();
        let Some(inventory) = self.query(&cancel)? else {
            return Ok(CollectorSnapshot::Noop);
        };
        if cancel.is_cancelled() {
            return Ok(CollectorSnapshot::Noop);
        }
        let task_store_available = inventory.task_store_available;
        let mut current = state::IndexedInventory::from(state::sanitize(inventory));
        if !task_store_available {
            // Retain unseen tasks to suppress false removals, but overlay every
            // readable row so additions and changes in accessible folders are
            // still reported while another folder is unavailable.
            let mut merged = self.state.committed().acknowledged.tasks.clone();
            merged.append(&mut current.tasks);
            current.tasks = merged;
        }
        if !self.state.committed().established {
            return self.stage(
                context.token(),
                state::StartupState {
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
            state::StartupState {
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
            reason: "the startup collector is not in the canonical catalog".to_owned(),
        })
}

fn timeout_error() -> AppError {
    AppError::Platform {
        operation: "enumerate startup entries",
        reason: "the fifteen-second sweep deadline elapsed".to_owned(),
    }
}
