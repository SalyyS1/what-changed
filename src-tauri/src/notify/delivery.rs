//! Thin Tauri adapter. Policy code never depends on this module.

use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::app::AppState;
use crate::domain::now_ms;
use crate::storage::{event_repository, settings_repository};

use super::{
    should_notify, NotificationContext, NotificationCopy, NotificationRateLimiter, QuietHours,
    RateLimitDecision,
};

/// Frontend payload emitted after the OS adapter accepts a notification.
/// `event_id` is durable and is the only value needed to route to a timeline
/// row; the copy is included for a lightweight in-app confirmation surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NotificationEvent {
    pub event_id: i64,
    pub title: String,
    pub category: String,
}

/// The event name is duplicated here so the adapter remains usable without
/// importing the app event-bus module (which owns Tauri runtime wiring).
pub const EVENT_NOTIFICATION: &str = "notification-event";

static LIMITER: OnceLock<Mutex<NotificationRateLimiter>> = OnceLock::new();

/// Start the rate-limit clock as the event bus is attached during app setup.
/// This preserves the 60-second launch grace period even when the first event
/// arrives long after process startup.
pub fn initialize() {
    let _ = LIMITER.set(Mutex::new(NotificationRateLimiter::new(now_ms())));
}

/// Deliver one already-approved notification. Permission failures are
/// returned to the caller so it can keep the in-app recent-important surface
/// as the fallback; this function never opens a modal or retries.
pub fn deliver<R: Runtime>(
    app: &AppHandle<R>,
    copy: &NotificationCopy,
) -> tauri_plugin_notification::Result<()> {
    let builder = app
        .notification()
        .builder()
        .title(copy.title.clone())
        .body(copy.body.clone())
        .auto_cancel();
    let builder = match copy.event_id {
        Some(event_id) => builder.extra("event_id", event_id),
        None => builder,
    };
    builder.show()
}

/// Evaluate and deliver the bounded event ids carried by one coalesced writer
/// notice. Database reads stay inside the repository boundary and policy is
/// applied before any OS call, so routine events never become interruptions.
pub fn deliver_batch<R: Runtime>(app: &AppHandle<R>, event_ids: &[i64]) {
    if event_ids.is_empty() {
        return;
    }
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    // Notifications are part of the same local-only promise as collection:
    // do not surface pre-consent history, and stay quiet in crash-loop safe
    // mode until the user explicitly resumes normal startup.
    if state.startup.safe_mode {
        return;
    }
    let (onboarding_completed, quiet_hours) = match state.db.read(|conn| {
        let settings = settings_repository::load(conn)?;
        let quiet_hours = settings_repository::get::<QuietHours>(conn, "quiet_hours")?;
        Ok((settings.onboarding_completed, quiet_hours))
    }) {
        Ok((completed, value)) => (completed, value.or(Some(QuietHours::default_off()))),
        Err(error) => {
            tracing::debug!(%error, "notification settings could not be read; staying quiet until they are available");
            (false, Some(QuietHours::default_off()))
        }
    };
    if !onboarding_completed {
        return;
    }
    let events = match state.db.read(|conn| {
        event_ids
            .iter()
            .filter_map(|id| match event_repository::by_id(conn, *id) {
                Ok(Some(event)) => Some(Ok(event)),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<crate::error::AppResult<Vec<_>>>()
    }) {
        Ok(events) => events,
        Err(error) => {
            tracing::debug!(%error, "notification candidates could not be loaded");
            return;
        }
    };
    let Some(limiter) = LIMITER.get() else {
        // Defensive fallback for callers that emit before the bus is attached.
        initialize();
        return deliver_batch(app, event_ids);
    };
    let launched_at_ms = limiter
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .launched_at_ms();
    let paused = state.monitoring.snapshot().paused;
    for event in events {
        let now = now_ms();
        let context = NotificationContext {
            now_ms: now,
            launched_at_ms,
            monitoring_paused: paused,
            service_has_relationship: !event.event.related_entity_ids.is_empty(),
            linked_to_failure: false,
            docker_exit_nonzero: false,
            quiet_hours,
        };
        let Some(mut copy) = should_notify(&event.event, &context) else {
            continue;
        };
        copy.event_id = Some(event.id);
        let decision = limiter
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .admit(&copy.category, now, paused, quiet_hours);
        if decision != RateLimitDecision::Allowed {
            continue;
        }
        if let Err(error) = deliver(app, &copy) {
            tracing::debug!(%error, event_id = event.id, "OS notification could not be delivered");
            continue;
        }
        let payload = NotificationEvent {
            event_id: event.id,
            title: copy.title,
            category: copy.category,
        };
        if let Err(error) = Emitter::emit(app, EVENT_NOTIFICATION, payload) {
            tracing::debug!(%error, event_id = event.id, "notification event could not be delivered to the window");
        }
    }
}
