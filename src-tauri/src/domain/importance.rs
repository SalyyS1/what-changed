//! Whether an event deserves attention.
//!
//! This classifier is deliberately a small table of observable facts. It does
//! not infer a cause and it does not hide routine events; it only decides how
//! prominently the timeline presents them.

use super::{Category, EventType, Importance, NormalizedEvent, Severity};

/// Facts that can raise an event above its ordinary category/type baseline.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClassifierContext {
    /// A stopped service was configured to start automatically.
    pub automatic_service: bool,
    /// The driver belongs to a class that commonly affects user-visible I/O.
    pub relevant_driver: bool,
    /// A checkpoint recorded this port as listening.
    pub checkpoint_port: bool,
    /// A shutdown was reported as unexpected by Windows.
    pub unexpected_shutdown: bool,
    /// A browser's own helper subprocess: its parent is a known browser and its
    /// image matches the parent's. Browsing spawns and reaps these constantly,
    /// so they are recorded and never presented as something to notice.
    pub browser_subprocess: bool,
}

/// Classify one event from its facts and the evidence already known about it.
pub fn classify(event: &NormalizedEvent, context: &ClassifierContext) -> Importance {
    // A browser's helper subprocesses come and go with every tab. They are real
    // facts and stay in the history, but nothing about them is worth a person's
    // attention, and no other rule below is allowed to disagree.
    if context.browser_subprocess {
        return Importance::Informational;
    }

    // Routine application updates are always informational. This explicit rule
    // stops the update treadmill from taking over the important-changes list.
    if event.category == Category::Application && event.event_type == EventType::Updated {
        return Importance::Informational;
    }

    if context.unexpected_shutdown && event.category == Category::System {
        return Importance::Critical;
    }

    let important = (event.category == Category::Update && event.event_type == EventType::Failed)
        || (event.category == Category::Driver
            && matches!(
                event.event_type,
                EventType::Updated | EventType::VersionChanged
            )
            && context.relevant_driver)
        || (event.category == Category::Network
            && event.event_type == EventType::ConfigurationChanged)
        || (event.category == Category::Service
            && event.event_type == EventType::Stopped
            && context.automatic_service)
        || (event.entity_type == super::EntityType::Port
            && event.event_type == EventType::Closed
            && context.checkpoint_port);
    if important {
        return Importance::Important;
    }

    match (event.event_type, event.severity) {
        (EventType::Failed, _) => Importance::Important,
        (EventType::Installed | EventType::Uninstalled, _) | (_, Severity::Error) => {
            Importance::Notable
        }
        _ => Importance::Informational,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::synthetic_event;

    #[test]
    fn application_updates_never_become_attention_events() {
        let mut event = synthetic_event(1, 1);
        event.category = Category::Application;
        event.event_type = EventType::Updated;
        event.severity = Severity::Error;
        assert_eq!(
            classify(&event, &ClassifierContext::default()),
            Importance::Informational
        );
    }

    #[test]
    fn only_evidence_backed_escalations_reach_the_top_classes() {
        let mut event = synthetic_event(1, 1);
        event.category = Category::System;
        event.event_type = EventType::Failed;
        assert_eq!(
            classify(&event, &ClassifierContext::default()),
            Importance::Important
        );
        assert_eq!(
            classify(
                &event,
                &ClassifierContext {
                    unexpected_shutdown: true,
                    ..Default::default()
                }
            ),
            Importance::Critical
        );
    }

    #[test]
    fn a_browser_helper_subprocess_is_recorded_but_never_escalated() {
        let mut event = synthetic_event(1, 1);
        event.category = Category::Process;
        event.event_type = EventType::Failed;
        event.severity = Severity::Error;
        assert_eq!(
            classify(&event, &ClassifierContext::default()),
            Importance::Important
        );
        assert_eq!(
            classify(
                &event,
                &ClassifierContext {
                    browser_subprocess: true,
                    ..Default::default()
                }
            ),
            Importance::Informational
        );
    }

    #[test]
    fn an_evidence_backed_driver_rollback_is_as_important_as_an_update() {
        let mut event = synthetic_event(1, 1);
        event.category = Category::Driver;
        event.event_type = EventType::VersionChanged;
        assert_eq!(
            classify(
                &event,
                &ClassifierContext {
                    relevant_driver: true,
                    ..Default::default()
                }
            ),
            Importance::Important
        );
    }
}
