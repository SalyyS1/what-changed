//! Deterministic, OS-free investigation engine.

pub mod chain;
pub mod change_map;
pub mod confidence;
pub mod downsample;
pub mod evidence;
pub mod explain;
pub mod graph;
pub mod remediation;
pub mod score;
pub mod symptom;
pub mod system_map;
pub mod window;

mod stage_timings;

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{AnalysisInput, Investigation, NoCauseReason, Suspect};
use stage_timings::{Stage, StageTimings};

const MAX_SUSPECTS: usize = 20;

/// Analyze already-redacted facts. No storage, collector or platform access.
pub fn analyze(input: AnalysisInput) -> Investigation {
    analyze_with_provider(input, &explain::DeterministicExplanationProvider)
}

/// Analyze with an explicitly selected explanation renderer. Ranking and
/// confidence remain deterministic and outside the provider's control.
pub fn analyze_with_provider(
    input: AnalysisInput,
    provider: &dyn explain::ExplanationProvider,
) -> Investigation {
    let mut timings = StageTimings::new();
    let source_count = input.events.len();
    let total_events = if input.total_events_in_window == 0 {
        source_count + input.dropped_events
    } else {
        input.total_events_in_window
    };
    let (mut events, downsampled) = downsample::downsample(input.events, &input.relationships);
    events.sort_by_key(|event| (event.event.timestamp, event.id));
    let dropped = input.dropped_events.saturating_add(downsampled);
    let symptom = symptom::extract(&input.problem, &input.entities, input.symptom_time_ms);

    timings.enter(Stage::StructuralGraph);
    let mut graph_events = input.boundary_events.clone();
    graph_events.extend(events.iter().cloned());
    let graph = graph::StructuralGraph::build(&graph_events, &input.relationships);
    timings.enter(Stage::ChainIndex);
    let chain_index = chain::EventIndex::new(&events);
    timings.enter(Stage::FailureIndex);
    let failure_index = score::FailureIndex::build(&events, &graph);
    timings.enter(Stage::CandidatePreparation);
    let severity_by_id = events
        .iter()
        .map(|event| (event.id, event.event.severity))
        .collect::<BTreeMap<_, _>>();
    let timestamp_by_id = events
        .iter()
        .map(|event| (event.id, event.event.timestamp))
        .collect::<BTreeMap<_, _>>();

    let early_reason = if events.is_empty() {
        Some(
            if input
                .coverage
                .watching_from_ms
                .is_some_and(|from| from > input.resolved_window.to_ms)
            {
                NoCauseReason::NotWatchingYet
            } else {
                NoCauseReason::NothingChanged
            },
        )
    } else if events.iter().all(downsample::is_routine) {
        Some(NoCauseReason::OnlyRoutineChanges)
    } else {
        None
    };

    let mut ranked = Vec::new();
    if early_reason.is_none() {
        let symptom_distances = graph.distances_to_any(&symptom.entity_ids, 2);
        let mut candidates = events
            .iter()
            .filter(|event| {
                event.event.occurred_at_is_uncertain
                    || event.event.timestamp <= input.symptom_time_ms
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|event| (event.event.timestamp, event.id));
        timings.enter(Stage::CandidateScoring);
        for event in candidates {
            // Informational background churn needs an explicit recorded edge
            // before it is reportable inside a mixed window. Rarity and
            // recency alone must not promote the update treadmill.
            if downsample::is_routine(event)
                && !graph.has_recorded_edge(&event.event.entity_id)
                && !score::is_failure_signal(event)
            {
                continue;
            }
            let chain =
                chain::detect_from(event, &events, &input.boundary_events, &graph, &chain_index);
            let chain_len = chain.semantic_len;
            let ctx = score::ScoreContext {
                symptom_time_ms: input.symptom_time_ms,
                symptom_entities: &symptom.entity_ids,
                symptom_categories: &symptom.categories,
                graph: &graph,
                failure_index: &failure_index,
                prior_occurrences: &input.prior_occurrences_90d,
                chain_len,
            };
            let distance = symptom_distances
                .get(event.event.entity_id.as_str())
                .copied();
            let breakdown = score::score_with_relationship_distance(event, &ctx, distance);
            if breakdown.total < score::MIN_REPORTABLE_SCORE {
                continue;
            }
            let has_structural_chain = chain_len >= 3 && breakdown.relationship > 0.0;
            let class =
                confidence::classify(event, &breakdown, has_structural_chain, chain.boundary_head);
            ranked.push((event.id, breakdown, class, chain));
        }
        timings.enter(Stage::SelectionExplanations);
        ranked.sort_by(|left, right| {
            right
                .1
                .total
                .partial_cmp(&left.1.total)
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    severity_by_id
                        .get(&right.0)
                        .cmp(&severity_by_id.get(&left.0))
                })
                .then_with(|| {
                    timestamp_by_id
                        .get(&right.0)
                        .cmp(&timestamp_by_id.get(&left.0))
                })
                .then_with(|| left.0.cmp(&right.0))
        });
        let mut selected = Vec::with_capacity(MAX_SUSPECTS);
        let mut absorbed = BTreeSet::new();
        for candidate in ranked.drain(..) {
            if absorbed.contains(&candidate.0) {
                continue;
            }
            if candidate.3.semantic_len >= 3 {
                absorbed.extend(
                    candidate
                        .3
                        .event_ids
                        .iter()
                        .copied()
                        .filter(|id| *id != candidate.0),
                );
            }
            selected.push(candidate);
            if selected.len() == MAX_SUSPECTS {
                break;
            }
        }
        ranked = selected;
    }

    if early_reason.is_some() {
        timings.enter(Stage::SelectionExplanations);
    }
    let named_entity = symptom.entity_ids.first().and_then(|id| {
        input
            .entities
            .iter()
            .find(|entity| &entity.id == id)
            .map(|entity| entity.display_name.as_str())
    });
    let suspects = ranked
        .into_iter()
        .filter_map(|(event_id, breakdown, class, chain)| {
            let event = events.iter().find(|event| event.id == event_id)?;
            let exact_gap = exact_pair_gap(&chain.event_ids, &graph_events);
            let rendered_chain = if chain.semantic_len >= 3 {
                chain::render_steps(&chain.event_ids, &graph_events)
            } else {
                Vec::new()
            };
            let explanation = provider.explain(explain::ExplanationFacts {
                class,
                entity_display_name: named_entity
                    .unwrap_or(event.event.entity_display_name.as_str()),
                occurred_at_is_uncertain: event.event.occurred_at_is_uncertain,
                event_timestamp_ms: event.event.timestamp,
                symptom_time_ms: input.symptom_time_ms,
                score: &breakdown,
                exact_pair_gap_ms: exact_gap,
            });
            Some(Suspect {
                event_id,
                title: event.event.user_friendly_summary.clone(),
                timestamp_ms: event.event.timestamp,
                score: breakdown.total,
                breakdown,
                confidence: class,
                confidence_text: explanation.confidence_text,
                relative_time: explanation.relative_time,
                why_ranked: explanation.why_ranked,
                chain: rendered_chain,
                what_to_try: remediation::advice(event).to_owned(),
            })
        })
        .collect::<Vec<_>>();

    let reason = early_reason.or_else(|| {
        suspects
            .is_empty()
            .then_some(NoCauseReason::WindowTooNarrow)
    });
    let result = Investigation {
        id: 0,
        project_id: None,
        problem: input.problem,
        window: input.window,
        resolved_window: input.resolved_window,
        suspects,
        no_cause_reason: reason,
        no_cause_message: reason.map(|value| value.message().to_owned()),
        total_events_in_window: total_events,
        analyzed_events: events.len(),
        dropped_events: dropped,
        created_at: input.created_at,
    };
    timings.complete();
    result
}

