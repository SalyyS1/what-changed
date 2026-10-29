//! The per-key baseline a config diff compares against.
//!
//! `keys` holds key names and already-redacted values, never raw ones, which
//! is the schema-level restatement of the redaction promise: even a stolen
//! database has no plaintext config history in it.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{count_column, from_json, to_json, SqlContext};
use crate::domain::ids::{canonical_path, short_hash};
use crate::error::AppResult;
use crate::privacy::{Redaction, Sanitized, Sanitizer, Scrub};

const DELETE_BATCH_SIZE: usize = 400;

/// One config baseline operation carried by an atomic collector commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSnapshotMutation {
    Upsert(Sanitized<ConfigSnapshot>),
    Delete { path: String },
}

impl ConfigSnapshotMutation {
    pub fn upsert(snapshot: Sanitized<ConfigSnapshot>) -> Self {
        Self::Upsert(snapshot)
    }

    pub fn delete(path: impl Into<String>) -> Self {
        Self::Delete { path: path.into() }
    }

    pub(crate) fn encoded_bytes(&self) -> AppResult<usize> {
        match self {
            Self::Upsert(snapshot) => {
                let snapshot = snapshot.get();
                Ok(to_json(&snapshot.keys)?
                    .len()
                    .saturating_add(snapshot.path.len())
                    .saturating_add(snapshot.format.len()))
            }
            Self::Delete { path } => Ok(path.len()),
        }
    }

    pub(crate) fn validate(&self) -> bool {
        const MAX_PATH_BYTES: usize = 32_768;
        match self {
            Self::Upsert(snapshot) => {
                let snapshot = snapshot.get();
                let format = snapshot
                    .format
                    .strip_suffix("+truncated")
                    .unwrap_or(&snapshot.format);
                !snapshot.path.is_empty()
                    && snapshot.path.len() <= MAX_PATH_BYTES
                    && matches!(
                        format,
                        "env" | "json" | "yaml" | "toml" | "ini" | "properties" | "xml"
                    )
                    && snapshot.keys.is_object()
            }
            Self::Delete { path } => !path.is_empty() && path.len() <= MAX_PATH_BYTES,
        }
    }
}

/// The parsed, redacted key map of one config file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    /// Absolute path of the file.
    pub path: String,
    /// Flattened `dotted.key` to redacted value.
    pub keys: Value,
    /// When it was parsed, epoch milliseconds UTC.
    pub captured_at: i64,
    /// Which parser read it: `env`, `json`, `yaml`, `toml`, `ini`, and so on.
    pub format: String,
}

impl Scrub for ConfigSnapshot {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        let path = if field.is_empty() {
            "keys".to_owned()
        } else {
            format!("{field}.keys")
        };
        self.keys.scrub_in_place(sanitizer, &path, report);
    }
}

/// The primary key for a path: a hash of its canonical form.
pub fn path_hash(path: &str) -> String {
    short_hash(&canonical_path(path))
}

/// The stored snapshot for a path, if any.
pub fn get(conn: &Connection, path: &str) -> AppResult<Option<ConfigSnapshot>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT path, keys_json, captured_at, format FROM config_snapshots WHERE path_hash = ?1",
        )
        .during("prepare a config snapshot read")?;
    let mut rows = statement
        .query([path_hash(path)])
        .during("read a config snapshot")?;
    let Some(row) = rows.next().during("read a config snapshot")? else {
        return Ok(None);
    };
    let keys: String = row.get(1).during("read a config snapshot")?;
    Ok(Some(ConfigSnapshot {
        path: row.get(0).during("read a config snapshot")?,
        keys: from_json("keys_json", &keys)?,
        captured_at: row.get(2).during("read a config snapshot")?,
        format: row.get(3).during("read a config snapshot")?,
    }))
}

/// Every current watched-file state, ordered for deterministic checkpoint capture.
pub fn all(conn: &Connection) -> AppResult<Vec<ConfigSnapshot>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT path, keys_json, captured_at, format FROM config_snapshots ORDER BY path_hash",
        )
        .during("prepare all config snapshot reads")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .during("read all config snapshots")?;
    let mut snapshots = Vec::new();
    for row in rows {
        let (path, keys, captured_at, format) = row.during("read all config snapshots")?;
        snapshots.push(ConfigSnapshot {
            path,
            keys: from_json("keys_json", &keys)?,
            captured_at,
            format,
        });
    }
    Ok(snapshots)
}

/// Persisted snapshot paths covered by any supplied watched root.
pub fn paths_under(conn: &Connection, roots: &[String]) -> AppResult<Vec<String>> {
    if roots.is_empty() {
        return Ok(Vec::new());
    }
    let roots = roots
        .iter()
        .map(|root| canonical_path(root))
        .collect::<Vec<_>>();
    let mut statement = conn
        .prepare_cached("SELECT path FROM config_snapshots ORDER BY path")
        .during("prepare a config snapshot path scan")?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .during("scan config snapshot paths")?;
    let mut paths = Vec::new();
    for row in rows {
        let path = row.during("scan config snapshot paths")?;
        let canonical = canonical_path(&path);
        if roots.iter().any(|root| {
            canonical == *root
                || canonical
                    .strip_prefix(root)
                    .is_some_and(|suffix| suffix.starts_with('\\'))
        }) {
            paths.push(path);
        }
    }
    Ok(paths)
}

