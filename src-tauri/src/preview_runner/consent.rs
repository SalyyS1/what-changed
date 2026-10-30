//! Freeze and validate the exact scope before any browser authority exists.
use super::coordinator::{Pending, PreviewRunner};
use super::{digest, invalid, types::*, SEMANTICS};
use crate::error::AppResult;
use crate::owner_workspace::{validation, OwnerStore};
use std::time::{Duration, Instant};

impl PreviewRunner {
    pub fn prepare(
        &self,
        store: &OwnerStore,
        input: PreparePreviewRunInput,
    ) -> AppResult<PreviewRunPlanDto> {
        if input.routes.len() > 150
            || input.fixture.len() > 50
            || !["run", "record"].contains(&input.mode.as_str())
        {
            return Err(invalid("Choose a supported run or locator-selection mode"));
        }
        for (field, value) in &input.fixture {
            if field.len() > 80
                || !field.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                || value.len() > 1000
                || ["password", "secret", "token", "credential"]
                    .iter()
                    .any(|word| field.to_ascii_lowercase().contains(word))
            {
                return Err(invalid("Use bounded synthetic fixture fields; credentials belong only in the isolated browser"));
            }
            let (_clean, redactions) = crate::privacy::Sanitizer::new().sanitize(value.clone());
            if !redactions.is_empty() {
                return Err(invalid(
                    "A fixture value resembles a secret; replace it with synthetic test data",
                ));
            }
        }
        let mut manifest = store.preview_browser_manifest(&input.run)?;
        manifest.semantics_version = SEMANTICS.into();
        if manifest.environment.origins.iter().any(|origin| {
            origin.scheme != "http" || !["127.0.0.1", "::1"].contains(&origin.host.as_str())
        }) {
            return Err(invalid("Preview execution currently supports only an explicit http://127.0.0.1:PORT or http://[::1]:PORT test origin. Prepare a literal-loopback environment; remote previews await native confinement validation."));
        }
        if manifest.scenarios.len() > 10 {
            return Err(invalid("Choose at most ten scenarios per run"));
        }
        let mut roles = std::collections::BTreeSet::new();
        let mut recipe_ids = std::collections::BTreeSet::new();
        for scenario in &manifest.scenarios {
            validation::recipe(&scenario.recipe)?;
            for step in &scenario.recipe.steps {
                if !recipe_ids.insert(step.id.clone())
                    || step
                        .assertions
                        .iter()
                        .any(|assertion| !recipe_ids.insert(assertion.id.clone()))
                {
                    return Err(invalid(
                        "Step and assertion IDs must be unique across the selected scenarios",
                    ));
                }
            }
            roles.extend(scenario.recipe.role_refs.iter().cloned());
            if scenario
                .recipe
                .fixture_refs
                .iter()
                .any(|field| !input.fixture.contains_key(field))
            {
                return Err(invalid(
                    "Supply a synthetic value for every frozen fixture field",
                ));
            }
        }
        if roles.is_empty() || roles.len() > 2 {
            return Err(invalid(
                "A run supports one or two separate test-role sessions",
            ));
        }
        for route in &input.routes {
            let url =
                tauri::Url::parse(&route.origin).map_err(|_| invalid("Invalid route origin"))?;
            if !manifest.environment.origins.iter().any(|origin| {
                url.scheme() == origin.scheme
                    && url.host_str() == Some(origin.host.as_str())
                    && url.port_or_known_default() == Some(origin.port)
            }) || url.path() != "/"
                || url.query().is_some()
                || url.fragment().is_some()
                || !url.username().is_empty()
                || url.password().is_some()
                || !["GET", "POST", "PUT", "PATCH", "DELETE"].contains(&route.method.as_str())
                || !route.path.starts_with('/')
                || route.path.len() > 300
                || route.path.contains("..")
                || route.path.contains("//")
                || !route
                    .path
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "/_-.{}".contains(c))
            {
                return Err(invalid(
                    "Every route must name an exact granted origin, method and bounded path",
                ));
            }
        }
        let mut effects=vec!["One isolated visible test browser; at most two test roles.".into(),"Approved UI actions may create or change test records. Review the target's cleanup instructions.".into()];
        if input.mode == "record" {
            effects.push("Locator selection disables new network requests on the loaded test surface and exports suggested locators, never field values. Review each suggestion.".into());
        }
        if input.capture_screenshots {
            effects.push("Screenshots are sensitive raw artifacts. Each requested capture must be reviewed before retention for at most seven days; they are excluded from portable memory export.".into());
        }
        let manifest_digest = digest(&manifest)?;
        let scope_digest = digest(&(
            &manifest_digest,
            &input.routes,
            &input.fixture,
            &input.mode,
            &input.capture_screenshots,
        ))?;
        let plan=PreviewRunPlanDto{consent_id:uuid::Uuid::new_v4().to_string(),manifest,manifest_digest,scope_digest,roles:roles.into_iter().collect(),routes:input.routes.clone(),effects,capture_screenshots:input.capture_screenshots,
            limitations:vec!["Temporary browser profiles may contain login/cache data; deleting them does not guarantee forensic erasure.".into(),"Remote previews remain unavailable until connection-level confinement is independently verified.".into(),"Missing revision markers produce observation-only evidence; owner acceptance remains a separate action.".into()]};
        *self.pending.lock().unwrap_or_else(|p| p.into_inner()) = Some(Pending {
            input,
            plan: plan.clone(),
            expires: Instant::now() + Duration::from_secs(300),
        });
        Ok(plan)
    }
}
