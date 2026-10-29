//! Service collector acceptance checks with a scripted provider.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use what_changed_lib::collectors::application_install_map::{
    ApplicationInstallEvidence, ApplicationInstallMap,
};
use what_changed_lib::collectors::services::{
    self, ServiceCollector, ServiceProvider, ServiceRecord, ServiceRunState, ServiceSnapshot,
    ServiceStartType,
};
use what_changed_lib::collectors::stateful::DOWNTIME_GAP_MS;
use what_changed_lib::collectors::testing::live_context;
use what_changed_lib::collectors::{
    CancelToken, Collector, CollectorSnapshot, EntityIdentity, ObservationClock, SnapshotContext,
    WakeHandle,
};
use what_changed_lib::domain::{EventType, RelationshipKind};
use what_changed_lib::error::AppResult;
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{
    baseline_repository, Baseline, CollectorCommitReceipt, MAX_ATOMIC_RELATIONSHIPS,
};

#[derive(Clone, Default)]
struct ScriptedServices {
    tables: Arc<Mutex<VecDeque<Vec<ServiceRecord>>>>,
    notify_ok: Arc<AtomicBool>,
    notify_attempts: Arc<AtomicU64>,
}

impl ScriptedServices {
    fn new(tables: Vec<Vec<ServiceRecord>>) -> Self {
        Self {
            tables: Arc::new(Mutex::new(tables.into())),
            ..Self::default()
        }
    }

    fn script(&self, table: Vec<ServiceRecord>) {
        *lock(&self.tables) = VecDeque::from([table]);
    }
}

impl ServiceProvider for ScriptedServices {
    fn services(&self) -> AppResult<Vec<ServiceRecord>> {
        let mut tables = lock(&self.tables);
        if tables.len() > 1 {
            return Ok(tables.pop_front().unwrap_or_default());
        }
        Ok(tables.front().cloned().unwrap_or_default())
    }

