//! The migration runner and its ledger.
//!
//! Migrations are SQL files under `migrations/`, embedded at compile time so
//! a schema change is a reviewable diff. `rusqlite_migration` tracks the
//! version in `user_version`; this module additionally keeps the
//! `schema_migrations` table the brief asks for, with the SHA-256 of the SQL
//! that was applied — so a migration edited after it has run somewhere is
//! detected on the next open instead of trusted.
//!
//! Numbering is allocated here and nowhere else: the next migration takes the
//! next free integer in this directory.

use std::fs;
use std::path::Path;

use rusqlite::Connection;
use rusqlite_migration::{Migrations, SchemaVersion, M};
use sha2::{Digest, Sha256};

use super::SqlContext;
use crate::domain::now_ms;
use crate::error::{AppError, AppResult, DbError};

/// One schema migration.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    /// Position in the sequence, starting at 1.
    pub version: usize,
    /// Short name, recorded in the ledger.
    pub name: &'static str,
    /// The SQL, applied as one batch.
    pub sql: &'static str,
    /// Whether the migration only adds (tables, columns, indexes, rows). A
    /// migration that is not purely additive gets a backup taken first.
    pub additive: bool,
}

/// Every migration this build knows, in order.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial",
        sql: include_str!("../../migrations/001_initial.sql"),
        additive: true,
    },
    Migration {
        version: 2,
        name: "search_index",
        sql: include_str!("../../migrations/002_search_index.sql"),
        additive: true,
    },
    Migration {
        version: 3,
        name: "search_metadata",
        sql: include_str!("../../migrations/003_search_metadata.sql"),
        additive: true,
    },
    Migration {
        version: 4,
        name: "event_field",
        sql: include_str!("../../migrations/004_event_field.sql"),
        additive: true,
    },
    Migration {
        version: 5,
        name: "checkpoint_items",
        sql: include_str!("../../migrations/005_checkpoint_items.sql"),
        additive: true,
    },
    Migration {
        version: 6,
        name: "investigation_storage",
        sql: include_str!("../../migrations/006_investigation_storage.sql"),
        additive: true,
    },
    Migration {
        version: 7,
        name: "system_map_indexes",
        sql: include_str!("../../migrations/007_system_map_indexes.sql"),
        additive: true,
    },
    Migration {
        version: 8,
        name: "projects",
        sql: include_str!("../../migrations/008_projects.sql"),
        additive: true,
    },
    Migration {
        version: 9,
        name: "search_order",
        sql: include_str!("../../migrations/009_search_order.sql"),
        additive: true,
    },
    Migration {
        version: 10,
        name: "search_importance_fts",
        sql: include_str!("../../migrations/010_search_importance_fts.sql"),
        additive: true,
    },
    Migration {
        version: 11,
        name: "summary_range_index",
        sql: include_str!("../../migrations/011_summary_range_index.sql"),
        additive: true,
    },
    Migration {
        version: 12,
        name: "daily_importance_counts",
        sql: include_str!("../../migrations/012_daily_importance_counts.sql"),
        additive: true,
    },
    Migration {
        version: 13,
        name: "daily_importance_floor",
        sql: include_str!("../../migrations/013_daily_importance_floor.sql"),
        additive: false,
    },
    Migration {
        version: 14,
        name: "daily_importance_first_day_boundary",
        sql: include_str!("../../migrations/014_daily_importance_first_day_boundary.sql"),
        additive: false,
    },
    Migration {
        version: 15,
        name: "event_category_counts",
        sql: include_str!("../../migrations/015_event_category_counts.sql"),
        additive: true,
    },
    Migration {
        version: 16,
        name: "project_baseline",
        sql: include_str!("../../migrations/016_project_baseline.sql"),
        additive: true,
    },
];

/// Backups kept before a non-additive migration: the newest this many.
const KEPT_BACKUPS: usize = 2;

