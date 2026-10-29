//! Windows Update history collection and durable no-replay diffing.
//!
//! The provider returns plain rows. Its Windows implementation confines every
//! COM object to one owned apartment thread; this module contains no platform
//! calls and remains testable with scripted history.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;

use crate::collectors::stateful;
use crate::collectors::{
    AckedState, Collector, CollectorContext, CollectorDescriptor, CollectorSnapshot, CommitToken,
    EntityIdentity, ObservationClock, ObservationKind, RawEntity, RawObservation, SnapshotContext,
    SystemObservationClock,
};
use crate::domain::{Category, ClassifierContext, Confidence, EventType, Severity};
use crate::error::{AppError, AppResult};
use crate::storage::CollectorCommitReceipt;

/// Canonical collector id.
pub const ID: &str = "updates";
/// Durable state kind owned by this collector.
pub const BASELINE_KIND: &str = "windows_update_history";

/// Update-history entries are immutable and keyed by complete durable
/// identity, so an equal checkpoint key never carries a field mutation.
pub(crate) fn checkpoint_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    let before: UpdateKey = serde_json::from_value(before.clone()).ok()?;
    let after: UpdateKey = serde_json::from_value(after.clone()).ok()?;
    (before == after).then(Vec::new)
}

/// WUA operation value for an installation.
pub const OPERATION_INSTALLATION: i32 = 1;
/// WUA operation value for an uninstallation.
pub const OPERATION_UNINSTALLATION: i32 = 2;
/// WUA result value for success.
pub const RESULT_SUCCEEDED: i32 = 2;
/// WUA result value for success with errors.
pub const RESULT_SUCCEEDED_WITH_ERRORS: i32 = 3;
/// WUA result value for failure.
pub const RESULT_FAILED: i32 = 4;
/// WUA result value for an aborted operation.
pub const RESULT_ABORTED: i32 = 5;

/// One platform-neutral entry from Windows Update Agent history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateHistoryRow {
    pub update_id: String,
    pub revision: i32,
    pub title: String,
    /// WUA's history date converted to Unix epoch milliseconds.
    pub date_ms: i64,
    pub result_code: i32,
    pub operation: i32,
    pub hresult: i32,
    pub client_application_id: String,
    pub support_url: String,
}

impl UpdateHistoryRow {
    fn key(&self) -> UpdateKey {
        UpdateKey {
            update_id: self.update_id.clone(),
            date_ms: self.date_ms,
        }
    }
}

