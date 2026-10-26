//! Runtime ownership of atomic collector prepare, admission and acknowledgement.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::unwrap_used
)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rusqlite::Connection;
use serde_json::json;
use what_changed_lib::app::Monitoring;
use what_changed_lib::collectors::registry::{CollectorRegistry, RegistryEntry};
use what_changed_lib::collectors::runtime::{
    BackoffPolicy, CollectorRuntimeConfig, CollectorSupervisor, RuntimeClock, RuntimeStatusSink,
};
use what_changed_lib::collectors::schedule::QueryBusy;
use what_changed_lib::collectors::{
    AckedState, Collector, CollectorContext, CollectorSnapshot, CommitToken, EntityIdentity,
    ObservationKind, RawBaseline, RawEntity, RawObservation, RawRelationshipSpec, SnapshotContext,
};
use what_changed_lib::domain::{
    ids, Category, ClassifierContext, CollectorCadence, CollectorCost, CollectorDescriptor,
    Confidence, EventType, MonitoringProfile, RelationshipKind, Severity,
};
use what_changed_lib::dto::{CollectorState, MonitoringDto};
use what_changed_lib::error::{AppError, AppResult, DbError};
use what_changed_lib::privacy::Sanitizer;
use what_changed_lib::storage::{
    baseline_repository, event_repository, CollectorCommitReceipt, Db,
};

const COLLECTOR_ID: &str = "atomic";
const AT: i64 = 1_700_000_000_000;

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn descriptor() -> CollectorDescriptor {
    CollectorDescriptor {
        id: COLLECTOR_ID,
        display_name: "Atomic test",
        what_it_watches: "deterministic facts",
        why_it_matters: "proves runtime commit ordering",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Notify,
        requires_admin: false,
    }
}

fn process(marker: &str) -> RawEntity {
    RawEntity {
        identity: EntityIdentity::Process {
            executable: format!("{marker}.exe"),
        },
        display_name: marker.to_owned(),
    }
}

fn port() -> RawEntity {
    RawEntity {
        identity: EntityIdentity::Port {
            protocol: ids::Protocol::Tcp,
            address: "127.0.0.1".to_owned(),
            port: 4_321,
        },
        display_name: "local listener".to_owned(),
    }
}

fn observation(marker: &str, at: i64) -> RawObservation {
    let executable = format!("{marker}.exe");
    let process_id = ids::process(&executable);
    let port_id = ids::port(ids::Protocol::Tcp, "127.0.0.1", 4_321);
    RawObservation {
        collector_id: COLLECTOR_ID,
        kind: ObservationKind {
            category: Category::Process,
            event_type: EventType::Started,
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary: format!("{marker} started"),
            technical_summary: format!("observed {marker}"),
            classifier: ClassifierContext::default(),
        },
        entity: process(marker),
        before: None,
        after: Some(json!({ "marker": marker })),
        field: None,
        occurred_at: at,
        details: None,
        // The normalizer must remove the primary id and deduplicate the
        // relationship endpoint id before storage validation.
        related_entity_ids: vec![port_id.clone(), process_id, port_id],
        occurred_at_is_uncertain: false,
    }
}

fn atomic_with_event(
    token: CommitToken,
    marker: &str,
    at: i64,
    baseline: bool,
) -> CollectorSnapshot {
    CollectorSnapshot::Atomic {
        token,
        observations: vec![observation(marker, at)],
        relationships: vec![RawRelationshipSpec {
            evidence_observation: 0,
            source: process(marker),
            target: port(),
            kind: RelationshipKind::ListensOn,
        }],
        baseline: baseline.then(|| RawBaseline {
            entity_kind: "state".to_owned(),
            snapshot: json!({ "marker": marker }),
            captured_at: at,
        }),
        config_snapshots: Vec::new(),
    }
}

fn baseline_only(token: CommitToken, marker: &str, at: i64) -> CollectorSnapshot {
    CollectorSnapshot::Atomic {
        token,
        observations: Vec::new(),
        relationships: Vec::new(),
        baseline: Some(RawBaseline {
            entity_kind: "state".to_owned(),
            snapshot: json!({ "marker": marker }),
            captured_at: at,
        }),
        config_snapshots: Vec::new(),
    }
}

fn count(connection: &Connection, table: &'static str) -> AppResult<i64> {
    let sql = match table {
        "events" => "SELECT COUNT(*) FROM events",
        "entities" => "SELECT COUNT(*) FROM entities",
        "relationships" => "SELECT COUNT(*) FROM entity_relationships",
        "baselines" => "SELECT COUNT(*) FROM collector_baselines",
        _ => panic!("unsupported test table"),
    };
    connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(|source| DbError::Sqlite {
            operation: "count atomic runtime residue",
            source,
        })
        .map_err(Into::into)
}

fn residue(db: &Db) -> (i64, i64, i64, i64) {
    db.read(|connection| {
        Ok((
            count(connection, "events")?,
            count(connection, "entities")?,
            count(connection, "relationships")?,
            count(connection, "baselines")?,
        ))
    })
    .unwrap()
}

#[derive(Debug)]
struct ManualClock {
    base: Instant,
    elapsed_ms: AtomicU64,
}

impl ManualClock {
    fn new() -> Self {
        Self {
            base: Instant::now(),
            elapsed_ms: AtomicU64::new(0),
        }
    }

    fn advance(&self, duration: Duration) {
        self.elapsed_ms.fetch_add(
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
            Ordering::AcqRel,
        );
    }
}

impl RuntimeClock for ManualClock {
    fn now(&self) -> Instant {
        self.base + Duration::from_millis(self.elapsed_ms.load(Ordering::Acquire))
    }

    fn now_ms(&self) -> i64 {
        i64::try_from(self.elapsed_ms.load(Ordering::Acquire)).unwrap_or(i64::MAX)
    }
}

struct FixedBackoff(Duration);

impl BackoffPolicy for FixedBackoff {
    fn delay(&self, _consecutive_failures: u64) -> Duration {
        self.0
    }
}

