use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::domain::{Category, ChainStep, EventType, SystemEvent};

use super::graph::StructuralGraph;

pub const MAX_CHAIN_DEPTH: usize = 4;
pub const MAX_CHAIN_PATHS: usize = 200;
pub const MAX_CHAIN_STEP_GAP_MS: i64 = 5 * 60_000;

#[derive(Debug, Clone)]
pub struct DetectedChain {
    pub event_ids: Vec<i64>,
    pub semantic_len: usize,
    pub boundary_head: bool,
}

pub struct EventIndex {
    by_entity: BTreeMap<String, Vec<usize>>,
    by_id: BTreeMap<i64, usize>,
    path_cache: RefCell<BTreeMap<(i64, usize), Vec<i64>>>,
}

impl EventIndex {
    pub fn new(events: &[SystemEvent]) -> Self {
        let mut by_entity = BTreeMap::<String, Vec<usize>>::new();
        for (index, event) in events.iter().enumerate() {
            by_entity
                .entry(event.event.entity_id.clone())
                .or_default()
                .push(index);
        }
        let by_id = events
            .iter()
            .enumerate()
            .map(|(index, event)| (event.id, index))
            .collect();
        Self {
            by_entity,
            by_id,
            path_cache: RefCell::new(BTreeMap::new()),
        }
    }

    fn first_after<'a>(
        &self,
        entity_id: &str,
        cursor: (i64, i64),
        events: &'a [SystemEvent],
    ) -> Option<&'a SystemEvent> {
        let indices = self.by_entity.get(entity_id)?;
        let position = indices.partition_point(|index| {
            let event = &events[*index];
            (event.event.timestamp, event.id) <= cursor
        });
        indices.get(position).map(|index| &events[*index])
    }

    fn successors<'a>(
        &self,
        current: &SystemEvent,
        graph: &StructuralGraph,
        events: &'a [SystemEvent],
    ) -> Vec<&'a SystemEvent> {
        let cursor = (current.event.timestamp, current.id);
        let latest = current
            .event
            .timestamp
            .saturating_add(MAX_CHAIN_STEP_GAP_MS);
        let mut successors = graph
            .connected_entity_ids(&current.event.entity_id)
            .into_iter()
            .filter_map(|entity| self.first_after(&entity, cursor, events))
            .filter(|event| event.event.timestamp <= latest)
            .collect::<Vec<_>>();
        successors.sort_by_key(|event| (event.event.timestamp, event.id));
        successors.dedup_by_key(|event| event.id);
        successors
    }

    fn event<'a>(&self, id: i64, events: &'a [SystemEvent]) -> Option<&'a SystemEvent> {
        self.by_id.get(&id).map(|index| &events[*index])
    }

    fn best_path_ids(
        &self,
        current: &SystemEvent,
        remaining_nodes: usize,
        graph: &StructuralGraph,
        events: &[SystemEvent],
    ) -> Vec<i64> {
        let key = (current.id, remaining_nodes);
        if let Some(cached) = self.path_cache.borrow().get(&key) {
            return cached.clone();
        }
        let mut best = vec![current.id];
        if remaining_nodes > 1 {
            for next in self
                .successors(current, graph, events)
                .into_iter()
                .take(MAX_CHAIN_PATHS)
            {
                let mut candidate = vec![current.id];
                candidate.extend(self.best_path_ids(next, remaining_nodes - 1, graph, events));
                if self.path_ids_are_better(&candidate, &best, events) {
                    best = candidate;
                }
            }
        }
        self.path_cache.borrow_mut().insert(key, best.clone());
        best
    }

    fn path_ids_are_better(
        &self,
        candidate: &[i64],
        current: &[i64],
        events: &[SystemEvent],
    ) -> bool {
        let candidate_events = candidate
            .iter()
            .filter_map(|id| self.event(*id, events))
            .collect::<Vec<_>>();
        let current_events = current
            .iter()
            .filter_map(|id| self.event(*id, events))
            .collect::<Vec<_>>();
        semantic_step_count(&candidate_events)
            .cmp(&semantic_step_count(&current_events))
            .then_with(|| candidate.len().cmp(&current.len()))
            .is_gt()
    }
}

