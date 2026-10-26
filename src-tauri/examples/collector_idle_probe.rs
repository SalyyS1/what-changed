//! Reproducible 60-second idle-overhead probe for the collector supervisor.
//!
//! Build with the `devtools` feature, run the resulting example executable,
//! and sample that process after `IDLE_PROBE_READY` is printed. The extra
//! eight seconds keep the process alive for a complete external 60-second
//! CPU/RSS sample.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::CollectorSupervisor;
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::{Collector, CollectorContext, RawObservation};
use what_changed_lib::domain::{
    CollectorCadence, CollectorCost, CollectorDescriptor, MonitoringProfile,
};
use what_changed_lib::error::AppResult;
use what_changed_lib::storage::Db;

struct EmptyPollCollector {
    id: &'static str,
    display_name: &'static str,
    snapshots: Arc<AtomicU64>,
}

impl Collector for EmptyPollCollector {
    fn id(&self) -> &'static str {
        self.id
    }

    fn descriptor(&self) -> CollectorDescriptor {
        descriptor(self.id, self.display_name)
    }

    fn start(&mut self, _context: CollectorContext) -> AppResult<()> {
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.snapshots.fetch_add(1, Ordering::Relaxed);
        Ok(Vec::new())
    }
}

fn descriptor(id: &'static str, display_name: &'static str) -> CollectorDescriptor {
    CollectorDescriptor {
        id,
        display_name,
        what_it_watches: "an empty QA source",
        why_it_matters: "measures idle supervisor overhead",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(Duration::from_secs(1)),
        requires_admin: false,
    }
}

fn main() {
    let first = Arc::new(AtomicU64::new(0));
    let second = Arc::new(AtomicU64::new(0));
    let first_factory = Arc::clone(&first);
    let second_factory = Arc::clone(&second);
    let registry = CollectorRegistry::new(vec![
        RegistryEntry::active(descriptor("idle-a", "Idle A"), move || {
            Box::new(EmptyPollCollector {
                id: "idle-a",
                display_name: "Idle A",
                snapshots: Arc::clone(&first_factory),
            })
        }),
        RegistryEntry::active(descriptor("idle-b", "Idle B"), move || {
            Box::new(EmptyPollCollector {
                id: "idle-b",
                display_name: "Idle B",
                snapshots: Arc::clone(&second_factory),
            })
        }),
    ]);
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir("collector-idle-probe")).unwrap(),
    );
    let supervisor = CollectorSupervisor::new(
        registry,
        db,
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::from([("idle-a".to_owned(), true), ("idle-b".to_owned(), true)]),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();

    let startup_deadline = Instant::now() + Duration::from_secs(10);
    while (first.load(Ordering::Relaxed) < 2 || second.load(Ordering::Relaxed) < 2)
        && Instant::now() < startup_deadline
    {
        std::thread::sleep(Duration::from_millis(20));
    }
    let first_start = first.load(Ordering::Relaxed);
    let second_start = second.load(Ordering::Relaxed);
    assert!(first_start >= 2 && second_start >= 2);
    println!(
        "IDLE_PROBE_READY pid={} first={} second={}",
        std::process::id(),
        first_start,
        second_start
    );
    std::io::stdout().flush().unwrap();

    std::thread::sleep(Duration::from_secs(68));

    let first_end = first.load(Ordering::Relaxed);
    let second_end = second.load(Ordering::Relaxed);
    println!("IDLE_PROBE_COUNTS first={first_end} second={second_end}");
    assert!(first_end >= first_start + 50);
    assert!(second_end >= second_start + 50);
    supervisor.stop_all();
}
