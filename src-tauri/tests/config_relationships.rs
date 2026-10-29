//! File configuration relationships require exact, live listener evidence.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::thread;
use std::time::{Duration, Instant};

use what_changed_lib::collectors::files::{catalog_descriptor, FileCollector, ID};
use what_changed_lib::collectors::listeners::diff::TrackedSocket;
use what_changed_lib::collectors::listening_port_map::ListeningPortMap;
use what_changed_lib::collectors::service_evidence_map::{ServiceEvidence, ServiceEvidenceMap};
use what_changed_lib::collectors::testing::{live_context, LiveContext};
use what_changed_lib::collectors::{
    CancelToken, Collector, CollectorSnapshot, RawRelationshipSpec, SnapshotContext,
};
use what_changed_lib::domain::ids::Protocol;
use what_changed_lib::domain::{
    ids, Category, Confidence, ConfidenceClass, EntityType, EventType, Importance,
    InvestigationWindow, NormalizedEvent, Relationship, RelationshipKind, Severity,
};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::services::investigation_service::InvestigationService;
use what_changed_lib::storage::{
    config_snapshot_repository, entity_repository, event_repository, project_repository,
    watched_location_repository, CollectorCommitReceipt, ConfigSnapshot,
};

fn register_root(live: &LiveContext, root: &std::path::Path) {
    let path = root.to_string_lossy().into_owned();
    live.db
        .write(move |connection| {
            watched_location_repository::insert(connection, &path, 6, 1).map(|_| ())
        })
        .unwrap();
}

fn store_previous(live: &LiveContext, path: &std::path::Path) {
    let snapshot = Sanitizer::new()
        .sanitize(ConfigSnapshot {
            path: path.to_string_lossy().into_owned(),
            keys: serde_json::json!({ "PORT": "3000" }),
            captured_at: 1,
            format: "env".to_owned(),
        })
        .0;
    live.db
        .write(move |connection| config_snapshot_repository::put(connection, &snapshot))
        .unwrap();
}

fn timeline_event(
    timestamp: i64,
    category: Category,
    event_type: EventType,
    entity_type: EntityType,
    entity_id: String,
    display_name: &str,
    source_collector: &str,
) -> NormalizedEvent {
    NormalizedEvent {
        timestamp,
        category,
        event_type,
        entity_type,
        entity_id,
        entity_display_name: display_name.to_owned(),
        severity: Severity::Warning,
        importance: Importance::Important,
        source_collector: source_collector.to_owned(),
        before: None,
        after: None,
        field: None,
        metadata: None,
        related_entity_ids: Vec::new(),
        user_friendly_summary: format!("{display_name} changed"),
        technical_summary: "integration fixture".to_owned(),
        confidence: Confidence::High,
        sensitive: false,
        occurred_at_is_uncertain: false,
    }
}

fn insert_timeline_events(live: &LiveContext, events: Vec<NormalizedEvent>) -> Vec<i64> {
    live.db
        .insert_events(
            events
                .into_iter()
                .map(|event| Sanitizer::new().sanitize(event).0)
                .collect(),
        )
        .unwrap()
}

fn custom_window(from_ms: i64, to_ms: i64) -> InvestigationWindow {
    InvestigationWindow::Custom { from_ms, to_ms }
}

fn relate(
    live: &LiveContext,
    source_id: &str,
    target_id: &str,
    kind: RelationshipKind,
    event_id: Option<i64>,
    recorded_at: i64,
) {
    let relationship = Relationship {
        source_id: source_id.to_owned(),
        target_id: target_id.to_owned(),
        kind,
        event_id,
        recorded_at,
    };
    live.db
        .write(move |connection| entity_repository::insert_relationship(connection, &relationship))
        .unwrap();
}

fn register_project_process(
    live: &LiveContext,
    root: &std::path::Path,
    executable: &std::path::Path,
) {
    let root = root.to_string_lossy().into_owned();
    let executable = executable.to_string_lossy().into_owned();
    let process_id = ids::process(&executable);
    live.db
        .write(move |connection| {
            project_repository::insert(connection, &root, "my-api", 1)?;
            connection
                .execute(
                    "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, \
                 entity_display_name, severity, importance, source_collector, metadata_json, \
                 user_friendly_summary, technical_summary, confidence) \
                 VALUES (2, 'process', 'started', 'process', ?1, 'api.exe', 'info', \
                 'informational', 'process', ?2, 'api.exe started', 'process started', 'high')",
                    (
                        &process_id,
                        serde_json::json!({ "path": executable }).to_string(),
                    ),
                )
                .unwrap();
            Ok(())
        })
        .unwrap();
}

