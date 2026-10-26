use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{importance_value, Category, EventType, ScoreBreakdown, SystemEvent};

use super::graph::StructuralGraph;

pub const W_TEMPORAL: f64 = 1.0;
pub const W_RELATIONSHIP: f64 = 3.0;
pub const W_CATEGORY: f64 = 1.5;
pub const W_IMPORTANCE: f64 = 1.0;
pub const W_RARITY: f64 = 1.25;
pub const W_FAILURE_PROXIMITY: f64 = 2.0;
pub const MIN_REPORTABLE_SCORE: f64 = 1.2;

pub struct ScoreContext<'a> {
    pub symptom_time_ms: i64,
    pub symptom_entities: &'a [String],
    pub symptom_categories: &'a [Category],
    pub graph: &'a StructuralGraph,
    pub failure_index: &'a FailureIndex,
    pub prior_occurrences: &'a BTreeMap<i64, u32>,
    pub chain_len: usize,
}

pub fn score(event: &SystemEvent, ctx: &ScoreContext<'_>) -> ScoreBreakdown {
    let distance = ctx
        .graph
        .distance_to_any(&event.event.entity_id, ctx.symptom_entities, 2);
    score_with_relationship_distance(event, ctx, distance)
}

/// The investigation runner resolves graph distance once per window. Keep
/// `score` above as the exact per-event reference for other callers and parity
/// tests; only this already-computed term differs between the two paths.
pub(crate) fn score_with_relationship_distance(
    event: &SystemEvent,
    ctx: &ScoreContext<'_>,
    distance: Option<usize>,
) -> ScoreBreakdown {
    let temporal = temporal(event, ctx.symptom_time_ms);
    let relationship = match distance {
        Some(0 | 1) => 1.0,
        Some(2) => 0.6,
        _ => 0.0,
    };
    let category = category(event.event.category, ctx.symptom_categories);
    let importance = importance_value(event.event.importance);
    let count = ctx.prior_occurrences.get(&event.id).copied().unwrap_or(0);
    let rarity = rarity(count);
    let failure_proximity = ctx.failure_index.proximity(event);
    let chain_bonus = (0.75 * ctx.chain_len.saturating_sub(1) as f64).min(2.25);
    let total = quantize(
        W_TEMPORAL * temporal
            + W_RELATIONSHIP * relationship
            + W_CATEGORY * category
            + W_IMPORTANCE * importance
            + W_RARITY * rarity
            + W_FAILURE_PROXIMITY * failure_proximity
            + chain_bonus,
    );
    ScoreBreakdown {
        temporal: quantize(temporal),
        relationship: quantize(relationship),
        category: quantize(category),
        importance: quantize(importance),
        rarity: quantize(rarity),
        failure_proximity: quantize(failure_proximity),
        chain_bonus: quantize(chain_bonus),
        total,
    }
}

pub fn rarity(prior_occurrences_90d: u32) -> f64 {
    quantize(1.0 - (f64::from(prior_occurrences_90d) / 20.0).min(1.0))
}

pub fn temporal(event: &SystemEvent, symptom_time_ms: i64) -> f64 {
    if event.event.occurred_at_is_uncertain || event.event.timestamp > symptom_time_ms {
        return 0.0;
    }
    let minutes = symptom_time_ms.saturating_sub(event.event.timestamp) as f64 / 60_000.0;
    (-minutes / 90.0).exp().clamp(0.0, 1.0)
}

fn category(category: Category, targets: &[Category]) -> f64 {
    if targets.contains(&category) {
        return 1.0;
    }
    if targets.iter().any(|target| affinity(*target, category)) {
        0.35
    } else {
        0.0
    }
}

