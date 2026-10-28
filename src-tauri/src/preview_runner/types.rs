use crate::dto::owner_evidence_dto::{OwnerManualManifestDto, StartOwnerManualRunInput};
use crate::owner_workspace::{LocatorDto, ResultStatus};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RunnerRouteDto {
    pub origin: String,
    pub method: String,
    pub path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct PreparePreviewRunInput {
    pub run: StartOwnerManualRunInput,
    pub routes: Vec<RunnerRouteDto>,
    pub fixture: BTreeMap<String, String>,
    pub mode: String,
    #[serde(default)]
    pub capture_screenshots: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PreviewRunPlanDto {
    pub consent_id: String,
    pub manifest: OwnerManualManifestDto,
    pub manifest_digest: String,
    pub scope_digest: String,
    pub roles: Vec<String>,
    pub routes: Vec<RunnerRouteDto>,
    pub effects: Vec<String>,
    pub limitations: Vec<String>,
    pub capture_screenshots: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PreviewRecipeFileDto {
    pub path: String,
    pub content: String,
    pub media_type: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PreviewRecipeExportDto {
    pub app_id: String,
    pub digest: String,
    pub files: Vec<PreviewRecipeFileDto>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct GrantPreviewRunInput {
    pub consent_id: String,
    pub scope_digest: String,
    pub authorized_test_environment: bool,
    pub isolated_test_data: bool,
    pub acknowledge_temporary_auth: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PreviewRunnerEventDto {
    pub sequence: u32,
    pub kind: String,
    pub message: String,
    pub scenario_id: Option<String>,
    pub assertion_id: Option<String>,
    pub status: Option<ResultStatus>,
    pub locator: Option<LocatorDto>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PreviewRunnerStatusDto {
    pub app_id: String,
    pub run_id: String,
    pub lifecycle: String,
    pub roles: Vec<String>,
    pub can_continue: bool,
    pub mode: String,
    pub events: Vec<PreviewRunnerEventDto>,
    pub error: Option<String>,
    pub capture_screenshots: bool,
    pub captures: Vec<PreviewCaptureDto>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PreviewCaptureDto {
    pub id: String,
    pub role: String,
    pub data_url: String,
    pub retained: bool,
    #[ts(type = "number")]
    pub expires_at: i64,
}
