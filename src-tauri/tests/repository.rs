//! The repositories through the real `Db`: writes go through the writer
//! thread, reads through the pool, and every assertion is about behaviour a
//! screen depends on — stable pages under concurrent inserts, retention
//! that keeps what it promised to keep, a baseline that survives a restart.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use serde_json::json;
use what_changed_lib::domain::{
    Category, Cursor, EventQuery, Importance, NormalizedEvent, Relationship, RelationshipKind,
    RetentionPolicy, DAY_MS,
};
use what_changed_lib::privacy::{Sanitized, Sanitizer};
use what_changed_lib::storage::{
    baseline_repository, config_snapshot_repository, entity_repository, event_repository,
    maintenance,
    search_index::{Fts5Index, SearchIndex},
    settings_repository, snapshot_repository, watched_location_repository, Baseline,
    ConfigSnapshot, Db, Snapshot,
};
use what_changed_lib::testing::{synthetic_event, temp_dir};

fn sanitize(events: Vec<NormalizedEvent>) -> Vec<Sanitized<NormalizedEvent>> {
    let sanitizer = Sanitizer::new();
    events
        .into_iter()
        .map(|event| sanitizer.sanitize(event).0)
        .collect()
}

fn corpus(count: u64, base: i64) -> Vec<Sanitized<NormalizedEvent>> {
    sanitize(
        (0..count)
            .map(|seed| synthetic_event(seed, base + seed as i64 * 1000))
            .collect(),
    )
}

#[test]
fn dense_relationship_scan_matches_keyed_lookup_with_time_and_endpoint_filters() {
    let db = Db::open_in(&temp_dir("repo-relationship-adaptive")).unwrap();
    let entity_id = |index: usize| format!("service:adaptive-{index:03}");
    let make_events = |range: std::ops::Range<usize>| {
        sanitize(
            range
                .map(|index| {
                    let mut event = synthetic_event(index as u64, index as i64);
                    event.entity_id = entity_id(index);
                    event.entity_display_name = event.entity_id.clone();
                    event
                })
                .collect(),
        )
    };
    db.insert_events(make_events(0..300)).unwrap();
    let edges = [(0, 255, 0), (100, 280, 10), (280, 281, 5), (1, 2, 11)].map(
        |(source, target, recorded_at)| Relationship {
            source_id: entity_id(source),
            target_id: entity_id(target),
            kind: RelationshipKind::Provides,
            event_id: None,
            recorded_at,
        },
    );
    db.write(move |conn| {
        for edge in &edges {
            entity_repository::insert_relationship(conn, edge)?;
        }
        Ok(())
    })
    .unwrap();

    let mut requested = (0..256).map(entity_id).collect::<Vec<_>>();
    requested.reverse();
    requested.push(requested[0].clone());
    let dense = db
        .read(|conn| entity_repository::relationships_for_entities_at(conn, &requested, 10))
        .unwrap();
    assert_eq!(dense.len(), 2, "exclude unrelated and future edges");
    assert_eq!(dense[0].recorded_at, 0);
    assert_eq!(dense[1].recorded_at, 10);

    // Adding unrelated entities changes only the adaptive access path:
    // 256/300 uses one scan, while 256/400 uses keyed endpoint probes.
    db.insert_events(make_events(300..400)).unwrap();
    let keyed = db
        .read(|conn| entity_repository::relationships_for_entities_at(conn, &requested, 10))
        .unwrap();
    assert_eq!(keyed, dense);
}

#[cfg(feature = "devtools")]
#[test]
fn skewed_dense_relationship_scan_retries_keyed_even_after_early_matches() {
    let db = Db::open_in(&temp_dir("repo-relationship-skew-fallback")).unwrap();
    let entity_id = |index: usize| format!("service:skew-{index:04}");
    let events = sanitize(
        (0..1000)
            .map(|index| {
                let mut event = synthetic_event(index as u64, index as i64);
                event.entity_id = entity_id(index);
                event.entity_display_name = event.entity_id.clone();
                event
            })
            .collect(),
    );
    db.insert_events(events).unwrap();
    let relationship = |source: usize, target: usize, recorded_at| Relationship {
        source_id: entity_id(source),
        target_id: entity_id(target),
        kind: RelationshipKind::Provides,
        event_id: None,
        recorded_at,
    };
    let requested = (0..750).map(entity_id).collect::<Vec<_>>();
    let matches = vec![relationship(100, 200, 1), relationship(201, 202, 1)];
    let excluded = (850..1000)
        .flat_map(|source| (750..850).map(move |target| relationship(source, target, 2)))
        .take(12_500)
        .collect::<Vec<_>>();
    db.write({
        let matches = matches.clone();
        move |conn| {
            for edge in matches.iter().chain(excluded.iter()) {
                entity_repository::insert_relationship(conn, edge)?;
            }
            Ok(())
        }
    })
    .unwrap();

    let before = entity_repository::dense_relationship_fallback_count();
    let first = db
        .read(|conn| entity_repository::relationships_for_entities_at(conn, &requested, 2))
        .unwrap();
    assert_eq!(first, matches);
    assert!(entity_repository::dense_relationship_fallback_count() > before);

    // The first 2,000 rows match, but the following 12,500 edges belong
    // entirely to the excluded quarter. The continuous guard must still
    // retry keyed access and return exactly the same ordered evidence.
    let early_matches = (0..40)
        .flat_map(|source| (40..90).map(move |target| relationship(source, target, 0)))
        .collect::<Vec<_>>();
    db.write({
        let early_matches = early_matches.clone();
        move |conn| {
            for edge in &early_matches {
                entity_repository::insert_relationship(conn, edge)?;
            }
            Ok(())
        }
    })
    .unwrap();
    let before = entity_repository::dense_relationship_fallback_count();
    let second = db
        .read(|conn| entity_repository::relationships_for_entities_at(conn, &requested, 2))
        .unwrap();
    let mut expected = early_matches;
    expected.extend(matches);
    assert_eq!(second, expected);
    assert!(entity_repository::dense_relationship_fallback_count() > before);
}

