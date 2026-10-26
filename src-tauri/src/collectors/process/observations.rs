//! Turning one process difference into the event that describes it.
//!
//! Two rules shape the copy. A process that denied its path read is named by
//! image and never shown as "unknown", because the recorder knows what it is
//! called and refusing to say so is not honesty. And an exit is never called
//! unexpected: a diff-poll can see that a pid is gone and cannot see why, so
//! the word is reserved for an exit the event-log collector corroborates.

use serde_json::json;

use crate::collectors::process::churn::{Summary, WINDOW_MS};
use crate::collectors::process::diff::Change;
use crate::collectors::process::state::TrackedInstance;
use crate::collectors::process::{identity_confidence, ID};
use crate::collectors::{EntityIdentity, ObservationKind, RawEntity, RawObservation};
use crate::domain::ids::ProcessId;
use crate::domain::{Category, ClassifierContext, EventType, ExitKind, Severity};

/// The event rows for one change, in the order they should be read.
///
/// A reuse writes its stop before its start, so the two rows land in the order
/// the timeline shows them in even though both carry the same timestamp — the
/// cycle that noticed them cannot say which came first.
pub fn for_change(change: &Change, now_ms: i64, uncertain: bool) -> Vec<RawObservation> {
    match change {
        Change::Started(instance) => vec![started(instance, now_ms, uncertain)],
        Change::Stopped(instance) => vec![stopped(instance, now_ms, uncertain)],
        Change::Reused {
            stopped: before,
            started: after,
        } => vec![
            stopped(before, now_ms, uncertain),
            started(after, now_ms, uncertain),
        ],
    }
}

