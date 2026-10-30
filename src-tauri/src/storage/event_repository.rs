//! The events table. The only place a row is written, paged, or expired.
//!
//! Writes take `Sanitized<NormalizedEvent>`, so an event that has not been
//! through redaction cannot be stored: it is a type error, not a review
//! comment. Every statement is prepared once and cached; every batch is one
//! transaction; paging is keyset so deep pages cost what shallow ones cost.

use rusqlite::{
    types::Value as SqlValue, CachedStatement, Connection, OptionalExtension, Row, Transaction,
};
use serde_json::Value;

use super::entity_repository;
use super::{count_column, from_json, parse_word, to_json, SqlContext};
use crate::domain::{
    Category, Cursor, EventQuery, EventSignature, EventSummary, Importance, NormalizedEvent, Page,
    SystemEvent,
};
use crate::error::AppResult;
use crate::privacy::Sanitized;

#[cfg(feature = "devtools")]
#[path = "insertion_timing_capture.rs"]
mod insertion_timing_capture;
#[cfg(feature = "devtools")]
pub use insertion_timing_capture::{
    BatchInsertionTimings, BodyInsertionTimings, CapturedBatchInsertion,
    CapturedPreservingInsertion, InsertionBodyOutcome, PreservingEntityInsertion,
};

#[cfg(feature = "devtools")]
#[path = "recurrence_plan_capture.rs"]
mod recurrence_plan_capture;
#[cfg(feature = "devtools")]
pub use recurrence_plan_capture::{
    capture_sorted_recurrence_plan, CapturedRecurrence, SortedRecurrenceBoundPlan,
    SortedRecurrenceCaptureError, SortedRecurrencePlan, SortedRecurrenceWindowStatistics,
};
#[cfg(feature = "devtools")]
pub use recurrence_rolling_counts::SortedRecurrenceCountKind;

#[path = "recurrence_rolling_counts.rs"]
mod recurrence_rolling_counts;

#[cfg(test)]
#[path = "recurrence_incremental_prototype.rs"]
mod recurrence_incremental_prototype;

/// Maximum full events handed to one deterministic analysis run.
pub const MAX_ANALYSIS_EVENTS: usize = 5_000;
pub const MAX_ANALYSIS_BOUNDARY_EVENTS: usize = 200;

const FULL_EVENT_COLUMNS: &str = "id, timestamp, category, event_type, entity_type, entity_id, \
    entity_display_name, severity, importance, source_collector, before_json, after_json, \
    metadata_json, related_entity_ids, user_friendly_summary, technical_summary, confidence, \
    sensitive, occurred_at_is_uncertain, field";

/// One bounded event-window read plus the exact count before downsampling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisEventWindow {
    pub events: Vec<SystemEvent>,
    pub total: u64,
    pub downsampled: u64,
}

/// Observation coverage and immediate rows outside a requested range. The
/// analysis layer uses this to distinguish empty history from a too-narrow or
/// pre-observation window without allocating another event page.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WindowBoundaryContext {
    pub coverage_from: Option<i64>,
    pub coverage_to: Option<i64>,
    pub immediately_before: Option<(i64, i64)>,
    pub immediately_after: Option<(i64, i64)>,
}

/// Input for the batched 90-day recurrence calculation.
#[derive(Debug, Clone)]
pub struct PriorOccurrenceQuery {
    pub event_id: i64,
    pub signature: EventSignature,
    pub since: i64,
    pub before: i64,
}

/// Insert a batch in one transaction and return the ids in order.
///
/// Also records or refreshes the entity each event is about, so the
/// relationship graph always has a node for anything an event mentions.
pub fn insert_batch(
    conn: &Connection,
    events: &[Sanitized<NormalizedEvent>],
) -> AppResult<Vec<i64>> {
    insert_batch_with_entity_policy(
        conn,
        events,
        None,
        #[cfg(feature = "devtools")]
        None,
    )
}

/// Devtools seeding keeps a real entity unchanged when a generated event id
/// happens to name it. The event still follows the regular sanitized insert
/// and reconciliation transaction; only the implicit entity touch differs.
#[cfg(feature = "devtools")]
pub fn insert_batch_preserving_entities(
    conn: &Connection,
    events: &[Sanitized<NormalizedEvent>],
    mut owned_entities: std::collections::HashSet<String>,
) -> AppResult<(Vec<i64>, std::collections::HashSet<String>)> {
    let ids = insert_batch_with_entity_policy(conn, events, Some(&mut owned_entities), None)?;
    Ok((ids, owned_entities))
}

/// Explicit devtools attribution of one repository call, created on the actual
/// writer thread. Does not time queue admission or introduce capture state.
#[cfg(feature = "devtools")]
pub fn insert_batch_preserving_entities_with_timings(
    conn: &Connection,
    events: &[Sanitized<NormalizedEvent>],
    mut owned_entities: std::collections::HashSet<String>,
) -> CapturedPreservingInsertion {
    let mut recorder = insertion_timing_capture::Recorder::start(events.len());
    let value = insert_batch_with_entity_policy(
        conn,
        events,
        Some(&mut owned_entities),
        Some(&mut recorder),
    )
    .map(|ids| (ids, owned_entities));
    let timings = recorder.finish(value.is_ok());
    CapturedBatchInsertion { value, timings }
}

fn insert_batch_with_entity_policy(
    conn: &Connection,
    events: &[Sanitized<NormalizedEvent>],
    owned_entities: Option<&mut std::collections::HashSet<String>>,
    #[cfg(feature = "devtools")] mut attribution: Option<&mut insertion_timing_capture::Recorder>,
) -> AppResult<Vec<i64>> {
    if events.is_empty() {
        return Ok(Vec::new());
    }
    let tx = conn
        .unchecked_transaction()
        .during("begin an insert transaction")?;
    let ids = insert_batch_body(
        &tx,
        events,
        owned_entities,
        #[cfg(feature = "devtools")]
        attribution.as_deref_mut(),
    )?;
    #[cfg(feature = "devtools")]
    if let Some(attribution) = &mut attribution {
        attribution.advance(insertion_timing_capture::Phase::Commit);
    }
    tx.commit().during("commit an insert transaction")?;
    #[cfg(feature = "devtools")]
    if let Some(attribution) = &mut attribution {
        attribution.advance(insertion_timing_capture::Phase::Complete);
    }
    Ok(ids)
}

/// Inserts into an existing devtools transaction without committing or rolling
/// it back. Returned ownership is provisional until the caller commits. On any
/// error the caller must abandon the entire transaction: partial body work may
/// remain pending, and no provisional ownership is returned.
///
/// Attribution describes only this body, excluding outer begin/commit/rollback.
/// Disabled attribution and empty input perform no attribution clock reads.
#[cfg(feature = "devtools")]
pub fn insert_batch_preserving_entities_in_transaction(
    tx: &Transaction<'_>,
    events: &[Sanitized<NormalizedEvent>],
    mut owned_entities: std::collections::HashSet<String>,
    capture: bool,
) -> InsertionBodyOutcome {
    if events.is_empty() {
        return InsertionBodyOutcome {
            value: Ok((Vec::new(), owned_entities)),
            timings: capture.then_some(BodyInsertionTimings {
                bodies: 1,
                empty_bodies: 1,
                ..BodyInsertionTimings::default()
            }),
        };
    }
    let mut recorder = capture.then(|| insertion_timing_capture::Recorder::start(events.len()));
    let value = insert_batch_body(tx, events, Some(&mut owned_entities), recorder.as_mut())
        .map(|ids| (ids, owned_entities));
    let timings = recorder.map(|recorder| recorder.finish_body(value.is_ok()));
    InsertionBodyOutcome { value, timings }
}

fn insert_batch_body(
    tx: &Transaction<'_>,
    events: &[Sanitized<NormalizedEvent>],
    mut owned_entities: Option<&mut std::collections::HashSet<String>>,
    #[cfg(feature = "devtools")] mut attribution: Option<&mut insertion_timing_capture::Recorder>,
) -> AppResult<Vec<i64>> {
    let mut ids = Vec::with_capacity(events.len());
    {
        let mut insert = tx
            .prepare_cached(EVENT_INSERT_SQL)
            .during("prepare the event insert")?;
        #[cfg(feature = "devtools")]
        if let Some(attribution) = &mut attribution {
            attribution.advance(insertion_timing_capture::Phase::EventEntityLoop);
        }
        for sanitized in events {
            let id = if let Some(existing_id) = merge_eventlog_duplicate(tx, sanitized.get())? {
                existing_id
            } else {
                insert_prepared(&mut insert, sanitized)?
            };
            ids.push(id);
            if let Some(owned) = owned_entities.as_deref_mut() {
                let entity_id = &sanitized.get().entity_id;
                if owned.contains(entity_id) {
                    entity_repository::touch(tx, sanitized.get())?;
                } else if entity_repository::by_id(tx, entity_id)?.is_none() {
                    entity_repository::touch(tx, sanitized.get())?;
                    owned.insert(entity_id.clone());
                }
            } else {
                entity_repository::touch(tx, sanitized.get())?;
            }
        }
    }
    #[cfg(feature = "devtools")]
    if let Some(attribution) = &mut attribution {
        attribution.advance(insertion_timing_capture::Phase::Reconciliation);
    }
    for (sanitized, stored_id) in events.iter().zip(&ids) {
        reconcile_inserted_event(tx, sanitized.get(), *stored_id)?;
    }
    Ok(ids)
}

