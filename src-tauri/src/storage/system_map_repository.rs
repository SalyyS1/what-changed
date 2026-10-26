//! Storage-backed bounded traversal for the System Map.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;

use crate::analysis::system_map::{
    EntityKindAvailability, EntityRef, SystemMapData, SystemMapEdge, SystemMapNode,
    SystemMapRecentEvent, MAX_SYSTEM_MAP_DEPTH, MAX_SYSTEM_MAP_EDGES, MAX_SYSTEM_MAP_NODES,
};
use crate::domain::{now_ms, Entity, EntityType, EventQuery, Relationship, RelationshipKind};
use crate::error::AppResult;

use super::{entity_repository, event_repository};

const ROOT_LIMIT: usize = 12;
const RECENT_WINDOW_MS: i64 = 7 * 24 * 60 * 60 * 1_000;

/// Build a stable breadth-first view. An empty kind list means all kinds.
pub fn build(
    conn: &Connection,
    focus: Option<EntityRef>,
    depth: u8,
    kinds: &[EntityType],
) -> AppResult<SystemMapData> {
    let depth = depth.clamp(1, MAX_SYSTEM_MAP_DEPTH);
    let allowed = normalized_kinds(kinds);
    let allowed_list = allowed.iter().copied().collect::<Vec<_>>();
    let availability = kind_availability(conn)?;
    let roots = load_roots(conn, focus.as_ref(), &allowed)?;
    let root_ids = roots
        .iter()
        .map(|entity| entity.id.clone())
        .collect::<Vec<_>>();
    let total_nodes_available = entity_repository::reachable_node_count(
        conn,
        &root_ids,
        &allowed_list,
        focus.as_ref().map(|value| value.entity_id.as_str()),
        depth,
    )?;
    let recent_since = now_ms().saturating_sub(RECENT_WINDOW_MS);

    let mut visited = BTreeMap::<String, (Entity, u8)>::new();
    let mut frontier = Vec::new();
    for entity in roots {
        frontier.push(entity.id.clone());
        visited.insert(entity.id.clone(), (entity, 0));
    }

    let mut relationships = BTreeMap::<(String, String, RelationshipKind), Relationship>::new();
    let mut dropped = BTreeSet::<(String, String, RelationshipKind)>::new();
    let mut traversal_truncated = false;
    for hop in 0..depth {
        if frontier.is_empty() {
            break;
        }
        frontier.sort();
        frontier.dedup();
        let edge_budget = MAX_SYSTEM_MAP_EDGES
            .saturating_add(1)
            .saturating_sub(relationships.len());
        if edge_budget == 0 {
            traversal_truncated = true;
            break;
        }
        let page = entity_repository::relationships_touching_with_limit(
            conn,
            &frontier,
            &allowed_list,
            focus.as_ref().map(|value| value.entity_id.as_str()),
            edge_budget,
        )?;
        traversal_truncated |= page.has_more;
        let touching = page.relationships;
        let endpoint_ids = touching
            .iter()
            .flat_map(|edge| [&edge.source_id, &edge.target_id])
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let resolved = entity_repository::entities_by_ids(conn, &endpoint_ids)?
            .into_iter()
            .map(|entity| (entity.id.clone(), entity))
            .collect::<BTreeMap<_, _>>();
        let mut next = BTreeSet::new();
        'edges: for edge in touching {
            let key = (edge.source_id.clone(), edge.target_id.clone(), edge.kind);
            let (Some(source), Some(target)) =
                (resolved.get(&edge.source_id), resolved.get(&edge.target_id))
            else {
                dropped.insert(key);
                continue;
            };
            let source_allowed = allowed.contains(&source.entity_type)
                || focus
                    .as_ref()
                    .is_some_and(|value| value.entity_id == source.id);
            let target_allowed = allowed.contains(&target.entity_type)
                || focus
                    .as_ref()
                    .is_some_and(|value| value.entity_id == target.id);
            if !source_allowed || !target_allowed {
                continue;
            }
            for entity in [source, target] {
                if !visited.contains_key(&entity.id) {
                    if visited.len() > MAX_SYSTEM_MAP_NODES {
                        traversal_truncated = true;
                        break 'edges;
                    }
                    visited.insert(entity.id.clone(), (entity.clone(), hop + 1));
                    next.insert(entity.id.clone());
                }
            }
            relationships.entry(key).or_insert(edge);
            if relationships.len() > MAX_SYSTEM_MAP_EDGES {
                traversal_truncated = true;
                break 'edges;
            }
        }
        if traversal_truncated
            && (visited.len() > MAX_SYSTEM_MAP_NODES
                || relationships.len() > MAX_SYSTEM_MAP_EDGES
                || page.has_more)
        {
            break;
        }
        frontier = next.into_iter().collect();
    }

    let mut ranked = visited.into_values().collect::<Vec<_>>();
    ranked.sort_by(|(left, left_hop), (right, right_hop)| {
        (
            *left_hop,
            left.entity_type.as_str(),
            &left.display_name,
            &left.id,
        )
            .cmp(&(
                *right_hop,
                right.entity_type.as_str(),
                &right.display_name,
                &right.id,
            ))
    });
    ranked.truncate(MAX_SYSTEM_MAP_NODES);
    let selected_ids = ranked
        .iter()
        .map(|(entity, _)| entity.id.clone())
        .collect::<Vec<_>>();
    let selected = selected_ids.iter().cloned().collect::<BTreeSet<_>>();
    let metrics = entity_repository::system_map_metrics(conn, &selected_ids, recent_since)?;
    let nodes = ranked
        .into_iter()
        .map(|(entity, hop)| {
            let metric = metrics.get(&entity.id).copied().unwrap_or(
                entity_repository::SystemMapNodeMetrics {
                    degree: 0,
                    last_changed_at: None,
                    recent_event_count: 0,
                },
            );
            SystemMapNode {
                entity_id: entity.id,
                entity_type: entity.entity_type,
                display_name: entity.display_name,
                hop,
                degree: metric.degree,
                last_changed_at: metric.last_changed_at,
                recent_event_count: metric.recent_event_count,
            }
        })
        .collect();

    let eligible_edges = relationships
        .into_values()
        .filter(|edge| selected.contains(&edge.source_id) && selected.contains(&edge.target_id))
        .collect::<Vec<_>>();
    let edge_count = eligible_edges.len();
    let edges = eligible_edges
        .into_iter()
        .take(MAX_SYSTEM_MAP_EDGES)
        .map(|edge| SystemMapEdge {
            source: edge.source_id,
            target: edge.target_id,
            kind: edge.kind,
            recorded_at: edge.recorded_at,
            evidence_event_id: edge.event_id,
        })
        .collect();
    let dropped_edges = u32::try_from(dropped.len()).unwrap_or(u32::MAX);
    let truncated = traversal_truncated
        || usize::try_from(total_nodes_available).unwrap_or(usize::MAX) > MAX_SYSTEM_MAP_NODES
        || edge_count > MAX_SYSTEM_MAP_EDGES
        || dropped_edges > 0;
    let focused_recent_events = focused_recent_events(conn, focus.as_ref(), recent_since)?;

    Ok(SystemMapData {
        nodes,
        edges,
        focus,
        truncated,
        total_nodes_available,
        dropped_edges,
        kind_availability: availability,
        focused_recent_events,
    })
}

