//! Convert durable collector state into compact, collector-neutral checkpoint items.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::collectors::{applications, drivers, listeners, services, startup, updates};
use crate::domain::ids::ProcessId;
use crate::domain::CheckpointItem;
use crate::storage::{config_snapshot_repository, Baseline, ConfigSnapshot};

/// Items extracted from durable state, plus collectors whose durable view was
/// not complete enough to compare without inventing removals.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DurableStateItems {
    pub items: Vec<CheckpointItem>,
    pub incomplete_collectors: Vec<String>,
    /// Structural collectors with their exact, collector-owned durable
    /// baseline row present. This is separate from `items`: an acknowledged
    /// empty machine state is a valid complete baseline with zero items.
    pub complete_baseline_collectors: Vec<String>,
}

/// Convert the acknowledged state produced by completed collector cycles.
pub fn from_durable_state(
    baselines: Vec<Baseline>,
    config_snapshots: Vec<ConfigSnapshot>,
) -> DurableStateItems {
    let baseline_collectors: BTreeSet<String> = baselines
        .iter()
        .filter(|baseline| {
            expected_baseline_kind(&baseline.collector_id)
                .is_some_and(|kind| baseline.entity_kind == kind)
        })
        .map(|baseline| baseline.collector_id.clone())
        .collect();
    let mut incomplete: BTreeSet<String> = baselines
        .iter()
        .filter(|baseline| baseline.truncated || has_pending_work(&baseline.snapshot))
        .map(|baseline| baseline.collector_id.clone())
        .collect();
    if config_snapshots
        .iter()
        .any(|snapshot| snapshot.format.ends_with("+truncated"))
    {
        // A config snapshot is a per-file row, but `files` is the comparison
        // unit. One truncated file means the durable collector view is partial;
        // excluding the whole collector avoids phantom key removals.
        incomplete.insert("files".to_owned());
    }
    let mut items = Vec::new();
    for baseline in baselines {
        if baseline.truncated || incomplete.contains(&baseline.collector_id) {
            continue;
        }
        let collector = baseline.collector_id.clone();
        if !append_baseline(&mut items, baseline) {
            incomplete.insert(collector.clone());
            items.retain(|item| item.collector_id != collector);
        }
    }
    for snapshot in config_snapshots {
        if !incomplete.contains("files") {
            items.push(config_item(snapshot));
        }
    }
    items.sort_by(|left, right| {
        (
            &left.collector_id,
            &left.entity_kind,
            &left.entity_external_id,
        )
            .cmp(&(
                &right.collector_id,
                &right.entity_kind,
                &right.entity_external_id,
            ))
    });
    let complete_baseline_collectors = baseline_collectors
        .difference(&incomplete)
        .cloned()
        .collect();
    DurableStateItems {
        items,
        incomplete_collectors: incomplete.into_iter().collect(),
        complete_baseline_collectors,
    }
}

/// P09-P13 collectors whose point-in-time state is represented by one exact
/// durable baseline row. Files use per-path config snapshots and Event Log is
/// event evidence, so neither belongs in this contract.
pub(super) fn expected_baseline_kind(collector_id: &str) -> Option<&'static str> {
    match collector_id {
        "process" => Some("process_instances"),
        "listeners" => Some("listening_sockets"),
        "services" => Some("service_instances"),
        "applications" => Some("installed_applications"),
        "updates" => Some("windows_update_history"),
        "drivers" => Some("present_drivers"),
        "network" => Some("network_configuration"),
        "startup" => Some("startup_entries_and_tasks"),
        _ => None,
    }
}

/// Stateful collectors persist their acknowledged frontier and the remaining
/// target atomically. A non-empty `pending` value therefore means the durable
/// frontier is only an intermediate chunk, not a truthful point-in-time view.
///
/// Most collectors encode pending work as `Option<PendingTarget<_>>`; Windows
/// Update uses a queue and encodes its drained state as an empty array. Treat
/// only that empty queue as drained while conservatively rejecting every other
/// non-null representation.
fn has_pending_work(snapshot: &Value) -> bool {
    match snapshot.get("pending") {
        None | Some(Value::Null) => false,
        Some(Value::Array(pending)) => !pending.is_empty(),
        Some(Value::Object(_)) => true,
        Some(_) => true,
    }
}

