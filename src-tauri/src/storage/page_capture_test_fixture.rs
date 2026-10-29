//! Real migrated in-memory connections; no diagnostic SQL/row substitutes.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use super::{capture_page_plan, PageExplainOutcome, PagePlanVariant};
use crate::domain::{EventQuery, NormalizedEvent};
use crate::privacy::Sanitizer;
use crate::storage::{event_repository, migrations};
use rusqlite::Connection;

pub(crate) fn connection() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrations::run(
        &mut conn,
        migrations::MIGRATIONS,
        std::path::Path::new("unused-page-capture-backups"),
    )
    .unwrap();
    conn
}

pub(crate) fn insert(conn: &Connection, events: Vec<NormalizedEvent>) -> Vec<i64> {
    let sanitizer = Sanitizer::new();
    let events = events
        .into_iter()
        .map(|event| sanitizer.sanitize(event).0)
        .collect::<Vec<_>>();
    event_repository::insert_batch(conn, &events).unwrap()
}

pub(crate) fn assert_clean(conn: &Connection) {
    let query = EventQuery::default();
    let captured = capture_page_plan(PagePlanVariant::for_query(&query), || {
        event_repository::page(conn, &query)
    })
    .unwrap();
    assert!(captured.value.is_ok());
    let evidence = captured.evidence.unwrap();
    assert!(evidence.prepared && evidence.query_started && evidence.fully_decoded);
    assert!(matches!(evidence.explain, PageExplainOutcome::Complete(_)));
}
