//! Query-plan regression gate for Phase 20.
//!
//! This is intentionally ignored: it opens a seeded database and inspects the
//! actual SQLite plan, rather than pretending a unit test can predict a plan
//! after an index or SQLite upgrade.
//! It covers representative access paths, not full §61 parity: this test still
//! uses the legacy 10k profile, and most private SQL is copied from production
//! (with narrower projections), so corpus and SQL drift remain possible.

#![cfg(feature = "devtools")]
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use rusqlite::ToSql;
use serde_json::json;
use what_changed_lib::checkpoint::prepare_capture;
use what_changed_lib::domain::{CheckpointItem, CheckpointKind, EventQuery};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::search_index::match_expression;
use what_changed_lib::storage::{checkpoint_repository, event_repository, Db};
use what_changed_lib::testing::{synthetic, temp_dir};

// These are the private production predicates/orderings, copied from the
// repository modules. `explain_page_plan` below uses production SQL directly.
// Projection-only reductions (`SELECT id`/`events.*`) retain the access path.
const BROAD_SEARCH_SQL: &str = "SELECT search_hits.event_id FROM ( \
    SELECT rowid AS event_id FROM events_search_fts WHERE events_search_fts MATCH ?1 \
    UNION SELECT event_id FROM event_search_metadata \
    WHERE ?2 IS NOT NULL AND port = ?2 \
    ) AS search_hits \
    JOIN event_search_order ON event_search_order.event_id = search_hits.event_id \
    WHERE event_search_order.importance_rank = ?4 \
    ORDER BY event_search_order.timestamp DESC, event_search_order.event_id DESC LIMIT ?3";
const SUMMARY_EDGE_SQL: &str = "SELECT \
    SUM(CASE WHEN importance = 'informational' THEN 1 ELSE 0 END), \
    SUM(CASE WHEN importance = 'notable' THEN 1 ELSE 0 END), \
    SUM(CASE WHEN importance = 'important' THEN 1 ELSE 0 END), \
    SUM(CASE WHEN importance = 'critical' THEN 1 ELSE 0 END) \
 FROM events WHERE timestamp >= ?1 AND timestamp < ?2";
const SUMMARY_DAILY_SQL: &str = "SELECT \
    SUM(informational), SUM(notable), SUM(important), SUM(critical) \
 FROM event_importance_daily WHERE day_start > ?1 AND day_start < ?2";
const INVESTIGATION_RANKED_SQL: &str = "SELECT events.* FROM events \
    WHERE timestamp >= ?1 AND timestamp <= ?2 \
    ORDER BY CASE \
      WHEN EXISTS (SELECT 1 FROM entity_relationships r WHERE r.event_id = events.id) \
        OR EXISTS (SELECT 1 FROM entity_relationships r \
                   WHERE r.recorded_at <= ?2 AND r.source_id = events.entity_id) \
        OR EXISTS (SELECT 1 FROM entity_relationships r \
                   WHERE r.recorded_at <= ?2 AND r.target_id = events.entity_id) \
        OR (category IN ('service', 'system') AND event_type = 'failed') \
        OR (entity_type = 'port' AND event_type = 'closed') \
        OR (event_type = 'stopped' AND instr(COALESCE(metadata_json, ''), 'unexpected') > 0) \
        THEN 0 \
      WHEN importance = 'critical' THEN 1 \
      WHEN importance = 'important' THEN 2 \
      WHEN importance = 'notable' THEN 3 ELSE 4 END, \
      timestamp DESC, id DESC LIMIT ?3";
const RECURRENCE_BATCH_SIZE: usize = 150;
const MAX_PRINTED_PLAN_STEPS: usize = 32;

