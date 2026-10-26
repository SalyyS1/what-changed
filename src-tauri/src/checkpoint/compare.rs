//! In-memory hash-join and field-level JSON comparison.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde_json::Value;

use crate::collectors::{
    applications, drivers, files, listeners, network, process, services, startup, updates,
};
use crate::domain::{
    category_for_collector, CategoryCheckpointDiff, CheckpointDiff, CheckpointDiffItem,
    CheckpointItem, CheckpointItemKey,
};

/// Compare two complete/partial state sets. Only collectors explicitly
/// captured on both sides participate, preventing phantom removals when a
/// collector failed or a valid-empty set has no rows.
#[allow(clippy::too_many_arguments)]
pub fn compare_item_sets(
    left_items: &[CheckpointItem],
    left_captured: &[String],
    left_missing: &[String],
    right_items: &[CheckpointItem],
    right_captured: &[String],
    right_missing: &[String],
    event_ids_in_window: Vec<i64>,
) -> CheckpointDiff {
    let left_manifest: BTreeSet<&str> = left_captured.iter().map(String::as_str).collect();
    let right_manifest: BTreeSet<&str> = right_captured.iter().map(String::as_str).collect();
    let incomplete: BTreeSet<&str> = left_missing
        .iter()
        .chain(right_missing.iter())
        .map(String::as_str)
        .collect();
    let comparable: BTreeSet<&str> = left_manifest
        .intersection(&right_manifest)
        .copied()
        .filter(|collector| !incomplete.contains(*collector))
        .filter(|collector| has_semantic_adapter(collector))
        .collect();
    let mut skipped_collectors: BTreeSet<String> = left_manifest
        .symmetric_difference(&right_manifest)
        .map(|collector| (*collector).to_owned())
        .collect();
    skipped_collectors.extend(left_missing.iter().cloned());
    skipped_collectors.extend(right_missing.iter().cloned());
    skipped_collectors.extend(
        left_manifest
            .union(&right_manifest)
            .filter(|collector| !has_semantic_adapter(collector))
            .map(|collector| (*collector).to_owned()),
    );

    let left = index(left_items, &comparable);
    let right = index(right_items, &comparable);
    let mut result = CheckpointDiff {
        event_ids_in_window,
        skipped_collectors,
        ..CheckpointDiff::default()
    };

    for (key, item) in &left {
        let Some(after) = right.get(key) else {
            result.removed.push(entity_diff(item, None, None));
            continue;
        };
        if item.collector_id == "process" && item.entity_kind == "process_instance" {
            if !process_continues(&item.state, &after.state) {
                result.removed.push(entity_diff(item, None, None));
                result.added.push(entity_diff(after, None, None));
            }
            continue;
        }
        // Most checkpoint rows are unchanged. Avoid reparsing their JSON and
        // running collector-specific semantic adapters when the persisted
        // state is byte-for-byte equal; this keeps large comparisons bounded
        // by the actual change set rather than the full baseline size.
        if item.state == after.state && item.entity_name == after.entity_name {
            continue;
        }
        let fields = semantic_fields(item, after);
        for (field, before, after_value) in fields {
            result
                .changed
                .push(entity_diff(after, Some(field), Some((before, after_value))));
        }
    }
    for (key, item) in &right {
        if !left.contains_key(key) {
            result.added.push(entity_diff(item, None, None));
        }
    }
    sort_items(&mut result.added);
    sort_items(&mut result.removed);
    sort_items(&mut result.changed);
    result.grouped_by_category = group_by_category(&result);
    result
}

fn semantic_fields(before: &CheckpointItem, after: &CheckpointItem) -> Vec<(String, Value, Value)> {
    match before.collector_id.as_str() {
        "services" => services::checkpoint_fields(&before.state, &after.state).unwrap_or_default(),
        "startup" => startup::checkpoint_fields(&before.entity_kind, &before.state, &after.state)
            .unwrap_or_default(),
        "applications" => {
            applications::checkpoint_fields(&before.state, &after.state).unwrap_or_default()
        }
        "drivers" => drivers::checkpoint_fields(&before.state, &after.state).unwrap_or_default(),
        "listeners" => {
            listeners::checkpoint_fields(&before.state, &after.state).unwrap_or_default()
        }
        "updates" => updates::checkpoint_fields(&before.state, &after.state).unwrap_or_default(),
        "files" => files::checkpoint_fields(&before.state, &after.state).unwrap_or_default(),
        "network" => {
            network::diff::checkpoint_fields(&before.entity_kind, &before.state, &after.state)
                .map(|changes| {
                    changes
                        .into_iter()
                        .map(|change| (change.field.to_owned(), change.before, change.after))
                        .collect()
                })
                .unwrap_or_default()
        }
        // Process continuity is handled before this function.
        "process" => Vec::new(),
        // Unsupported collectors are excluded before indexing. Reaching this
        // branch would mean a newly registered adapter was not routed here.
        _ => Vec::new(),
    }
}

