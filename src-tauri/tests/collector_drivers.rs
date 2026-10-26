//! Driver collector acceptance checks with an injected provider.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::drivers::{
    self, compare_versions, DriverCollector, DriverProvider, DriverQueryCoordinator, DriverRecord,
    DriverSnapshot, DriverWakeTier,
};
use what_changed_lib::collectors::runtime::{
    BackoffPolicy, CollectorRuntimeConfig, RuntimeClock, SystemClock,
};
use what_changed_lib::collectors::stateful::DOWNTIME_GAP_MS;
use what_changed_lib::collectors::testing::live_context;
use what_changed_lib::collectors::{registry::CollectorRegistry, registry::RegistryEntry};
use what_changed_lib::collectors::{runtime::CollectorSupervisor, schedule::QueryBusy};
use what_changed_lib::collectors::{
    CancelToken, Collector, CollectorSnapshot, ObservationClock, SnapshotContext, WakeHandle,
};
use what_changed_lib::domain::{
    CollectorStatusDetails, CollectorWakeTier, EventType, MonitoringProfile,
};
use what_changed_lib::error::{AppError, AppResult};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{baseline_repository, Baseline, CollectorCommitReceipt, Db};

#[derive(Clone)]
struct ScriptedDrivers {
    tables: Arc<Mutex<VecDeque<Vec<DriverRecord>>>>,
    failure: Arc<Mutex<Option<String>>>,
    tier: DriverWakeTier,
    registrations: Arc<AtomicU64>,
    unregisters: Arc<AtomicU64>,
}

impl ScriptedDrivers {
    fn new(tables: Vec<Vec<DriverRecord>>) -> Self {
        Self {
            tables: Arc::new(Mutex::new(tables.into())),
            failure: Arc::new(Mutex::new(None)),
            tier: DriverWakeTier::TimerOnly,
            registrations: Arc::new(AtomicU64::new(0)),
            unregisters: Arc::new(AtomicU64::new(0)),
        }
    }

    fn with_tier(mut self, tier: DriverWakeTier) -> Self {
        self.tier = tier;
        self
    }

    fn script(&self, rows: Vec<DriverRecord>) {
        *lock(&self.failure) = None;
        *lock(&self.tables) = VecDeque::from([rows]);
    }

    fn fail(&self, error: AppError) {
        *lock(&self.failure) = Some(error.to_string());
    }
}

impl DriverProvider for ScriptedDrivers {
    fn drivers(&self, cancel: &CancelToken, _deadline: Instant) -> AppResult<Vec<DriverRecord>> {
        if cancel.is_cancelled() {
            return Ok(Vec::new());
        }
        if let Some(reason) = lock(&self.failure).clone() {
            return Err(AppError::Platform {
                operation: "scripted driver provider",
                reason,
            });
        }
        let mut tables = lock(&self.tables);
        if tables.len() > 1 {
            Ok(tables.pop_front().unwrap_or_default())
        } else {
            Ok(tables.front().cloned().unwrap_or_default())
        }
    }

    fn register_wake(&self, _wake: WakeHandle, _cancel: CancelToken) -> DriverWakeTier {
        self.registrations.fetch_add(1, Ordering::AcqRel);
        self.tier
    }

