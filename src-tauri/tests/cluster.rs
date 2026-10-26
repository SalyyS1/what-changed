//! Phase 06 clustering contract: noisy runs collapse without losing members.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::domain::{Category, EventQuery};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::services::cluster::TimelineItem;
use what_changed_lib::services::query_service;
use what_changed_lib::storage::Db;
use what_changed_lib::testing::{synthetic_event, temp_dir};

#[test]
fn forty_docker_events_collapse_and_expand_to_the_exact_same_rows() {
    let db = Db::open_in(&temp_dir("cluster-contract")).expect("open cluster database");
    let sanitizer = Sanitizer::new();
    let events = (0..40)
        .map(|index| {
            let mut event = synthetic_event(index, 1_020_000 + index as i64 * 1_000);
            event.category = Category::Docker;
            event.entity_id = "container:api".into();
            event.entity_display_name = "api".into();
            sanitizer.sanitize(event).0
        })
        .collect();
    let inserted = db.insert_events(events).expect("insert cluster events");

    let page = query_service::page(
        &db,
        EventQuery {
            limit: 100,
            ..EventQuery::default()
        },
    )
    .expect("query clustered page");

    let TimelineItem::Cluster {
        cluster,
        count,
        label,
        ..
    } = &page.items[0]
    else {
        panic!("the Docker burst did not collapse");
    };
    assert_eq!(*count, 40);
    assert_eq!(label, "40 Docker-related changes");

    let expanded = query_service::expand_cluster(&db, cluster).expect("expand cluster");
    assert_eq!(expanded.len(), 40);
    let mut expanded_ids: Vec<_> = expanded.into_iter().map(|event| event.id).collect();
    let mut inserted_ids = inserted;
    expanded_ids.sort_unstable();
    inserted_ids.sort_unstable();
    assert_eq!(expanded_ids, inserted_ids);
}

#[test]
fn four_events_stay_individual_and_five_events_cluster() {
    for (rows, expected_items) in [(4, 4), (5, 1)] {
        let db = Db::open_in(&temp_dir("cluster-threshold")).expect("open cluster database");
        let sanitizer = Sanitizer::new();
        let events = (0..rows)
            .map(|index| {
                let mut event = synthetic_event(index, 1_020_000 + index as i64 * 1_000);
                event.category = Category::Docker;
                event.entity_id = "container:worker".into();
                sanitizer.sanitize(event).0
            })
            .collect();
        db.insert_events(events).expect("insert threshold events");
        let page = query_service::page(&db, EventQuery::default()).expect("query timeline");
        assert_eq!(page.items.len(), expected_items);
    }
}

#[test]
fn expansion_matches_the_coarse_entity_on_a_colon_boundary() {
    let db = Db::open_in(&temp_dir("cluster-entity-boundary")).expect("open cluster database");
    let sanitizer = Sanitizer::new();
    let mut neighbouring = synthetic_event(100, 1_020_000);
    neighbouring.category = Category::Docker;
    neighbouring.entity_id = "container:api2".into();
    let mut events = vec![sanitizer.sanitize(neighbouring).0];
    events.extend((0..5).map(|index| {
        let mut event = synthetic_event(index, 1_020_000);
        event.category = Category::Docker;
        event.entity_id = "container:api".into();
        sanitizer.sanitize(event).0
    }));
    let inserted = db.insert_events(events).expect("insert boundary events");

    let page = query_service::page(&db, EventQuery::default()).expect("query timeline");
    let TimelineItem::Cluster { cluster, count, .. } = &page.items[0] else {
        panic!("the exact api events should cluster");
    };
    assert_eq!(*count, 5);
    let expanded = query_service::expand_cluster(&db, cluster).expect("expand cluster");
    let expanded_ids: Vec<_> = expanded.iter().map(|event| event.id).collect();
    assert_eq!(
        expanded_ids,
        inserted[1..].iter().rev().copied().collect::<Vec<_>>()
    );
    assert!(expanded
        .iter()
        .all(|event| event.entity_id == "container:api"));
}

#[test]
fn expansion_respects_page_id_bounds_when_matching_rows_share_a_timestamp() {
    let db = Db::open_in(&temp_dir("cluster-same-timestamp-page-boundary"))
        .expect("open cluster database");
    let sanitizer = Sanitizer::new();
    let events = (0..8)
        .map(|index| {
            let mut event = synthetic_event(index, 1_020_000);
            event.category = Category::Docker;
            event.entity_id = "container:api".into();
            event.entity_display_name = "api".into();
            sanitizer.sanitize(event).0
        })
        .collect();
    let inserted = db.insert_events(events).expect("insert cluster events");

    let page = query_service::page(
        &db,
        EventQuery {
            limit: 5,
            ..EventQuery::default()
        },
    )
    .expect("query first timeline page");
    let TimelineItem::Cluster { cluster, count, .. } = &page.items[0] else {
        panic!("the first five matching events should cluster");
    };
    assert_eq!(*count, 5);

    let expanded = query_service::expand_cluster(&db, cluster).expect("expand cluster");
    let expanded_ids: Vec<_> = expanded.iter().map(|event| event.id).collect();
    assert_eq!(
        expanded_ids,
        inserted[3..].iter().rev().copied().collect::<Vec<_>>()
    );
}
