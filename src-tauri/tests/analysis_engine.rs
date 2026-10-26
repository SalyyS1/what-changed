#![allow(clippy::expect_used, clippy::too_many_arguments)]

use std::collections::BTreeMap;

use serde_json::json;
use what_changed_lib::analysis::analyze;
use what_changed_lib::domain::*;

fn event(
    id: i64,
    at: i64,
    category: Category,
    kind: EventType,
    entity_type: EntityType,
    entity_id: &str,
    name: &str,
    importance: Importance,
) -> SystemEvent {
    SystemEvent {
        id,
        event: NormalizedEvent {
            timestamp: at,
            category,
            event_type: kind,
            entity_type,
            entity_id: entity_id.into(),
            entity_display_name: name.into(),
            severity: Severity::Warning,
            importance,
            source_collector: "fixture".into(),
            before: None,
            after: None,
            field: None,
            metadata: None,
            related_entity_ids: vec![],
            user_friendly_summary: format!("{name} changed"),
            technical_summary: format!("{entity_id} changed"),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: false,
        },
    }
}

fn relationship(source: &str, target: &str, kind: RelationshipKind) -> Relationship {
    Relationship {
        source_id: source.into(),
        target_id: target.into(),
        kind,
        event_id: None,
        recorded_at: 0,
    }
}

fn input(events: Vec<SystemEvent>, relationships: Vec<Relationship>) -> AnalysisInput {
    AnalysisInput {
        problem: "my app can't connect to the api".into(),
        window: InvestigationWindow::Today,
        resolved_window: ResolvedInvestigationWindow {
            from_ms: 0,
            to_ms: 10_000_000,
        },
        symptom_time_ms: 10_000_000,
        created_at: 10_000_001,
        events,
        boundary_events: vec![],
        entities: vec![Entity {
            id: "service:api".into(),
            entity_type: EntityType::Service,
            display_name: "API service".into(),
            first_seen: 0,
            last_seen: 10_000_000,
            metadata: None,
        }],
        relationships,
        prior_occurrences_90d: BTreeMap::new(),
        total_events_in_window: 0,
        dropped_events: 0,
        coverage: Coverage::default(),
    }
}

#[test]
fn worked_port_chain_beats_recent_browser_update_and_collapses_to_three_stages() {
    let mut config = event(
        1,
        7_600_000,
        Category::Configuration,
        EventType::ConfigurationChanged,
        EntityType::File,
        "file:api-env",
        ".env PORT 3000 to Port 4000",
        Importance::Important,
    );
    config.event.field = Some("PORT".into());
    config.event.after = Some(json!("[REDACTED]"));
    let events = vec![
        config,
        event(
            2,
            7_660_000,
            Category::Service,
            EventType::Stopped,
            EntityType::Service,
            "service:api",
            "API service",
            Importance::Important,
        ),
        event(
            3,
            7_660_300,
            Category::Service,
            EventType::Started,
            EntityType::Service,
            "service:api",
            "API service",
            Importance::Notable,
        ),
        event(
            4,
            7_720_000,
            Category::Network,
            EventType::Closed,
            EntityType::Port,
            "port:tcp:3000",
            "Port 3000",
            Importance::Important,
        ),
        event(
            5,
            7_720_300,
            Category::Network,
            EventType::Opened,
            EntityType::Port,
            "port:tcp:4000",
            "Port 4000",
            Importance::Important,
        ),
        event(
            6,
            9_880_000,
            Category::Application,
            EventType::Updated,
            EntityType::Application,
            "app:chrome",
            "Chrome",
            Importance::Informational,
        ),
    ];
    let relationships = vec![
        relationship("file:api-env", "service:api", RelationshipKind::ConfigOf),
        relationship("service:api", "port:tcp:3000", RelationshipKind::ListensOn),
        relationship("service:api", "port:tcp:4000", RelationshipKind::ListensOn),
    ];
    let mut request = input(events, relationships);
    request.prior_occurrences_90d.insert(6, 18);
    let first = analyze(request.clone());
    let second = analyze(request);
    assert_eq!(first, second, "analysis must be byte-stable in its values");
    let top = &first.suspects[0];
    assert_eq!(top.event_id, 1);
    assert_eq!(top.confidence, ConfidenceClass::Confirmed);
    assert_eq!(top.chain.len(), 3);
    assert!(top
        .chain
        .iter()
        .any(|step| step.label.contains("Port 4000")));
    assert!(top
        .why_ranked
        .iter()
        .any(|line| line == "These events occurred 300ms apart"));
}

#[test]
fn after_symptom_is_excluded_and_uncertain_time_is_capped() {
    let after = event(
        1,
        10_000_001,
        Category::Network,
        EventType::ConfigurationChanged,
        EntityType::Adapter,
        "adapter:wifi",
        "Wi-Fi",
        Importance::Critical,
    );
    let mut uncertain = after.clone();
    uncertain.id = 2;
    uncertain.event.occurred_at_is_uncertain = true;
    let result = analyze(input(vec![after, uncertain], vec![]));
    assert!(!result.suspects.iter().any(|item| item.event_id == 1));
    let retained = result
        .suspects
        .iter()
        .find(|item| item.event_id == 2)
        .expect("uncertain event retained");
    assert_ne!(retained.confidence, ConfidenceClass::Confirmed);
    assert_eq!(retained.breakdown.temporal, 0.0);
}

