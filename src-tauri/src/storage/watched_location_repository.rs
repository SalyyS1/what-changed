//! User-approved filesystem roots.
//!
//! Validation belongs at the command boundary; this repository owns the
//! serialized uniqueness/capacity check and the atomic unwatch cleanup.

use rusqlite::{Connection, OptionalExtension};

use super::{config_snapshot_repository, SqlContext};
use crate::error::AppResult;

/// One persisted watched root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchedLocation {
    pub id: i64,
    pub path: String,
    pub added_at: i64,
    pub enabled: bool,
    pub max_depth: u8,
}

/// All watched roots, oldest first for a stable Settings list.
pub fn list(conn: &Connection) -> AppResult<Vec<WatchedLocation>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT id, path, added_at, enabled, max_depth FROM watched_locations \
             WHERE kind = 'folder' ORDER BY added_at, id",
        )
        .during("prepare watched locations")?;
    let rows = statement
        .query_map([], |row| {
            Ok(WatchedLocation {
                id: row.get(0)?,
                path: row.get(1)?,
                added_at: row.get(2)?,
                enabled: row.get(3)?,
                max_depth: u8::try_from(row.get::<_, i64>(4)?).unwrap_or(u8::MAX),
            })
        })
        .during("list watched locations")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .during("list watched locations")?;
    Ok(rows)
}

/// Number of watched folder roots.
pub fn count(conn: &Connection) -> AppResult<usize> {
    let count: i64 = conn
        .prepare_cached("SELECT COUNT(*) FROM watched_locations WHERE kind = 'folder'")
        .during("prepare watched location count")?
        .query_row([], |row| row.get(0))
        .during("count watched locations")?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

/// Existing row for the same canonical path, compared case-insensitively for
/// Windows path semantics.
pub fn by_path(conn: &Connection, path: &str) -> AppResult<Option<WatchedLocation>> {
    conn.prepare_cached(
        "SELECT id, path, added_at, enabled, max_depth FROM watched_locations \
         WHERE kind = 'folder' AND lower(path) = lower(?1)",
    )
    .during("prepare watched location lookup")?
    .query_row([path], |row| {
        Ok(WatchedLocation {
            id: row.get(0)?,
            path: row.get(1)?,
            added_at: row.get(2)?,
            enabled: row.get(3)?,
            max_depth: u8::try_from(row.get::<_, i64>(4)?).unwrap_or(u8::MAX),
        })
    })
    .optional()
    .during("read watched location")
}

/// Insert a canonical path and return the persisted row.
pub fn insert(
    conn: &Connection,
    path: &str,
    max_depth: u8,
    added_at: i64,
) -> AppResult<WatchedLocation> {
    conn.prepare_cached(
        "INSERT INTO watched_locations (path, kind, added_at, enabled, max_depth) \
         SELECT ?1, 'folder', ?2, 1, ?3 \
         WHERE NOT EXISTS (SELECT 1 FROM watched_locations \
                           WHERE kind = 'folder' AND lower(path) = lower(?1))",
    )
    .during("prepare watched location insert")?
    .execute((path, added_at, i64::from(max_depth)))
    .during("insert watched location")?;

    by_path(conn, path)?.ok_or_else(|| {
        crate::error::AppError::Db(crate::error::DbError::Decoding {
            reason: "inserted watched location could not be read".to_owned(),
        })
    })
}

/// Remove a root and every config baseline below it in one transaction.
pub fn remove_with_snapshots(conn: &Connection, id: i64) -> AppResult<bool> {
    let tx = conn
        .unchecked_transaction()
        .during("begin watched location removal")?;
    let path: Option<String> = tx
        .prepare_cached("SELECT path FROM watched_locations WHERE id = ?1 AND kind = 'folder'")
        .during("prepare watched location removal")?
        .query_row([id], |row| row.get(0))
        .optional()
        .during("read watched location removal")?;
    let Some(path) = path else {
        tx.commit().during("commit watched location removal")?;
        return Ok(false);
    };
    tx.prepare_cached("DELETE FROM watched_locations WHERE id = ?1")
        .during("prepare watched location delete")?
        .execute([id])
        .during("delete watched location")?;
    config_snapshot_repository::delete_uncovered_under(&tx, &path)?;
    tx.commit().during("commit watched location removal")?;
    Ok(true)
}
