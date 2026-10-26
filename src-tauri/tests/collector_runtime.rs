#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::{
    CollectorRuntimeConfig, CollectorSupervisor, ExponentialBackoff, RuntimeClock,
    RuntimeStatusSink,
};
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::{
    Collector, CollectorContext, EmitHandle, EntityIdentity, ObservationKind, RawEntity,
    RawObservation,
};
use what_changed_lib::domain::{
    Category, ClassifierContext, CollectorCadence, CollectorCost, CollectorDescriptor, Confidence,
    EventQuery, EventType, MonitoringProfile, SettingsUpdate, Severity,
};
use what_changed_lib::dto::MonitoringDto;
use what_changed_lib::error::{AppError, AppResult, DbError};
use what_changed_lib::storage::{settings_repository, Db};

struct CounterCollector {
    count: Arc<AtomicU64>,
}
impl Collector for CounterCollector {
    fn id(&self) -> &'static str {
        "counter"
    }
    fn descriptor(&self) -> CollectorDescriptor {
        descriptor()
    }
    fn start(&mut self, _ctx: CollectorContext) -> AppResult<()> {
        Ok(())
    }
    fn stop(&mut self) {}
    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(Vec::new())
    }
}

fn descriptor() -> CollectorDescriptor {
    CollectorDescriptor {
        id: "counter",
        display_name: "Counter",
        what_it_watches: "test ticks",
        why_it_matters: "proves scheduling",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(Duration::from_millis(20)),
        requires_admin: false,
    }
}

fn observation() -> RawObservation {
    RawObservation {
        collector_id: "counter",
        kind: ObservationKind {
            category: Category::Process,
            event_type: EventType::Started,
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary: "test event".to_owned(),
            technical_summary: "test event".to_owned(),
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            identity: EntityIdentity::Process {
                executable: "counter.exe".to_owned(),
            },
            display_name: "counter".to_owned(),
        },
        before: None,
        after: None,
        field: None,
        occurred_at: 1_700_000_000_000,
        details: None,
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: false,
    }
}

fn wait_for(label: &str, predicate: impl Fn() -> bool) {
    for _ in 0..400 {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("{label} did not become true");
}

#[test]
fn start_pause_resume_disable_and_enable_take_effect_without_restart() {
    let count = Arc::new(AtomicU64::new(0));
    let factory_count = Arc::clone(&count);
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor(), move || {
        Box::new(CounterCollector {
            count: Arc::clone(&factory_count),
        })
    })]);
    let db =
        Arc::new(Db::open_in(&what_changed_lib::testing::temp_dir("collector-runtime")).unwrap());
    let monitor = Arc::new(Monitoring::running());
    let mut enabled = BTreeMap::new();
    enabled.insert("counter".to_owned(), true);
    let supervisor = CollectorSupervisor::new(
        registry,
        db,
        Arc::clone(&monitor),
        QueryBusy::default(),
        &enabled,
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    wait_for("collector startup", || count.load(Ordering::Relaxed) >= 2);

    supervisor.set_monitoring_paused(true, None);
    std::thread::sleep(Duration::from_millis(35));
    let paused_at = count.load(Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(60));
    assert_eq!(count.load(Ordering::Relaxed), paused_at);
    supervisor.set_monitoring_paused(false, None);
    wait_for("collector resume", || {
        count.load(Ordering::Relaxed) > paused_at
    });
    monitor.pause(Some(0));
    assert!(
        !monitor.is_paused(),
        "an expired timed pause auto-resumes on the next cycle/read"
    );

    supervisor.set_enabled("counter", false).unwrap();
    std::thread::sleep(Duration::from_millis(35));
    let disabled_at = count.load(Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(60));
    assert_eq!(count.load(Ordering::Relaxed), disabled_at);
    supervisor.set_enabled("counter", true).unwrap();
    wait_for("collector re-enable", || {
        count.load(Ordering::Relaxed) > disabled_at
    });
    let rows = supervisor.list_collector_status();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].requires_admin);
    assert_eq!(rows[0].descriptor.what_it_watches, "test ticks");
    supervisor.stop_all();
}

#[test]
fn cost_profiles_and_query_gate_are_explicit() {
    use what_changed_lib::collectors::schedule::{
        collector_interval, expensive_stagger, expensive_staggers, interval,
    };
    assert_eq!(
        interval(
            CollectorCadence::Poll(Duration::from_secs(10)),
            MonitoringProfile::Minimal
        ),
        Duration::from_secs(20)
    );
    assert_eq!(
        collector_interval(
            CollectorCadence::Poll(Duration::from_millis(10)),
            CollectorCost::Expensive,
            MonitoringProfile::Thorough,
        ),
        Duration::from_secs(30),
        "expensive work never joins the fast tick",
    );
    assert_eq!(
        interval(
            CollectorCadence::Poll(Duration::from_secs(10)),
            MonitoringProfile::Thorough
        ),
        Duration::from_secs(5)
    );
    assert!(expensive_stagger("drivers", CollectorCost::Expensive) < Duration::from_secs(30));
    assert_eq!(
        expensive_stagger("process", CollectorCost::Cheap),
        Duration::ZERO
    );
    let registry = CollectorRegistry::production();
    let expensive: Vec<&str> = registry
        .entries()
        .iter()
        .filter(|entry| entry.descriptor.cost_hint == CollectorCost::Expensive)
        .map(|entry| entry.descriptor.id)
        .collect();
    let staggers = expensive_staggers(&expensive);
    let mut expected_ids: Vec<String> = expensive.iter().map(|id| (*id).to_owned()).collect();
    expected_ids.sort_unstable();
    assert_eq!(
        staggers.keys().cloned().collect::<Vec<_>>(),
        expected_ids,
        "every production expensive collector receives a slot",
    );
    let mut slots: Vec<Duration> = staggers.values().copied().collect();
    let count = slots.len();
    slots.sort_unstable();
    slots.dedup();
    assert_eq!(
        slots.len(),
        count,
        "production expensive slots do not collide"
    );
    assert!(slots.iter().all(|slot| *slot < Duration::from_secs(30)));
    let mut reversed = expensive.clone();
    reversed.reverse();
    assert_eq!(
        staggers,
        expensive_staggers(&reversed),
        "assigned offsets are stable when registry construction order changes",
    );
    let gate = QueryBusy::default();
    {
        let _guard = gate.enter();
        assert!(gate.is_busy());
    }
    assert!(!gate.is_busy());
}

