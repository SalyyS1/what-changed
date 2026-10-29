//! Portable memory fixtures use isolated stores and never touch a user profile.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use what_changed_lib::dto::{owner_memory_dto::*, owner_portability_dto::*};
use what_changed_lib::owner_workspace::{archive, portability::*, *};

#[path = "owner_portability/review_lifecycle.rs"]
mod review_lifecycle;

#[path = "owner_portability/historical_recipe.rs"]
mod historical_recipe;

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn store() -> OwnerStore {
    let path = std::env::temp_dir()
        .join(format!("what-changed-test-portability-{}", id()))
        .join("owner-workspace.sqlite");
    OwnerStore::open(&path).unwrap()
}
fn fixture() -> (OwnerStore, String, String, OwnerRuleRevisionDto) {
    let store = store();
    let app = store
        .save_app(SaveOwnerAppInput {
            operation_id: id(),
            id: id(),
            expected_version: 0,
            name: "Booking app".into(),
            description: "Keep booking details visible".into(),
            legacy_project_id: None,
            archived: false,
        })
        .unwrap();
    let cap = store
        .save_capability(SaveOwnerCapabilityInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            expected_version: 0,
            label: "Bookings".into(),
            description: "Booking lifecycle".into(),
            sort_order: 0,
            archived: false,
        })
        .unwrap();
    let rule = store
        .confirm_rule(ConfirmOwnerRuleInput {
            operation_id: id(),
            rule_id: id(),
            app_id: app.id.clone(),
            capability_id: cap.id.clone(),
            expected_version: 0,
            expected_behavior: "Staff see confirmed bookings".into(),
            provenance: RuleProvenance::Owner,
            confirmed_by: "Owner".into(),
        })
        .unwrap();
    (store, app.id, cap.id, rule)
}
fn apply_input(data: &PortableWorkspace, digest: String) -> ApplyOwnerImportInput {
    ApplyOwnerImportInput {
        operation_id: id(),
        review_id: id(),
        digest,
        name: "Imported booking app".into(),
        confirmed_by: "Reviewed owner".into(),
        confirmed_rules: data
            .records
            .rules
            .iter()
            .map(|r| RuleRefDto {
                rule_id: r.rule_id.clone(),
                revision: r.revision,
            })
            .collect(),
    }
}

