//! Active evidence requires compatible methods and a recordable result budget.

use super::*;

pub(super) fn validate(input: &ScenarioRecipeDto) -> AppResult<()> {
    if input
        .steps
        .iter()
        .map(|step| step.assertions.len())
        .sum::<usize>()
        > MAX_ASSERTIONS_PER_SCENARIO
    {
        return Err(invalid(
            "recipe assertions",
            "at most 1000 assertions across all steps can fit one scenario result",
        ));
    }
    for role in &input.role_refs {
        required(role, "recipe role reference")?;
    }
    for adapter in &input.adapters {
        for field in &adapter.allowed_fields {
            required(field, "adapter response field")?;
        }
    }
    for step in &input.steps {
        let adapter = backend_action_adapter(input, &step.action)?;
        for assertion in &step.assertions {
            match &assertion.assertion {
                RecipeAssertionDto::FieldEquals { field, .. } => {
                    let adapter = adapter.filter(|value| value.effect == AdapterEffect::ReadOnly)
                        .ok_or_else(|| invalid("stored-field assertion", "requires a read-back action with a declared read-only route and role"))?;
                    if !adapter.allowed_fields.contains(field) {
                        return Err(invalid(
                            "stored-field assertion",
                            "must name a selected response field on its read-back route",
                        ));
                    }
                }
                RecipeAssertionDto::Denied { absent_fields, .. } => {
                    let adapter = adapter.filter(|value| value.effect == AdapterEffect::ReadDenied)
                        .ok_or_else(|| invalid("denied assertion", "requires a deny-other-role action with a declared denied-access route and role"))?;
                    if absent_fields
                        .iter()
                        .any(|field| !adapter.allowed_fields.contains(field))
                    {
                        return Err(invalid("denied assertion", "absent fields must be selected response fields on its denied-access route"));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn backend_action_adapter<'a>(
    recipe: &'a ScenarioRecipeDto,
    action: &RecipeActionDto,
) -> AppResult<Option<&'a ReadBackAdapterDto>> {
    let (id, expected_effect) = match action {
        RecipeActionDto::ReadBack { adapter_id } => (adapter_id, AdapterEffect::ReadOnly),
        RecipeActionDto::DenyOtherRole { adapter_id } => (adapter_id, AdapterEffect::ReadDenied),
        _ => return Ok(None),
    };
    let adapter = recipe
        .adapters
        .iter()
        .find(|adapter| &adapter.id == id)
        .ok_or_else(|| invalid("action adapter", "must refer to a declared adapter"))?;
    if adapter.effect != expected_effect {
        return Err(invalid(
            "action adapter",
            "read-back and denied-access actions require a route with the matching purpose",
        ));
    }
    Ok(Some(adapter))
}
