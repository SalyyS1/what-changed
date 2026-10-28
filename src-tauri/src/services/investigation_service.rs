//! Storage orchestration around the pure deterministic analysis engine.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde_json::json;

use crate::analysis;
use crate::domain::{
    AnalysisInput, Category, Coverage, EventSummary, Importance, Investigation,
    InvestigationWindow, ResolvedInvestigationWindow, SystemEvent, TimeRange,
};
use crate::error::{AppError, AppResult};
use crate::privacy::Sanitizer;
use crate::storage::{
    checkpoint_repository, entity_repository, event_repository, investigations, Db, SqlContext,
};

#[path = "investigation_stage_timings.rs"]
mod stage_timings;
use stage_timings::{Stage, StageTimings};

const DAY_MS: i64 = 24 * 60 * 60 * 1_000;
const RELATED_WINDOW_MS: i64 = 60 * 60 * 1_000;

#[derive(Clone, Copy)]
enum AnalysisFilters<'a> {
    Timeline {
        categories: &'a [Category],
        importance_min: Option<Importance>,
        entity_id: Option<&'a str>,
    },
    Workspace(&'a str),
}

#[derive(Clone)]
pub struct InvestigationService {
    db: Arc<Db>,
}

impl InvestigationService {
    pub fn new(db: Arc<Db>) -> Self {
        Self { db }
    }

    pub fn start(
        &self,
        problem: String,
        requested_window: Option<InvestigationWindow>,
    ) -> AppResult<Investigation> {
        self.start_scoped(problem, requested_window, None)
    }

    pub fn start_scoped(
        &self,
        problem: String,
        requested_window: Option<InvestigationWindow>,
        project_id: Option<String>,
    ) -> AppResult<Investigation> {
        let mut timings = StageTimings::new();
        let (problem, _) = Sanitizer::new().sanitize(problem);
        let problem = problem.into_inner();
        let window = requested_window.unwrap_or_else(|| analysis::window::infer(&problem));
        let result = self.run(problem, window, project_id, true, &mut timings);
        timings.complete(result.is_ok());
        result
    }

    pub fn analyze_window(&self, window: InvestigationWindow) -> AppResult<Investigation> {
        let mut timings = StageTimings::new();
        let result = self.run(
            "What changed recently?".to_owned(),
            window,
            None,
            false,
            &mut timings,
        );
        timings.complete(result.is_ok());
        result
    }

    /// Analyze exactly the filtered Timeline corpus before applying the
    /// 5,000-event bound. Hidden categories cannot crowd visible chain
    /// evidence out of the analysis and cannot re-enter through boundary rows.
    pub fn analyze_filtered_window(
        &self,
        window: InvestigationWindow,
        categories: &[Category],
        importance_min: Option<Importance>,
        entity_id: Option<&str>,
    ) -> AppResult<(Investigation, Vec<SystemEvent>)> {
        let mut timings = StageTimings::new();
        let created_at = crate::domain::now_ms();
        let resolved_window = resolve_window(&self.db, &window, created_at)?;
        let filters = AnalysisFilters::Timeline {
            categories,
            importance_min,
            entity_id,
        };
        let (input, source_events) = self.load_input(
            "What changed recently?".to_owned(),
            window,
            resolved_window,
            created_at,
            Some(filters),
            &mut timings,
        )?;
        let result = (analysis::analyze(input), source_events);
        timings.complete(true);
        Ok(result)
    }