#[derive(Debug)]
struct ManualClock {
    base: Instant,
    elapsed_ms: AtomicU64,
    reads: AtomicU64,
}

impl ManualClock {
    fn new() -> Self {
        Self {
            base: Instant::now(),
            elapsed_ms: AtomicU64::new(0),
            reads: AtomicU64::new(0),
        }
    }

    fn advance(&self, duration: Duration) {
        self.elapsed_ms.fetch_add(
            u64::try_from(duration.as_millis()).unwrap(),
            Ordering::AcqRel,
        );
    }
}

impl RuntimeClock for ManualClock {
    fn now(&self) -> Instant {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.base + Duration::from_millis(self.elapsed_ms.load(Ordering::Acquire))
    }

    fn now_ms(&self) -> i64 {
        i64::try_from(self.elapsed_ms.load(Ordering::Acquire)).unwrap_or(i64::MAX)
    }
}

struct ContenderObservedClock {
    manual: ManualClock,
    contender_reads: AtomicU64,
}

impl ContenderObservedClock {
    fn new() -> Self {
        Self {
            manual: ManualClock::new(),
            contender_reads: AtomicU64::new(0),
        }
    }

    fn advance(&self, duration: Duration) {
        self.manual.advance(duration);
    }
}

impl RuntimeClock for ContenderObservedClock {
    fn now(&self) -> Instant {
        if std::thread::current().name() == Some("collector-b-contender") {
            self.contender_reads.fetch_add(1, Ordering::AcqRel);
        }
        self.manual.now()
    }

    fn now_ms(&self) -> i64 {
        self.manual.now_ms()
    }
}

struct DescriptorCounterCollector {
    descriptor: CollectorDescriptor,
    count: Arc<AtomicU64>,
}

impl Collector for DescriptorCounterCollector {
    fn id(&self) -> &'static str {
        self.descriptor.id
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, _context: CollectorContext) -> AppResult<()> {
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.count.fetch_add(1, Ordering::Relaxed);
        Ok(Vec::new())
    }
}

fn transition_registry() -> (
    CollectorRegistry,
    BTreeMap<&'static str, Arc<AtomicU64>>,
    Vec<&'static str>,
) {
    let production = CollectorRegistry::production();
    let selected: Vec<CollectorDescriptor> = production
        .entries()
        .iter()
        .filter(|entry| {
            entry.descriptor.cost_hint == CollectorCost::Expensive
                || entry.descriptor.id == "process"
        })
        .map(|entry| entry.descriptor.clone())
        .collect();
    let expensive_ids = selected
        .iter()
        .filter(|descriptor| descriptor.cost_hint == CollectorCost::Expensive)
        .map(|descriptor| descriptor.id)
        .collect();
    let mut counters = BTreeMap::new();
    let entries = selected
        .into_iter()
        .map(|descriptor| {
            let count = Arc::new(AtomicU64::new(0));
            counters.insert(descriptor.id, Arc::clone(&count));
            let factory_descriptor = descriptor.clone();
            RegistryEntry::active(descriptor, move || {
                Box::new(DescriptorCounterCollector {
                    descriptor: factory_descriptor.clone(),
                    count: Arc::clone(&count),
                })
            })
        })
        .collect();
    (CollectorRegistry::new(entries), counters, expensive_ids)
}

#[derive(Clone, Copy)]
enum GenerationTransition {
    Profile,
    Enable,
    MonitoringResume,
}