fn corroborate_subject_from_existing_logs(
    conn: &Connection,
    subject: &NormalizedEvent,
    subject_id: i64,
) -> AppResult<()> {
    let (radius, crash_only) = match subject.source_collector.as_str() {
        "process" if subject.event_type == crate::domain::EventType::Stopped => (15_000, true),
        "updates" => (15 * 60_000, false),
        _ => return Ok(()),
    };
    let kind = if crash_only {
        EvidenceSubjectKind::Process
    } else {
        EvidenceSubjectKind::Update
    };
    let mut candidates = log_evidence_candidates(
        conn,
        &subject.entity_display_name.to_ascii_lowercase(),
        kind,
        subject.timestamp.saturating_sub(radius),
        subject.timestamp.saturating_add(radius),
        100,
    )?;
    if subject.source_collector == "updates" {
        if let Some(update_id) = subject
            .metadata
            .as_ref()
            .and_then(|details| details.get("update_id"))
            .and_then(Value::as_str)
        {
            candidates.extend(log_evidence_candidates(
                conn,
                &update_id.to_ascii_lowercase(),
                EvidenceSubjectKind::Update,
                subject.timestamp.saturating_sub(radius),
                subject.timestamp.saturating_add(radius),
                100,
            )?);
        }
    }
    candidates.sort_by_key(|candidate| candidate.0);
    candidates.dedup_by_key(|candidate| candidate.0);
    if candidates.is_empty() {
        return Ok(());
    }
    let mut details = subject
        .metadata
        .clone()
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let Some(object) = details.as_object_mut() else {
        return Ok(());
    };
    let mut matched = false;
    for (event_id, timestamp, metadata_json) in candidates {
        let metadata: Value = from_json("metadata_json", &metadata_json)?;
        let windows_event_id = metadata
            .get("event_id")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        if crash_only && !matches!(windows_event_id, 1000 | 1001 | 1002 | 1026) {
            continue;
        }
        matched = true;
        append_evidence(
            object,
            event_id,
            timestamp,
            metadata
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            windows_event_id,
            metadata.get("count").and_then(Value::as_u64).unwrap_or(1),
        );
    }
    if !matched {
        return Ok(());
    }
    if crash_only {
        object.insert(
            "exit_kind".to_owned(),
            Value::String("unexpected".to_owned()),
        );
    }
    conn.execute(
        "UPDATE events SET metadata_json = ?1 WHERE id = ?2",
        rusqlite::params![to_json(&details)?, subject_id],
    )
    .during("store existing Event Log corroboration")?;
    Ok(())
}

/// Apply order-independent Event Log reconciliation inside the caller's
/// transaction. Both ordinary batches and atomic collector commits must call
/// this after all rows in their batch exist.
pub(crate) fn reconcile_inserted_event(
    conn: &Connection,
    event: &NormalizedEvent,
    stored_id: i64,
) -> AppResult<()> {
    if event.source_collector == "eventlog" {
        reconcile_eventlog_evidence(conn, event, stored_id)
    } else if matches!(event.source_collector.as_str(), "process" | "updates") {
        corroborate_subject_from_existing_logs(conn, event, stored_id)
    } else {
        Ok(())
    }
}

fn reconcile_eventlog_evidence(
    conn: &Connection,
    evidence: &NormalizedEvent,
    stored_event_id: i64,
) -> AppResult<()> {
    let Some(metadata) = evidence.metadata.as_ref() else {
        return Ok(());
    };
    let windows_event_id = metadata
        .get("event_id")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let provider = metadata
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if matches!(windows_event_id, 1000 | 1001 | 1002 | 1026) {
        reconcile_candidates(
            conn,
            ReconcileCriteria {
                from: evidence.timestamp.saturating_sub(15_000),
                to: evidence.timestamp.saturating_add(15_000),
                source_collector: "process",
                event_type: Some("stopped"),
                evidence_metadata: metadata,
                kind: EvidenceSubjectKind::Process,
            },
            |details| {
                details.insert(
                    "exit_kind".to_owned(),
                    Value::String("unexpected".to_owned()),
                );
                append_evidence(
                    details,
                    stored_event_id,
                    evidence.timestamp,
                    provider,
                    windows_event_id,
                    metadata.get("count").and_then(Value::as_u64).unwrap_or(1),
                );
            },
        )?;
    }
    if update_source_allowed(metadata) {
        reconcile_candidates(
            conn,
            ReconcileCriteria {
                from: evidence.timestamp.saturating_sub(15 * 60_000),
                to: evidence.timestamp.saturating_add(15 * 60_000),
                source_collector: "updates",
                event_type: None,
                evidence_metadata: metadata,
                kind: EvidenceSubjectKind::Update,
            },
            |details| {
                append_evidence(
                    details,
                    stored_event_id,
                    evidence.timestamp,
                    provider,
                    windows_event_id,
                    metadata.get("count").and_then(Value::as_u64).unwrap_or(1),
                );
            },
        )?;
    }
    Ok(())
}

struct ReconcileCriteria<'a> {
    from: i64,
    to: i64,
    source_collector: &'a str,
    event_type: Option<&'a str>,
    evidence_metadata: &'a Value,
    kind: EvidenceSubjectKind,
}

