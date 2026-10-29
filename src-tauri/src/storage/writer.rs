//! The one writer.
//!
//! SQLite allows a single writer at a time, so the product has exactly one:
//! a dedicated thread that owns the read-write connection and drains a
//! bounded queue. Event-only commands are coalesced — up to a short window or
//! a batch size, whichever comes first — while each stateful collector cycle
//! stays one all-or-nothing transaction. When the queue is full the command is
//! dropped and counted rather than blocking a collector; a collector that
//! cannot keep up must not stall the thing it is watching.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle, ThreadId};
use std::time::{Duration, Instant};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::collector_commit;
use super::connection::is_corruption;
use super::db::lock;
use super::event_repository;
use super::{AtomicCollectorCommit, CollectorCommitReceipt};
use crate::domain::{Category, Importance, NormalizedEvent};
use crate::error::{AppError, AppResult, DbError};
use crate::privacy::Sanitized;

/// Queue depth. A burst larger than this within one batching window is an
/// event flood, and the response to a flood is to drop, count and say so.
const QUEUE_CAPACITY: usize = 4096;

/// Longest an event waits in memory before it is committed.
const COALESCE_WINDOW: Duration = Duration::from_millis(250);

/// Events that trigger a commit before the window closes.
const COALESCE_EVENTS: usize = 200;

/// How often the idle loop wakes to notice a shutdown.
const IDLE_POLL: Duration = Duration::from_secs(1);

/// Minimum gap between two "queue full" warnings.
const THROTTLE_NOTICE: Duration = Duration::from_secs(10);

