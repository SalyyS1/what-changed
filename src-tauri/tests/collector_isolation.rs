#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::{
    BackoffPolicy, CollectorRuntimeConfig, CollectorSupervisor, ExponentialBackoff,
};
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::{Collector, CollectorContext, RawObservation};
use what_changed_lib::domain::{
    CollectorCadence, CollectorCost, CollectorDescriptor, MonitoringProfile,
};
use what_changed_lib::error::{AppError, AppResult};
use what_changed_lib::storage::Db;

struct RecordingBackoff {
    seen: Arc<Mutex<Vec<(u64, Duration)>>>,
}
impl BackoffPolicy for RecordingBackoff {
    fn delay(&self, failures: u64) -> Duration {
        let production_delay = ExponentialBackoff.delay(failures);
        self.seen
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push((failures, production_delay));
        Duration::ZERO
    }
}

enum Behavior {
    Healthy,
    Fail,
    Panic,
}
struct TestCollector {
    id: &'static str,
    attempts: Arc<AtomicU64>,
    stops: Arc<AtomicU64>,
    behavior: Behavior,
}
impl Collector for TestCollector {
    fn id(&self) -> &'static str {
        self.id
    }
    fn descriptor(&self) -> CollectorDescriptor {
        descriptor(self.id)
    }
    fn start(&mut self, _ctx: CollectorContext) -> AppResult<()> {
        Ok(())
    }
    fn stop(&mut self) {
        self.stops.fetch_add(1, Ordering::Relaxed);
    }
    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.attempts.fetch_add(1, Ordering::Relaxed);
        match self.behavior {
            Behavior::Healthy => Ok(Vec::new()),
            Behavior::Fail => Err(AppError::InvalidInput {
                what: "collector",
                reason: "failed".to_owned(),
            }),
            Behavior::Panic => panic!("isolated panic"),
        }
    }
}

fn descriptor(id: &'static str) -> CollectorDescriptor {
    CollectorDescriptor {
        id,
        display_name: id,
        what_it_watches: "test",
        why_it_matters: "isolation",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(Duration::from_millis(5)),
        requires_admin: false,
    }
}

fn wait_for(predicate: impl Fn() -> bool) {
    for _ in 0..200 {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("condition did not become true");
}

#[test]
fn failures_and_panics_stop_at_five_while_a_healthy_neighbour_keeps_running() {
    let failed = Arc::new(AtomicU64::new(0));
    let panicked = Arc::new(AtomicU64::new(0));
    let healthy = Arc::new(AtomicU64::new(0));
    let failed_stops = Arc::new(AtomicU64::new(0));
    let panicked_stops = Arc::new(AtomicU64::new(0));
    let panicked_factories = Arc::new(AtomicU64::new(0));
    let healthy_stops = Arc::new(AtomicU64::new(0));
    let backoffs = Arc::new(Mutex::new(Vec::new()));
    let entries = vec![
        {
            let attempts = Arc::clone(&failed);
            let stops = Arc::clone(&failed_stops);
            RegistryEntry::active(descriptor("failed"), move || {
                Box::new(TestCollector {
                    id: "failed",
                    attempts: Arc::clone(&attempts),
                    stops: Arc::clone(&stops),
                    behavior: Behavior::Fail,
                })
            })
        },
        {
            let attempts = Arc::clone(&panicked);
            let stops = Arc::clone(&panicked_stops);
            let factories = Arc::clone(&panicked_factories);
            RegistryEntry::active(descriptor("panicked"), move || {
                factories.fetch_add(1, Ordering::Relaxed);
                Box::new(TestCollector {
                    id: "panicked",
                    attempts: Arc::clone(&attempts),
                    stops: Arc::clone(&stops),
                    behavior: Behavior::Panic,
                })
            })
        },
        {
            let attempts = Arc::clone(&healthy);
            let stops = Arc::clone(&healthy_stops);
            RegistryEntry::active(descriptor("healthy"), move || {
                Box::new(TestCollector {
                    id: "healthy",
                    attempts: Arc::clone(&attempts),
                    stops: Arc::clone(&stops),
                    behavior: Behavior::Healthy,
                })
            })
        },
    ];
    let db =
        Arc::new(Db::open_in(&what_changed_lib::testing::temp_dir("collector-isolation")).unwrap());
    let enabled = BTreeMap::from([
        ("failed".to_owned(), true),
        ("panicked".to_owned(), true),
        ("healthy".to_owned(), true),
    ]);
    let supervisor = CollectorSupervisor::with_runtime(
        CollectorRegistry::new(entries),
        CollectorRuntimeConfig {
            db,
            monitor: Arc::new(Monitoring::running()),
            query_busy: QueryBusy::default(),
            enabled,
            profile: MonitoringProfile::Balanced,
            clock: Arc::new(what_changed_lib::collectors::runtime::SystemClock),
            backoff: Arc::new(RecordingBackoff {
                seen: Arc::clone(&backoffs),
            }),
        },
    );
    supervisor.spawn_all().unwrap();
    wait_for(|| {
        failed.load(Ordering::Relaxed) >= 5
            && panicked.load(Ordering::Relaxed) >= 5
            && healthy.load(Ordering::Relaxed) >= 3
    });
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(failed.load(Ordering::Relaxed), 5);
    assert_eq!(panicked.load(Ordering::Relaxed), 5);
    assert!(healthy.load(Ordering::Relaxed) >= 3);
    assert_eq!(failed_stops.load(Ordering::Relaxed), 5);
    assert_eq!(
        panicked_stops.load(Ordering::Relaxed),
        0,
        "a panicked instance is poisoned and must be dropped without stop()",
    );
    assert_eq!(
        panicked_factories.load(Ordering::Relaxed),
        5,
        "every panic recreates a fresh collector until the failure cap",
    );
    let mut backoffs = backoffs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    backoffs.sort_unstable();
    assert_eq!(
        backoffs,
        vec![
            (1, Duration::from_secs(5)),
            (1, Duration::from_secs(5)),
            (2, Duration::from_secs(30)),
            (2, Duration::from_secs(30)),
            (3, Duration::from_secs(300)),
            (3, Duration::from_secs(300)),
            (4, Duration::from_secs(300)),
            (4, Duration::from_secs(300)),
        ],
        "both failing workers traverse the exact production backoff schedule before the cap",
    );
    let rows = supervisor.list_collector_status();
    assert_eq!(
        rows.iter().find(|r| r.id == "failed").unwrap().state,
        what_changed_lib::dto::CollectorState::Failed
    );
    assert_eq!(
        rows.iter().find(|r| r.id == "panicked").unwrap().state,
        what_changed_lib::dto::CollectorState::Failed
    );
    supervisor.stop_all();
    assert_eq!(healthy_stops.load(Ordering::Relaxed), 1);
}

#[test]
fn production_backoff_is_exact_and_capped() {
    let policy = ExponentialBackoff;
    assert_eq!(policy.delay(1), Duration::from_secs(5));
    assert_eq!(policy.delay(2), Duration::from_secs(30));
    assert_eq!(policy.delay(3), Duration::from_secs(300));
    assert_eq!(policy.delay(99), Duration::from_secs(300));
}
