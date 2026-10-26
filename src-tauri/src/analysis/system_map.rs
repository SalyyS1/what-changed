//! Deterministic, bounded relationship graph for the System Map.
//!
//! This module consumes only persisted entities and proven relationships. It
//! never infers an edge and never reaches into collectors or platform code.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{EntityType, Importance, RelationshipKind};

pub const MAX_SYSTEM_MAP_NODES: usize = 400;
pub const MAX_SYSTEM_MAP_EDGES: usize = 1_200;
pub const MAX_SYSTEM_MAP_DEPTH: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
pub struct EntityRef {
    pub entity_id: String,
    pub entity_type: EntityType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SystemMapNode {
    pub entity_id: String,
    pub entity_type: EntityType,
    pub display_name: String,
    pub hop: u8,
    pub degree: u32,
    pub last_changed_at: Option<i64>,
    pub recent_event_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SystemMapEdge {
    pub source: String,
    pub target: String,
    pub kind: RelationshipKind,
    /// Observation timestamp retained by the relationship row. The schema
    /// keeps the latest proof for each directed triple.
    pub recorded_at: i64,
    pub evidence_event_id: Option<i64>,
}

/// One actual recent event for the focused node. The payload contains at most
/// five, which is enough for the dependency panel without another round trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SystemMapRecentEvent {
    pub event_id: i64,
    pub timestamp_ms: i64,
    pub summary: String,
    pub importance: Importance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EntityKindAvailability {
    pub kind: EntityType,
    pub count: u32,
    pub available: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SystemMapData {
    pub nodes: Vec<SystemMapNode>,
    pub edges: Vec<SystemMapEdge>,
    pub focus: Option<EntityRef>,
    pub truncated: bool,
    pub total_nodes_available: u32,
    /// Relationships whose source or target entity no longer resolves.
    pub dropped_edges: u32,
    pub kind_availability: Vec<EntityKindAvailability>,
    pub focused_recent_events: Vec<SystemMapRecentEvent>,
}
