//! Shared plumbing for the OS-backed collector tests.
//!
//! These drive real collectors through the real supervisor, the real normalizer
//! and a real SQLite database, with scripted providers standing in for the
//! operating system. What they prove that a collector's own unit tests cannot is
//! the far end: that the identities, summaries and graph evidence a collector
//! produces survive normalization, redaction, the atomic commit's validation,
//! and a read back out of the database.
//!
//! The cadence is the one thing not taken from the catalog. A one-second poll
//! would make an eight-pair churn test cost a minute of wall clock, so the test
//! descriptor keeps the catalog's id and copy and shortens the interval; the
//! shared clock is advanced by hand instead.

#![allow(dead_code)]

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rusqlite::Connection;
use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::listeners::{self, ListenerCollector};
use what_changed_lib::collectors::process::{self, ProcessCollector};
use what_changed_lib::collectors::process_map::ProcessMap;
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::{
    BackoffPolicy, CollectorRuntimeConfig, CollectorSupervisor, RuntimeClock,
};
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::ObservationClock;
use what_changed_lib::domain::ids::Protocol;
use what_changed_lib::domain::{
    CollectorCadence, CollectorDescriptor, Entity, MonitoringProfile, Relationship,
    RelationshipKind,
};
use what_changed_lib::dto::CollectorStatusDto;
use what_changed_lib::error::{AppError, AppResult};
use what_changed_lib::platform::traits::{ProcessProvider, ProcessRow, SocketProvider, SocketRow};
use what_changed_lib::storage::{entity_repository, Db};

/// Where the shared clock starts, as epoch milliseconds UTC.
pub const EPOCH_MS: i64 = 1_700_000_000_000;

/// One clock for both the runtime's scheduling and the collectors' timestamps,
/// so a test advances time once and both agree about what happened when.
#[derive(Clone)]
pub struct ManualClock {
    elapsed_ms: Arc<AtomicI64>,
    base: Instant,
}

impl Default for ManualClock {
    fn default() -> Self {
        Self::new()
    }
}

impl ManualClock {
    pub fn new() -> Self {
        Self::starting_at(Duration::ZERO)
    }

    /// A clock that begins some way into the session, so a test can start a
    /// second recorder far enough after the first that the gap between them
    /// really is downtime.
    pub fn starting_at(elapsed: Duration) -> Self {
        let millis = i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX);
        Self {
            elapsed_ms: Arc::new(AtomicI64::new(millis)),
            base: Instant::now(),
        }
    }

    pub fn advance(&self, by: Duration) {
        let millis = i64::try_from(by.as_millis()).unwrap_or(i64::MAX);
        self.elapsed_ms.fetch_add(millis, Ordering::AcqRel);
    }

    /// Now, in epoch milliseconds.
    ///
    /// An inherent method rather than only a trait one: this clock implements
    /// both clock traits, each of which defines `now_ms`, and a call site should
    /// not have to say which it means when the two agree.
    pub fn now_ms(&self) -> i64 {
        EPOCH_MS.saturating_add(self.elapsed())
    }

    fn elapsed(&self) -> i64 {
        self.elapsed_ms.load(Ordering::Acquire)
    }
}

impl RuntimeClock for ManualClock {
    fn now(&self) -> Instant {
        let elapsed = u64::try_from(self.elapsed()).unwrap_or(u64::MAX);
        self.base + Duration::from_millis(elapsed)
    }

    fn now_ms(&self) -> i64 {
        ManualClock::now_ms(self)
    }
}

impl ObservationClock for ManualClock {
    fn now_ms(&self) -> i64 {
        ManualClock::now_ms(self)
    }
}

/// Backoff short enough that a failure never ends a test by parking a worker.
pub struct FixedBackoff(pub Duration);

impl BackoffPolicy for FixedBackoff {
    fn delay(&self, _consecutive_failures: u64) -> Duration {
        self.0
    }
}

