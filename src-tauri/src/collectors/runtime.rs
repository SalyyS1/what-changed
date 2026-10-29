//! One blocking worker per active collector, with isolation and restart policy.

mod capture;
mod policy;
mod worker;

pub use capture::{
    AutomaticCheckpointSink, CollectorCaptureManifest, CollectorCaptureResult,
    CollectorCaptureStatus,
};
pub use policy::{
    BackoffPolicy, CollectorRuntimeConfig, ExponentialBackoff, RuntimeClock, RuntimeStatusSink,
    SystemClock,
};

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::app::Monitoring;
use crate::collectors::registry::{CollectorFactory, CollectorRegistry, RegistryEntry};
use crate::collectors::schedule::{self, QueryBusy};
use crate::collectors::{AutomaticCheckpointTrigger, CancelToken};
use crate::domain::{CollectorCost, MonitoringProfile};
use crate::dto::{CollectorState, CollectorStatusDto, MonitoringDto};
use crate::error::{AppError, AppResult};
use crate::storage::Db;

use capture::{CaptureSlot, SilentAutomaticCheckpointSink};
use policy::SilentStatusSink;

const AUTOMATIC_CHECKPOINT_QUEUE_CAPACITY: usize = 16;

#[derive(Debug)]
struct LiveStatus {
    state: CollectorState,
    last_run_at: Option<i64>,
    last_duration_ms: Option<i64>,
    last_error: Option<String>,
}

struct EntryState {
    descriptor: crate::domain::CollectorDescriptor,
    factory: Option<CollectorFactory>,
    enabled: AtomicBool,
    live: Mutex<LiveStatus>,
    status_details: Mutex<Option<crate::domain::CollectorStatusDetails>>,
    events_emitted: AtomicU64,
    consecutive_failures: AtomicU64,
    queue_drops: AtomicU64,
    generation: AtomicU64,
    instance_generation: AtomicU64,
    targeted_wake_pending: AtomicBool,
    capture: CaptureSlot,
    cancel: Mutex<CancelToken>,
    initial_stagger: Duration,
}

impl EntryState {
    fn from_registry(entry: &RegistryEntry, enabled: bool, initial_stagger: Duration) -> Self {
        let factory = if entry.descriptor.requires_admin {
            None
        } else {
            entry.factory.clone()
        };
        let available = factory.is_some();
        Self {
            descriptor: entry.descriptor.clone(),
            // The MVP runtime has no elevation capability. A descriptor that
            // asks for it is visible but mechanically cannot be started.
            factory,
            enabled: AtomicBool::new(enabled),
            live: Mutex::new(LiveStatus {
                state: configured_state(enabled, available),
                last_run_at: None,
                last_duration_ms: None,
                last_error: None,
            }),
            status_details: Mutex::new(None),
            events_emitted: AtomicU64::new(0),
            consecutive_failures: AtomicU64::new(0),
            queue_drops: AtomicU64::new(0),
            generation: AtomicU64::new(0),
            instance_generation: AtomicU64::new(0),
            targeted_wake_pending: AtomicBool::new(false),
            capture: CaptureSlot::new(),
            cancel: Mutex::new(CancelToken::default()),
            initial_stagger,
        }
    }

    fn id(&self) -> &'static str {
        self.descriptor.id
    }

    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    fn is_available(&self) -> bool {
        self.factory.is_some()
    }
}

struct CollectorHandle {
    state: Arc<EntryState>,
    thread: Option<JoinHandle<()>>,
}

struct Shared {
    db: Arc<Db>,
    monitor: Arc<Monitoring>,
    query_busy: QueryBusy,
    profile: AtomicU8,
    profile_generation: AtomicU64,
    stop: AtomicBool,
    wake: (Mutex<u64>, Condvar),
    emission_gate: RwLock<()>,
    monitoring_generation: AtomicU64,
    expensive_snapshot_active: AtomicBool,
    next_commit_token: AtomicU64,
    entries: Vec<Arc<EntryState>>,
    status_sink: Mutex<Arc<dyn RuntimeStatusSink>>,
    automatic_checkpoint_tx: SyncSender<AutomaticCheckpointTrigger>,
    automatic_checkpoint_sink: Mutex<Arc<dyn AutomaticCheckpointSink>>,
    last_monitoring: Mutex<MonitoringDto>,
    clock: Arc<dyn RuntimeClock>,
    backoff: Arc<dyn BackoffPolicy>,
}

