//! Bounded checkpoint capture coordination for the existing collector workers.

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::collectors::AutomaticCheckpointTrigger;
use crate::domain::CollectorCost;

use super::{lock, signal, EntryState, Shared};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorCaptureStatus {
    Complete,
    Disabled,
    Unavailable,
    Paused,
    Failed,
    TimedOutWaitingForWorker,
    TimedOutDuringCycle,
}

/// Durable-cycle metadata consumed by checkpoint creation.
///
/// `Complete` means the existing worker received its durable acknowledgement,
/// so checkpoint creation may read that collector's durable baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorCaptureResult {
    pub collector_id: String,
    pub display_name: String,
    pub cost: CollectorCost,
    pub status: CollectorCaptureStatus,
    pub requested_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub events_committed: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorCaptureManifest {
    pub requested_at: i64,
    pub completed_at: i64,
    pub collectors: Vec<CollectorCaptureResult>,
}

impl CollectorCaptureManifest {
    pub fn is_partial(&self) -> bool {
        self.collectors
            .iter()
            .any(|result| result.status != CollectorCaptureStatus::Complete)
    }
}

/// Receives automatic requests on the runtime dispatcher, not a collector worker.
pub trait AutomaticCheckpointSink: Send + Sync + 'static {
    fn checkpoint_requested(&self, trigger: AutomaticCheckpointTrigger);
}

#[derive(Debug, Default)]
pub(super) struct SilentAutomaticCheckpointSink;

impl AutomaticCheckpointSink for SilentAutomaticCheckpointSink {
    fn checkpoint_requested(&self, _trigger: AutomaticCheckpointTrigger) {}
}

#[derive(Debug)]
pub(super) struct CaptureSlot {
    flight: Mutex<Option<Arc<CaptureFlight>>>,
}

impl CaptureSlot {
    pub(super) fn new() -> Self {
        Self {
            flight: Mutex::new(None),
        }
    }

    fn request(&self, requested_at: i64) -> Arc<CaptureFlight> {
        let mut slot = lock(&self.flight);
        if let Some(flight) = slot.as_ref() {
            if !flight.is_finished() {
                return Arc::clone(flight);
            }
        }
        let flight = Arc::new(CaptureFlight::new(requested_at));
        *slot = Some(Arc::clone(&flight));
        flight
    }

    pub(super) fn has_queued(&self) -> bool {
        lock(&self.flight)
            .as_ref()
            .is_some_and(|flight| flight.is_queued())
    }

    pub(super) fn start_queued(&self, started_at: i64) -> Option<Arc<CaptureFlight>> {
        let flight = lock(&self.flight).as_ref().cloned()?;
        flight.start(started_at).then_some(flight)
    }

    /// Fail whichever capture flight is active, including provider work that
    /// has already started. The provider may finish later, but shutdown
    /// callers must not remain tied to its timeout or the number of workers.
    pub(super) fn fail_active(&self, completed_at: i64, error: String) {
        if let Some(flight) = lock(&self.flight).as_ref().cloned() {
            flight.finish(completed_at, Err(error));
        }
    }
}

#[derive(Debug)]
pub(super) struct CaptureFlight {
    requested_at: i64,
    requested_at_instant: Instant,
    state: Mutex<FlightState>,
    changed: Condvar,
}

#[derive(Debug)]
enum FlightState {
    Queued,
    Running {
        started_at: i64,
    },
    Finished {
        started_at: i64,
        completed_at: i64,
        outcome: Result<u64, String>,
    },
}

impl CaptureFlight {
    fn new(requested_at: i64) -> Self {
        Self {
            requested_at,
            requested_at_instant: Instant::now(),
            state: Mutex::new(FlightState::Queued),
            changed: Condvar::new(),
        }
    }

    fn is_queued(&self) -> bool {
        matches!(*lock(&self.state), FlightState::Queued)
    }

    fn is_finished(&self) -> bool {
        matches!(*lock(&self.state), FlightState::Finished { .. })
    }

    fn start(&self, started_at: i64) -> bool {
        let mut state = lock(&self.state);
        if !matches!(*state, FlightState::Queued) {
            return false;
        }
        *state = FlightState::Running { started_at };
        self.changed.notify_all();
        true
    }

    pub(super) fn finish(&self, completed_at: i64, outcome: Result<u64, String>) {
        let mut state = lock(&self.state);
        let started_at = match *state {
            FlightState::Running { started_at, .. } => started_at,
            FlightState::Queued => completed_at,
            FlightState::Finished { .. } => return,
        };
        *state = FlightState::Finished {
            started_at,
            completed_at,
            outcome,
        };
        self.changed.notify_all();
    }

