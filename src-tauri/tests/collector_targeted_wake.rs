#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::{
    BackoffPolicy, CollectorRuntimeConfig, CollectorSupervisor, ExponentialBackoff, RuntimeClock,
    SystemClock,
};
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::{Collector, CollectorContext, RawObservation, WakeHandle};
use what_changed_lib::domain::{
    CollectorCadence, CollectorCost, CollectorDescriptor, MonitoringProfile,
};
use what_changed_lib::error::{AppError, AppResult};
use what_changed_lib::storage::Db;

fn descriptor(
    id: &'static str,
    cadence: CollectorCadence,
    cost_hint: CollectorCost,
) -> CollectorDescriptor {
    CollectorDescriptor {
        id,
        display_name: id,
        what_it_watches: "targeted wake test source",
        why_it_matters: "proves notification scheduling",
        cost_hint,
        cadence,
        requires_admin: false,
    }
}

fn enabled(ids: &[&str]) -> BTreeMap<String, bool> {
    ids.iter().map(|id| ((*id).to_owned(), true)).collect()
}

fn runtime(
    registry: CollectorRegistry,
    suffix: &str,
    ids: &[&str],
    query_busy: QueryBusy,
    clock: Arc<dyn RuntimeClock>,
    backoff: Arc<dyn BackoffPolicy>,
) -> CollectorSupervisor {
    CollectorSupervisor::with_runtime(
        registry,
        CollectorRuntimeConfig {
            db: Arc::new(Db::open_in(&what_changed_lib::testing::temp_dir(suffix)).unwrap()),
            monitor: Arc::new(Monitoring::running()),
            query_busy,
            enabled: enabled(ids),
            profile: MonitoringProfile::Balanced,
            clock,
            backoff,
        },
    )
}

fn system_runtime(registry: CollectorRegistry, suffix: &str, ids: &[&str]) -> CollectorSupervisor {
    runtime(
        registry,
        suffix,
        ids,
        QueryBusy::default(),
        Arc::new(SystemClock),
        Arc::new(ExponentialBackoff),
    )
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

struct WorkerObservedClock {
    manual: ManualClock,
    worker_name: &'static str,
    observed: AtomicBool,
    first_read: SyncSender<()>,
    release_first_read: Mutex<Receiver<()>>,
}

impl WorkerObservedClock {
    fn new(
        worker_name: &'static str,
        first_read: SyncSender<()>,
        release_first_read: Receiver<()>,
    ) -> Self {
        Self {
            manual: ManualClock::new(),
            worker_name,
            observed: AtomicBool::new(false),
            first_read,
            release_first_read: Mutex::new(release_first_read),
        }
    }

    fn advance(&self, duration: Duration) {
        self.manual.advance(duration);
    }
}

impl RuntimeClock for WorkerObservedClock {
    fn now(&self) -> Instant {
        // Capture time before publishing the observation. The receiver may
        // advance the manual clock immediately, so notifying first would not
        // mean the worker had actually established its deadline at that time.
        let now = self.manual.now();
        if std::thread::current().name() == Some(self.worker_name)
            && !self.observed.swap(true, Ordering::AcqRel)
        {
            let _ = self.first_read.try_send(());
            let _ = self
                .release_first_read
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .recv();
        }
        now
    }

    fn now_ms(&self) -> i64 {
        self.manual.now_ms()
    }
}

struct WakeCollector {
    descriptor: CollectorDescriptor,
    handles: SyncSender<WakeHandle>,
    snapshots: Arc<AtomicU64>,
}

impl Collector for WakeCollector {
    fn id(&self) -> &'static str {
        self.descriptor.id
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let _ = self.handles.send(context.wake);
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.snapshots.fetch_add(1, Ordering::AcqRel);
        Ok(Vec::new())
    }
}

fn wake_entry(
    descriptor: CollectorDescriptor,
    handles: SyncSender<WakeHandle>,
    snapshots: Arc<AtomicU64>,
) -> RegistryEntry {
    let factory_descriptor = descriptor.clone();
    RegistryEntry::active(descriptor, move || {
        Box::new(WakeCollector {
            descriptor: factory_descriptor.clone(),
            handles: handles.clone(),
            snapshots: Arc::clone(&snapshots),
        })
    })
}

