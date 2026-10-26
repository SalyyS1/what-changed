//! A small, connected, devtools-only story for the launch walkthrough.
//!
//! The edges describe observations in this synthetic story, not causation or
//! a claim about the user's machine. The two `ListensOn` edges are attached
//! to port-open observations, never to the later port-close observation.

use serde_json::{json, Value};

use crate::domain::{
    classify, ids, Category, ClassifierContext, Confidence, EntityType, EventType, Importance,
    NormalizedEvent, Relationship, RelationshipKind, Severity,
};
use crate::error::{AppError, AppResult};
use crate::privacy::{Sanitized, Sanitizer};

pub(super) const ROWS: usize = 8;

const CONFIG_PATH: &str = r"C:\WhatChangedDemo\api\.env";
const EXECUTABLE_PATH: &str = r"C:\WhatChangedDemo\api\api.exe";
const SERVICE_NAME: &str = "WhatChangedDemoApi";

/// One stable story shifted to end shortly before `end_at`.
pub(super) fn profile(end_at: i64) -> Vec<Sanitized<NormalizedEvent>> {
    let file_id = ids::file(CONFIG_PATH);
    let service_id = ids::service(SERVICE_NAME);
    let process_id = ids::process(EXECUTABLE_PATH);
    let old_port_id = ids::port(ids::Protocol::Tcp, "0.0.0.0", 3000);
    let new_port_id = ids::port(ids::Protocol::Tcp, "0.0.0.0", 8080);

    let mut config = event(DemoObservation {
        timestamp: end_at.saturating_sub(180_000),
        category: Category::Configuration,
        event_type: EventType::ConfigurationChanged,
        entity_type: EntityType::File,
        entity_id: file_id,
        display_name: ".env (demo project)",
        summary: "PORT changed from 3000 to 8080 in .env",
        before: json!({"PORT":"3000","API_TOKEN":"demo_before_value"}),
        after: json!({"PORT":"8080","API_TOKEN":"demo_after_value"}),
        related_entity_ids: vec![service_id.clone(), process_id.clone()],
        context: ClassifierContext::default(),
    });
    config.field = Some("PORT".to_owned());
    config.metadata = Some(json!({
        "file": ".env",
        "configured_service": SERVICE_NAME,
        "configured_process": "api.exe",
        "synthetic_fixture": "demo"
    }));

    let mut service_stopped = event(DemoObservation {
        timestamp: end_at.saturating_sub(120_000),
        category: Category::Service,
        event_type: EventType::Stopped,
        entity_type: EntityType::Service,
        entity_id: service_id.clone(),
        display_name: "Demo API service",
        summary: "Demo API service stopped",
        before: json!({"state":"running","start_type":"automatic"}),
        after: json!({"state":"stopped","start_type":"automatic"}),
        related_entity_ids: Vec::new(),
        context: ClassifierContext {
            automatic_service: true,
            ..Default::default()
        },
    });
    service_stopped.metadata = Some(json!({"start_type":"automatic"}));

    let process_stopped = event(DemoObservation {
        timestamp: end_at.saturating_sub(119_700),
        category: Category::Process,
        event_type: EventType::Stopped,
        entity_type: EntityType::Process,
        entity_id: process_id.clone(),
        display_name: "api.exe (demo)",
        summary: "Demo API process stopped",
        before: json!({"running":true}),
        after: json!({"running":false}),
        related_entity_ids: Vec::new(),
        context: ClassifierContext::default(),
    });

    let port_closed = event(DemoObservation {
        timestamp: end_at.saturating_sub(119_400),
        category: Category::Network,
        event_type: EventType::Closed,
        entity_type: EntityType::Port,
        entity_id: old_port_id.clone(),
        display_name: "TCP port 3000",
        summary: "TCP port 3000 stopped listening",
        before: json!({"listening":true}),
        after: json!({"listening":false}),
        related_entity_ids: vec![process_id.clone()],
        context: ClassifierContext::default(),
    });

    let service_started = event(DemoObservation {
        timestamp: end_at.saturating_sub(60_000),
        category: Category::Service,
        event_type: EventType::Started,
        entity_type: EntityType::Service,
        entity_id: service_id.clone(),
        display_name: "Demo API service",
        summary: "Demo API service started",
        before: json!({"state":"stopped"}),
        after: json!({"state":"running"}),
        related_entity_ids: Vec::new(),
        context: ClassifierContext::default(),
    });

    let process_started = event(DemoObservation {
        timestamp: end_at.saturating_sub(59_700),
        category: Category::Process,
        event_type: EventType::Started,
        entity_type: EntityType::Process,
        entity_id: process_id.clone(),
        display_name: "api.exe (demo)",
        summary: "Demo API process started",
        before: json!({"running":false}),
        after: json!({"running":true}),
        related_entity_ids: Vec::new(),
        context: ClassifierContext::default(),
    });

    let port_opened = event(DemoObservation {
        timestamp: end_at.saturating_sub(59_400),
        category: Category::Network,
        event_type: EventType::Opened,
        entity_type: EntityType::Port,
        entity_id: new_port_id,
        display_name: "TCP port 8080",
        summary: "TCP port 8080 began listening",
        before: json!({"listening":false}),
        after: json!({"listening":true}),
        related_entity_ids: vec![process_id.clone()],
        context: ClassifierContext::default(),
    });

    let initial_port_opened = event(DemoObservation {
        timestamp: end_at.saturating_sub(240_000),
        category: Category::Network,
        event_type: EventType::Opened,
        entity_type: EntityType::Port,
        entity_id: old_port_id,
        display_name: "TCP port 3000",
        summary: "TCP port 3000 was listening",
        before: json!({"listening":false}),
        after: json!({"listening":true}),
        related_entity_ids: vec![process_id],
        context: ClassifierContext::default(),
    });

    let sanitizer = Sanitizer::new();
    [
        initial_port_opened,
        config,
        service_stopped,
        process_stopped,
        port_closed,
        service_started,
        process_started,
        port_opened,
    ]
    .into_iter()
    .map(|row| sanitizer.sanitize(row).0)
    .collect()
}

