#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use what_changed_lib::app::Monitoring;
use what_changed_lib::checkpoint::CheckpointService;
use what_changed_lib::collectors::runtime::CollectorSupervisor;
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::domain::{CheckpointKind, MonitoringProfile};
use what_changed_lib::storage::Db;

#[test]
#[ignore = "real Windows checkpoint performance gate"]
fn all_available_collectors_complete_checkpoint_within_eight_seconds() {
    let registry = what_changed_lib::platform::production_registry();
    let enabled = registry
        .entries()
        .iter()
        .map(|entry| (entry.descriptor.id.to_owned(), entry.factory.is_some()))
        .collect::<BTreeMap<_, _>>();
    let db =
        Arc::new(Db::open_in(&what_changed_lib::testing::temp_dir("checkpoint-real")).unwrap());
    let supervisor = CollectorSupervisor::new(
        registry,
        Arc::clone(&db),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &enabled,
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    let service = CheckpointService::new(Arc::clone(&db), supervisor.capture_handle());
    let began = Instant::now();
    let checkpoint = service
        .create(
            Some("Real collector gate".to_owned()),
            CheckpointKind::Manual,
            None,
            |_| {},
        )
        .unwrap();
    let elapsed = began.elapsed();
    supervisor.stop_all();

    println!(
        "checkpoint real elapsed_ms={} items={} partial={}",
        elapsed.as_millis(),
        checkpoint.item_count,
        checkpoint.partial
    );

    assert!(
        elapsed <= Duration::from_secs(8),
        "capture took {elapsed:?}"
    );
    assert!(
        !checkpoint.partial,
        "missing: {:?}",
        checkpoint.missing_collectors
    );
}
