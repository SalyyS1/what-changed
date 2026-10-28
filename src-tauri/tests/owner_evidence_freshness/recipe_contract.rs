//! Historical raw observations are immutable; current semantic projection fails closed.

use super::*;
use sha2::{Digest, Sha256};

fn historical_recipe(f: &Fixture, case: &str) -> ScenarioRecipeDto {
    let mut recipe = f.run.manifest.scenarios[0].recipe.clone();
    match case {
        "missing-readback" => {
            recipe.steps[0].assertions[0].assertion = RecipeAssertionDto::FieldEquals {
                field: "customer_name".into(),
                fixture_field: "customer_name".into(),
            };
        }
        "foreign-origin" => {
            let adapter_id = id();
            let mut origin = f.run.manifest.environment.origins[0].clone();
            origin.port += 1;
            recipe.role_refs = vec!["staff-test".into()];
            recipe.adapters = vec![ReadBackAdapterDto {
                id: adapter_id.clone(),
                origin,
                path_template: "/api/bookings/{fixture_id}".into(),
                role_ref: "staff-test".into(),
                allowed_fields: vec!["customer_name".into()],
                effect: AdapterEffect::ReadOnly,
            }];
            recipe.steps[0].action = RecipeActionDto::ReadBack { adapter_id };
            recipe.steps[0].assertions[0].assertion = RecipeAssertionDto::FieldEquals {
                field: "customer_name".into(),
                fixture_field: "customer_name".into(),
            };
        }
        "over-budget" => {
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
        _ => unreachable!(),
    }
    recipe
}

fn seed_historical_recipe(f: &mut Fixture, recipe: ScenarioRecipeDto) {
    // Simulate a snapshot written before these recipe constraints existed.
    // The original observation row deliberately remains byte-for-byte unchanged.
    f.run.manifest.scenarios[0].recipe = recipe;
    f.run.manifest_digest = Sha256::digest(serde_json::to_vec(&f.run.manifest).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let run = f.run.clone();
    let scenario = run.manifest.scenarios[0].clone();
    let epoch = f.store.capture_epoch().unwrap();
    f.store
        .write_at_epoch(epoch, move |tx| {
            // Seed an older application's snapshot only in this isolated test.
            // Restore its real immutability trigger in the same transaction.
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
                "UPDATE owner_runs SET payload=?2 WHERE id=?1",
                rusqlite::params![run.id, serde_json::to_string(&run).unwrap()],
            )
            .unwrap();
            tx.execute(
                "UPDATE owner_scenario_revisions SET payload=?3 WHERE id=?1 AND revision=?2",
                rusqlite::params![
                    scenario.id,
                    scenario.revision,
                    serde_json::to_string(&scenario).unwrap()
                ],
            )
            .unwrap();
            tx.execute_batch(&trigger).unwrap();
            Ok(())
        })
        .unwrap();
}

#[test]
fn unsupported_historical_recipes_are_blocked_without_rewriting_raw_observations() {
    for case in ["missing-readback", "foreign-origin", "over-budget"] {
        let mut f = fixture();
        let original = f
            .store
            .get_manual_run(&f.app.id, &f.run.id)
            .unwrap()
            .scenarios[0]
            .result
            .clone()
            .unwrap();
        let recipe = historical_recipe(&f, case);
        seed_historical_recipe(&mut f, recipe);
        let review = f.store.get_manual_run(&f.app.id, &f.run.id).unwrap();
        assert_eq!(review.status, ResultStatus::Blocked, "{case}");
        assert_eq!(review.counts.blocked, 1, "{case}");
        assert_eq!(review.counts.passed, 0, "{case}");
        assert_eq!(review.scenarios[0].result.as_ref(), Some(&original));
        assert_eq!(original.status, ResultStatus::Passed);
        assert_eq!(review.freshness, EvidenceFreshness::Unknown);
        assert!(!review.can_accept);
        assert!(review.can_accept_with_exceptions);
        assert!(review
            .limitations
            .iter()
            .any(|text| text.contains("recipe") && text.contains("preserved")));
        assert_eq!(
            review.unresolved_scenarios,
            vec![ScenarioRefDto {
                scenario_id: f.scenario.id.clone(),
                revision: 1
            }]
        );
        assert!(f.store.record_owner_decision(decision(&f, 1)).is_err());
        let mut acknowledged = decision(&f, 1);
        acknowledged.acknowledged_unresolved = review.unresolved_scenarios.clone();
        f.store.record_owner_decision(acknowledged).unwrap();
        let exported = f
            .store
            .export_manual_run_review(&f.app.id, &f.run.id)
            .unwrap();
        assert_eq!(exported.status, ResultStatus::Blocked);
        assert_eq!(exported.scenarios[0].result.as_ref(), Some(&original));
        assert!(
            f.store
                .start_manual_run(StartOwnerManualRunInput {
                    operation_id: id(),
                    id: id(),
                    app_id: f.app.id.clone(),
                    change_id: f.change.id.clone(),
                    expected_change_version: f.change.version,
                    candidate_id: f.run.manifest.candidate.id.clone(),
                    scenarios: vec![ScenarioRefDto {
                        scenario_id: f.scenario.id.clone(),
                        revision: 1
                    }],
                    retry_of: Some(f.run.id.clone()),
                })
                .is_err(),
            "invalid retry admitted for {case}"
        );
    }
}

#[test]
fn unsupported_recipe_does_not_hide_a_recorded_failure() {
    let mut f = fixture();
    let mut original = f
        .store
        .get_manual_run(&f.app.id, &f.run.id)
        .unwrap()
        .scenarios[0]
        .result
        .clone()
        .unwrap();
    original.status = ResultStatus::Failed;
    original.assertions[0].status = ResultStatus::Failed;
    original.assertions[0].observed = "The recorded check failed".into();
    let stored = original.clone();
    let epoch = f.store.capture_epoch().unwrap();
    f.store
        .write_at_epoch(epoch, move |tx| {
            let trigger: String = tx
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE name='immutable_owner_results'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            tx.execute_batch("DROP TRIGGER immutable_owner_results")
                .unwrap();
            tx.execute(
                "UPDATE owner_results SET payload=?2 WHERE id=?1",
                rusqlite::params![stored.id, serde_json::to_string(&stored).unwrap()],
            )
            .unwrap();
            tx.execute_batch(&trigger).unwrap();
            Ok(())
        })
        .unwrap();
    let recipe = historical_recipe(&f, "missing-readback");
    seed_historical_recipe(&mut f, recipe);
    let review = f
        .store
        .export_manual_run_review(&f.app.id, &f.run.id)
        .unwrap();
    assert_eq!(review.status, ResultStatus::Failed);
    assert_eq!(review.counts.failed, 1);
    assert_eq!(review.counts.passed, 0);
    assert_eq!(review.scenarios[0].result.as_ref(), Some(&original));
    assert_eq!(review.unresolved_scenarios.len(), 1);
    assert!(review
        .limitations
        .iter()
        .any(|text| text.contains("recipe")));
}
