//! Owner-authored project decisions are independent of evidence acceptance.
use crate::owner_workspace::RuleProvenance;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct SaveProjectDecisionInput {
    pub operation_id: String,
    pub id: String,
    pub app_id: String,
    pub expected_revision: u32,
    pub capability_id: Option<String>,
    pub change_id: Option<String>,
    pub text: String,
    pub rationale: String,
    pub provenance: RuleProvenance,
    pub recorded_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ProjectDecisionRevisionDto {
    pub id: String,
    pub app_id: String,
    pub revision: u32,
    pub capability_id: Option<String>,
    pub change_id: Option<String>,
    pub text: String,
    pub rationale: String,
    pub provenance: RuleProvenance,
    pub recorded_by: String,
    pub supersedes: Option<u32>,
    #[ts(type = "number")]
    pub recorded_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ProjectMemoryFilterInput {
    pub capability_id: Option<String>,
    pub change_id: Option<String>,
    pub page: crate::owner_workspace::OwnerPageInput,
}
