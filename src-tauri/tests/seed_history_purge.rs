//! The devtools seeder's persisted corpus and its exact-marker purge boundary.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;
use what_changed_lib::checkpoint::{compare_item_sets, prepare_capture};
use what_changed_lib::domain::{
    ids, Category, CheckpointKind, Entity, EntityType, EventQuery, InvestigationWindow,
    Relationship, RelationshipKind, RetentionPolicy, TimeRange, DAY_MS,
};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::services::cluster::TimelineItem;
use what_changed_lib::services::investigation_service::InvestigationService;
use what_changed_lib::services::query_service;
use what_changed_lib::storage::{
    checkpoint_repository, entity_repository, event_repository, investigations, maintenance, Db,
    Fts5Index, SearchIndex,
};
use what_changed_lib::testing::synthetic_event;

const END_AT: i64 = 1_800_000_000_000;
const QA_LOCK_FILE: &str = ".what-changed-seed.lock";
const QA_SENTINEL_FILE: &str = ".what-changed-seed-owner.json";

struct OwnedDirectory(PathBuf);

impl OwnedDirectory {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "what-changed-seed-history-{label}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir(&root).expect("create exclusively owned seeder fixture");
        Self(root)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            if self.0.exists() {
                eprintln!("seeder fixture cleanup failed: {error}");
            }
        }
    }
}

fn persist_checkpoint(db: &Db, name: &str, reason: &str) {
    let (capture, _) = prepare_capture(
        Some(name.to_owned()),
        CheckpointKind::Manual,
        Some(reason.to_owned()),
        END_AT,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        &Sanitizer::new(),
    );
    db.write(move |conn| checkpoint_repository::create(conn, &capture))
        .expect("persist checkpoint fixture");
}

fn persist_investigation(db: &Arc<Db>, problem: String) {
    InvestigationService::new(Arc::clone(db))
        .start(
            problem,
            Some(InvestigationWindow::Custom {
                from_ms: END_AT,
                to_ms: END_AT + 1_000,
            }),
        )
        .expect("persist investigation fixture");
}

fn run_seeder(directory: &OwnedDirectory, args: &[&str]) -> String {
    let output = invoke_seeder(directory, args);
    assert!(
        output.status.success(),
        "seed_history failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("seeder output is UTF-8")
}

fn invoke_seeder(directory: &OwnedDirectory, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_seed_history"))
        .args(args)
        .arg("--data-dir")
        .arg(directory.path())
        .output()
        .expect("launch devtools seeder")
}

fn first_related_seed_entity(
    seed: u64,
) -> (String, EntityType, String, EntityType, RelationshipKind) {
    for index in 0..15_u64 {
        let event = synthetic_event(seed.wrapping_add(index).rotate_left(17), END_AT);
        let label = format!("synthetic-entity-{index}");
        let next_service = ids::service(&format!("synthetic-entity-{}", (index + 1) % 3_000));
        let port = ids::port(ids::Protocol::Tcp, "127.0.0.1", 20_000 + index as u16);
        let candidate = match event.category {
            Category::Process => Some((
                ids::process(&format!(r"C:\synthetic\{label}.exe")),
                EntityType::Process,
                port,
                EntityType::Port,
                RelationshipKind::ListensOn,
            )),
            Category::Docker => Some((
                ids::container(&format!("{index:064x}")),
                EntityType::Container,
                port,
                EntityType::Port,
                RelationshipKind::Publishes,
            )),
            Category::Configuration | Category::Filesystem => Some((
                ids::file(&format!(r"C:\synthetic\{label}.json")),
                EntityType::File,
                next_service,
                EntityType::Service,
                RelationshipKind::ConfigOf,
            )),
            Category::Application => Some((
                ids::app("synthetic", &label),
                EntityType::Application,
                next_service,
                EntityType::Service,
                RelationshipKind::Provides,
            )),
            _ => None,
        };
        if let Some(candidate) = candidate {
            return candidate;
        }
    }
    panic!("the first 15 related events contain a graph-supported category")
}

#[test]
fn cli_refuses_missing_data_dir_and_nonempty_seed_directory() {
    let missing = Command::new(env!("CARGO_BIN_EXE_seed_history"))
        .args(["--events", "100"])
        .output()
        .expect("launch seeder without data directory");
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("--data-dir is required"));

    let directory = OwnedDirectory::new("nonempty");
    let note = directory.path().join("real-user-note.txt");
    std::fs::write(&note, b"keep this real file").expect("write unrelated file");
    let refused = invoke_seeder(&directory, &["--events", "100"]);
    assert!(!refused.status.success());
    assert_eq!(
        std::fs::read(&note).expect("unrelated file survives"),
        b"keep this real file"
    );
    assert!(!directory.path().join("events.db").exists());
    assert!(!directory.path().join(QA_SENTINEL_FILE).exists());

    let normal = OwnedDirectory(
        std::env::temp_dir().join(format!("real-history-{}", uuid::Uuid::new_v4().simple())),
    );
    std::fs::create_dir(normal.path()).expect("create normal-looking test directory");
    assert!(!invoke_seeder(&normal, &["--events", "100"])
        .status
        .success());
    assert!(!normal.path().join("events.db").exists());
}

