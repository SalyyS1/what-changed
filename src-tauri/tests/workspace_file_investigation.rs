#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::Arc;
use std::time::Instant;

use serde_json::{json, Value};
use what_changed_lib::domain::{
    ids, Category, CheckpointCapture, CheckpointKind, Confidence, ConfidenceClass, EntityType,
    EventType, Importance, InvestigationWindow, NewCheckpoint, NoCauseReason, NormalizedEvent,
    Severity,
};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::services::investigation_service::InvestigationService;
use what_changed_lib::storage::{checkpoint_repository, event_repository, project_repository, Db};
use what_changed_lib::testing::temp_dir;

fn file_event(timestamp: i64, path: &str) -> NormalizedEvent {
    NormalizedEvent {
        timestamp,
        category: Category::Configuration,
        event_type: EventType::ConfigurationChanged,
        entity_type: EntityType::File,
        entity_id: ids::file(path),
        entity_display_name: ".env PORT".to_owned(),
        severity: Severity::Warning,
        importance: Importance::Important,
        source_collector: "files".to_owned(),
        before: Some(json!("3000")),
        after: Some(json!("4000")),
        field: Some("PORT".to_owned()),
        metadata: Some(json!({ "path": path })),
        related_entity_ids: vec![],
        user_friendly_summary: "PORT changed from 3000 to 4000".to_owned(),
        technical_summary: "PORT changed".to_owned(),
        confidence: Confidence::High,
        sensitive: false,
        occurred_at_is_uncertain: false,
    }
}

fn insert(db: &Db, events: Vec<NormalizedEvent>) -> Vec<i64> {
    let sanitizer = Sanitizer::new();
    db.insert_events(
        events
            .into_iter()
            .map(|event| sanitizer.sanitize(event).0)
            .collect(),
    )
    .expect("insert recorded events")
}

fn register(db: &Db, path: &str, added_at: i64) -> String {
    let path = path.to_owned();
    db.write(move |conn| project_repository::insert(conn, &path, "API workspace", added_at))
        .expect("register workspace")
        .id
}

fn corpus(db: &Db, project_id: &str, from: i64, to: i64) -> event_repository::AnalysisEventWindow {
    db.read(|conn| event_repository::analysis_window_workspace(conn, from, to, project_id))
        .expect("read workspace corpus")
}

#[test]
fn checkpoint_investigation_finds_historical_file_keys_without_process_evidence() {
    let db = Arc::new(Db::open_in(&temp_dir("workspace-historical-config")).unwrap());
    let now = what_changed_lib::domain::now_ms();
    let capture = CheckpointCapture {
        checkpoint: NewCheckpoint {
            name: "Known working".to_owned(),
            kind: CheckpointKind::Manual,
            reason: None,
            created_at: now - 20_000,
            duration_ms: 0,
            captured_collectors: vec![],
            missing_collectors: vec![],
        },
        items: vec![],
    };
    let checkpoint = db
        .write(move |conn| {
            checkpoint_repository::create(conn, &Sanitizer::new().sanitize(capture).0)
        })
        .unwrap();
    let mut host = file_event(now - 9_000, r"D:\work\api\.env");
    host.field = Some("HOST".to_owned());
    host.before = Some(json!("localhost"));
    host.after = Some(json!("127.0.0.1"));
    let ids = insert(
        &db,
        vec![file_event(now - 10_000, r"D:\work\api\.env"), host],
    );
    // Registration happens after both observations, with no running process.
    let project_id = register(&db, r"D:\work\api", now);
    let service = InvestigationService::new(Arc::clone(&db));
    let result = service
        .start_scoped(
            "API no longer starts on Port 3000".to_owned(),
            Some(InvestigationWindow::SinceCheckpoint {
                checkpoint_id: checkpoint.id,
            }),
            Some(project_id.clone()),
        )
        .unwrap();
    assert_eq!(result.total_events_in_window, 2);
    assert_eq!(result.analyzed_events, 2);
    assert_ne!(
        result.no_cause_reason,
        Some(NoCauseReason::NoRecordedEvidenceInWorkspace)
    );
    assert!(result
        .suspects
        .iter()
        .any(|suspect| suspect.event_id == ids[0]));
    assert!(result
        .suspects
        .iter()
        .all(|suspect| suspect.confidence != ConfidenceClass::Confirmed));
    assert_eq!(service.get(result.id).unwrap(), result);
    let recorded = corpus(&db, &project_id, now - 20_000, now);
    assert_eq!(
        recorded
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        ids
    );
    assert!(recorded
        .events
        .iter()
        .all(|event| event.event.related_entity_ids.is_empty()));
}

