//! The things events are about, and the proven relationships between them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use super::enums::{text_enum, EntityType};

/// A thing on the machine with a stable identity: a service, a port, a file.
///
/// Entities are the spine of the analysis graph. An event says what happened;
/// an entity is what it happened to, and it is what relationships connect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Entity {
    /// Stable id per the grammar in `ids`.
    pub id: String,
    /// What kind of thing.
    pub entity_type: EntityType,
    /// The name a person recognises. Updated as it changes; the id does not.
    pub display_name: String,
    /// First event about it, epoch milliseconds UTC.
    pub first_seen: i64,
    /// Most recent event about it, epoch milliseconds UTC.
    pub last_seen: i64,
    /// Anything a collector wants to keep about it, already redacted.
    #[serde(default)]
    pub metadata: Option<Value>,
}

text_enum! {
    /// How one entity relates to another. Every kind here is *structural*:
    /// it was observed, not inferred, and it is the only edge class the
    /// analysis engine lets reach its strongest confidence.
    RelationshipKind, "relationship kind" {
        /// A process holds a listening port.
        ListensOn => "listens_on",
        /// A port is held by a process (the reverse edge, stored explicitly
        /// so both directions are one index lookup).
        OpenedBy => "opened_by",
        /// A config file configures an entity, by evidence.
        ConfigOf => "config_of",
        /// An installed application provides a service.
        Provides => "provides",
        /// A container publishes a port.
        Publishes => "publishes",
    }
}

/// A directed edge between two entities, with the event that proved it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Relationship {
    /// Where the edge starts.
    pub source_id: String,
    /// Where it points.
    pub target_id: String,
    /// What the edge means.
    pub kind: RelationshipKind,
    /// The event that recorded this edge, so it can be shown as evidence. It
    /// becomes `None` if that event has since expired; the edge outlives it.
    pub event_id: Option<i64>,
    /// When the edge was recorded, epoch milliseconds UTC.
    pub recorded_at: i64,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::super::enums::UnknownVariant;
    use super::*;
    use std::str::FromStr;

    #[test]
    fn every_relationship_kind_round_trips() {
        assert_eq!(RelationshipKind::ALL.len(), 5);
        for kind in RelationshipKind::ALL {
            assert_eq!(RelationshipKind::from_str(kind.as_str()).unwrap(), *kind);
        }
        let error: UnknownVariant = RelationshipKind::from_str("guesses").unwrap_err();
        assert_eq!(error.kind, "relationship kind");
    }
}
