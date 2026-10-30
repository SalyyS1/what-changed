//! Owner fixtures use temporary databases only; no personal app-data paths are read.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::owner_workspace::*;

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn path() -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("what-changed-test-owner-{}", id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("owner-workspace.sqlite")
}
fn app_input() -> SaveOwnerAppInput {
    SaveOwnerAppInput {
        operation_id: id(),
        id: id(),
        expected_version: 0,
        name: "Fixture app".into(),
        description: "Owner meaning".into(),
        legacy_project_id: None,
        archived: false,
    }
}
fn capability(app_id: &str) -> SaveOwnerCapabilityInput {
    SaveOwnerCapabilityInput {
        operation_id: id(),
        id: id(),
        app_id: app_id.into(),
        expected_version: 0,
        label: "Bookings".into(),
        description: "Create a booking".into(),
        sort_order: 0,
        archived: false,
    }
}
fn rule(app_id: &str, capability_id: &str) -> ConfirmOwnerRuleInput {
    ConfirmOwnerRuleInput {
        operation_id: id(),
        rule_id: id(),
        app_id: app_id.into(),
        capability_id: capability_id.into(),
        expected_version: 0,
        expected_behavior: "A booking remains visible to staff".into(),
        provenance: RuleProvenance::Owner,
        confirmed_by: "fixture owner".into(),
    }
}
fn env(app_id: &str) -> SaveOwnerEnvironmentInput {
    SaveOwnerEnvironmentInput {
        operation_id: id(),
        id: id(),
        app_id: app_id.into(),
        expected_version: 0,
        label: "Local test".into(),
        preview_url: "http://localhost:4173/preview".into(),
        origins: vec![OriginDto {
            scheme: "http".into(),
            host: "localhost".into(),
            port: 4173,
        }],
        kind: EnvironmentKind::Local,
        revoked: false,
    }
}

#[test]
fn app_identity_restart_and_legacy_link_are_independent() {
    let path = path();
    let store = OwnerStore::open(&path).unwrap();
    let mut input = app_input();
    input.legacy_project_id = Some("project:0123456789abcdef".into());
    let app = store.save_app(input.clone()).unwrap();
    uuid::Uuid::parse_str(&app.id).unwrap();
    assert_eq!(app.id, input.id);
    store.shutdown().unwrap();
    let reopened = OwnerStore::open(&path).unwrap();
    assert_eq!(reopened.get_app(&app.id).unwrap(), Some(app.clone()));
    input.operation_id = id();
    input.expected_version = app.version;
    input.legacy_project_id = None;
    let detached = reopened.save_app(input).unwrap();
    assert_eq!(detached.id, app.id);
    assert!(detached.legacy_project_id.is_none());
}

#[test]
fn versions_and_operation_ids_reject_lost_edits_and_conflicting_retries() {
    let store = OwnerStore::open(&path()).unwrap();
    let input = app_input();
    let app = store.save_app(input.clone()).unwrap();
    assert_eq!(store.save_app(input.clone()).unwrap(), app);
    let mut changed = input.clone();
    changed.name = "Another request".into();
    assert!(store
        .save_app(changed)
        .unwrap_err()
        .to_string()
        .contains("different request"));
    let mut stale = input;
    stale.operation_id = id();
    assert!(store
        .save_app(stale)
        .unwrap_err()
        .to_string()
        .contains("conflict"));
    assert_eq!(store.get_app(&app.id).unwrap().unwrap(), app);
}

#[test]
fn cross_app_rule_and_candidate_references_fail_atomically() {
    let store = OwnerStore::open(&path()).unwrap();
    let a = store.save_app(app_input()).unwrap();
    let b = store.save_app(app_input()).unwrap();
    let cap = store.save_capability(capability(&a.id)).unwrap();
    assert!(store.confirm_rule(rule(&b.id, &cap.id)).is_err());
    let confirmed = store.confirm_rule(rule(&a.id, &cap.id)).unwrap();
    let request = SaveOwnerChangeInput {
        operation_id: id(),
        id: id(),
        app_id: b.id.clone(),
        expected_version: 0,
        intent: "Change".into(),
        example: String::new(),
        scope: String::new(),
        preserved_rules: vec![RuleRefDto {
            rule_id: confirmed.rule_id,
            revision: 1,
        }],
        candidate_id: None,
        state: ChangeState::Draft,
    };
    assert!(store.save_change_draft(request).is_err());
    assert!(store
        .list_change_drafts(&b.id, OwnerPageInput::default())
        .unwrap()
        .items
        .is_empty());
    let ea = store.save_environment(env(&a.id)).unwrap();
    assert!(store
        .save_candidate(SaveOwnerCandidateInput {
            operation_id: id(),
            id: id(),
            app_id: b.id,
            environment_id: ea.id,
            environment_revision: 1,
            identity: "build-1".into(),
            source: CandidateSource::OwnerLabel,
            trust: CandidateTrust::SelfReported,
            observed_at: 1
        })
        .is_err());
}