fn append_baseline(output: &mut Vec<CheckpointItem>, baseline: Baseline) -> bool {
    let collector = baseline.collector_id;
    let kind = baseline.entity_kind;
    let snapshot = baseline.snapshot;
    match collector.as_str() {
        "process" => return append_process_state(output, &collector, snapshot),
        "network" => return append_network_state(output, &collector, snapshot),
        "services" => {
            return append_owned_acknowledged(output, &collector, "service", snapshot, |state| {
                serde_json::from_value::<services::ServiceRecord>(state.clone()).is_ok()
            });
        }
        "applications" => {
            return append_owned_acknowledged(
                output,
                &collector,
                "application",
                snapshot,
                |state| applications::checkpoint_fields(state, state).is_some(),
            );
        }
        "drivers" => {
            return append_owned_acknowledged(output, &collector, "driver", snapshot, |state| {
                drivers::checkpoint_fields(state, state).is_some()
            });
        }
        "listeners" => {
            return append_owned_acknowledged(output, &collector, "listener", snapshot, |state| {
                listeners::checkpoint_fields(state, state).is_some()
            });
        }
        "startup" => return append_startup_state(output, &collector, &snapshot),
        "updates" => return append_update_state(output, &collector, &kind, &snapshot),
        _ => {}
    }
    if let Some(acknowledged) = snapshot.get("acknowledged") {
        if append_grouped_maps(output, &collector, acknowledged) {
            return true;
        }
        if append_map(output, &collector, &kind, acknowledged) {
            return true;
        }
    }
    if let Some(adapters) = snapshot.get("adapters") {
        append_map(output, &collector, "adapter", adapters);
        if let Some(proxy) = snapshot.get("proxy") {
            output.push(item(&collector, "proxy", "system_proxy", proxy.clone()));
        }
        return true;
    }
    if let Some(seen) = snapshot.get("seen").and_then(Value::as_array) {
        for (index, state) in seen.iter().enumerate() {
            let external_id = stable_value_id(state, index);
            output.push(item(&collector, &kind, &external_id, state.clone()));
        }
        return true;
    }
    output.push(item(&collector, &kind, "state", snapshot));
    true
}

fn append_owned_acknowledged(
    output: &mut Vec<CheckpointItem>,
    collector: &str,
    kind: &str,
    snapshot: Value,
    valid: impl Fn(&Value) -> bool,
) -> bool {
    let Value::Object(mut snapshot) = snapshot else {
        return false;
    };
    let Some(Value::Object(entries)) = snapshot.remove("acknowledged") else {
        return false;
    };
    if entries.values().any(|state| !valid(state)) {
        return false;
    }
    output.reserve(entries.len());
    for (external_id, state) in entries {
        output.push(item(collector, kind, &external_id, state));
    }
    true
}

fn append_startup_state(
    output: &mut Vec<CheckpointItem>,
    collector: &str,
    snapshot: &Value,
) -> bool {
    let Some(groups) = snapshot.get("acknowledged").and_then(Value::as_object) else {
        return false;
    };
    for (group, kind) in [
        ("tasks", "task"),
        ("run_entries", "run_entry"),
        ("startup_files", "startup_file"),
    ] {
        let Some(entries) = groups.get(group).and_then(Value::as_object) else {
            return false;
        };
        if entries
            .values()
            .any(|state| startup::checkpoint_fields(kind, state, state).is_none())
        {
            return false;
        }
        append_map(output, collector, kind, &Value::Object(entries.clone()));
    }
    true
}

fn append_update_state(
    output: &mut Vec<CheckpointItem>,
    collector: &str,
    kind: &str,
    snapshot: &Value,
) -> bool {
    let Some(seen) = snapshot.get("seen").and_then(Value::as_array) else {
        return false;
    };
    if seen
        .iter()
        .any(|state| updates::checkpoint_fields(state, state).is_none())
    {
        return false;
    }
    for (index, state) in seen.iter().enumerate() {
        output.push(item(
            collector,
            kind,
            &stable_value_id(state, index),
            state.clone(),
        ));
    }
    true
}

