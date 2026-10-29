//! Explicit insertion attribution, not a seed-time or query-budget gate.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::{BTreeMap, HashSet};
use std::process::Command;
use std::time::Instant;

use what_changed_lib::domain::Category;
use what_changed_lib::error::{AppError, DbError};
use what_changed_lib::privacy::Sanitizer;

#[path = "support/seed_insertion_fixture.rs"]
mod seed_insertion_fixture;
use seed_insertion_fixture::SeedInsertionFixture;

const END_AT: i64 = 1_800_000_000_000;
const DAYS: i64 = 90;
const SEED: u64 = 0xC0DE_2026;

#[test]
fn captured_writer_bridge_preserves_results_and_recovers_after_rollback() {
    let mut fixture = SeedInsertionFixture::new();
    fixture.open();
    let db = fixture.db();
    let batch = || {
        (0..4)
            .map(|seed| {
                Sanitizer::new()
                    .sanitize(what_changed_lib::testing::synthetic_event(
                        seed,
                        1_700_000_000_000 + seed as i64,
                    ))
                    .0
            })
            .collect::<Vec<_>>()
    };
    let caller_started = Instant::now();
    let captured = db
        .insert_events_preserving_entities_with_timings(batch(), HashSet::new())
        .unwrap();
    assert!(caller_started.elapsed() >= captured.timings.repository_total);
    let (ids, owned) = captured.value.unwrap();
    assert_eq!(ids.len(), 4);
    assert_eq!(captured.timings.committed_rows, 4);
    assert_eq!(captured.timings.completed_phases, 4);
    let old = db
        .insert_events_preserving_entities(batch(), owned.clone())
        .unwrap();
    assert_eq!(old.1, owned);
    db.write(|conn| {
        conn.execute_batch("CREATE TRIGGER fail_captured_insert BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'fixture rollback'); END;")
            .map_err(|source| AppError::Db(DbError::Sqlite { operation: "install insertion diagnostic rollback fixture", source }))
    }).unwrap();
    let failed = db
        .insert_events_preserving_entities_with_timings(batch(), owned.clone())
        .unwrap();
    assert!(failed.value.is_err());
    assert_eq!(failed.timings.failed_batches, 1);
    assert_eq!(failed.timings.committed_rows, 0);
    db.write(|conn| {
        conn.execute_batch("DROP TRIGGER fail_captured_insert")
            .map_err(|source| {
                AppError::Db(DbError::Sqlite {
                    operation: "remove insertion diagnostic rollback fixture",
                    source,
                })
            })
    })
    .unwrap();
    let next = db
        .insert_events_preserving_entities_with_timings(batch(), owned)
        .unwrap();
    assert_eq!(next.value.unwrap().0.len(), 4);
    assert_eq!(next.timings.batches, 1);
    assert_eq!(next.timings.failed_batches, 0);
    assert_eq!(next.timings.committed_rows, 4);
    db.read(|conn| {
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM events", [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            12
        );
        Ok(())
    })
    .unwrap();
}

