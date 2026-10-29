//! Portable history preserves old definitions without granting active test authority.

use super::*;

#[path = "historical_recipe/fixture.rs"]
mod fixture;

fn old_recipe(original: &ScenarioRecipeDto, oversized: bool) -> ScenarioRecipeDto {
    let mut recipe = original.clone();
    recipe.steps[0].assertions[0].assertion = RecipeAssertionDto::FieldEquals {
        field: "customer_name".into(),
        fixture_field: "customer_name".into(),
    };
    if oversized {
        let template = recipe.steps[0].clone();
        recipe.steps = (0..11)
            .map(|_| {
                let mut step = template.clone();
                step.id = id();
                step.assertions = (0..100)
                    .map(|_| {
                        let mut assertion = template.assertions[0].clone();
                        assertion.id = id();
                        assertion
                    })
                    .collect();
                step
            })
            .collect();
    }
    recipe
}

#[test]
fn malformed_historical_recipe_exports_round_trips_and_never_becomes_an_active_scenario() {
    for oversized in [false, true] {
        let (source, app, cap, rule) = super::fixture();
        let mut scenario = fixture::scenario(&source, &app, &cap, &rule);
        scenario.recipe = old_recipe(&scenario.recipe, oversized);
        let saved = scenario.clone();
        source
            .write_at_epoch(source.capture_epoch().unwrap(), move |tx| {
                // Simulate an immutable revision accepted by an earlier validator.
                // Only this fixture temporarily lifts and restores the trigger.
                let trigger: String = tx
                    .query_row(
                        "SELECT sql FROM sqlite_master WHERE name='immutable_owner_scenarios'",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap();
                tx.execute_batch("DROP TRIGGER immutable_owner_scenarios")
                    .unwrap();
                tx.execute(
                    "UPDATE owner_scenario_revisions SET payload=?3 WHERE id=?1 AND revision=?2",
                    rusqlite::params![
                        saved.id,
                        saved.revision,
                        serde_json::to_string(&saved).unwrap()
                    ],
                )
                .unwrap();
                tx.execute_batch(&trigger).unwrap();
                Ok(())
            })
            .unwrap();
        let start = fixture::start_request(&source, &app, &scenario, &rule);
        assert!(source.start_manual_run(start).is_err());

        let selection = OwnerExportSelectionInput {
            include_target_details: false,
        };
        let (bytes, preview) =
            prepare_bundle(source.portable_snapshot(&app).unwrap(), &selection).unwrap();
        let data = validate_bundle(&bytes).unwrap();
        assert_eq!(data.records.scenarios[0].recipe, scenario.recipe);
        assert!(preview.markdown.contains("Portable project memory"));
        let target = store();
        let input = apply_input(&data, preview.digest);
        let imported = target.apply_portable_import(data, input, id()).unwrap();
        let snapshot = target.portable_snapshot(&imported.app.id).unwrap();
        assert!(snapshot.records.scenarios.is_empty());
        assert!(snapshot.records.runs.is_empty());
        assert!(snapshot.records.candidates.is_empty());
        assert!(snapshot.records.environments.is_empty());
        assert_eq!(
            snapshot.history[0].records.scenarios[0].recipe,
            scenario.recipe
        );
        assert_eq!(
            snapshot.records.rules[0].provenance,
            RuleProvenance::ImportedUntrusted
        );
        let (again, _) = prepare_bundle(snapshot.clone(), &selection).unwrap();
        let second_export = validate_bundle(&again).unwrap();
        assert_eq!(
            second_export.history[0].records.scenarios[0].recipe,
            scenario.recipe
        );

        // Even explicit owner reconfirmation must repair the method/capacity;
        // import cannot activate this archival definition by changing its IDs.
        let attempt = ConfirmOwnerScenarioInput {
            operation_id: id(),
            id: id(),
            app_id: imported.app.id.clone(),
            capability_id: snapshot.records.capabilities[0].id.clone(),
            expected_version: 0,
            title: scenario.title.clone(),
            expected_behavior: scenario.expected_behavior.clone(),
            confirmed_by: "Owner".into(),
            method: VerificationMethod::OwnerManual,
            recipe: scenario.recipe.clone(),
            rules: vec![],
        };
        assert!(target.confirm_scenario(attempt).is_err());
    }
}

#[test]
fn archival_mode_keeps_schema_routes_ids_and_structural_bounds_closed() {
    let (source, app, cap, rule) = super::fixture();
    fixture::scenario(&source, &app, &cap, &rule);
    let snapshot = source.portable_snapshot(&app).unwrap();
    for case in [
        "unknown-version",
        "unsafe-route",
        "invalid-step-id",
        "duplicate-check-id",
        "too-many-steps",
        "too-many-checks-in-step",
    ] {
        let mut data = snapshot.clone();
        let recipe = &mut data.records.scenarios[0].recipe;
        match case {
            "unknown-version" => recipe.schema_version = 2,
            "unsafe-route" => {
                recipe.steps[0].action = RecipeActionDto::Navigate {
                    path: "//unapproved.test/path".into(),
                }
            }
            "invalid-step-id" => recipe.steps[0].id = "not-a-uuid".into(),
            "duplicate-check-id" => {
                let duplicate = recipe.steps[0].assertions[0].clone();
                recipe.steps[0].assertions.push(duplicate);
            }
            "too-many-steps" => {
                let step = recipe.steps[0].clone();
                recipe.steps = (0..51)
                    .map(|_| {
                        let mut next = step.clone();
                        next.id = id();
                        next.assertions[0].id = id();
                        next
                    })
                    .collect();
            }
            "too-many-checks-in-step" => {
                let check = recipe.steps[0].assertions[0].clone();
                recipe.steps[0].assertions = (0..101)
                    .map(|_| {
                        let mut next = check.clone();
                        next.id = id();
                        next
                    })
                    .collect();
            }
            _ => unreachable!(),
        }
        assert!(
            prepare_bundle(
                data,
                &OwnerExportSelectionInput {
                    include_target_details: false
                }
            )
            .is_err(),
            "accepted unsafe archival {case}"
        );
    }
}