fn affinity(left: Category, right: Category) -> bool {
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

pub struct FailureIndex {
    timestamps_by_entity: BTreeMap<String, Vec<i64>>,
}

impl FailureIndex {
    pub fn build(events: &[SystemEvent], graph: &StructuralGraph) -> Self {
        let mut failures_by_entity = BTreeMap::<String, Vec<i64>>::new();
        for event in events.iter().filter(|event| is_failure_signal(event)) {
            failures_by_entity
                .entry(event.event.entity_id.clone())
                .or_default()
                .push(event.event.timestamp);
        }
        let entities = events
            .iter()
            .map(|event| event.event.entity_id.clone())
            .collect::<BTreeSet<_>>();
        let mut timestamps_by_entity = entities
            .into_iter()
            .map(|entity| (entity, Vec::new()))
            .collect::<BTreeMap<_, _>>();
        // The structural graph is undirected: a candidate is within two hops
        // of a failure exactly when that failure is within two hops of the
        // candidate. Traverse once per distinct failure entity, not once per
        // event entity, which stays bounded when evidence edges are dense.
        for (failure_entity, failures) in failures_by_entity {
            for related in graph.entities_within(&failure_entity, 2) {
                if let Some(timestamps) = timestamps_by_entity.get_mut(&related) {
                    timestamps.extend(failures.iter().copied());
                }
            }
        }
        for timestamps in timestamps_by_entity.values_mut() {
            timestamps.sort_unstable();
            timestamps.dedup();
        }
        Self {
            timestamps_by_entity,
        }
    }

    pub fn proximity(&self, event: &SystemEvent) -> f64 {
        if event.event.occurred_at_is_uncertain {
            return 0.0;
        }
        let Some(timestamps) = self.timestamps_by_entity.get(&event.event.entity_id) else {
            return 0.0;
        };
        // Corroboration must follow the candidate. A failure never confirms
        // itself, including another row recorded at the same millisecond.
        let position = timestamps.partition_point(|timestamp| *timestamp <= event.event.timestamp);
        let Some(timestamp) = timestamps.get(position) else {
            return 0.0;
        };
        let delta = timestamp.saturating_sub(event.event.timestamp);
        if delta <= 10 * 60_000 {
            1.0
        } else if delta <= 60 * 60_000 {
            0.5
        } else {
            0.0
        }
    }
}

pub fn is_failure_signal(event: &SystemEvent) -> bool {
    (event.event.category == Category::Service && event.event.event_type == EventType::Failed)
        || (event.event.category == Category::System && event.event.event_type == EventType::Failed)
        || (event.event.entity_type == crate::domain::EntityType::Port
            && event.event.event_type == EventType::Closed)
        || (event.event.category == Category::Process
            && event.event.event_type == EventType::Stopped
            && event.event.metadata.as_ref().is_some_and(|metadata| {
                metadata
                    .get("exit_kind")
                    .and_then(serde_json::Value::as_str)
                    == Some("unexpected")
            }))
}

pub fn quantize(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::domain::{Confidence, EntityType, Importance, NormalizedEvent, Severity};

    fn event(timestamp: i64) -> SystemEvent {
        SystemEvent {
            id: 1,
            event: NormalizedEvent {
                timestamp,
                category: Category::Update,
                event_type: EventType::Updated,
                entity_type: EntityType::Update,
                entity_id: "update:x".into(),
                entity_display_name: "x".into(),
                severity: Severity::Info,
                importance: Importance::Notable,
                source_collector: "test".into(),
                before: None,
                after: None,
                field: None,
                metadata: None,
                related_entity_ids: vec![],
                user_friendly_summary: "x".into(),
                technical_summary: "x".into(),
                confidence: Confidence::High,
                sensitive: false,
                occurred_at_is_uncertain: false,
            },
        }
    }

    #[test]
    fn temporal_boundaries_are_explicit() {
        assert_eq!(temporal(&event(1_000), 1_000), 1.0);
        assert!((temporal(&event(1_000), 1_000 + 90 * 60_000) - (-1.0_f64).exp()).abs() < 1e-9);
        assert_eq!(temporal(&event(1_001), 1_000), 0.0);
    }

    #[test]
    fn importance_includes_notable_term() {
        assert_eq!(importance_value(Importance::Informational), 0.15);
        assert_eq!(importance_value(Importance::Notable), 0.35);
        assert_eq!(importance_value(Importance::Important), 0.6);
        assert_eq!(importance_value(Importance::Critical), 1.0);
    }
}
