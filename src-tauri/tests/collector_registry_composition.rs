#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::sync::Arc;

use what_changed_lib::app::{AppPaths, AppState};
use what_changed_lib::collectors::registry::{CollectorFactory, CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::{Collector, CollectorContext, RawObservation};
use what_changed_lib::domain::{CollectorCadence, CollectorCost, CollectorDescriptor};
use what_changed_lib::dto::CollectorState;
use what_changed_lib::error::{AppError, AppResult};
use what_changed_lib::storage::{settings_repository, Db};

struct ProcessCollector;

impl Collector for ProcessCollector {
    fn id(&self) -> &'static str {
        "process"
    }

    fn descriptor(&self) -> CollectorDescriptor {
        process_descriptor()
    }

    fn start(&mut self, _ctx: CollectorContext) -> AppResult<()> {
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Ok(Vec::new())
    }
}

fn process_descriptor() -> CollectorDescriptor {
    CollectorDescriptor {
        id: "process",
        display_name: "Processes",
        what_it_watches: "programs starting and stopping",
        why_it_matters: "shows which program changed the machine",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(std::time::Duration::from_secs(1)),
        requires_admin: false,
    }
}

fn process_factory() -> CollectorFactory {
    Arc::new(|| Box::new(ProcessCollector))
}

fn rejected(result: AppResult<CollectorRegistry>) -> AppError {
    match result {
        Ok(_) => panic!("registry activation unexpectedly succeeded"),
        Err(error) => error,
    }
}

#[test]
fn activating_a_known_entry_preserves_catalog_order_and_descriptor() {
    let before = CollectorRegistry::production();
    let descriptors: Vec<_> = before
        .entries()
        .iter()
        .map(|entry| entry.descriptor.clone())
        .collect();

    let after = before.with_factory("process", process_factory()).unwrap();

    assert_eq!(
        after
            .entries()
            .iter()
            .map(|entry| entry.descriptor.clone())
            .collect::<Vec<_>>(),
        descriptors,
    );
    assert!(after.entries()[0].factory.is_some());
    assert!(after.entries()[1..]
        .iter()
        .all(|entry| entry.factory.is_none()));
}

#[test]
fn activation_rejects_unknown_duplicate_and_admin_entries_without_echoing_ids() {
    let unknown_id = "secret-looking-collector-id";
    let unknown =
        rejected(CollectorRegistry::production().with_factory(unknown_id, process_factory()));
    assert!(matches!(unknown, AppError::InvalidInput { .. }));
    assert!(!unknown.to_string().contains(unknown_id));

    let active = CollectorRegistry::production()
        .with_factory("process", process_factory())
        .unwrap();
    let duplicate = rejected(active.with_factory("process", process_factory()));
    assert!(duplicate.to_string().contains("already active"));

    let admin = CollectorRegistry::new(vec![RegistryEntry::unavailable(CollectorDescriptor {
        requires_admin: true,
        ..process_descriptor()
    })]);
    let admin = rejected(admin.with_factory("process", process_factory()));
    assert!(admin.to_string().contains("administrator-required"));
}

#[test]
fn app_state_uses_the_injected_registry_while_default_open_stays_unavailable() {
    let injected_dir = what_changed_lib::testing::temp_dir("app-state-injected-registry");
    let injected_paths = AppPaths::under(&injected_dir);
    let seed = Db::open(&injected_paths.db).unwrap();
    seed.write(|conn| {
        settings_repository::put(conn, "onboarding_completed", &true)?;
        settings_repository::put(
            conn,
            "collectors",
            &BTreeMap::from([(String::from("process"), true)]),
        )
    })
    .unwrap();
    seed.close();
    let injected = CollectorRegistry::production()
        .with_constructor("process", || Box::new(ProcessCollector))
        .unwrap();
    let state = AppState::open_with_registry(injected_paths, injected).unwrap();
    let process = state
        .collectors
        .list_collector_status()
        .into_iter()
        .find(|status| status.id == "process")
        .unwrap();
    assert_eq!(process.state, CollectorState::Idle);
    state.shutdown_cleanly();

    let default_dir = what_changed_lib::testing::temp_dir("app-state-default-registry");
    let state = AppState::open(AppPaths::under(&default_dir)).unwrap();
    let process = state
        .collectors
        .list_collector_status()
        .into_iter()
        .find(|status| status.id == "process")
        .unwrap();
    assert_eq!(process.state, CollectorState::Unavailable);
    state.shutdown_cleanly();
}