#[derive(Deserialize)]
struct ProcessCheckpointSnapshot {
    acknowledged: BTreeMap<u32, ProcessCheckpointInstance>,
}

#[derive(Deserialize)]
struct ProcessCheckpointInstance {
    pid: u32,
    creation: Option<u64>,
    image: String,
    entity_id: ProcessId,
}

fn append_process_state(
    output: &mut Vec<CheckpointItem>,
    collector: &str,
    snapshot: Value,
) -> bool {
    let Ok(snapshot) = serde_json::from_value::<ProcessCheckpointSnapshot>(snapshot) else {
        return false;
    };
    for (pid_key, instance) in snapshot.acknowledged {
        if pid_key != instance.pid {
            return false;
        }
        let external_id = format!("pid:{}", instance.pid);
        output.push(CheckpointItem {
            collector_id: collector.to_owned(),
            entity_kind: "process_instance".to_owned(),
            entity_external_id: external_id,
            entity_name: Some(instance.image),
            // The live collector treats readability/confidence flips as
            // continuity. The collector-owned comparator uses these identity
            // facts to distinguish that promotion from authoritative PID reuse.
            state: serde_json::json!({
                "pid": instance.pid,
                "creation": instance.creation,
                "entity_id": instance.entity_id,
            }),
        });
    }
    true
}

#[derive(Deserialize)]
struct NetworkCheckpointSnapshot {
    acknowledged: NetworkCheckpointState,
}

#[derive(Deserialize)]
struct NetworkCheckpointState {
    adapters: BTreeMap<String, Value>,
    proxy: Value,
}

fn append_network_state(
    output: &mut Vec<CheckpointItem>,
    collector: &str,
    snapshot: Value,
) -> bool {
    let Ok(snapshot) = serde_json::from_value::<NetworkCheckpointSnapshot>(snapshot) else {
        return false;
    };
    for (guid, adapter) in snapshot.acknowledged.adapters {
        output.push(item(collector, "adapter", &guid, adapter));
    }
    output.push(item(
        collector,
        "proxy",
        "system_proxy",
        snapshot.acknowledged.proxy,
    ));
    true
}

fn append_grouped_maps(output: &mut Vec<CheckpointItem>, collector: &str, value: &Value) -> bool {
    let Some(groups) = value.as_object() else {
        return false;
    };
    let known = ["tasks", "run_entries", "startup_files"];
    if !known.iter().any(|key| groups.contains_key(*key)) {
        return false;
    }
    for key in known {
        if let Some(group) = groups.get(key) {
            append_map(output, collector, key.trim_end_matches('s'), group);
        }
    }
    true
}

fn append_map(
    output: &mut Vec<CheckpointItem>,
    collector: &str,
    kind: &str,
    value: &Value,
) -> bool {
    let Some(entries) = value.as_object() else {
        return false;
    };
    for (external_id, state) in entries {
        output.push(item(collector, kind, external_id, state.clone()));
    }
    true
}

fn item(collector: &str, kind: &str, external_id: &str, state: Value) -> CheckpointItem {
    let entity_name = display_name(&state).map(str::to_owned);
    CheckpointItem {
        collector_id: collector.to_owned(),
        entity_kind: kind.to_owned(),
        entity_external_id: external_id.to_owned(),
        entity_name,
        state,
    }
}

fn display_name(value: &Value) -> Option<&str> {
    let object = value.as_object()?;
    [
        "display_name",
        "device_description",
        "image",
        "name",
        "title",
        "path",
    ]
    .into_iter()
    .find_map(|field| object.get(field).and_then(Value::as_str))
}

