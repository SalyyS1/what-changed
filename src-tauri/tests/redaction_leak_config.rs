#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use serde_json::json;
use what_changed_lib::collectors::config_diff::{self, ReadOutcome};
use what_changed_lib::privacy::{Sanitizer, REDACTED};
use what_changed_lib::storage::{
    AtomicCollectorCommit, ConfigSnapshot, ConfigSnapshotMutation, Db,
};
use what_changed_lib::testing::temp_dir;

const SECRET: &str = "phase10-super-secret-database-password-7f4c";

#[test]
fn parsed_config_secret_never_reaches_database_or_log_bytes() {
    let root = temp_dir("config-leak");
    let logs = root.join("logs");
    let logging = what_changed_lib::observability::init(&logs).unwrap();
    let source = format!("DATABASE_URL=postgres://alice:{SECRET}@localhost/app\nPORT=4000\n");
    let parsed = match config_diff::parse(Path::new(".env"), source.as_bytes()).unwrap() {
        ReadOutcome::Parsed(parsed) => parsed,
        other => panic!("expected parsed env, got {other:?}"),
    };
    assert_eq!(parsed.keys["DATABASE_URL"], REDACTED);
    tracing::info!(?parsed, "config snapshot prepared");

    let db = Db::open_in(&root).unwrap();
    let snapshot = Sanitizer::new()
        .sanitize(ConfigSnapshot {
            path: r"C:\Users\dev\project\.env".to_owned(),
            keys: json!(parsed.keys),
            captured_at: 1,
            format: parsed.format.as_str().to_owned(),
        })
        .0;
    let commit = AtomicCollectorCommit::with_config_snapshots(
        "files",
        Vec::new(),
        None,
        vec![ConfigSnapshotMutation::upsert(snapshot)],
    )
    .unwrap();
    db.commit_collector(commit).unwrap();
    db.flush().unwrap();
    db.close();
    drop(logging);

    let mut bytes = std::fs::read(&db.paths().database).unwrap();
    for companion in [db.paths().wal(), db.paths().shm()] {
        if let Ok(more) = std::fs::read(companion) {
            bytes.extend(more);
        }
    }
    let database = String::from_utf8_lossy(&bytes);
    assert!(database.contains(REDACTED));
    assert!(!database.contains(SECRET));

    let mut log_text = String::new();
    for entry in std::fs::read_dir(&logs).unwrap().flatten() {
        if let Ok(text) = std::fs::read_to_string(entry.path()) {
            log_text.push_str(&text);
        }
    }
    assert!(!log_text.contains(SECRET));
}
