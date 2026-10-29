//! Phase 06 query services: clustering, details, search and dashboard summary.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use what_changed_lib::domain::{
    Category, Cursor, CursorDecodeError, EventQuery, EventType, Importance, NormalizedEvent,
};
use what_changed_lib::dto::SearchResultDto;
use what_changed_lib::error::DbError;
use what_changed_lib::privacy::{Sanitized, Sanitizer};
use what_changed_lib::services::cluster::TimelineItem;
use what_changed_lib::services::{query_service, search_service, summary_service};
use what_changed_lib::storage::Db;
use what_changed_lib::testing::{synthetic, synthetic_event, temp_dir};

#[cfg(feature = "devtools")]
#[path = "support/page_capture_assertions.rs"]
mod page_capture_assertions;
#[cfg(feature = "devtools")]
#[path = "support/page_capture_service_tests.rs"]
mod page_capture_service_tests;

fn sanitize(events: Vec<NormalizedEvent>) -> Vec<Sanitized<NormalizedEvent>> {
    let sanitizer = Sanitizer::new();
    events
        .into_iter()
        .map(|event| sanitizer.sanitize(event).0)
        .collect()
}

#[test]
fn a_docker_burst_clusters_and_the_cursor_still_follows_raw_rows() {
    let db = Db::open_in(&temp_dir("query-cluster")).unwrap();
    let events = (0..40)
        .map(|index| {
            let mut event = synthetic_event(index, 1_020_000 + index as i64 * 1_000);
            event.category = Category::Docker;
            event.entity_id = "container:api".into();
            event.entity_display_name = "api".into();
            event
        })
        .collect();
    db.insert_events(sanitize(events)).unwrap();

    let page = query_service::page(
        &db,
        EventQuery {
            limit: 100,
            ..EventQuery::default()
        },
    )
    .unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(matches!(
        &page.items[0],
        TimelineItem::Cluster {
            count: 40,
            category: Category::Docker,
            ..
        }
    ));
    assert!(page.next.is_none());
}

#[test]
fn clustered_pages_keep_the_raw_cursor_without_skipping_the_next_page() {
    let db = Db::open_in(&temp_dir("query-cluster-cursor")).unwrap();
    let events = (0..8)
        .map(|index| {
            let mut event = synthetic_event(index, 1_020_000 + index as i64 * 1_000);
            event.category = Category::Docker;
            event.entity_id = "container:api".into();
            event
        })
        .collect();
    let ids = db.insert_events(sanitize(events)).unwrap();

    let first = query_service::page(
        &db,
        EventQuery {
            limit: 5,
            ..EventQuery::default()
        },
    )
    .unwrap();
    assert_eq!(first.items.len(), 1, "five raw rows become one cluster");
    let TimelineItem::Cluster { cluster, .. } = &first.items[0] else {
        panic!("the first page should contain a cluster");
    };
    let expanded = query_service::expand_cluster(&db, cluster).unwrap();
    let expanded_ids: Vec<_> = expanded.iter().map(|event| event.id).collect();
    assert_eq!(
        expanded_ids,
        ids[3..].iter().rev().copied().collect::<Vec<_>>()
    );
    let cursor = first.next.expect("three raw rows remain");
    assert_eq!(cursor.id, ids[3]);

    let second = query_service::page(
        &db,
        EventQuery {
            limit: 5,
            cursor: Some(cursor),
            ..EventQuery::default()
        },
    )
    .unwrap();
    assert_eq!(second.items.len(), 3);
    assert!(second.next.is_none());
}

