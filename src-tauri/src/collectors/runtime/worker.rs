//! One serial collector worker and its persistence acknowledgement protocol.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use super::policy::{
    cadence_supports_targeted_wake, request_targeted_snapshot, ExpensiveSnapshotLease,
};
use super::{
    lock, notify_collector, profile, read_lock, set_live_state, wait_after, wake_epoch, write_lock,
    EntryState, Shared,
};
use crate::collectors::normalize::Normalizer;
use crate::collectors::{
    AutomaticCheckpointHandle, CancelToken, Collector, CollectorContext, CollectorSnapshot,
    CommitToken, EmitHandle, MonitorHandle, RawBaseline, RawConfigSnapshotMutation, RawObservation,
    RawRelationshipSpec, SnapshotContext, StateReadHandle, StatusDetailsHandle, WakeHandle,
};
use crate::domain::CollectorCost;
use crate::dto::CollectorState;
use crate::error::{AppError, AppResult, DbError};
use crate::privacy::Sanitizer;
use crate::storage::CollectorCommitReceipt;

const FAILURE_LIMIT: u64 = 5;

pub(super) fn run(shared: Arc<Shared>, state: Arc<EntryState>) {
    let Some(factory) = state.factory.clone() else {
        return;
    };
    let mut collector: Option<Box<dyn Collector>> = None;
    let mut started = false;
    let mut instance_cancel: Option<CancelToken> = None;
    let mut active_generation = state.generation.load(Ordering::Acquire);
    let mut active_monitoring_generation = shared.monitoring_generation.load(Ordering::Acquire);
    let mut active_profile_generation = shared.profile_generation.load(Ordering::Acquire);
    let mut next_due = shared.clock.now() + state.initial_stagger;
    while !shared.stop.load(Ordering::Acquire) {
        let observed_wake = wake_epoch(&shared);
        // Lifecycle writers publish state and generation under the emission
        // write gate. Sample them under the matching read gate so an enable or
        // resume cannot pair its new state with a stale schedule generation.
        let (enabled, paused, generation, monitoring_generation, profile_generation) = {
            let _gate = read_lock(&shared.emission_gate);
            (
                state.is_enabled(),
                shared.monitor.is_paused(),
                state.generation.load(Ordering::Acquire),
                shared.monitoring_generation.load(Ordering::Acquire),
                shared.profile_generation.load(Ordering::Acquire),
            )
        };
        let collector_changed = active_generation != generation;
        let monitoring_changed = active_monitoring_generation != monitoring_generation;
        let profile_changed = active_profile_generation != profile_generation;
        if collector_changed || monitoring_changed || profile_changed {
            if collector_changed || monitoring_changed {
                discard_instance(
                    &shared,
                    &state,
                    &mut collector,
                    &mut started,
                    &mut instance_cancel,
                    DiscardKind::Orderly,
                );
            }
            active_generation = generation;
            active_monitoring_generation = monitoring_generation;
            active_profile_generation = profile_generation;
            next_due = shared.clock.now()
                + transition_stagger(state.descriptor.cost_hint, state.initial_stagger);
        }
        if !enabled {
            state.capture.fail_active(
                shared.clock.now_ms(),
                "collector was disabled before the requested cycle began".to_owned(),
            );
            discard_instance(
                &shared,
                &state,
                &mut collector,
                &mut started,
                &mut instance_cancel,
                DiscardKind::Orderly,
            );
            wait_after(&shared, Duration::from_secs(60), observed_wake);
            continue;
        }
        if paused {
            state.capture.fail_active(
                shared.clock.now_ms(),
                "monitoring was paused before the requested cycle began".to_owned(),
            );
            set_live_state(&shared, &state, CollectorState::Paused);
            wait_after(&shared, Duration::from_secs(60), observed_wake);
            continue;
        }
        if state.consecutive_failures.load(Ordering::Relaxed) >= FAILURE_LIMIT {
            state.capture.fail_active(
                shared.clock.now_ms(),
                "collector failure limit prevents the requested cycle".to_owned(),
            );
            set_live_state(&shared, &state, CollectorState::Failed);
            wait_after(&shared, Duration::from_secs(60), observed_wake);
            continue;
        }
        let now = shared.clock.now();
        if state.capture.has_queued()
            || (state.consecutive_failures.load(Ordering::Acquire) == 0
                && cadence_supports_targeted_wake(state.descriptor.cadence)
                && state.targeted_wake_pending.load(Ordering::Acquire))
        {
            next_due = now;
        }
        if now < next_due {
            wait_after(
                &shared,
                next_due.saturating_duration_since(now),
                observed_wake,
            );
            continue;
        }
        if state.descriptor.cost_hint == CollectorCost::Expensive && shared.query_busy.is_busy() {
            next_due = now + Duration::from_secs(1);
            wait_after(&shared, Duration::from_secs(1), observed_wake);
            continue;
        }
        let Some(snapshot_lease) =
            ExpensiveSnapshotLease::try_acquire(&shared, state.descriptor.cost_hint)
        else {
            let retry_after = Duration::from_millis(25);
            // Contention delays execution; it does not change the collector's
            // cadence deadline. Keeping the original due time also makes the
            // wall-clock retry valid with an injected, manually advanced clock.
            wait_after(&shared, retry_after, observed_wake);
            continue;
        };
        let mut created_instance = false;
        if collector.is_none() {
            match catch_unwind(AssertUnwindSafe(|| factory())) {
                Ok(instance) => {
                    collector = Some(instance);
                    created_instance = true;
                }
                Err(_) => {
                    drop(snapshot_lease);
                    record_failure(&shared, &state, "collector factory panicked", None);
                    let failures = state.consecutive_failures.load(Ordering::Relaxed);
                    next_due = shared.clock.now() + shared.backoff.delay(failures);
                    continue;
                }
            }
        }
        if created_instance {
            let Some(cancel) = refresh_instance_cancel(
                &shared,
                &state,
                generation,
                monitoring_generation,
                profile_generation,
            ) else {
                drop_unstarted_collector(&mut collector);
                drop(snapshot_lease);
                continue;
            };
            instance_cancel = Some(cancel);
        }
        let Some(context_cancel) = instance_cancel.clone() else {
            drop_unstarted_collector(&mut collector);
            drop(snapshot_lease);
            record_failure(
                &shared,
                &state,
                "collector instance has no owned cancellation token",
                None,
            );
            continue;
        };
        let instance_generation = state.instance_generation.load(Ordering::Acquire);
        let context = collector_context(
            &shared,
            &state,
            generation,
            monitoring_generation,
            instance_generation,
            context_cancel,
        );
        if cadence_supports_targeted_wake(state.descriptor.cadence) {
            // Consume only at the provider boundary. A notification arriving
            // after this swap remains pending and earns one trailing cycle.
            state.targeted_wake_pending.swap(false, Ordering::AcqRel);
        }
        let token = CommitToken::mint(shared.next_commit_token.fetch_add(1, Ordering::AcqRel));
        set_live_state(&shared, &state, CollectorState::Running);
        let capture_flight = state.capture.start_queued(shared.clock.now_ms());
        let began = shared.clock.now();
        let provider_result = catch_unwind(AssertUnwindSafe(|| -> AppResult<CollectorSnapshot> {
            let current = collector.as_mut().ok_or_else(|| AppError::InvalidInput {
                what: "collector",
                reason: "collector factory returned no instance".to_owned(),
            })?;
            if current.id() != state.id() || current.descriptor().id != state.id() {
                return Err(AppError::InvalidInput {
                    what: "collector",
                    reason: "collector factory returned the wrong id".to_owned(),
                });
            }
            if !started {
                current.start(context)?;
                started = true;
            }
            current.snapshot_batch(SnapshotContext::new(token))
        }));
        let duration = shared.clock.now().saturating_duration_since(began);
        // Provider serialization ends here. SQLite admission and acknowledgement
        // never retain the global Expensive lease.
        drop(snapshot_lease);

        let capture_outcome = match provider_result {
            Ok(Ok(snapshot)) => handle_snapshot(
                &shared,
                &state,
                &mut collector,
                &mut started,
                &mut instance_cancel,
                generation,
                monitoring_generation,
                instance_generation,
                token,
                duration,
                snapshot,
            ),
            Ok(Err(error)) => {
                discard_instance(
                    &shared,
                    &state,
                    &mut collector,
                    &mut started,
                    &mut instance_cancel,
                    DiscardKind::Orderly,
                );
                if !is_inactive_emission(&error) {
                    record_failure(&shared, &state, &error.to_string(), Some(duration));
                }
                Err(sanitize_capture_error(&error.to_string()))
            }
            Err(_) => {
                discard_instance(
                    &shared,
                    &state,
                    &mut collector,
                    &mut started,
                    &mut instance_cancel,
                    DiscardKind::Poisoned,
                );
                record_failure(&shared, &state, "collector panicked", Some(duration));
                Err("collector panicked".to_owned())
            }
        };
        if let Some(flight) = capture_flight {
            flight.finish(shared.clock.now_ms(), capture_outcome);
        }

        let failures = state.consecutive_failures.load(Ordering::Relaxed);
        if failures >= FAILURE_LIMIT {
            set_live_state(&shared, &state, CollectorState::Failed);
            wait_after(&shared, Duration::from_secs(60), wake_epoch(&shared));
            continue;
        }
        let interval = super::schedule::collector_interval(
            state.descriptor.cadence,
            state.descriptor.cost_hint,
            profile(&shared),
        );
        next_due = shared.clock.now()
            + if failures == 0 {
                interval
            } else {
                shared.backoff.delay(failures)
            };
    }
    // Shutdown publishes its stop flag while holding the emission write gate.
    // Join that same fenced teardown path so the instance token is cancelled
    // before provider-owned `stop`, even if this worker observed the flag
    // before the supervisor reached this entry in its cancellation loop.
    discard_instance(
        &shared,
        &state,
        &mut collector,
        &mut started,
        &mut instance_cancel,
        DiscardKind::Orderly,
    );
    state.capture.fail_active(
        shared.clock.now_ms(),
        "collector runtime stopped before the requested cycle began".to_owned(),
    );
}

