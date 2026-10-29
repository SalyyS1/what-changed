//! Atomic checkpoint persistence and bounded comparison reads.
//!
//! Checkpoint items are checkpoint-owned rows in `snapshots`: this keeps the
//! original retention exemption and cascade contract as the single source of
//! truth. Every write accepts `Sanitized<CheckpointCapture>`, so state JSON
//! cannot bypass the privacy chokepoint.

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, Transaction};

use super::{count_column, from_json, parse_word, to_json, SqlContext};
use crate::domain::{Checkpoint, CheckpointCapture, CheckpointItem, CheckpointKind, NewCheckpoint};
use crate::dto::CheckpointRef;
use crate::error::{AppError, AppResult, DbError};
use crate::privacy::Sanitized;

/// Hard IPC/read bound for timeline links returned with one comparison.
pub const MAX_EVENT_IDS_IN_WINDOW: usize = 10_000;
/// Automatic checkpoints retained after a successful automatic capture.
pub const DEFAULT_AUTOMATIC_CHECKPOINT_CAP: usize = 10;

/// Store metadata and all item rows in one SQLite transaction. The same
/// prepared statement is reused for the complete bulk insert.
pub fn create(conn: &Connection, capture: &Sanitized<CheckpointCapture>) -> AppResult<Checkpoint> {
    validate_capture(capture.get())?;
    let transaction = conn
        .unchecked_transaction()
        .during("start a checkpoint transaction")?;
    let checkpoint_id = insert_checkpoint(&transaction, &capture.get().checkpoint)?;
    insert_items(
        &transaction,
        checkpoint_id,
        capture.get().checkpoint.created_at,
        &capture.get().items,
    )?;
    if capture.get().checkpoint.kind == CheckpointKind::Automatic {
        prune_automatic_in_transaction(&transaction, DEFAULT_AUTOMATIC_CHECKPOINT_CAP)?;
    }
    transaction
        .commit()
        .during("commit a checkpoint transaction")?;
    by_id(conn, checkpoint_id)?.ok_or_else(|| {
        AppError::Db(DbError::Decoding {
            reason: "the checkpoint committed but could not be read".to_owned(),
        })
    })
}

fn validate_capture(capture: &CheckpointCapture) -> AppResult<()> {
    if capture.checkpoint.name.trim().is_empty() {
        return invalid("checkpoint name", "must not be empty");
    }
    let captured: BTreeSet<&str> = capture
        .checkpoint
        .captured_collectors
        .iter()
        .map(String::as_str)
        .collect();
    let missing: BTreeSet<&str> = capture
        .checkpoint
        .missing_collectors
        .iter()
        .map(String::as_str)
        .collect();
    if captured.iter().any(|collector| missing.contains(collector)) {
        return invalid(
            "collector manifest",
            "a collector cannot be both captured and missing",
        );
    }
    let mut keys = BTreeSet::new();
    for item in &capture.items {
        if item.collector_id.is_empty()
            || item.entity_kind.is_empty()
            || item.entity_external_id.is_empty()
        {
            return invalid("checkpoint item", "identity fields must not be empty");
        }
        if !captured.contains(item.collector_id.as_str()) {
            return invalid(
                "checkpoint item",
                "its collector is absent from the captured manifest",
            );
        }
        if !keys.insert((
            item.collector_id.as_str(),
            item.entity_kind.as_str(),
            item.entity_external_id.as_str(),
        )) {
            return invalid("checkpoint item", "duplicate entity identity");
        }
    }
    Ok(())
}

fn invalid(what: &'static str, reason: &'static str) -> AppResult<()> {
    Err(AppError::InvalidInput {
        what,
        reason: reason.to_owned(),
    })
}

