#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::application_install_map::ApplicationInstallMap;
use what_changed_lib::collectors::applications::{
    self, ApplicationCollector, RegistryNotificationGuard, RegistryUninstallProvider,
    UninstallEntry, UninstallHive,
};
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::{
    BackoffPolicy, CollectorRuntimeConfig, CollectorSupervisor, RuntimeClock,
};
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::service_evidence_map::{ServiceEvidence, ServiceEvidenceMap};
use what_changed_lib::collectors::testing::{live_context, LiveContext};
use what_changed_lib::collectors::{
    CancelToken, Collector, CollectorSnapshot, EntityIdentity, ObservationClock, RawObservation,
    SnapshotContext, WakeHandle,
};
use what_changed_lib::domain::{
    CollectorCadence, EventType, Importance, MonitoringProfile, RelationshipKind,
};
use what_changed_lib::error::{AppError, AppResult};
use what_changed_lib::storage::{CollectorCommitReceipt, MAX_ATOMIC_RELATIONSHIPS};

#[derive(Clone, Default)]
struct ScriptedRegistry {
    tables: Arc<Mutex<VecDeque<Vec<UninstallEntry>>>>,
    notifications_fail: bool,
}

impl ScriptedRegistry {
    fn new(tables: Vec<Vec<UninstallEntry>>) -> Self {
        Self {
            tables: Arc::new(Mutex::new(tables.into())),
            notifications_fail: true,
        }
    }

    fn script(&self, table: Vec<UninstallEntry>) {
        *self.tables.lock().unwrap() = VecDeque::from([table]);
    }

    fn next(&self) -> Vec<UninstallEntry> {
        let mut tables = self.tables.lock().unwrap();
        if tables.len() > 1 {
            return tables.pop_front().unwrap_or_default();
        }
        tables.front().cloned().unwrap_or_default()
    }
}

struct InertNotification;
impl RegistryNotificationGuard for InertNotification {}

impl RegistryUninstallProvider for ScriptedRegistry {
    fn enumerate(&self) -> AppResult<Vec<UninstallEntry>> {
        Ok(self.next())
    }

