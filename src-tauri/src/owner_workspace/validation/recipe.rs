//! Recipe validation bounds named actions and assertions without executing any target.

use super::*;
use std::collections::BTreeSet;
mod active;

fn route(value: &str, template: bool) -> AppResult<()> {
    if !value.starts_with('/')
        || value.starts_with("//")
        || value.contains(['\\', '?', '#'])
        || value.contains("..")
        || value.chars().any(char::is_control)
    {
        return Err(invalid(
            "recipe route",
            "must be a relative path without authority, query, fragment or traversal",
        ));
    }
    if template && value.matches("{fixture_id}").count() != 1 {
        return Err(invalid(
            "adapter path",
            "requires exactly one fixture_id slot",
        ));
    }
    if !template && value.contains(['{', '}']) {
        return Err(invalid("recipe route", "does not accept template slots"));
    }
    Ok(())
}

fn locator(value: &LocatorDto) -> AppResult<()> {
    match value {
        LocatorDto::Role { role, name } => {
            required(role, "locator role")?;
            required(name, "locator name")?;
        }
        LocatorDto::Label { text } => required(text, "locator label")?,
        LocatorDto::TestId { value } => required(value, "locator test id")?,
    }
    Ok(())
}

pub(super) fn validate(input: &ScenarioRecipeDto) -> AppResult<()> {
    validate_archival(input)?;
    active::validate(input)
}

/// Safe, closed recipe structure retained for non-executable historical memory.
/// Current method compatibility and result capacity are active-admission rules.
pub(super) fn validate_archival(input: &ScenarioRecipeDto) -> AppResult<()> {
    payload(input)?;
    if input.schema_version != 1 {
        return Err(invalid("recipe schema_version", "only v1 is supported"));
    }
    if input.steps.is_empty() || input.steps.len() > MAX_STEPS_PER_SCENARIO {
        return Err(invalid("recipe steps", "requires between 1 and 50 steps"));
    }
    if input.role_refs.len() > 10 || input.fixture_refs.len() > 50 || input.adapters.len() > 16 {
        return Err(invalid(
            "recipe references",
            "role, fixture or adapter count exceeds the supported limit",
        ));
    }
    let mut adapters = BTreeSet::new();
    for adapter in &input.adapters {
        uuid(&adapter.id, "adapter id")?;
        if !adapters.insert(adapter.id.clone()) {
            return Err(invalid("adapter id", "duplicates are not allowed"));
        }
        // Loopback origins are explicit local configuration. Other origins must be HTTPS.
        origin(&adapter.origin, EnvironmentKind::Local)?;
        route(&adapter.path_template, true)?;
        if !input.role_refs.contains(&adapter.role_ref) {
            return Err(invalid("adapter role", "must refer to a declared role"));
        }
        if adapter.allowed_fields.is_empty() || adapter.allowed_fields.len() > 50 {
            return Err(invalid(
                "adapter fields",
                "requires between 1 and 50 named fields",
            ));
        }
    }
    let mut steps = BTreeSet::new();
    let mut assertions = BTreeSet::new();
    for step in &input.steps {
        uuid(&step.id, "step id")?;
        if !steps.insert(&step.id) {
            return Err(invalid("step id", "duplicates are not allowed"));
        }
        if step.assertions.len() > 100 {
            return Err(invalid(
                "assertions",
                "at most 100 assertions per step are supported",
            ));
        }
        match &step.action {
            RecipeActionDto::Navigate { path } => route(path, false)?,
            RecipeActionDto::Fill {
                locator: target,
                fixture_field,
            } => {
                locator(target)?;
                required(fixture_field, "fixture field")?;
            }
            RecipeActionDto::Click { locator: target }
            | RecipeActionDto::Cleanup { locator: target } => locator(target)?,
            RecipeActionDto::ReadBack { adapter_id }
            | RecipeActionDto::DenyOtherRole { adapter_id } => {
                if !adapters.contains(adapter_id) {
                    return Err(invalid(
                        "action adapter",
                        "must refer to a declared adapter",
                    ));
                }
            }
        }
        for assertion in &step.assertions {
            uuid(&assertion.id, "assertion id")?;
            if !assertions.insert(&assertion.id) {
                return Err(invalid("assertion id", "duplicates are not allowed"));
            }
            required(&assertion.expected_behavior, "assertion expected behavior")?;
            match &assertion.assertion {
                RecipeAssertionDto::Visible { locator: target }
                | RecipeAssertionDto::TextEquals {
                    locator: target, ..
                } => locator(target)?,
                RecipeAssertionDto::FieldEquals {
                    field,
                    fixture_field,
                } => {
                    required(field, "response field")?;
                    required(fixture_field, "fixture field")?;
                }
                RecipeAssertionDto::Denied {
                    allowed_statuses,
                    absent_fields,
                } => {
                    if allowed_statuses.is_empty()
                        || allowed_statuses.len() > 2
                        || allowed_statuses.iter().any(|s| !matches!(s, 403 | 404))
                        || absent_fields.len() > 50
                    {
                        return Err(invalid(
                            "denied assertion",
                            "requires 403/404 and at most 50 absent fields",
                        ));
                    }
                }
            }
        }
    }
    if assertions.is_empty() {
        return Err(invalid(
            "recipe assertions",
            "a confirmed scenario requires at least one named assertion",
        ));
    }
    Ok(())
}
