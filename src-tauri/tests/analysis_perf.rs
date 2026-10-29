#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use what_changed_lib::analysis::analyze;
use what_changed_lib::domain::*;
use what_changed_lib::privacy::Sanitized;
use what_changed_lib::services::investigation_service::InvestigationService;
use what_changed_lib::storage::Db;
use what_changed_lib::testing::synthetic::{self, SyntheticProfile};
use what_changed_lib::testing::temp_dir;

fn routine(id: i64) -> SystemEvent {
    SystemEvent {
        id,
        event: NormalizedEvent {
            timestamp: 9_000_000 + id,
            category: Category::Application,
            event_type: EventType::Updated,
            entity_type: EntityType::Application,
            entity_id: format!("app:{id}"),
            entity_display_name: format!("App {id}"),
            severity: Severity::Info,
            importance: Importance::Informational,
            source_collector: "fixture".into(),
            before: None,
            after: None,
            field: None,
            metadata: None,
            related_entity_ids: vec![],
            user_friendly_summary: "Routine update".into(),
            technical_summary: "routine".into(),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: false,
        },
    }
}

fn network_disconnection(id: i64) -> SystemEvent {
    let mut event = routine(id);
    event.event.category = Category::Network;
    event.event.event_type = EventType::Disconnected;
    event.event.entity_type = EntityType::Adapter;
    event.event.entity_id = format!("adapter:fixture-{id}");
    event.event.entity_display_name = format!("Network adapter {id}");
    event.event.severity = Severity::Warning;
    event.event.importance = Importance::Important;
    event.event.user_friendly_summary = "Network adapter disconnected".into();
    event.event.technical_summary = "fixture adapter disconnected".into();
    event
}

fn seed_events_fast(db: &Db, events: Vec<Sanitized<NormalizedEvent>>) {
    db.write(move |conn| {
        let tx = conn.unchecked_transaction().expect("begin fixture seed");
        {
            let mut insert = tx
                .prepare_cached(
                    "INSERT INTO events (timestamp, category, event_type, entity_type, entity_id, \
                     entity_display_name, severity, importance, source_collector, before_json, \
                     after_json, metadata_json, related_entity_ids, user_friendly_summary, \
                     technical_summary, confidence, sensitive, occurred_at_is_uncertain, field) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, \
                     ?15, ?16, ?17, ?18, ?19)",
                )
                .expect("prepare fixture seed");
            for sanitized in events {
                let event = sanitized.get();
                let before = event
                    .before
                    .as_ref()
                    .map(|value| serde_json::to_string(value).expect("encode before"));
                let after = event
                    .after
                    .as_ref()
                    .map(|value| serde_json::to_string(value).expect("encode after"));
                let metadata = event
                    .metadata
                    .as_ref()
                    .map(|value| serde_json::to_string(value).expect("encode metadata"));
                let related = serde_json::to_string(&event.related_entity_ids)
                    .expect("encode related entities");
                insert
                    .execute(rusqlite::params![
                        event.timestamp,
                        event.category.as_str(),
                        event.event_type.as_str(),
                        event.entity_type.as_str(),
                        event.entity_id,
                        event.entity_display_name,
                        event.severity.as_str(),
                        event.importance.as_str(),
                        event.source_collector,
                        before,
                        after,
                        metadata,
                        related,
                        event.user_friendly_summary,
                        event.technical_summary,
                        event.confidence.as_str(),
                        event.sensitive,
                        event.occurred_at_is_uncertain,
                        event.field,
                    ])
                    .expect("insert fixture event");
            }
        }
        tx.execute_batch(
            "INSERT INTO entities (id, entity_type, display_name, first_seen, last_seen, metadata_json) \
             SELECT entity_id, MIN(entity_type), MIN(entity_display_name), MIN(timestamp), \
                    MAX(timestamp), NULL FROM events GROUP BY entity_id; \
             INSERT OR IGNORE INTO entity_relationships \
                    (source_id, target_id, kind, event_id, recorded_at) \
             SELECT source.entity_id, target.entity_id, 'provides', source.id, source.timestamp \
             FROM events source JOIN events target ON target.id = source.id + 1 \
             WHERE source.id % 100 = 0 AND source.entity_id <> target.entity_id LIMIT 1000;",
        )
        .expect("seed representative entities and relationships");
        tx.commit().expect("commit fixture seed");
        Ok(())
    })
    .expect("seed performance corpus");
}

