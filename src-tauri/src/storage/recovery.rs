//! What happens when the database is corrupt.
//!
//! Three actions, all user-initiated, none automatic. Repair rebuilds a new
//! file from whatever SQLite can still read and swaps it in only if the new
//! file passes its own integrity check. Diagnostics writes a report with no
//! row data in it. Reset moves the damaged file aside — never deletes it —
//! and starts fresh. Every action leaves the original on disk under a name
//! that says what it is.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::connection::{
    compile_options, open_writer, quick_check, register_path_functions, sqlite_version,
};
use super::db::{DbPaths, DbState};
use super::migrations::{self, MIGRATIONS};
use crate::domain::now_ms;
use crate::error::{AppError, AppResult, DbError};

/// What the user chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAction {
    /// Rebuild a fresh file from what can be read; keep the original aside.
    Repair,
    /// Write a report about the damage, with no history in it.
    ExportDiagnostics,
    /// Move the damaged file aside and start with an empty history.
    ResetHistory,
}

/// What an action produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct RecoveryOutcome {
    /// The action that ran.
    pub action: RecoveryAction,
    /// Whether it achieved what it set out to do.
    pub succeeded: bool,
    /// One sentence for the screen.
    pub message: String,
    /// A file the action produced: the quarantined original, or the report.
    pub artifact: Option<PathBuf>,
    /// The database state afterwards.
    pub state: DbState,
}

impl RecoveryOutcome {
    pub(crate) fn with_state(mut self, state: DbState) -> Self {
        self.state = state;
        self
    }
}

/// Run `action` against closed files. The caller reopens afterwards.
pub(crate) fn perform(
    paths: &DbPaths,
    state: &DbState,
    action: RecoveryAction,
) -> AppResult<RecoveryOutcome> {
    let placeholder = DbState::Recovering;
    match action {
        RecoveryAction::Repair => repair(paths).map(|artifact| RecoveryOutcome {
            action,
            succeeded: artifact.is_some(),
            message: match &artifact {
                Some(_) => {
                    "The database was rebuilt. The damaged copy was kept beside it.".to_owned()
                }
                None => "The database could not be rebuilt. You can reset your history or export \
                         diagnostics."
                    .to_owned(),
            },
            artifact,
            state: placeholder,
        }),
        RecoveryAction::ExportDiagnostics => {
            diagnostics(paths, state).map(|artifact| RecoveryOutcome {
                action,
                succeeded: true,
                message: "A diagnostics report was written. It contains no history.".to_owned(),
                artifact: Some(artifact),
                state: placeholder,
            })
        }
        RecoveryAction::ResetHistory => reset(paths).map(|artifact| RecoveryOutcome {
            action,
            succeeded: true,
            message: "Your history was moved aside and a fresh one started.".to_owned(),
            artifact: Some(artifact),
            state: placeholder,
        }),
    }
}

/// The name the damaged file is kept under.
fn quarantine_name(paths: &DbPaths) -> PathBuf {
    paths.root.join(format!("events.corrupt-{}.db", now_ms()))
}

/// Move the database and its companions to a quarantine name.
fn quarantine(paths: &DbPaths) -> AppResult<PathBuf> {
    let target = quarantine_name(paths);
    rename(&paths.database, &target)?;
    // The log and the shared-memory index are meaningless without the file
    // they belong to; they go with it so a fresh open cannot read them.
    for (companion, suffix) in [(paths.wal(), "-wal"), (paths.shm(), "-shm")] {
        if companion.exists() {
            let mut moved = target.as_os_str().to_owned();
            moved.push(suffix);
            rename(&companion, Path::new(&moved))?;
        }
    }
    Ok(target)
}

fn rename(from: &Path, to: &Path) -> AppResult<()> {
    fs::rename(from, to).map_err(|source| AppError::File {
        operation: "move",
        path: from.to_path_buf(),
        source,
    })
}

/// Try to build a clean copy. Returns the quarantined original on success,
/// `None` if nothing usable could be read.
fn repair(paths: &DbPaths) -> AppResult<Option<PathBuf>> {
    let rebuilt = paths.root.join("events.repair.db");
    let _ = fs::remove_file(&rebuilt);

    let copied = copy_readable(&paths.database, &rebuilt);
    if let Err(error) = &copied {
        tracing::warn!(%error, "repair could not read the damaged database");
    }
    let verified = copied.is_ok() && verify(&rebuilt);
    if !verified {
        let _ = fs::remove_file(&rebuilt);
        return Ok(None);
    }

    let original = quarantine(paths)?;
    rename(&rebuilt, &paths.database)?;
    Ok(Some(original))
}

