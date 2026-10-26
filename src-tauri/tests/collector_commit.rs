//! Atomic collector persistence: event, graph evidence and next baseline are
//! either all durable or all absent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::mpsc::{RecvTimeoutError, TryRecvError};
use std::sync::Arc;
use std::time::Duration;

use rusqlite::Connection;
use serde_json::json;
use what_changed_lib::domain::{Category, Entity, EntityType, NormalizedEvent, RelationshipKind};
use what_changed_lib::error::{AppError, AppResult, DbError};
use what_changed_lib::privacy::{Sanitized, Sanitizer, REDACTED};
use what_changed_lib::storage::{
    baseline_repository, entity_repository, event_repository, AtomicCollectorCommit,
    AtomicObservation, Baseline, CollectorCommitReceipt, ConfigSnapshot, ConfigSnapshotMutation,
    Db, RelationshipSpec,
};
use what_changed_lib::testing::{synthetic_event, temp_dir};

const PROCESS_ID: &str = "process:0123456789abcdef";
const PORT_ID: &str = "port:tcp:127.0.0.1:3000";
const AT: i64 = 1_700_000_000_123;

fn sanitized_event_at(at: i64) -> Sanitized<NormalizedEvent> {
    let mut event = synthetic_event(1, at);
    event.category = Category::Process;
    event.entity_type = EntityType::Process;
    event.entity_id = PROCESS_ID.to_owned();
    event.entity_display_name = "local server".to_owned();
    event.source_collector = "process".to_owned();
    event.related_entity_ids = vec![PORT_ID.to_owned()];
    Sanitizer::new().sanitize(event).0
}

fn sanitized_port_at(at: i64) -> Sanitized<Entity> {
    Sanitizer::new()
        .sanitize(Entity {
            id: PORT_ID.to_owned(),
            entity_type: EntityType::Port,
            display_name: "token sk-abcdefghijklmnopqrstuvwx".to_owned(),
            first_seen: at,
            last_seen: at,
            metadata: Some(json!({ "DB_PASSWORD": "hunter2", "protocol": "tcp" })),
        })
        .0
}

fn sanitized_baseline(marker: &str) -> Sanitized<Baseline> {
    sanitized_baseline_at(marker, AT)
}

fn sanitized_baseline_at(marker: &str, captured_at: i64) -> Sanitized<Baseline> {
    Sanitizer::new()
        .sanitize(Baseline {
            collector_id: "process".to_owned(),
            entity_kind: "process_port".to_owned(),
            snapshot: json!({ "marker": marker, "API_KEY": "secret-value" }),
            captured_at,
            truncated: false,
        })
        .0
}

fn commit_with_event(marker: &str) -> AtomicCollectorCommit {
    commit_with_baseline(sanitized_baseline(marker))
}

fn commit_with_baseline(baseline: Sanitized<Baseline>) -> AtomicCollectorCommit {
    AtomicCollectorCommit::new(vec![observation_at(AT)], Some(baseline)).unwrap()
}

fn commit_without_baseline(at: i64) -> AtomicCollectorCommit {
    AtomicCollectorCommit::new(vec![observation_at(at)], None).unwrap()
}

fn observation_at(at: i64) -> AtomicObservation {
    AtomicObservation::new(
        sanitized_event_at(at),
        vec![sanitized_port_at(at)],
        vec![
            RelationshipSpec::new(PROCESS_ID, PORT_ID, RelationshipKind::ListensOn, at),
            RelationshipSpec::new(PORT_ID, PROCESS_ID, RelationshipKind::OpenedBy, at),
        ],
    )
    .unwrap()
}

fn file_observation(at: i64) -> AtomicObservation {
    let mut event = synthetic_event(1, at);
    event.category = Category::Configuration;
    event.entity_type = EntityType::File;
    event.entity_id = "file:0123456789abcdef".to_owned();
    event.entity_display_name = ".env".to_owned();
    event.source_collector = "files".to_owned();
    event.field = Some("PORT".to_owned());
    event.before = Some(json!("3000"));
    event.after = Some(json!("4000"));
    AtomicObservation::new(Sanitizer::new().sanitize(event).0, Vec::new(), Vec::new()).unwrap()
}