/// Store or replace the snapshot for a path.
pub fn put(conn: &Connection, snapshot: &Sanitized<ConfigSnapshot>) -> AppResult<()> {
    let snapshot = snapshot.get();
    conn.prepare_cached(
        "INSERT INTO config_snapshots (path_hash, path, keys_json, captured_at, format) \
         VALUES (?1, ?2, ?3, ?4, ?5) \
         ON CONFLICT(path_hash) DO UPDATE SET \
            path = excluded.path, keys_json = excluded.keys_json, \
            captured_at = excluded.captured_at, format = excluded.format",
    )
    .during("prepare a config snapshot write")?
    .execute((
        path_hash(&snapshot.path),
        &snapshot.path,
        to_json(&snapshot.keys)?,
        snapshot.captured_at,
        &snapshot.format,
    ))
    .during("store a config snapshot")?;
    Ok(())
}

/// Delete one file baseline by canonical-path hash.
pub fn delete(conn: &Connection, path: &str) -> AppResult<bool> {
    let deleted = conn
        .prepare_cached("DELETE FROM config_snapshots WHERE path_hash = ?1")
        .during("prepare a config snapshot delete")?
        .execute([path_hash(path)])
        .during("delete a config snapshot")?;
    Ok(deleted > 0)
}

pub(crate) fn apply(conn: &Connection, mutation: &ConfigSnapshotMutation) -> AppResult<()> {
    match mutation {
        ConfigSnapshotMutation::Upsert(snapshot) => put(conn, snapshot),
        ConfigSnapshotMutation::Delete { path } => delete(conn, path).map(|_| ()),
    }
}

/// Delete every snapshot for files under `root`, and return how many went.
/// Stopping a watch deletes its baselines: the promise is that the product
/// holds nothing about a folder the user stopped watching.
pub fn delete_under(conn: &Connection, root: &str) -> AppResult<usize> {
    let mut prefix = canonical_path(root);
    if !prefix.ends_with('\\') {
        prefix.push('\\');
    }
    // Paths are stored as given and compared canonically; the hash is
    // canonical so the comparison is done on the same form.
    let mut statement = conn
        .prepare_cached("SELECT path_hash, path FROM config_snapshots")
        .during("prepare a config snapshot scan")?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .during("scan config snapshots")?;
    let mut doomed = Vec::new();
    for row in rows {
        let (hash, path) = row.during("scan config snapshots")?;
        if canonical_path(&path).starts_with(&prefix) {
            doomed.push(hash);
        }
    }
    delete_hashes(conn, &doomed)
}

/// Delete snapshots under `root` that are not still covered by another
/// enabled watched folder at that folder's configured recursion depth.
pub fn delete_uncovered_under(conn: &Connection, root: &str) -> AppResult<usize> {
    let root = canonical_path(root);
    let kept = {
        let mut statement = conn
            .prepare_cached(
                "SELECT path, max_depth FROM watched_locations \
                 WHERE kind = 'folder' AND enabled = 1",
            )
            .during("prepare retained watched roots")?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    canonical_path(&row.get::<_, String>(0)?),
                    u8::try_from(row.get::<_, i64>(1)?).unwrap_or(u8::MAX),
                ))
            })
            .during("read retained watched roots")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .during("read retained watched roots")?;
        rows
    };
    let doomed = {
        let mut statement = conn
            .prepare_cached("SELECT path_hash, path FROM config_snapshots")
            .during("prepare uncovered config snapshot scan")?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .during("scan uncovered config snapshots")?;
        let mut doomed = Vec::new();
        for row in rows {
            let (hash, path) = row.during("scan uncovered config snapshots")?;
            let path = canonical_path(&path);
            if path_is_under(&path, &root)
                && !kept
                    .iter()
                    .any(|(kept_root, max_depth)| covered_at_depth(&path, kept_root, *max_depth))
            {
                doomed.push(hash);
            }
        }
        doomed
    };
    delete_hashes(conn, &doomed)
}

fn delete_hashes(conn: &Connection, hashes: &[String]) -> AppResult<usize> {
    let mut deleted = 0usize;
    for chunk in hashes.chunks(DELETE_BATCH_SIZE) {
        let placeholders = vec!["?"; chunk.len()].join(",");
        let sql = format!("DELETE FROM config_snapshots WHERE path_hash IN ({placeholders})");
        deleted = deleted.saturating_add(
            conn.execute(&sql, rusqlite::params_from_iter(chunk.iter()))
                .during("delete a config snapshot batch")?,
        );
    }
    Ok(deleted)
}

fn path_is_under(path: &str, root: &str) -> bool {
    path.strip_prefix(root)
        .is_some_and(|suffix| suffix.starts_with('\\'))
}

fn covered_at_depth(path: &str, root: &str, max_depth: u8) -> bool {
    let Some(relative) = path.strip_prefix(root) else {
        return false;
    };
    let Some(relative) = relative.strip_prefix('\\') else {
        return false;
    };
    let components = relative
        .split('\\')
        .filter(|component| !component.is_empty())
        .count();
    components <= usize::from(max_depth).saturating_add(1)
}

/// Total config snapshots.
pub fn count(conn: &Connection) -> AppResult<u64> {
    conn.prepare_cached("SELECT COUNT(*) FROM config_snapshots")
        .during("prepare a count")?
        .query_row([], |row| count_column(row, 0))
        .during("count config snapshots")
}