fn reconcile_candidates(
    conn: &Connection,
    criteria: ReconcileCriteria<'_>,
    mut enrich: impl FnMut(&mut serde_json::Map<String, Value>),
) -> AppResult<()> {
    let mut statement = conn
        .prepare_cached(
            "SELECT id, entity_display_name, metadata_json FROM events \
             WHERE source_collector = ?1 AND timestamp >= ?2 AND timestamp <= ?3 \
               AND (?4 IS NULL OR event_type = ?4) \
             ORDER BY ABS(timestamp - ?5), id ASC LIMIT 4096",
        )
        .during("prepare an Event Log reconciliation query")?;
    let candidates = statement
        .query_map(
            rusqlite::params![
                criteria.source_collector,
                criteria.from,
                criteria.to,
                criteria.event_type,
                criteria.from.saturating_add(criteria.to).div_euclid(2)
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .during("query Event Log reconciliation candidates")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .during("read Event Log reconciliation candidates")?;
    drop(statement);
    for (id, display_name, metadata_json) in candidates {
        let mut details = metadata_json
            .as_deref()
            .map(|json| from_json::<Value>("metadata_json", json))
            .transpose()?
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
        let metadata_subject = details
            .get("update_id")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let matches = eventlog_metadata_matches_subject(
            criteria.evidence_metadata,
            &display_name,
            criteria.kind,
        ) || metadata_subject.as_deref().is_some_and(|subject| {
            eventlog_metadata_matches_subject(criteria.evidence_metadata, subject, criteria.kind)
        });
        if !matches {
            continue;
        }
        if !details.is_object() {
            details = Value::Object(serde_json::Map::new());
        }
        if let Some(object) = details.as_object_mut() {
            enrich(object);
        }
        conn.execute(
            "UPDATE events SET metadata_json = ?1 WHERE id = ?2",
            rusqlite::params![to_json(&details)?, id],
        )
        .during("store Event Log corroboration")?;
    }
    Ok(())
}

fn append_evidence(
    details: &mut serde_json::Map<String, Value>,
    stored_event_id: i64,
    timestamp_ms: i64,
    provider: &str,
    event_id: u64,
    count: u64,
) {
    let list = details
        .entry("log_evidence".to_owned())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Some(list) = list.as_array_mut() else {
        return;
    };
    if let Some(existing) = list
        .iter_mut()
        .find(|item| item.get("stored_event_id").and_then(Value::as_i64) == Some(stored_event_id))
    {
        let previous = existing.get("count").and_then(Value::as_u64).unwrap_or(1);
        if let Some(object) = existing.as_object_mut() {
            object.insert(
                "count".to_owned(),
                Value::from(previous.saturating_add(count)),
            );
        }
        return;
    }
    list.push(serde_json::json!({
        "stored_event_id": stored_event_id,
        "timestamp_ms": timestamp_ms,
        "provider": provider,
        "event_id": event_id,
        "count": count,
    }));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceSubjectKind {
    Any,
    Process,
    Update,
}

pub(crate) fn eventlog_metadata_matches_subject(
    metadata: &Value,
    subject: &str,
    kind: EvidenceSubjectKind,
) -> bool {
    match kind {
        EvidenceSubjectKind::Any => {
            process_metadata_matches(metadata, subject)
                || service_metadata_matches(metadata, subject)
                || update_metadata_matches(metadata, subject)
        }
        EvidenceSubjectKind::Process => process_metadata_matches(metadata, subject),
        EvidenceSubjectKind::Update => update_metadata_matches(metadata, subject),
    }
}

fn service_metadata_matches(metadata: &Value, subject: &str) -> bool {
    if !string_eq(metadata, "channel", "System")
        || !string_eq(metadata, "provider", "Service Control Manager")
        || !matches!(
            metadata
                .get("event_id")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            7000 | 7001 | 7009 | 7011 | 7023 | 7024 | 7031 | 7034
        )
    {
        return false;
    }
    let wanted = subject.trim().to_ascii_lowercase();
    !wanted.is_empty()
        && data_fields(metadata).any(|(name, value)| {
            matches!(name, "param1" | "servicename") && value.trim().eq_ignore_ascii_case(&wanted)
        })
}

fn process_metadata_matches(metadata: &Value, subject: &str) -> bool {
    if !string_eq(metadata, "channel", "Application") {
        return false;
    }
    let event_id = metadata
        .get("event_id")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let provider = metadata
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let provider_allowed = match event_id {
        1000 => provider.eq_ignore_ascii_case("Application Error"),
        1001 => provider.eq_ignore_ascii_case("Windows Error Reporting"),
        1002 => provider.eq_ignore_ascii_case("Application Hang"),
        1026 => provider.eq_ignore_ascii_case(".NET Runtime"),
        _ => false,
    };
    if !provider_allowed {
        return false;
    }
    let wanted = process_basename(subject);
    !wanted.is_empty()
        && data_fields(metadata).any(|(name, value)| {
            let allowed =
                matches!(
                    name,
                    "appname"
                        | "application"
                        | "applicationname"
                        | "faultingapplicationname"
                        | "apppath"
                        | "p1"
                ) || (event_id == 1026 && name.is_empty() && value.starts_with("Application:"));
            allowed && process_basename(value) == wanted
        })
}

fn update_metadata_matches(metadata: &Value, subject: &str) -> bool {
    if !update_source_allowed(metadata) {
        return false;
    }
    let wanted = subject.trim().to_ascii_lowercase();
    !wanted.is_empty()
        && data_fields(metadata).any(|(name, value)| {
            matches!(
                name,
                "updatetitle" | "updateguid" | "packageidentifier" | "packageidentity"
            ) && exact_token(value, &wanted)
        })
}

fn update_source_allowed(metadata: &Value) -> bool {
    let channel = metadata
        .get("channel")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let provider = metadata
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or_default();
    (channel.eq_ignore_ascii_case("Microsoft-Windows-WindowsUpdateClient/Operational")
        && provider.eq_ignore_ascii_case("Microsoft-Windows-WindowsUpdateClient"))
        || (channel.eq_ignore_ascii_case("Setup")
            && provider.eq_ignore_ascii_case("Microsoft-Windows-Servicing"))
}

fn data_fields(metadata: &Value) -> impl Iterator<Item = (&str, &str)> {
    metadata
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|field| {
            Some((
                field
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                field.get("value").and_then(Value::as_str)?,
            ))
        })
        .map(|(name, value)| (name.to_ascii_lowercase(), value))
        .map(|(name, value)| {
            // Leak-free ownership is unnecessary here: normalize the finite
            // field names to the known literals without allocating downstream.
            let canonical = match name.as_str() {
                "appname" => "appname",
                "application" => "application",
                "applicationname" => "applicationname",
                "faultingapplicationname" => "faultingapplicationname",
                "apppath" => "apppath",
                "p1" => "p1",
                "param1" => "param1",
                "servicename" => "servicename",
                "updatetitle" => "updatetitle",
                "updateguid" => "updateguid",
                "packageidentifier" => "packageidentifier",
                "packageidentity" => "packageidentity",
                _ => "",
            };
            (canonical, value)
        })
}

fn process_basename(value: &str) -> String {
    let value = value
        .trim()
        .strip_prefix("Application:")
        .unwrap_or(value)
        .trim()
        .trim_matches(['\"', '\'', ' ']);
    value
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn exact_token(value: &str, wanted: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value == wanted
        || value
            .split(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
            .any(|token| token == wanted)
}

fn string_eq(metadata: &Value, field: &str, wanted: &str) -> bool {
    metadata
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|value| value.eq_ignore_ascii_case(wanted))
}

/// Event Log callbacks may be drained in separate runtime cycles. Merge the
/// same bounded storm key into its first durable row so callback batching does
/// not decide whether a storm is one event or several.
fn merge_eventlog_duplicate(conn: &Connection, event: &NormalizedEvent) -> AppResult<Option<i64>> {
    if event.source_collector != "eventlog" {
        return Ok(None);
    }
    let Some(metadata) = event.metadata.as_ref() else {
        return Ok(None);
    };
    let Some(provider) = metadata.get("provider").and_then(Value::as_str) else {
        return Ok(None);
    };
    let Some(event_id) = metadata.get("event_id").and_then(Value::as_u64) else {
        return Ok(None);
    };
    let first_data_value = metadata
        .get("data")
        .and_then(Value::as_array)
        .and_then(|data| data.first())
        .and_then(|field| field.get("value"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let existing = conn
        .query_row(
            "SELECT id, timestamp, metadata_json FROM events \
             WHERE source_collector = 'eventlog' AND timestamp >= ?1 AND timestamp <= ?2 \
               AND LOWER(json_extract(metadata_json, '$.provider')) = LOWER(?3) \
               AND CAST(json_extract(metadata_json, '$.event_id') AS INTEGER) = ?4 \
               AND LOWER(COALESCE(json_extract(metadata_json, '$.data[0].value'), '')) = LOWER(?5) \
             ORDER BY CASE WHEN timestamp = ?6 THEN 0 ELSE 1 END, timestamp DESC, id DESC LIMIT 1",
            rusqlite::params![
                event.timestamp.saturating_sub(60_000),
                event.timestamp.saturating_add(60_000),
                provider,
                i64::try_from(event_id).unwrap_or(i64::MAX),
                first_data_value,
                event.timestamp
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .during("find an Event Log storm aggregate")?;
    let Some((stored_id, stored_timestamp, stored_metadata)) = existing else {
        return Ok(None);
    };
    // The platform intentionally replays the latest 6005 on every app start
    // to recover the current boot boundary. Replaying that exact OS record is
    // an idempotent upsert, not another occurrence in a storm.
    if event_id == 6005 && stored_timestamp == event.timestamp {
        return Ok(Some(stored_id));
    }
    let mut stored: Value = from_json("metadata_json", &stored_metadata)?;
    let previous_count = stored.get("count").and_then(Value::as_u64).unwrap_or(1);
    let additional_count = metadata.get("count").and_then(Value::as_u64).unwrap_or(1);
    if let Some(object) = stored.as_object_mut() {
        object.insert(
            "count".to_owned(),
            Value::from(previous_count.saturating_add(additional_count)),
        );
    }
    conn.execute(
        "UPDATE events SET metadata_json = ?1 WHERE id = ?2",
        rusqlite::params![to_json(&stored)?, stored_id],
    )
    .during("increment an Event Log storm aggregate")?;
    Ok(Some(stored_id))
}

/// Insert one already-sanitized event row on an existing transaction.
///
/// This does not begin or commit a transaction and does not touch the primary
/// entity. It is the shared row primitive for the ordinary event batch and
/// the wider collector commit, whose graph and baseline writes must share the
/// generated event id atomically.
pub(crate) fn insert_row(
    conn: &Connection,
    sanitized: &Sanitized<NormalizedEvent>,
) -> AppResult<i64> {
    let mut insert = conn
        .prepare_cached(EVENT_INSERT_SQL)
        .during("prepare the event insert")?;
    insert_prepared(&mut insert, sanitized)
}

/// Insert one row, or merge an Event Log storm delta into its durable row.
/// The caller remains responsible for entity writes and reconciliation.
pub(crate) fn insert_or_merge(
    conn: &Connection,
    sanitized: &Sanitized<NormalizedEvent>,
) -> AppResult<i64> {
    if let Some(existing_id) = merge_eventlog_duplicate(conn, sanitized.get())? {
        Ok(existing_id)
    } else {
        insert_row(conn, sanitized)
    }
}

const EVENT_INSERT_SQL: &str = "INSERT INTO events (\
        timestamp, category, event_type, entity_type, entity_id, \
        entity_display_name, severity, importance, source_collector, \
        before_json, after_json, metadata_json, related_entity_ids, \
        user_friendly_summary, technical_summary, confidence, sensitive, \
        occurred_at_is_uncertain, field\
    ) VALUES (\
        ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19\
    )";

fn insert_prepared(
    insert: &mut CachedStatement<'_>,
    sanitized: &Sanitized<NormalizedEvent>,
) -> AppResult<i64> {
    let event = sanitized.get();
    let before = event.before.as_ref().map(to_json).transpose()?;
    let after = event.after.as_ref().map(to_json).transpose()?;
    let metadata = event.metadata.as_ref().map(to_json).transpose()?;
    let related = to_json(&event.related_entity_ids)?;
    insert
        .insert(rusqlite::params![
            event.timestamp,
            event.category.as_str(),
            event.event_type.as_str(),
            event.entity_type.as_str(),
            &event.entity_id,
            &event.entity_display_name,
            event.severity.as_str(),
            event.importance.as_str(),
            &event.source_collector,
            before,
            after,
            metadata,
            related,
            &event.user_friendly_summary,
            &event.technical_summary,
            event.confidence.as_str(),
            event.sensitive,
            event.occurred_at_is_uncertain,
            &event.field,
        ])
        .during("insert an event")
}

/// The columns a summary needs, in the order `summary_from_row` reads them.
const SUMMARY_COLUMNS: &str = "id, timestamp, category, event_type, entity_type, entity_id, \
    entity_display_name, severity, importance, confidence, source_collector, \
    before_json, after_json, user_friendly_summary, occurred_at_is_uncertain, field";

fn summary_from_row(row: &Row<'_>) -> AppResult<EventSummary> {
    let read = |index: usize| -> rusqlite::Result<String> { row.get(index) };
    let json = |index: usize, column: &'static str| -> AppResult<Option<serde_json::Value>> {
        let raw: Option<String> = row.get(index).during("read an event row")?;
        raw.as_deref()
            .map(|text| from_json(column, text))
            .transpose()
    };
    Ok(EventSummary {
        id: row.get(0).during("read an event row")?,
        timestamp: row.get(1).during("read an event row")?,
        category: parse_word("category", &read(2).during("read an event row")?)?,
        event_type: parse_word("event_type", &read(3).during("read an event row")?)?,
        entity_type: parse_word("entity_type", &read(4).during("read an event row")?)?,
        entity_id: read(5).during("read an event row")?,
        entity_display_name: read(6).during("read an event row")?,
        severity: parse_word("severity", &read(7).during("read an event row")?)?,
        importance: parse_word("importance", &read(8).during("read an event row")?)?,
        confidence: parse_word("confidence", &read(9).during("read an event row")?)?,
        source_collector: read(10).during("read an event row")?,
        before: json(11, "before_json")?,
        after: json(12, "after_json")?,
        user_friendly_summary: read(13).during("read an event row")?,
        occurred_at_is_uncertain: row.get(14).during("read an event row")?,
        field: row.get(15).during("read an event row")?,
    })
}

/// One page, newest first, continuing after the query's cursor.
///
/// The SQL is assembled from fixed fragments; user input only ever arrives as
/// a bound parameter. The importance list is expanded from the enum rather
/// than compared with `>=`, so the `(importance, timestamp)` index applies.
pub fn page(conn: &Connection, query: &EventQuery) -> AppResult<Page<EventSummary>> {
    #[cfg(feature = "devtools")]
    let capture = super::page_plan_capture::begin(query);
    let (sql, params, limit) = page_statement(query);
    let mut statement = conn.prepare_cached(&sql).during("prepare a page query")?;
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        // These are the exact prepare input, connection and concrete values
        // borrowed by the original statement below, never reconstructed SQL.
        capture.prepared(conn, &sql, &params, statement.parameter_count());
    }
    let refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
    let mut rows = statement
        .query(refs.as_slice())
        .during("run a page query")?;
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        capture.started();
    }
    let mut items = Vec::with_capacity(usize::from(limit));
    while let Some(row) = rows.next().during("read a page")? {
        items.push(summary_from_row(row)?);
    }
    #[cfg(feature = "devtools")]
    if let Some(capture) = &capture {
        // Include the decoded lookahead row before unchanged truncation.
        capture.completed(items.len());
    }
    let next = if items.len() > usize::from(limit) {
        items.truncate(usize::from(limit));
        items.last().map(|last| Cursor {
            timestamp: last.timestamp,
            id: last.id,
        })
    } else {
        None
    };
    Ok(Page { items, next })
}

/// Build the exact bounded SQL and owned parameters used by [`page`].
fn page_statement(query: &EventQuery) -> (String, Vec<SqlValue>, u16) {
    let limit = query.effective_limit();
    let mut sql =
        format!("SELECT {SUMMARY_COLUMNS} FROM events WHERE timestamp >= ?1 AND timestamp <= ?2");
    let mut params = vec![SqlValue::Integer(query.from), SqlValue::Integer(query.to)];

    if let Some(cursor) = query.cursor {
        params.push(SqlValue::Integer(cursor.timestamp));
        params.push(SqlValue::Integer(cursor.id));
        let n = params.len();
        sql.push_str(&format!(
            " AND (timestamp < ?{} OR (timestamp = ?{} AND id < ?{}))",
            n - 1,
            n - 1,
            n
        ));
    }
    if !query.categories.is_empty() {
        let placeholders = page_bind_list(&mut params, query.categories.iter().map(|c| c.as_str()));
        sql.push_str(&format!(" AND category IN ({placeholders})"));
    }
    if let Some(minimum) = query.importance_min {
        let allowed = Importance::ALL
            .iter()
            .filter(|i| **i >= minimum)
            .map(|i| i.as_str());
        let placeholders = page_bind_list(&mut params, allowed);
        sql.push_str(&format!(" AND importance IN ({placeholders})"));
    }
    if let Some(entity_id) = &query.entity_id {
        params.push(SqlValue::Text(entity_id.clone()));
        sql.push_str(&format!(" AND entity_id = ?{}", params.len()));
    }
    // One more than the page so the presence of a next page is known without
    // a count query.
    params.push(SqlValue::Integer(i64::from(limit) + 1));
    sql.push_str(&format!(
        " ORDER BY timestamp DESC, id DESC LIMIT ?{}",
        params.len()
    ));

    (sql, params, limit)
}

/// Page-only concrete bindings; the shared arbitrary-ToSql helper stays intact.
fn page_bind_list<'a>(params: &mut Vec<SqlValue>, values: impl Iterator<Item = &'a str>) -> String {
    let mut placeholders = Vec::new();
    for value in values {
        params.push(SqlValue::Text(value.to_owned()));
        placeholders.push(format!("?{}", params.len()));
    }
    placeholders.join(", ")
}

#[cfg(all(test, feature = "devtools"))]
#[path = "page_capture_owner_tests.rs"]
mod page_capture_owner_tests;

/// Query-planner evidence for every UI filter combination. Kept out of a
/// release library unless development tooling was explicitly enabled.
#[cfg(any(test, debug_assertions, feature = "devtools"))]
pub fn explain_page_plan(conn: &Connection, query: &EventQuery) -> AppResult<Vec<String>> {
    let (sql, params, _) = page_statement(query);
    let mut statement = conn
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .during("prepare a page query plan")?;
    let refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();
    let rows = statement
        .query_map(refs.as_slice(), |row| row.get::<_, String>(3))
        .during("explain a page query")?;
    rows.collect::<rusqlite::Result<Vec<String>>>()
        .during("read a page query plan")
}

/// Push each value as a parameter and return `?n, ?m, …` for them.
fn bind_list<'a>(
    params: &mut Vec<Box<dyn rusqlite::ToSql>>,
    values: impl Iterator<Item = &'a str>,
) -> String {
    let mut placeholders = Vec::new();
    for value in values {
        params.push(Box::new(value.to_owned()));
        placeholders.push(format!("?{}", params.len()));
    }
    placeholders.join(", ")
}

/// The whole event, or `None` if it has expired.
pub fn by_id(conn: &Connection, id: i64) -> AppResult<Option<SystemEvent>> {
    let mut statement = conn
        .prepare_cached(&format!(
            "SELECT {FULL_EVENT_COLUMNS} FROM events WHERE id = ?1"
        ))
        .during("prepare an event lookup")?;
    let mut rows = statement.query([id]).during("look up an event")?;
    let Some(row) = rows.next().during("look up an event")? else {
        return Ok(None);
    };
    Ok(Some(system_event_from_row(row)?))
}

fn system_event_from_row(row: &Row<'_>) -> AppResult<SystemEvent> {
    let text = |index: usize| -> AppResult<String> { row.get(index).during("read an event") };
    let json = |index: usize, column: &'static str| -> AppResult<Option<serde_json::Value>> {
        let raw: Option<String> = row.get(index).during("read an event")?;
        raw.as_deref().map(|t| from_json(column, t)).transpose()
    };
    let related: String = text(13)?;
    Ok(SystemEvent {
        id: row.get(0).during("read an event")?,
        event: NormalizedEvent {
            timestamp: row.get(1).during("read an event")?,
            category: parse_word("category", &text(2)?)?,
            event_type: parse_word("event_type", &text(3)?)?,
            entity_type: parse_word("entity_type", &text(4)?)?,
            entity_id: text(5)?,
            entity_display_name: text(6)?,
            severity: parse_word("severity", &text(7)?)?,
            importance: parse_word("importance", &text(8)?)?,
            source_collector: text(9)?,
            before: json(10, "before_json")?,
            after: json(11, "after_json")?,
            field: row.get(19).during("read an event")?,
            metadata: json(12, "metadata_json")?,
            related_entity_ids: from_json("related_entity_ids", &related)?,
            user_friendly_summary: text(14)?,
            technical_summary: text(15)?,
            confidence: parse_word("confidence", &text(16)?)?,
            sensitive: row.get(17).during("read an event")?,
            occurred_at_is_uncertain: row.get(18).during("read an event")?,
        },
    })
}

/// Fetch the deterministic analysis corpus for an inclusive window. SQL does
/// the downsampling: non-routine evidence is retained first, then the newest
/// informational rows fill the remaining capacity. At most 5,000 full rows
/// are ever decoded, regardless of table size.
pub fn analysis_window(conn: &Connection, from: i64, to: i64) -> AppResult<AnalysisEventWindow> {
    analysis_window_filtered(conn, from, to, &[], None, None)
}

/// Fetch the deterministic analysis corpus after applying the Timeline's
/// category, importance and entity predicates. Filtering happens in SQL
/// before the 5,000-row analysis bound, so unrelated churn cannot crowd a
/// selected Timeline slice out of the Change Map.
pub fn analysis_window_filtered(
    conn: &Connection,
    from: i64,
    to: i64,
    categories: &[Category],
    importance_min: Option<Importance>,
    entity_id: Option<&str>,
) -> AppResult<AnalysisEventWindow> {
    analysis_window_with_scope(
        conn,
        from,
        to,
        categories,
        importance_min,
        entity_id.map(AnalysisEntityScope::Entity),
    )
}

/// A workspace includes its direct/related evidence and recorded file paths
/// beneath its current registered root. Membership is resolved for this new
/// analysis only; persisted event associations and investigation snapshots do
/// not change when a workspace is registered or removed.
pub fn analysis_window_workspace(
    conn: &Connection,
    from: i64,
    to: i64,
    project_id: &str,
) -> AppResult<AnalysisEventWindow> {
    analysis_window_with_scope(
        conn,
        from,
        to,
        &[],
        None,
        Some(AnalysisEntityScope::Workspace(project_id)),
    )
}

#[derive(Clone, Copy)]
enum AnalysisEntityScope<'a> {
    Entity(&'a str),
    Workspace(&'a str),
}

fn analysis_window_with_scope(
    conn: &Connection,
    from: i64,
    to: i64,
    categories: &[Category],
    importance_min: Option<Importance>,
    scope: Option<AnalysisEntityScope<'_>>,
) -> AppResult<AnalysisEventWindow> {
    let (from, to) = if from <= to { (from, to) } else { (to, from) };
    let mut predicate = "timestamp >= ?1 AND timestamp <= ?2".to_owned();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(from), Box::new(to)];
    if !categories.is_empty() {
        let placeholders = bind_list(&mut params, categories.iter().map(|item| item.as_str()));
        predicate.push_str(&format!(" AND category IN ({placeholders})"));
    }
    if let Some(minimum) = importance_min {
        let allowed = Importance::ALL
            .iter()
            .filter(|item| **item >= minimum)
            .map(|item| item.as_str());
        let placeholders = bind_list(&mut params, allowed);
        predicate.push_str(&format!(" AND importance IN ({placeholders})"));
    }
    if let Some(scope) = scope {
        let (AnalysisEntityScope::Entity(entity_id) | AnalysisEntityScope::Workspace(entity_id)) =
            scope;
        params.push(Box::new(entity_id.to_owned()));
        predicate.push_str(&format!(
            " AND (entity_id = ?{} OR EXISTS (SELECT 1 FROM json_each(COALESCE(events.related_entity_ids, '[]')) WHERE json_each.value = ?{})",
            params.len(),
            params.len()
        ));
        if matches!(scope, AnalysisEntityScope::Workspace(_)) {
            if let Some(project) = super::project_repository::by_id(conn, entity_id)? {
                let root = crate::domain::ids::canonical_path(&project.path);
                let prefix = format!("{}\\", root.trim_end_matches('\\'));
                let upper_bound = format!("{}]", root.trim_end_matches('\\'));
                // Keep the time/category/importance predicates outside this OR.
                // CASE prevents non-string JSON values reaching canonical_path.
                params.push(Box::new(root));
                params.push(Box::new(prefix));
                params.push(Box::new(upper_bound));
                predicate.push_str(&format!(
                    " OR (source_collector = 'files' AND entity_type = 'file' \
                     AND CASE WHEN json_type(metadata_json, '$.path') = 'text' THEN \
                         (canonical_path(json_extract(metadata_json, '$.path')) = ?{} \
                          OR (canonical_path(json_extract(metadata_json, '$.path')) >= ?{} \
                              AND canonical_path(json_extract(metadata_json, '$.path')) < ?{})) \
                         ELSE 0 END)",
                    params.len() - 2,
                    params.len() - 1,
                    params.len()
                ));
            }
        }
        predicate.push(')');
    }
    let refs = params
        .iter()
        .map(|parameter| parameter.as_ref())
        .collect::<Vec<_>>();
    let total = conn
        .prepare(&format!("SELECT COUNT(*) FROM events WHERE {predicate}"))
        .during("prepare analysis window count")?
        .query_row(refs.as_slice(), |row| count_column(row, 0))
        .during("count analysis window")?;
    params.push(Box::new(
        i64::try_from(MAX_ANALYSIS_EVENTS).unwrap_or(i64::MAX),
    ));
    let mut statement = conn
        .prepare(&format!(
            "SELECT {FULL_EVENT_COLUMNS} FROM events \
             WHERE {predicate} \
             ORDER BY CASE \
                 WHEN EXISTS (SELECT 1 FROM entity_relationships r WHERE r.event_id = events.id) \
                   OR EXISTS (SELECT 1 FROM entity_relationships r \
                              WHERE r.recorded_at <= ?2 AND r.source_id = events.entity_id) \
                   OR EXISTS (SELECT 1 FROM entity_relationships r \
                              WHERE r.recorded_at <= ?2 AND r.target_id = events.entity_id) \
                   OR (category IN ('service', 'system') AND event_type = 'failed') \
                   OR (entity_type = 'port' AND event_type = 'closed') \
                   OR (event_type = 'stopped' AND instr(COALESCE(metadata_json, ''), 'unexpected') > 0) \
                   THEN 0 \
                 WHEN importance = 'critical' THEN 1 \
                 WHEN importance = 'important' THEN 2 \
                 WHEN importance = 'notable' THEN 3 ELSE 4 END, \
                 timestamp DESC, id DESC LIMIT ?{}",
            params.len()
        ))
        .during("prepare analysis event window")?;
    let refs = params
        .iter()
        .map(|parameter| parameter.as_ref())
        .collect::<Vec<_>>();
    let mut rows = statement
        .query(refs.as_slice())
        .during("read analysis event window")?;
    let mut events = Vec::with_capacity(
        usize::try_from(total)
            .unwrap_or(MAX_ANALYSIS_EVENTS)
            .min(MAX_ANALYSIS_EVENTS),
    );
    while let Some(row) = rows.next().during("read analysis event window")? {
        events.push(system_event_from_row(row)?);
    }
    events.sort_by_key(|event| (event.event.timestamp, event.id));
    Ok(AnalysisEventWindow {
        downsampled: total.saturating_sub(events.len() as u64),
        total,
        events,
    })
}

/// Coverage bounds and the nearest stored row on either side of an inclusive
/// range. Each subquery uses timestamp indexes and returns only scalar values.
pub fn window_boundary_context(
    conn: &Connection,
    from: i64,
    to: i64,
) -> AppResult<WindowBoundaryContext> {
    let (from, to) = if from <= to { (from, to) } else { (to, from) };
    conn.prepare_cached(
        "SELECT \
         (SELECT MIN(timestamp) FROM events), \
         (SELECT MAX(timestamp) FROM events), \
         (SELECT timestamp FROM events WHERE timestamp < ?1 ORDER BY timestamp DESC, id DESC LIMIT 1), \
         (SELECT id FROM events WHERE timestamp < ?1 ORDER BY timestamp DESC, id DESC LIMIT 1), \
         (SELECT timestamp FROM events WHERE timestamp > ?2 ORDER BY timestamp, id LIMIT 1), \
         (SELECT id FROM events WHERE timestamp > ?2 ORDER BY timestamp, id LIMIT 1)",
    )
    .during("prepare analysis window boundaries")?
    .query_row((from, to), |row| {
        let before_timestamp: Option<i64> = row.get(2)?;
        let before_id: Option<i64> = row.get(3)?;
        let after_timestamp: Option<i64> = row.get(4)?;
        let after_id: Option<i64> = row.get(5)?;
        Ok(WindowBoundaryContext {
            coverage_from: row.get(0)?,
            coverage_to: row.get(1)?,
            immediately_before: before_timestamp.zip(before_id),
            immediately_after: after_timestamp.zip(after_id),
        })
    })
    .during("read analysis window boundaries")
}

/// Nearest events before an analysis range. A small bounded set lets chain
/// detection find a relevant head even when an unrelated row is globally
/// nearer to the window boundary.
pub fn analysis_boundary_events(conn: &Connection, from: i64) -> AppResult<Vec<SystemEvent>> {
    let mut statement = conn
        .prepare_cached(&format!(
            "SELECT {FULL_EVENT_COLUMNS} FROM events WHERE timestamp < ?1 \
             ORDER BY timestamp DESC, id DESC LIMIT ?2"
        ))
        .during("prepare analysis boundary events")?;
    let mut rows = statement
        .query((
            from,
            i64::try_from(MAX_ANALYSIS_BOUNDARY_EVENTS).unwrap_or(i64::MAX),
        ))
        .during("read analysis boundary events")?;
    let mut events = Vec::new();
    while let Some(row) = rows.next().during("read analysis boundary events")? {
        events.push(system_event_from_row(row)?);
    }
    events.sort_by_key(|event| (event.event.timestamp, event.id));
    Ok(events)
}

/// Repository-level scope for the exact rows represented by a cluster.
///
/// Keeping this storage-facing shape separate from the service `ClusterRef`
/// preserves the dependency direction while ensuring the tuple bounds travel
/// together.
pub struct ClusterMembersQuery<'a> {
    pub category: &'a str,
    pub coarse_entity: &'a str,
    pub first: (i64, i64),
    pub last: (i64, i64),
    pub limit: u16,
}

