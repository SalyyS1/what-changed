use serde_json::json;

use crate::collectors::{EntityIdentity, ObservationKind, RawEntity, RawObservation};
use crate::domain::{Category, ClassifierContext, Confidence, EventType, Severity};

use super::diff::Change;
use super::state::{RunEntry, ScheduledTask, StartupFile};

pub(super) fn for_change(
    change: &Change,
    occurred_at: i64,
    uncertain: bool,
) -> Option<RawObservation> {
    match change {
        Change::TaskAdded(task) => Some(task_observation(
            task,
            EventType::Created,
            format!("{} task added", task.name),
            None,
            task_value(task),
            occurred_at,
            uncertain,
            "startup/task_added",
            "added",
        )),
        Change::TaskRemoved(task) => Some(task_observation(
            task,
            EventType::Deleted,
            format!("{} task removed", task.name),
            task_value(task),
            None,
            occurred_at,
            uncertain,
            "startup/task_removed",
            "removed",
        )),
        Change::TaskEnabled(task) => Some(task_observation(
            task,
            EventType::Enabled,
            format!("{} task enabled", task.name),
            Some(json!(false)),
            Some(json!(true)),
            occurred_at,
            uncertain,
            "startup/task_enabled",
            "enabled",
        )),
        Change::TaskDisabled(task) => Some(task_observation(
            task,
            EventType::Disabled,
            format!("{} task disabled", task.name),
            Some(json!(true)),
            Some(json!(false)),
            occurred_at,
            uncertain,
            "startup/task_disabled",
            "disabled",
        )),
        Change::TaskChanged { before, after } => Some(task_observation(
            after,
            EventType::Modified,
            format!("{} task changed", after.name),
            task_value(before),
            task_value(after),
            occurred_at,
            uncertain,
            "startup/task_changed",
            "modified",
        )),
        Change::RunAdded(entry) => Some(run_observation(entry, true, occurred_at, uncertain)),
        Change::RunRemoved(entry) => Some(run_observation(entry, false, occurred_at, uncertain)),
        Change::RunChanged { before, after } => Some(changed_item_observation(
            after.identity(),
            after.name.clone(),
            after.location.label(),
            json!({ "command": before.command, "approved": before.approved }),
            json!({ "command": after.command, "approved": after.approved }),
            occurred_at,
            uncertain,
            "startup/run_key_changed",
        )),
        Change::StartupFileAdded(file) => {
            Some(file_observation(file, true, occurred_at, uncertain))
        }
        Change::StartupFileRemoved(file) => {
            Some(file_observation(file, false, occurred_at, uncertain))
        }
        Change::StartupFileChanged { before, after } => Some(changed_item_observation(
            after.identity(),
            after.name.clone(),
            after.location.label(),
            json!({ "path": before.path, "approved": before.approved }),
            json!({ "path": after.path, "approved": after.approved }),
            occurred_at,
            uncertain,
            "startup/run_key_changed",
        )),
        Change::TaskRuntimeMetadata(_) => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn task_observation(
    task: &ScheduledTask,
    event_type: EventType,
    summary: String,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    occurred_at: i64,
    uncertain: bool,
    event_name: &'static str,
    change_kind: &'static str,
) -> RawObservation {
    RawObservation {
        collector_id: super::ID,
        kind: ObservationKind {
            category: Category::Startup,
            event_type,
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary: summary,
            technical_summary: format!("scheduled task {} changed", task.path),
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            identity: EntityIdentity::Task {
                path: task.path.clone(),
            },
            display_name: task.name.clone(),
        },
        before,
        after,
        field: None,
        occurred_at,
        details: Some(json!({
            "event_name": event_name,
            "change_kind": change_kind,
            "source": "scheduled_task",
            "startup": task.is_startup(),
            "triggers": task.triggers,
            "action_path": task.action_path,
            "last_run_ms": task.last_run_ms,
            "last_result": task.last_result,
        })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: uncertain,
    }
}

fn task_value(task: &ScheduledTask) -> Option<serde_json::Value> {
    Some(json!({
        "enabled": task.enabled,
        "triggers": task.triggers,
        "action_path": task.action_path,
    }))
}

fn run_observation(
    entry: &RunEntry,
    added: bool,
    occurred_at: i64,
    uncertain: bool,
) -> RawObservation {
    let value = json!({ "command": entry.command, "approved": entry.approved });
    item_observation(
        entry.identity(),
        entry.name.clone(),
        entry.location.label(),
        added,
        value,
        occurred_at,
        uncertain,
        if added {
            "startup/run_key_added"
        } else {
            "startup/run_key_removed"
        },
    )
}

fn file_observation(
    file: &StartupFile,
    added: bool,
    occurred_at: i64,
    uncertain: bool,
) -> RawObservation {
    item_observation(
        file.identity(),
        file.name.clone(),
        file.location.label(),
        added,
        json!({ "path": file.path, "approved": file.approved }),
        occurred_at,
        uncertain,
        if added {
            "startup/run_key_added"
        } else {
            "startup/run_key_removed"
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn item_observation(
    identity: String,
    name: String,
    location: &str,
    added: bool,
    value: serde_json::Value,
    occurred_at: i64,
    uncertain: bool,
    event_name: &'static str,
) -> RawObservation {
    let verb = if added { "added" } else { "removed" };
    RawObservation {
        collector_id: super::ID,
        kind: ObservationKind {
            category: Category::Startup,
            event_type: if added {
                EventType::Created
            } else {
                EventType::Deleted
            },
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary: format!("{name} startup entry {verb}"),
            technical_summary: format!("{location} entry {name} {verb}"),
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            identity: EntityIdentity::Task { path: identity },
            display_name: name,
        },
        before: (!added).then_some(value.clone()),
        after: added.then_some(value),
        field: None,
        occurred_at,
        details: Some(json!({
            "event_name": event_name,
            "change_kind": if added { "added" } else { "removed" },
            "source": location,
        })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: uncertain,
    }
}

#[allow(clippy::too_many_arguments)]
fn changed_item_observation(
    identity: String,
    name: String,
    location: &str,
    before: serde_json::Value,
    after: serde_json::Value,
    occurred_at: i64,
    uncertain: bool,
    event_name: &'static str,
) -> RawObservation {
    RawObservation {
        collector_id: super::ID,
        kind: ObservationKind {
            category: Category::Startup,
            event_type: EventType::ConfigurationChanged,
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary: format!("{name} startup entry changed"),
            technical_summary: format!("{location} entry {name} changed"),
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            identity: EntityIdentity::Task { path: identity },
            display_name: name,
        },
        before: Some(before),
        after: Some(after),
        field: None,
        occurred_at,
        details: Some(json!({
            "event_name": event_name,
            "change_kind": "modified",
            "source": location,
        })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: uncertain,
    }
}
