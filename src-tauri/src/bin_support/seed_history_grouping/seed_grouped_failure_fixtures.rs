//! Real SQLite failures in exclusive QA databases, not mocked SQL results.

use super::super::seed_grouped_fixture::OwnedFixture;
use super::super::seed_grouped_persisted_parity::{assert_exact_event_graph, count};
use super::super::*;
use what_changed_lib::storage::maintenance;

pub(in super::super) const END_AT: i64 = 1_800_000_000_000;

pub(in super::super) fn options() -> Options {
    Options {
        insertion_timings: true,
        end_at: Some(END_AT),
        ..Options::default()
    }
}

#[derive(Clone, Copy)]
pub(in super::super) enum FailureKind {
    Event,
    Graph,
    Commit,
    AlreadyRolledBack,
}

pub(in super::super) fn install(fixture: &OwnedFixture, kind: FailureKind) {
    let sql = match kind {
        FailureKind::Event => "CREATE TRIGGER fixture_event_abort BEFORE INSERT ON events \
            WHEN (SELECT COUNT(*) FROM events)=6500 BEGIN SELECT RAISE(ABORT, 'fixture event failure'); END;",
        FailureKind::Graph => "CREATE TRIGGER fixture_graph_abort BEFORE INSERT ON entity_relationships \
            WHEN (SELECT COUNT(*) FROM events)>5000 BEGIN SELECT RAISE(ABORT, 'fixture graph failure'); END;",
        FailureKind::Commit => "CREATE TABLE fixture_parent(id INTEGER PRIMARY KEY); \
            CREATE TABLE fixture_child(parent_id INTEGER REFERENCES fixture_parent(id) DEFERRABLE INITIALLY DEFERRED); \
            CREATE TRIGGER fixture_commit_failure AFTER INSERT ON events \
            WHEN (SELECT COUNT(*) FROM events)>5000 BEGIN INSERT INTO fixture_child VALUES(1); END;",
        FailureKind::AlreadyRolledBack => "CREATE TRIGGER fixture_event_rollback BEFORE INSERT ON events \
            WHEN (SELECT COUNT(*) FROM events)=6500 BEGIN SELECT RAISE(ROLLBACK, 'fixture already rolled back'); END;",
    };
    fixture
        .db()
        .write(move |conn| {
            let foreign_keys: bool = conn
                .pragma_query_value(None, "foreign_keys", |row| row.get(0))
                .unwrap();
            assert!(
                foreign_keys,
                "deferred failure uses the unchanged production foreign-key setting"
            );
            conn.execute_batch(sql).unwrap();
            Ok(())
        })
        .unwrap();
}

pub(super) fn first_committed_group(fixture: &OwnedFixture, options: &Options) {
    let selected = relationship_selection(options.events, options.seed).unwrap();
    let state = GroupState::default();
    let prepared = prepare_group(&state, options, END_AT, &selected, &[]);
    assert_eq!(prepared.units.len(), 5);
    assert_eq!(prepared.input_rows, 5_000);
    assert!(prepared.units.iter().all(|unit| unit.events.len() == 1_000));
    assert_eq!(state.relation_ordinals.len(), 3_000);
    assert!(state.relation_ordinals.iter().all(|ordinal| *ordinal == 0));
    assert!(prepared.next_ordinals.iter().sum::<u64>() > 0);
    let outcome = fixture
        .db()
        .write(move |conn| Ok(execute_group(conn, prepared, state, true)))
        .unwrap();
    assert_eq!(outcome.timings.committed_groups, 1);
    assert_eq!(outcome.value.unwrap().inserted, 5_000);
}

pub(super) fn assert_failed_second_group(
    kind: FailureKind,
    started: usize,
    body_rows: usize,
    units: usize,
) {
    let options = options();
    let expected = OwnedFixture::new();
    first_committed_group(&expected, &options);
    let actual = OwnedFixture::new();
    maintenance::register_synthetic_run(actual.db()).unwrap();
    install(&actual, kind);
    let run = seed_grouped_events(actual.db(), &options, END_AT, &mut SeedTimings::default());
    let failure = match run.value {
        Err(error) => error,
        Ok(_) => panic!("real SQLite failure did not abort grouping"),
    };
    assert!(
        failure.rollback().is_none(),
        "fixture's explicit rollback unexpectedly failed"
    );
    let operation = match failure.primary() {
        what_changed_lib::error::AppError::Db(what_changed_lib::error::DbError::Sqlite {
            operation,
            ..
        }) => *operation,
        _ => panic!("fixture did not return its actual SQLite operation error"),
    };
    assert_eq!(
        operation,
        match kind {
            FailureKind::Event | FailureKind::AlreadyRolledBack => "insert an event",
            FailureKind::Graph => "record a QA relationship",
            FailureKind::Commit => "commit a grouped synthetic seed transaction",
        }
    );
    assert_eq!(run.attribution.attempted_groups, 2);
    assert_eq!(run.attribution.committed_groups, 1);
    assert_eq!(run.attribution.failed_groups, 1);
    assert_eq!(run.attribution.attempted_input_rows, 10_000);
    assert_eq!(run.attribution.started_units, started);
    assert_eq!(run.attribution.completed_body_rows, body_rows);
    assert_eq!(run.attribution.completed_units, units);
    assert_eq!(run.attribution.committed_input_rows, 5_000);
    assert_eq!(run.progress.rows, 5_000);
    assert_eq!(run.progress.groups, 1);
    assert_eq!(run.progress.notifications, 0);
    assert!(
        run.attribution.group_total
            >= run.attribution.body.body_total + run.attribution.graph + run.attribution.commit
    );
    assert!(actual.db().write(|conn| Ok(conn.is_autocommit())).unwrap());
    expected
        .db()
        .read(|left| {
            actual.db().read(|right| {
                assert_exact_event_graph(left, right);
                assert_eq!(count(right, "events"), 5_000);
                for table in [
                    "checkpoints",
                    "snapshots",
                    "investigations",
                    "analysis_sessions",
                    "analysis_results",
                ] {
                    assert_eq!(count(right, table), 0);
                }
                if matches!(kind, FailureKind::Commit) {
                    assert_eq!(count(right, "fixture_child"), 0);
                }
                Ok(())
            })
        })
        .unwrap();
    assert_eq!(maintenance::purge_synthetic(actual.db()).unwrap(), 5_000);
    actual
        .db()
        .read(|conn| {
            assert_eq!(count(conn, "events"), 0);
            assert_eq!(count(conn, "entity_relationships"), 0);
            // Ownership is not provable after purge; possible entities are retained.
            assert!(count(conn, "entities") > 0);
            Ok(())
        })
        .unwrap();
}
