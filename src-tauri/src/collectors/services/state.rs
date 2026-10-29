use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::collectors::stateful::{self, PendingTarget};
use crate::error::{AppError, AppResult};
use crate::storage::Baseline;

pub const BASELINE_KIND: &str = "service_instances";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceRunState {
    Stopped,
    StartPending,
    StopPending,
    Running,
    ContinuePending,
    PausePending,
    Paused,
    Unknown(u32),
}

impl ServiceRunState {
    pub const fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Stopped => "Stopped",
            Self::StartPending => "Start pending",
            Self::StopPending => "Stop pending",
            Self::Running => "Running",
            Self::ContinuePending => "Continue pending",
            Self::PausePending => "Pause pending",
            Self::Paused => "Paused",
            Self::Unknown(_) => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStartType {
    Boot,
    System,
    Automatic,
    Manual,
    Disabled,
    Unknown(u32),
}

impl ServiceStartType {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Boot => "Boot",
            Self::System => "System",
            Self::Automatic => "Automatic",
            Self::Manual => "Manual",
            Self::Disabled => "Disabled",
            Self::Unknown(_) => "Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceRecord {
    pub name: String,
    pub display_name: String,
    pub state: ServiceRunState,
    pub start_type: ServiceStartType,
    /// Whether this cycle could open the service with `SERVICE_QUERY_CONFIG`.
    /// A denial is ordinary degradation and must not look like a config edit.
    #[serde(default)]
    pub config_available: bool,
    /// Session-local relationship evidence. Paths are published through the
    /// bounded evidence map and carried by immediate events, never rehydrated
    /// from a redacted durable baseline.
    #[serde(skip)]
    pub binary_path: Option<String>,
    pub process_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ServiceSnapshot {
    pub acknowledged: BTreeMap<String, ServiceRecord>,
    #[serde(default)]
    pub pending: Option<PendingTarget<BTreeMap<String, ServiceRecord>>>,
    pub established: bool,
}

impl ServiceSnapshot {
    pub fn hydrate(baseline: Option<Baseline>, now_ms: i64) -> (Self, bool) {
        stateful::hydrate(baseline, now_ms, "the service collector's service table")
    }
}

pub fn index(rows: Vec<ServiceRecord>) -> AppResult<BTreeMap<String, ServiceRecord>> {
    let mut indexed = BTreeMap::new();
    for row in rows {
        if row.name.trim().is_empty() {
            return Err(invalid_table("a service had an empty name"));
        }
        let key = row.name.to_ascii_lowercase();
        if indexed.insert(key, row).is_some() {
            return Err(invalid_table("a service name appeared more than once"));
        }
    }
    Ok(indexed)
}

/// A temporary configuration-query denial is absence of evidence, not a new
/// configuration. Keeping the acknowledged value means recovery can still
/// reveal a start-type edit made while access was unavailable.
pub fn preserve_unavailable_config(
    current: &mut BTreeMap<String, ServiceRecord>,
    acknowledged: &BTreeMap<String, ServiceRecord>,
) {
    for (key, record) in current {
        if record.config_available {
            continue;
        }
        let Some(previous) = acknowledged.get(key) else {
            continue;
        };
        record.start_type = previous.start_type;
        record.config_available = previous.config_available;
    }
}

fn invalid_table(reason: &'static str) -> AppError {
    AppError::InvalidInput {
        what: "service table",
        reason: reason.to_owned(),
    }
}