    fn run(
        &self,
        problem: String,
        window: InvestigationWindow,
        project_id: Option<String>,
        persist: bool,
        timings: &mut StageTimings,
    ) -> AppResult<Investigation> {
        let created_at = crate::domain::now_ms();
        let resolved_window = resolve_window(&self.db, &window, created_at)?;
        let filters = project_id.as_deref().map(AnalysisFilters::Workspace);
        let (input, _) = self.load_input(
            problem,
            window,
            resolved_window,
            created_at,
            filters,
            timings,
        )?;
        let started_at = crate::domain::now_ms();
        let mut investigation = analysis::analyze(input);
        investigation.project_id = project_id;
        if investigation.project_id.is_some() && investigation.total_events_in_window == 0 {
            investigation.no_cause_reason =
                Some(crate::domain::NoCauseReason::NoRecordedEvidenceInWorkspace);
            investigation.no_cause_message = Some(
                crate::domain::NoCauseReason::NoRecordedEvidenceInWorkspace
                    .message()
                    .to_owned(),
            );
        }
        if !persist {
            return Ok(investigation);
        }

        timings.enter(Stage::ResultSanitization);
        let record = persistence_record(&investigation, started_at)?;
        let (record, _) = Sanitizer::new().sanitize(record);
        timings.enter(Stage::Persistence);
        let stored = self
            .db
            .write(move |conn| investigations::create(conn, &record))?;
        investigation.id = stored.id;
        Ok(investigation)
    }

    fn load_input(
        &self,
        problem: String,
        window: InvestigationWindow,
        resolved_window: ResolvedInvestigationWindow,
        created_at: i64,
        filters: Option<AnalysisFilters<'_>>,
        timings: &mut StageTimings,
    ) -> AppResult<(AnalysisInput, Vec<SystemEvent>)> {
        // Reader acquisition and transaction start belong to window hydration.
        timings.begin_input();
        let result = self.db.read(|conn| {
            let tx = conn
                .unchecked_transaction()
                .during("start a consistent investigation read")?;
            let corpus = match filters {
                Some(AnalysisFilters::Workspace(project_id)) => {
                    event_repository::analysis_window_workspace(
                        &tx,
                        resolved_window.from_ms,
                        resolved_window.to_ms,
                        project_id,
                    )?
                }
                Some(AnalysisFilters::Timeline {
                    categories,
                    importance_min,
                    entity_id,
                }) => event_repository::analysis_window_filtered(
                    &tx,
                    resolved_window.from_ms,
                    resolved_window.to_ms,
                    categories,
                    importance_min,
                    entity_id,
                )?,
                None => event_repository::analysis_window(
                    &tx,
                    resolved_window.from_ms,
                    resolved_window.to_ms,
                )?,
            };
            timings.enter(Stage::BoundaryQueries);
            let boundary = event_repository::window_boundary_context(
                &tx,
                resolved_window.from_ms,
                resolved_window.to_ms,
            )?;
            let boundary_events = if filters.is_some() {
                Vec::new()
            } else {
                event_repository::analysis_boundary_events(&tx, resolved_window.from_ms)?
            };

            timings.enter(Stage::EntityLoad);
            let needles = symptom_needles(&problem);
            let candidates = entity_repository::name_candidates(&tx, &needles, 100)?;
            let mut entities = candidates
                .into_iter()
                .map(|entity| (entity.id.clone(), entity))
                .collect::<BTreeMap<_, _>>();
            for event in corpus.events.iter().chain(boundary_events.iter()) {
                entities
                    .entry(event.event.entity_id.clone())
                    .and_modify(|entity| {
                        entity.first_seen = entity.first_seen.min(event.event.timestamp);
                        entity.last_seen = entity.last_seen.max(event.event.timestamp);
                    })
                    .or_insert_with(|| crate::domain::Entity {
                        id: event.event.entity_id.clone(),
                        entity_type: event.event.entity_type,
                        display_name: event.event.entity_display_name.clone(),
                        first_seen: event.event.timestamp,
                        last_seen: event.event.timestamp,
                        metadata: None,
                    });
            }
            let entities = entities.into_values().collect::<Vec<_>>();
            timings.enter(Stage::Relationships);
            let mut entity_ids: BTreeSet<String> =
                entities.iter().map(|entity| entity.id.clone()).collect();
            for event in corpus.events.iter().chain(boundary_events.iter()) {
                entity_ids.insert(event.event.entity_id.clone());
                entity_ids.extend(event.event.related_entity_ids.iter().cloned());
            }
            let relationships = entity_repository::relationships_for_entities_at(
                &tx,
                &entity_ids.into_iter().collect::<Vec<_>>(),
                resolved_window.to_ms,
            )?;
            timings.enter(Stage::Recurrence);
            let recurrence = corpus
                .events
                .iter()
                .map(|event| event_repository::PriorOccurrenceQuery {
                    event_id: event.id,
                    signature: event.event.signature(),
                    since: event.event.timestamp.saturating_sub(90 * DAY_MS),
                    before: event.event.timestamp,
                })
                .collect::<Vec<_>>();
            let prior_occurrences_90d =
                event_repository::prior_occurrences_batch(&tx, &recurrence)?
                    .into_iter()
                    .map(|(id, count)| (id, u32::try_from(count).unwrap_or(u32::MAX)))
                    .collect::<BTreeMap<_, _>>();
            // Input assembly and read commit are exclusive of query stages.
            timings.enter(Stage::InputFinalize);
            let source_events = if filters.is_some() {
                corpus.events.clone()
            } else {
                Vec::new()
            };
            let input = AnalysisInput {
                problem,
                window,
                resolved_window,
                symptom_time_ms: resolved_window.to_ms,
                created_at,
                events: corpus.events,
                boundary_events,
                entities,
                relationships,
                prior_occurrences_90d,
                total_events_in_window: usize::try_from(corpus.total).unwrap_or(usize::MAX),
                dropped_events: usize::try_from(corpus.downsampled).unwrap_or(usize::MAX),
                coverage: Coverage {
                    watching_from_ms: boundary.coverage_from,
                    complete: boundary
                        .coverage_from
                        .is_none_or(|from| from <= resolved_window.from_ms),
                },
            };
            tx.commit()
                .during("finish a consistent investigation read")?;
            Ok((input, source_events))
        });
        timings.finish_input(result.is_ok());
        result
    }

