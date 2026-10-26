#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::VecDeque;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use what_changed_lib::collectors::eventlog::{
    event_message, parse_event_xml, EventLogCollector, EventLogSource, RenderedEvent, CHANNELS,
    SUBSCRIPTIONS,
};
use what_changed_lib::collectors::normalize::Normalizer;
use what_changed_lib::collectors::registry::CollectorRegistry;
use what_changed_lib::collectors::testing::live_context;
use what_changed_lib::collectors::{
    CancelToken, Collector, ObservationClock, RawObservation, WakeHandle,
};
use what_changed_lib::domain::{EventType, Importance};
use what_changed_lib::error::AppResult;
use what_changed_lib::privacy::REDACTED;

const FIXTURES: [(&str, &str, u32, u8, &str); 10] = [
    (
        include_str!("fixtures/eventlog/01-application-error-1000.xml"),
        "Application Error",
        1000,
        2,
        "sample-app.exe | sample-module.dll | c0000005 | C:\\SANITIZED\\sample-app.exe",
    ),
    (
        include_str!("fixtures/eventlog/02-application-hang-1002.xml"),
        "Application Hang",
        1002,
        2,
        "editor.exe | 1.2.3.4",
    ),
    (
        include_str!("fixtures/eventlog/03-dotnet-runtime-1026.xml"),
        ".NET Runtime",
        1026,
        2,
        "Application: worker.exe | Framework Version: 8.0.0 | Description: process terminated",
    ),
    (
        include_str!("fixtures/eventlog/04-eventlog-6005.xml"),
        "EventLog",
        6005,
        4,
        "SANITIZED",
    ),
    (
        include_str!("fixtures/eventlog/05-service-control-manager-7031.xml"),
        "Service Control Manager",
        7031,
        2,
        "Sample Service | 1 | Restart the service",
    ),
    (
        include_str!("fixtures/eventlog/06-eventlog-6008.xml"),
        "EventLog",
        6008,
        2,
        "12:00:00 AM | 9/20/2026",
    ),
    (
        include_str!("fixtures/eventlog/07-kernel-power-41.xml"),
        "Microsoft-Windows-Kernel-Power",
        41,
        1,
        "0 | 0",
    ),
    (
        include_str!("fixtures/eventlog/08-user32-1074.xml"),
        "User32",
        1074,
        4,
        "shutdown.exe | Operating System: Reconfiguration",
    ),
    (
        include_str!("fixtures/eventlog/09-windows-update-19.xml"),
        "Microsoft-Windows-WindowsUpdateClient",
        19,
        4,
        "Sanitized Cumulative Update (KB0000000) | 00000000-0000-0000-0000-000000000000",
    ),
    (
        include_str!("fixtures/eventlog/10-setup-servicing-2.xml"),
        "Microsoft-Windows-Servicing",
        2,
        4,
        "Package_for_Sanitized_Update | Installed",
    ),
];

#[derive(Clone, Default)]
struct ScriptedSource {
    queue: Arc<Mutex<VecDeque<RenderedEvent>>>,
}

impl ScriptedSource {
    fn push(&self, channel: &str, xml: &str) {
        self.queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push_back(RenderedEvent {
                channel: channel.to_owned(),
                xml: xml.to_owned(),
            });
    }
}

impl EventLogSource for ScriptedSource {
    fn start(&mut self, _wake: WakeHandle, _cancel: CancelToken) -> AppResult<()> {
        Ok(())
    }

    fn drain(&mut self) -> AppResult<Vec<RenderedEvent>> {
        Ok(self
            .queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .drain(..)
            .collect())
    }

    fn stop(&mut self) {}
}

#[derive(Clone)]
struct Clock(Arc<AtomicI64>);

impl ObservationClock for Clock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::Acquire)
    }
}

fn descriptor() -> what_changed_lib::domain::CollectorDescriptor {
    CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == "eventlog")
        .unwrap()
        .descriptor
        .clone()
}

fn snapshot(source: &ScriptedSource, channel: &str, xml: &str) -> Vec<RawObservation> {
    let clock = Clock(Arc::new(AtomicI64::new(1_800_000_000_000)));
    let mut collector = EventLogCollector::with_clock(source.clone(), descriptor(), clock);
    let live = live_context("eventlog").unwrap();
    collector.start(live.context).unwrap();
    source.push(channel, xml);
    collector.snapshot().unwrap()
}

