//! Authoritative typed offline owner-workspace IPC payloads.

use crate::owner_workspace::domain::{
    CandidateSource, CandidateTrust, ChangeState, DecisionKind, EnvironmentKind, ResultStatus,
    RuleProvenance, RunLifecycle, VerificationMethod,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SaveOwnerAppInput {
    pub operation_id: String,
    pub id: String,
    pub expected_version: u32,
    pub name: String,
    pub description: String,
    pub legacy_project_id: Option<String>,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerAppDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub legacy_project_id: Option<String>,
    pub current_candidate_id: Option<String>,
    pub version: u32,
    #[ts(type = "number | null")]
    pub archived_at: Option<i64>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SaveOwnerCapabilityInput {
    pub operation_id: String,
    pub id: String,
    pub app_id: String,
    pub expected_version: u32,
    pub label: String,
    pub description: String,
    pub sort_order: u32,
    pub archived: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerCapabilityDto {
    pub id: String,
    pub app_id: String,
    pub label: String,
    pub description: String,
    pub sort_order: u32,
    pub version: u32,
    #[ts(type = "number | null")]
    pub archived_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ConfirmOwnerRuleInput {
    pub operation_id: String,
    pub rule_id: String,
    pub app_id: String,
    pub capability_id: String,
    pub expected_version: u32,
    pub expected_behavior: String,
    pub provenance: RuleProvenance,
    pub confirmed_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerRuleRevisionDto {
    pub rule_id: String,
    pub app_id: String,
    pub capability_id: String,
    pub revision: u32,
    pub expected_behavior: String,
    pub provenance: RuleProvenance,
    pub confirmed_by: String,
    #[ts(type = "number")]
    pub confirmed_at: i64,
    pub supersedes: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SaveOwnerChangeInput {
    pub operation_id: String,
    pub id: String,
    pub app_id: String,
    pub expected_version: u32,
    pub intent: String,
    pub example: String,
    pub scope: String,
    pub preserved_rules: Vec<RuleRefDto>,
    pub candidate_id: Option<String>,
    pub state: ChangeState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerChangeDraftDto {
    pub id: String,
    pub app_id: String,
    pub version: u32,
    pub intent: String,
    pub example: String,
    pub scope: String,
    pub preserved_rules: Vec<RuleRefDto>,
    pub candidate_id: Option<String>,
    pub state: ChangeState,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SaveOwnerEnvironmentInput {
    pub operation_id: String,
    pub id: String,
    pub app_id: String,
    pub expected_version: u32,
    pub label: String,
    pub preview_url: String,
    pub origins: Vec<OriginDto>,
    pub kind: EnvironmentKind,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerEnvironmentRevisionDto {
    pub id: String,
    pub app_id: String,
    pub revision: u32,
    pub label: String,
    pub preview_url: String,
    pub origins: Vec<OriginDto>,
    pub kind: EnvironmentKind,
    pub revoked: bool,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SaveOwnerCandidateInput {
    pub operation_id: String,
    pub id: String,
    pub app_id: String,
    pub environment_id: String,
    pub environment_revision: u32,
    pub identity: String,
    pub source: CandidateSource,
    pub trust: CandidateTrust,
    #[ts(type = "number")]
    pub observed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerCandidateRevisionDto {
    pub id: String,
    pub app_id: String,
    pub environment_id: String,
    pub environment_revision: u32,
    pub identity: String,
    pub source: CandidateSource,
    pub trust: CandidateTrust,
    #[ts(type = "number")]
    pub observed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SelectOwnerCandidateInput {
    pub operation_id: String,
    pub app_id: String,
    pub expected_version: u32,
    pub candidate_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerPageInput {
    pub after: Option<String>,
    pub limit: u32,
}

impl Default for OwnerPageInput {
    fn default() -> Self {
        Self {
            after: None,
            limit: 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerPageDto<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerChangeReviewDto {
    pub app: OwnerAppDto,
    pub change: OwnerChangeDraftDto,
    pub preserved_rules: Vec<OwnerRuleRevisionDto>,
    pub candidate: Option<OwnerCandidateRevisionDto>,
    pub environment: Option<OwnerEnvironmentRevisionDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ConfirmOwnerScenarioInput {
    pub operation_id: String,
    pub id: String,
    pub app_id: String,
    pub capability_id: String,
    pub expected_version: u32,
    pub title: String,
    pub expected_behavior: String,
    pub confirmed_by: String,
    pub method: VerificationMethod,
    pub recipe: ScenarioRecipeDto,
    pub rules: Vec<RuleRefDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ScenarioRevisionDto {
    pub id: String,
    pub app_id: String,
    pub capability_id: String,
    pub revision: u32,
    pub title: String,
    pub expected_behavior: String,
    pub confirmed_by: String,
    pub method: VerificationMethod,
    pub recipe: ScenarioRecipeDto,
    pub rules: Vec<RuleRefDto>,
    #[ts(type = "number")]
    pub confirmed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct VerificationRunDto {
    pub id: String,
    pub app_id: String,
    pub manifest: FrozenManifestDto,
    pub lifecycle: RunLifecycle,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number | null")]
    pub ended_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ScenarioResultDto {
    pub id: String,
    pub app_id: String,
    pub run_id: String,
    pub scenario_id: String,
    pub scenario_revision: u32,
    pub method: VerificationMethod,
    pub status: ResultStatus,
    pub assertions: Vec<AssertionOutcomeDto>,
    pub observer: Option<String>,
    #[ts(type = "number")]
    pub observed_at: i64,
    pub observed_steps: Vec<String>,
    pub limitation: Option<String>,
    #[ts(type = "number")]
    pub recorded_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerDecisionDto {
    pub id: String,
    pub app_id: String,
    pub change_id: String,
    pub change_version: u32,
    pub candidate_id: String,
    pub manifest: FrozenManifestDto,
    pub decision: DecisionKind,
    pub reason: Option<String>,
    pub acknowledged_unresolved: bool,
    #[ts(type = "number")]
    pub recorded_at: i64,
}

/// Controls which review-safe sections are included in a copyable brief.
/// Empty `scenario_ids` means all confirmed scenarios for the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerBriefSelectionInput {
    pub include_scenarios: bool,
    pub scenario_ids: Vec<ScenarioRefDto>,
    pub include_candidate_identity: bool,
    pub include_environment_details: bool,
    pub include_limitations: bool,
}

impl Default for OwnerBriefSelectionInput {
    fn default() -> Self {
        Self {
            include_scenarios: true,
            scenario_ids: Vec::new(),
            include_candidate_identity: false,
            include_environment_details: false,
            include_limitations: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerBriefPreviewDto {
    pub app_id: String,
    pub change_id: String,
    pub change_version: u32,
    pub markdown: String,
    #[ts(type = "number")]
    pub bytes: u64,
    #[ts(type = "number")]
    pub estimated_tokens: u64,
    pub omitted_sections: Vec<String>,
    pub limitations: Vec<String>,
}

pub use crate::owner_workspace::domain::{
    AssertionOutcomeDto, FrozenManifestDto, OriginDto, RuleRefDto, ScenarioRecipeDto,
    ScenarioRefDto,
};
