//! Read, export and acceptance recompute freshness from live frozen dependencies.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::owner_workspace::evidence::*;
use what_changed_lib::owner_workspace::*;

#[path = "owner_evidence_freshness/recipe_contract.rs"]
mod recipe_contract;

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
struct Fixture {
    store: OwnerStore,
    app: OwnerAppDto,
    change: OwnerChangeDraftDto,
    rule: ConfirmOwnerRuleInput,
    scenario: ConfirmOwnerScenarioInput,
    environment: SaveOwnerEnvironmentInput,
    run: OwnerManualRunDto,
}
fn fixture() -> Fixture {
    let folder = std::env::temp_dir().join(format!("what-changed-test-freshness-{}", id()));
    std::fs::create_dir_all(&folder).unwrap();
    let store = OwnerStore::open(&folder.join("owner-workspace.sqlite")).unwrap();
    let app = store
        .save_app(SaveOwnerAppInput {
            operation_id: id(),
            id: id(),
            expected_version: 0,
            name: "App".into(),
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
    let rule = ConfirmOwnerRuleInput {
        operation_id: id(),
        rule_id: id(),
        app_id: app.id.clone(),
        capability_id: cap.id.clone(),
        expected_version: 0,
        expected_behavior: "Exact booking remains".into(),
        provenance: RuleProvenance::Owner,
        confirmed_by: "Owner".into(),
    };
    store.confirm_rule(rule.clone()).unwrap();
    let environment = SaveOwnerEnvironmentInput {
        operation_id: id(),
        id: id(),
        app_id: app.id.clone(),
        expected_version: 0,
        label: "Local".into(),
        preview_url: "http://localhost:4173/".into(),
        origins: vec![OriginDto {
            scheme: "http".into(),
            host: "localhost".into(),
            port: 4173,
        }],
        kind: EnvironmentKind::Local,
        revoked: false,
    };
    store.save_environment(environment.clone()).unwrap();
    let candidate = store
        .save_candidate(SaveOwnerCandidateInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            environment_id: environment.id.clone(),
            environment_revision: 1,
            identity: "owner label".into(),
            source: CandidateSource::OwnerLabel,
            trust: CandidateTrust::SelfReported,
            observed_at: 1,
        })
        .unwrap();
    let app = store
        .select_candidate(SelectOwnerCandidateInput {
            operation_id: id(),
            app_id: app.id,
            expected_version: 1,
            candidate_id: candidate.id.clone(),
        })
        .unwrap();
    let rules = vec![RuleRefDto {
        rule_id: rule.rule_id.clone(),
        revision: 1,
    }];
    let scenario = ConfirmOwnerScenarioInput {
        operation_id: id(),
        id: id(),
        app_id: app.id.clone(),
        capability_id: cap.id,
        expected_version: 0,
        title: "Check".into(),
        expected_behavior: "Exact booking".into(),
        confirmed_by: "Owner".into(),
        method: VerificationMethod::OwnerManual,
        rules: rules.clone(),
        recipe: ScenarioRecipeDto {
            schema_version: 1,
            role_refs: vec![],
            fixture_refs: vec![],
            adapters: vec![],
            steps: vec![RecipeStepDto {
                id: id(),
                action: RecipeActionDto::Navigate {
                    path: "/bookings".into(),
                },
                assertions: vec![NamedAssertionDto {
                    id: id(),
                    expected_behavior: "Present".into(),
                    assertion: RecipeAssertionDto::Visible {
                        locator: LocatorDto::TestId {
                            value: "booking".into(),
                        },
                    },
                }],
            }],
        },
    };
    store.confirm_scenario(scenario.clone()).unwrap();
    let change = store
        .save_change_draft(SaveOwnerChangeInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            expected_version: 0,
            intent: "Keep booking".into(),
            example: String::new(),
            scope: String::new(),
            preserved_rules: rules,
            candidate_id: Some(candidate.id.clone()),
            state: ChangeState::ReadyForReview,
        })
        .unwrap();
    let run = store
        .start_manual_run(StartOwnerManualRunInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            change_id: change.id.clone(),
            expected_change_version: 1,
            candidate_id: candidate.id,
            scenarios: vec![ScenarioRefDto {
                scenario_id: scenario.id.clone(),
                revision: 1,
            }],
            retry_of: None,
        })
        .unwrap();
    let step = &scenario.recipe.steps[0];
    store
        .record_manual_result(RecordOwnerManualResultInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            run_id: run.id.clone(),
            expected_run_version: 1,
            scenario_id: scenario.id.clone(),
            scenario_revision: 1,
            observer: "Owner".into(),
            observed_at: what_changed_lib::domain::now_ms(),
            observed_steps: vec![ManualStepObservationDto {
                step_id: step.id.clone(),
                observed: "Viewed exact booking".into(),
            }],
            assertions: vec![ManualAssertionObservationInput {
                assertion_id: step.assertions[0].id.clone(),
                status: ResultStatus::Passed,
                observed: "Named booking is visible".into(),
                limitation: None,
            }],
            limitation: None,
        })
        .unwrap();
    let run = store
        .finish_manual_run(FinishOwnerManualRunInput {
            operation_id: id(),
            app_id: app.id.clone(),
            run_id: run.id,
            expected_run_version: 1,
            lifecycle: RunLifecycle::Completed,
        })
        .unwrap();
    Fixture {
        store,
        app,
        change,
        rule,
        scenario,
        environment,
        run,
    }
}
fn decision(f: &Fixture, change_version: u32) -> RecordOwnerEvidenceDecisionInput {
    RecordOwnerEvidenceDecisionInput {
        operation_id: id(),
        id: id(),
        app_id: f.app.id.clone(),
        change_id: f.change.id.clone(),
        expected_change_version: change_version,
        run_id: f.run.id.clone(),
        expected_run_version: f.run.version,
        decision: DecisionKind::AcceptedWithExceptions,
        reason: "Acknowledge manual unknown-build observation".into(),
        acknowledged_unresolved: vec![],
        acknowledge_unknown_build: true,
    }
}

