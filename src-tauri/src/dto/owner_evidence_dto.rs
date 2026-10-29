//! Offline manual observations and owner decisions; callers cannot choose freshness.

use crate::dto::owner_workspace_dto::{
    OwnerCandidateRevisionDto, OwnerChangeDraftDto, OwnerEnvironmentRevisionDto,
    OwnerRuleRevisionDto, ScenarioRevisionDto,
};
use crate::owner_workspace::domain::{
    AssertionOutcomeDto, DecisionKind, EvidenceFreshness, ResultStatus, RunLifecycle,
    ScenarioRefDto, VerificationMethod,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct StartOwnerManualRunInput {
    pub operation_id: String,
    pub id: String,
    pub app_id: String,
    pub change_id: String,
    pub expected_change_version: u32,
    pub candidate_id: String,
    pub scenarios: Vec<ScenarioRefDto>,
    pub retry_of: Option<String>,
}

/// Exact sanitized inputs are frozen, not pointers that can follow later edits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerManualManifestDto {
    pub schema_version: u32,
    pub change: OwnerChangeDraftDto,
    pub candidate: OwnerCandidateRevisionDto,
    pub environment: OwnerEnvironmentRevisionDto,
    pub rules: Vec<OwnerRuleRevisionDto>,
    pub scenarios: Vec<ScenarioRevisionDto>,
    pub fixture_schema_digest: String,
    pub semantics_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerManualRunDto {
    pub id: String,
    pub app_id: String,
    pub change_id: String,
    pub version: u32,
    pub method: VerificationMethod,
    pub lifecycle: RunLifecycle,
    pub retry_of: Option<String>,
    pub manifest: OwnerManualManifestDto,
    pub manifest_digest: String,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number | null")]
    pub ended_at: Option<i64>,
    /// Absent on older/manual/imported runs. Only the native supervisor writes it.
    #[serde(default)]
    pub execution: Option<OwnerExecutionSummaryDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerExecutionSummaryDto {
    pub runner_semantics: String,
    pub scope_digest: String,
    pub identity: String,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ManualStepObservationDto {
    pub step_id: String,
    pub observed: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct ManualAssertionObservationInput {
    pub assertion_id: String,
    pub status: ResultStatus,
    pub observed: String,
    pub limitation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RecordOwnerManualResultInput {
    pub operation_id: String,
    pub id: String,
    pub app_id: String,
    pub run_id: String,
    pub expected_run_version: u32,
    pub scenario_id: String,
    pub scenario_revision: u32,
    pub observer: String,
    #[ts(type = "number")]
    pub observed_at: i64,
    pub observed_steps: Vec<ManualStepObservationDto>,
    pub assertions: Vec<ManualAssertionObservationInput>,
    pub limitation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerManualResultDto {
    pub id: String,
    pub app_id: String,
    pub run_id: String,
    pub scenario_id: String,
    pub scenario_revision: u32,
    pub method: VerificationMethod,
    pub status: ResultStatus,
    pub observer: String,
    #[ts(type = "number")]
    pub observed_at: i64,
    pub observed_steps: Vec<ManualStepObservationDto>,
    pub assertions: Vec<AssertionOutcomeDto>,
    pub limitation: Option<String>,
    #[ts(type = "number")]
    pub recorded_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct FinishOwnerManualRunInput {
    pub operation_id: String,
    pub app_id: String,
    pub run_id: String,
    pub expected_run_version: u32,
    pub lifecycle: RunLifecycle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct RecordOwnerEvidenceDecisionInput {
    pub operation_id: String,
    pub id: String,
    pub app_id: String,
    pub change_id: String,
    pub expected_change_version: u32,
    pub run_id: String,
    pub expected_run_version: u32,
    pub decision: DecisionKind,
    pub reason: String,
    pub acknowledged_unresolved: Vec<ScenarioRefDto>,
    pub acknowledge_unknown_build: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerEvidenceDecisionDto {
    pub id: String,
    pub app_id: String,
    pub change_id: String,
    pub change_version: u32,
    pub run_id: String,
    pub run_version: u32,
    pub candidate_id: String,
    pub manifest_digest: String,
    pub decision: DecisionKind,
    pub reason: String,
    pub acknowledged_unresolved: Vec<ScenarioRefDto>,
    pub acknowledged_unknown_build: bool,
    pub evidence_freshness: EvidenceFreshness,
    /// Recomputed on read; evidence_freshness records the decision-time value.
    pub current_freshness: EvidenceFreshness,
    #[ts(type = "number")]
    pub recorded_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerEvidenceCountsDto {
    pub passed: u32,
    pub failed: u32,
    pub blocked: u32,
    pub not_run: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerScenarioEvidenceDto {
    pub scenario: ScenarioRevisionDto,
    pub status: ResultStatus,
    pub result: Option<OwnerManualResultDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct OwnerEvidenceReviewDto {
    pub run: OwnerManualRunDto,
    pub freshness: EvidenceFreshness,
    pub freshness_reasons: Vec<String>,
    pub status: ResultStatus,
    pub counts: OwnerEvidenceCountsDto,
    pub scenarios: Vec<OwnerScenarioEvidenceDto>,
    pub decisions: Vec<OwnerEvidenceDecisionDto>,
    pub unresolved_scenarios: Vec<ScenarioRefDto>,
    pub can_accept: bool,
    pub can_accept_with_exceptions: bool,
    pub decision_blockers: Vec<String>,
    pub limitations: Vec<String>,
}
