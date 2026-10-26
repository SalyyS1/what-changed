#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Real supervisor branches for the backend diagnostic observer contract.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::{CollectorSupervisor, RuntimeStatusSink};
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::{
    Collector, CollectorContext, CollectorSnapshot, CommitToken, EmitHandle, EntityIdentity,
    ObservationKind, RawBaseline, RawEntity, RawObservation, SnapshotContext,
};
use what_changed_lib::domain::{
    Category, ClassifierContext, CollectorCadence, CollectorCost, CollectorDescriptor, Confidence,
    EventType, MonitoringProfile, Severity,
};
use what_changed_lib::dto::MonitoringDto;
use what_changed_lib::error::{AppError, AppResult};
use what_changed_lib::storage::{baseline_repository, CollectorCommitReceipt, Db};

#[derive(Default)]
struct Sink {
    successes: AtomicU64,
    failures: AtomicU64,
    panic: AtomicBool,
}
impl RuntimeStatusSink for Sink {
    fn collector_changed(&self, _: &str) {}
    fn monitoring_changed(&self, _: MonitoringDto) {}
    fn collector_outcome_observed(&self, _: &str, success: bool) {
        if success {
            self.successes.fetch_add(1, Ordering::SeqCst);
        } else {
            self.failures.fetch_add(1, Ordering::SeqCst);
        }
        assert!(
            !self.panic.load(Ordering::SeqCst),
            "observer panic is contained"
        );
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Noop,
    Legacy,
    Recover,
    HookFailure,
    BlockedHook,
    BlockedStop,
}

#[derive(Default)]
struct Control {
    calls: AtomicU64,
    hook_entered: AtomicBool,
    released: Mutex<bool>,
    wake: Condvar,
    stopped: AtomicBool,
    emit: Mutex<Option<EmitHandle>>,
}
struct Probe {
    mode: Mode,
    control: Arc<Control>,
}

fn descriptor() -> CollectorDescriptor {
    CollectorDescriptor {
        id: "soak-probe",
        display_name: "Probe",
        what_it_watches: "test",
        why_it_matters: "observer contract",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(Duration::from_secs(3600)),
        requires_admin: false,
    }
}
fn failed() -> AppError {
    AppError::Platform {
        operation: "test observer",
        reason: "deliberate failure".to_owned(),
    }
}
impl Collector for Probe {
    fn id(&self) -> &'static str {
        "soak-probe"
    }
    fn descriptor(&self) -> CollectorDescriptor {
        descriptor()
    }
    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        *self.control.emit.lock().unwrap() = Some(context.emit);
        Ok(())
    }
    fn stop(&mut self) {
        if matches!(self.mode, Mode::BlockedStop) {
            let guard = self.control.released.lock().unwrap();
            drop(
                self.control
                    .wake
                    .wait_timeout_while(guard, Duration::from_secs(8), |released| !*released)
                    .unwrap(),
            );
        }
        *self.control.emit.lock().unwrap() = None;
        self.control.stopped.store(true, Ordering::SeqCst);
    }
    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Ok(Vec::new())
    }
    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        let call = self.control.calls.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            Mode::Recover if call == 0 => Err(failed()),
            Mode::Noop | Mode::Recover | Mode::BlockedStop => Ok(CollectorSnapshot::Noop),
            Mode::Legacy => Ok(CollectorSnapshot::Legacy(Vec::new())),
            Mode::HookFailure | Mode::BlockedHook => Ok(CollectorSnapshot::Atomic {
                token: context.token(),
                observations: Vec::new(),
                relationships: Vec::new(),
                baseline: Some(RawBaseline {
                    entity_kind: "process".to_owned(),
                    snapshot: serde_json::json!({ "observed": true }),
                    captured_at: 1_700_000_000_000,
                }),
                config_snapshots: Vec::new(),
            }),
        }
    }
    fn commit_succeeded(&mut self, _: CommitToken, _: &CollectorCommitReceipt) -> AppResult<()> {
        self.control.hook_entered.store(true, Ordering::SeqCst);
        if matches!(self.mode, Mode::HookFailure) {
            return Err(failed());
        }
        if matches!(self.mode, Mode::BlockedHook) {
            let guard = self.control.released.lock().unwrap();
            let (guard, _) = self
                .control
                .wake
                .wait_timeout_while(guard, Duration::from_secs(5), |released| !*released)
                .unwrap();
            if !*guard {
                return Err(failed());
            }
        }
        Ok(())
    }
}