#[test]
fn no_relations_seed_has_neither_related_ids_nor_graph_edges() {
    let directory = OwnedDirectory::new("no-relations");
    let output = run_seeder(
        &directory,
        &[
            "--events",
            "100",
            "--no-relations",
            "--end-at",
            "1800000000000",
        ],
    );
    assert!(output.contains("relations=false"));

    let db = Db::open_in(directory.path()).expect("inspect write-only QA corpus");
    db.read(|conn| {
        assert_eq!(event_repository::count(conn)?, 100);
        let related_events: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE json_array_length(related_entity_ids) > 0",
                [],
                |row| row.get(0),
            )
            .expect("count write-only related IDs");
        let graph_edges: i64 = conn
            .query_row("SELECT COUNT(*) FROM entity_relationships", [], |row| {
                row.get(0)
            })
            .expect("count write-only graph edges");
        assert_eq!(related_events, 0);
        assert_eq!(graph_edges, 0);
        Ok(())
    })
    .expect("write-only corpus has no graph claims");
    db.close();
}

#[test]
fn collector_payload_sensitivity_keeps_corpus_identity_and_persists_checked_in_shapes() {
    let canonical = OwnedDirectory::new("payload-canonical");
    let sensitivity = OwnedDirectory::new("payload-sensitivity");
    let base_args = [
        "--events",
        "1000",
        "--days",
        "90",
        "--end-at",
        "1800000000000",
    ];
    let default_output = run_seeder(&canonical, &base_args);
    assert!(default_output.contains("collector_payload_sensitivity=false"));
    let sensitive_output = run_seeder(
        &sensitivity,
        &[
            "--events",
            "1000",
            "--days",
            "90",
            "--end-at",
            "1800000000000",
            "--collector-payload-sensitivity",
        ],
    );
    assert!(sensitive_output.contains("collector_payload_sensitivity=true"));

    type Fingerprint = (i64, i64, String, String, String, i64);
    let inspect = |directory: &OwnedDirectory| {
        let db = Db::open_in(directory.path()).expect("open payload corpus");
        let result = db
            .read(|conn| {
                let mut statement = conn
                    .prepare(
                        "SELECT id, timestamp, category, entity_id, related_entity_ids, \
                         json_extract(metadata_json, '$.burst') FROM events ORDER BY id",
                    )
                    .expect("prepare payload parity rows");
                let fingerprints = statement
                    .query_map([], |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    })
                    .expect("read payload parity rows")
                    .collect::<rusqlite::Result<Vec<Fingerprint>>>()
                    .expect("decode payload parity rows");
                let payload_bytes: i64 = conn
                    .query_row(
                        "SELECT SUM(COALESCE(LENGTH(before_json), 0) + \
                         COALESCE(LENGTH(after_json), 0) + LENGTH(metadata_json)) FROM events",
                        [],
                        |row| row.get(0),
                    )
                    .expect("sum persisted payload lengths");
                let edges: i64 = conn
                    .query_row("SELECT COUNT(*) FROM entity_relationships", [], |row| {
                        row.get(0)
                    })
                    .expect("count graph evidence");
                let entities: i64 = conn
                    .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
                    .expect("count graph entities");
                Ok((fingerprints, payload_bytes, edges, entities))
            })
            .expect("inspect payload corpus");
        db.close();
        result
    };
    let (default_rows, default_bytes, default_edges, default_entities) = inspect(&canonical);
    let (sensitive_rows, sensitive_bytes, sensitive_edges, sensitive_entities) =
        inspect(&sensitivity);
    assert_eq!(
        default_rows, sensitive_rows,
        "opt-in payloads cannot change identity, time, category, graph claims, or burst schedule"
    );
    assert_eq!(default_rows.len(), 1_000);
    assert_eq!((default_edges, sensitive_edges), (150, 150));
    assert_eq!((default_entities, sensitive_entities), (3_000, 3_000));
    assert!(
        sensitive_bytes > default_bytes,
        "collector-shaped values must increase persisted payload bytes"
    );

    let db = Db::open_in(sensitivity.path()).expect("inspect collector-shaped values");
    db.read(|conn| {
        let count = |sql: &str| -> i64 {
            conn.query_row(sql, [], |row| row.get(0))
                .expect("count collector-shaped rows")
        };
        for (category, shape) in [
            (
                "application",
                "event_type = 'updated' AND field = 'version' AND before_json = '\"4.35.1\"' \
                 AND after_json = '\"4.36.0\"' AND json_extract(metadata_json, '$.publisher') = 'Example publisher'",
            ),
            (
                "process",
                "event_type = 'started' AND before_json IS NULL AND after_json IS NULL \
                 AND json_extract(metadata_json, '$.path_unavailable') = 0 \
                 AND json_extract(metadata_json, '$.path') LIKE 'C:\\synthetic\\synthetic-entity-%'",
            ),
            (
                "network",
                "event_type = 'modified' AND field = 'ip_addresses' \
                 AND before_json = '[\"192.168.1.10/24\"]' AND after_json = '[\"10.0.0.2/24\"]' \
                 AND json_extract(metadata_json, '$.change_kind') = 'ip_changed'",
            ),
            (
                "service",
                "event_type = 'stopped' AND field = 'state' AND before_json = '\"running\"' \
                 AND after_json = '\"stopped\"' AND json_extract(metadata_json, '$.start_type') = 'Automatic'",
            ),
        ] {
            let all = count(&format!("SELECT COUNT(*) FROM events WHERE category = '{category}'"));
            let matching = count(&format!(
                "SELECT COUNT(*) FROM events WHERE category = '{category}' AND {shape}"
            ));
            assert_eq!(matching, all, "every {category} row must carry its collector shape");
        }
        let system = count("SELECT COUNT(*) FROM events WHERE category = 'system'");
        let shutdowns = count(
            "SELECT COUNT(*) FROM events WHERE category = 'system' \
             AND event_type = 'failed' AND json_extract(metadata_json, '$.event_id') = 41 \
             AND before_json IS NULL AND after_json IS NULL",
        );
        assert_eq!(shutdowns, system / 10);
        let app_nonburst_length: i64 = conn
            .query_row(
                "SELECT LENGTH(before_json) + LENGTH(after_json) + LENGTH(metadata_json) \
                 FROM events WHERE category = 'application' \
                 AND json_extract(metadata_json, '$.burst') = 0 LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("measure checked-in application version payload");
        assert_eq!(app_nonburst_length, 89);
        let undersized = count(
            "SELECT COUNT(*) FROM events WHERE \
             (category = 'process' AND LENGTH(metadata_json) < 157) \
             OR (category = 'network' AND LENGTH(metadata_json) < 150) \
             OR (category = 'service' AND LENGTH(metadata_json) < 181) \
             OR (category = 'system' AND json_extract(metadata_json, '$.event_id') = 41 \
                 AND LENGTH(metadata_json) < 255)",
        );
        assert_eq!(undersized, 0, "collector payload lengths must not collapse to the tiny canonical metadata");
        Ok(())
    })
    .expect("checked-in payload shapes are persisted");
    db.close();
}