#[test]
fn a_targeted_wake_runs_only_the_named_collector() {
    let (first_tx, first_rx) = sync_channel(2);
    let (second_tx, second_rx) = sync_channel(2);
    let first = Arc::new(AtomicU64::new(0));
    let second = Arc::new(AtomicU64::new(0));
    let registry = CollectorRegistry::new(vec![
        wake_entry(
            descriptor(
                "notify-first",
                CollectorCadence::Notify,
                CollectorCost::Cheap,
            ),
            first_tx,
            Arc::clone(&first),
        ),
        wake_entry(
            descriptor(
                "notify-second",
                CollectorCadence::Notify,
                CollectorCost::Cheap,
            ),
            second_tx,
            Arc::clone(&second),
        ),
    ]);
    let supervisor = system_runtime(
        registry,
        "collector-targeted-named",
        &["notify-first", "notify-second"],
    );
    supervisor.spawn_all().unwrap();
    let first_wake = first_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let _second_wake = second_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("both notification collectors' baseline snapshots", || {
        first.load(Ordering::Acquire) == 1 && second.load(Ordering::Acquire) == 1
    });

    first_wake.request_snapshot().unwrap();
    wait_for("the named collector's targeted snapshot", || {
        first.load(Ordering::Acquire) == 2
    });
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(second.load(Ordering::Acquire), 1);
    supervisor.stop_all();
}

struct BlockingWakeCollector {
    handles: SyncSender<WakeHandle>,
    entered: SyncSender<()>,
    release: Arc<Mutex<Receiver<()>>>,
    snapshots: Arc<AtomicU64>,
    completed: Arc<AtomicU64>,
}

impl Collector for BlockingWakeCollector {
    fn id(&self) -> &'static str {
        "blocked-notify"
    }

    fn descriptor(&self) -> CollectorDescriptor {
        descriptor(
            "blocked-notify",
            CollectorCadence::Notify,
            CollectorCost::Cheap,
        )
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let _ = self.handles.send(context.wake);
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.snapshots.fetch_add(1, Ordering::AcqRel);
        let _ = self.entered.send(());
        let _ = self
            .release
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .recv();
        self.completed.fetch_add(1, Ordering::AcqRel);
        Ok(Vec::new())
    }
}

fn assert_wakes_during_snapshot_coalesce(requests: usize, suffix: &str) {
    let (handle_tx, handle_rx) = sync_channel(1);
    let (entered_tx, entered_rx) = sync_channel(2);
    let (release_tx, release_rx) = sync_channel(2);
    let release_rx = Arc::new(Mutex::new(release_rx));
    let snapshots = Arc::new(AtomicU64::new(0));
    let completed = Arc::new(AtomicU64::new(0));
    let factory_release = Arc::clone(&release_rx);
    let factory_snapshots = Arc::clone(&snapshots);
    let factory_completed = Arc::clone(&completed);
    let entry = RegistryEntry::active(
        descriptor(
            "blocked-notify",
            CollectorCadence::Notify,
            CollectorCost::Cheap,
        ),
        move || {
            Box::new(BlockingWakeCollector {
                handles: handle_tx.clone(),
                entered: entered_tx.clone(),
                release: Arc::clone(&factory_release),
                snapshots: Arc::clone(&factory_snapshots),
                completed: Arc::clone(&factory_completed),
            })
        },
    );
    let supervisor = system_runtime(
        CollectorRegistry::new(vec![entry]),
        suffix,
        &["blocked-notify"],
    );
    supervisor.spawn_all().unwrap();
    let wake = handle_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("initial snapshot entered the deterministic block");

    for _ in 0..requests {
        wake.request_snapshot().unwrap();
    }
    release_tx.send(()).unwrap();
    entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("one trailing snapshot ran after the blocked snapshot");
    release_tx.send(()).unwrap();
    wait_for("both blocked snapshots to finish", || {
        completed.load(Ordering::Acquire) == 2
    });
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(snapshots.load(Ordering::Acquire), 2);
    supervisor.stop_all();
}

#[test]
fn a_wake_during_snapshot_preserves_exactly_one_trailing_run() {
    assert_wakes_during_snapshot_coalesce(1, "collector-targeted-trailing");
}

