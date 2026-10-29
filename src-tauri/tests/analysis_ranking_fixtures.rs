#![allow(clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use what_changed_lib::analysis::analyze;
use what_changed_lib::domain::*;

#[derive(Deserialize)]
struct Fixture {
    name: String,
    problem: String,
    symptom_time_ms: i64,
    expected_top_three: Vec<i64>,
    expected_confidence: Option<String>,
    expected_no_cause: Option<String>,
    #[serde(default)]
    excluded_ids: Vec<i64>,
    #[serde(default)]
    generated_routine_events: usize,
    events: Vec<FixtureEvent>,
    #[serde(default)]
    boundary_events: Vec<FixtureEvent>,
    #[serde(default)]
    relationships: Vec<String>,
}

#[derive(Deserialize)]
struct FixtureEvent {
    id: i64,
    kind: String,
    entity_id: Option<String>,
    importance: Option<String>,
    at: i64,
}

#[test]
fn all_eight_declared_scenarios_execute_through_the_real_engine() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/analysis/scenarios");
    let mut paths = fs::read_dir(root)
        .expect("scenario directory")
        .map(|entry| entry.expect("scenario entry").path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths.len(), 8);

    for path in paths {
        let fixture: Fixture =
            serde_json::from_str(&fs::read_to_string(&path).expect("fixture text"))
                .expect("fixture json");
        let result = analyze(build_input(&fixture));
        let actual = result
            .suspects
            .iter()
            .take(fixture.expected_top_three.len())
            .map(|suspect| suspect.event_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual, fixture.expected_top_three,
            "{} ranking",
            fixture.name
        );
        for excluded in &fixture.excluded_ids {
            assert!(
                !result
                    .suspects
                    .iter()
                    .any(|suspect| suspect.event_id == *excluded),
                "{} must exclude event {excluded}",
                fixture.name
            );
        }
        if let Some(expected) = fixture.expected_confidence.as_deref() {
            assert_eq!(
                result.suspects.first().map(|suspect| suspect.confidence),
                Some(parse_confidence(expected)),
                "{} confidence",
                fixture.name
            );
        }
        if let Some(expected) = fixture.expected_no_cause.as_deref() {
            assert_eq!(
                result.no_cause_reason,
                Some(parse_no_cause(expected)),
                "{} no-cause reason",
                fixture.name
            );
        }
        if fixture.generated_routine_events > 0 {
            assert_eq!(result.analyzed_events, 5_000, "{} cap", fixture.name);
            assert!(result.dropped_events > 0, "{} disclosure", fixture.name);
        }
    }
}

fn build_input(fixture: &Fixture) -> AnalysisInput {
    let mut events = fixture.events.iter().map(build_event).collect::<Vec<_>>();
    for index in 0..fixture.generated_routine_events {
        events.push(routine_event(
            10_000 + i64::try_from(index).unwrap_or(i64::MAX),
        ));
    }
    let boundary_events = fixture
        .boundary_events
        .iter()
        .map(build_event)
        .collect::<Vec<_>>();
    let relationships = fixture
        .relationships
        .iter()
        .map(|edge| build_relationship(edge, fixture.symptom_time_ms))
        .collect::<Vec<_>>();
    let mut ids = events
        .iter()
        .chain(boundary_events.iter())
        .map(|event| event.event.entity_id.clone())
        .collect::<BTreeSet<_>>();
    for edge in &relationships {
        ids.insert(edge.source_id.clone());
        ids.insert(edge.target_id.clone());
    }
    let entities = ids.into_iter().map(build_entity).collect::<Vec<_>>();
    let mut prior_occurrences_90d = BTreeMap::new();
    for event in &events {
        if event.event.event_type == EventType::Updated
            && event.event.category == Category::Application
        {
            prior_occurrences_90d.insert(event.id, 18);
        } else if event.event.event_type == EventType::Restarted {
            prior_occurrences_90d.insert(event.id, 15);
        }
    }
    AnalysisInput {
        problem: fixture.problem.clone(),
        window: InvestigationWindow::Today,
        resolved_window: ResolvedInvestigationWindow {
            from_ms: 0,
            to_ms: fixture.symptom_time_ms,
        },
        symptom_time_ms: fixture.symptom_time_ms,
        created_at: fixture.symptom_time_ms.saturating_add(1),
        total_events_in_window: events.len(),
        events,
        boundary_events,
        entities,
        relationships,
        prior_occurrences_90d,
        dropped_events: 0,
        coverage: Coverage::default(),
    }
}