#[test]
fn wrong_owner_marker_and_stale_lock_refuse_purge() {
    let directory = OwnedDirectory::new("wrong-owner");
    run_seeder(
        &directory,
        &["--events", "100", "--end-at", "1800000000000"],
    );
    let owner_path = directory.path().join(QA_SENTINEL_FILE);
    let original_owner = std::fs::read(&owner_path).expect("read authentic QA owner marker");
    let wrong_owner = json!({
        "version": 1,
        "canonical_dir": "C:\\not-the-qa-directory",
        "nonce": uuid::Uuid::new_v4().simple().to_string(),
    });
    std::fs::write(&owner_path, wrong_owner.to_string()).expect("tamper test marker");
    assert!(!invoke_seeder(&directory, &["--purge-synthetic"])
        .status
        .success());
    std::fs::write(&owner_path, original_owner).expect("restore authentic test marker");

    let lock_path = directory.path().join(QA_LOCK_FILE);
    std::fs::write(&lock_path, b"interrupted run").expect("simulate stale lock");
    assert!(!invoke_seeder(&directory, &["--purge-synthetic"])
        .status
        .success());
    std::fs::remove_file(&lock_path).expect("clear simulated stale test lock");

    let db = Db::open_in(directory.path()).expect("inspect refusal without mutation");
    db.read(|conn| {
        assert_eq!(event_repository::count(conn)?, 100);
        assert_eq!(checkpoint_repository::list(conn)?.len(), 40);
        assert_eq!(investigations::count(conn)?, 20);
        Ok(())
    })
    .expect("failed purge did not mutate corpus");
    db.close();
    drop(db);
    assert!(run_seeder(&directory, &["--purge-synthetic"]).contains("purged 100"));
    assert!(run_seeder(&directory, &["--purge-synthetic"]).contains("purged 0"));
}

