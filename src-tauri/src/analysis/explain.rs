use crate::domain::{ConfidenceClass, ScoreBreakdown, SystemEvent};

/// The already-ranked facts an explanation may use. In particular, a provider
/// cannot access event before/after values or metadata through this contract.
#[derive(Debug, Clone, Copy)]
pub struct ExplanationFacts<'a> {
    pub class: ConfidenceClass,
    pub entity_display_name: &'a str,
    pub occurred_at_is_uncertain: bool,
    pub event_timestamp_ms: i64,
    pub symptom_time_ms: i64,
    pub score: &'a ScoreBreakdown,
    pub exact_pair_gap_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplanationCopy {
    pub confidence_text: String,
    pub relative_time: String,
    pub why_ranked: Vec<String>,
}

/// A replaceable rendering seam, not permission to send facts off-device.
pub trait ExplanationProvider {
    fn id(&self) -> &'static str;
    fn explain(&self, facts: ExplanationFacts<'_>) -> ExplanationCopy;
}

/// The only provider wired into the MVP; it performs no I/O.
pub struct DeterministicExplanationProvider;

impl ExplanationProvider for DeterministicExplanationProvider {
    fn id(&self) -> &'static str {
        "deterministic"
    }

    fn explain(&self, facts: ExplanationFacts<'_>) -> ExplanationCopy {
        ExplanationCopy {
            confidence_text: Explanation::for_class(facts.class, facts.entity_display_name),
            relative_time: relative_time_for_facts(
                facts.occurred_at_is_uncertain,
                facts.event_timestamp_ms,
                facts.symptom_time_ms,
            ),
            why_ranked: why_ranked_for_facts(
                facts.occurred_at_is_uncertain,
                facts.score,
                facts.exact_pair_gap_ms,
            ),
        }
    }
}

pub struct Explanation;

impl Explanation {
    pub fn for_class(class: ConfidenceClass, entity: &str) -> String {
        match class {
            ConfidenceClass::Confirmed => format!("This change directly affected {entity}."),
            ConfidenceClass::Likely => format!("This change likely affected {entity}."),
            ConfidenceClass::Correlated => "This changed around the same time.".to_owned(),
        }
    }
}

pub fn relative_time(event: &SystemEvent, symptom_time_ms: i64) -> String {
    relative_time_for_facts(
        event.event.occurred_at_is_uncertain,
        event.event.timestamp,
        symptom_time_ms,
    )
}

fn relative_time_for_facts(
    occurred_at_is_uncertain: bool,
    event_timestamp_ms: i64,
    symptom_time_ms: i64,
) -> String {
    if occurred_at_is_uncertain {
        return "Detected when What Changed started; the occurrence time is unknown.".to_owned();
    }
    let delta = symptom_time_ms.saturating_sub(event_timestamp_ms);
    if delta < 0 {
        return "Happened after you noticed the problem.".to_owned();
    }
    if delta < 60_000 {
        format!(
            "Happened {} seconds before you noticed the problem.",
            delta / 1_000
        )
    } else {
        format!(
            "Happened {} minutes before you noticed the problem.",
            delta / 60_000
        )
    }
}

pub fn why_ranked(
    event: &SystemEvent,
    score: &ScoreBreakdown,
    exact_pair_gap_ms: Option<i64>,
) -> Vec<String> {
    why_ranked_for_facts(
        event.event.occurred_at_is_uncertain,
        score,
        exact_pair_gap_ms,
    )
}

