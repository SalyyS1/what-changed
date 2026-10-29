//! Pure input and output contracts for deterministic investigations.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{Category, Entity, Importance, Relationship, SystemEvent};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InvestigationWindow {
    LastHour,
    Today,
    SinceYesterday,
    SinceLastRestart,
    SinceCheckpoint { checkpoint_id: i64 },
    Custom { from_ms: i64, to_ms: i64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ResolvedInvestigationWindow {
    pub from_ms: i64,
    pub to_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceClass {
    Correlated,
    Likely,
    Confirmed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SymptomTag {
    Audio,
    Network,
    Port,
    Performance,
    Crash,
    Build,
    Docker,
    Login,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum NoCauseReason {
    NothingChanged,
    OnlyRoutineChanges,
    WindowTooNarrow,
    NotWatchingYet,
    NoRecordedEvidenceInWorkspace,
}

impl NoCauseReason {
    pub const fn message(self) -> &'static str {
        match self {
            Self::NothingChanged => "Nothing meaningful changed in this window.",
            Self::OnlyRoutineChanges => "Only routine changes here — updates and background tasks.",
            Self::WindowTooNarrow => "Try widening the time range.",
            Self::NotWatchingYet => {
                "We started watching after this window. Try a more recent range."
            }
            Self::NoRecordedEvidenceInWorkspace => {
                "No recorded evidence was associated with this workspace in this window."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS, Default)]
pub struct ScoreBreakdown {
    pub temporal: f64,
    pub relationship: f64,
    pub category: f64,
    pub importance: f64,
    pub rarity: f64,
    pub failure_proximity: f64,
    pub chain_bonus: f64,
    pub total: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ChainStep {
    pub event_ids: Vec<i64>,
    pub timestamp_ms: i64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Suspect {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct Investigation {
    pub id: i64,
    #[serde(default)]
    pub project_id: Option<String>,
    pub problem: String,
    pub window: InvestigationWindow,
    pub resolved_window: ResolvedInvestigationWindow,
    pub suspects: Vec<Suspect>,
    pub no_cause_reason: Option<NoCauseReason>,
    pub no_cause_message: Option<String>,
    pub total_events_in_window: usize,
    pub analyzed_events: usize,
    pub dropped_events: usize,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Coverage {
    pub watching_from_ms: Option<i64>,
    pub complete: bool,
}

impl Default for Coverage {
    fn default() -> Self {
        Self {
            watching_from_ms: None,
            complete: true,
        }
    }
}

/// All already-redacted facts needed by the OS-free engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisInput {
    pub problem: String,
    pub window: InvestigationWindow,
    pub resolved_window: ResolvedInvestigationWindow,
    pub symptom_time_ms: i64,
    pub created_at: i64,
    pub events: Vec<SystemEvent>,
    #[serde(default)]
    pub boundary_events: Vec<SystemEvent>,
    #[serde(default)]
    pub entities: Vec<Entity>,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
    #[serde(default)]
    pub prior_occurrences_90d: BTreeMap<i64, u32>,
    #[serde(default)]
    pub total_events_in_window: usize,
    #[serde(default)]
    pub dropped_events: usize,
    #[serde(default)]
    pub coverage: Coverage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymptomModel {
    pub entity_ids: Vec<String>,
    pub categories: Vec<Category>,
    pub entity_types: Vec<super::EntityType>,
    pub tags: Vec<SymptomTag>,
    pub symptom_time_ms: i64,
}

pub fn importance_value(importance: Importance) -> f64 {
    match importance {
        Importance::Informational => 0.15,
        Importance::Notable => 0.35,
        Importance::Important => 0.6,
        Importance::Critical => 1.0,
    }
}