    fn wait(&self, entry: &EntryState, timeout: Duration) -> CollectorCaptureResult {
        let deadline = self.requested_at_instant + timeout;
        let mut state = lock(&self.state);
        loop {
            match &*state {
                FlightState::Queued => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return result(
                            entry,
                            self.requested_at,
                            CollectorCaptureStatus::TimedOutWaitingForWorker,
                            None,
                            None,
                            0,
                            Some("collector worker did not begin the requested cycle".to_owned()),
                        );
                    }
                    state = self
                        .changed
                        .wait_timeout(state, remaining)
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .0;
                }
                FlightState::Running { started_at } => {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        return result(
                            entry,
                            self.requested_at,
                            CollectorCaptureStatus::TimedOutDuringCycle,
                            Some(*started_at),
                            None,
                            0,
                            Some("collector cycle exceeded its capture timeout".to_owned()),
                        );
                    }
                    state = self
                        .changed
                        .wait_timeout(state, remaining)
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .0;
                }
                FlightState::Finished {
                    started_at,
                    completed_at,
                    outcome,
                } => {
                    let (status, events, error) = match outcome {
                        Ok(events) => (CollectorCaptureStatus::Complete, *events, None),
                        Err(error) => (CollectorCaptureStatus::Failed, 0, Some(error.clone())),
                    };
                    let mut captured = result(
                        entry,
                        self.requested_at,
                        status,
                        Some(*started_at),
                        Some(*completed_at),
                        events,
                        error,
                    );
                    captured.duration_ms = Some(completed_at.saturating_sub(*started_at).max(0));
                    return captured;
                }
            }
        }
    }
}

fn result(
    entry: &EntryState,
    requested_at: i64,
    status: CollectorCaptureStatus,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    events_committed: u64,
    error: Option<String>,
) -> CollectorCaptureResult {
    CollectorCaptureResult {
        collector_id: entry.id().to_owned(),
        display_name: entry.descriptor.display_name.to_owned(),
        cost: entry.descriptor.cost_hint,
        status,
        requested_at,
        started_at,
        completed_at,
        duration_ms: None,
        events_committed,
        error,
    }
}

pub(super) fn request_all(
    shared: &Arc<Shared>,
    cycle_timeout: Duration,
) -> CollectorCaptureManifest {
    request_all_with_progress(shared, cycle_timeout, |_| {})
}

/// Describe the latest durable collector state without scheduling provider work.
///
/// Checkpoint comparisons use this snapshot so a read-only comparison never
/// turns into an eight-second OS sweep. A collector is current only after a
/// successful acknowledged cycle and while no later failure is outstanding.
pub(super) fn current_manifest(shared: &Arc<Shared>) -> CollectorCaptureManifest {
    let observed_at = shared.clock.now_ms();
    let collectors = shared
        .entries
        .iter()
        .map(|entry| {
            if !entry.is_enabled() {
                return result(
                    entry,
                    observed_at,
                    CollectorCaptureStatus::Disabled,
                    None,
                    Some(observed_at),
                    0,
                    None,
                );
            }
            if !entry.is_available() {
                return result(
                    entry,
                    observed_at,
                    CollectorCaptureStatus::Unavailable,
                    None,
                    Some(observed_at),
                    0,
                    None,
                );
            }

            let live = lock(&entry.live);
            let last_run_at = live.last_run_at;
            let duration_ms = live.last_duration_ms;
            let successful = last_run_at.is_some()
                && entry
                    .consecutive_failures
                    .load(std::sync::atomic::Ordering::Acquire)
                    == 0;
            let status = if successful {
                CollectorCaptureStatus::Complete
            } else {
                CollectorCaptureStatus::Failed
            };
            CollectorCaptureResult {
                collector_id: entry.id().to_owned(),
                display_name: entry.descriptor.display_name.to_owned(),
                cost: entry.descriptor.cost_hint,
                status,
                requested_at: observed_at,
                started_at: last_run_at.map(|completed| {
                    completed.saturating_sub(duration_ms.unwrap_or_default().max(0))
                }),
                completed_at: last_run_at,
                duration_ms,
                events_committed: 0,
                error: (!successful).then(|| {
                    "collector has no current successfully acknowledged durable state".to_owned()
                }),
            }
        })
        .collect();
    CollectorCaptureManifest {
        requested_at: observed_at,
        completed_at: observed_at,
        collectors,
    }
}

