//! Deterministic synthetic history seeder for performance and UI gates.
//!
//! This binary is deliberately behind Cargo's `devtools` feature.  It writes
//! through the same sanitised writer used by the app, preserving pre-existing
//! entity evidence if a generated id collides with a real one,
//! and labels every event `source_collector = "synthetic"`. Checkpoints and
//! investigations carry an exact registered run marker, so a run can be
//! removed without matching a user's real rows by name:
//!
//! ```text
//! cargo run --bin seed_history --features devtools -- --events 10000 --days 30 --seed 42 --data-dir <fresh-qa-dir>
//! cargo run --bin seed_history --features devtools -- --purge-synthetic --data-dir <seeded-qa-dir>
//! ```
//!
//! The generator is intentionally streamed in bounded batches.  A million
//! rows should exercise SQLite, not require a million-row Rust allocation.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;
use what_changed_lib::app::AppPaths;
use what_changed_lib::checkpoint::prepare_capture;
use what_changed_lib::domain::{
    classify, ids, Category, CheckpointItem, CheckpointKind, ClassifierContext, Entity, EntityType,
    EventType, InvestigationWindow, NormalizedEvent, RelationshipKind, Severity,
};
use what_changed_lib::error::{AppError, AppResult, DbError};
use what_changed_lib::privacy::{Sanitized, Sanitizer};
use what_changed_lib::services::investigation_service::InvestigationService;
use what_changed_lib::storage::event_repository::BatchInsertionTimings;
use what_changed_lib::storage::{checkpoint_repository, maintenance, Db, DbPaths};
use what_changed_lib::testing::synthetic_event;

#[path = "../bin_support/seed_history_grouping/seed_graph_body.rs"]
mod seed_graph_body;

#[cfg(test)]
#[path = "../bin_support/seed_history_grouping/seed_grouped_events.rs"]
mod seed_grouped_events;

const DEFAULT_SEED: u64 = 0xC0DE_2026;
const DEFAULT_DAYS: u64 = 90;
const BATCH_SIZE: usize = 1_000;
const ENTITY_COUNT: u64 = 3_000;
const RELATION_PERCENT: u64 = 15;
const RELATION_TARGET_POOL: u64 = 400;
const DAY_MS: i64 = 86_400_000;
const CHECKPOINT_COUNT: usize = 40;
const INVESTIGATION_COUNT: usize = 20;
const CHECKPOINT_ITEMS: usize = 100;
const LARGE_CHECKPOINT_ITEMS: usize = 3_000;
const QA_LOCK_FILE: &str = ".what-changed-seed.lock";
const QA_SENTINEL_FILE: &str = ".what-changed-seed-owner.json";
const QA_SENTINEL_VERSION: u32 = 1;

/// The alternate transaction granularity exists only in the private proof.
/// Public CLI execution always selects the unchanged standard batch.
#[derive(Clone, Copy)]
enum SeedBatchSize {
    Standard,
    #[cfg(test)]
    Candidate,
}

impl SeedBatchSize {
    fn rows(self) -> usize {
        match self {
            Self::Standard => BATCH_SIZE,
            #[cfg(test)]
            Self::Candidate => 5_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct QaSentinel {
    version: u32,
    canonical_dir: PathBuf,
    nonce: String,
}

/// An exclusive per-directory claim. A crash leaves the file behind so a
/// later command fails closed instead of guessing whether an old writer lives.
struct QaLock {
    file: Option<File>,
    path: PathBuf,
}

impl QaLock {
    fn acquire(root: &Path) -> AppResult<Self> {
        let path = root.join(QA_LOCK_FILE);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| AppError::File {
                operation: "acquire the exclusive seed-history lock",
                path: path.clone(),
                source,
            })?;
        Ok(Self {
            file: Some(file),
            path,
        })
    }
}

impl Drop for QaLock {
    fn drop(&mut self) {
        drop(self.file.take());
        if let Err(error) = fs::remove_file(&self.path) {
            eprintln!("seed-history lock cleanup failed: {error}");
        }
    }
}

#[derive(Debug, Clone)]
struct Options {
    events: usize,
    days: u64,
    seed: u64,
    relations: bool,
    collector_payload_sensitivity: bool,
    timings: bool,
    insertion_timings: bool,
    purge: bool,
    data_dir: Option<PathBuf>,
    end_at: Option<i64>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            events: 10_000,
            days: DEFAULT_DAYS,
            seed: DEFAULT_SEED,
            // The Phase 20 corpus always includes the explicit 15% graph
            // sample unless a caller opts out for a write-only benchmark.
            relations: true,
            collector_payload_sensitivity: false,
            timings: false,
            insertion_timings: false,
            purge: false,
            data_dir: None,
            end_at: None,
        }
    }
}

fn usage() -> &'static str {
    "usage: seed_history [--events 10000|100000|1000000] [--days N] [--seed N] \
     [--relations|--no-relations] [--collector-payload-sensitivity] [--timings] [--insertion-timings] \
     [--end-at UTC_EPOCH_MS] --data-dir FRESH_QA_DIR \
     | --purge-synthetic --data-dir SEEDED_QA_DIR"
}

