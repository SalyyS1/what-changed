//! Durable known-good checkpoints and collector-neutral comparison shapes.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use super::Category;
use crate::privacy::{Redaction, Sanitizer, Scrub};

/// How a checkpoint was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointKind {
    /// Explicitly requested by a person.
    Manual,
    /// Created by a bounded automatic trigger.
    Automatic,
}

impl CheckpointKind {
    /// Stable SQLite representation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Automatic => "automatic",
        }
    }
}

impl std::fmt::Display for CheckpointKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CheckpointKind {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "manual" => Ok(Self::Manual),
            "automatic" => Ok(Self::Automatic),
            _ => Err(()),
        }
    }
}

/// Stored checkpoint metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: i64,
    pub name: String,
    pub kind: CheckpointKind,
    pub reason: Option<String>,
    pub created_at: i64,
    pub item_count: u64,
    pub duration_ms: u64,
    pub partial: bool,
    pub missing_collectors: Vec<String>,
    pub captured_collectors: Vec<String>,
}

/// Metadata supplied when atomically creating a checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewCheckpoint {
    pub name: String,
    pub kind: CheckpointKind,
    pub reason: Option<String>,
    pub created_at: i64,
    pub duration_ms: u64,
    pub missing_collectors: Vec<String>,
    pub captured_collectors: Vec<String>,
}

/// One collector entity stored in a checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointItem {
    pub collector_id: String,
    pub entity_kind: String,
    pub entity_external_id: String,
    pub entity_name: Option<String>,
    pub state: Value,
}

/// The only value accepted by checkpoint persistence. Wrapping this whole
/// aggregate in `Sanitized` makes redaction a compile-time write precondition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointCapture {
    pub checkpoint: NewCheckpoint,
    pub items: Vec<CheckpointItem>,
}

impl Scrub for CheckpointCapture {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        let prefix = if field.is_empty() {
            "checkpoint"
        } else {
            field
        };
        self.checkpoint
            .name
            .scrub_in_place(sanitizer, &format!("{prefix}.name"), report);
        self.checkpoint
            .reason
            .scrub_in_place(sanitizer, &format!("{prefix}.reason"), report);
        for (index, item) in self.items.iter_mut().enumerate() {
            let item_field = format!("{prefix}.items[{index}]");
            item.entity_name.scrub_in_place(
                sanitizer,
                &format!("{item_field}.entity_name"),
                report,
            );
            item.state
                .scrub_in_place(sanitizer, &format!("{item_field}.state"), report);
        }
    }
}

/// Stable identity for a state row within a collector.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckpointItemKey {
    pub collector_id: String,
    pub entity_kind: String,
    pub entity_external_id: String,
}

impl CheckpointItem {
    pub fn key(&self) -> CheckpointItemKey {
        CheckpointItemKey {
            collector_id: self.collector_id.clone(),
            entity_kind: self.entity_kind.clone(),
            entity_external_id: self.entity_external_id.clone(),
        }
    }
}

/// Whether the right side comes from the live collector sweep or storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", content = "checkpoint_id", rename_all = "snake_case")]
pub enum CheckpointComparisonTarget {
    Now,
    Checkpoint(i64),
}

/// One structural difference. `field` is absent for whole-entity additions
/// and removals and is a JSON path for field-level changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointDiffItem {
    pub collector_id: String,
    pub category: Category,
    pub entity_kind: String,
    pub entity_external_id: String,
    pub entity_name: String,
    pub field: Option<String>,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// Differences for one category, duplicated from the flat lists by design so
/// IPC consumers can render either the three sections or category groups.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CategoryCheckpointDiff {
    pub added: Vec<CheckpointDiffItem>,
    pub removed: Vec<CheckpointDiffItem>,
    pub changed: Vec<CheckpointDiffItem>,
}

/// Collector-neutral comparison result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckpointDiff {
    pub added: Vec<CheckpointDiffItem>,
    pub removed: Vec<CheckpointDiffItem>,
    pub changed: Vec<CheckpointDiffItem>,
    pub grouped_by_category: BTreeMap<Category, CategoryCheckpointDiff>,
    pub event_ids_in_window: Vec<i64>,
    pub skipped_collectors: BTreeSet<String>,
    /// Collectors unavailable or incomplete on the historical/left side.
    pub left_missing_collectors: BTreeSet<String>,
    /// Collectors unavailable or incomplete on the comparison/right side.
    pub right_missing_collectors: BTreeSet<String>,
    /// Inclusive event-window start used by the Timeline deep link.
    pub window_from: i64,
    /// Inclusive event-window end used by the Timeline deep link.
    pub window_to: i64,
}

/// Category used when the collector does not yet expose richer metadata.
pub fn category_for_collector(collector_id: &str) -> Category {
    match collector_id {
        "applications" => Category::Application,
        "process" | "processes" => Category::Process,
        "files" => Category::Filesystem,
        "network" | "listeners" => Category::Network,
        "services" => Category::Service,
        "drivers" => Category::Driver,
        "updates" => Category::Update,
        "docker" => Category::Docker,
        "startup" => Category::Startup,
        "eventlog" => Category::System,
        _ => Category::System,
    }
}