struct DemoObservation {
    timestamp: i64,
    category: Category,
    event_type: EventType,
    entity_type: EntityType,
    entity_id: String,
    display_name: &'static str,
    summary: &'static str,
    before: Value,
    after: Value,
    related_entity_ids: Vec<String>,
    context: ClassifierContext,
}

fn event(observation: DemoObservation) -> NormalizedEvent {
    let DemoObservation {
        timestamp,
        category,
        event_type,
        entity_type,
        entity_id,
        display_name,
        summary,
        before,
        after,
        related_entity_ids,
        context,
    } = observation;
    let mut row = NormalizedEvent {
        timestamp,
        category,
        event_type,
        entity_type,
        entity_id,
        entity_display_name: display_name.to_owned(),
        severity: Severity::Info,
        importance: Importance::Informational,
        source_collector: "synthetic".to_owned(),
        before: Some(before),
        after: Some(after),
        field: None,
        metadata: None,
        related_entity_ids,
        user_friendly_summary: summary.to_owned(),
        technical_summary: format!("synthetic demo {category:?} {event_type:?}"),
        confidence: Confidence::High,
        sensitive: false,
        occurred_at_is_uncertain: false,
    };
    row.importance = classify(&row, &context);
    row
}

/// Graph evidence is anchored to its observation; old-port listening is a
/// historical edge and must not be narrated as the current socket state.
pub(super) fn relationships(
    event_ids: &[i64],
    events: &[NormalizedEvent],
) -> AppResult<Vec<Relationship>> {
    if event_ids.len() != ROWS || events.len() != ROWS {
        return Err(AppError::InvalidInput {
            what: "profile",
            reason: "the demo fixture did not write its expected rows".to_owned(),
        });
    }
    let file_id = ids::file(CONFIG_PATH);
    let service_id = ids::service(SERVICE_NAME);
    let process_id = ids::process(EXECUTABLE_PATH);
    let old_port_id = ids::port(ids::Protocol::Tcp, "0.0.0.0", 3000);
    let new_port_id = ids::port(ids::Protocol::Tcp, "0.0.0.0", 8080);
    let observed = |index: usize, source_id: String, target_id: String, kind| {
        Some(Relationship {
            source_id,
            target_id,
            kind,
            event_id: Some(*event_ids.get(index)?),
            recorded_at: events.get(index)?.timestamp,
        })
    };
    [
        observed(1, file_id.clone(), service_id, RelationshipKind::ConfigOf),
        observed(1, file_id, process_id.clone(), RelationshipKind::ConfigOf),
        observed(
            0,
            process_id.clone(),
            old_port_id,
            RelationshipKind::ListensOn,
        ),
        observed(7, process_id, new_port_id, RelationshipKind::ListensOn),
    ]
    .into_iter()
    .collect::<Option<Vec<_>>>()
    .ok_or_else(|| AppError::InvalidInput {
        what: "profile",
        reason: "the demo fixture did not write its expected rows".to_owned(),
    })
}