fn parse_u64(value: Option<&String>, name: &'static str) -> Result<u64, String> {
    value
        .ok_or_else(|| format!("missing value for --{name}"))?
        .parse::<u64>()
        .map_err(|_| format!("--{name} must be an unsigned integer"))
}

fn parse_options(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--events" => {
                let value = parse_u64(args.next().as_ref(), "events")?;
                options.events = usize::try_from(value)
                    .map_err(|_| "--events is too large for this platform".to_owned())?;
            }
            "--days" => options.days = parse_u64(args.next().as_ref(), "days")?,
            "--seed" => options.seed = parse_u64(args.next().as_ref(), "seed")?,
            "--end-at" => {
                options.end_at = Some(
                    args.next()
                        .ok_or_else(|| "missing value for --end-at".to_owned())?
                        .parse::<i64>()
                        .map_err(|_| "--end-at must be signed UTC epoch milliseconds".to_owned())?,
                )
            }
            "--relations" => options.relations = true,
            "--no-relations" => options.relations = false,
            "--collector-payload-sensitivity" => options.collector_payload_sensitivity = true,
            "--timings" => options.timings = true,
            "--insertion-timings" => options.insertion_timings = true,
            "--purge-synthetic" => options.purge = true,
            "--data-dir" => {
                options.data_dir = Some(PathBuf::from(
                    args.next()
                        .ok_or_else(|| "missing value for --data-dir".to_owned())?,
                ))
            }
            "--help" | "-h" => return Err(usage().to_owned()),
            unknown => return Err(format!("unknown argument {unknown}\n{}", usage())),
        }
    }
    if options.data_dir.is_none() {
        return Err(
            "--data-dir is required; the seeder never uses the normal app database".to_owned(),
        );
    }
    if options.purge {
        return Ok(options);
    }
    if options.events == 0 {
        return Err("--events must be greater than zero".to_owned());
    }
    if options.events > 1_000_000 {
        return Err("--events is capped at 1,000,000 for the release gate".to_owned());
    }
    if options.days == 0 {
        return Err("--days must be greater than zero".to_owned());
    }
    Ok(options)
}

fn qa_directory(path: &Path) -> AppResult<PathBuf> {
    let root = fs::canonicalize(path).map_err(|source| AppError::Directory {
        path: path.to_path_buf(),
        source,
    })?;
    let name = root.file_name().and_then(|name| name.to_str());
    if !name.is_some_and(|name| {
        name.starts_with("what-changed-perf-") || name.starts_with("what-changed-seed-")
    }) || !root.is_dir()
    {
        return Err(AppError::InvalidInput {
            what: "--data-dir",
            reason: "expected an existing, reserved what-changed-perf-* or what-changed-seed-* directory"
                .to_owned(),
        });
    }
    if let Ok(normal_data) = fs::canonicalize(AppPaths::resolve().data) {
        if root == normal_data || root.starts_with(&normal_data) {
            return Err(AppError::InvalidInput {
                what: "--data-dir",
                reason: "the normal application data directory cannot be used for QA seeding"
                    .to_owned(),
            });
        }
    }
    Ok(root)
}

fn ensure_empty_seed_directory(root: &Path, lock_is_expected: bool) -> AppResult<()> {
    let entries = fs::read_dir(root).map_err(|source| AppError::Directory {
        path: root.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| AppError::Directory {
            path: root.to_path_buf(),
            source,
        })?;
        if !lock_is_expected || entry.file_name() != QA_LOCK_FILE {
            return Err(AppError::InvalidInput {
                what: "--data-dir",
                reason: "seeding requires an empty QA directory with no existing database"
                    .to_owned(),
            });
        }
    }
    Ok(())
}

fn sentinel_path(root: &Path) -> PathBuf {
    root.join(QA_SENTINEL_FILE)
}

fn write_sentinel(root: &Path) -> AppResult<()> {
    let path = sentinel_path(root);
    let sentinel = QaSentinel {
        version: QA_SENTINEL_VERSION,
        canonical_dir: root.to_path_buf(),
        nonce: uuid::Uuid::new_v4().simple().to_string(),
    };
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| AppError::File {
            operation: "create the seed-history ownership marker",
            path: path.clone(),
            source,
        })?;
    let bytes = serde_json::to_vec(&sentinel).expect("QA sentinel serialization is infallible");
    file.write_all(&bytes).map_err(|source| AppError::File {
        operation: "write the seed-history ownership marker",
        path: path.clone(),
        source,
    })?;
    file.sync_all().map_err(|source| AppError::File {
        operation: "sync the seed-history ownership marker",
        path,
        source,
    })
}

fn read_sentinel(root: &Path) -> AppResult<QaSentinel> {
    let path = sentinel_path(root);
    let bytes = fs::read(&path).map_err(|source| AppError::File {
        operation: "read the seed-history ownership marker",
        path,
        source,
    })?;
    let sentinel: QaSentinel =
        serde_json::from_slice(&bytes).map_err(|_| AppError::InvalidInput {
            what: "seed-history ownership marker",
            reason: "the QA marker is malformed".to_owned(),
        })?;
    if sentinel.version != QA_SENTINEL_VERSION
        || sentinel.canonical_dir != root
        || uuid::Uuid::parse_str(&sentinel.nonce).is_err()
    {
        return Err(AppError::InvalidInput {
            what: "seed-history ownership marker",
            reason: "the QA marker does not belong to this canonical directory".to_owned(),
        });
    }
    Ok(sentinel)
}

