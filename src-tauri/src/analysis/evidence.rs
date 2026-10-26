//! Persisted Windows Event Log corroboration.
//!
//! Callers ask the local history database rather than re-reading a live log.
//! A match is evidence near a change, never proof that the log record caused
//! that change.

use serde_json::{json, Value};

use crate::domain::{Category, EventType, ExitKind, SystemEvent};
use crate::error::{AppError, AppResult};
use crate::storage::{event_repository, Db};

const MAX_EVIDENCE_ROWS: usize = 100;
const PROCESS_EVIDENCE_RADIUS_MS: i64 = 15_000;
const UPDATE_EVIDENCE_RADIUS_MS: i64 = 15 * 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceWindow {
    pub from_ms: i64,
    pub to_ms: i64,
}

impl EvidenceWindow {
    pub fn around(timestamp_ms: i64, radius_ms: i64) -> Self {
        let radius_ms = radius_ms.max(0);
        Self {
            from_ms: timestamp_ms.saturating_sub(radius_ms),
            to_ms: timestamp_ms.saturating_add(radius_ms),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEvidence {
    pub stored_event_id: i64,
    pub timestamp_ms: i64,
    pub provider: String,
    pub event_id: u32,
    pub message: String,
    pub count: u64,
}

/// Find persisted Event Log rows whose provider or `<Data>` value names an
/// image/service inside the requested window.
pub fn find_log_evidence(
    db: &Db,
    image_or_service: &str,
    window: EvidenceWindow,
) -> AppResult<Vec<LogEvidence>> {
    find_log_evidence_kind(
        db,
        image_or_service,
        window,
        event_repository::EvidenceSubjectKind::Any,
    )
}

fn find_log_evidence_kind(
    db: &Db,
    image_or_service: &str,
    window: EvidenceWindow,
    kind: event_repository::EvidenceSubjectKind,
) -> AppResult<Vec<LogEvidence>> {
    let needle = image_or_service.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Err(AppError::InvalidInput {
            what: "event-log evidence subject",
            reason: "an image or service name is required".to_owned(),
        });
    }
    if window.from_ms > window.to_ms {
        return Err(AppError::InvalidInput {
            what: "event-log evidence window",
            reason: "the start must not be after the end".to_owned(),
        });
    }

    db.read(|connection| {
        let rows = event_repository::log_evidence_candidates(
            connection,
            &needle,
            kind,
            window.from_ms,
            window.to_ms,
            MAX_EVIDENCE_ROWS,
        )?;
        let mut evidence = Vec::new();
        for (stored_event_id, timestamp_ms, metadata) in rows {
            let value: Value =
                serde_json::from_str(&metadata).map_err(|_| crate::error::DbError::Decoding {
                    reason: "column metadata_json is not valid JSON".to_owned(),
                })?;
            let event_id = value
                .get("event_id")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(0);
            evidence.push(LogEvidence {
                stored_event_id,
                timestamp_ms,
                provider: string_field(&value, "provider"),
                event_id,
                message: string_field(&value, "message"),
                count: value.get("count").and_then(Value::as_u64).unwrap_or(1),
            });
        }
        Ok(evidence)
    })
}

/// Re-evaluate a stored event when it is opened. This closes the ordering race
/// where the process/update collector commits just before the matching Event
/// Log callback reaches SQLite; the user-facing detail becomes corroborated as
/// soon as the evidence exists, without ever guessing in its absence.
pub fn corroborate_stored_event(db: &Db, stored: &mut SystemEvent) {
    let event = &mut stored.event;
    if event.source_collector == "process"
        && event.category == Category::Process
        && event.event_type == EventType::Stopped
    {
        let evidence = evidence_or_empty(
            db,
            &event.entity_display_name,
            EvidenceWindow::around(event.timestamp, PROCESS_EVIDENCE_RADIUS_MS),
        )
        .into_iter()
        .filter(|item| matches!(item.event_id, 1000 | 1001 | 1002 | 1026))
        .collect::<Vec<_>>();
        if !evidence.is_empty() {
            if !event.metadata.as_ref().is_some_and(Value::is_object) {
                event.metadata = Some(json!({}));
            }
            if let Some(details) = event.metadata.as_mut().and_then(Value::as_object_mut) {
                details.insert(
                    "exit_kind".to_owned(),
                    Value::String(ExitKind::Unexpected.as_str().to_owned()),
                );
                details.insert("log_evidence".to_owned(), evidence_json(&evidence));
            }
        }
    } else if event.source_collector == "updates" && event.category == Category::Update {
        let evidence = evidence_or_empty_kind(
            db,
            &event.entity_display_name,
            EvidenceWindow::around(event.timestamp, UPDATE_EVIDENCE_RADIUS_MS),
            event_repository::EvidenceSubjectKind::Update,
        );
        if !evidence.is_empty() {
            if !event.metadata.as_ref().is_some_and(Value::is_object) {
                event.metadata = Some(json!({}));
            }
            if let Some(details) = event.metadata.as_mut().and_then(Value::as_object_mut) {
                details.insert("log_evidence".to_owned(), evidence_json(&evidence));
            }
        }
    }
}

fn evidence_or_empty(db: &Db, subject: &str, window: EvidenceWindow) -> Vec<LogEvidence> {
    evidence_or_empty_kind(
        db,
        subject,
        window,
        event_repository::EvidenceSubjectKind::Any,
    )
}

fn evidence_or_empty_kind(
    db: &Db,
    subject: &str,
    window: EvidenceWindow,
    kind: event_repository::EvidenceSubjectKind,
) -> Vec<LogEvidence> {
    match find_log_evidence_kind(db, subject, window, kind) {
        Ok(evidence) => evidence,
        Err(error) => {
            tracing::debug!(%error, "event-log corroboration query failed");
            Vec::new()
        }
    }
}

fn evidence_json(evidence: &[LogEvidence]) -> Value {
    Value::Array(
        evidence
            .iter()
            .map(|item| {
                json!({
                    "stored_event_id": item.stored_event_id,
                    "timestamp_ms": item.timestamp_ms,
                    "provider": item.provider,
                    "event_id": item.event_id,
                    "count": item.count,
                })
            })
            .collect(),
    )
}

fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}