/// The summary event one closed churn window earns.
pub fn for_churn(summary: &Summary) -> RawObservation {
    RawObservation {
        collector_id: ID,
        kind: ObservationKind {
            category: Category::Process,
            event_type: EventType::Churn,
            severity: Severity::Info,
            confidence: summary.confidence,
            user_friendly_summary: format!(
                "{} started and stopped {} times in one minute",
                summary.display_name, summary.pairs
            ),
            technical_summary: format!(
                "process {}: {} matched start/stop pairs in the {} s window",
                summary.display_name,
                summary.pairs,
                WINDOW_MS / 1_000
            ),
            classifier: ClassifierContext::default(),
        },
        entity: entity(summary.display_name.clone(), &summary.entity_id),
        before: None,
        after: None,
        field: None,
        // The window's close, not a process start. The metadata says which, so
        // a reader is never invited to treat this row as one more start.
        occurred_at: summary.window_end_ms,
        details: Some(json!({
            "summary_of_window": true,
            "count": summary.pairs,
            "pairs": summary.pairs,
            "window_start_ms": summary.window_start_ms,
            "window_end_ms": summary.window_end_ms,
        })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: false,
    }
}

fn started(instance: &TrackedInstance, now_ms: i64, uncertain: bool) -> RawObservation {
    observation(
        instance,
        EventType::Started,
        format!("{} started", instance.image),
        format!("process {} (pid {}) started", instance.image, instance.pid),
        now_ms,
        uncertain,
        json!({
            "pid": instance.pid,
            "path": instance.executable,
            // Deferred rows deliberately discard the raw path after their
            // first bounded batch. `path_available` still describes the
            // identity evidence; the payload can only disclose what remains.
            "path_unavailable": instance.executable.is_none(),
            "path_omitted_after_defer": instance.path_available && instance.executable.is_none(),
            "browser_subprocess": instance.browser_subtree,
        }),
    )
}

fn stopped(instance: &TrackedInstance, now_ms: i64, uncertain: bool) -> RawObservation {
    observation(
        instance,
        EventType::Stopped,
        format!("{} stopped", instance.image),
        format!("process {} (pid {}) stopped", instance.image, instance.pid),
        now_ms,
        uncertain,
        json!({
            "pid": instance.pid,
            "path_unavailable": !instance.path_available,
            "exit_kind": ExitKind::Unknown.as_str(),
            "browser_subprocess": instance.browser_subtree,
        }),
    )
}

fn observation(
    instance: &TrackedInstance,
    event_type: EventType,
    user_friendly_summary: String,
    technical_summary: String,
    now_ms: i64,
    uncertain: bool,
    details: serde_json::Value,
) -> RawObservation {
    RawObservation {
        collector_id: ID,
        kind: ObservationKind {
            category: Category::Process,
            event_type,
            severity: Severity::Info,
            // A process needs both a stable executable identity and a creation
            // time before a recycled pid can be ruled out.
            confidence: identity_confidence(instance),
            user_friendly_summary,
            technical_summary,
            classifier: ClassifierContext {
                browser_subprocess: instance.browser_subtree,
                ..Default::default()
            },
        },
        entity: entity(instance.image.clone(), &instance.entity_id),
        before: None,
        after: None,
        field: None,
        occurred_at: now_ms,
        details: Some(details),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: uncertain,
    }
}

fn entity(display_name: String, id: &ProcessId) -> RawEntity {
    RawEntity {
        identity: EntityIdentity::ProcessById { id: id.clone() },
        display_name,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::collectors::process::churn;
    use crate::collectors::process::diff::Change;
    use crate::domain::ids::ProcessId;
    use crate::domain::{Confidence, Importance};

    fn tracked(image: &str, path: Option<&str>) -> TrackedInstance {
        let executable = path.map(str::to_owned);
        TrackedInstance {
            pid: 4_242,
            creation: Some(7),
            creation_available: true,
            image: image.to_owned(),
            entity_id: ProcessId::from_executable(path.unwrap_or(image)),
            identity_path_based: executable.is_some(),
            path_available: executable.is_some(),
            executable,
            browser_subtree: false,
            observed_start: true,
        }
    }

    #[test]
    fn a_denied_path_degrades_the_event_rather_than_naming_it_unknown() {
        let denied = tracked("svchost.exe", None);
        let event = started(&denied, 1_000, false);
        assert_eq!(event.entity.display_name, "svchost.exe");
        assert_eq!(event.kind.user_friendly_summary, "svchost.exe started");
        assert_eq!(event.kind.confidence, Confidence::Low);
        let details = event.details.unwrap();
        assert_eq!(details["path"], serde_json::Value::Null);
        assert_eq!(details["path_unavailable"], true);
        assert_eq!(details["path_omitted_after_defer"], false);
        assert!(
            !event.kind.user_friendly_summary.contains("unknown"),
            "the recorder knows the image name and says so"
        );

        let readable = tracked("api.exe", Some(r"C:\dev\api.exe"));
        let details = started(&readable, 1_000, false).details.unwrap();
        assert_eq!(details["path"], r"C:\dev\api.exe");
        assert_eq!(details["path_unavailable"], false);
        assert_eq!(details["path_omitted_after_defer"], false);
    }

    #[test]
    fn a_missing_creation_time_degrades_an_otherwise_readable_process() {
        let mut instance = tracked("api.exe", Some(r"C:\dev\api.exe"));
        instance.creation_available = false;
        let event = started(&instance, 1_000, false);
        assert_eq!(event.kind.confidence, Confidence::Low);
        assert_eq!(event.details.unwrap()["path_unavailable"], false);
    }

    #[test]
    fn an_exit_is_unknown_because_a_diff_poll_cannot_see_why() {
        let details = stopped(&tracked("api.exe", Some("api.exe")), 1_000, false)
            .details
            .unwrap();
        assert_eq!(details["exit_kind"], ExitKind::Unknown.as_str());
        assert_eq!(details["exit_kind"], "unknown");
    }

    #[test]
    fn a_reuse_writes_its_stop_before_its_start() {
        let change = Change::Reused {
            stopped: tracked("api.exe", Some("api.exe")),
            started: tracked("api.exe", Some("api.exe")),
        };
        let events = for_change(&change, 1_000, false);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind.event_type, EventType::Stopped);
        assert_eq!(events[1].kind.event_type, EventType::Started);
        assert_eq!(events[0].occurred_at, events[1].occurred_at);
    }

    #[test]
    fn a_downtime_diff_marks_its_timestamps_uncertain() {
        let certain = for_change(
            &Change::Started(tracked("api.exe", Some("api.exe"))),
            1_000,
            false,
        );
        assert!(!certain[0].occurred_at_is_uncertain);
        let uncertain = for_change(
            &Change::Started(tracked("api.exe", Some("api.exe"))),
            1_000,
            true,
        );
        assert!(uncertain[0].occurred_at_is_uncertain);
        assert_eq!(
            uncertain[0].occurred_at, 1_000,
            "the detection time is real and is what the row carries"
        );
    }

    #[test]
    fn a_churn_summary_describes_a_window_and_not_a_start() {
        let summary = Summary {
            entity_id: ProcessId::from_executable("flaky.exe"),
            display_name: "flaky.exe".to_owned(),
            pairs: 8,
            confidence: Confidence::High,
            window_start_ms: 1_000,
            window_end_ms: 1_000 + churn::WINDOW_MS,
        };
        let event = for_churn(&summary);
        assert_eq!(event.kind.event_type, EventType::Churn);
        assert_eq!(event.kind.category, Category::Process);
        assert_eq!(event.occurred_at, summary.window_end_ms);
        assert_eq!(
            event.kind.user_friendly_summary,
            "flaky.exe started and stopped 8 times in one minute"
        );
        let details = event.details.as_ref().unwrap();
        assert_eq!(details["pairs"], 8);
        assert_eq!(details["count"], 8);
        assert_eq!(details["summary_of_window"], true);
        assert_eq!(details["window_start_ms"], 1_000);
        assert!(!event.occurred_at_is_uncertain);
        // A churn summary is a fact about noise, so it must not present itself
        // as something to notice.
        let normalized = crate::collectors::normalize::Normalizer::new().normalize(event);
        assert_eq!(normalized.get().importance, Importance::Informational);
        assert_eq!(normalized.get().event_type, EventType::Churn);
    }

    #[test]
    fn a_browser_helper_is_flagged_for_the_classifier() {
        let mut helper = tracked("chrome.exe", Some("chrome.exe"));
        helper.browser_subtree = true;
        let event = started(&helper, 1_000, false);
        assert!(event.kind.classifier.browser_subprocess);
        assert_eq!(event.details.unwrap()["browser_subprocess"], true);
    }
}
