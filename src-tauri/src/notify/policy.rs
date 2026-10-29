//! The allow-list for interruptive notifications.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{Category, EventType, Importance, NormalizedEvent};

use super::copy::why_you_might_care;

/// Inputs that are external to an event but affect whether an interruption is
/// appropriate.  Callers should use the event timestamp for replay tests and
/// `now_ms` for live delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationContext {
    /// Current wall-clock time, epoch milliseconds UTC.
    pub now_ms: i64,
    /// Launch time, epoch milliseconds UTC.
    pub launched_at_ms: i64,
    /// The recorder is paused.
    pub monitoring_paused: bool,
    /// A service has an explicit user relationship.
    pub service_has_relationship: bool,
    /// An analysis window already linked this event to a detected failure.
    pub linked_to_failure: bool,
    /// A Docker stop/exit carried a non-zero exit code.
    pub docker_exit_nonzero: bool,
    /// Quiet-hours configuration. `None` means quiet hours are disabled.
    pub quiet_hours: Option<super::rate_limit::QuietHours>,
}

impl Default for NotificationContext {
    fn default() -> Self {
        let now = crate::domain::now_ms();
        Self {
            now_ms: now,
            launched_at_ms: now,
            monitoring_paused: false,
            service_has_relationship: false,
            linked_to_failure: false,
            docker_exit_nonzero: false,
            quiet_hours: None,
        }
    }
}

/// The safe payload handed to a notification adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationCopy {
    /// Event summary, bounded by the event normalizer.
    pub title: String,
    /// Fixed why-care copy plus an optional causal-link sentence.
    pub body: String,
    /// Category used for cooldown accounting.
    pub category: String,
    /// The database event id, when the adapter has one.
    pub event_id: Option<i64>,
}

/// Decide whether one event belongs to the interruptive allow-list.
///
/// Rate limits are intentionally separate (`NotificationRateLimiter`) so this
/// function stays pure and cheap.  Unknown/future event combinations return
/// `None` by default: adding an interruptive surface is an explicit code and
/// test change, never an accidental consequence of a new collector.
pub fn should_notify(
    event: &NormalizedEvent,
    context: &NotificationContext,
) -> Option<NotificationCopy> {
    if context.monitoring_paused || context.now_ms < context.launched_at_ms {
        return None;
    }
    if context.now_ms.saturating_sub(context.launched_at_ms) < 60_000 {
        return None;
    }
    if context
        .quiet_hours
        .is_some_and(|quiet| quiet.contains_epoch_ms(context.now_ms))
    {
        return None;
    }

    let allowed = match event.importance {
        Importance::Critical => true,
        _ => match event.category {
            Category::Driver => matches!(
                event.event_type,
                EventType::Updated | EventType::VersionChanged
            ),
            Category::Network => {
                event.event_type == EventType::ConfigurationChanged
                    && metadata_change_kind(event).is_some_and(|kind| {
                        matches!(kind, "proxy_changed" | "dns_changed" | "proxy" | "dns")
                    })
            }
            Category::Update => event.event_type == EventType::Failed,
            Category::Service => {
                event.event_type == EventType::Failed && context.service_has_relationship
            }
            Category::Docker => {
                matches!(event.event_type, EventType::Stopped | EventType::Failed)
                    && (context.docker_exit_nonzero || metadata_exit_nonzero(event))
            }
            _ => false,
        },
    };
    if !allowed {
        return None;
    }

    let mut body = why_you_might_care(event).to_owned();
    if context.linked_to_failure {
        body.push_str(" This may be related to a problem detected just now.");
    }
    Some(NotificationCopy {
        title: event.user_friendly_summary.chars().take(240).collect(),
        body,
        category: event.category.as_str().to_owned(),
        event_id: None,
    })
}

fn metadata_change_kind(event: &NormalizedEvent) -> Option<&str> {
    let object = event.metadata.as_ref()?.as_object()?;
    object
        .get("change_kind")
        .or_else(|| object.get("kind"))
        .and_then(Value::as_str)
}

fn metadata_exit_nonzero(event: &NormalizedEvent) -> bool {
    let Some(object) = event.metadata.as_ref().and_then(Value::as_object) else {
        return false;
    };
    let numeric = object
        .get("exit_code")
        .or_else(|| object.get("exitCode"))
        .and_then(Value::as_i64)
        .is_some_and(|code| code != 0);
    let unexpected = object
        .get("exit_kind")
        .or_else(|| object.get("exitKind"))
        .or_else(|| object.get("status"))
        .and_then(Value::as_str)
        .is_some_and(|value| {
            matches!(
                value,
                "unexpected" | "container_died" | "died" | "exited_nonzero"
            )
        });
    numeric || unexpected
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::domain::{Confidence, EntityType, Severity};

    fn event(category: Category, event_type: EventType, importance: Importance) -> NormalizedEvent {
        NormalizedEvent {
            timestamp: 1_800_000_000_000,
            category,
            event_type,
            entity_type: EntityType::System,
            entity_id: "system:fixture".into(),
            entity_display_name: "fixture entity".into(),
            severity: Severity::Error,
            importance,
            source_collector: "fixture".into(),
            before: None,
            after: None,
            field: None,
            metadata: None,
            related_entity_ids: vec![],
            user_friendly_summary: "Fixture changed".into(),
            technical_summary: "fixture".into(),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: false,
        }
    }

    fn live() -> NotificationContext {
        NotificationContext {
            now_ms: 1_800_000_000_000 + 120_000,
            launched_at_ms: 1_800_000_000_000,
            ..Default::default()
        }
    }

    #[test]
    fn startup_and_pause_are_always_quiet() {
        let mut context = live();
        context.now_ms = context.launched_at_ms + 59_999;
        assert!(should_notify(
            &event(Category::Driver, EventType::Updated, Importance::Critical),
            &context
        )
        .is_none());
        context.now_ms += 2_000;
        context.monitoring_paused = true;
        assert!(should_notify(
            &event(Category::Driver, EventType::Updated, Importance::Critical),
            &context
        )
        .is_none());
    }

    #[test]
    fn allow_list_and_docker_exit_are_explicit() {
        let mut context = live();
        let driver = should_notify(
            &event(Category::Driver, EventType::Updated, Importance::Important),
            &context,
        );
        assert!(driver.is_some());
        assert!(should_notify(
            &event(
                Category::Application,
                EventType::Updated,
                Importance::Important
            ),
            &context
        )
        .is_none());
        let mut docker = event(Category::Docker, EventType::Stopped, Importance::Important);
        docker.metadata = Some(serde_json::json!({"exit_code": 2}));
        assert!(should_notify(&docker, &context).is_some());
        context.service_has_relationship = false;
        assert!(should_notify(
            &event(Category::Service, EventType::Failed, Importance::Important),
            &context
        )
        .is_none());
    }

    #[test]
    fn critical_events_are_allowed_but_copy_has_no_entity_name() {
        let event = event(Category::System, EventType::Failed, Importance::Critical);
        let copy = should_notify(&event, &live()).unwrap();
        assert_eq!(copy.category, "system");
        assert!(!copy.body.contains("fixture entity"));
    }
}
