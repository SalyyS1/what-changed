use std::collections::BTreeSet;

use serde_json::{json, Value};

use super::{NetworkAdapter, NetworkState, ProxyConfiguration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Change {
    AdapterAdded(NetworkAdapter),
    AdapterRemoved(NetworkAdapter),
    AdapterChanged {
        before: NetworkAdapter,
        after: NetworkAdapter,
    },
    IpChanged {
        before: NetworkAdapter,
        after: NetworkAdapter,
    },
    DnsChanged {
        before: NetworkAdapter,
        after: NetworkAdapter,
    },
    GatewayChanged {
        before: NetworkAdapter,
        after: NetworkAdapter,
    },
    VpnDetected(NetworkAdapter),
    ProxyChanged {
        before: ProxyConfiguration,
        after: ProxyConfiguration,
    },
}

pub(super) fn changes(before: &NetworkState, after: &NetworkState) -> Vec<Change> {
    let mut changes = Vec::new();
    let keys: BTreeSet<&String> = before
        .adapters
        .keys()
        .chain(after.adapters.keys())
        .collect();
    for key in keys {
        match (before.adapters.get(key), after.adapters.get(key)) {
            (None, Some(adapter)) => {
                changes.push(Change::AdapterAdded(adapter.clone()));
            }
            (Some(adapter), None) => changes.push(Change::AdapterRemoved(adapter.clone())),
            (Some(old), Some(new)) => {
                if adapter_metadata(old) != adapter_metadata(new) {
                    changes.push(Change::AdapterChanged {
                        before: old.clone(),
                        after: new.clone(),
                    });
                }
                if old.addresses != new.addresses {
                    changes.push(Change::IpChanged {
                        before: old.clone(),
                        after: new.clone(),
                    });
                }
                if old.dns_servers != new.dns_servers {
                    changes.push(Change::DnsChanged {
                        before: old.clone(),
                        after: new.clone(),
                    });
                }
                if old.gateways != new.gateways || old.default_routes != new.default_routes {
                    changes.push(Change::GatewayChanged {
                        before: old.clone(),
                        after: new.clone(),
                    });
                }
                if !old.vpn && new.vpn {
                    changes.push(Change::VpnDetected(new.clone()));
                }
            }
            (None, None) => {}
        }
    }
    if before.proxy != after.proxy {
        changes.push(Change::ProxyChanged {
            before: before.proxy.clone(),
            after: after.proxy.clone(),
        });
    }
    changes
}

pub(super) fn apply(state: &mut NetworkState, change: &Change) {
    match change {
        Change::AdapterAdded(adapter) => {
            state.adapters.insert(adapter.guid.clone(), adapter.clone());
        }
        Change::AdapterRemoved(adapter) => {
            state.adapters.remove(&adapter.guid);
        }
        Change::AdapterChanged { after, .. } => {
            if let Some(current) = state.adapters.get_mut(&after.guid) {
                current.friendly_name.clone_from(&after.friendly_name);
                current.description.clone_from(&after.description);
                current.if_type = after.if_type;
                current.interface_index = after.interface_index;
            }
        }
        Change::IpChanged { after, .. } => {
            if let Some(current) = state.adapters.get_mut(&after.guid) {
                current.addresses.clone_from(&after.addresses);
            }
        }
        Change::DnsChanged { after, .. } => {
            if let Some(current) = state.adapters.get_mut(&after.guid) {
                current.dns_servers.clone_from(&after.dns_servers);
            }
        }
        Change::GatewayChanged { after, .. } => {
            if let Some(current) = state.adapters.get_mut(&after.guid) {
                current.gateways.clone_from(&after.gateways);
                current.default_routes.clone_from(&after.default_routes);
            }
        }
        Change::VpnDetected(adapter) => {
            if let Some(current) = state.adapters.get_mut(&adapter.guid) {
                current.vpn = true;
            }
        }
        Change::ProxyChanged { after, .. } => state.proxy = after.clone(),
    }
}

fn adapter_metadata(adapter: &NetworkAdapter) -> (&str, &str, u32, u32) {
    (
        &adapter.friendly_name,
        &adapter.description,
        adapter.if_type,
        adapter.interface_index,
    )
}

/// One collector-semantic checkpoint field change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckpointFieldChange {
    pub field: &'static str,
    pub before: Value,
    pub after: Value,
}

/// Compare checkpoint state with exactly the same grouping rules as the live
/// network collector. In particular gateway and default-route movement is one
/// change, and the proxy is one indivisible configuration change.
pub(crate) fn checkpoint_fields(
    entity_kind: &str,
    before: &Value,
    after: &Value,
) -> Option<Vec<CheckpointFieldChange>> {
    let changes = match entity_kind {
        "adapter" => {
            let before = serde_json::from_value::<NetworkAdapter>(before.clone()).ok()?;
            let after = serde_json::from_value::<NetworkAdapter>(after.clone()).ok()?;
            if before.guid != after.guid {
                return None;
            }
            let before = NetworkState {
                adapters: [(before.guid.clone(), before)].into_iter().collect(),
                proxy: ProxyConfiguration::default(),
            };
            let after = NetworkState {
                adapters: [(after.guid.clone(), after)].into_iter().collect(),
                proxy: ProxyConfiguration::default(),
            };
            changes(&before, &after)
        }
        "proxy" => {
            let before_proxy = serde_json::from_value::<ProxyConfiguration>(before.clone()).ok()?;
            let after_proxy = serde_json::from_value::<ProxyConfiguration>(after.clone()).ok()?;
            changes(
                &NetworkState {
                    adapters: Default::default(),
                    proxy: before_proxy,
                },
                &NetworkState {
                    adapters: Default::default(),
                    proxy: after_proxy,
                },
            )
        }
        _ => return None,
    };
    Some(
        changes
            .into_iter()
            .filter_map(|change| match change {
                Change::AdapterChanged { before, after } => Some(CheckpointFieldChange {
                    field: "adapter",
                    before: adapter_snapshot(&before),
                    after: adapter_snapshot(&after),
                }),
                Change::IpChanged { before, after } => Some(CheckpointFieldChange {
                    field: "ip_addresses",
                    before: json!(before.addresses),
                    after: json!(after.addresses),
                }),
                Change::DnsChanged { before, after } => Some(CheckpointFieldChange {
                    field: "dns_servers",
                    before: json!(before.dns_servers),
                    after: json!(after.dns_servers),
                }),
                Change::GatewayChanged { before, after } => Some(CheckpointFieldChange {
                    field: "gateways",
                    before: gateway_snapshot(&before),
                    after: gateway_snapshot(&after),
                }),
                Change::VpnDetected(adapter) => Some(CheckpointFieldChange {
                    field: "vpn",
                    before: json!(false),
                    after: json!(adapter.vpn),
                }),
                Change::ProxyChanged { before, after } => Some(CheckpointFieldChange {
                    field: "proxy",
                    before: json!(before),
                    after: json!(after),
                }),
                Change::AdapterAdded(_) | Change::AdapterRemoved(_) => None,
            })
            .collect(),
    )
}

fn adapter_snapshot(adapter: &NetworkAdapter) -> Value {
    json!({
        "friendly_name": adapter.friendly_name,
        "description": adapter.description,
        "if_type": adapter.if_type,
        "interface_index": adapter.interface_index,
    })
}

fn gateway_snapshot(adapter: &NetworkAdapter) -> Value {
    json!({
        "gateways": adapter.gateways,
        "default_routes": adapter.default_routes,
    })
}