fn config_upsert() -> ConfigSnapshotMutation {
    let snapshot = Sanitizer::new()
        .sanitize(ConfigSnapshot {
            path: r"C:\Users\dev\api\.env".to_owned(),
            keys: json!({
                "PORT": "4000",
                "DATABASE_URL": "postgres://alice:secret@localhost/app"
            }),
            captured_at: AT,
            format: "env".to_owned(),
        })
        .0;
    ConfigSnapshotMutation::upsert(snapshot)
}

fn count(conn: &Connection, sql: &'static str) -> AppResult<i64> {
    conn.query_row(sql, [], |row| row.get(0))
        .map_err(|source| DbError::Sqlite {
            operation: "count collector commit residue",
            source,
        })
        .map_err(Into::into)
}

fn residue(db: &Db) -> (i64, i64, i64, i64) {
    db.read(|conn| {
        Ok((
            count(conn, "SELECT COUNT(*) FROM events")?,
            count(conn, "SELECT COUNT(*) FROM entities")?,
            count(conn, "SELECT COUNT(*) FROM entity_relationships")?,
            count(conn, "SELECT COUNT(*) FROM collector_baselines")?,
        ))
    })
    .unwrap()
}

#[test]
fn config_event_and_sanitized_baseline_commit_together() {
    let db = Db::open_in(&temp_dir("collector-config-atomic")).unwrap();
    let commit = AtomicCollectorCommit::with_config_snapshots(
        "files",
        vec![file_observation(AT)],
        None,
        vec![config_upsert()],
    )
    .unwrap();
    let receipt = db.commit_collector(commit).unwrap();
    assert_eq!(receipt.event_ids.len(), 1);
    let event = db
        .read(|conn| event_repository::by_id(conn, receipt.event_ids[0]))
        .unwrap()
        .unwrap();
    assert_eq!(event.event.field.as_deref(), Some("PORT"));
    let snapshot = db
        .read(|conn| {
            what_changed_lib::storage::config_snapshot_repository::get(
                conn,
                r"C:\Users\dev\api\.env",
            )
        })
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.keys["PORT"], "4000");
    assert_eq!(snapshot.keys["DATABASE_URL"], REDACTED);
}

#[test]
fn explicitly_truncated_config_baseline_is_a_valid_atomic_commit() {
    let db = Db::open_in(&temp_dir("collector-config-truncated")).unwrap();
    let snapshot = Sanitizer::new()
        .sanitize(ConfigSnapshot {
            path: r"C:\Users\dev\api\large.json".to_owned(),
            keys: json!({}),
            captured_at: AT,
            format: "json+truncated".to_owned(),
        })
        .0;
    let commit = AtomicCollectorCommit::with_config_snapshots(
        "files",
        vec![file_observation(AT)],
        None,
        vec![ConfigSnapshotMutation::upsert(snapshot)],
    )
    .unwrap();
    assert_eq!(db.commit_collector(commit).unwrap().event_ids.len(), 1);
}

#[test]
fn config_snapshot_failure_rolls_back_its_event_and_entity() {
    let db = Db::open_in(&temp_dir("collector-config-rollback")).unwrap();
    db.write(|conn| {
        conn.execute_batch(
            "CREATE TRIGGER reject_config_insert BEFORE INSERT ON config_snapshots \
             BEGIN SELECT RAISE(ABORT, 'injected config failure'); END;",
        )
        .map_err(|source| DbError::Sqlite {
            operation: "install config failure",
            source,
        })?;
        Ok(())
    })
    .unwrap();
    let commit = AtomicCollectorCommit::with_config_snapshots(
        "files",
        vec![file_observation(AT)],
        None,
        vec![config_upsert()],
    )
    .unwrap();
    assert!(db.commit_collector(commit).is_err());
    let (events, entities, _, _) = residue(&db);
    assert_eq!(events, 0);
    assert_eq!(entities, 0);
}

