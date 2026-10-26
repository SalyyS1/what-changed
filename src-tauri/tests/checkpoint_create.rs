#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::fs;
use std::sync::mpsc::sync_channel;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;
use what_changed_lib::app::Monitoring;
use what_changed_lib::checkpoint::{prepare_capture, CheckpointService};
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::CollectorSupervisor;
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::{
    Collector, CollectorContext, CollectorSnapshot, RawBaseline, RawObservation, SnapshotContext,
};
use what_changed_lib::domain::{
    Checkpoint, CheckpointItem, CheckpointKind, CollectorCadence, CollectorCost,
    CollectorDescriptor, MonitoringProfile,
};
use what_changed_lib::error::AppResult;
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{checkpoint_repository, Db};

#[test]
fn scripted_3000_item_checkpoint_is_atomic_compact_and_counted() {
    let directory = what_changed_lib::testing::temp_dir("checkpoint-create-3000");
    let db = Db::open_in(&directory).unwrap();
    let before = database_bytes(&directory);
    let items = (0..3_000)
        .map(|index| CheckpointItem {
            collector_id: "services".to_owned(),
            entity_kind: "service".to_owned(),
            entity_external_id: format!("service-{index}"),
            entity_name: Some(format!("Service {index}")),
            state: json!({ "state": if index % 2 == 0 { "running" } else { "stopped" } }),
        })
        .collect();
    let (capture, _) = prepare_capture(
        Some("Known good".to_owned()),
        CheckpointKind::Manual,
        None,
        10,
        25,
        vec!["services".to_owned()],
        Vec::new(),
        items,
        &Sanitizer::new(),
    );
    let checkpoint = db
        .write(move |conn| checkpoint_repository::create(conn, &capture))
        .unwrap();
    db.flush().unwrap();
    let after = database_bytes(&directory);

    assert_eq!(checkpoint.item_count, 3_000);
    assert_eq!(
        db.read(|conn| checkpoint_repository::items(conn, checkpoint.id))
            .unwrap()
            .len(),
        3_000
    );
    assert!(
        after.saturating_sub(before) <= 2 * 1024 * 1024,
        "checkpoint grew SQLite files by {} bytes",
        after.saturating_sub(before)
    );
}

#[test]
fn a_missing_collector_marks_the_checkpoint_partial() {
    let db = Db::open_in(&what_changed_lib::testing::temp_dir("checkpoint-partial")).unwrap();
    let (capture, _) = prepare_capture(
        None,
        CheckpointKind::Manual,
        None,
        10,
        8_001,
        vec!["services".to_owned()],
        vec!["drivers".to_owned()],
        Vec::new(),
        &Sanitizer::new(),
    );
    let checkpoint = db
        .write(move |conn| checkpoint_repository::create(conn, &capture))
        .unwrap();
    assert!(checkpoint.partial);
    assert_eq!(checkpoint.missing_collectors, ["drivers"]);
}

#[test]
fn completed_noop_without_required_baseline_is_partial_but_empty_baseline_is_complete() {
    let absent = checkpoint_from_services_collector(BaselineMode::Absent);
    assert!(absent.partial);
    assert!(absent.captured_collectors.is_empty());
    assert_eq!(absent.missing_collectors, ["services"]);
    assert_eq!(absent.item_count, 0);

    let acknowledged_empty = checkpoint_from_services_collector(BaselineMode::Empty);
    assert!(!acknowledged_empty.partial);
    assert_eq!(acknowledged_empty.captured_collectors, ["services"]);
    assert!(acknowledged_empty.missing_collectors.is_empty());
    assert_eq!(acknowledged_empty.item_count, 0);
}

#[test]
fn reader_queries_stay_under_20ms_during_checkpoint_writer_work() {
    let db =
        Arc::new(Db::open_in(&what_changed_lib::testing::temp_dir("checkpoint-query")).unwrap());
    let writer_db = Arc::clone(&db);
    let (entered_tx, entered_rx) = sync_channel(1);
    let writer = std::thread::spawn(move || {
        writer_db
            .write(move |conn| {
                conn.execute_batch("BEGIN IMMEDIATE")
                    .expect("begin simulated checkpoint transaction");
                entered_tx.send(()).unwrap();
                std::thread::sleep(Duration::from_millis(120));
                conn.execute_batch("COMMIT")
                    .expect("commit simulated checkpoint transaction");
                Ok(())
            })
            .unwrap();
    });
    entered_rx.recv().unwrap();
    let mut samples = Vec::new();
    for _ in 0..100 {
        let began = Instant::now();
        db.read(checkpoint_repository::list).unwrap();
        samples.push(began.elapsed());
    }
    writer.join().unwrap();
    samples.sort_unstable();
    let p95 = samples[samples.len() * 95 / 100];
    assert!(p95.as_millis() <= 20, "query p95 was {p95:?}");
}

fn database_bytes(directory: &std::path::Path) -> u64 {
    ["events.db", "events.db-wal", "events.db-shm"]
        .into_iter()
        .filter_map(|name| fs::metadata(directory.join(name)).ok())
        .map(|metadata| metadata.len())
        .sum()
}

#[derive(Clone, Copy)]
enum BaselineMode {
    Absent,
    Empty,
}

struct BaselineFixtureCollector {
    mode: BaselineMode,
}

impl Collector for BaselineFixtureCollector {
    fn id(&self) -> &'static str {
        "services"
    }

    fn descriptor(&self) -> CollectorDescriptor {
        services_descriptor()
    }

    fn start(&mut self, _context: CollectorContext) -> AppResult<()> {
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Ok(Vec::new())
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        match self.mode {
            BaselineMode::Absent => Ok(CollectorSnapshot::Noop),
            BaselineMode::Empty => Ok(CollectorSnapshot::Atomic {
                token: context.token(),
                observations: Vec::new(),
                relationships: Vec::new(),
                baseline: Some(RawBaseline {
                    entity_kind: "service_instances".to_owned(),
                    snapshot: json!({
                        "acknowledged": {},
                        "pending": null,
                        "established": true
                    }),
                    captured_at: what_changed_lib::domain::now_ms(),
                }),
                config_snapshots: Vec::new(),
            }),
        }
    }
}

fn checkpoint_from_services_collector(mode: BaselineMode) -> Checkpoint {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(match mode {
            BaselineMode::Absent => "checkpoint-noop-no-baseline",
            BaselineMode::Empty => "checkpoint-valid-empty-baseline",
        }))
        .unwrap(),
    );
    let supervisor = CollectorSupervisor::new(
        CollectorRegistry::new(vec![RegistryEntry::active(
            services_descriptor(),
            move || Box::new(BaselineFixtureCollector { mode }),
        )]),
        Arc::clone(&db),
        Arc::new(Monitoring::running()),
        QueryBusy::default(),
        &BTreeMap::new(),
        MonitoringProfile::Balanced,
    );
    supervisor.spawn_all().unwrap();
    let checkpoint = CheckpointService::new(db, supervisor.capture_handle())
        .create(None, CheckpointKind::Manual, None, |_| {})
        .unwrap();
    supervisor.stop_all();
    checkpoint
}

fn services_descriptor() -> CollectorDescriptor {
    CollectorDescriptor {
        id: "services",
        display_name: "Services",
        what_it_watches: "tests",
        why_it_matters: "tests",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Poll(Duration::from_secs(60)),
        requires_admin: false,
    }
}
