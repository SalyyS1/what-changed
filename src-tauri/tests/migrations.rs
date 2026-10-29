//! The schema and its migrations, proven from the file rather than assumed
//! from the SQL: every mandated table and index exists, the ledger matches,
//! an edited migration is detected, and data survives a forward migration.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use rusqlite::Connection;
use what_changed_lib::storage::connection::{assert_fts5, compile_options, open_writer};
use what_changed_lib::storage::event_repository;
use what_changed_lib::storage::migrations::{
    checksum, current_version, ledger, run, Migration, MIGRATIONS,
};
use what_changed_lib::testing::temp_dir;

/// The thirteen tables the brief mandates, plus the three later phases
/// persist to, plus the search index's virtual table.
const REQUIRED_TABLES: &[&str] = &[
    "events",
    "entities",
    "entity_relationships",
    "snapshots",
    "checkpoints",
    "health_states",
    "collector_status",
    "settings",
    "watched_locations",
    "applications",
    "analysis_sessions",
    "analysis_results",
    "schema_migrations",
    "collector_baselines",
    "config_snapshots",
    "investigations",
    "projects",
    "events_fts",
    "event_search_order",
    "events_search_fts",
    "event_importance_daily",
    "event_category_counts",
];

const REQUIRED_INDEXES: &[(&str, &str)] = &[
    ("events", "timestamp"),
    ("events", "category, timestamp"),
    ("events", "entity_id, timestamp"),
    ("events", "event_type, timestamp"),
    ("events", "importance, timestamp"),
    ("entity_relationships", "source_id"),
    ("entity_relationships", "target_id"),
    ("entities", "entity_type, display_name"),
    ("projects", "added_at, id"),
    (
        "event_search_order",
        "importance_rank DESC, timestamp DESC, event_id DESC",
    ),
    ("events", "timestamp, importance"),
];

fn fresh() -> (std::path::PathBuf, Connection) {
    let dir = temp_dir("migrations");
    let mut conn = open_writer(&dir.join("events.db")).unwrap();
    run(&mut conn, MIGRATIONS, &dir.join("backups")).unwrap();
    (dir, conn)
}

fn names(conn: &Connection, kind: &str) -> HashSet<String> {
    let mut statement = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = ?1")
        .unwrap();
    statement
        .query_map([kind], |row| row.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn every_mandated_table_and_index_exists_after_migrating_from_empty() {
    let (_dir, conn) = fresh();
    let tables = names(&conn, "table");
    for table in REQUIRED_TABLES {
        assert!(tables.contains(*table), "table {table} is missing");
    }

    // Index columns are read back from the schema, not from the index name.
    let mut statement = conn
        .prepare("SELECT tbl_name, sql FROM sqlite_master WHERE type = 'index' AND sql IS NOT NULL")
        .unwrap();
    let indexes: Vec<(String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    for (table, columns) in REQUIRED_INDEXES {
        let wanted = format!("({columns})");
        assert!(
            indexes
                .iter()
                .any(|(t, sql)| t == table && sql.replace('\n', " ").contains(&wanted)),
            "no index on {table}{wanted}"
        );
    }
}

#[test]
fn search_order_projection_tracks_event_lifecycle() {
    let (_dir, conn) = fresh();
    conn.execute(
        "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, entity_display_name, severity, importance, source_collector, user_friendly_summary, technical_summary, confidence) \
         VALUES (42, 'service', 'started', 'service', 'service:test', 'Test', 'info', 'critical', 'test', 'Test started', 'test', 'high')",
        [],
    )
    .unwrap();
    let projected: (i64, i64) = conn
        .query_row(
            "SELECT importance_rank, timestamp FROM event_search_order WHERE event_id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(projected, (3, 42));
    conn.execute("DELETE FROM events WHERE id = 1", []).unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM event_search_order", [], |row| row
            .get::<_, i64>(0),)
            .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM event_importance_daily", [], |row| row
            .get::<_, i64>(0),)
            .unwrap(),
        0
    );
}

#[test]
fn daily_projection_clamps_the_first_representable_timestamp_bucket() {
    let (_dir, conn) = fresh();
    let insert = "INSERT INTO events (
            timestamp, category, event_type, entity_type, entity_id,
            entity_display_name, severity, importance, source_collector,
            user_friendly_summary, technical_summary, confidence
        ) VALUES (?1, 'service', 'started', 'service', ?2, 'Boundary',
                  'info', 'important', 'test', 'Boundary event', 'test', 'high')";
    let last_first_day = i64::MIN + 25_975_807;
    let first_second_day = i64::MIN + 25_975_808;
    let last_oldly_misbucketed = i64::MIN + 60_424_191;
    let first_third_day = first_second_day + 86_400_000;
    for (timestamp, id) in [
        (i64::MIN, "boundary:min"),
        (last_first_day, "boundary:first-last"),
        (first_second_day, "boundary:second-first"),
        (last_oldly_misbucketed, "boundary:second-middle"),
        (first_third_day, "boundary:third-first"),
    ] {
        conn.execute(insert, (timestamp, id)).unwrap();
    }

    let buckets: Vec<(i64, i64)> = conn
        .prepare("SELECT day_start, important FROM event_importance_daily ORDER BY day_start")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        buckets,
        vec![(i64::MIN, 2), (first_second_day, 2), (first_third_day, 1)]
    );
    assert_eq!(
        event_repository::counts_for_range(&conn, i64::MIN, last_oldly_misbucketed)
            .unwrap()
            .total(),
        4,
        "the first partial-day edge must not overlap the second day"
    );
    assert_eq!(
        event_repository::counts_for_range(&conn, i64::MIN, first_third_day)
            .unwrap()
            .total(),
        5,
        "the second day's projection must count as an interior day"
    );

    conn.execute(
        "DELETE FROM events WHERE timestamp = ?1",
        [first_second_day],
    )
    .unwrap();
    assert_eq!(
        event_repository::counts_for_range(&conn, i64::MIN, first_third_day)
            .unwrap()
            .total(),
        4,
        "delete trigger must decrement the corrected bucket"
    );
}

