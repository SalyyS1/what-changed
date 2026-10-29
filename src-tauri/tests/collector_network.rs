//! Network collector acceptance checks with an injected provider.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use what_changed_lib::collectors::network::{
    self, NetworkAdapter, NetworkCollector, NetworkProvider, NetworkSnapshot, NetworkState,
    NetworkWakeTier, ProxyConfiguration,
};
use what_changed_lib::collectors::normalize::Normalizer;
use what_changed_lib::collectors::stateful::DOWNTIME_GAP_MS;
use what_changed_lib::collectors::testing::live_context;
use what_changed_lib::collectors::{
    CancelToken, Collector, CollectorSnapshot, ObservationClock, SnapshotContext, WakeHandle,
};
use what_changed_lib::domain::{EventType, Importance};
use what_changed_lib::error::AppResult;
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{baseline_repository, Baseline, CollectorCommitReceipt};

#[derive(Clone)]
struct ScriptedNetwork {
    state: Arc<Mutex<NetworkState>>,
    tier: NetworkWakeTier,
    registrations: Arc<AtomicU64>,
    unregisters: Arc<AtomicU64>,
}

impl ScriptedNetwork {
    fn new(state: NetworkState) -> Self {
        Self {
            state: Arc::new(Mutex::new(state)),
            tier: NetworkWakeTier::TimerOnly,
            registrations: Arc::new(AtomicU64::new(0)),
            unregisters: Arc::new(AtomicU64::new(0)),
        }
    }

    fn with_tier(mut self, tier: NetworkWakeTier) -> Self {
        self.tier = tier;
        self
    }

    fn replace(&self, state: NetworkState) {
        *lock(&self.state) = state;
    }
}

impl NetworkProvider for ScriptedNetwork {
    fn network_state(&self, cancel: &CancelToken, _deadline: Instant) -> AppResult<NetworkState> {
        if cancel.is_cancelled() {
            return Ok(NetworkState::default());
        }
        Ok(lock(&self.state).clone())
    }

    fn register_wake(&self, _wake: WakeHandle, _cancel: CancelToken) -> NetworkWakeTier {
        self.registrations.fetch_add(1, Ordering::AcqRel);
        self.tier
    }

    fn unregister_wake(&self) {
        self.unregisters.fetch_add(1, Ordering::AcqRel);
    }
}

#[derive(Clone)]
struct ManualClock(Arc<AtomicI64>);

impl ManualClock {
    fn at(now_ms: i64) -> Self {
        Self(Arc::new(AtomicI64::new(now_ms)))
    }
}

impl ObservationClock for ManualClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }
}

fn adapter(
    guid: &str,
    name: &str,
    addresses: &[&str],
    dns: &[&str],
    gateways: &[&str],
    vpn: bool,
) -> NetworkAdapter {
    NetworkAdapter {
        guid: guid.to_owned(),
        friendly_name: name.to_owned(),
        description: format!("{name} fixture"),
        if_type: if vpn { 131 } else { 6 },
        interface_index: 7,
        addresses: addresses.iter().map(|value| (*value).to_owned()).collect(),
        dns_servers: dns.iter().map(|value| (*value).to_owned()).collect(),
        gateways: gateways.iter().map(|value| (*value).to_owned()).collect(),
        default_routes: gateways.iter().map(|value| (*value).to_owned()).collect(),
        vpn,
    }
}

fn state(adapters: Vec<NetworkAdapter>, proxy: ProxyConfiguration) -> NetworkState {
    NetworkState::from_parts(adapters, proxy)
}

fn observations(snapshot: &CollectorSnapshot) -> &[what_changed_lib::collectors::RawObservation] {
    match snapshot {
        CollectorSnapshot::Atomic { observations, .. } => observations,
        CollectorSnapshot::Noop => &[],
        CollectorSnapshot::Legacy(_) => panic!("network collector must use atomic snapshots"),
    }
}

fn cycle<P: NetworkProvider, C: ObservationClock>(
    collector: &mut NetworkCollector<P, C>,
    token: u64,
) -> CollectorSnapshot {
    let snapshot = collector
        .snapshot_batch(SnapshotContext::for_devtools(token))
        .unwrap();
    if let CollectorSnapshot::Atomic { token, .. } = &snapshot {
        collector
            .commit_succeeded(*token, &CollectorCommitReceipt { event_ids: vec![] })
            .unwrap();
    }
    snapshot
}