#[test]
fn blocking_collector_commit_rejects_writer_thread_reentrancy() {
    let db = Arc::new(Db::open_in(&temp_dir("collector-atomic-reentrant")).unwrap());
    let nested = Arc::clone(&db);

    let error = db
        .write(move |_| {
            nested
                .commit_collector(commit_with_event("must-not-run"))
                .map(|_| ())
        })
        .unwrap_err();

    assert!(matches!(
        error,
        AppError::InvalidInput {
            what: "database writer job",
            ..
        }
    ));
    assert_eq!(residue(&db), (0, 0, 0, 0));
}

#[test]
fn split_collector_commit_rejects_writer_thread_reentrancy_before_admission() {
    let db = Arc::new(Db::open_in(&temp_dir("collector-split-reentrant")).unwrap());
    let nested = Arc::clone(&db);
    let error = db
        .write(move |_| {
            nested
                .submit_collector(commit_with_event("must-not-run"))?
                .wait()
        })
        .unwrap_err();
    assert!(matches!(
        error,
        AppError::InvalidInput {
            what: "database writer job",
            ..
        }
    ));
    db.flush().unwrap();
    assert_eq!(residue(&db), (0, 0, 0, 0));
}

#[test]
fn a_success_commits_event_endpoints_bidirectional_evidence_and_baseline() {
    let db = Db::open_in(&temp_dir("collector-atomic-success")).unwrap();
    let notices = db.subscribe();

    let ticket = db.submit_collector(commit_with_event("current")).unwrap();
    let receipt = ticket.wait().unwrap();
    assert_eq!(receipt.event_ids.len(), 1);
    let event_id = receipt.event_ids[0];
    assert!(event_id > 0);

    let stored_event = db
        .read(|conn| event_repository::by_id(conn, event_id))
        .unwrap()
        .expect("event is durable");
    assert_eq!(stored_event.event.entity_id, PROCESS_ID);

    let primary = db
        .read(|conn| entity_repository::by_id(conn, PROCESS_ID))
        .unwrap()
        .expect("primary endpoint is durable");
    let port = db
        .read(|conn| entity_repository::by_id(conn, PORT_ID))
        .unwrap()
        .expect("related endpoint is durable");
    assert_eq!(primary.entity_type, EntityType::Process);
    assert_eq!(port.entity_type, EntityType::Port);
    assert_eq!(port.display_name, format!("token {REDACTED}"));
    assert_eq!(port.metadata.unwrap()["DB_PASSWORD"], json!(REDACTED));

    let process_edges = db
        .read(|conn| entity_repository::relationships_from(conn, PROCESS_ID))
        .unwrap();
    let port_edges = db
        .read(|conn| entity_repository::relationships_from(conn, PORT_ID))
        .unwrap();
    assert_eq!(process_edges.len(), 1);
    assert_eq!(process_edges[0].target_id, PORT_ID);
    assert_eq!(process_edges[0].kind, RelationshipKind::ListensOn);
    assert_eq!(process_edges[0].event_id, Some(event_id));
    assert_eq!(port_edges.len(), 1);
    assert_eq!(port_edges[0].target_id, PROCESS_ID);
    assert_eq!(port_edges[0].kind, RelationshipKind::OpenedBy);
    assert_eq!(port_edges[0].event_id, Some(event_id));

    let baseline = db
        .read(|conn| baseline_repository::load(conn, "process", "process_port"))
        .unwrap()
        .expect("baseline is durable");
    assert_eq!(baseline.snapshot["marker"], "current");
    assert_eq!(baseline.snapshot["API_KEY"], REDACTED);

    let notice = notices.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(notice.count, 1);
    assert_eq!(notice.newest_id, event_id);
    assert_eq!(notice.categories, vec![Category::Process]);
}