fn has_semantic_adapter(collector: &str) -> bool {
    matches!(
        collector,
        "applications"
            | "drivers"
            | "files"
            | "listeners"
            | "network"
            | "process"
            | "services"
            | "startup"
            | "updates"
    )
}

fn process_continues(before: &Value, after: &Value) -> bool {
    #[derive(serde::Deserialize)]
    struct Identity {
        creation: Option<u64>,
        entity_id: crate::domain::ids::ProcessId,
    }
    let (Ok(before), Ok(after)) = (
        serde_json::from_value::<Identity>(before.clone()),
        serde_json::from_value::<Identity>(after.clone()),
    ) else {
        return false;
    };
    process::diff::same_identity(
        before.creation,
        &before.entity_id,
        after.creation,
        &after.entity_id,
    )
}

fn index<'a>(
    items: &'a [CheckpointItem],
    comparable: &BTreeSet<&str>,
) -> HashMap<CheckpointItemKey, &'a CheckpointItem> {
    let mut indexed = HashMap::with_capacity(items.len());
    indexed.extend(
        items
            .iter()
            .filter(|item| comparable.contains(item.collector_id.as_str()))
            .map(|item| (item.key(), item)),
    );
    indexed
}

fn entity_diff(
    item: &CheckpointItem,
    field: Option<String>,
    values: Option<(Value, Value)>,
) -> CheckpointDiffItem {
    let (before, after) = values
        .map(|(before, after)| (Some(before), Some(after)))
        .unwrap_or((None, None));
    CheckpointDiffItem {
        collector_id: item.collector_id.clone(),
        category: category_for_collector(&item.collector_id),
        entity_kind: item.entity_kind.clone(),
        entity_external_id: item.entity_external_id.clone(),
        entity_name: item
            .entity_name
            .clone()
            .unwrap_or_else(|| item.entity_external_id.clone()),
        field,
        before,
        after,
    }
}

fn sort_items(items: &mut [CheckpointDiffItem]) {
    items.sort_by(|left, right| {
        (
            left.category,
            &left.entity_name,
            &left.entity_external_id,
            &left.field,
        )
            .cmp(&(
                right.category,
                &right.entity_name,
                &right.entity_external_id,
                &right.field,
            ))
    });
}

fn group_by_category(
    result: &CheckpointDiff,
) -> BTreeMap<crate::domain::Category, CategoryCheckpointDiff> {
    let mut groups: BTreeMap<_, CategoryCheckpointDiff> = BTreeMap::new();
    for item in &result.added {
        groups
            .entry(item.category)
            .or_default()
            .added
            .push(item.clone());
    }
    for item in &result.removed {
        groups
            .entry(item.category)
            .or_default()
            .removed
            .push(item.clone());
    }
    for item in &result.changed {
        groups
            .entry(item.category)
            .or_default()
            .changed
            .push(item.clone());
    }
    groups
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn item(collector_id: &str, id: &str, state: Value) -> CheckpointItem {
        CheckpointItem {
            collector_id: collector_id.to_owned(),
            entity_kind: "service".to_owned(),
            entity_external_id: id.to_owned(),
            entity_name: Some(id.to_owned()),
            state,
        }
    }

    #[test]
    fn unknown_collectors_are_skipped_instead_of_compared_structurally() {
        let left = [
            item("future", "removed", json!({"state": "running"})),
            item("future", "changed", json!({"state": "running"})),
        ];
        let right = [
            item("future", "added", json!({"state": "running"})),
            item("future", "changed", json!({"state": "stopped"})),
        ];
        let manifest = ["future".to_owned()];
        let diff = compare_item_sets(&left, &manifest, &[], &right, &manifest, &[], vec![7]);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.changed.is_empty());
        assert_eq!(diff.event_ids_in_window, [7]);
        assert!(diff.skipped_collectors.contains("future"));
    }

    #[test]
    fn skips_a_collector_missing_from_either_side() {
        let left = [item("services", "service", json!({"state": "running"}))];
        let diff = compare_item_sets(
            &left,
            &["services".to_owned()],
            &[],
            &[],
            &[],
            &["services".to_owned()],
            vec![],
        );
        assert!(diff.removed.is_empty());
        assert!(diff.skipped_collectors.contains("services"));
    }
}