#[test]
fn cursor_encoding_is_opaque_round_trippable_and_rejects_invalid_values() {
    let cursor = Cursor {
        timestamp: 1_700_000_000_123,
        id: 42,
    };
    let encoded = cursor.encode();
    assert_eq!(Cursor::decode(&encoded).unwrap(), cursor);
    assert_eq!(
        serde_json::from_str::<Cursor>(&serde_json::to_string(&cursor).unwrap()).unwrap(),
        cursor
    );
    assert!(encoded
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_')));

    assert_eq!(Cursor::decode(""), Err(CursorDecodeError::InvalidEncoding));
    assert_eq!(
        Cursor::decode("not+a+cursor"),
        Err(CursorDecodeError::InvalidEncoding)
    );
    assert_eq!(
        Cursor::decode("AAAA"),
        Err(CursorDecodeError::InvalidLength)
    );
    assert_eq!(
        Cursor::decode(
            &Cursor {
                timestamp: -1,
                id: 1
            }
            .encode()
        ),
        Err(CursorDecodeError::InvalidParts)
    );
    assert_eq!(
        Cursor::decode(
            &Cursor {
                timestamp: 1,
                id: 0
            }
            .encode()
        ),
        Err(CursorDecodeError::InvalidParts)
    );
}

#[test]
fn details_return_related_rows_and_prior_occurrences() {
    let db = Db::open_in(&temp_dir("query-details")).unwrap();
    let mut events = Vec::new();
    for (index, timestamp) in [1_000, 2_000, 3_000].into_iter().enumerate() {
        let mut event = synthetic_event(1, timestamp);
        event.category = Category::Service;
        event.event_type = EventType::Stopped;
        event.entity_id = "service:api".into();
        event.related_entity_ids = vec!["port:tcp:0.0.0.0:3000".into()];
        event.user_friendly_summary = format!("api stopped {index}");
        events.push(event);
    }
    let mut related = synthetic_event(2, 2_500);
    related.entity_id = "port:tcp:0.0.0.0:3000".into();
    events.push(related);
    let ids = db.insert_events(sanitize(events)).unwrap();

    let details = query_service::details(&db, ids[2]).unwrap();
    assert_eq!(details.prior_occurrences, 2);
    assert!(details
        .related
        .iter()
        .any(|event| { event.entity_id == "port:tcp:0.0.0.0:3000" }));
}

#[test]
fn search_and_dashboard_summary_use_the_real_index_and_counts() {
    let db = Db::open_in(&temp_dir("query-search-summary")).unwrap();
    let mut event = synthetic_event(1, 10_000);
    event.entity_display_name = "Discord.exe".into();
    event.user_friendly_summary = "Discord stopped detecting the microphone".into();
    event.importance = Importance::Important;
    db.insert_events(sanitize(vec![event])).unwrap();
    db.write(|conn| {
        conn.execute(
            "INSERT INTO checkpoints (name, created_at) VALUES (?1, ?2)",
            ("Before microphone test", 9_000i64),
        )
        .map(|_| ())
        .map_err(|source| {
            DbError::Sqlite {
                operation: "insert query test checkpoint",
                source,
            }
            .into()
        })
    })
    .unwrap();

    let hits = search_service::search(&db, "Discord micro", 20).unwrap();
    assert_eq!(hits.len(), 1);
    let SearchResultDto::Event { event, .. } = &hits[0] else {
        panic!("event search returned a checkpoint");
    };
    assert_eq!(event.entity_display_name, "Discord.exe");

    let checkpoint_hits = search_service::search(&db, "microphone test", 20).unwrap();
    assert!(matches!(
        checkpoint_hits.as_slice(),
        [SearchResultDto::Checkpoint { checkpoint }]
            if checkpoint.name == "Before microphone test"
    ));

    let summary = summary_service::summary(&db, 0, 20_000).unwrap();
    assert_eq!(summary.total, 1);
    assert_eq!(summary.important, 1);
    assert_eq!(summary.top_important.len(), 1);
    assert_eq!(summary.status_line, "1 recent changes may need attention.");
    assert_eq!(
        summary
            .last_checkpoint
            .as_ref()
            .map(|item| item.name.as_str()),
        Some("Before microphone test")
    );
}

#[test]
fn synthetic_profiles_are_sanitized_deterministic_and_calm() {
    let events = synthetic::profile(synthetic::SyntheticProfile::Day, 42, 10_000_000);
    let informational = events
        .iter()
        .filter(|event| event.get().importance == Importance::Informational)
        .count();
    let critical = events
        .iter()
        .filter(|event| event.get().importance == Importance::Critical)
        .count();
    assert!(informational * 100 >= events.len() * 70);
    assert!(critical * 100 <= events.len() * 5);
    assert!(events
        .iter()
        .all(|event| event.get().source_collector == "synthetic"));
}

#[test]
fn every_palette_fixture_term_and_numeric_port_is_searchable() {
    let db = Db::open_in(&temp_dir("query-search-corpus")).unwrap();
    let names = ["Discord", "NVIDIA", "nginx", "DNS", "Minecraft"];
    let events = names
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let mut event = synthetic_event(index as u64, 10_000 + index as i64);
            event.entity_display_name = name.into();
            event.user_friendly_summary = format!("{name} changed");
            if name == "Minecraft" {
                event.metadata = Some(serde_json::json!({ "port": 25565 }));
                event.entity_id = "port:tcp:0.0.0.0:25565".into();
            }
            event
        })
        .collect();
    db.insert_events(sanitize(events)).unwrap();

    for term in ["Discord", "25565", "NVIDIA", "nginx", "DNS"] {
        let hits = search_service::search(&db, term, 20).unwrap();
        assert!(!hits.is_empty(), "synthetic corpus did not match {term}");
    }
}

#[test]
fn every_ui_filter_and_cursor_combination_uses_an_index() {
    let db = Db::open_in(&temp_dir("query-filter-plans")).unwrap();
    db.insert_events(sanitize(vec![synthetic_event(1, 10_000)]))
        .unwrap();

    // category, minimum importance, entity and cursor are the four optional
    // predicates the timeline UI can combine: all 16 shapes exercise the
    // actual SQL builder used by `event_repository::page`.
    for mask in 0u8..16 {
        let query = EventQuery {
            from: 0,
            to: 20_000,
            categories: (mask & 1 != 0)
                .then_some(vec![Category::Docker])
                .unwrap_or_default(),
            importance_min: (mask & 2 != 0).then_some(Importance::Notable),
            entity_id: (mask & 4 != 0).then_some("container:api".into()),
            cursor: (mask & 8 != 0).then_some(Cursor {
                timestamp: 15_000,
                id: 1,
            }),
            ..EventQuery::default()
        };
        let plan = db
            .read(|conn| {
                what_changed_lib::storage::event_repository::explain_page_plan(conn, &query)
            })
            .unwrap();
        assert!(
            plan.iter().any(|step| step.contains("INDEX")),
            "filter mask {mask:04b} uses no index: {plan:?}"
        );
        assert!(
            plan.iter()
                .all(|step| !(step.starts_with("SCAN events") && !step.contains("INDEX"))),
            "filter mask {mask:04b} scans events: {plan:?}"
        );
        #[cfg(feature = "devtools")]
        {
            use what_changed_lib::storage::page_plan_capture::{
                capture_page_plan, PagePlanVariant,
            };
            let expected = db
                .read(|conn| what_changed_lib::storage::event_repository::page(conn, &query))
                .unwrap();
            let captured = capture_page_plan(PagePlanVariant::for_query(&query), || {
                db.read(|conn| what_changed_lib::storage::event_repository::page(conn, &query))
            })
            .unwrap();
            assert_eq!(captured.value.unwrap(), expected);
            page_capture_assertions::assert_execution(
                &captured.evidence.unwrap(),
                PagePlanVariant::for_query(&query),
                expected.items.len() + usize::from(expected.next.is_some()),
            );
        }
    }
}