/// Fetch exactly the rows represented by a cluster reference.
pub fn cluster_members(
    conn: &Connection,
    query: &ClusterMembersQuery<'_>,
) -> AppResult<Vec<EventSummary>> {
    let mut statement = conn
        .prepare_cached(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM events WHERE category = ?1 \
         AND (entity_id = ?2 OR (substr(entity_id, 1, length(?2)) = ?2 \
              AND substr(entity_id, length(?2) + 1, 1) = ':')) \
         AND (timestamp > ?3 OR (timestamp = ?3 AND id >= ?4)) \
         AND (timestamp < ?5 OR (timestamp = ?5 AND id <= ?6)) \
         ORDER BY timestamp DESC, id DESC LIMIT ?7"
        ))
        .during("prepare cluster expansion")?;
    let mut rows = statement
        .query(rusqlite::params![
            query.category,
            query.coarse_entity,
            query.first.0,
            query.first.1,
            query.last.0,
            query.last.1,
            i64::from(query.limit)
        ])
        .during("read cluster expansion")?;
    let mut events = Vec::new();
    while let Some(row) = rows.next().during("read cluster expansion")? {
        events.push(summary_from_row(row)?);
    }
    Ok(events)
}

pub fn top_important(
    conn: &Connection,
    from: i64,
    to: i64,
    limit: u16,
) -> AppResult<Vec<EventSummary>> {
    let mut statement = conn
        .prepare_cached(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM events \
             WHERE timestamp >= ?1 AND timestamp <= ?2 \
             AND importance IN ('important', 'critical') \
             ORDER BY timestamp DESC, id DESC LIMIT ?3"
        ))
        .during("prepare important events")?;
    let mut rows = statement
        .query((from, to, i64::from(limit)))
        .during("read important events")?;
    let mut events = Vec::new();
    while let Some(row) = rows.next().during("read important events")? {
        events.push(summary_from_row(row)?);
    }
    Ok(events)
}

