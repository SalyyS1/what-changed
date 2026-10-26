use std::collections::BTreeMap;

use super::ServiceRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Installed(ServiceRecord),
    Removed(ServiceRecord),
    Started {
        before: ServiceRecord,
        after: ServiceRecord,
    },
    Stopped {
        before: ServiceRecord,
        after: ServiceRecord,
    },
    StartTypeChanged {
        before: ServiceRecord,
        after: ServiceRecord,
    },
}

pub fn between(
    before: &BTreeMap<String, ServiceRecord>,
    after: &BTreeMap<String, ServiceRecord>,
) -> Vec<Change> {
    let mut changes = Vec::new();
    for (key, old) in before {
        let Some(new) = after.get(key) else {
            changes.push(Change::Removed(old.clone()));
            continue;
        };
        match (old.state.is_running(), new.state.is_running()) {
            (false, true) => changes.push(Change::Started {
                before: old.clone(),
                after: new.clone(),
            }),
            (true, false) => changes.push(Change::Stopped {
                before: old.clone(),
                after: new.clone(),
            }),
            _ => {}
        }
        if old.config_available && new.config_available && old.start_type != new.start_type {
            changes.push(Change::StartTypeChanged {
                before: old.clone(),
                after: new.clone(),
            });
        }
    }
    for (key, new) in after {
        if !before.contains_key(key) {
            changes.push(Change::Installed(new.clone()));
        }
    }
    changes
}

/// Project one existing service through the same transition classifier used
/// by live collection. Checkpoint comparison deliberately ignores refresh-only
/// fields because [`between`] does too.
pub(super) fn checkpoint_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    let before: ServiceRecord = serde_json::from_value(before.clone()).ok()?;
    let after: ServiceRecord = serde_json::from_value(after.clone()).ok()?;
    let key = "checkpoint".to_owned();
    let before_map = [(key.clone(), before)].into_iter().collect();
    let after_map = [(key, after)].into_iter().collect();
    Some(
        between(&before_map, &after_map)
            .into_iter()
            .filter_map(|change| match change {
                Change::Started { before, after } | Change::Stopped { before, after } => Some((
                    "running".to_owned(),
                    serde_json::json!(before.state.is_running()),
                    serde_json::json!(after.state.is_running()),
                )),
                Change::StartTypeChanged { before, after } => Some((
                    "start_type".to_owned(),
                    serde_json::json!(before.start_type),
                    serde_json::json!(after.start_type),
                )),
                Change::Installed(_) | Change::Removed(_) => None,
            })
            .collect(),
    )
}

pub fn apply(state: &mut BTreeMap<String, ServiceRecord>, change: &Change) {
    match change {
        Change::Installed(record) => {
            state.insert(record.name.to_ascii_lowercase(), record.clone());
        }
        Change::Removed(record) => {
            state.remove(&record.name.to_ascii_lowercase());
        }
        Change::Started { after, .. } | Change::Stopped { after, .. } => {
            let key = after.name.to_ascii_lowercase();
            let mut advanced = after.clone();
            // A state transition and a start-type transition for the same
            // service are two facts. If a capacity boundary falls between
            // them, preserve the old start type so the latter is still present
            // in the next diff rather than being silently absorbed here.
            if let Some(previous) = state.get(&key) {
                advanced.start_type = previous.start_type;
            }
            state.insert(key, advanced);
        }
        Change::StartTypeChanged { after, .. } => {
            state.insert(after.name.to_ascii_lowercase(), after.clone());
        }
    }
}