#[test]
fn one_hundred_wakes_coalesce_into_one_trailing_run() {
    assert_wakes_during_snapshot_coalesce(100, "collector-targeted-storm");
}

#[test]
fn stale_wake_handles_fail_closed_after_disable_pause_and_stop() {
    let (handles_tx, handles_rx) = sync_channel(8);
    let snapshots = Arc::new(AtomicU64::new(0));
    let entry = wake_entry(
        descriptor("notify", CollectorCadence::Notify, CollectorCost::Cheap),
        handles_tx,
        Arc::clone(&snapshots),
    );
    let supervisor = system_runtime(
        CollectorRegistry::new(vec![entry]),
        "collector-targeted-stale",
        &["notify"],
    );
    supervisor.spawn_all().unwrap();
    let first = handles_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("first context snapshot", || {
        snapshots.load(Ordering::Acquire) == 1
    });

    supervisor.set_enabled("notify", false).unwrap();
    assert!(first.request_snapshot().is_err());
    std::thread::sleep(Duration::from_millis(40));
    assert_eq!(snapshots.load(Ordering::Acquire), 1);

    supervisor.set_enabled("notify", true).unwrap();
    let second = handles_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("second context snapshot", || {
        snapshots.load(Ordering::Acquire) == 2
    });
    supervisor.set_monitoring_paused(true, None);
    assert!(second.request_snapshot().is_err());
    std::thread::sleep(Duration::from_millis(40));
    assert_eq!(snapshots.load(Ordering::Acquire), 2);

    supervisor.set_monitoring_paused(false, None);
    let third = handles_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("third context snapshot", || {
        snapshots.load(Ordering::Acquire) == 3
    });
    supervisor.stop_all();
    assert!(third.request_snapshot().is_err());
    std::thread::sleep(Duration::from_millis(40));
    assert_eq!(snapshots.load(Ordering::Acquire), 3);
}

struct FixedBackoff(Duration);

impl BackoffPolicy for FixedBackoff {
    fn delay(&self, _consecutive_failures: u64) -> Duration {
        self.0
    }
}

struct FailOnceWakeCollector {
    contexts: SyncSender<CollectorContext>,
    attempts: Arc<AtomicU64>,
    fail_next: Arc<AtomicBool>,
}

impl Collector for FailOnceWakeCollector {
    fn id(&self) -> &'static str {
        "failing-notify"
    }

    fn descriptor(&self) -> CollectorDescriptor {
        descriptor(
            "failing-notify",
            CollectorCadence::Notify,
            CollectorCost::Cheap,
        )
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let _ = self.contexts.send(context);
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.attempts.fetch_add(1, Ordering::AcqRel);
        if self.fail_next.swap(false, Ordering::AcqRel) {
            return Err(AppError::InvalidInput {
                what: "targeted wake test",
                reason: "injected first snapshot failure".to_owned(),
            });
        }
        Ok(Vec::new())
    }
}

#[test]
fn a_failed_instance_wake_is_rejected_and_its_replacement_obeys_backoff() {
    let clock = Arc::new(ManualClock::new());
    let (failing_tx, failing_rx) = sync_channel(4);
    let (signal_tx, signal_rx) = sync_channel(2);
    let attempts = Arc::new(AtomicU64::new(0));
    let fail_next = Arc::new(AtomicBool::new(true));
    let signal_snapshots = Arc::new(AtomicU64::new(0));
    let factory_attempts = Arc::clone(&attempts);
    let factory_fail_next = Arc::clone(&fail_next);
    let registry = CollectorRegistry::new(vec![
        RegistryEntry::active(
            descriptor(
                "failing-notify",
                CollectorCadence::Notify,
                CollectorCost::Cheap,
            ),
            move || {
                Box::new(FailOnceWakeCollector {
                    contexts: failing_tx.clone(),
                    attempts: Arc::clone(&factory_attempts),
                    fail_next: Arc::clone(&factory_fail_next),
                })
            },
        ),
        wake_entry(
            descriptor("signal", CollectorCadence::Notify, CollectorCost::Cheap),
            signal_tx,
            Arc::clone(&signal_snapshots),
        ),
    ]);
    let supervisor = runtime(
        registry,
        "collector-targeted-backoff",
        &["failing-notify", "signal"],
        QueryBusy::default(),
        Arc::clone(&clock) as Arc<dyn RuntimeClock>,
        Arc::new(FixedBackoff(Duration::from_secs(10))),
    );
    supervisor.spawn_all().unwrap();
    let failing_context = failing_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let signal_wake = signal_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("the injected failure to enter backoff", || {
        attempts.load(Ordering::Acquire) == 1
            && supervisor.list_collector_status()[0].consecutive_failures == 1
    });

    assert!(
        failing_context.wake.request_snapshot().is_err(),
        "a provider failure fences its instance callbacks before backoff",
    );
    assert!(
        failing_context.emit.emit(Vec::new()).is_err(),
        "a provider failure fences the old instance's emission callback too",
    );
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(attempts.load(Ordering::Acquire), 1);

    clock.advance(Duration::from_secs(10));
    signal_wake.request_snapshot().unwrap();
    let replacement_context = failing_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("the retry at the fake-clock backoff deadline", || {
        attempts.load(Ordering::Acquire) == 2
    });
    replacement_context.wake.request_snapshot().unwrap();
    replacement_context.emit.emit(Vec::new()).unwrap();
    wait_for("the replacement instance's targeted wake", || {
        attempts.load(Ordering::Acquire) == 3
    });
    assert_eq!(
        attempts.load(Ordering::Acquire),
        3,
        "only the replacement instance can request another snapshot",
    );
    supervisor.stop_all();
}