/// Owns every collector worker and its health counters.
pub struct CollectorSupervisor {
    shared: Arc<Shared>,
    handles: Mutex<Vec<CollectorHandle>>,
    watchdog: Mutex<Option<JoinHandle<()>>>,
    automatic_checkpoint_rx: Mutex<Option<Receiver<AutomaticCheckpointTrigger>>>,
    automatic_checkpoint_dispatcher: Mutex<Option<JoinHandle<()>>>,
    shutdown_incomplete: AtomicBool,
}

/// Cloneable checkpoint-facing view of the collector runtime.
#[derive(Clone)]
pub struct CollectorCaptureHandle {
    shared: Arc<Shared>,
}

impl CollectorCaptureHandle {
    /// Force one cycle on every enabled and available existing worker.
    ///
    /// `cycle_timeout` begins when that worker enters provider code; time spent
    /// behind the global expensive-collector lease is tracked separately.
    pub fn capture_enabled(&self, cycle_timeout: Duration) -> CollectorCaptureManifest {
        capture::request_all(&self.shared, cycle_timeout)
    }

    /// Progress is reported after each collector reaches a terminal capture
    /// status. The callback runs on the caller, never on collector workers.
    pub fn capture_enabled_with_progress(
        &self,
        cycle_timeout: Duration,
        progress: impl FnMut(&CollectorCaptureResult),
    ) -> CollectorCaptureManifest {
        capture::request_all_with_progress(&self.shared, cycle_timeout, progress)
    }

    /// Read the latest worker health/durable-cycle status without waking any
    /// collector. Used by checkpoint compare-to-now latency-sensitive reads.
    pub fn current_manifest(&self) -> CollectorCaptureManifest {
        capture::current_manifest(&self.shared)
    }
}

/// Narrow callback retained by the platform sleep/resume monitor.
///
/// It can only request the collectors whose contracts require an
/// immediate post-resume sweep; it cannot change settings or emit rows.
#[derive(Clone)]
pub struct ResumeWakeHandle {
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl ResumeWakeHandle {
    pub fn request(&self) {
        (self.wake)();
    }
}

impl CollectorSupervisor {
    pub fn new(
        registry: CollectorRegistry,
        db: Arc<Db>,
        monitor: Arc<Monitoring>,
        query_busy: QueryBusy,
        enabled: &BTreeMap<String, bool>,
        profile: MonitoringProfile,
    ) -> Self {
        Self::with_runtime(
            registry,
            CollectorRuntimeConfig {
                db,
                monitor,
                query_busy,
                enabled: enabled.clone(),
                profile,
                clock: Arc::new(SystemClock),
                backoff: Arc::new(ExponentialBackoff),
            },
        )
    }