fn insert_checkpoint(transaction: &Transaction<'_>, row: &NewCheckpoint) -> AppResult<i64> {
    transaction
        .prepare_cached(
            "INSERT INTO checkpoints (name, kind, reason, created_at, item_count, duration_ms, \
             partial, missing_collectors, captured_collectors) \
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8)",
        )
        .during("prepare a checkpoint insert")?
        .insert((
            &row.name,
            row.kind.as_str(),
            &row.reason,
            row.created_at,
            i64::try_from(row.duration_ms).unwrap_or(i64::MAX),
            !row.missing_collectors.is_empty(),
            to_json(&row.missing_collectors)?,
            to_json(&row.captured_collectors)?,
        ))
        .during("store a checkpoint")
}

fn insert_items(
    transaction: &Transaction<'_>,
    checkpoint_id: i64,
    captured_at: i64,
    items: &[CheckpointItem],
) -> AppResult<()> {
    {
        let mut statement = transaction
            .prepare_cached(
                "INSERT INTO snapshots (collector_id, entity_kind, entity_external_id, entity_name, \
                 state_json, captured_at, checkpoint_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .during("prepare checkpoint item inserts")?;
        for item in items {
            statement
                .execute((
                    &item.collector_id,
                    &item.entity_kind,
                    &item.entity_external_id,
                    &item.entity_name,
                    to_json(&item.state)?,
                    captured_at,
                    checkpoint_id,
                ))
                .during("store checkpoint items")?;
        }
    }
    transaction
        .execute(
            "UPDATE checkpoints SET item_count = ?1 WHERE id = ?2",
            (
                i64::try_from(items.len()).unwrap_or(i64::MAX),
                checkpoint_id,
            ),
        )
        .during("finish checkpoint item count")?;
    Ok(())
}

/// Every checkpoint, newest first.
pub fn list(conn: &Connection) -> AppResult<Vec<Checkpoint>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT id, name, kind, reason, created_at, item_count, duration_ms, partial, \
             missing_collectors, captured_collectors FROM checkpoints \
             ORDER BY created_at DESC, id DESC",
        )
        .during("prepare checkpoint list")?;
    let rows = statement
        .query_map([], raw_checkpoint)
        .during("list checkpoints")?;
    let mut checkpoints = Vec::new();
    for raw in rows {
        checkpoints.push(decode_checkpoint(raw.during("read checkpoint list")?)?);
    }
    Ok(checkpoints)
}

/// One checkpoint, if it still exists.
pub fn by_id(conn: &Connection, checkpoint_id: i64) -> AppResult<Option<Checkpoint>> {
    #[cfg(feature = "devtools")]
    let capture = super::checkpoint_compare_plan_capture::begin(
        super::checkpoint_compare_plan_capture::CheckpointCompareOperation::Metadata,
    );
    let sql = "SELECT id, name, kind, reason, created_at, item_count, duration_ms, partial, \
             missing_collectors, captured_collectors FROM checkpoints WHERE id = ?1";
    let parameters = [checkpoint_id];
    // Preserve the original cached-statement reset/release before domain decoding.
    let raw = {
        let mut statement = conn
            .prepare_cached(sql)
            .during("prepare checkpoint lookup")?;
        #[cfg(feature = "devtools")]
        if let Some(capture) = &capture {
            capture.prepared(conn, sql, &parameters, statement.parameter_count());
        }
        statement
            .query_row(parameters, |row| {
                #[cfg(feature = "devtools")]
                if let Some(capture) = &capture {
                    capture.started();
                }
                raw_checkpoint(row)
            })
            .optional()
            .during("read checkpoint")?
    };
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        // Successful zero-row optional lookup also stepped the original query.
        capture.started();
    }
    let output = raw.map(decode_checkpoint).transpose()?;
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        capture.completed(usize::from(output.is_some()));
    }
    Ok(output)
}

type RawCheckpoint = (
    i64,
    String,
    String,
    Option<String>,
    i64,
    i64,
    i64,
    bool,
    String,
    String,
);

fn raw_checkpoint(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawCheckpoint> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
    ))
}