#[test]
fn routine_only_is_an_explicit_no_answer() {
    let routine = event(
        1,
        9_900_000,
        Category::Application,
        EventType::Updated,
        EntityType::Application,
        "app:x",
        "Routine app",
        Importance::Informational,
    );
    let result = analyze(input(vec![routine], vec![]));
    assert_eq!(
        result.no_cause_reason,
        Some(NoCauseReason::OnlyRoutineChanges)
    );
    assert_eq!(
        result.no_cause_message.as_deref(),
        Some("Only routine changes here — updates and background tasks.")
    );
}

#[test]
fn boundary_head_can_add_context_but_caps_the_suspect_at_likely() {
    let boundary = event(
        20,
        7_900_000,
        Category::Configuration,
        EventType::ConfigurationChanged,
        EntityType::File,
        "file:api-env",
        ".env",
        Importance::Important,
    );
    let events = vec![
        event(
            21,
            8_100_000,
            Category::Service,
            EventType::Restarted,
            EntityType::Service,
            "service:api",
            "API service",
            Importance::Important,
        ),
        event(
            22,
            8_200_000,
            Category::Network,
            EventType::Closed,
            EntityType::Port,
            "port:tcp:3000",
            "Port 3000",
            Importance::Important,
        ),
    ];
    let relationships = vec![
        relationship("file:api-env", "service:api", RelationshipKind::ConfigOf),
        relationship("service:api", "port:tcp:3000", RelationshipKind::ListensOn),
    ];
    let mut request = input(events, relationships);
    request.boundary_events = vec![boundary];
    let result = analyze(request);
    assert_eq!(result.suspects[0].event_id, 21);
    assert_ne!(result.suspects[0].confidence, ConfidenceClass::Confirmed);
    assert_eq!(result.suspects[0].confidence, ConfidenceClass::Likely);
}

#[test]
fn downsampling_keeps_a_structurally_relevant_planted_change() {
    let mut events = (100..5_200)
        .map(|id| {
            event(
                id,
                8_000_000 + id,
                Category::Application,
                EventType::Updated,
                EntityType::Application,
                &format!("app:{id}"),
                "Routine app",
                Importance::Informational,
            )
        })
        .collect::<Vec<_>>();
    events.push(event(
        1,
        7_000_000,
        Category::Configuration,
        EventType::ConfigurationChanged,
        EntityType::File,
        "file:api-env",
        "API config",
        Importance::Important,
    ));
    let relationships = vec![relationship(
        "file:api-env",
        "service:api",
        RelationshipKind::ConfigOf,
    )];
    let result = analyze(input(events, relationships));
    assert_eq!(result.analyzed_events, 5_000);
    assert_eq!(result.dropped_events, 101);
    assert_eq!(result.suspects[0].event_id, 1);
}

#[test]
fn a_lone_failure_cannot_corroborate_or_confirm_itself() {
    let failure = event(
        90,
        9_900_000,
        Category::Service,
        EventType::Failed,
        EntityType::Service,
        "service:api",
        "API service",
        Importance::Important,
    );
    let result = analyze(input(vec![failure], vec![]));
    let suspect = result.suspects.first().expect("failure remains reportable");
    assert_eq!(suspect.breakdown.failure_proximity, 0.0);
    assert_ne!(suspect.confidence, ConfidenceClass::Confirmed);
}

#[test]
fn stale_structural_events_do_not_form_or_absorb_a_cascade() {
    let events = vec![
        event(
            91,
            1_000,
            Category::Configuration,
            EventType::ConfigurationChanged,
            EntityType::File,
            "file:api-env",
            "API config",
            Importance::Important,
        ),
        event(
            92,
            24 * 60 * 60_000,
            Category::Service,
            EventType::Restarted,
            EntityType::Service,
            "service:api",
            "API service",
            Importance::Important,
        ),
        event(
            93,
            24 * 60 * 60_000 + 60_000,
            Category::Network,
            EventType::Closed,
            EntityType::Port,
            "port:tcp:3000",
            "Port 3000",
            Importance::Important,
        ),
    ];
    let relationships = vec![
        relationship("file:api-env", "service:api", RelationshipKind::ConfigOf),
        relationship("service:api", "port:tcp:3000", RelationshipKind::ListensOn),
    ];
    let mut request = input(events, relationships);
    request.symptom_time_ms = 24 * 60 * 60_000 + 120_000;
    request.resolved_window.to_ms = request.symptom_time_ms;
    let result = analyze(request);
    let config = result
        .suspects
        .iter()
        .find(|suspect| suspect.event_id == 91)
        .expect("old config remains independent");
    assert!(config.chain.is_empty());
    assert!(result.suspects.iter().any(|suspect| suspect.event_id == 92));
}

#[test]
fn unsupported_after_update_hint_defaults_to_today() {
    assert_eq!(
        what_changed_lib::analysis::window::infer("it failed after the update"),
        InvestigationWindow::Today
    );
}
