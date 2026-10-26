//! Collector self-description and live health.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::enums::text_enum;

text_enum! {
    /// Relative cost of taking a collector snapshot.
    CollectorCost, "collector cost" {
        Cheap => "cheap",
        Moderate => "moderate",
        Expensive => "expensive",
    }
}

text_enum! {
    /// Docker integration state shown in Diagnostics without exposing pipe errors.
    DockerCollectorState, "docker collector state" {
        Probing => "probing",
        Available => "available",
        Reconnecting => "reconnecting",
        Unavailable => "unavailable",
    }
}

text_enum! {
    /// Notification source a collector actually obtained at runtime.
    CollectorWakeTier, "collector wake tier" {
        ConfigurationManager => "configuration_manager",
        Registry => "registry",
        TimerOnly => "timer_only",
    }
}

/// Small, closed collector-specific diagnostics exposed over IPC.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CollectorStatusDetails {
    pub wake_tier: Option<CollectorWakeTier>,
    #[serde(default)]
    pub docker_state: Option<DockerCollectorState>,
    #[serde(default)]
    pub docker_api_version: Option<String>,
}

/// How a collector is scheduled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorCadence {
    Poll(Duration),
    Notify,
    NotifyPlusSweep(Duration),
}

/// Copy and scheduling facts owned by the collector itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub what_it_watches: &'static str,
    pub why_it_matters: &'static str,
    pub cost_hint: CollectorCost,
    pub cadence: CollectorCadence,
    pub requires_admin: bool,
}

/// Serializable cadence shown by Settings and Diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CollectorCadenceDto {
    Poll { interval_ms: u64 },
    Notify,
    NotifyPlusSweep { interval_ms: u64 },
}

impl From<CollectorCadence> for CollectorCadenceDto {
    fn from(value: CollectorCadence) -> Self {
        match value {
            CollectorCadence::Poll(value) => Self::Poll {
                interval_ms: millis(value),
            },
            CollectorCadence::Notify => Self::Notify,
            CollectorCadence::NotifyPlusSweep(value) => Self::NotifyPlusSweep {
                interval_ms: millis(value),
            },
        }
    }
}

fn millis(value: Duration) -> u64 {
    u64::try_from(value.as_millis()).unwrap_or(u64::MAX)
}

/// Descriptor shape returned over IPC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CollectorDescriptorDto {
    pub id: String,
    pub display_name: String,
    pub what_it_watches: String,
    pub why_it_matters: String,
    pub cost_hint: CollectorCost,
    pub cadence: CollectorCadenceDto,
    pub requires_admin: bool,
}

impl From<&CollectorDescriptor> for CollectorDescriptorDto {
    fn from(value: &CollectorDescriptor) -> Self {
        Self {
            id: value.id.to_owned(),
            display_name: value.display_name.to_owned(),
            what_it_watches: value.what_it_watches.to_owned(),
            why_it_matters: value.why_it_matters.to_owned(),
            cost_hint: value.cost_hint,
            cadence: value.cadence.into(),
            requires_admin: value.requires_admin,
        }
    }
}
