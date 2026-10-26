#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::Arc;

use serde_json::json;
use what_changed_lib::domain::{
    Category, Confidence, ConfidenceClass, EntityType, EventType, Importance, InvestigationWindow,
    NoCauseReason, NormalizedEvent, Relationship, RelationshipKind, Severity,
};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::services::investigation_service::InvestigationService;
use what_changed_lib::storage::{entity_repository, event_repository, Db};
use what_changed_lib::testing::temp_dir;

fn event(
    timestamp: i64,
    category: Category,
    event_type: EventType,
    entity_type: EntityType,
    entity_id: &str,
    display_name: &str,
    importance: Importance,
) -> NormalizedEvent {
    NormalizedEvent {
        timestamp,
        category,
        event_type,
        entity_type,
        entity_id: entity_id.to_owned(),
        entity_display_name: display_name.to_owned(),
        severity: Severity::Warning,
        importance,
        source_collector: "service-integration-test".to_owned(),
        before: None,
        after: None,
        field: None,
        metadata: None,
        related_entity_ids: vec![],
        user_friendly_summary: format!("{display_name} changed"),
        technical_summary: format!("{entity_id} changed"),
        confidence: Confidence::High,
        sensitive: false,
        occurred_at_is_uncertain: false,
    }
}

fn custom(from_ms: i64, to_ms: i64) -> InvestigationWindow {
    InvestigationWindow::Custom { from_ms, to_ms }
}

fn insert(db: &Db, events: Vec<NormalizedEvent>) -> Vec<i64> {
    let sanitizer = Sanitizer::new();
    db.insert_events(
        events
            .into_iter()
            .map(|event| sanitizer.sanitize(event).0)
            .collect(),
    )
    .expect("insert events")
}

fn relate(
    db: &Db,
    source_id: &str,
    target_id: &str,
    kind: RelationshipKind,
    event_id: Option<i64>,
    recorded_at: i64,
) {
    let relationship = Relationship {
        source_id: source_id.to_owned(),
        target_id: target_id.to_owned(),
        kind,
        event_id,
        recorded_at,
    };
    db.write(move |conn| entity_repository::insert_relationship(conn, &relationship))
        .expect("insert relationship");
}

#[test]
fn start_round_trips_sanitized_snapshot_and_survives_event_deletion() {
    let db = Arc::new(Db::open_in(&temp_dir("investigation-service-roundtrip")).unwrap());
    let service = InvestigationService::new(Arc::clone(&db));
    let now = what_changed_lib::domain::now_ms();
    insert(
        &db,
        vec![event(
            now - 1_000,
            Category::Application,
            EventType::Updated,
            EntityType::Application,
            "application:routine",
            "Routine App",
            Importance::Informational,
        )],
    );

    let created = service
        .start(
            "yesterday token sk-abcdefghijklmnopqrstuvwx the app changed".to_owned(),
            None,
        )
        .expect("start investigation");
    assert!(
        created.id > 0,
        "persisted ids must remain numeric and positive"
    );
    assert_eq!(created.window, InvestigationWindow::SinceYesterday);
    assert!(!created.problem.contains("sk-abcdefghijklmnopqrstuvwx"));
    assert!(created.problem.contains("[REDACTED]"));

    let listed = service.list(10).expect("list investigations");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0], created);
    assert_eq!(service.get(created.id).expect("get investigation"), created);

    let mut legacy = serde_json::to_value(&created).expect("encode snapshot");
    legacy
        .as_object_mut()
        .expect("snapshot object")
        .remove("project_id");
    let reopened_legacy: what_changed_lib::domain::Investigation =
        serde_json::from_value(legacy).expect("legacy snapshot remains readable");
    assert_eq!(reopened_legacy.project_id, None);

    db.write(|conn| {
        conn.execute("DELETE FROM events", [])
            .map(|_| ())
            .map_err(|source| {
                what_changed_lib::error::DbError::Sqlite {
                    operation: "delete source events for snapshot test",
                    source,
                }
                .into()
            })
    })
    .expect("delete retained source events");
    assert_eq!(
        service.get(created.id).expect("reopen immutable snapshot"),
        created
    );

    service.delete(created.id).expect("delete investigation");
    assert!(service.list(10).expect("list after delete").is_empty());
    assert!(service.get(created.id).is_err());
}

