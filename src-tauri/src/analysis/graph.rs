use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::domain::{Relationship, RelationshipKind, SystemEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StructuralKind {
    SameEntity,
    ListensOn,
    ConfigOf,
    Provides,
    Publishes,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeKind {
    Structural(StructuralKind),
    Temporal,
    CategoryAffinity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub from: i64,
    pub to: i64,
    pub kind: EdgeKind,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Graph {
    pub nodes: Vec<i64>,
    pub edges: Vec<Edge>,
    pub structural: StructuralGraph,
}

impl Graph {
    pub fn build(events: &[SystemEvent], relationships: &[Relationship]) -> Self {
        let structural = StructuralGraph::build(events, relationships);
        let mut ordered = events.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|event| (event.event.timestamp, event.id));
        let mut edges = Vec::new();
        for (index, left) in ordered.iter().enumerate() {
            for right in ordered.iter().skip(index + 1) {
                let delta = right.event.timestamp.saturating_sub(left.event.timestamp);
                if delta > 5 * 60_000 {
                    break;
                }
                let (kind, weight) = edge_between(left, right, &structural, delta);
                edges.push(Edge {
                    from: left.id,
                    to: right.id,
                    kind,
                    weight,
                });
            }
        }
        edges.sort_by_key(|edge| (edge.from, edge.to));
        Self {
            nodes: ordered.iter().map(|event| event.id).collect(),
            edges,
            structural,
        }
    }
}

/// Incident graph neighbours for one event without materializing the complete
/// O(n²) edge set. This is the production path for the event-details drawer.
pub fn neighbour_ids(
    focus: &SystemEvent,
    events: &[SystemEvent],
    relationships: &[Relationship],
) -> Vec<i64> {
    let structural = StructuralGraph::build(events, relationships);
    let mut neighbours = events
        .iter()
        .filter(|candidate| candidate.id != focus.id)
        .filter_map(|candidate| {
            let delta = candidate.event.timestamp.abs_diff(focus.event.timestamp);
            (delta <= 5 * 60_000).then(|| {
                let delta = i64::try_from(delta).unwrap_or(i64::MAX);
                let _edge = edge_between(focus, candidate, &structural, delta);
                (candidate.event.timestamp, candidate.id)
            })
        })
        .collect::<Vec<_>>();
    neighbours.sort_by(|left, right| right.cmp(left));
    neighbours.into_iter().map(|(_, id)| id).collect()
}

fn edge_between(
    left: &SystemEvent,
    right: &SystemEvent,
    structural: &StructuralGraph,
    delta: i64,
) -> (EdgeKind, f64) {
    if left.event.entity_id == right.event.entity_id {
        (EdgeKind::Structural(StructuralKind::SameEntity), 1.0)
    } else if let Some(kind) =
        structural.kind_between(&left.event.entity_id, &right.event.entity_id)
    {
        (EdgeKind::Structural(kind), 1.0)
    } else if category_affinity(left.event.category, right.event.category) {
        (EdgeKind::CategoryAffinity, 0.35)
    } else {
        (EdgeKind::Temporal, 1.0 - delta as f64 / (5.0 * 60_000.0))
    }
}

fn category_affinity(left: crate::domain::Category, right: crate::domain::Category) -> bool {
    use crate::domain::Category;
    matches!(
        (left, right),
        (Category::Configuration, Category::Service)
            | (Category::Service, Category::Configuration)
            | (Category::Update, Category::Driver)
            | (Category::Driver, Category::Update)
            | (Category::Service, Category::Network)
            | (Category::Network, Category::Service)
            | (Category::Startup, Category::Process)
            | (Category::Process, Category::Startup)
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralGraph {
    adjacency: BTreeMap<String, BTreeSet<(String, StructuralKind)>>,
}

impl StructuralGraph {
    pub fn build(events: &[SystemEvent], relationships: &[Relationship]) -> Self {
        let mut adjacency: BTreeMap<String, BTreeSet<(String, StructuralKind)>> = BTreeMap::new();
        for event in events {
            adjacency.entry(event.event.entity_id.clone()).or_default();
            for related in &event.event.related_entity_ids {
                adjacency
                    .entry(event.event.entity_id.clone())
                    .or_default()
                    .insert((related.clone(), StructuralKind::SameEntity));
                adjacency
                    .entry(related.clone())
                    .or_default()
                    .insert((event.event.entity_id.clone(), StructuralKind::SameEntity));
            }
        }
        // Deduplicate borrowed undirected facts before allocating adjacency
        // entries. Kind remains part of the fact so parallel kinds survive.
        let mut facts = relationships
            .iter()
            .map(|relationship| {
                let kind = match relationship.kind {
                    RelationshipKind::ListensOn | RelationshipKind::OpenedBy => {
                        StructuralKind::ListensOn
                    }
                    RelationshipKind::ConfigOf => StructuralKind::ConfigOf,
                    RelationshipKind::Provides => StructuralKind::Provides,
                    RelationshipKind::Publishes => StructuralKind::Publishes,
                };
                let source = relationship.source_id.as_str();
                let target = relationship.target_id.as_str();
                if source <= target {
                    (source, target, kind)
                } else {
                    (target, source, kind)
                }
            })
            .collect::<Vec<_>>();
        facts.sort_unstable();
        facts.dedup();
        for (source, target, kind) in facts {
            adjacency
                .entry(source.to_owned())
                .or_default()
                .insert((target.to_owned(), kind));
            adjacency
                .entry(target.to_owned())
                .or_default()
                .insert((source.to_owned(), kind));
        }
        Self { adjacency }
    }

    pub fn distance_to_any(
        &self,
        from: &str,
        targets: &[String],
        max_depth: usize,
    ) -> Option<usize> {
        if targets.is_empty() {
            return None;
        }
        if targets.iter().any(|target| target == from) {
            return Some(0);
        }
        if !self.adjacency.contains_key(from) {
            return None;
        }
        let target_set = targets.iter().collect::<BTreeSet<_>>();
        let mut queue = VecDeque::from([(from.to_owned(), 0_usize)]);
        let mut seen = BTreeSet::from([from.to_owned()]);
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            if let Some(neighbours) = self.adjacency.get(&node) {
                for (next, _) in neighbours {
                    if !seen.insert(next.clone()) {
                        continue;
                    }
                    if target_set.contains(next) {
                        return Some(depth + 1);
                    }
                    queue.push_back((next.clone(), depth + 1));
                }
            }
        }
        None
    }

    /// Resolve the same undirected shortest paths as `distance_to_any` once
    /// for every candidate in an investigation. Include unknown targets at
    /// distance zero, exactly as the per-candidate lookup does before checking
    /// whether its starting node exists in the graph.
    pub fn distances_to_any(
        &self,
        targets: &[String],
        max_depth: usize,
    ) -> BTreeMap<String, usize> {
        let mut distances = BTreeMap::new();
        let mut queue = VecDeque::new();
        for target in targets {
            if distances.insert(target.clone(), 0).is_none() {
                queue.push_back((target.clone(), 0_usize));
            }
        }
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            if let Some(neighbours) = self.adjacency.get(node.as_str()) {
                for (next, _) in neighbours {
                    if distances.contains_key(next.as_str()) {
                        continue;
                    }
                    let next_depth = depth + 1;
                    distances.insert(next.clone(), next_depth);
                    queue.push_back((next.clone(), next_depth));
                }
            }
        }
        distances
    }

    pub fn connected(&self, left: &str, right: &str) -> bool {
        left == right
            || self
                .adjacency
                .get(left)
                .is_some_and(|items| items.iter().any(|(id, _)| id == right))
    }

    pub fn connected_entity_ids(&self, entity_id: &str) -> BTreeSet<String> {
        let mut ids = BTreeSet::from([entity_id.to_owned()]);
        if let Some(neighbours) = self.adjacency.get(entity_id) {
            ids.extend(neighbours.iter().map(|(id, _)| id.clone()));
        }
        ids
    }

    pub fn entities_within(&self, from: &str, max_depth: usize) -> BTreeSet<String> {
        let mut found = BTreeSet::from([from.to_owned()]);
        let mut queue = VecDeque::from([(from.to_owned(), 0_usize)]);
        while let Some((node, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }
            if let Some(neighbours) = self.adjacency.get(&node) {
                for (next, _) in neighbours {
                    if found.insert(next.clone()) {
                        queue.push_back((next.clone(), depth + 1));
                    }
                }
            }
        }
        found
    }

    pub fn kind_between(&self, left: &str, right: &str) -> Option<StructuralKind> {
        if left == right {
            return Some(StructuralKind::SameEntity);
        }
        self.adjacency.get(left).and_then(|items| {
            items
                .iter()
                .find_map(|(id, kind)| (id == right).then_some(*kind))
        })
    }

    pub fn has_recorded_edge(&self, entity_id: &str) -> bool {
        self.adjacency
            .get(entity_id)
            .is_some_and(|neighbours| !neighbours.is_empty())
    }
}

#[cfg(test)]
#[path = "graph_builder_tests.rs"]
mod builder_tests;

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn relation(source: &str, target: &str) -> Relationship {
        Relationship {
            source_id: source.to_owned(),
            target_id: target.to_owned(),
            kind: RelationshipKind::Provides,
            event_id: None,
            recorded_at: 0,
        }
    }

    #[test]
    fn multi_source_distances_match_per_candidate_bfs_for_cycles_and_unknown_nodes() {
        let relationships = [
            relation("a", "b"),
            relation("b", "c"),
            relation("c", "a"), // cycle
            relation("c", "d"),
            relation("d", "e"),
            relation("x", "y"), // disconnected component
        ];
        let graph = StructuralGraph::build(&[], &relationships);
        for targets in [
            vec![],
            vec!["missing".to_owned()],
            vec!["a".to_owned(), "x".to_owned(), "missing".to_owned()],
            vec!["b".to_owned(), "c".to_owned(), "b".to_owned()],
        ] {
            for max_depth in 0..=3 {
                let distances = graph.distances_to_any(&targets, max_depth);
                for from in ["a", "b", "c", "d", "e", "x", "y", "missing", "other"] {
                    assert_eq!(
                        distances.get(from).copied(),
                        graph.distance_to_any(from, &targets, max_depth),
                        "from={from} targets={targets:?} max_depth={max_depth}"
                    );
                }
            }
        }
    }
}
