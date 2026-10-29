#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use what_changed_lib::analysis::evidence::{find_log_evidence, EvidenceWindow};
use what_changed_lib::collectors::eventlog::{EventLogCollector, EventLogSource, RenderedEvent};
use what_changed_lib::collectors::normalize::Normalizer;
use what_changed_lib::collectors::registry::CollectorRegistry;
use what_changed_lib::collectors::testing::live_context;
use what_changed_lib::collectors::{CancelToken, Collector, ObservationClock, WakeHandle};
use what_changed_lib::error::AppResult;

const ERROR_XML: &str = include_str!("fixtures/eventlog/01-application-error-1000.xml");

#[derive(Clone, Default)]
struct BurstSource {
    queue: Arc<Mutex<VecDeque<RenderedEvent>>>,
}

impl BurstSource {
    fn fill(&self, count: usize) {
        let mut queue = self
            .queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        queue.extend((0..count).map(|_| RenderedEvent {
            channel: "Application".to_owned(),
            xml: ERROR_XML.to_owned(),
        }));
    }
}

impl EventLogSource for BurstSource {
    fn start(&mut self, _wake: WakeHandle, _cancel: CancelToken) -> AppResult<()> {
        Ok(())
    }

    fn drain(&mut self) -> AppResult<Vec<RenderedEvent>> {
        Ok(self
            .queue
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .drain(..)
            .collect())
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

fn descriptor() -> what_changed_lib::domain::CollectorDescriptor {
    CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == "eventlog")
        .unwrap()
        .descriptor
        .clone()
}

#[test]
fn five_hundred_identical_errors_store_one_aggregate_with_the_full_count() {
    let source = BurstSource::default();
    source.fill(500);
    let now = 1_800_000_000_000;
    let mut collector = EventLogCollector::with_clock(source, descriptor(), FixedClock(now));
    let live = live_context("eventlog").unwrap();
    collector.start(live.context.clone()).unwrap();

    let raw = collector.snapshot().unwrap();
    assert!(
        raw.len() <= 20,
        "storm emitted {} candidate rows",
        raw.len()
    );
    let normalized = Normalizer::new().normalize_all(raw);
    live.db.insert_events(normalized).unwrap();

    let (stored, max_count): (i64, i64) = live
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
                    operation: "measure the event-log storm",
                    source,
                })
                .map_err(Into::into)
        })
        .unwrap();
    assert!(stored <= 20);
    assert!(max_count >= 480, "largest aggregate count was {max_count}");

    let evidence = find_log_evidence(
        &live.db,
        "sample-app.exe",
        EvidenceWindow::around(now, 30_000_000_000),
    )
    .unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].provider, "Application Error");
    assert_eq!(evidence[0].event_id, 1000);
    assert_eq!(evidence[0].count, 500);
}

#[test]
fn identical_errors_drained_in_separate_cycles_increment_the_first_row() {
    let source = BurstSource::default();
    source.fill(1);
    let now = 1_800_000_000_000;
    let mut collector =
        EventLogCollector::with_clock(source.clone(), descriptor(), FixedClock(now));
    let live = live_context("eventlog-cross-drain").unwrap();
    collector.start(live.context.clone()).unwrap();

    for _ in 0..2 {
        let raw = collector.snapshot().unwrap();
        live.db
            .insert_events(Normalizer::new().normalize_all(raw))
            .unwrap();
        source.fill(1);
    }

    let (stored, count): (i64, i64) = live
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
                    operation: "measure a cross-drain Event Log aggregate",
                    source,
                })
                .map_err(Into::into)
        })
        .unwrap();
    assert_eq!(stored, 1);
    assert_eq!(count, 2);
}