/// Summaries for ids in the caller's order. Missing/expired rows are skipped.
pub fn summaries_by_ids(conn: &Connection, ids: &[i64]) -> AppResult<Vec<EventSummary>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = (0..ids.len()).map(|_| "?").collect::<Vec<_>>().join(", ");
    let mut statement = conn
        .prepare_cached(&format!(
            "SELECT {SUMMARY_COLUMNS} FROM events WHERE id IN ({placeholders})"
        ))
        .during("prepare event summaries")?;
    let mut rows = statement
        .query(rusqlite::params_from_iter(ids.iter()))
        .during("read event summaries")?;
    let mut by_id = std::collections::HashMap::with_capacity(ids.len());
    while let Some(row) = rows.next().during("read event summaries")? {
        let event = summary_from_row(row)?;
        by_id.insert(event.id, event);
    }
    Ok(ids.iter().filter_map(|id| by_id.get(id).cloned()).collect())
}

/// Nearby events sharing the event's entity or one of its related entities.
pub fn related(
    conn: &Connection,
    event_id: i64,
    entity_ids: &[String],
    from: i64,
    to: i64,
    limit: u16,
) -> AppResult<Vec<EventSummary>> {
    if entity_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut params: Vec<Box<dyn rusqlite::ToSql>> =
        vec![Box::new(event_id), Box::new(from), Box::new(to)];
    let placeholders = bind_list(&mut params, entity_ids.iter().map(String::as_str));
    params.push(Box::new(i64::from(limit)));
    let sql = format!(
        "SELECT {SUMMARY_COLUMNS} FROM events \
         WHERE id != ?1 AND timestamp >= ?2 AND timestamp <= ?3 \
         AND entity_id IN ({placeholders}) \
         ORDER BY timestamp DESC, id DESC LIMIT ?{}",
        params.len()
    );
    let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|value| value.as_ref()).collect();
    let mut statement = conn.prepare_cached(&sql).during("prepare related events")?;
    let mut rows = statement
        .query(refs.as_slice())
        .during("read related events")?;
    let mut events = Vec::new();
    while let Some(row) = rows.next().during("read related events")? {
        events.push(summary_from_row(row)?);
    }
    Ok(events)
}