/// Read-only seam around Windows Update Agent history.
pub trait UpdateHistoryProvider: Send + Sync + 'static {
    fn history(&self) -> AppResult<Vec<UpdateHistoryRow>>;

    /// Return `None` when a previous non-cancellable platform query is still
    /// finishing. Providers without that constraint use the ordinary history
    /// call and always return a result.
    fn try_history(&self) -> AppResult<Option<Vec<UpdateHistoryRow>>> {
        self.history().map(Some)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct UpdateKey {
    update_id: String,
    date_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct UpdateState {
    seen: BTreeSet<UpdateKey>,
    #[serde(default)]
    pending: Vec<UpdateHistoryRow>,
    established: bool,
}

/// Polls WUA history and emits each newly observed history entry once.
pub struct UpdateCollector<P, C = SystemObservationClock> {
    provider: P,
    clock: C,
    descriptor: CollectorDescriptor,
    state: AckedState<UpdateState>,
    context: Option<CollectorContext>,
    downtime: bool,
    downtime_clear_token: Option<CommitToken>,
}

impl<P: UpdateHistoryProvider> UpdateCollector<P, SystemObservationClock> {
    pub fn new(provider: P, descriptor: CollectorDescriptor) -> Self {
        Self::with_clock(provider, descriptor, SystemObservationClock)
    }
}

impl<P: UpdateHistoryProvider, C: ObservationClock> UpdateCollector<P, C> {
    pub fn with_clock(provider: P, descriptor: CollectorDescriptor, clock: C) -> Self {
        Self {
            provider,
            clock,
            descriptor,
            state: AckedState::new(UpdateState::default()),
            context: None,
            downtime: false,
            downtime_clear_token: None,
        }
    }

    fn stage(
        &mut self,
        token: CommitToken,
        next: UpdateState,
        now_ms: i64,
        observations: Vec<RawObservation>,
        clears_downtime: bool,
    ) -> AppResult<CollectorSnapshot> {
        let baseline = stateful::baseline_for(BASELINE_KIND, &next, now_ms)?;
        self.state.stage(token, next)?;
        self.downtime_clear_token = clears_downtime.then_some(token);
        Ok(CollectorSnapshot::Atomic {
            token,
            observations,
            relationships: Vec::new(),
            baseline: Some(baseline),
            config_snapshots: Vec::new(),
        })
    }
}

impl<P: UpdateHistoryProvider, C: ObservationClock> Collector for UpdateCollector<P, C> {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let (state, downtime) = stateful::hydrate(
            context.state.baseline(BASELINE_KIND)?,
            self.clock.now_ms(),
            "Windows Update history",
        );
        self.state = AckedState::new(state);
        self.context = Some(context);
        self.downtime = downtime;
        self.downtime_clear_token = None;
        Ok(())
    }

    fn stop(&mut self) {
        self.context = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Err(AppError::InvalidInput {
            what: "update collector",
            reason: "a stateful collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        let Some(cancel) = self.context.as_ref().map(|held| held.cancel.clone()) else {
            return Ok(CollectorSnapshot::Noop);
        };
        let now_ms = self.clock.now_ms();
        let Some(rows) = self.provider.try_history()? else {
            return Ok(CollectorSnapshot::Noop);
        };
        if cancel.is_cancelled() {
            return Ok(CollectorSnapshot::Noop);
        }

        if !self.state.committed().established {
            let seen = rows.iter().map(UpdateHistoryRow::key).collect();
            return self.stage(
                context.token(),
                UpdateState {
                    seen,
                    pending: Vec::new(),
                    established: true,
                },
                now_ms,
                Vec::new(),
                true,
            );
        }

        let mut pending = if self.state.committed().pending.is_empty() {
            let mut additions: Vec<_> = rows
                .into_iter()
                .filter(|row| !self.state.committed().seen.contains(&row.key()))
                .collect();
            // WUA returns newest first. Emit old-to-new for a stable timeline
            // and deterministic chunking when a machine accumulated a burst.
            additions.sort_by_key(|row| (row.date_ms, row.update_id.clone()));
            additions.dedup_by(|left, right| left.key() == right.key());
            additions
        } else {
            self.state.committed().pending.clone()
        };

        if pending.is_empty() {
            self.downtime = false;
            return Ok(CollectorSnapshot::Noop);
        }

        let take = pending.len().min(crate::storage::MAX_ATOMIC_OBSERVATIONS);
        let drained: Vec<_> = pending.drain(..take).collect();
        let mut seen = self.state.committed().seen.clone();
        let observations = drained
            .iter()
            .filter_map(|row| {
                seen.insert(row.key());
                observation(row, now_ms, self.downtime)
            })
            .collect();
        let clears_downtime = pending.is_empty();
        self.stage(
            context.token(),
            UpdateState {
                seen,
                pending,
                established: true,
            },
            now_ms,
            observations,
            clears_downtime,
        )
    }

    fn commit_succeeded(
        &mut self,
        token: CommitToken,
        _receipt: &CollectorCommitReceipt,
    ) -> AppResult<()> {
        self.state.ack(token)?;
        if self.downtime_clear_token == Some(token) {
            self.downtime = false;
        }
        self.downtime_clear_token = None;
        Ok(())
    }

    fn commit_failed(&mut self, token: CommitToken) -> AppResult<()> {
        let result = self.state.reject(token);
        if self.downtime_clear_token == Some(token) {
            self.downtime_clear_token = None;
        }
        result
    }
}

fn observation(
    row: &UpdateHistoryRow,
    detected_at_ms: i64,
    uncertain: bool,
) -> Option<RawObservation> {
    let kb = extract_kb(&row.title);
    let stable_id = kb
        .clone()
        .unwrap_or_else(|| crate::domain::ids::short_hash(&row.update_id));
    let label = kb
        .as_deref()
        .map(str::to_owned)
        .unwrap_or_else(|| fallback_label(row, &stable_id));
    let failed = matches!(row.result_code, RESULT_FAILED | RESULT_ABORTED);
    let succeeded = matches!(
        row.result_code,
        RESULT_SUCCEEDED | RESULT_SUCCEEDED_WITH_ERRORS
    );
    let (event_type, severity, user_friendly_summary) = if failed {
        (
            EventType::Failed,
            Severity::Error,
            format!(
                "Windows Update failed: {label} (error 0x{:08X})",
                row.hresult as u32
            ),
        )
    } else if succeeded && row.operation == OPERATION_UNINSTALLATION {
        (
            EventType::Uninstalled,
            Severity::Info,
            format!("Windows Update uninstalled {label}"),
        )
    } else if succeeded && row.operation == OPERATION_INSTALLATION {
        let severity = if row.result_code == RESULT_SUCCEEDED_WITH_ERRORS {
            Severity::Warning
        } else {
            Severity::Info
        };
        (
            EventType::Installed,
            severity,
            format!("Windows Update installed {label}"),
        )
    } else {
        return None;
    };

    Some(RawObservation {
        collector_id: ID,
        kind: ObservationKind {
            category: Category::Update,
            event_type,
            severity,
            confidence: Confidence::High,
            technical_summary: format!(
                "Windows Update {} operation={} result={} HRESULT=0x{:08X}",
                row.update_id, row.operation, row.result_code, row.hresult as u32
            ),
            user_friendly_summary,
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            identity: EntityIdentity::Update { id: stable_id },
            display_name: label,
        },
        before: None,
        after: Some(json!({
            "operation": operation_label(row.operation),
            "result": result_label(row.result_code),
        })),
        field: None,
        // WUA history can briefly report a timestamp ahead of the system clock
        // after a clock correction. Keep the original timestamp in the durable
        // de-duplication key, but do not let an event postdate the baseline that
        // acknowledges it: the atomic commit contract requires captured_at to
        // be at least the latest occurred_at in the batch.
        occurred_at: row.date_ms.min(detected_at_ms),
        details: Some(json!({
            "update_id": row.update_id,
            "revision": row.revision,
            "title": row.title,
            "result_code": row.result_code,
            "operation": row.operation,
            "hresult": format!("0x{:08X}", row.hresult as u32),
            "client_application_id": row.client_application_id,
        })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: uncertain || row.date_ms > detected_at_ms,
    })
}

fn fallback_label(row: &UpdateHistoryRow, stable_id: &str) -> String {
    let title = row.title.trim();
    if title.is_empty() {
        format!("update {stable_id}")
    } else {
        title.to_owned()
    }
}

/// Extract a parenthesized six- or seven-digit KB identifier from a WUA title.
pub fn extract_kb(title: &str) -> Option<String> {
    let bytes = title.as_bytes();
    (0..bytes.len()).find_map(|start| {
        for digits in [7usize, 6] {
            let end = start.saturating_add(digits + 4);
            let Some(candidate) = bytes.get(start..end) else {
                continue;
            };
            if candidate[0] == b'('
                && candidate[1..3] == *b"KB"
                && candidate[3..3 + digits].iter().all(u8::is_ascii_digit)
                && candidate[3 + digits] == b')'
            {
                return std::str::from_utf8(&candidate[1..3 + digits])
                    .ok()
                    .map(str::to_owned);
            }
        }
        None
    })
}

const fn operation_label(value: i32) -> &'static str {
    match value {
        OPERATION_INSTALLATION => "installation",
        OPERATION_UNINSTALLATION => "uninstallation",
        _ => "other",
    }
}

const fn result_label(value: i32) -> &'static str {
    match value {
        RESULT_SUCCEEDED => "succeeded",
        RESULT_SUCCEEDED_WITH_ERRORS => "succeeded_with_errors",
        RESULT_FAILED => "failed",
        RESULT_ABORTED => "aborted",
        _ => "other",
    }
}

/// Descriptor from the canonical collector catalog.
pub fn catalog_descriptor() -> AppResult<CollectorDescriptor> {
    crate::collectors::registry::CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == ID)
        .map(|entry| entry.descriptor.clone())
        .ok_or_else(|| AppError::InvalidInput {
            what: "collector catalog",
            reason: "the update collector is not in the canonical catalog".to_owned(),
        })
}