#[test]
fn inserted_events_come_back_with_ids_and_their_entities_recorded() {
    let db = Db::open_in(&temp_dir("repo-insert")).unwrap();
    let expected = synthetic_event(2, 3_000);
    let ids = db.insert_events(corpus(5, 1_000)).unwrap();
    assert_eq!(ids.len(), 5);
    assert!(ids.windows(2).all(|w| w[1] > w[0]));

    let found = db
        .read(|conn| event_repository::by_id(conn, ids[2]))
        .unwrap()
        .expect("the event exists");
    assert_eq!(found.id, ids[2]);
    assert_eq!(found.event.timestamp, 3_000);
    assert_eq!(found.event.metadata, expected.metadata);

    let entity = db
        .read(|conn| what_changed_lib::storage::entity_repository::by_id(conn, &expected.entity_id))
        .unwrap()
        .expect("the entity was recorded");
    assert_eq!(entity.display_name, expected.entity_display_name);
    assert_eq!(entity.first_seen, 3_000);
}

#[test]
fn event_summaries_round_trip_before_and_after_values_on_the_wire() {
    let db = Db::open_in(&temp_dir("repo-summary-delta")).unwrap();
    let mut event = synthetic_event(2, 3_000);
    event.before = Some(json!({ "port": 3000 }));
    event.after = Some(json!({ "port": 25565 }));
    db.insert_events(sanitize(vec![event])).unwrap();

    let page = db
        .read(|conn| event_repository::page(conn, &EventQuery::default()))
        .unwrap();
    let summary = page.items.first().expect("the summary exists");
    assert_eq!(summary.before, Some(json!({ "port": 3000 })));
    assert_eq!(summary.after, Some(json!({ "port": 25565 })));

    let wire = serde_json::to_value(summary).unwrap();
    assert_eq!(wire["before"], json!({ "port": 3000 }));
    assert_eq!(wire["after"], json!({ "port": 25565 }));
}

#[test]
fn keyset_pages_are_stable_and_non_overlapping_under_concurrent_inserts() {
    let db = Arc::new(Db::open_in(&temp_dir("repo-paging")).unwrap());
    let before = db.insert_events(corpus(1_000, 1_000_000)).unwrap();
    let expected: HashSet<i64> = before.iter().copied().collect();

    // A writer keeps inserting rows *inside* the paged range while the reader
    // pages backwards through it. A row that lands ahead of the cursor may or
    // may not be seen; a row may never be seen twice, and nothing that was
    // there at the start may be lost.
    let writer = {
        let db = Arc::clone(&db);
        std::thread::spawn(move || {
            for batch in 0..10u64 {
                let events = sanitize(
                    (0..50)
                        .map(|i| synthetic_event(i, 1_000_500 + (batch * 50 + i) as i64 * 1000))
                        .collect(),
                );
                db.insert_events(events).unwrap();
            }
        })
    };

    let mut seen: Vec<i64> = Vec::new();
    let mut query = EventQuery {
        from: 0,
        to: 2_000_000,
        limit: 64,
        ..EventQuery::default()
    };
    loop {
        let page = db
            .read(|conn| event_repository::page(conn, &query))
            .unwrap();
        assert!(page.items.len() <= 64);
        for pair in page.items.windows(2) {
            assert!(
                (pair[0].timestamp, pair[0].id) > (pair[1].timestamp, pair[1].id),
                "newest first, strictly"
            );
        }
        seen.extend(page.items.iter().map(|s| s.id));
        match page.next {
            Some(cursor) => query.cursor = Some(cursor),
            None => break,
        }
    }
    writer.join().unwrap();

    let unique: HashSet<i64> = seen.iter().copied().collect();
    assert_eq!(unique.len(), seen.len(), "a row was shown twice");
    assert!(
        unique.is_superset(&expected),
        "a row that existed before paging began was lost"
    );
}

#[test]
fn filters_narrow_by_category_importance_and_entity() {
    let db = Db::open_in(&temp_dir("repo-filters")).unwrap();
    db.insert_events(corpus(300, 10_000)).unwrap();

    let query = EventQuery {
        categories: vec![Category::Docker, Category::Network],
        limit: 200,
        ..EventQuery::default()
    };
    let page = db
        .read(|conn| event_repository::page(conn, &query))
        .unwrap();
    assert!(!page.items.is_empty());
    assert!(page
        .items
        .iter()
        .all(|s| matches!(s.category, Category::Docker | Category::Network)));

    let query = EventQuery {
        importance_min: Some(Importance::Important),
        limit: 200,
        ..EventQuery::default()
    };
    let page = db
        .read(|conn| event_repository::page(conn, &query))
        .unwrap();
    assert!(!page.items.is_empty());
    assert!(page
        .items
        .iter()
        .all(|s| s.importance >= Importance::Important));

    let query = EventQuery {
        entity_id: Some("service:Synthetic7".to_owned()),
        limit: 200,
        ..EventQuery::default()
    };
    let page = db
        .read(|conn| event_repository::page(conn, &query))
        .unwrap();
    assert!(!page.items.is_empty());
    assert!(page
        .items
        .iter()
        .all(|s| s.entity_id == "service:Synthetic7"));
}

