use super::*;

// Preserve the prior builder independently as the full-graph reference.
fn original_build(events: &[SystemEvent], relationships: &[Relationship]) -> StructuralGraph {
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
    for relationship in relationships {
        let kind = match relationship.kind {
            RelationshipKind::ListensOn | RelationshipKind::OpenedBy => StructuralKind::ListensOn,
            RelationshipKind::ConfigOf => StructuralKind::ConfigOf,
            RelationshipKind::Provides => StructuralKind::Provides,
            RelationshipKind::Publishes => StructuralKind::Publishes,
        };
        adjacency
            .entry(relationship.source_id.clone())
            .or_default()
            .insert((relationship.target_id.clone(), kind));
        adjacency
            .entry(relationship.target_id.clone())
            .or_default()
            .insert((relationship.source_id.clone(), kind));
    }
    StructuralGraph { adjacency }
}

fn event(id: usize, entity: &str, related: &[&str]) -> SystemEvent {
    let mut event = crate::testing::synthetic_event(id as u64, id as i64);
    event.entity_id = entity.to_owned();
    event.related_entity_ids = related.iter().map(|id| (*id).to_owned()).collect();
    SystemEvent {
        id: id as i64,
        event,
    }
}

fn relation(source: &str, target: &str, kind: RelationshipKind) -> Relationship {
    Relationship {
        source_id: source.to_owned(),
        target_id: target.to_owned(),
        kind,
        event_id: None,
        recorded_at: 0,
    }
}

fn fixture(
    event_count: usize,
    relation_count: usize,
    duplicates: bool,
) -> (Vec<SystemEvent>, Vec<Relationship>) {
    const KINDS: [RelationshipKind; 5] = [
        RelationshipKind::ListensOn,
        RelationshipKind::OpenedBy,
        RelationshipKind::ConfigOf,
        RelationshipKind::Provides,
        RelationshipKind::Publishes,
    ];
    let events = (0..event_count)
        .map(|index| {
            let entity = format!("node:{:04}", index % 3000);
            let related = format!("node:{:04}", (index + 1) % 3000);
            if index % 7 == 0 {
                event(index, &entity, &[&related, &entity])
            } else {
                event(index, &entity, &[])
            }
        })
        .collect();
    let relationships = (0..relation_count)
        .map(|index| {
            let fact = if duplicates { index % 1500 } else { index };
            let source = fact % 3000;
            let target = (source + fact / 3000 + 1) % 3000;
            let kind = KINDS[(fact + if duplicates { index / 1500 } else { 0 }) % KINDS.len()];
            let source = format!("node:{source:04}");
            let target = format!("node:{target:04}");
            if (index / 1500 + fact) % 2 == 0 {
                relation(&target, &source, kind)
            } else {
                relation(&source, &target, kind)
            }
        })
        .collect();
    (events, relationships)
}

#[test]
fn full_graph_matches_original_for_parallel_kinds_self_loops_and_event_overlap() {
    let events = [
        event(1, "event:isolated", &[]),
        event(
            2,
            "event:overlap",
            &["fact:peer", "event:overlap", "fact:peer"],
        ),
    ];
    let relationships = [
        relation("event:overlap", "fact:peer", RelationshipKind::Provides),
        relation("fact:peer", "event:overlap", RelationshipKind::Provides),
        relation("event:overlap", "fact:peer", RelationshipKind::ConfigOf),
        relation("self:only", "self:only", RelationshipKind::Publishes),
        relation("a", "b", RelationshipKind::OpenedBy),
        relation("b", "a", RelationshipKind::ListensOn),
        relation("a", "b", RelationshipKind::Provides),
    ];
    let graph = StructuralGraph::build(&events, &relationships);
    assert_eq!(graph, original_build(&events, &relationships));
    assert_eq!(graph.adjacency["event:isolated"].len(), 0);
    assert_eq!(graph.adjacency["self:only"].len(), 1);
    assert_eq!(graph.adjacency["a"].len(), 2);
    assert_eq!(
        graph.kind_between("a", "b"),
        Some(StructuralKind::ListensOn)
    );
    assert_eq!(
        graph.kind_between("event:overlap", "fact:peer"),
        Some(StructuralKind::SameEntity)
    );
    assert!(graph.has_recorded_edge("self:only"));
    assert!(graph.connected("unknown", "unknown"));
    assert_eq!(
        graph.kind_between("unknown", "unknown"),
        Some(StructuralKind::SameEntity)
    );
    assert_eq!(
        graph.distance_to_any("unknown", &["unknown".to_owned()], 0),
        Some(0)
    );
    assert_eq!(graph.distance_to_any("unknown", &["a".to_owned()], 3), None);
}

#[test]
fn generated_duplicate_and_mostly_unique_graphs_match_original_in_both_input_orders() {
    for duplicates in [false, true] {
        for relations in [0, 1, 13, 150, 4096] {
            let (mut events, mut relationships) = fixture(96, relations, duplicates);
            let expected = original_build(&events, &relationships);
            assert_eq!(StructuralGraph::build(&events, &relationships), expected);
            events.reverse();
            relationships.reverse();
            assert_eq!(StructuralGraph::build(&events, &relationships), expected);
            assert_eq!(original_build(&events, &relationships), expected);
        }
    }
}

#[path = "graph_builder_benchmark.rs"]
mod benchmark;