/// Hex SHA-256 of a migration's SQL.
pub fn checksum(sql: &str) -> String {
    Sha256::digest(sql.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// What `run` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    /// Schema version before.
    pub from: usize,
    /// Schema version after.
    pub to: usize,
    /// Backup file written before a non-additive migration, if one was.
    pub backup: Option<std::path::PathBuf>,
}

/// The schema version stored in the file, as a plain number.
pub fn current_version(conn: &Connection, list: &[Migration]) -> AppResult<usize> {
    let migrations = build(list);
    match migrations.current_version(conn) {
        Ok(SchemaVersion::NoneSet) => Ok(0),
        Ok(SchemaVersion::Inside(v)) => Ok(v.get()),
        Ok(SchemaVersion::Outside(v)) => Err(DbError::Ahead {
            found: v.get(),
            known: list.len(),
        }
        .into()),
        Err(error) => Err(migration_error(0, &error)),
    }
}

fn build(list: &[Migration]) -> Migrations<'static> {
    Migrations::new(list.iter().map(|m| M::up(m.sql)).collect())
}

fn migration_error(version: usize, error: &dyn std::fmt::Display) -> AppError {
    DbError::Migration {
        version,
        reason: error.to_string(),
    }
    .into()
}

/// Bring the database to the latest version in `list`, taking a backup first
/// when a pending migration is not purely additive, and record every newly
/// applied version in the ledger.
///
/// Takes the writer connection by unique reference: migrations run once, at
/// open, before the connection is handed to the writer thread. `VACUUM INTO`
/// refuses to run inside a transaction, which is why the backup happens
/// before the runner opens one.
pub fn run(
    conn: &mut Connection,
    list: &[Migration],
    backups_dir: &Path,
) -> AppResult<MigrationReport> {
    super::connection::register_path_functions(conn)?;
    let from = current_version(conn, list)?;
    if from > 0 {
        verify_ledger(conn, list, from)?;
    }

    let first_risky = list
        .iter()
        .filter(|m| m.version > from && !m.additive)
        .map(|m| m.version)
        .next();
    let backup = match first_risky {
        Some(version) if from > 0 => Some(backup_before(conn, backups_dir, version)?),
        _ => None,
    };

    let migrations = build(list);
    // Validate against a fresh schema with the same deterministic functions as
    // production. The library's default validator opens an unconfigured DB.
    let mut validation = Connection::open_in_memory().during("open migration validation")?;
    super::connection::register_path_functions(&validation)?;
    migrations
        .to_latest(&mut validation)
        .map_err(|error| migration_error(0, &error))?;
    migrations
        .to_latest(conn)
        .map_err(|error| migration_error(from + 1, &error))?;

    let to = list.last().map(|m| m.version).unwrap_or(0);
    record_applied(conn, list, from)?;
    Ok(MigrationReport { from, to, backup })
}

/// Compare the ledger with the SQL this build ships for every version already
/// applied. A mismatch is reported, never ignored.
fn verify_ledger(conn: &Connection, list: &[Migration], applied: usize) -> AppResult<()> {
    let mut statement = conn
        .prepare_cached("SELECT version, sha256 FROM schema_migrations ORDER BY version")
        .during("read the migration ledger")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .during("read the migration ledger")?;
    let mut recorded_versions = std::collections::BTreeSet::new();
    for row in rows {
        let (version, recorded) = row.during("read the migration ledger")?;
        let version = usize::try_from(version).unwrap_or(0);
        if version == 0 || version > applied || !recorded_versions.insert(version) {
            return Err(DbError::Drift { version }.into());
        }
        let Some(migration) = list.iter().find(|m| m.version == version) else {
            return Err(DbError::Ahead {
                found: version,
                known: list.len(),
            }
            .into());
        };
        if checksum(migration.sql) != recorded {
            return Err(DbError::Drift { version }.into());
        }
    }
    for expected in 1..=applied {
        if !recorded_versions.contains(&expected) {
            return Err(DbError::Drift { version: expected }.into());
        }
    }
    Ok(())
}

