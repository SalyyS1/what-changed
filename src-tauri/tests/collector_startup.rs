//! Startup/task collector acceptance checks with a provider-injected inventory.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use what_changed_lib::collectors::startup::{
    self, RunEntry, ScheduledTask, StartupCollector, StartupFile, StartupInventory,
    StartupLocation, StartupNotificationGuard, StartupProvider, TaskTriggerKind,
};
use what_changed_lib::collectors::stateful::DOWNTIME_GAP_MS;
use what_changed_lib::collectors::testing::live_context;
use what_changed_lib::collectors::{
    CancelToken, Collector, CollectorSnapshot, ObservationClock, RawObservation, SnapshotContext,
    WakeHandle,
};
use what_changed_lib::domain::{EventType, Importance};
use what_changed_lib::error::AppResult;
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{baseline_repository, Baseline, CollectorCommitReceipt};

#[derive(Clone)]
struct ScriptedStartup {
    inventories: Arc<Mutex<VecDeque<StartupInventory>>>,
    registrations: Arc<AtomicU64>,
    drops: Arc<AtomicU64>,
}

impl ScriptedStartup {
    fn new(initial: StartupInventory) -> Self {
        Self {
            inventories: Arc::new(Mutex::new(VecDeque::from([initial]))),
            registrations: Arc::new(AtomicU64::new(0)),
            drops: Arc::new(AtomicU64::new(0)),
        }
    }

    fn script(&self, inventory: StartupInventory) {
        *lock(&self.inventories) = VecDeque::from([inventory]);
    }
}

impl StartupProvider for ScriptedStartup {
    fn inventory(&self, cancel: &CancelToken, _deadline: Instant) -> AppResult<StartupInventory> {
        if cancel.is_cancelled() {
            return Ok(StartupInventory::default());
        }
        let mut inventories = lock(&self.inventories);
        if inventories.len() > 1 {
            Ok(inventories.pop_front().unwrap_or_default())
        } else {
            Ok(inventories.front().cloned().unwrap_or_default())
        }
    }

    fn register_notifications(
        &self,
        _wake: WakeHandle,
        _cancel: CancelToken,
    ) -> AppResult<Box<dyn StartupNotificationGuard>> {
        self.registrations.fetch_add(1, Ordering::AcqRel);
        Ok(Box::new(TestGuard(self.drops.clone())))
    }
}

struct TestGuard(Arc<AtomicU64>);

impl StartupNotificationGuard for TestGuard {}

impl Drop for TestGuard {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }
}

#[derive(Clone)]
struct ManualClock(Arc<AtomicI64>);

impl ManualClock {
    fn at(now_ms: i64) -> Self {
        Self(Arc::new(AtomicI64::new(now_ms)))
    }
}

impl ObservationClock for ManualClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }
}

type UnderTest = StartupCollector<ScriptedStartup, ManualClock>;

fn task(index: usize, trigger: TaskTriggerKind) -> ScheduledTask {
    ScheduledTask {
        name: format!("Fixture task {index}"),
        path: format!(r"\Fixture\Task-{index}"),
        enabled: true,
        triggers: vec![trigger],
        action_path: Some(format!(r"C:\Fixture\task-{index}.exe")),
        last_run_ms: Some(1_700_000_000_000),
        last_result: 0,
    }
}

fn run_entry(name: &str) -> RunEntry {
    RunEntry {
        location: StartupLocation::CurrentUserRun,
        name: name.to_owned(),
        command: format!(
            r#""C:\Fixture\{name}.exe" --token ghp_abcdefghijklmnopqrstuvwxyz0123456789"#
        ),
        approved: Some(true),
    }
}

fn startup_file(name: &str) -> StartupFile {
    StartupFile {
        location: StartupLocation::CurrentUserStartupFolder,
        path: format!(r"C:\Users\fixture\Startup\{name}.lnk"),
        name: format!("{name}.lnk"),
        approved: Some(true),
    }
}

fn started(initial: StartupInventory) -> (UnderTest, ScriptedStartup) {
    let provider = ScriptedStartup::new(initial);
    let mut collector = StartupCollector::with_clock(
        provider.clone(),
        startup::catalog_descriptor().unwrap(),
        ManualClock::at(1_700_000_000_000),
    );
    collector
        .start(live_context(startup::ID).unwrap().context)
        .unwrap();
    (collector, provider)
}