#[test]
fn all_ten_sanitized_rendered_fixtures_parse_to_expected_fields() {
    let channels = [
        "Application",
        "Application",
        "Application",
        "System",
        "System",
        "System",
        "System",
        "System",
        "Microsoft-Windows-WindowsUpdateClient/Operational",
        "Setup",
    ];
    for ((xml, provider, event_id, level, message), channel) in FIXTURES.into_iter().zip(channels) {
        let parsed = parse_event_xml(channel, xml).unwrap();
        assert_eq!(parsed.provider, provider);
        assert_eq!(parsed.event_id, event_id);
        assert_eq!(parsed.level, level);
        assert_eq!(event_message(&parsed), message);
        assert!(parsed.timestamp_ms.is_some());
    }
}

#[test]
fn catalog_contains_exactly_the_four_approved_channels_and_queries() {
    assert_eq!(CHANNELS.len(), 4);
    assert_eq!(SUBSCRIPTIONS.map(|(channel, _)| channel), CHANNELS);
    assert!(SUBSCRIPTIONS.iter().all(|(channel, query)| {
        (*channel == "Setup" || *channel == "Microsoft-Windows-WindowsUpdateClient/Operational")
            == (*query == "*")
    }));
}

#[test]
fn application_error_maps_to_a_high_confidence_system_failure() {
    let events = snapshot(&ScriptedSource::default(), "Application", FIXTURES[0].0);
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.kind.event_type, EventType::Failed);
    assert_eq!(
        event.details.as_ref().unwrap()["event_name"],
        "system/error_logged"
    );
    assert_eq!(event.details.as_ref().unwrap()["count"], 1);
    assert!(!event.kind.technical_summary.contains("<Event"));
}

#[test]
fn service_failure_and_unexpected_shutdown_get_specific_semantics() {
    let service = snapshot(&ScriptedSource::default(), "System", FIXTURES[4].0);
    assert_eq!(
        service[0].details.as_ref().unwrap()["event_name"],
        "system/service_failed"
    );
    assert!(matches!(
        service[0].entity.identity,
        what_changed_lib::collectors::EntityIdentity::Service { .. }
    ));

    let shutdown = snapshot(&ScriptedSource::default(), "System", FIXTURES[5].0);
    assert_eq!(
        shutdown[0].details.as_ref().unwrap()["event_name"],
        "system/unexpected_shutdown"
    );
    let normalized = what_changed_lib::collectors::normalize::Normalizer::new()
        .normalize(shutdown.into_iter().next().unwrap());
    assert_eq!(normalized.get().importance, Importance::Critical);
}

#[test]
fn an_empty_data_event_still_records_a_generic_message() {
    let xml = r#"<Event><System><Provider Name="Minimal Provider"/><EventID>2</EventID><Level>4</Level><TimeCreated SystemTime="2026-09-20T01:10:00Z"/></System></Event>"#;
    let events = snapshot(&ScriptedSource::default(), "Setup", xml);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].details.as_ref().unwrap()["message"],
        "Minimal Provider event 2"
    );
}

#[test]
fn events_outside_the_subscription_filter_are_discarded() {
    let xml = r#"<Event><System><Provider Name="Chatty Provider"/><EventID>55</EventID><Level>4</Level><TimeCreated SystemTime="2026-09-20T01:10:00Z"/></System><EventData><Data>noise</Data></EventData></Event>"#;
    assert!(snapshot(&ScriptedSource::default(), "Application", xml).is_empty());
}

#[test]
fn event_data_secrets_are_redacted_before_they_can_reach_storage() {
    let secret = "postgres://user:VerySecret123@db.internal.example/app";
    let xml = format!(
        r#"<Event><System><Provider Name="Application Error"/><EventID>1000</EventID><Level>2</Level><TimeCreated SystemTime="2026-09-20T01:10:00Z"/></System><EventData><Data Name="Payload">{secret}</Data></EventData></Event>"#
    );
    let raw = snapshot(&ScriptedSource::default(), "Application", &xml);
    let normalized = Normalizer::new().normalize_all(raw);
    let serialized = serde_json::to_string(normalized[0].get()).unwrap();
    assert!(!serialized.contains("VerySecret123"));
    assert!(serialized.contains(REDACTED));
}

#[test]
#[ignore = "real Windows subscription lifetime stress gate"]
fn real_subscriptions_open_and_close_two_hundred_times_without_callback_lifetime_leaks() {
    use what_changed_lib::platform::windows::eventlog::WindowsEventLogSource;

    let live = live_context("eventlog-subscription-stress").unwrap();
    for _ in 0..200 {
        let mut source = WindowsEventLogSource::new();
        source
            .start(live.context.wake.clone(), live.context.cancel.clone())
            .unwrap();
        source.stop();
    }
}
