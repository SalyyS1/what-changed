//! Real production mutations create only synthetic, temporary test records.
use super::*;

pub(super) fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub(super) fn create() -> (OwnerStore, StartOwnerManualRunInput) {
    let root = std::env::temp_dir().join(format!("ownvero-run-source-test-{}", id()));
    let store = OwnerStore::open(&root.join("owner-workspace.sqlite")).unwrap();
    let app = store
        .save_app(SaveOwnerAppInput {
            operation_id: id(),
            id: id(),
            expected_version: 0,
            name: "Synthetic source authority fixture".into(),
            description: String::new(),
            legacy_project_id: None,
            archived: false,
        })
        .unwrap();
    let capability = store
        .save_capability(SaveOwnerCapabilityInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            expected_version: 0,
            label: "Fixture surface".into(),
            description: String::new(),
            sort_order: 0,
            archived: false,
        })
        .unwrap();
    let environment = store
        .save_environment(SaveOwnerEnvironmentInput {
            operation_id: id(),
            id: id(),
            app_id: app.id.clone(),
            expected_version: 0,
            label: "Synthetic local test".into(),
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
            environment_id: environment.id,
            environment_revision: environment.revision,
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
            capability_id: capability.id,
            expected_version: 0,
            title: "Observe fixture surface".into(),
            expected_behavior: "The named surface is visible".into(),
            confirmed_by: "Fixture owner".into(),
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
                        expected_behavior: "Named surface is visible".into(),
                        assertion: RecipeAssertionDto::Visible {
                            locator: LocatorDto::TestId {
                                value: "fixture-surface".into(),
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
            intent: "Preserve fixture surface".into(),
            example: String::new(),
            scope: "Named assertion".into(),
            preserved_rules: vec![],
            candidate_id: Some(candidate.id.clone()),
            state: ChangeState::ReadyForReview,
        })
        .unwrap();
    let input = StartOwnerManualRunInput {
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
    };
    (store, input)
}