fn explain(db: &Db, sql: &str, params: &[&dyn ToSql]) -> Vec<String> {
    db.read(|conn| {
        let mut statement =
            conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                .map_err(|source| {
                    what_changed_lib::error::AppError::Db(
                        what_changed_lib::error::DbError::Sqlite {
                            operation: "prepare explain query",
                            source,
                        },
                    )
                })?;
        let rows = statement
            .query_map(params, |row| row.get::<_, String>(3))
            .map_err(|source| {
                what_changed_lib::error::AppError::Db(what_changed_lib::error::DbError::Sqlite {
                    operation: "read explain query",
                    source,
                })
            })?;
        rows.map(|row| {
            row.map_err(|source| {
                what_changed_lib::error::AppError::Db(what_changed_lib::error::DbError::Sqlite {
                    operation: "decode explain query",
                    source,
                })
            })
        })
        .collect()
    })
    .expect("read query plan")
}

fn print_plan(label: &str, details: &[String]) {
    println!("phase20 plan={label} steps={}", details.len());
    for detail in details.iter().take(MAX_PRINTED_PLAN_STEPS) {
        println!("  {detail}");
    }
    if details.len() > MAX_PRINTED_PLAN_STEPS {
        println!(
            "  ... {} more steps",
            details.len() - MAX_PRINTED_PLAN_STEPS
        );
    }
}

fn assert_searches(label: &str, details: &[String], table: &str) {
    let needle = format!("SEARCH {} USING", table.to_ascii_uppercase());
    assert!(
        details
            .iter()
            .any(|detail| detail.to_ascii_uppercase().contains(&needle)),
        "{label} did not search {table} through an index: {details:?}"
    );
}

fn assert_no_bare_events_scan(label: &str, details: &[String]) {
    assert!(
        !details.iter().any(|detail| {
            let upper = detail.to_ascii_uppercase();
            let scan = upper
                .strip_prefix("SCAN TABLE EVENTS")
                .or_else(|| upper.strip_prefix("SCAN EVENTS"));
            scan.is_some_and(|tail| {
                (tail.is_empty() || tail.starts_with(' ')) && !tail.contains("USING")
            })
        }),
        "{label} has an unindexed events scan: {details:?}"
    );
}

fn assert_relationship_probe(label: &str, details: &[String], index: &str, column: &str) {
    let index = index.to_ascii_uppercase();
    let predicate = format!("{column}=?").to_ascii_uppercase();
    assert!(
        details.iter().any(|detail| {
            let upper = detail.to_ascii_uppercase();
            upper.contains("SEARCH R USING") && upper.contains(&index) && upper.contains(&predicate)
        }),
        "{label} did not use {index} for {predicate}: {details:?}"
    );
    assert!(
        !details.iter().any(|detail| {
            let upper = detail.to_ascii_uppercase();
            upper == "SCAN R" || upper.starts_with("SCAN R ") || upper.contains("AUTOMATIC")
        }),
        "{label} scans relationships or builds an automatic index: {details:?}"
    );
}

