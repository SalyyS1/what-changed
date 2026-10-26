//! Streaming complete persisted parity with a field-specific provenance allowlist.

use rusqlite::{types::Value as SqlValue, Connection};
use serde_json::Value;
use what_changed_lib::storage::settings_repository;

const MARKERS_KEY: &str = "devtools_seed_history_markers_v1";
const NORMALIZED_MARKER: &str = "registered synthetic run";

fn index(columns: &[String], name: &str) -> usize {
    columns.iter().position(|column| column == name).unwrap()
}

fn text(value: &SqlValue) -> &str {
    match value {
        SqlValue::Text(text) => text,
        _ => panic!("expected persisted text"),
    }
}

fn integer(value: &SqlValue) -> i64 {
    match value {
        SqlValue::Integer(number) => *number,
        _ => panic!("expected persisted integer"),
    }
}

pub(super) fn registered_marker(conn: &Connection) -> String {
    let markers: Vec<String> = settings_repository::get(conn, MARKERS_KEY)
        .unwrap()
        .unwrap();
    assert_eq!(
        markers.len(),
        1,
        "fixture must have exactly one registered run"
    );
    let marker = &markers[0];
    let nonce = marker
        .strip_prefix("Synthetic service failed seed ")
        .unwrap();
    assert_eq!(nonce.len(), 32);
    assert!(uuid::Uuid::parse_str(nonce).is_ok());
    marker.clone()
}

pub(super) fn normalize(
    conn: &Connection,
    table: &str,
    columns: &[String],
    values: &mut [SqlValue],
    marker: &str,
) {
    match table {
        "checkpoints" => {
            let reason = index(columns, "reason");
            if values[reason] == SqlValue::Text(marker.to_owned()) {
                values[reason] = SqlValue::Text(NORMALIZED_MARKER.to_owned());
            }
        }
        "investigations" => {
            let problem = index(columns, "problem");
            if text(&values[problem]) != marker {
                return;
            }
            let created = index(columns, "created_at");
            let created_at = integer(&values[created]);
            assert!(created_at > 0);
            let suspects = index(columns, "suspects_json");
            let mut presentation: Value = serde_json::from_str(text(&values[suspects])).unwrap();
            let object = presentation.as_object_mut().unwrap();
            assert!(object.get("problem").and_then(Value::as_str) == Some(marker));
            assert_eq!(
                object.get("created_at").and_then(Value::as_i64),
                Some(created_at)
            );
            // The persisted presentation is captured before service assigns its
            // SQLite row id. Do not normalize this or any nested event/chain id.
            assert_eq!(object.get("id").and_then(Value::as_i64), Some(0));
            object.insert(
                "problem".to_owned(),
                Value::String(NORMALIZED_MARKER.to_owned()),
            );
            object.insert("created_at".to_owned(), Value::from(0));
            values[suspects] = SqlValue::Text(serde_json::to_string(&presentation).unwrap());
            values[problem] = SqlValue::Text(NORMALIZED_MARKER.to_owned());
            values[created] = SqlValue::Integer(0);
        }
        "analysis_sessions" => {
            let parent = integer(&values[index(columns, "investigation_id")]);
            let (problem, created): (String, i64) = conn
                .query_row(
                    "SELECT problem, created_at FROM investigations WHERE id=?1",
                    [parent],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            if problem != marker {
                return;
            }
            let started = index(columns, "started_at");
            let finished = index(columns, "finished_at");
            // Require an actually finished session and valid original audit
            // ordering before replacing its two wall-clock values.
            assert!(integer(&values[started]) >= created);
            assert!(integer(&values[finished]) >= integer(&values[started]));
            values[started] = SqlValue::Integer(0);
            values[finished] = SqlValue::Integer(0);
        }
        "settings" => {
            if text(&values[index(columns, "key")]) != MARKERS_KEY {
                return;
            }
            let manifest = index(columns, "value");
            let original: Vec<String> = serde_json::from_str(text(&values[manifest])).unwrap();
            assert_eq!(original.len(), 1);
            assert!(original[0] == marker);
            let updated = index(columns, "updated_at");
            assert!(integer(&values[updated]) > 0);
            values[manifest] = SqlValue::Text(serde_json::to_string(&[NORMALIZED_MARKER]).unwrap());
            values[updated] = SqlValue::Integer(0);
        }
        _ => {}
    }
}

pub(super) fn assert_complete_corpus(left: &Connection, right: &Connection) {
    let left_marker = registered_marker(left);
    let right_marker = registered_marker(right);
    for table in [
        "events",
        "entities",
        "entity_relationships",
        "checkpoints",
        "snapshots",
        "investigations",
        "analysis_sessions",
        "analysis_results",
        "settings",
        "event_category_counts",
    ] {
        compare_table(left, right, table, Some((&left_marker, &right_marker)));
    }
}

pub(super) fn assert_exact_event_graph(left: &Connection, right: &Connection) {
    for table in [
        "events",
        "entities",
        "entity_relationships",
        "event_category_counts",
    ] {
        compare_table(left, right, table, None);
    }
}

fn compare_table(
    left: &Connection,
    right: &Connection,
    table: &str,
    markers: Option<(&str, &str)>,
) {
    let order = match table {
        "settings" => "key",
        "event_category_counts" => "category",
        _ => "id",
    };
    let sql = format!("SELECT * FROM {table} ORDER BY {order}");
    let mut left_statement = left.prepare(&sql).unwrap();
    let mut right_statement = right.prepare(&sql).unwrap();
    let columns = left_statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert!(columns
        .iter()
        .map(String::as_str)
        .eq(right_statement.column_names()));
    let mut left_rows = left_statement.query([]).unwrap();
    let mut right_rows = right_statement.query([]).unwrap();
    loop {
        match (left_rows.next().unwrap(), right_rows.next().unwrap()) {
            (None, None) => break,
            (Some(left_row), Some(right_row)) => {
                let mut left_values = (0..columns.len())
                    .map(|column| left_row.get::<_, SqlValue>(column).unwrap())
                    .collect::<Vec<_>>();
                let mut right_values = (0..columns.len())
                    .map(|column| right_row.get::<_, SqlValue>(column).unwrap())
                    .collect::<Vec<_>>();
                if let Some((left_marker, right_marker)) = markers {
                    normalize(left, table, &columns, &mut left_values, left_marker);
                    normalize(right, table, &columns, &mut right_values, right_marker);
                }
                for (column, (old, new)) in
                    columns.iter().zip(left_values.iter().zip(&right_values))
                {
                    assert!(
                        old == new,
                        "complete persisted parity differs in {table}.{column}"
                    );
                }
            }
            _ => panic!("complete persisted row counts differ in {table}"),
        }
    }
}

pub(super) fn count(conn: &Connection, table: &str) -> usize {
    let count: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap();
    usize::try_from(count).expect("persisted row count must be nonnegative and fit usize")
}

pub(super) fn rows(conn: &Connection, table: &str) -> Vec<Vec<SqlValue>> {
    let mut statement = conn
        .prepare(&format!("SELECT * FROM {table} ORDER BY id"))
        .unwrap();
    let selected = statement
        .query_map([], |row| {
            (0..row.as_ref().column_count())
                .map(|column| row.get(column))
                .collect()
        })
        .unwrap();
    selected.collect::<rusqlite::Result<_>>().unwrap()
}
