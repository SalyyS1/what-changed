#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;
use what_changed_lib::app::Monitoring;
use what_changed_lib::checkpoint::{compare_item_sets, prepare_capture, CheckpointService};
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::CollectorSupervisor;
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::{
    Collector, CollectorContext, CollectorSnapshot, RawBaseline, RawObservation, SnapshotContext,
};
use what_changed_lib::domain::{
    CheckpointItem, CheckpointKind, CollectorCadence, CollectorCost, CollectorDescriptor,
    MonitoringProfile,
};
use what_changed_lib::error::AppResult;
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{checkpoint_repository, Db};

#[test]
fn comparison_returns_exact_added_removed_and_field_changes() {
    let left = (0..10).map(item_before).collect::<Vec<_>>();
    let mut right = (0..7).map(item_after).collect::<Vec<_>>();
    right.extend((10..15).map(item_after));
    let manifest = vec!["services".to_owned()];
    let diff = compare_item_sets(&left, &manifest, &[], &right, &manifest, &[], vec![4, 7]);

    assert_eq!(diff.added.len(), 5);
    assert_eq!(diff.removed.len(), 3);
    assert_eq!(diff.changed.len(), 2);
    assert_eq!(diff.event_ids_in_window, [4, 7]);
}

#[test]
fn application_install_and_uninstall_are_added_removed_in_application_category() {
    let application = |id: &str, name: &str| CheckpointItem {
        collector_id: "applications".to_owned(),
        entity_kind: "application".to_owned(),
        entity_external_id: id.to_owned(),
        entity_name: Some(name.to_owned()),
        state: json!({ "display_name": name, "version": "1.0" }),
    };
    let left = vec![
        application("kept", "Kept Tool"),
        application("removed", "Removed Tool"),
    ];
    let right = vec![
        application("kept", "Kept Tool"),
        application("added", "Added Tool"),
    ];
    let manifest = vec!["applications".to_owned()];
    let diff = compare_item_sets(&left, &manifest, &[], &right, &manifest, &[], vec![]);

    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.added[0].entity_name, "Added Tool");
    assert_eq!(
        diff.added[0].category,
        what_changed_lib::domain::Category::Application
    );
    assert_eq!(diff.removed.len(), 1);
    assert_eq!(diff.removed[0].entity_name, "Removed Tool");
    assert_eq!(
        diff.removed[0].category,
        what_changed_lib::domain::Category::Application
    );
}

