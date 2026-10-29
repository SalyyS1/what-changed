//! Atomic persistence for deterministic investigations.
//!
//! `suspects_json` is deliberately a complete, sanitized presentation
//! snapshot. Reopening an investigation therefore does not depend on events
//! that may already have expired under the shorter event-retention policy.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{count_column, from_json, to_json, SqlContext};
use crate::error::{AppError, AppResult};
use crate::privacy::{Redaction, Sanitized, Sanitizer, Scrub};

/// Investigation history has its own fixed retention class.
pub const RETENTION_DAYS: i64 = 90;
/// A UI history page is intentionally small and bounded.
pub const MAX_LIST_ROWS: u16 = 100;

/// One persisted score row. JSON fields retain the exact inputs that produced
/// the rank while `suspects_json` retains the immutable user-facing snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisResultRecord {
    pub event_id: Option<i64>,
    pub rank: u32,
    pub score: f64,
    pub score_terms: Value,
    pub confidence_class: String,
    pub chain: Option<Value>,
}

/// One analysis execution belonging to an investigation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisSessionRecord {
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub weights: Value,
    pub events_considered: u64,
    /// Number of events omitted by deterministic downsampling.
    pub downsampled: u64,
    pub results: Vec<AnalysisResultRecord>,
}

/// Complete write model. Callers must pass it through [`Sanitizer`] before it
/// reaches [`create`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvestigationPersistence {
    pub problem: String,
    pub window: Value,
    pub resolved_from: i64,
    pub resolved_to: i64,
    /// Complete presentation payload, not only event ids. It remains useful
    /// after the referenced events have expired.
    pub suspects: Value,
    pub total_events_in_window: u64,
    pub created_at: i64,
    pub session: AnalysisSessionRecord,
}

impl Scrub for InvestigationPersistence {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        let path = |name: &str| {
            if field.is_empty() {
                name.to_owned()
            } else {
                format!("{field}.{name}")
            }
        };
        self.problem
            .scrub_in_place(sanitizer, &path("problem"), report);
        self.window
            .scrub_in_place(sanitizer, &path("window"), report);
        self.suspects
            .scrub_in_place(sanitizer, &path("suspects"), report);
        self.session
            .weights
            .scrub_in_place(sanitizer, &path("session.weights"), report);
        for (index, result) in self.session.results.iter_mut().enumerate() {
            result.score_terms.scrub_in_place(
                sanitizer,
                &path(&format!("session.results[{index}].score_terms")),
                report,
            );
            result.chain.scrub_in_place(
                sanitizer,
                &path(&format!("session.results[{index}].chain")),
                report,
            );
            result.confidence_class.scrub_in_place(
                sanitizer,
                &path(&format!("session.results[{index}].confidence_class")),
                report,
            );
        }
    }
}

/// Stored row plus its audit session and score terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredInvestigation {
    pub id: i64,
    #[serde(flatten)]
    pub investigation: InvestigationPersistence,
    pub session_id: i64,
}

/// Bounded history row. The immutable presentation is included so the list
/// remains independent of event retention; score audit rows stay in `by_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvestigationListItem {
    pub id: i64,
    pub problem: String,
    pub window: Value,
    pub resolved_from: i64,
    pub resolved_to: i64,
    pub suspects: Value,
    pub total_events_in_window: u64,
    pub created_at: i64,
}

