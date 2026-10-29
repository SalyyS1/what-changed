//! Windows Event Log subscription drain, targeted XML parsing and storm control.
//!
//! Platform callbacks only render an event to XML and enqueue it. Parsing,
//! classification, redaction and persistence stay on the serial collector
//! worker, away from the Event Log service's callback thread.

use std::collections::BTreeMap;

use chrono::DateTime;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use serde_json::json;

use crate::collectors::{
    AutomaticCheckpointTrigger, CancelToken, Collector, CollectorContext, CollectorDescriptor,
    EntityIdentity, ObservationClock, ObservationKind, RawEntity, RawObservation,
    SystemObservationClock, WakeHandle,
};
use crate::domain::{Category, ClassifierContext, Confidence, EventType, Severity};
use crate::error::{AppError, AppResult};

pub const ID: &str = "eventlog";
pub const DEDUP_WINDOW_MS: i64 = 60_000;
const MAX_DEDUP_KEYS: usize = 2_048;
const MAX_DATA_FIELDS: usize = 64;
const MAX_DATA_VALUE_BYTES: usize = 16 * 1_024;

/// The only four non-elevated channels this product subscribes to.
pub const CHANNELS: [&str; 4] = [
    "Application",
    "System",
    "Microsoft-Windows-WindowsUpdateClient/Operational",
    "Setup",
];

// The `"Security"` channel is deliberately absent: its measured non-elevated
// subscription is unauthorized, and this application never asks for admin.

/// XPath queries applied by `EvtSubscribe` and repeated defensively here.
pub const SUBSCRIPTIONS: [(&str, &str); 4] = [
    (
        "Application",
        "*[System[(Level<=2 or EventID=1000 or EventID=1001 or EventID=1002 or EventID=1026)]]",
    ),
    (
        "System",
        "*[System[(Level<=2 or EventID=41 or EventID=6005 or EventID=6006 or EventID=6008 or EventID=7000 or EventID=7001 or EventID=7009 or EventID=7011 or EventID=7023 or EventID=7024 or EventID=7031 or EventID=7034 or EventID=1074)]]",
    ),
    ("Microsoft-Windows-WindowsUpdateClient/Operational", "*"),
    ("Setup", "*"),
];

/// One bounded callback payload. It is still untrusted XML until the collector
/// parses it, and never reaches storage without the ordinary sanitizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedEvent {
    pub channel: String,
    pub xml: String,
}

/// Provider seam around subscription lifetime and its bounded queue.
pub trait EventLogSource: Send + 'static {
    fn start(&mut self, wake: WakeHandle, cancel: CancelToken) -> AppResult<()>;
    fn drain(&mut self) -> AppResult<Vec<RenderedEvent>>;
    fn stop(&mut self);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDataField {
    pub name: Option<String>,
    pub value: String,
}

/// The small subset of rendered XML the product intentionally retains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEventLog {
    pub channel: String,
    pub provider: String,
    pub event_id: u32,
    pub level: u8,
    pub timestamp_ms: Option<i64>,
    pub data: Vec<EventDataField>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DedupKey {
    provider: String,
    event_id: u32,
    first_data_value: String,
}

#[derive(Debug, Clone)]
struct DedupState {
    window_started_ms: i64,
    last_seen_ms: i64,
    total: u64,
    reported: u64,
    exemplar: ParsedEventLog,
}

/// Notification-driven Event Log collector.
pub struct EventLogCollector<S, C = SystemObservationClock> {
    source: S,
    clock: C,
    descriptor: CollectorDescriptor,
    context: Option<CollectorContext>,
    dedup: BTreeMap<DedupKey, DedupState>,
}

impl<S: EventLogSource> EventLogCollector<S, SystemObservationClock> {
    pub fn new(source: S, descriptor: CollectorDescriptor) -> Self {
        Self::with_clock(source, descriptor, SystemObservationClock)
    }
}

impl<S: EventLogSource, C: ObservationClock> EventLogCollector<S, C> {
    pub fn with_clock(source: S, descriptor: CollectorDescriptor, clock: C) -> Self {
        Self {
            source,
            clock,
            descriptor,
            context: None,
            dedup: BTreeMap::new(),
        }
    }