type StartFn = dyn Fn(usize, CollectorContext) -> AppResult<()> + Send + Sync;
type SnapshotFn = dyn Fn(usize, SnapshotContext) -> AppResult<CollectorSnapshot> + Send + Sync;
type SuccessFn = dyn Fn(usize, CommitToken, &CollectorCommitReceipt) -> AppResult<()> + Send + Sync;
type FailureFn = dyn Fn(usize, CommitToken) -> AppResult<()> + Send + Sync;
type StopFn = dyn Fn(usize) + Send + Sync;

#[derive(Clone)]
struct Script {
    start: Arc<StartFn>,
    snapshot: Arc<SnapshotFn>,
    success: Arc<SuccessFn>,
    failure: Arc<FailureFn>,
    stop: Arc<StopFn>,
}

impl Script {
    fn new(
        snapshot: impl Fn(usize, SnapshotContext) -> AppResult<CollectorSnapshot>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            start: Arc::new(|_, _| Ok(())),
            snapshot: Arc::new(snapshot),
            success: Arc::new(|_, _, _| Ok(())),
            failure: Arc::new(|_, _| Ok(())),
            stop: Arc::new(|_| {}),
        }
    }
}

struct ScriptedCollector {
    instance: usize,
    script: Script,
}

impl Collector for ScriptedCollector {
    fn id(&self) -> &'static str {
        COLLECTOR_ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        descriptor()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        (self.script.start)(self.instance, context)
    }

    fn stop(&mut self) {
        (self.script.stop)(self.instance);
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Err(AppError::InvalidInput {
            what: "atomic runtime test",
            reason: "snapshot_batch should be used".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        (self.script.snapshot)(self.instance, context)
    }

    fn commit_succeeded(
        &mut self,
        token: CommitToken,
        receipt: &CollectorCommitReceipt,
    ) -> AppResult<()> {
        (self.script.success)(self.instance, token, receipt)
    }

    fn commit_failed(&mut self, token: CommitToken) -> AppResult<()> {
        (self.script.failure)(self.instance, token)
    }
}

fn scripted_registry(script: Script, factories: Arc<AtomicUsize>) -> CollectorRegistry {
    CollectorRegistry::new(vec![RegistryEntry::active(descriptor(), move || {
        let instance = factories.fetch_add(1, Ordering::AcqRel);
        Box::new(ScriptedCollector {
            instance,
            script: script.clone(),
        })
    })])
}

fn build_supervisor(
    script: Script,
    db: Arc<Db>,
    clock: Arc<ManualClock>,
    backoff: Duration,
) -> (CollectorSupervisor, Arc<AtomicUsize>) {
    let factories = Arc::new(AtomicUsize::new(0));
    let supervisor = CollectorSupervisor::with_runtime(
        scripted_registry(script, Arc::clone(&factories)),
        CollectorRuntimeConfig {
            db,
            monitor: Arc::new(Monitoring::running()),
            query_busy: QueryBusy::default(),
            enabled: BTreeMap::from([(COLLECTOR_ID.to_owned(), true)]),
            profile: MonitoringProfile::Balanced,
            clock,
            backoff: Arc::new(FixedBackoff(backoff)),
        },
    );
    (supervisor, factories)
}

fn wait_for(label: &str, predicate: impl Fn() -> bool) {
    for _ in 0..400 {
        if predicate() {
            return;
        }
        std::thread::yield_now();
    }
    for _ in 0..200 {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("{label} did not become true");
}

fn block_writer(db: &Arc<Db>) -> (SyncSender<()>, Receiver<()>, JoinHandle<AppResult<()>>) {
    let (entered_tx, entered_rx) = sync_channel(1);
    let (release_tx, release_rx) = sync_channel(1);
    let blocked = Arc::clone(db);
    let thread = std::thread::spawn(move || {
        blocked.write(move |_| {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(())
        })
    });
    (release_tx, entered_rx, thread)
}

struct AdmissionSink {
    admitted: SyncSender<String>,
    panic_after_signal: bool,
}

impl RuntimeStatusSink for AdmissionSink {
    fn collector_changed(&self, _collector_id: &str) {}

    fn monitoring_changed(&self, _monitoring: MonitoringDto) {}

    fn collector_commit_admitted(&self, collector_id: &str) {
        self.admitted.send(collector_id.to_owned()).unwrap();
        assert!(
            !self.panic_after_signal,
            "injected admission observer panic"
        );
    }
}

#[test]
fn durable_rows_exist_before_success_hook_promotes_staged_state() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-success",
        ))
        .unwrap(),
    );
    let state = Arc::new(Mutex::new(AckedState::new("old".to_owned())));
    let snapshot_state = Arc::clone(&state);
    let mut script = Script::new(move |_, context| {
        lock(&snapshot_state).stage(context.token(), "new".to_owned())?;
        Ok(atomic_with_event(context.token(), "server", AT, true))
    });
    let success_state = Arc::clone(&state);
    let (entered_tx, entered_rx) = sync_channel(1);
    let (release_tx, release_rx) = sync_channel(1);
    let release_rx = Arc::new(Mutex::new(release_rx));
    let hook_release = Arc::clone(&release_rx);
    let event_id = Arc::new(AtomicU64::new(0));
    let hook_event_id = Arc::clone(&event_id);
    script.success = Arc::new(move |_, token, receipt| {
        hook_event_id.store(
            u64::try_from(receipt.event_ids[0]).unwrap(),
            Ordering::Release,
        );
        entered_tx.send(()).unwrap();
        lock(&hook_release).recv().unwrap();
        lock(&success_state).ack(token)
    });

    let (supervisor, _) = build_supervisor(
        script,
        Arc::clone(&db),
        Arc::new(ManualClock::new()),
        Duration::from_secs(60),
    );
    supervisor.spawn_all().unwrap();
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    assert_eq!(lock(&state).committed(), "old");
    assert_eq!(residue(&db), (1, 2, 1, 1));
    let stored = db
        .read(|connection| {
            event_repository::by_id(
                connection,
                i64::try_from(event_id.load(Ordering::Acquire)).unwrap(),
            )
        })
        .unwrap()
        .unwrap();
    assert_eq!(stored.event.related_entity_ids.len(), 1);
    let baseline = db
        .read(|connection| baseline_repository::load(connection, COLLECTOR_ID, "state"))
        .unwrap()
        .unwrap();
    assert_eq!(baseline.snapshot["marker"], "server");

    release_tx.send(()).unwrap();
    wait_for("state and runtime acknowledgement", || {
        let status = supervisor.list_collector_status().remove(0);
        lock(&state).committed() == "new"
            && status.events_emitted == 1
            && status.consecutive_failures == 0
    });
    let status = supervisor.list_collector_status().remove(0);
    assert_eq!(status.events_emitted, 1);
    assert_eq!(status.consecutive_failures, 0);
    supervisor.stop_all();
}