#[test]
fn a_limit_over_the_cap_is_clamped_and_a_zero_limit_returns_one_row() {
    let db = Db::open_in(&temp_dir("repo-limit")).unwrap();
    db.insert_events(corpus(300, 10_000)).unwrap();
    let huge = EventQuery {
        limit: u16::MAX,
        ..EventQuery::default()
    };
    let page = db.read(|conn| event_repository::page(conn, &huge)).unwrap();
    assert_eq!(page.items.len(), 200);
    assert!(page.next.is_some());
    let zero = EventQuery {
        limit: 0,
        ..EventQuery::default()
    };
    let page = db.read(|conn| event_repository::page(conn, &zero)).unwrap();
    assert_eq!(page.items.len(), 1);
}

#[test]
fn every_filter_path_uses_an_index_rather_than_scanning_events() {
    let db = Db::open_in(&temp_dir("repo-plans")).unwrap();
    db.insert_events(corpus(50, 10_000)).unwrap();
    let plans = [
        (
            "range",
            "EXPLAIN QUERY PLAN SELECT id FROM events WHERE timestamp >= 1 AND timestamp <= 2 \
             ORDER BY timestamp DESC, id DESC LIMIT 10",
        ),
        (
            "category",
            "EXPLAIN QUERY PLAN SELECT id FROM events WHERE timestamp >= 1 AND timestamp <= 2 \
             AND category IN ('docker') ORDER BY timestamp DESC, id DESC LIMIT 10",
        ),
        (
            "entity",
            "EXPLAIN QUERY PLAN SELECT id FROM events WHERE timestamp >= 1 AND timestamp <= 2 \
             AND entity_id = 'x' ORDER BY timestamp DESC, id DESC LIMIT 10",
        ),
        (
            "importance",
            "EXPLAIN QUERY PLAN SELECT importance, COUNT(*) FROM events \
             WHERE timestamp >= 1 AND timestamp <= 2 GROUP BY importance",
        ),
        (
            "relationships",
            "EXPLAIN QUERY PLAN SELECT target_id FROM entity_relationships WHERE source_id = 'x'",
        ),
    ];
    for (name, sql) in plans {
        let plan: Vec<String> = db
            .read(|conn| {
                let mut statement = conn.prepare(sql).unwrap();
                Ok(statement
                    .query_map([], |row| row.get::<_, String>(3))
                    .unwrap()
                    .map(Result::unwrap)
                    .collect())
            })
            .unwrap();
        // A covering-index scan is an index; a bare table scan is not.
        for row in &plan {
            let bare_scan = row.starts_with("SCAN") && !row.contains("INDEX");
            assert!(!bare_scan, "{name} scans a table: {row}");
        }
        let text = plan.join(" | ");
        assert!(text.contains("INDEX"), "{name} uses no index: {text}");
    }
}