/// A process provider that hands back one scripted table per call, repeating the
/// last one once the script runs out.
#[derive(Clone, Default)]
pub struct ScriptedProcesses {
    tables: Arc<Mutex<VecDeque<Vec<ProcessRow>>>>,
    calls: Arc<AtomicU64>,
}

impl ScriptedProcesses {
    pub fn new(tables: Vec<Vec<ProcessRow>>) -> Self {
        Self {
            tables: Arc::new(Mutex::new(tables.into())),
            calls: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Replace the script. Later cycles read the new tables in order.
    pub fn script(&self, tables: Vec<Vec<ProcessRow>>) {
        *lock(&self.tables) = tables.into();
    }

    pub fn calls(&self) -> u64 {
        self.calls.load(Ordering::Acquire)
    }
}

impl ProcessProvider for ScriptedProcesses {
    fn enumerate(&self) -> AppResult<Vec<ProcessRow>> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        let mut tables = lock(&self.tables);
        // The last scripted table is held rather than consumed, so a cycle that
        // was not given a new one reads the machine as unchanged instead of as
        // empty — which would report every running program as having stopped.
        if tables.len() > 1 {
            return Ok(tables.pop_front().unwrap_or_default());
        }
        Ok(tables.front().cloned().unwrap_or_default())
    }
}

/// A socket provider that hands back one scripted table per call.
#[derive(Clone, Default)]
pub struct ScriptedSockets {
    tables: Arc<Mutex<VecDeque<Vec<SocketRow>>>>,
    calls: Arc<AtomicU64>,
}

impl ScriptedSockets {
    pub fn new(tables: Vec<Vec<SocketRow>>) -> Self {
        Self {
            tables: Arc::new(Mutex::new(tables.into())),
            calls: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn script(&self, tables: Vec<Vec<SocketRow>>) {
        *lock(&self.tables) = tables.into();
    }

    pub fn calls(&self) -> u64 {
        self.calls.load(Ordering::Acquire)
    }
}

impl SocketProvider for ScriptedSockets {
    fn listeners(&self) -> AppResult<Vec<SocketRow>> {
        self.calls.fetch_add(1, Ordering::AcqRel);
        let mut tables = lock(&self.tables);
        if tables.len() > 1 {
            return Ok(tables.pop_front().unwrap_or_default());
        }
        Ok(tables.front().cloned().unwrap_or_default())
    }
}

/// A provider that always fails, for the isolation assertions.
#[derive(Clone, Copy, Default)]
pub struct FailingProcesses;

impl ProcessProvider for FailingProcesses {
    fn enumerate(&self) -> AppResult<Vec<ProcessRow>> {
        Err(AppError::Platform {
            operation: "take a process snapshot",
            reason: "0x80070005".to_owned(),
        })
    }
}

/// A process whose executable path is readable.
pub fn process(pid: u32, image: &str, creation: u64) -> ProcessRow {
    ProcessRow {
        pid,
        parent_pid: 0,
        image_name: image.to_owned(),
        executable_path: Some(format!(r"C:\dev\{image}")),
        creation_time: Some(creation),
    }
}

/// A process that denies the path read, which is what roughly 46% of a real
/// machine does for a non-elevated caller.
pub fn opaque_process(pid: u32, image: &str, creation: u64) -> ProcessRow {
    ProcessRow {
        pid,
        parent_pid: 0,
        image_name: image.to_owned(),
        executable_path: None,
        creation_time: Some(creation),
    }
}

/// A process with neither a readable path nor a readable creation time: the
/// weakest identity the recorder can hold.
pub fn unidentified_process(pid: u32, image: &str) -> ProcessRow {
    ProcessRow {
        pid,
        parent_pid: 0,
        image_name: image.to_owned(),
        executable_path: None,
        creation_time: None,
    }
}

/// A child of `parent_pid` running the same image, which is the browser-helper
/// shape the damping rule looks for.
pub fn browser_child(pid: u32, parent_pid: u32, image: &str, creation: u64) -> ProcessRow {
    ProcessRow {
        parent_pid,
        ..process(pid, image, creation)
    }
}

/// One listening socket.
pub fn socket(protocol: Protocol, address: &str, port: u16, pid: u32) -> SocketRow {
    SocketRow {
        protocol,
        address: address.to_owned(),
        port,
        owning_pid: pid,
    }
}

/// One stored event row, read straight back out of the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredEvent {
    pub id: i64,
    pub timestamp: i64,
    pub category: String,
    pub event_type: String,
    pub entity_id: String,
    pub entity_display_name: String,
    pub importance: String,
    pub confidence: String,
    pub source_collector: String,
    pub user_friendly_summary: String,
    pub metadata: serde_json::Value,
    pub occurred_at_is_uncertain: bool,
}

/// One stored graph edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRelationship {
    pub source_id: String,
    pub target_id: String,
    pub kind: RelationshipKind,
    pub event_id: Option<i64>,
    pub recorded_at: i64,
}

/// Both collectors, one shared process map, one clock, one real database.
pub struct Rig {
    pub db: Arc<Db>,
    pub clock: ManualClock,
    pub map: ProcessMap,
    pub directory: PathBuf,
    supervisor: CollectorSupervisor,
}

impl Rig {
    /// A rig over any two providers, so the same plumbing drives scripted
    /// tables and the real operating system.
    pub fn with<P, S>(processes: P, sockets: S, label: &str) -> Self
    where
        P: ProcessProvider + Clone,
        S: SocketProvider + Clone,
    {
        let directory = what_changed_lib::testing::temp_dir(&format!("collectors-{label}"));
        Self::at(directory, processes, sockets)
    }