#[test]
fn concurrent_purge_refuses_while_seed_holds_directory_lock() {
    let directory = OwnedDirectory::new("concurrent-lock");
    let mut child = Command::new(env!("CARGO_BIN_EXE_seed_history"))
        .args(["--events", "10000", "--end-at", "1800000000000"])
        .arg("--data-dir")
        .arg(directory.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("launch first seeder process");
    let lock_path = directory.path().join(QA_LOCK_FILE);
    let owner_path = directory.path().join(QA_SENTINEL_FILE);
    let database = directory.path().join("events.db");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !(lock_path.exists() && owner_path.exists() && database.exists()) {
        assert!(
            Instant::now() < deadline,
            "seed did not establish QA ownership"
        );
        assert!(
            child
                .try_wait()
                .expect("check first seed process")
                .is_none(),
            "seed exited before lock contention could be tested"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let refused = invoke_seeder(&directory, &["--purge-synthetic"]);
    assert!(!refused.status.success(), "purge must not race active seed");
    let first = child
        .wait_with_output()
        .expect("wait for first seed process");
    assert!(
        first.status.success(),
        "first seed failed: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(run_seeder(&directory, &["--purge-synthetic"]).contains("purged 10000"));
}

#[test]
fn registered_purge_is_exact_idempotent_and_preserves_real_lookalikes() {
    let directory = OwnedDirectory::new("scoped-purge");
    let db = Arc::new(Db::open_in(directory.path()).expect("open fixture"));
    let marker = maintenance::register_synthetic_run(&db).expect("register seeded run");
    let crashed_marker = maintenance::register_synthetic_run(&db)
        .expect("register interrupted run without artifacts");
    assert_ne!(marker, crashed_marker);

    let real_entity_id = ids::service("synthetic-lookalike");
    let real_entity = Entity {
        id: real_entity_id.clone(),
        entity_type: EntityType::Service,
        display_name: "A real synthetic-named service".to_owned(),
        first_seen: END_AT,
        last_seen: END_AT,
        metadata: None,
    };
    let (real_entity, _) = Sanitizer::new().sanitize(real_entity);
    db.write(move |conn| entity_repository::upsert_sanitized(conn, &real_entity))
        .expect("persist real lookalike entity");

    let mut real_event = synthetic_event(1, END_AT);
    real_event.source_collector = "services".to_owned();
    real_event.entity_id = real_entity_id.clone();
    real_event.user_friendly_summary = "realpreservedneedle".to_owned();
    let mut seeded_event = synthetic_event(2, END_AT + 1);
    seeded_event.source_collector = "synthetic".to_owned();
    seeded_event.user_friendly_summary = "syntheticremovedneedle".to_owned();
    let inserted = db
        .insert_events(vec![
            Sanitizer::new().sanitize(real_event).0,
            Sanitizer::new().sanitize(seeded_event).0,
        ])
        .expect("persist real and synthetic events");
    db.read(|conn| {
        assert_eq!(
            Fts5Index.search(conn, "realpreservedneedle", 10)?,
            vec![inserted[0]]
        );
        assert_eq!(
            Fts5Index.search(conn, "syntheticremovedneedle", 10)?,
            vec![inserted[1]]
        );
        Ok(())
    })
    .expect("both events are searchable before purge");

    persist_checkpoint(&db, "Synthetic fixture", &marker);
    persist_checkpoint(
        &db,
        "Real synthetic-named marker",
        &format!("{marker} real suffix"),
    );
    persist_investigation(&db, marker.clone());
    persist_investigation(&db, format!("{marker} real suffix"));
    assert_eq!(maintenance::purge_synthetic(&db).expect("first purge"), 1);
    assert_eq!(maintenance::purge_synthetic(&db).expect("repeat purge"), 0);

    db.read(|conn| {
        assert_eq!(event_repository::count(conn)?, 1);
        let checkpoints = checkpoint_repository::list(conn)?;
        assert_eq!(checkpoints.len(), 1);
        assert_eq!(checkpoints[0].name, "Real synthetic-named marker");
        let investigations = investigations::list(conn, 100)?;
        assert_eq!(investigations.len(), 1);
        assert_eq!(investigations[0].problem, format!("{marker} real suffix"));
        assert!(entity_repository::by_id(conn, &real_entity_id)?.is_some());
        assert_eq!(
            Fts5Index.search(conn, "realpreservedneedle", 10)?,
            vec![inserted[0]]
        );
        assert!(Fts5Index
            .search(conn, "syntheticremovedneedle", 10)?
            .is_empty());
        Ok(())
    })
    .expect("real lookalike history survives");
    db.close();
}

#[test]
fn retention_then_purge_preserves_edges_without_attributable_event_provenance() {
    let directory = OwnedDirectory::new("retained-edge-provenance");
    let db = Db::open_in(directory.path()).expect("open retention fixture");
    maintenance::register_synthetic_run(&db).expect("register seeded run");

    let expired_at = END_AT - 100 * DAY_MS;
    let source_id = ids::file(r"C:\synthetic\shared-config.json");
    let synthetic_target_id = ids::service("synthetic-target");
    let real_target_id = ids::service("real-target");
    let mut old_event = synthetic_event(18, expired_at);
    old_event.category = Category::Configuration;
    old_event.entity_type = EntityType::File;
    old_event.entity_id = source_id.clone();
    old_event.related_entity_ids = vec![synthetic_target_id.clone()];
    let mut real_event = synthetic_event(19, END_AT);
    real_event.category = Category::Configuration;
    real_event.entity_type = EntityType::File;
    real_event.entity_id = source_id.clone();
    real_event.related_entity_ids = vec![real_target_id.clone()];
    real_event.source_collector = "config_files".to_owned();
    let event_ids = db
        .insert_events(vec![
            Sanitizer::new().sanitize(old_event).0,
            Sanitizer::new().sanitize(real_event).0,
        ])
        .expect("persist synthetic and real evidence on the same entity");

    let target = |id: String, name: &str, at: i64| Entity {
        id,
        entity_type: EntityType::Service,
        display_name: name.to_owned(),
        first_seen: at,
        last_seen: at,
        metadata: None,
    };
    let (synthetic_target, _) = Sanitizer::new().sanitize(target(
        synthetic_target_id.clone(),
        "Synthetic target",
        expired_at,
    ));
    let (real_target, _) =
        Sanitizer::new().sanitize(target(real_target_id.clone(), "Real target", END_AT));
    let synthetic_edge = Relationship {
        source_id: source_id.clone(),
        target_id: synthetic_target_id.clone(),
        kind: RelationshipKind::ConfigOf,
        event_id: Some(event_ids[0]),
        recorded_at: expired_at,
    };
    let real_edge = Relationship {
        source_id: source_id.clone(),
        target_id: real_target_id.clone(),
        kind: RelationshipKind::ConfigOf,
        event_id: Some(event_ids[1]),
        recorded_at: END_AT,
    };
    db.write(move |conn| {
        entity_repository::upsert_sanitized(conn, &synthetic_target)?;
        entity_repository::upsert_sanitized(conn, &real_target)?;
        entity_repository::insert_relationship(conn, &synthetic_edge)?;
        entity_repository::insert_relationship(conn, &real_edge)
    })
    .expect("persist both graph claims");

    let report = maintenance::run_pass(&db, RetentionPolicy::default(), END_AT)
        .expect("expire the old synthetic event");
    assert_eq!(report.events_deleted, 1);
    db.read(|conn| {
        assert_eq!(event_repository::count(conn)?, 1);
        let edges = entity_repository::relationships_from(conn, &source_id)?;
        assert_eq!(edges.len(), 2);
        let expired = edges
            .iter()
            .find(|edge| edge.target_id == synthetic_target_id)
            .expect("retention does not delete the old graph fact");
        assert_eq!(
            expired.event_id, None,
            "retention nulls expired evidence FK"
        );
        let real = edges
            .iter()
            .find(|edge| edge.target_id == real_target_id)
            .expect("real edge shares the source entity");
        assert_eq!(real.event_id, Some(event_ids[1]));
        Ok(())
    })
    .expect("inspect retained edge provenance");

    assert_eq!(
        maintenance::purge_synthetic(&db).expect("purge tagged run"),
        0
    );
    db.read(|conn| {
        assert_eq!(event_repository::count(conn)?, 1);
        let survivor: String = conn
            .query_row(
                "SELECT source_collector FROM events WHERE id = ?1",
                [event_ids[1]],
                |row| row.get(0),
            )
            .expect("real event still exists");
        assert_eq!(survivor, "config_files");
        let edges = entity_repository::relationships_from(conn, &source_id)?;
        assert_eq!(edges.len(), 2, "endpoints cannot prove synthetic ownership");
        assert_eq!(
            edges
                .iter()
                .find(|edge| edge.target_id == synthetic_target_id)
                .expect("unattributable old edge remains")
                .event_id,
            None
        );
        assert_eq!(
            edges
                .iter()
                .find(|edge| edge.target_id == real_target_id)
                .expect("real edge remains")
                .event_id,
            Some(event_ids[1])
        );
        Ok(())
    })
    .expect("purge preserves real and unattributable graph rows");
    db.close();
}

#[test]
fn nonfresh_database_with_real_entity_and_edge_is_rejected_unchanged() {
    let directory = OwnedDirectory::new("real-collision");
    let db = Db::open_in(directory.path()).expect("open real graph fixture");
    let (source_id, source_type, target_id, target_type, kind) = first_related_seed_entity(42);
    let real_at = END_AT - 2 * 86_400_000;
    let mut real_event = synthetic_event(777, real_at);
    real_event.source_collector = "services".to_owned();
    real_event.entity_id = source_id.clone();
    real_event.entity_type = source_type;
    let real_event_id = db
        .insert_events(vec![Sanitizer::new().sanitize(real_event).0])
        .expect("persist real graph evidence")[0];
    let source = Entity {
        id: source_id.clone(),
        entity_type: source_type,
        display_name: "Real source despite synthetic-looking id".to_owned(),
        first_seen: real_at,
        last_seen: real_at,
        metadata: Some(json!({"owner": "real"})),
    };
    let target = Entity {
        id: target_id.clone(),
        entity_type: target_type,
        display_name: "Real target despite synthetic-looking id".to_owned(),
        first_seen: real_at,
        last_seen: real_at,
        metadata: Some(json!({"owner": "real"})),
    };
    let inventory_id = ids::service("synthetic-entity-0");
    let real_inventory = Entity {
        id: inventory_id.clone(),
        entity_type: EntityType::Service,
        display_name: "Real service with a seeded inventory ID".to_owned(),
        first_seen: real_at,
        last_seen: real_at,
        metadata: Some(json!({"owner": "real inventory"})),
    };
    let (source, _) = Sanitizer::new().sanitize(source);
    let (target, _) = Sanitizer::new().sanitize(target);
    let (real_inventory, _) = Sanitizer::new().sanitize(real_inventory);
    let edge = Relationship {
        source_id: source_id.clone(),
        target_id: target_id.clone(),
        kind,
        event_id: Some(real_event_id),
        recorded_at: real_at,
    };
    db.write(move |conn| {
        entity_repository::upsert_sanitized(conn, &source)?;
        entity_repository::upsert_sanitized(conn, &target)?;
        entity_repository::upsert_sanitized(conn, &real_inventory)?;
        entity_repository::insert_relationship(conn, &edge)
    })
    .expect("persist real graph row");
    db.close();
    drop(db);

    let output = invoke_seeder(
        &directory,
        &[
            "--events",
            "100",
            "--days",
            "1",
            "--seed",
            "42",
            "--end-at",
            "1800000000000",
        ],
    );
    assert!(!output.status.success(), "seed must refuse existing DB");
    let output = invoke_seeder(&directory, &["--purge-synthetic"]);
    assert!(!output.status.success(), "purge must refuse a non-QA DB");
    let db = Db::open_in(directory.path()).expect("inspect refused graph collision");
    db.read(|conn| {
        assert_eq!(event_repository::count(conn)?, 1);
        let source = entity_repository::by_id(conn, &source_id)?.expect("source survives");
        let target = entity_repository::by_id(conn, &target_id)?.expect("target survives");
        assert_eq!(
            source.display_name,
            "Real source despite synthetic-looking id"
        );
        assert_eq!(
            target.display_name,
            "Real target despite synthetic-looking id"
        );
        assert_eq!(source.last_seen, real_at);
        assert_eq!(target.last_seen, real_at);
        assert_eq!(source.metadata, Some(json!({"owner": "real"})));
        let inventory =
            entity_repository::by_id(conn, &inventory_id)?.expect("real service survives");
        assert_eq!(
            inventory.display_name,
            "Real service with a seeded inventory ID"
        );
        assert_eq!(inventory.first_seen, real_at);
        assert_eq!(inventory.last_seen, real_at);
        assert_eq!(inventory.metadata, Some(json!({"owner": "real inventory"})));
        let edges = entity_repository::relationships_from(conn, &source_id)?;
        let preserved = edges
            .iter()
            .find(|edge| edge.target_id == target_id && edge.kind == kind)
            .expect("real edge still exists");
        assert_eq!(preserved.event_id, Some(real_event_id));
        assert_eq!(preserved.recorded_at, real_at);
        Ok(())
    })
    .expect("refused seed and purge preserved real graph provenance");
    db.close();
}

#[test]
fn ten_thousand_seed_has_required_distribution_and_purge_removes_its_artifacts() {
    let directory = OwnedDirectory::new("10k-corpus");
    let output = run_seeder(
        &directory,
        &[
            "--events",
            "10000",
            "--days",
            "90",
            "--seed",
            "42",
            "--end-at",
            "1800000000000",
        ],
    );
    assert!(output.contains("events=10000 entities=3000 checkpoints=40 investigations=20"));

    let db = Db::open_in(directory.path()).expect("inspect seeded corpus");
    db.read(|conn| {
        assert_eq!(event_repository::count(conn)?, 10_000);
        let graph_entities: i64 = conn
            .query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
            .expect("count canonical graph entities");
        assert_eq!(graph_entities, 3_000);
        let checkpoints = checkpoint_repository::list(conn)?;
        assert_eq!(checkpoints.len(), 40);
        assert_eq!(
            checkpoints
                .iter()
                .filter(|row| row.item_count == 3_000)
                .count(),
            2
        );
        assert_eq!(
            checkpoints
                .iter()
                .filter(|row| row.item_count == 100)
                .count(),
            38
        );
        let mut large: Vec<_> = checkpoints
            .iter()
            .filter(|row| row.item_count == 3_000)
            .collect();
        large.sort_by_key(|row| row.id);
        let before = checkpoint_repository::items(conn, large[0].id)?;
        let after = checkpoint_repository::items(conn, large[1].id)?;
        let diff = compare_item_sets(
            &before,
            &large[0].captured_collectors,
            &large[0].missing_collectors,
            &after,
            &large[1].captured_collectors,
            &large[1].missing_collectors,
            Vec::new(),
        );
        assert!(
            !diff.changed.is_empty(),
            "the persisted 3k-item service checkpoints must have a semantic change"
        );
        assert_eq!(investigations::count(conn)?, 20);
        let (bursts, related, first_at): (i64, i64, i64) = conn
            .query_row(
                "SELECT \
                   SUM(json_extract(metadata_json, '$.burst') = 1), \
                   SUM(json_array_length(related_entity_ids) > 0), \
                   MIN(timestamp) \
                 FROM events WHERE source_collector = 'synthetic'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("inspect deterministic corpus distribution");
        assert_eq!(bursts, 800);
        assert_eq!(related, 1_500);
        assert_eq!(first_at, END_AT - 90 * 86_400_000);
        let meaningful_transitions: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events \
                 WHERE json_extract(before_json, '$.state') = 'running' \
                   AND json_extract(after_json, '$.state') = 'stopped'",
                [],
                |row| row.get(0),
            )
            .expect("count persisted state changes");
        assert_eq!(meaningful_transitions, 10_000);
        let last_at: i64 = conn
            .query_row("SELECT MAX(timestamp) FROM events", [], |row| row.get(0))
            .expect("read canonical final instant");
        assert_eq!(last_at, END_AT);
        let category_count: i64 = conn
            .query_row("SELECT COUNT(DISTINCT category) FROM events", [], |row| row.get(0))
            .expect("count canonical category mix");
        assert_eq!(category_count, Category::ALL.len() as i64);
        let undersized_burst_groups: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ( \
                 SELECT category, entity_id, timestamp / 60000 AS minute \
                 FROM events WHERE json_extract(metadata_json, '$.burst') = 1 \
                 GROUP BY category, entity_id, minute HAVING COUNT(*) < 5)",
                [],
                |row| row.get(0),
            )
            .expect("validate real same-minute burst groups");
        assert_eq!(undersized_burst_groups, 0);
        let (stored_edges, event_backed_edges): (i64, i64) = conn
            .query_row(
                "SELECT COUNT(*), COUNT(DISTINCT event_id) FROM entity_relationships",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("measure persisted relationship density");
        eprintln!(
            "10k canonical seed: related event rows={related}, stored edges={stored_edges}, event-backed edges={event_backed_edges}"
        );
        assert_eq!(stored_edges, 1_500, "15% means durable unique graph facts");
        assert_eq!(event_backed_edges, 1_500, "each graph fact needs its own evidence event");
        let distinct_kinds: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT kind) FROM entity_relationships",
                [],
                |row| row.get(0),
            )
            .expect("count supported seeded graph kinds");
        assert_eq!(distinct_kinds, 4);
        let uncovered: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM events e \
                 WHERE e.source_collector = 'synthetic' \
                   AND json_array_length(e.related_entity_ids) > 0 \
                   AND NOT EXISTS (SELECT 1 FROM entity_relationships r WHERE r.event_id = e.id)",
                [],
                |row| row.get(0),
            )
            .expect("count related events lacking persisted graph evidence");
        assert_eq!(uncovered, 0);
        let invalid_edges: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entity_relationships r \
                 LEFT JOIN events e ON e.id = r.event_id \
                 LEFT JOIN entities s ON s.id = r.source_id \
                 LEFT JOIN entities t ON t.id = r.target_id \
                 WHERE e.id IS NULL OR s.id IS NULL OR t.id IS NULL \
                    OR e.source_collector != 'synthetic' \
                    OR r.source_id != e.entity_id OR r.recorded_at != e.timestamp \
                    OR NOT EXISTS (SELECT 1 FROM json_each(e.related_entity_ids) j WHERE j.value = r.target_id) \
                    OR NOT ( \
                      (r.kind = 'listens_on' AND s.entity_type = 'process' AND t.entity_type = 'port') \
                      OR (r.kind = 'publishes' AND s.entity_type = 'container' AND t.entity_type = 'port') \
                      OR (r.kind = 'config_of' AND s.entity_type = 'file' AND t.entity_type = 'service') \
                      OR (r.kind = 'provides' AND s.entity_type = 'app' AND t.entity_type = 'service') \
                    )",
                [],
                |row| row.get(0),
            )
            .expect("validate every seeded graph direction and evidence link");
        assert_eq!(invalid_edges, 0);
        let (entity_id, first_event_at, last_event_at): (String, i64, i64) = conn
            .query_row(
                "SELECT entity_id, MIN(timestamp), MAX(timestamp) \
                 FROM events WHERE source_collector = 'synthetic' \
                 GROUP BY entity_id HAVING COUNT(*) > 1 \
                 ORDER BY COUNT(*) DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("inspect a reused seeded entity");
        let owned = entity_repository::by_id(conn, &entity_id)?.expect("seeded entity exists");
        assert_eq!(owned.first_seen, first_event_at);
        assert_eq!(owned.last_seen, last_event_at);
        let event_entity_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT entity_id) FROM events WHERE source_collector = 'synthetic'",
                [],
                |row| row.get(0),
            )
            .expect("measure actual entity cardinality");
        eprintln!(
            "10k canonical seed: event entity ids={event_entity_count}, graph entities={graph_entities}"
        );
        Ok(())
    })
    .expect("seed distribution is complete");
    let (timestamp, category, entity_id): (i64, String, String) = db
        .read(|conn| {
            let row = conn
                .query_row(
                    "SELECT timestamp, category, entity_id FROM events \
                     WHERE json_extract(metadata_json, '$.burst') = 1 \
                     ORDER BY timestamp DESC, id DESC LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .expect("read a burst for the production timeline path");
            Ok(row)
        })
        .expect("find a marked burst");
    let minute_start = timestamp.div_euclid(60_000) * 60_000;
    let page = query_service::page(
        &db,
        EventQuery {
            range: TimeRange::Custom {
                from: minute_start,
                to: minute_start + 59_999,
            },
            categories: vec![category.parse().expect("valid category")],
            entity_id: Some(entity_id),
            limit: 200,
            ..EventQuery::default()
        },
    )
    .expect("query a marked burst through the production timeline path");
    assert!(
        page.items
            .iter()
            .any(|item| matches!(item, TimelineItem::Cluster { count, .. } if *count >= 5)),
        "the synthetic burst must become an actual timeline cluster"
    );
    db.close();
    drop(db);

    let output = run_seeder(&directory, &["--purge-synthetic"]);
    assert!(output.contains("purged 10000 synthetic events"));
    let db = Db::open_in(directory.path()).expect("inspect purged corpus");
    db.read(|conn| {
        assert_eq!(event_repository::count(conn)?, 0);
        assert!(checkpoint_repository::list(conn)?.is_empty());
        assert_eq!(investigations::count(conn)?, 0);
        Ok(())
    })
    .expect("registered seed artifacts were removed");
    db.close();
}