fn assert_expensive_transition_reapplies_stagger(transition: GenerationTransition) {
    let (registry, counters, expensive_ids) = transition_registry();
    let assigned = what_changed_lib::collectors::schedule::expensive_staggers(&expensive_ids);
    assert_eq!(assigned.len(), expensive_ids.len());
    let zero_offset: Vec<&str> = assigned
        .iter()
        .filter(|(_, delay)| **delay == Duration::ZERO)
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        zero_offset.len(),
        1,
        "exactly one expensive collector owns the zero-second slot",
    );
    let zero_offset = zero_offset[0];
    let clock = Arc::new(ManualClock::new());
    let monitor = Arc::new(Monitoring::running());
    if matches!(transition, GenerationTransition::MonitoringResume) {
        monitor.pause(None);
    }
    let initially_enabled = !matches!(transition, GenerationTransition::Enable);
    let enabled = counters
        .keys()
        .map(|id| ((*id).to_owned(), initially_enabled))
        .collect();
    let supervisor = CollectorSupervisor::with_runtime(
        registry,
        CollectorRuntimeConfig {
            db: Arc::new(
                Db::open_in(&what_changed_lib::testing::temp_dir("collector-restagger")).unwrap(),
            ),
            monitor,
            query_busy: QueryBusy::default(),
            enabled,
            profile: MonitoringProfile::Balanced,
            clock: Arc::clone(&clock) as Arc<dyn RuntimeClock>,
            backoff: Arc::new(ExponentialBackoff),
        },
    );
    supervisor.spawn_all().unwrap();

    if matches!(transition, GenerationTransition::Profile) {
        wait_for("zero-offset collectors to start", || {
            counters[zero_offset].load(Ordering::Relaxed) > 0
                && counters["process"].load(Ordering::Relaxed) > 0
        });
    } else {
        wait_for("all workers to initialize their first due time", || {
            clock.reads.load(Ordering::Relaxed) >= counters.len() as u64
        });
    }
    let before: BTreeMap<&str, u64> = counters
        .iter()
        .map(|(id, count)| (*id, count.load(Ordering::Relaxed)))
        .collect();
    clock.advance(Duration::from_secs(60));

    match transition {
        GenerationTransition::Profile => supervisor.set_profile(MonitoringProfile::Minimal),
        GenerationTransition::Enable => {
            for id in counters.keys() {
                supervisor.set_enabled(id, true).unwrap();
            }
        }
        GenerationTransition::MonitoringResume => {
            supervisor.set_monitoring_paused(false, None);
        }
    }

    wait_for("cheap and zero-offset collectors to run", || {
        counters[zero_offset].load(Ordering::Relaxed) > before[zero_offset]
            && counters["process"].load(Ordering::Relaxed) > before["process"]
    });
    std::thread::sleep(Duration::from_millis(80));
    for id in expensive_ids.into_iter().filter(|id| *id != zero_offset) {
        assert_eq!(
            counters[id].load(Ordering::Relaxed),
            before[id],
            "{id} ran immediately instead of keeping its assigned non-zero stagger",
        );
    }
    supervisor.stop_all();
}

#[test]
fn expensive_collectors_reapply_stagger_after_profile_generation_change() {
    assert_expensive_transition_reapplies_stagger(GenerationTransition::Profile);
}

#[test]
fn expensive_collectors_reapply_stagger_after_enable_generation_change() {
    assert_expensive_transition_reapplies_stagger(GenerationTransition::Enable);
}

#[test]
fn expensive_collectors_reapply_stagger_after_monitoring_resume_generation() {
    assert_expensive_transition_reapplies_stagger(GenerationTransition::MonitoringResume);
}

struct TimestampCollector {
    descriptor: CollectorDescriptor,
    snapshots: SyncSender<Instant>,
}

impl Collector for TimestampCollector {
    fn id(&self) -> &'static str {
        self.descriptor.id
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, _context: CollectorContext) -> AppResult<()> {
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.snapshots.send(Instant::now()).unwrap();
        Ok(Vec::new())
    }
}

#[test]
fn polling_cadence_is_within_twenty_percent_over_an_averaged_window() {
    const CADENCE: Duration = Duration::from_millis(200);
    const SAMPLE_COUNT: usize = 6;
    let descriptor = CollectorDescriptor {
        id: "cadence",
        display_name: "cadence",
        what_it_watches: "scheduler timestamps",
        why_it_matters: "proves the configured cadence",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(CADENCE),
        requires_admin: false,
    };
    let factory_descriptor = descriptor.clone();
    let (snapshots_tx, snapshots_rx) = sync_channel(SAMPLE_COUNT);
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor, move || {
        Box::new(TimestampCollector {
            descriptor: factory_descriptor.clone(),
            snapshots: snapshots_tx.clone(),
        })
    })]);
    let supervisor = CollectorSupervisor::new(
        registry,
        Arc::new(
            Db::open_in(&what_changed_lib::testing::temp_dir(
                "collector-cadence-window",
            ))
            .unwrap(),
        ),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::from([("cadence".to_owned(), true)]),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    let observed: Vec<Instant> = (0..SAMPLE_COUNT)
        .map(|_| {
            snapshots_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("the polling collector produced every averaged sample")
        })
        .collect();
    supervisor.stop_all();

    let span = observed[SAMPLE_COUNT - 1].duration_since(observed[0]);
    let average = span / u32::try_from(SAMPLE_COUNT - 1).unwrap();
    assert!(
        (CADENCE.mul_f32(0.8)..=CADENCE.mul_f32(1.2)).contains(&average),
        "average cadence {average:?} fell outside ±20% of {CADENCE:?} over {span:?}",
    );
}

struct TimedFailureCollector {
    descriptor: CollectorDescriptor,
    clock: Arc<ManualClock>,
    attempts: u8,
}