/// Store investigation, session, result rows, and immutable presentation in a
/// single writer transaction.
pub fn create(
    conn: &Connection,
    record: &Sanitized<InvestigationPersistence>,
) -> AppResult<StoredInvestigation> {
    validate(record.get())?;
    let tx = conn
        .unchecked_transaction()
        .during("start an investigation transaction")?;
    let value = record.get();
    let investigation_id = tx
        .prepare_cached(
            "INSERT INTO investigations (problem, window_json, resolved_from, resolved_to, \
             suspects_json, total_events_in_window, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .during("prepare an investigation insert")?
        .insert((
            &value.problem,
            to_json(&value.window)?,
            value.resolved_from,
            value.resolved_to,
            to_json(&value.suspects)?,
            i64::try_from(value.total_events_in_window).unwrap_or(i64::MAX),
            value.created_at,
        ))
        .during("store an investigation")?;
    let session_id = tx
        .prepare_cached(
            "INSERT INTO analysis_sessions (investigation_id, started_at, finished_at, weights_json, \
             events_considered, downsampled) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .during("prepare an analysis session insert")?
        .insert((
            investigation_id,
            value.session.started_at,
            value.session.finished_at,
            to_json(&value.session.weights)?,
            i64::try_from(value.session.events_considered).unwrap_or(i64::MAX),
            i64::try_from(value.session.downsampled).unwrap_or(i64::MAX),
        ))
        .during("store an analysis session")?;
    {
        let mut insert = tx
            .prepare_cached(
                "INSERT INTO analysis_results (session_id, event_id, rank, score, score_terms_json, \
                 confidence_class, chain_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .during("prepare analysis result inserts")?;
        for result in &value.session.results {
            insert
                .execute((
                    session_id,
                    result.event_id,
                    i64::from(result.rank),
                    result.score,
                    to_json(&result.score_terms)?,
                    &result.confidence_class,
                    result.chain.as_ref().map(to_json).transpose()?,
                ))
                .during("store an analysis result")?;
        }
    }
    tx.commit().during("commit an investigation transaction")?;
    Ok(StoredInvestigation {
        id: investigation_id,
        investigation: value.clone(),
        session_id,
    })
}

fn validate(record: &InvestigationPersistence) -> AppResult<()> {
    if record.problem.trim().is_empty() {
        return invalid("investigation problem", "must not be empty");
    }
    if record.resolved_from > record.resolved_to {
        return invalid("investigation window", "start must not be after end");
    }
    let Some(finished_at) = record.session.finished_at else {
        return invalid(
            "analysis session",
            "completed investigation needs a finish time",
        );
    };
    if finished_at < record.session.started_at {
        return invalid("analysis session", "finish must not precede start");
    }
    if !record.suspects.is_object() {
        return invalid(
            "investigation presentation",
            "must be a complete JSON object",
        );
    }
    for (index, result) in record.session.results.iter().enumerate() {
        let expected = u32::try_from(index + 1).unwrap_or(u32::MAX);
        if result.rank != expected {
            return invalid("analysis results", "ranks must be consecutive from one");
        }
        if !result.score.is_finite() {
            return invalid("analysis result score", "must be finite");
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

/// Reopen one investigation without consulting retained events.
pub fn by_id(conn: &Connection, investigation_id: i64) -> AppResult<Option<StoredInvestigation>> {
    let row = conn
        .prepare_cached(
            "SELECT i.id, i.problem, i.window_json, i.resolved_from, i.resolved_to, \
             i.suspects_json, i.total_events_in_window, i.created_at, s.id, s.started_at, \
             s.finished_at, s.weights_json, s.events_considered, s.downsampled \
             FROM investigations i JOIN analysis_sessions s ON s.investigation_id = i.id \
             WHERE i.id = ?1 ORDER BY s.id DESC LIMIT 1",
        )
        .during("prepare an investigation lookup")?
        .query_row([investigation_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, String>(11)?,
                row.get::<_, i64>(12)?,
                row.get::<_, i64>(13)?,
            ))
        })
        .optional()
        .during("read an investigation")?;
    let Some(raw) = row else {
        return Ok(None);
    };
    let results = results_for_session(conn, raw.8)?;
    Ok(Some(StoredInvestigation {
        id: raw.0,
        investigation: InvestigationPersistence {
            problem: raw.1,
            window: from_json("window_json", &raw.2)?,
            resolved_from: raw.3,
            resolved_to: raw.4,
            suspects: from_json("suspects_json", &raw.5)?,
            total_events_in_window: u64::try_from(raw.6).unwrap_or(0),
            created_at: raw.7,
            session: AnalysisSessionRecord {
                started_at: raw.9,
                finished_at: raw.10,
                weights: from_json("weights_json", &raw.11)?,
                events_considered: u64::try_from(raw.12).unwrap_or(0),
                downsampled: u64::try_from(raw.13).unwrap_or(0),
                results,
            },
        },
        session_id: raw.8,
    }))
}

fn results_for_session(conn: &Connection, session_id: i64) -> AppResult<Vec<AnalysisResultRecord>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT event_id, rank, score, score_terms_json, confidence_class, chain_json \
             FROM analysis_results WHERE session_id = ?1 ORDER BY rank, id",
        )
        .during("prepare analysis results lookup")?;
    let rows = statement
        .query_map([session_id], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .during("read analysis results")?;
    let mut results = Vec::new();
    for row in rows {
        let raw = row.during("read an analysis result")?;
        results.push(AnalysisResultRecord {
            event_id: raw.0,
            rank: u32::try_from(raw.1).unwrap_or(0),
            score: raw.2,
            score_terms: from_json("score_terms_json", &raw.3)?,
            confidence_class: raw.4,
            chain: raw
                .5
                .as_deref()
                .map(|text| from_json("chain_json", text))
                .transpose()?,
        });
    }
    Ok(results)
}

/// Newest investigation ids for the history screen. Detail is loaded through
/// [`by_id`], keeping the list query one indexed, bounded read.
pub fn list_ids(conn: &Connection, limit: u16) -> AppResult<Vec<i64>> {
    let limit = limit.clamp(1, MAX_LIST_ROWS);
    let mut statement = conn
        .prepare_cached("SELECT id FROM investigations ORDER BY created_at DESC, id DESC LIMIT ?1")
        .during("prepare investigation history")?;
    let rows = statement
        .query_map([i64::from(limit)], |row| row.get::<_, i64>(0))
        .during("read investigation history")?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .during("read investigation history")
}

/// Newest investigations, loaded by one indexed bounded query.
pub fn list(conn: &Connection, limit: u16) -> AppResult<Vec<InvestigationListItem>> {
    let limit = limit.clamp(1, MAX_LIST_ROWS);
    let mut statement = conn
        .prepare_cached(
            "SELECT id, problem, window_json, resolved_from, resolved_to, suspects_json, \
             total_events_in_window, created_at FROM investigations \
             ORDER BY created_at DESC, id DESC LIMIT ?1",
        )
        .during("prepare investigation list")?;
    let rows = statement
        .query_map([i64::from(limit)], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })
        .during("read investigation list")?;
    let mut investigations = Vec::new();
    for row in rows {
        let raw = row.during("read an investigation list row")?;
        investigations.push(InvestigationListItem {
            id: raw.0,
            problem: raw.1,
            window: from_json("window_json", &raw.2)?,
            resolved_from: raw.3,
            resolved_to: raw.4,
            suspects: from_json("suspects_json", &raw.5)?,
            total_events_in_window: u64::try_from(raw.6).unwrap_or(0),
            created_at: raw.7,
        });
    }
    Ok(investigations)
}

/// Explicit user deletion. Sessions and result audit rows cascade.
pub fn delete(conn: &Connection, investigation_id: i64) -> AppResult<bool> {
    Ok(conn
        .prepare_cached("DELETE FROM investigations WHERE id = ?1")
        .during("prepare investigation deletion")?
        .execute([investigation_id])
        .during("delete investigation")?
        > 0)
}

/// Delete a bounded batch older than `cutoff`.
pub fn delete_expired(conn: &Connection, cutoff: i64, batch: usize) -> AppResult<usize> {
    conn.prepare_cached(
        "DELETE FROM investigations WHERE id IN (\
         SELECT id FROM investigations WHERE created_at < ?1 \
         ORDER BY created_at, id LIMIT ?2)",
    )
    .during("prepare investigation retention")?
    .execute((cutoff, i64::try_from(batch).unwrap_or(i64::MAX)))
    .during("delete expired investigations")
}

/// Total retained investigation rows.
pub fn count(conn: &Connection) -> AppResult<u64> {
    conn.prepare_cached("SELECT COUNT(*) FROM investigations")
        .during("prepare an investigation count")?
        .query_row([], |row| count_column(row, 0))
        .during("count investigations")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::domain::{RetentionPolicy, DAY_MS};
    use crate::privacy::REDACTED;
    use crate::storage::{maintenance, Db};
    use crate::testing::temp_dir;
    use serde_json::json;

    fn record(created_at: i64) -> InvestigationPersistence {
        InvestigationPersistence {
            problem: "API token sk-abcdefghijklmnopqrstuvwx stopped working".to_owned(),
            window: json!({ "kind": "today" }),
            resolved_from: 10,
            resolved_to: 20,
            suspects: json!({
                "cards": [{ "title": "DB_PASSWORD changed", "DB_PASSWORD": "hunter2" }],
                "no_cause_reason": null
            }),
            total_events_in_window: 9,
            created_at,
            session: AnalysisSessionRecord {
                started_at: created_at,
                finished_at: Some(created_at + 2),
                weights: json!({ "temporal": 1.0 }),
                events_considered: 9,
                downsampled: 0,
                results: vec![AnalysisResultRecord {
                    event_id: Some(7),
                    rank: 1,
                    score: 4.25,
                    score_terms: json!({ "temporal": 0.5 }),
                    confidence_class: "likely".to_owned(),
                    chain: Some(json!({ "events": [7, 8] })),
                }],
            },
        }
    }

    #[test]
    fn create_is_sanitized_atomic_and_reopens_without_events() {
        let db = Db::open_in(&temp_dir("investigation-persistence")).unwrap();
        let (record, report) = Sanitizer::new().sanitize(record(100));
        assert!(!report.is_empty());
        let stored = db
            .write(move |conn| create(conn, &record))
            .expect("persist investigation");
        let reopened = db
            .read(|conn| by_id(conn, stored.id))
            .unwrap()
            .expect("reopen investigation");
        assert_eq!(reopened, stored);
        assert!(reopened.investigation.problem.contains(REDACTED));
        assert_eq!(
            reopened.investigation.suspects["cards"][0]["DB_PASSWORD"],
            json!(REDACTED)
        );
        assert_eq!(reopened.investigation.session.results.len(), 1);
        assert!(db.write(move |conn| delete(conn, stored.id)).unwrap());
        let audit_rows: i64 = db
            .read(|conn| {
                conn.query_row("SELECT COUNT(*) FROM analysis_results", [], |row| {
                    row.get(0)
                })
                .during("count analysis results")
            })
            .unwrap();
        assert_eq!(audit_rows, 0, "session and results cascade");
    }

    #[test]
    fn validation_rolls_back_the_whole_investigation() {
        let db = Db::open_in(&temp_dir("investigation-rollback")).unwrap();
        let mut invalid_record = record(100);
        invalid_record.session.results[0].rank = 2;
        let sanitized = Sanitizer::new().sanitize(invalid_record).0;
        assert!(db.write(move |conn| create(conn, &sanitized)).is_err());
        assert_eq!(db.read(count).unwrap(), 0);
    }

    #[test]
    fn a_result_insert_failure_rolls_back_metadata_session_and_results() {
        let db = Db::open_in(&temp_dir("investigation-atomic-rollback")).unwrap();
        db.write(|conn| {
            conn.execute_batch(
                "CREATE TRIGGER reject_second_result BEFORE INSERT ON analysis_results \
                 WHEN NEW.rank = 2 BEGIN SELECT RAISE(ABORT, 'scripted failure'); END;",
            )
            .during("install scripted investigation failure")
        })
        .unwrap();
        let mut failed = record(100);
        let mut second = failed.session.results[0].clone();
        second.rank = 2;
        second.event_id = Some(8);
        failed.session.results.push(second);
        let sanitized = Sanitizer::new().sanitize(failed).0;
        assert!(db.write(move |conn| create(conn, &sanitized)).is_err());
        let counts: (i64, i64, i64) = db
            .read(|conn| {
                conn.query_row(
                    "SELECT (SELECT COUNT(*) FROM investigations), \
                     (SELECT COUNT(*) FROM analysis_sessions), \
                     (SELECT COUNT(*) FROM analysis_results)",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .during("count rolled back investigation rows")
            })
            .unwrap();
        assert_eq!(counts, (0, 0, 0));
    }

    #[test]
    fn maintenance_keeps_ninety_days_of_investigation_history() {
        let db = Db::open_in(&temp_dir("investigation-retention")).unwrap();
        let now = 200 * DAY_MS;
        for created_at in [now - 89 * DAY_MS, now - 91 * DAY_MS] {
            let sanitized = Sanitizer::new().sanitize(record(created_at)).0;
            db.write(move |conn| create(conn, &sanitized)).unwrap();
        }
        let report = maintenance::run_pass(&db, RetentionPolicy::default(), now).unwrap();
        assert_eq!(report.investigations_deleted, 1);
        let history = db.read(|conn| list(conn, 10)).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].created_at, now - 89 * DAY_MS);
    }
}