    pub fn with_runtime(registry: CollectorRegistry, config: CollectorRuntimeConfig) -> Self {
        let (automatic_checkpoint_tx, automatic_checkpoint_rx) =
            sync_channel(AUTOMATIC_CHECKPOINT_QUEUE_CAPACITY);
        let expensive_ids: Vec<&str> = registry
            .entries()
            .iter()
            .filter(|entry| entry.descriptor.cost_hint == CollectorCost::Expensive)
            .map(|entry| entry.descriptor.id)
            .collect();
        let staggers = schedule::expensive_staggers(&expensive_ids);
        let entries: Vec<Arc<EntryState>> = registry
            .entries()
            .iter()
            .map(|entry| {
                Arc::new(EntryState::from_registry(
                    entry,
                    config
                        .enabled
                        .get(entry.descriptor.id)
                        .copied()
                        .unwrap_or(true),
                    staggers
                        .get(entry.descriptor.id)
                        .copied()
                        .unwrap_or(Duration::ZERO),
                ))
            })
            .collect();
        let monitoring = config.monitor.snapshot();
        let shared = Arc::new(Shared {
            db: config.db,
            monitor: config.monitor,
            query_busy: config.query_busy,
            profile: AtomicU8::new(profile_code(config.profile)),
            profile_generation: AtomicU64::new(0),
            stop: AtomicBool::new(false),
            wake: (Mutex::new(0), Condvar::new()),
            emission_gate: RwLock::new(()),
            monitoring_generation: AtomicU64::new(0),
            expensive_snapshot_active: AtomicBool::new(false),
            next_commit_token: AtomicU64::new(1),
            entries: entries.clone(),
            status_sink: Mutex::new(Arc::new(SilentStatusSink)),
            automatic_checkpoint_tx,
            automatic_checkpoint_sink: Mutex::new(Arc::new(SilentAutomaticCheckpointSink)),
            last_monitoring: Mutex::new(monitoring),
            clock: config.clock,
            backoff: config.backoff,
        });
        let handles = entries
            .into_iter()
            .map(|state| CollectorHandle {
                state,
                thread: None,
            })
            .collect();
        Self {
            shared,
            handles: Mutex::new(handles),
            watchdog: Mutex::new(None),
            automatic_checkpoint_rx: Mutex::new(Some(automatic_checkpoint_rx)),
            automatic_checkpoint_dispatcher: Mutex::new(None),
            shutdown_incomplete: AtomicBool::new(false),
        }
    }

    /// Start every implemented collector once. Calling twice is harmless.
    pub fn spawn_all(&self) -> AppResult<()> {
        self.spawn_monitoring_watchdog()?;
        self.spawn_automatic_checkpoint_dispatcher()?;
        let mut handles = lock(&self.handles);
        for handle in handles.iter_mut() {
            if handle.thread.is_some() || handle.state.factory.is_none() {
                continue;
            }
            let shared = Arc::clone(&self.shared);
            let state = Arc::clone(&handle.state);
            let name = format!("collector-{}", state.id());
            let thread = std::thread::Builder::new()
                .name(name)
                .stack_size(256 * 1024)
                .spawn(move || worker::run(shared, state))
                .map_err(|source| AppError::File {
                    operation: "start collector thread",
                    path: std::path::PathBuf::from(handle.state.id()),
                    source,
                })?;
            handle.thread = Some(thread);
        }
        Ok(())
    }

    fn spawn_automatic_checkpoint_dispatcher(&self) -> AppResult<()> {
        let mut dispatcher = lock(&self.automatic_checkpoint_dispatcher);
        if dispatcher.is_some() {
            return Ok(());
        }
        let Some(receiver) = lock(&self.automatic_checkpoint_rx).take() else {
            return Ok(());
        };
        let shared = Arc::clone(&self.shared);
        let thread = std::thread::Builder::new()
            .name("collector-checkpoint-dispatch".to_owned())
            .stack_size(128 * 1024)
            .spawn(move || automatic_checkpoint_dispatcher(shared, receiver))
            .map_err(|source| AppError::File {
                operation: "start checkpoint trigger dispatcher",
                path: std::path::PathBuf::from("automatic-checkpoint"),
                source,
            })?;
        *dispatcher = Some(thread);
        Ok(())
    }

    fn spawn_monitoring_watchdog(&self) -> AppResult<()> {
        let mut watchdog = lock(&self.watchdog);
        if watchdog.is_some() {
            return Ok(());
        }
        let shared = Arc::clone(&self.shared);
        let thread = std::thread::Builder::new()
            .name("collector-monitoring-watchdog".to_owned())
            .stack_size(128 * 1024)
            .spawn(move || monitoring_watchdog(shared))
            .map_err(|source| AppError::File {
                operation: "start monitoring watchdog",
                path: std::path::PathBuf::from("monitoring"),
                source,
            })?;
        *watchdog = Some(thread);
        Ok(())
    }

    /// Replace the notification adapter. Call before spawning collectors so
    /// no initial transition can race past the frontend/tray bridge.
    pub fn attach_status_sink(&self, sink: Arc<dyn RuntimeStatusSink>) {
        *lock(&self.shared.status_sink) = sink;
    }

