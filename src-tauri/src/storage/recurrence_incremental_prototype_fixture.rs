use super::*;
use crate::domain::{Category, EventSignature, EventType};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn signature(entity: &str) -> EventSignature {
    EventSignature {
        entity_id: entity.to_owned(),
        category: Category::Service,
        event_type: EventType::Stopped,
    }
}

pub(super) fn query(id: i64, entity: &str, since: i64, before: i64) -> PriorOccurrenceQuery {
    PriorOccurrenceQuery {
        event_id: id,
        signature: signature(entity),
        since,
        before,
    }
}

pub(super) fn schema(conn: &Connection) {
    conn.execute_batch(
        "CREATE TABLE events(entity_id TEXT NOT NULL, category TEXT NOT NULL, \
         event_type TEXT NOT NULL, timestamp INTEGER NOT NULL); \
         CREATE INDEX idx_recurrence_prototype ON \
         events(entity_id, category, event_type, timestamp);",
    )
    .unwrap();
}

pub(super) fn connection() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    schema(&conn);
    conn
}

pub(super) fn insert(conn: &Connection, signature: &EventSignature, timestamp: i64) {
    conn.execute(
        "INSERT INTO events VALUES (?1, ?2, ?3, ?4)",
        (
            &signature.entity_id,
            signature.category.as_str(),
            signature.event_type.as_str(),
            timestamp,
        ),
    )
    .unwrap();
}

/// Pad with unique signatures so threshold tests exercise the real candidate.
pub(super) fn padded(
    mut queries: Vec<PriorOccurrenceQuery>,
    len: usize,
) -> Vec<PriorOccurrenceQuery> {
    while queries.len() < len {
        let index = queries.len();
        queries.push(query(
            index as i64 + 10_000,
            &format!("padding-{index:05}"),
            0,
            1,
        ));
    }
    queries
}

pub(super) fn assert_parity(conn: &Connection, queries: &[PriorOccurrenceQuery]) -> CandidateRun {
    assert!(!conn.is_autocommit(), "parity requires one shared snapshot");
    let old = original_batch(conn, queries).unwrap();
    let actual = prior_occurrences_batch(conn, queries).unwrap();
    assert_eq!(actual, old, "public production recurrence maps differ");
    let candidate = incremental_batch(conn, queries).unwrap();
    assert_eq!(candidate.counts, actual, "real-helper evidence maps differ");
    candidate
}

pub(super) struct OwnedDirectory(pub PathBuf, PathBuf);

impl OwnedDirectory {
    pub(super) fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_parent = std::env::temp_dir().canonicalize().unwrap();
        let path = temp_parent.join(format!(
            "what-changed-recurrence-prototype-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap(), temp_parent)
    }

    pub(super) fn cleanup(self) {
        self.remove()
            .expect("remove exclusively owned recurrence fixture");
    }

    fn remove(&self) -> std::io::Result<()> {
        let resolved = match self.0.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };
        if resolved != self.0 || resolved.parent() != Some(self.1.as_path()) {
            return Err(std::io::Error::other(
                "owned recurrence fixture target changed",
            ));
        }
        std::fs::remove_dir_all(resolved)
    }
}

impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        if let Err(error) = self.remove() {
            if std::thread::panicking() {
                eprintln!("recurrence fixture cleanup failed during unwind: {error}");
            } else {
                panic!("recurrence fixture cleanup failed: {error}");
            }
        }
    }
}
