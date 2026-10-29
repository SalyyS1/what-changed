#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::Arc;

use serde_json::json;
use what_changed_lib::domain::{
    ids, now_ms, Category, EventType, InvestigationWindow, RelationshipKind,
};
use what_changed_lib::services::investigation_service::InvestigationService;
use what_changed_lib::storage::{entity_repository, event_repository, Db};
use what_changed_lib::testing::{synthetic, temp_dir};

#[test]
fn demo_profile_has_canonical_ids_chronology_and_sanitized_secret_values() {
    let end_at = 1_700_000_000_000;
    let first = synthetic::profile(synthetic::SyntheticProfile::Demo, 1, end_at);
    let second = synthetic::profile(synthetic::SyntheticProfile::Demo, 99, end_at);
    assert_eq!(first, second, "random seed must not change the demo story");
    assert_eq!(first.len(), 8);

    let rows = first.iter().map(|row| row.get()).collect::<Vec<_>>();
    assert!(rows
        .windows(2)
        .all(|pair| pair[0].timestamp < pair[1].timestamp));
    assert!(rows.iter().all(|row| {
        row.source_collector == "synthetic"
            && ids::is_valid_entity_id_for(&row.entity_id, row.entity_type)
            && row
                .related_entity_ids
                .iter()
                .all(|related| ids::is_valid_entity_id(related))
    }));
    assert_eq!(rows[0].event_type, EventType::Opened);
    assert_eq!(rows[1].category, Category::Configuration);
    assert_eq!(rows[4].event_type, EventType::Closed);
    assert_eq!(rows[7].event_type, EventType::Opened);
    assert_eq!(rows[1].field.as_deref(), Some("PORT"));
    assert_eq!(rows[1].before.as_ref().unwrap()["PORT"], json!("3000"));
    assert_eq!(rows[1].after.as_ref().unwrap()["PORT"], json!("8080"));
    assert_eq!(
        rows[1].before.as_ref().unwrap()["API_TOKEN"],
        json!("[REDACTED]")
    );
    assert_eq!(
        rows[1].after.as_ref().unwrap()["API_TOKEN"],
        json!("[REDACTED]")
    );
    assert!(rows[1].sensitive);
    assert!(!format!("{first:?}").contains("demo_before_value"));
    assert!(!format!("{first:?}").contains("demo_after_value"));
}

#[test]
fn demo_seed_persists_four_edges_even_when_generic_relations_are_off() {
    let db = Db::open_in(&temp_dir("demo-story-edges")).expect("open demo DB");
    let end_at = 1_700_000_000_000;
    let ids = synthetic::seed(&db, "demo", 123, end_at, false).expect("seed demo story");
    assert_eq!(ids.len(), 8);

    let file_id = ids::file(r"C:\WhatChangedDemo\api\.env");
    let service_id = ids::service("WhatChangedDemoApi");
    let process_id = ids::process(r"C:\WhatChangedDemo\api\api.exe");
    let old_port_id = ids::port(ids::Protocol::Tcp, "0.0.0.0", 3000);
    let new_port_id = ids::port(ids::Protocol::Tcp, "0.0.0.0", 8080);
    let (from_file, from_process) = db
        .read(|conn| {
            Ok((
                entity_repository::relationships_from(conn, &file_id)?,
                entity_repository::relationships_from(conn, &process_id)?,
            ))
        })
        .expect("read persisted relationships");
    assert_eq!(from_file.len(), 2);
    assert_eq!(from_process.len(), 2);
    assert!(from_file.iter().any(|edge| {
        edge.target_id == service_id
            && edge.kind == RelationshipKind::ConfigOf
            && edge.event_id == Some(ids[1])
    }));
    assert!(from_file.iter().any(|edge| {
        edge.target_id == process_id
            && edge.kind == RelationshipKind::ConfigOf
            && edge.event_id == Some(ids[1])
    }));
    assert!(from_process.iter().any(|edge| {
        edge.target_id == old_port_id
            && edge.kind == RelationshipKind::ListensOn
            && edge.event_id == Some(ids[0])
    }));
    assert!(from_process.iter().any(|edge| {
        edge.target_id == new_port_id
            && edge.kind == RelationshipKind::ListensOn
            && edge.event_id == Some(ids[7])
    }));
    assert!(from_process
        .iter()
        .all(|edge| edge.event_id != Some(ids[4])));

    for id in ids {
        let event = db
            .read(|conn| event_repository::by_id(conn, id))
            .expect("read demo event")
            .expect("persisted demo event");
        assert_eq!(event.event.source_collector, "synthetic");
    }
}

#[test]
fn demo_investigation_reports_linked_evidence_without_a_causal_claim() {
    let db = Arc::new(Db::open_in(&temp_dir("demo-story-investigation")).expect("open demo DB"));
    let end_at = now_ms();
    let ids = synthetic::seed(&db, "demo", 0, end_at, false).expect("seed demo story");
    let service = InvestigationService::new(Arc::clone(&db));
    let result = service
        .start(
            "port 3000 stopped responding".to_owned(),
            Some(InvestigationWindow::Custom {
                from_ms: end_at - 300_000,
                to_ms: end_at,
            }),
        )
        .expect("analyze the demo window");

    assert_eq!(result.total_events_in_window, 8);
    assert!(result.suspects.iter().any(|suspect| {
        suspect.breakdown.relationship > 0.0
            && suspect
                .chain
                .iter()
                .any(|step| step.event_ids.contains(&ids[1]))
            && suspect
                .chain
                .iter()
                .any(|step| step.event_ids.contains(&ids[4]) || step.event_ids.contains(&ids[7]))
    }));
    assert!(result.suspects.iter().all(|suspect| {
        let explanation = format!("{:?} {}", suspect.why_ranked, suspect.confidence_text);
        !explanation.contains("caused") && !explanation.contains("because of")
    }));
}