#[test]
#[ignore = "Explicit diagnostic: one fresh canonical corpus with insertion attribution; no acceptance budget"]
fn canonical_seed_reports_aggregate_insertion_attribution() {
    let rows = match std::env::var("WHAT_CHANGED_PERF_ROWS") {
        Err(std::env::VarError::NotPresent) => 10_000,
        Ok(value) => value.parse::<usize>().unwrap(),
        Err(error) => panic!("invalid diagnostic row selection: {error}"),
    };
    assert!(
        matches!(rows, 10_000 | 1_000_000),
        "only the approved smoke and attribution scales"
    );
    let mut fixture = SeedInsertionFixture::new();
    let started = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_seed_history"))
        .arg("--events")
        .arg(rows.to_string())
        .arg("--days")
        .arg(DAYS.to_string())
        .arg("--seed")
        .arg(SEED.to_string())
        .arg("--relations")
        .arg("--timings")
        .arg("--insertion-timings")
        .arg("--end-at")
        .arg(END_AT.to_string())
        .arg("--data-dir")
        .arg(fixture.root())
        .output()
        .expect("run seed_history on exclusive diagnostic directory");
    let elapsed = started.elapsed();
    // Forward aggregate records only; never child paths or individual rows.
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().filter(|line| {
        line.starts_with("seed timings: ") || line.starts_with("seed insertion timings: ")
    }) {
        println!("{line}");
    }
    assert!(
        output.status.success(),
        "diagnostic seeder exited with {}",
        output.status
    );
    let reports = stdout
        .lines()
        .filter_map(|line| line.strip_prefix("seed insertion timings: "))
        .collect::<Vec<_>>();
    assert_eq!(reports.len(), 1, "one aggregate attribution record");
    let report = reports[0]
        .split_whitespace()
        .map(|field| field.split_once('=').unwrap())
        .collect::<BTreeMap<_, _>>();
    let integer = |name: &str| report[name].parse::<usize>().unwrap();
    let duration = |name: &str| report[name].parse::<f64>().unwrap();
    assert_eq!(integer("batches"), rows.div_ceil(1_000));
    assert_eq!(integer("committed_batches"), integer("batches"));
    assert_eq!(integer("attempted_rows"), rows);
    assert_eq!(integer("committed_rows"), rows);
    assert_eq!(integer("failed_batches"), 0);
    assert_eq!(integer("empty_batches"), 0);
    assert_eq!(integer("completed_phases"), integer("batches") * 4);
    for name in [
        "begin_prepare_ms",
        "event_entity_loop_ms",
        "reconciliation_ms",
        "commit_ms",
        "repository_total_ms",
        "caller_insert_ms",
        "caller_writer_overhead_ms",
    ] {
        assert!(duration(name).is_finite() && duration(name) >= 0.0);
    }
    assert!(duration("caller_insert_ms") >= duration("repository_total_ms"));
    // Three-decimal report rounding can change a sum by a few thousandths.
    assert!(
        duration("repository_total_ms") + 0.004
            >= [
                "begin_prepare_ms",
                "event_entity_loop_ms",
                "reconciliation_ms",
                "commit_ms"
            ]
            .into_iter()
            .map(duration)
            .sum::<f64>()
    );
    fixture.open();
    verify_diagnostic_corpus(&fixture, rows);
    println!("DIAGNOSTIC seed_insertion_profile rows={rows} elapsed_ms={:.3} status=attributed_not_acceptance",
        elapsed.as_secs_f64() * 1000.0);
}

fn verify_diagnostic_corpus(fixture: &SeedInsertionFixture, rows: usize) {
    fixture.db().read(|conn| {
        let count = |sql| conn.query_row(sql, [], |row| row.get::<_, i64>(0)).unwrap();
        let expected = i64::try_from(rows).unwrap();
        assert_eq!(count("SELECT COUNT(*) FROM events"), expected);
        assert_eq!(count("SELECT COUNT(*) FROM events WHERE source_collector='synthetic'"), expected);
        assert_eq!(count("SELECT COUNT(*) FROM entities"), 3_000);
        assert_eq!(count("SELECT COUNT(*) FROM events WHERE json_array_length(related_entity_ids)>0"), expected * 15 / 100);
        assert_eq!(count("SELECT COUNT(*) FROM entity_relationships WHERE event_id IN (SELECT id FROM events WHERE source_collector='synthetic')"), expected * 15 / 100);
        assert_eq!(count("SELECT COUNT(*) FROM events WHERE json_extract(metadata_json, '$.burst')=1"), expected * 8 / 100);
        assert_eq!(count("SELECT COUNT(DISTINCT category) FROM events"), Category::ALL.len() as i64);
        assert_eq!(count("SELECT COUNT(*) FROM checkpoints"), 40);
        assert_eq!(count("SELECT COUNT(*) FROM snapshots"), 9_800);
        assert_eq!(count("SELECT COUNT(*) FROM investigations"), 20);
        let (from, to): (i64, i64) = conn.query_row("SELECT MIN(timestamp), MAX(timestamp) FROM events", [],
            |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!((from, to), (END_AT - DAYS * 86_400_000, END_AT));
        Ok(())
    }).unwrap();
    // Counts/span validate this diagnostic fixture, not every graph invariant,
    // scoring contract or any of the eight query performance budgets.
    println!("DIAGNOSTIC seed_insertion_corpus rows={rows} entities=3000 relation_percent=15 burst_percent=8 checkpoints=40 snapshots=9800 investigations=20 span_days=90");
}