#[test]
fn a_poll_wake_is_a_noop_and_does_not_change_its_fake_clock_cadence() {
    let clock = Arc::new(ManualClock::new());
    let (poll_tx, poll_rx) = sync_channel(2);
    let (signal_tx, signal_rx) = sync_channel(2);
    let poll_snapshots = Arc::new(AtomicU64::new(0));
    let signal_snapshots = Arc::new(AtomicU64::new(0));
    let registry = CollectorRegistry::new(vec![
        wake_entry(
            descriptor(
                "poll",
                CollectorCadence::Poll(Duration::from_secs(60)),
                CollectorCost::Cheap,
            ),
            poll_tx,
            Arc::clone(&poll_snapshots),
        ),
        wake_entry(
            descriptor("signal", CollectorCadence::Notify, CollectorCost::Cheap),
            signal_tx,
            Arc::clone(&signal_snapshots),
        ),
    ]);
    let supervisor = runtime(
        registry,
        "collector-targeted-poll",
        &["poll", "signal"],
        QueryBusy::default(),
        Arc::clone(&clock) as Arc<dyn RuntimeClock>,
        Arc::new(ExponentialBackoff),
    );
    supervisor.spawn_all().unwrap();
    let poll_wake = poll_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let signal_wake = signal_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("poll and signal baselines", || {
        poll_snapshots.load(Ordering::Acquire) == 1 && signal_snapshots.load(Ordering::Acquire) == 1
    });

    poll_wake.request_snapshot().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(poll_snapshots.load(Ordering::Acquire), 1);

    clock.advance(Duration::from_secs(59));
    signal_wake.request_snapshot().unwrap();
    wait_for("the signal collector at fake second 59", || {
        signal_snapshots.load(Ordering::Acquire) == 2
    });
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(poll_snapshots.load(Ordering::Acquire), 1);

    clock.advance(Duration::from_secs(1));
    signal_wake.request_snapshot().unwrap();
    wait_for("the unchanged poll deadline", || {
        poll_snapshots.load(Ordering::Acquire) == 2
    });
    supervisor.stop_all();
}

struct ActiveSnapshot<'a>(&'a AtomicUsize);

impl Drop for ActiveSnapshot<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

struct OverlapCollector {
    label: &'static str,
    handles: SyncSender<WakeHandle>,
    snapshots: Arc<AtomicU64>,
    active: Arc<AtomicUsize>,
    max_active: Arc<AtomicUsize>,
    block_next: Arc<AtomicBool>,
    panic_next: Arc<AtomicBool>,
    entered: SyncSender<&'static str>,
    release: Arc<Mutex<Receiver<()>>>,
}

