//! Phase 19 notification contract tests: calm by default and allow-list only.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use what_changed_lib::domain::{Category, EventType, Importance};
use what_changed_lib::notify::{
    should_notify, NotificationContext, NotificationRateLimiter, RateLimitDecision,
};
use what_changed_lib::testing::synthetic_event;

#[test]
fn routine_events_never_cross_the_policy_boundary() {
    let mut event = synthetic_event(1, 1_800_000_000_000);
    event.category = Category::Application;
    event.event_type = EventType::Updated;
    event.importance = Importance::Informational;
    let context = NotificationContext {
        launched_at_ms: event.timestamp - 120_000,
        now_ms: event.timestamp,
        ..Default::default()
    };
    assert!(should_notify(&event, &context).is_none());
}

#[test]
fn three_hourly_slots_and_category_cooldown_are_enforced() {
    let mut limiter = NotificationRateLimiter::new(1_800_000_000_000);
    let start = limiter.launched_at_ms() + 60_000;
    assert_eq!(
        limiter.admit("driver", start, false, None),
        RateLimitDecision::Allowed
    );
    assert_eq!(
        limiter.admit("driver", start + 1, false, None),
        RateLimitDecision::CategoryCooldown
    );
    assert_eq!(
        limiter.admit("network", start + 2, false, None),
        RateLimitDecision::Allowed
    );
    assert_eq!(
        limiter.admit("update", start + 3, false, None),
        RateLimitDecision::Allowed
    );
    assert_eq!(
        limiter.admit("service", start + 4, false, None),
        RateLimitDecision::HourlyCap
    );
}

#[test]
fn a_day_of_four_thousand_events_yields_only_three_calm_notifications() {
    // Keep the replay deterministic and deliberately spread the three
    // candidates farther apart than both the hourly cap and category
    // cooldown. The remaining 3,900 rows are realistic routine noise.
    let launch = 1_800_000_000_000_i64;
    let mut limiter = NotificationRateLimiter::new(launch);
    let mut notified = Vec::new();
    for index in 0..4_000_u64 {
        // Keep the complete 4,000-row replay inside one 24-hour window. The
        // three interruptive candidates still land six hours apart, so this
        // exercises the daily budget rather than accidentally spanning weeks.
        let timestamp = launch + 60_000 + (index as i64) * (24 * 60 * 60_000 / 4_000);
        let mut event = synthetic_event(index, timestamp);
        event.importance = Importance::Informational;
        event.category = Category::Application;
        event.event_type = EventType::Started;

        // Two driver updates and one proxy change are the only interruptive
        // rows in the stream; their timestamps are over an hour apart.
        match index {
            1_000 | 2_000 => {
                event.category = Category::Driver;
                event.event_type = EventType::Updated;
                event.importance = Importance::Important;
            }
            3_000 => {
                event.category = Category::Network;
                event.event_type = EventType::ConfigurationChanged;
                event.metadata = Some(serde_json::json!({"change_kind": "proxy_changed"}));
                event.importance = Importance::Important;
            }
            _ => {}
        }

        let context = NotificationContext {
            launched_at_ms: launch,
            now_ms: timestamp,
            ..Default::default()
        };
        if let Some(copy) = should_notify(&event, &context) {
            if limiter.admit(&copy.category, timestamp, false, None) == RateLimitDecision::Allowed {
                notified.push((timestamp, copy.category));
            }
        }
    }
    assert_eq!(notified.len(), 3);
    assert!(notified
        .windows(2)
        .all(|pair| pair[1].0 - pair[0].0 >= 30 * 60_000));
    assert!(notified
        .iter()
        .all(|(_, category)| { matches!(category.as_str(), "driver" | "network") }));
}

#[test]
fn startup_pause_and_quiet_hours_never_consume_a_notification_slot() {
    let launch = 1_800_000_000_000_i64;
    let mut event = synthetic_event(7, launch + 120_000);
    event.importance = Importance::Critical;
    let quiet = what_changed_lib::notify::QuietHours {
        enabled: true,
        start_hour: 0,
        end_hour: 23,
    };
    let base = NotificationContext {
        launched_at_ms: launch,
        now_ms: launch + 120_000,
        ..Default::default()
    };
    assert!(should_notify(
        &event,
        &NotificationContext {
            now_ms: launch + 59_999,
            ..base
        }
    )
    .is_none());
    assert!(should_notify(
        &event,
        &NotificationContext {
            monitoring_paused: true,
            ..base
        }
    )
    .is_none());
    assert!(should_notify(
        &event,
        &NotificationContext {
            quiet_hours: Some(quiet),
            ..base
        }
    )
    .is_none());
}