#[test]
fn repeat_exports_are_deterministic_and_import_keeps_history_without_authority() {
    let (source, app, cap, rule) = fixture();
    source
        .confirm_scenario(ConfirmOwnerScenarioInput {
            operation_id: id(),
            id: id(),
            app_id: app.clone(),
            capability_id: cap,
            expected_version: 0,
            title: "Open user form".into(),
            expected_behavior: "The new user form is visible".into(),
            confirmed_by: "Owner".into(),
            method: VerificationMethod::OwnerManual,
            rules: vec![RuleRefDto {
                rule_id: rule.rule_id.clone(),
                revision: 1,
            }],
            recipe: ScenarioRecipeDto {
                schema_version: 1,
                role_refs: vec![],
                fixture_refs: vec![],
                adapters: vec![],
                steps: vec![RecipeStepDto {
                    id: id(),
                    action: RecipeActionDto::Navigate {
                        path: "/users/new".into(),
                    },
                    assertions: vec![NamedAssertionDto {
                        id: id(),
                        expected_behavior: "Name input is visible".into(),
                        assertion: RecipeAssertionDto::Visible {
                            locator: LocatorDto::Label {
                                text: "Name".into(),
                            },
                        },
                    }],
                }],
            },
        })
        .unwrap();
    source
        .save_change_draft(SaveOwnerChangeInput {
            operation_id: id(),
            id: id(),
            app_id: app.clone(),
            expected_version: 0,
            intent: "Add booking notes".into(),
            example: "Staff see the note".into(),
            scope: "Booking form only".into(),
            preserved_rules: vec![RuleRefDto {
                rule_id: rule.rule_id.clone(),
                revision: 1,
            }],
            candidate_id: None,
            state: ChangeState::ReadyForReview,
        })
        .unwrap();
    let selection = OwnerExportSelectionInput {
        include_target_details: false,
    };
    let data = source.portable_snapshot(&app).unwrap();
    let (a, preview) = prepare_bundle(data.clone(), &selection).unwrap();
    let (b, _) = prepare_bundle(source.portable_snapshot(&app).unwrap(), &selection).unwrap();
    assert_eq!(a, b);
    let validated = validate_bundle(&a).unwrap();
    let target = store();
    let input = apply_input(&validated, preview.digest);
    let new_id = id();
    let imported = target
        .apply_portable_import(validated.clone(), input.clone(), new_id.clone())
        .unwrap();
    assert_eq!(
        target
            .apply_portable_import(validated, input, new_id)
            .unwrap(),
        imported
    );
    assert_eq!(target.list_apps(true).unwrap().len(), 1);
    let snapshot = target.portable_snapshot(&imported.app.id).unwrap();
    assert_ne!(snapshot.records.app.id, app);
    assert_eq!(snapshot.records.app.current_candidate_id, None);
    assert_eq!(snapshot.records.app.legacy_project_id, None);
    assert_eq!(
        snapshot.records.rules[0].expected_behavior,
        rule.expected_behavior
    );
    assert_ne!(snapshot.records.rules[0].rule_id, rule.rule_id);
    assert_eq!(
        snapshot.records.rules[0].provenance,
        RuleProvenance::ImportedUntrusted
    );
    assert_eq!(snapshot.records.changes[0].state, ChangeState::Draft);
    assert!(
        snapshot.records.runs.is_empty()
            && snapshot.records.candidates.is_empty()
            && snapshot.records.environments.is_empty()
    );
    assert_eq!(snapshot.history.len(), 1);
    assert_eq!(snapshot.history[0].records.app.id, app);
    let (again, _) = prepare_bundle(snapshot, &selection).unwrap();
    let round = validate_bundle(&again).unwrap();
    assert_eq!(
        round.history[0].records.changes[0].intent,
        "Add booking notes"
    );
    assert_eq!(round.history[0].records.rules[0].rule_id, rule.rule_id);
    assert_eq!(round.history[0].records.scenarios.len(), 1);
    assert!(
        matches!(&round.history[0].records.scenarios[0].recipe.steps[0].action,RecipeActionDto::Navigate{path} if path=="/users/new")
    );
}

#[test]
fn import_requires_each_reviewed_rule_and_rolls_back_every_record_on_conflict() {
    let (source, app, _, _) = fixture();
    let data = source.portable_snapshot(&app).unwrap();
    let target = store();
    let mut input = apply_input(&data, "digest".into());
    input.confirmed_rules.clear();
    assert!(target
        .apply_portable_import(data.clone(), input, id())
        .is_err());
    assert!(target.list_apps(true).unwrap().is_empty());
    let input = apply_input(&data, "digest".into());
    let new_id = id();
    target
        .apply_portable_import(data.clone(), input.clone(), new_id)
        .unwrap();
    let mut conflict = input;
    conflict.name = "Changed retry".into();
    assert!(target.apply_portable_import(data, conflict, id()).is_err());
    assert_eq!(target.list_apps(true).unwrap().len(), 1);
}

#[test]
fn default_export_omits_secret_paths_owner_and_urls_without_omitting_rule_meaning() {
    let (store, app, cap, _) = fixture();
    let canary = "\"C:\\Users\\Private Owner\\app\\config.env\"";
    store.confirm_rule(ConfirmOwnerRuleInput{operation_id:id(),rule_id:id(),app_id:app.clone(),capability_id:cap,expected_version:0,expected_behavior:format!("Keep staff view; local {canary}; https://private.example.test; api_key=secret-canary-value"),provenance:RuleProvenance::Owner,confirmed_by:"PrivateOwner".into()}).unwrap();
    let (bytes, preview) = prepare_bundle(
        store.portable_snapshot(&app).unwrap(),
        &OwnerExportSelectionInput {
            include_target_details: false,
        },
    )
    .unwrap();
    let text = String::from_utf8_lossy(&bytes);
    for canary in [
        "PrivateOwner",
        "Private Owner",
        "Owner\\\\app",
        "secret-canary-value",
        "private.example.test",
        "config.env",
    ] {
        assert!(!text.contains(canary), "{canary}");
    }
    assert!(preview.markdown.contains("Keep staff view"));
    assert_eq!(preview.rules.len(), 2);
}