fn prepare_seed_directory(root: &Path) -> AppResult<QaLock> {
    ensure_empty_seed_directory(root, false)?;
    let lock = QaLock::acquire(root)?;
    ensure_empty_seed_directory(root, true)?;
    write_sentinel(root)?;
    Ok(lock)
}

fn prepare_purge_directory(root: &Path) -> AppResult<QaLock> {
    let owner = read_sentinel(root)?;
    if !DbPaths::under(root).database.is_file() {
        return Err(AppError::InvalidInput {
            what: "--purge-synthetic",
            reason: "the QA directory has no existing seed-history database".to_owned(),
        });
    }
    let lock = QaLock::acquire(root)?;
    if read_sentinel(root)? != owner || !DbPaths::under(root).database.is_file() {
        return Err(AppError::InvalidInput {
            what: "--purge-synthetic",
            reason: "the QA ownership marker or database changed while acquiring the lock"
                .to_owned(),
        });
    }
    Ok(lock)
}

fn synthetic_entity_id(entity_type: EntityType, slot: u64) -> String {
    let label = format!("synthetic-entity-{slot}");
    match entity_type {
        EntityType::Process => ids::process(&format!(r"C:\synthetic\{label}.exe")),
        EntityType::File => ids::file(&format!(r"C:\synthetic\{label}.json")),
        EntityType::Project => ids::project(&format!(r"C:\synthetic\{label}")),
        EntityType::Port => ids::port(
            ids::Protocol::Tcp,
            "127.0.0.1",
            20_000 + (slot as u16 % 20_000),
        ),
        EntityType::System => ids::system(),
        // Docker ids are short ids; put the varying slot in the first twelve
        // digits, not at the end of a 64-digit string that gets truncated.
        EntityType::Container => ids::container(&format!("{slot:012x}")),
        EntityType::Image => ids::image(&format!("synthetic/image:{slot}")),
        EntityType::DockerVolume => ids::docker_volume(&label),
        EntityType::DockerNetwork => ids::docker_network(&label),
        EntityType::Service => ids::service(&label),
        EntityType::Application => ids::app("synthetic", &label),
        EntityType::Driver => ids::driver(&label),
        EntityType::Adapter => ids::adapter(&label),
        EntityType::Update => ids::update(&format!("SYNTH-{slot}")),
        EntityType::Task => ids::task(&format!(r"\Synthetic\{label}")),
    }
}

fn relation_kind(category: Category) -> Option<RelationshipKind> {
    match category {
        Category::Process => Some(RelationshipKind::ListensOn),
        Category::Docker => Some(RelationshipKind::Publishes),
        Category::Configuration | Category::Filesystem => Some(RelationshipKind::ConfigOf),
        Category::Application => Some(RelationshipKind::Provides),
        _ => None,
    }
}

fn related_entity_id(entity_type: EntityType, slot: u64, ordinal: u64) -> String {
    // Each selected event for a reused source gets a distinct target. The
    // bounded target pool keeps the graph below 3k nodes at every scale.
    let target_slot = (slot % RELATION_TARGET_POOL + ordinal) % RELATION_TARGET_POOL;
    match entity_type {
        // These directions mirror the collector relationship validator:
        // processes and containers point to the port they own.
        EntityType::Process | EntityType::Container => {
            ids::port(ids::Protocol::Tcp, "127.0.0.1", 20_000 + target_slot as u16)
        }
        // A file or application can provide/configure a service.
        EntityType::File | EntityType::Application => {
            ids::service(&format!("synthetic-entity-{target_slot}"))
        }
        _ => ids::service(&format!("synthetic-entity-{target_slot}")),
    }
}

fn category_at(index: usize, total: usize, seed: u64) -> Category {
    let stream_seed = event_seed(index, total, seed);
    Category::ALL[(stream_seed % Category::ALL.len() as u64) as usize]
}

fn relationship_selection(total: usize, seed: u64) -> AppResult<Vec<bool>> {
    let eligible: Vec<usize> = (0..total)
        .filter(|&index| relation_kind(category_at(index, total, seed)).is_some())
        .collect();
    let wanted = total.saturating_mul(RELATION_PERCENT as usize) / 100;
    let mut selected = vec![false; total];
    if eligible.len() < wanted {
        return Err(AppError::InvalidInput {
            what: "synthetic relationship quota",
            reason: "too few valid source categories for 15% persisted relationships".to_owned(),
        });
    }
    if wanted == 0 {
        return Ok(selected);
    }
    // Spread the exact quota through supported collector directions. A
    // `related_entity_ids` row without a persisted edge is not counted as a
    // relationship in this performance corpus.
    let eligible_count = eligible.len() as u128;
    for (rank, index) in eligible.into_iter().enumerate() {
        let left = rank as u128 * wanted as u128 / eligible_count;
        let right = (rank as u128 + 1) * wanted as u128 / eligible_count;
        if right > left {
            selected[index] = true;
        }
    }
    Ok(selected)
}

