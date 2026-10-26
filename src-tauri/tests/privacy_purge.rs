//! Phase 19 purge contract tests.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Instant;

use what_changed_lib::app::{AppPaths, AppState};
use what_changed_lib::commands::privacy::privacy_summary;
use what_changed_lib::domain::{NormalizedEvent, RetentionPolicy, DAY_MS};
use what_changed_lib::error::{AppError, DbError};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{event_repository, maintenance, purge, Db};
use what_changed_lib::testing::{synthetic_event, temp_dir};

fn event(seed: u64, timestamp: i64) -> what_changed_lib::privacy::Sanitized<NormalizedEvent> {
    Sanitizer::new()
        .sanitize(synthetic_event(seed, timestamp))
        .0
}

fn assert_category_counts_match_events(db: &Db) {
    let (actual, projected) = db
        .read(|conn| {
            let actual: Vec<(String, u64)> = conn
                .prepare(
                    "SELECT category, COUNT(*) FROM events GROUP BY category ORDER BY category",
                )
                .unwrap()
                .query_map([], |row| {
                    Ok((row.get(0)?, u64::try_from(row.get::<_, i64>(1)?).unwrap()))
                })
                .unwrap()
                .map(Result::unwrap)
                .collect();
            let projected = maintenance::privacy_counts(conn)?.categories;
            Ok((actual, projected))
        })
        .unwrap();
    assert_eq!(projected, actual);
}

#[test]
fn category_purge_leaves_other_categories_and_full_purge_is_empty() {
    let db = Db::open_in(&temp_dir("phase19-purge")).unwrap();
    db.insert_events(vec![
        event(1, 1_800_000_000_000),
        event(2, 1_800_000_000_001),
    ])
    .unwrap();
    db.flush().unwrap();
    let before = db.read(event_repository::count).unwrap();
    assert_eq!(before, 2);
    assert_category_counts_match_events(&db);
    let category = synthetic_event(1, 1_800_000_000_000)
        .category
        .as_str()
        .to_owned();
    db.write(move |conn| purge::category(conn, &category))
        .unwrap();
    db.flush().unwrap();
    assert!(db.read(event_repository::count).unwrap() <= before);
    assert_category_counts_match_events(&db);
    db.write(purge::all).unwrap();
    db.flush().unwrap();
    assert_eq!(db.read(event_repository::count).unwrap(), 0);
    assert_category_counts_match_events(&db);
}

#[test]
fn retention_keeps_category_projection_exact() {
    let db = Db::open_in(&temp_dir("phase19-privacy-retention-categories")).unwrap();
    let now = 100 * DAY_MS;
    db.insert_events(vec![event(1, DAY_MS), event(2, now)])
        .unwrap();
    db.flush().unwrap();
    assert_category_counts_match_events(&db);

    let report = maintenance::run_pass(
        &db,
        RetentionPolicy {
            events_days: 1,
            snapshots_days: 1,
            important_days: 1,
        },
        now,
    )
    .unwrap();
    assert_eq!(report.events_deleted, 1);
    assert_category_counts_match_events(&db);
}

#[test]
fn full_purge_replaces_the_file_and_keeps_only_appearance_preferences() {
    let dir = temp_dir("phase19-purge-replacement");
    let db = Db::open_in(&dir).unwrap();
    db.insert_events(vec![event(9, 1_800_000_000_000)]).unwrap();
    db.flush().unwrap();
    db.write(|conn| {
        conn.execute(
            "UPDATE settings SET value = '\"dark\"' WHERE key = 'theme'",
            [],
        )
        .map_err(|source| {
            AppError::Db(DbError::Sqlite {
                operation: "seed purge theme",
                source,
            })
        })?;
        conn.execute(
            "UPDATE settings SET value = '\"2d\"' WHERE key = 'visualization'",
            [],
        )
        .map_err(|source| {
            AppError::Db(DbError::Sqlite {
                operation: "seed purge visualization",
                source,
            })
        })?;
        Ok(())
    })
    .unwrap();

    db.replace_history("\"dark\"", "\"2d\"").unwrap();
    assert_eq!(db.read(event_repository::count).unwrap(), 0);
    assert_category_counts_match_events(&db);
    let settings = db
        .read(what_changed_lib::storage::settings_repository::load)
        .unwrap();
    assert_eq!(settings.theme, what_changed_lib::domain::Theme::Dark);
    assert_eq!(
        settings.visualization,
        what_changed_lib::domain::VisualizationMode::TwoD
    );
    let quarantined: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("events.purge-old-")
        })
        .collect();
    assert!(
        quarantined.is_empty(),
        "verified old history must be removed"
    );
}

#[test]
fn full_purge_removes_fixture_bytes_from_database_and_sqlite_companions() {
    let dir = temp_dir("phase19-purge-byte-scan");
    let db = Db::open_in(&dir).unwrap();
    let marker = "privacy-byte-fixture-9f2d7a";
    let mut fixture = synthetic_event(77, 1_800_000_000_000);
    fixture.entity_display_name = marker.to_owned();
    fixture.user_friendly_summary = format!("{marker} summary");
    fixture.technical_summary = format!("{marker} technical");
    fixture.metadata = Some(serde_json::json!({"path": format!("C:\\{marker}\\history.env")}));
    db.insert_events(vec![event_from(fixture)]).unwrap();
    db.flush().unwrap();

    let paths = db.paths().clone();
    db.replace_history("\"system\"", "\"auto\"").unwrap();
    assert_eq!(db.read(event_repository::count).unwrap(), 0);

    for path in [paths.database.clone(), paths.wal(), paths.shm()] {
        if !path.exists() {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            !bytes
                .windows(marker.len())
                .any(|window| window == marker.as_bytes()),
            "purged marker remained in {}",
            path.display()
        );
    }
}

fn event_from(event: NormalizedEvent) -> what_changed_lib::privacy::Sanitized<NormalizedEvent> {
    Sanitizer::new().sanitize(event).0
}

#[test]
fn privacy_counts_stay_under_sixty_ms_at_one_hundred_thousand_events() {
    let state = AppState::open(AppPaths::under(&temp_dir("phase19-privacy-summary-100k"))).unwrap();
    let events = (0..100_000u64)
        .map(|seed| event(seed, 1_800_000_000_000 + seed as i64))
        .collect();
    state.db.insert_events(events).unwrap();
    state.db.flush().unwrap();

    // Warm the prepared statements and page cache before measuring the exact
    // synchronous body used by get_privacy_summary.
    privacy_summary(&state).unwrap();
    let mut samples = Vec::with_capacity(21);
    for _ in 0..21 {
        let started = Instant::now();
        let summary = privacy_summary(&state).unwrap();
        assert_eq!(
            summary
                .categories
                .iter()
                .map(|category| category.count)
                .sum::<u64>(),
            100_000
        );
        samples.push(started.elapsed());
    }
    samples.sort_unstable();
    let p95 = samples[19];
    eprintln!("privacy summary 100k p95={p95:?}");
    assert!(
        p95 <= std::time::Duration::from_millis(60),
        "privacy summary p95 {p95:?} exceeded 60 ms; samples={samples:?}"
    );
}