#[test]
fn database_worked_example_ranks_config_chain_confirmed() {
    let db = Arc::new(Db::open_in(&temp_dir("investigation-service-worked")).unwrap());
    let service = InvestigationService::new(Arc::clone(&db));
    let now = what_changed_lib::domain::now_ms();
    let start = now - 10 * 60_000;
    let mut config = event(
        start,
        Category::Configuration,
        EventType::ConfigurationChanged,
        EntityType::File,
        "file:api-env",
        ".env PORT 3000 to Port 4000",
        Importance::Important,
    );
    config.field = Some("PORT".to_owned());
    config.before = Some(json!("3000"));
    config.after = Some(json!("4000"));
    let ids = insert(
        &db,
        vec![
            config,
            event(
                start + 60_000,
                Category::Service,
                EventType::Stopped,
                EntityType::Service,
                "service:api",
                "API service",
                Importance::Important,
            ),
            event(
                start + 60_300,
                Category::Service,
                EventType::Started,
                EntityType::Service,
                "service:api",
                "API service",
                Importance::Notable,
            ),
            event(
                start + 120_000,
                Category::Network,
                EventType::Closed,
                EntityType::Port,
                "port:tcp:3000",
                "Port 3000",
                Importance::Important,
            ),
            event(
                start + 120_300,
                Category::Network,
                EventType::Opened,
                EntityType::Port,
                "port:tcp:4000",
                "Port 4000",
                Importance::Important,
            ),
        ],
    );
    relate(
        &db,
        "file:api-env",
        "service:api",
        RelationshipKind::ConfigOf,
        Some(ids[0]),
        start,
    );
    relate(
        &db,
        "service:api",
        "port:tcp:3000",
        RelationshipKind::ListensOn,
        Some(ids[3]),
        start + 120_000,
    );
    relate(
        &db,
        "service:api",
        "port:tcp:4000",
        RelationshipKind::ListensOn,
        Some(ids[4]),
        start + 120_300,
    );

    let result = service
        .start(
            "my API service cannot connect to Port 4000".to_owned(),
            Some(custom(start - 1, now)),
        )
        .expect("run persisted investigation");
    let top = result.suspects.first().expect("top suspect");
    assert_eq!(top.event_id, ids[0]);
    assert_eq!(top.confidence, ConfidenceClass::Confirmed);
    assert_eq!(top.chain.len(), 3);
    assert!(top
        .chain
        .iter()
        .any(|step| step.label.contains("Port 4000")));
    assert_eq!(service.get(result.id).unwrap(), result);

    // More than the global analysis cap of high-importance hidden-category
    // churn must not crowd this visible Timeline chain out before analysis.
    insert(
        &db,
        (0..5_100)
            .map(|index| {
                let entity_id = format!("application:hidden-{index}");
                let display_name = format!("Hidden app {index}");
                event(
                    start + i64::from(index),
                    Category::Application,
                    EventType::Updated,
                    EntityType::Application,
                    &entity_id,
                    &display_name,
                    Importance::Critical,
                )
            })
            .collect(),
    );
    let (filtered, filtered_source) = service
        .analyze_filtered_window(
            custom(start - 1, now),
            &[
                Category::Configuration,
                Category::Service,
                Category::Network,
            ],
            None,
            None,
        )
        .expect("analyze filtered Timeline window");
    assert_eq!(filtered_source.len(), 5);
    let filtered_top = filtered.suspects.first().expect("filtered top suspect");
    assert_eq!(filtered_top.event_id, ids[0]);
    assert_eq!(filtered_top.chain.len(), 3);
}

#[test]
fn related_events_uses_persisted_structural_graph() {
    let db = Arc::new(Db::open_in(&temp_dir("investigation-related-events")).unwrap());
    let service = InvestigationService::new(Arc::clone(&db));
    let now = what_changed_lib::domain::now_ms();
    let ids = insert(
        &db,
        vec![
            event(
                now - 2_000,
                Category::Configuration,
                EventType::ConfigurationChanged,
                EntityType::File,
                "file:api-env",
                "API config",
                Importance::Important,
            ),
            event(
                now - 1_000,
                Category::Service,
                EventType::Restarted,
                EntityType::Service,
                "service:api",
                "API service",
                Importance::Important,
            ),
            event(
                now - 1_500,
                Category::Application,
                EventType::Updated,
                EntityType::Application,
                "application:nearby",
                "Nearby application",
                Importance::Notable,
            ),
        ],
    );
    relate(
        &db,
        "file:api-env",
        "service:api",
        RelationshipKind::ConfigOf,
        Some(ids[0]),
        now - 2_000,
    );

    let related = service.related_events(ids[0]).expect("related event query");
    assert!(related.iter().any(|event| event.id == ids[1]));
    assert!(
        related.iter().any(|event| event.id == ids[2]),
        "the graph neighbourhood includes temporal edges too"
    );
    assert!(related.iter().all(|event| event.id != ids[0]));
}

