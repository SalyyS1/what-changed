//! Bounded portable snapshots carry history without portable execution authority.
use crate::owner_workspace::{OwnerAppDto, OwnerRuleRevisionDto};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerExportSelectionInput {
    pub include_target_details: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerBundlePreviewDto {
    pub app_id: String,
    pub digest: String,
    #[ts(type = "number")]
    pub bytes: u64,
    pub markdown: String,
    pub rules: Vec<OwnerRuleRevisionDto>,
    pub omissions: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerImportPreviewDto {
    pub review_id: String,
    pub digest: String,
    pub app_name: String,
    pub proposed_app_id: String,
    pub rules: Vec<OwnerRuleRevisionDto>,
    pub markdown: String,
    pub conflicts: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ApplyOwnerImportInput {
    pub operation_id: String,
    pub review_id: String,
    pub digest: String,
    pub name: String,
    pub confirmed_by: String,
    pub confirmed_rules: Vec<crate::owner_workspace::RuleRefDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OwnerImportResultDto {
    pub app: OwnerAppDto,
    pub source_digest: String,
    pub limitations: Vec<String>,
}