pub(super) fn request_all_with_progress(
    shared: &Arc<Shared>,
    cycle_timeout: Duration,
    mut progress: impl FnMut(&CollectorCaptureResult),
) -> CollectorCaptureManifest {
    let requested_at = shared.clock.now_ms();
    let mut pending = Vec::with_capacity(shared.entries.len());
    for entry in &shared.entries {
        let immediate = if !entry.is_enabled() {
            Some(CollectorCaptureStatus::Disabled)
        } else if !entry.is_available() {
            Some(CollectorCaptureStatus::Unavailable)
        } else if shared.monitor.is_paused() {
            Some(CollectorCaptureStatus::Paused)
        } else if shared.stop.load(std::sync::atomic::Ordering::Acquire)
            || entry
                .consecutive_failures
                .load(std::sync::atomic::Ordering::Acquire)
                >= 5
        {
            Some(CollectorCaptureStatus::Failed)
        } else {
            None
        };
        if let Some(status) = immediate {
            pending.push((
                Arc::clone(entry),
                None,
                Some(result(
                    entry,
                    requested_at,
                    status,
                    None,
                    Some(shared.clock.now_ms()),
                    0,
                    (status == CollectorCaptureStatus::Failed)
                        .then(|| "collector is not able to run a capture cycle".to_owned()),
                )),
            ));
        } else {
            pending.push((
                Arc::clone(entry),
                Some(entry.capture.request(requested_at)),
                None,
            ));
        }
    }
    signal(shared);

    let collectors = pending
        .into_iter()
        .map(|(entry, flight, immediate)| {
            let captured = if let Some(immediate) = immediate {
                immediate
            } else if let Some(flight) = flight {
                flight.wait(&entry, cycle_timeout)
            } else {
                result(
                    &entry,
                    requested_at,
                    CollectorCaptureStatus::Failed,
                    None,
                    Some(shared.clock.now_ms()),
                    0,
                    Some("collector capture request was not scheduled".to_owned()),
                )
            };
            progress(&captured);
            captured
        })
        .collect();
    CollectorCaptureManifest {
        requested_at,
        completed_at: shared.clock.now_ms(),
        collectors,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::*;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use crate::app::Monitoring;
    use crate::collectors::registry::{CollectorRegistry, RegistryEntry};
    use crate::collectors::schedule::QueryBusy;
    use crate::collectors::{
        AutomaticCheckpointHandle, Collector, CollectorContext, CollectorSnapshot, RawBaseline,
        SnapshotContext,
    };
    use crate::domain::{CollectorCadence, CollectorDescriptor, MonitoringProfile};
    use crate::error::{AppError, AppResult};
    use crate::storage::{baseline_repository, CollectorCommitReceipt, Db};

    use super::super::CollectorSupervisor;

    fn entry() -> EntryState {
        EntryState::from_registry(
            &RegistryEntry::active(
                CollectorDescriptor {
                    id: "scripted",
                    display_name: "Scripted",
                    what_it_watches: "tests",
                    why_it_matters: "tests",
                    cost_hint: CollectorCost::Cheap,
                    cadence: CollectorCadence::Poll(Duration::from_secs(60)),
                    requires_admin: false,
                },
                || panic!("not used"),
            ),
            true,
            Duration::ZERO,
        )
    }

    #[test]
    fn duplicate_requests_share_one_flight() {
        let slot = CaptureSlot::new();
        let first = slot.request(10);
        let second = slot.request(11);
        assert!(Arc::ptr_eq(&first, &second));
        assert!(slot.start_queued(12).is_some());
        first.finish(13, Ok(2));
        assert!(!Arc::ptr_eq(&first, &slot.request(14)));
    }

    #[test]
    fn one_absolute_budget_includes_queue_and_provider_time() {
        // Put the request deadline in the past without racing a worker thread
        // against a 25 ms scheduler window. A deadline reset on start would
        // make the running wait last two seconds and fail the bound below.
        let timeout = Duration::from_secs(2);
        let mut flight = CaptureFlight::new(10);
        flight.requested_at_instant = Instant::now() - Duration::from_secs(3);
        assert!(flight.start(20));
        let began = Instant::now();
        let captured = flight.wait(&entry(), timeout);
        assert_eq!(captured.status, CollectorCaptureStatus::TimedOutDuringCycle);
        assert!(began.elapsed() < Duration::from_secs(1));

        let mut queued = CaptureFlight::new(30);
        queued.requested_at_instant = Instant::now() - Duration::from_secs(3);
        let began = Instant::now();
        let captured = queued.wait(&entry(), timeout);
        assert_eq!(
            captured.status,
            CollectorCaptureStatus::TimedOutWaitingForWorker
        );
        assert!(began.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn shutdown_failure_releases_queued_and_running_capture_waiters_immediately() {
        for running in [false, true] {
            let slot = Arc::new(CaptureSlot::new());
            let flight = slot.request(10);
            if running {
                assert!(slot.start_queued(11).is_some());
            }
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            let waiter = std::thread::spawn(move || {
                sender
                    .send(flight.wait(&entry(), Duration::from_secs(30)))
                    .unwrap();
            });
            let began = Instant::now();
            slot.fail_active(12, "collector runtime stopped".to_owned());
            let result = receiver
                .recv_timeout(Duration::from_millis(100))
                .expect("capture waiter is released by shutdown");
            waiter.join().unwrap();
            assert_eq!(result.status, CollectorCaptureStatus::Failed);
            assert!(began.elapsed() < Duration::from_millis(100));
        }
    }

    #[test]
    fn automatic_trigger_route_drops_instead_of_blocking_when_full() {
        let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
        let handle =
            AutomaticCheckpointHandle::new(move |trigger| sender.try_send(trigger).is_ok());
        assert!(handle.try_request(AutomaticCheckpointTrigger::InstallerProcessStarted));
        assert!(!handle.try_request(AutomaticCheckpointTrigger::InstallerProcessStarted));
    }

    struct AtomicTestCollector {
        acknowledged: Arc<AtomicBool>,
    }

    impl Collector for AtomicTestCollector {
        fn id(&self) -> &'static str {
            "scripted"
        }

        fn descriptor(&self) -> CollectorDescriptor {
            entry().descriptor
        }

        fn start(&mut self, _ctx: CollectorContext) -> AppResult<()> {
            Ok(())
        }

        fn stop(&mut self) {}

        fn snapshot(&mut self) -> AppResult<Vec<crate::collectors::RawObservation>> {
            Err(AppError::InvalidInput {
                what: "scripted collector",
                reason: "atomic test uses snapshot_batch".to_owned(),
            })
        }

        fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
            Ok(CollectorSnapshot::Atomic {
                token: context.token(),
                observations: Vec::new(),
                relationships: Vec::new(),
                baseline: Some(RawBaseline {
                    entity_kind: "scripted_state".to_owned(),
                    snapshot: serde_json::json!({"ready": true}),
                    captured_at: crate::domain::now_ms(),
                }),
                config_snapshots: Vec::new(),
            })
        }

        fn commit_succeeded(
            &mut self,
            _token: crate::collectors::CommitToken,
            _receipt: &CollectorCommitReceipt,
        ) -> AppResult<()> {
            self.acknowledged.store(true, Ordering::Release);
            Ok(())
        }
    }

    #[test]
    fn supervisor_capture_reuses_worker_and_waits_for_durable_ack() {
        let directory = crate::testing::temp_dir("collector-runtime-capture");
        let db = Arc::new(Db::open_in(&directory).unwrap());
        let factories = Arc::new(AtomicUsize::new(0));
        let acknowledged = Arc::new(AtomicBool::new(false));
        let factory_count = Arc::clone(&factories);
        let factory_ack = Arc::clone(&acknowledged);
        let registry =
            CollectorRegistry::new(vec![RegistryEntry::active(entry().descriptor, move || {
                factory_count.fetch_add(1, Ordering::AcqRel);
                Box::new(AtomicTestCollector {
                    acknowledged: Arc::clone(&factory_ack),
                })
            })]);
        let supervisor = CollectorSupervisor::new(
            registry,
            Arc::clone(&db),
            Arc::new(Monitoring::running()),
            QueryBusy::default(),
            &BTreeMap::new(),
            MonitoringProfile::Balanced,
        );
        supervisor.spawn_all().unwrap();

        let manifest = supervisor.capture_enabled(Duration::from_secs(1));
        supervisor.stop_all();

        assert_eq!(manifest.collectors.len(), 1);
        assert_eq!(
            manifest.collectors[0].status,
            CollectorCaptureStatus::Complete
        );
        assert!(acknowledged.load(Ordering::Acquire));
        assert_eq!(factories.load(Ordering::Acquire), 1);
        let baseline = db
            .read(|connection| baseline_repository::load(connection, "scripted", "scripted_state"))
            .unwrap();
        assert!(baseline.is_some());
    }
}