fn setup(mode: Mode, sink: Arc<Sink>) -> (CollectorSupervisor, Arc<Control>, Arc<Db>) {
    let control = Arc::new(Control::default());
    let factory_control = Arc::clone(&control);
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor(), move || {
        Box::new(Probe {
            mode,
            control: Arc::clone(&factory_control),
        })
    })]);
    let db = Arc::new(Db::open_in(&what_changed_lib::testing::temp_dir("soak-observer")).unwrap());
    let supervisor = CollectorSupervisor::new(
        registry,
        Arc::clone(&db),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::from([("soak-probe".to_owned(), true)]),
        MonitoringProfile::Thorough,
    );
    supervisor.attach_status_sink(sink);
    supervisor.spawn_all().unwrap();
    (supervisor, control, db)
}
fn wait(predicate: impl Fn() -> bool) {
    for _ in 0..400 {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("observer outcome did not arrive");
}

#[test]
fn noop_and_legacy_successes_are_exactly_once_and_observer_panics_are_contained() {
    for mode in [Mode::Noop, Mode::Legacy] {
        let sink = Arc::new(Sink::default());
        sink.panic.store(true, Ordering::SeqCst);
        let (supervisor, _, _) = setup(mode, Arc::clone(&sink));
        wait(|| sink.successes.load(Ordering::SeqCst) == 1);
        assert!(!supervisor
            .capture_enabled(Duration::from_secs(2))
            .is_partial());
        assert_eq!(sink.successes.load(Ordering::SeqCst), 2);
        assert_eq!(sink.failures.load(Ordering::SeqCst), 0);
        supervisor.stop_all();
        assert!(!supervisor.shutdown_incomplete());
    }
}

#[test]
fn a_recovered_provider_failure_remains_observed_and_hook_failure_is_not_success() {
    for mode in [Mode::Recover, Mode::HookFailure] {
        let sink = Arc::new(Sink::default());
        let (supervisor, control, _) = setup(mode, Arc::clone(&sink));
        wait(|| sink.failures.load(Ordering::SeqCst) == 1);
        if matches!(mode, Mode::HookFailure) {
            assert!(control.hook_entered.load(Ordering::SeqCst));
        }
        assert_eq!(sink.successes.load(Ordering::SeqCst), 0);
        let capture = supervisor.capture_enabled(Duration::from_secs(2));
        if matches!(mode, Mode::Recover) {
            assert!(!capture.is_partial());
            assert_eq!(sink.successes.load(Ordering::SeqCst), 1);
            assert_eq!(sink.failures.load(Ordering::SeqCst), 1);
        } else {
            assert!(capture.is_partial());
            assert_eq!(sink.successes.load(Ordering::SeqCst), 0);
            assert_eq!(sink.failures.load(Ordering::SeqCst), 2);
        }
        supervisor.stop_all();
    }
}

#[test]
fn atomic_success_is_observed_only_after_the_durable_acknowledgement_hook() {
    let sink = Arc::new(Sink::default());
    let (supervisor, control, db) = setup(Mode::BlockedHook, Arc::clone(&sink));
    wait(|| control.hook_entered.load(Ordering::SeqCst));
    assert_eq!(sink.successes.load(Ordering::SeqCst), 0);
    let persisted = db
        .read(|conn| baseline_repository::load(conn, "soak-probe", "process"))
        .unwrap();
    assert!(persisted.is_some());
    *control.released.lock().unwrap() = true;
    control.wake.notify_all();
    wait(|| sink.successes.load(Ordering::SeqCst) == 1);
    supervisor.stop_all();
    assert_eq!(sink.successes.load(Ordering::SeqCst), 1);
    assert_eq!(sink.failures.load(Ordering::SeqCst), 0);
}

#[test]
fn blocked_collector_stop_latches_incomplete_shutdown_even_while_idle() {
    let sink = Arc::new(Sink::default());
    let (supervisor, control, _) = setup(Mode::BlockedStop, Arc::clone(&sink));
    wait(|| sink.successes.load(Ordering::SeqCst) == 1);
    supervisor.stop_all();
    let incomplete = supervisor.shutdown_incomplete();
    *control.released.lock().unwrap() = true;
    control.wake.notify_all();
    wait(|| control.stopped.load(Ordering::SeqCst));
    assert!(incomplete);
    supervisor.stop_all();
    assert!(supervisor.shutdown_incomplete());
}

#[test]
fn asynchronous_legacy_delivery_failure_is_observed_without_a_snapshot_error() {
    let sink = Arc::new(Sink::default());
    let (supervisor, control, db) = setup(Mode::Legacy, Arc::clone(&sink));
    wait(|| sink.successes.load(Ordering::SeqCst) == 1);
    db.close();
    let emitted = control
        .emit
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .emit(vec![RawObservation {
            collector_id: "soak-probe",
            kind: ObservationKind {
                category: Category::Process,
                event_type: EventType::Started,
                severity: Severity::Info,
                confidence: Confidence::High,
                user_friendly_summary: "probe".to_owned(),
                technical_summary: "probe".to_owned(),
                classifier: ClassifierContext::default(),
            },
            entity: RawEntity {
                identity: EntityIdentity::Process {
                    executable: "probe.exe".to_owned(),
                },
                display_name: "probe".to_owned(),
            },
            before: None,
            after: None,
            field: None,
            occurred_at: 1_700_000_000_000,
            details: None,
            related_entity_ids: Vec::new(),
            occurred_at_is_uncertain: false,
        }]);
    assert!(emitted.is_err());
    assert_eq!(sink.failures.load(Ordering::SeqCst), 1);
    assert_eq!(sink.successes.load(Ordering::SeqCst), 1);
    supervisor.stop_all();
}