impl Collector for OverlapCollector {
    fn id(&self) -> &'static str {
        "expensive"
    }

    fn descriptor(&self) -> CollectorDescriptor {
        descriptor(
            "expensive",
            CollectorCadence::NotifyPlusSweep(Duration::from_secs(3_600)),
            CollectorCost::Expensive,
        )
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let _ = self.handles.send(context.wake);
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        let active = self.active.fetch_add(1, Ordering::AcqRel) + 1;
        let _active_snapshot = ActiveSnapshot(&self.active);
        self.max_active.fetch_max(active, Ordering::AcqRel);
        self.snapshots.fetch_add(1, Ordering::AcqRel);
        if self.panic_next.swap(false, Ordering::AcqRel) {
            panic!("injected expensive snapshot panic");
        }
        if self.block_next.swap(false, Ordering::AcqRel) {
            let _ = self.entered.send(self.label);
            let _ = self
                .release
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .recv();
        }
        Ok(Vec::new())
    }
}

#[test]
fn targeted_expensive_snapshots_never_overlap_and_a_panic_releases_the_lease() {
    let (first_tx, first_rx) = sync_channel(2);
    let (second_tx, second_rx) = sync_channel(2);
    let (entered_tx, entered_rx) = sync_channel(2);
    let (release_tx, release_rx) = sync_channel(2);
    let release_rx = Arc::new(Mutex::new(release_rx));
    let first_snapshots = Arc::new(AtomicU64::new(0));
    let second_snapshots = Arc::new(AtomicU64::new(0));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let first_block = Arc::new(AtomicBool::new(false));
    let second_block = Arc::new(AtomicBool::new(false));
    let first_panic = Arc::new(AtomicBool::new(false));
    let second_panic = Arc::new(AtomicBool::new(false));
    let expensive_descriptor = descriptor(
        "expensive",
        CollectorCadence::NotifyPlusSweep(Duration::from_secs(3_600)),
        CollectorCost::Expensive,
    );
    let make_entry = |label,
                      handles: SyncSender<WakeHandle>,
                      snapshots: Arc<AtomicU64>,
                      block_next: Arc<AtomicBool>,
                      panic_next: Arc<AtomicBool>| {
        let descriptor = expensive_descriptor.clone();
        let active = Arc::clone(&active);
        let max_active = Arc::clone(&max_active);
        let entered = entered_tx.clone();
        let release = Arc::clone(&release_rx);
        RegistryEntry::active(descriptor, move || {
            Box::new(OverlapCollector {
                label,
                handles: handles.clone(),
                snapshots: Arc::clone(&snapshots),
                active: Arc::clone(&active),
                max_active: Arc::clone(&max_active),
                block_next: Arc::clone(&block_next),
                panic_next: Arc::clone(&panic_next),
                entered: entered.clone(),
                release: Arc::clone(&release),
            })
        })
    };
    // Duplicate IDs are deliberate here: both workers receive the same zero
    // stagger, creating the adversarial simultaneous-due condition needed to
    // exercise the runtime-wide lease rather than the stagger policy.
    let registry = CollectorRegistry::new(vec![
        make_entry(
            "first",
            first_tx,
            Arc::clone(&first_snapshots),
            Arc::clone(&first_block),
            Arc::clone(&first_panic),
        ),
        make_entry(
            "second",
            second_tx,
            Arc::clone(&second_snapshots),
            Arc::clone(&second_block),
            Arc::clone(&second_panic),
        ),
    ]);
    let supervisor = system_runtime(
        registry,
        "collector-targeted-expensive-lease",
        &["expensive"],
    );
    supervisor.spawn_all().unwrap();
    let first_wake = first_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let second_wake = second_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("both expensive baseline snapshots", || {
        first_snapshots.load(Ordering::Acquire) == 1
            && second_snapshots.load(Ordering::Acquire) == 1
    });

    first_block.store(true, Ordering::Release);
    second_block.store(true, Ordering::Release);
    first_wake.request_snapshot().unwrap();
    second_wake.request_snapshot().unwrap();
    entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("one expensive targeted snapshot entered");
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(active.load(Ordering::Acquire), 1);
    assert_eq!(max_active.load(Ordering::Acquire), 1);

    release_tx.send(()).unwrap();
    entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("the other expensive targeted snapshot entered after release");
    assert_eq!(max_active.load(Ordering::Acquire), 1);
    release_tx.send(()).unwrap();
    wait_for("both expensive targeted snapshots", || {
        first_snapshots.load(Ordering::Acquire) == 2
            && second_snapshots.load(Ordering::Acquire) == 2
            && active.load(Ordering::Acquire) == 0
    });

    first_panic.store(true, Ordering::Release);
    first_wake.request_snapshot().unwrap();
    wait_for("the injected expensive panic", || {
        first_snapshots.load(Ordering::Acquire) == 3
    });
    second_wake.request_snapshot().unwrap();
    wait_for(
        "the healthy expensive worker after its neighbour panicked",
        || second_snapshots.load(Ordering::Acquire) == 3,
    );
    assert_eq!(max_active.load(Ordering::Acquire), 1);
    supervisor.stop_all();
}