#[test]
fn rule_and_environment_revisions_are_immutable_and_history_survives_revocation() {
    let path = path();
    let store = OwnerStore::open(&path).unwrap();
    let app = store.save_app(app_input()).unwrap();
    let cap = store.save_capability(capability(&app.id)).unwrap();
    let first_input = rule(&app.id, &cap.id);
    let first = store.confirm_rule(first_input.clone()).unwrap();
    let mut second_input = first_input;
    second_input.operation_id = id();
    second_input.expected_version = 1;
    second_input.expected_behavior = "Staff can read the exact booking ID".into();
    let second = store.confirm_rule(second_input).unwrap();
    assert_eq!(second.supersedes, Some(1));
    let history = store
        .list_rule_revisions(&app.id, OwnerPageInput::default())
        .unwrap();
    assert_eq!(history.items, vec![first, second]);
    let ei = env(&app.id);
    let e = store.save_environment(ei.clone()).unwrap();
    let c = store
        .save_candidate(SaveOwnerCandidateInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            environment_id: e.id,
            environment_revision: 1,
            identity: "self reported build".into(),
            source: CandidateSource::OwnerLabel,
            trust: CandidateTrust::SelfReported,
            observed_at: 1,
        })
        .unwrap();
    let mut revoked = ei;
    revoked.operation_id = id();
    revoked.expected_version = 1;
    revoked.revoked = true;
    store.save_environment(revoked).unwrap();
    assert_eq!(
        store
            .list_candidates(&app.id, OwnerPageInput::default())
            .unwrap()
            .items,
        vec![c.clone()]
    );
    assert!(store
        .select_candidate(SelectOwnerCandidateInput {
            operation_id: id(),
            app_id: app.id,
            expected_version: 1,
            candidate_id: c.id
        })
        .is_err());
    store.shutdown().unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert!(conn
        .execute("UPDATE owner_rule_revisions SET revision=99", [])
        .is_err());
    assert!(conn
        .execute("UPDATE owner_environment_revisions SET revoked=0", [])
        .is_err());
    assert!(conn
        .execute("UPDATE owner_candidates SET environment_revision=2", [])
        .is_err());
}

#[test]
fn checksum_newer_and_corrupt_files_are_preserved_and_report_unavailable() {
    for mode in ["checksum", "newer", "corrupt"] {
        let path = path();
        let store = OwnerStore::open(&path).unwrap();
        store.save_app(app_input()).unwrap();
        store.shutdown().unwrap();
        if mode == "corrupt" {
            std::fs::write(&path, b"canary-invalid-sqlite-owner").unwrap();
        } else {
            let conn = rusqlite::Connection::open(&path).unwrap();
            if mode == "checksum" {
                conn.execute("UPDATE owner_schema_migrations SET checksum='wrong'", [])
                    .unwrap();
            } else {
                conn.pragma_update(None, "user_version", 99).unwrap();
            }
        }
        let before = std::fs::read(&path).unwrap();
        let unusable = OwnerStore::open(&path).unwrap();
        assert!(matches!(
            unusable.state(),
            OwnerStoreState::Unavailable { .. }
        ));
        assert!(unusable.list_apps(true).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[test]
fn stable_pagination_and_limits_are_explicit() {
    let store = OwnerStore::open(&path()).unwrap();
    let app = store.save_app(app_input()).unwrap();
    for _ in 0..3 {
        store
            .save_change_draft(SaveOwnerChangeInput {
                operation_id: id(),
                id: id(),
                app_id: app.id.clone(),
                expected_version: 0,
                intent: "Change".into(),
                example: String::new(),
                scope: String::new(),
                preserved_rules: vec![],
                candidate_id: None,
                state: ChangeState::Draft,
            })
            .unwrap();
    }
    let first = store
        .list_change_drafts(
            &app.id,
            OwnerPageInput {
                after: None,
                limit: 2,
            },
        )
        .unwrap();
    assert_eq!(first.items.len(), 2);
    let second = store
        .list_change_drafts(
            &app.id,
            OwnerPageInput {
                after: first.next_cursor,
                limit: 2,
            },
        )
        .unwrap();
    assert_eq!(second.items.len(), 1);
    assert!(second.next_cursor.is_none());
    assert!(first
        .items
        .iter()
        .all(|a| second.items.iter().all(|b| a.id != b.id)));
    let mut large = app_input();
    large.description = "é".repeat(MAX_TEXT_BYTES / 2 + 1);
    assert!(store
        .save_app(large)
        .unwrap_err()
        .to_string()
        .contains("16 KiB"));
    assert!(store
        .list_change_drafts(
            &app.id,
            OwnerPageInput {
                after: None,
                limit: 201
            }
        )
        .is_err());
    for _ in 1..MAX_APPS {
        store.save_app(app_input()).unwrap();
    }
    assert!(store
        .save_app(app_input())
        .unwrap_err()
        .to_string()
        .contains("20 apps"));
}

#[test]
fn environment_configuration_rejects_credentials_and_private_remote_origins() {
    let store = OwnerStore::open(&path()).unwrap();
    let app = store.save_app(app_input()).unwrap();
    for url in [
        "https://user:pass@example.test/",
        "https://example.test/?token=secret",
        "http://10.0.0.2/",
        "https://192.168.1.5/",
        "https://[::ffff:192.168.1.5]/",
        "https://[::ffff:127.0.0.1]/",
    ] {
        let parsed = tauri::Url::parse(url).unwrap();
        let mut input = env(&app.id);
        input.preview_url = url.into();
        input.kind = EnvironmentKind::Test;
        input.origins = vec![OriginDto {
            scheme: parsed.scheme().into(),
            host: parsed.host_str().unwrap().into(),
            port: parsed.port_or_known_default().unwrap(),
        }];
        assert!(store.save_environment(input).is_err());
    }
    assert!(store
        .list_environment_revisions(&app.id, OwnerPageInput::default())
        .unwrap()
        .items
        .is_empty());
}