    /// A rig over an existing directory, so a test can stop one recorder and
    /// start another over the same history. That is what an app restart is, and
    /// it is the only way to test what a collector does with a baseline it did
    /// not write in this session.
    pub fn at<P, S>(directory: PathBuf, processes: P, sockets: S) -> Self
    where
        P: ProcessProvider + Clone,
        S: SocketProvider + Clone,
    {
        Self::at_elapsed(directory, processes, sockets, Duration::ZERO)
    }

    /// [`at`](Self::at) with the clock already some way into the session.
    pub fn at_elapsed<P, S>(directory: PathBuf, processes: P, sockets: S, elapsed: Duration) -> Self
    where
        P: ProcessProvider + Clone,
        S: SocketProvider + Clone,
    {
        let db = Arc::new(Db::open_in(&directory).unwrap());
        let clock = ManualClock::starting_at(elapsed);
        let map = ProcessMap::new();

        let process_descriptor = fast(process::catalog_descriptor().unwrap());
        let listener_descriptor = fast(listeners::catalog_descriptor().unwrap());
        let process_collector = clock.clone();
        let listener_collector = clock.clone();
        let process_map = map.clone();
        let listener_map = map.clone();
        let process_provider = processes;
        let listener_provider = sockets;

        let registry = CollectorRegistry::new(vec![
            RegistryEntry::active(process_descriptor.clone(), move || {
                Box::new(ProcessCollector::with_clock(
                    process_provider.clone(),
                    process_map.clone(),
                    process_descriptor.clone(),
                    process_collector.clone(),
                ))
            }),
            RegistryEntry::active(listener_descriptor.clone(), move || {
                Box::new(ListenerCollector::with_clock(
                    listener_provider.clone(),
                    listener_map.clone(),
                    listener_descriptor.clone(),
                    listener_collector.clone(),
                ))
            }),
        ]);

        let supervisor = CollectorSupervisor::with_runtime(
            registry,
            CollectorRuntimeConfig {
                db: Arc::clone(&db),
                monitor: Arc::new(Monitoring::running()),
                query_busy: QueryBusy::default(),
                enabled: BTreeMap::from([
                    (process::ID.to_owned(), true),
                    (listeners::ID.to_owned(), true),
                ]),
                profile: MonitoringProfile::Balanced,
                clock: Arc::new(clock.clone()) as Arc<dyn RuntimeClock>,
                backoff: Arc::new(FixedBackoff(Duration::from_millis(1))),
            },
        );
        supervisor.spawn_all().unwrap();

        let rig = Self {
            db,
            clock,
            map,
            directory,
            supervisor,
        };
        // Both collectors take a silent first snapshot at spawn. Waiting for it
        // here is what lets a test start from a known, established baseline
        // instead of racing the first cycle.
        rig.wait_for_first_cycles();
        rig
    }