fn focused_recent_events(
    conn: &Connection,
    focus: Option<&EntityRef>,
    recent_since: i64,
) -> AppResult<Vec<SystemMapRecentEvent>> {
    let Some(focus) = focus else {
        return Ok(Vec::new());
    };
    let query = EventQuery {
        from: recent_since,
        to: i64::MAX,
        entity_id: Some(focus.entity_id.clone()),
        limit: 5,
        ..EventQuery::default()
    };
    Ok(event_repository::page(conn, &query)?
        .items
        .into_iter()
        .map(|event| SystemMapRecentEvent {
            event_id: event.id,
            timestamp_ms: event.timestamp,
            summary: event.user_friendly_summary,
            importance: event.importance,
        })
        .collect())
}

fn normalized_kinds(kinds: &[EntityType]) -> BTreeSet<EntityType> {
    if kinds.is_empty() {
        EntityType::ALL.iter().copied().collect()
    } else {
        kinds.iter().copied().collect()
    }
}

fn load_roots(
    conn: &Connection,
    focus: Option<&EntityRef>,
    allowed: &BTreeSet<EntityType>,
) -> AppResult<Vec<Entity>> {
    if let Some(focus) = focus {
        return Ok(entity_repository::by_id(conn, &focus.entity_id)?
            .filter(|entity| entity.entity_type == focus.entity_type)
            .into_iter()
            .collect());
    }
    let kinds = allowed.iter().copied().collect::<Vec<_>>();
    entity_repository::highest_degree_entities(conn, &kinds, ROOT_LIMIT)
}

fn kind_availability(conn: &Connection) -> AppResult<Vec<EntityKindAvailability>> {
    let counts = entity_repository::counts_by_entity_type(conn)?
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    Ok(EntityType::ALL
        .iter()
        .copied()
        .map(|kind| {
            let count = counts.get(&kind).copied().unwrap_or(0);
            EntityKindAvailability {
                kind,
                count,
                available: count > 0,
                reason: (count == 0).then(|| unavailable_reason(kind).to_owned()),
            }
        })
        .collect())
}

fn unavailable_reason(kind: EntityType) -> &'static str {
    match kind {
        EntityType::Container => "No Docker containers recorded — the Docker collector may be off",
        EntityType::Adapter => "No network adapters recorded",
        EntityType::File => "No config files recorded — add a watched folder",
        EntityType::Application => "No applications recorded",
        EntityType::Process => "No processes recorded",
        EntityType::Port => "No listening ports recorded",
        EntityType::Service => "No services recorded",
        EntityType::Driver => "No drivers recorded",
        EntityType::Image => "No Docker images recorded — the Docker collector may be off",
        EntityType::DockerVolume => "No Docker volumes recorded — the Docker collector may be off",
        EntityType::DockerNetwork => {
            "No Docker networks recorded — the Docker collector may be off"
        }
        EntityType::Project => "No projects recorded",
        EntityType::Update => "No Windows updates recorded",
        EntityType::Task => "No scheduled tasks recorded",
        EntityType::System => "No system entity recorded",
    }
}