    /// Replace the automatic-checkpoint consumer. Delivery remains bounded and
    /// happens on the runtime dispatcher, never on a collector worker.
    pub fn attach_automatic_checkpoint_sink(&self, sink: Arc<dyn AutomaticCheckpointSink>) {
        *lock(&self.shared.automatic_checkpoint_sink) = sink;
    }

    pub fn capture_handle(&self) -> CollectorCaptureHandle {
        CollectorCaptureHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    pub fn capture_enabled(&self, cycle_timeout: Duration) -> CollectorCaptureManifest {
        self.capture_handle().capture_enabled(cycle_timeout)
    }

    pub fn list_collector_status(&self) -> Vec<CollectorStatusDto> {
        let failed_row_counts = self.shared.db.failed_commit_rows();
        let rejected_cycle_counts = self.shared.db.collector_cycle_admission_drops();
        let failed_cycle_counts = self.shared.db.failed_collector_cycles();
        let paused = self.shared.monitor.is_paused();
        lock(&self.handles)
            .iter()
            .map(|handle| {
                let live = lock(&handle.state.live);
                let failed_commit_rows = failed_row_counts
                    .get(handle.state.id())
                    .copied()
                    .unwrap_or(0);
                let queue_drops = handle.state.queue_drops.load(Ordering::Relaxed);
                let cycle_admission_drops = rejected_cycle_counts
                    .get(handle.state.id())
                    .copied()
                    .unwrap_or(0);
                let failed_cycles = failed_cycle_counts
                    .get(handle.state.id())
                    .copied()
                    .unwrap_or(0);
                let health = HealthTelemetry {
                    queue_drops,
                    failed_commit_rows,
                    cycle_admission_drops,
                    failed_cycles,
                };
                let enabled = handle.state.is_enabled();
                let details = lock(&handle.state.status_details).clone();
                let visible = if enabled
                    && details.as_ref().is_some_and(|details| {
                        details.docker_state
                            == Some(crate::domain::DockerCollectorState::Unavailable)
                    }) {
                    CollectorState::Unavailable
                } else {
                    visible_state(
                        live.state,
                        enabled,
                        handle.state.is_available(),
                        paused,
                        health,
                    )
                };
                CollectorStatusDto {
                    descriptor: (&handle.state.descriptor).into(),
                    id: handle.state.id().to_owned(),
                    enabled,
                    state: visible,
                    last_run_at: live.last_run_at,
                    last_duration_ms: live.last_duration_ms,
                    events_emitted: handle.state.events_emitted.load(Ordering::Relaxed),
                    consecutive_failures: handle.state.consecutive_failures.load(Ordering::Relaxed),
                    last_error: live.last_error.clone(),
                    details,
                    requires_admin: handle.state.descriptor.requires_admin,
                    queue_drops,
                    failed_commit_rows,
                    cycle_admission_drops,
                    failed_cycles,
                }
            })
            .collect()
    }

    pub fn resume_wake_handle(&self) -> ResumeWakeHandle {
        let shared = Arc::clone(&self.shared);
        ResumeWakeHandle {
            wake: Arc::new(move || {
                for id in ["updates", "drivers", "network"] {
                    request_named_snapshot(&shared, id);
                }
            }),
        }
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> AppResult<()> {
        let Some(state) = self.shared.entries.iter().find(|state| state.id() == id) else {
            return Err(AppError::InvalidInput {
                what: "collector",
                reason: "unknown collector id".to_owned(),
            });
        };
        {
            let _gate = write_lock(&self.shared.emission_gate);
            if state.is_enabled() == enabled {
                return Ok(());
            }
            state.targeted_wake_pending.store(false, Ordering::Release);
            state.generation.fetch_add(1, Ordering::AcqRel);
            lock(&state.cancel).cancel();
            let mut live = lock(&state.live);
            state.enabled.store(enabled, Ordering::Release);
            if enabled {
                state.consecutive_failures.store(0, Ordering::Relaxed);
            }
            live.state = configured_state(enabled, state.is_available());
        }
        signal(&self.shared);
        notify_collector(&self.shared, id);
        Ok(())
    }

    /// Wake one notification-backed collector after its persisted inputs change.
    ///
    /// This is the command-side counterpart of [`crate::collectors::WakeHandle`].
    /// A watched-root add/remove has already committed before this is called;
    /// the collector reads the new root set on the targeted cycle. Disabled or
    /// paused collectors simply pick it up on their next ordinary start.
    pub fn notify_source_changed(&self, id: &str) -> AppResult<()> {
        let Some(state) = self.shared.entries.iter().find(|state| state.id() == id) else {
            return Err(AppError::InvalidInput {
                what: "collector",
                reason: "unknown collector id".to_owned(),
            });
        };
        request_entry_snapshot(&self.shared, state);
        Ok(())
    }

    /// Apply a persisted collector-input change under the same lifecycle fence
    /// used for event admission. Any snapshot that read the old input set is
    /// invalidated before the closure runs, so it cannot commit after (for
    /// example) a watched root and its baselines have been removed.
    pub fn apply_source_change<T>(
        &self,
        id: &str,
        change: impl FnOnce() -> AppResult<T>,
    ) -> AppResult<T> {
        self.apply_sources_change(&[id], change)
    }

    /// Invalidate all consumers of one persisted input under a single gate.
    /// Project roots feed both branch monitoring and file/process evidence;
    /// separate calls would admit a stale batch between the two changes.
    pub fn apply_sources_change<T>(
        &self,
        ids: &[&str],
        change: impl FnOnce() -> AppResult<T>,
    ) -> AppResult<T> {
        self.validate_enabled(ids.iter().copied())?;
        let states: Vec<_> = self
            .shared
            .entries
            .iter()
            .filter(|state| ids.contains(&state.id()))
            .collect();
        let result = {
            let _gate = write_lock(&self.shared.emission_gate);
            for state in &states {
                state.targeted_wake_pending.store(false, Ordering::Release);
                state.generation.fetch_add(1, Ordering::AcqRel);
                lock(&state.cancel).cancel();
            }
            change()
        };
        signal(&self.shared);
        for state in states {
            notify_collector(&self.shared, state.id());
        }
        result
    }

    /// Reject a settings update before persistence, making the subsequent
    /// runtime application infallible under the app's config mutex.
    pub fn validate_enabled<'a>(&self, ids: impl IntoIterator<Item = &'a str>) -> AppResult<()> {
        for id in ids {
            if !self.shared.entries.iter().any(|state| state.id() == id) {
                return Err(AppError::InvalidInput {
                    what: "collector",
                    reason: format!("unknown collector id: {id}"),
                });
            }
        }
        Ok(())
    }

    pub fn set_profile(&self, profile: MonitoringProfile) {
        let code = profile_code(profile);
        let changed = {
            let _gate = write_lock(&self.shared.emission_gate);
            if self.shared.profile.swap(code, Ordering::AcqRel) == code {
                false
            } else {
                self.shared
                    .profile_generation
                    .fetch_add(1, Ordering::AcqRel);
                true
            }
        };
        if !changed {
            return;
        }
        signal(&self.shared);
        for state in &self.shared.entries {
            notify_collector(&self.shared, state.id());
        }
    }

    pub fn profile(&self) -> MonitoringProfile {
        profile(&self.shared)
    }

    /// Change the global monitoring gate and wake sleeping collectors.
    ///
    /// Mutating `Monitoring` alone cannot interrupt a worker already blocked
    /// on the scheduler condition variable. Keeping the mutation and wake in
    /// one supervisor operation makes pause and resume take effect promptly,
    /// even when the collector cadence is long.
    pub fn set_monitoring_paused(&self, paused: bool, minutes: Option<u32>) {
        {
            let _gate = write_lock(&self.shared.emission_gate);
            self.shared.monitor.set_paused(paused, minutes);
            fence_monitoring_transition(&self.shared);
        }
        announce_monitoring_transition(&self.shared);
    }

    /// Exact-duration variant used by the watchdog contract test.
    pub fn pause_monitoring_for(&self, duration: Duration) {
        {
            let _gate = write_lock(&self.shared.emission_gate);
            self.shared.monitor.pause_for(duration);
            fence_monitoring_transition(&self.shared);
        }
        announce_monitoring_transition(&self.shared);
    }

    /// Sticky evidence of a detached or panicked runtime thread. Inspect after
    /// stop_all returns; repeated shutdown calls cannot erase a missed join.
    pub fn shutdown_incomplete(&self) -> bool {
        self.shutdown_incomplete.load(Ordering::Acquire)
    }

    pub fn stop_all(&self) {
        let deadline = Instant::now() + Duration::from_secs(2);
        {
            let _gate = write_lock(&self.shared.emission_gate);
            if self.shared.stop.swap(true, Ordering::AcqRel) {
                return;
            }
            self.shared
                .monitoring_generation
                .fetch_add(1, Ordering::AcqRel);
            for state in &self.shared.entries {
                state.targeted_wake_pending.store(false, Ordering::Release);
                state.generation.fetch_add(1, Ordering::AcqRel);
                lock(&state.cancel).cancel();
                state.capture.fail_active(
                    self.shared.clock.now_ms(),
                    "collector runtime stopped during checkpoint capture".to_owned(),
                );
            }
        }
        signal(&self.shared);
        for handle in lock(&self.handles).iter_mut() {
            if let Some(thread) = handle.thread.take() {
                if !join_before_deadline(thread, deadline, handle.state.id()) {
                    self.shutdown_incomplete.store(true, Ordering::Release);
                }
            }
        }
        if let Some(thread) = lock(&self.watchdog).take() {
            if !join_before_deadline(thread, deadline, "monitoring-watchdog") {
                self.shutdown_incomplete.store(true, Ordering::Release);
            }
        }
        if let Some(thread) = lock(&self.automatic_checkpoint_dispatcher).take() {
            if !join_before_deadline(thread, deadline, "automatic-checkpoint-dispatcher") {
                self.shutdown_incomplete.store(true, Ordering::Release);
            }
        }
    }
}

fn automatic_checkpoint_dispatcher(
    shared: Arc<Shared>,
    receiver: Receiver<AutomaticCheckpointTrigger>,
) {
    while !shared.stop.load(Ordering::Acquire) {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(trigger) => {
                let sink = Arc::clone(&lock(&shared.automatic_checkpoint_sink));
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    sink.checkpoint_requested(trigger);
                }));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
}

fn request_named_snapshot(shared: &Shared, id: &str) {
    if let Some(state) = shared.entries.iter().find(|state| state.id() == id) {
        request_entry_snapshot(shared, state);
    }
}

fn request_entry_snapshot(shared: &Shared, state: &EntryState) {
    let should_signal = {
        let _gate = read_lock(&shared.emission_gate);
        if shared.stop.load(Ordering::Acquire)
            || !state.is_enabled()
            || !state.is_available()
            || shared.monitor.is_paused()
            || !policy::cadence_supports_targeted_wake(state.descriptor.cadence)
        {
            false
        } else {
            !state.targeted_wake_pending.swap(true, Ordering::AcqRel)
        }
    };
    if should_signal {
        signal(shared);
    }
}

impl Drop for CollectorSupervisor {
    fn drop(&mut self) {
        self.stop_all();
    }
}

fn configured_state(enabled: bool, available: bool) -> CollectorState {
    if !available {
        CollectorState::Unavailable
    } else if enabled {
        CollectorState::Idle
    } else {
        CollectorState::Disabled
    }
}

fn visible_state(
    live: CollectorState,
    enabled: bool,
    available: bool,
    paused: bool,
    health: HealthTelemetry,
) -> CollectorState {
    if !available {
        return CollectorState::Unavailable;
    }
    if !enabled {
        return CollectorState::Disabled;
    }
    if paused && live != CollectorState::Failed {
        return CollectorState::Paused;
    }
    if health.has_loss() && !matches!(live, CollectorState::Failed | CollectorState::Disabled) {
        return CollectorState::Degraded;
    }
    live
}

#[derive(Clone, Copy)]
struct HealthTelemetry {
    queue_drops: u64,
    failed_commit_rows: u64,
    cycle_admission_drops: u64,
    failed_cycles: u64,
}

impl HealthTelemetry {
    const fn has_loss(self) -> bool {
        self.queue_drops > 0
            || self.failed_commit_rows > 0
            || self.cycle_admission_drops > 0
            || self.failed_cycles > 0
    }
}

fn monitoring_watchdog(shared: Arc<Shared>) {
    while !shared.stop.load(Ordering::Acquire) {
        let observed = wake_epoch(&shared);
        let wait_for = shared
            .monitor
            .resume_in()
            .unwrap_or(Duration::from_secs(60));
        wait_after(&shared, wait_for, observed);
        if shared.stop.load(Ordering::Acquire) {
            break;
        }
        let expired = {
            let _gate = write_lock(&shared.emission_gate);
            if shared.monitor.expire_if_due() {
                fence_monitoring_transition(&shared);
                true
            } else {
                false
            }
        };
        if expired {
            announce_monitoring_transition(&shared);
        }
    }
}

/// Invalidate every context created under the prior monitoring state. This is
/// called while the emission write gate is held, so after it returns no old
/// callback can enqueue a row.
fn fence_monitoring_transition(shared: &Shared) {
    shared.monitoring_generation.fetch_add(1, Ordering::AcqRel);
    for state in &shared.entries {
        state.targeted_wake_pending.store(false, Ordering::Release);
        lock(&state.cancel).cancel();
    }
    signal(shared);
}

fn announce_monitoring_transition(shared: &Shared) {
    let monitoring = shared.monitor.snapshot();
    let changed = {
        let mut previous = lock(&shared.last_monitoring);
        if *previous == monitoring {
            false
        } else {
            *previous = monitoring;
            true
        }
    };
    if !changed {
        return;
    }
    let sink = Arc::clone(&lock(&shared.status_sink));
    sink.monitoring_changed(monitoring);
    for state in &shared.entries {
        notify_collector(shared, state.id());
    }
}

fn notify_collector(shared: &Shared, id: &str) {
    Arc::clone(&lock(&shared.status_sink)).collector_changed(id);
}

fn set_live_state(shared: &Shared, state: &EntryState, next: CollectorState) {
    let changed = {
        let mut live = lock(&state.live);
        if live.state == next {
            false
        } else {
            live.state = next;
            true
        }
    };
    if changed {
        notify_collector(shared, state.id());
    }
}

fn join_before_deadline(thread: JoinHandle<()>, deadline: Instant, component: &str) -> bool {
    while !thread.is_finished() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            tracing::warn!(
                component,
                "runtime component did not stop within the shared two-second deadline; detaching thread"
            );
            return false;
        }
        std::thread::sleep(remaining.min(Duration::from_millis(5)));
    }
    thread.join().is_ok()
}