fn exact_pair_gap(ids: &[i64], events: &[crate::domain::SystemEvent]) -> Option<i64> {
    let mut timestamps = ids
        .iter()
        .filter_map(|id| {
            events
                .iter()
                .find(|event| event.id == *id)
                .map(|event| event.event.timestamp)
        })
        .collect::<Vec<_>>();
    timestamps.sort_unstable();
    timestamps.windows(2).find_map(|pair| {
        let gap = pair[1].saturating_sub(pair[0]);
        (gap == 300).then_some(gap)
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::{
        Category, Confidence, Coverage, Entity, EntityType, EventType, Importance,
        InvestigationWindow, NormalizedEvent, Relationship, RelationshipKind,
        ResolvedInvestigationWindow, Severity, SystemEvent,
    };

    fn event(id: i64, timestamp: i64, entity_id: &str, importance: Importance) -> SystemEvent {
        SystemEvent {
            id,
            event: NormalizedEvent {
                timestamp,
                category: Category::Service,
                event_type: EventType::Stopped,
                entity_type: EntityType::Service,
                entity_id: entity_id.to_owned(),
                entity_display_name: entity_id.to_owned(),
                severity: Severity::Warning,
                importance,
                source_collector: "fixture".to_owned(),
                before: None,
                after: None,
                field: None,
                metadata: None,
                related_entity_ids: vec![],
                user_friendly_summary: format!("{entity_id} stopped"),
                technical_summary: format!("{entity_id} stopped"),
                confidence: Confidence::High,
                sensitive: false,
                occurred_at_is_uncertain: false,
            },
        }
    }

    fn relation(source_id: &str, target_id: &str) -> Relationship {
        Relationship {
            source_id: source_id.to_owned(),
            target_id: target_id.to_owned(),
            kind: RelationshipKind::Provides,
            event_id: None,
            recorded_at: 0,
        }
    }

    #[test]
    fn nonempty_investigation_matches_per_event_bfs_scores_rank_and_confidence() {
        let events = vec![
            event(1, 1_000_000, "service:a", Importance::Critical),
            event(2, 2_000_000, "service:b", Importance::Important),
            event(3, 3_000_000, "service:database", Importance::Notable),
            event(4, 4_000_000, "service:isolated", Importance::Critical),
        ];
        let relationships = vec![
            relation("service:a", "service:b"),
            relation("service:b", "service:database"),
        ];
        let symptom_time_ms = 4_100_000;
        let entities = vec![Entity {
            id: "service:database".to_owned(),
            entity_type: EntityType::Service,
            display_name: "Database service".to_owned(),
            first_seen: 0,
            last_seen: symptom_time_ms,
            metadata: None,
        }];
        let symptom = symptom::extract("database stopped", &entities, symptom_time_ms);
        assert_eq!(symptom.entity_ids, vec!["service:database".to_owned()]);
        let graph = graph::StructuralGraph::build(&events, &relationships);
        let chain_index = chain::EventIndex::new(&events);
        let failure_index = score::FailureIndex::build(&events, &graph);
        let prior_occurrences_90d = [(1, 0), (2, 3), (3, 10), (4, 0)]
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        let mut reference = events
            .iter()
            .filter_map(|event| {
                let chain = chain::detect_from(event, &events, &[], &graph, &chain_index);
                let ctx = score::ScoreContext {
                    symptom_time_ms,
                    symptom_entities: &symptom.entity_ids,
                    symptom_categories: &symptom.categories,
                    graph: &graph,
                    failure_index: &failure_index,
                    prior_occurrences: &prior_occurrences_90d,
                    chain_len: chain.semantic_len,
                };
                let breakdown = score::score(event, &ctx);
                (breakdown.total >= score::MIN_REPORTABLE_SCORE).then(|| {
                    let confidence = confidence::classify(
                        event,
                        &breakdown,
                        chain.semantic_len >= 3 && breakdown.relationship > 0.0,
                        chain.boundary_head,
                    );
                    (
                        event.id,
                        breakdown,
                        confidence,
                        event.event.severity,
                        event.event.timestamp,
                    )
                })
            })
            .collect::<Vec<_>>();
        reference.sort_by(|left, right| {
            right
                .1
                .total
                .partial_cmp(&left.1.total)
                .unwrap_or(Ordering::Equal)
                .then_with(|| right.3.cmp(&left.3))
                .then_with(|| right.4.cmp(&left.4))
                .then_with(|| left.0.cmp(&right.0))
        });
        let reference = reference
            .into_iter()
            .map(|(id, breakdown, confidence, _, _)| (id, breakdown, confidence))
            .collect::<Vec<_>>();
        assert_eq!(reference.len(), 4);

        let investigation = analyze(AnalysisInput {
            problem: "database stopped".to_owned(),
            window: InvestigationWindow::Today,
            resolved_window: ResolvedInvestigationWindow {
                from_ms: 0,
                to_ms: symptom_time_ms,
            },
            symptom_time_ms,
            created_at: symptom_time_ms,
            events,
            boundary_events: vec![],
            entities,
            relationships,
            prior_occurrences_90d,
            total_events_in_window: 4,
            dropped_events: 0,
            coverage: Coverage::default(),
        });
        let actual = investigation
            .suspects
            .iter()
            .map(|suspect| (suspect.event_id, suspect.breakdown, suspect.confidence))
            .collect::<Vec<_>>();
        assert_eq!(actual, reference);
    }
}