fn decode_checkpoint(raw: RawCheckpoint) -> AppResult<Checkpoint> {
    Ok(Checkpoint {
        id: raw.0,
        name: raw.1,
        kind: parse_word("kind", &raw.2)?,
        reason: raw.3,
        created_at: raw.4,
        item_count: u64::try_from(raw.5).unwrap_or(0),
        duration_ms: u64::try_from(raw.6).unwrap_or(0),
        partial: raw.7,
        missing_collectors: from_json("missing_collectors", &raw.8)?,
        captured_collectors: from_json("captured_collectors", &raw.9)?,
    })
}

/// Semantic checkpoint items, loaded by one indexed query.
pub fn items(conn: &Connection, checkpoint_id: i64) -> AppResult<Vec<CheckpointItem>> {
    #[cfg(feature = "devtools")]
    let capture = super::checkpoint_compare_plan_capture::begin(
        super::checkpoint_compare_plan_capture::CheckpointCompareOperation::Items,
    );
    let sql = "SELECT collector_id, entity_kind, entity_external_id, entity_name, state_json \
             FROM snapshots WHERE checkpoint_id = ?1 \
             ORDER BY collector_id, entity_kind, entity_external_id";
    let parameters = [checkpoint_id];
    let mut statement = conn
        .prepare_cached(sql)
        .during("prepare checkpoint item query")?;
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        capture.prepared(conn, sql, &parameters, statement.parameter_count());
    }
    let rows = statement
        .query_map(parameters, |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .during("read checkpoint items")?;
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        capture.started();
    }
    let mut output = Vec::new();
    for row in rows {
        let (collector_id, entity_kind, entity_external_id, entity_name, state) =
            row.during("read checkpoint items")?;
        output.push(CheckpointItem {
            collector_id,
            entity_kind,
            entity_external_id,
            entity_name,
            state: from_json("state_json", &state)?,
        });
    }
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        capture.completed(output.len());
    }
    Ok(output)
}

/// Rename metadata; state rows are untouched. Returns `false` if absent.
pub fn rename(conn: &Connection, checkpoint_id: i64, name: &Sanitized<String>) -> AppResult<bool> {
    Ok(conn
        .prepare_cached("UPDATE checkpoints SET name = ?1 WHERE id = ?2")
        .during("prepare checkpoint rename")?
        .execute((name.get(), checkpoint_id))
        .during("rename checkpoint")?
        > 0)
}

/// Delete metadata and its checkpoint-owned snapshot rows via cascade.
pub fn delete(conn: &Connection, checkpoint_id: i64) -> AppResult<bool> {
    Ok(conn
        .prepare_cached("DELETE FROM checkpoints WHERE id = ?1")
        .during("prepare checkpoint delete")?
        .execute([checkpoint_id])
        .during("delete checkpoint")?
        > 0)
}

/// Keep only the newest `keep` automatic checkpoints. Manual rows are never
/// selected. `create` applies the default cap atomically to automatic inserts.
pub fn prune_automatic(conn: &Connection, keep: usize) -> AppResult<usize> {
    let transaction = conn
        .unchecked_transaction()
        .during("start automatic checkpoint pruning")?;
    let deleted = prune_automatic_in_transaction(&transaction, keep)?;
    transaction
        .commit()
        .during("commit automatic checkpoint pruning")?;
    Ok(deleted)
}

fn prune_automatic_in_transaction(transaction: &Transaction<'_>, keep: usize) -> AppResult<usize> {
    transaction
        .prepare_cached(
            "DELETE FROM checkpoints WHERE kind = 'automatic' AND id NOT IN (\
             SELECT id FROM checkpoints WHERE kind = 'automatic' \
             ORDER BY created_at DESC, id DESC LIMIT ?1)",
        )
        .during("prepare automatic checkpoint pruning")?
        .execute([i64::try_from(keep).unwrap_or(i64::MAX)])
        .during("prune automatic checkpoints")
}

