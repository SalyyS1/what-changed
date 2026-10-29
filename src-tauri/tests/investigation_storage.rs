#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use what_changed_lib::domain::{
    Category, EntityType, EventType, Importance, Relationship, RelationshipKind,
};
use what_changed_lib::privacy::{Sanitized, Sanitizer};
use what_changed_lib::storage::{entity_repository, event_repository, Db};
use what_changed_lib::testing::{synthetic_event, temp_dir};

#[test]
fn analysis_window_is_exact_bounded_and_keeps_important_evidence() {
    let db = Db::open_in(&temp_dir("analysis-window-storage")).unwrap();
    let sanitizer = Sanitizer::new();
    let mut expected_critical_summary = String::new();
    let mut expected_structural_summary = String::new();
    let mut expected_failure_summary = String::new();
    let structural_source = synthetic_event(1, 1).entity_id;
    let structural_target = synthetic_event(2, 2).entity_id;
    let events: Vec<_> = (0..5_002_u64)
        .map(|seed| {
            let mut event = synthetic_event(seed, i64::try_from(seed).unwrap());
            event.importance = Importance::Informational;
            if seed == 0 {
                event.importance = Importance::Critical;
                expected_critical_summary = event.user_friendly_summary.clone();
            } else if seed == 1 {
                expected_structural_summary = event.user_friendly_summary.clone();
            } else if seed == 2 {
                event.category = Category::Service;
                event.event_type = EventType::Failed;
                event.entity_type = EntityType::Service;
                expected_failure_summary = event.user_friendly_summary.clone();
            }
            sanitizer.sanitize(event).0
        })
        .collect();
    let mut ids = Vec::new();
    for batch in events.chunks(500) {
        ids.extend(db.insert_events(batch.to_vec()).unwrap());
    }
    db.write(move |conn| {
        entity_repository::insert_relationship(
            conn,
            &Relationship {
                source_id: structural_source,
                target_id: structural_target,
                kind: RelationshipKind::Provides,
                event_id: Some(ids[1]),
                recorded_at: 1,
            },
        )
    })
    .unwrap();

    let window = db
        .read(|conn| event_repository::analysis_window(conn, 0, 5_001))
        .unwrap();
    assert_eq!(window.total, 5_002);
    assert_eq!(window.events.len(), event_repository::MAX_ANALYSIS_EVENTS);
    assert_eq!(window.downsampled, 2);
    assert!(window
        .events
        .iter()
        .any(|event| event.event.user_friendly_summary == expected_critical_summary));
    assert!(window
        .events
        .iter()
        .any(|event| event.event.user_friendly_summary == expected_structural_summary));
    assert!(window
        .events
        .iter()
        .any(|event| event.event.user_friendly_summary == expected_failure_summary));
    assert!(window
        .events
        .windows(2)
        .all(|pair| (pair[0].event.timestamp, pair[0].id) < (pair[1].event.timestamp, pair[1].id)));

    let boundary = db
        .read(|conn| event_repository::window_boundary_context(conn, 100, 200))
        .unwrap();
    assert_eq!(boundary.coverage_from, Some(0));
    assert_eq!(boundary.coverage_to, Some(5_001));
    assert_eq!(boundary.immediately_before.map(|row| row.0), Some(99));
    assert_eq!(boundary.immediately_after.map(|row| row.0), Some(201));
}

#[test]
fn relationships_recurrence_and_entity_names_are_batched_and_time_bounded() {
    let db = Db::open_in(&temp_dir("analysis-batch-storage")).unwrap();
    let sanitizer = Sanitizer::new();
    let first = synthetic_event(1, 10);
    let second = synthetic_event(2, 20);
    let mut repeat = first.clone();
    repeat.timestamp = 30;
    let stored: Vec<Sanitized<_>> = [first.clone(), second.clone(), repeat.clone()]
        .into_iter()
        .map(|event| sanitizer.sanitize(event).0)
        .collect();
    let ids = db.insert_events(stored).unwrap();
    let relationship = Relationship {
        source_id: first.entity_id.clone(),
        target_id: second.entity_id.clone(),
        kind: RelationshipKind::Provides,
        event_id: Some(ids[1]),
        recorded_at: 20,
    };
    db.write(move |conn| entity_repository::insert_relationship(conn, &relationship))
        .unwrap();

    let entity_ids = vec![first.entity_id.clone(), second.entity_id.clone()];
    assert!(db
        .read(|conn| entity_repository::relationships_for_entities_at(conn, &entity_ids, 19))
        .unwrap()
        .is_empty());
    let relationships = db
        .read(|conn| entity_repository::relationships_for_entities_at(conn, &entity_ids, 20))
        .unwrap();
    assert_eq!(relationships.len(), 1);

    let recurrence = vec![event_repository::PriorOccurrenceQuery {
        event_id: ids[2],
        signature: repeat.signature(),
        since: 0,
        before: repeat.timestamp,
    }];
    let counts = db
        .read(|conn| event_repository::prior_occurrences_batch(conn, &recurrence))
        .unwrap();
    assert_eq!(counts.get(&ids[2]), Some(&1));

    let candidates = db
        .read(|conn| entity_repository::name_candidates(conn, &["nvidia".to_owned()], 10))
        .unwrap();
    assert!(candidates
        .iter()
        .any(|entity| entity.display_name.to_ascii_lowercase().contains("nvidia")));
}