#[test]
fn per_field_changes_have_stable_subtypes_and_values() {
    let initial = state(
        vec![adapter(
            "{ETHERNET}",
            "Ethernet",
            &["192.168.1.10/24"],
            &["192.168.1.1"],
            &["192.168.1.1"],
            false,
        )],
        ProxyConfiguration::default(),
    );
    let provider = ScriptedNetwork::new(initial);
    let mut collector = NetworkCollector::with_clock(
        provider.clone(),
        network::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(network::ID).unwrap().context)
        .unwrap();
    assert!(observations(&cycle(&mut collector, 1)).is_empty());

    provider.replace(state(
        vec![adapter(
            "{ETHERNET}",
            "Ethernet",
            &["10.0.0.2/24"],
            &["8.8.8.8"],
            &["10.0.0.1"],
            false,
        )],
        ProxyConfiguration {
            enabled: true,
            server: Some("127.0.0.1:8888".to_owned()),
            bypass: Some("<local>".to_owned()),
            auto_config_url: None,
        },
    ));
    let changed = cycle(&mut collector, 2);
    let rows = observations(&changed);
    let kinds: Vec<_> = rows
        .iter()
        .map(|row| {
            row.details.as_ref().unwrap()["change_kind"]
                .as_str()
                .unwrap()
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "ip_changed",
            "dns_changed",
            "gateway_changed",
            "proxy_changed"
        ]
    );
    for row in rows {
        assert!(row.before.is_some());
        assert!(row.after.is_some());
    }
    let dns = rows
        .iter()
        .find(|row| row.field.as_deref() == Some("dns_servers"))
        .unwrap();
    assert_eq!(dns.before, Some(serde_json::json!(["192.168.1.1"])));
    assert_eq!(dns.after, Some(serde_json::json!(["8.8.8.8"])));
    assert_eq!(dns.kind.event_type, EventType::ConfigurationChanged);
    assert_eq!(
        Normalizer::new().normalize(dns.clone()).get().importance,
        Importance::Important
    );

    let proxy = rows
        .iter()
        .find(|row| row.field.as_deref() == Some("proxy"))
        .unwrap();
    assert_eq!(proxy.kind.event_type, EventType::ConfigurationChanged);
    assert_eq!(proxy.after.as_ref().unwrap()["server"], "127.0.0.1:8888");
    assert_eq!(
        Normalizer::new().normalize(proxy.clone()).get().importance,
        Importance::Important
    );
}

#[test]
fn adapter_and_vpn_appearance_emit_distinct_events() {
    let provider = ScriptedNetwork::new(NetworkState::default());
    let mut collector = NetworkCollector::with_clock(
        provider.clone(),
        network::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(network::ID).unwrap().context)
        .unwrap();
    cycle(&mut collector, 1);

    provider.replace(state(
        vec![adapter(
            "{VPN}",
            "Tailscale",
            &["100.64.0.1/32"],
            &["100.100.100.100"],
            &[],
            true,
        )],
        ProxyConfiguration::default(),
    ));
    let appeared = cycle(&mut collector, 2);
    let kinds: Vec<_> = observations(&appeared)
        .iter()
        .map(|row| {
            row.details.as_ref().unwrap()["change_kind"]
                .as_str()
                .unwrap()
        })
        .collect();
    assert_eq!(kinds, ["adapter_changed", "vpn_detected"]);
}

#[test]
fn stale_baseline_detects_downtime_dns_change_without_replaying_it() {
    let live = live_context(network::ID).unwrap();
    let old = state(
        vec![adapter(
            "{ETHERNET}",
            "Ethernet",
            &["192.168.1.10/24"],
            &["192.168.1.1"],
            &["192.168.1.1"],
            false,
        )],
        ProxyConfiguration::default(),
    );
    let snapshot = NetworkSnapshot {
        acknowledged: old,
        pending: None,
        established: true,
    };
    let baseline = Baseline {
        collector_id: network::ID.to_owned(),
        entity_kind: network::BASELINE_KIND.to_owned(),
        snapshot: serde_json::to_value(snapshot).unwrap(),
        captured_at: 1_000,
        truncated: false,
    };
    live.db
        .write(move |connection| {
            baseline_repository::store(connection, &Sanitizer::new().sanitize(baseline).0)
        })
        .unwrap();

    let current = state(
        vec![adapter(
            "{ETHERNET}",
            "Ethernet",
            &["192.168.1.10/24"],
            &["8.8.8.8"],
            &["192.168.1.1"],
            false,
        )],
        ProxyConfiguration::default(),
    );
    let provider = ScriptedNetwork::new(current);
    let mut collector = NetworkCollector::with_clock(
        provider,
        network::catalog_descriptor().unwrap(),
        ManualClock::at(1_000 + DOWNTIME_GAP_MS + 1),
    );
    collector.start(live.context).unwrap();
    let detected = cycle(&mut collector, 1);
    assert_eq!(
        observations(&detected).len(),
        1,
        "unexpected downtime observations: {:?}",
        observations(&detected)
            .iter()
            .map(|row| (&row.field, &row.before, &row.after))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        observations(&detected)[0].field.as_deref(),
        Some("dns_servers")
    );
    assert!(observations(&detected)[0].occurred_at_is_uncertain);
    assert!(matches!(cycle(&mut collector, 2), CollectorSnapshot::Noop));
}

