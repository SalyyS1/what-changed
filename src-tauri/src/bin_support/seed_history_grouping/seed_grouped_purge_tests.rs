use rusqlite::types::Value;
use serde_json::json;
use what_changed_lib::checkpoint::prepare_capture;
use what_changed_lib::domain::{
    CheckpointItem, CheckpointKind, Entity, EntityType, InvestigationWindow, Relationship,
    RelationshipKind,
};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::services::investigation_service::InvestigationService;
use what_changed_lib::storage::{
    checkpoint_repository, entity_repository, maintenance, settings_repository,
};

use super::failure_tests::seed_grouped_failure_fixtures::{options, END_AT};
use super::seed_grouped_fixture::OwnedFixture;
use super::seed_grouped_persisted_parity::{count, rows};
use super::*;

struct RealTable {
    name: &'static str,
    rows: Vec<Vec<Value>>,
}

#[test]
fn purge_removes_registered_grouped_artifacts_and_preserves_real_and_possible_entities() {
    let fixture = OwnedFixture::new();
    let db = fixture.db();
    let at = END_AT - crate::DAY_MS;
    let mut event = what_changed_lib::testing::synthetic_event(7, at);
    event.source_collector = "real-fixture".to_owned();
    event.entity_id = "application:RealQaEvidence".to_owned();
    event.entity_type = EntityType::Application;
    event.entity_display_name = "Real evidence".to_owned();
    event.related_entity_ids.clear();
    let real_id = event.entity_id.clone();
    let ids = db
        .insert_events(vec![Sanitizer::new().sanitize(event).0])
        .unwrap();
    let possible = Entity {
        id: "service:PossibleQaOwner".to_owned(),
        entity_type: EntityType::Service,
        display_name: "Possible ownership".to_owned(),
        first_seen: at,
        last_seen: at,
        metadata: Some(json!({"evidence": "unchanged"})),
    };
    let possible_id = possible.id.clone();
    let edge = Relationship {
        source_id: real_id.clone(),
        target_id: possible_id.clone(),
        kind: RelationshipKind::Provides,
        event_id: Some(ids[0]),
        recorded_at: at,
    };
    db.write(move |conn| {
        entity_repository::upsert_sanitized(conn, &Sanitizer::new().sanitize(possible).0)?;
        entity_repository::insert_relationship(conn, &edge)
    })
    .unwrap();
    let (checkpoint, _) = prepare_capture(
        Some("Synthetic words in a real checkpoint".to_owned()),
        CheckpointKind::Manual,
        Some("synthetic but not a registered marker".to_owned()),
        at,
        0,
        vec!["services".to_owned()],
        Vec::new(),
        vec![CheckpointItem {
            collector_id: "services".to_owned(),
            entity_kind: "service".to_owned(),
            entity_external_id: possible_id.clone(),
            entity_name: Some("Possible ownership".to_owned()),
            state: json!({"state": "running"}),
        }],
        &Sanitizer::new(),
    );
    db.write(move |conn| checkpoint_repository::create(conn, &checkpoint))
        .unwrap();
    InvestigationService::new(std::sync::Arc::clone(db))
        .start(
            "Synthetic words in a real investigation".to_owned(),
            Some(InvestigationWindow::Custom {
                from_ms: at - 1_000,
                to_ms: at + 1_000,
            }),
        )
        .unwrap();
    let original = db
        .read(|conn| {
            Ok([
                "events",
                "entity_relationships",
                "checkpoints",
                "snapshots",
                "investigations",
                "analysis_sessions",
                "analysis_results",
            ]
            .into_iter()
            .map(|name| RealTable {
                name,
                rows: rows(conn, name),
            })
            .collect::<Vec<_>>())
        })
        .unwrap();
    let original_entities = db
        .read(|conn| {
            Ok([
                entity_repository::by_id(conn, &real_id).unwrap().unwrap(),
                entity_repository::by_id(conn, &possible_id)
                    .unwrap()
                    .unwrap(),
            ])
        })
        .unwrap();
    let options = options();
    let marker = maintenance::register_synthetic_run(db).unwrap();
    let mut coarse = SeedTimings::default();
    let run = seed_grouped_events(db, &options, END_AT, &mut coarse);
    let state = run.value.unwrap();
    assert_eq!(run.attribution.committed_input_rows, options.events);
    crate::finish_seed(
        db,
        &options,
        &mut coarse,
        END_AT,
        &marker,
        state.inserted,
        state.owned_entities,
    )
    .unwrap();
    assert_eq!(maintenance::purge_synthetic(db).unwrap(), options.events);
    assert_eq!(maintenance::purge_synthetic(db).unwrap(), 0);
    db.read(|conn| {
        for original in &original {
            assert!(
                rows(conn, original.name) == original.rows,
                "real persisted table changed during purge: {}",
                original.name
            );
        }
        for original in &original_entities {
            assert!(
                entity_repository::by_id(conn, &original.id)
                    .unwrap()
                    .as_ref()
                    == Some(original)
            );
        }
        assert!(count(conn, "entities") >= ENTITY_COUNT as usize);
        assert!(
            settings_repository::get_json(conn, "devtools_seed_history_markers_v1")
                .unwrap()
                .is_none()
        );
        Ok(())
    })
    .unwrap();
}
