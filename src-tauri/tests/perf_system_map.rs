#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::time::{Duration, Instant};

use what_changed_lib::analysis::system_map::EntityRef;
use what_changed_lib::domain::EntityType;
use what_changed_lib::error::{AppResult, DbError};
use what_changed_lib::storage::system_map_repository::build;
use what_changed_lib::storage::Db;
use what_changed_lib::testing::temp_dir;

const RELATIONSHIPS: usize = 50_000;
const QUERY_RUNS: usize = 20;
const QUERY_BUDGET: Duration = Duration::from_millis(120);

#[test]
fn depth_three_uses_both_relationship_indexes_and_meets_the_budget() {
    let db = Db::open_in(&temp_dir("system-map-perf")).unwrap();
    db.write(|conn| {
        conn.execute_batch("BEGIN IMMEDIATE")
            .map_err(|source| sqlite("start system-map perf seed", source))?;
        let seeded = (|| -> AppResult<()> {
            conn.execute(
                "INSERT INTO entities (id, entity_type, display_name, first_seen, last_seen) \
                 VALUES ('service:focus', 'service', 'Focus', 1, 1)",
                [],
            )
            .map_err(|source| sqlite("seed system-map focus", source))?;
            let mut entity = conn
                .prepare_cached(
                    "INSERT INTO entities (id, entity_type, display_name, first_seen, last_seen) \
                     VALUES (?1, 'app', ?2, 1, 1)",
                )
                .map_err(|source| sqlite("prepare system-map entity seed", source))?;
            let mut edge = conn
                .prepare_cached(
                    "INSERT INTO entity_relationships \
                     (source_id, target_id, kind, event_id, recorded_at) \
                     VALUES (?1, ?2, 'provides', NULL, 1)",
                )
                .map_err(|source| sqlite("prepare system-map edge seed", source))?;
            // A deliberately dense star: traversal must stop at the node cap
            // plus one sentinel rather than materializing all 50k endpoints.
            for index in 0..RELATIONSHIPS {
                let id = format!("app:HKLM:perf-{index:05}");
                entity
                    .execute((&id, format!("Perf app {index:05}")))
                    .map_err(|source| sqlite("seed system-map entity", source))?;
                edge.execute((&id, "service:focus"))
                    .map_err(|source| sqlite("seed system-map relationship", source))?;
            }
            Ok(())
        })();
        if seeded.is_ok() {
            conn.execute_batch("COMMIT")
                .map_err(|source| sqlite("commit system-map perf seed", source))?;
        } else {
            let _ = conn.execute_batch("ROLLBACK");
        }
        seeded
    })
    .unwrap();

    db.read(|conn| {
        assert_uses_index(
            conn,
            "EXPLAIN QUERY PLAN SELECT target_id FROM entity_relationships WHERE source_id = 'service:focus'",
            "source_id=?",
        );
        assert_uses_index(
            conn,
            "EXPLAIN QUERY PLAN SELECT source_id FROM entity_relationships WHERE target_id = 'service:focus'",
            "idx_relationships_target",
        );
        Ok(())
    })
    .unwrap();

    let mut timings = Vec::with_capacity(QUERY_RUNS);
    for _ in 0..QUERY_RUNS {
        let started = Instant::now();
        let data = db
            .read(|conn| {
                build(
                    conn,
                    Some(EntityRef {
                        entity_id: "service:focus".to_owned(),
                        entity_type: EntityType::Service,
                    }),
                    3,
                    &[],
                )
            })
            .unwrap();
        timings.push(started.elapsed());
        assert_eq!(data.nodes.len(), 400);
        assert_eq!(data.edges.len(), 399);
        assert_eq!(data.total_nodes_available, 50_001, "exact disclosure count");
        assert!(data.truncated);
    }

    let first = timings[0];
    let mut ordered = timings.clone();
    ordered.sort_unstable();
    let median = ordered[QUERY_RUNS / 2];
    let p95 = ordered[(QUERY_RUNS * 95).div_ceil(100) - 1];
    let max = *ordered.last().expect("at least one timing");
    println!(
        "system map: 400 nodes / 399 edges from {RELATIONSHIPS} relationships; \
         {QUERY_RUNS} runs first={first:?}, median={median:?}, p95={p95:?}, max={max:?}"
    );
    assert!(
        first <= QUERY_BUDGET,
        "fresh-reader depth-three system map exceeded {QUERY_BUDGET:?}: {first:?}"
    );
    assert!(
        max <= QUERY_BUDGET,
        "depth-three system map exceeded {QUERY_BUDGET:?}; p95={p95:?}, max={max:?}"
    );
}

fn assert_uses_index(conn: &rusqlite::Connection, sql: &str, index: &str) {
    let mut statement = conn.prepare(sql).unwrap();
    let details = statement
        .query_map([], |row| row.get::<_, String>(3))
        .unwrap()
        .map(Result::unwrap)
        .collect::<Vec<_>>()
        .join(" | ");
    assert!(
        details.contains("USING") && details.contains(index),
        "query plan did not use an index on {index}: {details}"
    );
    assert!(!details.contains("SCAN entity_relationships"), "{details}");
}

fn sqlite(operation: &'static str, source: rusqlite::Error) -> what_changed_lib::error::AppError {
    DbError::Sqlite { operation, source }.into()
}