    fn ingest(
        &mut self,
        parsed: ParsedEventLog,
        detected_at: i64,
        observations: &mut Vec<RawObservation>,
    ) {
        let occurred_at = parsed.timestamp_ms.unwrap_or(detected_at).min(detected_at);
        let key = DedupKey {
            provider: parsed.provider.to_ascii_lowercase(),
            event_id: parsed.event_id,
            first_data_value: parsed
                .data
                .first()
                .map(|field| field.value.to_ascii_lowercase())
                .unwrap_or_default(),
        };

        if let Some(state) = self.dedup.get_mut(&key) {
            if occurred_at.abs_diff(state.window_started_ms) <= DEDUP_WINDOW_MS as u64 {
                state.last_seen_ms = state.last_seen_ms.max(occurred_at);
                state.total = state.total.saturating_add(1);
                state.exemplar = parsed;
                return;
            }
            if state.total > state.reported {
                observations.push(observation(
                    &state.exemplar,
                    state.last_seen_ms,
                    state.total.saturating_sub(state.reported),
                ));
            }
        }

        self.dedup.insert(
            key,
            DedupState {
                window_started_ms: occurred_at,
                last_seen_ms: occurred_at,
                total: 1,
                reported: 0,
                exemplar: parsed,
            },
        );
    }

    fn flush_affected(&mut self, observations: &mut Vec<RawObservation>) {
        for state in self.dedup.values_mut() {
            let unreported = state.total.saturating_sub(state.reported);
            if unreported > 0 {
                observations.push(observation(&state.exemplar, state.last_seen_ms, unreported));
                state.reported = state.total;
            }
        }

        while self.dedup.len() > MAX_DEDUP_KEYS {
            let Some(oldest) = self
                .dedup
                .iter()
                .min_by_key(|(_, state)| state.last_seen_ms)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            self.dedup.remove(&oldest);
        }
    }
}

impl<S: EventLogSource, C: ObservationClock> Collector for EventLogCollector<S, C> {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        self.source
            .start(context.wake.clone(), context.cancel.clone())?;
        self.context = Some(context);
        Ok(())
    }

    fn stop(&mut self) {
        self.source.stop();
        self.context = None;
        self.dedup.clear();
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        if self.context.is_none() {
            return Ok(Vec::new());
        }
        let detected_at = self.clock.now_ms();
        let mut observations = Vec::new();
        for rendered in self.source.drain()? {
            let Ok(parsed) = parse_event_xml(&rendered.channel, &rendered.xml) else {
                tracing::debug!(
                    channel = rendered.channel,
                    "skipped one malformed Event Log record"
                );
                continue;
            };
            if subscription_allows(&parsed) {
                if parsed.channel == "Microsoft-Windows-WindowsUpdateClient/Operational"
                    && parsed.event_id == 43
                {
                    if let Some(context) = &self.context {
                        let _ = context
                            .automatic_checkpoint
                            .try_request(AutomaticCheckpointTrigger::WindowsUpdateInstallStarted);
                    }
                }
                self.ingest(parsed, detected_at, &mut observations);
            }
        }
        self.flush_affected(&mut observations);
        Ok(observations)
    }
}

