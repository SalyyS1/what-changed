//! Retention and housekeeping.
//!
//! Runs at startup and every six hours. Deletes in small batches with a
//! yield between them, each batch its own write job, so event inserts
//! interleave with a purge instead of queueing behind it. Returns disk with
//! an incremental vacuum after a pass; a full `VACUUM` is an explicit action so
//! routine six-hour housekeeping never monopolizes the serial writer.

use std::path::Path;
use std::sync::mpsc::{sync_channel, RecvTimeoutError, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::db::{Db, DbPaths};
#[cfg(any(test, debug_assertions, feature = "devtools"))]
use super::search_index::{Fts5Index, SearchIndex};
use super::{
    baseline_repository, config_snapshot_repository, count_column, entity_repository,
    event_repository, investigations, settings_repository, snapshot_repository, SqlContext,
};
use crate::domain::retention::{MAX_DATABASE_BYTES, SIZE_CAP_TARGET_PERCENT};
use crate::domain::{now_ms, RetentionPolicy, DAY_MS};
use crate::error::AppResult;

/// Rows deleted per batch.
pub const BATCH_ROWS: usize = 500;

/// Pause between batches, so the writer serves other work.
const BATCH_YIELD: Duration = Duration::from_millis(25);

/// Free pages reclaimed by one routine pass before yielding the writer.
const RECLAIM_PAGES_PER_PASS: usize = 256;

/// How often a pass runs after the one at startup.
const INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// What one pass did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct MaintenanceReport {
    /// Events deleted.
    pub events_deleted: usize,
    /// Snapshots deleted.
    pub snapshots_deleted: usize,
    /// Investigation histories deleted under their fixed 90-day policy.
    pub investigations_deleted: usize,
    /// Delete statements run.
    pub batches: usize,
    /// Wall time of the pass, milliseconds.
    pub duration_ms: u64,
}

/// What the database costs on disk.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct StorageUsage {
    /// `page_count * page_size` of the main file.
    pub database_bytes: u64,
    /// Size of the write-ahead log.
    pub wal_bytes: u64,
    /// Separate owner SQLite files and owned migration copies; None when the
    /// IPC layer has not measured them or their inventory cannot be inspected.
    pub owner_workspace_bytes: Option<u64>,
    /// Rows per table, by name.
    pub table_rows: Vec<(String, u64)>,
}

/// Aggregate event types for privacy-safe diagnostics. The query stays in the
/// storage boundary so command modules never depend on rusqlite or SQL text.
pub fn event_type_counts(conn: &Connection) -> AppResult<Vec<(String, u64)>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT event_type, COUNT(*) FROM events GROUP BY event_type ORDER BY event_type",
        )
        .during("prepare diagnostics event type counts")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                u64::try_from(row.get::<_, i64>(1)?).unwrap_or(0),
            ))
        })
        .during("read diagnostics event type counts")?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .during("read diagnostics event type counts")
}

/// Privacy-screen aggregates without exposing event content to the command
/// layer. Keeping these reads here preserves the storage/IPC boundary.
#[derive(Debug, Default)]
pub struct PrivacyCounts {
    pub categories: Vec<(String, u64)>,
    pub checkpoints: u64,
    pub checkpoint_items: u64,
    pub investigations: u64,
    pub watched_folders: u64,
    pub config_baselines: u64,
    pub oldest_record_at: Option<i64>,
}

