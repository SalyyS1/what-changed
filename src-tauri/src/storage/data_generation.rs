//! SQLite-owned generation snapshots; callers own filesystem admission and paths.

use std::path::Path;
use std::time::{Duration, Instant};

use rusqlite::backup::{Backup, StepResult};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::{connection, migrations, SqlContext};
use crate::error::{AppResult, DbError};
use crate::owner_workspace::OwnerStore;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoreSchemas {
    pub legacy: usize,
    pub owner: usize,
}

fn invalid(reason: &str) -> crate::error::AppError {
    DbError::Recovery {
        action: "validate data generation",
        reason: reason.into(),
    }
    .into()
}

fn reader(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .during("open generation reader")?;
    connection::register_path_functions(&conn)?;
    Ok(conn)
}

fn legacy_schema(conn: &Connection) -> AppResult<usize> {
    if connection::quick_check(conn)?.is_some() {
        return Err(invalid("legacy integrity check failed"));
    }
    let version = migrations::current_version(conn, migrations::MIGRATIONS)?;
    if version == 0 {
        return Err(invalid(
            "a generation requires an initialized legacy schema",
        ));
    }
    let ledger = migrations::ledger(conn)?;
    if ledger.len() != version {
        return Err(invalid("legacy migration inventory differs"));
    }
    for (index, (recorded, _, checksum)) in ledger.into_iter().enumerate() {
        let expected = migrations::MIGRATIONS
            .get(index)
            .ok_or_else(|| invalid("legacy schema is newer"))?;
        if recorded != index + 1
            || recorded != expected.version
            || checksum != migrations::checksum(expected.sql)
        {
            return Err(invalid("legacy migration checksum differs"));
        }
    }
    let violation: Option<String> = conn
        .query_row("PRAGMA foreign_key_check", [], |r| r.get(0))
        .optional()
        .during("check restored legacy references")?;
    if violation.is_some() {
        return Err(invalid("legacy references are invalid"));
    }
    Ok(version)
}

pub(crate) fn validate_pair(root: &Path) -> AppResult<StoreSchemas> {
    let legacy = legacy_schema(&reader(&root.join("events.db"))?)?;
    let owner = OwnerStore::inspect_backup(&root.join("owner-workspace.sqlite"))?;
    if owner == 0 {
        return Err(invalid("a generation requires an initialized owner schema"));
    }
    Ok(StoreSchemas { legacy, owner })
}

fn snapshot(source: &Path, destination: &Path) -> AppResult<()> {
    let from = reader(source)?;
    // The caller reserved a new file below a pinned app-owned staging root.
    // NOFOLLOW also rejects a redirected final component at SQLite open.
    let mut to = Connection::open_with_flags(
        destination,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .during("open generation snapshot")?;
    to.pragma_update(None, "synchronous", "FULL")
        .during("require durable generation pages")?;
    let began = Instant::now();
    {
        let backup = Backup::new(&from, &mut to).during("start consistent generation snapshot")?;
        loop {
            if began.elapsed() > Duration::from_secs(30) {
                return Err(invalid("SQLite backup exceeded its bounded deadline"));
            }
            match backup.step(256).during("copy committed generation pages")? {
                StepResult::Done => break,
                StepResult::More => {}
                StepResult::Busy | StepResult::Locked => {
                    return Err(invalid(
                        "SQLite backup is locked; preserve the incomplete stage",
                    ))
                }
                _ => return Err(invalid("unsupported SQLite backup progress")),
            }
        }
    }
    // A complete generation is a self-contained DB, never a DB plus an omitted WAL.
    to.pragma_update(None, "journal_mode", "DELETE")
        .during("seal generation journal mode")?;
    to.close().map_err(|(_, source)| DbError::Sqlite {
        operation: "close generation snapshot",
        source,
    })?;
    Ok(())
}

pub(crate) fn snapshot_pair(source: &Path, destination: &Path) -> AppResult<StoreSchemas> {
    let schemas = validate_pair(source)?;
    for name in ["events.db", "owner-workspace.sqlite"] {
        snapshot(&source.join(name), &destination.join(name))?;
    }
    if validate_pair(destination)? != schemas {
        return Err(invalid("snapshot schema identity changed"));
    }
    Ok(schemas)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;