#[test]
fn boundary_search_skips_unrelated_noise_and_finds_the_cascade_head() {
    let db = Arc::new(Db::open_in(&temp_dir("investigation-boundary-context")).unwrap());
    let service = InvestigationService::new(Arc::clone(&db));
    let now = what_changed_lib::domain::now_ms();
    let config_at = now - 180_000;
    let service_at = now - 120_000;
    let ids = insert(
        &db,
        vec![
            event(
                config_at,
                Category::Configuration,
                EventType::ConfigurationChanged,
                EntityType::File,
                "file:api-env",
                "API config",
                Importance::Important,
            ),
            event(
                service_at - 1,
                Category::Application,
                EventType::Updated,
                EntityType::Application,
                "application:noise",
                "Unrelated update",
                Importance::Informational,
            ),
            event(
                service_at,
                Category::Service,
                EventType::Restarted,
                EntityType::Service,
                "service:api",
                "API service",
                Importance::Important,
            ),
            event(
                service_at + 60_000,
                Category::Network,
                EventType::Closed,
                EntityType::Port,
                "port:tcp:3000",
                "Port 3000",
                Importance::Important,
            ),
        ],
    );
    relate(
        &db,
        "file:api-env",
        "service:api",
        RelationshipKind::ConfigOf,
        Some(ids[0]),
        config_at,
    );
    relate(
        &db,
        "service:api",
        "port:tcp:3000",
        RelationshipKind::ListensOn,
        Some(ids[3]),
        service_at + 60_000,
    );

    let result = service
        .start("api port refused".to_owned(), Some(custom(service_at, now)))
        .expect("analyze with noisy boundary");
    let service_suspect = result
        .suspects
        .iter()
        .find(|suspect| suspect.event_id == ids[2])
        .expect("service cascade suspect");
    assert_eq!(service_suspect.chain.len(), 3);
    assert_eq!(service_suspect.confidence, ConfidenceClass::Likely);
}

#[test]
fn empty_database_returns_explicit_no_answer() {
    let db = Arc::new(Db::open_in(&temp_dir("investigation-service-empty")).unwrap());
    let service = InvestigationService::new(db);
    let now = what_changed_lib::domain::now_ms();
    let result = service
        .start(
            "wifi stopped working".to_owned(),
            Some(custom(now - 60_000, now)),
        )
        .expect("empty investigation");
    assert!(result.suspects.is_empty());
    assert_eq!(result.no_cause_reason, Some(NoCauseReason::NothingChanged));
    assert_eq!(
        result.no_cause_message.as_deref(),
        Some("Nothing meaningful changed in this window.")
    );
}

#[test]
fn scoped_investigation_includes_direct_and_related_project_events_before_the_limit() {
    let db = Arc::new(Db::open_in(&temp_dir("investigation-service-project-scope")).unwrap());
    let service = InvestigationService::new(Arc::clone(&db));
    let now = what_changed_lib::domain::now_ms();
    let project_id = "project:workspace";
    let mut related = event(
        now - 900,
        Category::Service,
        EventType::Restarted,
        EntityType::Service,
        "service:api",
        "API service",
        Importance::Important,
    );
    related.related_entity_ids = vec![project_id.to_owned()];
    insert(
        &db,
        vec![
            event(
                now - 1_000,
                Category::Configuration,
                EventType::ConfigurationChanged,
                EntityType::Project,
                project_id,
                "workspace config",
                Importance::Important,
            ),
            related,
            event(
                now - 800,
                Category::Application,
                EventType::Updated,
                EntityType::Application,
                "application:unrelated",
                "Unrelated application",
                Importance::Critical,
            ),
        ],
    );
    insert(
        &db,
        (0..5_100)
            .map(|index| {
                event(
                    now - 10_000 + i64::from(index),
                    Category::Application,
                    EventType::Updated,
                    EntityType::Application,
                    &format!("application:noise-{index}"),
                    "Unrelated noise",
                    Importance::Critical,
                )
            })
            .collect(),
    );

    let result = service
        .start_scoped(
            "workspace API stopped".to_owned(),
            Some(custom(now - 20_000, now)),
            Some(project_id.to_owned()),
        )
        .expect("scoped investigation");
    assert_eq!(result.project_id.as_deref(), Some(project_id));
    assert_eq!(result.total_events_in_window, 2);
    assert_eq!(result.analyzed_events, 2);
    assert!(result.suspects.iter().all(|suspect| suspect.event_id > 0));
}

#[test]
fn service_caps_an_exact_5001_row_window_at_5000() {
    let db = Arc::new(Db::open_in(&temp_dir("investigation-service-cap")).unwrap());
    let service = InvestigationService::new(Arc::clone(&db));
    let now = what_changed_lib::domain::now_ms();
    let sanitizer = Sanitizer::new();
    for batch_start in (0..5_001_usize).step_by(500) {
        let batch_end = (batch_start + 500).min(5_001);
        let batch = (batch_start..batch_end)
            .map(|index| {
                let mut row = event(
                    now - 10_000 + i64::try_from(index).unwrap(),
                    Category::Application,
                    EventType::Updated,
                    EntityType::Application,
                    &format!("application:routine-{index}"),
                    "Routine App",
                    Importance::Informational,
                );
                row.user_friendly_summary = format!("Routine update {index}");
                sanitizer.sanitize(row).0
            })
            .collect();
        db.insert_events(batch).expect("insert capped corpus");
    }

    let result = service
        .analyze_window(custom(now - 10_001, now))
        .expect("analyze capped corpus");
    assert_eq!(result.total_events_in_window, 5_001);
    assert_eq!(
        result.analyzed_events,
        event_repository::MAX_ANALYSIS_EVENTS
    );
    assert_eq!(result.dropped_events, 1);
    assert_eq!(result.id, 0, "dashboard analysis must not persist history");
}
