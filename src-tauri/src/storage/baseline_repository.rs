//! Each collector's last-known state, persisted so a restart diffs against
//! what the machine looked like when the app was last running instead of
//! treating the current state as a fresh baseline. Without this the recorder
//! is blind to every change made while it was closed — the window a user is
//! most likely to be asking about.
//!
//! One row per `(collector, entity kind)`, holding the whole snapshot, so the
//! table is bounded by collectors times kinds rather than by entities.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{count_column, from_json, to_json, SqlContext};
use crate::error::{AppError, AppResult};
use crate::privacy::{Redaction, Sanitized, Sanitizer, Scrub};

/// A snapshot larger than this is not stored; a marker is stored instead, so
/// a collector that enumerates something enormous cannot fill the database
/// with baselines.
pub const MAX_SNAPSHOT_BYTES: usize = 1024 * 1024;

/// A collector's last-known state for one kind of entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    /// Which collector.
    pub collector_id: String,
    /// Which kind of entity, in the collector's own vocabulary.
    pub entity_kind: String,
    /// The state set, already redacted.
    pub snapshot: Value,
    /// When it was captured, epoch milliseconds UTC.
    pub captured_at: i64,
    /// Whether the stored snapshot is a marker standing in for one that was
    /// too large to keep.
    #[serde(default)]
    pub truncated: bool,
}

impl Scrub for Baseline {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        let path = if field.is_empty() {
            "snapshot".to_owned()
        } else {
            format!("{field}.snapshot")
        };
        self.snapshot.scrub_in_place(sanitizer, &path, report);
    }
}

/// The stored baseline, or `None` if this collector has never stored one.
pub fn load(
    conn: &Connection,
    collector_id: &str,
    entity_kind: &str,
) -> AppResult<Option<Baseline>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT snapshot_json, captured_at, truncated FROM collector_baselines \
             WHERE collector_id = ?1 AND entity_kind = ?2",
        )
        .during("prepare a baseline read")?;
    let mut rows = statement
        .query([collector_id, entity_kind])
        .during("read a baseline")?;
    let Some(row) = rows.next().during("read a baseline")? else {
        return Ok(None);
    };
    let text: String = row.get(0).during("read a baseline")?;
    Ok(Some(Baseline {
        collector_id: collector_id.to_owned(),
        entity_kind: entity_kind.to_owned(),
        snapshot: from_json("snapshot_json", &text)?,
        captured_at: row.get(1).during("read a baseline")?,
        truncated: row.get(2).during("read a baseline")?,
    }))
}

/// Every durable collector baseline, ordered for deterministic checkpoint capture.
pub fn all(conn: &Connection) -> AppResult<Vec<Baseline>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT collector_id, entity_kind, snapshot_json, captured_at, truncated \
             FROM collector_baselines ORDER BY collector_id, entity_kind",
        )
        .during("prepare all baseline reads")?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, bool>(4)?,
            ))
        })
        .during("read all baselines")?;
    let mut baselines = Vec::new();
    for row in rows {
        let (collector_id, entity_kind, snapshot, captured_at, truncated) =
            row.during("read all baselines")?;
        baselines.push(Baseline {
            collector_id,
            entity_kind,
            snapshot: from_json("snapshot_json", &snapshot)?,
            captured_at,
            truncated,
        });
    }
    Ok(baselines)
}

/// Store a baseline, replacing the previous one for the same collector and
/// kind. A snapshot over `MAX_SNAPSHOT_BYTES` is replaced by a marker
/// recording its size, and the replacement is logged.
pub fn store(conn: &Connection, baseline: &Sanitized<Baseline>) -> AppResult<()> {
    store_with_sql(conn, baseline, BASELINE_UPSERT)?;
    Ok(())
}

/// Store a baseline only when it is newer than the durable one, or is an
/// exact idempotent replay at the same capture time.
///
/// Collector commits use this variant so an out-of-order cycle rolls its
/// event and graph facts back instead of replacing newer diff state.
pub(crate) fn store_monotonic(conn: &Connection, baseline: &Sanitized<Baseline>) -> AppResult<()> {
    if store_with_sql(conn, baseline, MONOTONIC_BASELINE_UPSERT)? == 0 {
        return Err(AppError::InvalidInput {
            what: "collector commit",
            reason: "the baseline is older or conflicts at the durable capture time".to_owned(),
        });
    }
    Ok(())
}

const BASELINE_UPSERT: &str =
    "INSERT INTO collector_baselines (collector_id, entity_kind, snapshot_json, captured_at, truncated) \
     VALUES (?1, ?2, ?3, ?4, ?5) \
     ON CONFLICT(collector_id, entity_kind) DO UPDATE SET \
        snapshot_json = excluded.snapshot_json, \
        captured_at = excluded.captured_at, \
        truncated = excluded.truncated";

const MONOTONIC_BASELINE_UPSERT: &str =
    "INSERT INTO collector_baselines (collector_id, entity_kind, snapshot_json, captured_at, truncated) \
     VALUES (?1, ?2, ?3, ?4, ?5) \
     ON CONFLICT(collector_id, entity_kind) DO UPDATE SET \
        snapshot_json = excluded.snapshot_json, \
        captured_at = excluded.captured_at, \
        truncated = excluded.truncated \
     WHERE collector_baselines.captured_at < excluded.captured_at \
        OR (collector_baselines.captured_at = excluded.captured_at \
            AND collector_baselines.snapshot_json = excluded.snapshot_json \
            AND collector_baselines.truncated = excluded.truncated)";

fn store_with_sql(
    conn: &Connection,
    baseline: &Sanitized<Baseline>,
    sql: &'static str,
) -> AppResult<usize> {
    let baseline = baseline.get();
    let mut json = to_json(&baseline.snapshot)?;
    let mut truncated = baseline.truncated;
    if json.len() > MAX_SNAPSHOT_BYTES {
        tracing::warn!(
            collector = %baseline.collector_id,
            kind = %baseline.entity_kind,
            bytes = json.len(),
            "a baseline snapshot was too large to keep and was replaced by a marker"
        );
        json = to_json(&serde_json::json!({ "truncated": true, "bytes": json.len() }))?;
        truncated = true;
    }
    conn.prepare_cached(sql)
        .during("prepare a baseline write")?
        .execute((
            &baseline.collector_id,
            &baseline.entity_kind,
            json,
            baseline.captured_at,
            truncated,
        ))
        .during("store a baseline")
}

/// Forget every baseline. A history reset does this so a fresh database
/// re-baselines with zero events instead of replaying old state as change.
pub fn clear(conn: &Connection) -> AppResult<usize> {
    conn.prepare_cached("DELETE FROM collector_baselines")
        .during("prepare a baseline clear")?
        .execute([])
        .during("clear baselines")
}

/// Total baselines.
pub fn count(conn: &Connection) -> AppResult<u64> {
    conn.prepare_cached("SELECT COUNT(*) FROM collector_baselines")
        .during("prepare a count")?
        .query_row([], |row| count_column(row, 0))
        .during("count baselines")
}