    /// A rig over scripted tables that both start empty.
    pub fn scripted(label: &str) -> (Self, ScriptedProcesses, ScriptedSockets) {
        let processes = ScriptedProcesses::new(vec![Vec::new()]);
        let sockets = ScriptedSockets::new(vec![Vec::new()]);
        let rig = Self::with(processes.clone(), sockets.clone(), label);
        (rig, processes, sockets)
    }

    /// Shorten the cadence without touching anything the catalog owns.
    pub fn status(&self, id: &str) -> CollectorStatusDto {
        self.supervisor
            .list_collector_status()
            .into_iter()
            .find(|status| status.id == id)
            .unwrap_or_else(|| panic!("no collector status for {id}"))
    }

    /// Advance one cadence tick and wait for both collectors to finish a cycle
    /// at or after the new time. The atomic commit is durable before its cycle
    /// is recorded as run, so a completed cycle means readable rows.
    pub fn cycle(&self) {
        self.advance(Duration::from_secs(1));
    }

    /// Advance by an explicit amount, for tests that need a window to elapse.
    pub fn advance(&self, by: Duration) {
        self.clock.advance(by);
        let target = self.clock.now_ms();
        wait_for("both collectors to finish a cycle", || {
            self.ran_since(process::ID, target) && self.ran_since(listeners::ID, target)
        });
    }

    fn ran_since(&self, id: &str, target: i64) -> bool {
        self.status(id).last_run_at.is_some_and(|at| at >= target)
    }

    fn wait_for_first_cycles(&self) {
        let target = self.clock.now_ms();
        wait_for("the establishing cycles", || {
            self.ran_since(process::ID, target) && self.ran_since(listeners::ID, target)
        });
    }