#[test]
fn every_live_dependency_edit_stales_read_export_and_acceptance() {
    for kind in [
        "rule",
        "scenario",
        "locator",
        "fixture",
        "environment",
        "revocation",
        "change",
        "candidate",
        "semantics",
    ] {
        let mut f = fixture();
        assert_eq!(
            f.store
                .get_manual_run(&f.app.id, &f.run.id)
                .unwrap()
                .freshness,
            EvidenceFreshness::Unknown
        );
        let before = f.store.record_owner_decision(decision(&f, 1)).unwrap();
        let mut change_version = 1;
        match kind {
            "rule" => {
                f.rule.operation_id = id();
                f.rule.expected_version = 1;
                f.rule.expected_behavior = "Changed owner expectation".into();
                f.store.confirm_rule(f.rule.clone()).unwrap();
            }
            "scenario" | "locator" | "fixture" => {
                f.scenario.operation_id = id();
                f.scenario.expected_version = 1;
                if kind == "scenario" {
                    f.scenario.expected_behavior = "Changed expectation".into();
                } else if kind == "locator" {
                    f.scenario.recipe.steps[0].action = RecipeActionDto::Click {
                        locator: LocatorDto::TestId {
                            value: "new-locator".into(),
                        },
                    };
                } else {
                    f.scenario
                        .recipe
                        .fixture_refs
                        .push("new-fixture-schema".into());
                }
                f.store.confirm_scenario(f.scenario.clone()).unwrap();
            }
            "environment" | "revocation" => {
                f.environment.operation_id = id();
                f.environment.expected_version = 1;
                if kind == "environment" {
                    f.environment.label = "New environment config".into();
                } else {
                    f.environment.revoked = true;
                }
                f.store.save_environment(f.environment.clone()).unwrap();
            }
            "change" => {
                let changed = f
                    .store
                    .save_change_draft(SaveOwnerChangeInput {
                        operation_id: id(),
                        id: f.change.id.clone(),
                        app_id: f.app.id.clone(),
                        expected_version: 1,
                        intent: "Edited intent".into(),
                        example: f.change.example.clone(),
                        scope: f.change.scope.clone(),
                        preserved_rules: f.change.preserved_rules.clone(),
                        candidate_id: f.change.candidate_id.clone(),
                        state: f.change.state,
                    })
                    .unwrap();
                change_version = changed.version;
            }
            "candidate" => {
                let candidate = f
                    .store
                    .save_candidate(SaveOwnerCandidateInput {
                        operation_id: id(),
                        id: id(),
                        app_id: f.app.id.clone(),
                        environment_id: f.environment.id.clone(),
                        environment_revision: 1,
                        identity: "another reported build".into(),
                        source: CandidateSource::OwnerLabel,
                        trust: CandidateTrust::SelfReported,
                        observed_at: 1,
                    })
                    .unwrap();
                f.store
                    .select_candidate(SelectOwnerCandidateInput {
                        operation_id: id(),
                        app_id: f.app.id.clone(),
                        expected_version: f.app.version,
                        candidate_id: candidate.id,
                    })
                    .unwrap();
            }
            "semantics" => {
                use sha2::{Digest, Sha256};
                let mut run = f.run.clone();
                run.manifest.semantics_version = "owner-manual/v0".into();
                run.manifest_digest = Sha256::digest(serde_json::to_vec(&run.manifest).unwrap())
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                let epoch = f.store.capture_epoch().unwrap();
                f.store
                    .write_at_epoch(epoch, move |tx| {
                        tx.execute(
                            "UPDATE owner_runs SET payload=?2 WHERE id=?1",
                            rusqlite::params![run.id, serde_json::to_string(&run).unwrap()],
                        )
                        .unwrap();
                        Ok(())
                    })
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let current = f.store.get_manual_run(&f.app.id, &f.run.id).unwrap();
        assert_eq!(
            current.freshness,
            EvidenceFreshness::Stale,
            "dependency {kind}"
        );
        assert!(!current.can_accept_with_exceptions);
        assert_eq!(current.status, ResultStatus::Passed);
        assert_eq!(current.decisions[0].id, before.id);
        assert_eq!(
            current.decisions[0].current_freshness,
            EvidenceFreshness::Stale
        );
        assert_eq!(
            f.store
                .export_manual_run_review(&f.app.id, &f.run.id)
                .unwrap()
                .freshness,
            EvidenceFreshness::Stale
        );
        assert!(f
            .store
            .record_owner_decision(decision(&f, change_version))
            .is_err());
        assert_eq!(
            f.store
                .list_owner_decisions(&f.app.id, &f.change.id, OwnerPageInput::default())
                .unwrap()
                .items[0]
                .current_freshness,
            EvidenceFreshness::Stale
        );
    }
}

#[test]
fn stale_late_result_stays_in_original_history_and_cannot_target_another_candidate() {
    let f = fixture();
    let original = f.run.manifest_digest.clone();
    let changed = f
        .store
        .save_change_draft(SaveOwnerChangeInput {
            operation_id: id(),
            id: f.change.id.clone(),
            app_id: f.app.id.clone(),
            expected_version: 1,
            intent: "New intent".into(),
            example: String::new(),
            scope: String::new(),
            preserved_rules: f.change.preserved_rules.clone(),
            candidate_id: f.change.candidate_id.clone(),
            state: ChangeState::Draft,
        })
        .unwrap();
    let read = f.store.get_manual_run(&f.app.id, &f.run.id).unwrap();
    assert_eq!(read.run.manifest.change.version, 1);
    assert_eq!(read.run.manifest_digest, original);
    assert_eq!(read.freshness, EvidenceFreshness::Stale);
    assert_eq!(changed.version, 2);
    assert!(f
        .store
        .start_manual_run(StartOwnerManualRunInput {
            operation_id: id(),
            id: id(),
            app_id: f.app.id.clone(),
            change_id: f.change.id.clone(),
            expected_change_version: 2,
            candidate_id: f.run.manifest.candidate.id.clone(),
            scenarios: vec![ScenarioRefDto {
                scenario_id: f.scenario.id,
                revision: 1
            }],
            retry_of: Some(f.run.id)
        })
        .is_err());
}
