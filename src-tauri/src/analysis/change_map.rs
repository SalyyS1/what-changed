//! Stable, bounded payload shaping for the Change Map.
//!
//! This module is the causal boundary for the visualization. The webview may
//! position these facts, but it never decides which nodes form a chain or
//! whether an edge is structural.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{Category, EntityType, Importance, Investigation, SystemEvent, TimeRange};

pub const MAX_CHANGE_MAP_NODES: usize = 3_000;
pub const MAX_CHANGE_MAP_EDGES: usize = 6_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChangeMapRequest {
    Investigation {
        investigation_id: i64,
    },
    Window {
        range: TimeRange,
        #[serde(default)]
        categories: Vec<Category>,
        #[serde(default)]
        importance_min: Option<Importance>,
        #[serde(default)]
        entity_id: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ChangeMapLane {
    Process,
    Port,
    Config,
    Service,
    Application,
    Update,
    Driver,
    Network,
    Startup,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ChangeMapLaneData {
    pub id: ChangeMapLane,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ChangeMapNode {
    pub id: String,
    pub event_id: i64,
    pub timestamp_ms: i64,
    pub category: Category,
    pub lane: ChangeMapLane,
    pub importance: Importance,
    pub label: String,
    pub chain_ids: Vec<String>,
    pub chain_depth: u16,
    pub is_suspect: bool,
    /// True when the immutable investigation snapshot supplied this node
    /// because the original event row is no longer retained.
    pub snapshot_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ChangeMapEdgeKind {
    Structural,
    Temporal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ChangeMapEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: ChangeMapEdgeKind,
    pub chain_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ChangeMapData {
    pub nodes: Vec<ChangeMapNode>,
    pub edges: Vec<ChangeMapEdge>,
    pub symptom_time_ms: Option<i64>,
    pub window_start_ms: i64,
    pub window_end_ms: i64,
    pub lanes: Vec<ChangeMapLaneData>,
    pub truncated: bool,
    pub total_nodes: usize,
    pub omitted_nodes: usize,
    pub total_edges: usize,
    pub omitted_edges: usize,
}

#[derive(Debug, Clone)]
struct NodeWork {
    node: ChangeMapNode,
    causal: bool,
}

/// Shape a filtered Timeline window without letting hidden events leak back
/// through an analysis chain. A partially filtered chain is removed instead
/// of connecting its surviving endpoints across an unshown event.
pub fn build_window(
    investigation: &Investigation,
    source_events: &[SystemEvent],
    total_events: u64,
    dropped_events: u64,
) -> ChangeMapData {
    let visible = source_events
        .iter()
        .map(|event| event.id)
        .collect::<BTreeSet<_>>();
    let mut scoped = investigation.clone();
    scoped.suspects.retain_mut(|suspect| {
        if !visible.contains(&suspect.event_id) {
            return false;
        }
        let complete_chain = suspect.chain.iter().all(|step| {
            step.event_ids
                .first()
                .is_some_and(|event_id| visible.contains(event_id))
        });
        if !complete_chain {
            suspect.chain.clear();
        }
        true
    });
    scoped.total_events_in_window = usize::try_from(total_events).unwrap_or(usize::MAX);
    scoped.dropped_events = usize::try_from(dropped_events).unwrap_or(usize::MAX);
    let mut data = build(&scoped, source_events);
    data.symptom_time_ms = None;
    data
}

/// Shape an investigation and any still-retained source rows into a map.
///
/// `source_events` may be empty. Stored suspect titles and chain steps are an
/// immutable fallback, so a historical investigation remains mappable after
/// event retention has removed the source rows.
pub fn build(investigation: &Investigation, source_events: &[SystemEvent]) -> ChangeMapData {
    let suspect_ids = investigation
        .suspects
        .iter()
        .map(|suspect| suspect.event_id)
        .collect::<BTreeSet<_>>();
    let mut nodes = BTreeMap::<i64, NodeWork>::new();

    for event in source_events {
        nodes.entry(event.id).or_insert_with(|| NodeWork {
            node: node_from_event(event, suspect_ids.contains(&event.id)),
            causal: suspect_ids.contains(&event.id),
        });
    }

    let source_by_id = source_events
        .iter()
        .map(|event| (event.id, event))
        .collect::<BTreeMap<_, _>>();
    let mut structural = BTreeMap::<(String, String), BTreeSet<String>>::new();

    for suspect in &investigation.suspects {
        let chain_id = format!("chain:{}", suspect.event_id);
        if suspect.chain.is_empty() {
            let entry = nodes.entry(suspect.event_id).or_insert_with(|| NodeWork {
                node: snapshot_node(
                    suspect.event_id,
                    suspect.timestamp_ms,
                    &suspect.title,
                    0,
                    true,
                ),
                causal: true,
            });
            attach_to_chain(entry, &chain_id, 0, true);
            continue;
        }

        let mut representatives = Vec::new();
        for (depth, step) in suspect.chain.iter().enumerate() {
            let Some(event_id) = step.event_ids.first().copied() else {
                continue;
            };
            let entry = nodes.entry(event_id).or_insert_with(|| {
                source_by_id.get(&event_id).map_or_else(
                    || NodeWork {
                        node: snapshot_node(
                            event_id,
                            step.timestamp_ms,
                            &step.label,
                            depth,
                            event_id == suspect.event_id,
                        ),
                        causal: true,
                    },
                    |event| NodeWork {
                        node: node_from_event(event, event_id == suspect.event_id),
                        causal: true,
                    },
                )
            });
            attach_to_chain(entry, &chain_id, depth, event_id == suspect.event_id);
            representatives.push(event_id);
        }
        for pair in representatives.windows(2) {
            let source = event_node_id(pair[0]);
            let target = event_node_id(pair[1]);
            structural
                .entry((source, target))
                .or_default()
                .insert(chain_id.clone());
        }
    }

    // The investigation records the pre-downsample row count. Keep it in the
    // disclosure even when only the bounded analysis corpus is still
    // available, so "3,000 shown" never masquerades as the whole window.
    let total_nodes = nodes.len().max(investigation.total_events_in_window);
    let mut selected = nodes.into_values().collect::<Vec<_>>();
    selected.sort_by(|left, right| {
        right
            .causal
            .cmp(&left.causal)
            .then_with(|| right.node.importance.cmp(&left.node.importance))
            .then_with(|| left.node.timestamp_ms.cmp(&right.node.timestamp_ms))
            .then_with(|| left.node.event_id.cmp(&right.node.event_id))
    });
    selected.truncate(MAX_CHANGE_MAP_NODES);
    selected.sort_by_key(|work| (work.node.timestamp_ms, work.node.event_id));
    let kept_ids = selected
        .iter()
        .map(|work| work.node.id.clone())
        .collect::<BTreeSet<_>>();
    let nodes = selected
        .into_iter()
        .map(|work| work.node)
        .collect::<Vec<_>>();

    let mut structural_edges = structural
        .into_iter()
        .filter(|((source, target), _)| kept_ids.contains(source) && kept_ids.contains(target))
        .map(|((source, target), chains)| ChangeMapEdge {
            id: format!("structural:{source}:{target}"),
            source,
            target,
            kind: ChangeMapEdgeKind::Structural,
            chain_ids: chains.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    structural_edges.sort_by(|left, right| left.id.cmp(&right.id));

    let structural_pairs = structural_edges
        .iter()
        .map(|edge| (edge.source.clone(), edge.target.clone()))
        .collect::<BTreeSet<_>>();
    let temporal_edges = nodes
        .windows(2)
        .filter(|pair| !structural_pairs.contains(&(pair[0].id.clone(), pair[1].id.clone())))
        .map(|pair| ChangeMapEdge {
            id: format!("temporal:{}:{}", pair[0].id, pair[1].id),
            source: pair[0].id.clone(),
            target: pair[1].id.clone(),
            kind: ChangeMapEdgeKind::Temporal,
            chain_ids: Vec::new(),
        })
        .collect::<Vec<_>>();

    let total_edges = structural_edges.len().saturating_add(temporal_edges.len());
    structural_edges.extend(temporal_edges);
    structural_edges.truncate(MAX_CHANGE_MAP_EDGES);
    let omitted_nodes = total_nodes.saturating_sub(nodes.len());
    let omitted_edges = total_edges.saturating_sub(structural_edges.len());

    ChangeMapData {
        nodes,
        edges: structural_edges,
        symptom_time_ms: Some(investigation.resolved_window.to_ms),
        window_start_ms: investigation.resolved_window.from_ms,
        window_end_ms: investigation.resolved_window.to_ms,
        lanes: lanes(),
        truncated: omitted_nodes > 0 || omitted_edges > 0 || investigation.dropped_events > 0,
        total_nodes,
        omitted_nodes,
        total_edges,
        omitted_edges,
    }
}

fn attach_to_chain(work: &mut NodeWork, chain_id: &str, depth: usize, suspect: bool) {
    work.causal = true;
    work.node.is_suspect |= suspect;
    let depth = u16::try_from(depth).unwrap_or(u16::MAX);
    work.node.chain_depth = if work.node.chain_ids.is_empty() {
        depth
    } else {
        work.node.chain_depth.min(depth)
    };
    if !work.node.chain_ids.iter().any(|id| id == chain_id) {
        work.node.chain_ids.push(chain_id.to_owned());
        work.node.chain_ids.sort();
    }
}

fn node_from_event(event: &SystemEvent, suspect: bool) -> ChangeMapNode {
    ChangeMapNode {
        id: event_node_id(event.id),
        event_id: event.id,
        timestamp_ms: event.event.timestamp,
        category: event.event.category,
        lane: lane_for(event.event.category, event.event.entity_type),
        importance: event.event.importance,
        label: event.event.user_friendly_summary.clone(),
        chain_ids: Vec::new(),
        chain_depth: 0,
        is_suspect: suspect,
        snapshot_only: false,
    }
}

fn snapshot_node(
    event_id: i64,
    timestamp_ms: i64,
    label: &str,
    depth: usize,
    suspect: bool,
) -> ChangeMapNode {
    let category = snapshot_category(label);
    ChangeMapNode {
        id: event_node_id(event_id),
        event_id,
        timestamp_ms,
        category,
        lane: lane_for(category, EntityType::System),
        importance: Importance::Important,
        label: label.to_owned(),
        chain_ids: Vec::new(),
        chain_depth: u16::try_from(depth).unwrap_or(u16::MAX),
        is_suspect: suspect,
        snapshot_only: true,
    }
}

fn event_node_id(event_id: i64) -> String {
    format!("event:{event_id}")
}

fn snapshot_category(label: &str) -> Category {
    if label.starts_with("Configuration") {
        Category::Configuration
    } else if label.starts_with("Service") {
        Category::Service
    } else if label.starts_with("Port") {
        Category::Network
    } else {
        Category::System
    }
}

fn lane_for(category: Category, entity_type: EntityType) -> ChangeMapLane {
    if entity_type == EntityType::Port {
        return ChangeMapLane::Port;
    }
    match category {
        Category::Process => ChangeMapLane::Process,
        Category::Filesystem | Category::Configuration => ChangeMapLane::Config,
        Category::Service => ChangeMapLane::Service,
        Category::Application => ChangeMapLane::Application,
        Category::Update => ChangeMapLane::Update,
        Category::Driver | Category::Hardware => ChangeMapLane::Driver,
        Category::Network => ChangeMapLane::Network,
        Category::Startup => ChangeMapLane::Startup,
        Category::Docker
        | Category::Git
        | Category::Security
        | Category::Runtime
        | Category::System => ChangeMapLane::System,
    }
}

fn lanes() -> Vec<ChangeMapLaneData> {
    [
        (ChangeMapLane::Process, "Process"),
        (ChangeMapLane::Port, "Port"),
        (ChangeMapLane::Config, "Config"),
        (ChangeMapLane::Service, "Service"),
        (ChangeMapLane::Application, "Application"),
        (ChangeMapLane::Update, "Update"),
        (ChangeMapLane::Driver, "Driver"),
        (ChangeMapLane::Network, "Network"),
        (ChangeMapLane::Startup, "Startup"),
        (ChangeMapLane::System, "System"),
    ]
    .into_iter()
    .map(|(id, label)| ChangeMapLaneData {
        id,
        label: label.to_owned(),
    })
    .collect()
}