/// The System fixture is deliberately a bounded rare-event stress sample,
/// not a claim that 10% of a user's real System history is a shutdown.
fn system_payload_selection(total: usize, seed: u64) -> Vec<bool> {
    let mut selected = vec![false; total];
    let mut system_rank = 0_usize;
    for (index, slot) in selected.iter_mut().enumerate() {
        if category_at(index, total, seed) == Category::System {
            system_rank += 1;
            *slot = system_rank % 10 == 0;
        }
    }
    selected
}

/// Spread exactly floor(8% of rows) through five-row-or-larger interior runs.
/// The release sizes divide cleanly into five-row runs; a smaller ad-hoc seed
/// can put a one-to-four-row remainder into its last run.
fn burst_anchor(index: usize, total: usize) -> Option<usize> {
    let quota = total.saturating_mul(8) / 100;
    let groups = quota / 5;
    if groups == 0 {
        return None;
    }
    let gap = total.saturating_sub(quota).saturating_sub(2) / (groups + 1);
    let first = 1 + gap;
    let stride = gap + 5;
    if index < first {
        return None;
    }
    let group = (index - first) / stride;
    let offset = (index - first) % stride;
    if group >= groups || offset >= 5 + usize::from(group + 1 == groups) * (quota % 5) {
        return None;
    }
    Some(first + group * stride)
}

fn event_seed(index: usize, total: usize, seed: u64) -> u64 {
    seed.wrapping_add(burst_anchor(index, total).unwrap_or(index) as u64)
        .rotate_left(17)
}

fn linear_timestamp(index: usize, total: usize, days: u64, end_at: i64) -> i64 {
    let span = i64::try_from(days)
        .unwrap_or(i64::MAX / DAY_MS)
        .saturating_mul(DAY_MS);
    let denominator = i64::try_from(total.saturating_sub(1))
        .unwrap_or(i64::MAX)
        .max(1);
    let start = end_at.saturating_sub(span);
    let index_i64 = i64::try_from(index).unwrap_or(i64::MAX);
    start.saturating_add(span.saturating_mul(index_i64) / denominator)
}

fn event_timestamp(index: usize, total: usize, days: u64, end_at: i64) -> i64 {
    let Some(anchor) = burst_anchor(index, total) else {
        return linear_timestamp(index, total, days, end_at);
    };
    // A single timestamp plus consecutive row IDs makes every marked run
    // contiguous in the newest-first order used by TimelineItem::Cluster.
    let minute = linear_timestamp(anchor, total, days, end_at).div_euclid(60_000);
    minute.saturating_mul(60_000).saturating_add(30_000)
}

/// Opt-in payload-size sensitivity, using shapes from checked-in collector
/// observations and fixtures. These fixed examples are not production telemetry
/// or a claim about the real-world frequency of any kind of event. The generic
/// synthetic technical summary stays unchanged. User-facing summaries and
/// importance are updated for semantic coherence, so the resulting database
/// size includes their search/index effects: this is a corpus sensitivity
/// check, not an isolated JSON-column A/B measurement.
fn apply_collector_payload_sensitivity(
    event: &mut NormalizedEvent,
    slot: u64,
    burst: bool,
    selected_system: bool,
) {
    let context = match event.category {
        Category::Application => {
            // collector_applications.rs and applications/observations.rs.
            event.event_type = EventType::Updated;
            event.field = Some("version".to_owned());
            event.before = Some(json!("4.35.1"));
            event.after = Some(json!("4.36.0"));
            event.metadata = Some(json!({
                "publisher": "Example publisher",
                "install_date": "20260919",
                "burst": burst,
            }));
            event.user_friendly_summary =
                format!("{} updated 4.35.1 → 4.36.0", event.entity_display_name);
            ClassifierContext::default()
        }
        Category::Process => {
            // process/observations.rs: readable executable start (pid 4242).
            // Use the same synthetic path that produced this entity's ID.
            let path = format!(r"C:\synthetic\synthetic-entity-{slot}.exe");
            event.event_type = EventType::Started;
            event.field = None;
            event.before = None;
            event.after = None;
            event.metadata = Some(json!({
                "pid": 4242,
                "path": path,
                "path_unavailable": false,
                "path_omitted_after_defer": false,
                "browser_subprocess": false,
                "burst": burst,
            }));
            event.user_friendly_summary = format!("{} started", event.entity_display_name);
            ClassifierContext::default()
        }
        Category::Network => {
            // collector_network.rs and network/observations.rs: IP change.
            event.event_type = EventType::Modified;
            event.field = Some("ip_addresses".to_owned());
            event.before = Some(json!(["192.168.1.10/24"]));
            event.after = Some(json!(["10.0.0.2/24"]));
            event.metadata = Some(json!({
                "change_kind": "ip_changed",
                "event_name": "network/ip_changed",
                "description": format!("{} fixture", event.entity_display_name),
                "if_type": 6,
                "interface_index": 7,
                "vpn": false,
                "burst": burst,
            }));
            event.user_friendly_summary =
                format!("{} IP addresses changed", event.entity_display_name);
            ClassifierContext::default()
        }
        Category::Service => {
            // collector_services.rs and services/observations.rs: an automatic
            // service stopped, with the path and name tied to its entity ID.
            let name = format!("synthetic-entity-{slot}");
            let path = format!(r"C:\Program Files\Fixture\{name}.exe");
            event.event_type = EventType::Stopped;
            event.severity = Severity::Warning;
            event.field = Some("state".to_owned());
            event.before = Some(json!("running"));
            event.after = Some(json!("stopped"));
            event.metadata = Some(json!({
                "service_name": name,
                "process_id": null,
                "binary_path": path,
                "state": "stopped",
                "start_type": "Automatic",
                "config_available": true,
                "burst": burst,
            }));
            event.user_friendly_summary = format!("{} stopped", event.entity_display_name);
            ClassifierContext {
                automatic_service: true,
                ..ClassifierContext::default()
            }
        }
        Category::System if selected_system => {
            // eventlog/07-kernel-power-41.xml and eventlog.rs: a sampled
            // unexpected-shutdown record, not a fabricated ordinary event.
            event.event_type = EventType::Failed;
            event.severity = Severity::Error;
            event.field = None;
            event.before = None;
            event.after = None;
            event.metadata = Some(json!({
                "event_name": "system/unexpected_shutdown",
                "channel": "System",
                "provider": "Microsoft-Windows-Kernel-Power",
                "event_id": 41,
                "level": 1,
                "message": "0 | 0",
                "data": [
                    { "name": "BugcheckCode", "value": "0" },
                    { "name": "SleepInProgress", "value": "0" },
                ],
                "count": 1,
                "burst": burst,
            }));
            event.user_friendly_summary = "Windows reported an unexpected shutdown".to_owned();
            ClassifierContext {
                unexpected_shutdown: true,
                ..ClassifierContext::default()
            }
        }
        _ => return,
    };
    event.importance = classify(event, &context);
}

