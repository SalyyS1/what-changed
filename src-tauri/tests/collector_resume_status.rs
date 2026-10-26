#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::CollectorSupervisor;
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::{
    Collector, CollectorContext, RawObservation, StatusDetailsHandle,
};
use what_changed_lib::domain::{
    CollectorCadence, CollectorCost, CollectorDescriptor, CollectorStatusDetails,
    CollectorWakeTier, MonitoringProfile,
};
use what_changed_lib::error::AppResult;
use what_changed_lib::storage::Db;

struct ProbeCollector {
    descriptor: CollectorDescriptor,
    snapshots: Arc<AtomicU64>,
    status_handle: Option<Arc<Mutex<Option<StatusDetailsHandle>>>>,
}

impl Collector for ProbeCollector {
    fn id(&self) -> &'static str {
        self.descriptor.id
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        if self.id() == "drivers" {
            context
                .status_details
                .replace(Some(CollectorStatusDetails {
                    wake_tier: Some(CollectorWakeTier::TimerOnly),
                    docker_state: None,
                    docker_api_version: None,
                }))?;
        }
        if let Some(slot) = &self.status_handle {
            *slot.lock().unwrap() = Some(context.status_details);
        }
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.snapshots.fetch_add(1, Ordering::Relaxed);
        Ok(Vec::new())
    }
}

fn descriptor(id: &'static str) -> CollectorDescriptor {
    CollectorDescriptor {
        id,
        display_name: id,
        what_it_watches: "resume probe",
        why_it_matters: "proves resume wake and diagnostics",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::NotifyPlusSweep(Duration::from_secs(60 * 60)),
        requires_admin: false,
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
fn resume_wakes_the_three_resume_sensitive_sources_and_driver_status_names_the_tier() {
    assert_eq!(
        what_changed_lib::collectors::updates::catalog_descriptor()
            .unwrap()
            .cadence,
        CollectorCadence::NotifyPlusSweep(Duration::from_secs(15 * 60)),
        "the update collector needs a notify-capable cadence for resume wake"
    );
    let updates = Arc::new(AtomicU64::new(0));
    let drivers = Arc::new(AtomicU64::new(0));
    let network = Arc::new(AtomicU64::new(0));
    let stale_driver_status = Arc::new(Mutex::new(None));

    let registry = CollectorRegistry::new(vec![
        RegistryEntry::active(descriptor("updates"), {
            let updates = Arc::clone(&updates);
            move || {
                Box::new(ProbeCollector {
                    descriptor: descriptor("updates"),
                    snapshots: Arc::clone(&updates),
                    status_handle: None,
                })
            }
        }),
        RegistryEntry::active(descriptor("drivers"), {
            let drivers = Arc::clone(&drivers);
            let status = Arc::clone(&stale_driver_status);
            move || {
                Box::new(ProbeCollector {
                    descriptor: descriptor("drivers"),
                    snapshots: Arc::clone(&drivers),
                    status_handle: Some(Arc::clone(&status)),
                })
            }
        }),
        RegistryEntry::active(descriptor("network"), {
            let network = Arc::clone(&network);
            move || {
                Box::new(ProbeCollector {
                    descriptor: descriptor("network"),
                    snapshots: Arc::clone(&network),
                    status_handle: None,
                })
            }
        }),
    ]);
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "resume-status-runtime",
        ))
        .unwrap(),
    );
    let enabled = BTreeMap::from([
        ("updates".to_owned(), true),
        ("drivers".to_owned(), true),
        ("network".to_owned(), true),
    ]);
    let supervisor = CollectorSupervisor::new(
        registry,
        db,
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &enabled,
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    wait_for("initial resume-sensitive snapshots", || {
        updates.load(Ordering::Relaxed) >= 1
            && drivers.load(Ordering::Relaxed) >= 1
            && network.load(Ordering::Relaxed) >= 1
    });

    let update_before = updates.load(Ordering::Relaxed);
    let driver_before = drivers.load(Ordering::Relaxed);
    let network_before = network.load(Ordering::Relaxed);
    supervisor.resume_wake_handle().request();
    wait_for("post-resume snapshots", || {
        updates.load(Ordering::Relaxed) > update_before
            && drivers.load(Ordering::Relaxed) > driver_before
            && network.load(Ordering::Relaxed) > network_before
    });

    let driver_status = supervisor
        .list_collector_status()
        .into_iter()
        .find(|status| status.id == "drivers")
        .expect("driver status exists");
    assert_eq!(
        driver_status.details,
        Some(CollectorStatusDetails {
            wake_tier: Some(CollectorWakeTier::TimerOnly),
            docker_state: None,
            docker_api_version: None,
        })
    );

    let old_handle = stale_driver_status
        .lock()
        .unwrap()
        .clone()
        .expect("start published its status handle");
    supervisor.set_enabled("drivers", false).unwrap();
    assert!(old_handle
        .replace(Some(CollectorStatusDetails {
            wake_tier: Some(CollectorWakeTier::Registry),
            docker_state: None,
            docker_api_version: None,
        }))
        .is_err());
    let driver_status = supervisor
        .list_collector_status()
        .into_iter()
        .find(|status| status.id == "drivers")
        .unwrap();
    assert_eq!(
        driver_status.details,
        Some(CollectorStatusDetails {
            wake_tier: Some(CollectorWakeTier::TimerOnly),
            docker_state: None,
            docker_api_version: None,
        })
    );
}
