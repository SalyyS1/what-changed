//! Settings as the Settings screen sees them.
//!
//! The wire shape *is* the domain shape here, so these are re-exports rather
//! than copies. Settings are domain state — the retention policy decides what
//! the maintenance pass deletes and the collector flags decide what runs — and
//! a second struct with the same fields would be two definitions of one thing,
//! which is how they drift.
//!
//! The stored form is one JSON value per key (P04's `settings` table); the
//! assembly into one object lives in `storage::settings_repository`, because
//! that is the module allowed to read the table.

pub use crate::domain::settings::{
    MonitoringProfile, Settings as SettingsDto, SettingsUpdate as SettingsUpdateDto, Theme,
    VisualizationMode,
};
