use what_changed_lib::analysis::explain::{relative_time, why_ranked, Explanation};
use what_changed_lib::analysis::remediation;
use what_changed_lib::domain::*;

#[test]
fn non_confirmed_copy_never_claims_a_cause() {
    let forbidden = [
        "caused",
        "because of",
        "the problem is",
        "this broke",
        "definitely",
        "root cause",
    ];
    for index in 0..200 {
        let class = match index % 3 {
            0 => ConfidenceClass::Confirmed,
            1 => ConfidenceClass::Likely,
            _ => ConfidenceClass::Correlated,
        };
        let event = fixture_event(i64::from(index));
        let breakdown = ScoreBreakdown {
            temporal: 0.5,
            relationship: if class == ConfidenceClass::Correlated {
                0.0
            } else {
                1.0
            },
            category: 1.0,
            importance: 0.35,
            rarity: 1.0,
            failure_proximity: if class == ConfidenceClass::Confirmed {
                1.0
            } else {
                0.0
            },
            chain_bonus: 0.75,
            total: 5.0,
        };
        let copy = [
            Explanation::for_class(class, &format!("entity-{index}")),
            relative_time(&event, 100_000),
            why_ranked(&event, &breakdown, None).join(" "),
            remediation::advice(&event).to_owned(),
        ]
        .join(" ")
        .to_lowercase();
        if class != ConfidenceClass::Confirmed {
            for phrase in forbidden {
                assert!(!copy.contains(phrase), "{copy}");
            }
        }
    }
}

fn fixture_event(id: i64) -> SystemEvent {
    SystemEvent {
        id,
        event: NormalizedEvent {
            timestamp: 50_000,
            category: Category::Configuration,
            event_type: EventType::ConfigurationChanged,
            entity_type: EntityType::File,
            entity_id: format!("file:{id}"),
            entity_display_name: format!("Config {id}"),
            severity: Severity::Info,
            importance: Importance::Notable,
            source_collector: "fixture".into(),
            before: None,
            after: None,
            field: None,
            metadata: None,
            related_entity_ids: vec![],
            user_friendly_summary: "Configuration changed".into(),
            technical_summary: "configuration changed".into(),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: false,
        },
    }
}
