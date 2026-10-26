//! Database-size and retention/vacuum evidence for Phase 20.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::storage::{maintenance, Db};

#[path = "support/perf_fixture.rs"]
mod perf_fixture;
use perf_fixture::PerfFixture;

fn database_budget_bytes(rows: usize) -> u64 {
    // Decimal MB/GB from Phase 20, not MiB/GiB. The metric is logical SQLite
    // database pages, including indexes; WAL bytes are reported separately
    // because the WAL may duplicate pages already included in page_count.
    match rows {
        10_000 => 12_000_000,
        100_000 => 110_000_000,
        1_000_000 => 1_100_000_000,
        _ => panic!("WHAT_CHANGED_PERF_ROWS must be 10000, 100000, or 1000000"),
    }
}

fn page_stats(db: &Db) -> (u64, u64) {
    db.read(|conn| {
        let page_count = conn
            .pragma_query_value(None, "page_count", |row| row.get::<_, i64>(0))
            .map_err(|source| {
                what_changed_lib::error::AppError::Db(what_changed_lib::error::DbError::Sqlite {
                    operation: "read page_count for perf gate",
                    source,
                })
            })?;
        let freelist = conn
            .pragma_query_value(None, "freelist_count", |row| row.get::<_, i64>(0))
            .map_err(|source| {
                what_changed_lib::error::AppError::Db(what_changed_lib::error::DbError::Sqlite {
                    operation: "read freelist_count for perf gate",
                    source,
                })
            })?;
        Ok((
            u64::try_from(page_count).unwrap_or(0),
            u64::try_from(freelist).unwrap_or(0),
        ))
    })
    .expect("read page statistics")
}

#[test]
#[ignore = "Phase 20 database-size gate; run explicitly with --include-ignored"]
fn selected_corpus_reports_size_and_reclaims_after_purge() {
    let rows = std::env::var("WHAT_CHANGED_PERF_ROWS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(10_000);
    let (fixture, corpus) = PerfFixture::new_seeded_db_for_size_gate(rows);
    let db = fixture.db();
    let before_pages = page_stats(db);
    let before = db
        .read(|conn| maintenance::storage_usage(conn, db.paths()))
        .expect("read storage usage");
    let payload_json_bytes: i64 = db
        .read(|conn| {
            Ok(conn
                .query_row(
                    "SELECT COALESCE(SUM(COALESCE(LENGTH(before_json), 0) + \
                 COALESCE(LENGTH(after_json), 0) + COALESCE(LENGTH(metadata_json), 0)), 0) \
                 FROM events",
                    [],
                    |row| row.get(0),
                )
                .expect("sum persisted payload JSON bytes"))
        })
        .expect("read persisted before/after/metadata byte count");
    println!(
        "phase20 rows={rows} payload_profile={} payload_json_bytes={payload_json_bytes} database_bytes={} database_budget_bytes={} wal_bytes={} size_metric=page_count_times_page_size",
        corpus.payload_profile, before.database_bytes, database_budget_bytes(rows), before.wal_bytes
    );
    assert!(
        before.database_bytes <= database_budget_bytes(rows),
        "{rows}-row {} database is {} bytes; decimal-byte budget is {}",
        corpus.payload_profile,
        before.database_bytes,
        database_budget_bytes(rows)
    );
    let events_deleted = maintenance::purge_synthetic(db).expect("purge canonical synthetic run");
    assert_eq!(
        events_deleted, rows,
        "synthetic purge must remove every event"
    );
    let (
        events_left,
        checkpoints_left,
        investigations_left,
        snapshots_left,
        edges_left,
        entities_left,
    ) = db
        .read(|conn| {
            let count = |table: &str| -> i64 {
                conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("count rows after synthetic purge")
            };
            Ok((
                count("events"),
                count("checkpoints"),
                count("investigations"),
                count("snapshots"),
                count("entity_relationships"),
                count("entities"),
            ))
        })
        .expect("read synthetic purge results");
    assert_eq!(
        (
            events_left,
            checkpoints_left,
            investigations_left,
            snapshots_left,
            edges_left
        ),
        (0, 0, 0, 0, 0),
        "synthetic purge left owned events, checkpoints, investigations, snapshots, or edges"
    );
    // Entity rows have no immutable ownership marker. Purge intentionally
    // preserves them rather than risk deleting real evidence after an ID collision.
    println!("phase20 purge_orphan_entity_limitation remaining_entities={entities_left}");
    // Full VACUUM is an explicit compaction action, separate from routine retention.
    maintenance::vacuum_full(db).expect("explicit full compaction");
    let after = db
        .read(|conn| maintenance::storage_usage(conn, db.paths()))
        .expect("read reclaimed usage");
    let after_pages = page_stats(db);
    println!(
        "purge events_deleted={} after_bytes={} page_count={}=>{} freelist_count={}=>{}",
        events_deleted,
        after.database_bytes,
        before_pages.0,
        after_pages.0,
        before_pages.1,
        after_pages.1
    );
    assert_eq!(after_pages.1, 0, "full compaction left free pages");
    assert!(
        after_pages.0 < before_pages.0,
        "synthetic purge and full compaction did not reclaim pages: before={before_pages:?} after={after_pages:?}"
    );
    // This disposable corpus has no surviving event rows after purge. The
    // remaining 3k unowned entity identities must not leave a large FTS5
    // tombstone segment behind after the explicit full compaction.
    assert!(
        after.database_bytes <= database_budget_bytes(10_000),
        "post-purge QA database retained {} bytes without events (possible FTS shadow bloat)",
        after.database_bytes
    );
}

#[test]
fn database_budgets_use_decimal_bytes_at_each_required_scale() {
    assert_eq!(database_budget_bytes(10_000), 12_000_000);
    assert_eq!(database_budget_bytes(100_000), 110_000_000);
    assert_eq!(database_budget_bytes(1_000_000), 1_100_000_000);
}
