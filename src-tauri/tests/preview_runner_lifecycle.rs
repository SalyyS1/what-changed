//! Real owner persistence and native consent, without spawning an unauthorized target.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use std::sync::Arc;
use what_changed_lib::app::{paths::AppPaths, AppState};
use what_changed_lib::owner_workspace::evidence::*;
use what_changed_lib::owner_workspace::*;
use what_changed_lib::preview_runner::*;

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn fixture() -> (AppState, PreparePreviewRunInput) {
    let folder = std::env::temp_dir().join(format!("ownvero-runner-test-{}", id()));
    let state = AppState::open(AppPaths::under(&folder)).unwrap();
    let store = &state.owner_store;
    let app = store
        .save_app(SaveOwnerAppInput {
            operation_id: id(),
            id: id(),
            expected_version: 0,
            name: "Synthetic booking".into(),
            description: String::new(),
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
            label: "Booking".into(),
            description: String::new(),
            sort_order: 0,
            archived: false,
        })
        .unwrap();
    let env = store
        .save_environment(SaveOwnerEnvironmentInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            expected_version: 0,
            label: "Test".into(),
            preview_url: "http://127.0.0.1:5185/".into(),
            origins: vec![OriginDto {
                scheme: "http".into(),
                host: "127.0.0.1".into(),
                port: 5185,
            }],
            kind: EnvironmentKind::Local,
            revoked: false,
        })
        .unwrap();
    let candidate = store
        .save_candidate(SaveOwnerCandidateInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            environment_id: env.id,
            environment_revision: env.revision,
            identity: "fixture-v1".into(),
            source: CandidateSource::OwnerLabel,
            trust: CandidateTrust::SelfReported,
            observed_at: 1,
        })
        .unwrap();
    let app = store
        .select_candidate(SelectOwnerCandidateInput {
            operation_id: id(),
            app_id: app.id,
            expected_version: app.version,
            candidate_id: candidate.id.clone(),
        })
        .unwrap();
    let scenario = store
        .confirm_scenario(ConfirmOwnerScenarioInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            capability_id: cap.id,
            expected_version: 0,
            title: "Observe booking".into(),
            expected_behavior: "Booking is visible".into(),
            confirmed_by: "Owner".into(),
            method: VerificationMethod::Automated,
            rules: vec![],
            recipe: ScenarioRecipeDto {
                schema_version: 1,
                role_refs: vec!["staff".into()],
                fixture_refs: vec![],
                adapters: vec![],
                steps: vec![RecipeStepDto {
                    id: id(),
                    action: RecipeActionDto::Navigate { path: "/".into() },
                    assertions: vec![NamedAssertionDto {
                        id: id(),
                        expected_behavior: "Booking form is visible".into(),
                        assertion: RecipeAssertionDto::Visible {
                            locator: LocatorDto::TestId {
                                value: "booking".into(),
                            },
                        },
                    }],
                }],
            },
        })
        .unwrap();
    let change = store
        .save_change_draft(SaveOwnerChangeInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            expected_version: 0,
            intent: "Preserve booking".into(),
            example: String::new(),
            scope: "Named assertion".into(),
            preserved_rules: vec![],
            candidate_id: Some(candidate.id.clone()),
            state: ChangeState::ReadyForReview,
        })
        .unwrap();
    let input = PreparePreviewRunInput {
        run: StartOwnerManualRunInput {
            operation_id: id(),
            id: id(),
            app_id: app.id,
            change_id: change.id,
            expected_change_version: change.version,
            candidate_id: candidate.id,
            scenarios: vec![ScenarioRefDto {
                scenario_id: scenario.id,
                revision: scenario.revision,
            }],
            retry_of: None,
        },
        routes: vec![RunnerRouteDto {
            origin: "http://127.0.0.1:5185".into(),
            method: "GET".into(),
            path: "/".into(),
        }],
        fixture: Default::default(),
        mode: "run".into(),
        capture_screenshots: false,
    };
    (state, input)
}
fn grant(plan: &PreviewRunPlanDto) -> GrantPreviewRunInput {
    GrantPreviewRunInput {
        consent_id: plan.consent_id.clone(),
        scope_digest: plan.scope_digest.clone(),
        authorized_test_environment: true,
        isolated_test_data: true,
        acknowledge_temporary_auth: true,
    }
}