impl Collector for TimedFailureCollector {
    fn id(&self) -> &'static str {
        self.descriptor.id
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, _context: CollectorContext) -> AppResult<()> {
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.attempts = self.attempts.saturating_add(1);
        if self.attempts == 1 {
            self.clock.advance(Duration::from_millis(20));
            Ok(Vec::new())
        } else {
            self.clock.advance(Duration::from_millis(35));
            Err(AppError::InvalidInput {
                what: "timed snapshot",
                reason: "injected provider failure".to_owned(),
            })
        }
    }
}

#[test]
fn failed_snapshot_status_pairs_last_run_with_that_attempts_duration() {
    let clock = Arc::new(ManualClock::new());
    let descriptor = CollectorDescriptor {
        id: "timed-failure",
        display_name: "timed failure",
        what_it_watches: "provider duration",
        why_it_matters: "keeps status timestamps internally consistent",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(Duration::from_millis(10)),
        requires_admin: false,
    };
    let factory_descriptor = descriptor.clone();
    let factory_clock = Arc::clone(&clock);
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor, move || {
        Box::new(TimedFailureCollector {
            descriptor: factory_descriptor.clone(),
            clock: Arc::clone(&factory_clock),
            attempts: 0,
        })
    })]);
    let supervisor = CollectorSupervisor::with_runtime(
        registry,
        CollectorRuntimeConfig {
            db: Arc::new(
                Db::open_in(&what_changed_lib::testing::temp_dir(
                    "collector-failure-duration",
                ))
                .unwrap(),
            ),
            monitor: Arc::new(Monitoring::running()),
            query_busy: QueryBusy::default(),
            enabled: BTreeMap::from([("timed-failure".to_owned(), true)]),
            profile: MonitoringProfile::Balanced,
            clock: Arc::clone(&clock) as Arc<dyn RuntimeClock>,
            backoff: Arc::new(ExponentialBackoff),
        },
    );
    supervisor.spawn_all().unwrap();
    wait_for("the successful timed snapshot", || {
        supervisor.list_collector_status()[0].last_duration_ms == Some(20)
    });

    supervisor.set_profile(MonitoringProfile::Minimal);
    wait_for("the failed timed snapshot", || {
        supervisor.list_collector_status()[0].consecutive_failures == 1
    });
    let status = supervisor.list_collector_status().remove(0);
    assert_eq!(status.last_run_at, Some(55));
    assert_eq!(status.last_duration_ms, Some(35));
    assert_eq!(status.consecutive_failures, 1);
    supervisor.stop_all();
}

#[test]
fn expensive_lease_contention_does_not_move_a_due_deadline_on_a_frozen_clock() {
    let clock = Arc::new(ContenderObservedClock::new());
    let (holder_entered_tx, holder_entered_rx) = sync_channel(1);
    let (holder_release_tx, holder_release_rx) = sync_channel(1);
    let holder_release_rx = Arc::new(Mutex::new(holder_release_rx));
    let (contender_entered_tx, contender_entered_rx) = sync_channel(1);

    let expensive_descriptor = |id: &'static str| CollectorDescriptor {
        id,
        display_name: id,
        what_it_watches: "lease retry test source",
        why_it_matters: "proves a due collector remains due after contention",
        cost_hint: CollectorCost::Expensive,
        cadence: CollectorCadence::Poll(Duration::from_secs(3_600)),
        requires_admin: false,
    };
    let holder_descriptor = expensive_descriptor("a-holder");
    let holder_factory_descriptor = holder_descriptor.clone();
    let holder_release = Arc::clone(&holder_release_rx);
    let holder_entry = RegistryEntry::active(holder_descriptor, move || {
        holder_entered_tx.send(()).unwrap();
        holder_release
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .recv()
            .unwrap();
        Box::new(DescriptorCounterCollector {
            descriptor: holder_factory_descriptor.clone(),
            count: Arc::new(AtomicU64::new(0)),
        })
    });
    let contender_descriptor = expensive_descriptor("b-contender");
    let contender_factory_descriptor = contender_descriptor.clone();
    let contender_entry = RegistryEntry::active(contender_descriptor, move || {
        contender_entered_tx.send(()).unwrap();
        Box::new(DescriptorCounterCollector {
            descriptor: contender_factory_descriptor.clone(),
            count: Arc::new(AtomicU64::new(0)),
        })
    });
    let signal_descriptor = CollectorDescriptor {
        id: "signal",
        display_name: "signal",
        what_it_watches: "lease retry test signal",
        why_it_matters: "wakes a worker after fake time advances",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(Duration::from_secs(3_600)),
        requires_admin: false,
    };
    let signal_factory_descriptor = signal_descriptor.clone();
    let signal_entry = RegistryEntry::active(signal_descriptor, move || {
        Box::new(DescriptorCounterCollector {
            descriptor: signal_factory_descriptor.clone(),
            count: Arc::new(AtomicU64::new(0)),
        })
    });
    let supervisor = CollectorSupervisor::with_runtime(
        CollectorRegistry::new(vec![holder_entry, contender_entry, signal_entry]),
        CollectorRuntimeConfig {
            db: Arc::new(
                Db::open_in(&what_changed_lib::testing::temp_dir(
                    "collector-expensive-lease-frozen-clock",
                ))
                .unwrap(),
            ),
            monitor: Arc::new(Monitoring::running()),
            query_busy: QueryBusy::default(),
            enabled: BTreeMap::from([
                ("a-holder".to_owned(), true),
                ("b-contender".to_owned(), true),
                ("signal".to_owned(), false),
            ]),
            profile: MonitoringProfile::Balanced,
            clock: Arc::clone(&clock) as Arc<dyn RuntimeClock>,
            backoff: Arc::new(ExponentialBackoff),
        },
    );
    supervisor.spawn_all().unwrap();
    holder_entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("the zero-offset collector holds the expensive lease");
    wait_for("the contender to establish its initial deadline", || {
        clock.contender_reads.load(Ordering::Acquire) >= 1
    });

    clock.advance(Duration::from_secs(15));
    supervisor.set_enabled("signal", true).unwrap();
    wait_for("the contender to observe at least one lease retry", || {
        clock.contender_reads.load(Ordering::Acquire) >= 3
    });
    holder_release_tx.send(()).unwrap();

    let contender_entered = contender_entered_rx.recv_timeout(Duration::from_millis(500));
    supervisor.stop_all();
    contender_entered.expect("a due contender retries after the expensive lease is released");
}