#[test]
fn project_memory_supersession_is_immutable_filtered_and_app_scoped() {
    let (store, app, cap, _) = fixture();
    let input = SaveProjectDecisionInput {
        operation_id: id(),
        id: id(),
        app_id: app.clone(),
        expected_revision: 0,
        capability_id: Some(cap.clone()),
        change_id: None,
        text: "Staff approve bookings".into(),
        rationale: "Avoid duplicate slots".into(),
        provenance: RuleProvenance::Owner,
        recorded_by: "Owner".into(),
    };
    let first = store.save_project_decision(input.clone()).unwrap();
    let mut next = input.clone();
    next.operation_id = id();
    next.expected_revision = 1;
    next.text = "Staff or manager approve bookings".into();
    let second = store.save_project_decision(next).unwrap();
    assert_eq!(second.supersedes, Some(first.revision));
    let history = store
        .list_project_decisions(
            &app,
            ProjectMemoryFilterInput {
                capability_id: Some(cap),
                change_id: None,
                page: OwnerPageInput::default(),
            },
        )
        .unwrap();
    assert_eq!(history.items, vec![first, second]);
    let mut stale = input;
    stale.operation_id = id();
    assert!(store.save_project_decision(stale).is_err());
}

#[test]
fn hostile_archive_names_metadata_and_size_are_rejected_before_any_extraction() {
    // These independent Python stdlib ZIP fixtures retain complete filenames;
    // the trailing-dot alias is 15 bytes and must not be truncated to the
    // original 14-byte allowed filename by the fixed-width mutation below.
    assert!(archive::decode(include_bytes!(
        "fixtures/owner-portability/valid-stored.zip.bin"
    ))
    .is_ok());
    assert!(archive::decode(include_bytes!(
        "fixtures/owner-portability/trailing-dot.zip.bin"
    ))
    .is_err());
    let (store, app, _, _) = fixture();
    let (bytes, _) = prepare_bundle(
        store.portable_snapshot(&app).unwrap(),
        &OwnerExportSelectionInput {
            include_target_details: false,
        },
    )
    .unwrap();
    for name in [
        "../escape.json",
        "C:/escape.json",
        "CON           ",
        "data:stream   ",
        "WORKSPACE.JSON",
        "workspace.jso.",
        "\\\\server\\share",
    ] {
        let mut hostile = bytes.clone();
        let needle = b"workspace.json";
        let replacement = format!("{name:<14}");
        let replacement = &replacement.as_bytes()[..14];
        for i in 0..hostile.len() - needle.len() {
            if &hostile[i..i + needle.len()] == needle {
                hostile[i..i + needle.len()].copy_from_slice(replacement);
            }
        }
        assert!(archive::decode(&hostile).is_err(), "{name}");
    }
    let central = bytes
        .windows(4)
        .position(|s| s == [0x50, 0x4b, 0x01, 0x02])
        .unwrap();
    for attributes in [0xa1ff0000_u32, 0x400, 0x10] {
        let mut b = bytes.clone();
        b[central + 38..central + 42].copy_from_slice(&attributes.to_le_bytes());
        assert!(archive::decode(&b).is_err());
    }
    let mut compressed = bytes.clone();
    compressed[8] = 8;
    assert!(archive::decode(&compressed).is_err());
    let mut huge = bytes.clone();
    huge[22..26].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(archive::decode(&huge).is_err());
    let mut trailing = bytes;
    trailing.push(0);
    assert!(archive::decode(&trailing).is_err());
}