#[test]
fn pending_wakes_are_cleared_before_disable_and_pause_transition_staggers() {
    const TARGET: &str = "z-expensive-target";
    let (initialized_tx, initialized_rx) = sync_channel(1);
    let (release_initial_read_tx, release_initial_read_rx) = sync_channel(1);
    let clock = Arc::new(WorkerObservedClock::new(
        "collector-z-expensive-target",
        initialized_tx,
        release_initial_read_rx,
    ));
    let query_busy = QueryBusy::default();
    let (target_tx, target_rx) = sync_channel(8);
    let (signal_tx, signal_rx) = sync_channel(8);
    let target_snapshots = Arc::new(AtomicU64::new(0));
    let signal_snapshots = Arc::new(AtomicU64::new(0));
    let registry = CollectorRegistry::new(vec![
        RegistryEntry::unavailable(descriptor(
            "a-expensive-anchor",
            CollectorCadence::NotifyPlusSweep(Duration::from_secs(3_600)),
            CollectorCost::Expensive,
        )),
        wake_entry(
            descriptor(
                TARGET,
                CollectorCadence::NotifyPlusSweep(Duration::from_secs(3_600)),
                CollectorCost::Expensive,
            ),
            target_tx,
            Arc::clone(&target_snapshots),
        ),
        wake_entry(
            descriptor("signal", CollectorCadence::Notify, CollectorCost::Cheap),
            signal_tx,
            Arc::clone(&signal_snapshots),
        ),
    ]);
    let supervisor = runtime(
        registry,
        "collector-targeted-transition-clear",
        &["a-expensive-anchor", TARGET, "signal"],
        query_busy.clone(),
        Arc::clone(&clock) as Arc<dyn RuntimeClock>,
        Arc::new(ExponentialBackoff),
    );
    supervisor.spawn_all().unwrap();
    let mut signal_wake = signal_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    initialized_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("the expensive target established its initial deadline");
    clock.advance(Duration::from_secs(15));
    release_initial_read_tx.send(()).unwrap();
    signal_wake.request_snapshot().unwrap();
    let first_target = target_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("the target's initial assigned-stagger snapshot", || {
        target_snapshots.load(Ordering::Acquire) == 1
    });

    let busy = query_busy.enter();
    first_target.request_snapshot().unwrap();
    supervisor.set_enabled(TARGET, false).unwrap();
    supervisor.set_enabled(TARGET, true).unwrap();
    drop(busy);
    std::thread::sleep(Duration::from_millis(75));
    assert_eq!(
        target_snapshots.load(Ordering::Acquire),
        1,
        "a pre-disable pending wake bypassed the re-enable stagger",
    );
    clock.advance(Duration::from_secs(15));
    signal_wake.request_snapshot().unwrap();
    let second_target = target_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("the re-enabled assigned-stagger snapshot", || {
        target_snapshots.load(Ordering::Acquire) == 2
    });

    let busy = query_busy.enter();
    second_target.request_snapshot().unwrap();
    supervisor.set_monitoring_paused(true, None);
    supervisor.set_monitoring_paused(false, None);
    drop(busy);
    signal_wake = signal_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("monitor resume created a freshly fenced signal context");
    std::thread::sleep(Duration::from_millis(75));
    assert_eq!(
        target_snapshots.load(Ordering::Acquire),
        2,
        "a pre-pause pending wake bypassed the resume stagger",
    );
    clock.advance(Duration::from_secs(15));
    signal_wake.request_snapshot().unwrap();
    wait_for("the resumed assigned-stagger snapshot", || {
        target_snapshots.load(Ordering::Acquire) == 3
    });
    supervisor.stop_all();
}
