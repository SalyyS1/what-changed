#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;

use what_changed_lib::analysis::evidence::{find_log_evidence, EvidenceWindow};
use what_changed_lib::collectors::normalize::Normalizer;
use what_changed_lib::collectors::{EntityIdentity, ObservationKind, RawEntity, RawObservation};
use what_changed_lib::domain::{
    Category, ClassifierContext, Confidence, EventType, ExitKind, Severity,
};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::services::query_service;
use what_changed_lib::storage::{event_repository, AtomicCollectorCommit, AtomicObservation, Db};
use what_changed_lib::testing::{synthetic_event, temp_dir};

const AT: i64 = 1_800_000_000_000;

fn process_exit_named(executable: &str) -> RawObservation {
    RawObservation {
        collector_id: "process",
        kind: ObservationKind {
            category: Category::Process,
            event_type: EventType::Stopped,
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary: format!("{executable} stopped"),
            technical_summary: format!("process {executable} stopped"),
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            identity: EntityIdentity::Process {
                executable: executable.to_owned(),
            },
            display_name: executable.to_owned(),
        },
        before: None,
        after: None,
        field: None,
        occurred_at: AT,
        details: Some(json!({ "exit_kind": ExitKind::Unknown.as_str() })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: false,
    }
}

fn process_exit() -> RawObservation {
    process_exit_named("sample-app.exe")
}

fn update_failure() -> RawObservation {
    RawObservation {
        collector_id: "updates",
        kind: ObservationKind {
            category: Category::Update,
            event_type: EventType::Failed,
            severity: Severity::Error,
            confidence: Confidence::High,
            user_friendly_summary: "Windows Update failed: KB5034123".to_owned(),
            technical_summary: "Windows Update failed".to_owned(),
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            identity: EntityIdentity::Update {
                id: "KB5034123".to_owned(),
            },
            display_name: "KB5034123".to_owned(),
        },
        before: None,
        after: None,
        field: None,
        occurred_at: AT,
        details: Some(json!({ "update_id": "update-guid" })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: false,
    }
}

fn insert_log(db: &Db, data_value: &str, windows_event_id: u32, seed: u64) {
    let mut event = synthetic_event(seed, AT + 1_000);
    event.source_collector = "eventlog".to_owned();
    event.metadata = Some(if windows_event_id == 20 {
        json!({
            "channel": "Microsoft-Windows-WindowsUpdateClient/Operational",
            "provider": "Microsoft-Windows-WindowsUpdateClient",
            "event_id": windows_event_id,
            "message": data_value,
            "data": [{ "name": "updateTitle", "value": data_value }],
            "count": 1,
        })
    } else {
        json!({
            "channel": "Application",
            "provider": "Application Error",
            "event_id": windows_event_id,
            "message": data_value,
            "data": [{ "name": "AppName", "value": data_value }],
            "count": 1,
        })
    });
    db.insert_events(vec![Sanitizer::new().sanitize(event).0])
        .unwrap();
}

fn insert_service_log(db: &Db, service: &str, seed: u64) {
    let mut event = synthetic_event(seed, AT + 1_000);
    event.source_collector = "eventlog".to_owned();
    event.metadata = Some(json!({
        "channel": "System",
        "provider": "Service Control Manager",
        "event_id": 7031,
        "message": format!("{service} terminated unexpectedly"),
        "data": [{ "name": "param1", "value": service }],
        "count": 1,
    }));
    db.insert_events(vec![Sanitizer::new().sanitize(event).0])
        .unwrap();
}

fn submit_process_exit(db: &Db, executable: &str) -> i64 {
    let event = Normalizer::new().normalize(process_exit_named(executable));
    let observation = AtomicObservation::new(event, Vec::new(), Vec::new()).unwrap();
    db.submit_collector(AtomicCollectorCommit::new(vec![observation], None).unwrap())
        .unwrap()
        .wait()
        .unwrap()
        .event_ids[0]
}

#[test]
fn process_exit_is_unexpected_only_with_matching_crash_evidence() {
    let db = Db::open_in(&temp_dir("process-log-corroboration")).unwrap();
    let absent_id = db
        .insert_events(Normalizer::new().normalize_all(vec![process_exit()]))
        .unwrap()[0];
    let absent = db
        .read(|connection| event_repository::by_id(connection, absent_id))
        .unwrap()
        .unwrap();
    assert_eq!(absent.event.metadata.unwrap()["exit_kind"], "unknown");

    insert_log(&db, "sample-app.exe", 1000, 1);
    let present_id = db
        .insert_events(Normalizer::new().normalize_all(vec![process_exit()]))
        .unwrap()[0];
    let present = db
        .read(|connection| event_repository::by_id(connection, present_id))
        .unwrap()
        .unwrap();
    let details = present.event.metadata.unwrap();
    assert_eq!(details["exit_kind"], "unexpected");
    assert_eq!(details["log_evidence"][0]["event_id"], 1000);
}

#[test]
fn update_observation_keeps_matching_persisted_log_evidence() {
    let db = Db::open_in(&temp_dir("update-log-corroboration")).unwrap();
    insert_log(&db, "KB5034123 installation failed", 20, 2);
    let update_id = db
        .insert_events(Normalizer::new().normalize_all(vec![update_failure()]))
        .unwrap()[0];
    let update = db
        .read(|connection| event_repository::by_id(connection, update_id))
        .unwrap()
        .unwrap();
    assert_eq!(
        update.event.metadata.unwrap()["log_evidence"][0]["event_id"],
        20
    );
}

#[test]
fn update_detail_never_uses_an_application_crash_with_the_same_display_token() {
    let db = Db::open_in(&temp_dir("update-crash-cross-kind")).unwrap();
    insert_log(&db, "KB5034123", 1000, 2_100);
    let update_id = db
        .insert_events(Normalizer::new().normalize_all(vec![update_failure()]))
        .unwrap()[0];
    let details = query_service::details(&db, update_id).unwrap();
    assert!(details.event.event.metadata.unwrap()["log_evidence"].is_null());
}

#[test]
fn later_crash_evidence_is_visible_when_an_earlier_process_exit_is_opened() {
    let db = Db::open_in(&temp_dir("late-process-log-corroboration")).unwrap();
    let process_id = db
        .insert_events(Normalizer::new().normalize_all(vec![process_exit()]))
        .unwrap()[0];
    insert_log(&db, "sample-app.exe", 1000, 3);

    let details = query_service::details(&db, process_id).unwrap();
    assert_eq!(
        details.event.event.metadata.unwrap()["exit_kind"],
        "unexpected"
    );
}

#[test]
fn evidence_filtering_happens_before_the_hundred_match_limit() {
    let db = Db::open_in(&temp_dir("event-log-evidence-volume")).unwrap();
    for seed in 0..101 {
        insert_log(&db, &format!("unrelated-{seed}.exe"), 1000, seed + 100);
    }
    insert_log(&db, "needle.exe", 1000, 999);

    let matches = find_log_evidence(&db, "needle.exe", EvidenceWindow::around(AT, 10_000)).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].event_id, 1000);
}

