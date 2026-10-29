#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeSet;

use what_changed_lib::analysis::change_map::{
    build, build_window, ChangeMapEdgeKind, ChangeMapLane, MAX_CHANGE_MAP_EDGES,
    MAX_CHANGE_MAP_NODES,
};
use what_changed_lib::domain::{
    Category, ChainStep, Confidence, ConfidenceClass, EntityType, EventType, Importance,
    Investigation, InvestigationWindow, NormalizedEvent, ResolvedInvestigationWindow,
    ScoreBreakdown, Severity, Suspect, SystemEvent,
};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{event_repository, Db};
use what_changed_lib::testing::temp_dir;

fn investigation(suspects: Vec<Suspect>, dropped_events: usize) -> Investigation {
    Investigation {
        id: 7,
        project_id: None,
        problem: "my API returns 502".to_owned(),
        window: InvestigationWindow::Custom {
            from_ms: 1_000,
            to_ms: 10_000,
        },
        resolved_window: ResolvedInvestigationWindow {
            from_ms: 1_000,
            to_ms: 10_000,
        },
        suspects,
        no_cause_reason: None,
        no_cause_message: None,
        total_events_in_window: 0,
        analyzed_events: 0,
        dropped_events,
        created_at: 10_001,
    }
}

fn ridge_suspect() -> Suspect {
    Suspect {
        event_id: 1,
        title: ".env PORT changed".to_owned(),
        timestamp_ms: 2_000,
        score: 9.0,
        breakdown: ScoreBreakdown::default(),
        confidence: ConfidenceClass::Confirmed,
        confidence_text: "Strong match".to_owned(),
        relative_time: "8 minutes before".to_owned(),
        why_ranked: vec!["recorded chain".to_owned()],
        chain: vec![
            ChainStep {
                event_ids: vec![1],
                timestamp_ms: 2_000,
                label: "Configuration changed: .env PORT 3000 to 4000".to_owned(),
            },
            ChainStep {
                event_ids: vec![2, 3],
                timestamp_ms: 3_000,
                label: "Service restarted: API service".to_owned(),
            },
            ChainStep {
                event_ids: vec![4, 5],
                timestamp_ms: 4_000,
                label: "Port transition: Port 4000".to_owned(),
            },
        ],
        what_to_try: "Check the configured port.".to_owned(),
    }
}

fn event(id: i64, timestamp: i64, category: Category, entity_type: EntityType) -> SystemEvent {
    SystemEvent {
        id,
        event: NormalizedEvent {
            timestamp,
            category,
            event_type: EventType::Modified,
            entity_type,
            entity_id: format!("entity:{id}"),
            entity_display_name: format!("Entity {id}"),
            severity: Severity::Info,
            importance: if id % 11 == 0 {
                Importance::Critical
            } else {
                Importance::Informational
            },
            source_collector: "change-map-test".to_owned(),
            before: None,
            after: None,
            field: None,
            metadata: None,
            related_entity_ids: Vec::new(),
            user_friendly_summary: format!("Event {id} changed"),
            technical_summary: format!("event {id}"),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: false,
        },
    }
}