/// Event ids in the inclusive structural-diff window. This is one indexed
/// query, ordered deterministically and hard-capped for IPC safety.
pub fn event_ids_in_window(conn: &Connection, from: i64, to: i64) -> AppResult<Vec<i64>> {
    #[cfg(feature = "devtools")]
    super::checkpoint_compare_plan_capture::window_invoked();
    #[cfg(feature = "devtools")]
    let capture = super::checkpoint_window_plan_capture::begin();
    let (start, end) = if from <= to { (from, to) } else { (to, from) };
    let sql = "SELECT id FROM events WHERE timestamp >= ?1 AND timestamp <= ?2 \
             ORDER BY timestamp, id LIMIT ?3";
    let parameters = (
        start,
        end,
        i64::try_from(MAX_EVENT_IDS_IN_WINDOW).unwrap_or(i64::MAX),
    );
    let mut statement = conn
        .prepare_cached(sql)
        .during("prepare checkpoint event window")?;
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        capture.prepared(conn, sql, parameters, statement.parameter_count());
    }
    let rows = statement
        .query_map(parameters, |row| row.get::<_, i64>(0))
        .during("read checkpoint event window")?;
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        capture.started();
    }
    let output = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .during("read checkpoint event window")?;
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        capture.completed(output.len());
    }
    Ok(output)
}

/// Most recently created known-good marker.
pub fn latest(conn: &Connection) -> AppResult<Option<CheckpointRef>> {
    conn.prepare_cached(
        "SELECT id, name, created_at FROM checkpoints \
         ORDER BY created_at DESC, id DESC LIMIT 1",
    )
    .during("prepare latest checkpoint")?
    .query_row([], |row| {
        Ok(CheckpointRef {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at: row.get(2)?,
        })
    })
    .optional()
    .during("read latest checkpoint")
}

/// Timestamp used to resolve an investigation's `SinceCheckpoint` window
/// without loading the checkpoint item set.
pub fn created_at_by_id(conn: &Connection, checkpoint_id: i64) -> AppResult<Option<i64>> {
    conn.prepare_cached("SELECT created_at FROM checkpoints WHERE id = ?1")
        .during("prepare checkpoint timestamp lookup")?
        .query_row([checkpoint_id], |row| row.get(0))
        .optional()
        .during("read checkpoint timestamp")
}