#[test]
fn exact_port_change_uses_only_live_listener_evidence() {
    let live = live_context(ID).unwrap();
    let root = live.directory.join("relationship-root");
    std::fs::create_dir(&root).unwrap();
    register_root(&live, &root);
    let config = root.join(".env");
    std::fs::write(&config, "PORT=4000\n").unwrap();
    store_previous(&live, &config);
    std::fs::write(root.join("README.txt"), "PORT=4000 is documentation").unwrap();

    let map = ListeningPortMap::new();
    let publisher = CancelToken::default();
    map.publish(
        [TrackedSocket {
            protocol: Protocol::Tcp,
            address: "127.0.0.1".to_owned(),
            port: 4_000,
            owners: BTreeSet::new(),
        }],
        &publisher,
        Instant::now(),
    );
    let mut collector = FileCollector::new(map, catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    let context = SnapshotContext::for_devtools(1);
    let CollectorSnapshot::Atomic {
        observations,
        relationships,
        ..
    } = collector.snapshot_batch(context).unwrap()
    else {
        panic!("changed config must produce an atomic snapshot");
    };
    collector
        .commit_succeeded(
            context.token(),
            &CollectorCommitReceipt { event_ids: vec![] },
        )
        .unwrap();

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].field.as_deref(), Some("PORT"));
    assert_eq!(relationships.len(), 1);
    assert_exact_config_edge(&relationships[0], 0, 4_000);
}

fn assert_exact_config_edge(edge: &RawRelationshipSpec, evidence: usize, port: u16) {
    assert_eq!(edge.evidence_observation, evidence);
    assert_eq!(edge.kind, RelationshipKind::ConfigOf);
    assert!(matches!(
        edge.target.identity,
        what_changed_lib::collectors::EntityIdentity::Port {
            protocol: Protocol::Tcp,
            ref address,
            port: value,
        } if address == "127.0.0.1" && value == port
    ));
}

#[test]
fn config_under_a_service_binary_root_gets_one_service_edge() {
    let live = live_context(ID).unwrap();
    let root = live.directory.join("service-root");
    std::fs::create_dir_all(root.join("bin")).unwrap();
    register_root(&live, &root);
    let config = root.join(".env");
    std::fs::write(&config, "PORT=3000\nMODE=production\n").unwrap();
    store_previous(&live, &config);

    let services = ServiceEvidenceMap::new();
    let publisher = CancelToken::default();
    services.publish(
        [ServiceEvidence {
            name: "AcmeApi".to_owned(),
            display_name: "Acme API".to_owned(),
            binary_path: root.join("bin/acme-api.exe").to_string_lossy().into_owned(),
        }],
        &publisher,
        Instant::now(),
    );
    let mut collector = FileCollector::with_service_map(
        ListeningPortMap::new(),
        services,
        catalog_descriptor().unwrap(),
    );
    collector.start(live.context.clone()).unwrap();
    let context = SnapshotContext::for_devtools(10);
    let CollectorSnapshot::Atomic {
        observations,
        relationships,
        ..
    } = collector.snapshot_batch(context).unwrap()
    else {
        panic!("changed config must produce an atomic snapshot");
    };

    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].field.as_deref(), Some("MODE"));
    assert_eq!(relationships.len(), 1);
    assert_eq!(relationships[0].kind, RelationshipKind::ConfigOf);
    assert!(matches!(
        relationships[0].target.identity,
        what_changed_lib::collectors::EntityIdentity::Service { ref name }
            if name == "AcmeApi"
    ));
}