/// How many events of each importance fall in a range.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportanceCounts {
    /// Rows rated informational.
    pub informational: u64,
    /// Rows rated notable.
    pub notable: u64,
    /// Rows rated important.
    pub important: u64,
    /// Rows rated critical.
    pub critical: u64,
}

impl ImportanceCounts {
    /// All rows in the range.
    pub fn total(&self) -> u64 {
        self.informational + self.notable + self.important + self.critical
    }
}

/// Counts by importance for `from..=to`.
pub fn counts_for_range(conn: &Connection, from: i64, to: i64) -> AppResult<ImportanceCounts> {
    const DAY_MS: i64 = 86_400_000;
    if from > to {
        return Ok(ImportanceCounts::default());
    }
    // Saturating arithmetic keeps the renderer-controlled full-range sentinel
    // (`i64::MIN..=i64::MAX`) inside the query contract. SQLite can compare the
    // original endpoints without any arithmetic, while day-boundary math must
    // never panic in a debug build or wrap in a release build.
    let from_day = from.div_euclid(DAY_MS).saturating_mul(DAY_MS);
    let to_day = to.div_euclid(DAY_MS).saturating_mul(DAY_MS);
    if from_day == to_day {
        return counts_for_event_range(conn, from, to);
    }
    // The first mathematical UTC day starts below i64::MIN. Its stored
    // projection key is clamped to MIN, but the next midnight is only
    // 25,975,808 ms later, not a full DAY_MS after MIN.
    let next_day_start = from.saturating_add(DAY_MS - from.rem_euclid(DAY_MS));

    let mut statement = conn
        .prepare_cached(
            "SELECT \
                left_edge.informational + right_edge.informational + interior.informational, \
                left_edge.notable + right_edge.notable + interior.notable, \
                left_edge.important + right_edge.important + interior.important, \
                left_edge.critical + right_edge.critical + interior.critical \
             FROM ( \
                SELECT \
                    COALESCE(SUM(CASE WHEN importance = 'informational' THEN 1 ELSE 0 END), 0) AS informational, \
                    COALESCE(SUM(CASE WHEN importance = 'notable' THEN 1 ELSE 0 END), 0) AS notable, \
                    COALESCE(SUM(CASE WHEN importance = 'important' THEN 1 ELSE 0 END), 0) AS important, \
                    COALESCE(SUM(CASE WHEN importance = 'critical' THEN 1 ELSE 0 END), 0) AS critical \
                FROM events \
                WHERE timestamp >= ?1 AND timestamp < ?3 \
             ) AS left_edge, ( \
                SELECT \
                    COALESCE(SUM(CASE WHEN importance = 'informational' THEN 1 ELSE 0 END), 0) AS informational, \
                    COALESCE(SUM(CASE WHEN importance = 'notable' THEN 1 ELSE 0 END), 0) AS notable, \
                    COALESCE(SUM(CASE WHEN importance = 'important' THEN 1 ELSE 0 END), 0) AS important, \
                    COALESCE(SUM(CASE WHEN importance = 'critical' THEN 1 ELSE 0 END), 0) AS critical \
                FROM events \
                WHERE timestamp >= ?4 AND timestamp <= ?2 \
             ) AS right_edge, ( \
                SELECT \
                    COALESCE(SUM(informational), 0) AS informational, \
                    COALESCE(SUM(notable), 0) AS notable, \
                    COALESCE(SUM(important), 0) AS important, \
                    COALESCE(SUM(critical), 0) AS critical \
                FROM event_importance_daily \
                WHERE day_start > ?5 AND day_start < ?6 \
             ) AS interior",
        )
        .during("prepare a count query")?;
    statement
        .query_row(
            (from, to, next_day_start, to_day, from_day, to_day),
            |row| {
                Ok(ImportanceCounts {
                    informational: count_column(row, 0)?,
                    notable: count_column(row, 1)?,
                    important: count_column(row, 2)?,
                    critical: count_column(row, 3)?,
                })
            },
        )
        .during("count events")
}

