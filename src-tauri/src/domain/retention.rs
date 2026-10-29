//! How long history is kept.
//!
//! Three lifetimes, all in days: routine events, detailed snapshots, and
//! events a person was told to care about. Snapshots that belong to a
//! checkpoint are exempt from the snapshot lifetime — a marker you named as
//! "it worked here" has to still be comparable when you come back to it —
//! and that exemption lives in the query that prunes, not here.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Milliseconds in one day.
pub const DAY_MS: i64 = 86_400_000;

/// The lifetimes a user can pick without typing a number.
pub const PRESET_DAYS: &[u32] = &[7, 30, 90];

/// Lifetime of informational process start/stop events, in days.
///
/// Short-lived helper processes (shells, consoles, build tools) are the
/// volume driver on a developer machine, with hundreds of events a minute and
/// almost none of them important. They matter for a recent symptom and
/// rarely after a few days, so they expire sooner than `events_days`. The
/// effective lifetime is never longer than the user's own `events_days`.
pub const PROCESS_NOISE_DAYS: u32 = 3;

/// Upper bound on live database pages, in bytes (1 GiB).
///
/// Past it, the oldest informational events go first until the database is
/// back under [`SIZE_CAP_TARGET_PERCENT`] of the cap. Notable, important and
/// critical events are never removed to satisfy the cap.
pub const MAX_DATABASE_BYTES: u64 = 1_073_741_824;

/// How far under the cap a size prune continues, so one overflow does not
/// trigger a prune at every pass.
pub const SIZE_CAP_TARGET_PERCENT: u64 = 90;

/// Retention lifetimes, in days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct RetentionPolicy {
    /// Routine events: everything below `important`.
    pub events_days: u32,
    /// Detailed snapshots not owned by a checkpoint.
    pub snapshots_days: u32,
    /// Events classified `important` or `critical`.
    pub important_days: u32,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            events_days: 30,
            snapshots_days: 7,
            important_days: 90,
        }
    }
}

impl RetentionPolicy {
    /// Rows with a timestamp before this are expired routine events.
    pub fn events_cutoff(&self, now: i64) -> i64 {
        now.saturating_sub(i64::from(self.events_days) * DAY_MS)
    }

    /// Rows with a timestamp before this are expired important events.
    pub fn important_cutoff(&self, now: i64) -> i64 {
        now.saturating_sub(i64::from(self.important_days) * DAY_MS)
    }

    /// Informational process events captured before this are expired. This is
    /// at most `events_days` back, so a user's shorter choice still wins.
    pub fn process_noise_cutoff(&self, now: i64) -> i64 {
        let days = self.events_days.min(PROCESS_NOISE_DAYS);
        now.saturating_sub(i64::from(days) * DAY_MS)
    }

    /// Snapshots captured before this are expired, unless a checkpoint owns them.
    pub fn snapshots_cutoff(&self, now: i64) -> i64 {
        now.saturating_sub(i64::from(self.snapshots_days) * DAY_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_thirty_seven_ninety() {
        let policy = RetentionPolicy::default();
        assert_eq!(
            (
                policy.events_days,
                policy.snapshots_days,
                policy.important_days
            ),
            (30, 7, 90)
        );
    }

    #[test]
    fn cutoffs_count_back_whole_days() {
        let policy = RetentionPolicy::default();
        let now = 100 * DAY_MS;
        assert_eq!(policy.events_cutoff(now), 70 * DAY_MS);
        assert_eq!(policy.snapshots_cutoff(now), 93 * DAY_MS);
        assert_eq!(policy.important_cutoff(now), 10 * DAY_MS);
    }

    #[test]
    fn process_noise_expires_after_three_days_but_never_after_the_users_own_lifetime() {
        let now = 100 * DAY_MS;
        assert_eq!(
            RetentionPolicy::default().process_noise_cutoff(now),
            97 * DAY_MS
        );
        let short = RetentionPolicy {
            events_days: 1,
            snapshots_days: 7,
            important_days: 90,
        };
        assert_eq!(short.process_noise_cutoff(now), 99 * DAY_MS);
    }

    #[test]
    fn a_cutoff_never_underflows() {
        let policy = RetentionPolicy {
            events_days: u32::MAX,
            snapshots_days: 7,
            important_days: 90,
        };
        assert!(policy.events_cutoff(0) < 0);
        assert_eq!(policy.events_cutoff(i64::MIN), i64::MIN);
    }
}
