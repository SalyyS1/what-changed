//! The one event shape every collector emits and every screen reads.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use super::enums::{Category, Confidence, EntityType, EventType, Importance, Severity};

/// A change as a collector describes it, before it has a database id.
///
/// The fields are the brief's normalized event, in its order. One field sits
/// outside that list and is documented as such: `occurred_at_is_uncertain` is
/// provenance, not description. A change discovered by diffing a persisted
/// baseline after the app was closed has a *detection* time and no known
/// occurrence time, and the analysis engine must never reason over a
/// fabricated one — so the flag rides with the event from the collector that
/// discovered it to the column that stores it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct NormalizedEvent {
    /// When it happened, epoch milliseconds UTC. When `occurred_at_is_uncertain`
    /// is set this is when it was *detected*, which is the best that is known.
    pub timestamp: i64,
    /// What part of the machine this is about.
    pub category: Category,
    /// What happened.
    pub event_type: EventType,
    /// What kind of thing it happened to.
    pub entity_type: EntityType,
    /// Stable identifier of the thing, per the grammar in `ids`.
    pub entity_id: String,
    /// The name a person recognises: a service name, a file name, a port.
    pub entity_display_name: String,
    /// How bad it was, on its own terms.
    pub severity: Severity,
    /// Whether a person should care.
    pub importance: Importance,
    /// The collector that produced this event.
    pub source_collector: String,
    /// State before the change, already redacted. Never file contents.
    #[serde(default)]
    pub before: Option<Value>,
    /// State after the change, already redacted. Never file contents.
    #[serde(default)]
    pub after: Option<Value>,
    /// Structured config key for a provable key-level change.
    #[serde(default)]
    pub field: Option<String>,
    /// Anything else worth keeping, already redacted.
    #[serde(default)]
    pub metadata: Option<Value>,
    /// Other entities this change touches, by id.
    #[serde(default)]
    pub related_entity_ids: Vec<String>,
    /// One sentence in the product's voice.
    pub user_friendly_summary: String,
    /// The same fact with the identifiers a technical reader wants.
    pub technical_summary: String,
    /// How sure the collector is of the facts above.
    pub confidence: Confidence,
    /// Whether redaction replaced something in this event.
    #[serde(default)]
    pub sensitive: bool,
    /// Provenance: the timestamp is a detection time, not an occurrence time.
    #[serde(default)]
    pub occurred_at_is_uncertain: bool,
}

impl NormalizedEvent {
    /// The part of an event that repeats when the same thing happens again.
    pub fn signature(&self) -> EventSignature {
        EventSignature {
            category: self.category,
            event_type: self.event_type,
            entity_id: self.entity_id.clone(),
        }
    }
}

/// What makes two events "the same thing happening again": the entity, the
/// category and what happened to it. This is how the details drawer can say
/// "this happened 2 times before".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EventSignature {
    /// What part of the machine.
    pub category: Category,
    /// What happened.
    pub event_type: EventType,
    /// To what.
    pub entity_id: String,
}

/// An event as the repository returns it: the same shape, plus its id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SystemEvent {
    /// Database id; the second half of a pagination cursor.
    pub id: i64,
    /// Everything else.
    #[serde(flatten)]
    pub event: NormalizedEvent,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> NormalizedEvent {
        NormalizedEvent {
            timestamp: 1_700_000_000_000,
            category: Category::Service,
            event_type: EventType::Stopped,
            entity_type: EntityType::Service,
            entity_id: "service:Spooler".to_owned(),
            entity_display_name: "Print Spooler".to_owned(),
            severity: Severity::Warning,
            importance: Importance::Important,
            source_collector: "services".to_owned(),
            before: Some(json!({ "state": "running" })),
            after: Some(json!({ "state": "stopped" })),
            field: None,
            metadata: None,
            related_entity_ids: vec![],
            user_friendly_summary: "Print Spooler stopped".to_owned(),
            technical_summary: "service Spooler: running -> stopped".to_owned(),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: false,
        }
    }

    #[test]
    fn a_system_event_serializes_flat_with_its_id_first() {
        let event = SystemEvent {
            id: 42,
            event: sample(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["id"], 42);
        assert_eq!(json["category"], "service");
        assert_eq!(json["occurred_at_is_uncertain"], false);
        assert!(json.get("event").is_none(), "the wrapper must not show");

        let back: SystemEvent = serde_json::from_value(json).unwrap();
        assert_eq!(back, event);
    }

    #[test]
    fn optional_fields_default_when_absent() {
        let text = r#"{
            "timestamp": 1, "category": "process", "event_type": "started",
            "entity_type": "process", "entity_id": "process:abc", "entity_display_name": "x.exe",
            "severity": "info", "importance": "informational", "source_collector": "process",
            "user_friendly_summary": "x started", "technical_summary": "x", "confidence": "low"
        }"#;
        let event: NormalizedEvent = serde_json::from_str(text).unwrap();
        assert!(event.before.is_none());
        assert!(event.field.is_none());
        assert!(event.related_entity_ids.is_empty());
        assert!(!event.sensitive);
        assert!(!event.occurred_at_is_uncertain);
    }

    #[test]
    fn the_signature_is_the_entity_and_what_happened_to_it() {
        let signature = sample().signature();
        assert_eq!(signature.entity_id, "service:Spooler");
        assert_eq!(signature.event_type, EventType::Stopped);
        assert_eq!(signature.category, Category::Service);
    }
}
