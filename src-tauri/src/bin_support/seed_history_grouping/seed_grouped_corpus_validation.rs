//! Corpus checks outside measurement, not a duplicate query-latency gate.

use rusqlite::Connection;
use what_changed_lib::domain::{Category, EventQuery, TimeRange};
use what_changed_lib::services::{cluster::TimelineItem, query_service};
use what_changed_lib::storage::Db;

use super::super::seed_grouped_persisted_parity::registered_marker;
use crate::DAY_MS;

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}

pub(super) fn validate(conn: &Connection, rows: usize, end_at: i64) {
    let expected = i64::try_from(rows).unwrap();
    assert_eq!(count(conn, "SELECT COUNT(*) FROM events"), expected);
    assert_eq!(
        count(
            conn,
            "SELECT COUNT(*) FROM events WHERE source_collector='synthetic'"
        ),
        expected
    );
    assert_eq!(count(conn, "SELECT COUNT(*) FROM entities"), 3_000);
    assert_eq!(
        count(conn, "SELECT COUNT(DISTINCT category) FROM events"),
        Category::ALL.len() as i64
    );
    assert_eq!(
        count(
            conn,
            "SELECT COUNT(*) FROM events WHERE json_array_length(related_entity_ids)>0"
        ),
        expected * 15 / 100
    );
    assert_eq!(
        count(
            conn,
            "SELECT COUNT(*) FROM events WHERE json_extract(metadata_json,'$.burst')=1"
        ),
        expected * 8 / 100
    );
    assert_eq!(
        count(conn, "SELECT COUNT(*) FROM entity_relationships"),
        expected * 15 / 100
    );
    assert_eq!(
        count(
            conn,
            "SELECT COUNT(DISTINCT event_id) FROM entity_relationships"
        ),
        expected * 15 / 100
    );
    assert_eq!(count(conn,
        "SELECT COUNT(*) FROM events e WHERE json_array_length(e.related_entity_ids)>0 AND NOT EXISTS(SELECT 1 FROM entity_relationships r WHERE r.event_id=e.id)"), 0);
    assert_eq!(count(conn,
        "SELECT COUNT(*) FROM entity_relationships r \
         LEFT JOIN events e ON e.id=r.event_id \
         LEFT JOIN entities source ON source.id=r.source_id \
         LEFT JOIN entities target ON target.id=r.target_id \
         WHERE e.id IS NULL OR source.id IS NULL OR target.id IS NULL \
           OR e.source_collector!='synthetic' OR r.source_id!=e.entity_id \
           OR r.recorded_at!=e.timestamp \
           OR NOT EXISTS(SELECT 1 FROM json_each(e.related_entity_ids) j WHERE j.value=r.target_id) \
           OR NOT ((r.kind='listens_on' AND source.entity_type='process' AND target.entity_type='port') \
             OR (r.kind='publishes' AND source.entity_type='container' AND target.entity_type='port') \
             OR (r.kind='config_of' AND source.entity_type='file' AND target.entity_type='service') \
             OR (r.kind='provides' AND source.entity_type='app' AND target.entity_type='service'))"), 0);
    assert_eq!(count(conn,
        "SELECT COUNT(*) FROM (SELECT category,entity_id,timestamp/60000 FROM events \
         WHERE json_extract(metadata_json,'$.burst')=1 GROUP BY category,entity_id,timestamp/60000 HAVING COUNT(*)<5)"), 0);
    let span: (i64, i64) = conn
        .query_row(
            "SELECT MIN(timestamp),MAX(timestamp) FROM events",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(span, (end_at - 90 * DAY_MS, end_at));
    let window_from: i64 = conn
        .query_row(
            "SELECT timestamp FROM events ORDER BY timestamp DESC,id DESC LIMIT 1 OFFSET 4999",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let window_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE timestamp>=?1 AND timestamp<=?2",
            (window_from, end_at),
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(window_rows, 5_000);
    let recurrent: i64 = conn.query_row("SELECT e.id FROM events e WHERE EXISTS(SELECT 1 FROM events earlier WHERE earlier.entity_id=e.entity_id AND earlier.category=e.category AND earlier.event_type=e.event_type AND earlier.timestamp<e.timestamp) ORDER BY e.id DESC LIMIT 1", [], |row| row.get(0)).unwrap();
    assert!(recurrent > 0);
    let marker = registered_marker(conn);
    assert_eq!(count(conn, "SELECT COUNT(*) FROM checkpoints"), 40);
    assert_eq!(count(conn, "SELECT COUNT(*) FROM snapshots"), 9_800);
    assert_eq!(count(conn, "SELECT COUNT(*) FROM investigations"), 20);
    assert_eq!(count(conn, "SELECT COUNT(*) FROM analysis_sessions"), 20);
    let marked: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM checkpoints WHERE reason=?1",
            [&marker],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(marked, 40);
    let marked: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM investigations WHERE problem=?1",
            [&marker],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(marked, 20);
    assert_eq!(
        count(
            conn,
            "SELECT COUNT(*) FROM checkpoints c WHERE c.id<1 OR c.id>40 OR c.duration_ms!=0 \
         OR c.item_count!=CASE WHEN c.id<=2 THEN 3000 ELSE 100 END \
         OR c.item_count!=(SELECT COUNT(*) FROM snapshots s WHERE s.checkpoint_id=c.id)"
        ),
        0
    );
    for id in 1..=2 {
        let (name, items): (String, i64) = conn
            .query_row(
                "SELECT name,item_count FROM checkpoints WHERE id=?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(name, format!("Synthetic checkpoint {id:02}"));
        assert_eq!(items, 3_000);
    }
    assert_eq!(count(conn,
        "SELECT COUNT(*) FROM snapshots s LEFT JOIN checkpoints c ON c.id=s.checkpoint_id WHERE c.id IS NULL"), 0);
    assert_eq!(count(conn,
        "SELECT COUNT(*) FROM analysis_sessions s LEFT JOIN investigations i ON i.id=s.investigation_id \
         WHERE i.id IS NULL OR s.finished_at IS NULL OR s.started_at<i.created_at OR s.finished_at<s.started_at"), 0);
    assert_eq!(count(conn,
        "SELECT COUNT(*) FROM analysis_results r LEFT JOIN analysis_sessions s ON s.id=r.session_id \
         LEFT JOIN events e ON e.id=r.event_id WHERE s.id IS NULL OR e.id IS NULL"), 0);
    assert_eq!(count(conn,
        "SELECT COUNT(*) FROM event_category_counts c WHERE c.event_count!=(SELECT COUNT(*) FROM events e WHERE e.category=c.category)"), 0);
    assert_eq!(
        count(
            conn,
            "SELECT COALESCE(SUM(event_count),0) FROM event_category_counts"
        ),
        expected
    );
}

pub(super) fn validate_burst(db: &Db) {
    let (timestamp, category, entity_id): (i64, String, String) = db.read(|conn| {
        Ok(conn.query_row("SELECT timestamp,category,entity_id FROM events WHERE json_extract(metadata_json,'$.burst')=1 ORDER BY timestamp DESC,id DESC LIMIT 1", [], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap())
    }).unwrap();
    let minute = timestamp.div_euclid(60_000) * 60_000;
    let page = query_service::page(
        db,
        EventQuery {
            range: TimeRange::Custom {
                from: minute,
                to: minute + 59_999,
            },
            categories: vec![category.parse().unwrap()],
            entity_id: Some(entity_id),
            limit: 200,
            ..EventQuery::default()
        },
    )
    .unwrap();
    assert!(
        page.items
            .iter()
            .any(|item| matches!(item, TimelineItem::Cluster { count, .. } if *count>=5)),
        "the actual canonical timeline must emit a marked burst cluster"
    );
}