thread_local! {
    static ON_WRITER_THREAD: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn on_writer_thread() -> bool {
    ON_WRITER_THREAD.get()
}

struct WriterThreadMarker;

impl WriterThreadMarker {
    fn enter() -> Self {
        ON_WRITER_THREAD.set(true);
        Self
    }
}

impl Drop for WriterThreadMarker {
    fn drop(&mut self) {
        ON_WRITER_THREAD.set(false);
    }
}

/// What the writer tells subscribers after a commit. Deliberately carries no
/// event bodies: the frontend decides whether to fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
pub struct BatchNotice {
    /// Rows committed.
    pub count: usize,
    /// Highest id committed.
    pub newest_id: i64,
    /// Distinct categories in the batch.
    pub categories: Vec<Category>,
    /// The most important row in the batch.
    pub highest_importance: Importance,
    /// The newest bounded set of committed ids. The event bus uses these to
    /// evaluate notification policy without putting event bodies on the IPC
    /// payload. Older ids are intentionally omitted during large bursts.
    #[serde(default)]
    pub event_ids: Vec<i64>,
}

/// A collector commit admitted to the bounded writer queue.
///
/// Admission and acknowledgement are deliberately separate: a collector
/// runtime can release its lifecycle fence immediately after `submit`, then
/// wait for SQLite without preventing pause, disable, or shutdown.
#[must_use = "an admitted collector commit must be awaited with wait()"]
pub struct CollectorCommitTicket {
    result: Receiver<AppResult<CollectorCommitReceipt>>,
    writer_thread_id: ThreadId,
}

impl CollectorCommitTicket {
    /// Wait until the writer atomically commits or rolls back the cycle.
    pub fn wait(self) -> AppResult<CollectorCommitReceipt> {
        reject_writer_thread_wait(self.writer_thread_id)?;
        self.result.recv().map_err(|_| DbError::WriterGone)?
    }
}

/// A job the writer runs on its connection.
type Job = Box<dyn FnOnce(&Connection) + Send + 'static>;

enum Command {
    Events(Vec<Sanitized<NormalizedEvent>>),
    CollectorCommit(
        AtomicCollectorCommit,
        SyncSender<AppResult<CollectorCommitReceipt>>,
    ),
    Job(Job),
    Flush(SyncSender<()>),
    Shutdown,
}

/// Handle to the writer thread. Dropping it stops the thread after the queue
/// is drained.
pub struct WriterHandle {
    tx: SyncSender<Command>,
    queue_depth: Arc<AtomicU64>,
    dropped: Arc<AtomicU64>,
    last_notice: Mutex<Option<Instant>>,
    corrupt: Arc<Mutex<Option<String>>>,
    failed_commits: Arc<Mutex<BTreeMap<String, u64>>>,
    cycle_admission_drops: Arc<Mutex<BTreeMap<String, u64>>>,
    failed_cycles: Arc<Mutex<BTreeMap<String, u64>>>,
    thread_id: ThreadId,
    thread: Option<JoinHandle<()>>,
}

impl WriterHandle {
    /// Move `conn` onto a new thread and start draining.
    pub fn spawn(
        conn: Connection,
        subscribers: Arc<Mutex<Vec<SyncSender<BatchNotice>>>>,
        dropped: Arc<AtomicU64>,
        failed_commits: Arc<Mutex<BTreeMap<String, u64>>>,
        cycle_admission_drops: Arc<Mutex<BTreeMap<String, u64>>>,
        failed_cycles: Arc<Mutex<BTreeMap<String, u64>>>,
    ) -> AppResult<Self> {
        Self::spawn_with_capacity(
            conn,
            subscribers,
            dropped,
            failed_commits,
            cycle_admission_drops,
            failed_cycles,
            QUEUE_CAPACITY,
        )
    }

    fn spawn_with_capacity(
        conn: Connection,
        subscribers: Arc<Mutex<Vec<SyncSender<BatchNotice>>>>,
        dropped: Arc<AtomicU64>,
        failed_commits: Arc<Mutex<BTreeMap<String, u64>>>,
        cycle_admission_drops: Arc<Mutex<BTreeMap<String, u64>>>,
        failed_cycles: Arc<Mutex<BTreeMap<String, u64>>>,
        capacity: usize,
    ) -> AppResult<Self> {
        let (tx, rx) = sync_channel(capacity);
        let queue_depth = Arc::new(AtomicU64::new(0));
        let corrupt = Arc::new(Mutex::new(None));
        let corrupt_flag = Arc::clone(&corrupt);
        let queue_depth_for_thread = Arc::clone(&queue_depth);
        let thread = std::thread::Builder::new()
            .name("db-writer".to_owned())
            .spawn({
                let failed = Arc::clone(&failed_commits);
                let failed_cycle_counts = Arc::clone(&failed_cycles);
                move || {
                    run_loop(
                        conn,
                        rx,
                        subscribers,
                        corrupt_flag,
                        failed,
                        failed_cycle_counts,
                        queue_depth_for_thread,
                    )
                }
            })
            .map_err(|source| AppError::File {
                operation: "start the writer thread for",
                path: std::path::PathBuf::from("events.db"),
                source,
            })?;
        let thread_id = thread.thread().id();
        Ok(Self {
            tx,
            queue_depth,
            dropped,
            last_notice: Mutex::new(None),
            corrupt,
            failed_commits,
            cycle_admission_drops,
            failed_cycles,
            thread_id,
            thread: Some(thread),
        })
    }

    /// Queue events without waiting. Fails, counts and warns when full.
    pub fn enqueue(&self, events: Vec<Sanitized<NormalizedEvent>>) -> AppResult<()> {
        if events.is_empty() {
            return Ok(());
        }
        let count = events.len() as u64;
        self.queue_depth.fetch_add(1, Ordering::Relaxed);
        match self.tx.try_send(Command::Events(events)) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => {
                decrement_queue_depth(&self.queue_depth);
                self.dropped.fetch_add(count, Ordering::Relaxed);
                self.throttled_warning();
                Err(DbError::QueueFull.into())
            }
            Err(TrySendError::Disconnected(_)) => {
                decrement_queue_depth(&self.queue_depth);
                Err(DbError::WriterGone.into())
            }
        }
    }

    /// Insert now and wait for the ids.
    pub fn insert_blocking(&self, events: Vec<Sanitized<NormalizedEvent>>) -> AppResult<Vec<i64>> {
        self.run(move |conn| event_repository::insert_batch(conn, &events))
    }

    /// Devtools-only blocking insert that does not rewrite pre-existing
    /// entity evidence when synthetic event ids collide with real ids.
    #[cfg(feature = "devtools")]
    pub fn insert_blocking_preserving_entities(
        &self,
        events: Vec<Sanitized<NormalizedEvent>>,
        owned_entities: std::collections::HashSet<String>,
    ) -> AppResult<(Vec<i64>, std::collections::HashSet<String>)> {
        self.run(move |conn| {
            event_repository::insert_batch_preserving_entities(conn, &events, owned_entities)
        })
    }

    /// Explicit attribution starts inside this job, after queue admission. The
    /// returned inner result preserves repository rollback and error behavior.
    #[cfg(feature = "devtools")]
    pub fn insert_blocking_preserving_entities_with_timings(
        &self,
        events: Vec<Sanitized<NormalizedEvent>>,
        owned_entities: std::collections::HashSet<String>,
    ) -> AppResult<event_repository::CapturedPreservingInsertion> {
        self.run(move |conn| {
            Ok(
                event_repository::insert_batch_preserving_entities_with_timings(
                    conn,
                    &events,
                    owned_entities,
                ),
            )
        })
    }

    /// Admit one collector cycle without waiting for queue capacity.
    /// Queue rejection happens before any event, graph fact, or baseline can
    /// be written.
    pub fn submit_collector(
        &self,
        commit: AtomicCollectorCommit,
    ) -> AppResult<CollectorCommitTicket> {
        // Reject before admission: an invalid nested write must leave no
        // durable residue after its caller receives an error.
        self.reject_reentrant_wait()?;
        let count = commit.event_count() as u64;
        let collector_id = commit.collector_id().to_owned();
        let (reply, result) = sync_channel(1);
        self.queue_depth.fetch_add(1, Ordering::Relaxed);
        match self.tx.try_send(Command::CollectorCommit(commit, reply)) {
            Ok(()) => Ok(CollectorCommitTicket {
                result,
                writer_thread_id: self.thread_id,
            }),
            Err(TrySendError::Full(_)) => {
                decrement_queue_depth(&self.queue_depth);
                self.dropped.fetch_add(count, Ordering::Relaxed);
                increment_collector_count(&self.cycle_admission_drops, &collector_id, 1);
                self.throttled_warning();
                Err(DbError::QueueFull.into())
            }
            Err(TrySendError::Disconnected(_)) => {
                decrement_queue_depth(&self.queue_depth);
                Err(DbError::WriterGone.into())
            }
        }
    }

    /// Admit one collector cycle and wait for its transaction result.
    pub fn commit_collector(
        &self,
        commit: AtomicCollectorCommit,
    ) -> AppResult<CollectorCommitReceipt> {
        self.reject_reentrant_wait()?;
        self.submit_collector(commit)?.wait()
    }

    /// Run a job on the writer thread and wait for its result.
    pub fn run<T, F>(&self, job: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> AppResult<T> + Send + 'static,
    {
        self.reject_reentrant_wait()?;
        let (reply, result) = sync_channel(1);
        let wrapped: Job = Box::new(move |conn| {
            // A receiver that went away has nobody to tell.
            let _ = reply.send(job(conn));
        });
        self.queue_depth.fetch_add(1, Ordering::Relaxed);
        if self.tx.send(Command::Job(wrapped)).is_err() {
            decrement_queue_depth(&self.queue_depth);
            return Err(DbError::WriterGone.into());
        }
        result.recv().map_err(|_| DbError::WriterGone)?
    }

    /// Wait until everything queued so far has been committed.
    pub fn flush(&self) -> AppResult<()> {
        self.reject_reentrant_wait()?;
        let (ack, done) = sync_channel(1);
        self.queue_depth.fetch_add(1, Ordering::Relaxed);
        if self.tx.send(Command::Flush(ack)).is_err() {
            decrement_queue_depth(&self.queue_depth);
            return Err(DbError::WriterGone.into());
        }
        done.recv().map_err(|_| DbError::WriterGone.into())
    }

    /// Number of commands admitted to the bounded queue and not yet picked up
    /// by the writer thread. This is a live diagnostic signal, not a durable
    /// counter; it intentionally excludes the writer's private coalescing
    /// buffer after a command has been received.
    pub fn queue_depth(&self) -> u64 {
        self.queue_depth.load(Ordering::Relaxed)
    }

    /// If a write reported corruption, the reason.
    pub fn corruption(&self) -> Option<String> {
        lock(&self.corrupt).clone()
    }

    /// Rows lost after SQLite rejected a whole batch, grouped by collector.
    pub fn failed_commit_rows(&self) -> BTreeMap<String, u64> {
        lock(&self.failed_commits).clone()
    }

    /// Collector cycles refused before entering the writer queue.
    pub fn collector_cycle_admission_drops(&self) -> BTreeMap<String, u64> {
        lock(&self.cycle_admission_drops).clone()
    }

    /// Collector cycles rolled back after writer admission.
    pub fn failed_collector_cycles(&self) -> BTreeMap<String, u64> {
        lock(&self.failed_cycles).clone()
    }

    fn reject_reentrant_wait(&self) -> AppResult<()> {
        reject_writer_thread_wait(self.thread_id)
    }

    fn throttled_warning(&self) {
        let mut last = lock(&self.last_notice);
        let due = last.is_none_or(|at| at.elapsed() >= THROTTLE_NOTICE);
        if due {
            *last = Some(Instant::now());
            tracing::warn!(
                dropped_total = self.dropped.load(Ordering::Relaxed),
                "the write queue is full; events are being dropped"
            );
        }
    }
}

pub(super) fn writer_reentrant_error() -> AppError {
    AppError::InvalidInput {
        what: "database writer job",
        reason: "a writer-thread job cannot wait on the same writer".to_owned(),
    }
}

fn reject_writer_thread_wait(writer_thread_id: ThreadId) -> AppResult<()> {
    if std::thread::current().id() == writer_thread_id {
        return Err(writer_reentrant_error());
    }
    Ok(())
}

impl Drop for WriterHandle {
    fn drop(&mut self) {
        // The thread may already be gone; either way there is nothing to do
        // about a failed send here.
        let _ = self.tx.send(Command::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The loop the writer thread runs.
fn run_loop(
    conn: Connection,
    rx: Receiver<Command>,
    subscribers: Arc<Mutex<Vec<SyncSender<BatchNotice>>>>,
    corrupt: Arc<Mutex<Option<String>>>,
    failed_commits: Arc<Mutex<BTreeMap<String, u64>>>,
    failed_cycles: Arc<Mutex<BTreeMap<String, u64>>>,
    queue_depth: Arc<AtomicU64>,
) {
    let _writer_thread = WriterThreadMarker::enter();
    let mut pending: Vec<Sanitized<NormalizedEvent>> = Vec::new();
    let mut first_at: Option<Instant> = None;

    loop {
        let timeout = match first_at {
            Some(started) => COALESCE_WINDOW.saturating_sub(started.elapsed()),
            None => IDLE_POLL,
        };
        match rx.recv_timeout(timeout) {
            Ok(Command::Events(events)) => {
                decrement_queue_depth(&queue_depth);
                if first_at.is_none() {
                    first_at = Some(Instant::now());
                }
                pending.extend(events);
                if pending.len() >= COALESCE_EVENTS {
                    commit_pending(&conn, &mut pending, &subscribers, &corrupt, &failed_commits);
                    first_at = None;
                }
            }
            Ok(Command::CollectorCommit(commit, reply)) => {
                decrement_queue_depth(&queue_depth);
                // Preserve command order: an atomic collector commit starts
                // only after earlier coalesced events are durable.
                commit_pending(&conn, &mut pending, &subscribers, &corrupt, &failed_commits);
                first_at = None;
                let result = commit_atomic(
                    &conn,
                    &commit,
                    &subscribers,
                    &corrupt,
                    &failed_commits,
                    &failed_cycles,
                );
                let _ = reply.send(result);
            }
            Ok(Command::Job(job)) => {
                decrement_queue_depth(&queue_depth);
                // A job must see everything queued before it.
                commit_pending(&conn, &mut pending, &subscribers, &corrupt, &failed_commits);
                first_at = None;
                job(&conn);
            }
            Ok(Command::Flush(ack)) => {
                decrement_queue_depth(&queue_depth);
                commit_pending(&conn, &mut pending, &subscribers, &corrupt, &failed_commits);
                first_at = None;
                let _ = ack.send(());
            }
            Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                queue_depth.store(0, Ordering::Relaxed);
                commit_pending(&conn, &mut pending, &subscribers, &corrupt, &failed_commits);
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                if first_at.is_some() {
                    commit_pending(&conn, &mut pending, &subscribers, &corrupt, &failed_commits);
                    first_at = None;
                }
            }
        }
    }
}

fn decrement_queue_depth(queue_depth: &AtomicU64) {
    let _ = queue_depth.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |depth| {
        Some(depth.saturating_sub(1))
    });
}

fn commit_pending(
    conn: &Connection,
    pending: &mut Vec<Sanitized<NormalizedEvent>>,
    subscribers: &Mutex<Vec<SyncSender<BatchNotice>>>,
    corrupt: &Mutex<Option<String>>,
    failed_commits: &Mutex<BTreeMap<String, u64>>,
) {
    commit_pending_with(pending, subscribers, corrupt, failed_commits, |batch| {
        event_repository::insert_batch(conn, batch)
    });
}

fn commit_pending_with(
    pending: &mut Vec<Sanitized<NormalizedEvent>>,
    subscribers: &Mutex<Vec<SyncSender<BatchNotice>>>,
    corrupt: &Mutex<Option<String>>,
    failed_commits: &Mutex<BTreeMap<String, u64>>,
    insert: impl FnOnce(&[Sanitized<NormalizedEvent>]) -> AppResult<Vec<i64>>,
) {
    if pending.is_empty() {
        return;
    }
    match insert(pending) {
        Ok(ids) => publish(subscribers, pending, &ids),
        Err(error) => {
            record_failed_rows(failed_commits, pending);
            record_write_error(&error, pending.len(), corrupt);
        }
    }
    pending.clear();
}

fn commit_atomic(
    conn: &Connection,
    commit: &AtomicCollectorCommit,
    subscribers: &Mutex<Vec<SyncSender<BatchNotice>>>,
    corrupt: &Mutex<Option<String>>,
    failed_commits: &Mutex<BTreeMap<String, u64>>,
    failed_cycles: &Mutex<BTreeMap<String, u64>>,
) -> AppResult<CollectorCommitReceipt> {
    let events: Vec<_> = commit
        .observations()
        .iter()
        .map(|observation| observation.event().clone())
        .collect();
    match collector_commit::insert(conn, commit) {
        Ok(receipt) => {
            if !events.is_empty() {
                publish(subscribers, &events, &receipt.event_ids);
            }
            Ok(receipt)
        }
        Err(error) => {
            record_failed_rows(failed_commits, &events);
            increment_collector_count(failed_cycles, commit.collector_id(), 1);
            record_write_error(&error, events.len(), corrupt);
            Err(error)
        }
    }
}

fn increment_collector_count(
    totals: &Mutex<BTreeMap<String, u64>>,
    collector_id: &str,
    amount: u64,
) {
    let mut totals = lock(totals);
    let count = totals.entry(collector_id.to_owned()).or_default();
    *count = count.saturating_add(amount);
}

fn record_write_error(error: &AppError, rows: usize, corrupt: &Mutex<Option<String>>) {
    if let AppError::Db(DbError::Sqlite { operation, source }) = error {
        if is_corruption(source) {
            tracing::error!(operation, "a write found the database corrupt");
            *lock(corrupt) = Some(source.to_string());
            return;
        }
    }
    tracing::error!(%error, rows, "a batch could not be committed");
}

fn record_failed_rows(
    totals: &Mutex<BTreeMap<String, u64>>,
    pending: &[Sanitized<NormalizedEvent>],
) {
    let mut totals = lock(totals);
    for event in pending {
        let count = totals
            .entry(event.get().source_collector.clone())
            .or_default();
        *count = count.saturating_add(1);
    }
}

/// Tell every subscriber what was committed, dropping the ones that left.
fn publish(
    subscribers: &Mutex<Vec<SyncSender<BatchNotice>>>,
    batch: &[Sanitized<NormalizedEvent>],
    ids: &[i64],
) {
    let mut categories: Vec<Category> = batch.iter().map(|e| e.get().category).collect();
    categories.sort_unstable();
    categories.dedup();
    let highest_importance = batch
        .iter()
        .map(|e| e.get().importance)
        .max()
        .unwrap_or(Importance::Informational);
    let notice = BatchNotice {
        count: batch.len(),
        newest_id: ids.iter().copied().max().unwrap_or(0),
        categories,
        highest_importance,
        event_ids: ids.iter().rev().take(256).copied().collect(),
    };
    let mut list = lock(subscribers);
    list.retain(|tx| {
        !matches!(
            tx.try_send(notice.clone()),
            Err(TrySendError::Disconnected(_))
        )
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::privacy::Sanitizer;
    use crate::storage::baseline_repository;
    use crate::storage::migrations::{self, MIGRATIONS};
    use crate::storage::{Baseline, DbPaths};
    use serde_json::json;

    fn event(source: &str, seed: u64) -> Sanitized<NormalizedEvent> {
        let mut event = crate::testing::synthetic_event(seed, 1_700_000_000_000);
        event.source_collector = source.to_owned();
        Sanitizer::new().sanitize(event).0
    }

    fn baseline(source: &str, marker: &str) -> Sanitized<Baseline> {
        Sanitizer::new()
            .sanitize(Baseline {
                collector_id: source.to_owned(),
                entity_kind: "queue-test".to_owned(),
                snapshot: json!({ "marker": marker }),
                captured_at: 1,
                truncated: false,
            })
            .0
    }

    #[test]
    fn a_ticket_transferred_to_its_writer_rejects_wait_without_blocking() {
        let (_reply, result) = sync_channel(1);
        let ticket = CollectorCommitTicket {
            result,
            writer_thread_id: std::thread::current().id(),
        };
        assert!(matches!(ticket.wait(), Err(AppError::InvalidInput { .. })));
    }

    #[test]
    fn a_full_queue_counts_admission_drops_without_calling_them_commit_failures() {
        let (tx, rx) = sync_channel(1);
        let dropped = Arc::new(AtomicU64::new(0));
        let failed_commits = Arc::new(Mutex::new(BTreeMap::new()));
        let cycle_admission_drops = Arc::new(Mutex::new(BTreeMap::new()));
        let failed_cycles = Arc::new(Mutex::new(BTreeMap::new()));
        let handle = WriterHandle {
            tx,
            queue_depth: Arc::new(AtomicU64::new(0)),
            dropped: Arc::clone(&dropped),
            last_notice: Mutex::new(None),
            corrupt: Arc::new(Mutex::new(None)),
            failed_commits: Arc::clone(&failed_commits),
            cycle_admission_drops,
            failed_cycles,
            thread_id: std::thread::current().id(),
            thread: None,
        };
        handle.enqueue(vec![event("one", 1)]).unwrap();
        let error = handle.enqueue(vec![event("two", 2)]).unwrap_err();
        assert!(matches!(error, AppError::Db(DbError::QueueFull)));
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
        assert!(lock(&failed_commits).is_empty());

        // This deliberately constructed handle has no consumer thread. Drop
        // the receiver first so its production Drop path observes a closed
        // channel instead of blocking behind the intentionally full queue.
        drop(rx);
        drop(handle);
    }

    #[test]
    fn an_injected_commit_failure_counts_rows_by_collector_not_as_queue_drops() {
        let mut pending = vec![event("one", 1), event("one", 2), event("two", 3)];
        let subscribers = Mutex::new(Vec::new());
        let corrupt = Mutex::new(None);
        let failed_commits = Mutex::new(BTreeMap::new());
        commit_pending_with(
            &mut pending,
            &subscribers,
            &corrupt,
            &failed_commits,
            |_| {
                Err(AppError::InvalidInput {
                    what: "injected commit",
                    reason: "deterministic test failure".to_owned(),
                })
            },
        );
        assert!(pending.is_empty());
        assert_eq!(
            *lock(&failed_commits),
            BTreeMap::from([("one".to_owned(), 2), ("two".to_owned(), 1)])
        );
        assert!(lock(&corrupt).is_none());
    }

    #[test]
    fn a_rejected_baseline_only_cycle_has_cycle_telemetry_and_cannot_advance() {
        let root = crate::testing::temp_dir("atomic-queue-full");
        let paths = DbPaths::under(&root);
        let mut conn = crate::storage::connection::open_writer(&paths.database).unwrap();
        crate::storage::connection::assert_fts5(&conn).unwrap();
        migrations::run(&mut conn, MIGRATIONS, &paths.backups).unwrap();
        baseline_repository::store(&conn, &baseline("one", "old")).unwrap();

        let dropped = Arc::new(AtomicU64::new(0));
        let failed_commits = Arc::new(Mutex::new(BTreeMap::new()));
        let cycle_admission_drops = Arc::new(Mutex::new(BTreeMap::new()));
        let failed_cycles = Arc::new(Mutex::new(BTreeMap::new()));
        let (notice_tx, notice_rx) = sync_channel(2);
        let handle = WriterHandle::spawn_with_capacity(
            conn,
            Arc::new(Mutex::new(vec![notice_tx])),
            Arc::clone(&dropped),
            Arc::clone(&failed_commits),
            Arc::clone(&cycle_admission_drops),
            Arc::clone(&failed_cycles),
            1,
        )
        .unwrap();

        let (entered_tx, entered_rx) = sync_channel(1);
        let (release_tx, release_rx) = sync_channel(1);
        handle
            .tx
            .send(Command::Job(Box::new(move |_| {
                let _ = entered_tx.send(());
                let _ = release_rx.recv();
            })))
            .unwrap();
        entered_rx.recv().unwrap();

        // The writer is held inside the job, so this one event occupies the
        // only queue slot and the atomic command is deterministically refused.
        handle.enqueue(vec![event("queued", 1)]).unwrap();
        let replacement =
            AtomicCollectorCommit::new(Vec::new(), Some(baseline("one", "new"))).unwrap();
        let error = handle.commit_collector(replacement).unwrap_err();
        assert!(matches!(error, AppError::Db(DbError::QueueFull)));
        assert_eq!(dropped.load(Ordering::Relaxed), 0);
        assert!(lock(&failed_commits).is_empty());
        assert_eq!(
            *lock(&cycle_admission_drops),
            BTreeMap::from([("one".to_owned(), 1)])
        );
        assert!(lock(&failed_cycles).is_empty());
        assert_eq!(
            notice_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        );

        release_tx.send(()).unwrap();
        handle.flush().unwrap();
        drop(handle);

        let conn = Connection::open(&paths.database).unwrap();
        let stored = baseline_repository::load(&conn, "one", "queue-test")
            .unwrap()
            .unwrap();
        assert_eq!(stored.snapshot["marker"], "old");
    }
}
