use super::*;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn values_digest(values: &[Value]) -> String {
    let mut hash = Sha256::new();
    for field in values {
        value(&mut hash, field);
    }
    finish(hash)
}

#[test]
fn framing_distinguishes_types_lengths_and_exact_float_bits() {
    let examples = [
        Value::Null,
        Value::Integer(0),
        Value::Real(0.0),
        Value::Real(-0.0),
        Value::Text(String::new()),
        Value::Blob(Vec::new()),
    ];
    for (index, left) in examples.iter().enumerate() {
        for right in examples.iter().skip(index + 1) {
            assert_ne!(
                values_digest(std::slice::from_ref(left)),
                values_digest(std::slice::from_ref(right))
            );
        }
    }
    assert_ne!(
        values_digest(&[Value::Text("ab".into()), Value::Text("c".into())]),
        values_digest(&[Value::Text("a".into()), Value::Text("bc".into())])
    );
    assert_ne!(
        values_digest(&[Value::Real(f64::from_bits(1))]),
        values_digest(&[Value::Real(f64::from_bits(2))])
    );
}

#[test]
fn ordered_digest_preserves_schema_physical_ids_all_values_and_row_count() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE content(id INTEGER PRIMARY KEY, evidence); INSERT INTO content VALUES(1,'same'),(2,'different');").unwrap();
    let original = table_digest(&conn, "content", "id", None);
    assert_eq!(original.rows, 2);
    assert_eq!(original, table_digest(&conn, "content", "id", None));
    assert_ne!(original, table_digest(&conn, "content", "id DESC", None));
    conn.execute("UPDATE content SET evidence='changed' WHERE id=2", [])
        .unwrap();
    assert_ne!(
        original.sha256,
        table_digest(&conn, "content", "id", None).sha256
    );
    conn.execute("UPDATE content SET evidence='different' WHERE id=2", [])
        .unwrap();
    conn.execute("UPDATE content SET id=3 WHERE id=2", [])
        .unwrap();
    assert_ne!(
        original.sha256,
        table_digest(&conn, "content", "id", None).sha256
    );
    conn.execute(
        "UPDATE content SET id=2,evidence=x'646966666572656e74' WHERE id=3",
        [],
    )
    .unwrap();
    assert_ne!(
        original.sha256,
        table_digest(&conn, "content", "id", None).sha256
    );
    conn.execute("DELETE FROM content WHERE id=2", []).unwrap();
    assert_eq!(table_digest(&conn, "content", "id", None).rows, 1);
    let other = Connection::open_in_memory().unwrap();
    other.execute_batch("CREATE TABLE content(id INTEGER PRIMARY KEY, evidence TEXT); INSERT INTO content VALUES(1,'same');").unwrap();
    assert_ne!(
        table_digest(&conn, "content", "id", None).sha256,
        table_digest(&other, "content", "id", None).sha256
    );
}

#[test]
fn ownership_digest_is_sorted_but_not_delimiter_ambiguous() {
    let left = ["ab", "c"].map(str::to_owned).into_iter().collect();
    let reordered = ["c", "ab"].map(str::to_owned).into_iter().collect();
    let different = ["a", "bc"].map(str::to_owned).into_iter().collect();
    assert_eq!(ownership(&left), ownership(&reordered));
    assert_ne!(ownership(&left), ownership(&different));
}

#[test]
fn normalization_keeps_nested_evidence_and_rejects_unproven_audit_fields() {
    let conn = Connection::open_in_memory().unwrap();
    let columns = ["problem", "created_at", "suspects_json"].map(str::to_owned);
    let presentation = serde_json::json!({"id":0,"problem":"owned","created_at":10,
        "evidence":{"created_at":7,"problem":"owned","id":91}});
    let original = vec![
        Value::Text("owned".into()),
        Value::Integer(10),
        Value::Text(presentation.to_string()),
    ];
    let mut normalized = original.clone();
    normalize(&conn, "investigations", &columns, &mut normalized, "owned");
    let Value::Text(json) = &normalized[2] else {
        panic!("normalized presentation must be text");
    };
    let json: serde_json::Value = serde_json::from_str(json).unwrap();
    assert_eq!(json["evidence"], presentation["evidence"]);
    let mut other_owner = original.clone();
    normalize(&conn, "investigations", &columns, &mut other_owner, "other");
    assert_eq!(other_owner, original);
    let mut meaningful = original.clone();
    normalize(&conn, "events", &columns, &mut meaningful, "owned");
    assert_eq!(meaningful, original);
    let mut bad = original;
    bad[1] = Value::Integer(11);
    assert!(catch_unwind(AssertUnwindSafe(|| normalize(
        &conn,
        "investigations",
        &columns,
        &mut bad,
        "owned"
    )))
    .is_err());
}

#[test]
fn normalization_requires_finished_ordered_session_and_exact_registered_manifest() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE investigations(id INTEGER PRIMARY KEY, problem TEXT, created_at INTEGER); INSERT INTO investigations VALUES(1,'owned',10);").unwrap();
    let columns = ["investigation_id", "started_at", "finished_at"].map(str::to_owned);
    for (started, finished) in [
        (9, Value::Integer(11)),
        (10, Value::Null),
        (11, Value::Integer(10)),
    ] {
        let mut values = vec![Value::Integer(1), Value::Integer(started), finished];
        assert!(catch_unwind(AssertUnwindSafe(|| normalize(
            &conn,
            "analysis_sessions",
            &columns,
            &mut values,
            "owned"
        )))
        .is_err());
    }
    let columns = ["key", "value", "updated_at"].map(str::to_owned);
    for (manifest, updated) in [
        ("[\"other\"]", 10),
        ("[\"owned\",\"other\"]", 10),
        ("[\"owned\"]", 0),
    ] {
        let mut values = vec![
            Value::Text("devtools_seed_history_markers_v1".into()),
            Value::Text(manifest.into()),
            Value::Integer(updated),
        ];
        assert!(catch_unwind(AssertUnwindSafe(|| normalize(
            &conn,
            "settings",
            &columns,
            &mut values,
            "owned"
        )))
        .is_err());
    }
}

#[test]
fn corpus_marker_provenance_requires_one_registered_uuid_run() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL,updated_at INTEGER NOT NULL); INSERT INTO settings VALUES('devtools_seed_history_markers_v1','[]',0);").unwrap();
    for manifest in [
        "[]",
        "[\"unregistered\"]",
        "[\"Synthetic service failed seed 0000000000000000000000000000000z\"]",
        "[\"one\",\"two\"]",
    ] {
        conn.execute("UPDATE settings SET value=?1", [manifest])
            .unwrap();
        assert!(catch_unwind(AssertUnwindSafe(|| registered_marker(&conn))).is_err());
    }
    conn.execute(
        "UPDATE settings SET value=?1",
        ["[\"Synthetic service failed seed 00000000000000000000000000000001\"]"],
    )
    .unwrap();
    let marker = registered_marker(&conn);
    assert_eq!(marker.len(), "Synthetic service failed seed ".len() + 32);
}