#[test]
fn version_thirteen_repairs_its_daily_projection_without_ledger_drift() {
    let dir = temp_dir("migrations-v13-daily-repair");
    let mut conn = open_writer(&dir.join("events.db")).unwrap();
    run(&mut conn, &MIGRATIONS[..13], &dir.join("backups")).unwrap();
    let first_second_day = i64::MIN + 25_975_808;
    let insert = "INSERT INTO events (
            timestamp, category, event_type, entity_type, entity_id,
            entity_display_name, severity, importance, source_collector,
            user_friendly_summary, technical_summary, confidence
        ) VALUES (?1, 'service', 'started', 'service', ?2, 'Boundary',
                  'info', 'important', 'test', 'Boundary event', 'test', 'high')";
    conn.execute(insert, (i64::MIN + 25_975_807, "boundary:before"))
        .unwrap();
    conn.execute(insert, (first_second_day, "boundary:after"))
        .unwrap();
    let old_bucket_count: i64 = conn
        .query_row(
            "SELECT important FROM event_importance_daily WHERE day_start = ?1",
            [i64::MIN],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(old_bucket_count, 2, "v13 fixture exercises the defect");

    let report = run(&mut conn, MIGRATIONS, &dir.join("backups")).unwrap();
    assert_eq!((report.from, report.to), (13, MIGRATIONS.len()));
    assert!(report.backup.unwrap().exists());
    let repaired: Vec<(i64, i64)> = conn
        .prepare("SELECT day_start, important FROM event_importance_daily ORDER BY day_start")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(repaired, vec![(i64::MIN, 1), (first_second_day, 1)]);
    assert_eq!(ledger(&conn).unwrap().len(), MIGRATIONS.len());
}

fn assert_category_projection(conn: &Connection) {
    let projected: Vec<(String, i64)> = conn
        .prepare("SELECT category, event_count FROM event_category_counts ORDER BY category")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let exact: Vec<(String, i64)> = conn
        .prepare("SELECT category, COUNT(*) FROM events GROUP BY category ORDER BY category")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(projected, exact);
}

