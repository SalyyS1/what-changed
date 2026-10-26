//! Detailed snapshots: a collector's state set at one moment.
//!
//! A snapshot that belongs to a checkpoint is exempt from retention and goes
//! when the checkpoint goes. One with no checkpoint expires on the snapshot
//! lifetime. The exemption is in the delete statement, so it cannot be
//! forgotten by a caller.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{count_column, from_json, to_json, SqlContext};
use crate::error::AppResult;
use crate::privacy::{Redaction, Sanitized, Sanitizer, Scrub};

/// One entity's state as a collector saw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Which collector.
    pub collector_id: String,
    /// What kind of entity, in the collector's own vocabulary.
    pub entity_kind: String,
    /// The collector's identifier for it, if it has one.
    pub entity_external_id: Option<String>,
    /// The name a person recognises.
    pub entity_name: Option<String>,
    /// The state, already redacted.
    pub state: Value,
    /// When it was captured, epoch milliseconds UTC.
    pub captured_at: i64,
    /// The checkpoint that owns it, if any.
    pub checkpoint_id: Option<i64>,
}

impl Scrub for Snapshot {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        let prefix = if field.is_empty() {
            String::new()
        } else {
            format!("{field}.")
        };
        self.entity_name
            .scrub_in_place(sanitizer, &format!("{prefix}entity_name"), report);
        self.state
            .scrub_in_place(sanitizer, &format!("{prefix}state"), report);
    }
}

/// Store one snapshot and return its id.
pub fn insert(conn: &Connection, snapshot: &Sanitized<Snapshot>) -> AppResult<i64> {
    let snapshot = snapshot.get();
    conn.prepare_cached(
        "INSERT INTO snapshots (collector_id, entity_kind, entity_external_id, entity_name, \
         state_json, captured_at, checkpoint_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .during("prepare a snapshot insert")?
    .insert((
        &snapshot.collector_id,
        &snapshot.entity_kind,
        &snapshot.entity_external_id,
        &snapshot.entity_name,
        to_json(&snapshot.state)?,
        snapshot.captured_at,
        snapshot.checkpoint_id,
    ))
    .during("store a snapshot")
}

/// Every snapshot a checkpoint owns.
pub fn by_checkpoint(conn: &Connection, checkpoint_id: i64) -> AppResult<Vec<(i64, Snapshot)>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT id, collector_id, entity_kind, entity_external_id, entity_name, state_json, \
             captured_at, checkpoint_id FROM snapshots WHERE checkpoint_id = ?1 ORDER BY id",
        )
        .during("prepare a snapshot query")?;
    let rows = statement
        .query_map([checkpoint_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, Option<i64>>(7)?,
            ))
        })
        .during("read snapshots")?;
    let mut out = Vec::new();
    for row in rows {
        let (
            id,
            collector_id,
            entity_kind,
            entity_external_id,
            entity_name,
            state,
            captured_at,
            owner,
        ) = row.during("read snapshots")?;
        out.push((
            id,
            Snapshot {
                collector_id,
                entity_kind,
                entity_external_id,
                entity_name,
                state: from_json("state_json", &state)?,
                captured_at,
                checkpoint_id: owner,
            },
        ));
    }
    Ok(out)
}

/// Delete up to `batch` snapshots captured before `cutoff` that no
/// checkpoint owns. Returns how many went.
pub fn delete_expired(conn: &Connection, cutoff: i64, batch: usize) -> AppResult<usize> {
    conn.prepare_cached(
        "DELETE FROM snapshots WHERE id IN (\
            SELECT id FROM snapshots WHERE checkpoint_id IS NULL AND captured_at < ?1 LIMIT ?2\
        )",
    )
    .during("prepare snapshot retention")?
    .execute((cutoff, i64::try_from(batch).unwrap_or(i64::MAX)))
    .during("delete expired snapshots")
}

/// Total snapshots.
pub fn count(conn: &Connection) -> AppResult<u64> {
    conn.prepare_cached("SELECT COUNT(*) FROM snapshots")
        .during("prepare a count")?
        .query_row([], |row| count_column(row, 0))
        .during("count snapshots")
}
