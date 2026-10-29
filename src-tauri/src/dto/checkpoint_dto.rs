//! Checkpoint payloads. Shapes owned by phase 14; the commands that return
//! them are registered now so the typed surface is closed from the start and a
//! later phase adds a body rather than an entry point.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{Category, Checkpoint, CheckpointDiff, CheckpointDiffItem, CheckpointKind};

/// A known-good marker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CheckpointDto {
    /// Row id.
    pub id: i64,
    /// What the user called it, or the generated name.
    pub name: String,
    /// Manual or bounded automatic capture.
    pub kind: CheckpointKind,
    /// Why it was created, when it was automatic.
    pub reason: Option<String>,
    /// When, epoch milliseconds UTC.
    pub created_at: i64,
    /// How many items it captured.
    pub item_count: u64,
    /// How long the capture took.
    pub duration_ms: u64,
    /// Whether a collector failed or timed out during capture.
    pub partial: bool,
    /// Which collectors are missing from it, if it is partial.
    pub missing_collectors: Vec<String>,
}

impl From<Checkpoint> for CheckpointDto {
    fn from(value: Checkpoint) -> Self {
        Self {
            id: value.id,
            name: value.name,
            kind: value.kind,
            reason: value.reason,
            created_at: value.created_at,
            item_count: value.item_count,
            duration_ms: value.duration_ms,
            partial: value.partial,
            missing_collectors: value.missing_collectors,
        }
    }
}

/// One difference between two moments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DiffItemDto {
    /// What part of the machine.
    pub category: Category,
    /// The thing that differs.
    pub entity_display_name: String,
    /// Which field of it, when the item is a change rather than an
    /// appearance or a disappearance.
    pub field: Option<String>,
    /// Value before. Already redacted.
    pub before: Option<String>,
    /// Value after. Already redacted.
    pub after: Option<String>,
}

impl From<CheckpointDiffItem> for DiffItemDto {
    fn from(value: CheckpointDiffItem) -> Self {
        Self {
            category: value.category,
            entity_display_name: value.entity_name,
            field: value.field,
            before: value.before.map(compact_json),
            after: value.after.map(compact_json),
        }
    }
}

fn compact_json(value: serde_json::Value) -> String {
    serde_json::to_string(&value).unwrap_or_else(|_| "null".to_owned())
}

/// One category group for direct comparison-view rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CheckpointCategoryDiffDto {
    pub category: Category,
    pub added: Vec<DiffItemDto>,
    pub removed: Vec<DiffItemDto>,
    pub changed: Vec<DiffItemDto>,
}

/// What changed between a checkpoint and another moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CheckpointDiffDto {
    /// Things that exist now and did not then.
    pub added: Vec<DiffItemDto>,
    /// Things that existed then and do not now.
    pub removed: Vec<DiffItemDto>,
    /// Things that exist in both and differ.
    pub changed: Vec<DiffItemDto>,
    /// The same rows partitioned for category-first rendering.
    pub grouped_by_category: Vec<CheckpointCategoryDiffDto>,
    /// Events recorded between the two moments, so the structural diff can be
    /// opened as a timeline range rather than read as a list of assertions.
    pub event_ids_in_window: Vec<i64>,
    /// Inclusive beginning of the event window, epoch milliseconds UTC.
    pub window_from: i64,
    /// Inclusive end of the event window, epoch milliseconds UTC.
    pub window_to: i64,
    /// Collectors unavailable from the historical (left) side.
    pub left_missing_collectors: Vec<String>,
    /// Collectors unavailable from the comparison (right) side.
    pub right_missing_collectors: Vec<String>,
    /// Collectors omitted because either side did not capture them.
    pub skipped_collectors: Vec<String>,
}

impl From<CheckpointDiff> for CheckpointDiffDto {
    fn from(value: CheckpointDiff) -> Self {
        let grouped_by_category = value
            .grouped_by_category
            .into_iter()
            .map(|(category, group)| CheckpointCategoryDiffDto {
                category,
                added: group.added.into_iter().map(Into::into).collect(),
                removed: group.removed.into_iter().map(Into::into).collect(),
                changed: group.changed.into_iter().map(Into::into).collect(),
            })
            .collect();
        Self {
            added: value.added.into_iter().map(Into::into).collect(),
            removed: value.removed.into_iter().map(Into::into).collect(),
            changed: value.changed.into_iter().map(Into::into).collect(),
            grouped_by_category,
            event_ids_in_window: value.event_ids_in_window,
            window_from: value.window_from,
            window_to: value.window_to,
            left_missing_collectors: value.left_missing_collectors.into_iter().collect(),
            right_missing_collectors: value.right_missing_collectors.into_iter().collect(),
            skipped_collectors: value.skipped_collectors.into_iter().collect(),
        }
    }
}