#[test]
fn membership_uses_canonical_paths_and_literal_separator_boundaries() {
    let db = Db::open_in(&temp_dir("workspace-path-boundaries")).unwrap();
    let project_id = register(&db, r"D:\Work\ÁPI%_", 1);
    let matching = [
        r"D:\Work\ÁPI%_",
        r"d:/work/ápi%_/.env",
        r"\\?\D:\WORK\ÁPI%_\nested\settings.json",
    ];
    let mut events = matching
        .iter()
        .map(|path| file_event(100, path))
        .collect::<Vec<_>>();
    events.extend([
        file_event(100, r"D:\Work\ÁPI%_-sibling\.env"),
        file_event(100, r"D:\Work\ÁPIxx\.env"),
        file_event(100, r"D:\Work\ÁPI%X\.env"),
        file_event(100, r"D:\Work\ÁPI%_extra\.env"),
    ]);
    let ids = insert(&db, events);
    let result = corpus(&db, &project_id, 100, 100);
    assert_eq!(result.total, 3);
    assert_eq!(
        result
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        ids[..3]
    );
}

#[test]
fn only_string_paths_from_file_collector_and_exact_registered_project_qualify() {
    let db = Db::open_in(&temp_dir("workspace-path-provenance")).unwrap();
    let project_id = register(&db, r"D:\work\api", 1);
    let mut events = vec![file_event(100, r"D:\work\api\.env")];
    for metadata in [
        None,
        Some(json!({})),
        Some(json!({ "path": null })),
        Some(json!({ "path": 42 })),
        Some(json!({ "path": false })),
        Some(json!({ "path": ["D:\\work\\api\\.env"] })),
        Some(json!({ "path": { "value": "D:\\work\\api\\.env" } })),
        Some(json!({ "path": "" })),
        Some(json!({ "path": ".env" })),
    ] {
        let mut invalid = file_event(100, r"D:\work\api\invalid.env");
        invalid.metadata = metadata;
        events.push(invalid);
    }
    let mut wrong_collector = file_event(100, r"D:\work\api\other.env");
    wrong_collector.source_collector = "synthetic".to_owned();
    let mut wrong_entity = file_event(100, r"D:\work\api\process.exe");
    wrong_entity.entity_type = EntityType::Process;
    wrong_entity.entity_id = ids::process(r"D:\work\api\process.exe");
    events.extend([wrong_collector, wrong_entity]);
    let ids = insert(&db, events);
    let result = corpus(&db, &project_id, 0, 200);
    assert_eq!(result.total, 1);
    assert_eq!(result.events[0].id, ids[0]);
    assert_eq!(corpus(&db, "project:unknown", 0, 200).total, 0);
    assert_eq!(corpus(&db, &project_id.to_uppercase(), 0, 200).total, 0);
}

#[test]
fn direct_related_and_path_matches_form_one_bounded_corpus_without_changing_timeline_filters() {
    let db = Db::open_in(&temp_dir("workspace-membership-union")).unwrap();
    let project_id = register(&db, r"D:\work\api", 1);
    let mut direct = file_event(100, r"D:\elsewhere\direct.env");
    direct.entity_id = project_id.clone();
    direct.entity_type = EntityType::Project;
    let mut related = file_event(200, r"D:\work\api\related.env");
    related.related_entity_ids = vec![project_id.clone(), project_id.clone()];
    let path_only = file_event(150, r"D:\work\api\path-only.env");
    let mut low = direct.clone();
    low.timestamp = 160;
    low.importance = Importance::Notable;
    let mut other_category = related.clone();
    other_category.timestamp = 170;
    other_category.category = Category::Application;
    let mut before_direct = direct.clone();
    before_direct.timestamp = 99;
    let mut after_related = related.clone();
    after_related.timestamp = 201;
    let ids = insert(
        &db,
        vec![
            direct,
            related,
            path_only,
            low,
            other_category,
            before_direct,
            after_related,
            file_event(99, r"D:\work\api\before.env"),
            file_event(201, r"D:\work\api\after.env"),
        ],
    );
    let result = corpus(&db, &project_id, 100, 200);
    assert_eq!(result.total, 5);
    assert_eq!(result.downsampled, 0);
    let mut selected = result
        .events
        .iter()
        .map(|event| event.id)
        .collect::<Vec<_>>();
    selected.sort_unstable();
    assert_eq!(selected, ids[..5]);
    // The ordinary Timeline entity selection still requires recorded IDs.
    let timeline = db
        .read(|conn| {
            event_repository::analysis_window_filtered(
                conn,
                100,
                200,
                &[Category::Configuration],
                Some(Importance::Important),
                Some(&project_id),
            )
        })
        .unwrap();
    assert_eq!(timeline.total, 2);
    assert_eq!(
        timeline
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![ids[0], ids[1]]
    );
    let reversed = corpus(&db, &project_id, 200, 100);
    assert_eq!(reversed, result);
}

