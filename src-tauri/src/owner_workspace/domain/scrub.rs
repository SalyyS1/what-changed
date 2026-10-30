//! Every text-bearing write type implements the existing sanitizer contract.

use super::*;
use crate::dto::owner_workspace_dto::*;
use crate::privacy::{Redaction, Sanitizer, Scrub};

macro_rules! fields {
    ($ty:ty; $($member:ident),* $(,)?) => {
        impl Scrub for $ty {
            fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
                $(self.$member.scrub_in_place(sanitizer, &format!("{field}.{}", stringify!($member)), report);)*
                let _ = (sanitizer, field, report);
            }
        }
    };
}

fields!(SaveOwnerAppInput; name, description);
fields!(SaveOwnerCapabilityInput; label, description);
fields!(ConfirmOwnerRuleInput; expected_behavior, confirmed_by);
fields!(SaveOwnerChangeInput; intent, example, scope);
impl Scrub for SaveOwnerEnvironmentInput {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        self.label
            .scrub_in_place(sanitizer, &format!("{field}.label"), report);
        // This typed HTTP(S) address is validated before and after sanitation:
        // no userinfo, query, fragment or private remote origin is allowed.
        // The generic *_url key rule targets unconstrained connection strings.
        // Shape and pair redaction still run through the same sanitizer here.
        self.preview_url
            .scrub_in_place(sanitizer, &format!("{field}.preview_address"), report);
        self.origins
            .scrub_in_place(sanitizer, &format!("{field}.origins"), report);
    }
}
fields!(SaveOwnerCandidateInput; identity);
fields!(SelectOwnerCandidateInput;);
fields!(ConfirmOwnerScenarioInput; title, expected_behavior, confirmed_by, recipe);
fields!(OriginDto; scheme, host);
fields!(ScenarioRecipeDto; role_refs, fixture_refs, steps, adapters);
fields!(RecipeStepDto; action, assertions);
fields!(NamedAssertionDto; expected_behavior, assertion);
fields!(ReadBackAdapterDto; origin, path_template, role_ref, allowed_fields);
fields!(AssertionOutcomeDto; expected, observed, limitation);
fields!(FrozenManifestDto; runner_semantics);

impl Scrub for LocatorDto {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        match self {
            Self::Role { role, name } => {
                role.scrub_in_place(sanitizer, field, report);
                name.scrub_in_place(sanitizer, field, report);
            }
            Self::Label { text } => text.scrub_in_place(sanitizer, field, report),
            Self::TestId { value } => value.scrub_in_place(sanitizer, field, report),
        }
    }
}

impl Scrub for RecipeActionDto {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        match self {
            Self::Navigate { path } => path.scrub_in_place(sanitizer, field, report),
            Self::Fill {
                locator,
                fixture_field,
            } => {
                locator.scrub_in_place(sanitizer, field, report);
                fixture_field.scrub_in_place(sanitizer, field, report);
            }
            Self::Click { locator } | Self::Cleanup { locator } => {
                locator.scrub_in_place(sanitizer, field, report)
            }
            Self::ReadBack { .. } | Self::DenyOtherRole { .. } => {}
        }
    }
}

impl Scrub for RecipeAssertionDto {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        match self {
            Self::Visible { locator } => locator.scrub_in_place(sanitizer, field, report),
            Self::TextEquals { locator, expected } => {
                locator.scrub_in_place(sanitizer, field, report);
                expected.scrub_in_place(sanitizer, field, report);
            }
            Self::FieldEquals {
                field: name,
                fixture_field,
            } => {
                name.scrub_in_place(sanitizer, field, report);
                fixture_field.scrub_in_place(sanitizer, field, report);
            }
            Self::Denied { absent_fields, .. } => {
                absent_fields.scrub_in_place(sanitizer, field, report)
            }
        }
    }
}
