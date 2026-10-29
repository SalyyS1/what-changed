//! Event-shaped payloads.
//!
//! The page itself is the domain type: `Page<EventSummary>` is already exactly
//! what the timeline renders, and wrapping it in a second struct would give the
//! same shape two names. The types here are the ones with no domain equivalent
//! yet, each owned by the phase named in its doc comment.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{Category, EventSummary, Importance, Page, SystemEvent};

/// One page of the timeline.
pub type EventPageDto = Page<EventSummary>;

use crate::dto::status_dto::MonitoringDto;

/// A lightweight reference to the latest checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CheckpointRef {
    pub id: i64,
    pub name: String,
    pub created_at: i64,
}

/// Counts shown on the Home screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DashboardCountsDto {
    pub total: u64,
    pub notable: u64,
    pub important: u64,
    pub critical: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DashboardSummaryDto {
    pub today: DashboardCountsDto,
    pub status_line: String,
    pub top_important: Vec<EventSummary>,
    pub monitoring: MonitoringDto,
    pub last_checkpoint: Option<CheckpointRef>,
    /// Legacy flattened aliases retained for existing internal consumers.
    #[serde(skip)]
    #[ts(skip)]
    pub total: u64,
    #[serde(skip)]
    #[ts(skip)]
    pub notable: u64,
    #[serde(skip)]
    #[ts(skip)]
    pub important: u64,
    #[serde(skip)]
    #[ts(skip)]
    pub critical: u64,
}

/// The details drawer. Shape owned by phase 06.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EventDetailsDto {
    /// The event itself.
    pub event: SystemEvent,
    /// Events about the same or a related entity, nearby in time.
    pub related: Vec<EventSummary>,
    /// How many times the same thing happened to the same entity before.
    pub prior_occurrences: u64,
}

/// One global-search hit, tagged so the palette can route each result without
/// guessing from nullable fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SearchResultDto {
    Event {
        event: EventSummary,
        category: Category,
        importance: Importance,
    },
    Checkpoint {
        checkpoint: CheckpointRef,
    },
}

/// A folder the user chose to watch. Shape owned by phase 10.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct WatchedLocationDto {
    /// Row id.
    pub id: i64,
    /// The canonical path, as it will be shown back to the user.
    pub path: String,
    /// Whether it is currently watched.
    pub enabled: bool,
    /// Whether the root still resolves to the same safe, ordinary directory.
    pub available: bool,
    /// How deep below the root the collector descends.
    pub max_depth: u8,
    /// When it was added, epoch milliseconds UTC.
    pub added_at: i64,
}

/// A registered project root. Shape owned by phase 18.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ProjectDto {
    /// Stable project entity id.
    pub id: String,
    /// The canonical path.
    pub path: String,
    /// The folder name, which is what a person calls the project.
    pub name: String,
    /// The branch checked out when it was last read, if it is a git checkout.
    pub branch: Option<String>,
    /// The known-good checkpoint selected for this project, if any.
    pub baseline_checkpoint_id: Option<i64>,
    pub baseline_checkpoint_name: Option<String>,
    pub baseline_checkpoint_created_at: Option<i64>,
}