#[test]
fn queue_rejection_rejects_state_and_counts_rows_and_cycles_separately() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-queue-full",
        ))
        .unwrap(),
    );
    let (release_writer, writer_entered, blocker) = block_writer(&db);
    writer_entered.recv_timeout(Duration::from_secs(1)).unwrap();
    let filler = Sanitizer::new()
        .sanitize(what_changed_lib::testing::synthetic_event(9_999, AT))
        .0;
    loop {
        match db.enqueue_events(vec![filler.clone()]) {
            Ok(()) => {}
            Err(AppError::Db(DbError::QueueFull)) => break,
            Err(error) => panic!("unexpected queue fill error: {error}"),
        }
    }

    let state = Arc::new(Mutex::new(AckedState::new("old".to_owned())));
    let snapshot_state = Arc::clone(&state);
    let mut script = Script::new(move |_, context| {
        lock(&snapshot_state).stage(context.token(), "new".to_owned())?;
        Ok(CollectorSnapshot::Atomic {
            token: context.token(),
            observations: vec![observation("one", AT), observation("two", AT + 1)],
            relationships: Vec::new(),
            baseline: Some(RawBaseline {
                entity_kind: "state".to_owned(),
                snapshot: json!({ "marker": "new" }),
                captured_at: AT + 1,
            }),
            config_snapshots: Vec::new(),
        })
    });
    let failed_state = Arc::clone(&state);
    let (failed_tx, failed_rx) = sync_channel(1);
    script.failure = Arc::new(move |_, token| {
        lock(&failed_state).reject(token)?;
        failed_tx.send(()).unwrap();
        Ok(())
    });
    let (supervisor, _) = build_supervisor(
        script,
        Arc::clone(&db),
        Arc::new(ManualClock::new()),
        Duration::from_secs(60),
    );
    supervisor.spawn_all().unwrap();
    failed_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("queue rejection runtime status", || {
        let status = supervisor.list_collector_status().remove(0);
        status.consecutive_failures == 1 && status.cycle_admission_drops == 1
    });

    assert_eq!(lock(&state).committed(), "old");
    assert_eq!(residue(&db), (0, 0, 0, 0));
    let status = supervisor.list_collector_status().remove(0);
    assert_eq!(status.state, CollectorState::Degraded);
    assert_eq!(status.consecutive_failures, 1);
    assert_eq!(status.queue_drops, 2);
    assert_eq!(status.cycle_admission_drops, 1);
    assert_eq!(status.failed_commit_rows, 0);
    assert_eq!(status.failed_cycles, 0);

    release_writer.send(()).unwrap();
    blocker.join().unwrap().unwrap();
    supervisor.stop_all();
    db.flush().unwrap();
}

#[test]
fn baseline_transaction_failure_rejects_state_and_reports_a_cycle_not_rows() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-rollback",
        ))
        .unwrap(),
    );
    db.write(|connection| {
        connection
            .execute_batch(
                "CREATE TRIGGER injected_atomic_runtime_failure BEFORE INSERT ON collector_baselines \
                 BEGIN SELECT RAISE(ABORT, 'injected atomic runtime failure'); END;",
            )
            .map_err(|source| DbError::Sqlite {
                operation: "install atomic runtime failure",
                source,
            })?;
        Ok(())
    })
    .unwrap();
    let state = Arc::new(Mutex::new(AckedState::new("old".to_owned())));
    let snapshot_state = Arc::clone(&state);
    let mut script = Script::new(move |_, context| {
        lock(&snapshot_state).stage(context.token(), "new".to_owned())?;
        Ok(baseline_only(context.token(), "new", AT))
    });
    let failed_state = Arc::clone(&state);
    let (failed_tx, failed_rx) = sync_channel(1);
    script.failure = Arc::new(move |_, token| {
        lock(&failed_state).reject(token)?;
        failed_tx.send(()).unwrap();
        Ok(())
    });
    let (supervisor, _) = build_supervisor(
        script,
        Arc::clone(&db),
        Arc::new(ManualClock::new()),
        Duration::from_secs(60),
    );
    supervisor.spawn_all().unwrap();
    failed_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("transaction failure runtime status", || {
        let status = supervisor.list_collector_status().remove(0);
        status.consecutive_failures == 1 && status.failed_cycles == 1
    });

    assert_eq!(lock(&state).committed(), "old");
    assert_eq!(residue(&db), (0, 0, 0, 0));
    let status = supervisor.list_collector_status().remove(0);
    assert_eq!(status.state, CollectorState::Degraded);
    assert_eq!(status.queue_drops, 0);
    assert_eq!(status.failed_commit_rows, 0);
    assert_eq!(status.cycle_admission_drops, 0);
    assert_eq!(status.failed_cycles, 1);
    supervisor.stop_all();
}

