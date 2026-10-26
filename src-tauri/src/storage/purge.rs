//! Explicit history deletion used by the privacy surface.
//!
//! Purges run on the writer connection so event rows, graph edges, FTS and
//! collector state move together.  The caller owns the visible confirmation;
//! this module only accepts a validated database operation and never touches
//! the reviewed appearance/retention preference allow-list.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::SqlContext;
use crate::domain::now_ms;
use crate::domain::RetentionPolicy;
use crate::error::AppResult;
use crate::storage::connection::{open_writer, quick_check};
use crate::storage::db::DbPaths;
use crate::storage::migrations::{self, MIGRATIONS};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PurgeCounts {
    pub events_deleted: u64,
    pub checkpoints_deleted: u64,
    pub investigations_deleted: u64,
    pub baselines_deleted: u64,
    pub watched_folders_deleted: u64,
}

fn count(conn: &Connection, table: &str) -> AppResult<u64> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    conn.query_row(&sql, [], |row| row.get::<_, i64>(0))
        .during("count rows before purge")
        .map(|value| u64::try_from(value).unwrap_or(0))
}

/// Count all history owned by the database before a replacement purge.
/// Keeping this in storage lets the lifecycle-level replacement return the
/// same report as the transactional scoped purges without exposing SQL to a
/// command or UI layer.
pub fn counts(conn: &Connection) -> AppResult<PurgeCounts> {
    Ok(PurgeCounts {
        events_deleted: count(conn, "events")?,
        checkpoints_deleted: count(conn, "checkpoints")?,
        investigations_deleted: count(conn, "investigations")?,
        baselines_deleted: count(conn, "collector_baselines")?,
        watched_folders_deleted: count(conn, "watched_locations")?,
    })
}

/// Delete all observed history while retaining only non-history preferences.
pub fn all(conn: &Connection) -> AppResult<PurgeCounts> {
    let counts = counts(conn)?;
    let preserved_theme: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'theme'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "\"system\"".to_owned());
    let preserved_visualization: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'visualization'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| "\"auto\"".to_owned());
    let transaction = conn
        .unchecked_transaction()
        .during("begin full history purge")?;
    // Delete children explicitly even though most foreign keys cascade; this
    // also clears rows from optional tables introduced by later migrations.
    for sql in [
        "DELETE FROM analysis_results",
        "DELETE FROM analysis_sessions",
        "DELETE FROM investigations",
        "DELETE FROM snapshots",
        "DELETE FROM checkpoints",
        "DELETE FROM entity_relationships",
        "DELETE FROM event_search_metadata",
        "DELETE FROM events",
        "DELETE FROM entities",
        "DELETE FROM collector_baselines",
        "DELETE FROM config_snapshots",
        "DELETE FROM applications",
        "DELETE FROM watched_locations",
        "DELETE FROM health_states",
        "DELETE FROM collector_status",
        "DELETE FROM projects",
        "DELETE FROM settings",
    ] {
        transaction.execute(sql, []).during("delete history rows")?;
    }
    // Restore the documented fresh-install defaults, preserving only the two
    // appearance preferences explicitly named by the privacy contract.
    let defaults = [
        ("theme", preserved_theme),
        ("visualization", preserved_visualization),
        (
            "retention",
            serde_json::to_string(&RetentionPolicy::default()).unwrap_or_else(|_| {
                "{\"events_days\":30,\"snapshots_days\":7,\"important_days\":90}".to_owned()
            }),
        ),
        ("monitoring_profile", "\"balanced\"".to_owned()),
        (
            "collectors",
            "{\"process\":true,\"listeners\":true,\"files\":true,\"services\":true,\"applications\":true,\"updates\":true,\"drivers\":true,\"network\":true,\"startup\":true,\"eventlog\":true,\"docker\":false,\"dev_env\":false}".to_owned(),
        ),
        ("first_hide_notified", "false".to_owned()),
        ("onboarding_completed", "false".to_owned()),
        ("developer_mode", "false".to_owned()),
        ("quiet_hours", "{\"enabled\":false,\"start_hour\":22,\"end_hour\":8}".to_owned()),
    ];
    for (key, value) in defaults {
        transaction
            .execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![key, value, crate::domain::now_ms()],
            )
            .during("restore fresh settings defaults")?;
    }
    // External-content FTS normally follows the event triggers. Rebuild is
    // cheap after a full purge and removes any stale shadow-table row left by
    // an interrupted migration.
    transaction
        .execute_batch(
            "INSERT INTO events_fts(events_fts) VALUES ('rebuild'); \
             INSERT INTO events_search_fts(events_search_fts) VALUES ('rebuild')",
        )
        .during("rebuild the history search index")?;
    transaction.commit().during("commit full history purge")?;
    reclaim(conn)?;
    Ok(counts)
}