fn cycle<P: StartupProvider, C: ObservationClock>(
    collector: &mut StartupCollector<P, C>,
    token_value: u64,
) -> CollectorSnapshot {
    let snapshot = collector
        .snapshot_batch(SnapshotContext::for_devtools(token_value))
        .unwrap();
    if let CollectorSnapshot::Atomic { token, .. } = &snapshot {
        collector
            .commit_succeeded(*token, &CollectorCommitReceipt { event_ids: vec![] })
            .unwrap();
    }
    snapshot
}

fn observations(snapshot: &CollectorSnapshot) -> &[RawObservation] {
    match snapshot {
        CollectorSnapshot::Atomic { observations, .. } => observations,
        CollectorSnapshot::Noop => &[],
        other => panic!("expected atomic/noop snapshot, got {other:?}"),
    }
}

#[test]
fn first_cycle_is_silent_and_task_lifecycle_is_distinct() {
    let (mut collector, provider) = started(StartupInventory::default());
    assert!(observations(&cycle(&mut collector, 1)).is_empty());

    let original = task(1, TaskTriggerKind::Logon);
    provider.script(StartupInventory {
        tasks: vec![original.clone()],
        ..Default::default()
    });
    let added = cycle(&mut collector, 2);
    assert_eq!(observations(&added)[0].kind.event_type, EventType::Created);
    assert_eq!(
        observations(&added)[0].details.as_ref().unwrap()["startup"],
        true
    );

    let mut disabled = original.clone();
    disabled.enabled = false;
    provider.script(StartupInventory {
        tasks: vec![disabled.clone()],
        ..Default::default()
    });
    assert_eq!(
        observations(&cycle(&mut collector, 3))[0].kind.event_type,
        EventType::Disabled
    );

    disabled.action_path = Some(r"C:\Fixture\replacement.exe".to_owned());
    provider.script(StartupInventory {
        tasks: vec![disabled],
        ..Default::default()
    });
    assert_eq!(
        observations(&cycle(&mut collector, 4))[0].kind.event_type,
        EventType::Modified
    );

    provider.script(StartupInventory::default());
    assert_eq!(
        observations(&cycle(&mut collector, 5))[0].kind.event_type,
        EventType::Deleted
    );
}

#[test]
fn only_boot_logon_and_idle_tasks_are_marked_as_startup() {
    let tasks = (0..194)
        .map(|index| {
            let trigger = if index < 10 {
                TaskTriggerKind::Boot
            } else if index < 20 {
                TaskTriggerKind::Logon
            } else if index < 28 {
                TaskTriggerKind::Idle
            } else {
                TaskTriggerKind::Schedule
            };
            task(index, trigger)
        })
        .collect();
    let (mut collector, provider) = started(StartupInventory::default());
    cycle(&mut collector, 1);
    provider.script(StartupInventory {
        tasks,
        ..Default::default()
    });
    let snapshot = cycle(&mut collector, 2);
    let events = observations(&snapshot);
    assert_eq!(events.len(), 194);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.details.as_ref().unwrap()["startup"] == true)
            .count(),
        28
    );
    assert!(events.iter().all(|event| {
        what_changed_lib::collectors::normalize::Normalizer::new()
            .normalize(event.clone())
            .get()
            .importance
            == Importance::Informational
    }));
}

#[test]
fn run_key_and_startup_folder_additions_use_startup_entry_events() {
    let (mut collector, provider) = started(StartupInventory::default());
    cycle(&mut collector, 1);
    provider.script(StartupInventory {
        run_entries: vec![run_entry("FixtureAgent")],
        startup_files: vec![startup_file("FixtureShortcut")],
        ..Default::default()
    });
    let snapshot = cycle(&mut collector, 2);
    assert_eq!(observations(&snapshot).len(), 2);
    assert!(observations(&snapshot)
        .iter()
        .all(|event| event.kind.event_type == EventType::Created));
    assert!(observations(&snapshot).iter().all(|event| event
        .kind
        .user_friendly_summary
        .contains("startup entry added")));
}