#[test]
fn disable_during_provider_snapshot_rejects_before_admission_and_fences_stop_callbacks() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-disable-before",
        ))
        .unwrap(),
    );
    let state = Arc::new(Mutex::new(AckedState::new("old".to_owned())));
    let retained = Arc::new(Mutex::new(None::<CollectorContext>));
    let start_retained = Arc::clone(&retained);
    let (entered_tx, entered_rx) = sync_channel(1);
    let (release_tx, release_rx) = sync_channel(1);
    let release_rx = Arc::new(Mutex::new(release_rx));
    let snapshot_release = Arc::clone(&release_rx);
    let snapshot_state = Arc::clone(&state);
    let mut script = Script::new(move |_, context| {
        lock(&snapshot_state).stage(context.token(), "new".to_owned())?;
        entered_tx.send(()).unwrap();
        lock(&snapshot_release).recv().unwrap();
        Ok(baseline_only(context.token(), "new", AT))
    });
    script.start = Arc::new(move |_, context| {
        *lock(&start_retained) = Some(context);
        Ok(())
    });
    let failed_state = Arc::clone(&state);
    let (failed_tx, failed_rx) = sync_channel(1);
    script.failure = Arc::new(move |_, token| {
        lock(&failed_state).reject(token)?;
        failed_tx.send(()).unwrap();
        Ok(())
    });
    let stop_retained = Arc::clone(&retained);
    let (stopped_tx, stopped_rx) = sync_channel(1);
    script.stop = Arc::new(move |_| {
        let context = lock(&stop_retained).clone().unwrap();
        stopped_tx
            .send((
                context.cancel.is_cancelled(),
                context.emit.emit(Vec::new()).is_err(),
                context.wake.request_snapshot().is_err(),
            ))
            .unwrap();
    });

    let (supervisor, _) = build_supervisor(
        script,
        Arc::clone(&db),
        Arc::new(ManualClock::new()),
        Duration::from_secs(60),
    );
    supervisor.spawn_all().unwrap();
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    supervisor.set_enabled(COLLECTOR_ID, false).unwrap();
    release_tx.send(()).unwrap();
    failed_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(
        stopped_rx.recv_timeout(Duration::from_secs(1)).unwrap(),
        (true, true, true),
        "cancellation and instance fences must precede stop"
    );

    assert_eq!(lock(&state).committed(), "old");
    assert_eq!(residue(&db), (0, 0, 0, 0));
    let status = supervisor.list_collector_status().remove(0);
    assert_eq!(status.state, CollectorState::Disabled);
    assert_eq!(status.consecutive_failures, 0);
    assert_eq!(status.cycle_admission_drops, 0);
    assert_eq!(status.failed_cycles, 0);
    supervisor.stop_all();
}

#[test]
fn disable_after_admission_still_commits_once_and_observer_panic_cannot_change_it() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-disable-after",
        ))
        .unwrap(),
    );
    let (release_writer, writer_entered, blocker) = block_writer(&db);
    writer_entered.recv_timeout(Duration::from_secs(1)).unwrap();

    let state = Arc::new(Mutex::new(AckedState::new("old".to_owned())));
    let snapshot_state = Arc::clone(&state);
    let mut script = Script::new(move |_, context| {
        lock(&snapshot_state).stage(context.token(), "new".to_owned())?;
        Ok(atomic_with_event(context.token(), "admitted", AT, true))
    });
    let success_state = Arc::clone(&state);
    let (acked_tx, acked_rx) = sync_channel(1);
    script.success = Arc::new(move |_, token, _| {
        lock(&success_state).ack(token)?;
        acked_tx.send(()).unwrap();
        Ok(())
    });
    let (supervisor, _) = build_supervisor(
        script,
        Arc::clone(&db),
        Arc::new(ManualClock::new()),
        Duration::from_secs(60),
    );
    let supervisor = Arc::new(supervisor);
    let (admitted_tx, admitted_rx) = sync_channel(2);
    supervisor.attach_status_sink(Arc::new(AdmissionSink {
        admitted: admitted_tx,
        panic_after_signal: true,
    }));
    supervisor.spawn_all().unwrap();
    assert_eq!(
        admitted_rx.recv_timeout(Duration::from_secs(1)).unwrap(),
        COLLECTOR_ID
    );

    let setter_supervisor = Arc::clone(&supervisor);
    let (disabled_tx, disabled_rx) = sync_channel(1);
    let setter = std::thread::spawn(move || {
        disabled_tx
            .send(setter_supervisor.set_enabled(COLLECTOR_ID, false))
            .unwrap();
    });
    let disabled = disabled_rx.recv_timeout(Duration::from_millis(500));
    if disabled.is_err() {
        // Release every deliberately blocked resource before failing so the
        // test never strands a writer or lifecycle setter.
        release_writer.send(()).unwrap();
        setter.join().unwrap();
        blocker.join().unwrap().unwrap();
        panic!("disable blocked behind a post-admission database wait");
    }
    disabled.unwrap().unwrap();
    setter.join().unwrap();

    release_writer.send(()).unwrap();
    blocker.join().unwrap().unwrap();
    acked_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("post-admission status acknowledgement", || {
        supervisor.list_collector_status()[0].events_emitted == 1
    });
    assert_eq!(lock(&state).committed(), "new");
    assert_eq!(residue(&db), (1, 2, 1, 1));
    let status = supervisor.list_collector_status().remove(0);
    assert_eq!(status.state, CollectorState::Disabled);
    assert_eq!(status.events_emitted, 1);
    assert_eq!(status.consecutive_failures, 0);
    supervisor.stop_all();
}

#[test]
fn noop_skips_writer_and_ack_hooks() {
    let db =
        Arc::new(Db::open_in(&what_changed_lib::testing::temp_dir("atomic-runtime-noop")).unwrap());
    let snapshots = Arc::new(AtomicUsize::new(0));
    let snapshot_count = Arc::clone(&snapshots);
    let mut script = Script::new(move |_, _| {
        snapshot_count.fetch_add(1, Ordering::AcqRel);
        Ok(CollectorSnapshot::Noop)
    });
    let success_hooks = Arc::new(AtomicUsize::new(0));
    let counted_success = Arc::clone(&success_hooks);
    script.success = Arc::new(move |_, _, _| {
        counted_success.fetch_add(1, Ordering::AcqRel);
        Ok(())
    });
    let failure_hooks = Arc::new(AtomicUsize::new(0));
    let counted_failure = Arc::clone(&failure_hooks);
    script.failure = Arc::new(move |_, _| {
        counted_failure.fetch_add(1, Ordering::AcqRel);
        Ok(())
    });
    let (supervisor, _) = build_supervisor(
        script,
        Arc::clone(&db),
        Arc::new(ManualClock::new()),
        Duration::from_secs(60),
    );
    supervisor.spawn_all().unwrap();
    wait_for("noop completion", || {
        snapshots.load(Ordering::Acquire) == 1
            && supervisor.list_collector_status()[0].last_run_at.is_some()
    });

    assert_eq!(residue(&db), (0, 0, 0, 0));
    assert_eq!(success_hooks.load(Ordering::Acquire), 0);
    assert_eq!(failure_hooks.load(Ordering::Acquire), 0);
    assert_eq!(db.dropped_writes(), 0);
    assert!(db.collector_cycle_admission_drops().is_empty());
    assert!(db.failed_collector_cycles().is_empty());
    supervisor.stop_all();
}