    fn unregister_wake(&self) {
        self.unregisters.fetch_add(1, Ordering::AcqRel);
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

fn driver(version: Option<&str>) -> DriverRecord {
    DriverRecord {
        device_instance_id: r"PCI\VEN_10DE&DEV_2684\FIXTURE".to_owned(),
        device_description: "NVIDIA GeForce RTX fixture".to_owned(),
        class_name: "Display".to_owned(),
        manufacturer: "NVIDIA Corporation".to_owned(),
        driver_version: version.map(str::to_owned),
        driver_date: Some("2026-08-29".to_owned()),
        provider_name: Some("NVIDIA".to_owned()),
    }
}

fn observations(snapshot: &CollectorSnapshot) -> &[what_changed_lib::collectors::RawObservation] {
    match snapshot {
        CollectorSnapshot::Atomic { observations, .. } => observations,
        CollectorSnapshot::Noop => &[],
        CollectorSnapshot::Legacy(_) => panic!("driver collector must use atomic snapshots"),
    }
}

fn cycle<P: DriverProvider, C: ObservationClock>(
    collector: &mut DriverCollector<P, C>,
    token: u64,
) -> CollectorSnapshot {
    let snapshot = collector
        .snapshot_batch(SnapshotContext::for_devtools(token))
        .unwrap();
    if let CollectorSnapshot::Atomic { token, .. } = &snapshot {
        collector
            .commit_succeeded(*token, &CollectorCommitReceipt { event_ids: vec![] })
            .unwrap();
    }
    snapshot
}

#[test]
fn install_update_rollback_and_remove_are_distinct() {
    let provider = ScriptedDrivers::new(vec![Vec::new()]);
    let mut collector = DriverCollector::with_clock(
        provider.clone(),
        drivers::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(drivers::ID).unwrap().context)
        .unwrap();
    assert!(observations(&cycle(&mut collector, 1)).is_empty());

    provider.script(vec![driver(Some("566.36"))]);
    let installed = cycle(&mut collector, 2);
    assert_eq!(
        observations(&installed)[0].kind.event_type,
        EventType::Installed
    );

    provider.script(vec![driver(Some("566.45"))]);
    let updated = cycle(&mut collector, 3);
    assert_eq!(
        observations(&updated)[0].kind.event_type,
        EventType::Updated
    );
    assert_eq!(
        observations(&updated)[0].kind.user_friendly_summary,
        "NVIDIA graphics driver updated 566.36 → 566.45"
    );
    assert_eq!(
        observations(&updated)[0].details.as_ref().unwrap()["change_kind"],
        "updated"
    );

    provider.script(vec![driver(Some("565.90"))]);
    let rollback = cycle(&mut collector, 4);
    assert_eq!(
        observations(&rollback)[0].kind.event_type,
        EventType::VersionChanged
    );
    assert_eq!(
        observations(&rollback)[0].details.as_ref().unwrap()["change_kind"],
        "rolled_back"
    );
    assert!(observations(&rollback)[0]
        .kind
        .user_friendly_summary
        .contains("rolled back 566.45 → 565.90"));

    provider.script(Vec::new());
    let removed = cycle(&mut collector, 5);
    assert_eq!(
        observations(&removed)[0].kind.event_type,
        EventType::Uninstalled
    );
}

#[test]
fn version_comparison_is_numeric_then_lexical_and_missing_safe() {
    use what_changed_lib::collectors::drivers::VersionDirection;

    assert_eq!(
        compare_versions(Some("31.0.9"), Some("31.0.15")),
        VersionDirection::Increased
    );
    assert_eq!(
        compare_versions(Some("2.0"), Some("1.99")),
        VersionDirection::Decreased
    );
    assert_eq!(
        compare_versions(Some("1.2.3-hp"), Some("1.2.4-hp")),
        VersionDirection::Increased
    );
    assert_eq!(
        compare_versions(None, Some("1.0")),
        VersionDirection::Unknown
    );
}

#[test]
fn baseline_is_silent_and_restart_does_not_replay() {
    let live = live_context(drivers::ID).unwrap();
    let row = driver(Some("566.45"));
    let snapshot = DriverSnapshot {
        acknowledged: BTreeMap::from([(row.device_instance_id.to_ascii_uppercase(), row.clone())]),
        pending: None,
        established: true,
    };
    let baseline = Baseline {
        collector_id: drivers::ID.to_owned(),
        entity_kind: drivers::BASELINE_KIND.to_owned(),
        snapshot: serde_json::to_value(snapshot).unwrap(),
        captured_at: 1_000,
        truncated: false,
    };
    live.db
        .write(move |connection| {
            baseline_repository::store(connection, &Sanitizer::new().sanitize(baseline).0)
        })
        .unwrap();

    let provider = ScriptedDrivers::new(vec![vec![row]]);
    let mut collector = DriverCollector::with_clock(
        provider,
        drivers::catalog_descriptor().unwrap(),
        ManualClock::at(1_000 + DOWNTIME_GAP_MS + 1),
    );
    collector.start(live.context).unwrap();
    assert!(observations(&cycle(&mut collector, 1)).is_empty());
}

#[test]
fn wake_tier_and_lifecycle_are_owned_by_the_provider() {
    let provider =
        ScriptedDrivers::new(vec![Vec::new()]).with_tier(DriverWakeTier::ConfigurationManager);
    let registrations = provider.registrations.clone();
    let unregisters = provider.unregisters.clone();
    let mut collector = DriverCollector::with_clock(
        provider,
        drivers::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(drivers::ID).unwrap().context)
        .unwrap();
    assert_eq!(collector.wake_tier(), DriverWakeTier::ConfigurationManager);
    assert_eq!(registrations.load(Ordering::Acquire), 1);
    collector.stop();
    assert_eq!(unregisters.load(Ordering::Acquire), 1);
}

#[test]
fn runtime_status_reports_the_tier_published_by_the_real_driver_collector() {
    let provider = ScriptedDrivers::new(vec![Vec::new()]).with_tier(DriverWakeTier::Registry);
    let descriptor = drivers::catalog_descriptor().unwrap();
    let registry =
        CollectorRegistry::new(vec![RegistryEntry::active(descriptor.clone(), move || {
            Box::new(DriverCollector::new(provider.clone(), descriptor.clone()))
        })]);
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "driver-status-runtime",
        ))
        .unwrap(),
    );
    let enabled = BTreeMap::from([(drivers::ID.to_owned(), true)]);
    let supervisor = CollectorSupervisor::new(
        registry,
        db,
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &enabled,
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    for _ in 0..400 {
        let status = supervisor.list_collector_status().remove(0);
        if status.details
            == Some(CollectorStatusDetails {
                wake_tier: Some(CollectorWakeTier::Registry),
                docker_state: None,
                docker_api_version: None,
            })
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("driver wake tier was not published to collector status");
}

#[test]
fn timed_out_sweep_does_not_advance_the_acknowledged_baseline() {
    let provider = ScriptedDrivers::new(vec![vec![driver(Some("1.0"))]]);
    let mut collector = DriverCollector::with_clock(
        provider.clone(),
        drivers::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(drivers::ID).unwrap().context)
        .unwrap();
    cycle(&mut collector, 1);
    provider.fail(AppError::Platform {
        operation: "enumerate present drivers",
        reason: "the ten-second sweep deadline elapsed".to_owned(),
    });
    assert!(collector
        .snapshot_batch(SnapshotContext::for_devtools(2))
        .unwrap_err()
        .to_string()
        .contains("ten-second"));

    provider.script(vec![driver(Some("2.0"))]);
    let retry = cycle(&mut collector, 2);
    assert_eq!(observations(&retry).len(), 1);
    assert_eq!(
        observations(&retry)[0].before,
        Some(serde_json::json!("1.0"))
    );
}

#[derive(Clone, Default)]
struct StallingDrivers {
    calls: Arc<AtomicU64>,
}

impl DriverProvider for StallingDrivers {
    fn drivers(&self, _cancel: &CancelToken, _deadline: Instant) -> AppResult<Vec<DriverRecord>> {
        let call = self.calls.fetch_add(1, Ordering::AcqRel);
        if call == 0 {
            std::thread::sleep(Duration::from_millis(10_500));
        }
        Ok(Vec::new())
    }
}

#[test]
#[ignore = "forced ten-second timeout gate"]
fn hard_timeout_abandons_a_stall_and_prevents_an_overlapping_query() {
    let provider = StallingDrivers::default();
    let calls = provider.calls.clone();
    let mut collector = DriverCollector::with_clock(
        provider,
        drivers::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(drivers::ID).unwrap().context)
        .unwrap();

    let started = Instant::now();
    let error = collector
        .snapshot_batch(SnapshotContext::for_devtools(1))
        .unwrap_err();
    let elapsed = started.elapsed();
    assert!(error.to_string().contains("ten-second"));
    assert!(elapsed >= Duration::from_millis(9_500));
    assert!(elapsed < Duration::from_millis(10_900));
    assert!(matches!(
        collector
            .snapshot_batch(SnapshotContext::for_devtools(2))
            .unwrap(),
        CollectorSnapshot::Noop
    ));
    assert_eq!(calls.load(Ordering::Acquire), 1, "no overlapping query");

    std::thread::sleep(Duration::from_millis(700));
    assert!(observations(&cycle(&mut collector, 3)).is_empty());
    assert_eq!(calls.load(Ordering::Acquire), 2);
    eprintln!("forced driver stall abandoned after {elapsed:?}");
}

#[derive(Clone, Default)]
struct RuntimeRecreationDrivers {
    calls: Arc<AtomicU64>,
    active: Arc<AtomicU64>,
    overlaps: Arc<AtomicU64>,
    release_first: Arc<AtomicBool>,
    first_query_exited: Arc<AtomicBool>,
}

impl DriverProvider for RuntimeRecreationDrivers {
    fn drivers(&self, _cancel: &CancelToken, _deadline: Instant) -> AppResult<Vec<DriverRecord>> {
        let active = self.active.fetch_add(1, Ordering::AcqRel) + 1;
        if active > 1 {
            self.overlaps.fetch_add(1, Ordering::AcqRel);
        }
        let call = self.calls.fetch_add(1, Ordering::AcqRel);
        if call == 0 {
            while !self.release_first.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(5));
            }
            self.first_query_exited.store(true, Ordering::Release);
        }
        self.active.fetch_sub(1, Ordering::AcqRel);
        Ok(Vec::new())
    }
}