fn signal(shared: &Shared) {
    let mut epoch = lock(&shared.wake.0);
    *epoch = epoch.wrapping_add(1);
    shared.wake.1.notify_all();
}

fn wake_epoch(shared: &Shared) -> u64 {
    *lock(&shared.wake.0)
}

fn wait_after(shared: &Shared, duration: Duration, observed: u64) {
    let guard = lock(&shared.wake.0);
    if *guard != observed || shared.stop.load(Ordering::Acquire) {
        return;
    }
    let _unused = shared
        .wake
        .1
        .wait_timeout(guard, duration)
        .unwrap_or_else(|poisoned| poisoned.into_inner());
}

fn profile(shared: &Shared) -> MonitoringProfile {
    match shared.profile.load(Ordering::Acquire) {
        0 => MonitoringProfile::Minimal,
        2 => MonitoringProfile::Thorough,
        _ => MonitoringProfile::Balanced,
    }
}

const fn profile_code(profile: MonitoringProfile) -> u8 {
    match profile {
        MonitoringProfile::Minimal => 0,
        MonitoringProfile::Balanced => 1,
        MonitoringProfile::Thorough => 2,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn read_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_lock<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod shutdown_tests {
    #[test]
    fn shutdown_join_distinguishes_success_from_a_panicked_thread() {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        assert!(super::join_before_deadline(
            std::thread::spawn(|| {}),
            deadline,
            "successful-test"
        ));
        let panicked =
            std::thread::spawn(|| std::panic::resume_unwind(Box::new("test thread panic")));
        assert!(!super::join_before_deadline(
            panicked,
            deadline,
            "panicked-test"
        ));
    }
}
