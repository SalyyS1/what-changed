use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use super::DriverRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionDirection {
    Increased,
    Decreased,
    Equal,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Change {
    Installed(DriverRecord),
    Removed(DriverRecord),
    Updated {
        before: DriverRecord,
        after: DriverRecord,
        direction: VersionDirection,
    },
    Metadata(DriverRecord),
}

pub(super) fn changes(
    before: &BTreeMap<String, DriverRecord>,
    after: &BTreeMap<String, DriverRecord>,
) -> Vec<Change> {
    let keys: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    keys.into_iter()
        .filter_map(|key| match (before.get(key), after.get(key)) {
            (None, Some(row)) => Some(Change::Installed(row.clone())),
            (Some(row), None) => Some(Change::Removed(row.clone())),
            (Some(old), Some(new)) if old.driver_version != new.driver_version => {
                Some(Change::Updated {
                    before: old.clone(),
                    after: new.clone(),
                    direction: compare_versions(
                        old.driver_version.as_deref(),
                        new.driver_version.as_deref(),
                    ),
                })
            }
            (Some(old), Some(new)) if old != new => Some(Change::Metadata(new.clone())),
            _ => None,
        })
        .collect()
}

/// Compare an existing durable driver through the live mutation classifier.
/// Device metadata remains a silent baseline refresh in both paths.
pub(super) fn checkpoint_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    let before: DriverRecord = serde_json::from_value(before.clone()).ok()?;
    let after: DriverRecord = serde_json::from_value(after.clone()).ok()?;
    let key = "checkpoint".to_owned();
    let before_map = [(key.clone(), before)].into_iter().collect();
    let after_map = [(key, after)].into_iter().collect();
    Some(
        changes(&before_map, &after_map)
            .into_iter()
            .filter_map(|change| match change {
                Change::Updated { before, after, .. } => Some((
                    "version".to_owned(),
                    serde_json::json!(before.driver_version),
                    serde_json::json!(after.driver_version),
                )),
                Change::Installed(_) | Change::Removed(_) | Change::Metadata(_) => None,
            })
            .collect(),
    )
}

pub(super) fn apply(state: &mut BTreeMap<String, DriverRecord>, change: &Change) {
    match change {
        Change::Installed(row) | Change::Metadata(row) => {
            state.insert(row.key(), row.clone());
        }
        Change::Removed(row) => {
            state.remove(&row.key());
        }
        Change::Updated { after, .. } => {
            state.insert(after.key(), after.clone());
        }
    }
}

/// Compare dotted numeric tuples, falling back to case-insensitive lexical
/// order only when both non-empty versions are non-numeric. Missing versions
/// have unknown direction and can never create a rollback claim.
pub fn compare_versions(before: Option<&str>, after: Option<&str>) -> VersionDirection {
    let (Some(before), Some(after)) = (trimmed(before), trimmed(after)) else {
        return VersionDirection::Unknown;
    };
    let ordering = match (numeric_tuple(before), numeric_tuple(after)) {
        (Some(before), Some(after)) => compare_numeric(&before, &after),
        _ => before.to_ascii_lowercase().cmp(&after.to_ascii_lowercase()),
    };
    match ordering {
        Ordering::Less => VersionDirection::Increased,
        Ordering::Greater => VersionDirection::Decreased,
        Ordering::Equal => VersionDirection::Equal,
    }
}

fn trimmed(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn numeric_tuple(value: &str) -> Option<Vec<u64>> {
    value
        .split('.')
        .map(|part| part.parse::<u64>().ok())
        .collect()
}

fn compare_numeric(before: &[u64], after: &[u64]) -> Ordering {
    let length = before.len().max(after.len());
    (0..length)
        .map(|index| {
            before
                .get(index)
                .copied()
                .unwrap_or(0)
                .cmp(&after.get(index).copied().unwrap_or(0))
        })
        .find(|ordering| *ordering != Ordering::Equal)
        .unwrap_or(Ordering::Equal)
}
