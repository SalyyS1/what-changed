//! Frozen manual evidence cannot self-approve or lose missing observations.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use what_changed_lib::owner_workspace::evidence::*;
use what_changed_lib::owner_workspace::*;

#[path = "owner_acceptance/recipe_contract.rs"]
mod recipe_contract;

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
struct Fixture {
    store: OwnerStore,
    app: OwnerAppDto,
    change: OwnerChangeDraftDto,
    scenario: ScenarioRevisionDto,
    run: OwnerManualRunDto,
}
fn fixture() -> Fixture {
    let folder = std::env::temp_dir().join(format!("what-changed-test-manual-{}", id()));
    std::fs::create_dir_all(&folder).unwrap();
    let store = OwnerStore::open(&folder.join("owner-workspace.sqlite")).unwrap();
    let app = store
        .save_app(SaveOwnerAppInput {
            operation_id: id(),
            id: id(),
            expected_version: 0,
            name: "Booking fixture".into(),
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
    let rule = store
        .confirm_rule(ConfirmOwnerRuleInput {
            operation_id: id(),
            rule_id: id(),
            app_id: app.id.clone(),
            capability_id: cap.id.clone(),
            expected_version: 0,
            expected_behavior: "Persist exact booking".into(),
            provenance: RuleProvenance::Owner,
            confirmed_by: "Owner".into(),
        })
        .unwrap();
    let env = store
        .save_environment(SaveOwnerEnvironmentInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            expected_version: 0,
            label: "Test".into(),
            preview_url: "http://localhost:4173/".into(),
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
            app_id: app.id.clone(),
            environment_id: env.id,
            environment_revision: 1,
            identity: "owner build label".into(),
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
    let rules = vec![RuleRefDto {
        rule_id: rule.rule_id,
        revision: 1,
    }];
    let scenario = store
        .confirm_scenario(ConfirmOwnerScenarioInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            capability_id: cap.id,
            expected_version: 0,
            title: "Read booking".into(),
            expected_behavior: "Exact booking is present".into(),
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
                    assertions: vec![
                        NamedAssertionDto {
                            id: id(),
                            expected_behavior: "Booking status".into(),
                            assertion: RecipeAssertionDto::TextEquals {
                                locator: LocatorDto::TestId {
                                    value: "status".into(),
                                },
                                expected: "confirmed".into(),
                            },
                        },
                        NamedAssertionDto {
                            id: id(),
                            expected_behavior: "Booking ID".into(),
                            assertion: RecipeAssertionDto::Visible {
                                locator: LocatorDto::TestId {
                                    value: "booking-id".into(),
                                },
                            },
                        },
                    ],
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
            intent: "Keep booking".into(),
            example: String::new(),
            scope: "Selected scenario".into(),
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
            expected_change_version: change.version,
            candidate_id: candidate.id,
            scenarios: vec![ScenarioRefDto {
                scenario_id: scenario.id.clone(),
                revision: scenario.revision,
            }],
            retry_of: None,
        })
        .unwrap();
    Fixture {
        store,
        app,
        change,
        scenario,
        run,
    }
}
fn result(f: &Fixture, status: ResultStatus) -> RecordOwnerManualResultInput {
    let step = &f.scenario.recipe.steps[0];
    RecordOwnerManualResultInput {
        operation_id: id(),
        id: id(),
        app_id: f.app.id.clone(),
        run_id: f.run.id.clone(),
        expected_run_version: 1,
        scenario_id: f.scenario.id.clone(),
        scenario_revision: 1,
        observer: "Owner".into(),
        observed_at: crate_time(),
        observed_steps: vec![ManualStepObservationDto {
            step_id: step.id.clone(),
            observed: "Opened the named booking route".into(),
        }],
        assertions: step
            .assertions
            .iter()
            .map(|assertion| ManualAssertionObservationInput {
                assertion_id: assertion.id.clone(),
                status,
                observed: "Observed exact record".into(),
                limitation: None,
            })
            .collect(),
        limitation: Some("Manual test fixture only".into()),
    }
}
fn crate_time() -> i64 {
    what_changed_lib::domain::now_ms()
}
fn finish(f: &Fixture) -> OwnerManualRunDto {
    f.store
        .finish_manual_run(FinishOwnerManualRunInput {
            operation_id: id(),
            app_id: f.app.id.clone(),
            run_id: f.run.id.clone(),
            expected_run_version: 1,
            lifecycle: RunLifecycle::Completed,
        })
        .unwrap()
}
fn decision(f: &Fixture, version: u32) -> RecordOwnerEvidenceDecisionInput {
    RecordOwnerEvidenceDecisionInput {
        operation_id: id(),
        id: id(),
        app_id: f.app.id.clone(),
        change_id: f.change.id.clone(),
        expected_change_version: f.change.version,
        run_id: f.run.id.clone(),
        expected_run_version: version,
        decision: DecisionKind::AcceptedWithExceptions,
        reason: "Owner accepts the explicit manual build limitation".into(),
        acknowledged_unresolved: vec![],
        acknowledge_unknown_build: true,
    }
}

#[test]
fn missing_assertion_is_not_run_and_empty_step_cannot_pass() {
    let f = fixture();
    let mut input = result(&f, ResultStatus::Passed);
    input.assertions.pop();
    let saved = f.store.record_manual_result(input.clone()).unwrap();
    assert_eq!(saved.status, ResultStatus::NotRun);
    assert_eq!(saved.assertions.len(), 2);
    assert!(saved.assertions[0].expected.contains("confirmed"));
    assert_eq!(f.store.record_manual_result(input).unwrap(), saved);
    finish(&f);
    let review = f.store.get_manual_run(&f.app.id, &f.run.id).unwrap();
    assert_eq!(review.counts.not_run, 1);
    assert!(!review.can_accept);
    assert_eq!(review.freshness, EvidenceFreshness::Unknown);
    let f = fixture();
    let mut input = result(&f, ResultStatus::Passed);
    input.observed_steps.clear();
    let saved = f.store.record_manual_result(input).unwrap();
    assert_eq!(saved.status, ResultStatus::NotRun);
    assert!(saved
        .assertions
        .iter()
        .all(|a| a.status == ResultStatus::NotRun));
}

#[test]
fn manual_pass_stays_unknown_and_acceptance_requires_explicit_exception() {
    let f = fixture();
    let saved = f
        .store
        .record_manual_result(result(&f, ResultStatus::Passed))
        .unwrap();
    assert_eq!(saved.status, ResultStatus::Passed);
    assert_eq!(saved.method, VerificationMethod::OwnerManual);
    let ended = finish(&f);
    let review = f.store.get_manual_run(&f.app.id, &f.run.id).unwrap();
    assert_eq!(review.status, ResultStatus::Passed);
    assert_eq!(review.freshness, EvidenceFreshness::Unknown);
    assert!(!review.can_accept);
    assert!(review.can_accept_with_exceptions);
    let mut ordinary = decision(&f, ended.version);
    ordinary.decision = DecisionKind::Accepted;
    assert!(f.store.record_owner_decision(ordinary).is_err());
    let mut no_ack = decision(&f, ended.version);
    no_ack.acknowledge_unknown_build = false;
    assert!(f.store.record_owner_decision(no_ack).is_err());
    let input = decision(&f, ended.version);
    let saved = f.store.record_owner_decision(input.clone()).unwrap();
    assert_eq!(saved.decision, DecisionKind::AcceptedWithExceptions);
    assert_eq!(f.store.record_owner_decision(input).unwrap(), saved);
}

#[test]
fn failure_remains_failed_after_owner_exception_and_requires_each_gap_ack() {
    let f = fixture();
    let saved = f
        .store
        .record_manual_result(result(&f, ResultStatus::Failed))
        .unwrap();
    let ended = finish(&f);
    let mut input = decision(&f, ended.version);
    assert!(f.store.record_owner_decision(input.clone()).is_err());
    input.operation_id = id();
    input.acknowledged_unresolved = vec![ScenarioRefDto {
        scenario_id: f.scenario.id.clone(),
        revision: 1,
    }];
    let recorded = f.store.record_owner_decision(input).unwrap();
    assert_eq!(recorded.decision, DecisionKind::AcceptedWithExceptions);
    let review = f.store.get_manual_run(&f.app.id, &f.run.id).unwrap();
    assert_eq!(review.status, ResultStatus::Failed);
    assert_eq!(review.scenarios[0].result.as_ref(), Some(&saved));
    assert_eq!(review.counts.failed, 1);
}

#[test]
fn observations_are_immutable_retries_are_separate_and_cancel_preserves_results() {
    let f = fixture();
    let first = result(&f, ResultStatus::Failed);
    f.store.record_manual_result(first.clone()).unwrap();
    let mut overwrite = first;
    overwrite.operation_id = id();
    overwrite.id = id();
    assert!(f.store.record_manual_result(overwrite).is_err());
    let ended = f
        .store
        .finish_manual_run(FinishOwnerManualRunInput {
            operation_id: id(),
            app_id: f.app.id.clone(),
            run_id: f.run.id.clone(),
            expected_run_version: 1,
            lifecycle: RunLifecycle::Cancelled,
        })
        .unwrap();
    assert_eq!(ended.lifecycle, RunLifecycle::Cancelled);
    assert_eq!(
        f.store.get_manual_run(&f.app.id, &f.run.id).unwrap().status,
        ResultStatus::Failed
    );
    let retry = f
        .store
        .start_manual_run(StartOwnerManualRunInput {
            operation_id: id(),
            id: id(),
            app_id: f.app.id.clone(),
            change_id: f.change.id.clone(),
            expected_change_version: f.change.version,
            candidate_id: f.run.manifest.candidate.id.clone(),
            scenarios: vec![ScenarioRefDto {
                scenario_id: f.scenario.id.clone(),
                revision: 1,
            }],
            retry_of: Some(f.run.id.clone()),
        })
        .unwrap();
    assert_ne!(retry.id, f.run.id);
    assert_eq!(retry.manifest_digest, f.run.manifest_digest);
    assert_eq!(
        f.store.get_manual_run(&f.app.id, &retry.id).unwrap().status,
        ResultStatus::NotRun
    );
    assert_eq!(
        f.store
            .list_manual_runs(&f.app.id, &f.change.id, OwnerPageInput::default())
            .unwrap()
            .items
            .len(),
        2
    );
}

#[test]
fn foreign_ids_unknown_assertions_and_operation_reuse_are_rejected() {
    let f = fixture();
    let other = f
        .store
        .save_app(SaveOwnerAppInput {
            operation_id: id(),
            id: id(),
            expected_version: 0,
            name: "Other app in same store".into(),
            description: String::new(),
            legacy_project_id: None,
            archived: false,
        })
        .unwrap();
    let mut foreign = result(&f, ResultStatus::Passed);
    foreign.app_id = other.id;
    assert!(f.store.record_manual_result(foreign).is_err());
    let mut unknown = result(&f, ResultStatus::Passed);
    unknown.assertions[0].assertion_id = id();
    assert!(f.store.record_manual_result(unknown).is_err());
    let input = result(&f, ResultStatus::Passed);
    f.store.record_manual_result(input.clone()).unwrap();
    let mut conflicting = input;
    conflicting.observer = "Different observer".into();
    assert!(f
        .store
        .record_manual_result(conflicting)
        .unwrap_err()
        .to_string()
        .contains("different request"));
}

#[test]
fn late_result_after_change_edit_commits_only_to_frozen_history() {
    let f = fixture();
    let frozen_digest = f.run.manifest_digest.clone();
    let newer = f
        .store
        .save_change_draft(SaveOwnerChangeInput {
            operation_id: id(),
            id: f.change.id.clone(),
            app_id: f.app.id.clone(),
            expected_version: f.change.version,
            intent: "A newly edited request while manual observation was pending".into(),
            example: f.change.example.clone(),
            scope: f.change.scope.clone(),
            preserved_rules: f.change.preserved_rules.clone(),
            candidate_id: f.change.candidate_id.clone(),
            state: f.change.state,
        })
        .unwrap();
    let late = f
        .store
        .record_manual_result(result(&f, ResultStatus::Passed))
        .unwrap();
    assert_eq!(late.run_id, f.run.id);
    assert_eq!(late.scenario_revision, f.scenario.revision);
    let ended = finish(&f);
    let review = f.store.get_manual_run(&f.app.id, &f.run.id).unwrap();
    assert_eq!(review.run.manifest_digest, frozen_digest);
    assert_eq!(review.run.manifest.change.version, f.change.version);
    assert_eq!(review.run.manifest.change.intent, f.change.intent);
    assert_eq!(review.scenarios[0].result.as_ref(), Some(&late));
    assert_eq!(review.status, ResultStatus::Passed);
    assert_eq!(review.freshness, EvidenceFreshness::Stale);
    assert!(!review.can_accept_with_exceptions);
    let mut attempt = decision(&f, ended.version);
    attempt.expected_change_version = newer.version;
    assert!(f.store.record_owner_decision(attempt).is_err());
    assert_eq!(
        f.store
            .get_change_draft(&f.app.id, &f.change.id)
            .unwrap()
            .unwrap(),
        newer
    );
}

#[test]
fn empty_observation_does_not_pass_and_sensitive_manual_text_is_scrubbed() {
    let f = fixture();
    let mut empty = result(&f, ResultStatus::Passed);
    empty.assertions[0].observed.clear();
    assert!(f.store.record_manual_result(empty).is_err());
    let mut input = result(&f, ResultStatus::Blocked);
    input.assertions[0].observed = "password=private-canary".into();
    input.observer = "token sk-abcdefghijklmnopqrstuvwx".into();
    let result = f.store.record_manual_result(input).unwrap();
    assert!(result.observer.contains("[REDACTED]"));
    assert!(!result.assertions[0].observed.contains("private-canary"));
    assert_eq!(result.status, ResultStatus::Blocked);
}
