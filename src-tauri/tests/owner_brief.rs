//! Deterministic handoff previews use only temporary owner-workspace fixtures.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::owner_workspace::*;

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn path() -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!("what-changed-test-brief-{}", id()));
    std::fs::create_dir_all(&directory).unwrap();
    directory.join("owner-workspace.sqlite")
}
fn app() -> SaveOwnerAppInput {
    SaveOwnerAppInput {
        operation_id: id(),
        id: id(),
        expected_version: 0,
        name: "Booking *app*".into(),
        description: "Offline owner workspace".into(),
        legacy_project_id: None,
        archived: false,
    }
}
fn change(app_id: &str) -> SaveOwnerChangeInput {
    SaveOwnerChangeInput {
        operation_id: id(),
        id: id(),
        app_id: app_id.into(),
        expected_version: 0,
        intent: "Keep `booking` behavior; token sk-abcdefghijklmnopqrstuvwx".into(),
        example: "A customer [creates] a booking".into(),
        scope: "UI + read-back".into(),
        preserved_rules: Vec::new(),
        candidate_id: None,
        state: ChangeState::Draft,
    }
}

#[test]
fn brief_is_deterministic_escaped_and_redacted_with_explicit_no_run_state() {
    let store = OwnerStore::open(&path()).unwrap();
    let owner_app = store.save_app(app()).unwrap();
    let request = store.save_change_draft(change(&owner_app.id)).unwrap();
    let first = store
        .render_brief(
            &owner_app.id,
            &request.id,
            OwnerBriefSelectionInput::default(),
        )
        .unwrap();
    let second = store
        .render_brief(
            &owner_app.id,
            &request.id,
            OwnerBriefSelectionInput::default(),
        )
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(first.app_id, owner_app.id);
    assert_eq!(first.change_id, request.id);
    assert_eq!(first.change_version, request.version);
    assert!(first.markdown.contains("No confirmed acceptance scenarios"));
    assert!(first.markdown.contains("No runner was started"));
    assert!(first.markdown.contains("\\*app\\*"));
    assert!(first.markdown.contains("\\[creates\\]"));
    assert!(first.markdown.contains("\\[REDACTED\\]"));
    assert!(!first.markdown.contains("sk-abcdefghijklmnopqrstuvwx"));
    assert!(first.bytes <= 32 * 1024);
    assert_eq!(first.estimated_tokens, first.bytes.div_ceil(4));
    assert!(first
        .omitted_sections
        .iter()
        .any(|section| section.contains("environment_details")));
}

#[test]
fn free_text_paths_urls_and_markdown_payloads_are_substituted_without_dropping_rules() {
    let store = OwnerStore::open(&path()).unwrap();
    let owner_app = store.save_app(app()).unwrap();
    let capability = store
        .save_capability(SaveOwnerCapabilityInput {
            operation_id: id(),
            id: id(),
            app_id: owner_app.id.clone(),
            expected_version: 0,
            label: "Protected files".into(),
            description: String::new(),
            sort_order: 0,
            archived: false,
        })
        .unwrap();
    let rule = store.confirm_rule(ConfirmOwnerRuleInput {
        operation_id: id(), rule_id: id(), app_id: owner_app.id.clone(), capability_id: capability.id,
        expected_version: 0,
        expected_behavior: r#"Never delete "C:\Users\Private Owner\project" or /home/private-owner/app; preserve that safety rule."#.into(),
        provenance: RuleProvenance::Owner, confirmed_by: "fixture owner".into(),
    }).unwrap();
    let mut draft = change(&owner_app.id);
    draft.intent = "Keep `span` <script>alert(1)</script> ![image](https://unsafe.example/image) <https://unsafe.example/link> intact; source \\server\\private\\repo".into();
    draft.preserved_rules = vec![RuleRefDto {
        rule_id: rule.rule_id,
        revision: rule.revision,
    }];
    let request = store.save_change_draft(draft).unwrap();
    // Explicit environment selection never permits arbitrary free-text URLs/paths.
    let preview = store
        .render_brief(
            &owner_app.id,
            &request.id,
            OwnerBriefSelectionInput {
                include_environment_details: true,
                ..OwnerBriefSelectionInput::default()
            },
        )
        .unwrap();
    assert!(preview.markdown.contains("Never delete"));
    assert!(preview.markdown.contains("preserve that safety rule"));
    assert!(preview.markdown.contains("local path omitted"));
    assert!(preview.markdown.contains("URL omitted"));
    assert!(preview.markdown.contains("&#96;span&#96;"));
    for private in [
        "Private Owner",
        "private-owner",
        "unsafe.example",
        "server",
        "<script>",
        "![image]",
        "`span`",
    ] {
        assert!(
            !preview.markdown.contains(private),
            "private or active fragment retained: {private}"
        );
    }
    assert!(preview
        .omitted_sections
        .iter()
        .any(|section| section.starts_with("free_text_urls_and_local_paths")));
}

