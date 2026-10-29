//! Ordered, typed SHA-256 content evidence; memory scales with one row, not history.

use std::collections::HashSet;

use rusqlite::{types::Value, Connection};
use sha2::{Digest, Sha256};

use super::super::seed_grouped_persisted_parity::{normalize, registered_marker};

pub(super) const TABLES: [&str; 10] = [
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
];

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ContentDigest {
    pub table: &'static str,
    pub rows: u64,
    pub sha256: String,
}

// Each field includes a type/domain byte and byte length. Integer and real
// encodings are fixed-width little-endian; real values retain their exact bits.
fn frame(hash: &mut Sha256, kind: u8, bytes: &[u8]) {
    hash.update([kind]);
    hash.update(u64::try_from(bytes.len()).unwrap().to_le_bytes());
    hash.update(bytes);
}

fn value(hash: &mut Sha256, value: &Value) {
    match value {
        Value::Null => frame(hash, 0, &[]),
        Value::Integer(number) => frame(hash, 1, &number.to_le_bytes()),
        Value::Real(number) => frame(hash, 2, &number.to_bits().to_le_bytes()),
        Value::Text(text) => frame(hash, 3, text.as_bytes()),
        Value::Blob(bytes) => frame(hash, 4, bytes),
    }
}

fn finish(hash: Sha256) -> String {
    format!("{:x}", hash.finalize())
}

pub(super) fn corpus(conn: &Connection) -> Vec<ContentDigest> {
    let marker = registered_marker(conn);
    TABLES
        .into_iter()
        .map(|table| {
            let order = match table {
                "settings" => "key",
                "event_category_counts" => "category",
                _ => "id",
            };
            table_digest(conn, table, order, Some(&marker))
        })
        .collect()
}

fn table_digest(
    conn: &Connection,
    table: &'static str,
    order: &str,
    marker: Option<&str>,
) -> ContentDigest {
    let mut hash = Sha256::new();
    frame(&mut hash, 10, b"seed-corpus-content-v1");
    frame(&mut hash, 11, table.as_bytes());
    frame(&mut hash, 12, order.as_bytes());
    let schema: String = conn
        .query_row(
            "SELECT sql FROM sqlite_schema WHERE type='table' AND name=?1",
            [table],
            |row| row.get(0),
        )
        .unwrap();
    frame(&mut hash, 13, schema.as_bytes());
    // table_info preserves cid order and hashes declarations, nullability,
    // defaults and primary-key ordinals, not just SELECT column labels.
    let mut info = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .unwrap();
    let mut schema_rows = info.query([]).unwrap();
    let mut schema_count = 0_u64;
    while let Some(row) = schema_rows.next().unwrap() {
        frame(&mut hash, 14, &schema_count.to_le_bytes());
        for column in 0..6 {
            value(&mut hash, &row.get::<_, Value>(column).unwrap());
        }
        schema_count += 1;
    }
    frame(&mut hash, 15, &schema_count.to_le_bytes());
    let mut statement = conn
        .prepare(&format!("SELECT * FROM {table} ORDER BY {order}"))
        .unwrap();
    let columns = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    frame(
        &mut hash,
        16,
        &u64::try_from(columns.len()).unwrap().to_le_bytes(),
    );
    for column in &columns {
        frame(&mut hash, 17, column.as_bytes());
    }
    let mut rows = statement.query([]).unwrap();
    let mut row_count = 0_u64;
    while let Some(row) = rows.next().unwrap() {
        let mut values = (0..columns.len())
            .map(|column| row.get::<_, Value>(column).unwrap())
            .collect::<Vec<_>>();
        if let Some(marker) = marker {
            normalize(conn, table, &columns, &mut values, marker);
        }
        frame(&mut hash, 20, &row_count.to_le_bytes());
        for field in &values {
            value(&mut hash, field);
        }
        row_count += 1;
    }
    frame(&mut hash, 21, &row_count.to_le_bytes());
    ContentDigest {
        table,
        rows: row_count,
        sha256: finish(hash),
    }
}

pub(super) fn ownership(owned: &HashSet<String>) -> ContentDigest {
    // At most the fixed 3,000 identities, never event-history-sized storage.
    let mut sorted = owned.iter().collect::<Vec<_>>();
    sorted.sort_unstable();
    let mut hash = Sha256::new();
    frame(&mut hash, 30, b"seed-event-graph-ownership-v1");
    for identity in &sorted {
        frame(&mut hash, 31, identity.as_bytes());
    }
    let rows = u64::try_from(sorted.len()).unwrap();
    frame(&mut hash, 32, &rows.to_le_bytes());
    ContentDigest {
        table: "event_graph_ownership",
        rows,
        sha256: finish(hash),
    }
}

#[path = "seed_grouped_digest_tests.rs"]
mod tests;
