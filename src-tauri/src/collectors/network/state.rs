use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::collectors::stateful;
use crate::privacy::{Redaction, Sanitizer, Scrub};
use crate::storage::Baseline;

/// One adapter as exposed by `GetAdaptersAddresses` plus route-table evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkAdapter {
    pub guid: String,
    pub friendly_name: String,
    pub description: String,
    pub if_type: u32,
    pub interface_index: u32,
    pub addresses: Vec<String>,
    pub dns_servers: Vec<String>,
    pub gateways: Vec<String>,
    pub default_routes: Vec<String>,
    pub vpn: bool,
}

impl NetworkAdapter {
    pub(super) fn normalize(mut self) -> Self {
        self.guid = self.guid.trim().to_ascii_lowercase();
        normalize_values(&mut self.addresses);
        normalize_values(&mut self.dns_servers);
        normalize_values(&mut self.gateways);
        normalize_values(&mut self.default_routes);
        self
    }
}

impl Scrub for NetworkAdapter {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        self.guid
            .scrub_in_place(sanitizer, &format!("{field}.guid"), report);
        self.friendly_name
            .scrub_in_place(sanitizer, &format!("{field}.friendly_name"), report);
        self.description
            .scrub_in_place(sanitizer, &format!("{field}.description"), report);
        self.addresses
            .scrub_in_place(sanitizer, &format!("{field}.addresses"), report);
        self.dns_servers
            .scrub_in_place(sanitizer, &format!("{field}.dns_servers"), report);
        self.gateways
            .scrub_in_place(sanitizer, &format!("{field}.gateways"), report);
        self.default_routes
            .scrub_in_place(sanitizer, &format!("{field}.default_routes"), report);
    }
}

/// Per-user Internet Settings values. Values are facts, never interpreted as
/// connectivity or causation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProxyConfiguration {
    pub enabled: bool,
    pub server: Option<String>,
    pub bypass: Option<String>,
    /// Serialized under a key that does not end in `url`: the privacy
    /// chokepoint treats every `*url` field as credential-bearing and would
    /// otherwise replace even `null` with `[REDACTED]` in the durable baseline,
    /// creating a phantom proxy change after every restart. Credential-shaped
    /// content inside this value is still caught by the value sanitizer.
    #[serde(rename = "automatic_proxy_script")]
    pub auto_config_url: Option<String>,
}

impl ProxyConfiguration {
    pub(super) fn normalize(mut self) -> Self {
        self.server = normalize_optional(self.server);
        self.bypass = normalize_optional(self.bypass);
        self.auto_config_url = normalize_optional(self.auto_config_url);
        self
    }
}

impl Scrub for ProxyConfiguration {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        self.server
            .scrub_in_place(sanitizer, &format!("{field}.server"), report);
        self.bypass
            .scrub_in_place(sanitizer, &format!("{field}.bypass"), report);
        self.auto_config_url.scrub_in_place(
            sanitizer,
            &format!("{field}.automatic_proxy_script"),
            report,
        );
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkState {
    pub adapters: BTreeMap<String, NetworkAdapter>,
    pub proxy: ProxyConfiguration,
}

impl NetworkState {
    pub fn from_parts(adapters: Vec<NetworkAdapter>, proxy: ProxyConfiguration) -> Self {
        Self {
            adapters: adapters
                .into_iter()
                .map(NetworkAdapter::normalize)
                .map(|adapter| (adapter.guid.clone(), adapter))
                .collect(),
            proxy: proxy.normalize(),
        }
    }
}

impl Scrub for NetworkState {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        for (guid, adapter) in &mut self.adapters {
            adapter.scrub_in_place(sanitizer, &format!("{field}.adapters.{guid}"), report);
        }
        self.proxy
            .scrub_in_place(sanitizer, &format!("{field}.proxy"), report);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSnapshot {
    pub acknowledged: NetworkState,
    #[serde(default)]
    pub pending: Option<stateful::PendingTarget<NetworkState>>,
    pub established: bool,
}

impl NetworkSnapshot {
    pub(super) fn hydrate(baseline: Option<Baseline>, now_ms: i64) -> (Self, bool) {
        stateful::hydrate(baseline, now_ms, "the network configuration")
    }
}

fn normalize_values(values: &mut Vec<String>) {
    values.retain(|value| !value.trim().is_empty());
    for value in values.iter_mut() {
        *value = value.trim().to_owned();
    }
    values.sort_by_key(|value| value.to_ascii_lowercase());
    values.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
