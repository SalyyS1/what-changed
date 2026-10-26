//! Stateful notification throttling, kept independent from toast delivery.

use std::collections::{HashMap, VecDeque};

use chrono::Timelike;

use crate::domain::now_ms;

const HOUR_MS: i64 = 60 * 60 * 1_000;
const DAY_MS: i64 = 24 * HOUR_MS;
const CATEGORY_COOLDOWN_MS: i64 = 30 * 60 * 1_000;

/// User-configurable quiet-hours window in local clock hours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
pub struct QuietHours {
    /// Whether the setting is enabled.
    pub enabled: bool,
    /// Start hour, inclusive, 0..=23.
    pub start_hour: u8,
    /// End hour, exclusive, 0..=23. A different value denotes an overnight
    /// window when it is below `start_hour`.
    pub end_hour: u8,
}

impl QuietHours {
    /// The product default: disabled, with the documented fallback window.
    pub const fn default_off() -> Self {
        Self {
            enabled: false,
            start_hour: 22,
            end_hour: 8,
        }
    }

    /// Whether an epoch timestamp falls inside the local quiet window.
    pub fn contains_epoch_ms(self, timestamp_ms: i64) -> bool {
        if !self.enabled {
            return false;
        }
        let hour = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(timestamp_ms)
            .map(|instant| instant.with_timezone(&chrono::Local).hour() as u8)
            .unwrap_or(0);
        if self.start_hour == self.end_hour {
            return true;
        }
        if self.start_hour < self.end_hour {
            (self.start_hour..self.end_hour).contains(&hour)
        } else {
            hour >= self.start_hour || hour < self.end_hour
        }
    }
}

/// Why a candidate was rejected by the limiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allowed,
    Startup,
    HourlyCap,
    DailyCap,
    CategoryCooldown,
    Paused,
    QuietHours,
}

/// A bounded state machine: at most one timestamp per accepted notification
/// and one timestamp per category are retained.
#[derive(Debug, Clone)]
pub struct NotificationRateLimiter {
    launched_at_ms: i64,
    accepted: VecDeque<i64>,
    by_category: HashMap<String, i64>,
}

impl Default for NotificationRateLimiter {
    fn default() -> Self {
        Self::new(now_ms())
    }
}

impl NotificationRateLimiter {
    pub fn new(launched_at_ms: i64) -> Self {
        Self {
            launched_at_ms,
            accepted: VecDeque::new(),
            by_category: HashMap::new(),
        }
    }

    pub fn launched_at_ms(&self) -> i64 {
        self.launched_at_ms
    }

    /// Check and consume one notification slot.
    pub fn admit(
        &mut self,
        category: &str,
        now: i64,
        paused: bool,
        quiet_hours: Option<QuietHours>,
    ) -> RateLimitDecision {
        if paused {
            return RateLimitDecision::Paused;
        }
        if now.saturating_sub(self.launched_at_ms) < 60_000 {
            return RateLimitDecision::Startup;
        }
        if quiet_hours.is_some_and(|quiet| quiet.contains_epoch_ms(now)) {
            return RateLimitDecision::QuietHours;
        }
        while self
            .accepted
            .front()
            .is_some_and(|timestamp| now.saturating_sub(*timestamp) >= DAY_MS)
        {
            self.accepted.pop_front();
        }
        if self.accepted.len() >= 8 {
            return RateLimitDecision::DailyCap;
        }
        let hourly = self
            .accepted
            .iter()
            .filter(|timestamp| now.saturating_sub(**timestamp) < HOUR_MS)
            .count();
        if hourly >= 3 {
            return RateLimitDecision::HourlyCap;
        }
        if self
            .by_category
            .get(category)
            .is_some_and(|timestamp| now.saturating_sub(*timestamp) < CATEGORY_COOLDOWN_MS)
        {
            return RateLimitDecision::CategoryCooldown;
        }
        self.accepted.push_back(now);
        self.by_category.insert(category.to_owned(), now);
        RateLimitDecision::Allowed
    }

    /// Convenience alias for callers that prefer a boolean result.
    pub fn allow(
        &mut self,
        category: &str,
        now: i64,
        paused: bool,
        quiet_hours: Option<QuietHours>,
    ) -> bool {
        self.admit(category, now, paused, quiet_hours) == RateLimitDecision::Allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Local, TimeZone};

    #[test]
    fn enforces_startup_hour_day_and_category_limits() {
        let launch = 1_800_000_000_000;
        let mut limiter = NotificationRateLimiter::new(launch);
        assert_eq!(
            limiter.admit("driver", launch + 59_999, false, None),
            RateLimitDecision::Startup
        );
        assert_eq!(
            limiter.admit("driver", launch + 60_000, false, None),
            RateLimitDecision::Allowed
        );
        assert_eq!(
            limiter.admit("driver", launch + 60_001, false, None),
            RateLimitDecision::CategoryCooldown
        );
        assert_eq!(
            limiter.admit("network", launch + 60_001, false, None),
            RateLimitDecision::Allowed
        );
        assert_eq!(
            limiter.admit("update", launch + 60_002, false, None),
            RateLimitDecision::Allowed
        );
        assert_eq!(
            limiter.admit("service", launch + 60_003, false, None),
            RateLimitDecision::HourlyCap
        );
    }

    #[test]
    fn quiet_hours_can_cross_midnight() {
        let quiet = QuietHours {
            enabled: true,
            start_hour: 22,
            end_hour: 8,
        };
        let today = Local::now().date_naive();
        let at = |hour| {
            Local
                .with_ymd_and_hms(today.year(), today.month(), today.day(), hour, 0, 0)
                .single()
                .map(|instant| instant.timestamp_millis())
        };
        for hour in [22, 2, 12] {
            if let Some(timestamp) = at(hour) {
                assert_eq!(quiet.contains_epoch_ms(timestamp), !(8..22).contains(&hour));
            }
        }
    }
}