/// Build and verify a fresh migrated database, then swap it into place while
/// the caller has detached every old SQLite handle. The old file remains
/// beside the database until the replacement passes integrity and migration
/// checks; this is deliberately stronger than deleting rows in place because
/// FTS shadow tables and WAL pages can otherwise retain old bytes.
pub(crate) fn replace_database(
    paths: &DbPaths,
    preserved_theme: &str,
    preserved_visualization: &str,
) -> AppResult<()> {
    let fresh = paths
        .root
        .join(format!("events.purge-fresh-{}.db", now_ms()));
    let old = paths.root.join(format!("events.purge-old-{}.db", now_ms()));
    let _ = fs::remove_file(&fresh);
    let _ = fs::remove_file(&old);
    let mut connection = open_writer(&fresh)?;
    migrations::run(&mut connection, MIGRATIONS, &paths.backups)?;
    connection
        .execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES ('theme', ?1, ?3), ('visualization', ?2, ?3)",
            rusqlite::params![preserved_theme, preserved_visualization, now_ms()],
        )
        .during("restore appearance preferences in replacement database")?;
    connection
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .during("checkpoint replacement database")?;
    drop(connection);
    let _ = fs::remove_file(companion(&fresh, "-wal"));
    let _ = fs::remove_file(companion(&fresh, "-shm"));

    // A full purge must not leave migration snapshots containing the history
    // the user just asked us to delete. Remove only app-owned backup files
    // before swapping the live database; failure leaves the original DB in
    // place and the caller can retry safely.
    if paths.backups.exists() {
        for entry in
            fs::read_dir(&paths.backups).map_err(|source| crate::error::AppError::Directory {
                path: paths.backups.clone(),
                source,
            })?
        {
            let path = entry
                .map_err(|source| crate::error::AppError::Directory {
                    path: paths.backups.clone(),
                    source,
                })?
                .path();
            if path.is_file() {
                fs::remove_file(&path).map_err(|source| crate::error::AppError::File {
                    operation: "remove privacy purge backup",
                    path,
                    source,
                })?;
            }
        }
    }

    fs::rename(&paths.database, &old).map_err(|source| crate::error::AppError::File {
        operation: "move old database before privacy replacement",
        path: paths.database.clone(),
        source,
    })?;
    for companion_path in [paths.wal(), paths.shm()] {
        let _ = fs::remove_file(companion_path);
    }
    if let Err(source) = fs::rename(&fresh, &paths.database) {
        let _ = fs::rename(&old, &paths.database);
        return Err(crate::error::AppError::File {
            operation: "install fresh database after privacy purge",
            path: paths.database.clone(),
            source,
        });
    }

    let verified = open_writer(&paths.database)
        .and_then(|conn| quick_check(&conn).map(|result| result.is_none()))
        .unwrap_or(false);
    if !verified {
        let _ = fs::remove_file(&paths.database);
        let _ = fs::rename(&old, &paths.database);
        return Err(crate::error::AppError::Platform {
            operation: "verify fresh database after privacy purge",
            reason: "the replacement failed SQLite integrity verification".to_owned(),
        });
    }
    fs::remove_file(&old).map_err(|source| crate::error::AppError::File {
        operation: "remove verified old database after privacy purge",
        path: old,
        source,
    })?;
    Ok(())
}