struct ContextCollector {
    context: std::sync::mpsc::SyncSender<EmitHandle>,
}

impl Collector for ContextCollector {
    fn id(&self) -> &'static str {
        "counter"
    }
    fn descriptor(&self) -> CollectorDescriptor {
        descriptor()
    }
    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let _ = self.context.send(context.emit);
        Ok(())
    }
    fn stop(&mut self) {}
    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Ok(Vec::new())
    }
}

struct NamedContextCollector {
    id: &'static str,
    contexts: std::sync::mpsc::SyncSender<(&'static str, EmitHandle)>,
}

impl Collector for NamedContextCollector {
    fn id(&self) -> &'static str {
        self.id
    }

    fn descriptor(&self) -> CollectorDescriptor {
        CollectorDescriptor {
            id: self.id,
            display_name: self.id,
            what_it_watches: "mutable test input",
            why_it_matters: "proves multi-source fencing",
            cost_hint: CollectorCost::Cheap,
            cadence: CollectorCadence::Notify,
            requires_admin: false,
        }
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let _ = self.contexts.send((self.id, context.emit));
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Ok(Vec::new())
    }
}

#[test]
fn one_input_change_fences_every_named_source_before_mutation() {
    let (contexts_tx, contexts_rx) = sync_channel(8);
    let files_tx = contexts_tx.clone();
    let dev_env_tx = contexts_tx;
    let registry = CollectorRegistry::new(vec![
        RegistryEntry::active(
            NamedContextCollector {
                id: "files",
                contexts: files_tx.clone(),
            }
            .descriptor(),
            move || {
                Box::new(NamedContextCollector {
                    id: "files",
                    contexts: files_tx.clone(),
                })
            },
        ),
        RegistryEntry::active(
            NamedContextCollector {
                id: "dev_env",
                contexts: dev_env_tx.clone(),
            }
            .descriptor(),
            move || {
                Box::new(NamedContextCollector {
                    id: "dev_env",
                    contexts: dev_env_tx.clone(),
                })
            },
        ),
    ]);
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "collector-multi-source-fence",
        ))
        .unwrap(),
    );
    let supervisor = CollectorSupervisor::new(
        registry,
        Arc::clone(&db),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::from([("files".to_owned(), true), ("dev_env".to_owned(), true)]),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    let mut handles = BTreeMap::new();
    for _ in 0..2 {
        let (id, handle) = contexts_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        handles.insert(id, handle);
    }

    // Start stale callbacks before the persisted-input mutation.  They must
    // be fenced while the write gate is held; otherwise one could enqueue an
    // event from the old roots before the mutation becomes visible.
    let stale_handles: Vec<_> = handles.values().cloned().collect();
    let (attempt_started_tx, attempt_started_rx) = sync_channel(1);
    let (results_tx, results_rx) = sync_channel(1);
    let join_slot = Arc::new(Mutex::new(None));
    let join_slot_for_change = Arc::clone(&join_slot);
    let db_for_change = Arc::clone(&db);
    supervisor
        .apply_sources_change(&["files", "dev_env"], move || {
            let attempt_started_tx = attempt_started_tx.clone();
            let results_tx = results_tx.clone();
            let join = std::thread::spawn(move || {
                attempt_started_tx.send(()).unwrap();
                let results = stale_handles
                    .into_iter()
                    .map(|handle| handle.emit(vec![observation()]).is_err())
                    .collect::<Vec<_>>();
                results_tx.send(results).unwrap();
            });
            attempt_started_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("stale callbacks started before the input mutation");
            db_for_change
                .write(|connection| {
                    settings_repository::put(connection, "test_fence_marker", &true)
                })
                .unwrap();
            *join_slot_for_change.lock().unwrap() = Some(join);
            Ok(())
        })
        .unwrap();
    join_slot
        .lock()
        .unwrap()
        .take()
        .expect("stale callback worker was retained")
        .join()
        .unwrap();
    assert!(results_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap()
        .into_iter()
        .all(|rejected| rejected));
    assert_eq!(
        db.read(|connection| settings_repository::get_json(connection, "test_fence_marker"))
            .unwrap()
            .unwrap(),
        serde_json::Value::Bool(true)
    );

    supervisor.stop_all();
    db.flush().unwrap();
    let page = what_changed_lib::services::query_service::page(&db, EventQuery::default()).unwrap();
    assert!(page.items.is_empty());
}