#[test]
fn run_command_and_startup_approval_changes_are_single_truthful_modifications() {
    let original_run = run_entry("MutableAgent");
    let original_file = startup_file("MutableShortcut");
    let (mut collector, provider) = started(StartupInventory {
        run_entries: vec![original_run.clone()],
        startup_files: vec![original_file.clone()],
        ..Default::default()
    });
    cycle(&mut collector, 1);
    let mut changed_run = original_run;
    changed_run.command = r"C:\Fixture\replacement.exe --safe".to_owned();
    let mut changed_file = original_file;
    changed_file.approved = Some(false);
    provider.script(StartupInventory {
        run_entries: vec![changed_run],
        startup_files: vec![changed_file],
        ..Default::default()
    });
    let snapshot = cycle(&mut collector, 2);
    assert_eq!(observations(&snapshot).len(), 2);
    assert!(observations(&snapshot).iter().all(|event| {
        event.kind.event_type == EventType::ConfigurationChanged
            && event.details.as_ref().unwrap()["change_kind"] == "modified"
    }));
}

#[test]
fn unavailable_task_store_preserves_tasks_but_still_diffs_run_entries() {
    let scheduled = task(1, TaskTriggerKind::Logon);
    let (mut collector, provider) = started(StartupInventory {
        tasks: vec![scheduled],
        ..Default::default()
    });
    cycle(&mut collector, 1);
    let readable_addition = task(2, TaskTriggerKind::Boot);
    provider.script(StartupInventory {
        tasks: vec![readable_addition],
        run_entries: vec![run_entry("StillCollected")],
        task_store_available: false,
        ..Default::default()
    });
    let snapshot = cycle(&mut collector, 2);
    assert_eq!(observations(&snapshot).len(), 2);
    assert!(observations(&snapshot)
        .iter()
        .any(|event| event.kind.user_friendly_summary.contains("StillCollected")));
    assert!(observations(&snapshot)
        .iter()
        .any(|event| event.kind.user_friendly_summary.contains("Fixture task 2")));
}

#[test]
fn task_runtime_metadata_advances_the_baseline_without_emitting_noise() {
    let original = task(1, TaskTriggerKind::Schedule);
    let (mut collector, provider) = started(StartupInventory {
        tasks: vec![original.clone()],
        ..Default::default()
    });
    cycle(&mut collector, 1);
    let mut ran = original;
    ran.last_run_ms = Some(1_700_000_100_000);
    ran.last_result = 1;
    provider.script(StartupInventory {
        tasks: vec![ran],
        ..Default::default()
    });
    let snapshot = cycle(&mut collector, 2);
    assert!(observations(&snapshot).is_empty());
    assert!(matches!(snapshot, CollectorSnapshot::Atomic { .. }));
    assert_eq!(cycle(&mut collector, 3), CollectorSnapshot::Noop);
}

