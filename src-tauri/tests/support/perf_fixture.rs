//! A performance database whose exact, exclusively-created directory is disposable.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use what_changed_lib::app::{AppPaths, AppState};
use what_changed_lib::domain::{Category, EventQuery, TimeRange};
use what_changed_lib::services::cluster::TimelineItem;
use what_changed_lib::services::query_service;
use what_changed_lib::storage::Db;

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
const CORPUS_END_AT: i64 = 1_800_000_000_000;
const CORPUS_DAYS: i64 = 90;
const CORPUS_SEED: u64 = 0xC0DE_2026;
const DAY_MS: i64 = 86_400_000;
const PAYLOAD_SENSITIVITY_ENV: &str = "WHAT_CHANGED_PERF_COLLECTOR_PAYLOAD_SENSITIVITY";

#[derive(Clone, Copy, PartialEq, Eq)]
enum PayloadProfile {
    Canonical,
    CollectorSensitivity,
}

impl PayloadProfile {
    fn label(self) -> &'static str {
        match self {
            Self::Canonical => "canonical",
            Self::CollectorSensitivity => "collector_payload_sensitivity",
        }
    }
}

#[allow(dead_code)] // The DB-size gate validates the corpus but needs no query selectors.
pub struct CanonicalCorpus {
    pub from: i64,
    pub to: i64,
    pub investigation_from: i64,
    pub details_id: i64,
    pub checkpoint_ids: [i64; 2],
    pub payload_profile: &'static str,
}

pub struct PerfFixture {
    db: Option<Db>,
    state: Option<AppState>,
    root: PathBuf,
}

impl PerfFixture {
    pub fn new() -> Self {
        let mut fixture = Self::empty();
        fixture.db = Some(Db::open_in(&fixture.root).expect("open performance fixture"));
        fixture
    }

    /// Open the same application state used by command bodies that need paths
    /// or checkpoint orchestration. The DB-only constructor stays available
    /// for the independent size/reclaim gate.
    #[allow(dead_code)]
    pub fn new_app_state() -> Self {
        let mut fixture = Self::empty();
        fixture.state = Some(
            AppState::open(AppPaths::under(&fixture.root)).expect("open performance app state"),
        );
        fixture
    }

    /// Seed the canonical streamed corpus before opening any SQLite handles.
    /// The child and this fixture share only the directory created below.
    #[allow(dead_code)] // The query gate opens AppState; the size gate opens Db only.
    pub fn new_seeded_db(rows: usize) -> (Self, CanonicalCorpus) {
        Self::seeded(rows, false, PayloadProfile::Canonical)
    }

    /// Only the independent size gate may opt into collector-shaped payloads.
    /// Query and retention fixtures keep their canonical workload unchanged.
    #[allow(dead_code)]
    pub fn new_seeded_db_for_size_gate(rows: usize) -> (Self, CanonicalCorpus) {
        let profile = match std::env::var(PAYLOAD_SENSITIVITY_ENV) {
            Err(std::env::VarError::NotPresent) => PayloadProfile::Canonical,
            Ok(value) if value == "1" => {
                assert_eq!(
                    rows, 10_000,
                    "collector payload sensitivity is 10k-only until reviewed"
                );
                PayloadProfile::CollectorSensitivity
            }
            _ => panic!("{PAYLOAD_SENSITIVITY_ENV} must be unset or exactly 1"),
        };
        Self::seeded(rows, false, profile)
    }

    /// AppState starts maintenance work, so it must open only after seeding.
    #[allow(dead_code)] // The size gate never starts AppState.
    pub fn new_seeded_app_state(rows: usize) -> (Self, CanonicalCorpus) {
        Self::seeded(rows, true, PayloadProfile::Canonical)
    }