#[test]
fn stale_emit_handles_fail_closed_across_toggle_pause_and_shutdown() {
    let (context_tx, context_rx) = sync_channel(8);
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor(), move || {
        Box::new(ContextCollector {
            context: context_tx.clone(),
        })
    })]);
    let db =
        Arc::new(Db::open_in(&what_changed_lib::testing::temp_dir("collector-epoch")).unwrap());
    let supervisor = CollectorSupervisor::new(
        registry,
        Arc::clone(&db),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::from([("counter".to_owned(), true)]),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    let first = context_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    supervisor.set_enabled("counter", false).unwrap();
    supervisor.set_enabled("counter", true).unwrap();
    assert!(
        first.emit(vec![observation()]).is_err(),
        "off/on fences the old context"
    );
    let second = context_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    supervisor.set_monitoring_paused(true, None);
    supervisor.set_monitoring_paused(false, None);
    assert!(
        second.emit(vec![observation()]).is_err(),
        "pause/resume fences the old context"
    );
    let third = context_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    supervisor.stop_all();
    assert!(
        third.emit(vec![observation()]).is_err(),
        "shutdown fences every context"
    );
    db.flush().unwrap();
    let page = what_changed_lib::services::query_service::page(&db, EventQuery::default()).unwrap();
    assert!(page.items.is_empty(), "no stale callback reached storage");
}

struct BlockingCollector {
    entered: std::sync::mpsc::SyncSender<()>,
    release: Arc<Mutex<std::sync::mpsc::Receiver<()>>>,
    stops: Arc<AtomicU64>,
}

impl Collector for BlockingCollector {
    fn id(&self) -> &'static str {
        "counter"
    }
    fn descriptor(&self) -> CollectorDescriptor {
        descriptor()
    }
    fn start(&mut self, _context: CollectorContext) -> AppResult<()> {
        Ok(())
    }
    fn stop(&mut self) {
        self.stops.fetch_add(1, Ordering::Relaxed);
    }
    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        let _ = self.entered.send(());
        let _ = self
            .release
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .recv();
        Ok(vec![observation()])
    }
}

#[test]
fn a_snapshot_that_finishes_after_disable_is_rejected_and_stopped() {
    let (entered_tx, entered_rx) = sync_channel(1);
    let (release_tx, release_rx) = sync_channel(1);
    let release_rx = Arc::new(Mutex::new(release_rx));
    let stops = Arc::new(AtomicU64::new(0));
    let factory_release = Arc::clone(&release_rx);
    let factory_stops = Arc::clone(&stops);
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor(), move || {
        Box::new(BlockingCollector {
            entered: entered_tx.clone(),
            release: Arc::clone(&factory_release),
            stops: Arc::clone(&factory_stops),
        })
    })]);
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "collector-long-snapshot",
        ))
        .unwrap(),
    );
    let supervisor = CollectorSupervisor::new(
        registry,
        Arc::clone(&db),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::from([("counter".to_owned(), true)]),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    supervisor.set_enabled("counter", false).unwrap();
    release_tx.send(()).unwrap();
    wait_for("collector stop acknowledgement", || {
        stops.load(Ordering::Relaxed) == 1
    });
    db.flush().unwrap();
    let page = what_changed_lib::services::query_service::page(&db, EventQuery::default()).unwrap();
    assert!(page.items.is_empty());
    assert_eq!(supervisor.list_collector_status()[0].events_emitted, 0);
    supervisor.stop_all();
}

#[derive(Default)]
struct StatusRecorder {
    monitoring: Arc<Mutex<Vec<MonitoringDto>>>,
    collectors: Arc<Mutex<Vec<String>>>,
}

impl RuntimeStatusSink for StatusRecorder {
    fn collector_changed(&self, collector_id: &str) {
        self.collectors
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(collector_id.to_owned());
    }
    fn monitoring_changed(&self, monitoring: MonitoringDto) {
        self.monitoring
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(monitoring);
    }
}

#[test]
fn timed_pause_publishes_pause_and_automatic_resume_once() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "collector-timed-pause",
        ))
        .unwrap(),
    );
    let supervisor = CollectorSupervisor::new(
        CollectorRegistry::new(Vec::new()),
        db,
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::new(),
        MonitoringProfile::Balanced,
    );
    let recorder = Arc::new(StatusRecorder::default());
    let seen = Arc::clone(&recorder.monitoring);
    supervisor.attach_status_sink(recorder);
    supervisor.spawn_all().unwrap();
    supervisor.pause_monitoring_for(Duration::from_millis(40));
    wait_for("timed pause auto-resume event", || {
        seen.lock().unwrap_or_else(|p| p.into_inner()).len() == 2
    });
    let events = seen.lock().unwrap_or_else(|p| p.into_inner()).clone();
    assert!(events[0].paused);
    assert!(events[0].resume_at.is_some());
    assert_eq!(
        events[1],
        MonitoringDto {
            paused: false,
            resume_at: None
        }
    );
    supervisor.stop_all();
}

struct BlockingResumeSink {
    entered: SyncSender<()>,
    release: Mutex<Receiver<()>>,
    finished: SyncSender<()>,
}