struct LegacyCollector {
    snapshots: Arc<AtomicUsize>,
}

impl Collector for LegacyCollector {
    fn id(&self) -> &'static str {
        COLLECTOR_ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        descriptor()
    }

    fn start(&mut self, _context: CollectorContext) -> AppResult<()> {
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        self.snapshots.fetch_add(1, Ordering::AcqRel);
        Ok(vec![observation("legacy", AT)])
    }
}

#[test]
fn default_snapshot_batch_preserves_legacy_async_delivery() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-legacy",
        ))
        .unwrap(),
    );
    let snapshots = Arc::new(AtomicUsize::new(0));
    let collector_snapshots = Arc::clone(&snapshots);
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor(), move || {
        Box::new(LegacyCollector {
            snapshots: Arc::clone(&collector_snapshots),
        })
    })]);
    let supervisor = CollectorSupervisor::with_runtime(
        registry,
        CollectorRuntimeConfig {
            db: Arc::clone(&db),
            monitor: Arc::new(Monitoring::running()),
            query_busy: QueryBusy::default(),
            enabled: BTreeMap::from([(COLLECTOR_ID.to_owned(), true)]),
            profile: MonitoringProfile::Balanced,
            clock: Arc::new(ManualClock::new()),
            backoff: Arc::new(FixedBackoff(Duration::from_secs(60))),
        },
    );
    supervisor.spawn_all().unwrap();
    wait_for("legacy enqueue", || {
        snapshots.load(Ordering::Acquire) == 1
            && supervisor.list_collector_status()[0].events_emitted == 1
    });
    db.flush().unwrap();

    assert_eq!(residue(&db), (1, 1, 0, 0));
    assert!(db.collector_cycle_admission_drops().is_empty());
    assert!(db.failed_collector_cycles().is_empty());
    supervisor.stop_all();
}

#[test]
fn returned_token_mismatch_rejects_only_current_candidate_without_writing() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-token-mismatch",
        ))
        .unwrap(),
    );
    let state = Arc::new(Mutex::new(AckedState::new("old".to_owned())));
    let previous_token = Arc::new(Mutex::new(None::<CommitToken>));
    let snapshots = Arc::new(AtomicUsize::new(0));
    let snapshot_state = Arc::clone(&state);
    let snapshot_previous = Arc::clone(&previous_token);
    let snapshot_count = Arc::clone(&snapshots);
    let mut script = Script::new(move |_, context| {
        let cycle = snapshot_count.fetch_add(1, Ordering::AcqRel);
        if cycle == 0 {
            lock(&snapshot_state).stage(context.token(), "first".to_owned())?;
            *lock(&snapshot_previous) = Some(context.token());
            Ok(baseline_only(context.token(), "first", AT))
        } else {
            lock(&snapshot_state).stage(context.token(), "second".to_owned())?;
            let stale = lock(&snapshot_previous).unwrap();
            Ok(baseline_only(stale, "second", AT + 1))
        }
    });
    let retained = Arc::new(Mutex::new(None::<CollectorContext>));
    let start_retained = Arc::clone(&retained);
    script.start = Arc::new(move |_, context| {
        *lock(&start_retained) = Some(context);
        Ok(())
    });
    let success_state = Arc::clone(&state);
    let (success_tx, success_rx) = sync_channel(1);
    script.success = Arc::new(move |_, token, _| {
        lock(&success_state).ack(token)?;
        success_tx.send(()).unwrap();
        Ok(())
    });
    let failure_state = Arc::clone(&state);
    let (failure_tx, failure_rx) = sync_channel(1);
    script.failure = Arc::new(move |_, token| {
        lock(&failure_state).reject(token)?;
        failure_tx.send(()).unwrap();
        Ok(())
    });
    let (supervisor, _) = build_supervisor(
        script,
        Arc::clone(&db),
        Arc::new(ManualClock::new()),
        Duration::from_secs(60),
    );
    supervisor.spawn_all().unwrap();
    success_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    lock(&retained)
        .as_ref()
        .unwrap()
        .wake
        .request_snapshot()
        .unwrap();
    failure_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    wait_for("token mismatch runtime status", || {
        supervisor.list_collector_status()[0].consecutive_failures == 1
    });

    assert_eq!(lock(&state).committed(), "first");
    assert_eq!(residue(&db), (0, 0, 0, 1));
    let baseline = db
        .read(|connection| baseline_repository::load(connection, COLLECTOR_ID, "state"))
        .unwrap()
        .unwrap();
    assert_eq!(baseline.snapshot["marker"], "first");
    let status = supervisor.list_collector_status().remove(0);
    assert_eq!(status.consecutive_failures, 1);
    assert_eq!(status.events_emitted, 0);
    assert_eq!(status.cycle_admission_drops, 0);
    assert_eq!(status.failed_cycles, 0);
    supervisor.stop_all();
}

