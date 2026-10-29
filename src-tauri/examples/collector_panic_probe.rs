//! Run with `cargo run --release --features devtools --example collector_panic_probe`.
//! Deliberately panics one provider and proves its neighbour keeps collecting.
#![allow(clippy::panic)]

use std::collections::BTreeMap;
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

struct Probe {
    id: &'static str,
    count: Arc<AtomicU64>,
}

fn descriptor(id: &'static str) -> CollectorDescriptor {
    CollectorDescriptor {
        id,
        display_name: id,
        what_it_watches: "release panic isolation probe",
        why_it_matters: "one failing provider must not terminate the application",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(Duration::from_millis(20)),
        requires_admin: false,
    }
}

impl Collector for Probe {
    fn id(&self) -> &'static str {
        self.id
    }
    fn descriptor(&self) -> CollectorDescriptor {
        descriptor(self.id)
    }
    fn start(&mut self, _: CollectorContext) -> AppResult<()> {
        Ok(())
    }
    fn stop(&mut self) {}
    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        if self.id == "panicking" {
            panic!("intentional release collector probe");
        }
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(Vec::new())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !cfg!(panic = "unwind") {
        return Err("collector isolation requires an unwind release profile".into());
    }
    let count = Arc::new(AtomicU64::new(0));
    let entries = ["panicking", "healthy"]
        .into_iter()
        .map(|id| {
            let count = Arc::clone(&count);
            RegistryEntry::active(descriptor(id), move || {
                Box::new(Probe {
                    id,
                    count: Arc::clone(&count),
                })
            })
        })
        .collect();
    let supervisor = CollectorSupervisor::new(
        CollectorRegistry::new(entries),
        Arc::new(Db::open_in(&what_changed_lib::testing::temp_dir(
            "release-panic",
        ))?),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::from([("panicking".to_owned(), true), ("healthy".to_owned(), true)]),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all()?;
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut after_panic = None;
    let passed = loop {
        let panicked = supervisor
            .list_collector_status()
            .iter()
            .any(|row| row.id == "panicking" && row.consecutive_failures > 0);
        if panicked {
            let initial = *after_panic.get_or_insert_with(|| count.load(Ordering::SeqCst));
            if count.load(Ordering::SeqCst) >= initial + 2 {
                break true;
            }
        }
        if Instant::now() >= deadline {
            break false;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    supervisor.stop_all();
    if !passed {
        return Err("healthy collector did not continue after neighbour panic".into());
    }
    println!("PASS release panic isolation: healthy collector continued after provider panic");
    Ok(())
}