#[test]
fn config_under_a_registered_project_uses_only_observed_process_path_evidence() {
    let live = live_context(ID).unwrap();
    let root = live.directory.join("project-process-root");
    std::fs::create_dir_all(root.join("bin")).unwrap();
    register_root(&live, &root);
    register_project_process(&live, &root, &root.join("bin/api.exe"));
    let config = root.join(".env");
    std::fs::write(&config, "PORT=3000\nMODE=production\n").unwrap();
    store_previous(&live, &config);

    let mut collector = FileCollector::new(ListeningPortMap::new(), catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    let context = SnapshotContext::for_devtools(15);
    let snapshot = collector.snapshot_batch(context).unwrap();
    let CollectorSnapshot::Atomic {
        observations,
        relationships,
        ..
    } = &snapshot
    else {
        panic!("changed config must produce an atomic snapshot");
    };

    assert_eq!(observations.len(), 1);
    assert_eq!(relationships.len(), 1);
    assert_eq!(relationships[0].kind, RelationshipKind::ConfigOf);
    assert!(matches!(
        relationships[0].target.identity,
        what_changed_lib::collectors::EntityIdentity::ProcessById { ref id }
            if id.as_str() == ids::process(&root.join("bin/api.exe").to_string_lossy())
    ));

    // Exercise the production normalizer and one all-or-nothing storage
    // transaction as well as the collector's raw edge.  This catches a
    // ProcessById/config_of direction or endpoint-type mismatch that a raw
    // collector assertion would miss.
    let Some((token, receipt)) =
        what_changed_lib::collectors::testing::persist_atomic_snapshot(&live.db, ID, snapshot)
            .unwrap()
    else {
        panic!("project config evidence must use an atomic snapshot");
    };
    collector.commit_succeeded(token, &receipt).unwrap();
    let file_id = ids::file(&config.to_string_lossy());
    let persisted = live
        .db
        .read(|connection| entity_repository::relationships_from(connection, &file_id))
        .unwrap();
    assert_eq!(persisted.len(), 1);
    assert_eq!(persisted[0].kind, RelationshipKind::ConfigOf);
    assert_eq!(
        persisted[0].target_id,
        ids::process(&root.join("bin/api.exe").to_string_lossy())
    );
}

#[test]
fn collector_config_edge_reaches_confirmed_investigation_chain() {
    let live = live_context(ID).unwrap();
    let root = live.directory.join("confirmed-chain-root");
    std::fs::create_dir_all(root.join("bin")).unwrap();
    register_root(&live, &root);
    let executable = root.join("bin/api.exe");
    register_project_process(&live, &root, &executable);

    let config = root.join(".env");
    // Let the first sweep refresh the persisted baseline without emitting an
    // uncertain downtime event. The watched change below then arrives through
    // notify as a normal, time-certain modification.
    std::fs::write(&config, "PORT=3000\n").unwrap();
    store_previous(&live, &config);

    let port_map = ListeningPortMap::new();
    let publisher = CancelToken::default();
    port_map.publish(
        [TrackedSocket {
            protocol: Protocol::Tcp,
            address: "127.0.0.1".to_owned(),
            port: 4_000,
            owners: BTreeSet::new(),
        }],
        &publisher,
        Instant::now(),
    );
    let mut collector = FileCollector::new(port_map, catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    let initial_context = SnapshotContext::for_devtools(30);
    let initial_snapshot = collector.snapshot_batch(initial_context).unwrap();
    let Some((initial_token, initial_receipt)) =
        what_changed_lib::collectors::testing::persist_atomic_snapshot(
            &live.db,
            ID,
            initial_snapshot,
        )
        .unwrap()
    else {
        panic!("initial sweep must persist the refreshed config baseline");
    };
    collector
        .commit_succeeded(initial_token, &initial_receipt)
        .unwrap();

    std::fs::write(&config, "PORT=4000\n").unwrap();
    thread::sleep(Duration::from_millis(700));

    let process_id = ids::process(&executable.to_string_lossy());
    let port_id = ids::port(Protocol::Tcp, "127.0.0.1", 4_000);
    let mut config_receipt = None;
    for token in 31..=40 {
        let candidate = collector
            .snapshot_batch(SnapshotContext::for_devtools(token))
            .unwrap();
        let is_config_change = matches!(
            &candidate,
            CollectorSnapshot::Atomic { observations, .. }
                if observations.len() == 1
                    && observations[0].field.as_deref() == Some("PORT")
        );
        if is_config_change {
            let CollectorSnapshot::Atomic {
                observations,
                relationships,
                ..
            } = &candidate
            else {
                unreachable!();
            };
            assert!(!observations[0].occurred_at_is_uncertain);
            assert!(relationships.iter().any(|edge| {
                edge.kind == RelationshipKind::ConfigOf
                    && matches!(edge.target.identity, what_changed_lib::collectors::EntityIdentity::ProcessById { ref id } if id.as_str() == process_id)
            }));
            assert!(relationships.iter().any(|edge| {
                edge.kind == RelationshipKind::ConfigOf
                    && matches!(
                        edge.target.identity,
                        what_changed_lib::collectors::EntityIdentity::Port { port: 4_000, .. }
                    )
            }));
            let Some((commit_token, receipt)) =
                what_changed_lib::collectors::testing::persist_atomic_snapshot(
                    &live.db, ID, candidate,
                )
                .unwrap()
            else {
                panic!("collector output must be persisted as an atomic snapshot");
            };
            collector.commit_succeeded(commit_token, &receipt).unwrap();
            config_receipt = Some(receipt);
            break;
        }
        if let CollectorSnapshot::Atomic { .. } = candidate {
            let Some((commit_token, receipt)) =
                what_changed_lib::collectors::testing::persist_atomic_snapshot(
                    &live.db, ID, candidate,
                )
                .unwrap()
            else {
                panic!("a file snapshot must persist before the next sweep");
            };
            collector.commit_succeeded(commit_token, &receipt).unwrap();
        }
        thread::sleep(Duration::from_millis(50));
    }
    let receipt = config_receipt.expect("watcher modification must produce a config event");
    let config_event_id = receipt.event_ids[0];
    let config_event = live
        .db
        .read(|connection| event_repository::by_id(connection, config_event_id))
        .unwrap()
        .expect("collector config event");
    let config_at = config_event.event.timestamp;

    // The remaining rows model the already-proven process/listener collector
    // output.  The config event and its relationship are the only facts under
    // test here; these rows let the production investigation service walk the
    // same persisted graph that P09 supplies at runtime.
    let process_events = insert_timeline_events(
        &live,
        vec![
            timeline_event(
                config_at + 60_000,
                Category::Process,
                EventType::Stopped,
                EntityType::Process,
                process_id.clone(),
                "api.exe",
                "process",
            ),
            timeline_event(
                config_at + 60_300,
                Category::Process,
                EventType::Started,
                EntityType::Process,
                process_id.clone(),
                "api.exe",
                "process",
            ),
            timeline_event(
                config_at + 120_000,
                Category::Network,
                EventType::Closed,
                EntityType::Port,
                port_id.clone(),
                "TCP 127.0.0.1:4000",
                "listeners",
            ),
            timeline_event(
                config_at + 120_300,
                Category::Network,
                EventType::Opened,
                EntityType::Port,
                port_id.clone(),
                "TCP 127.0.0.1:4000",
                "listeners",
            ),
        ],
    );
    relate(
        &live,
        &process_id,
        &port_id,
        RelationshipKind::ListensOn,
        Some(process_events[3]),
        config_at + 120_300,
    );
    relate(
        &live,
        &port_id,
        &process_id,
        RelationshipKind::OpenedBy,
        Some(process_events[3]),
        config_at + 120_300,
    );

    let service = InvestigationService::new(live.db.clone());
    let result = service
        .start(
            "my API cannot connect to TCP 127.0.0.1:4000".to_owned(),
            Some(custom_window(config_at - 1, config_at + 180_000)),
        )
        .unwrap();
    let config_suspect = result
        .suspects
        .iter()
        .find(|suspect| suspect.event_id == config_event_id)
        .expect("collector config event must be investigated");
    assert_eq!(config_suspect.confidence, ConfidenceClass::Confirmed);
    assert_eq!(config_suspect.chain.len(), 3);
    assert!(config_suspect
        .chain
        .iter()
        .any(|step| step.label.contains("TCP 127.0.0.1:4000")));

    let file_id = ids::file(&config.to_string_lossy());
    let persisted = live
        .db
        .read(|connection| entity_repository::relationships_from(connection, &file_id))
        .unwrap();
    assert!(persisted.iter().any(|edge| {
        edge.kind == RelationshipKind::ConfigOf
            && edge.target_id == process_id
            && edge.event_id == Some(config_event_id)
    }));
}

#[test]
fn many_project_processes_are_bounded_without_poisoning_live_port_evidence() {
    let live = live_context(ID).unwrap();
    let root = live.directory.join("large-project-root");
    std::fs::create_dir_all(root.join("bin")).unwrap();
    register_root(&live, &root);
    let root_text = root.to_string_lossy().into_owned();
    live.db
        .write(move |connection| {
            project_repository::insert(connection, &root_text, "large-project", 1)?;
            for index in 0..250 {
                let executable = format!(r"{root_text}\bin\worker-{index}.exe");
                let process_id = ids::process(&executable);
                connection.execute(
                    "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, \
                     entity_display_name, severity, importance, source_collector, metadata_json, \
                     user_friendly_summary, technical_summary, confidence) \
                     VALUES (?1, 'process', 'started', 'process', ?2, 'worker', 'info', \
                     'informational', 'process', ?3, 'started', 'started', 'high')",
                    (
                        index as i64 + 2,
                        &process_id,
                        serde_json::json!({ "path": executable }).to_string(),
                    ),
                )
                .unwrap();
            }
            Ok(())
        })
        .unwrap();

    let config = root.join(".env");
    std::fs::write(&config, "PORT=4000\n").unwrap();
    store_previous(&live, &config);
    let map = ListeningPortMap::new();
    let publisher = CancelToken::default();
    map.publish(
        [TrackedSocket {
            protocol: Protocol::Tcp,
            address: "127.0.0.1".to_owned(),
            port: 4_000,
            owners: BTreeSet::new(),
        }],
        &publisher,
        Instant::now(),
    );
    let mut collector = FileCollector::new(map, catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    let context = SnapshotContext::for_devtools(16);
    let CollectorSnapshot::Atomic {
        observations,
        relationships,
        ..
    } = collector.snapshot_batch(context).unwrap()
    else {
        panic!("changed config must produce an atomic snapshot");
    };

    assert_eq!(observations.len(), 1);
    assert_eq!(
        relationships.len(),
        201,
        "200 project edges plus the live port"
    );
    assert!(relationships.iter().any(|edge| matches!(
        edge.target.identity,
        what_changed_lib::collectors::EntityIdentity::Port { port: 4_000, .. }
    )));
    assert_eq!(
        observations[0].details.as_ref().unwrap()["project_process_evidence_bounded"],
        true
    );
}

#[test]
fn unrelated_non_config_modification_never_creates_a_config_edge() {
    let live = live_context(ID).unwrap();
    let root = live.directory.join("non-config-root");
    std::fs::create_dir(&root).unwrap();
    register_root(&live, &root);
    let note = root.join("README.txt");
    std::fs::write(&note, "PORT=4000 is documentation").unwrap();

    let map = ListeningPortMap::new();
    let publisher = CancelToken::default();
    map.publish(
        [TrackedSocket {
            protocol: Protocol::Tcp,
            address: "127.0.0.1".to_owned(),
            port: 4_000,
            owners: BTreeSet::new(),
        }],
        &publisher,
        Instant::now(),
    );
    let mut collector = FileCollector::new(map, catalog_descriptor().unwrap());
    collector.start(live.context.clone()).unwrap();
    let initial = SnapshotContext::for_devtools(20);
    assert!(matches!(
        collector.snapshot_batch(initial).unwrap(),
        CollectorSnapshot::Noop
    ));

    thread::sleep(Duration::from_millis(100));
    std::fs::write(&note, "PORT=4000 remains documentation").unwrap();
    thread::sleep(Duration::from_millis(700));
    let mut saw_modification = false;
    for token in 21..29 {
        let context = SnapshotContext::for_devtools(token);
        match collector.snapshot_batch(context).unwrap() {
            CollectorSnapshot::Atomic {
                observations,
                relationships,
                ..
            } => {
                assert!(relationships.is_empty());
                saw_modification |= observations
                    .iter()
                    .any(|event| event.kind.event_type == EventType::Modified);
                collector
                    .commit_succeeded(
                        context.token(),
                        &CollectorCommitReceipt { event_ids: vec![] },
                    )
                    .unwrap();
            }
            CollectorSnapshot::Noop => {}
            CollectorSnapshot::Legacy(_) => panic!("files must use atomic snapshots"),
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(saw_modification);
}
