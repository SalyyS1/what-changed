use std::collections::{BTreeMap, BTreeSet};

use super::{TrackedApplication, UninstallEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Change {
    Installed(TrackedApplication),
    Removed(TrackedApplication),
    Updated {
        before: TrackedApplication,
        after: TrackedApplication,
    },
    Metadata(TrackedApplication),
}

pub(super) fn tracked(rows: &[UninstallEntry]) -> BTreeMap<String, TrackedApplication> {
    rows.iter()
        .filter_map(|row| {
            let display_name = row.display_name.trim();
            if row.system_component || display_name.is_empty() {
                return None;
            }
            let app = TrackedApplication {
                hive: row.hive,
                subkey_name: row.subkey_name.clone(),
                display_name: display_name.to_owned(),
                display_version: normalized(&row.display_version),
                publisher: normalized(&row.publisher),
                install_date: normalized(&row.install_date),
                install_location: normalized(&row.install_location),
            };
            Some((app.key(), app))
        })
        .collect()
}

fn normalized(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub(super) fn changes(
    before: &BTreeMap<String, TrackedApplication>,
    after: &BTreeMap<String, TrackedApplication>,
) -> Vec<Change> {
    let keys: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    keys.into_iter()
        .filter_map(|key| match (before.get(key), after.get(key)) {
            (None, Some(app)) => Some(Change::Installed(app.clone())),
            (Some(app), None) => Some(Change::Removed(app.clone())),
            (Some(old), Some(new)) if old.display_version != new.display_version => {
                Some(Change::Updated {
                    before: old.clone(),
                    after: new.clone(),
                })
            }
            (Some(old), Some(new)) if durable_metadata_changed(old, new) => {
                Some(Change::Metadata(new.clone()))
            }
            _ => None,
        })
        .collect()
}

/// Compare an existing durable application through the live mutation
/// classifier. Metadata refreshes remain baseline-only in both paths.
pub(super) fn checkpoint_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    let before: TrackedApplication = serde_json::from_value(before.clone()).ok()?;
    let after: TrackedApplication = serde_json::from_value(after.clone()).ok()?;
    let key = "checkpoint".to_owned();
    let before_map = [(key.clone(), before)].into_iter().collect();
    let after_map = [(key, after)].into_iter().collect();
    Some(
        changes(&before_map, &after_map)
            .into_iter()
            .filter_map(|change| match change {
                Change::Updated { before, after } => Some((
                    "version".to_owned(),
                    serde_json::json!(before.display_version),
                    serde_json::json!(after.display_version),
                )),
                Change::Installed(_) | Change::Removed(_) | Change::Metadata(_) => None,
            })
            .collect(),
    )
}

fn durable_metadata_changed(old: &TrackedApplication, new: &TrackedApplication) -> bool {
    old.hive != new.hive
        || old.subkey_name != new.subkey_name
        || old.display_name != new.display_name
        || old.publisher != new.publisher
        || old.install_date != new.install_date
    // Install locations are session-only relationship evidence. Their absence
    // after baseline hydration must not manufacture a metadata change.
}

pub(super) fn apply(state: &mut BTreeMap<String, TrackedApplication>, change: &Change) {
    match change {
        Change::Installed(app) | Change::Metadata(app) => {
            state.insert(app.key(), app.clone());
        }
        Change::Removed(app) => {
            state.remove(&app.key());
        }
        Change::Updated { after, .. } => {
            state.insert(after.key(), after.clone());
        }
    }
}
