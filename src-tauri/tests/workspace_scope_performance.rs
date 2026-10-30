//! Opt-in diagnostic for the production workspace corpus query, not a Phase 20 gate.
//!
//! From src-tauri, set WHAT_CHANGED_PERF_ROWS to exactly 100000 or 1000000:
//! cargo test --release --features devtools --test workspace_scope_performance
//!   workspace_scope_query_at_scale -- --ignored --exact --nocapture --test-threads=1
//! Setup uses the prepared fixture insertion pattern in analysis_perf.rs; its
//! elapsed time says nothing about production collector/seeder throughput.
//! One owned, disposable database serves both workspace sizes. This compact
//! file/process corpus differs from the canonical Phase 20 graph-rich corpus,
//! so neither the 600/1500 ms investigation budgets nor an EXPLAIN claim apply.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::{Duration, Instant};

use serde_json::json;
use sha2::{Digest, Sha256};
use what_changed_lib::domain::{
    ids, Category, Confidence, EntityType, EventType, Importance, NormalizedEvent, Severity,
};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{event_repository, migrations, project_repository, Db};

#[path = "support/perf_fixture.rs"]
mod perf_fixture;

const FROM: i64 = 1_800_000_000_000;
const SPARSE_ROOT: &str = r"D:\work\api";
const DENSE_ROOT: &str = r"D:\work\dense";
const DENSE_ROWS: usize = 5_001;
const WARM_SAMPLES: usize = 10;

fn file_event(path: &str) -> NormalizedEvent {
    NormalizedEvent {
        timestamp: FROM,
        category: Category::Configuration,
        event_type: EventType::ConfigurationChanged,
        entity_type: EntityType::File,
        entity_id: ids::file(path),
        entity_display_name: ".env PORT".into(),
        severity: Severity::Warning,
        importance: Importance::Important,
        source_collector: "files".into(),
        before: Some(json!("3000")),
        after: Some(json!("4000")),
        field: Some("PORT".into()),
        metadata: Some(json!({ "path": path })),
        related_entity_ids: vec![],
        user_friendly_summary: "PORT changed".into(),
        technical_summary: "PORT changed".into(),
        confidence: Confidence::High,
        sensitive: false,
        occurred_at_is_uncertain: false,
    }
}

/// Keep memory bounded: repeat 64 sanitized noise templates, never collect a
/// million NormalizedEvents. All schema indexes and FTS insert triggers remain.
fn seed_fixture(db: &Db, rows: usize) {
    let mut templates = vec![
        file_event(&format!(r"{SPARSE_ROOT}\.env")),
        file_event(&format!(r"{DENSE_ROOT}\.env")),
    ];
    for index in 0..32 {
        let mut file = file_event(&format!(r"D:\noise\{index}\.env"));
        file.importance = Importance::Critical;
        templates.push(file);
        // A process with an in-root text path must still be unrelated noise.
        let path = format!(r"{SPARSE_ROOT}\worker-{index}.exe");
        let mut process = file_event(&path);
        process.category = Category::Process;
        process.event_type = EventType::Started;
        process.entity_type = EntityType::Process;
        process.entity_id = ids::process(&path);
        process.source_collector = "process".into();
        process.importance = Importance::Critical;
        process.before = None;
        process.after = None;
        process.field = None;
        process.entity_display_name = "worker.exe".into();
        process.user_friendly_summary = "Worker started".into();
        process.technical_summary = "Worker started".into();
        templates.push(process);
    }
    let sanitizer = Sanitizer::new();
    let templates = templates
        .into_iter()
        .map(|event| sanitizer.sanitize(event).0)
        .collect::<Vec<_>>();
    db.write(move |conn| {
        let tx = conn.unchecked_transaction().expect("begin fixture setup");
        {
            let mut insert = tx.prepare_cached(
                "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, \
                 entity_display_name, severity, importance, source_collector, before_json, \
                 after_json, metadata_json, related_entity_ids, user_friendly_summary, \
                 technical_summary, confidence, sensitive, occurred_at_is_uncertain, field) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, '[]', ?13, \
                 ?14, ?15, ?16, ?17, ?18)",
            ).expect("prepare fixture insertion");
            for index in 0..rows {
                let template = match index {
                    0..=1 => 0,
                    2..=5_002 => 1,
                    _ => 2 + (index - 5_003) % 64,
                };
                let event = templates[template].get();
                let encode = |value: &Option<serde_json::Value>| {
                    value.as_ref().map(|value| serde_json::to_string(value).unwrap())
                };
                insert.execute(rusqlite::params![
                    FROM + i64::try_from(index).unwrap(),
                    event.category.as_str(), event.event_type.as_str(), event.entity_type.as_str(),
                    event.entity_id, event.entity_display_name, event.severity.as_str(),
                    event.importance.as_str(), event.source_collector, encode(&event.before),
                    encode(&event.after), encode(&event.metadata), event.user_friendly_summary,
                    event.technical_summary, event.confidence.as_str(), event.sensitive,
                    event.occurred_at_is_uncertain, event.field,
                ]).expect("insert fixture row");
            }
        }
        tx.execute_batch(
            "INSERT INTO entities (id, entity_type, display_name, first_seen, last_seen, metadata_json) \
             SELECT entity_id, MIN(entity_type), MIN(entity_display_name), MIN(timestamp), \
                    MAX(timestamp), NULL FROM events GROUP BY entity_id;",
        ).expect("seed fixture entity rows");
        tx.commit().expect("commit fixture setup");
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .expect("persist fixture and truncate setup WAL");
        Ok(())
    }).expect("seed performance fixture");
}

