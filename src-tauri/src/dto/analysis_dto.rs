//! Investigation and map payloads. Analysis stays in the domain layer; these
//! wrappers keep the public IPC names stable while exposing structured proof.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{
    ChainStep, ConfidenceClass, Investigation, InvestigationWindow, NoCauseReason,
    ResolvedInvestigationWindow, ScoreBreakdown, Suspect,
};

pub use crate::analysis::change_map::{
    ChangeMapData, ChangeMapEdge, ChangeMapEdgeKind, ChangeMapLane, ChangeMapLaneData,
    ChangeMapNode, ChangeMapRequest,
};
pub use crate::analysis::system_map::{
    EntityKindAvailability, EntityRef, SystemMapData, SystemMapEdge, SystemMapNode,
    SystemMapRecentEvent,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct InvestigationRequestDto {
    pub problem: String,
    pub window: Option<InvestigationWindow>,
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct SuspectDto {
    pub event_id: i64,
    pub title: String,
    pub timestamp_ms: i64,
    pub score: f64,
    pub breakdown: ScoreBreakdown,
    pub confidence: ConfidenceClass,
    pub confidence_text: String,
    pub relative_time: String,
    pub why_ranked: Vec<String>,
    pub chain: Vec<ChainStep>,
    pub what_to_try: String,
}

impl From<Suspect> for SuspectDto {
    fn from(value: Suspect) -> Self {
        Self {
            event_id: value.event_id,
            title: value.title,
            timestamp_ms: value.timestamp_ms,
            score: value.score,
            breakdown: value.breakdown,
            confidence: value.confidence,
            confidence_text: value.confidence_text,
            relative_time: value.relative_time,
            why_ranked: value.why_ranked,
            chain: value.chain,
            what_to_try: value.what_to_try,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct InvestigationDto {
    pub id: i64,
    #[serde(default)]
    pub project_id: Option<String>,
    pub problem: String,
    pub window: InvestigationWindow,
    pub resolved_window: ResolvedInvestigationWindow,
    pub suspects: Vec<SuspectDto>,
    pub no_cause_reason: Option<NoCauseReason>,
    pub no_cause_message: Option<String>,
    pub total_events_in_window: usize,
    pub analyzed_events: usize,
    pub dropped_events: usize,
    pub created_at: i64,
}

impl From<Investigation> for InvestigationDto {
    fn from(value: Investigation) -> Self {
        Self {
            id: value.id,
            project_id: value.project_id,
            problem: value.problem,
            window: value.window,
            resolved_window: value.resolved_window,
            suspects: value.suspects.into_iter().map(Into::into).collect(),
            no_cause_reason: value.no_cause_reason,
            no_cause_message: value.no_cause_message,
            total_events_in_window: value.total_events_in_window,
            analyzed_events: value.analyzed_events,
            dropped_events: value.dropped_events,
            created_at: value.created_at,
        }
    }
}