#[test]
fn durable_startup_baseline_never_contains_raw_command_credentials() {
    let (mut collector, _) = started(StartupInventory {
        run_entries: vec![run_entry("SecretAgent")],
        ..Default::default()
    });
    let encoded = match cycle(&mut collector, 1) {
        CollectorSnapshot::Atomic {
            baseline: Some(baseline),
            ..
        } => serde_json::to_string(&baseline.snapshot).unwrap(),
        other => panic!("missing baseline: {other:?}"),
    };
    assert!(!encoded.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789"));
    assert!(encoded.contains("[REDACTED]"));
}

#[test]
fn rejected_commit_retries_the_same_startup_change() {
    let (mut collector, provider) = started(StartupInventory::default());
    cycle(&mut collector, 1);
    provider.script(StartupInventory {
        run_entries: vec![run_entry("RetryAgent")],
        ..Default::default()
    });
    let snapshot = collector
        .snapshot_batch(SnapshotContext::for_devtools(2))
        .unwrap();
    let token = match snapshot {
        CollectorSnapshot::Atomic { token, .. } => token,
        other => panic!("expected atomic snapshot, got {other:?}"),
    };
    collector.commit_failed(token).unwrap();
    let retry = cycle(&mut collector, 2);
    assert_eq!(observations(&retry).len(), 1);
    assert!(observations(&retry)[0]
        .kind
        .user_friendly_summary
        .contains("RetryAgent"));
}

#[test]
fn notification_guard_lives_exactly_until_collector_stop() {
    let (mut collector, provider) = started(StartupInventory::default());
    assert_eq!(provider.registrations.load(Ordering::Acquire), 1);
    assert_eq!(provider.drops.load(Ordering::Acquire), 0);
    collector.stop();
    assert_eq!(provider.drops.load(Ordering::Acquire), 1);
}

#[test]
fn run_entry_added_while_stopped_is_reported_with_uncertain_time() {
    let live = live_context(startup::ID).unwrap();
    let first_provider = ScriptedStartup::new(StartupInventory::default());
    let mut first = StartupCollector::with_clock(
        first_provider,
        startup::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    first.start(live.context.clone()).unwrap();
    let baseline = match cycle(&mut first, 1) {
        CollectorSnapshot::Atomic {
            baseline: Some(baseline),
            ..
        } => Baseline {
            collector_id: startup::ID.to_owned(),
            entity_kind: baseline.entity_kind,
            snapshot: baseline.snapshot,
            captured_at: baseline.captured_at,
            truncated: false,
        },
        other => panic!("missing startup baseline: {other:?}"),
    };
    live.db
        .write(move |connection| {
            baseline_repository::store(connection, &Sanitizer::new().sanitize(baseline).0)
        })
        .unwrap();
    first.stop();

    let resumed_provider = ScriptedStartup::new(StartupInventory {
        run_entries: vec![run_entry("DowntimeAgent")],
        ..Default::default()
    });
    let mut resumed = StartupCollector::with_clock(
        resumed_provider,
        startup::catalog_descriptor().unwrap(),
        ManualClock::at(1_000 + DOWNTIME_GAP_MS + 1),
    );
    resumed.start(live.context).unwrap();
    let detected = cycle(&mut resumed, 2);
    assert_eq!(observations(&detected).len(), 1);
    assert!(observations(&detected)[0].occurred_at_is_uncertain);
}

#[cfg(windows)]
#[test]
#[ignore = "real Task Scheduler p95 gate"]
fn real_task_scheduler_sweep_stays_inside_phase_budget() {
    use std::time::Duration;
    use what_changed_lib::platform::windows::tasks::WindowsStartupProvider;

    let provider = WindowsStartupProvider::new().unwrap();
    let cancel = CancelToken::default();
    let mut samples = Vec::with_capacity(20);
    let mut task_count = 0usize;
    let mut task_store_available = true;
    for _ in 0..20 {
        let started = Instant::now();
        let inventory = provider
            .inventory(&cancel, started + Duration::from_secs(15))
            .unwrap();
        samples.push(started.elapsed());
        task_count = inventory.tasks.len();
        task_store_available &= inventory.task_store_available;
    }
    samples.sort_unstable();
    let p95 = samples[(samples.len() * 95).div_ceil(100) - 1];
    eprintln!(
        "Task Scheduler rows={task_count}; available={task_store_available}; warm p95={p95:?}"
    );
    assert!(
        task_count > 0 || !task_store_available,
        "an available real task store should not be empty"
    );
    assert!(p95 <= Duration::from_millis(2_500), "p95 was {p95:?}");
}

#[derive(Clone, Default)]
struct StallingStartup {
    calls: Arc<AtomicU64>,
}

impl StartupProvider for StallingStartup {
    fn inventory(&self, _cancel: &CancelToken, _deadline: Instant) -> AppResult<StartupInventory> {
        if self.calls.fetch_add(1, Ordering::AcqRel) == 0 {
            std::thread::sleep(std::time::Duration::from_millis(15_300));
        }
        Ok(StartupInventory::default())
    }
}

#[test]
#[ignore = "forced fifteen-second timeout gate"]
fn hard_timeout_abandons_a_stall_without_overlapping_the_query() {
    let provider = StallingStartup::default();
    let calls = provider.calls.clone();
    let mut collector = StartupCollector::with_clock(
        provider,
        startup::catalog_descriptor().unwrap(),
        ManualClock::at(1_700_000_000_000),
    );
    collector
        .start(live_context(startup::ID).unwrap().context)
        .unwrap();
    let started = Instant::now();
    let error = collector
        .snapshot_batch(SnapshotContext::for_devtools(1))
        .unwrap_err();
    let elapsed = started.elapsed();
    assert!(error.to_string().contains("fifteen-second"));
    assert!(elapsed >= std::time::Duration::from_millis(14_500));
    assert!(elapsed < std::time::Duration::from_millis(15_500));
    assert_eq!(
        collector
            .snapshot_batch(SnapshotContext::for_devtools(2))
            .unwrap(),
        CollectorSnapshot::Noop
    );
    assert_eq!(calls.load(Ordering::Acquire), 1, "no overlapping query");
    std::thread::sleep(std::time::Duration::from_millis(500));
    assert!(observations(&cycle(&mut collector, 3)).is_empty());
    assert_eq!(calls.load(Ordering::Acquire), 2);
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
