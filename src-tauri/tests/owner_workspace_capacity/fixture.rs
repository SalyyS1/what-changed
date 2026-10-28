#![cfg(test)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use what_changed_lib::owner_workspace::*;

pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub struct Fixture {
    pub store: Arc<OwnerStore>,
    root: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!("what-changed-test-owner-capacity-{}", id()));
        std::fs::create_dir(&root).unwrap();
        let store = Arc::new(OwnerStore::open(&root.join("owner-workspace.sqlite")).unwrap());
        Self { store, root }
    }

    /// Twenty real apps, each with a capability, confirmed rule and five
    /// confirmed recipes. No large blobs or direct inserts bypass validation.
    pub fn corpus(&self) -> BTreeMap<String, Vec<String>> {
        let mut corpus = BTreeMap::new();
        for index in 0..20 {
            let app = self
                .store
                .save_app(app_input(&format!("Capacity app {index}")))
                .unwrap();
            let capability = self.store.save_capability(capability(&app.id)).unwrap();
            let rule = self
                .store
                .confirm_rule(ConfirmOwnerRuleInput {
                    operation_id: id(),
                    rule_id: id(),
                    app_id: app.id.clone(),
                    capability_id: capability.id.clone(),
                    expected_version: 0,
                    expected_behavior: "A booking remains visible to its owner".into(),
                    provenance: RuleProvenance::Owner,
                    confirmed_by: "Fixture owner".into(),
                })
                .unwrap();
            let mut scenarios = Vec::new();
            for number in 0..5 {
                let scenario = self
                    .store
                    .confirm_scenario(ConfirmOwnerScenarioInput {
                        operation_id: id(),
                        id: id(),
                        app_id: app.id.clone(),
                        capability_id: capability.id.clone(),
                        expected_version: 0,
                        title: format!("Booking scenario {number}"),
                        expected_behavior: "The saved booking is visible".into(),
                        confirmed_by: "Fixture owner".into(),
                        method: VerificationMethod::OwnerManual,
                        rules: vec![RuleRefDto {
                            rule_id: rule.rule_id.clone(),
                            revision: 1,
                        }],
                        recipe: recipe(),
                    })
                    .unwrap();
                scenarios.push(scenario.id);
            }
            scenarios.sort();
            corpus.insert(app.id, scenarios);
        }
        corpus
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.store.shutdown().is_err() {
            eprintln!("capacity fixture shutdown failed; preserving its temporary database");
            return;
        }
        // Only this newly created UUID fixture may be removed. Never traverse
        // a caller-supplied path or any personal application data directory.
        let temporary = std::env::temp_dir();
        let owned = self.root.is_absolute()
            && self.root.parent() == Some(temporary.as_path())
            && self
                .root
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("what-changed-test-owner-capacity-"));
        if owned {
            if let Err(error) = std::fs::remove_dir_all(&self.root) {
                eprintln!("capacity fixture cleanup failed: {error}");
            }
        }
    }
}

pub fn app_input(name: &str) -> SaveOwnerAppInput {
    SaveOwnerAppInput {
        operation_id: id(),
        id: id(),
        expected_version: 0,
        name: name.into(),
        description: "Bounded owner storage fixture".into(),
        legacy_project_id: None,
        archived: false,
    }
}

pub fn capability(app_id: &str) -> SaveOwnerCapabilityInput {
    SaveOwnerCapabilityInput {
        operation_id: id(),
        id: id(),
        app_id: app_id.into(),
        expected_version: 0,
        label: "Bookings".into(),
        description: "The owner can read a saved booking".into(),
        sort_order: 0,
        archived: false,
    }
}

fn recipe() -> ScenarioRecipeDto {
    ScenarioRecipeDto {
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
                expected_behavior: "The booking is present".into(),
                assertion: RecipeAssertionDto::Visible {
                    locator: LocatorDto::TestId {
                        value: "booking".into(),
                    },
                },
            }],
        }],
    }
}