    /// Every event row, oldest first.
    pub fn events(&self) -> Vec<StoredEvent> {
        self.db
            .read(|connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT id, timestamp, category, event_type, entity_id, \
                         entity_display_name, importance, confidence, source_collector, \
                         user_friendly_summary, metadata_json, occurred_at_is_uncertain \
                         FROM events ORDER BY id ASC",
                    )
                    .unwrap();
                let rows = statement
                    .query_map([], |row| {
                        let metadata: Option<String> = row.get(10)?;
                        Ok(StoredEvent {
                            id: row.get(0)?,
                            timestamp: row.get(1)?,
                            category: row.get(2)?,
                            event_type: row.get(3)?,
                            entity_id: row.get(4)?,
                            entity_display_name: row.get(5)?,
                            importance: row.get(6)?,
                            confidence: row.get(7)?,
                            source_collector: row.get(8)?,
                            user_friendly_summary: row.get(9)?,
                            metadata: metadata
                                .and_then(|text| serde_json::from_str(&text).ok())
                                .unwrap_or(serde_json::Value::Null),
                            occurred_at_is_uncertain: row.get::<_, i64>(11)? != 0,
                        })
                    })
                    .unwrap();
                Ok(rows.collect::<Result<Vec<_>, _>>().unwrap())
            })
            .unwrap()
    }

    /// Events from one collector, oldest first.
    pub fn events_from(&self, collector: &str) -> Vec<StoredEvent> {
        self.events()
            .into_iter()
            .filter(|event| event.source_collector == collector)
            .collect()
    }

    pub fn event_count(&self) -> u64 {
        self.db
            .read(|connection| {
                connection
                    .query_row("SELECT COUNT(*) FROM events", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .map_err(|source| what_changed_lib::error::DbError::Sqlite {
                        operation: "count events in a collector test",
                        source,
                    })
                    .map_err(Into::into)
                    .and_then(|count| {
                        u64::try_from(count).map_err(|_| AppError::InvalidInput {
                            what: "event count",
                            reason: "a row count was negative".to_owned(),
                        })
                    })
            })
            .unwrap()
    }

    /// Every graph edge, ordered so a test can assert on both directions.
    pub fn relationships(&self) -> Vec<StoredRelationship> {
        self.db
            .read(|connection| Ok(read_relationships(connection)))
            .unwrap()
    }

    /// Edges leaving one entity, through the repository the analysis engine and
    /// the System Map will use rather than through this harness's own SQL.
    pub fn relationships_from(&self, source_id: &str) -> Vec<Relationship> {
        let id = source_id.to_owned();
        self.db
            .read(move |connection| entity_repository::relationships_from(connection, &id))
            .unwrap()
    }

    /// Edges arriving at one entity, through the same repository.
    pub fn relationships_to(&self, target_id: &str) -> Vec<Relationship> {
        let id = target_id.to_owned();
        self.db
            .read(move |connection| entity_repository::relationships_to(connection, &id))
            .unwrap()
    }

    /// The stored entity row for one id, or `None`.
    pub fn entity(&self, id: &str) -> Option<Entity> {
        let wanted = id.to_owned();
        self.db
            .read(move |connection| entity_repository::by_id(connection, &wanted))
            .ok()
            .flatten()
    }

    /// The stored baseline for one collector, or `None`.
    pub fn baseline(&self, collector: &str, kind: &str) -> Option<serde_json::Value> {
        self.db
            .read(|connection| {
                what_changed_lib::storage::baseline_repository::load(connection, collector, kind)
            })
            .ok()
            .flatten()
            .map(|stored| stored.snapshot)
    }

    pub fn stop(&self) {
        self.supervisor.stop_all();
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.supervisor.stop_all();
    }
}

fn read_relationships(connection: &Connection) -> Vec<StoredRelationship> {
    let mut statement = connection
        .prepare(
            "SELECT source_id, target_id, kind, event_id, recorded_at \
             FROM entity_relationships ORDER BY id ASC",
        )
        .unwrap();
    let rows = statement
        .query_map([], |row| {
            let kind: String = row.get(2)?;
            let parsed = kind
                .parse::<RelationshipKind>()
                .expect("the database only holds relationship kinds this build defines");
            Ok(StoredRelationship {
                source_id: row.get(0)?,
                target_id: row.get(1)?,
                kind: parsed,
                event_id: row.get(3)?,
                recorded_at: row.get(4)?,
            })
        })
        .unwrap();
    rows.collect::<Result<Vec<_>, _>>().unwrap()
}

/// The catalog descriptor with a cadence a test can drive by hand.
///
/// The id, the copy, the cost and the privilege declaration all stay exactly as
/// the registry owns them; only the interval changes, because a test that waits
/// a real second per cycle is a test nobody runs.
pub fn fast(descriptor: CollectorDescriptor) -> CollectorDescriptor {
    CollectorDescriptor {
        cadence: CollectorCadence::Poll(Duration::from_millis(1)),
        ..descriptor
    }
}

/// Poll until a predicate holds, and name what timed out when it does not.
pub fn wait_for(label: &str, predicate: impl Fn() -> bool) {
    for _ in 0..400 {
        if predicate() {
            return;
        }
        std::thread::yield_now();
    }
    for _ in 0..2_000 {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("{label} did not become true");
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