fn why_ranked_for_facts(
    occurred_at_is_uncertain: bool,
    score: &ScoreBreakdown,
    exact_pair_gap_ms: Option<i64>,
) -> Vec<String> {
    let mut why = Vec::new();
    if occurred_at_is_uncertain {
        why.push("Detected after downtime; timing did not increase its rank".to_owned());
    } else if score.temporal > 0.0 {
        why.push("Happened before the problem".to_owned());
    }
    if score.relationship == 1.0 {
        why.push("Touches the thing you named through recorded evidence".to_owned());
    } else if score.relationship > 0.0 {
        why.push("Two recorded relationship steps from the thing you named".to_owned());
    }
    if score.category > 0.0 {
        why.push("Matches the kind of problem you described".to_owned());
    }
    if score.rarity >= 0.95 {
        why.push("First or unusually rare change in 90 days".to_owned());
    }
    if score.failure_proximity > 0.0 {
        why.push("A related failure signal followed this change".to_owned());
    }
    if score.chain_bonus > 0.0 {
        why.push("Starts a sequence of recorded structural changes".to_owned());
    }
    if let Some(300) = exact_pair_gap_ms {
        why.push("These events occurred 300ms apart".to_owned());
    }
    why
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        relative_time, why_ranked, DeterministicExplanationProvider, Explanation, ExplanationCopy,
        ExplanationFacts, ExplanationProvider,
    };
    use crate::analysis::{analyze, analyze_with_provider};
    use crate::domain::{
        AnalysisInput, Category, Confidence, ConfidenceClass, Coverage, Entity, EntityType,
        EventType, Importance, InvestigationWindow, NormalizedEvent, ResolvedInvestigationWindow,
        ScoreBreakdown, Severity, SystemEvent,
    };

    #[test]
    fn deterministic_provider_matches_the_existing_copy_for_every_class() {
        let provider = DeterministicExplanationProvider;
        assert_eq!(provider.id(), "deterministic");
        let score = ScoreBreakdown {
            temporal: 0.5,
            relationship: 1.0,
            category: 0.25,
            importance: 0.6,
            rarity: 1.0,
            failure_proximity: 1.0,
            chain_bonus: 0.75,
            total: 5.1,
        };
        let mut event = fixture_event();
        for class in [
            ConfidenceClass::Confirmed,
            ConfidenceClass::Likely,
            ConfidenceClass::Correlated,
        ] {
            for uncertain in [false, true] {
                event.event.occurred_at_is_uncertain = uncertain;
                let actual = provider.explain(ExplanationFacts {
                    class,
                    entity_display_name: "API service",
                    occurred_at_is_uncertain: uncertain,
                    event_timestamp_ms: event.event.timestamp,
                    symptom_time_ms: 10_000_000,
                    score: &score,
                    exact_pair_gap_ms: Some(300),
                });
                assert_eq!(
                    actual,
                    ExplanationCopy {
                        confidence_text: Explanation::for_class(class, "API service"),
                        relative_time: relative_time(&event, 10_000_000),
                        why_ranked: why_ranked(&event, &score, Some(300)),
                    }
                );
            }
        }
    }

    struct MarkerProvider;

    impl ExplanationProvider for MarkerProvider {
        fn id(&self) -> &'static str {
            "marker-test"
        }

        fn explain(&self, facts: ExplanationFacts<'_>) -> ExplanationCopy {
            let mut copy = DeterministicExplanationProvider.explain(facts);
            copy.relative_time.push_str(" [provider]");
            copy
        }
    }

    #[test]
    fn injected_provider_changes_only_explanation_copy() {
        let request = AnalysisInput {
            problem: "my app cannot connect to api".to_owned(),
            window: InvestigationWindow::Today,
            resolved_window: ResolvedInvestigationWindow {
                from_ms: 0,
                to_ms: 10_000_000,
            },
            symptom_time_ms: 10_000_000,
            created_at: 10_000_001,
            events: vec![fixture_event()],
            boundary_events: vec![],
            entities: vec![Entity {
                id: "service:api".to_owned(),
                entity_type: EntityType::Service,
                display_name: "API service".to_owned(),
                first_seen: 0,
                last_seen: 10_000_000,
                metadata: None,
            }],
            relationships: vec![],
            prior_occurrences_90d: BTreeMap::new(),
            total_events_in_window: 0,
            dropped_events: 0,
            coverage: Coverage::default(),
        };
        let mut expected = analyze(request.clone());
        assert!(!expected.suspects.is_empty(), "fixture must be reportable");
        for suspect in &mut expected.suspects {
            suspect.relative_time.push_str(" [provider]");
        }
        assert_eq!(analyze_with_provider(request, &MarkerProvider), expected);
    }

    fn fixture_event() -> SystemEvent {
        SystemEvent {
            id: 90,
            event: NormalizedEvent {
                timestamp: 9_900_000,
                category: Category::Service,
                event_type: EventType::Failed,
                entity_type: EntityType::Service,
                entity_id: "service:api".to_owned(),
                entity_display_name: "API service".to_owned(),
                severity: Severity::Warning,
                importance: Importance::Important,
                source_collector: "fixture".to_owned(),
                before: None,
                after: None,
                field: None,
                metadata: None,
                related_entity_ids: vec![],
                user_friendly_summary: "API service failed".to_owned(),
                technical_summary: "service:api failed".to_owned(),
                confidence: Confidence::High,
                sensitive: false,
                occurred_at_is_uncertain: false,
            },
        }
    }
}
