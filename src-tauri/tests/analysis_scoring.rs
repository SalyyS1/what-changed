use what_changed_lib::analysis::score::{rarity, temporal};
use what_changed_lib::domain::*;

fn event(timestamp: i64, uncertain: bool) -> SystemEvent {
    SystemEvent {
        id: 1,
        event: NormalizedEvent {
            timestamp,
            category: Category::Update,
            event_type: EventType::Updated,
            entity_type: EntityType::Update,
            entity_id: "update:test".into(),
            entity_display_name: "Test update".into(),
            severity: Severity::Info,
            importance: Importance::Notable,
            source_collector: "fixture".into(),
            before: None,
            after: None,
            field: None,
            metadata: None,
            related_entity_ids: vec![],
            user_friendly_summary: "Updated".into(),
            technical_summary: "updated".into(),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: uncertain,
        },
    }
}

#[test]
fn temporal_term_covers_zero_ninety_after_and_uncertain_boundaries() {
    assert_eq!(temporal(&event(1_000, false), 1_000), 1.0);
    let ninety = temporal(&event(1_000, false), 1_000 + 90 * 60_000);
    assert!((ninety - (-1.0_f64).exp()).abs() < 1e-9);
    assert_eq!(temporal(&event(1_001, false), 1_000), 0.0);
    assert_eq!(temporal(&event(900, true), 1_000), 0.0);
}

#[test]
fn importance_term_uses_the_four_calm_attention_levels() {
    assert_eq!(importance_value(Importance::Informational), 0.15);
    assert_eq!(importance_value(Importance::Notable), 0.35);
    assert_eq!(importance_value(Importance::Important), 0.6);
    assert_eq!(importance_value(Importance::Critical), 1.0);
}

#[test]
fn rarity_term_has_first_and_twentieth_occurrence_boundaries() {
    assert_eq!(rarity(0), 1.0);
    assert_eq!(rarity(18), 0.1);
    assert_eq!(rarity(20), 0.0);
    assert_eq!(rarity(u32::MAX), 0.0);
}
