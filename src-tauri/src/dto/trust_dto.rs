//! Wire types owned by the trust layer (privacy, purge and onboarding).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::RetentionPolicy;
use crate::owner_workspace::OwnerStoreState;

/// One event-category count shown by the privacy screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PrivacyCategoryCountDto {
    pub category: String,
    pub count: u64,
}

/// Bounded, explainable view of what the local database contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PrivacySummaryDto {
    pub categories: Vec<PrivacyCategoryCountDto>,
    pub checkpoints: u64,
    pub checkpoint_items: u64,
    pub investigations: u64,
    pub watched_folders: u64,
    pub config_baselines: u64,
    pub database_bytes: u64,
    pub wal_bytes: u64,
    pub oldest_record_at: Option<i64>,
    pub log_bytes: u64,
    pub data_path: String,
    pub database_path: String,
    pub retention: RetentionPolicy,
    /// The separate owner-workspace store is included without exposing its
    /// filesystem path. A failed owner store must not make healthy history
    /// look empty or vice versa.
    pub owner_store: OwnerStoreState,
    pub owner_workspace_bytes: Option<u64>,
}

/// Result of a destructive operation. Counts are useful feedback, but no row
/// content or path is returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS, Default)]
pub struct PurgeReportDto {
    pub events_deleted: u64,
    pub checkpoints_deleted: u64,
    pub investigations_deleted: u64,
    pub baselines_deleted: u64,
    pub watched_folders_deleted: u64,
}

/// Small onboarding state machine response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OnboardingStateDto {
    pub completed: bool,
    pub can_skip: bool,
}
