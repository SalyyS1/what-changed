//! The leak suite. A security control, not a nicety: it may never be ignored,
//! and a new secret shape found in the wild becomes a permanent fixture here.
//!
//! Every fixture is replayed through both sinks a value can escape through —
//! the structured path that ends at storage, and the log file — and each
//! artifact is then searched for any eight-character run of the original. Eight
//! rather than the whole value because a partial leak is still a leak.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod fixtures;

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::json;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;

use fixtures::benign_values::BENIGN;
use fixtures::secrets::{Secret, SECRETS};
use what_changed_lib::domain::{
    Category, Confidence, EntityType, EventType, Importance, NormalizedEvent, Severity,
};
use what_changed_lib::observability::RedactingMakeWriter;
use what_changed_lib::privacy::{scrub_line, scrub_value, Sanitizer, REDACTED};
use what_changed_lib::storage::{
    baseline_repository, config_snapshot_repository, snapshot_repository, Baseline, ConfigSnapshot,
    Db, Snapshot,
};
use what_changed_lib::testing::temp_dir;

/// Shortest run of a secret that counts as a leak.
const LEAK_WINDOW: usize = 8;

/// Every `LEAK_WINDOW`-character window of `secret` that appears in `artifact`.
fn leaked_runs(artifact: &str, secret: &str) -> Vec<String> {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() < LEAK_WINDOW {
        panic!("fixture `{secret}` is shorter than the leak window and cannot be asserted on");
    }
    chars
        .windows(LEAK_WINDOW)
        .map(|window| window.iter().collect::<String>())
        .filter(|run| artifact.contains(run.as_str()))
        .collect()
}

fn assert_no_leak(artifact: &str, secret: &str, what: &str, sink: &str) {
    let runs = leaked_runs(artifact, secret);
    assert!(
        runs.is_empty(),
        "{what} leaked through {sink}: {} run(s) of the original survive, first is {:?}",
        runs.len(),
        runs.first()
    );
}

/// A sink a test can read back.
#[derive(Debug, Clone, Default)]
struct Recorder(Arc<Mutex<Vec<u8>>>);