#[test]
#[ignore = "explicit Phase 15 latency gate"]
fn five_thousand_event_analysis_stays_below_400ms_p95() {
    // Keep background churn in the window while ensuring the scoring, ranking,
    // and explanation paths run for more than one reportable candidate.
    let events = (1..=5_000)
        .map(|id| {
            if id % 100 == 0 {
                network_disconnection(id)
            } else {
                routine(id)
            }
        })
        .collect::<Vec<_>>();
    let base = AnalysisInput {
        problem: "why did the network stop working".into(),
        window: InvestigationWindow::Today,
        resolved_window: ResolvedInvestigationWindow {
            from_ms: 0,
            to_ms: 10_000_000,
        },
        symptom_time_ms: 10_000_000,
        created_at: 10_000_001,
        events,
        boundary_events: vec![],
        entities: vec![],
        relationships: vec![],
        prior_occurrences_90d: BTreeMap::new(),
        total_events_in_window: 5_000,
        dropped_events: 0,
        coverage: Coverage::default(),
    };
    let mut samples = Vec::new();
    for _ in 0..20 {
        let input = base.clone();
        let start = Instant::now();
        let result = analyze(input);
        assert_eq!(result.no_cause_reason, None);
        assert_eq!(result.analyzed_events, 5_000);
        assert!(
            result.suspects.len() >= 10,
            "scored candidates were skipped"
        );
        assert_eq!(result.suspects[0].event_id, 5_000);
        assert!(result.suspects.iter().all(|suspect| {
            suspect.event_id % 100 == 0
                && suspect.breakdown.category > 0.0
                && suspect.breakdown.importance > 0.0
                && !suspect.why_ranked.is_empty()
        }));
        samples.push(start.elapsed());
    }
    samples.sort();
    eprintln!("5k analysis p95={:?}; samples={samples:?}", samples[18]);
    assert!(
        samples[18] <= Duration::from_millis(400),
        "p95 was {:?}; samples={samples:?}",
        samples[18]
    );
}

#[test]
#[ignore = "explicit Phase 15 100k-row database latency gate"]
fn hundred_thousand_row_database_investigation_stays_below_1500ms_p95() {
    let db = Arc::new(Db::open_in(&temp_dir("analysis-perf-100k")).expect("open database"));
    let now = what_changed_lib::domain::now_ms();
    let events = synthetic::profile(SyntheticProfile::HundredThousand, 15, now);
    seed_events_fast(&db, events);
    let service = InvestigationService::new(db);
    let window = InvestigationWindow::Custom {
        from_ms: now.saturating_sub(30 * 24 * 60 * 60 * 1_000),
        to_ms: now,
    };

    let mut samples = Vec::with_capacity(20);
    for _ in 0..20 {
        let started = Instant::now();
        let result = service
            .start(
                "why did the network stop working".to_owned(),
                Some(window.clone()),
            )
            .expect("run persisted 100k investigation");
        samples.push(started.elapsed());
        assert_eq!(result.total_events_in_window, 100_000);
        assert_eq!(result.analyzed_events, 5_000);
        assert_eq!(result.dropped_events, 95_000);
        assert!(result.id > 0);
    }
    samples.sort();
    assert!(
        samples[18] <= Duration::from_millis(1_500),
        "100k-row database p95 was {:?}; samples={samples:?}",
        samples[18]
    );
}
