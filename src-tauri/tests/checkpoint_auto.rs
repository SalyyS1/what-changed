#![allow(clippy::unwrap_used)]

use serde_json::json;
use what_changed_lib::checkpoint::prepare_capture;
use what_changed_lib::domain::{CheckpointItem, CheckpointKind};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{checkpoint_repository, snapshot_repository, Db};

#[test]
fn the_eleventh_automatic_prunes_only_the_oldest_automatic() {
    let db = Db::open_in(&what_changed_lib::testing::temp_dir("checkpoint-auto-cap")).unwrap();
    create(&db, CheckpointKind::Manual, 0);
    for timestamp in 1..=11 {
        create(&db, CheckpointKind::Automatic, timestamp);
    }
    let rows = db.read(checkpoint_repository::list).unwrap();
    assert_eq!(
        rows.iter()
            .filter(|row| row.kind == CheckpointKind::Automatic)
            .count(),
        10
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row.kind == CheckpointKind::Manual)
            .count(),
        1
    );
    assert!(!rows
        .iter()
        .any(|row| row.kind == CheckpointKind::Automatic && row.created_at == 1));
}

#[test]
fn retention_never_deletes_checkpoint_owned_state() {
    let db = Db::open_in(&what_changed_lib::testing::temp_dir("checkpoint-retention")).unwrap();
    let (capture, _) = prepare_capture(
        Some("Checkpoint with durable state".to_owned()),
        CheckpointKind::Manual,
        None,
        1,
        1,
        vec!["services".to_owned()],
        Vec::new(),
        vec![CheckpointItem {
            collector_id: "services".to_owned(),
            entity_kind: "service".to_owned(),
            entity_external_id: "spooler".to_owned(),
            entity_name: Some("Print Spooler".to_owned()),
            state: json!({"state": "running"}),
        }],
        &Sanitizer::new(),
    );
    let checkpoint = db
        .write(move |conn| checkpoint_repository::create(conn, &capture))
        .unwrap();
    let deleted = db
        .write(move |conn| snapshot_repository::delete_expired(conn, i64::MAX, 400))
        .unwrap();
    assert_eq!(deleted, 0);
    assert_eq!(
        db.read(|conn| checkpoint_repository::items(conn, checkpoint.id))
            .unwrap()
            .len(),
        1
    );
}

fn create(db: &Db, kind: CheckpointKind, created_at: i64) -> what_changed_lib::domain::Checkpoint {
    let (capture, _) = prepare_capture(
        Some(format!("Checkpoint {created_at}")),
        kind,
        None,
        created_at,
        1,
        vec!["eventlog".to_owned()],
        Vec::new(),
        Vec::new(),
        &Sanitizer::new(),
    );
    db.write(move |conn| checkpoint_repository::create(conn, &capture))
        .unwrap()
}