    fn register_notifications(
        &self,
        _wake: WakeHandle,
        _cancel: CancelToken,
    ) -> AppResult<Box<dyn RegistryNotificationGuard>> {
        if self.notifications_fail {
            Err(AppError::Platform {
                operation: "register scripted registry notification",
                reason: "fault injected".to_owned(),
            })
        } else {
            Ok(Box::new(InertNotification))
        }
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

impl RuntimeClock for ManualClock {
    fn now(&self) -> Instant {
        static BASE: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let millis = self
            .0
            .load(Ordering::Acquire)
            .saturating_sub(1_700_000_000_000)
            .max(0) as u64;
        *BASE.get_or_init(Instant::now) + Duration::from_millis(millis)
    }

    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }
}

struct FixedBackoff;

impl BackoffPolicy for FixedBackoff {
    fn delay(&self, _consecutive_failures: u64) -> Duration {
        Duration::from_millis(1)
    }
}

type UnderTest = ApplicationCollector<ScriptedRegistry, ManualClock>;

fn application(index: usize, version: &str) -> UninstallEntry {
    UninstallEntry {
        hive: UninstallHive::LocalMachine64,
        subkey_name: format!("App-{index}"),
        display_name: format!("Application {index}"),
        display_version: Some(version.to_owned()),
        publisher: Some("Example publisher".to_owned()),
        install_date: Some("20260919".to_owned()),
        install_location: Some(format!(r"C:\Apps\Application-{index}")),
        system_component: false,
    }
}

fn docker(version: &str) -> UninstallEntry {
    UninstallEntry {
        display_name: "Docker Desktop".to_owned(),
        subkey_name: "DockerDesktop".to_owned(),
        install_location: Some(r"C:\Program Files\Docker\Docker".to_owned()),
        ..application(0, version)
    }
}

fn started(table: Vec<UninstallEntry>) -> (UnderTest, ScriptedRegistry, LiveContext) {
    let provider = ScriptedRegistry::new(vec![table]);
    let mut collector = ApplicationCollector::with_clock(
        provider.clone(),
        ApplicationInstallMap::new(),
        applications::catalog_descriptor().unwrap(),
        ManualClock::at(1_700_000_000_000),
    );
    let live = live_context(applications::ID).unwrap();
    collector.start(live.context.clone()).unwrap();
    (collector, provider, live)
}

fn cycle(collector: &mut UnderTest, token_value: u64) -> CollectorSnapshot {
    let snapshot = collector
        .snapshot_batch(SnapshotContext::for_devtools(token_value))
        .unwrap();
    if let CollectorSnapshot::Atomic { token, .. } = &snapshot {
        collector
            .commit_succeeded(
                *token,
                &CollectorCommitReceipt {
                    event_ids: Vec::new(),
                },
            )
            .unwrap();
    }
    snapshot
}

fn observations(snapshot: &CollectorSnapshot) -> &[RawObservation] {
    match snapshot {
        CollectorSnapshot::Atomic { observations, .. } => observations,
        CollectorSnapshot::Noop => &[],
        other => panic!("expected an atomic/noop snapshot, got {other:?}"),
    }
}

fn relationships(
    snapshot: &CollectorSnapshot,
) -> &[what_changed_lib::collectors::RawRelationshipSpec] {
    match snapshot {
        CollectorSnapshot::Atomic { relationships, .. } => relationships,
        CollectorSnapshot::Noop => &[],
        other => panic!("expected an atomic/noop snapshot, got {other:?}"),
    }
}

#[test]
fn filters_sixty_system_entries_out_of_a_148_entry_registry_view() {
    let table: Vec<UninstallEntry> = (0..148)
        .map(|index| UninstallEntry {
            system_component: index < 60,
            ..application(index, "1.0")
        })
        .collect();
    let (mut collector, _, _live) = started(table);
    let snapshot = cycle(&mut collector, 1);
    assert!(observations(&snapshot).is_empty());
    let baseline = match snapshot {
        CollectorSnapshot::Atomic {
            baseline: Some(baseline),
            ..
        } => baseline.snapshot,
        other => panic!("missing baseline: {other:?}"),
    };
    assert_eq!(baseline["acknowledged"].as_object().unwrap().len(), 88);
}

#[test]
fn update_and_downgrade_both_use_exact_version_phrasing_and_stay_informational() {
    let (mut collector, provider, _live) = started(vec![docker("4.35.1")]);
    cycle(&mut collector, 1);

    provider.script(vec![docker("4.36.0")]);
    let updated = cycle(&mut collector, 2);
    let event = &observations(&updated)[0];
    assert_eq!(event.kind.event_type, EventType::Updated);
    assert_eq!(event.field.as_deref(), Some("version"));
    assert_eq!(
        event.kind.user_friendly_summary,
        "Docker Desktop updated 4.35.1 → 4.36.0"
    );
    let normalized =
        what_changed_lib::collectors::normalize::Normalizer::new().normalize(event.clone());
    assert_eq!(normalized.get().importance, Importance::Informational);

    provider.script(vec![docker("4.20.0")]);
    let downgraded = cycle(&mut collector, 3);
    assert_eq!(
        observations(&downgraded)[0].kind.user_friendly_summary,
        "Docker Desktop updated 4.36.0 → 4.20.0"
    );
}

#[test]
fn empty_display_names_and_system_components_are_not_entities() {
    let mut empty = application(1, "1.0");
    empty.display_name = "   ".to_owned();
    let mut system = application(2, "1.0");
    system.system_component = true;
    let (mut collector, _, _live) = started(vec![application(0, "1.0"), empty, system]);
    let baseline = match cycle(&mut collector, 1) {
        CollectorSnapshot::Atomic {
            baseline: Some(baseline),
            ..
        } => baseline.snapshot,
        other => panic!("missing baseline: {other:?}"),
    };
    assert_eq!(baseline["acknowledged"].as_object().unwrap().len(), 1);
}

#[test]
fn first_cycle_is_silent_and_an_unchanged_follow_up_is_a_noop() {
    let (mut collector, _, _live) = started(vec![application(0, "1.0")]);
    assert!(observations(&cycle(&mut collector, 1)).is_empty());
    assert_eq!(cycle(&mut collector, 2), CollectorSnapshot::Noop);
}

#[test]
fn notification_registration_failure_keeps_fallback_collection_running() {
    let (mut collector, provider, _live) = started(Vec::new());
    cycle(&mut collector, 1);
    provider.script(vec![application(0, "1.0")]);
    let snapshot = cycle(&mut collector, 2);
    assert_eq!(observations(&snapshot).len(), 1);
    assert_eq!(
        observations(&snapshot)[0].kind.event_type,
        EventType::Installed
    );
}

#[test]
fn install_locations_are_session_evidence_not_durable_baseline_text() {
    let (mut collector, _, _live) = started(vec![docker("4.36.0")]);
    let snapshot = cycle(&mut collector, 1);
    let encoded = match snapshot {
        CollectorSnapshot::Atomic {
            baseline: Some(baseline),
            ..
        } => serde_json::to_string(&baseline.snapshot).unwrap(),
        other => panic!("missing baseline: {other:?}"),
    };
    assert!(!encoded.contains("Program Files"));
    assert!(!encoded.contains("install_location"));
}

#[test]
fn an_installed_application_links_to_services_under_its_install_root() {
    let services = ServiceEvidenceMap::new();
    let service_token = CancelToken::default();
    services.publish(
        [ServiceEvidence {
            name: "fixture-api".to_owned(),
            display_name: "Fixture API".to_owned(),
            binary_path: r"C:\Apps\Application-1\api.exe".to_owned(),
        }],
        &service_token,
        Instant::now(),
    );
    let provider = ScriptedRegistry::new(vec![Vec::new()]);
    let mut collector = ApplicationCollector::with_clock_and_maps(
        provider.clone(),
        ApplicationInstallMap::new(),
        services,
        applications::catalog_descriptor().unwrap(),
        ManualClock::at(1_700_000_000_000),
    );
    collector
        .start(live_context(applications::ID).unwrap().context)
        .unwrap();
    cycle(&mut collector, 1);

    provider.script(vec![application(1, "1.0")]);
    let installed = cycle(&mut collector, 2);
    assert_eq!(relationships(&installed).len(), 1);
    let relationship = &relationships(&installed)[0];
    assert_eq!(relationship.kind, RelationshipKind::Provides);
    assert_eq!(
        relationship.source.identity,
        EntityIdentity::Application {
            hive: "HKLM64".to_owned(),
            registry_key: "App-1".to_owned(),
        }
    );
    assert_eq!(
        relationship.target.identity,
        EntityIdentity::Service {
            name: "fixture-api".to_owned(),
        }
    );
}

#[test]
fn a_broad_install_root_cannot_claim_a_service_owned_by_a_more_specific_app() {
    let services = ServiceEvidenceMap::new();
    let service_token = CancelToken::default();
    services.publish(
        [ServiceEvidence {
            name: "fixture-api".to_owned(),
            display_name: "Fixture API".to_owned(),
            binary_path: r"C:\Apps\Fixture\api.exe".to_owned(),
        }],
        &service_token,
        Instant::now(),
    );
    let provider = ScriptedRegistry::new(vec![Vec::new()]);
    let mut collector = ApplicationCollector::with_clock_and_maps(
        provider.clone(),
        ApplicationInstallMap::new(),
        services,
        applications::catalog_descriptor().unwrap(),
        ManualClock::at(1_700_000_000_000),
    );
    collector
        .start(live_context(applications::ID).unwrap().context)
        .unwrap();
    cycle(&mut collector, 1);

    let mut broad = application(1, "1.0");
    broad.subkey_name = "Broad".to_owned();
    broad.display_name = "Broad Suite".to_owned();
    broad.install_location = Some(r"C:\Apps".to_owned());
    let mut specific = application(2, "1.0");
    specific.subkey_name = "Fixture".to_owned();
    specific.display_name = "Fixture Suite".to_owned();
    specific.install_location = Some(r"C:\Apps\Fixture".to_owned());
    provider.script(vec![broad, specific]);

    let installed = cycle(&mut collector, 2);
    assert_eq!(relationships(&installed).len(), 1);
    assert_eq!(
        relationships(&installed)[0].source.identity,
        EntityIdentity::Application {
            hive: "HKLM64".to_owned(),
            registry_key: "Fixture".to_owned(),
        }
    );
}

#[test]
fn one_application_never_truncates_an_oversized_service_set_and_advances_baseline() {
    let services = ServiceEvidenceMap::new();
    let service_token = CancelToken::default();
    services.publish(
        (0..=MAX_ATOMIC_RELATIONSHIPS).map(|index| ServiceEvidence {
            name: format!("fixture-{index}"),
            display_name: format!("Fixture Service {index}"),
            binary_path: format!(r"C:\Apps\Application-1\service-{index}.exe"),
        }),
        &service_token,
        Instant::now(),
    );
    let provider = ScriptedRegistry::new(vec![Vec::new()]);
    let mut collector = ApplicationCollector::with_clock_and_maps(
        provider.clone(),
        ApplicationInstallMap::new(),
        services.clone(),
        applications::catalog_descriptor().unwrap(),
        ManualClock::at(1_700_000_000_000),
    );
    collector
        .start(live_context(applications::ID).unwrap().context)
        .unwrap();
    cycle(&mut collector, 1);

    provider.script(vec![application(1, "1.0")]);
    let error = collector
        .snapshot_batch(SnapshotContext::for_devtools(2))
        .unwrap_err();
    assert!(error.to_string().contains("atomic relationship bounds"));

    services.publish(
        [ServiceEvidence {
            name: "fixture-api".to_owned(),
            display_name: "Fixture API".to_owned(),
            binary_path: r"C:\Apps\Application-1\api.exe".to_owned(),
        }],
        &service_token,
        Instant::now(),
    );
    let retry = cycle(&mut collector, 2);
    assert_eq!(observations(&retry).len(), 1);
    assert_eq!(relationships(&retry).len(), 1);
}

struct ApplicationRig {
    db: Arc<what_changed_lib::storage::Db>,
    clock: ManualClock,
    supervisor: CollectorSupervisor,
}

impl ApplicationRig {
    fn at(directory: PathBuf, provider: ScriptedRegistry, now_ms: i64) -> Self {
        let db = Arc::new(what_changed_lib::storage::Db::open_in(&directory).unwrap());
        let clock = ManualClock::at(now_ms);
        let mut descriptor = applications::catalog_descriptor().unwrap();
        descriptor.cadence = CollectorCadence::Poll(Duration::from_millis(10));
        let collector_descriptor = descriptor.clone();
        let collector_clock = clock.clone();
        let install_map = ApplicationInstallMap::new();
        let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor, move || {
            Box::new(ApplicationCollector::with_clock(
                provider.clone(),
                install_map.clone(),
                collector_descriptor.clone(),
                collector_clock.clone(),
            ))
        })]);
        let supervisor = CollectorSupervisor::with_runtime(
            registry,
            CollectorRuntimeConfig {
                db: Arc::clone(&db),
                monitor: Arc::new(Monitoring::running()),
                query_busy: QueryBusy::default(),
                enabled: BTreeMap::from([(applications::ID.to_owned(), true)]),
                profile: MonitoringProfile::Balanced,
                clock: Arc::new(clock.clone()),
                backoff: Arc::new(FixedBackoff),
            },
        );
        supervisor.spawn_all().unwrap();
        let rig = Self {
            db,
            clock,
            supervisor,
        };
        rig.wait_for_cycle();
        rig
    }

    fn wait_for_cycle(&self) {
        let target = RuntimeClock::now_ms(&self.clock);
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let ran = self
                .supervisor
                .list_collector_status()
                .into_iter()
                .find(|status| status.id == applications::ID)
                .and_then(|status| status.last_run_at)
                .is_some_and(|last_run| last_run >= target);
            if ran {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("application collector did not finish its cycle");
    }

    fn stop(&self) {
        self.supervisor.stop_all();
    }
}

impl Drop for ApplicationRig {
    fn drop(&mut self) {
        self.supervisor.stop_all();
    }
}

#[test]
fn a_version_change_while_stopped_is_reported_as_uncertain_after_restart() {
    let directory = what_changed_lib::testing::temp_dir("collector-applications-downtime");
    {
        let first = ApplicationRig::at(
            directory.clone(),
            ScriptedRegistry::new(vec![vec![docker("4.35.1")]]),
            1_700_000_000_000,
        );
        first.stop();
    }

    let resumed = ApplicationRig::at(
        directory,
        ScriptedRegistry::new(vec![vec![docker("4.36.0")]]),
        1_700_000_600_000,
    );
    let events = resumed
        .db
        .read(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT user_friendly_summary, occurred_at_is_uncertain \
                     FROM events WHERE source_collector = ?1 ORDER BY id",
                )
                .unwrap();
            let rows = statement
                .query_map([applications::ID], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
                })
                .unwrap();
            Ok(rows.collect::<Result<Vec<_>, _>>().unwrap())
        })
        .unwrap();
    assert_eq!(
        events,
        vec![("Docker Desktop updated 4.35.1 → 4.36.0".to_owned(), true,)]
    );
}
