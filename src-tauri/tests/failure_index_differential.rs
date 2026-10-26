use std::collections::{BTreeMap, BTreeSet};

use what_changed_lib::analysis::{
    graph::StructuralGraph,
    score::{is_failure_signal, FailureIndex},
};
use what_changed_lib::domain::{
    Category, Confidence, EntityType, EventType, Importance, NormalizedEvent, Relationship,
    RelationshipKind, Severity, SystemEvent,
};

const MINUTE: i64 = 60_000;

fn event(
    id: i64,
    entity_id: &str,
    timestamp: i64,
    kind: EventType,
    uncertain: bool,
) -> SystemEvent {
    SystemEvent {
        id,
        event: NormalizedEvent {
            timestamp,
            category: Category::Service,
            event_type: kind,
            entity_type: EntityType::Service,
            entity_id: entity_id.to_owned(),
            entity_display_name: entity_id.to_owned(),
            severity: Severity::Info,
            importance: Importance::Informational,
            source_collector: "failure-index-differential".to_owned(),
            before: None,
            after: None,
            field: None,
            metadata: None,
            related_entity_ids: Vec::new(),
            user_friendly_summary: entity_id.to_owned(),
            technical_summary: entity_id.to_owned(),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: uncertain,
        },
    }
}

fn relation(source_id: &str, target_id: &str) -> Relationship {
    Relationship {
        source_id: source_id.to_owned(),
        target_id: target_id.to_owned(),
        kind: RelationshipKind::Provides,
        event_id: None,
        recorded_at: 0,
    }
}

// The pre-optimization algorithm traversed from each distinct candidate
// entity, then merged failures found within two structural hops.
fn original_failure_timestamps(
    events: &[SystemEvent],
    graph: &StructuralGraph,
) -> BTreeMap<String, Vec<i64>> {
    let mut failures_by_entity = BTreeMap::<String, Vec<i64>>::new();
    for failure in events.iter().filter(|item| is_failure_signal(item)) {
        failures_by_entity
            .entry(failure.event.entity_id.clone())
            .or_default()
            .push(failure.event.timestamp);
    }
    let entities = events
        .iter()
        .map(|item| item.event.entity_id.clone())
        .collect::<BTreeSet<_>>();
    let mut timestamps_by_entity = BTreeMap::new();
    for entity in entities {
        let mut timestamps = graph
            .entities_within(&entity, 2)
            .into_iter()
            .filter_map(|related| failures_by_entity.get(&related))
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        timestamps.sort_unstable();
        timestamps.dedup();
        timestamps_by_entity.insert(entity, timestamps);
    }
    timestamps_by_entity
}

fn original_proximity(event: &SystemEvent, timestamps: &BTreeMap<String, Vec<i64>>) -> f64 {
    if event.event.occurred_at_is_uncertain {
        return 0.0;
    }
    let Some(nearby) = timestamps.get(&event.event.entity_id) else {
        return 0.0;
    };
    let position = nearby.partition_point(|timestamp| *timestamp <= event.event.timestamp);
    let Some(timestamp) = nearby.get(position) else {
        return 0.0;
    };
    match timestamp.saturating_sub(event.event.timestamp) {
        delta if delta <= 10 * MINUTE => 1.0,
        delta if delta <= 60 * MINUTE => 0.5,
        _ => 0.0,
    }
}

fn assert_equivalent(
    events: &[SystemEvent],
    relationships: &[Relationship],
    explicit_expectations: &[(i64, f64)],
) {
    let graph = StructuralGraph::build(events, relationships);
    let original = original_failure_timestamps(events, &graph);
    let optimized = FailureIndex::build(events, &graph);
    for candidate in events {
        assert_eq!(
            optimized.proximity(candidate),
            original_proximity(candidate, &original),
            "event {} on {} changed proximity",
            candidate.id,
            candidate.event.entity_id
        );
    }
    for (id, expected) in explicit_expectations {
        let candidate = events.iter().find(|candidate| candidate.id == *id);
        assert_eq!(
            candidate.map(|candidate| optimized.proximity(candidate)),
            Some(*expected),
            "event {id} violated the expected boundary or reachability"
        );
    }
}

#[test]
fn disconnected_entities_do_not_inherit_failures() {
    let events = [
        event(1, "service:a", 0, EventType::Updated, false),
        event(2, "service:b", 5 * MINUTE, EventType::Failed, false),
        event(3, "service:c", 0, EventType::Updated, false),
        event(4, "service:d", 30 * MINUTE, EventType::Failed, false),
    ];
    assert_equivalent(
        &events,
        &[relation("service:a", "service:b")],
        &[(1, 1.0), (2, 0.0), (3, 0.0), (4, 0.0)],
    );
}

#[test]
fn dense_two_hop_graph_keeps_three_hop_failure_out_of_reach() {
    let mut events = vec![event(1, "root", 0, EventType::Updated, false)];
    let mut relationships = Vec::new();
    for hub in 0..6 {
        let hub_id = format!("hub:{hub}");
        relationships.push(relation("root", &hub_id));
        for leaf in 0..6 {
            relationships.push(relation(&hub_id, &format!("leaf:{leaf}")));
        }
    }
    for leaf in 0..6 {
        events.push(event(
            i64::from(leaf + 2),
            &format!("leaf:{leaf}"),
            0,
            EventType::Updated,
            false,
        ));
    }
    events.push(event(8, "leaf:5", 30 * MINUTE, EventType::Failed, false));
    events.push(event(9, "outside", MINUTE, EventType::Failed, false));
    relationships.push(relation("leaf:0", "outside"));
    assert_equivalent(
        &events,
        &relationships,
        &[(1, 0.5), (2, 1.0), (7, 0.5), (8, 0.0), (9, 0.0)],
    );
}

#[test]
fn repeated_timestamps_never_corroborate_the_same_millisecond() {
    let events = [
        event(1, "service:a", 0, EventType::Updated, false),
        event(2, "service:a", 0, EventType::Failed, false),
        event(3, "service:b", 0, EventType::Failed, false),
        event(4, "service:b", 10 * MINUTE, EventType::Failed, false),
        event(5, "service:a", 10 * MINUTE, EventType::Updated, false),
        event(6, "service:b", 60 * MINUTE, EventType::Failed, false),
        event(7, "service:a", 60 * MINUTE, EventType::Updated, false),
        event(8, "service:same", 0, EventType::Updated, false),
        event(9, "service:same", 0, EventType::Failed, false),
    ];
    assert_equivalent(
        &events,
        &[relation("service:a", "service:b")],
        &[
            (1, 1.0),
            (2, 1.0),
            (3, 1.0),
            (4, 0.5),
            (5, 0.5),
            (6, 0.0),
            (7, 0.0),
            (8, 0.0),
            (9, 0.0),
        ],
    );
}

#[test]
fn uncertain_candidate_does_not_gain_failure_proximity() {
    let events = [
        event(1, "service:a", 0, EventType::Updated, true),
        event(2, "service:a", 0, EventType::Updated, false),
        event(3, "service:b", MINUTE, EventType::Failed, false),
    ];
    assert_equivalent(
        &events,
        &[relation("service:a", "service:b")],
        &[(1, 0.0), (2, 1.0), (3, 0.0)],
    );
}