fn recurrence_batch_plan(
    db: &Db,
    samples: &[(i64, &what_changed_lib::domain::NormalizedEvent)],
) -> Vec<String> {
    assert_eq!(samples.len(), RECURRENCE_BATCH_SIZE);
    let values = (0..samples.len())
        .map(|index| {
            let first = index * 6 + 1;
            format!(
                "(?{first}, ?{}, ?{}, ?{}, ?{}, ?{})",
                first + 1,
                first + 2,
                first + 3,
                first + 4,
                first + 5
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "WITH input(event_id, entity_id, category, event_type, since_ms, before_ms) AS \
         (VALUES {values}) \
         SELECT input.event_id, SUM((SELECT COUNT(*) FROM events e \
           WHERE e.entity_id = input.entity_id AND e.category = input.category \
             AND e.event_type = input.event_type \
             AND e.timestamp >= input.since_ms AND e.timestamp < input.before_ms)) FROM input \
         GROUP BY input.event_id ORDER BY input.event_id"
    );
    let mut parameters: Vec<Box<dyn ToSql>> = Vec::with_capacity(samples.len() * 6);
    for (id, event) in samples {
        parameters.push(Box::new(*id));
        parameters.push(Box::new(event.entity_id.clone()));
        parameters.push(Box::new(event.category.as_str().to_owned()));
        parameters.push(Box::new(event.event_type.as_str().to_owned()));
        parameters.push(Box::new(event.timestamp - 90 * 86_400_000_i64));
        parameters.push(Box::new(event.timestamp));
    }
    let refs = parameters
        .iter()
        .map(|parameter| parameter.as_ref())
        .collect::<Vec<_>>();
    explain(db, &sql, &refs)
}

fn indexed_events(db: &Db, label: &str, sql: &str, params: &[&dyn ToSql]) {
    let details = explain(db, sql, params);
    print_plan(label, &details);
    assert_searches(label, &details, "events");
    assert_no_bare_events_scan(label, &details);
}

fn checkpoint_pair(db: &Db, to: i64) -> (i64, i64) {
    let persist = |name: &str, at: i64| {
        let items = (0..3_000)
            .map(|index| CheckpointItem {
                collector_id: "services".to_owned(),
                entity_kind: "service".to_owned(),
                entity_external_id: format!("service-{index}"),
                entity_name: Some(format!("Service {index}")),
                state: json!({ "state": "running", "process_id": index }),
            })
            .collect();
        let (capture, _) = prepare_capture(
            Some(name.to_owned()),
            CheckpointKind::Manual,
            None,
            at,
            1,
            vec!["services".to_owned()],
            Vec::new(),
            items,
            &Sanitizer::new(),
        );
        db.write(move |conn| checkpoint_repository::create(conn, &capture))
            .expect("persist plan checkpoint")
            .id
    };
    (persist("Before", to - 5 * 60_000), persist("After", to))
}

#[test]
#[ignore = "Phase 20 query-plan gate; run explicitly with --include-ignored"]
fn all_eight_service_paths_have_inspectable_index_plans() {
    let db = Db::open_in(&temp_dir("phase20-query-plan")).expect("open plan database");
    let events = synthetic::profile(
        synthetic::SyntheticProfile::TenThousand,
        42,
        1_800_000_000_000,
    );
    let from = events.first().expect("non-empty corpus").get().timestamp;
    let to = events.last().expect("non-empty corpus").get().timestamp;
    let investigation_from = events[events.len() - 5_000].get().timestamp;
    let selected = events.last().expect("non-empty corpus").get();
    let entity_id = selected.entity_id.clone();
    let category = selected.category.as_str().to_owned();
    let event_type = selected.event_type.as_str().to_owned();
    let mut last_batch_ids = Vec::new();
    for batch in events.chunks(1_000) {
        last_batch_ids = db.insert_events(batch.to_vec()).expect("insert plan batch");
    }
    db.flush().expect("flush plan database");
    let details_id = *last_batch_ids.last().expect("persisted event id");
    let (left_checkpoint, right_checkpoint) = checkpoint_pair(&db, to);

    // 1–2. Both pages use the production repository's SQL builder, including
    // the real ninth-page keyset cursor rather than a guessed OFFSET query.
    let mut query = EventQuery {
        from,
        to,
        limit: 200,
        ..EventQuery::default()
    };
    let first = db
        .read(|conn| event_repository::explain_page_plan(conn, &query))
        .expect("first page plan");
    print_plan("get_events_first_page", &first);
    assert_searches("get_events_first_page", &first, "events");
    assert_no_bare_events_scan("get_events_first_page", &first);
    for page_number in 1..=9 {
        let page = db
            .read(|conn| event_repository::page(conn, &query))
            .expect("advance keyset page");
        query.cursor = Some(
            page.next
                .unwrap_or_else(|| panic!("page {page_number} ended before the tenth page")),
        );
    }
    let tenth = db
        .read(|conn| event_repository::explain_page_plan(conn, &query))
        .expect("tenth page plan");
    print_plan("get_events_page_10_keyset", &tenth);
    assert_searches("get_events_page_10_keyset", &tenth, "events");
    assert_no_bare_events_scan("get_events_page_10_keyset", &tenth);

    // 3. Summary: two timestamp-indexed edge scans, a daily projection
    // lookup, three important rows, and a tiny latest-checkpoint sort.
    let day_ms = 86_400_000_i64;
    let from_day = from.div_euclid(day_ms) * day_ms;
    let to_day = to.div_euclid(day_ms) * day_ms;
    indexed_events(
        &db,
        "summary_left_edge",
        SUMMARY_EDGE_SQL,
        &[&from, &(from_day + day_ms)],
    );
    indexed_events(&db, "summary_right_edge", SUMMARY_EDGE_SQL, &[&to_day, &to]);
    let daily = explain(&db, SUMMARY_DAILY_SQL, &[&from_day, &to_day]);
    print_plan("summary_daily_aggregate", &daily);
    assert_searches("summary_daily_aggregate", &daily, "event_importance_daily");
    indexed_events(
        &db,
        "summary_top_important",
        "SELECT id FROM events WHERE timestamp >= ?1 AND timestamp <= ?2 \
         AND importance IN ('important', 'critical') \
         ORDER BY timestamp DESC, id DESC LIMIT ?3",
        &[&from, &to, &3_i64],
    );
    print_plan(
        "summary_latest_checkpoint_small_table",
        &explain(
            &db,
            "SELECT id, name, created_at FROM checkpoints \
             ORDER BY created_at DESC, id DESC LIMIT 1",
            &[],
        ),
    );

    // 4. Search: FTS5 virtual-index lookup, optional numeric-port branch,
    // keyed order projection, and bounded id hydration. A contains-LIKE over
    // checkpoint labels is a deliberate small-table scan, not an events scan.
    let expression = match_expression("Synthetic Service").expect("broad FTS expression");
    let bucket_expression = format!("{expression} AND importance:critical");
    let numeric_port: Option<i64> = None;
    let limit = 50_i64;
    let rank = 3_i64;
    let search = explain(
        &db,
        BROAD_SEARCH_SQL,
        &[&bucket_expression, &numeric_port, &limit, &rank],
    );
    print_plan("search_events_fts_bucket", &search);
    assert!(
        search.iter().any(|detail| {
            let upper = detail.to_ascii_uppercase();
            upper.contains("EVENTS_SEARCH_FTS") && upper.contains("VIRTUAL TABLE INDEX")
        }),
        "search did not use the FTS5 virtual index: {search:?}"
    );
    assert_searches("search_events_fts_bucket", &search, "event_search_order");
    indexed_events(
        &db,
        "search_events_id_hydration",
        "SELECT id FROM events WHERE id IN (?1, ?2)",
        &[&details_id, &(details_id - 1)],
    );
    print_plan(
        "search_checkpoint_labels_contains_like",
        &explain(
            &db,
            "SELECT id, name, created_at FROM checkpoints \
             WHERE name LIKE ?1 ESCAPE '\\' \
             ORDER BY created_at DESC, id DESC LIMIT ?2",
            &[&"%Synthetic Service%", &20_i64],
        ),
    );

    // 5. Details: rowid lookup, nearby entity match, and recurrence count.
    indexed_events(
        &db,
        "get_event_details_by_id",
        "SELECT * FROM events WHERE id = ?1",
        &[&details_id],
    );
    indexed_events(
        &db,
        "get_event_details_related",
        "SELECT id FROM events WHERE id != ?1 \
         AND timestamp >= ?2 AND timestamp <= ?3 AND entity_id IN (?4) \
         ORDER BY timestamp DESC, id DESC LIMIT ?5",
        &[
            &details_id,
            &(to - 5 * 60_000),
            &(to + 5 * 60_000),
            &entity_id,
            &50_i64,
        ],
    );
    indexed_events(
        &db,
        "get_event_details_prior",
        "SELECT COUNT(*) FROM events WHERE entity_id = ?1 \
         AND timestamp >= ?2 AND timestamp < ?3 \
         AND category = ?4 AND event_type = ?5",
        &[&entity_id, &i64::MIN, &to, &category, &event_type],
    );

    // 6. Investigation: the inclusive count, 5k ranked window, boundary
    // probe and batched scalar recurrence count all start from indexed event ranges.
    indexed_events(
        &db,
        "start_investigation_window_count",
        "SELECT COUNT(*) FROM events WHERE timestamp >= ?1 AND timestamp <= ?2",
        &[&investigation_from, &to],
    );
    let ranked = explain(
        &db,
        INVESTIGATION_RANKED_SQL,
        &[&investigation_from, &to, &5_000_i64],
    );
    print_plan("start_investigation_ranked_5k", &ranked);
    assert_searches("start_investigation_ranked_5k", &ranked, "events");
    assert_no_bare_events_scan("start_investigation_ranked_5k", &ranked);
    for (index, column) in [
        ("idx_relationships_event_id", "event_id"),
        ("idx_relationships_source", "source_id"),
        ("idx_relationships_target_source", "target_id"),
    ] {
        assert_relationship_probe("start_investigation_ranked_5k", &ranked, index, column);
    }
    indexed_events(
        &db,
        "start_investigation_boundary",
        "SELECT * FROM events WHERE timestamp < ?1 \
         ORDER BY timestamp DESC, id DESC LIMIT ?2",
        &[&investigation_from, &200_i64],
    );
    indexed_events(
        &db,
        "start_investigation_boundary_context",
        "SELECT \
         (SELECT MIN(timestamp) FROM events), \
         (SELECT MAX(timestamp) FROM events), \
         (SELECT timestamp FROM events WHERE timestamp < ?1 ORDER BY timestamp DESC, id DESC LIMIT 1), \
         (SELECT id FROM events WHERE timestamp < ?1 ORDER BY timestamp DESC, id DESC LIMIT 1), \
         (SELECT timestamp FROM events WHERE timestamp > ?2 ORDER BY timestamp, id LIMIT 1), \
         (SELECT id FROM events WHERE timestamp > ?2 ORDER BY timestamp, id LIMIT 1)",
        &[&investigation_from, &to],
    );
    let samples = last_batch_ids
        .iter()
        .rev()
        .zip(events.iter().rev())
        .take(RECURRENCE_BATCH_SIZE)
        .map(|(id, event)| (*id, event.get()))
        .collect::<Vec<_>>();
    let recurrence = recurrence_batch_plan(&db, &samples);
    print_plan("start_investigation_recurrence_150", &recurrence);
    assert_no_bare_events_scan("start_investigation_recurrence_150", &recurrence);
    assert!(
        !recurrence.iter().any(|detail| {
            let upper = detail.trim_start().to_ascii_uppercase();
            upper == "SCAN E" || upper.starts_with("SCAN E ")
        }),
        "150-row recurrence scans events through alias e: {recurrence:?}"
    );
    assert!(
        recurrence.iter().any(|detail| {
            let upper = detail.to_ascii_uppercase();
            upper.contains("SEARCH E USING COVERING INDEX IDX_EVENTS_RECURRENCE")
                && upper.contains("ENTITY_ID=?")
                && upper.contains("CATEGORY=?")
                && upper.contains("EVENT_TYPE=?")
                && upper.contains("TIMESTAMP>?")
        }),
        "150-row recurrence lost selective composite index access: {recurrence:?}"
    );
    // A 5k investigation uses one prepared COUNT per exact signature, sorted
    // by signature to keep this covering index's nearby pages hot.
    let sample_event = samples[0].1;
    let sample_category = sample_event.category.as_str();
    let sample_event_type = sample_event.event_type.as_str();
    let sample_since = sample_event.timestamp - 90 * 86_400_000_i64;
    let sorted_recurrence = explain(
        &db,
        "SELECT COUNT(*) FROM events WHERE entity_id = ?1 \
         AND category = ?2 AND event_type = ?3 \
         AND timestamp >= ?4 AND timestamp < ?5",
        &[
            &sample_event.entity_id,
            &sample_category,
            &sample_event_type,
            &sample_since,
            &sample_event.timestamp,
        ],
    );
    print_plan(
        "start_investigation_recurrence_sorted_count",
        &sorted_recurrence,
    );
    assert_no_bare_events_scan(
        "start_investigation_recurrence_sorted_count",
        &sorted_recurrence,
    );
    assert!(
        sorted_recurrence.iter().any(|detail| {
            let upper = detail.to_ascii_uppercase();
            upper.contains("SEARCH EVENTS USING COVERING INDEX IDX_EVENTS_RECURRENCE")
                && upper.contains("ENTITY_ID=?")
                && upper.contains("CATEGORY=?")
                && upper.contains("EVENT_TYPE=?")
                && upper.contains("TIMESTAMP>?")
        }),
        "sorted recurrence lost selective covering index access: {sorted_recurrence:?}"
    );
    assert!(
        !sorted_recurrence
            .iter()
            .any(|detail| detail.to_ascii_uppercase().contains("USE TEMP B-TREE")),
        "sorted recurrence unexpectedly sorts SQL rows: {sorted_recurrence:?}"
    );
    print_plan(
        "start_investigation_entity_name_contains_like",
        &explain(
            &db,
            "SELECT id FROM entities WHERE LOWER(display_name) LIKE ?1 ESCAPE '\\' \
             ORDER BY last_seen DESC, id LIMIT ?2",
            &[&"%synthetic%", &100_i64],
        ),
    );
    let relationship_lookup = explain(
        &db,
        "WITH requested(id) AS (VALUES (?2)) \
         SELECT source_id, target_id, kind, event_id, recorded_at \
         FROM entity_relationships WHERE recorded_at <= ?1 \
           AND (source_id IN (SELECT id FROM requested) \
                OR target_id IN (SELECT id FROM requested)) \
         ORDER BY recorded_at, source_id, target_id, kind",
        &[&to, &entity_id],
    );
    print_plan(
        "start_investigation_relationship_lookup",
        &relationship_lookup,
    );
    for (index, column) in [
        ("idx_relationships_source", "source_id"),
        ("idx_relationships_target_source", "target_id"),
    ] {
        let needle = format!("USING INDEX {index} ({column}=?)").to_ascii_uppercase();
        assert!(
            relationship_lookup
                .iter()
                .any(|detail| detail.to_ascii_uppercase().contains(&needle)),
            "relationship lookup lost endpoint index {needle}: {relationship_lookup:?}"
        );
    }
    assert!(
        !relationship_lookup.iter().any(|detail| {
            let upper = detail.to_ascii_uppercase();
            upper == "SCAN ENTITY_RELATIONSHIPS"
                || upper.starts_with("SCAN ENTITY_RELATIONSHIPS ")
                || upper.contains("AUTOMATIC")
        }),
        "relationship lookup scans all relationships or builds an automatic index: {relationship_lookup:?}"
    );

    // The adaptive dense branch deliberately scans eligible evidence once
    // when requested ids cover most known entities. It must use the temporal
    // index rather than a bare relationship scan or a temporary sort.
    let dense_relationship_lookup = explain(
        &db,
        "SELECT source_id, target_id, kind, event_id, recorded_at \
         FROM entity_relationships WHERE recorded_at <= ?1 \
         ORDER BY recorded_at, source_id, target_id",
        &[&to],
    );
    print_plan(
        "start_investigation_dense_relationship_lookup",
        &dense_relationship_lookup,
    );
    assert!(
        dense_relationship_lookup.iter().any(|detail| {
            let upper = detail.to_ascii_uppercase();
            upper.contains("ENTITY_RELATIONSHIPS")
                && upper.contains("USING INDEX IDX_RELATIONSHIPS_RECORDED_AT")
                && upper.contains("RECORDED_AT<?")
        }),
        "dense relationship lookup lost temporal index: {dense_relationship_lookup:?}"
    );
    assert!(
        !dense_relationship_lookup.iter().any(|detail| {
            let upper = detail.to_ascii_uppercase();
            upper.starts_with("SCAN ENTITY_RELATIONSHIPS")
                || upper.contains("AUTOMATIC")
                || upper.contains("USE TEMP B-TREE")
        }),
        "dense relationship lookup added unbounded scan or sort: {dense_relationship_lookup:?}"
    );

    // 7. Comparing two 3k-item checkpoints: keyed metadata and snapshot
    // reads plus an indexed, bounded event-id window. The 6k-item structural
    // diff is in-memory and has no SQLite plan to inspect.
    let checkpoint_lookup = explain(
        &db,
        "SELECT id FROM checkpoints WHERE id = ?1",
        &[&left_checkpoint],
    );
    print_plan("compare_checkpoint_by_id", &checkpoint_lookup);
    assert_searches(
        "compare_checkpoint_by_id",
        &checkpoint_lookup,
        "checkpoints",
    );
    for id in [left_checkpoint, right_checkpoint] {
        let items = explain(
            &db,
            "SELECT collector_id, entity_kind, entity_external_id, entity_name, state_json \
             FROM snapshots WHERE checkpoint_id = ?1 \
             ORDER BY collector_id, entity_kind, entity_external_id",
            &[&id],
        );
        print_plan("compare_checkpoint_3k_items", &items);
        assert_searches("compare_checkpoint_3k_items", &items, "snapshots");
    }
    indexed_events(
        &db,
        "compare_checkpoint_event_window",
        "SELECT id FROM events WHERE timestamp >= ?1 AND timestamp <= ?2 \
         ORDER BY timestamp, id LIMIT ?3",
        &[&(to - 5 * 60_000), &to, &10_000_i64],
    );

    // 8. Privacy reads the compact, transactionally maintained category
    // projection. The screen must not rescan the growing events table.
    let privacy_categories = explain(
        &db,
        "SELECT category, event_count FROM event_category_counts ORDER BY category",
        &[],
    );
    print_plan(
        "get_privacy_summary_category_projection",
        &privacy_categories,
    );
    assert!(
        privacy_categories.iter().any(|detail| {
            let detail = detail.to_ascii_uppercase();
            detail.contains("SCAN EVENT_CATEGORY_COUNTS USING INDEX")
                || detail.contains("SCAN EVENT_CATEGORY_COUNTS USING COVERING INDEX")
        }),
        "privacy category projection lost its key index: {privacy_categories:?}"
    );
    assert!(
        privacy_categories
            .iter()
            .all(|detail| !detail.to_ascii_uppercase().contains("SCAN EVENTS")),
        "privacy category projection scans events: {privacy_categories:?}"
    );
    indexed_events(
        &db,
        "get_privacy_summary_oldest",
        "SELECT MIN(timestamp) FROM events",
        &[],
    );
    let checkpoint_items = explain(
        &db,
        "SELECT COUNT(*) FROM snapshots WHERE checkpoint_id IS NOT NULL",
        &[],
    );
    print_plan("get_privacy_summary_checkpoint_items", &checkpoint_items);
    assert_searches(
        "get_privacy_summary_checkpoint_items",
        &checkpoint_items,
        "snapshots",
    );
    for table in [
        "checkpoints",
        "investigations",
        "watched_locations",
        "config_snapshots",
    ] {
        print_plan(
            &format!("get_privacy_summary_count_{table}"),
            &explain(&db, &format!("SELECT COUNT(*) FROM {table}"), &[]),
        );
    }
}