/// `VACUUM INTO` reads every table it can and writes a fresh file; it fails
/// on damage it cannot read past, which is the honest outcome.
fn copy_readable(from: &Path, to: &Path) -> AppResult<()> {
    let conn = Connection::open_with_flags(
        from,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|source| DbError::Recovery {
        action: "open the damaged file",
        reason: source.to_string(),
    })?;
    // The latest schema contains expression indexes over canonical_path().
    // SQLite resolves those functions while VACUUM reconstructs the schema,
    // so recovery connections need the same deterministic UDF as normal ones.
    register_path_functions(&conn).map_err(|source| DbError::Recovery {
        action: "configure the repair connection",
        reason: source.to_string(),
    })?;
    conn.execute("VACUUM INTO ?1", [to.to_string_lossy().as_ref()])
        .map_err(|source| DbError::Recovery {
            action: "copy what can be read",
            reason: source.to_string(),
        })?;
    Ok(())
}

/// A rebuilt file counts only if it passes the same check the original failed.
fn verify(path: &Path) -> bool {
    match open_writer(path) {
        Ok(conn) => matches!(quick_check(&conn), Ok(None)),
        Err(_) => false,
    }
}

/// Move the damaged file aside and create an empty, migrated one.
fn reset(paths: &DbPaths) -> AppResult<PathBuf> {
    let original = quarantine(paths)?;
    let mut conn = open_writer(&paths.database)?;
    migrations::run(&mut conn, MIGRATIONS, &paths.backups)?;
    Ok(original)
}

/// What the report says. Structure and sizes, never rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    /// When it was written, epoch milliseconds UTC.
    pub written_at: i64,
    /// The state that prompted it.
    pub state: DbState,
    /// SQLite library version.
    pub sqlite_version: String,
    /// Path of the database.
    pub database: PathBuf,
    /// Size of the database file, if it exists.
    pub database_bytes: Option<u64>,
    /// Size of the write-ahead log, if it exists.
    pub wal_bytes: Option<u64>,
    /// What `quick_check` says now, if the file can be opened.
    pub quick_check: Option<String>,
    /// Schema version in the file, if readable.
    pub schema_version: Option<usize>,
    /// Migration ledger, if readable.
    pub ledger: Vec<(usize, String, String)>,
    /// Compile options of the library, so a missing feature is visible.
    pub compile_options: Vec<String>,
}

/// Write the report beside the database and return its path.
fn diagnostics(paths: &DbPaths, state: &DbState) -> AppResult<PathBuf> {
    let mut report = DiagnosticsReport {
        written_at: now_ms(),
        state: state.clone(),
        sqlite_version: sqlite_version().to_owned(),
        database: paths.database.clone(),
        database_bytes: fs::metadata(&paths.database).ok().map(|m| m.len()),
        wal_bytes: fs::metadata(paths.wal()).ok().map(|m| m.len()),
        quick_check: None,
        schema_version: None,
        ledger: Vec::new(),
        compile_options: Vec::new(),
    };
    if let Ok(conn) = Connection::open_with_flags(
        &paths.database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        report.quick_check = match quick_check(&conn) {
            Ok(None) => Some("ok".to_owned()),
            Ok(Some(problem)) => Some(problem),
            Err(error) => Some(error.to_string()),
        };
        report.schema_version = migrations::current_version(&conn, MIGRATIONS).ok();
        report.ledger = migrations::ledger(&conn).unwrap_or_default();
        report.compile_options = compile_options(&conn).unwrap_or_default();
    }
    let target = paths
        .root
        .join(format!("diagnostics-{}.json", report.written_at));
    let text = serde_json::to_string_pretty(&report).map_err(|error| DbError::Encoding {
        reason: error.to_string(),
    })?;
    fs::write(&target, text).map_err(|source| AppError::File {
        operation: "write",
        path: target.clone(),
        source,
    })?;
    Ok(target)
}