impl RuntimeStatusSink for BlockingResumeSink {
    fn collector_changed(&self, _collector_id: &str) {}

    fn monitoring_changed(&self, monitoring: MonitoringDto) {
        if monitoring.paused {
            return;
        }
        let _ = self.entered.send(());
        let _ = self
            .release
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .recv();
        let _ = self.finished.send(());
    }
}

#[test]
fn shutdown_deadline_includes_a_watchdog_blocked_in_the_status_sink() {
    let supervisor = CollectorSupervisor::new(
        CollectorRegistry::new(Vec::new()),
        Arc::new(
            Db::open_in(&what_changed_lib::testing::temp_dir(
                "collector-blocked-sink",
            ))
            .unwrap(),
        ),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::new(),
        MonitoringProfile::Balanced,
    );
    let (entered_tx, entered_rx) = sync_channel(1);
    let (release_tx, release_rx) = sync_channel(1);
    let (finished_tx, finished_rx) = sync_channel(1);
    supervisor.attach_status_sink(Arc::new(BlockingResumeSink {
        entered: entered_tx,
        release: Mutex::new(release_rx),
        finished: finished_tx,
    }));
    supervisor.spawn_all().unwrap();
    supervisor.pause_monitoring_for(Duration::from_millis(20));
    entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("the watchdog entered the deliberately blocking resume callback");

    // Release after the product deadline so the old unconditional watchdog
    // join fails this assertion without leaking the watchdog from the test.
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(2_500));
        let _ = release_tx.send(());
    });
    let began = Instant::now();
    supervisor.stop_all();
    let elapsed = began.elapsed();
    assert!(
        elapsed <= Duration::from_millis(2_250),
        "one global two-second shutdown deadline was exceeded: {elapsed:?}",
    );
    releaser.join().unwrap();
    finished_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("the detached watchdog exited after the test released its sink");
}

#[test]
fn production_status_has_one_non_admin_row_per_registry_entry() {
    let registry = CollectorRegistry::production();
    let expected = registry.entries().len();
    let supervisor = CollectorSupervisor::new(
        registry,
        Arc::new(Db::open_in(&what_changed_lib::testing::temp_dir("collector-rows")).unwrap()),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::new(),
        MonitoringProfile::Balanced,
    );
    let rows = supervisor.list_collector_status();
    assert_eq!(rows.len(), expected);
    assert!(rows
        .iter()
        .all(|row| !row.requires_admin && !row.descriptor.requires_admin));
}

#[test]
fn production_catalog_matches_seeded_settings_and_every_id_persists() {
    let registry = CollectorRegistry::production();
    let mut registry_ids: Vec<String> = registry
        .entries()
        .iter()
        .map(|entry| entry.descriptor.id.to_owned())
        .collect();
    registry_ids.sort_unstable();
    let db = Db::open_in(&what_changed_lib::testing::temp_dir(
        "collector-catalog-contract",
    ))
    .unwrap();
    let seeded = db.read(settings_repository::load).unwrap().collectors;
    assert_eq!(
        registry_ids,
        seeded.keys().cloned().collect::<Vec<_>>(),
        "the immutable descriptor catalog and migration-seeded settings IDs drifted",
    );

    for id in registry_ids {
        let expected = !seeded[&id];
        let changes = BTreeMap::from([(id.clone(), expected)]);
        let changed = db
            .write(move |connection| {
                settings_repository::update(
                    connection,
                    &SettingsUpdate {
                        collectors: Some(changes),
                        ..Default::default()
                    },
                )
            })
            .unwrap();
        assert_eq!(
            changed.collectors.get(&id),
            Some(&expected),
            "descriptor-derived toggle {id} did not persist",
        );
        assert_eq!(changed.collectors.len(), seeded.len());
    }
}

#[test]
fn production_catalog_has_the_future_collectors_costs_and_cadences() {
    let actual: BTreeMap<&str, (CollectorCost, CollectorCadence)> = CollectorRegistry::production()
        .entries()
        .iter()
        .map(|entry| {
            (
                entry.descriptor.id,
                (entry.descriptor.cost_hint, entry.descriptor.cadence),
            )
        })
        .collect();
    let expected = BTreeMap::from([
        (
            "process",
            (
                CollectorCost::Cheap,
                CollectorCadence::Poll(Duration::from_secs(1)),
            ),
        ),
        (
            "listeners",
            (
                CollectorCost::Cheap,
                CollectorCadence::Poll(Duration::from_secs(2)),
            ),
        ),
        (
            "files",
            (
                CollectorCost::Moderate,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(60)),
            ),
        ),
        (
            "services",
            (
                CollectorCost::Cheap,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(5)),
            ),
        ),
        (
            "applications",
            (
                CollectorCost::Moderate,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(300)),
            ),
        ),
        (
            "updates",
            (
                CollectorCost::Cheap,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(900)),
            ),
        ),
        (
            "drivers",
            (
                CollectorCost::Expensive,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(21_600)),
            ),
        ),
        (
            "network",
            (
                CollectorCost::Expensive,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(1_800)),
            ),
        ),
        (
            "startup",
            (
                CollectorCost::Expensive,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(3_600)),
            ),
        ),
        ("eventlog", (CollectorCost::Cheap, CollectorCadence::Notify)),
        (
            "docker",
            (
                CollectorCost::Cheap,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(60)),
            ),
        ),
        (
            "dev_env",
            (
                CollectorCost::Expensive,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(21_600)),
            ),
        ),
    ]);
    assert_eq!(actual, expected);
}