fn print_provenance() {
    for (name, bytes) in [
        (
            "diagnostic",
            include_bytes!("workspace_scope_performance.rs").as_slice(),
        ),
        (
            "event_repository",
            include_bytes!("../src/storage/event_repository.rs").as_slice(),
        ),
        (
            "connection",
            include_bytes!("../src/storage/connection.rs").as_slice(),
        ),
        (
            "fixture",
            include_bytes!("support/perf_fixture.rs").as_slice(),
        ),
        ("cargo_lock", include_bytes!("../Cargo.lock").as_slice()),
    ] {
        println!(
            "workspace diagnostic source={name} sha256={:x}",
            Sha256::digest(bytes)
        );
    }
    let mut schema = Sha256::new();
    for migration in migrations::MIGRATIONS {
        schema.update(migration.version.to_le_bytes());
        schema.update(migration.sql.as_bytes());
    }
    println!(
        "workspace diagnostic migrations={} sha256={:x}",
        migrations::MIGRATIONS.len(),
        schema.finalize()
    );
}

fn measure(db: &Db, project_id: &str, rows: usize, expected_ids: &[i64]) {
    let mut samples = Vec::<Duration>::with_capacity(WARM_SAMPLES);
    for sample in 0..=WARM_SAMPLES {
        // Only the real repository call and reader acquisition are timed.
        // Assertions and printing are outside the measurement.
        let started = Instant::now();
        let result = db
            .read(|conn| {
                event_repository::analysis_window_workspace(
                    conn,
                    FROM,
                    FROM + rows as i64,
                    project_id,
                )
            })
            .expect("run production workspace query");
        let elapsed = started.elapsed();
        let total = if expected_ids.len() == 2 {
            2
        } else {
            DENSE_ROWS as u64
        };
        assert_eq!(result.total, total);
        assert_eq!(result.events.len(), expected_ids.len());
        assert_eq!(result.downsampled, total - expected_ids.len() as u64);
        assert!(result.events.len() <= event_repository::MAX_ANALYSIS_EVENTS);
        assert_eq!(
            result
                .events
                .iter()
                .map(|event| event.id)
                .collect::<Vec<_>>(),
            expected_ids
        );
        assert!(result.events.iter().all(|row| {
            row.event.source_collector == "files"
                && row.event.entity_type == EntityType::File
                && row.event.related_entity_ids.is_empty()
        }));
        println!("workspace diagnostic rows={rows} matching={total} sample={sample} first_call={} elapsed_us={}", sample == 0, elapsed.as_micros());
        if sample > 0 {
            samples.push(elapsed);
        }
    }
    samples.sort_unstable();
    println!("workspace diagnostic rows={rows} retained={} warm_samples={} warm_min_us={} warm_median_us={} warm_max_us={} acceptance=not_applicable", expected_ids.len(), samples.len(), samples[0].as_micros(), samples[WARM_SAMPLES / 2].as_micros(), samples[WARM_SAMPLES - 1].as_micros());
}

#[test]
#[ignore = "explicit 100k/1m persisted workspace query diagnostic; requires WHAT_CHANGED_PERF_ROWS"]
fn workspace_scope_query_at_scale() {
    let rows = match std::env::var("WHAT_CHANGED_PERF_ROWS").as_deref() {
        Ok("100000") => 100_000,
        Ok("1000000") => 1_000_000,
        _ => panic!("set WHAT_CHANGED_PERF_ROWS to exactly 100000 or 1000000"),
    };
    print_provenance();
    let setup = Instant::now();
    let fixture = perf_fixture::PerfFixture::new();
    let db = fixture.db();
    seed_fixture(db, rows);
    let projects = db
        .write(|conn| {
            Ok([
                project_repository::insert(conn, SPARSE_ROOT, "Sparse fixture", FROM)?.id,
                project_repository::insert(conn, DENSE_ROOT, "Dense fixture", FROM)?.id,
            ])
        })
        .expect("register fixture workspaces");
    db.read(|conn| {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0)).unwrap();
        assert_eq!(count, i64::try_from(rows).unwrap());
        let files: i64 = conn.query_row("SELECT COUNT(*) FROM events WHERE source_collector = 'files' AND json_type(metadata_json, '$.path') = 'text'", [], |row| row.get(0)).unwrap();
        assert_eq!(files, i64::try_from(5_003 + (rows - 5_003).div_ceil(2)).unwrap());
        let schema: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(schema, i64::try_from(migrations::MIGRATIONS.len()).unwrap());
        Ok(())
    }).expect("verify persisted fixture counts and current schema");
    println!("workspace diagnostic setup rows={rows} selected_sparse=2 selected_dense={DENSE_ROWS} unrelated={} elapsed_ms={:.3} db_bytes={} canonical_phase20=false production_seeder_measurement=false", rows - 5_003, setup.elapsed().as_secs_f64() * 1000.0, std::fs::metadata(&db.paths().database).unwrap().len());
    measure(db, &projects[0], rows, &[1, 2]);
    // The oldest dense event (id 3) loses the deterministic 5,000-event cap.
    measure(db, &projects[1], rows, &(4..=5_003).collect::<Vec<_>>());
    // PerfFixture closes SQLite handles before deleting only its exclusively
    // created .perf-temp child, including when a correctness assertion fails.
}