/// Parse only provider, id, level, time and `<Data>` children. Computer,
/// security identifiers, execution metadata and the original XML are ignored.
pub fn parse_event_xml(channel: &str, xml: &str) -> AppResult<ParsedEventLog> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut stack = Vec::<String>::new();
    let mut provider = None;
    let mut event_id = None;
    let mut level = None;
    let mut timestamp_ms = None;
    let mut data = Vec::new();
    let mut current_data_name = None;

    loop {
        match reader.read_event().map_err(|_| invalid_xml())? {
            Event::Start(start) => {
                let name = local_name(start.name().as_ref())?;
                if stack.last().is_some_and(|parent| parent == "EventData") && name == "Data" {
                    current_data_name = attribute(&reader, &start, b"Name")?;
                }
                stack.push(name);
            }
            Event::Empty(start) => {
                let name = local_name(start.name().as_ref())?;
                if stack.last().is_some_and(|parent| parent == "System") {
                    if name == "Provider" {
                        provider = attribute(&reader, &start, b"Name")?;
                    } else if name == "TimeCreated" {
                        timestamp_ms = attribute(&reader, &start, b"SystemTime")?
                            .and_then(|value| parse_timestamp(&value));
                    }
                }
            }
            Event::Text(text) => {
                let decoded = text.decode().map_err(|_| invalid_xml())?;
                let value = quick_xml::escape::unescape(&decoded)
                    .map_err(|_| invalid_xml())?
                    .trim()
                    .to_owned();
                if value.is_empty() {
                    continue;
                }
                match stack.last().map(String::as_str) {
                    Some("EventID") => event_id = value.parse().ok(),
                    Some("Level") => level = value.parse().ok(),
                    Some("Data") if data.len() < MAX_DATA_FIELDS => {
                        data.push(EventDataField {
                            name: current_data_name.clone(),
                            value: truncate_utf8(&value, MAX_DATA_VALUE_BYTES),
                        });
                    }
                    _ => {}
                }
            }
            Event::CData(text) => {
                if stack.last().is_some_and(|name| name == "Data") && data.len() < MAX_DATA_FIELDS {
                    let value = text.decode().map_err(|_| invalid_xml())?;
                    data.push(EventDataField {
                        name: current_data_name.clone(),
                        value: truncate_utf8(value.trim(), MAX_DATA_VALUE_BYTES),
                    });
                }
            }
            Event::End(end) => {
                if local_name(end.name().as_ref())? == "Data" {
                    current_data_name = None;
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(ParsedEventLog {
        channel: channel.to_owned(),
        provider: provider.ok_or_else(invalid_xml)?,
        event_id: event_id.ok_or_else(invalid_xml)?,
        level: level.unwrap_or(0),
        timestamp_ms,
        data,
    })
}

fn attribute(
    reader: &Reader<&[u8]>,
    start: &BytesStart<'_>,
    wanted: &[u8],
) -> AppResult<Option<String>> {
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|_| invalid_xml())?;
        if local_bytes(attribute.key.as_ref()) == wanted {
            return attribute
                .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
                .map(|value| Some(value.into_owned()))
                .map_err(|_| invalid_xml());
        }
    }
    Ok(None)
}

fn local_name(raw: &[u8]) -> AppResult<String> {
    std::str::from_utf8(local_bytes(raw))
        .map(str::to_owned)
        .map_err(|_| invalid_xml())
}

fn local_bytes(raw: &[u8]) -> &[u8] {
    raw.rsplit(|byte| *byte == b':').next().unwrap_or(raw)
}

fn parse_timestamp(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp_millis())
}

fn truncate_utf8(value: &str, max: usize) -> String {
    if value.len() <= max {
        return value.to_owned();
    }
    let mut boundary = max;
    while !value.is_char_boundary(boundary) {
        boundary = boundary.saturating_sub(1);
    }
    value[..boundary].to_owned()
}

fn invalid_xml() -> AppError {
    AppError::InvalidInput {
        what: "event log XML",
        reason: "the rendered event was malformed or omitted required system fields".to_owned(),
    }
}

fn subscription_allows(event: &ParsedEventLog) -> bool {
    match event.channel.as_str() {
        "Application" => event.level <= 2 || matches!(event.event_id, 1000 | 1001 | 1002 | 1026),
        "System" => {
            event.level <= 2
                || matches!(
                    event.event_id,
                    41 | 6005
                        | 6006
                        | 6008
                        | 7000
                        | 7001
                        | 7009
                        | 7011
                        | 7023
                        | 7024
                        | 7031
                        | 7034
                        | 1074
                )
        }
        "Microsoft-Windows-WindowsUpdateClient/Operational" | "Setup" => true,
        _ => false,
    }
}