#[test]
fn counts_and_prior_occurrences_answer_the_details_drawer() {
    let db = Db::open_in(&temp_dir("repo-counts")).unwrap();
    let events: Vec<_> = (0..400)
        .map(|seed| synthetic_event(seed, 10_000 + seed as i64 * 1_000))
        .collect();
    let expected = events.iter().fold([0u64; 4], |mut counts, event| {
        let index = match event.importance {
            Importance::Informational => 0,
            Importance::Notable => 1,
            Importance::Important => 2,
            Importance::Critical => 3,
        };
        counts[index] += 1;
        counts
    });
    db.insert_events(sanitize(events)).unwrap();
    let counts = db
        .read(|conn| event_repository::counts_for_range(conn, 0, i64::MAX))
        .unwrap();
    assert_eq!(counts.total(), 400);
    assert_eq!(
        [
            counts.informational,
            counts.notable,
            counts.important,
            counts.critical,
        ],
        expected
    );
    let full_range_counts = db
        .read(|conn| event_repository::counts_for_range(conn, i64::MIN, i64::MAX))
        .unwrap();
    assert_eq!(full_range_counts, counts);

    let negative_timestamp = sanitize(vec![synthetic_event(401, -1)]);
    db.insert_events(negative_timestamp).unwrap();
    let negative_day = db
        .read(|conn| event_repository::counts_for_range(conn, -86_400_000, -1))
        .unwrap();
    assert_eq!(negative_day.total(), 1);
    let stored_day = db
        .read(|conn| {
            Ok(conn
                .query_row(
                    "SELECT day_start FROM event_importance_daily WHERE day_start = ?1",
                    [-86_400_000_i64],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap())
        })
        .unwrap();
    assert_eq!(stored_day, -86_400_000);

    // The same thing happening to the same entity, three more times after the
    // one occurrence the corpus already holds for seed 5.
    let repeats = sanitize(
        [1_000_000, 2_000_000, 3_000_000]
            .into_iter()
            .map(|ts| synthetic_event(5, ts))
            .collect(),
    );
    let ids = db.insert_events(repeats).unwrap();
    let prior = |id: i64, since: i64| {
        let event = db
            .read(|conn| event_repository::by_id(conn, id))
            .unwrap()
            .unwrap();
        db.read(|conn| {
            event_repository::prior_occurrences(
                conn,
                &event.event.signature(),
                since,
                event.event.timestamp,
            )
        })
        .unwrap()
    };
    assert_eq!(prior(ids[2], 0), 3, "the corpus row plus two repeats");
    assert_eq!(prior(ids[0], 0), 1, "only the corpus row");
    assert_eq!(prior(ids[2], 1_500_000), 1, "since narrows the window");
}

#[test]
fn batched_prior_occurrences_keep_exclusive_before_inclusive_since_and_zero_counts() {
    let db = Db::open_in(&temp_dir("repo-batched-prior-boundaries")).unwrap();
    let first = synthetic_event(5, 10);
    let mut at_twenty = first.clone();
    at_twenty.timestamp = 20;
    let mut at_twenty_again = first.clone();
    at_twenty_again.timestamp = 20;
    let mut at_thirty = first.clone();
    at_thirty.timestamp = 30;
    let unrelated = synthetic_event(6, 20);
    let ids = db
        .insert_events(sanitize(vec![
            first.clone(),
            at_twenty,
            at_twenty_again,
            at_thirty.clone(),
            unrelated.clone(),
        ]))
        .unwrap();
    assert_eq!(ids.len(), 5);

    let query = |event_id, since, before| event_repository::PriorOccurrenceQuery {
        event_id,
        signature: first.signature(),
        since,
        before,
    };
    let queries = vec![
        query(ids[3], 10, 30),
        query(ids[2], 0, 20),
        query(10_003, 20, 30),
        query(10_004, 21, 30),
        query(10_005, i64::MIN, 10),
        event_repository::PriorOccurrenceQuery {
            event_id: ids[4],
            signature: unrelated.signature(),
            since: 0,
            before: 10,
        },
    ];
    let counts = db
        .read(|conn| event_repository::prior_occurrences_batch(conn, &queries))
        .unwrap();
    assert_eq!(counts.get(&ids[3]), Some(&3));
    assert_eq!(counts.get(&ids[2]), Some(&1));
    assert_eq!(counts.get(&10_003), Some(&2));
    assert_eq!(counts.get(&10_004), Some(&0));
    assert_eq!(counts.get(&10_005), Some(&0));
    assert_eq!(counts.get(&ids[4]), Some(&0));

    // The legacy batch groups duplicate IDs *within* a batch and reports
    // their summed counts; preserve that public repository behavior.
    let duplicate = db
        .read(|conn| {
            event_repository::prior_occurrences_batch(conn, &[query(42, 10, 30), query(42, 10, 30)])
        })
        .unwrap();
    assert_eq!(duplicate.get(&42), Some(&6));

    let across_batches = (0..151)
        .map(|index| query(20_000 + index, 10, 30))
        .collect::<Vec<_>>();
    let counts = db
        .read(|conn| event_repository::prior_occurrences_batch(conn, &across_batches))
        .unwrap();
    assert_eq!(counts.len(), 151);
    assert!(counts.values().all(|count| *count == 3));
}

#[test]
fn large_sorted_recurrence_matches_individual_index_counts_and_batch_overwrites() {
    let db = Db::open_in(&temp_dir("repo-large-sorted-recurrence")).unwrap();
    let first = synthetic_event(5, -10);
    let mut observations = [i64::MIN, -10, 0, 10, 20, 20, 30, 40, i64::MAX - 1, i64::MAX]
        .into_iter()
        .map(|timestamp| {
            let mut event = first.clone();
            event.timestamp = timestamp;
            event
        })
        .collect::<Vec<_>>();
    let unrelated = synthetic_event(6, 20);
    observations.push(unrelated.clone());
    db.insert_events(sanitize(observations)).unwrap();

    let signatures = [
        first.signature(),
        unrelated.signature(),
        synthetic_event(7, 0).signature(),
    ];
    let windows = [
        (i64::MIN, i64::MAX),
        (20, 30),
        (0, 20),
        (20, 20),
        (31, 20),
        (-10, 0),
        (i64::MAX - 1, i64::MAX),
        (i64::MIN, -10),
    ];
    let queries = (0..2_100)
        .map(|index| {
            let (since, before) = windows[index % windows.len()];
            event_repository::PriorOccurrenceQuery {
                // ID 42 repeats inside and across the legacy 150-row batches.
                event_id: if index % 17 == 0 {
                    42
                } else {
                    10_000 + index as i64
                },
                signature: signatures[index % signatures.len()].clone(),
                since,
                before,
            }
        })
        .collect::<Vec<_>>();

    let expected = db
        .read(|conn| {
            let mut counts = std::collections::BTreeMap::new();
            for batch in queries.chunks(150) {
                let mut batch_counts = std::collections::BTreeMap::<i64, u64>::new();
                for query in batch {
                    let count = event_repository::prior_occurrences(
                        conn,
                        &query.signature,
                        query.since,
                        query.before,
                    )?;
                    *batch_counts.entry(query.event_id).or_default() += count;
                }
                counts.extend(batch_counts);
            }
            Ok(counts)
        })
        .unwrap();
    let actual = db
        .read(|conn| event_repository::prior_occurrences_batch(conn, &queries))
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.get(&42), expected.get(&42));
}