impl Write for Recorder {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Recorder {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

#[test]
fn every_secret_shape_is_redacted_on_the_structured_path() {
    let sanitizer = Sanitizer::new();

    for secret in SECRETS {
        // The shape rules must stand on their own, with no key to help them.
        let by_shape = scrub_value(None, secret.value);
        assert!(
            by_shape.redacted(),
            "{}: no shape rule fired on the value alone",
            secret.what
        );
        let kinds: Vec<&str> = by_shape.kinds.iter().map(|k| k.as_str()).collect();
        assert!(
            kinds.contains(&secret.kind),
            "{}: expected rule {} to fire, got {kinds:?}",
            secret.what,
            secret.kind
        );
        assert_no_leak(
            &by_shape.text,
            secret.must_vanish,
            secret.what,
            "scrub_value with no key",
        );

        // An event carries the value in its before/after pair and in metadata,
        // so all three go through the sanitizer as one document.
        let event = json!({
            "entity": "config file",
            "before": secret.value,
            "after": secret.value,
            "metadata": {
                secret.key: secret.value,
                "path": "C:\\Users\\dev\\project\\.env",
                "changed": true
            }
        });
        let (sanitized, report) = sanitizer.sanitize(event);
        let serialized = serde_json::to_string(sanitized.get()).unwrap();

        assert_no_leak(
            &serialized,
            secret.must_vanish,
            secret.what,
            "the serialized event",
        );
        assert!(
            serialized.contains(REDACTED),
            "{}: the event must say a value was hidden",
            secret.what
        );
        assert!(
            !report.is_empty(),
            "{}: a redaction must be reported so the UI can explain it",
            secret.what
        );
        assert!(
            serialized.contains("\"changed\":true"),
            "{}: the changed flag is computed before redaction and must survive",
            secret.what
        );
        assert!(
            serialized.contains("project\\\\.env"),
            "{}: the path is evidence and must survive",
            secret.what
        );
    }
}

#[test]
fn every_secret_shape_is_redacted_on_the_log_path() {
    // Both formatter shapes the installed stack uses: the file layer writes
    // plain text, the development console writes colour. Colour is not
    // cosmetic here — an ANSI reset ends in a letter and lands directly
    // against the value, so a rule anchored on a word boundary stops firing
    // exactly there. That is how `postgres://user:pass@host` once reached a
    // console while the same value was redacted in the file beside it.
    for ansi in [false, true] {
        let sink = if ansi {
            "the log sink, colour on"
        } else {
            "the log sink"
        };
        for secret in SECRETS {
            let recorder = Recorder::default();
            let factory = RedactingMakeWriter::new(recorder.clone());

            {
                let subscriber = tracing_subscriber::registry().with(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(ansi)
                        .with_writer(factory),
                );
                tracing::subscriber::with_default(subscriber, || {
                    // Interpolated deliberately: the point is that a caller who
                    // logs a raw value cannot defeat the writer.
                    tracing::error!(payload = %secret.value, key = %secret.key, "config read failed");
                    tracing::warn!("{}={}", secret.key, secret.value);
                });
            }

            let written = recorder.text();
            assert!(!written.is_empty(), "{}: nothing was logged", secret.what);
            assert_no_leak(&written, secret.must_vanish, secret.what, sink);
        }
    }
}

#[test]
fn a_secret_split_across_two_writes_still_cannot_reach_the_log() {
    for secret in SECRETS {
        let recorder = Recorder::default();
        let factory = RedactingMakeWriter::new(recorder.clone());
        let mut writer = factory.make_writer();

        let midpoint = secret.value.len() / 2;
        let split = secret
            .value
            .char_indices()
            .map(|(i, _)| i)
            .find(|&i| i >= midpoint)
            .unwrap_or(0);
        writer.write_all(&secret.value.as_bytes()[..split]).unwrap();
        writer.write_all(&secret.value.as_bytes()[split..]).unwrap();
        writer.write_all(b"\n").unwrap();
        drop(writer);

        assert_no_leak(
            &recorder.text(),
            secret.must_vanish,
            secret.what,
            "a split write",
        );
    }
}

#[test]
fn the_installed_logging_stack_scrubs_the_file_on_disk() {
    // The test above builds its own subscriber, which proves the writer scrubs.
    // This one proves the wiring: whatever `init` assembles, every sink it
    // installs has to be wrapped, and the artifact a user could open and paste
    // into a bug report is the file itself.
    //
    // It installs the global subscriber, so it is the only test here that may.
    let directory = std::env::temp_dir().join(format!(
        "what-changed-log-leak-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default()
    ));
    let control = what_changed_lib::observability::init(&directory)
        .expect("the logging stack must install into a fresh directory");

    for secret in SECRETS {
        tracing::error!(payload = %secret.value, key = %secret.key, "config read failed");
    }
    // Dropping the control drops the non-blocking writer's guard, which blocks
    // until the worker has written everything.
    drop(control);

    let mut files = 0;
    let mut written = String::new();
    for entry in std::fs::read_dir(&directory).expect("the log directory must exist") {
        let path = entry.expect("a directory entry").path();
        written
            .push_str(&std::fs::read_to_string(&path).expect("a log file must be readable text"));
        files += 1;
    }
    // One file per day, so normally one — two only if the run straddled
    // midnight, which is a reason to read both rather than to fail.
    assert!(files >= 1, "no log file was written");

    for secret in SECRETS {
        assert_no_leak(
            &written,
            secret.must_vanish,
            secret.what,
            "the rolling log file on disk",
        );
    }
    assert!(
        written.contains(REDACTED),
        "the file must show that values were hidden"
    );

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn the_host_of_a_connection_uri_survives_redaction() {
    // The credential goes, the host stays: an event that cannot say *which*
    // database changed is not evidence of anything.
    for secret in SECRETS.iter().filter(|s| s.kind == "uri_userinfo") {
        let scrubbed = scrub_value(None, secret.value);
        assert!(
            scrubbed.text.contains("mongodb.net") || scrubbed.text.contains("db.internal.example"),
            "{}: the host was lost: {}",
            secret.what,
            scrubbed.text
        );
    }
}

#[test]
fn every_benign_value_survives_both_paths_byte_identical() {
    for benign in BENIGN {
        let by_value = scrub_value(None, benign.value);
        assert_eq!(
            by_value.text, benign.value,
            "{} was altered by the value path: {:?}",
            benign.what, by_value.kinds
        );
        assert!(
            !by_value.redacted(),
            "{} triggered a rule it must not",
            benign.what
        );

        let by_line = scrub_line(benign.value);
        assert_eq!(
            by_line, benign.value,
            "{} was altered by the line path",
            benign.what
        );
    }
}

#[test]
fn the_corpora_are_the_documented_size() {
    // The counts are part of the promise in PRIVACY.md, and a fixture silently
    // dropped is a rule silently untested.
    assert_eq!(SECRETS.len(), 12, "the leak suite is append-only");
    assert_eq!(BENIGN.len(), 40, "the false-positive corpus is append-only");
}

#[test]
fn redaction_stays_within_its_time_budget() {
    // A kilobyte of realistic config with one secret in it. The recorded
    // release figure lives in docs/PERFORMANCE.md; this ceiling is loose enough
    // to pass in a debug build, where the regex engine is unoptimised, and
    // tight enough to catch a rule that turns quadratic.
    const CEILING_PER_KB: std::time::Duration = std::time::Duration::from_micros(2000);
    const ROUNDS: u32 = 200;

    let mut payload = String::new();
    while payload.len() < 1024 {
        payload.push_str("PATH=C:\\Program Files\\nodejs;API_PORT=8080;BUILD=10.0.26200.1234\n");
    }
    payload.push_str("DB_PASSWORD=hunter2\n");

    let start = Instant::now();
    for _ in 0..ROUNDS {
        let _ = scrub_line(&payload);
    }
    // The payload is not exactly a kilobyte, so the budget figure is scaled to
    // one rather than quoted as though it were.
    let per_round = start.elapsed() / ROUNDS;
    let per_kb = per_round.mul_f64(1024.0 / payload.len() as f64);

    println!(
        "redaction: {per_round:?} per {} bytes = {per_kb:?} per KiB",
        payload.len()
    );
    assert!(per_kb < CEILING_PER_KB, "redaction took {per_kb:?} per KiB");
}

/// A value stored in the clear on purpose, so the raw-file assertion proves it
/// is reading the bytes the repository wrote and not an empty file.
const CANARY: &str = "canary-value-persisted-in-the-clear";

/// A config-change event carrying the secret everywhere a collector could
/// put one: both sides of the diff, the metadata, and the technical line.
fn event_carrying(secret: &Secret, timestamp: i64) -> NormalizedEvent {
    NormalizedEvent {
        timestamp,
        category: Category::Configuration,
        event_type: EventType::ConfigurationChanged,
        entity_type: EntityType::File,
        entity_id: "file:0123456789abcdef".to_owned(),
        entity_display_name: ".env".to_owned(),
        severity: Severity::Info,
        importance: Importance::Notable,
        source_collector: "files".to_owned(),
        before: Some(json!({ secret.key: "previous", "note": secret.value })),
        after: Some(json!({ secret.key: secret.value })),
        field: Some(secret.key.to_owned()),
        metadata: Some(json!({
            "env": { secret.key: secret.value },
            "path": r"C:\Users\dev\project\.env",
            "changed": true
        })),
        related_entity_ids: vec!["project:fedcba9876543210".to_owned()],
        user_friendly_summary: format!("{} changed in .env", secret.key),
        technical_summary: format!("{}={}", secret.key, secret.value),
        confidence: Confidence::High,
        sensitive: false,
        occurred_at_is_uncertain: false,
    }
}

#[test]
fn no_secret_survives_into_the_raw_database_file() {
    // The structured-path test proves the sanitizer's output is clean. This
    // one proves the *file* is: every fixture goes through the real
    // repositories — as an event, a detailed snapshot, a collector baseline
    // and a config baseline — the database is closed, and the bytes on disk
    // are searched directly. SQLite stores TEXT verbatim, so a value that
    // reached a column is findable here.
    let dir = temp_dir("leak-db");
    let sanitizer = Sanitizer::new();
    let db = Db::open_in(&dir).unwrap();

    let mut events: Vec<_> = SECRETS
        .iter()
        .enumerate()
        .map(|(i, secret)| {
            sanitizer
                .sanitize(event_carrying(secret, 1_000 + i as i64))
                .0
        })
        .collect();
    let mut canary = event_carrying(&SECRETS[0], 1);
    canary.technical_summary = CANARY.to_owned();
    canary.before = None;
    canary.after = None;
    canary.metadata = None;
    events.push(sanitizer.sanitize(canary).0);
    db.insert_events(events).unwrap();

    for (i, secret) in SECRETS.iter().enumerate() {
        let (snapshot, _) = sanitizer.sanitize(Snapshot {
            collector_id: "files".to_owned(),
            entity_kind: "config".to_owned(),
            entity_external_id: None,
            entity_name: Some(".env".to_owned()),
            state: json!({ secret.key: secret.value, "canary": CANARY }),
            captured_at: 1,
            checkpoint_id: None,
        });
        db.write(move |conn| snapshot_repository::insert(conn, &snapshot))
            .unwrap();

        let (baseline, _) = sanitizer.sanitize(Baseline {
            collector_id: format!("collector-{i}"),
            entity_kind: "config".to_owned(),
            snapshot: json!({ ".env": { secret.key: secret.value } }),
            captured_at: 1,
            truncated: false,
        });
        db.write(move |conn| baseline_repository::store(conn, &baseline))
            .unwrap();

        let (config, _) = sanitizer.sanitize(ConfigSnapshot {
            path: format!(r"C:\Users\dev\project-{i}\.env"),
            keys: json!({ secret.key: secret.value, "PORT": "3000" }),
            captured_at: 1,
            format: "env".to_owned(),
        });
        db.write(move |conn| config_snapshot_repository::put(conn, &config))
            .unwrap();
    }
    db.flush().unwrap();
    db.close();

    let paths = db.paths();
    let mut bytes = std::fs::read(&paths.database).unwrap();
    for companion in [paths.wal(), paths.shm()] {
        if let Ok(more) = std::fs::read(&companion) {
            bytes.extend(more);
        }
    }
    let artifact = String::from_utf8_lossy(&bytes);

    assert!(
        artifact.contains(CANARY),
        "the canary is missing, so these are not the bytes the repository wrote"
    );
    assert!(
        artifact.contains(REDACTED),
        "nothing in the file says a value was hidden"
    );
    for secret in SECRETS {
        assert_no_leak(
            &artifact,
            secret.must_vanish,
            secret.what,
            "the raw database file",
        );
    }
}