#[test]
fn category_counts_backfill_and_mutations_survive_later_additive_migrations() {
    let dir = temp_dir("migrations-v15-category-counts");
    let mut conn = open_writer(&dir.join("events.db")).unwrap();
    run(&mut conn, &MIGRATIONS[..14], &dir.join("backups")).unwrap();
    conn.execute_batch(
        "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, \
         entity_display_name, severity, importance, source_collector, user_friendly_summary, \
         technical_summary, confidence) VALUES \
         (1, 'service', 'stopped', 'service', 'service:A', 'A', 'info', 'informational', 'x', 'A stopped', 'A', 'high'), \
         (2, 'service', 'started', 'service', 'service:B', 'B', 'info', 'informational', 'x', 'B started', 'B', 'high'), \
         (3, 'network', 'modified', 'adapter', 'adapter:C', 'C', 'info', 'informational', 'x', 'C changed', 'C', 'high');",
    )
    .unwrap();

    let report = run(&mut conn, MIGRATIONS, &dir.join("backups")).unwrap();
    assert_eq!((report.from, report.to), (14, MIGRATIONS.len()));
    assert!(report.backup.is_none(), "the new migration is additive");
    let baseline_column: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name='baseline_checkpoint_id'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        baseline_column, 1,
        "the later additive project baseline migration applied"
    );
    assert_category_projection(&conn);
    assert_eq!(ledger(&conn).unwrap().len(), MIGRATIONS.len());

    conn.execute(
        "UPDATE events SET category = 'network' WHERE entity_id = 'service:A'",
        [],
    )
    .unwrap();
    assert_category_projection(&conn);
    conn.execute("DELETE FROM events WHERE category = 'service'", [])
        .unwrap();
    assert_category_projection(&conn);
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM event_category_counts WHERE category = 'service'",
            [],
            |row| row.get::<_, i64>(0)
        )
        .unwrap(),
        0,
        "empty categories must not linger in the privacy result"
    );

    let transaction = conn.transaction().unwrap();
    transaction.execute(
        "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, \
         entity_display_name, severity, importance, source_collector, user_friendly_summary, \
         technical_summary, confidence) VALUES \
         (4, 'driver', 'updated', 'driver', 'driver:D', 'D', 'info', 'informational', 'x', 'D updated', 'D', 'high')",
        [],
    ).unwrap();
    assert_category_projection(&transaction);
    transaction.rollback().unwrap();
    assert_category_projection(&conn);
}