#[test]
fn typed_environment_origin_requires_explicit_selection() {
    let store = OwnerStore::open(&path()).unwrap();
    let owner_app = store.save_app(app()).unwrap();
    let environment = store
        .save_environment(SaveOwnerEnvironmentInput {
            operation_id: id(),
            id: id(),
            app_id: owner_app.id.clone(),
            expected_version: 0,
            label: "Local fixture".into(),
            preview_url: "http://localhost:4173/preview".into(),
            origins: vec![OriginDto {
                scheme: "http".into(),
                host: "localhost".into(),
                port: 4173,
            }],
            kind: EnvironmentKind::Local,
            revoked: false,
        })
        .unwrap();
    let candidate = store
        .save_candidate(SaveOwnerCandidateInput {
            operation_id: id(),
            id: id(),
            app_id: owner_app.id.clone(),
            environment_id: environment.id,
            environment_revision: environment.revision,
            identity: "owner fixture".into(),
            source: CandidateSource::OwnerLabel,
            trust: CandidateTrust::SelfReported,
            observed_at: 1,
        })
        .unwrap();
    let mut draft = change(&owner_app.id);
    draft.candidate_id = Some(candidate.id);
    let request = store.save_change_draft(draft).unwrap();
    let default_preview = store
        .render_brief(
            &owner_app.id,
            &request.id,
            OwnerBriefSelectionInput::default(),
        )
        .unwrap();
    assert!(!default_preview.markdown.contains("localhost"));
    let selected = store
        .render_brief(
            &owner_app.id,
            &request.id,
            OwnerBriefSelectionInput {
                include_environment_details: true,
                ..OwnerBriefSelectionInput::default()
            },
        )
        .unwrap();
    assert!(selected.markdown.contains("http://localhost:4173"));
    assert!(!selected.markdown.contains("/preview"));
}

#[test]
fn brief_uses_latest_confirmed_scenario_with_exact_typed_criterion() {
    let store = OwnerStore::open(&path()).unwrap();
    let owner_app = store.save_app(app()).unwrap();
    let capability = store
        .save_capability(SaveOwnerCapabilityInput {
            operation_id: id(),
            id: id(),
            app_id: owner_app.id.clone(),
            expected_version: 0,
            label: "Bookings".into(),
            description: String::new(),
            sort_order: 0,
            archived: false,
        })
        .unwrap();
    let mut scenario = ConfirmOwnerScenarioInput {
        operation_id: id(),
        id: id(),
        app_id: owner_app.id.clone(),
        capability_id: capability.id,
        expected_version: 0,
        title: "Booking detail".into(),
        expected_behavior: "The exact persisted booking is visible".into(),
        confirmed_by: "fixture owner".into(),
        method: VerificationMethod::OwnerManual,
        rules: Vec::new(),
        recipe: ScenarioRecipeDto {
            schema_version: 1,
            role_refs: Vec::new(),
            fixture_refs: Vec::new(),
            adapters: Vec::new(),
            steps: vec![RecipeStepDto {
                id: id(),
                action: RecipeActionDto::Navigate {
                    path: "/bookings".into(),
                },
                assertions: vec![NamedAssertionDto {
                    id: id(),
                    expected_behavior: "The booking status is correct".into(),
                    assertion: RecipeAssertionDto::TextEquals {
                        locator: LocatorDto::TestId {
                            value: "booking-status".into(),
                        },
                        expected: "pending".into(),
                    },
                }],
            }],
        },
    };
    store.confirm_scenario(scenario.clone()).unwrap();
    scenario.operation_id = id();
    scenario.expected_version = 1;
    scenario.title = "Booking detail current".into();
    if let RecipeAssertionDto::TextEquals { expected, .. } =
        &mut scenario.recipe.steps[0].assertions[0].assertion
    {
        *expected = "confirmed".into();
    }
    let current = store.confirm_scenario(scenario).unwrap();
    let request = store.save_change_draft(change(&owner_app.id)).unwrap();
    let preview = store
        .render_brief(
            &owner_app.id,
            &request.id,
            OwnerBriefSelectionInput::default(),
        )
        .unwrap();
    assert!(preview
        .markdown
        .contains("Booking detail current revision 2"));
    assert!(preview.markdown.contains("text equals `confirmed`"));
    assert!(!preview.markdown.contains("text equals `pending`"));
    assert!(preview.markdown.contains("OwnerManual"));
    let prior_selection = OwnerBriefSelectionInput {
        scenario_ids: vec![ScenarioRefDto {
            scenario_id: current.id,
            revision: 1,
        }],
        ..OwnerBriefSelectionInput::default()
    };
    let prior = store
        .render_brief(&owner_app.id, &request.id, prior_selection)
        .unwrap();
    assert!(prior.markdown.contains("text equals `pending`"));
    let missing = OwnerBriefSelectionInput {
        scenario_ids: vec![ScenarioRefDto {
            scenario_id: id(),
            revision: 1,
        }],
        ..OwnerBriefSelectionInput::default()
    };
    assert!(store
        .render_brief(&owner_app.id, &request.id, missing)
        .is_err());
}