#[test]
fn credential_proxy_is_canonical_before_persist_and_does_not_repeat_after_restart() {
    let live = live_context(network::ID).unwrap();
    let credential_proxy = state(
        vec![],
        ProxyConfiguration {
            enabled: true,
            server: Some("http://user:secret@proxy.example:8080".to_owned()),
            bypass: None,
            auto_config_url: Some("https://user:secret@proxy.example/proxy.pac".to_owned()),
        },
    );
    let provider = ScriptedNetwork::new(credential_proxy.clone());
    let mut first = NetworkCollector::with_clock(
        provider.clone(),
        network::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    first.start(live.context.clone()).unwrap();
    let initial = first
        .snapshot_batch(SnapshotContext::for_devtools(1))
        .unwrap();
    let CollectorSnapshot::Atomic {
        token,
        observations,
        baseline,
        ..
    } = initial
    else {
        panic!("first network snapshot must establish a baseline");
    };
    assert!(observations.is_empty());
    let baseline = baseline.unwrap();
    assert!(!baseline.snapshot.to_string().contains("secret"));
    let baseline = Baseline {
        collector_id: network::ID.to_owned(),
        entity_kind: baseline.entity_kind,
        snapshot: baseline.snapshot,
        captured_at: baseline.captured_at,
        truncated: false,
    };
    live.db
        .write(move |connection| {
            baseline_repository::store(connection, &Sanitizer::new().sanitize(baseline).0)
        })
        .unwrap();
    first
        .commit_succeeded(token, &CollectorCommitReceipt { event_ids: vec![] })
        .unwrap();
    first.stop();

    let mut restarted = NetworkCollector::with_clock(
        provider,
        network::catalog_descriptor().unwrap(),
        ManualClock::at(2_000),
    );
    restarted.start(live.context).unwrap();
    let after_restart = restarted
        .snapshot_batch(SnapshotContext::for_devtools(2))
        .unwrap();
    assert!(
        matches!(after_restart, CollectorSnapshot::Noop),
        "credential proxy repeated after restart: {after_restart:?}"
    );
}

#[test]
fn provider_owns_wake_registration_lifecycle() {
    let provider = ScriptedNetwork::new(NetworkState::default())
        .with_tier(NetworkWakeTier::IpHelperAndRegistry);
    let registrations = provider.registrations.clone();
    let unregisters = provider.unregisters.clone();
    let mut collector = NetworkCollector::with_clock(
        provider,
        network::catalog_descriptor().unwrap(),
        ManualClock::at(1_000),
    );
    collector
        .start(live_context(network::ID).unwrap().context)
        .unwrap();
    assert_eq!(collector.wake_tier(), NetworkWakeTier::IpHelperAndRegistry);
    assert_eq!(registrations.load(Ordering::Acquire), 1);
    collector.stop();
    assert_eq!(unregisters.load(Ordering::Acquire), 1);
}

#[cfg(windows)]
#[test]
#[ignore = "real IP Helper and registry performance gate"]
fn real_network_sweep_stays_inside_the_phase_budget() {
    use std::time::Duration;

    use what_changed_lib::platform::windows::network::WindowsNetworkProvider;

    let provider = WindowsNetworkProvider::default();
    let cancel = CancelToken::default();
    let mut elapsed = Vec::new();
    let mut adapter_count = 0usize;
    for _ in 0..5 {
        let started = Instant::now();
        let snapshot = provider
            .network_state(&cancel, started + Duration::from_secs(15))
            .unwrap();
        elapsed.push(started.elapsed());
        adapter_count = snapshot.adapters.len();
        assert!(adapter_count > 0, "Windows exposes at least one adapter");
    }
    elapsed.sort_unstable();
    assert!(
        elapsed[4] <= Duration::from_millis(2_500),
        "network sweep p95 {:?} exceeded 2500 ms",
        elapsed[4]
    );
    eprintln!(
        "network adapters={adapter_count}, sweep p95={:?}",
        elapsed[4]
    );
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
