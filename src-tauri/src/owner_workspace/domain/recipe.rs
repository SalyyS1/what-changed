//! Recipe v1 describes bounded browser actions and named read-back routes.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LocatorDto {
    Role { role: String, name: String },
    Label { text: String },
    TestId { value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecipeActionDto {
    Navigate {
        path: String,
    },
    Fill {
        locator: LocatorDto,
        fixture_field: String,
    },
    Click {
        locator: LocatorDto,
    },
    ReadBack {
        adapter_id: String,
    },
    DenyOtherRole {
        adapter_id: String,
    },
    Cleanup {
        locator: LocatorDto,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecipeAssertionDto {
    Visible {
        locator: LocatorDto,
    },
    TextEquals {
        locator: LocatorDto,
        expected: String,
    },
    FieldEquals {
        field: String,
        fixture_field: String,
    },
    Denied {
        allowed_statuses: Vec<u16>,
        absent_fields: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RecipeStepDto {
    pub id: String,
    pub action: RecipeActionDto,
    pub assertions: Vec<NamedAssertionDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct NamedAssertionDto {
    pub id: String,
    pub expected_behavior: String,
    pub assertion: RecipeAssertionDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ReadBackAdapterDto {
    pub id: String,
    pub origin: super::OriginDto,
    pub path_template: String,
    pub role_ref: String,
    pub allowed_fields: Vec<String>,
    pub effect: AdapterEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AdapterEffect {
    ReadOnly,
    ReadDenied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ScenarioRecipeDto {
    pub schema_version: u32,
    pub role_refs: Vec<String>,
    pub fixture_refs: Vec<String>,
    pub steps: Vec<RecipeStepDto>,
    pub adapters: Vec<ReadBackAdapterDto>,
}
