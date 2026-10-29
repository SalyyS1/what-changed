use serde_json::{json, Value};

use crate::collectors::{EntityIdentity, ObservationKind, RawEntity, RawObservation};
use crate::domain::{Category, ClassifierContext, Confidence, EventType, Severity};

use super::diff::Change;
use super::{NetworkAdapter, ID};

pub(super) fn for_change(
    change: &Change,
    occurred_at: i64,
    uncertain: bool,
) -> Vec<RawObservation> {
    let primary = match change {
        Change::AdapterAdded(adapter) => observation(
            adapter,
            EventType::Connected,
            format!("{} appeared", adapter.friendly_name),
            "adapter_changed",
            None,
            Some(adapter_snapshot(adapter)),
            None,
            occurred_at,
            uncertain,
        ),
        Change::AdapterRemoved(adapter) => observation(
            adapter,
            EventType::Disconnected,
            format!("{} disappeared", adapter.friendly_name),
            "adapter_changed",
            Some(adapter_snapshot(adapter)),
            None,
            None,
            occurred_at,
            uncertain,
        ),
        Change::AdapterChanged { before, after } => observation(
            after,
            EventType::Modified,
            format!("{} adapter details changed", after.friendly_name),
            "adapter_changed",
            Some(adapter_snapshot(before)),
            Some(adapter_snapshot(after)),
            Some("adapter"),
            occurred_at,
            uncertain,
        ),
        Change::IpChanged { before, after } => observation(
            after,
            EventType::Modified,
            format!("{} IP addresses changed", after.friendly_name),
            "ip_changed",
            Some(json!(before.addresses)),
            Some(json!(after.addresses)),
            Some("ip_addresses"),
            occurred_at,
            uncertain,
        ),
        Change::DnsChanged { before, after } => observation(
            after,
            EventType::ConfigurationChanged,
            format!("{} DNS servers changed", after.friendly_name),
            "dns_changed",
            Some(json!(before.dns_servers)),
            Some(json!(after.dns_servers)),
            Some("dns_servers"),
            occurred_at,
            uncertain,
        ),
        Change::GatewayChanged { before, after } => observation(
            after,
            EventType::Modified,
            format!("{} gateways changed", after.friendly_name),
            "gateway_changed",
            Some(gateway_snapshot(before)),
            Some(gateway_snapshot(after)),
            Some("gateways"),
            occurred_at,
            uncertain,
        ),
        Change::VpnDetected(adapter) => observation(
            adapter,
            EventType::Connected,
            format!("{} VPN adapter detected", adapter.friendly_name),
            "vpn_detected",
            None,
            Some(json!(true)),
            Some("vpn"),
            occurred_at,
            uncertain,
        ),
        Change::ProxyChanged { before, after } => RawObservation {
            collector_id: ID,
            kind: kind(
                EventType::ConfigurationChanged,
                format!(
                    "System proxy {}",
                    if after.enabled || after.auto_config_url.is_some() {
                        "changed"
                    } else {
                        "was turned off"
                    }
                ),
                "the current-user Internet Settings proxy values changed".to_owned(),
            ),
            entity: RawEntity {
                identity: EntityIdentity::System,
                display_name: "Network proxy".to_owned(),
            },
            before: Some(json!(before)),
            after: Some(json!(after)),
            field: Some("proxy".to_owned()),
            occurred_at,
            details: Some(json!({
                "change_kind": "proxy_changed",
                "event_name": "network/proxy_changed",
            })),
            related_entity_ids: Vec::new(),
            occurred_at_is_uncertain: uncertain,
        },
    };
    if let Change::AdapterAdded(adapter) = change {
        if adapter.vpn {
            return vec![
                primary,
                observation(
                    adapter,
                    EventType::Connected,
                    format!("{} VPN adapter detected", adapter.friendly_name),
                    "vpn_detected",
                    None,
                    Some(json!(true)),
                    Some("vpn"),
                    occurred_at,
                    uncertain,
                ),
            ];
        }
    }
    vec![primary]
}

#[allow(clippy::too_many_arguments)]
fn observation(
    adapter: &NetworkAdapter,
    event_type: EventType,
    user_summary: String,
    change_kind: &'static str,
    before: Option<Value>,
    after: Option<Value>,
    field: Option<&str>,
    occurred_at: i64,
    uncertain: bool,
) -> RawObservation {
    RawObservation {
        collector_id: ID,
        kind: kind(
            event_type,
            user_summary,
            format!("network adapter {} reported {change_kind}", adapter.guid),
        ),
        entity: RawEntity {
            identity: EntityIdentity::Adapter {
                guid: adapter.guid.clone(),
            },
            display_name: adapter.friendly_name.clone(),
        },
        before,
        after,
        field: field.map(str::to_owned),
        occurred_at,
        details: Some(json!({
            "change_kind": change_kind,
            "event_name": format!("network/{change_kind}"),
            "description": adapter.description,
            "if_type": adapter.if_type,
            "interface_index": adapter.interface_index,
            "vpn": adapter.vpn,
        })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: uncertain,
    }
}

fn kind(
    event_type: EventType,
    user_friendly_summary: String,
    technical_summary: String,
) -> ObservationKind {
    ObservationKind {
        category: Category::Network,
        event_type,
        severity: Severity::Info,
        confidence: Confidence::High,
        user_friendly_summary,
        technical_summary,
        classifier: ClassifierContext::default(),
    }
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