#[test]
fn relationship_evidence_keeps_the_newest_observation_when_an_older_cycle_arrives_late() {
    let db = Db::open_in(&temp_dir("collector-edge-newer-first")).unwrap();

    let newer = db
        .commit_collector(commit_without_baseline(AT + 100))
        .unwrap()
        .event_ids[0];
    let older = db
        .commit_collector(commit_without_baseline(AT))
        .unwrap()
        .event_ids[0];
    assert_ne!(newer, older);
    assert_eq!(residue(&db), (2, 2, 2, 0));

    let process_edge = db
        .read(|conn| entity_repository::relationships_from(conn, PROCESS_ID))
        .unwrap()
        .pop()
        .unwrap();
    let port_edge = db
        .read(|conn| entity_repository::relationships_from(conn, PORT_ID))
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(process_edge.event_id, Some(newer));
    assert_eq!(process_edge.recorded_at, AT + 100);
    assert_eq!(port_edge.event_id, Some(newer));
    assert_eq!(port_edge.recorded_at, AT + 100);
}

#[test]
fn equal_time_relationship_evidence_keeps_the_first_committed_event() {
    let db = Db::open_in(&temp_dir("collector-edge-equal-time")).unwrap();

    let first = db
        .commit_collector(commit_without_baseline(AT))
        .unwrap()
        .event_ids[0];
    let second = db
        .commit_collector(commit_without_baseline(AT))
        .unwrap()
        .event_ids[0];
    assert_ne!(first, second);
    assert_eq!(residue(&db), (2, 2, 2, 0));

    let process_edge = db
        .read(|conn| entity_repository::relationships_from(conn, PROCESS_ID))
        .unwrap()
        .pop()
        .unwrap();
    let port_edge = db
        .read(|conn| entity_repository::relationships_from(conn, PORT_ID))
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(process_edge.event_id, Some(first));
    assert_eq!(process_edge.recorded_at, AT);
    assert_eq!(port_edge.event_id, Some(first));
    assert_eq!(port_edge.recorded_at, AT);
}

#[test]
fn endpoint_edge_or_baseline_failure_rolls_back_every_table_and_acks_the_error() {
    let cases = [
        (
            "endpoint",
            "CREATE TRIGGER injected_failure BEFORE INSERT ON entities \
             WHEN NEW.id = 'port:tcp:127.0.0.1:3000' \
             BEGIN SELECT RAISE(ABORT, 'injected endpoint failure'); END;",
        ),
        (
            "edge",
            "CREATE TRIGGER injected_failure BEFORE INSERT ON entity_relationships \
             BEGIN SELECT RAISE(ABORT, 'injected edge failure'); END;",
        ),
        (
            "baseline",
            "CREATE TRIGGER injected_failure BEFORE INSERT ON collector_baselines \
             BEGIN SELECT RAISE(ABORT, 'injected baseline failure'); END;",
        ),
    ];

    for (stage, trigger) in cases {
        let db = Db::open_in(&temp_dir(&format!("collector-atomic-{stage}"))).unwrap();
        let notices = db.subscribe();
        db.write(move |conn| {
            conn.execute_batch(trigger)
                .map_err(|source| DbError::Sqlite {
                    operation: "install a collector commit fault",
                    source,
                })?;
            Ok(())
        })
        .unwrap();

        let error = db.commit_collector(commit_with_event(stage)).unwrap_err();
        assert!(matches!(error, AppError::Db(DbError::Sqlite { .. })));
        assert_eq!(residue(&db), (0, 0, 0, 0), "residue after {stage}");
        assert_eq!(
            db.failed_commit_rows().get("process"),
            Some(&1),
            "failed row telemetry after {stage}"
        );
        assert_eq!(
            db.failed_collector_cycles().get("process"),
            Some(&1),
            "failed cycle telemetry after {stage}"
        );
        assert!(db.collector_cycle_admission_drops().is_empty());
        assert_eq!(db.dropped_writes(), 0, "admission succeeded for {stage}");
        assert_eq!(notices.try_recv(), Err(TryRecvError::Empty));
    }
}