#[allow(clippy::type_complexity)]
fn event_shape(
    kind: &str,
) -> (
    Category,
    EventType,
    EntityType,
    &'static str,
    &'static str,
    Importance,
) {
    match kind {
        "config" => (
            Category::Configuration,
            EventType::ConfigurationChanged,
            EntityType::File,
            "file:config",
            "Configuration",
            Importance::Important,
        ),
        "service_stopped" => (
            Category::Service,
            EventType::Stopped,
            EntityType::Service,
            "service:api",
            "API service",
            Importance::Important,
        ),
        "service_started" => (
            Category::Service,
            EventType::Started,
            EntityType::Service,
            "service:api",
            "API service",
            Importance::Notable,
        ),
        "service_restarted" => (
            Category::Service,
            EventType::Restarted,
            EntityType::Service,
            "service:api",
            "API service",
            Importance::Important,
        ),
        "port_closed" => (
            Category::Network,
            EventType::Closed,
            EntityType::Port,
            "port:tcp:3000",
            "Port 3000",
            Importance::Important,
        ),
        "port_opened" => (
            Category::Network,
            EventType::Opened,
            EntityType::Port,
            "port:tcp:4000",
            "Port 4000",
            Importance::Important,
        ),
        "chrome_update" | "app_updated" => (
            Category::Application,
            EventType::Updated,
            EntityType::Application,
            "app:chrome",
            "Chrome",
            Importance::Informational,
        ),
        "app_installed" => (
            Category::Application,
            EventType::Installed,
            EntityType::Application,
            "app:new",
            "Unrelated application",
            Importance::Notable,
        ),
        "audio_driver_updated" => (
            Category::Driver,
            EventType::Updated,
            EntityType::Driver,
            "driver:audio",
            "USB microphone audio driver",
            Importance::Important,
        ),
        "display_driver_updated" => (
            Category::Driver,
            EventType::Updated,
            EntityType::Driver,
            "driver:display",
            "Display graphics driver",
            Importance::Important,
        ),
        "proxy_changed" => (
            Category::Network,
            EventType::ConfigurationChanged,
            EntityType::Adapter,
            "adapter:proxy",
            "Proxy adapter",
            Importance::Important,
        ),
        "dns_changed" => (
            Category::Network,
            EventType::ConfigurationChanged,
            EntityType::Adapter,
            "adapter:dns",
            "DNS adapter",
            Importance::Important,
        ),
        other => panic!("unknown fixture event kind {other}"),
    }
}

fn build_event(source: &FixtureEvent) -> SystemEvent {
    let (category, event_type, entity_type, default_id, display, default_importance) =
        event_shape(&source.kind);
    let importance = source
        .importance
        .as_deref()
        .map_or(default_importance, |value| match value {
            "informational" => Importance::Informational,
            "notable" => Importance::Notable,
            "important" => Importance::Important,
            "critical" => Importance::Critical,
            other => panic!("unknown fixture importance {other}"),
        });
    let entity_id = source
        .entity_id
        .clone()
        .unwrap_or_else(|| default_id.to_owned());
    SystemEvent {
        id: source.id,
        event: NormalizedEvent {
            timestamp: source.at,
            category,
            event_type,
            entity_type,
            entity_id: entity_id.clone(),
            entity_display_name: display.into(),
            severity: if importance >= Importance::Important {
                Severity::Warning
            } else {
                Severity::Info
            },
            importance,
            source_collector: "fixture".into(),
            before: None,
            after: None,
            field: None,
            metadata: None,
            related_entity_ids: vec![],
            user_friendly_summary: format!("{display} changed"),
            technical_summary: format!("{entity_id} changed"),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: false,
        },
    }
}

fn routine_event(id: i64) -> SystemEvent {
    build_event(&FixtureEvent {
        id,
        kind: "app_updated".into(),
        entity_id: Some(format!("app:routine-{id}")),
        importance: Some("informational".into()),
        at: 5_000_000 + id,
    })
}

fn build_relationship(value: &str, recorded_at: i64) -> Relationship {
    let (source, target) = value.split_once("->").expect("relationship arrow");
    let kind = if target.starts_with("port:") {
        RelationshipKind::ListensOn
    } else if source.starts_with("config:") || source.starts_with("file:") {
        RelationshipKind::ConfigOf
    } else {
        RelationshipKind::Provides
    };
    Relationship {
        source_id: source.into(),
        target_id: target.into(),
        kind,
        event_id: None,
        recorded_at,
    }
}

fn build_entity(id: String) -> Entity {
    let entity_type = match id.split(':').next().unwrap_or_default() {
        "service" => EntityType::Service,
        "port" => EntityType::Port,
        "driver" | "device" => EntityType::Driver,
        "adapter" => EntityType::Adapter,
        "app" => EntityType::Application,
        _ => EntityType::File,
    };
    let display_name = match id.as_str() {
        "service:api" => "API service".to_owned(),
        "device:microphone" | "device:usb-microphone" => "USB microphone".to_owned(),
        "driver:audio" => "USB microphone audio driver".to_owned(),
        "driver:display" => "Display graphics driver".to_owned(),
        "adapter:proxy" => "Proxy adapter".to_owned(),
        "adapter:dns" => "DNS adapter".to_owned(),
        _ => id.split(':').next_back().unwrap_or(&id).replace('-', " "),
    };
    Entity {
        id,
        entity_type,
        display_name,
        first_seen: 0,
        last_seen: i64::MAX,
        metadata: None,
    }
}

fn parse_confidence(value: &str) -> ConfidenceClass {
    match value {
        "confirmed" => ConfidenceClass::Confirmed,
        "likely" => ConfidenceClass::Likely,
        "correlated" => ConfidenceClass::Correlated,
        other => panic!("unknown confidence {other}"),
    }
}

fn parse_no_cause(value: &str) -> NoCauseReason {
    match value {
        "only_routine_changes" => NoCauseReason::OnlyRoutineChanges,
        "nothing_changed" => NoCauseReason::NothingChanged,
        "window_too_narrow" => NoCauseReason::WindowTooNarrow,
        "not_watching_yet" => NoCauseReason::NotWatchingYet,
        other => panic!("unknown no-cause {other}"),
    }
}