    fn try_register_notify(&self, _wake: WakeHandle) -> bool {
        self.notify_attempts.fetch_add(1, Ordering::AcqRel);
        self.notify_ok.load(Ordering::Acquire)
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

fn service(index: usize, state: ServiceRunState) -> ServiceRecord {
    ServiceRecord {
        name: format!("svc-{index:03}"),
        display_name: format!("Service {index:03}"),
        state,
        start_type: ServiceStartType::Automatic,
        config_available: true,
        binary_path: Some(format!(r"C:\Program Files\Fixture\svc-{index:03}.exe")),
        process_id: state.is_running().then_some(1_000 + index as u32),
    }
}

fn observations(snapshot: &CollectorSnapshot) -> &[what_changed_lib::collectors::RawObservation] {
    match snapshot {
        CollectorSnapshot::Atomic { observations, .. } => observations,
        CollectorSnapshot::Noop => &[],
        CollectorSnapshot::Legacy(_) => panic!("service collector must use atomic snapshots"),
    }
}

fn relationships(
    snapshot: &CollectorSnapshot,
) -> &[what_changed_lib::collectors::RawRelationshipSpec] {
    match snapshot {
        CollectorSnapshot::Atomic { relationships, .. } => relationships,
        CollectorSnapshot::Noop => &[],
        CollectorSnapshot::Legacy(_) => panic!("service collector must use atomic snapshots"),
    }
}

fn cycle<P: ServiceProvider, C: ObservationClock>(
    collector: &mut ServiceCollector<P, C>,
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
fn two_hundred_services_and_one_stop_emit_exactly_one_event() {
    let running: Vec<_> = (0..200)
        .map(|index| service(index, ServiceRunState::Running))
        .collect();
    let provider = ScriptedServices::new(vec![running.clone()]);
    let mut collector = ServiceCollector::with_clock(
        provider.clone(),
        services::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(services::ID).unwrap().context)
        .unwrap();
    assert!(observations(&cycle(&mut collector, 1)).is_empty());

    let mut after = running;
    after[73].state = ServiceRunState::Stopped;
    after[73].process_id = None;
    provider.script(after);
    let stopped = cycle(&mut collector, 2);
    assert_eq!(observations(&stopped).len(), 1);
    assert_eq!(
        observations(&stopped)[0].kind.event_type,
        EventType::Stopped
    );
    assert!(observations(&stopped)[0].kind.classifier.automatic_service);
    assert_eq!(observations(&stopped)[0].entity.display_name, "Service 073");
}

#[test]
fn a_temporary_config_denial_never_fabricates_a_start_type_change() {
    let original = service(1, ServiceRunState::Running);
    let provider = ScriptedServices::new(vec![vec![original.clone()]]);
    let mut collector = ServiceCollector::with_clock(
        provider.clone(),
        services::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(services::ID).unwrap().context)
        .unwrap();
    cycle(&mut collector, 1);

    let mut denied = original.clone();
    denied.config_available = false;
    denied.start_type = ServiceStartType::Unknown(u32::MAX);
    denied.binary_path = None;
    provider.script(vec![denied]);
    assert!(observations(&cycle(&mut collector, 2)).is_empty());

    let mut changed_while_denied = original;
    changed_while_denied.start_type = ServiceStartType::Manual;
    provider.script(vec![changed_while_denied]);
    let recovered = cycle(&mut collector, 3);
    assert_eq!(observations(&recovered).len(), 1);
    assert_eq!(
        observations(&recovered)[0].kind.event_type,
        EventType::ConfigurationChanged
    );
    assert_eq!(
        observations(&recovered)[0].before,
        Some(serde_json::json!("Automatic"))
    );
    assert_eq!(
        observations(&recovered)[0].after,
        Some(serde_json::json!("Manual"))
    );
}

#[test]
fn an_oversized_install_burst_drains_without_truncation() {
    let provider = ScriptedServices::new(vec![Vec::new()]);
    let mut collector = ServiceCollector::with_clock(
        provider.clone(),
        services::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(services::ID).unwrap().context)
        .unwrap();
    cycle(&mut collector, 1);
    provider.script(
        (0..450)
            .map(|index| service(index, ServiceRunState::Running))
            .collect(),
    );
    assert_eq!(observations(&cycle(&mut collector, 2)).len(), 200);
    assert_eq!(observations(&cycle(&mut collector, 3)).len(), 200);
    assert_eq!(observations(&cycle(&mut collector, 4)).len(), 50);
    assert!(observations(&cycle(&mut collector, 5)).is_empty());
}

#[test]
fn a_start_type_change_carries_the_exact_before_and_after_values() {
    let original = service(1, ServiceRunState::Running);
    let provider = ScriptedServices::new(vec![vec![original.clone()]]);
    let mut collector = ServiceCollector::with_clock(
        provider.clone(),
        services::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(services::ID).unwrap().context)
        .unwrap();
    cycle(&mut collector, 1);

    let mut changed = original;
    changed.start_type = ServiceStartType::Manual;
    provider.script(vec![changed]);
    let snapshot = cycle(&mut collector, 2);
    let event = &observations(&snapshot)[0];
    assert_eq!(event.kind.event_type, EventType::ConfigurationChanged);
    assert_eq!(event.field.as_deref(), Some("start_type"));
    assert_eq!(event.before, Some(serde_json::json!("Automatic")));
    assert_eq!(event.after, Some(serde_json::json!("Manual")));
}

#[test]
fn no_baseline_is_silent_and_notify_failure_does_not_stop_collection() {
    let provider = ScriptedServices::new(vec![vec![service(1, ServiceRunState::Running)]]);
    let mut collector = ServiceCollector::with_clock(
        provider.clone(),
        services::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(services::ID).unwrap().context)
        .unwrap();
    assert_eq!(provider.notify_attempts.load(Ordering::Acquire), 1);
    assert!(observations(&cycle(&mut collector, 1)).is_empty());

    provider.script(Vec::new());
    let removed = cycle(&mut collector, 2);
    assert_eq!(observations(&removed).len(), 1);
    assert_eq!(
        observations(&removed)[0].kind.event_type,
        EventType::Uninstalled
    );
}

#[test]
fn a_persisted_baseline_reports_a_downtime_removal_as_uncertain() {
    let live = live_context(services::ID).unwrap();
    let old = service(7, ServiceRunState::Running);
    let state = ServiceSnapshot {
        acknowledged: BTreeMap::from([(old.name.to_ascii_lowercase(), old)]),
        pending: None,
        established: true,
    };
    let baseline = Baseline {
        collector_id: services::ID.to_owned(),
        entity_kind: services::BASELINE_KIND.to_owned(),
        snapshot: serde_json::to_value(state).unwrap(),
        captured_at: 1_000,
        truncated: false,
    };
    live.db
        .write(move |connection| {
            baseline_repository::store(connection, &Sanitizer::new().sanitize(baseline).0)
        })
        .unwrap();

    let provider = ScriptedServices::new(vec![Vec::new()]);
    let mut collector = ServiceCollector::with_clock(
        provider,
        services::catalog_descriptor().unwrap(),
        ManualClock::at(1_000 + DOWNTIME_GAP_MS + 1),
    );
    collector.start(live.context).unwrap();
    let snapshot = cycle(&mut collector, 1);
    assert_eq!(observations(&snapshot).len(), 1);
    assert_eq!(
        observations(&snapshot)[0].kind.event_type,
        EventType::Uninstalled
    );
    assert!(observations(&snapshot)[0].occurred_at_is_uncertain);
}

#[test]
fn a_new_service_under_an_install_root_links_back_to_its_application() {
    let applications = ApplicationInstallMap::new();
    let application_token = CancelToken::default();
    applications.publish(
        [ApplicationInstallEvidence {
            hive: "HKLM64".to_owned(),
            registry_key: "Fixture".to_owned(),
            display_name: "Fixture Suite".to_owned(),
            install_location: r"C:\Program Files\Fixture".to_owned(),
        }],
        &application_token,
        std::time::Instant::now(),
    );
    let provider = ScriptedServices::new(vec![Vec::new()]);
    let mut collector = ServiceCollector::with_clock_and_maps(
        provider.clone(),
        what_changed_lib::collectors::service_evidence_map::ServiceEvidenceMap::new(),
        applications,
        services::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(services::ID).unwrap().context)
        .unwrap();
    cycle(&mut collector, 1);

    provider.script(vec![service(1, ServiceRunState::Running)]);
    let installed = cycle(&mut collector, 2);
    assert_eq!(relationships(&installed).len(), 1);
    let relationship = &relationships(&installed)[0];
    assert_eq!(relationship.kind, RelationshipKind::Provides);
    assert_eq!(
        relationship.source.identity,
        EntityIdentity::Application {
            hive: "HKLM64".to_owned(),
            registry_key: "Fixture".to_owned(),
        }
    );
    assert_eq!(
        relationship.target.identity,
        EntityIdentity::Service {
            name: "svc-001".to_owned(),
        }
    );
}

#[test]
fn one_service_never_truncates_an_oversized_provider_set_and_advances_baseline() {
    let applications = ApplicationInstallMap::new();
    let application_token = CancelToken::default();
    applications.publish(
        (0..=MAX_ATOMIC_RELATIONSHIPS).map(|index| ApplicationInstallEvidence {
            hive: "HKLM64".to_owned(),
            registry_key: format!("Fixture-{index}"),
            display_name: format!("Fixture Suite {index}"),
            install_location: r"C:\Program Files\Fixture".to_owned(),
        }),
        &application_token,
        std::time::Instant::now(),
    );
    let provider = ScriptedServices::new(vec![Vec::new()]);
    let mut collector = ServiceCollector::with_clock_and_maps(
        provider.clone(),
        what_changed_lib::collectors::service_evidence_map::ServiceEvidenceMap::new(),
        applications.clone(),
        services::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(services::ID).unwrap().context)
        .unwrap();
    cycle(&mut collector, 1);

    provider.script(vec![service(1, ServiceRunState::Running)]);
    let error = collector
        .snapshot_batch(SnapshotContext::for_devtools(2))
        .unwrap_err();
    assert!(error.to_string().contains("atomic relationship bounds"));

    applications.publish(
        [ApplicationInstallEvidence {
            hive: "HKLM64".to_owned(),
            registry_key: "Fixture".to_owned(),
            display_name: "Fixture Suite".to_owned(),
            install_location: r"C:\Program Files\Fixture".to_owned(),
        }],
        &application_token,
        std::time::Instant::now(),
    );
    let retry = cycle(&mut collector, 2);
    assert_eq!(observations(&retry).len(), 1);
    assert_eq!(relationships(&retry).len(), 1);
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