#[test]
fn notification_storm_during_atomic_wait_coalesces_to_one_trailing_cycle() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-wake-storm",
        ))
        .unwrap(),
    );
    let (release_writer, writer_entered, blocker) = block_writer(&db);
    writer_entered.recv_timeout(Duration::from_secs(1)).unwrap();
    let retained = Arc::new(Mutex::new(None::<CollectorContext>));
    let start_retained = Arc::clone(&retained);
    let snapshots = Arc::new(AtomicUsize::new(0));
    let snapshot_count = Arc::clone(&snapshots);
    let mut script = Script::new(move |_, context| {
        let cycle = snapshot_count.fetch_add(1, Ordering::AcqRel);
        Ok(atomic_with_event(
            context.token(),
            &format!("storm-{cycle}"),
            AT + i64::try_from(cycle).unwrap(),
            false,
        ))
    });
    script.start = Arc::new(move |_, context| {
        *lock(&start_retained) = Some(context);
        Ok(())
    });
    let (success_tx, success_rx) = sync_channel(4);
    script.success = Arc::new(move |_, _, _| {
        success_tx.send(()).unwrap();
        Ok(())
    });
    let (supervisor, _) = build_supervisor(
        script,
        Arc::clone(&db),
        Arc::new(ManualClock::new()),
        Duration::from_secs(60),
    );
    let (admitted_tx, admitted_rx) = sync_channel(4);
    supervisor.attach_status_sink(Arc::new(AdmissionSink {
        admitted: admitted_tx,
        panic_after_signal: false,
    }));
    supervisor.spawn_all().unwrap();
    admitted_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let wake = lock(&retained).clone().unwrap().wake;
    for _ in 0..100 {
        wake.request_snapshot().unwrap();
    }

    release_writer.send(()).unwrap();
    blocker.join().unwrap().unwrap();
    success_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    admitted_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    success_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(snapshots.load(Ordering::Acquire), 2);
    assert!(success_rx.recv_timeout(Duration::from_millis(100)).is_err());
    db.flush().unwrap();
    assert_eq!(residue(&db), (2, 3, 2, 0));
    assert_eq!(supervisor.list_collector_status()[0].events_emitted, 2);
    supervisor.stop_all();
}

#[test]
fn durable_hook_panic_never_resubmits_and_replacement_hydrates_durable_baseline() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-hook-panic",
        ))
        .unwrap(),
    );
    let clock = Arc::new(ManualClock::new());
    let (contexts_tx, contexts_rx) = sync_channel(4);
    let replacement_snapshots = Arc::new(AtomicUsize::new(0));
    let replacement_count = Arc::clone(&replacement_snapshots);
    let mut script = Script::new(move |instance, context| {
        if instance == 0 {
            Ok(atomic_with_event(context.token(), "durable", AT, true))
        } else {
            replacement_count.fetch_add(1, Ordering::AcqRel);
            Ok(CollectorSnapshot::Noop)
        }
    });
    script.start = Arc::new(move |instance, context| {
        let loaded = context
            .state
            .baseline("state")?
            .and_then(|baseline| baseline.snapshot["marker"].as_str().map(str::to_owned));
        contexts_tx.send((instance, context, loaded)).unwrap();
        Ok(())
    });
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let counted_stops = Arc::clone(&stop_calls);
    script.stop = Arc::new(move |_| {
        counted_stops.fetch_add(1, Ordering::AcqRel);
    });
    script.success = Arc::new(move |instance, _, _| {
        assert_ne!(instance, 0, "injected durable acknowledgement divergence");
        Ok(())
    });

    let (supervisor, factories) = build_supervisor(
        script,
        Arc::clone(&db),
        Arc::clone(&clock),
        Duration::from_secs(1),
    );
    supervisor.spawn_all().unwrap();
    let (first_instance, first_context, first_loaded) =
        contexts_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(first_instance, 0);
    assert_eq!(first_loaded, None);
    wait_for("durable hook failure status", || {
        let status = supervisor.list_collector_status().remove(0);
        status.consecutive_failures == 1
            && status.state == CollectorState::Degraded
            && status.events_emitted == 1
    });
    assert_eq!(
        supervisor.list_collector_status()[0].state,
        CollectorState::Degraded
    );
    assert_eq!(residue(&db), (1, 2, 1, 1));
    assert_eq!(supervisor.list_collector_status()[0].events_emitted, 1);
    assert_eq!(stop_calls.load(Ordering::Acquire), 0);
    assert!(first_context.emit.emit(Vec::new()).is_err());
    assert!(first_context.wake.request_snapshot().is_err());

    clock.advance(Duration::from_secs(1));
    supervisor.set_profile(MonitoringProfile::Minimal);
    let (replacement_instance, replacement_context, replacement_loaded) =
        contexts_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(replacement_instance, 1);
    assert_eq!(replacement_loaded.as_deref(), Some("durable"));
    wait_for("replacement noop", || {
        replacement_snapshots.load(Ordering::Acquire) == 1
    });
    replacement_context.wake.request_snapshot().unwrap();
    wait_for("replacement targeted wake", || {
        replacement_snapshots.load(Ordering::Acquire) == 2
    });

    assert_eq!(factories.load(Ordering::Acquire), 2);
    assert_eq!(residue(&db), (1, 2, 1, 1));
    assert_eq!(supervisor.list_collector_status()[0].events_emitted, 1);
    supervisor.stop_all();
    assert_eq!(stop_calls.load(Ordering::Acquire), 1);
}

struct StartProbeCollector {
    instance: usize,
    starts: SyncSender<usize>,
}

impl Collector for StartProbeCollector {
    fn id(&self) -> &'static str {
        COLLECTOR_ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        descriptor()
    }

    fn start(&mut self, _context: CollectorContext) -> AppResult<()> {
        self.starts.send(self.instance).unwrap();
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Ok(Vec::new())
    }

    fn snapshot_batch(&mut self, _context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        Ok(CollectorSnapshot::Noop)
    }
}