#[test]
fn persisted_3000_item_service_comparison_and_event_window_stay_under_150ms_p95() {
    let left = (0..3_000).map(item_before).collect::<Vec<_>>();
    let right = (0..3_000).map(item_after).collect::<Vec<_>>();
    let directory = what_changed_lib::testing::temp_dir("checkpoint-compare-service-3000");
    let db = Arc::new(Db::open_in(&directory).unwrap());
    let left_id = persist_checkpoint(&db, "Left", 10, left);
    let right_id = persist_checkpoint(&db, "Right", 20, right);
    let event_ids = db
        .insert_events(
            [12, 15, 19, 25]
                .into_iter()
                .enumerate()
                .map(|(index, timestamp)| {
                    Sanitizer::new()
                        .sanitize(what_changed_lib::testing::synthetic_event(
                            index as u64,
                            timestamp,
                        ))
                        .0
                })
                .collect(),
        )
        .unwrap();
    let snapshots = Arc::new(AtomicUsize::new(0));
    let factory_snapshots = Arc::clone(&snapshots);
    let supervisor = CollectorSupervisor::new(
        CollectorRegistry::new(vec![RegistryEntry::active(test_descriptor(), move || {
            Box::new(DurableServicesCollector {
                snapshots: Arc::clone(&factory_snapshots),
            })
        })]),
        Arc::clone(&db),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::new(),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    let ready_deadline = Instant::now() + Duration::from_secs(2);
    while supervisor.list_collector_status()[0].last_run_at.is_none() {
        assert!(
            Instant::now() < ready_deadline,
            "durable baseline was not captured"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let service = CheckpointService::new(Arc::clone(&db), supervisor.capture_handle());

    let mut samples = Vec::new();
    for _ in 0..20 {
        let began = Instant::now();
        let diff = service.compare(left_id, Some(right_id)).unwrap();
        assert_eq!(diff.changed.len(), 2);
        assert_eq!(diff.event_ids_in_window, event_ids[..3]);
        assert_eq!(diff.window_from, 10);
        assert_eq!(diff.window_to, 20);
        samples.push(began.elapsed());
    }
    samples.sort_unstable();
    let p95 = samples[percentile_index(samples.len(), 95)];
    assert!(
        p95 <= Duration::from_millis(150),
        "comparison p95 was {p95:?}"
    );

    let snapshots_before_compare = snapshots.load(Ordering::Acquire);
    let mut current_samples = Vec::new();
    for _ in 0..20 {
        let began = Instant::now();
        let now = service.compare(left_id, None).unwrap();
        assert_eq!(now.changed.len(), 2);
        assert_eq!(now.event_ids_in_window, event_ids);
        assert!(!now.right_missing_collectors.contains("services"));
        current_samples.push(began.elapsed());
    }
    current_samples.sort_unstable();
    let current_p95 = current_samples[percentile_index(current_samples.len(), 95)];
    assert!(
        current_p95 <= Duration::from_millis(150),
        "compare-to-now p95 was {current_p95:?}"
    );
    assert_eq!(
        snapshots.load(Ordering::Acquire),
        snapshots_before_compare,
        "compare-to-now must not request a collector sweep"
    );
    supervisor.stop_all();
}

fn percentile_index(sample_count: usize, percentile: usize) -> usize {
    // Nearest-rank percentile is one-based. The old `n * 95 / 100` index
    // selected the maximum for 20 samples, accidentally turning the p95 gate
    // into a p100/outlier gate.
    (sample_count * percentile).div_ceil(100).saturating_sub(1)
}

#[test]
fn p95_uses_nearest_rank_instead_of_the_maximum_sample() {
    assert_eq!(percentile_index(20, 95), 18);
    assert_eq!(percentile_index(100, 95), 94);
}

struct DurableServicesCollector {
    snapshots: Arc<AtomicUsize>,
}

impl Collector for DurableServicesCollector {
    fn id(&self) -> &'static str {
        "services"
    }

    fn descriptor(&self) -> CollectorDescriptor {
        test_descriptor()
    }

    fn start(&mut self, _ctx: CollectorContext) -> AppResult<()> {
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Ok(Vec::new())
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        self.snapshots.fetch_add(1, Ordering::AcqRel);
        let acknowledged = (0..3_000)
            .map(|index| (format!("service-{index}"), item_after(index).state))
            .collect::<serde_json::Map<String, serde_json::Value>>();
        Ok(CollectorSnapshot::Atomic {
            token: context.token(),
            observations: Vec::new(),
            relationships: Vec::new(),
            baseline: Some(RawBaseline {
                entity_kind: "service_instances".to_owned(),
                snapshot: json!({ "acknowledged": acknowledged }),
                captured_at: what_changed_lib::domain::now_ms(),
            }),
            config_snapshots: Vec::new(),
        })
    }
}

fn test_descriptor() -> CollectorDescriptor {
    CollectorDescriptor {
        id: "services",
        display_name: "Services",
        what_it_watches: "tests",
        why_it_matters: "tests",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(std::time::Duration::from_secs(60)),
        requires_admin: false,
    }
}

fn persist_checkpoint(db: &Db, name: &str, created_at: i64, items: Vec<CheckpointItem>) -> i64 {
    let (capture, _) = prepare_capture(
        Some(name.to_owned()),
        CheckpointKind::Manual,
        None,
        created_at,
        1,
        vec!["services".to_owned()],
        Vec::new(),
        items,
        &Sanitizer::new(),
    );
    db.write(move |connection| checkpoint_repository::create(connection, &capture))
        .unwrap()
        .id
}

fn item_before(index: usize) -> CheckpointItem {
    CheckpointItem {
        collector_id: "services".to_owned(),
        entity_kind: "service".to_owned(),
        entity_external_id: format!("service-{index}"),
        entity_name: Some(format!("Service {index}")),
        state: json!({
            "name": format!("service-{index}"),
            "display_name": format!("Service {index}"),
            "state": "running",
            "start_type": "automatic",
            "config_available": true,
            "process_id": index,
        }),
    }
}

fn item_after(index: usize) -> CheckpointItem {
    let mut item = item_before(index);
    if index < 2 {
        item.state["state"] = json!("stopped");
        item.state["process_id"] = json!(null);
    }
    item
}