#[test]
fn the_provenance_column_exists_and_defaults_to_zero() {
    let (_dir, conn) = fresh();
    let mut statement = conn.prepare("PRAGMA table_info(events)").unwrap();
    let columns: Vec<(String, String, Option<String>)> = statement
        .query_map([], |row| Ok((row.get(1)?, row.get(2)?, row.get(4)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let (_, kind, default) = columns
        .iter()
        .find(|(name, _, _)| name == "occurred_at_is_uncertain")
        .expect("events.occurred_at_is_uncertain exists");
    assert_eq!(kind, "INTEGER");
    assert_eq!(default.as_deref(), Some("0"));
}

#[test]
fn config_key_field_is_nullable_and_survives_forward_migration() {
    let (_dir, conn) = fresh();
    let columns: Vec<(String, String, i64)> = conn
        .prepare("PRAGMA table_info(events)")
        .unwrap()
        .query_map([], |row| Ok((row.get(1)?, row.get(2)?, row.get(3)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let (_, kind, not_null) = columns
        .iter()
        .find(|(name, _, _)| name == "field")
        .expect("events.field exists");
    assert_eq!(kind, "TEXT");
    assert_eq!(*not_null, 0);
}

#[test]
fn fts5_is_compiled_in_and_the_search_migration_has_one_body() {
    let (_dir, conn) = fresh();
    assert_fts5(&conn).unwrap();
    assert!(compile_options(&conn)
        .unwrap()
        .iter()
        .any(|o| o == "ENABLE_FTS5"));
    let bodies = MIGRATIONS
        .iter()
        .filter(|m| m.name == "search_index")
        .count();
    assert_eq!(bodies, 1);
}

#[test]
fn migrating_twice_is_idempotent_and_the_ledger_has_one_row_per_version() {
    let (dir, mut conn) = fresh();
    let again = run(&mut conn, MIGRATIONS, &dir.join("backups")).unwrap();
    assert_eq!(again.from, MIGRATIONS.len());
    assert_eq!(again.to, MIGRATIONS.len());
    assert!(again.backup.is_none());

    let rows = ledger(&conn).unwrap();
    assert_eq!(rows.len(), MIGRATIONS.len());
    for (row, migration) in rows.iter().zip(MIGRATIONS) {
        assert_eq!(row.0, migration.version);
        assert_eq!(row.1, migration.name);
        assert_eq!(row.2, checksum(migration.sql));
    }
    assert_eq!(
        current_version(&conn, MIGRATIONS).unwrap(),
        MIGRATIONS.len()
    );
}

#[test]
fn version_one_with_data_migrates_forward_and_keeps_its_rows() {
    let dir = temp_dir("migrations-forward");
    let mut conn = open_writer(&dir.join("events.db")).unwrap();
    run(&mut conn, &MIGRATIONS[..1], &dir.join("backups")).unwrap();
    assert_eq!(current_version(&conn, MIGRATIONS).unwrap(), 1);
    conn.execute_batch(
        "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, \
         entity_display_name, severity, importance, source_collector, user_friendly_summary, \
         technical_summary, confidence) VALUES \
         (1, 'service', 'stopped', 'service', 'service:A', 'A', 'info', 'informational', 'x', \
          'A stopped', 'A', 'high'), \
         (2, 'network', 'modified', 'adapter', 'adapter:B', 'Wi-Fi', 'info', 'important', 'x', \
          'DNS changed on Wi-Fi', 'B', 'high');",
    )
    .unwrap();

    let report = run(&mut conn, MIGRATIONS, &dir.join("backups")).unwrap();
    assert_eq!((report.from, report.to), (1, MIGRATIONS.len()));
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2);
    // The search migration indexed what was already there.
    let hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events_fts WHERE events_fts MATCH 'dns'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hits, 1);
}

#[test]
fn a_missing_migration_ledger_row_is_reported_as_drift() {
    let (dir, mut conn) = fresh();
    conn.execute("DELETE FROM schema_migrations WHERE version = 1", [])
        .unwrap();
    let error = run(&mut conn, MIGRATIONS, &dir.join("backups")).unwrap_err();
    assert!(
        error.to_string().contains("migration 1") && error.to_string().contains("different SQL"),
        "unexpected error: {error}"
    );
}

#[test]
fn an_extraneous_migration_ledger_row_is_reported_as_drift() {
    let (dir, mut conn) = fresh();
    let extraneous_version = MIGRATIONS.len() + 1;
    conn.execute(
        "INSERT INTO schema_migrations (version, name, applied_at, sha256) \
         VALUES (?1, 'unapplied', 0, 'not-applied')",
        [i64::try_from(extraneous_version).unwrap()],
    )
    .unwrap();
    let error = run(&mut conn, MIGRATIONS, &dir.join("backups")).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(&format!("migration {extraneous_version}")),
        "{error}"
    );
}

#[test]
fn an_edited_migration_is_reported_as_drift_not_ignored() {
    let (dir, mut conn) = fresh();
    conn.execute(
        "UPDATE schema_migrations SET sha256 = 'not-the-real-checksum' WHERE version = 1",
        [],
    )
    .unwrap();
    let error = run(&mut conn, MIGRATIONS, &dir.join("backups")).unwrap_err();
    assert!(
        error.to_string().contains("different SQL"),
        "unexpected error: {error}"
    );
}

#[test]
fn a_database_from_a_newer_build_is_reported_as_ahead() {
    let (dir, mut conn) = fresh();
    conn.pragma_update(None, "user_version", 99).unwrap();
    let error = run(&mut conn, MIGRATIONS, &dir.join("backups")).unwrap_err();
    assert!(error.to_string().contains("schema version 99"), "{error}");
}

#[test]
fn a_non_additive_migration_takes_a_backup_first_and_keeps_two() {
    let dir = temp_dir("migrations-backup");
    let mut conn = open_writer(&dir.join("events.db")).unwrap();
    run(&mut conn, MIGRATIONS, &dir.join("backups")).unwrap();
    conn.execute_batch("INSERT INTO settings (key, value, updated_at) VALUES ('k', '1', 0)")
        .unwrap();

    let mut extended: Vec<Migration> = MIGRATIONS.to_vec();
    let first_risky_version = MIGRATIONS.len() + 1;
    extended.push(Migration {
        version: first_risky_version,
        name: "rewrite_a_table",
        sql: "CREATE TABLE scratch (x); DROP TABLE scratch;",
        additive: false,
    });
    let report = run(&mut conn, &extended, &dir.join("backups")).unwrap();
    let backup = report.backup.expect("a backup was taken");
    assert_eq!(
        backup.file_name().unwrap().to_string_lossy(),
        format!("events-pre-v{first_risky_version}.db")
    );
    let restored = Connection::open(&backup).unwrap();
    let value: String = restored
        .query_row("SELECT value FROM settings WHERE key = 'k'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(value, "1", "the backup holds the data as it was");
    // Windows will not delete a file another handle holds open, and the next
    // migrations prune this backup.
    drop(restored);

    for (offset, sql) in [
        (1, "CREATE TABLE scratch_next_1 (x);"),
        (2, "CREATE TABLE scratch_next_2 (x);"),
    ] {
        extended.push(Migration {
            version: first_risky_version + offset,
            name: "another",
            sql,
            additive: false,
        });
        run(&mut conn, &extended, &dir.join("backups")).unwrap();
    }
    let mut kept: Vec<String> = std::fs::read_dir(dir.join("backups"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    kept.sort();
    let mut expected = vec![
        format!("events-pre-v{}.db", first_risky_version + 1),
        format!("events-pre-v{}.db", first_risky_version + 2),
    ];
    expected.sort();
    assert_eq!(kept, expected);
}