#[test]
fn a_first_baseline_needs_no_fake_event_and_publishes_no_notice() {
    let db = Db::open_in(&temp_dir("collector-atomic-baseline-only")).unwrap();
    let notices = db.subscribe();
    let commit = AtomicCollectorCommit::new(Vec::new(), Some(sanitized_baseline("first"))).unwrap();

    let receipt = db.commit_collector(commit).unwrap();
    assert!(receipt.event_ids.is_empty());
    assert_eq!(residue(&db), (0, 0, 0, 1));
    let baseline = db
        .read(|conn| baseline_repository::load(conn, "process", "process_port"))
        .unwrap()
        .unwrap();
    assert_eq!(baseline.snapshot["marker"], "first");
    assert_eq!(
        notices.recv_timeout(Duration::from_millis(50)),
        Err(RecvTimeoutError::Timeout)
    );
    assert!(db.failed_commit_rows().is_empty());
    assert!(db.failed_collector_cycles().is_empty());
    assert!(db.collector_cycle_admission_drops().is_empty());
}

#[test]
fn an_identical_equal_time_baseline_replay_is_idempotent() {
    let db = Db::open_in(&temp_dir("collector-baseline-equal-replay")).unwrap();
    let notices = db.subscribe();

    for _ in 0..2 {
        let receipt = db
            .commit_collector(
                AtomicCollectorCommit::new(Vec::new(), Some(sanitized_baseline_at("same", AT)))
                    .unwrap(),
            )
            .unwrap();
        assert!(receipt.event_ids.is_empty());
    }

    assert_eq!(residue(&db), (0, 0, 0, 1));
    assert!(db.failed_commit_rows().is_empty());
    assert!(db.failed_collector_cycles().is_empty());
    assert!(db.collector_cycle_admission_drops().is_empty());
    assert_eq!(
        notices.recv_timeout(Duration::from_millis(50)),
        Err(RecvTimeoutError::Timeout)
    );
}

#[test]
fn conflicting_equal_time_baselines_reject_in_either_submission_order() {
    for (first, second) in [("alpha", "beta"), ("beta", "alpha")] {
        let db = Db::open_in(&temp_dir(&format!(
            "collector-baseline-equal-conflict-{first}"
        )))
        .unwrap();
        db.commit_collector(
            AtomicCollectorCommit::new(Vec::new(), Some(sanitized_baseline_at(first, AT))).unwrap(),
        )
        .unwrap();
        let notices = db.subscribe();

        let error = db
            .commit_collector(commit_with_baseline(sanitized_baseline_at(second, AT)))
            .unwrap_err();
        assert!(matches!(error, AppError::InvalidInput { .. }));
        assert_eq!(residue(&db), (0, 0, 0, 1));
        let durable = db
            .read(|conn| baseline_repository::load(conn, "process", "process_port"))
            .unwrap()
            .unwrap();
        assert_eq!(durable.snapshot["marker"], first);
        assert_eq!(durable.captured_at, AT);
        assert_eq!(db.failed_commit_rows().get("process"), Some(&1));
        assert_eq!(db.failed_collector_cycles().get("process"), Some(&1));
        assert!(db.collector_cycle_admission_drops().is_empty());
        assert_eq!(db.dropped_writes(), 0);
        assert_eq!(notices.try_recv(), Err(TryRecvError::Empty));
    }
}

#[test]
fn a_failed_baseline_only_cycle_has_cycle_but_not_row_telemetry() {
    let db = Db::open_in(&temp_dir("collector-baseline-only-fault")).unwrap();
    let notices = db.subscribe();
    db.write(|conn| {
        conn.execute_batch(
            "CREATE TRIGGER injected_baseline_only_failure BEFORE INSERT ON collector_baselines \
             BEGIN SELECT RAISE(ABORT, 'injected baseline-only failure'); END;",
        )
        .map_err(|source| DbError::Sqlite {
            operation: "install a baseline-only fault",
            source,
        })?;
        Ok(())
    })
    .unwrap();

    let error = db
        .commit_collector(
            AtomicCollectorCommit::new(Vec::new(), Some(sanitized_baseline("failed"))).unwrap(),
        )
        .unwrap_err();
    assert!(matches!(error, AppError::Db(DbError::Sqlite { .. })));
    assert_eq!(residue(&db), (0, 0, 0, 0));
    assert_eq!(db.dropped_writes(), 0);
    assert!(db.failed_commit_rows().is_empty());
    assert!(db.collector_cycle_admission_drops().is_empty());
    assert_eq!(db.failed_collector_cycles().get("process"), Some(&1));
    assert_eq!(notices.try_recv(), Err(TryRecvError::Empty));
}

