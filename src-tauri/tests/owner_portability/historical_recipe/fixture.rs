use super::*;
use what_changed_lib::dto::owner_evidence_dto::StartOwnerManualRunInput;

pub(super) fn scenario(
    store: &OwnerStore,
    app: &str,
    cap: &str,
    rule: &OwnerRuleRevisionDto,
) -> ScenarioRevisionDto {
    store
        .confirm_scenario(ConfirmOwnerScenarioInput {
            operation_id: id(),
            id: id(),
            app_id: app.into(),
            capability_id: cap.into(),
            expected_version: 0,
            title: "Historical booking check".into(),
            expected_behavior: "Named booking remains visible".into(),
            confirmed_by: "Owner".into(),
            method: VerificationMethod::OwnerManual,
            rules: vec![RuleRefDto {
                rule_id: rule.rule_id.clone(),
                revision: rule.revision,
            }],
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
                        expected_behavior: "Named booking".into(),
                        assertion: RecipeAssertionDto::Visible {
                            locator: LocatorDto::TestId {
                                value: "booking".into(),
                            },
                        },
                    }],
                }],
            },
        })
        .unwrap()
}

pub(super) fn start_request(
    store: &OwnerStore,
    app: &str,
    scenario: &ScenarioRevisionDto,
    rule: &OwnerRuleRevisionDto,
) -> StartOwnerManualRunInput {
    let environment = store
        .save_environment(SaveOwnerEnvironmentInput {
            operation_id: id(),
            id: id(),
            app_id: app.into(),
            expected_version: 0,
            label: "Local fixture".into(),
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
            app_id: app.into(),
            environment_id: environment.id,
            environment_revision: environment.revision,
            identity: "owner label".into(),
            source: CandidateSource::OwnerLabel,
            trust: CandidateTrust::SelfReported,
            observed_at: 1,
        })
        .unwrap();
    store
        .select_candidate(SelectOwnerCandidateInput {
            operation_id: id(),
            app_id: app.into(),
            expected_version: 1,
            candidate_id: candidate.id.clone(),
        })
        .unwrap();
    let change = store
        .save_change_draft(SaveOwnerChangeInput {
            operation_id: id(),
            id: id(),
            app_id: app.into(),
            expected_version: 0,
            intent: "Check preserved booking".into(),
            example: String::new(),
            scope: "Named fixture".into(),
            preserved_rules: vec![RuleRefDto {
                rule_id: rule.rule_id.clone(),
                revision: rule.revision,
            }],
            candidate_id: Some(candidate.id.clone()),
            state: ChangeState::ReadyForReview,
        })
        .unwrap();
    StartOwnerManualRunInput {
        operation_id: id(),
        id: id(),
        app_id: app.into(),
        change_id: change.id,
        expected_change_version: change.version,
        candidate_id: candidate.id,
        scenarios: vec![ScenarioRefDto {
            scenario_id: scenario.id.clone(),
            revision: scenario.revision,
        }],
        retry_of: None,
    }
}