#[test]
fn an_expensive_worker_skips_while_a_query_is_busy() {
    let count = Arc::new(AtomicU64::new(0));
    let factory_count = Arc::clone(&count);
    let mut expensive = descriptor();
    expensive.cost_hint = CollectorCost::Expensive;
    expensive.cadence = CollectorCadence::Poll(Duration::from_millis(10));
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(expensive, move || {
        Box::new(CounterCollector {
            count: Arc::clone(&factory_count),
        })
    })]);
    let gate = QueryBusy::default();
    let busy = gate.enter();
    let supervisor = CollectorSupervisor::new(
        registry,
        Arc::new(
            Db::open_in(&what_changed_lib::testing::temp_dir("collector-query-busy")).unwrap(),
        ),
        Arc::new(Monitoring::running()),
        gate,
        &BTreeMap::from([("counter".to_owned(), true)]),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    std::thread::sleep(Duration::from_millis(80));
    assert_eq!(count.load(Ordering::Relaxed), 0);
    drop(busy);
    for _ in 0..300 {
        if count.load(Ordering::Relaxed) > 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        count.load(Ordering::Relaxed) > 0,
        "an expensive collector resumes after the final query guard drops",
    );
    supervisor.stop_all();
}

#[test]
fn a_full_writer_queue_degrades_only_the_affected_collector() {
    let (context_tx, context_rx) = sync_channel(1);
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir("collector-queue-full")).unwrap(),
    );
    let supervisor = CollectorSupervisor::new(
        CollectorRegistry::new(vec![RegistryEntry::active(descriptor(), move || {
            Box::new(ContextCollector {
                context: context_tx.clone(),
            })
        })]),
        Arc::clone(&db),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::from([("counter".to_owned(), true)]),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    let emit = context_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    let (entered_tx, entered_rx) = sync_channel(1);
    let (release_tx, release_rx) = sync_channel(1);
    let blocker_db = Arc::clone(&db);
    let blocker = std::thread::spawn(move || {
        blocker_db
            .write(move |_connection| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(())
            })
            .unwrap();
    });
    entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("the writer is deterministically blocked before filling its queue");

    let mut accepted = 0usize;
    let queue_error = loop {
        match emit.emit(vec![observation()]) {
            Ok(()) => accepted += 1,
            Err(error) => break error,
        }
        assert!(accepted < 10_000, "the bounded writer queue never filled");
    };
    assert!(matches!(queue_error, AppError::Db(DbError::QueueFull)));
    let row = &supervisor.list_collector_status()[0];
    assert_eq!(row.queue_drops, 1);
    assert_eq!(row.failed_commit_rows, 0);
    assert_eq!(row.state, what_changed_lib::dto::CollectorState::Degraded);
    assert_eq!(db.dropped_writes(), 1);
    assert!(row.last_run_at.is_some());
    assert_eq!(
        row.last_duration_ms, None,
        "an out-of-band delivery failure has no measured provider duration",
    );

    release_tx.send(()).unwrap();
    blocker.join().unwrap();
    db.flush().unwrap();
    supervisor.stop_all();
}

struct OneShotCollector(bool);

impl Collector for OneShotCollector {
    fn id(&self) -> &'static str {
        "counter"
    }
    fn descriptor(&self) -> CollectorDescriptor {
        descriptor()
    }
    fn start(&mut self, _context: CollectorContext) -> AppResult<()> {
        Ok(())
    }
    fn stop(&mut self) {}
    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        if std::mem::replace(&mut self.0, true) {
            Ok(Vec::new())
        } else {
            Ok(vec![observation()])
        }
    }
}

#[test]
fn a_failed_commit_is_distinct_from_queue_admission_and_degrades_the_collector() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "collector-commit-fault",
        ))
        .unwrap(),
    );
    db.write(|conn| {
        conn.execute_batch(
            "CREATE TRIGGER fail_collector_commit BEFORE INSERT ON events \
             BEGIN SELECT RAISE(FAIL, 'injected commit failure'); END;",
        )
        .unwrap();
        Ok(())
    })
    .unwrap();
    let supervisor = CollectorSupervisor::new(
        CollectorRegistry::new(vec![RegistryEntry::active(descriptor(), || {
            Box::new(OneShotCollector(false))
        })]),
        Arc::clone(&db),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::from([("counter".to_owned(), true)]),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    wait_for("writer queue acceptance", || {
        supervisor.list_collector_status()[0].events_emitted == 1
    });
    db.flush().unwrap();
    let row = &supervisor.list_collector_status()[0];
    assert_eq!(row.events_emitted, 1, "accepted by the writer queue");
    assert_eq!(row.queue_drops, 0, "not rejected at admission");
    assert_eq!(
        row.failed_commit_rows, 1,
        "lost only after SQLite rejected the batch"
    );
    assert_eq!(row.state, what_changed_lib::dto::CollectorState::Degraded);
    let page = what_changed_lib::services::query_service::page(&db, EventQuery::default()).unwrap();
    assert!(
        page.items.is_empty(),
        "accepted is not the same as recorded"
    );
    supervisor.stop_all();
}