    fn seeded(rows: usize, app_state: bool, profile: PayloadProfile) -> (Self, CanonicalCorpus) {
        assert!(
            matches!(rows, 10_000 | 100_000 | 1_000_000),
            "WHAT_CHANGED_PERF_ROWS must be 10000, 100000, or 1000000"
        );
        let mut fixture = Self::empty();
        let seed_started = Instant::now();
        let mut command = Command::new(env!("CARGO_BIN_EXE_seed_history"));
        command
            .arg("--events")
            .arg(rows.to_string())
            .arg("--days")
            .arg(CORPUS_DAYS.to_string())
            .arg("--seed")
            .arg(CORPUS_SEED.to_string())
            .arg("--relations")
            .arg("--timings")
            .arg("--end-at")
            .arg(CORPUS_END_AT.to_string())
            .arg("--data-dir")
            .arg(&fixture.root);
        if profile == PayloadProfile::CollectorSensitivity {
            command.arg("--collector-payload-sensitivity");
        }
        let output = command
            .output()
            .expect("launch canonical seed_history binary");
        assert!(
            output.status.success(),
            "seed_history failed with {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        // Forward only the seeder's aggregate timing record, never its paths
        // or event output. This diagnoses setup cost without changing corpus.
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if line.starts_with("seed timings: ") {
                println!("{line}");
            }
        }
        println!(
            "phase20 seed events={rows} payload_profile={} elapsed_ms={:.3}",
            profile.label(),
            seed_started.elapsed().as_secs_f64() * 1000.0
        );
        if app_state {
            fixture.state = Some(
                AppState::open(AppPaths::under(&fixture.root))
                    .expect("open seeded performance app state"),
            );
        } else {
            fixture.db =
                Some(Db::open_in(&fixture.root).expect("open seeded performance database"));
        }
        let corpus = fixture.verify_canonical(rows, profile);
        (fixture, corpus)
    }

