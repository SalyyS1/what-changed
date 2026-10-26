//! Black-box checks for the debounced watched-folder collector.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::thread;
use std::time::Duration;

use what_changed_lib::collectors::files::{catalog_descriptor, FileCollector, ID};
use what_changed_lib::collectors::listening_port_map::ListeningPortMap;
use what_changed_lib::collectors::testing::{live_context, LiveContext};
use what_changed_lib::collectors::{Collector, CollectorSnapshot, RawObservation, SnapshotContext};
use what_changed_lib::domain::EventType;
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{
    config_snapshot_repository, watched_location_repository, CollectorCommitReceipt, ConfigSnapshot,
};

fn register_root(live: &LiveContext, root: &std::path::Path) {
    let path = root.to_string_lossy().into_owned();
    live.db
        .write(move |connection| {
            watched_location_repository::insert(connection, &path, 6, 1).map(|_| ())
        })
        .unwrap();
}

fn store_config(live: &LiveContext, path: &std::path::Path, keys: serde_json::Value) {
    let snapshot = Sanitizer::new()
        .sanitize(ConfigSnapshot {
            path: path.to_string_lossy().into_owned(),
            keys,
            captured_at: 1,
            format: "env".to_owned(),
        })
        .0;
    live.db
        .write(move |connection| config_snapshot_repository::put(connection, &snapshot))
        .unwrap();
}

fn cycle(collector: &mut FileCollector, token: &mut u64) -> Vec<RawObservation> {
    let context = SnapshotContext::for_devtools(*token);
    *token += 1;
    match collector.snapshot_batch(context).unwrap() {
        CollectorSnapshot::Atomic { observations, .. } => {
            collector
                .commit_succeeded(
                    context.token(),
                    &CollectorCommitReceipt { event_ids: vec![] },
                )
                .unwrap();
            observations
        }
        CollectorSnapshot::Noop => Vec::new(),
        CollectorSnapshot::Legacy(_) => panic!("files must use atomic snapshots"),
    }
}

fn after_debounce(collector: &mut FileCollector, token: &mut u64) -> Vec<RawObservation> {
    thread::sleep(Duration::from_millis(700));
    let mut observations = Vec::new();
    for _ in 0..8 {
        observations.extend(cycle(collector, token));
        thread::sleep(Duration::from_millis(50));
    }
    observations
}

fn started(live: &LiveContext) -> (FileCollector, u64) {
    let mut collector = FileCollector::new(ListeningPortMap::new(), catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    let mut token = 1;
    for _ in 0..8 {
        let _ = cycle(&mut collector, &mut token);
    }
    (collector, token)
}

#[test]
fn editor_write_rename_and_delete_are_logical_file_events() {
    let live = live_context(ID).unwrap();
    let root = live.directory.join("watched-events");
    std::fs::create_dir(&root).unwrap();
    register_root(&live, &root);

    let config = root.join(".env");
    std::fs::write(&config, "PORT=3000\n").unwrap();
    store_config(&live, &config, serde_json::json!({ "PORT": "3000" }));
    let original = root.join("before.txt");
    std::fs::write(&original, "before").unwrap();

    let (mut collector, mut token) = started(&live);

    std::fs::write(&config, "PORT=4000\n").unwrap();
    collector.stop();
    collector.start(live.context.clone()).unwrap();
    let mut modified = Vec::new();
    for _ in 0..8 {
        modified.extend(cycle(&mut collector, &mut token));
    }
    assert!(
        modified.iter().any(|event| {
            event.kind.event_type == EventType::ConfigurationChanged
                && event.field.as_deref() == Some("PORT")
        }),
        "restart sweep observations: {modified:?}"
    );

    let renamed = root.join("after.txt");
    std::fs::rename(&original, &renamed).unwrap();
    let rename_events = after_debounce(&mut collector, &mut token);
    assert!(rename_events
        .iter()
        .any(|event| event.kind.event_type == EventType::Renamed));

    std::fs::remove_file(&config).unwrap();
    let deleted = after_debounce(&mut collector, &mut token);
    assert!(deleted
        .iter()
        .any(|event| event.kind.event_type == EventType::Deleted));
}

#[test]
fn five_thousand_ignored_dependency_files_stay_below_three_events() {
    let live = live_context(ID).unwrap();
    let root = live.directory.join("watched-storm");
    std::fs::create_dir(&root).unwrap();
    register_root(&live, &root);
    let (mut collector, mut token) = started(&live);

    let dependencies = root.join("node_modules").join("package");
    std::fs::create_dir_all(&dependencies).unwrap();
    for index in 0..5_000 {
        std::fs::write(dependencies.join(format!("file-{index}.js")), "x").unwrap();
    }

    let observations = after_debounce(&mut collector, &mut token);
    assert!(
        observations.len() <= 3,
        "ignored dependency storm emitted {} events",
        observations.len()
    );
}