#[test]
fn service_control_manager_evidence_uses_an_exact_named_service_field() {
    let db = Db::open_in(&temp_dir("service-log-evidence")).unwrap();
    insert_service_log(&db, "Sample Service", 1_500);
    let exact =
        find_log_evidence(&db, "Sample Service", EvidenceWindow::around(AT, 10_000)).unwrap();
    assert_eq!(exact.len(), 1);
    assert_eq!(exact[0].event_id, 7031);
    assert!(
        find_log_evidence(&db, "Sample", EvidenceWindow::around(AT, 10_000))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn atomic_subject_arriving_after_evidence_is_reconciled_in_its_transaction() {
    let db = Db::open_in(&temp_dir("atomic-process-log-corroboration")).unwrap();
    insert_log(&db, "sample-app.exe", 1000, 2_000);
    let process_id = submit_process_exit(&db, "sample-app.exe");
    let stored = db
        .read(|connection| event_repository::by_id(connection, process_id))
        .unwrap()
        .unwrap();
    assert_eq!(stored.event.metadata.unwrap()["exit_kind"], "unexpected");
}

#[test]
fn a_short_process_name_never_prefix_matches_in_either_arrival_order() {
    let evidence_first = Db::open_in(&temp_dir("short-name-evidence-first")).unwrap();
    insert_log(&evidence_first, "sample-app.exe", 1000, 3_000);
    let process_id = submit_process_exit(&evidence_first, "app");
    let stored = evidence_first
        .read(|connection| event_repository::by_id(connection, process_id))
        .unwrap()
        .unwrap();
    assert_eq!(stored.event.metadata.unwrap()["exit_kind"], "unknown");

    let subject_first = Db::open_in(&temp_dir("short-name-subject-first")).unwrap();
    let process_id = submit_process_exit(&subject_first, "app");
    insert_log(&subject_first, "sample-app.exe", 1000, 3_001);
    let details = query_service::details(&subject_first, process_id).unwrap();
    assert_eq!(
        details.event.event.metadata.unwrap()["exit_kind"],
        "unknown"
    );
}

#[test]
fn late_evidence_finds_the_exact_process_after_more_than_two_hundred_noise_rows() {
    let db = Db::open_in(&temp_dir("late-evidence-process-volume")).unwrap();
    let mut exits = (0..201)
        .map(|index| process_exit_named(&format!("noise-{index}.exe")))
        .collect::<Vec<_>>();
    exits.push(process_exit());
    let ids = db
        .insert_events(Normalizer::new().normalize_all(exits))
        .unwrap();
    let target_id = *ids.last().unwrap();
    insert_log(&db, "sample-app.exe", 1000, 4_000);
    let target = db
        .read(|connection| event_repository::by_id(connection, target_id))
        .unwrap()
        .unwrap();
    assert_eq!(target.event.metadata.unwrap()["exit_kind"], "unexpected");
}