fn observation(event: &ParsedEventLog, occurred_at: i64, count: u64) -> RawObservation {
    let service_failure = event.channel == "System"
        && matches!(
            event.event_id,
            7000 | 7001 | 7009 | 7011 | 7023 | 7024 | 7031 | 7034
        );
    let unexpected_shutdown = event.channel == "System" && matches!(event.event_id, 41 | 6008);
    let boot = event.channel == "System" && event.event_id == 6005;
    let restart = event.channel == "System" && matches!(event.event_id, 6006 | 1074);
    let message = event_message(event);
    let service = service_failure
        .then(|| {
            event
                .data
                .first()
                .map(|field| field.value.trim().to_owned())
        })
        .flatten()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Windows service".to_owned());

    let (event_name, category, event_type, severity, entity, summary) = if service_failure {
        (
            "system/service_failed",
            Category::Service,
            EventType::Failed,
            Severity::Error,
            RawEntity {
                identity: EntityIdentity::Service {
                    name: service.clone(),
                },
                display_name: service.clone(),
            },
            format!("{service} reported a service failure"),
        )
    } else if unexpected_shutdown {
        (
            "system/unexpected_shutdown",
            Category::System,
            EventType::Failed,
            Severity::Error,
            system_entity(),
            "Windows reported an unexpected shutdown".to_owned(),
        )
    } else if boot {
        (
            "system/boot",
            Category::System,
            EventType::Started,
            Severity::Info,
            system_entity(),
            "Windows recorded a boot boundary".to_owned(),
        )
    } else if restart {
        (
            "system/restart",
            Category::System,
            EventType::Restarted,
            Severity::Info,
            system_entity(),
            "Windows recorded a restart or shutdown".to_owned(),
        )
    } else {
        (
            "system/error_logged",
            Category::System,
            if event.level <= 2 {
                EventType::Failed
            } else {
                EventType::Modified
            },
            severity(event.level),
            system_entity(),
            format!("{} logged event {}", event.provider, event.event_id),
        )
    };
    let data = event
        .data
        .iter()
        .map(|field| json!({ "name": field.name, "value": field.value }))
        .collect::<Vec<_>>();

    RawObservation {
        collector_id: ID,
        kind: ObservationKind {
            category,
            event_type,
            severity,
            confidence: Confidence::High,
            user_friendly_summary: summary,
            technical_summary: format!(
                "{} event {} level {} on {}: {}",
                event.provider, event.event_id, event.level, event.channel, message
            ),
            classifier: ClassifierContext {
                unexpected_shutdown,
                ..ClassifierContext::default()
            },
        },
        entity,
        before: None,
        after: None,
        field: None,
        occurred_at,
        details: Some(json!({
            "event_name": event_name,
            "channel": event.channel,
            "provider": event.provider,
            "event_id": event.event_id,
            "level": event.level,
            "message": message,
            "data": data,
            "count": count,
        })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: event.timestamp_ms.is_none(),
    }
}

fn system_entity() -> RawEntity {
    RawEntity {
        identity: EntityIdentity::System,
        display_name: "This PC".to_owned(),
    }
}

pub fn event_message(event: &ParsedEventLog) -> String {
    let values = event
        .data
        .iter()
        .map(|field| field.value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        format!("{} event {}", event.provider, event.event_id)
    } else {
        values.join(" | ")
    }
}

/// Descriptor from the canonical collector catalog.
pub fn catalog_descriptor() -> AppResult<CollectorDescriptor> {
    crate::collectors::registry::CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == ID)
        .map(|entry| entry.descriptor.clone())
        .ok_or_else(|| AppError::InvalidInput {
            what: "collector catalog",
            reason: "eventlog descriptor is missing".to_owned(),
        })
}

fn severity(level: u8) -> Severity {
    match level {
        1 | 2 => Severity::Error,
        3 => Severity::Warning,
        _ => Severity::Info,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parser_ignores_machine_and_security_fields() {
        let parsed = parse_event_xml(
            "Application",
            r#"<Event xmlns="http://schemas.microsoft.com/win/2004/08/events/event"><System><Provider Name="Application Error"/><EventID>1000</EventID><Level>2</Level><TimeCreated SystemTime="2026-09-20T01:02:03.004Z"/><Computer>REDACTED-PC</Computer><Security UserID="S-1-5-21-REDACTED"/></System><EventData><Data Name="AppName">sample.exe</Data></EventData></Event>"#,
        )
        .unwrap();
        assert_eq!(parsed.provider, "Application Error");
        assert_eq!(parsed.event_id, 1000);
        assert_eq!(parsed.level, 2);
        assert_eq!(parsed.data[0].value, "sample.exe");
        assert!(parsed.timestamp_ms.is_some());
    }
}
