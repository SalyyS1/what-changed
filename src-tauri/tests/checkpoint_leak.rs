#![allow(clippy::unwrap_used)]

use serde_json::json;
use what_changed_lib::checkpoint::prepare_capture;
use what_changed_lib::domain::{CheckpointItem, CheckpointKind};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{checkpoint_repository, Db};

#[test]
fn a_secret_in_scripted_state_never_lands_in_checkpoint_storage() {
    let secret = "sk-abcdefghijklmnopqrstuvwx";
    let directory = what_changed_lib::testing::temp_dir("checkpoint-leak");
    let db = Db::open_in(&directory).unwrap();
    let (capture, _) = prepare_capture(
        Some("Before API test".to_owned()),
        CheckpointKind::Manual,
        None,
        1,
        1,
        vec!["files".to_owned()],
        Vec::new(),
        vec![CheckpointItem {
            collector_id: "files".to_owned(),
            entity_kind: "config_file".to_owned(),
            entity_external_id: "config".to_owned(),
            entity_name: Some("settings.env".to_owned()),
            state: json!({ "API_KEY": secret }),
        }],
        &Sanitizer::new(),
    );
    db.write(move |conn| checkpoint_repository::create(conn, &capture))
        .unwrap();
    db.flush().unwrap();
    db.close();

    for name in ["events.db", "events.db-wal", "events.db-shm"] {
        let path = directory.join(name);
        if let Ok(bytes) = std::fs::read(path) {
            assert!(!String::from_utf8_lossy(&bytes).contains(secret));
        }
    }
}