#[test]
fn retention_deletes_only_expired_rows_keeps_important_longer_and_spares_checkpoint_snapshots() {
    let db = Db::open_in(&temp_dir("repo-retention")).unwrap();
    let now = 200 * DAY_MS;
    let policy = RetentionPolicy::default();

    let mut events = Vec::new();
    let mut make = |age_days: i64, importance: Importance| {
        let mut event = synthetic_event(7, now - age_days * DAY_MS);
        event.importance = importance;
        event.user_friendly_summary = format!("{age_days} days old {importance}");
        events.push(event);
    };
    make(1, Importance::Informational);
    make(31, Importance::Informational);
    make(31, Importance::Important);
    make(91, Importance::Important);
    make(91, Importance::Critical);
    make(29, Importance::Notable);
    db.insert_events(sanitize(events)).unwrap();

    let checkpoint_id = db
        .write(|conn| {
            conn.execute(
                "INSERT INTO checkpoints (name, created_at) VALUES ('good', 0)",
                [],
            )
            .map_err(|e| what_changed_lib::error::DbError::Sqlite {
                operation: "seed",
                source: e,
            })?;
            Ok(conn.last_insert_rowid())
        })
        .unwrap();
    let sanitizer = Sanitizer::new();
    for (owner, age_days) in [(Some(checkpoint_id), 30), (None, 30), (None, 1)] {
        let snapshot = Snapshot {
            collector_id: "services".into(),
            entity_kind: "service".into(),
            entity_external_id: None,
            entity_name: Some("Spooler".into()),
            state: json!({ "state": "running" }),
            captured_at: now - age_days * DAY_MS,
            checkpoint_id: owner,
        };
        let (sanitized, _) = sanitizer.sanitize(snapshot);
        db.write(move |conn| snapshot_repository::insert(conn, &sanitized))
            .unwrap();
    }

    let report = maintenance::run_pass(&db, policy, now).unwrap();
    assert_eq!(
        report.events_deleted, 3,
        "31d informational, 91d important, 91d critical"
    );
    assert_eq!(
        report.snapshots_deleted, 1,
        "only the unowned 30-day snapshot"
    );

    let remaining: Vec<String> = db
        .read(|conn| {
            let mut s = conn
                .prepare("SELECT user_friendly_summary FROM events ORDER BY timestamp")
                .unwrap();
            Ok(s.query_map([], |r| r.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect())
        })
        .unwrap();
    assert_eq!(
        remaining,
        vec![
            "31 days old important",
            "29 days old notable",
            "1 days old informational"
        ]
    );
    let owned: i64 = db
        .read(|conn| {
            Ok(conn
                .query_row(
                    "SELECT COUNT(*) FROM snapshots WHERE checkpoint_id IS NOT NULL",
                    [],
                    |r| r.get(0),
                )
                .unwrap())
        })
        .unwrap();
    assert_eq!(owned, 1, "the checkpoint's snapshot survived");
}

fn summaries(db: &Db) -> Vec<String> {
    db.read(|conn| {
        let mut statement = conn
            .prepare("SELECT user_friendly_summary FROM events ORDER BY timestamp, id")
            .unwrap();
        Ok(statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect())
    })
    .unwrap()
}

#[test]
fn informational_process_events_expire_after_three_days_and_nothing_else_does() {
    let db = Db::open_in(&temp_dir("repo-process-noise")).unwrap();
    let now = 200 * DAY_MS;

    let mut events = Vec::new();
    let mut make = |label: &str, collector: &str, age_days: i64, importance: Importance| {
        let mut event = synthetic_event(7, now - age_days * DAY_MS);
        event.source_collector = collector.to_owned();
        event.importance = importance;
        event.user_friendly_summary = label.to_owned();
        events.push(event);
    };
    make(
        "process 4d informational",
        "process",
        4,
        Importance::Informational,
    );
    make(
        "process 2d informational",
        "process",
        2,
        Importance::Informational,
    );
    make("process 10d notable", "process", 10, Importance::Notable);
    make(
        "process 10d important",
        "process",
        10,
        Importance::Important,
    );
    make(
        "listener 10d informational",
        "listeners",
        10,
        Importance::Informational,
    );
    make(
        "service 10d informational",
        "services",
        10,
        Importance::Informational,
    );
    db.insert_events(sanitize(events)).unwrap();

    let report = maintenance::run_pass(&db, RetentionPolicy::default(), now).unwrap();

    assert_eq!(
        report.events_deleted, 1,
        "only the informational process event older than three days"
    );
    assert_eq!(
        summaries(&db),
        vec![
            "process 10d notable",
            "process 10d important",
            "listener 10d informational",
            "service 10d informational",
            "process 2d informational",
        ],
        "everything but the stale process noise remains, in time order"
    );
}

#[test]
fn a_users_shorter_lifetime_pulls_process_noise_in_further() {
    let db = Db::open_in(&temp_dir("repo-process-noise-short")).unwrap();
    let now = 200 * DAY_MS;
    let policy = RetentionPolicy {
        events_days: 1,
        snapshots_days: 7,
        important_days: 90,
    };
    let mut event = synthetic_event(7, now - 2 * DAY_MS);
    event.source_collector = "process".to_owned();
    event.importance = Importance::Informational;
    db.insert_events(sanitize(vec![event])).unwrap();

    let report = maintenance::run_pass(&db, policy, now).unwrap();
    assert_eq!(report.events_deleted, 1);
}

#[test]
fn the_size_cap_removes_only_the_oldest_informational_events() {
    let db = Db::open_in(&temp_dir("repo-size-cap")).unwrap();
    let now = 200 * DAY_MS;
    let filler = "x".repeat(900);

    let mut events = Vec::new();
    let mut make = |label: String, age_minutes: i64, importance: Importance| {
        let mut event = synthetic_event(7, now - age_minutes * 60_000);
        event.source_collector = "listeners".to_owned();
        event.importance = importance;
        event.user_friendly_summary = format!("{label} {filler}");
        events.push(event);
    };
    make("keep notable".to_owned(), 1_000, Importance::Notable);
    make("keep important".to_owned(), 1_000, Importance::Important);
    for minute in 1..=1_500 {
        make(
            format!("info {minute:04}"),
            minute,
            Importance::Informational,
        );
    }
    db.insert_events(sanitize(events)).unwrap();
    db.flush().unwrap();

    let live_bytes = |db: &Db| -> u64 {
        db.read(|conn| {
            let pragma = |name: &str| -> u64 {
                conn.pragma_query_value(None, name, |row| row.get::<_, i64>(0))
                    .unwrap()
                    .try_into()
                    .unwrap()
            };
            Ok((pragma("page_count") - pragma("freelist_count")) * pragma("page_size"))
        })
        .unwrap()
    };
    let before = live_bytes(&db);
    let cap = before / 2;

    let report = maintenance::run_pass_with_cap(&db, RetentionPolicy::default(), now, cap).unwrap();

    assert!(report.events_deleted > 0, "the cap removed something");
    assert!(
        report.events_deleted < 1_500,
        "a delete bounded by the overage must not remove every informational event"
    );
    let remaining = summaries(&db);
    assert!(remaining.iter().any(|s| s.starts_with("keep notable")));
    assert!(remaining.iter().any(|s| s.starts_with("keep important")));
    let informational: Vec<&String> = remaining
        .iter()
        .filter(|s| s.starts_with("info "))
        .collect();
    assert!(
        informational.iter().any(|s| s.starts_with("info 0001")),
        "the newest informational event is the last to go"
    );
    assert!(
        !informational.iter().any(|s| s.starts_with("info 1500")),
        "the oldest informational event went first"
    );
}

#[test]
fn a_database_over_its_cap_with_no_informational_events_is_left_intact() {
    let db = Db::open_in(&temp_dir("repo-size-cap-protected")).unwrap();
    let now = 200 * DAY_MS;
    let mut event = synthetic_event(7, now - DAY_MS);
    event.importance = Importance::Important;
    event.user_friendly_summary = "protected".to_owned();
    db.insert_events(sanitize(vec![event])).unwrap();

    let report = maintenance::run_pass_with_cap(&db, RetentionPolicy::default(), now, 1).unwrap();

    assert_eq!(report.events_deleted, 0);
    assert_eq!(summaries(&db), vec!["protected"]);
}

#[test]
fn a_baseline_survives_a_restart_byte_identical() {
    let dir = temp_dir("repo-baseline");
    let baseline = Baseline {
        collector_id: "services".into(),
        entity_kind: "service".into(),
        snapshot: json!({ "Spooler": { "state": "running", "start": "auto" }, "count": 301 }),
        captured_at: 123_456,
        truncated: false,
    };
    {
        let db = Db::open_in(&dir).unwrap();
        let (sanitized, _) = Sanitizer::new().sanitize(baseline.clone());
        db.write(move |conn| baseline_repository::store(conn, &sanitized))
            .unwrap();
        db.flush().unwrap();
        db.close();
    }
    let db = Db::open_in(&dir).unwrap();
    let loaded = db
        .read(|conn| baseline_repository::load(conn, "services", "service"))
        .unwrap()
        .expect("the baseline is still there");
    assert_eq!(loaded, baseline);
}

#[test]
fn an_oversized_baseline_is_replaced_by_a_marker_not_stored() {
    let db = Db::open_in(&temp_dir("repo-baseline-big")).unwrap();
    let huge: Vec<String> = (0..70_000).map(|i| format!("entry-{i:08}")).collect();
    let (sanitized, _) = Sanitizer::new().sanitize(Baseline {
        collector_id: "drivers".into(),
        entity_kind: "driver".into(),
        snapshot: json!(huge),
        captured_at: 1,
        truncated: false,
    });
    db.write(move |conn| baseline_repository::store(conn, &sanitized))
        .unwrap();
    let loaded = db
        .read(|conn| baseline_repository::load(conn, "drivers", "driver"))
        .unwrap()
        .unwrap();
    assert!(loaded.truncated);
    assert_eq!(loaded.snapshot["truncated"], true);
}

#[test]
fn config_snapshots_key_on_the_canonical_path_and_go_with_their_folder() {
    let db = Db::open_in(&temp_dir("repo-config")).unwrap();
    let sanitizer = Sanitizer::new();
    for path in [
        r"C:\Users\dev\api\.env",
        r"C:\Users\dev\api\config.json",
        r"C:\Other\.env",
    ] {
        let (snapshot, _) = sanitizer.sanitize(ConfigSnapshot {
            path: path.to_owned(),
            keys: json!({ "PORT": "3000", "DB_PASSWORD": "hunter2" }),
            captured_at: 1,
            format: "env".into(),
        });
        db.write(move |conn| config_snapshot_repository::put(conn, &snapshot))
            .unwrap();
    }
    let stored = db
        .read(|conn| config_snapshot_repository::get(conn, r"c:\users\DEV\api\.ENV"))
        .unwrap()
        .expect("found through a differently-cased path");
    assert_eq!(stored.keys["PORT"], "3000");
    assert_eq!(
        stored.keys["DB_PASSWORD"], "[REDACTED]",
        "stored already redacted"
    );

    let removed = db
        .write(|conn| config_snapshot_repository::delete_under(conn, r"C:\Users\dev\api"))
        .unwrap();
    assert_eq!(removed, 2);
    let left = db.read(config_snapshot_repository::count).unwrap();
    assert_eq!(left, 1);
}

#[test]
fn removing_a_watched_location_deletes_its_snapshots_in_the_same_transaction() {
    let db = Db::open_in(&temp_dir("repo-watched-remove")).unwrap();
    let row = db
        .write(|conn| watched_location_repository::insert(conn, r"C:\Users\dev\api", 6, 10))
        .unwrap();
    let snapshot = Sanitizer::new()
        .sanitize(ConfigSnapshot {
            path: r"C:\Users\dev\api\.env".to_owned(),
            keys: json!({ "DATABASE_URL": "postgres://user:secret@localhost/db" }),
            captured_at: 11,
            format: "env".to_owned(),
        })
        .0;
    db.write(move |conn| config_snapshot_repository::put(conn, &snapshot))
        .unwrap();

    assert!(db
        .write(move |conn| watched_location_repository::remove_with_snapshots(conn, row.id))
        .unwrap());
    assert_eq!(db.read(watched_location_repository::count).unwrap(), 0);
    assert_eq!(db.read(config_snapshot_repository::count).unwrap(), 0);
}

#[test]
fn removing_an_overlapping_parent_preserves_child_baselines_until_the_child_is_removed() {
    let db = Db::open_in(&temp_dir("repo-watched-overlap")).unwrap();
    let parent = db
        .write(|conn| watched_location_repository::insert(conn, r"C:\Users\dev\work", 6, 10))
        .unwrap();
    let child = db
        .write(|conn| watched_location_repository::insert(conn, r"C:\Users\dev\work\api", 1, 11))
        .unwrap();
    for path in [
        r"C:\Users\dev\work\api\.env",
        r"C:\Users\dev\work\api\nested\deeper\.env",
        r"C:\Users\dev\work\web\.env",
        r"C:\Users\dev\workbench\.env",
    ] {
        let snapshot = Sanitizer::new()
            .sanitize(ConfigSnapshot {
                path: path.to_owned(),
                keys: json!({ "PORT": "3000" }),
                captured_at: 12,
                format: "env".to_owned(),
            })
            .0;
        db.write(move |conn| config_snapshot_repository::put(conn, &snapshot))
            .unwrap();
    }

    assert!(db
        .write(move |conn| watched_location_repository::remove_with_snapshots(conn, parent.id))
        .unwrap());
    assert!(db
        .read(|conn| config_snapshot_repository::get(conn, r"C:\Users\dev\work\api\.env"))
        .unwrap()
        .is_some());
    assert!(db
        .read(|conn| {
            config_snapshot_repository::get(conn, r"C:\Users\dev\work\api\nested\deeper\.env")
        })
        .unwrap()
        .is_none());
    assert!(db
        .read(|conn| config_snapshot_repository::get(conn, r"C:\Users\dev\work\web\.env"))
        .unwrap()
        .is_none());
    assert!(db
        .read(|conn| config_snapshot_repository::get(conn, r"C:\Users\dev\workbench\.env"))
        .unwrap()
        .is_some());

    assert!(db
        .write(move |conn| watched_location_repository::remove_with_snapshots(conn, child.id))
        .unwrap());
    assert!(db
        .read(|conn| config_snapshot_repository::get(conn, r"C:\Users\dev\work\api\.env"))
        .unwrap()
        .is_none());
}

#[test]
fn watched_location_removal_rolls_back_when_snapshot_cleanup_fails() {
    let db = Db::open_in(&temp_dir("repo-watched-rollback")).unwrap();
    let row = db
        .write(|conn| watched_location_repository::insert(conn, r"C:\Users\dev\api", 6, 10))
        .unwrap();
    let snapshot = Sanitizer::new()
        .sanitize(ConfigSnapshot {
            path: r"C:\Users\dev\api\.env".to_owned(),
            keys: json!({ "PORT": "3000" }),
            captured_at: 11,
            format: "env".to_owned(),
        })
        .0;
    db.write(move |conn| config_snapshot_repository::put(conn, &snapshot))
        .unwrap();
    db.write(|conn| {
        conn.execute_batch(
            "CREATE TRIGGER reject_config_delete BEFORE DELETE ON config_snapshots \
             BEGIN SELECT RAISE(ABORT, 'injected cleanup failure'); END;",
        )
        .map_err(|source| what_changed_lib::error::DbError::Sqlite {
            operation: "install cleanup failure",
            source,
        })?;
        Ok(())
    })
    .unwrap();

    assert!(db
        .write(move |conn| watched_location_repository::remove_with_snapshots(conn, row.id))
        .is_err());
    assert_eq!(db.read(watched_location_repository::count).unwrap(), 1);
    assert_eq!(db.read(config_snapshot_repository::count).unwrap(), 1);
}

#[test]
fn settings_are_seeded_and_round_trip() {
    let db = Db::open_in(&temp_dir("repo-settings")).unwrap();
    let retention = db.read(settings_repository::retention).unwrap();
    assert_eq!(retention, RetentionPolicy::default());
    let collectors: serde_json::Value = db
        .read(|conn| settings_repository::get(conn, "collectors"))
        .unwrap()
        .unwrap();
    assert_eq!(collectors["docker"], false);
    assert_eq!(collectors["services"], true);

    let custom = RetentionPolicy {
        events_days: 7,
        snapshots_days: 7,
        important_days: 30,
    };
    db.write(move |conn| settings_repository::put(conn, "retention", &custom))
        .unwrap();
    assert_eq!(db.read(settings_repository::retention).unwrap(), custom);
}

#[test]
fn search_finds_ports_prefixes_and_phrases() {
    let db = Db::open_in(&temp_dir("repo-search")).unwrap();
    let mut event = synthetic_event(1, 1);
    event.entity_display_name = "tcp:0.0.0.0:3000".into();
    event.user_friendly_summary = "Port 3000 stopped listening".into();
    event.technical_summary = "listener tcp 0.0.0.0:3000 closed by api.exe".into();
    db.insert_events(sanitize(vec![event])).unwrap();

    for query in ["3000", "port", "stopped listen", "api.exe"] {
        let hits = db.read(|conn| Fts5Index.search(conn, query, 10)).unwrap();
        assert_eq!(hits.len(), 1, "query {query:?}");
    }
    let none = db.read(|conn| Fts5Index.search(conn, "nginx", 10)).unwrap();
    assert!(none.is_empty());
    let garbage = db
        .read(|conn| Fts5Index.search(conn, "\"unbalanced OR ( ", 10))
        .unwrap();
    assert!(garbage.is_empty(), "a hostile query is not a syntax error");
}

#[test]
fn batched_search_summaries_keep_rank_order_and_missing_rows_are_skipped() {
    let db = Db::open_in(&temp_dir("repo-search-summary-order")).unwrap();
    let ids = db.insert_events(corpus(3, 1_000)).unwrap();
    let requested = [ids[2], i64::MAX, ids[0], ids[2]];
    let summaries = db
        .read(|conn| event_repository::summaries_by_ids(conn, &requested))
        .unwrap();
    let found: Vec<i64> = summaries.iter().map(|event| event.id).collect();
    assert_eq!(found, vec![ids[2], ids[0], ids[2]]);
}

#[test]
fn full_text_search_ranks_by_importance_then_timestamp() {
    let db = Db::open_in(&temp_dir("repo-search-rank")).unwrap();
    let mut older_critical = synthetic_event(1, 1_000);
    older_critical.entity_display_name = "Shared search needle".into();
    older_critical.importance = Importance::Critical;
    let mut newest_informational = synthetic_event(2, 3_000);
    newest_informational.entity_display_name = "Shared search needle".into();
    newest_informational.importance = Importance::Informational;
    let mut newer_critical = synthetic_event(3, 2_000);
    newer_critical.entity_display_name = "Shared search needle".into();
    newer_critical.importance = Importance::Critical;
    let ids = db
        .insert_events(sanitize(vec![
            older_critical,
            newest_informational,
            newer_critical,
        ]))
        .unwrap();

    let hits = db
        .read(|conn| Fts5Index.search(conn, "Shared search needle", 10))
        .unwrap();
    assert_eq!(hits, vec![ids[2], ids[0], ids[1]]);
}

#[test]
fn queued_writes_publish_one_notice_per_batch_with_no_bodies() {
    let db = Db::open_in(&temp_dir("repo-notice")).unwrap();
    let notices = db.subscribe();
    db.enqueue_events(corpus(3, 1_000)).unwrap();
    db.flush().unwrap();
    let notice = notices.recv().unwrap();
    assert_eq!(notice.count, 3);
    assert!(notice.newest_id >= 3);
    assert!(!notice.categories.is_empty());
    assert_eq!(db.dropped_writes(), 0);
}

#[test]
fn ten_thousand_events_insert_in_batches_within_one_measured_run() {
    let db = Db::open_in(&temp_dir("repo-throughput")).unwrap();
    let total = 10_000u64;
    let batch = 200u64;
    let started = Instant::now();
    for start in (0..total).step_by(batch as usize) {
        let events = sanitize(
            (start..start + batch)
                .map(|seed| synthetic_event(seed, 1_000_000 + seed as i64 * 250))
                .collect(),
        );
        db.insert_events(events).unwrap();
    }
    let elapsed = started.elapsed();
    let count = db.read(event_repository::count).unwrap();
    assert_eq!(count, total);
    let per_second = total as f64 / elapsed.as_secs_f64();
    let per_batch = elapsed / (total / batch) as u32;
    println!(
        "throughput: {total} events in {elapsed:?} = {per_second:.0} events/s \
         ({per_batch:?} per batch of {batch})"
    );

    let page_started = Instant::now();
    let query = EventQuery {
        limit: 200,
        cursor: Some(Cursor {
            timestamp: 1_000_000 + 5_000 * 250,
            id: 5_000,
        }),
        ..EventQuery::default()
    };
    let page = db
        .read(|conn| event_repository::page(conn, &query))
        .unwrap();
    println!(
        "page at 5,000 deep: {} rows in {:?}",
        page.items.len(),
        page_started.elapsed()
    );
    assert_eq!(page.items.len(), 200);

    let usage = db
        .read(|conn| maintenance::storage_usage(conn, db.paths()))
        .unwrap();
    println!(
        "storage: {} bytes database, {} bytes wal, {:.0} bytes/event",
        usage.database_bytes,
        usage.wal_bytes,
        (usage.database_bytes + usage.wal_bytes) as f64 / total as f64
    );
    assert!(elapsed.as_secs() < 60, "10k inserts took {elapsed:?}");
}