fn stable_value_id(value: &Value, index: usize) -> String {
    let encoded = serde_json::to_vec(value).unwrap_or_default();
    if encoded.is_empty() {
        return format!("item-{index}");
    }
    let digest = Sha256::digest(&encoded);
    digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn config_item(snapshot: ConfigSnapshot) -> CheckpointItem {
    let external_id = config_snapshot_repository::path_hash(&snapshot.path);
    CheckpointItem {
        collector_id: "files".to_owned(),
        entity_kind: "config_file".to_owned(),
        entity_external_id: external_id,
        entity_name: Some(snapshot.path),
        state: serde_json::json!({ "format": snapshot.format, "keys": snapshot.keys }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::compare_item_sets;

    fn baseline(collector_id: &str, entity_kind: &str, snapshot: Value) -> Baseline {
        Baseline {
            collector_id: collector_id.to_owned(),
            entity_kind: entity_kind.to_owned(),
            snapshot,
            captured_at: 1,
            truncated: false,
        }
    }

    #[test]
    fn required_baseline_contract_is_only_the_p09_to_p13_structural_collectors() {
        assert_eq!(expected_baseline_kind("process"), Some("process_instances"));
        assert_eq!(
            expected_baseline_kind("listeners"),
            Some("listening_sockets")
        );
        assert_eq!(
            expected_baseline_kind("services"),
            Some("service_instances")
        );
        assert_eq!(
            expected_baseline_kind("applications"),
            Some("installed_applications")
        );
        assert_eq!(
            expected_baseline_kind("updates"),
            Some("windows_update_history")
        );
        assert_eq!(expected_baseline_kind("drivers"), Some("present_drivers"));
        assert_eq!(
            expected_baseline_kind("network"),
            Some("network_configuration")
        );
        assert_eq!(
            expected_baseline_kind("startup"),
            Some("startup_entries_and_tasks")
        );
        for collector in ["files", "eventlog", "docker", "dev_env", "synthetic"] {
            assert_eq!(expected_baseline_kind(collector), None, "{collector}");
        }
    }

    fn process_snapshot(creation: u64, path_available: bool) -> Value {
        serde_json::json!({
            "acknowledged": {
                "42": {
                    "pid": 42,
                    "creation": creation,
                    "creation_available": path_available,
                    "image": "worker.exe",
                    "entity_id": "process:0123456789abcdef",
                    "identity_path_based": true,
                    "path_available": path_available,
                    "browser_subtree": false,
                    "observed_start": true
                }
            }
        })
    }

    #[test]
    fn acknowledged_maps_become_one_item_per_entity() {
        let mut output = Vec::new();
        assert!(append_baseline(
            &mut output,
            Baseline {
                collector_id: "services".to_owned(),
                entity_kind: "service_instances".to_owned(),
                snapshot: serde_json::json!({
                    "acknowledged": {
                        "spooler": { "name": "spooler", "display_name": "Print Spooler", "state": "running", "start_type": "automatic", "config_available": true, "process_id": 10 },
                        "w32time": { "name": "w32time", "display_name": "Windows Time", "state": "stopped", "start_type": "manual", "config_available": true, "process_id": null }
                    },
                    "established": true
                }),
                captured_at: 1,
                truncated: false,
            },
        ));
        assert_eq!(output.len(), 2);
        assert_eq!(output[0].entity_external_id, "spooler");
        assert_eq!(output[0].entity_name.as_deref(), Some("Print Spooler"));
    }

    #[test]
    fn recycled_pid_is_removed_and_added_while_readability_is_not_a_change() {
        let left = from_durable_state(
            vec![baseline(
                "process",
                "process_instances",
                process_snapshot(10, true),
            )],
            vec![],
        );
        let unreadable_same_instance = from_durable_state(
            vec![baseline(
                "process",
                "process_instances",
                process_snapshot(10, false),
            )],
            vec![],
        );
        let reused = from_durable_state(
            vec![baseline(
                "process",
                "process_instances",
                process_snapshot(11, true),
            )],
            vec![],
        );
        let manifest = vec!["process".to_owned()];

        let continuity = compare_item_sets(
            &left.items,
            &manifest,
            &[],
            &unreadable_same_instance.items,
            &manifest,
            &[],
            vec![],
        );
        assert!(continuity.added.is_empty());
        assert!(continuity.removed.is_empty());
        assert!(continuity.changed.is_empty());

        let restart = compare_item_sets(
            &left.items,
            &manifest,
            &[],
            &reused.items,
            &manifest,
            &[],
            vec![],
        );
        assert_eq!(restart.removed.len(), 1);
        assert_eq!(restart.added.len(), 1);
        assert!(restart.changed.is_empty());
    }

    #[test]
    fn network_adapters_are_individual_added_and_removed_items() {
        let snapshot = |guid: &str| {
            serde_json::json!({
                "acknowledged": {
                    "adapters": {
                        (guid): {
                            "guid": guid,
                            "friendly_name": "Ethernet",
                            "description": "Adapter",
                            "if_type": 6,
                            "interface_index": 4,
                            "addresses": ["192.0.2.1"],
                            "dns_servers": [],
                            "gateways": [],
                            "default_routes": [],
                            "vpn": false
                        }
                    },
                    "proxy": {
                        "enabled": false,
                        "server": null,
                        "bypass": null,
                        "automatic_proxy_script": null
                    }
                }
            })
        };
        let left = from_durable_state(
            vec![baseline(
                "network",
                "network_configuration",
                snapshot("adapter-a"),
            )],
            vec![],
        );
        let right = from_durable_state(
            vec![baseline(
                "network",
                "network_configuration",
                snapshot("adapter-b"),
            )],
            vec![],
        );
        let manifest = vec!["network".to_owned()];
        let diff = compare_item_sets(
            &left.items,
            &manifest,
            &[],
            &right.items,
            &manifest,
            &[],
            vec![],
        );
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.added.len(), 1);
        assert!(diff.changed.is_empty());
        assert_eq!(diff.removed[0].entity_kind, "adapter");
        assert_eq!(diff.added[0].entity_kind, "adapter");
    }

    #[test]
    fn truncated_baseline_makes_the_whole_collector_incomplete() {
        let mut marker = baseline(
            "services",
            "service_instances",
            serde_json::json!({ "truncated": true, "bytes": 2_000_000 }),
        );
        marker.truncated = true;
        let extracted = from_durable_state(
            vec![
                baseline(
                    "services",
                    "other_kind",
                    serde_json::json!({
                        "acknowledged": { "spooler": { "state": "running" } }
                    }),
                ),
                marker,
            ],
            vec![],
        );
        assert!(extracted.items.is_empty());
        assert_eq!(extracted.incomplete_collectors, ["services"]);
    }

    #[test]
    fn pending_chunk_is_excluded_until_the_final_target_is_drained() {
        let service = |state: &str| {
            serde_json::json!({
                "name": "spooler",
                "display_name": "Print Spooler",
                "state": state,
                "start_type": "automatic",
                "config_available": true,
                "process_id": 10
            })
        };
        let intermediate = from_durable_state(
            vec![baseline(
                "services",
                "service_instances",
                serde_json::json!({
                    "acknowledged": { "spooler": service("stopped") },
                    "pending": {
                        "target": { "spooler": service("running") },
                        "observed_at": 2,
                        "occurred_at_is_uncertain": false
                    },
                    "established": true
                }),
            )],
            vec![],
        );
        assert!(
            intermediate.items.is_empty(),
            "an intermediate acknowledged chunk must not look complete"
        );
        assert_eq!(intermediate.incomplete_collectors, ["services"]);

        let drained = from_durable_state(
            vec![baseline(
                "services",
                "service_instances",
                serde_json::json!({
                    "acknowledged": { "spooler": service("running") },
                    "pending": null,
                    "established": true
                }),
            )],
            vec![],
        );
        assert!(drained.incomplete_collectors.is_empty());
        assert_eq!(drained.items.len(), 1);
        assert_eq!(drained.items[0].state["state"], "running");
    }

    #[test]
    fn empty_update_queue_is_drained_but_non_empty_queue_is_incomplete() {
        let update = serde_json::json!({
            "update_id": "KB123:1",
            "title": "Example update",
            "operation": "Installation",
            "result": "Succeeded",
            "date_ms": 1,
            "support_url": null,
            "client_application_id": null
        });
        let pending = from_durable_state(
            vec![baseline(
                "updates",
                "update_history",
                serde_json::json!({ "seen": [], "pending": [update.clone()] }),
            )],
            vec![],
        );
        assert!(pending.items.is_empty());
        assert_eq!(pending.incomplete_collectors, ["updates"]);

        let drained = from_durable_state(
            vec![baseline(
                "updates",
                "update_history",
                serde_json::json!({ "seen": [update], "pending": [] }),
            )],
            vec![],
        );
        assert!(drained.incomplete_collectors.is_empty());
        assert_eq!(drained.items.len(), 1);
    }

    #[test]
    fn weak_process_identity_promotes_without_restart_but_pid_reuse_does_not() {
        let snapshot = |creation: Option<u64>, entity_id: &str| {
            serde_json::json!({
                "acknowledged": { "42": {
                    "pid": 42,
                    "creation": creation,
                    "image": "worker.exe",
                    "entity_id": entity_id
                }}
            })
        };
        let weak = from_durable_state(
            vec![baseline(
                "process",
                "process_instances",
                snapshot(None, "process:0123456789abcdef"),
            )],
            vec![],
        );
        let promoted = from_durable_state(
            vec![baseline(
                "process",
                "process_instances",
                snapshot(Some(10), "process:0123456789abcdef"),
            )],
            vec![],
        );
        let reused = from_durable_state(
            vec![baseline(
                "process",
                "process_instances",
                snapshot(Some(11), "process:0123456789abcdef"),
            )],
            vec![],
        );
        let manifest = vec!["process".to_owned()];
        let promotion = compare_item_sets(
            &weak.items,
            &manifest,
            &[],
            &promoted.items,
            &manifest,
            &[],
            vec![],
        );
        assert!(promotion.added.is_empty());
        assert!(promotion.removed.is_empty());
        assert!(promotion.changed.is_empty());

        let reuse = compare_item_sets(
            &promoted.items,
            &manifest,
            &[],
            &reused.items,
            &manifest,
            &[],
            vec![],
        );
        assert_eq!(reuse.removed.len(), 1);
        assert_eq!(reuse.added.len(), 1);
        assert!(reuse.changed.is_empty());
    }

    #[test]
    fn network_gateway_routes_and_proxy_follow_live_change_cardinality() {
        let snapshot = |gateway: &str, route: &str, proxy_server: &str| {
            serde_json::json!({ "acknowledged": {
                "adapters": { "adapter-a": {
                    "guid": "adapter-a", "friendly_name": "Ethernet",
                    "description": "Adapter", "if_type": 6, "interface_index": 4,
                    "addresses": ["192.0.2.1"], "dns_servers": ["192.0.2.53"],
                    "gateways": [gateway], "default_routes": [route], "vpn": false
                }},
                "proxy": { "enabled": true, "server": proxy_server, "bypass": null,
                    "automatic_proxy_script": null }
            }})
        };
        let left = from_durable_state(
            vec![baseline(
                "network",
                "network_configuration",
                snapshot("192.0.2.1", "0.0.0.0/0", "old:80"),
            )],
            vec![],
        );
        let right = from_durable_state(
            vec![baseline(
                "network",
                "network_configuration",
                snapshot("192.0.2.254", "::/0", "new:80"),
            )],
            vec![],
        );
        let manifest = vec!["network".to_owned()];
        let diff = compare_item_sets(
            &left.items,
            &manifest,
            &[],
            &right.items,
            &manifest,
            &[],
            vec![],
        );
        assert_eq!(
            diff.changed.len(),
            2,
            "one gateway group plus one proxy group"
        );
        assert_eq!(
            diff.changed
                .iter()
                .filter(|item| item.field.as_deref() == Some("gateways"))
                .count(),
            1
        );
        assert_eq!(
            diff.changed
                .iter()
                .filter(|item| item.field.as_deref() == Some("proxy"))
                .count(),
            1
        );
    }

    #[test]
    fn one_truncated_config_snapshot_makes_files_incomplete_on_either_side() {
        let full = ConfigSnapshot {
            path: r"C:\app\settings.json".to_owned(),
            keys: serde_json::json!({"feature": true}),
            captured_at: 1,
            format: "json".to_owned(),
        };
        let mut partial = full.clone();
        partial.format = "json+truncated".to_owned();
        let complete = from_durable_state(vec![], vec![full]);
        let incomplete = from_durable_state(vec![], vec![partial]);
        assert_eq!(complete.items.len(), 1);
        assert!(incomplete.items.is_empty());
        assert_eq!(incomplete.incomplete_collectors, ["files"]);

        let captured = vec!["files".to_owned()];
        for (left, right) in [(&complete, &incomplete), (&incomplete, &complete)] {
            let diff = compare_item_sets(
                &left.items,
                &captured,
                &left.incomplete_collectors,
                &right.items,
                &captured,
                &right.incomplete_collectors,
                vec![],
            );
            assert!(diff.added.is_empty() && diff.removed.is_empty());
            assert!(diff.skipped_collectors.contains("files"));
        }
    }

    #[test]
    fn service_checkpoint_diff_matches_running_boundary_and_start_type_rules() {
        let snapshot = |state: &str,
                        start_type: &str,
                        config_available: bool,
                        display_name: &str,
                        process_id: Option<u32>| {
            serde_json::json!({ "acknowledged": { "spooler": {
                "name": "spooler", "display_name": display_name, "state": state,
                "start_type": start_type, "config_available": config_available,
                "process_id": process_id
            }}, "established": true })
        };
        let extract = |value| {
            from_durable_state(
                vec![baseline("services", "service_instances", value)],
                vec![],
            )
        };
        let waiting = extract(snapshot("start_pending", "automatic", true, "Old", Some(4)));
        let paused = extract(snapshot("paused", "automatic", true, "New", Some(99)));
        let running = extract(snapshot("running", "manual", true, "New", Some(100)));
        let unreadable = extract(snapshot("running", "disabled", false, "New", Some(101)));
        let manifest = vec!["services".to_owned()];

        let noise = compare_item_sets(
            &waiting.items,
            &manifest,
            &[],
            &paused.items,
            &manifest,
            &[],
            vec![],
        );
        assert!(
            noise.changed.is_empty(),
            "display, PID, and non-running enum noise"
        );

        let semantic = compare_item_sets(
            &paused.items,
            &manifest,
            &[],
            &running.items,
            &manifest,
            &[],
            vec![],
        );
        assert_eq!(semantic.changed.len(), 2);
        assert_eq!(semantic.changed[0].field.as_deref(), Some("running"));
        assert_eq!(semantic.changed[1].field.as_deref(), Some("start_type"));

        let denied = compare_item_sets(
            &running.items,
            &manifest,
            &[],
            &unreadable.items,
            &manifest,
            &[],
            vec![],
        );
        assert!(
            denied.changed.is_empty(),
            "unreadable config is absence of evidence"
        );
    }

    #[test]
    fn startup_task_runtime_metadata_is_not_a_checkpoint_change() {
        let snapshot = |enabled: bool, action: &str, last_run_ms: i64, last_result: i32| {
            serde_json::json!({ "acknowledged": {
                "tasks": { "\\task": {
                    "name": "Task", "path": "\\Task", "enabled": enabled,
                    "triggers": ["boot"], "action_path": action,
                    "last_run_ms": last_run_ms, "last_result": last_result
                }},
                "run_entries": {}, "startup_files": {}
            }, "established": true })
        };
        let extract = |value| {
            from_durable_state(
                vec![baseline("startup", "startup_entries_and_tasks", value)],
                vec![],
            )
        };
        let old = extract(snapshot(true, "worker.exe", 10, 0));
        let runtime = extract(snapshot(true, "worker.exe", 20, 1));
        let disabled = extract(snapshot(false, "changed.exe", 30, 2));
        let manifest = vec!["startup".to_owned()];
        let ignored = compare_item_sets(
            &old.items,
            &manifest,
            &[],
            &runtime.items,
            &manifest,
            &[],
            vec![],
        );
        assert!(ignored.changed.is_empty());
        let precedence = compare_item_sets(
            &runtime.items,
            &manifest,
            &[],
            &disabled.items,
            &manifest,
            &[],
            vec![],
        );
        assert_eq!(precedence.changed.len(), 1);
        assert_eq!(precedence.changed[0].field.as_deref(), Some("enabled"));
    }

    #[test]
    fn remaining_structural_collectors_use_owned_checkpoint_semantics() {
        let compare = |collector: &str, kind: &str, left: Value, right: Value| {
            let left = from_durable_state(vec![baseline(collector, kind, left)], vec![]);
            let right = from_durable_state(vec![baseline(collector, kind, right)], vec![]);
            compare_item_sets(
                &left.items,
                &[collector.to_owned()],
                &[],
                &right.items,
                &[collector.to_owned()],
                &[],
                vec![],
            )
        };

        let application = |publisher: &str, version: &str| {
            serde_json::json!({
                "acknowledged": { "HKLM64:app": {
                    "hive": "LocalMachine64", "subkey_name": "app", "display_name": "App",
                    "display_version": version, "publisher": publisher, "install_date": null
                }}, "established": true
            })
        };
        let app_metadata = compare(
            "applications",
            "installed_applications",
            application("Old", "1"),
            application("New", "1"),
        );
        assert!(app_metadata.changed.is_empty());
        let app_version = compare(
            "applications",
            "installed_applications",
            application("Old", "1"),
            application("New", "2"),
        );
        assert_eq!(app_version.changed.len(), 1);
        assert_eq!(app_version.changed[0].field.as_deref(), Some("version"));

        let driver = |description: &str, version: &str| {
            serde_json::json!({
                "acknowledged": { "PCI\\DEV": {
                    "device_instance_id": "PCI\\DEV", "device_description": description,
                    "class_name": "Display", "manufacturer": "Vendor",
                    "driver_version": version, "driver_date": null, "provider_name": null
                }}, "established": true
            })
        };
        assert!(compare(
            "drivers",
            "present_drivers",
            driver("Old", "1"),
            driver("New", "1")
        )
        .changed
        .is_empty());
        assert_eq!(
            compare(
                "drivers",
                "present_drivers",
                driver("Old", "1"),
                driver("New", "2")
            )
            .changed
            .len(),
            1
        );

        let listener = |owners: &[u32]| {
            serde_json::json!({
                "acknowledged": { "port:tcp:0.0.0.0:80": {
                    "protocol": "Tcp", "address": "0.0.0.0", "port": 80, "owners": owners
                }}, "established": true
            })
        };
        let ownership = compare(
            "listeners",
            "listening_sockets",
            listener(&[1]),
            listener(&[2, 3]),
        );
        assert_eq!(ownership.changed.len(), 1);
        assert_eq!(ownership.changed[0].field.as_deref(), Some("owners"));
    }

    #[test]
    fn config_checkpoints_ignore_format_and_redacted_value_uncertainty() {
        let snapshot = |format: &str, ordinary: &str| ConfigSnapshot {
            path: r"C:\app\settings.conf".to_owned(),
            keys: serde_json::json!({
                "ordinary": ordinary,
                "password": "[REDACTED]"
            }),
            captured_at: 1,
            format: format.to_owned(),
        };
        let left = from_durable_state(vec![], vec![snapshot("ini", "old")]);
        let same = from_durable_state(vec![], vec![snapshot("properties", "old")]);
        let changed = from_durable_state(vec![], vec![snapshot("properties", "new")]);
        let manifest = vec!["files".to_owned()];
        assert!(compare_item_sets(
            &left.items,
            &manifest,
            &[],
            &same.items,
            &manifest,
            &[],
            vec![],
        )
        .changed
        .is_empty());
        let diff = compare_item_sets(
            &same.items,
            &manifest,
            &[],
            &changed.items,
            &manifest,
            &[],
            vec![],
        );
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.changed[0].field.as_deref(), Some("ordinary"));
    }
}