pub fn detect_from(
    head: &SystemEvent,
    window_events: &[SystemEvent],
    boundary_events: &[SystemEvent],
    graph: &StructuralGraph,
    index: &EventIndex,
) -> DetectedChain {
    let boundary = boundary_events
        .iter()
        .filter(|event| event.event.timestamp <= head.event.timestamp)
        .filter(|event| {
            head.event.timestamp.saturating_sub(event.event.timestamp) <= MAX_CHAIN_STEP_GAP_MS
        })
        .filter(|event| graph.connected(&event.event.entity_id, &head.event.entity_id))
        .max_by_key(|event| (event.event.timestamp, event.id));
    let max_window_nodes = MAX_CHAIN_DEPTH
        .saturating_add(1)
        .saturating_sub(usize::from(boundary.is_some()));
    let best = index.best_path_ids(head, max_window_nodes, graph, window_events);
    let mut chosen = Vec::with_capacity(best.len().saturating_add(1));
    if let Some(boundary) = boundary {
        chosen.push(boundary);
    }
    chosen.extend(
        best.into_iter()
            .filter_map(|id| index.event(id, window_events)),
    );
    let ids = chosen.iter().map(|event| event.id).collect::<Vec<_>>();
    let semantic_len = semantic_step_count(&chosen);
    DetectedChain {
        event_ids: ids,
        semantic_len,
        boundary_head: boundary.is_some(),
    }
}

pub fn render_steps(event_ids: &[i64], events: &[SystemEvent]) -> Vec<ChainStep> {
    let chosen = event_ids
        .iter()
        .filter_map(|id| events.iter().find(|event| event.id == *id))
        .collect::<Vec<_>>();
    semantic_steps(&chosen)
}

fn semantic_step_count(events: &[&SystemEvent]) -> usize {
    let mut count = 0;
    let mut previous = None;
    for event in events {
        let current = stage(event);
        if previous != Some(current) {
            count += 1;
            previous = Some(current);
        }
    }
    count.min(MAX_CHAIN_DEPTH)
}

fn semantic_steps(events: &[&SystemEvent]) -> Vec<ChainStep> {
    let mut steps = Vec::<ChainStep>::new();
    for event in events {
        let stage = stage(event);
        if let Some(last) = steps
            .last_mut()
            .filter(|last| stage_group(&last.label) == stage)
        {
            last.event_ids.push(event.id);
            last.timestamp_ms = last.timestamp_ms.min(event.event.timestamp);
            last.label = merged_label(stage, events, &last.event_ids);
        } else {
            steps.push(ChainStep {
                event_ids: vec![event.id],
                timestamp_ms: event.event.timestamp,
                label: initial_label(stage, event),
            });
        }
    }
    steps.truncate(MAX_CHAIN_DEPTH);
    steps
}

fn stage(event: &SystemEvent) -> &'static str {
    match (event.event.category, event.event.entity_type) {
        (Category::Configuration, _) | (Category::Filesystem, _) => "config",
        (Category::Service, _)
            if matches!(
                event.event.event_type,
                EventType::Stopped | EventType::Started | EventType::Restarted
            ) =>
        {
            "service"
        }
        (_, crate::domain::EntityType::Port) => "port",
        _ => "change",
    }
}

fn stage_group(label: &str) -> &'static str {
    if label.starts_with("Configuration") {
        "config"
    } else if label.starts_with("Service") {
        "service"
    } else if label.starts_with("Port") {
        "port"
    } else {
        "change"
    }
}

fn initial_label(stage: &str, event: &SystemEvent) -> String {
    match stage {
        "config" => format!(
            "Configuration changed: {}",
            event.event.user_friendly_summary
        ),
        "service" => format!("Service changed: {}", event.event.entity_display_name),
        "port" => format!("Port changed: {}", event.event.entity_display_name),
        _ => event.event.user_friendly_summary.clone(),
    }
}

fn merged_label(stage: &str, events: &[&SystemEvent], ids: &[i64]) -> String {
    let members = events
        .iter()
        .filter(|event| ids.contains(&event.id))
        .collect::<Vec<_>>();
    match stage {
        "service" => format!(
            "Service restarted: {}",
            members
                .last()
                .map_or("service", |event| event.event.entity_display_name.as_str())
        ),
        "port" => {
            let opened = members
                .iter()
                .rev()
                .find(|event| event.event.event_type == EventType::Opened)
                .copied()
                .or_else(|| members.last().copied());
            format!(
                "Port transition: {}",
                opened.map_or("port", |event| event.event.entity_display_name.as_str())
            )
        }
        _ => members.last().map_or_else(
            || "Related change".to_owned(),
            |event| event.event.user_friendly_summary.clone(),
        ),
    }
}