/// Checkpoint labels matching user text.
pub fn search_labels(conn: &Connection, text: &str, limit: u16) -> AppResult<Vec<CheckpointRef>> {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    let pattern = format!("%{escaped}%");
    let mut statement = conn
        .prepare_cached(
            "SELECT id, name, created_at FROM checkpoints \
             WHERE name LIKE ?1 ESCAPE '\\' \
             ORDER BY created_at DESC, id DESC LIMIT ?2",
        )
        .during("prepare checkpoint label search")?;
    let rows = statement
        .query_map((pattern, i64::from(limit)), |row| {
            Ok(CheckpointRef {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .during("search checkpoint labels")?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .during("read checkpoint label search")
}

/// Count semantic checkpoint items without touching unowned snapshots.
pub fn item_count(conn: &Connection) -> AppResult<u64> {
    conn.prepare_cached("SELECT COUNT(*) FROM snapshots WHERE checkpoint_id IS NOT NULL")
        .during("prepare checkpoint item count")?
        .query_row([], |row| count_column(row, 0))
        .during("count checkpoint items")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::privacy::Sanitizer;

    fn database() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", true).unwrap();
        conn.execute_batch(include_str!("../../migrations/001_initial.sql"))
            .unwrap();
        conn.execute_batch(include_str!("../../migrations/005_checkpoint_items.sql"))
            .unwrap();
        conn
    }

    fn capture(kind: CheckpointKind, created_at: i64) -> Sanitized<CheckpointCapture> {
        Sanitizer::new()
            .sanitize(CheckpointCapture {
                checkpoint: NewCheckpoint {
                    name: format!("Checkpoint {created_at}"),
                    kind,
                    reason: None,
                    created_at,
                    duration_ms: 5,
                    captured_collectors: vec!["services".to_owned()],
                    missing_collectors: vec![],
                },
                items: vec![CheckpointItem {
                    collector_id: "services".to_owned(),
                    entity_kind: "service".to_owned(),
                    entity_external_id: format!("service-{created_at}"),
                    entity_name: Some("Example".to_owned()),
                    state: json!({"state": "running"}),
                }],
            })
            .0
    }

    #[test]
    fn create_is_atomic_and_delete_cascades_items() {
        let conn = database();
        conn.execute_batch(
            "CREATE TRIGGER reject_checkpoint_item BEFORE INSERT ON snapshots \
             BEGIN SELECT RAISE(ABORT, 'test rejection'); END;",
        )
        .unwrap();
        assert!(create(&conn, &capture(CheckpointKind::Manual, 1)).is_err());
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM checkpoints", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);

        conn.execute_batch("DROP TRIGGER reject_checkpoint_item")
            .unwrap();
        let checkpoint = create(&conn, &capture(CheckpointKind::Manual, 2)).unwrap();
        assert_eq!(checkpoint.item_count, 1);
        assert_eq!(items(&conn, checkpoint.id).unwrap().len(), 1);
        assert!(delete(&conn, checkpoint.id).unwrap());
        assert_eq!(item_count(&conn).unwrap(), 0);
    }

    #[test]
    fn state_json_is_redacted_before_it_can_reach_storage() {
        let conn = database();
        let (capture, _) = Sanitizer::new().sanitize(CheckpointCapture {
            checkpoint: NewCheckpoint {
                name: "Safe".to_owned(),
                kind: CheckpointKind::Manual,
                reason: None,
                created_at: 1,
                duration_ms: 1,
                captured_collectors: vec!["services".to_owned()],
                missing_collectors: vec![],
            },
            items: vec![CheckpointItem {
                collector_id: "services".to_owned(),
                entity_kind: "service".to_owned(),
                entity_external_id: "example".to_owned(),
                entity_name: None,
                state: json!({"api_key": "sk-abcdefghijklmnopqrstuvwx"}),
            }],
        });
        create(&conn, &capture).unwrap();
        let stored: String = conn
            .query_row(
                "SELECT state_json FROM snapshots WHERE checkpoint_id IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!stored.contains("sk-"));
        assert!(stored.contains("[REDACTED]"));
    }

    #[test]
    fn eleventh_automatic_prunes_oldest_and_never_prunes_manual() {
        let conn = database();
        let manual = create(&conn, &capture(CheckpointKind::Manual, 0)).unwrap();
        for created_at in 1..=11 {
            create(&conn, &capture(CheckpointKind::Automatic, created_at)).unwrap();
        }
        let rows = list(&conn).unwrap();
        assert_eq!(
            rows.iter()
                .filter(|row| row.kind == CheckpointKind::Automatic)
                .count(),
            DEFAULT_AUTOMATIC_CHECKPOINT_CAP
        );
        assert!(rows.iter().any(|row| row.id == manual.id));
        assert!(!rows.iter().any(|row| row.created_at == 1));
    }

    #[test]
    fn event_window_is_inclusive_ordered_and_bounded_by_contract() {
        let conn = database();
        conn.execute_batch(
            "INSERT INTO events (id, timestamp, category, event_type, entity_type, entity_id, \
             entity_display_name, severity, importance, source_collector, related_entity_ids, \
             user_friendly_summary, technical_summary, confidence) VALUES \
             (1, 30, 'system', 'modified', 'system', 'system:a', 'A', 'info', 'informational', \
              'test', '[]', 'A', 'A', 'high'), \
             (2, 20, 'system', 'modified', 'system', 'system:b', 'B', 'info', 'informational', \
              'test', '[]', 'B', 'B', 'high');",
        )
        .unwrap();
        assert_eq!(event_ids_in_window(&conn, 30, 20).unwrap(), [2, 1]);
        assert_eq!(MAX_EVENT_IDS_IN_WINDOW, 10_000);
    }
}