#[test]
fn lifecycle_transition_while_factory_is_blocked_never_starts_the_stale_instance() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-pre-start-fence",
        ))
        .unwrap(),
    );
    let factories = Arc::new(AtomicUsize::new(0));
    let counted_factories = Arc::clone(&factories);
    let (factory_entered_tx, factory_entered_rx) = sync_channel(1);
    let (release_factory_tx, release_factory_rx) = sync_channel(1);
    let release_factory_rx = Arc::new(Mutex::new(release_factory_rx));
    let factory_release = Arc::clone(&release_factory_rx);
    let (starts_tx, starts_rx) = sync_channel(4);
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor(), move || {
        let instance = counted_factories.fetch_add(1, Ordering::AcqRel);
        if instance == 0 {
            factory_entered_tx.send(()).unwrap();
            lock(&factory_release).recv().unwrap();
        }
        Box::new(StartProbeCollector {
            instance,
            starts: starts_tx.clone(),
        })
    })]);
    let supervisor = CollectorSupervisor::with_runtime(
        registry,
        CollectorRuntimeConfig {
            db,
            monitor: Arc::new(Monitoring::running()),
            query_busy: QueryBusy::default(),
            enabled: BTreeMap::from([(COLLECTOR_ID.to_owned(), true)]),
            profile: MonitoringProfile::Balanced,
            clock: Arc::new(ManualClock::new()),
            backoff: Arc::new(FixedBackoff(Duration::from_secs(60))),
        },
    );
    supervisor.spawn_all().unwrap();
    factory_entered_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
    supervisor.set_monitoring_paused(true, None);
    supervisor.set_monitoring_paused(false, None);
    release_factory_tx.send(()).unwrap();

    assert_eq!(
        starts_rx.recv_timeout(Duration::from_secs(1)).unwrap(),
        1,
        "the instance created under the old monitoring generation must be dropped unstarted"
    );
    assert_eq!(factories.load(Ordering::Acquire), 2);
    supervisor.stop_all();
}

struct DropPanicCollector {
    instance: usize,
    starts: SyncSender<usize>,
    snapshots: Arc<AtomicUsize>,
    stops: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
    uncancelled_stop: Arc<AtomicUsize>,
    uncancelled_instance: Arc<AtomicUsize>,
    cancel: Option<what_changed_lib::collectors::CancelToken>,
}

impl Collector for DropPanicCollector {
    fn id(&self) -> &'static str {
        COLLECTOR_ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        descriptor()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        self.cancel = Some(context.cancel);
        self.starts.send(self.instance).unwrap();
        Ok(())
    }

    fn stop(&mut self) {
        let cancelled = self
            .cancel
            .as_ref()
            .is_some_and(what_changed_lib::collectors::CancelToken::is_cancelled);
        if !cancelled {
            self.uncancelled_stop.fetch_add(1, Ordering::AcqRel);
            self.uncancelled_instance
                .store(self.instance + 1, Ordering::Release);
        }
        self.stops.fetch_add(1, Ordering::AcqRel);
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Ok(Vec::new())
    }

    fn snapshot_batch(&mut self, _context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        self.snapshots.fetch_add(1, Ordering::AcqRel);
        Ok(CollectorSnapshot::Noop)
    }
}

impl Drop for DropPanicCollector {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::AcqRel);
        panic!("injected collector destructor panic");
    }
}

#[test]
fn factory_and_destructor_panics_are_contained_and_worker_recreates() {
    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-contained-panics",
        ))
        .unwrap(),
    );
    let clock = Arc::new(ManualClock::new());
    let factories = Arc::new(AtomicUsize::new(0));
    let counted_factories = Arc::clone(&factories);
    let snapshots = Arc::new(AtomicUsize::new(0));
    let factory_snapshots = Arc::clone(&snapshots);
    let stops = Arc::new(AtomicUsize::new(0));
    let factory_stops = Arc::clone(&stops);
    let drops = Arc::new(AtomicUsize::new(0));
    let factory_drops = Arc::clone(&drops);
    let uncancelled_stop = Arc::new(AtomicUsize::new(0));
    let factory_uncancelled = Arc::clone(&uncancelled_stop);
    let uncancelled_instance = Arc::new(AtomicUsize::new(0));
    let factory_uncancelled_instance = Arc::clone(&uncancelled_instance);
    let (starts_tx, starts_rx) = sync_channel(4);
    let registry = CollectorRegistry::new(vec![RegistryEntry::active(descriptor(), move || {
        let instance = counted_factories.fetch_add(1, Ordering::AcqRel);
        assert_ne!(instance, 0, "injected collector factory panic");
        Box::new(DropPanicCollector {
            instance,
            starts: starts_tx.clone(),
            snapshots: Arc::clone(&factory_snapshots),
            stops: Arc::clone(&factory_stops),
            drops: Arc::clone(&factory_drops),
            uncancelled_stop: Arc::clone(&factory_uncancelled),
            uncancelled_instance: Arc::clone(&factory_uncancelled_instance),
            cancel: None,
        })
    })]);
    let supervisor = CollectorSupervisor::with_runtime(
        registry,
        CollectorRuntimeConfig {
            db,
            monitor: Arc::new(Monitoring::running()),
            query_busy: QueryBusy::default(),
            enabled: BTreeMap::from([(COLLECTOR_ID.to_owned(), true)]),
            profile: MonitoringProfile::Balanced,
            clock: Arc::clone(&clock) as Arc<dyn RuntimeClock>,
            backoff: Arc::new(FixedBackoff(Duration::from_secs(1))),
        },
    );
    supervisor.spawn_all().unwrap();
    wait_for("factory panic status", || {
        supervisor.list_collector_status()[0].consecutive_failures == 1
    });

    clock.advance(Duration::from_secs(1));
    supervisor.set_profile(MonitoringProfile::Minimal);
    let first_started_instance = starts_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(
        first_started_instance >= 1,
        "a profile-raced unstarted instance may be fenced before start"
    );
    wait_for("first surviving instance snapshot", || {
        snapshots.load(Ordering::Acquire) == 1
    });
    let drops_before_disable = drops.load(Ordering::Acquire);
    supervisor.set_enabled(COLLECTOR_ID, false).unwrap();
    wait_for("contained destructor panic", || {
        drops.load(Ordering::Acquire) == drops_before_disable + 1
    });
    assert_eq!(stops.load(Ordering::Acquire), 1);
    assert_eq!(
        uncancelled_stop.load(Ordering::Acquire),
        0,
        "instance {} stopped without cancellation",
        uncancelled_instance
            .load(Ordering::Acquire)
            .saturating_sub(1),
    );

    supervisor.set_enabled(COLLECTOR_ID, true).unwrap();
    assert!(
        starts_rx.recv_timeout(Duration::from_secs(1)).unwrap() > first_started_instance,
        "a lifecycle-raced unstarted instance may be fenced and skipped"
    );
    wait_for("worker survived destructor panic", || {
        snapshots.load(Ordering::Acquire) == 2
    });
    supervisor.stop_all();
    assert_eq!(stops.load(Ordering::Acquire), 2);
    assert_eq!(
        drops.load(Ordering::Acquire),
        factories.load(Ordering::Acquire) - 1,
        "every successfully constructed instance is dropped inside containment"
    );
    assert_eq!(
        uncancelled_stop.load(Ordering::Acquire),
        0,
        "instance {} stopped without cancellation",
        uncancelled_instance
            .load(Ordering::Acquire)
            .saturating_sub(1),
    );
}

