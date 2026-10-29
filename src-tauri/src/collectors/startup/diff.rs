use std::collections::BTreeSet;

use super::state::{IndexedInventory, RunEntry, ScheduledTask, StartupFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Change {
    TaskAdded(ScheduledTask),
    TaskRemoved(ScheduledTask),
    TaskEnabled(ScheduledTask),
    TaskDisabled(ScheduledTask),
    TaskChanged {
        before: ScheduledTask,
        after: ScheduledTask,
    },
    TaskRuntimeMetadata(ScheduledTask),
    RunAdded(RunEntry),
    RunRemoved(RunEntry),
    RunChanged {
        before: RunEntry,
        after: RunEntry,
    },
    StartupFileAdded(StartupFile),
    StartupFileRemoved(StartupFile),
    StartupFileChanged {
        before: StartupFile,
        after: StartupFile,
    },
}

pub(super) fn changes(before: &IndexedInventory, after: &IndexedInventory) -> Vec<Change> {
    let mut changes = task_changes(before, after);
    let run_keys: BTreeSet<&String> = before
        .run_entries
        .keys()
        .chain(after.run_entries.keys())
        .collect();
    for key in run_keys {
        match (before.run_entries.get(key), after.run_entries.get(key)) {
            (None, Some(entry)) => changes.push(Change::RunAdded(entry.clone())),
            (Some(entry), None) => changes.push(Change::RunRemoved(entry.clone())),
            (Some(old), Some(new)) if old != new => changes.push(Change::RunChanged {
                before: old.clone(),
                after: new.clone(),
            }),
            _ => {}
        }
    }
    let file_keys: BTreeSet<&String> = before
        .startup_files
        .keys()
        .chain(after.startup_files.keys())
        .collect();
    for key in file_keys {
        match (before.startup_files.get(key), after.startup_files.get(key)) {
            (None, Some(file)) => changes.push(Change::StartupFileAdded(file.clone())),
            (Some(file), None) => changes.push(Change::StartupFileRemoved(file.clone())),
            (Some(old), Some(new)) if old != new => changes.push(Change::StartupFileChanged {
                before: old.clone(),
                after: new.clone(),
            }),
            _ => {}
        }
    }
    changes
}

fn task_changes(before: &IndexedInventory, after: &IndexedInventory) -> Vec<Change> {
    let keys: BTreeSet<&String> = before.tasks.keys().chain(after.tasks.keys()).collect();
    keys.into_iter()
        .filter_map(|key| match (before.tasks.get(key), after.tasks.get(key)) {
            (None, Some(task)) => Some(Change::TaskAdded(task.clone())),
            (Some(task), None) => Some(Change::TaskRemoved(task.clone())),
            (Some(old), Some(new)) if old.enabled != new.enabled => Some(if new.enabled {
                Change::TaskEnabled(new.clone())
            } else {
                Change::TaskDisabled(new.clone())
            }),
            (Some(old), Some(new)) if configuration_changed(old, new) => {
                Some(Change::TaskChanged {
                    before: old.clone(),
                    after: new.clone(),
                })
            }
            (Some(old), Some(new)) if old != new => Some(Change::TaskRuntimeMetadata(new.clone())),
            _ => None,
        })
        .collect()
}

fn configuration_changed(old: &ScheduledTask, new: &ScheduledTask) -> bool {
    old.name != new.name
        || old.path != new.path
        || old.triggers != new.triggers
        || old.action_path != new.action_path
}

/// Project one durable startup entity through the live diff precedence.
/// Runtime-only task metadata therefore produces no checkpoint field change.
pub(super) fn checkpoint_fields(
    entity_kind: &str,
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    let mut before_inventory = IndexedInventory::default();
    let mut after_inventory = IndexedInventory::default();
    match entity_kind {
        "task" => {
            let before_task: ScheduledTask = serde_json::from_value(before.clone()).ok()?;
            let after_task: ScheduledTask = serde_json::from_value(after.clone()).ok()?;
            before_inventory
                .tasks
                .insert("checkpoint".to_owned(), before_task);
            after_inventory
                .tasks
                .insert("checkpoint".to_owned(), after_task);
        }
        "run_entry" => {
            let before_entry: RunEntry = serde_json::from_value(before.clone()).ok()?;
            let after_entry: RunEntry = serde_json::from_value(after.clone()).ok()?;
            before_inventory
                .run_entries
                .insert("checkpoint".to_owned(), before_entry);
            after_inventory
                .run_entries
                .insert("checkpoint".to_owned(), after_entry);
        }
        "startup_file" => {
            let before_file: StartupFile = serde_json::from_value(before.clone()).ok()?;
            let after_file: StartupFile = serde_json::from_value(after.clone()).ok()?;
            before_inventory
                .startup_files
                .insert("checkpoint".to_owned(), before_file);
            after_inventory
                .startup_files
                .insert("checkpoint".to_owned(), after_file);
        }
        _ => return None,
    }

    Some(
        changes(&before_inventory, &after_inventory)
            .into_iter()
            .filter_map(|change| match change {
                Change::TaskEnabled(_) | Change::TaskDisabled(_) => Some((
                    "enabled".to_owned(),
                    before
                        .get("enabled")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                    after
                        .get("enabled")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                )),
                Change::TaskChanged { before, after } => Some((
                    "configuration".to_owned(),
                    task_configuration(&before),
                    task_configuration(&after),
                )),
                Change::RunChanged { .. } | Change::StartupFileChanged { .. } => {
                    Some(("configuration".to_owned(), before.clone(), after.clone()))
                }
                Change::TaskRuntimeMetadata(_)
                | Change::TaskAdded(_)
                | Change::TaskRemoved(_)
                | Change::RunAdded(_)
                | Change::RunRemoved(_)
                | Change::StartupFileAdded(_)
                | Change::StartupFileRemoved(_) => None,
            })
            .collect(),
    )
}

fn task_configuration(task: &ScheduledTask) -> serde_json::Value {
    serde_json::json!({
        "name": task.name,
        "path": task.path,
        "triggers": task.triggers,
        "action_path": task.action_path,
    })
}

pub(super) fn apply(state: &mut IndexedInventory, change: &Change) {
    match change {
        Change::TaskAdded(task)
        | Change::TaskEnabled(task)
        | Change::TaskDisabled(task)
        | Change::TaskRuntimeMetadata(task) => {
            state.tasks.insert(task.key(), task.clone());
        }
        Change::TaskChanged { after, .. } => {
            state.tasks.insert(after.key(), after.clone());
        }
        Change::TaskRemoved(task) => {
            state.tasks.remove(&task.key());
        }
        Change::RunAdded(entry) => {
            state.run_entries.insert(entry.key(), entry.clone());
        }
        Change::RunRemoved(entry) => {
            state.run_entries.remove(&entry.key());
        }
        Change::RunChanged { after, .. } => {
            state.run_entries.insert(after.key(), after.clone());
        }
        Change::StartupFileAdded(file) => {
            state.startup_files.insert(file.key(), file.clone());
        }
        Change::StartupFileRemoved(file) => {
            state.startup_files.remove(&file.key());
        }
        Change::StartupFileChanged { after, .. } => {
            state.startup_files.insert(after.key(), after.clone());
        }
    }
}