fn companion(database: &Path, suffix: &str) -> PathBuf {
    let mut name = database.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Delete events in one category and leave every other category untouched.
pub fn category(conn: &Connection, category: &str) -> AppResult<PurgeCounts> {
    let transaction = conn
        .unchecked_transaction()
        .during("begin category history purge")?;
    let events_deleted = transaction
        .query_row(
            "SELECT COUNT(*) FROM events WHERE category = ?1",
            [category],
            |row| row.get::<_, i64>(0),
        )
        .during("count category events")?
        .try_into()
        .unwrap_or(0);
    transaction
        .execute(
            "DELETE FROM entity_relationships WHERE event_id IN (SELECT id FROM events WHERE category = ?1)",
            [category],
        )
        .during("delete category relationships")?;
    transaction
        .execute("DELETE FROM events WHERE category = ?1", [category])
        .during("delete category events")?;
    transaction
        .execute(
            "DELETE FROM entities WHERE id NOT IN (SELECT entity_id FROM events UNION SELECT source_id FROM entity_relationships UNION SELECT target_id FROM entity_relationships)",
            [],
        )
        .during("delete orphaned category entities")?;
    transaction
        .execute_batch(
            "INSERT INTO events_fts(events_fts) VALUES ('rebuild'); \
             INSERT INTO events_search_fts(events_search_fts) VALUES ('rebuild')",
        )
        .during("rebuild category search index")?;
    transaction
        .commit()
        .during("commit category history purge")?;
    reclaim(conn)?;
    Ok(PurgeCounts {
        events_deleted,
        ..PurgeCounts::default()
    })
}

/// Stop watching one approved folder and remove only its configuration
/// history. The path is bound as a parameter and is never echoed in an error.
pub fn folder(conn: &Connection, path: &str) -> AppResult<PurgeCounts> {
    // File/config events retain the normalized source path in metadata so a
    // folder deletion can cover descendants even after a file is gone. The
    // separator checks avoid treating `C:\\work-old` as a child of
    // `C:\\work`.
    let predicate = "(json_extract(metadata_json, '$.path') = ?1 OR \
        instr(json_extract(metadata_json, '$.path'), ?1 || char(92)) = 1 OR \
        instr(json_extract(metadata_json, '$.path'), ?1 || '/') = 1)";
    let transaction = conn
        .unchecked_transaction()
        .during("begin folder history purge")?;
    let events_deleted = u64::try_from(
        transaction
            .query_row(
                &format!("SELECT COUNT(*) FROM events WHERE {predicate}"),
                [path],
                |row| row.get::<_, i64>(0),
            )
            .during("count folder events")?,
    )
    .unwrap_or(0);
    transaction
        .execute(
            &format!("DELETE FROM entity_relationships WHERE event_id IN (SELECT id FROM events WHERE {predicate})"),
            [path],
        )
        .during("delete folder relationships")?;
    transaction
        .execute(&format!("DELETE FROM events WHERE {predicate}"), [path])
        .during("delete folder events")?;
    let watched_folders_deleted = u64::try_from(
        transaction
            .execute("DELETE FROM watched_locations WHERE path = ?1", [path])
            .during("delete watched folder")?,
    )
    .unwrap_or(0);
    let baseline_predicate =
        "(path = ?1 OR instr(path, ?1 || char(92)) = 1 OR instr(path, ?1 || '/') = 1)";
    let baselines_deleted = u64::try_from(
        transaction
            .execute(
                &format!("DELETE FROM config_snapshots WHERE {baseline_predicate}"),
                [path],
            )
            .during("delete folder config baselines")?,
    )
    .unwrap_or(0);
    transaction
        .execute(
            "DELETE FROM entities WHERE id NOT IN (SELECT entity_id FROM events UNION SELECT source_id FROM entity_relationships UNION SELECT target_id FROM entity_relationships)",
            [],
        )
        .during("delete orphaned folder entities")?;
    transaction
        .execute_batch(
            "INSERT INTO events_fts(events_fts) VALUES ('rebuild'); \
             INSERT INTO events_search_fts(events_search_fts) VALUES ('rebuild')",
        )
        .during("rebuild folder search index")?;
    transaction.commit().during("commit folder history purge")?;
    reclaim(conn)?;
    Ok(PurgeCounts {
        events_deleted,
        baselines_deleted,
        watched_folders_deleted,
        ..PurgeCounts::default()
    })
}

fn reclaim(conn: &Connection) -> AppResult<()> {
    // Explicit purge is a user-requested destructive operation. Rebuilding
    // FTS alone does not remove free pages from the main file, so checkpoint
    // and vacuum while the writer is the only owner of this connection.
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA incremental_vacuum(256); VACUUM;")
        .during("reclaim purged history pages")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::Db;
    use crate::testing::temp_dir;

    #[test]
    fn full_purge_keeps_reviewed_preferences_and_clears_history_tables() {
        let db = Db::open_in(&temp_dir("purge-full")).unwrap();
        db.write(|conn| {
            conn.execute(
                "UPDATE settings SET value = '\"dark\"', updated_at = 1 WHERE key = 'theme'",
                [],
            )
            .during("insert purge fixture")?;
            conn.execute(
                "UPDATE settings SET value = 'true', updated_at = 1 WHERE key = 'onboarding_completed'",
                [],
            )
            .during("insert purge fixture")?;
            let report = all(conn).unwrap();
            assert_eq!(report.events_deleted, 0);
            let theme: String = conn.query_row(
                "SELECT value FROM settings WHERE key = 'theme'",
                [],
                |row| row.get(0),
            )
            .during("read purge fixture")?;
            assert_eq!(theme, "\"dark\"");
            let onboarding: String = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'onboarding_completed'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(onboarding, "false");
            Ok(())
        })
        .unwrap();
    }
}