fn make_batch(
    first: usize,
    count: usize,
    options: &Options,
    end_at: i64,
    selected_relationships: &[bool],
    selected_system_payloads: &[bool],
    relation_ordinals: &mut [u64],
) -> Vec<Sanitized<what_changed_lib::domain::NormalizedEvent>> {
    let sanitizer = Sanitizer::new();
    let total = options.events;
    (first..first.saturating_add(count))
        .map(|index| {
            let timestamp = event_timestamp(index, total, options.days, end_at);
            let stream_seed = event_seed(index, total, options.seed);
            let mut event = synthetic_event(stream_seed, timestamp);
            let slot = (burst_anchor(index, total).unwrap_or(index) as u64) % ENTITY_COUNT;
            event.entity_id = synthetic_entity_id(event.entity_type, slot);
            event.entity_display_name = format!("Synthetic {} {slot}", event.entity_type.as_str());
            event.technical_summary =
                format!("synthetic {} event {index}", event.entity_type.as_str());
            // Index and slot are QA bookkeeping, not machine evidence. Keep
            // only the marker needed to validate the simulated burst shape.
            event.metadata = Some(json!({ "burst": burst_anchor(index, total).is_some() }));
            // `synthetic_event` repeats its pseudo-random seed in both values;
            // the state transition is what a real change record needs.
            event.before = Some(json!({ "state": "running" }));
            event.after = Some(json!({ "state": "stopped" }));
            if options.collector_payload_sensitivity {
                apply_collector_payload_sensitivity(
                    &mut event,
                    slot,
                    burst_anchor(index, total).is_some(),
                    selected_system_payloads[index],
                );
            }
            // Keep graph density bounded and explicit: exactly about 15% of
            // rows carry a related entity, never a hidden inferred edge.
            event.related_entity_ids = if selected_relationships[index] {
                let ordinal = &mut relation_ordinals[slot as usize];
                let target = related_entity_id(event.entity_type, slot, *ordinal);
                *ordinal += 1;
                vec![target]
            } else {
                Vec::new()
            };
            sanitizer.sanitize(event).0
        })
        .collect()
}

/// Carry only graph evidence across the event writer boundary. Most corpus
/// rows have no edge, and cloning their JSON payloads cannot affect the graph.
struct RelationSeedInput {
    batch_index: usize,
    timestamp: i64,
    category: Category,
    entity_type: EntityType,
    entity_id: String,
    entity_display_name: String,
    related_entity_ids: Vec<String>,
}

fn selected_relation_inputs(batch: &[Sanitized<NormalizedEvent>]) -> Vec<RelationSeedInput> {
    batch
        .iter()
        .enumerate()
        .filter_map(|(batch_index, sanitized)| {
            let event = sanitized.get();
            (!event.related_entity_ids.is_empty()).then(|| RelationSeedInput {
                batch_index,
                timestamp: event.timestamp,
                category: event.category,
                entity_type: event.entity_type,
                entity_id: event.entity_id.clone(),
                entity_display_name: event.entity_display_name.clone(),
                related_entity_ids: event.related_entity_ids.clone(),
            })
        })
        .collect()
}