#[derive(Debug)]
struct ImmediateBackoff;

impl BackoffPolicy for ImmediateBackoff {
    fn delay(&self, _consecutive_failures: u64) -> Duration {
        Duration::ZERO
    }
}

#[test]
#[ignore = "forced ten-second runtime recreation gate"]
fn runtime_replacement_shares_query_ownership_until_timed_out_worker_exits() {
    let provider = RuntimeRecreationDrivers::default();
    let calls = provider.calls.clone();
    let overlaps = provider.overlaps.clone();
    let release_first = provider.release_first.clone();
    let first_query_exited = provider.first_query_exited.clone();
    let factory_calls = Arc::new(AtomicU64::new(0));
    let query_coordinator = DriverQueryCoordinator::default();
    let descriptor = drivers::catalog_descriptor().unwrap();
    let factory_descriptor = descriptor.clone();
    let factory_calls_for_factory = factory_calls.clone();
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor, move || {
        factory_calls_for_factory.fetch_add(1, Ordering::AcqRel);
        Box::new(DriverCollector::with_query_coordinator(
            provider.clone(),
            factory_descriptor.clone(),
            query_coordinator.clone(),
        ))
    })]);
    let supervisor = CollectorSupervisor::with_runtime(
        registry,
        CollectorRuntimeConfig {
            db: Arc::new(
                Db::open_in(&what_changed_lib::testing::temp_dir(
                    "driver-runtime-recreation",
                ))
                .unwrap(),
            ),
            monitor: Arc::new(Monitoring::running()),
            query_busy: QueryBusy::default(),
            enabled: BTreeMap::from([(drivers::ID.to_owned(), true)]),
            profile: MonitoringProfile::Balanced,
            clock: Arc::new(SystemClock) as Arc<dyn RuntimeClock>,
            backoff: Arc::new(ImmediateBackoff),
        },
    );
    supervisor.spawn_all().unwrap();

    wait_until(Duration::from_secs(11), || {
        factory_calls.load(Ordering::Acquire) >= 2
    });
    assert_eq!(calls.load(Ordering::Acquire), 1);
    assert_eq!(overlaps.load(Ordering::Acquire), 0);

    release_first.store(true, Ordering::Release);
    wait_until(Duration::from_secs(1), || {
        first_query_exited.load(Ordering::Acquire)
    });
    supervisor.resume_wake_handle().request();
    wait_until(Duration::from_secs(1), || {
        calls.load(Ordering::Acquire) >= 2
    });
    assert_eq!(overlaps.load(Ordering::Acquire), 0);
}

fn wait_until(timeout: Duration, predicate: impl Fn() -> bool) {
    let deadline = Instant::now() + timeout;
    while !predicate() {
        assert!(Instant::now() < deadline, "timed out waiting for condition");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