pub fn privacy_counts(conn: &Connection) -> AppResult<PrivacyCounts> {
    let mut statement = conn
        .prepare_cached("SELECT category, event_count FROM event_category_counts ORDER BY category")
        .during("prepare privacy category counts")?;
    let categories = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                u64::try_from(row.get::<_, i64>(1)?).unwrap_or(0),
            ))
        })
        .during("read privacy category counts")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .during("read privacy category counts")?;
    let count = |table: &str| -> AppResult<u64> {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        Ok(u64::try_from(
            conn.query_row(&sql, [], |row| row.get::<_, i64>(0))
                .during("read privacy table count")?,
        )
        .unwrap_or(0))
    };
    let checkpoint_items = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshots WHERE checkpoint_id IS NOT NULL",
            [],
            |row| row.get::<_, i64>(0),
        )
        .during("read checkpoint item count")
        .map(|value| u64::try_from(value).unwrap_or(0))?;
    let oldest_record_at = conn
        .query_row("SELECT MIN(timestamp) FROM events", [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .during("read oldest event")?;
    Ok(PrivacyCounts {
        categories,
        checkpoints: count("checkpoints")?,
        checkpoint_items,
        investigations: count("investigations")?,
        watched_folders: count("watched_locations")?,
        config_baselines: count("config_snapshots")?,
        oldest_record_at,
    })
}

/// Run one retention pass against `db` under `policy`, as of `now`.
pub fn run_pass(db: &Db, policy: RetentionPolicy, now: i64) -> AppResult<MaintenanceReport> {
    run_pass_with_cap(db, policy, now, MAX_DATABASE_BYTES)
}

/// Bytes held by live pages: the file less the pages already on the freelist,
/// which new rows reuse before the file grows again.
fn live_database_bytes(conn: &Connection) -> AppResult<u64> {
    let pragma = |name: &str| -> AppResult<u64> {
        conn.pragma_query_value(None, name, |row| count_column(row, 0))
            .during("read a page statistic")
    };
    let live_pages = pragma("page_count")?.saturating_sub(pragma("freelist_count")?);
    Ok(live_pages.saturating_mul(pragma("page_size")?))
}

/// [`run_pass`] with an explicit size cap, so the cap is testable without a
/// gigabyte of fixtures.
pub fn run_pass_with_cap(
    db: &Db,
    policy: RetentionPolicy,
    now: i64,
    max_database_bytes: u64,
) -> AppResult<MaintenanceReport> {
    let started = std::time::Instant::now();
    let mut report = MaintenanceReport::default();

    let routine = policy.events_cutoff(now);
    let important = policy.important_cutoff(now);
    loop {
        let deleted = db.write(move |conn| {
            event_repository::delete_expired(conn, routine, important, BATCH_ROWS)
        })?;
        report.events_deleted += deleted;
        report.batches += 1;
        if deleted < BATCH_ROWS {
            break;
        }
        std::thread::sleep(BATCH_YIELD);
    }

    let process_noise = policy.process_noise_cutoff(now);
    loop {
        let deleted = db.write(move |conn| {
            event_repository::delete_expired_process_noise(conn, process_noise, BATCH_ROWS)
        })?;
        report.events_deleted += deleted;
        report.batches += 1;
        if deleted < BATCH_ROWS {
            break;
        }
        std::thread::sleep(BATCH_YIELD);
    }

    let snapshots = policy.snapshots_cutoff(now);
    loop {
        let deleted =
            db.write(move |conn| snapshot_repository::delete_expired(conn, snapshots, BATCH_ROWS))?;
        report.snapshots_deleted += deleted;
        report.batches += 1;
        if deleted < BATCH_ROWS {
            break;
        }
        std::thread::sleep(BATCH_YIELD);
    }

    let investigations_cutoff =
        now.saturating_sub(investigations::RETENTION_DAYS.saturating_mul(DAY_MS));
    loop {
        let deleted = db.write(move |conn| {
            investigations::delete_expired(conn, investigations_cutoff, BATCH_ROWS)
        })?;
        report.investigations_deleted += deleted;
        report.batches += 1;
        if deleted < BATCH_ROWS {
            break;
        }
        std::thread::sleep(BATCH_YIELD);
    }

    // Size cap, last: ordinary expiry may already have brought the database
    // under it. Only informational rows are eligible, oldest first.
    //
    // The number of rows to remove is fixed up front from the overage and the
    // average bytes each event costs, rather than re-measured after each batch:
    // SQLite returns a page only once it is wholly empty and a full-text index
    // keeps tombstones until it merges, so live bytes can lag a delete and a
    // measured loop would run on and remove every informational row. The
    // average includes every table's overhead, so it under-deletes; the next
    // pass measures again and continues.
    let live = db.read(live_database_bytes)?;
    if live > max_database_bytes {
        let target = max_database_bytes / 100 * SIZE_CAP_TARGET_PERCENT;
        let events = db.read(event_repository::count)?.max(1);
        let bytes_per_event = (live / events).max(1);
        let mut remaining = (live - target).div_ceil(bytes_per_event);
        while remaining > 0 {
            let batch = usize::try_from(remaining)
                .unwrap_or(BATCH_ROWS)
                .min(BATCH_ROWS);
            let deleted =
                db.write(move |conn| event_repository::delete_oldest_informational(conn, batch))?;
            report.events_deleted += deleted;
            report.batches += 1;
            if deleted == 0 {
                tracing::warn!(
                    live_bytes = live,
                    cap_bytes = max_database_bytes,
                    "the database is over its size cap and has no informational events left to remove"
                );
                break;
            }
            remaining = remaining.saturating_sub(u64::try_from(deleted).unwrap_or(u64::MAX));
            std::thread::sleep(BATCH_YIELD);
        }
    }

    db.write(reclaim)?;
    report.duration_ms = started.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
    tracing::debug!(
        events = report.events_deleted,
        snapshots = report.snapshots_deleted,
        investigations = report.investigations_deleted,
        ms = report.duration_ms,
        "maintenance pass finished"
    );
    Ok(report)
}

/// Return a bounded number of tail pages and fold the log back into the main
/// file. Full compaction remains an explicit user/perf-gate action.
fn reclaim(conn: &Connection) -> AppResult<()> {
    // SQLite emits one zero-column row per page reclaimed. rusqlite's
    // execute_batch steps a statement only once, so explicitly drain all
    // rows to honor the page cap without monopolizing the serial writer.
    let sql = format!("PRAGMA incremental_vacuum({RECLAIM_PAGES_PER_PASS})");
    {
        let mut statement = conn.prepare(&sql).during("prepare incremental vacuum")?;
        let mut rows = statement.query([]).during("start incremental vacuum")?;
        while rows.next().during("incremental vacuum")?.is_some() {}
    }
    // `wal_checkpoint` returns a row (busy, log, checkpointed); a query is the
    // right shape even though the values are not needed.
    let _: (i64, i64, i64) = conn
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .during("checkpoint the write-ahead log")?;
    Ok(())
}

/// A full `VACUUM`. Explicit user action only; rewrites the whole file.
pub fn vacuum_full(db: &Db) -> AppResult<()> {
    db.write(|conn| conn.execute_batch("VACUUM").during("vacuum"))
}

/// The manifest records only unguessable markers issued by the devtools
/// seeder. A user row containing the word "synthetic" is never enough to
/// qualify it for deletion.
#[cfg(any(test, debug_assertions, feature = "devtools"))]
const SYNTHETIC_RUN_MARKERS_KEY: &str = "devtools_seed_history_markers_v1";

/// Register provenance before seeding any checkpoint or investigation. A
/// failed run therefore leaves a recoverable marker even if it crashes after
/// writing one artifact but before finishing the corpus.
#[cfg(any(test, debug_assertions, feature = "devtools"))]
pub fn register_synthetic_run(db: &Db) -> AppResult<String> {
    let marker = format!(
        "Synthetic service failed seed {}",
        uuid::Uuid::new_v4().simple()
    );
    let stored = marker.clone();
    db.write(move |conn| {
        let mut markers: Vec<String> =
            settings_repository::get(conn, SYNTHETIC_RUN_MARKERS_KEY)?.unwrap_or_default();
        if !markers.contains(&stored) {
            markers.push(stored);
            settings_repository::put(conn, SYNTHETIC_RUN_MARKERS_KEY, &markers)?;
        }
        Ok(())
    })?;
    Ok(marker)
}

/// Remove development-only synthetic history and its graph edges.
///
/// This is deliberately a storage-owned operation so the Phase 20 seeder
/// cannot bypass the SQL boundary. Checkpoints and investigations need an
/// exact registered per-run marker; events use their explicit collector label.
/// Entities have no immutable owner column, so this function leaves possible
/// unreferenced synthetic entities instead of risking deletion of a real row.
#[cfg(any(test, debug_assertions, feature = "devtools"))]
pub fn purge_synthetic(db: &Db) -> AppResult<usize> {
    db.write(|conn| {
        let tx = conn
            .unchecked_transaction()
            .during("begin synthetic purge")?;
        let markers: Vec<String> =
            settings_repository::get(&tx, SYNTHETIC_RUN_MARKERS_KEY)?.unwrap_or_default();
        if markers.is_empty() {
            // A copied/old QA marker is not proof that any remaining event
            // belongs to a registered seed run. Repeated purge is a no-op.
            tx.commit().during("commit empty synthetic purge")?;
            return Ok(0);
        }
        for marker in markers {
            tx.execute("DELETE FROM checkpoints WHERE reason = ?1", [&marker])
                .during("delete registered synthetic checkpoints")?;
            tx.execute("DELETE FROM investigations WHERE problem = ?1", [&marker])
                .during("delete registered synthetic investigations")?;
        }
        tx.execute(
            "DELETE FROM entity_relationships WHERE event_id IN (SELECT id FROM events WHERE source_collector = 'synthetic')",
            [],
        )
        .during("delete synthetic relationships")?;
        let deleted = tx
            .execute("DELETE FROM events WHERE source_collector = 'synthetic'", [])
            .during("delete synthetic events")?;
        // FTS5 delete commands can leave old term segments allocated even
        // after a full SQLite VACUUM. Rebuild from surviving events so a QA
        // purge releases synthetic search bytes without changing real rows.
        Fts5Index.rebuild(&tx)?;
        tx.execute(
            "DELETE FROM settings WHERE key = ?1",
            [SYNTHETIC_RUN_MARKERS_KEY],
        )
        .during("clear registered synthetic run markers")?;
        tx.commit().during("commit synthetic purge")?;
        Ok(deleted)
    })
}

/// Disk usage and per-table row counts.
pub fn storage_usage(conn: &Connection, paths: &DbPaths) -> AppResult<StorageUsage> {
    let (database_bytes, wal_bytes) = storage_bytes(conn, paths)?;
    let table_rows = vec![
        ("events".to_owned(), event_repository::count(conn)?),
        ("entities".to_owned(), entity_repository::count(conn)?),
        ("snapshots".to_owned(), snapshot_repository::count(conn)?),
        (
            "collector_baselines".to_owned(),
            baseline_repository::count(conn)?,
        ),
        (
            "config_snapshots".to_owned(),
            config_snapshot_repository::count(conn)?,
        ),
        ("investigations".to_owned(), investigations::count(conn)?),
    ];
    Ok(StorageUsage {
        database_bytes,
        wal_bytes,
        owner_workspace_bytes: None,
        table_rows,
    })
}

/// Main database and WAL sizes without the per-table row counts used by the
/// diagnostics screen. Privacy summary only needs this small read path.
pub fn storage_bytes(conn: &Connection, paths: &DbPaths) -> AppResult<(u64, u64)> {
    let page_count = conn
        .pragma_query_value(None, "page_count", |row| count_column(row, 0))
        .during("read page_count")?;
    let page_size = conn
        .pragma_query_value(None, "page_size", |row| count_column(row, 0))
        .during("read page_size")?;
    let wal_bytes = file_size(&paths.wal());
    Ok((page_count.saturating_mul(page_size), wal_bytes))
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// The background schedule. Dropping it stops the thread.
pub struct MaintenanceHandle {
    stop: SyncSender<()>,
    thread: Option<JoinHandle<()>>,
}

impl MaintenanceHandle {
    /// Run a pass now and every `INTERVAL` after, reading the retention
    /// policy from settings each time.
    pub fn spawn(db: Arc<Db>) -> MaintenanceHandle {
        let (stop, stopped) = sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("db-maintenance".to_owned())
            .spawn(move || loop {
                if db.state().is_ready() {
                    let policy = db.read(settings_repository::retention).unwrap_or_default();
                    if let Err(error) = run_pass(&db, policy, now_ms()) {
                        tracing::warn!(%error, "maintenance pass failed");
                    }
                }
                match stopped.recv_timeout(INTERVAL) {
                    Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    Err(RecvTimeoutError::Timeout) => {}
                }
            })
            .ok();
        MaintenanceHandle { stop, thread }
    }
}

impl Drop for MaintenanceHandle {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