/// Count a range contained within one UTC day. This is used for the two edge
/// days of a multi-day dashboard range and for short ranges before aggregation
/// can provide a benefit.
fn counts_for_event_range(conn: &Connection, from: i64, to: i64) -> AppResult<ImportanceCounts> {
    let mut statement = conn
        .prepare_cached(
            "SELECT \
                COALESCE(SUM(CASE WHEN importance = 'informational' THEN 1 ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN importance = 'notable' THEN 1 ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN importance = 'important' THEN 1 ELSE 0 END), 0), \
                COALESCE(SUM(CASE WHEN importance = 'critical' THEN 1 ELSE 0 END), 0) \
             FROM events WHERE timestamp >= ?1 AND timestamp <= ?2",
        )
        .during("prepare a count query")?;
    statement
        .query_row([from, to], |row| {
            Ok(ImportanceCounts {
                informational: count_column(row, 0)?,
                notable: count_column(row, 1)?,
                important: count_column(row, 2)?,
                critical: count_column(row, 3)?,
            })
        })
        .during("count events")
}

/// How many times the same thing happened to the same entity between `since`
/// and `before` (exclusive), which is what "this happened 2 times before" means.
pub fn prior_occurrences(
    conn: &Connection,
    signature: &EventSignature,
    since: i64,
    before: i64,
) -> AppResult<u64> {
    let mut statement = conn
        .prepare_cached(
            "SELECT COUNT(*) FROM events \
             WHERE entity_id = ?1 AND timestamp >= ?2 AND timestamp < ?3 \
             AND category = ?4 AND event_type = ?5",
        )
        .during("prepare a recurrence query")?;
    statement
        .query_row(
            (
                &signature.entity_id,
                since,
                before,
                signature.category.as_str(),
                signature.event_type.as_str(),
            ),
            |row| count_column(row, 0),
        )
        .during("count prior occurrences")
}

/// Count recurrence for an analysis corpus. Small inputs use bounded batches
/// of indexed counts. Large investigation windows sort the exact signatures
/// before reusing one prepared count query, keeping nearby index pages hot.
/// An existing transaction permits guarded rolling counts; autocommit keeps
/// independent full counts without starting an implicit transaction.
/// Both paths retain exact inclusive-since/exclusive-before counts.
pub fn prior_occurrences_batch(
    conn: &Connection,
    queries: &[PriorOccurrenceQuery],
) -> AppResult<std::collections::BTreeMap<i64, u64>> {
    const MIN_QUERIES_FOR_SORTED_COUNTS: usize = 2_000;
    if queries.len() >= MIN_QUERIES_FOR_SORTED_COUNTS {
        return if conn.is_autocommit() {
            prior_occurrences_sorted_full_counts(conn, queries)
        } else {
            recurrence_rolling_counts::prior_occurrences(conn, queries)
        };
    }
    prior_occurrences_indexed_batches(conn, queries)
}