#[test]
fn a_stale_baseline_rejects_and_rolls_back_the_observation() {
    let db = Db::open_in(&temp_dir("collector-atomic-stale-baseline")).unwrap();
    db.commit_collector(
        AtomicCollectorCommit::new(Vec::new(), Some(sanitized_baseline_at("newer", AT + 100)))
            .unwrap(),
    )
    .unwrap();
    let notices = db.subscribe();

    let error = db
        .commit_collector(commit_with_baseline(sanitized_baseline_at("stale", AT)))
        .unwrap_err();
    assert!(matches!(error, AppError::InvalidInput { .. }));
    assert_eq!(residue(&db), (0, 0, 0, 1));
    let durable = db
        .read(|conn| baseline_repository::load(conn, "process", "process_port"))
        .unwrap()
        .unwrap();
    assert_eq!(durable.snapshot["marker"], "newer");
    assert_eq!(durable.captured_at, AT + 100);
    assert_eq!(db.failed_commit_rows().get("process"), Some(&1));
    assert_eq!(db.failed_collector_cycles().get("process"), Some(&1));
    assert!(db.collector_cycle_admission_drops().is_empty());
    assert_eq!(notices.try_recv(), Err(TryRecvError::Empty));
}

#[test]
fn a_baseline_update_failure_preserves_the_old_row_and_rolls_back_new_facts() {
    let db = Db::open_in(&temp_dir("collector-atomic-baseline-update-fault")).unwrap();
    db.commit_collector(
        AtomicCollectorCommit::new(Vec::new(), Some(sanitized_baseline("old"))).unwrap(),
    )
    .unwrap();
    db.write(|conn| {
        conn.execute_batch(
            "CREATE TRIGGER injected_update_failure BEFORE UPDATE ON collector_baselines \
             BEGIN SELECT RAISE(ABORT, 'injected baseline update failure'); END;",
        )
        .map_err(|source| DbError::Sqlite {
            operation: "install a baseline update fault",
            source,
        })?;
        Ok(())
    })
    .unwrap();

    let error = db
        .commit_collector(commit_with_baseline(sanitized_baseline_at("new", AT + 1)))
        .unwrap_err();
    assert!(matches!(error, AppError::Db(DbError::Sqlite { .. })));
    assert_eq!(residue(&db), (0, 0, 0, 1));
    let durable = db
        .read(|conn| baseline_repository::load(conn, "process", "process_port"))
        .unwrap()
        .unwrap();
    assert_eq!(durable.snapshot["marker"], "old");
    assert_eq!(durable.captured_at, AT);
    assert_eq!(db.failed_commit_rows().get("process"), Some(&1));
    assert_eq!(db.failed_collector_cycles().get("process"), Some(&1));
    assert!(db.collector_cycle_admission_drops().is_empty());
}

#[test]
fn public_constructors_require_sanitized_event_entity_and_baseline_types() {
    type ObservationBoundary = fn(
        Sanitized<NormalizedEvent>,
        Vec<Sanitized<Entity>>,
        Vec<RelationshipSpec>,
    ) -> AppResult<AtomicObservation>;
    type CommitBoundary =
        fn(Vec<AtomicObservation>, Option<Sanitized<Baseline>>) -> AppResult<AtomicCollectorCommit>;

    let _observation_boundary: ObservationBoundary = AtomicObservation::new;
    let _commit_boundary: CommitBoundary = AtomicCollectorCommit::new;
    let _entity_write_boundary: fn(&Connection, &Sanitized<Entity>) -> AppResult<()> =
        entity_repository::upsert_sanitized;
    let _receipt_shape = CollectorCommitReceipt {
        event_ids: Vec::new(),
    };
}