#[test]
fn brief_selection_reports_omitted_sections_and_rejects_unknown_records() {
    let store = OwnerStore::open(&path()).unwrap();
    let owner_app = store.save_app(app()).unwrap();
    let request = store.save_change_draft(change(&owner_app.id)).unwrap();
    let selection = OwnerBriefSelectionInput {
        include_scenarios: false,
        scenario_ids: Vec::new(),
        include_candidate_identity: true,
        include_environment_details: true,
        include_limitations: false,
    };
    let preview = store
        .render_brief(&owner_app.id, &request.id, selection)
        .unwrap();
    assert!(preview
        .omitted_sections
        .iter()
        .any(|section| section.contains("acceptance_scenarios")));
    assert!(preview
        .omitted_sections
        .iter()
        .any(|section| section.contains("limitations")));
    assert!(store
        .render_brief(&owner_app.id, &id(), OwnerBriefSelectionInput::default())
        .is_err());
}

#[test]
fn brief_limit_is_an_explicit_error_and_never_partial_markdown() {
    let store = OwnerStore::open(&path()).unwrap();
    let mut owner_app = app();
    owner_app.description = "x".repeat(16 * 1024);
    let owner_app = store.save_app(owner_app).unwrap();
    let request = store
        .save_change_draft(SaveOwnerChangeInput {
            operation_id: id(),
            id: id(),
            app_id: owner_app.id.clone(),
            expected_version: 0,
            intent: "x".repeat(16 * 1024),
            example: "y".repeat(16 * 1024),
            scope: "z".repeat(16 * 1024),
            preserved_rules: Vec::new(),
            candidate_id: None,
            state: ChangeState::Draft,
        })
        .unwrap();
    let error = store
        .render_brief(
            &owner_app.id,
            &request.id,
            OwnerBriefSelectionInput::default(),
        )
        .unwrap_err();
    assert!(error.to_string().contains("32 KiB"));
}

#[test]
fn selection_revision_is_bounded_and_validated_before_read() {
    let store = OwnerStore::open(&path()).unwrap();
    let owner_app = store.save_app(app()).unwrap();
    let request = store.save_change_draft(change(&owner_app.id)).unwrap();
    let bad = OwnerBriefSelectionInput {
        include_scenarios: true,
        scenario_ids: vec![ScenarioRefDto {
            scenario_id: id(),
            revision: 0,
        }],
        ..OwnerBriefSelectionInput::default()
    };
    assert!(store.render_brief(&owner_app.id, &request.id, bad).is_err());
    let bad_id = OwnerBriefSelectionInput {
        include_scenarios: true,
        scenario_ids: vec![ScenarioRefDto {
            scenario_id: "not-an-id".into(),
            revision: 1,
        }],
        ..OwnerBriefSelectionInput::default()
    };
    assert!(store
        .render_brief(&owner_app.id, &request.id, bad_id)
        .is_err());
}