#[test]
fn section_74_snapshot_is_a_three_node_structural_ridge_after_source_expiry() {
    let data = build(&investigation(vec![ridge_suspect()], 0), &[]);

    assert_eq!(data.nodes.len(), 3);
    assert!(data.nodes.iter().all(|node| node.snapshot_only));
    assert_eq!(
        data.nodes
            .iter()
            .map(|node| node.chain_depth)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    let structural = data
        .edges
        .iter()
        .filter(|edge| edge.kind == ChangeMapEdgeKind::Structural)
        .collect::<Vec<_>>();
    assert_eq!(structural.len(), 2);
    assert_eq!(data.symptom_time_ms, Some(10_000));
    assert_eq!(data.lanes[0].id, ChangeMapLane::Process);
    assert_eq!(data.lanes[9].id, ChangeMapLane::System);
}

#[test]
fn caps_are_deterministic_disclosed_and_keep_causal_nodes() {
    let events = (1..=3_250)
        .map(|id| {
            event(
                id,
                1_000 + id,
                Category::Application,
                EntityType::Application,
            )
        })
        .collect::<Vec<_>>();
    let investigation = investigation(vec![ridge_suspect()], 250);

    let first = build(&investigation, &events);
    let second = build(&investigation, &events);

    assert_eq!(first, second);
    assert_eq!(first.nodes.len(), MAX_CHANGE_MAP_NODES);
    assert!(first.edges.len() <= MAX_CHANGE_MAP_EDGES);
    assert!(first.truncated);
    assert_eq!(first.total_nodes, 3_250);
    assert_eq!(first.omitted_nodes, 250);
    for causal in ["event:1", "event:2", "event:4"] {
        assert!(first.nodes.iter().any(|node| node.id == causal));
    }
}

#[test]
fn every_edge_endpoint_is_present_after_node_and_edge_caps() {
    let events = (1..=3_200)
        .map(|id| {
            let (category, entity_type) = if id % 17 == 0 {
                (Category::Network, EntityType::Port)
            } else {
                (Category::System, EntityType::System)
            };
            event(id, 1_000 + id, category, entity_type)
        })
        .collect::<Vec<_>>();
    let data = build(&investigation(vec![ridge_suspect()], 0), &events);
    let node_ids = data
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();

    for edge in &data.edges {
        assert!(node_ids.contains(edge.source.as_str()), "missing source");
        assert!(node_ids.contains(edge.target.as_str()), "missing target");
    }
}

#[test]
fn timeline_map_applies_the_same_category_importance_and_entity_filters_before_selection() {
    let db = Db::open_in(&temp_dir("change-map-filter-parity")).unwrap();
    let mut rows = vec![
        event(1, 2_000, Category::Application, EntityType::Application),
        event(2, 3_000, Category::Service, EntityType::Service),
        event(3, 4_000, Category::Service, EntityType::Service),
        event(4, 5_000, Category::Network, EntityType::Port),
    ];
    rows[0].event.importance = Importance::Critical;
    rows[1].event.importance = Importance::Important;
    rows[1].event.entity_id = "service:wanted".to_owned();
    rows[2].event.importance = Importance::Critical;
    rows[2].event.entity_id = "service:other".to_owned();
    rows[3].event.importance = Importance::Critical;
    rows[3].event.entity_id = "service:wanted".to_owned();
    let sanitizer = Sanitizer::new();
    let ids = db
        .insert_events(
            rows.into_iter()
                .map(|event| sanitizer.sanitize(event.event).0)
                .collect(),
        )
        .unwrap();

    let source = db
        .read(|conn| {
            event_repository::analysis_window_filtered(
                conn,
                1_000,
                10_000,
                &[Category::Service],
                Some(Importance::Important),
                Some("service:wanted"),
            )
        })
        .unwrap();
    assert_eq!(source.total, 1);
    assert_eq!(source.events[0].id, ids[1]);

    let mut suspect = ridge_suspect();
    suspect.event_id = ids[1];
    suspect.chain = vec![
        ChainStep {
            event_ids: vec![ids[1]],
            timestamp_ms: 3_000,
            label: "Service changed: wanted".to_owned(),
        },
        ChainStep {
            event_ids: vec![ids[2]],
            timestamp_ms: 4_000,
            label: "Service changed: hidden".to_owned(),
        },
    ];
    let data = build_window(
        &investigation(vec![suspect], 0),
        &source.events,
        source.total,
        source.downsampled,
    );
    assert_eq!(data.nodes.len(), 1);
    assert_eq!(data.nodes[0].event_id, ids[1]);
    assert_eq!(data.symptom_time_ms, None);
    assert!(data
        .edges
        .iter()
        .all(|edge| edge.kind != ChangeMapEdgeKind::Structural));
}
