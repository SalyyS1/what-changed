//! What the user chose.
//!
//! Settings are domain state, not a transfer shape: the retention policy
//! decides what the maintenance pass deletes, the monitoring profile decides
//! how hard collectors work, and the collector flags decide what runs at all.
//! They live here for the same reason the event vocabularies do — the layer
//! that acts on a value should not have to import the layer that displays it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::enums::text_enum;
use super::retention::RetentionPolicy;

text_enum! {
    /// What the user chose in Appearance. `System` follows Windows.
    Theme, "theme" {
        Dark => "dark",
        Light => "light",
        System => "system",
    }
}

text_enum! {
    /// Which Change Map renderer the user prefers.
    VisualizationMode, "visualization mode" {
        Auto => "auto",
        ThreeD => "3d",
        TwoD => "2d",
    }
}

text_enum! {
    /// How hard the collectors work. Cadence and sweep staggering read this.
    MonitoringProfile, "monitoring profile" {
        Minimal => "minimal",
        Balanced => "balanced",
        Thorough => "thorough",
    }
}

/// Every setting the product has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Settings {
    /// Appearance.
    pub theme: Theme,
    /// Automatic capability detection, forced 3D, or forced 2D.
    pub visualization: VisualizationMode,
    /// How long history is kept.
    pub retention: RetentionPolicy,
    /// How hard the collectors work.
    pub monitoring_profile: MonitoringProfile,
    /// Which collectors are enabled, by collector id.
    pub collectors: BTreeMap<String, bool>,
    /// Whether the first-run tour has been completed.
    pub onboarding_completed: bool,
    /// Whether the developer surfaces are shown.
    pub developer_mode: bool,
}

/// A partial update. Every field the caller omits keeps its stored value.
///
/// `optional_fields = nullable` so the generated TypeScript is `theme?: Theme
/// | null` rather than a required nullable field. The whole point of this
/// shape is that a toggle sends the one thing it changed, which means omitting
/// the rest — a required field would force every caller to spell out five
/// nulls, and the first one to forget would be a compile error for a value
/// serde already treats as absent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(optional_fields = nullable)]
pub struct SettingsUpdate {
    /// New appearance, if it changed.
    #[serde(default)]
    pub theme: Option<Theme>,
    /// New Change Map renderer preference, if it changed.
    #[serde(default)]
    pub visualization: Option<VisualizationMode>,
    /// New retention lifetimes, if they changed.
    #[serde(default)]
    pub retention: Option<RetentionPolicy>,
    /// New monitoring profile, if it changed.
    #[serde(default)]
    pub monitoring_profile: Option<MonitoringProfile>,
    /// Collector flags to change. Keys not present are left alone; a key that
    /// is not a known collector is rejected rather than stored.
    #[serde(default)]
    pub collectors: Option<BTreeMap<String, bool>>,
    /// Whether the first-run tour has been completed.
    #[serde(default)]
    pub onboarding_completed: Option<bool>,
    /// Whether the developer surfaces are shown.
    #[serde(default)]
    pub developer_mode: Option<bool>,
}

/// Retention lifetimes the product supports, in days.
///
/// The floor is one day, because zero would mean "delete everything on the
/// next maintenance pass" and that is the Delete-everything button's job, with
/// its own confirmation. The ceiling is ten years.
pub const MIN_RETENTION_DAYS: u32 = 1;
pub const MAX_RETENTION_DAYS: u32 = 3_650;

impl RetentionPolicy {
    /// Whether every lifetime is inside the supported range.
    pub fn is_supported(&self) -> bool {
        [self.events_days, self.snapshots_days, self.important_days]
            .iter()
            .all(|days| (MIN_RETENTION_DAYS..=MAX_RETENTION_DAYS).contains(days))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_supported_and_zero_days_is_not() {
        assert!(RetentionPolicy::default().is_supported());
        assert!(!RetentionPolicy {
            events_days: 0,
            snapshots_days: 7,
            important_days: 90,
        }
        .is_supported());
        assert!(!RetentionPolicy {
            events_days: 30,
            snapshots_days: 7,
            important_days: MAX_RETENTION_DAYS + 1,
        }
        .is_supported());
    }

    #[test]
    fn an_update_with_nothing_set_changes_nothing() {
        let update = SettingsUpdate::default();
        assert!(update.theme.is_none());
        assert!(update.collectors.is_none());
        assert!(update.retention.is_none());
    }
}
