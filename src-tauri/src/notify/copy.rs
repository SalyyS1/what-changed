//! Stable, short notification copy.

use crate::domain::{Category, NormalizedEvent};

/// Return the one-line explanation shown below a notification title.
///
/// This is deliberately a closed set of product copy.  It never interpolates
/// an entity name or a collector-owned value into the body, which keeps lock
/// screen notifications useful without turning them into a second data sink.
pub fn why_you_might_care(event: &NormalizedEvent) -> &'static str {
    match (event.category, event.event_type) {
        (Category::Driver, _) => "A driver change can affect devices and application behaviour.",
        (Category::Network, _) => "A network configuration change can affect connectivity.",
        (Category::Update, _) => "An update did not complete and may need attention.",
        (Category::Service, _) => "A service change can affect a feature that depends on it.",
        (Category::Docker, _) => "A container change can affect a local development workflow.",
        (Category::System, _) => "A critical system change deserves a closer look.",
        _ => "This change may be relevant to the problem you are investigating.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Confidence, EntityType, EventType, Importance, Severity};

    fn event(category: Category) -> NormalizedEvent {
        NormalizedEvent {
            timestamp: 1,
            category,
            event_type: EventType::Failed,
            entity_type: EntityType::System,
            entity_id: "system:local".into(),
            entity_display_name: "fixture-name".into(),
            severity: Severity::Error,
            importance: Importance::Critical,
            source_collector: "fixture".into(),
            before: None,
            after: None,
            field: None,
            metadata: None,
            related_entity_ids: vec![],
            user_friendly_summary: "fixture summary".into(),
            technical_summary: "fixture technical".into(),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: false,
        }
    }

    #[test]
    fn copy_is_closed_product_text() {
        let text = why_you_might_care(&event(Category::Network));
        assert!(text.contains("network"));
        assert!(!text.contains("fixture-name"));
    }
}
