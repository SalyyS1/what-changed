#![allow(clippy::unwrap_used, clippy::panic)]

use what_changed_lib::collectors::eventlog::{
    parse_event_xml, EventLogCollector, EventLogSource, RenderedEvent,
};
use what_changed_lib::collectors::normalize::Normalizer;
use what_changed_lib::collectors::registry::CollectorRegistry;
use what_changed_lib::collectors::testing::live_context;
use what_changed_lib::collectors::{CancelToken, Collector, ObservationClock, WakeHandle};
use what_changed_lib::domain::{Category, EventType, TimeRange};
use what_changed_lib::error::AppResult;
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::services::query_service;
use what_changed_lib::storage::Db;
use what_changed_lib::testing::{synthetic_event, temp_dir};

const BOOT_XML: &str = include_str!("fixtures/eventlog/04-eventlog-6005.xml");

struct OneEvent(Option<RenderedEvent>);

impl EventLogSource for OneEvent {
    fn start(&mut self, _wake: WakeHandle, _cancel: CancelToken) -> AppResult<()> {
        Ok(())
    }

    fn drain(&mut self) -> AppResult<Vec<RenderedEvent>> {
        Ok(self.0.take().into_iter().collect())
    }

    fn stop(&mut self) {}
}

#[derive(Clone, Copy)]
struct FixedClock(i64);

impl ObservationClock for FixedClock {
    fn now_ms(&self) -> i64 {
        self.0
    }
}

#[test]
fn persisted_boot_event_defines_the_since_last_restart_boundary() {
    let db = Db::open_in(&temp_dir("since-last-restart")).unwrap();
    let sanitizer = Sanitizer::new();
    let mut before = synthetic_event(1, 1_000);
    before.category = Category::System;
    let mut boot = synthetic_event(2, 2_000);
    boot.category = Category::System;
    boot.event_type = EventType::Started;
    boot.entity_id = what_changed_lib::domain::ids::system();
    boot.source_collector = "eventlog".to_owned();
    let mut after = synthetic_event(3, 3_000);
    after.category = Category::Network;
    db.insert_events(
        [before, boot, after]
            .into_iter()
            .map(|event| sanitizer.sanitize(event).0)
            .collect(),
    )
    .unwrap();

    assert_eq!(
        query_service::resolve_range_for_db(&db, &TimeRange::SinceLastRestart, 4_000).unwrap(),
        (2_000, 4_000)
    );
}

#[test]
fn missing_boot_evidence_keeps_the_existing_safe_fallback() {
    let db = Db::open_in(&temp_dir("since-last-restart-fallback")).unwrap();
    let now = 1_700_000_000_000;
    assert_eq!(
        query_service::resolve_range_for_db(&db, &TimeRange::SinceLastRestart, now).unwrap(),
        query_service::resolve_range(&TimeRange::SinceLastRestart, now).unwrap()
    );
}

#[test]
fn a_restart_request_does_not_hide_the_current_boot_boundary() {
    let db = Db::open_in(&temp_dir("since-last-restart-stale-boot")).unwrap();
    let sanitizer = Sanitizer::new();
    let mut boot = synthetic_event(10, 1_000);
    boot.category = Category::System;
    boot.event_type = EventType::Started;
    boot.entity_id = what_changed_lib::domain::ids::system();
    boot.source_collector = "eventlog".to_owned();
    let mut shutdown = synthetic_event(11, 2_000);
    shutdown.category = Category::System;
    shutdown.event_type = EventType::Restarted;
    shutdown.entity_id = what_changed_lib::domain::ids::system();
    shutdown.source_collector = "eventlog".to_owned();
    db.insert_events(vec![
        sanitizer.sanitize(boot).0,
        sanitizer.sanitize(shutdown).0,
    ])
    .unwrap();
    assert_eq!(
        query_service::resolve_range_for_db(&db, &TimeRange::SinceLastRestart, 3_000).unwrap(),
        (1_000, 3_000)
    );
}

#[test]
fn rendered_6005_fixture_persists_the_exact_post_boot_boundary() {
    let parsed = parse_event_xml("System", BOOT_XML).unwrap();
    let boot_at = parsed.timestamp_ms.unwrap();
    let now = boot_at + 60_000;
    let descriptor = CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == "eventlog")
        .unwrap()
        .descriptor
        .clone();
    let mut collector = EventLogCollector::with_clock(
        OneEvent(Some(RenderedEvent {
            channel: "System".to_owned(),
            xml: BOOT_XML.to_owned(),
        })),
        descriptor,
        FixedClock(now),
    );
    let live = live_context("since-restart-6005").unwrap();
    collector.start(live.context).unwrap();
    let boot = Normalizer::new().normalize_all(collector.snapshot().unwrap());
    live.db.insert_events(boot.clone()).unwrap();
    live.db.insert_events(boot).unwrap();

    let (rows, count): (i64, i64) = live
        .db
        .read(|connection| {
            connection
                .query_row(
                    "SELECT COUNT(*), MAX(CAST(json_extract(metadata_json, '$.count') AS INTEGER)) \
                     FROM events WHERE source_collector = 'eventlog'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|source| what_changed_lib::error::DbError::Sqlite {
                    operation: "measure idempotent boot replay",
                    source,
                })
                .map_err(Into::into)
        })
        .unwrap();
    assert_eq!((rows, count), (1, 1));

    assert_eq!(
        query_service::resolve_range_for_db(&live.db, &TimeRange::SinceLastRestart, now).unwrap(),
        (boot_at, now)
    );
}
