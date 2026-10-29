//! Opening a connection the way this product needs it: pragmas applied,
//! integrity checked, build-time features asserted.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use super::SqlContext;
use crate::error::{AppResult, DbError};

/// SQLite compile option the search index depends on.
pub const FTS5_OPTION: &str = "ENABLE_FTS5";

/// Pages `PRAGMA wal_autocheckpoint` lets the log grow to before it folds it
/// back into the main file.
const WAL_AUTOCHECKPOINT_PAGES: i64 = 1000;

/// Page cache, negative meaning kibibytes: about 8 MB.
const CACHE_SIZE_KIB: i64 = -8000;

/// How long a statement waits on a lock before failing.
const BUSY_TIMEOUT_MS: i64 = 5000;

/// Schema expressions must use the exact same Windows path identity as Rust.
/// SQLite's built-in lower() only folds ASCII and cannot prove Unicode paths.
pub(crate) fn register_path_functions(conn: &Connection) -> AppResult<()> {
    use rusqlite::functions::FunctionFlags;
    conn.create_scalar_function(
        "canonical_path",
        1,
        FunctionFlags::SQLITE_UTF8
            | FunctionFlags::SQLITE_DETERMINISTIC
            | FunctionFlags::SQLITE_INNOCUOUS,
        |context| {
            let path = context.get::<Option<String>>(0)?;
            Ok(path.map(|path| crate::domain::ids::canonical_path(&path)))
        },
    )
    .during("register canonical path function")
}

/// Open the one read-write connection and configure it.
///
/// `auto_vacuum` is set before any table exists, which is the only time it
/// can be set without a full `VACUUM`; on an existing file it is already
/// persistent and the pragma is a no-op.
pub fn open_writer(path: &Path) -> AppResult<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_CREATE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(path, flags).during("open the database")?;
    register_path_functions(&conn)?;
    conn.pragma_update(None, "busy_timeout", BUSY_TIMEOUT_MS)
        .during("set busy_timeout")?;
    conn.pragma_update(None, "auto_vacuum", "INCREMENTAL")
        .during("set auto_vacuum")?;
    let mode: String = conn
        .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
        .during("switch to WAL")?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(DbError::Sqlite {
            operation: "switch to WAL",
            source: rusqlite::Error::InvalidQuery,
        }
        .into());
    }
    conn.pragma_update(None, "synchronous", "NORMAL")
        .during("set synchronous")?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .during("enable foreign keys")?;
    conn.pragma_update(None, "temp_store", "MEMORY")
        .during("set temp_store")?;
    conn.pragma_update(None, "cache_size", CACHE_SIZE_KIB)
        .during("set cache_size")?;
    conn.pragma_update(None, "wal_autocheckpoint", WAL_AUTOCHECKPOINT_PAGES)
        .during("set wal_autocheckpoint")?;
    Ok(conn)
}

/// Open one read-only connection for the pool.
///
/// Read-only at the file-handle level, not by convention: the flag makes a
/// write on this connection an error from SQLite itself.
pub fn open_reader(path: &Path) -> AppResult<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(path, flags).during("open a reader")?;
    register_path_functions(&conn)?;
    conn.pragma_update(None, "busy_timeout", BUSY_TIMEOUT_MS)
        .during("set busy_timeout")?;
    conn.pragma_update(None, "temp_store", "MEMORY")
        .during("set temp_store")?;
    conn.pragma_update(None, "cache_size", CACHE_SIZE_KIB)
        .during("set cache_size")?;
    Ok(conn)
}

/// Run `PRAGMA quick_check` and return the first line that is not `ok`.
///
/// A corrupt file may also fail earlier, at the first pragma, with
/// "file is not a database"; callers treat both as corruption.
pub fn quick_check(conn: &Connection) -> AppResult<Option<String>> {
    let mut statement = conn
        .prepare("PRAGMA quick_check")
        .during("prepare quick_check")?;
    let mut rows = statement.query([]).during("run quick_check")?;
    while let Some(row) = rows.next().during("read quick_check")? {
        let line: String = row.get(0).during("read quick_check")?;
        if line != "ok" {
            return Ok(Some(line));
        }
    }
    Ok(None)
}

/// The compile options the vendored SQLite was built with.
pub fn compile_options(conn: &Connection) -> AppResult<Vec<String>> {
    let mut statement = conn
        .prepare("PRAGMA compile_options")
        .during("prepare compile_options")?;
    let options = statement
        .query_map([], |row| row.get::<_, String>(0))
        .during("read compile_options")?
        .collect::<rusqlite::Result<Vec<String>>>()
        .during("read compile_options")?;
    Ok(options)
}

/// Fail loudly if the vendored SQLite lost FTS5. A regression guard for a
/// dependency bump, not a branch: the schema has one shape.
pub fn assert_fts5(conn: &Connection) -> AppResult<()> {
    if compile_options(conn)?.iter().any(|o| o == FTS5_OPTION) {
        Ok(())
    } else {
        Err(DbError::MissingCompileOption {
            option: FTS5_OPTION,
        }
        .into())
    }
}

/// The SQLite library version, for diagnostics.
pub fn sqlite_version() -> &'static str {
    rusqlite::version()
}

/// Whether a `rusqlite` error means the file is not a usable database.
pub fn is_corruption(error: &rusqlite::Error) -> bool {
    use rusqlite::ErrorCode;
    match error {
        rusqlite::Error::SqliteFailure(failure, _) => matches!(
            failure.code,
            ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt
        ),
        _ => false,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_writer_is_in_wal_mode_with_the_documented_pragmas() {
        let dir = crate::testing::temp_dir("connection");
        let conn = open_writer(&dir.join("events.db")).unwrap();
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        let fk: i64 = conn
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
        let vacuum: i64 = conn
            .pragma_query_value(None, "auto_vacuum", |r| r.get(0))
            .unwrap();
        assert_eq!(vacuum, 2, "2 is INCREMENTAL");
        assert!(quick_check(&conn).unwrap().is_none());
        assert_fts5(&conn).unwrap();
    }

    #[test]
    fn a_reader_cannot_write() {
        let dir = crate::testing::temp_dir("reader");
        let path = dir.join("events.db");
        let writer = open_writer(&path).unwrap();
        writer.execute_batch("CREATE TABLE t (x)").unwrap();
        let reader = open_reader(&path).unwrap();
        assert!(reader.execute("INSERT INTO t VALUES (1)", []).is_err());
    }
}