#[test]
fn workspace_membership_precedes_count_and_cap_even_with_unrelated_critical_churn() {
    let db = Db::open_in(&temp_dir("workspace-membership-cap")).unwrap();
    let project_id = register(&db, r"D:\work\api", 1);
    let noise = (0..5_100)
        .map(|index| {
            let mut event = file_event(1_000 + index, &format!(r"D:\other\{index}.env"));
            event.importance = Importance::Critical;
            event
        })
        .collect();
    insert(&db, noise);
    let expected = insert(
        &db,
        vec![
            file_event(100, r"D:\work\api\.env"),
            file_event(200, r"D:\work\api\nested\.env"),
        ],
    );
    let start = Instant::now();
    let small = corpus(&db, &project_id, 0, 20_000);
    eprintln!(
        "workspace query, 5,102 stored / 2 matching: {:?}",
        start.elapsed()
    );
    assert_eq!(small.total, 2);
    assert_eq!(small.downsampled, 0);
    assert_eq!(
        small
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        expected
    );
    insert(
        &db,
        (0..4_999)
            .map(|index| file_event(1_000 + index, &format!(r"D:\work\api\{index}.env")))
            .collect(),
    );
    let start = Instant::now();
    let capped = corpus(&db, &project_id, 0, 20_000);
    eprintln!(
        "workspace query, 10,101 stored / 5,001 matching: {:?}",
        start.elapsed()
    );
    assert_eq!(capped.total, 5_001);
    assert_eq!(capped.events.len(), event_repository::MAX_ANALYSIS_EVENTS);
    assert_eq!(capped.downsampled, 1);
    assert!(capped.events.iter().all(|event| {
        event
            .event
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.get("path"))
            .and_then(Value::as_str)
            .unwrap()
            .starts_with(r"D:\work\api\")
    }));
    assert!(!capped.events.iter().any(|event| event.id == expected[0]));
}

#[test]
fn current_registration_changes_new_investigations_but_preserves_saved_evidence() {
    let db = Arc::new(Db::open_in(&temp_dir("workspace-immutable-snapshot")).unwrap());
    let now = what_changed_lib::domain::now_ms();
    let path = r"D:\work\api\.env";
    let ids = insert(&db, vec![file_event(now - 1_000, path)]);
    let project_id = ids::project(r"D:\work\api");
    let service = InvestigationService::new(Arc::clone(&db));
    let investigate = || {
        service
            .start_scoped(
                "API Port 3000 stopped".to_owned(),
                Some(InvestigationWindow::Custom {
                    from_ms: now - 2_000,
                    to_ms: now,
                }),
                Some(project_id.clone()),
            )
            .unwrap()
    };
    let absent = investigate();
    assert_eq!(
        absent.no_cause_reason,
        Some(NoCauseReason::NoRecordedEvidenceInWorkspace)
    );
    register(&db, r"D:\work\api", now);
    let present = investigate();
    assert_eq!(present.total_events_in_window, 1);
    assert_eq!(service.get(absent.id).unwrap(), absent);
    let remove_id = project_id.clone();
    db.write(move |conn| project_repository::remove(conn, &remove_id))
        .unwrap();
    assert_eq!(investigate().total_events_in_window, 0);
    assert_eq!(service.get(present.id).unwrap(), present);
    let stored = db
        .read(|conn| event_repository::by_id(conn, ids[0]))
        .unwrap()
        .unwrap();
    assert!(stored.event.related_entity_ids.is_empty());
    assert_eq!(stored.event.metadata, Some(json!({ "path": path })));
    let edges: i64 = db
        .read(|conn| {
            conn.query_row("SELECT COUNT(*) FROM entity_relationships", [], |row| {
                row.get(0)
            })
            .map_err(|source| {
                what_changed_lib::error::DbError::Sqlite {
                    operation: "check workspace membership did not create causal edges",
                    source,
                }
                .into()
            })
        })
        .unwrap();
    assert_eq!(edges, 0);
}