/// Aggregate QA timings contain no event fields or filesystem identities.
/// Keeping the production writer unchanged makes each stage comparable with
/// the regular persistence path, including sanitization and SQLite triggers.
#[derive(Default)]
struct SeedTimings {
    make_batch: Duration,
    insert_events: Duration,
    graph: Duration,
    inventory: Duration,
    checkpoints: Duration,
    investigations: Duration,
    insertion: BatchInsertionTimings,
}

impl SeedTimings {
    fn print_insertion(&self) {
        let insertion = self.insertion;
        // This residual is caller/writer overhead, not a queue-wait measurement:
        // it cannot separate locking, handoff, scheduling, queueing and reply.
        let overhead = self
            .insert_events
            .checked_sub(insertion.repository_total)
            .map(|duration| format!("{:.3}", duration.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "unavailable".to_owned());
        println!(
            "seed insertion timings: batches={} attempted_rows={} committed_batches={} committed_rows={} failed_batches={} empty_batches={} completed_phases={} begin_prepare_ms={:.3} event_entity_loop_ms={:.3} reconciliation_ms={:.3} commit_ms={:.3} repository_total_ms={:.3} caller_insert_ms={:.3} caller_writer_overhead_ms={overhead}",
            insertion.batches, insertion.attempted_rows, insertion.committed_batches,
            insertion.committed_rows, insertion.failed_batches, insertion.empty_batches,
            insertion.completed_phases,
            insertion.begin_prepare.as_secs_f64() * 1000.0,
            insertion.event_entity_loop.as_secs_f64() * 1000.0,
            insertion.reconciliation.as_secs_f64() * 1000.0,
            insertion.commit.as_secs_f64() * 1000.0,
            insertion.repository_total.as_secs_f64() * 1000.0,
            self.insert_events.as_secs_f64() * 1000.0,
        );
    }

    fn print(&self, total_run: Duration) {
        println!(
            "seed timings: elapsed_make_batch_ms={:.3} insert_events_ms={:.3} graph_ms={:.3} inventory_ms={:.3} checkpoints_ms={:.3} investigations_ms={:.3} total_run_ms={:.3}",
            self.make_batch.as_secs_f64() * 1000.0,
            self.insert_events.as_secs_f64() * 1000.0,
            self.graph.as_secs_f64() * 1000.0,
            self.inventory.as_secs_f64() * 1000.0,
            self.checkpoints.as_secs_f64() * 1000.0,
            self.investigations.as_secs_f64() * 1000.0,
            total_run.as_secs_f64() * 1000.0,
        );
    }
}

fn seed_events(
    db: &Db,
    options: &Options,
    end_at: i64,
    timings: &mut SeedTimings,
    batch_size: SeedBatchSize,
) -> AppResult<(usize, HashSet<String>)> {
    let selected_relationships = if options.relations {
        relationship_selection(options.events, options.seed)?
    } else {
        // A write-only QA run must not claim relationships in its event JSON
        // when it intentionally skips persisting graph evidence.
        vec![false; options.events]
    };
    let selected_system_payloads = if options.collector_payload_sensitivity {
        system_payload_selection(options.events, options.seed)
    } else {
        Vec::new()
    };
    let mut inserted = 0usize;
    let mut owned_entities = HashSet::<String>::new();
    let mut known_entities = HashSet::<String>::new();
    let mut relation_ordinals = vec![0_u64; ENTITY_COUNT as usize];
    while inserted < options.events {
        let count = batch_size.rows().min(options.events - inserted);
        let batch_started = Instant::now();
        let batch = make_batch(
            inserted,
            count,
            options,
            end_at,
            &selected_relationships,
            &selected_system_payloads,
            &mut relation_ordinals,
        );
        let relation_inputs = options.relations.then(|| selected_relation_inputs(&batch));
        // This stage includes generation, sanitization and compact graph
        // inputs; both blocking writer calls are measured separately below.
        timings.make_batch += batch_started.elapsed();
        let insert_started = Instant::now();
        let (ids, owned) = if options.insertion_timings {
            let captured =
                db.insert_events_preserving_entities_with_timings(batch, owned_entities)?;
            timings.insert_events += insert_started.elapsed();
            timings.insertion.accumulate(captured.timings);
            captured.value?
        } else {
            let inserted = db.insert_events_preserving_entities(batch, owned_entities)?;
            timings.insert_events += insert_started.elapsed();
            inserted
        };
        if let Some(relation_inputs) = relation_inputs {
            let graph_started = Instant::now();
            let relation_ids = ids.clone();
            let (entities, graph_owned) = db.write(move |conn| {
                // One QA graph batch is one SQLite transaction. Without this,
                // every edge INSERT autocommits independently, making the 1M
                // corpus spend most of its time on 150k tiny transactions.
                let tx = conn
                    .unchecked_transaction()
                    .map_err(|source| DbError::Sqlite {
                        operation: "start a synthetic graph batch",
                        source,
                    })?;
                let (known_entities, owned) = seed_graph_body::insert_graph_body(
                    &tx,
                    &relation_inputs,
                    &relation_ids,
                    known_entities,
                    owned,
                )?;
                tx.commit().map_err(|source| DbError::Sqlite {
                    operation: "commit a synthetic graph batch",
                    source,
                })?;
                Ok((known_entities, owned))
            })?;
            timings.graph += graph_started.elapsed();
            known_entities = entities;
            owned_entities = graph_owned;
        } else {
            owned_entities = owned;
        }
        inserted = inserted.saturating_add(ids.len());
        if inserted % 10_000 == 0 || inserted == options.events {
            println!("seeded {inserted}/{} synthetic events", options.events);
        }
    }
    Ok((inserted, owned_entities))
}

fn fill_checkpoint_inventory(
    db: &Db,
    owned_entities: HashSet<String>,
    at: i64,
) -> AppResult<usize> {
    db.write(move |conn| {
        let mut owned = owned_entities;
        // System history legitimately shares `system:local`. Complete the
        // 3k-node graph with unchanged services from the 3k-item checkpoint
        // inventory instead of inventing invalid system identities.
        for slot in 0..ENTITY_COUNT.saturating_mul(4) {
            if owned.len() >= ENTITY_COUNT as usize {
                break;
            }
            let id = synthetic_entity_id(EntityType::Service, slot);
            if owned.contains(&id)
                || what_changed_lib::storage::entity_repository::by_id(conn, &id)?.is_some()
            {
                continue;
            }
            let service = Entity {
                id: id.clone(),
                entity_type: EntityType::Service,
                display_name: format!("Synthetic service {slot}"),
                first_seen: at,
                last_seen: at,
                metadata: None,
            };
            let service = Sanitizer::new().sanitize(service).0;
            what_changed_lib::storage::entity_repository::upsert_sanitized(conn, &service)?;
            owned.insert(id);
        }
        if owned.len() < ENTITY_COUNT as usize {
            return Err(AppError::InvalidInput {
                what: "synthetic entity inventory",
                reason: "too many pre-existing entity IDs to seed a safe 3k-node corpus".to_owned(),
            });
        }
        Ok(owned.len())
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SeedSummary {
    events: usize,
    entities: usize,
    checkpoints: usize,
    investigations: usize,
    end_at: i64,
}

fn seed_checkpoints(db: &Db, options: &Options, end_at: i64, marker: &str) -> AppResult<usize> {
    for index in 0..CHECKPOINT_COUNT {
        let event_index = (index + 1) * options.events / (CHECKPOINT_COUNT + 1);
        let at = event_timestamp(event_index, options.events, options.days, end_at);
        // Two full-state checkpoints make a meaningful 3k-item comparison
        // pair; the remaining historical markers stay compact.
        let item_count = if index < 2 {
            LARGE_CHECKPOINT_ITEMS
        } else {
            CHECKPOINT_ITEMS
        };
        let items = (0..item_count)
            .map(|item_index| {
                let slot = (item_index as u64) % ENTITY_COUNT;
                CheckpointItem {
                    collector_id: "services".to_owned(),
                    entity_kind: "service".to_owned(),
                    entity_external_id: synthetic_entity_id(EntityType::Service, slot),
                    entity_name: Some(format!("Synthetic service {slot}")),
                    state: json!({
                        "name": format!("synthetic-entity-{slot}"),
                        "display_name": format!("Synthetic service {slot}"),
                        "state": if slot.wrapping_add(options.seed).wrapping_add(index as u64) % 97 == 0 {
                            "stopped"
                        } else {
                            "running"
                        },
                        "start_type": "automatic",
                        "config_available": true,
                        "process_id": slot,
                    }),
                }
            })
            .collect();
        let (capture, _) = prepare_capture(
            Some(format!("Synthetic checkpoint {:02}", index + 1)),
            CheckpointKind::Manual,
            Some(marker.to_owned()),
            at,
            0,
            vec!["services".to_owned()],
            Vec::new(),
            items,
            &Sanitizer::new(),
        );
        db.write(move |conn| checkpoint_repository::create(conn, &capture))?;
    }
    Ok(CHECKPOINT_COUNT)
}

fn seed_investigations(
    db: &Arc<Db>,
    options: &Options,
    end_at: i64,
    marker: &str,
) -> AppResult<usize> {
    let service = InvestigationService::new(Arc::clone(db));
    for index in 0..INVESTIGATION_COUNT {
        let center = (index + 1) * options.events / (INVESTIGATION_COUNT + 1);
        let first = center.saturating_sub(50);
        let last = center.saturating_add(50).min(options.events - 1);
        let window = InvestigationWindow::Custom {
            from_ms: event_timestamp(first, options.events, options.days, end_at),
            to_ms: event_timestamp(last, options.events, options.days, end_at),
        };
        // The exact random marker is the persisted problem, not a fuzzy name
        // pattern. Purge can therefore distinguish these rows from real
        // investigations even when a person's text says "synthetic".
        service.start(marker.to_owned(), Some(window))?;
    }
    Ok(INVESTIGATION_COUNT)
}

fn seed(
    db: &Arc<Db>,
    options: &Options,
    timings: &mut SeedTimings,
    batch_size: SeedBatchSize,
) -> AppResult<SeedSummary> {
    let end_at = options
        .end_at
        .unwrap_or_else(what_changed_lib::domain::now_ms);
    // Register before the first artifact: an interrupted run remains
    // discoverable and the next explicit purge can remove its partial rows.
    let marker = maintenance::register_synthetic_run(db)?;
    let (events, owned_entities) = seed_events(db, options, end_at, timings, batch_size)?;
    finish_seed(
        db,
        options,
        timings,
        end_at,
        &marker,
        events,
        owned_entities,
    )
}

/// Shared unchanged artifact recipe after an event route has completed.
fn finish_seed(
    db: &Arc<Db>,
    options: &Options,
    timings: &mut SeedTimings,
    end_at: i64,
    marker: &str,
    events: usize,
    owned_entities: HashSet<String>,
) -> AppResult<SeedSummary> {
    let inventory_started = Instant::now();
    let entities = fill_checkpoint_inventory(
        db,
        owned_entities,
        event_timestamp(0, options.events, options.days, end_at),
    )?;
    timings.inventory += inventory_started.elapsed();
    let checkpoints_started = Instant::now();
    let checkpoints = seed_checkpoints(db, options, end_at, marker)?;
    timings.checkpoints += checkpoints_started.elapsed();
    let investigations_started = Instant::now();
    let investigations = seed_investigations(db, options, end_at, marker)?;
    timings.investigations += investigations_started.elapsed();
    Ok(SeedSummary {
        events,
        entities,
        checkpoints,
        investigations,
        end_at,
    })
}

fn purge(db: &Db) -> AppResult<usize> {
    maintenance::purge_synthetic(db)
}

fn run(options: Options) -> Result<(), Box<dyn std::error::Error>> {
    let run_started = Instant::now();
    let root = qa_directory(
        options
            .data_dir
            .as_deref()
            .expect("--data-dir was validated during argument parsing"),
    )?;
    let _lock = if options.purge {
        prepare_purge_directory(&root)?
    } else {
        prepare_seed_directory(&root)?
    };
    let db = Arc::new(Db::open(&DbPaths::under(&root))?);
    let mut timings = SeedTimings::default();
    let operation = (|| -> AppResult<()> {
        if options.purge {
            let deleted = purge(&db)?;
            println!("purged {deleted} synthetic events");
        } else {
            let result = seed(&db, &options, &mut timings, SeedBatchSize::Standard)?;
            println!(
                "synthetic seed complete: events={} entities={} checkpoints={} investigations={} seed={} days={} relations={} collector_payload_sensitivity={} end_at={}",
                result.events,
                result.entities,
                result.checkpoints,
                result.investigations,
                options.seed,
                options.days,
                options.relations,
                options.collector_payload_sensitivity,
                result.end_at
            );
        }
        Ok(())
    })();
    // A partially completed run is expected to be purged later. Close the
    // writer cleanly before returning its error and exiting non-zero.
    let flush = db.flush();
    db.close();
    // One aggregate record, including completed work before a failed seed.
    // No per-batch output or change to the existing coarse --timings record.
    if options.insertion_timings && !options.purge {
        timings.print_insertion();
    }
    operation?;
    flush?;
    if options.timings && !options.purge {
        timings.print(run_started.elapsed());
    }
    Ok(())
}

fn main() {
    match parse_options(std::env::args().skip(1)) {
        Ok(options) => {
            if let Err(error) = run(options) {
                eprintln!("seed_history failed: {error}");
                std::process::exit(1);
            }
        }
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
#[path = "../bin_support/seed_history_proof/seed_batch_boundary_proof.rs"]
mod seed_batch_boundary_proof;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_attribution_requires_its_own_explicit_cli_flag() {
        let args = |extra: &[&str]| {
            ["--data-dir", "what-changed-seed-fixture"]
                .into_iter()
                .chain(extra.iter().copied())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert!(!parse_options(args(&[])).unwrap().insertion_timings);
        let coarse = parse_options(args(&["--timings"])).unwrap();
        assert!(coarse.timings && !coarse.insertion_timings);
        let attributed = parse_options(args(&["--insertion-timings"])).unwrap();
        assert!(attributed.insertion_timings && !attributed.timings);
    }

    #[test]
    fn graph_inputs_keep_selected_batch_positions_and_sanitized_fields() {
        let sanitizer = Sanitizer::new();
        let batch = (0..5)
            .map(|index| {
                let mut event = synthetic_event(index, index as i64);
                if index != 1 && index != 4 {
                    event.related_entity_ids.clear();
                }
                sanitizer.sanitize(event).0
            })
            .collect::<Vec<_>>();
        let inputs = selected_relation_inputs(&batch);
        let ids = [101_i64, 102, 103, 104, 105];
        assert_eq!(inputs.len(), 2);
        assert_eq!(
            inputs
                .iter()
                .map(|input| (input.batch_index, ids[input.batch_index]))
                .collect::<Vec<_>>(),
            [(1, 102), (4, 105)]
        );
        for input in &inputs {
            let event = batch[input.batch_index].get();
            assert_eq!(input.timestamp, event.timestamp);
            assert_eq!(input.category, event.category);
            assert_eq!(input.entity_type, event.entity_type);
            assert_eq!(input.entity_id, event.entity_id);
            assert_eq!(input.entity_display_name, event.entity_display_name);
            assert_eq!(input.related_entity_ids, event.related_entity_ids);
        }
    }
}
