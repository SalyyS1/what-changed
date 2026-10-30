//! Authoring and observation share the same typed-method and aggregate limits.

use super::*;

fn input(f: &Fixture, recipe: ScenarioRecipeDto) -> ConfirmOwnerScenarioInput {
    ConfirmOwnerScenarioInput {
        operation_id: id(),
        id: f.scenario.id.clone(),
        app_id: f.app.id.clone(),
        capability_id: f.scenario.capability_id.clone(),
        expected_version: f.scenario.revision,
        title: f.scenario.title.clone(),
        expected_behavior: f.scenario.expected_behavior.clone(),
        confirmed_by: "Owner".into(),
        method: VerificationMethod::OwnerManual,
        recipe,
        rules: f.scenario.rules.clone(),
    }
}

fn configured_recipe(f: &Fixture, denied: bool) -> ScenarioRecipeDto {
    let adapter_id = id();
    ScenarioRecipeDto {
        schema_version: 1,
        role_refs: vec!["staff-test".into()],
        fixture_refs: vec!["booking-test".into()],
        adapters: vec![ReadBackAdapterDto {
            id: adapter_id.clone(),
            origin: f.run.manifest.environment.origins[0].clone(),
            path_template: "/api/bookings/{fixture_id}".into(),
            role_ref: "staff-test".into(),
            allowed_fields: vec!["customer_name".into()],
            effect: if denied {
                AdapterEffect::ReadDenied
            } else {
                AdapterEffect::ReadOnly
            },
        }],
        steps: vec![RecipeStepDto {
            id: id(),
            action: if denied {
                RecipeActionDto::DenyOtherRole { adapter_id }
            } else {
                RecipeActionDto::ReadBack { adapter_id }
            },
            assertions: vec![NamedAssertionDto {
                id: id(),
                expected_behavior: "Check the named backend record using its declared role".into(),
                assertion: if denied {
                    RecipeAssertionDto::Denied {
                        allowed_statuses: vec![403, 404],
                        absent_fields: vec!["customer_name".into()],
                    }
                } else {
                    RecipeAssertionDto::FieldEquals {
                        field: "customer_name".into(),
                        fixture_field: "customer_name".into(),
                    }
                },
            }],
        }],
    }
}

fn install_recipe(f: &mut Fixture, recipe: ScenarioRecipeDto) {
    f.scenario = f.store.confirm_scenario(input(f, recipe)).unwrap();
    f.run = f
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
                revision: f.scenario.revision,
            }],
            retry_of: None,
        })
        .unwrap();
}

fn observations(f: &Fixture) -> RecordOwnerManualResultInput {
    let mut input = result(f, ResultStatus::Passed);
    input.scenario_revision = f.scenario.revision;
    input.observed_steps = f
        .scenario
        .recipe
        .steps
        .iter()
        .map(|step| ManualStepObservationDto {
            step_id: step.id.clone(),
            observed: "Used the declared action and named test record".into(),
        })
        .collect();
    input.assertions = f
        .scenario
        .recipe
        .steps
        .iter()
        .flat_map(|step| &step.assertions)
        .map(|assertion| ManualAssertionObservationInput {
            assertion_id: assertion.id.clone(),
            status: ResultStatus::Passed,
            observed: "Observed the expected selected field or denied response".into(),
            limitation: None,
        })
        .collect();
    input
}

#[test]
fn backend_assertions_require_matching_actions_routes_fields_and_named_roles() {
    let f = fixture();
    for case in [
        "missing-adapter",
        "field-on-navigation",
        "denied-on-navigation",
        "readback-on-denied-route",
        "denial-on-read-only-route",
        "unselected-response-field",
        "unselected-absent-field",
        "empty-role",
        "undeclared-role",
    ] {
        let denied = matches!(
            case,
            "denied-on-navigation" | "denial-on-read-only-route" | "unselected-absent-field"
        );
        let mut recipe = configured_recipe(&f, denied);
        match case {
            "missing-adapter" => recipe.adapters.clear(),
            "field-on-navigation" | "denied-on-navigation" => {
                recipe.steps[0].action = RecipeActionDto::Navigate {
                    path: "/bookings".into(),
                };
                recipe.adapters.clear();
                recipe.role_refs.clear();
            }
            "readback-on-denied-route" => recipe.adapters[0].effect = AdapterEffect::ReadDenied,
            "denial-on-read-only-route" => recipe.adapters[0].effect = AdapterEffect::ReadOnly,
            "unselected-response-field" | "unselected-absent-field" => {
                recipe.adapters[0].allowed_fields = vec!["status".into()]
            }
            "empty-role" => {
                recipe.role_refs = vec![String::new()];
                recipe.adapters[0].role_ref.clear();
            }
            "undeclared-role" => recipe.role_refs.clear(),
            _ => unreachable!(),
        }
        assert!(
            f.store.confirm_scenario(input(&f, recipe)).is_err(),
            "accepted {case}"
        );
    }
}