fn prior_occurrences_indexed_batches(
    conn: &Connection,
    queries: &[PriorOccurrenceQuery],
) -> AppResult<std::collections::BTreeMap<i64, u64>> {
    const QUERIES_PER_BATCH: usize = 150; // 900 bind parameters, below SQLite's legacy 999 limit.
    let mut counts = std::collections::BTreeMap::new();
    for batch in queries.chunks(QUERIES_PER_BATCH) {
        let values = (0..batch.len())
            .map(|index| {
                let first = index * 6 + 1;
                format!(
                    "(?{first}, ?{}, ?{}, ?{}, ?{}, ?{})",
                    first + 1,
                    first + 2,
                    first + 3,
                    first + 4,
                    first + 5
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "WITH input(event_id, entity_id, category, event_type, since_ms, before_ms) AS \
             (VALUES {values}) \
             SELECT input.event_id, SUM((SELECT COUNT(*) FROM events e \
               WHERE e.entity_id = input.entity_id AND e.category = input.category \
                 AND e.event_type = input.event_type \
                 AND e.timestamp >= input.since_ms AND e.timestamp < input.before_ms)) FROM input \
             GROUP BY input.event_id ORDER BY input.event_id"
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::with_capacity(batch.len() * 6);
        for query in batch {
            params.push(Box::new(query.event_id));
            params.push(Box::new(query.signature.entity_id.clone()));
            params.push(Box::new(query.signature.category.as_str().to_owned()));
            params.push(Box::new(query.signature.event_type.as_str().to_owned()));
            params.push(Box::new(query.since));
            params.push(Box::new(query.before));
        }
        let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|value| value.as_ref()).collect();
        let mut statement = conn
            .prepare(&sql)
            .during("prepare batched recurrence query")?;
        let rows = statement
            .query_map(refs.as_slice(), |row| {
                Ok((row.get::<_, i64>(0)?, count_column(row, 1)?))
            })
            .during("count batched prior occurrences")?;
        for row in rows {
            let (event_id, count) = row.during("read batched prior occurrences")?;
            counts.insert(event_id, count);
        }
    }
    Ok(counts)
}

const SORTED_RECURRENCE_SQL: &str = "SELECT COUNT(*) FROM events WHERE entity_id = ?1 \
    AND category = ?2 AND event_type = ?3 \
    AND timestamp >= ?4 AND timestamp < ?5";

fn sorted_recurrence_params(query: &PriorOccurrenceQuery) -> (&str, &str, &str, i64, i64) {
    (
        query.signature.entity_id.as_str(),
        query.signature.category.as_str(),
        query.signature.event_type.as_str(),
        query.since,
        query.before,
    )
}

fn sorted_recurrence_range_params(
    query: &PriorOccurrenceQuery,
    since: i64,
    before: i64,
) -> (&str, &str, &str, i64, i64) {
    (
        query.signature.entity_id.as_str(),
        query.signature.category.as_str(),
        query.signature.event_type.as_str(),
        since,
        before,
    )
}

/// Keep exact-signature index pages hot while counting a large corpus. Sorting
/// changes only execution order; results are restored to input order before
/// applying the legacy 150-query duplicate-ID aggregation semantics.
fn prior_occurrences_sorted_full_counts(
    conn: &Connection,
    queries: &[PriorOccurrenceQuery],
) -> AppResult<std::collections::BTreeMap<i64, u64>> {
    const QUERIES_PER_BATCH: usize = 150;
    let mut order = (0..queries.len()).collect::<Vec<_>>();
    order.sort_unstable_by(|left, right| {
        let left = &queries[*left];
        let right = &queries[*right];
        (
            &left.signature.entity_id,
            left.signature.category.as_str(),
            left.signature.event_type.as_str(),
            left.since,
            left.before,
        )
            .cmp(&(
                &right.signature.entity_id,
                right.signature.category.as_str(),
                right.signature.event_type.as_str(),
                right.since,
                right.before,
            ))
    });
    let mut statement = conn
        .prepare_cached(SORTED_RECURRENCE_SQL)
        .during("prepare sorted recurrence counts")?;
    #[cfg(feature = "devtools")]
    let mut capture = recurrence_plan_capture::begin(conn, SORTED_RECURRENCE_SQL, queries, &order);
    let mut by_query = vec![0_u64; queries.len()];
    for query_index in order {
        let query = &queries[query_index];
        by_query[query_index] = statement
            .query_row(sorted_recurrence_params(query), |row| count_column(row, 0))
            .during("count sorted prior occurrences")?;
        #[cfg(feature = "devtools")]
        if let Some(capture) = &mut capture {
            capture.completed_one();
        }
    }

    // A 150-query batch sums duplicate IDs, while a later batch overwrites
    // an earlier value for that ID. Preserve this public repository behavior.
    let mut counts = std::collections::BTreeMap::new();
    for (batch_index, batch) in queries.chunks(QUERIES_PER_BATCH).enumerate() {
        let mut batch_counts = std::collections::BTreeMap::<i64, u64>::new();
        for (index, query) in batch.iter().enumerate() {
            let query_index = batch_index * QUERIES_PER_BATCH + index;
            let entry = batch_counts.entry(query.event_id).or_default();
            *entry = entry.checked_add(by_query[query_index]).ok_or_else(|| {
                crate::error::DbError::Decoding {
                    reason: "recurrence count exceeds u64".to_owned(),
                }
            })?;
        }
        counts.extend(batch_counts);
    }
    Ok(counts)
}

/// Newest timestamp for one exact persisted event identity.
///
/// Phase 13 uses this to resolve `SinceLastRestart` from the event-log
/// collector's boot fact instead of guessing from a calendar boundary.
pub fn latest_timestamp(
    conn: &Connection,
    category: crate::domain::Category,
    event_type: crate::domain::EventType,
    entity_id: &str,
    source_collector: &str,
) -> AppResult<Option<i64>> {
    let mut statement = conn
        .prepare_cached(
            "SELECT MAX(timestamp) FROM events \
             WHERE category = ?1 AND event_type = ?2 \
             AND entity_id = ?3 AND source_collector = ?4",
        )
        .during("prepare latest event timestamp query")?;
    statement
        .query_row(
            (
                category.as_str(),
                event_type.as_str(),
                entity_id,
                source_collector,
            ),
            |row| row.get(0),
        )
        .during("read latest event timestamp")
}

/// Current Windows boot boundary recorded by the Event Log collector. This is
/// the exact storage fact behind `SinceLastRestart`, exposed without requiring
/// callers to repeat the collector identity tuple.
pub fn latest_restart_timestamp(conn: &Connection) -> AppResult<Option<i64>> {
    latest_timestamp(
        conn,
        crate::domain::Category::System,
        crate::domain::EventType::Started,
        &crate::domain::ids::system(),
        "eventlog",
    )
}

/// Bounded Event Log rows whose sanitized provider or Data value names the
/// requested subject. Filtering precedes LIMIT so a busy window cannot hide a
/// match behind one hundred unrelated rows.
pub(crate) fn log_evidence_candidates(
    conn: &Connection,
    needle: &str,
    kind: EvidenceSubjectKind,
    from_ms: i64,
    to_ms: i64,
    limit: usize,
) -> AppResult<Vec<(i64, i64, String)>> {
    const MAX_CANDIDATES_EXAMINED: usize = 4_096;
    let mut statement = conn
        .prepare_cached(
            "SELECT id, timestamp, metadata_json FROM events \
             WHERE source_collector = 'eventlog' AND timestamp >= ?1 AND timestamp <= ?2 \
               AND instr(LOWER(COALESCE(metadata_json, '')), ?3) > 0 \
             ORDER BY timestamp ASC, id ASC",
        )
        .during("prepare an event-log evidence query")?;
    let rows = statement
        .query_map(rusqlite::params![from_ms, to_ms, needle,], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .during("query event-log evidence")?;
    let mut matched = Vec::new();
    for row in rows.take(MAX_CANDIDATES_EXAMINED) {
        let row = row.during("read an event-log evidence candidate")?;
        let metadata: Value = from_json("metadata_json", &row.2)?;
        if eventlog_metadata_matches_subject(&metadata, needle, kind) {
            matched.push(row);
            if matched.len() == limit {
                break;
            }
        }
    }
    Ok(matched)
}

/// Delete up to `batch` expired rows and return how many went.
///
/// Routine rows expire at `routine_cutoff`; rows rated important or critical
/// live until `important_cutoff`. The caller loops until fewer than `batch`
/// rows are deleted, yielding between calls, so a large purge never holds
/// the writer for long.
pub fn delete_expired(
    conn: &Connection,
    routine_cutoff: i64,
    important_cutoff: i64,
    batch: usize,
) -> AppResult<usize> {
    let mut statement = conn
        .prepare_cached(
            "DELETE FROM events WHERE id IN (\
                SELECT id FROM events WHERE \
                    (importance IN ('important', 'critical') AND timestamp < ?2) \
                    OR (importance IN ('informational', 'notable') AND timestamp < ?1) \
                LIMIT ?3\
            )",
        )
        .during("prepare retention")?;
    statement
        .execute((
            routine_cutoff,
            important_cutoff,
            i64::try_from(batch).unwrap_or(i64::MAX),
        ))
        .during("delete expired events")
}

/// Delete up to `batch` informational process events older than `cutoff`.
///
/// Process start/stop events are the bulk of a busy developer machine's
/// history. Only the informational ones are touched; a process event rated
/// notable or above follows the ordinary lifetimes.
pub fn delete_expired_process_noise(
    conn: &Connection,
    cutoff: i64,
    batch: usize,
) -> AppResult<usize> {
    let mut statement = conn
        .prepare_cached(
            "DELETE FROM events WHERE id IN (\
                SELECT id FROM events WHERE timestamp < ?1 \
                    AND importance = 'informational' AND source_collector = 'process' \
                LIMIT ?2\
            )",
        )
        .during("prepare process noise retention")?;
    statement
        .execute((cutoff, i64::try_from(batch).unwrap_or(i64::MAX)))
        .during("delete expired process noise")
}

/// Delete the `batch` oldest informational events, whatever their source.
///
/// This is the size cap's only lever: notable, important and critical rows are
/// never selected, so an over-full database of important history is reported
/// rather than silently thinned.
pub fn delete_oldest_informational(conn: &Connection, batch: usize) -> AppResult<usize> {
    let mut statement = conn
        .prepare_cached(
            "DELETE FROM events WHERE id IN (\
                SELECT id FROM events WHERE importance = 'informational' \
                ORDER BY timestamp ASC, id ASC LIMIT ?1\
            )",
        )
        .during("prepare size cap prune")?;
    statement
        .execute([i64::try_from(batch).unwrap_or(i64::MAX)])
        .during("delete the oldest informational events")
}

/// Total rows.
pub fn count(conn: &Connection) -> AppResult<u64> {
    conn.prepare_cached("SELECT COUNT(*) FROM events")
        .during("prepare a count")?
        .query_row([], |row| count_column(row, 0))
        .during("count events")
}

/// The oldest timestamp, or `None` when there are no events.
pub fn oldest_timestamp(conn: &Connection) -> AppResult<Option<i64>> {
    conn.prepare_cached("SELECT MIN(timestamp) FROM events")
        .during("prepare an oldest lookup")?
        .query_row([], |row| row.get(0))
        .during("find the oldest event")
}