/// Insert a ledger row for every version above `from`.
fn record_applied(conn: &Connection, list: &[Migration], from: usize) -> AppResult<()> {
    let now = now_ms();
    let mut statement = conn
        .prepare_cached(
            "INSERT OR IGNORE INTO schema_migrations (version, name, applied_at, sha256) \
             VALUES (?1, ?2, ?3, ?4)",
        )
        .during("record a migration")?;
    for migration in list.iter().filter(|m| m.version > from) {
        statement
            .execute((
                i64::try_from(migration.version).unwrap_or(i64::MAX),
                migration.name,
                now,
                checksum(migration.sql),
            ))
            .during("record a migration")?;
    }
    Ok(())
}

/// `VACUUM INTO backups/events-pre-v<n>.db`, then keep only the newest few.
fn backup_before(
    conn: &Connection,
    backups_dir: &Path,
    version: usize,
) -> AppResult<std::path::PathBuf> {
    fs::create_dir_all(backups_dir).map_err(|source| AppError::Directory {
        path: backups_dir.to_path_buf(),
        source,
    })?;
    let target = backups_dir.join(format!("events-pre-v{version}.db"));
    if target.exists() {
        fs::remove_file(&target).map_err(|source| AppError::File {
            operation: "replace",
            path: target.clone(),
            source,
        })?;
    }
    conn.execute("VACUUM INTO ?1", [target.to_string_lossy().as_ref()])
        .during("back up before a migration")?;
    prune_backups(backups_dir)?;
    Ok(target)
}

/// Remove all but the newest `KEPT_BACKUPS` pre-migration backups, newest by
/// the version in the name, which is what "most recent" means for a schema.
fn prune_backups(backups_dir: &Path) -> AppResult<()> {
    let entries = fs::read_dir(backups_dir).map_err(|source| AppError::Directory {
        path: backups_dir.to_path_buf(),
        source,
    })?;
    let mut backups: Vec<(usize, std::path::PathBuf)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?;
            let version = name
                .strip_prefix("events-pre-v")?
                .strip_suffix(".db")?
                .parse()
                .ok()?;
            Some((version, path))
        })
        .collect();
    backups.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    for (_, path) in backups.into_iter().skip(KEPT_BACKUPS) {
        fs::remove_file(&path).map_err(|source| AppError::File {
            operation: "prune",
            path,
            source,
        })?;
    }
    Ok(())
}

/// The ledger, for diagnostics and tests.
pub fn ledger(conn: &Connection) -> AppResult<Vec<(usize, String, String)>> {
    let mut statement = conn
        .prepare_cached("SELECT version, name, sha256 FROM schema_migrations ORDER BY version")
        .during("read the migration ledger")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                usize::try_from(row.get::<_, i64>(0)?).unwrap_or(0),
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .during("read the migration ledger")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .during("read the migration ledger")?;
    Ok(rows)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_consecutive_from_one_and_the_files_are_distinct() {
        for (index, migration) in MIGRATIONS.iter().enumerate() {
            assert_eq!(migration.version, index + 1);
            assert!(!migration.sql.trim().is_empty());
        }
        let sums: std::collections::HashSet<String> =
            MIGRATIONS.iter().map(|m| checksum(m.sql)).collect();
        assert_eq!(sums.len(), MIGRATIONS.len());
    }

    #[test]
    fn the_search_migration_has_exactly_one_body() {
        let search = MIGRATIONS.iter().filter(|m| m.version == 2).count();
        assert_eq!(search, 1);
        let sql = MIGRATIONS[1].sql;
        assert_eq!(sql.matches("CREATE VIRTUAL TABLE").count(), 1);
        assert!(
            !sql.contains("CASE WHEN"),
            "no runtime branch in a migration"
        );
    }

    #[test]
    fn a_checksum_is_hex_sha256() {
        let sum = checksum("abc");
        assert_eq!(sum.len(), 64);
        assert_eq!(
            sum,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
