use std::time::{Duration, Instant};
use what_changed_lib::analysis::chain::{MAX_CHAIN_DEPTH, MAX_CHAIN_PATHS};

use what_changed_lib::analysis::graph::{neighbour_ids, EdgeKind, Graph, StructuralKind};
use what_changed_lib::domain::*;

fn event(id: i64, timestamp: i64, entity_id: &str) -> SystemEvent {
    SystemEvent {
        id,
        event: NormalizedEvent {
            timestamp,
            category: Category::Process,
            event_type: EventType::Started,
            entity_type: EntityType::Process,
            entity_id: entity_id.into(),
            entity_display_name: entity_id.into(),
            severity: Severity::Info,
            importance: Importance::Notable,
            source_collector: "fixture".into(),
            before: None,
            after: None,
            field: None,
            metadata: None,
            related_entity_ids: vec![],
            user_friendly_summary: "started".into(),
            technical_summary: "started".into(),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: false,
        },
    }
}

#[test]
fn graph_canonicalizes_reverse_relationships_and_exposes_fixed_caps() {
    let events = vec![
        event(1, 1_000, "process:a"),
        event(2, 1_300, "port:tcp:3000"),
    ];
    let relationships = vec![Relationship {
        source_id: "port:tcp:3000".into(),
        target_id: "process:a".into(),
        kind: RelationshipKind::OpenedBy,
        event_id: Some(2),
        recorded_at: 1_300,
    }];
    let graph = Graph::build(&events, &relationships);
    assert!(graph
        .edges
        .iter()
        .any(|edge| matches!(edge.kind, EdgeKind::Structural(StructuralKind::ListensOn))));
    assert_eq!(MAX_CHAIN_DEPTH, 4);
    assert_eq!(MAX_CHAIN_PATHS, 200);
}

#[test]
fn dense_incident_neighbour_lookup_stays_linear() {
    let events = (0_i64..5_000)
        .map(|id| event(id, 10_000 + id, &format!("process:{id}")))
        .collect::<Vec<_>>();
    let started = Instant::now();
    let neighbours = neighbour_ids(&events[2_500], &events, &[]);

    assert_eq!(neighbours.len(), 4_999);
    assert!(
        started.elapsed() <= Duration::from_millis(400),
        "incident-only lookup exceeded the 400 ms dense-window budget"
    );
}