    pub fn get(&self, id: i64) -> AppResult<Investigation> {
        self.db.read(|conn| {
            let stored =
                investigations::by_id(conn, id)?.ok_or_else(|| not_found("investigation id"))?;
            decode_snapshot(stored.id, stored.investigation.suspects)
        })
    }

    pub fn list(&self, limit: u16) -> AppResult<Vec<Investigation>> {
        self.db.read(|conn| {
            investigations::list(conn, limit)?
                .into_iter()
                .map(|row| decode_snapshot(row.id, row.suspects))
                .collect()
        })
    }

    pub fn delete(&self, id: i64) -> AppResult<()> {
        self.db.write(move |conn| {
            if investigations::delete(conn, id)? {
                Ok(())
            } else {
                Err(not_found("investigation id"))
            }
        })
    }

    pub fn related_events(&self, event_id: i64) -> AppResult<Vec<EventSummary>> {
        self.db.read(|conn| {
            let event =
                event_repository::by_id(conn, event_id)?.ok_or_else(|| not_found("event id"))?;
            let from = event.event.timestamp.saturating_sub(RELATED_WINDOW_MS);
            let to = event.event.timestamp.saturating_add(RELATED_WINDOW_MS);
            let mut corpus = event_repository::analysis_window(conn, from, to)?.events;
            if !corpus.iter().any(|candidate| candidate.id == event_id) {
                corpus.push(event.clone());
                corpus.sort_by_key(|candidate| (candidate.event.timestamp, candidate.id));
            }
            let entity_ids = corpus
                .iter()
                .flat_map(|candidate| {
                    std::iter::once(candidate.event.entity_id.clone())
                        .chain(candidate.event.related_entity_ids.iter().cloned())
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let relationships =
                entity_repository::relationships_for_entities_at(conn, &entity_ids, to)?;
            let ids = analysis::graph::neighbour_ids(&event, &corpus, &relationships)
                .into_iter()
                .take(50)
                .collect::<Vec<_>>();
            event_repository::summaries_by_ids(conn, &ids)
        })
    }
}

fn resolve_window(
    db: &Db,
    window: &InvestigationWindow,
    now: i64,
) -> AppResult<ResolvedInvestigationWindow> {
    let (from_ms, to_ms) = match window {
        InvestigationWindow::LastHour => (now.saturating_sub(60 * 60 * 1_000), now),
        InvestigationWindow::Today => super::query_service::resolve_range(&TimeRange::Today, now)?,
        InvestigationWindow::SinceYesterday => {
            let (from, _) = super::query_service::resolve_range(&TimeRange::Yesterday, now)?;
            (from, now)
        }
        InvestigationWindow::SinceLastRestart => {
            super::query_service::resolve_range_for_db(db, &TimeRange::SinceLastRestart, now)?
        }
        InvestigationWindow::SinceCheckpoint { checkpoint_id } => {
            let checkpoint = db
                .read(|conn| checkpoint_repository::by_id(conn, *checkpoint_id))?
                .ok_or_else(|| not_found("checkpoint id"))?;
            (checkpoint.created_at.min(now), now)
        }
        InvestigationWindow::Custom { from_ms, to_ms } => super::query_service::resolve_range(
            &TimeRange::Custom {
                from: *from_ms,
                to: *to_ms,
            },
            now,
        )?,
    };
    Ok(ResolvedInvestigationWindow { from_ms, to_ms })
}

fn symptom_needles(problem: &str) -> Vec<String> {
    let stopwords = [
        "after", "before", "changed", "from", "have", "into", "just", "that", "this", "what",
        "when", "with", "worked", "working",
    ];
    let mut words = problem
        .to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| word.len() >= 4 && !stopwords.contains(word))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    words.sort();
    words.dedup();
    words
}

fn persistence_record(
    investigation: &Investigation,
    started_at: i64,
) -> AppResult<investigations::InvestigationPersistence> {
    let results = investigation
        .suspects
        .iter()
        .enumerate()
        .map(|(index, suspect)| {
            Ok(investigations::AnalysisResultRecord {
                event_id: Some(suspect.event_id),
                rank: u32::try_from(index + 1).unwrap_or(u32::MAX),
                score: suspect.score,
                score_terms: serde_json::to_value(suspect.breakdown).map_err(json_error)?,
                confidence_class: format!("{:?}", suspect.confidence).to_ascii_lowercase(),
                chain: (!suspect.chain.is_empty())
                    .then(|| serde_json::to_value(&suspect.chain).map_err(json_error))
                    .transpose()?,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    Ok(investigations::InvestigationPersistence {
        problem: investigation.problem.clone(),
        window: serde_json::to_value(&investigation.window).map_err(json_error)?,
        resolved_from: investigation.resolved_window.from_ms,
        resolved_to: investigation.resolved_window.to_ms,
        suspects: serde_json::to_value(investigation).map_err(json_error)?,
        total_events_in_window: u64::try_from(investigation.total_events_in_window)
            .unwrap_or(u64::MAX),
        created_at: investigation.created_at,
        session: investigations::AnalysisSessionRecord {
            started_at,
            finished_at: Some(crate::domain::now_ms()),
            weights: json!({
                "temporal": analysis::score::W_TEMPORAL,
                "relationship": analysis::score::W_RELATIONSHIP,
                "category": analysis::score::W_CATEGORY,
                "importance": analysis::score::W_IMPORTANCE,
                "rarity": analysis::score::W_RARITY,
                "failure_proximity": analysis::score::W_FAILURE_PROXIMITY,
            }),
            events_considered: u64::try_from(investigation.analyzed_events).unwrap_or(u64::MAX),
            downsampled: u64::try_from(investigation.dropped_events).unwrap_or(u64::MAX),
            results,
        },
    })
}

fn decode_snapshot(id: i64, value: serde_json::Value) -> AppResult<Investigation> {
    let mut investigation = serde_json::from_value::<Investigation>(value).map_err(json_error)?;
    investigation.id = id;
    Ok(investigation)
}

fn not_found(what: &'static str) -> AppError {
    AppError::InvalidInput {
        what,
        reason: "was not found".to_owned(),
    }
}

fn json_error(source: serde_json::Error) -> AppError {
    crate::error::DbError::Encoding {
        reason: format!("investigation JSON: {source}"),
    }
    .into()
}