#[allow(clippy::too_many_arguments)]
fn handle_snapshot(
    shared: &Shared,
    state: &EntryState,
    collector: &mut Option<Box<dyn Collector>>,
    started: &mut bool,
    instance_cancel: &mut Option<CancelToken>,
    generation: u64,
    monitoring_generation: u64,
    instance_generation: u64,
    expected_token: CommitToken,
    duration: Duration,
    snapshot: CollectorSnapshot,
) -> Result<u64, String> {
    match snapshot {
        CollectorSnapshot::Legacy(observations) => {
            if emit_observations(
                shared,
                state,
                generation,
                monitoring_generation,
                instance_generation,
                observations,
            )
            .is_ok()
            {
                record_success(shared, state, duration, 0);
                // Legacy notification collectors (currently Event Log) have
                // no structural state rows. Their checkpoint contribution is
                // therefore a valid empty set once the drain succeeds.
                Ok(0)
            } else {
                Err("collector observations were not admitted for persistence".to_owned())
            }
        }
        CollectorSnapshot::Noop => {
            record_success(shared, state, duration, 0);
            Ok(0)
        }
        CollectorSnapshot::Atomic {
            token,
            observations,
            relationships,
            baseline,
            config_snapshots,
        } => {
            let Some(current) = collector.as_mut() else {
                record_failure(
                    shared,
                    state,
                    "collector instance disappeared before persistence",
                    Some(duration),
                );
                *started = false;
                return Err("collector instance disappeared before persistence".to_owned());
            };
            let outcome = persist_atomic(
                shared,
                state,
                current.as_mut(),
                generation,
                monitoring_generation,
                instance_generation,
                expected_token,
                token,
                observations,
                relationships,
                baseline,
                config_snapshots,
            );
            handle_atomic_outcome(
                shared,
                state,
                collector,
                started,
                instance_cancel,
                duration,
                outcome,
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn persist_atomic(
    shared: &Shared,
    state: &EntryState,
    collector: &mut dyn Collector,
    generation: u64,
    monitoring_generation: u64,
    instance_generation: u64,
    expected_token: CommitToken,
    returned_token: CommitToken,
    observations: Vec<RawObservation>,
    relationships: Vec<RawRelationshipSpec>,
    baseline: Option<RawBaseline>,
    config_snapshots: Vec<RawConfigSnapshotMutation>,
) -> AtomicOutcome {
    if returned_token != expected_token {
        let error = AppError::InvalidInput {
            what: "collector atomic snapshot",
            reason: "the returned commit token does not match this snapshot cycle".to_owned(),
        };
        return AtomicOutcome::Failed {
            error,
            requested_events: observations.len() as u64,
            hook_problem: call_failed_hook(collector, expected_token),
        };
    }

    let commit = match Normalizer::new().prepare_atomic_with_config(
        state.id(),
        observations,
        relationships,
        baseline,
        config_snapshots,
    ) {
        Ok(commit) => commit,
        Err(error) => {
            return AtomicOutcome::Failed {
                error,
                requested_events: 0,
                hook_problem: call_failed_hook(collector, expected_token),
            };
        }
    };
    let requested_events = commit.event_count() as u64;
    let admission = {
        let _gate = read_lock(&shared.emission_gate);
        if context_is_inactive(
            shared,
            state,
            generation,
            monitoring_generation,
            instance_generation,
        ) {
            None
        } else {
            Some(shared.db.submit_collector(commit))
        }
    };
    let Some(admission) = admission else {
        return AtomicOutcome::Inactive {
            hook_problem: call_failed_hook(collector, expected_token),
        };
    };
    let ticket = match admission {
        Ok(ticket) => ticket,
        Err(error) => {
            return AtomicOutcome::Failed {
                error,
                requested_events,
                hook_problem: call_failed_hook(collector, expected_token),
            };
        }
    };

    // The emission gate is out of scope before observers run. Instrumentation
    // therefore cannot extend the lifecycle fence across SQLite work.
    let sink = Arc::clone(&super::lock(&shared.status_sink));
    let _ = catch_unwind(AssertUnwindSafe(|| {
        sink.collector_commit_admitted(state.id());
    }));

    // Admission is irrevocable. Always wait and acknowledge its one durable
    // outcome even if pause, disable or shutdown happens while SQLite works.
    match ticket.wait() {
        Ok(receipt) => match call_success_hook(collector, expected_token, &receipt) {
            None => AtomicOutcome::Succeeded {
                event_count: receipt.event_ids.len() as u64,
            },
            Some(hook_problem) => AtomicOutcome::DurableHookFailed {
                event_count: receipt.event_ids.len() as u64,
                hook_problem,
            },
        },
        Err(error) => AtomicOutcome::Failed {
            error,
            requested_events,
            hook_problem: call_failed_hook(collector, expected_token),
        },
    }
}

fn handle_atomic_outcome(
    shared: &Shared,
    state: &EntryState,
    collector: &mut Option<Box<dyn Collector>>,
    started: &mut bool,
    instance_cancel: &mut Option<CancelToken>,
    duration: Duration,
    outcome: AtomicOutcome,
) -> Result<u64, String> {
    match outcome {
        AtomicOutcome::Succeeded { event_count } => {
            record_success(shared, state, duration, event_count);
            Ok(event_count)
        }
        AtomicOutcome::Inactive { hook_problem } => {
            if let Some(problem) = hook_problem {
                log_secondary_hook_problem(state.id(), "commit_failed", &problem);
                discard_instance(
                    shared,
                    state,
                    collector,
                    started,
                    instance_cancel,
                    problem.discard_kind(),
                );
            }
            Err("collector cycle became inactive before persistence".to_owned())
        }
        AtomicOutcome::Failed {
            error,
            requested_events,
            hook_problem,
        } => {
            if matches!(&error, AppError::Db(DbError::QueueFull)) {
                state
                    .queue_drops
                    .fetch_add(requested_events, Ordering::Relaxed);
            }
            if let Some(problem) = hook_problem {
                log_secondary_hook_problem(state.id(), "commit_failed", &problem);
                discard_instance(
                    shared,
                    state,
                    collector,
                    started,
                    instance_cancel,
                    problem.discard_kind(),
                );
            }
            record_failure(shared, state, &error.to_string(), Some(duration));
            Err(sanitize_capture_error(&error.to_string()))
        }
        AtomicOutcome::DurableHookFailed {
            event_count,
            hook_problem,
        } => {
            log_secondary_hook_problem(state.id(), "commit_succeeded", &hook_problem);
            let reason = hook_problem.reason.clone();
            discard_instance(
                shared,
                state,
                collector,
                started,
                instance_cancel,
                hook_problem.discard_kind(),
            );
            record_hook_failure(shared, state, &reason, duration, event_count);
            Err(sanitize_capture_error(&reason))
        }
    }
}

fn sanitize_capture_error(reason: &str) -> String {
    Sanitizer::new().sanitize(reason.to_owned()).0.get().clone()
}

fn call_success_hook(
    collector: &mut dyn Collector,
    token: CommitToken,
    receipt: &CollectorCommitReceipt,
) -> Option<HookProblem> {
    match catch_unwind(AssertUnwindSafe(|| {
        collector.commit_succeeded(token, receipt)
    })) {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(HookProblem::returned(error.to_string())),
        Err(_) => Some(HookProblem::panicked("collector success hook panicked")),
    }
}

fn call_failed_hook(collector: &mut dyn Collector, token: CommitToken) -> Option<HookProblem> {
    match catch_unwind(AssertUnwindSafe(|| collector.commit_failed(token))) {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(HookProblem::returned(error.to_string())),
        Err(_) => Some(HookProblem::panicked("collector failure hook panicked")),
    }
}

enum AtomicOutcome {
    Succeeded {
        event_count: u64,
    },
    Inactive {
        hook_problem: Option<HookProblem>,
    },
    Failed {
        error: AppError,
        requested_events: u64,
        hook_problem: Option<HookProblem>,
    },
    DurableHookFailed {
        event_count: u64,
        hook_problem: HookProblem,
    },
}

struct HookProblem {
    reason: String,
    panicked: bool,
}

impl HookProblem {
    fn returned(reason: String) -> Self {
        Self {
            reason,
            panicked: false,
        }
    }

    fn panicked(reason: &'static str) -> Self {
        Self {
            reason: reason.to_owned(),
            panicked: true,
        }
    }

    const fn discard_kind(&self) -> DiscardKind {
        if self.panicked {
            DiscardKind::Poisoned
        } else {
            DiscardKind::Orderly
        }
    }
}

fn log_secondary_hook_problem(collector_id: &str, hook: &'static str, problem: &HookProblem) {
    let (reason, _) = Sanitizer::new().sanitize(problem.reason.clone());
    tracing::warn!(
        collector = collector_id,
        hook,
        reason = %reason.get(),
        "collector acknowledgement hook failed"
    );
}

fn record_success(shared: &Shared, state: &EntryState, duration: Duration, event_count: u64) {
    let last_run_at = shared.clock.now_ms();
    let last_duration_ms = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
    let mut live = lock(&state.live);
    state
        .events_emitted
        .fetch_add(event_count, Ordering::Relaxed);
    state.consecutive_failures.store(0, Ordering::Relaxed);
    live.state = CollectorState::Idle;
    live.last_run_at = Some(last_run_at);
    live.last_duration_ms = Some(last_duration_ms);
    live.last_error = None;
    drop(live);
    observe_outcome(shared, state.id(), true);
    notify_collector(shared, state.id());
}

fn observe_outcome(shared: &Shared, collector_id: &str, success: bool) {
    let sink = Arc::clone(&lock(&shared.status_sink));
    let _ = catch_unwind(AssertUnwindSafe(|| {
        sink.collector_outcome_observed(collector_id, success);
    }));
}

fn collector_context(
    shared: &Arc<Shared>,
    state: &Arc<EntryState>,
    generation: u64,
    monitoring_generation: u64,
    instance_generation: u64,
    cancel: CancelToken,
) -> CollectorContext {
    let emit_state = Arc::clone(state);
    let emit_shared = Arc::clone(shared);
    let emit = EmitHandle::new(move |raw| {
        emit_observations(
            &emit_shared,
            &emit_state,
            generation,
            monitoring_generation,
            instance_generation,
            raw,
        )
    });
    let wake_state = Arc::clone(state);
    let wake_shared = Arc::clone(shared);
    let wake = WakeHandle::new(move || {
        request_targeted_snapshot(
            &wake_shared,
            &wake_state,
            generation,
            monitoring_generation,
            instance_generation,
        )
    });
    let status_state = Arc::clone(state);
    let status_shared = Arc::clone(shared);
    let status_details = StatusDetailsHandle::new(move |details| {
        let _gate = read_lock(&status_shared.emission_gate);
        if context_is_inactive(
            &status_shared,
            &status_state,
            generation,
            monitoring_generation,
            instance_generation,
        ) {
            return Err(inactive_emission_error());
        }
        *lock(&status_state.status_details) = details;
        notify_collector(&status_shared, status_state.id());
        Ok(())
    });
    let checkpoint_state = Arc::clone(state);
    let checkpoint_shared = Arc::clone(shared);
    let automatic_checkpoint = AutomaticCheckpointHandle::new(move |trigger| {
        if context_is_inactive(
            &checkpoint_shared,
            &checkpoint_state,
            generation,
            monitoring_generation,
            instance_generation,
        ) {
            return false;
        }
        checkpoint_shared
            .automatic_checkpoint_tx
            .try_send(trigger)
            .is_ok()
    });
    CollectorContext {
        monitor: MonitorHandle(Arc::clone(&shared.monitor)),
        emit,
        wake,
        status_details,
        automatic_checkpoint,
        state: StateReadHandle::new(Arc::clone(&shared.db), state.id()),
        cancel,
    }
}

fn emit_observations(
    shared: &Shared,
    state: &EntryState,
    generation: u64,
    monitoring_generation: u64,
    instance_generation: u64,
    raw: Vec<RawObservation>,
) -> AppResult<()> {
    let _gate = read_lock(&shared.emission_gate);
    if context_is_inactive(
        shared,
        state,
        generation,
        monitoring_generation,
        instance_generation,
    ) {
        return Err(inactive_emission_error());
    }
    if raw.is_empty() {
        return Ok(());
    }
    if raw.iter().any(|item| item.collector_id != state.id()) {
        return Err(AppError::InvalidInput {
            what: "collector observation",
            reason: "collector id does not match its worker".to_owned(),
        });
    }
    let count = raw.len() as u64;
    match shared
        .db
        .enqueue_events(Normalizer::new().normalize_all(raw))
    {
        Ok(()) => {
            state.events_emitted.fetch_add(count, Ordering::Relaxed);
            notify_collector(shared, state.id());
            Ok(())
        }
        Err(error) => {
            if matches!(&error, AppError::Db(DbError::QueueFull)) {
                state.queue_drops.fetch_add(count, Ordering::Relaxed);
            }
            record_delivery_failure(shared, state, &error.to_string());
            Err(error)
        }
    }
}

pub(super) fn context_is_inactive(
    shared: &Shared,
    state: &EntryState,
    generation: u64,
    monitoring_generation: u64,
    instance_generation: u64,
) -> bool {
    shared.stop.load(Ordering::Acquire)
        || !state.is_enabled()
        || shared.monitor.is_paused()
        || state.generation.load(Ordering::Acquire) != generation
        || shared.monitoring_generation.load(Ordering::Acquire) != monitoring_generation
        || state.instance_generation.load(Ordering::Acquire) != instance_generation
}

fn record_delivery_failure(shared: &Shared, state: &EntryState, reason: &str) {
    let (reason, _) = Sanitizer::new().sanitize(reason.to_owned());
    let last_run_at = shared.clock.now_ms();
    let mut live = lock(&state.live);
    live.state = CollectorState::Degraded;
    live.last_error = Some(reason.get().clone());
    live.last_run_at = Some(last_run_at);
    live.last_duration_ms = None;
    drop(live);
    observe_outcome(shared, state.id(), false);
    notify_collector(shared, state.id());
}

fn record_failure(shared: &Shared, state: &EntryState, reason: &str, duration: Option<Duration>) {
    let (reason, _) = Sanitizer::new().sanitize(reason.to_owned());
    let last_run_at = shared.clock.now_ms();
    let last_duration_ms =
        duration.map(|value| i64::try_from(value.as_millis()).unwrap_or(i64::MAX));
    let mut live = lock(&state.live);
    let failures = state.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
    live.state = if failures >= FAILURE_LIMIT {
        CollectorState::Failed
    } else {
        CollectorState::Idle
    };
    live.last_error = Some(reason.get().clone());
    live.last_run_at = Some(last_run_at);
    live.last_duration_ms = last_duration_ms;
    drop(live);
    observe_outcome(shared, state.id(), false);
    notify_collector(shared, state.id());
}

fn record_hook_failure(
    shared: &Shared,
    state: &EntryState,
    reason: &str,
    duration: Duration,
    event_count: u64,
) {
    let (reason, _) = Sanitizer::new().sanitize(reason.to_owned());
    let last_run_at = shared.clock.now_ms();
    let last_duration_ms = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
    let mut live = lock(&state.live);
    state
        .events_emitted
        .fetch_add(event_count, Ordering::Relaxed);
    let failures = state.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
    live.state = if failures >= FAILURE_LIMIT {
        CollectorState::Failed
    } else {
        CollectorState::Degraded
    };
    live.last_error = Some(reason.get().clone());
    live.last_run_at = Some(last_run_at);
    live.last_duration_ms = Some(last_duration_ms);
    drop(live);
    observe_outcome(shared, state.id(), false);
    notify_collector(shared, state.id());
}

#[derive(Clone, Copy)]
enum DiscardKind {
    Orderly,
    Poisoned,
}

fn discard_instance(
    shared: &Shared,
    state: &EntryState,
    collector: &mut Option<Box<dyn Collector>>,
    started: &mut bool,
    instance_cancel: &mut Option<CancelToken>,
    kind: DiscardKind,
) {
    if collector.is_none() && instance_cancel.is_none() {
        *started = false;
        return;
    }
    // Fence callbacks and cancel provider loops before collector-owned cleanup
    // runs. Once the write gate releases, no old instance can act on whatever
    // replacement this worker creates after backoff.
    invalidate_instance(shared, state, instance_cancel);
    match kind {
        DiscardKind::Orderly => stop_collector(collector),
        DiscardKind::Poisoned => drop_poisoned_collector(collector),
    }
    *started = false;
    *instance_cancel = None;
}

fn invalidate_instance(shared: &Shared, state: &EntryState, instance_cancel: &Option<CancelToken>) {
    let _gate = write_lock(&shared.emission_gate);
    state.instance_generation.fetch_add(1, Ordering::AcqRel);
    state.targeted_wake_pending.store(false, Ordering::Release);
    if let Some(cancel) = instance_cancel {
        cancel.cancel();
    }
    lock(&state.cancel).cancel();
}

/// Install a token only after the prior instance completed contained teardown.
/// A provider can therefore never inherit an uncancelled token while an old
/// watcher is still stopping.
fn refresh_instance_cancel(
    shared: &Shared,
    state: &EntryState,
    generation: u64,
    monitoring_generation: u64,
    profile_generation: u64,
) -> Option<CancelToken> {
    let _gate = write_lock(&shared.emission_gate);
    if shared.stop.load(Ordering::Acquire)
        || !state.is_enabled()
        || shared.monitor.is_paused()
        || state.generation.load(Ordering::Acquire) != generation
        || shared.monitoring_generation.load(Ordering::Acquire) != monitoring_generation
        || shared.profile_generation.load(Ordering::Acquire) != profile_generation
    {
        return None;
    }
    let cancel = CancelToken::default();
    *lock(&state.cancel) = cancel.clone();
    Some(cancel)
}

fn stop_collector(collector: &mut Option<Box<dyn Collector>>) {
    let Some(mut collector) = collector.take() else {
        return;
    };
    let _ = catch_unwind(AssertUnwindSafe(|| collector.stop()));
    // `Drop` is collector-owned code too. Keep it within a separate panic
    // boundary even when `stop` already failed.
    let _ = catch_unwind(AssertUnwindSafe(|| drop(collector)));
}

fn drop_poisoned_collector(collector: &mut Option<Box<dyn Collector>>) {
    let poisoned = collector.take();
    // A destructor is collector-owned code too. Contain a second panic without
    // invoking any method on the instance that crossed the panic boundary.
    let _ = catch_unwind(AssertUnwindSafe(|| drop(poisoned)));
}

fn drop_unstarted_collector(collector: &mut Option<Box<dyn Collector>>) {
    let unstarted = collector.take();
    let _ = catch_unwind(AssertUnwindSafe(|| drop(unstarted)));
}

fn transition_stagger(cost: CollectorCost, assigned: Duration) -> Duration {
    if cost == CollectorCost::Expensive {
        assigned
    } else {
        Duration::ZERO
    }
}

fn inactive_emission_error() -> AppError {
    AppError::InvalidInput {
        what: "collector emission",
        reason: "collector context is no longer active".to_owned(),
    }
}

fn is_inactive_emission(error: &AppError) -> bool {
    matches!(
        error,
        AppError::InvalidInput {
            what: "collector emission",
            ..
        }
    )
}