    fn verify_canonical(&self, rows: usize, profile: PayloadProfile) -> CanonicalCorpus {
        let corpus = self.db()
            .read(|conn| {
                let count = |sql: &str| -> i64 {
                    conn.query_row(sql, [], |row| row.get(0))
                        .expect("read canonical corpus count")
                };
                let expected = i64::try_from(rows).expect("supported corpus size fits i64");
                assert_eq!(count("SELECT COUNT(*) FROM events"), expected);
                assert_eq!(
                    count("SELECT COUNT(*) FROM events WHERE source_collector = 'synthetic'"),
                    expected
                );
                assert_eq!(count("SELECT COUNT(*) FROM entities"), 3_000);
                let related = count(
                    "SELECT COUNT(*) FROM events WHERE json_array_length(related_entity_ids) > 0",
                );
                let bursts = count(
                    "SELECT COUNT(*) FROM events WHERE json_extract(metadata_json, '$.burst') = 1",
                );
                assert_eq!(related, expected * 15 / 100);
                assert_eq!(bursts, expected * 8 / 100);
                assert_eq!(
                    count("SELECT COUNT(DISTINCT category) FROM events"),
                    Category::ALL.len() as i64,
                    "the canonical corpus must exercise every event category"
                );
                if profile == PayloadProfile::CollectorSensitivity {
                    let app = count("SELECT COUNT(*) FROM events WHERE category = 'application'");
                    let process = count("SELECT COUNT(*) FROM events WHERE category = 'process'");
                    let network = count("SELECT COUNT(*) FROM events WHERE category = 'network'");
                    let service = count("SELECT COUNT(*) FROM events WHERE category = 'service'");
                    let system = count("SELECT COUNT(*) FROM events WHERE category = 'system'");
                    assert_eq!(
                        count("SELECT COUNT(*) FROM events WHERE category = 'application' \
                               AND event_type = 'updated' AND field = 'version' \
                               AND json_extract(metadata_json, '$.publisher') = 'Example publisher'"),
                        app
                    );
                    assert_eq!(
                        count("SELECT COUNT(*) FROM events WHERE category = 'process' \
                               AND event_type = 'started' AND before_json IS NULL AND after_json IS NULL \
                               AND json_extract(metadata_json, '$.path_unavailable') = 0"),
                        process
                    );
                    assert_eq!(
                        count("SELECT COUNT(*) FROM events WHERE category = 'network' \
                               AND event_type = 'modified' AND field = 'ip_addresses' \
                               AND json_extract(metadata_json, '$.change_kind') = 'ip_changed'"),
                        network
                    );
                    assert_eq!(
                        count("SELECT COUNT(*) FROM events WHERE category = 'service' \
                               AND event_type = 'stopped' AND field = 'state' \
                               AND json_extract(metadata_json, '$.start_type') = 'Automatic'"),
                        service
                    );
                    assert_eq!(
                        count("SELECT COUNT(*) FROM events WHERE category = 'system' \
                               AND event_type = 'failed' \
                               AND json_extract(metadata_json, '$.event_id') = 41"),
                        system / 10,
                        "only every tenth System event is the rare-event payload sample"
                    );
                }
                let (edges, evidence_events): (i64, i64) = conn
                    .query_row(
                        "SELECT COUNT(*), COUNT(DISTINCT event_id) FROM entity_relationships",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .expect("count durable graph evidence");
                assert_eq!(edges, related, "15% must be persisted graph facts");
                assert_eq!(evidence_events, related, "every graph fact needs unique event evidence");
                assert_eq!(
                    count("SELECT COUNT(*) FROM events e \
                           WHERE json_array_length(e.related_entity_ids) > 0 \
                             AND NOT EXISTS (SELECT 1 FROM entity_relationships r WHERE r.event_id = e.id)"),
                    0,
                    "no related event may lack persisted evidence"
                );
                assert_eq!(
                    count("SELECT COUNT(*) FROM entity_relationships r \
                           LEFT JOIN events e ON e.id = r.event_id \
                           LEFT JOIN entities source ON source.id = r.source_id \
                           LEFT JOIN entities target ON target.id = r.target_id \
                           WHERE e.id IS NULL OR source.id IS NULL OR target.id IS NULL \
                              OR e.source_collector != 'synthetic' \
                              OR r.source_id != e.entity_id OR r.recorded_at != e.timestamp \
                              OR NOT EXISTS (SELECT 1 FROM json_each(e.related_entity_ids) j WHERE j.value = r.target_id) \
                              OR NOT ( \
                                (r.kind = 'listens_on' AND source.entity_type = 'process' AND target.entity_type = 'port') \
                                OR (r.kind = 'publishes' AND source.entity_type = 'container' AND target.entity_type = 'port') \
                                OR (r.kind = 'config_of' AND source.entity_type = 'file' AND target.entity_type = 'service') \
                                OR (r.kind = 'provides' AND source.entity_type = 'app' AND target.entity_type = 'service') \
                              )"),
                    0,
                    "graph edges must have valid direction and event provenance"
                );
                assert_eq!(
                    count("SELECT COUNT(*) FROM ( \
                           SELECT category, entity_id, timestamp / 60000 AS minute \
                           FROM events WHERE json_extract(metadata_json, '$.burst') = 1 \
                           GROUP BY category, entity_id, minute HAVING COUNT(*) < 5)"),
                    0,
                    "every marked burst must contain at least five same-key rows in one minute"
                );
                assert_eq!(count("SELECT COUNT(*) FROM checkpoints"), 40);
                assert_eq!(count("SELECT COUNT(*) FROM snapshots"), 9_800);
                assert_eq!(count("SELECT COUNT(*) FROM investigations"), 20);
                println!(
                    "phase20 corpus events={rows} payload_profile={} entities=3000 related={related} bursts={bursts} checkpoints=40 snapshots=9800 investigations=20 span_days=90",
                    profile.label()
                );

                let (from, to): (i64, i64) = conn
                    .query_row("SELECT MIN(timestamp), MAX(timestamp) FROM events", [], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .expect("read canonical time span");
                assert_eq!(from, CORPUS_END_AT - CORPUS_DAYS * DAY_MS);
                assert_eq!(to, CORPUS_END_AT);
                let investigation_from: i64 = conn
                    .query_row(
                        "SELECT timestamp FROM events ORDER BY timestamp DESC, id DESC LIMIT 1 OFFSET 4999",
                        [],
                        |row| row.get(0),
                    )
                    .expect("read 5k-window boundary");
                assert_eq!(
                    conn.query_row(
                        "SELECT COUNT(*) FROM events WHERE timestamp >= ?1 AND timestamp <= ?2",
                        (investigation_from, to),
                        |row| row.get::<_, i64>(0),
                    )
                    .expect("count 5k-window events"),
                    5_000
                );
                let details_id: i64 = conn
                    .query_row(
                        "SELECT e.id FROM events e WHERE EXISTS (SELECT 1 FROM events earlier \
                         WHERE earlier.entity_id = e.entity_id AND earlier.category = e.category \
                         AND earlier.event_type = e.event_type AND earlier.timestamp < e.timestamp) \
                         ORDER BY e.id DESC LIMIT 1",
                        [],
                        |row| row.get(0),
                    )
                    .expect("find event with prior recurrence");

                let mut statement = conn
                    .prepare("SELECT id, name, item_count FROM checkpoints ORDER BY created_at, id LIMIT 2")
                    .expect("prepare canonical checkpoint pair");
                let pair = statement
                    .query_map([], |row| {
                        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
                    })
                    .expect("read canonical checkpoint pair")
                    .map(|row| row.expect("decode canonical checkpoint"))
                    .collect::<Vec<_>>();
                assert_eq!(pair.len(), 2);
                let mut checkpoint_ids = [0_i64; 2];
                for (index, (id, name, item_count)) in pair.iter().enumerate() {
                    assert_eq!(name, &format!("Synthetic checkpoint {:02}", index + 1));
                    assert_eq!(*item_count, 3_000);
                    assert_eq!(
                        conn.query_row(
                            "SELECT COUNT(*) FROM snapshots WHERE checkpoint_id = ?1",
                            [id],
                            |row| row.get::<_, i64>(0),
                        )
                        .expect("count canonical checkpoint items"),
                        3_000
                    );
                    checkpoint_ids[index] = *id;
                }
                Ok(CanonicalCorpus {
                    from,
                    to,
                    investigation_from,
                    details_id,
                    checkpoint_ids,
                    payload_profile: profile.label(),
                })
            })
            .expect("verify canonical seed_history corpus");

        let (timestamp, category, entity_id): (i64, String, String) = self
            .db()
            .read(|conn| {
                let row = conn
                    .query_row(
                        "SELECT timestamp, category, entity_id FROM events \
                     WHERE json_extract(metadata_json, '$.burst') = 1 \
                     ORDER BY timestamp DESC, id DESC LIMIT 1",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .expect("read a marked burst row");
                Ok(row)
            })
            .expect("find a marked burst for the real timeline path");
        let category: Category = category.parse().expect("seeded category is valid");
        let minute_start = timestamp.div_euclid(60_000) * 60_000;
        let page = query_service::page(
            self.db(),
            EventQuery {
                range: TimeRange::Custom {
                    from: minute_start,
                    to: minute_start + 59_999,
                },
                categories: vec![category],
                entity_id: Some(entity_id),
                limit: 200,
                ..EventQuery::default()
            },
        )
        .expect("query an actual seeded timeline burst");
        assert!(
            page.items
                .iter()
                .any(|item| matches!(item, TimelineItem::Cluster { count, .. } if *count >= 5)),
            "the production timeline path must emit a cluster for a marked burst"
        );
        corpus
    }

    fn empty() -> Self {
        let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("Cargo package has a workspace parent");
        let workspace = std::fs::canonicalize(workspace).expect("resolve fixture workspace");
        let parent = workspace.join(".perf-temp");
        std::fs::create_dir_all(&parent).expect("create workspace-local performance temp parent");
        let parent = std::fs::canonicalize(parent).expect("resolve fixture parent");
        assert!(
            parent.starts_with(&workspace),
            "performance fixture parent must stay inside its workspace"
        );
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("fixture clock is after the epoch")
            .as_nanos();
        let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root = parent.join(format!(
            "what-changed-perf-{}-{nonce}-{serial}",
            std::process::id()
        ));
        // Exclusive creation is the ownership boundary: never adopt an existing
        // directory or sweep other runs' corpora during this test.
        std::fs::create_dir(&root).expect("create owned performance fixture");
        Self {
            db: None,
            state: None,
            root,
        }
    }

    pub fn db(&self) -> &Db {
        if let Some(state) = &self.state {
            state.db.as_ref()
        } else {
            self.db.as_ref().expect("fixture database is open")
        }
    }

    #[allow(dead_code)]
    pub fn state(&self) -> &AppState {
        self.state.as_ref().expect("fixture app state is open")
    }
}

impl Drop for PerfFixture {
    fn drop(&mut self) {
        if let Some(state) = self.state.take() {
            state.shutdown_cleanly();
            let db = Arc::clone(&state.db);
            // The state owns the maintenance thread. Join it before closing
            // SQLite so the exact directory can be removed on Windows.
            drop(state);
            db.close();
            drop(db);
        }
        if let Some(db) = self.db.take() {
            // Windows cannot remove SQLite files while readers or the writer
            // still hold them. Close before deleting, also during test unwind.
            db.close();
            drop(db);
        }
        if let Err(error) = std::fs::remove_dir_all(&self.root) {
            if std::thread::panicking() {
                eprintln!("performance fixture cleanup failed while unwinding: {error}");
            } else {
                panic!("performance fixture cleanup failed: {error}");
            }
        }
    }
}

#[test]
fn dropping_a_fixture_removes_only_its_owned_directory() {
    let fixture = PerfFixture::new();
    let sibling = PerfFixture::new();
    let root = fixture.root.clone();
    assert!(root.exists());
    drop(fixture);
    assert!(!root.exists());
    assert!(
        sibling.root.exists(),
        "another fixture must remain untouched"
    );
    assert!(sibling.db().state().is_ready());
}

#[test]
fn assertion_failure_still_closes_and_removes_the_fixture() {
    let fixture = PerfFixture::new();
    let root = fixture.root.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _fixture = fixture;
        panic!("simulate a failed performance budget assertion");
    }));
    assert!(result.is_err());
    assert!(!root.exists(), "unwinding must not retain the corpus");
}

#[test]
fn app_state_fixture_joins_maintenance_and_removes_its_owned_directory() {
    let fixture = PerfFixture::new_app_state();
    let root = fixture.root.clone();
    assert!(fixture.state().db.state().is_ready());
    drop(fixture);
    assert!(!root.exists(), "app state must release every SQLite handle");
}