#[test]
fn one_run_consent_cannot_reuse_changed_scope_and_missing_bundle_never_passes() {
    let (state, input) = fixture();
    let runner = &state.preview_runner;
    let plan = runner.prepare(&state.owner_store, input.clone()).unwrap();
    let mut wrong = grant(&plan);
    wrong.scope_digest = "0".repeat(64);
    assert!(runner
        .start(
            Arc::clone(&state.owner_store),
            Arc::clone(&state.data_lifecycle),
            state.paths.data.join("missing-resources"),
            state.paths.data.clone(),
            wrong
        )
        .is_err());
    let started = runner
        .start(
            Arc::clone(&state.owner_store),
            Arc::clone(&state.data_lifecycle),
            state.paths.data.join("missing-resources"),
            state.paths.data.clone(),
            grant(&plan),
        )
        .unwrap();
    for _ in 0..100 {
        if runner
            .status(&started.app_id, &started.run_id)
            .unwrap()
            .lifecycle
            == "interrupted"
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let status = runner.status(&started.app_id, &started.run_id).unwrap();
    assert_eq!(status.lifecycle, "interrupted");
    let review = state
        .owner_store
        .get_manual_run(&started.app_id, &started.run_id)
        .unwrap();
    assert_eq!(review.run.method, VerificationMethod::Automated);
    assert!(!review.can_accept);
    assert_ne!(review.status, ResultStatus::Passed);
    assert!(runner
        .start(
            Arc::clone(&state.owner_store),
            Arc::clone(&state.data_lifecycle),
            state.paths.data.clone(),
            state.paths.data.clone(),
            grant(&plan)
        )
        .is_err());
    let finished = state
        .owner_store
        .finish_manual_run(FinishOwnerManualRunInput {
            operation_id: id(),
            app_id: started.app_id,
            run_id: started.run_id,
            expected_run_version: review.run.version,
            lifecycle: RunLifecycle::Completed,
        });
    assert!(matches!(
        finished,
        Err(what_changed_lib::error::AppError::InvalidInput {
            what: "run method",
            ..
        })
    ));
    state.shutdown_cleanly();
}

#[test]
fn reviewed_export_is_runnable_and_does_not_include_consent_or_login_storage() {
    let (state, input) = fixture();
    let preview = export_recipe(&state.owner_store, input.clone()).unwrap();
    assert_eq!(
        preview.digest,
        export_recipe(&state.owner_store, input).unwrap().digest
    );
    let names = preview
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    for required in [
        "package.json",
        "src/run.mjs",
        "manifest.json",
        "fixture.json",
        "README.md",
        "fixtures/booking-app/server.mjs",
    ] {
        assert!(names.contains(&required));
    }
    let manifest = &preview
        .files
        .iter()
        .find(|file| file.path == "manifest.json")
        .unwrap()
        .content;
    assert!(!manifest.contains("consent_id"));
    assert!(!manifest.contains("storageState"));
    assert!(recipe_archive(&preview).unwrap().starts_with(b"PK\x03\x04"));
    state.shutdown_cleanly();
}

#[test]
fn permissions_expire_when_environment_changes_before_grant() {
    let (state, input) = fixture();
    let plan = state
        .preview_runner
        .prepare(&state.owner_store, input.clone())
        .unwrap();
    let env = &plan.manifest.environment;
    state
        .owner_store
        .save_environment(SaveOwnerEnvironmentInput {
            operation_id: id(),
            id: env.id.clone(),
            app_id: env.app_id.clone(),
            expected_version: env.revision,
            label: env.label.clone(),
            preview_url: env.preview_url.clone(),
            origins: env.origins.clone(),
            kind: env.kind,
            revoked: true,
        })
        .unwrap();
    assert!(state
        .preview_runner
        .start(
            Arc::clone(&state.owner_store),
            Arc::clone(&state.data_lifecycle),
            state.paths.data.clone(),
            state.paths.data.clone(),
            grant(&plan)
        )
        .is_err());
    assert!(state
        .owner_store
        .get_manual_run(&input.run.app_id, &input.run.id)
        .is_err());
    state.shutdown_cleanly();
}