#[test]
fn configured_manual_readback_and_denial_remain_unknown_build_observations() {
    for denied in [false, true] {
        let mut f = fixture();
        let recipe = configured_recipe(&f, denied);
        install_recipe(&mut f, recipe);
        let saved = f.store.record_manual_result(observations(&f)).unwrap();
        assert_eq!(saved.status, ResultStatus::Passed);
        assert_eq!(saved.method, VerificationMethod::OwnerManual);
        finish(&f);
        let review = f.store.get_manual_run(&f.app.id, &f.run.id).unwrap();
        assert_eq!(review.status, ResultStatus::Passed);
        assert_eq!(review.freshness, EvidenceFreshness::Unknown);
        assert!(!review.can_accept);
        assert!(review.unresolved_scenarios.is_empty());
    }
}

fn bounded_recipe(f: &Fixture, assertions: usize) -> ScenarioRecipeDto {
    let mut recipe = f.scenario.recipe.clone();
    let template = &recipe.steps[0];
    recipe.steps = (0..assertions)
        .collect::<Vec<_>>()
        .chunks(100)
        .map(|chunk| {
            let mut step = template.clone();
            step.id = id();
            step.assertions = chunk
                .iter()
                .map(|_| {
                    let mut assertion = template.assertions[0].clone();
                    assertion.id = id();
                    assertion
                })
                .collect();
            step
        })
        .collect();
    recipe
}

#[test]
fn aggregate_assertion_budget_accepts_boundary_and_rejects_one_more_before_confirmation() {
    let f = fixture();
    let error = f
        .store
        .confirm_scenario(input(&f, bounded_recipe(&f, 1001)))
        .unwrap_err();
    assert!(error.to_string().contains("1000"), "{error}");
    for partial in [false, true] {
        let mut f = fixture();
        let recipe = bounded_recipe(&f, 1000);
        install_recipe(&mut f, recipe);
        let mut input = observations(&f);
        if partial {
            input.assertions.truncate(1);
        }
        let saved = f.store.record_manual_result(input).unwrap();
        assert_eq!(saved.assertions.len(), 1000);
        assert_eq!(
            saved.status,
            if partial {
                ResultStatus::NotRun
            } else {
                ResultStatus::Passed
            }
        );
    }
}

#[test]
fn frozen_environment_rejects_an_adapter_from_another_registered_environment() {
    let f = fixture();
    let mut recipe = configured_recipe(&f, false);
    recipe.adapters[0].origin.port += 1;
    f.store
        .save_environment(SaveOwnerEnvironmentInput {
            operation_id: id(),
            id: id(),
            app_id: f.app.id.clone(),
            expected_version: 0,
            label: "Other authorized preview".into(),
            preview_url: "http://localhost:4174/".into(),
            origins: vec![recipe.adapters[0].origin.clone()],
            kind: EnvironmentKind::Local,
            revoked: false,
        })
        .unwrap();
    let scenario = f.store.confirm_scenario(input(&f, recipe)).unwrap();
    let error = f
        .store
        .start_manual_run(StartOwnerManualRunInput {
            operation_id: id(),
            id: id(),
            app_id: f.app.id.clone(),
            change_id: f.change.id.clone(),
            expected_change_version: f.change.version,
            candidate_id: f.run.manifest.candidate.id.clone(),
            scenarios: vec![ScenarioRefDto {
                scenario_id: scenario.id,
                revision: scenario.revision,
            }],
            retry_of: None,
        })
        .unwrap_err();
    assert!(error.to_string().contains("origin"), "{error}");
}