struct ShutdownSentinelCollector {
    cancel: Arc<Mutex<Option<what_changed_lib::collectors::CancelToken>>>,
    started: SyncSender<()>,
}

impl Collector for ShutdownSentinelCollector {
    fn id(&self) -> &'static str {
        "shutdown-sentinel"
    }

    fn descriptor(&self) -> CollectorDescriptor {
        shutdown_descriptor(self.id())
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        *lock(&self.cancel) = Some(context.cancel);
        self.started.send(()).unwrap();
        Ok(())
    }

    fn stop(&mut self) {}

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Ok(Vec::new())
    }

    fn snapshot_batch(&mut self, _context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        Ok(CollectorSnapshot::Noop)
    }
}

struct ShutdownTargetCollector {
    sentinel_cancel: Arc<Mutex<Option<what_changed_lib::collectors::CancelToken>>>,
    own_cancel: Option<what_changed_lib::collectors::CancelToken>,
    snapshot_entered: Option<SyncSender<()>>,
    stop_cancelled: SyncSender<bool>,
}

impl Collector for ShutdownTargetCollector {
    fn id(&self) -> &'static str {
        "shutdown-target"
    }

    fn descriptor(&self) -> CollectorDescriptor {
        shutdown_descriptor(self.id())
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        self.own_cancel = Some(context.cancel);
        Ok(())
    }

    fn stop(&mut self) {
        self.stop_cancelled
            .send(
                self.own_cancel
                    .as_ref()
                    .is_some_and(what_changed_lib::collectors::CancelToken::is_cancelled),
            )
            .unwrap();
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Ok(Vec::new())
    }

    fn snapshot_batch(&mut self, _context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        if let Some(entered) = self.snapshot_entered.take() {
            entered.send(()).unwrap();
        }
        loop {
            let sentinel_was_cancelled = lock(&self.sentinel_cancel)
                .as_ref()
                .is_some_and(what_changed_lib::collectors::CancelToken::is_cancelled);
            if sentinel_was_cancelled {
                return Ok(CollectorSnapshot::Noop);
            }
            std::thread::yield_now();
        }
    }
}

fn shutdown_descriptor(id: &'static str) -> CollectorDescriptor {
    CollectorDescriptor {
        id,
        display_name: "Shutdown ordering test",
        what_it_watches: "runtime shutdown ordering",
        why_it_matters: "proves cancellation precedes collector stop",
        cost_hint: CollectorCost::Cheap,
        cadence: CollectorCadence::Notify,
        requires_admin: false,
    }
}

#[test]
fn shutdown_cancels_the_active_instance_token_before_stop() {
    const FILLER_ENTRIES: usize = 20_000;

    let sentinel_cancel = Arc::new(Mutex::new(None));
    let sentinel_factory_cancel = Arc::clone(&sentinel_cancel);
    let (sentinel_started_tx, sentinel_started_rx) = sync_channel(1);
    let (snapshot_entered_tx, snapshot_entered_rx) = sync_channel(1);
    let (stop_cancelled_tx, stop_cancelled_rx) = sync_channel(1);
    let target_sentinel_cancel = Arc::clone(&sentinel_cancel);

    let mut entries = Vec::with_capacity(FILLER_ENTRIES + 2);
    entries.push(RegistryEntry::active(
        shutdown_descriptor("shutdown-sentinel"),
        move || {
            Box::new(ShutdownSentinelCollector {
                cancel: Arc::clone(&sentinel_factory_cancel),
                started: sentinel_started_tx.clone(),
            })
        },
    ));
    for index in 0..FILLER_ENTRIES {
        let id: &'static str = Box::leak(format!("shutdown-filler-{index}").into_boxed_str());
        entries.push(RegistryEntry::unavailable(shutdown_descriptor(id)));
    }
    entries.push(RegistryEntry::active(
        shutdown_descriptor("shutdown-target"),
        move || {
            Box::new(ShutdownTargetCollector {
                sentinel_cancel: Arc::clone(&target_sentinel_cancel),
                own_cancel: None,
                snapshot_entered: Some(snapshot_entered_tx.clone()),
                stop_cancelled: stop_cancelled_tx.clone(),
            })
        },
    ));

    let db = Arc::new(
        Db::open_in(&what_changed_lib::testing::temp_dir(
            "atomic-runtime-shutdown-cancel-order",
        ))
        .unwrap(),
    );
    let supervisor = CollectorSupervisor::with_runtime(
        CollectorRegistry::new(entries),
        CollectorRuntimeConfig {
            db,
            monitor: Arc::new(Monitoring::running()),
            query_busy: QueryBusy::default(),
            enabled: BTreeMap::new(),
            profile: MonitoringProfile::Balanced,
            clock: Arc::new(ManualClock::new()),
            backoff: Arc::new(FixedBackoff(Duration::from_secs(1))),
        },
    );
    supervisor.spawn_all().unwrap();
    sentinel_started_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
    snapshot_entered_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap();

    supervisor.stop_all();

    assert!(
        stop_cancelled_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap(),
        "the active instance must observe its own cancellation token before stop"
    );
}
