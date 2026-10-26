//! The database handle: one writer thread, a pool of readers, and a state
//! that says whether either can be used.
//!
//! `Db::open` does not fail on corruption. A file that does not pass its
//! integrity check leaves the handle in `DbState::Corrupt`, the application
//! finishes booting, and the user is offered repair, diagnostics or a reset.
//! Only a failure the user cannot act on — a directory that cannot be
//! created, a SQLite built without the feature the schema needs — is an error.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, TryLockError};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::connection::{assert_fts5, is_corruption, open_writer, quick_check};
use super::migrations::{self, MIGRATIONS};
use super::reader_pool::ReaderPool;
use super::recovery::{self, RecoveryAction, RecoveryOutcome};
use super::writer::{BatchNotice, WriterHandle};
use super::{AtomicCollectorCommit, CollectorCommitReceipt, CollectorCommitTicket};
use crate::domain::NormalizedEvent;
use crate::error::{AppError, AppResult, DbError};
use crate::privacy::Sanitized;

/// File name of the event database inside the data directory.
pub const DATABASE_FILE: &str = "events.db";

/// Read-only connections kept open. Queries are short and the UI issues few
/// at once; four is enough that a screen never waits on another screen.
const READER_POOL_SIZE: usize = 4;

/// How many batch notices a subscriber may fall behind before notices are
/// dropped for it. A notice carries no bodies, so dropping one loses nothing
/// the next one does not say again.
const NOTICE_BUFFER: usize = 64;

/// Where the database and its companions live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbPaths {
    /// The data directory.
    pub root: PathBuf,
    /// `events.db`.
    pub database: PathBuf,
    /// Pre-migration backups.
    pub backups: PathBuf,
}

impl DbPaths {
    /// The layout under a data directory.
    pub fn under(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            database: root.join(DATABASE_FILE),
            backups: root.join("backups"),
        }
    }

    /// The write-ahead log that accompanies the database.
    pub fn wal(&self) -> PathBuf {
        companion(&self.database, "-wal")
    }

    /// The shared-memory index that accompanies the write-ahead log.
    pub fn shm(&self) -> PathBuf {
        companion(&self.database, "-shm")
    }
}

fn companion(database: &Path, suffix: &str) -> PathBuf {
    let mut name = database.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Whether the database can be used, and if not, why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DbState {
    /// Open, migrated, accepting reads and writes.
    Ready {
        /// The schema version in the file.
        schema_version: usize,
    },
    /// The file failed its integrity check or is not a database.
    Corrupt {
        /// What the check said. Names structures, never rows.
        reason: String,
    },
    /// The file is intact but its schema disagrees with this build: written
    /// by a newer version, or by a migration whose SQL has since been edited.
    SchemaMismatch {
        /// Which disagreement.
        reason: String,
    },
    /// A recovery action is running.
    Recovering,
}

impl DbState {
    /// Whether reads and writes may proceed.
    pub fn is_ready(&self) -> bool {
        matches!(self, DbState::Ready { .. })
    }

    fn not_ready(&self) -> AppError {
        DbError::NotReady {
            state: match self {
                DbState::Ready { .. } => "ready".to_owned(),
                DbState::Corrupt { reason } => format!("corrupt ({reason})"),
                DbState::SchemaMismatch { reason } => format!("schema mismatch ({reason})"),
                DbState::Recovering => "recovering".to_owned(),
            },
        }
        .into()
    }
}

/// What a `Db` holds while it is open. Replaced wholesale by recovery.
struct Inner {
    state: DbState,
    writer: Option<WriterHandle>,
    readers: Option<ReaderPool>,
}

/// The shared parts every incarnation of the connection set uses.
#[derive(Clone)]
struct Shared {
    subscribers: Arc<Mutex<Vec<SyncSender<BatchNotice>>>>,
    dropped: Arc<AtomicU64>,
    failed_commits: Arc<Mutex<BTreeMap<String, u64>>>,
    cycle_admission_drops: Arc<Mutex<BTreeMap<String, u64>>>,
    failed_cycles: Arc<Mutex<BTreeMap<String, u64>>>,
}

/// The database.
pub struct Db {
    paths: DbPaths,
    inner: RwLock<Inner>,
    lifecycle: Mutex<()>,
    closed: AtomicBool,
    shared: Shared,
    #[cfg(test)]
    lifecycle_probe: Mutex<Option<LifecycleProbe>>,
}

#[cfg(test)]
struct LifecycleProbe {
    operation: &'static str,
    entered: SyncSender<()>,
    release: Receiver<()>,
}

impl std::fmt::Debug for Db {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Db")
            .field("paths", &self.paths)
            .field("state", &self.state())
            .finish()
    }
}

impl Db {
    /// Open the database under `paths`, creating it if needed.
    ///
    /// Corruption and schema disagreement are states, not errors. Everything
    /// else that fails here is something the user cannot fix from a screen.
    pub fn open(paths: &DbPaths) -> AppResult<Db> {
        std::fs::create_dir_all(&paths.root).map_err(|source| AppError::Directory {
            path: paths.root.clone(),
            source,
        })?;
        let shared = Shared {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            dropped: Arc::new(AtomicU64::new(0)),
            failed_commits: Arc::new(Mutex::new(BTreeMap::new())),
            cycle_admission_drops: Arc::new(Mutex::new(BTreeMap::new())),
            failed_cycles: Arc::new(Mutex::new(BTreeMap::new())),
        };
        let inner = open_inner(paths, &shared)?;
        Ok(Db {
            paths: paths.clone(),
            inner: RwLock::new(inner),
            lifecycle: Mutex::new(()),
            closed: AtomicBool::new(false),
            shared,
            #[cfg(test)]
            lifecycle_probe: Mutex::new(None),
        })
    }

    /// Open under a data directory, with the standard layout.
    pub fn open_in(root: &Path) -> AppResult<Db> {
        Db::open(&DbPaths::under(root))
    }

    /// Where the files are.
    pub fn paths(&self) -> &DbPaths {
        &self.paths
    }

    /// The current state. A write that ran into corruption after open is
    /// surfaced here, so the UI learns about it from the same place.
    pub fn state(&self) -> DbState {
        if self.closed.load(Ordering::Acquire) {
            return closed_state();
        }
        let found = {
            let inner = self.read_inner();
            match inner.writer.as_ref().and_then(WriterHandle::corruption) {
                Some(reason) => reason,
                None => return inner.state.clone(),
            }
        };
        self.mark_corrupt(found);
        self.read_inner().state.clone()
    }

    /// Writes dropped because the queue was full, since open.
    pub fn dropped_writes(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }

    /// Commands admitted to the writer queue and not yet picked up by the
    /// writer thread. This is a live diagnostic signal.
    pub fn write_queue_depth(&self) -> u64 {
        let inner = self.read_inner();
        inner
            .writer
            .as_ref()
            .map(WriterHandle::queue_depth)
            .unwrap_or(0)
    }

    /// Rows lost after a failed batch commit, separately from admission drops.
    pub fn failed_commit_rows(&self) -> BTreeMap<String, u64> {
        let inner = self.read_inner();
        inner
            .writer
            .as_ref()
            .map(WriterHandle::failed_commit_rows)
            .unwrap_or_else(|| lock(&self.shared.failed_commits).clone())
    }

    /// Collector cycles refused before they entered the bounded writer queue.
    pub fn collector_cycle_admission_drops(&self) -> BTreeMap<String, u64> {
        lock(&self.shared.cycle_admission_drops).clone()
    }

    /// Collector cycles admitted but rolled back by the writer.
    pub fn failed_collector_cycles(&self) -> BTreeMap<String, u64> {
        lock(&self.shared.failed_cycles).clone()
    }

    /// Receive a notice after every batch the writer commits.
    pub fn subscribe(&self) -> Receiver<BatchNotice> {
        let (tx, rx) = sync_channel(NOTICE_BUFFER);
        lock(&self.shared.subscribers).push(tx);
        rx
    }

    /// Queue events for the writer without waiting. Coalesced into one
    /// transaction with whatever else arrives within the batching window.
    pub fn enqueue_events(&self, events: Vec<Sanitized<NormalizedEvent>>) -> AppResult<()> {
        self.with_writer(|writer| writer.enqueue(events))
    }

    /// Insert events and wait for their ids.
    pub fn insert_events(&self, events: Vec<Sanitized<NormalizedEvent>>) -> AppResult<Vec<i64>> {
        self.with_writer(|writer| writer.insert_blocking(events))
    }

    /// Devtools-only synthetic event insert using the regular sanitized
    /// transaction while preserving any entity row already on disk.
    #[cfg(feature = "devtools")]
    pub fn insert_events_preserving_entities(
        &self,
        events: Vec<Sanitized<NormalizedEvent>>,
        owned_entities: std::collections::HashSet<String>,
    ) -> AppResult<(Vec<i64>, std::collections::HashSet<String>)> {
        self.with_writer(|writer| {
            writer.insert_blocking_preserving_entities(events, owned_entities)
        })
    }

    /// Devtools-only, explicitly captured attribution on the actual writer.
    #[cfg(feature = "devtools")]
    pub fn insert_events_preserving_entities_with_timings(
        &self,
        events: Vec<Sanitized<NormalizedEvent>>,
        owned_entities: std::collections::HashSet<String>,
    ) -> AppResult<super::event_repository::CapturedPreservingInsertion> {
        self.with_writer(|writer| {
            writer.insert_blocking_preserving_entities_with_timings(events, owned_entities)
        })
    }

    /// Atomically store one collector cycle and wait for its durable event
    /// ids. Admission is nonblocking; after admission the caller waits for the
    /// writer's commit acknowledgement so it never advances in-memory state
    /// on a rolled-back baseline.
    pub fn commit_collector(
        &self,
        commit: AtomicCollectorCommit,
    ) -> AppResult<CollectorCommitReceipt> {
        self.with_writer(|writer| writer.commit_collector(commit))
    }

    /// Admit one collector cycle and return its acknowledgement ticket.
    /// Callers may hold a lifecycle read fence through this nonblocking step,
    /// then release it before waiting on SQLite.
    pub fn submit_collector(
        &self,
        commit: AtomicCollectorCommit,
    ) -> AppResult<CollectorCommitTicket> {
        self.with_writer(|writer| writer.submit_collector(commit))
    }

    /// Run a write job on the writer thread and wait for its result. Events
    /// queued before the job are committed before it runs.
    pub fn write<T, F>(&self, job: F) -> AppResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> AppResult<T> + Send + 'static,
    {
        self.with_writer(|writer| writer.run(job))
    }

    /// Wait until every queued write has been committed.
    pub fn flush(&self) -> AppResult<()> {
        self.with_writer(WriterHandle::flush)
    }

    /// Run a query on a read-only connection.
    pub fn read<T, F>(&self, query: F) -> AppResult<T>
    where
        F: FnOnce(&Connection) -> AppResult<T>,
    {
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_state().not_ready());
        }
        let inner = self.read_inner();
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_state().not_ready());
        }
        let readers = inner
            .readers
            .as_ref()
            .ok_or_else(|| inner.state.not_ready())?;
        let conn = readers.get()?;
        query(&conn)
    }

    /// Perform a recovery action. The handle is unusable while it runs and
    /// comes back in whatever state the action produced.
    pub fn recover(&self, action: RecoveryAction) -> AppResult<RecoveryOutcome> {
        self.recover_inner(action, false)
    }

    /// Perform a recovery action requested through the user-facing command.
    /// Destructive recovery is only meaningful after corruption or a schema
    /// mismatch; keep this check under the lifecycle fence so a concurrent
    /// writer cannot turn a healthy preflight into a destructive reset.
    pub fn recover_for_command(&self, action: RecoveryAction) -> AppResult<RecoveryOutcome> {
        self.recover_inner(action, true)
    }

    fn recover_inner(
        &self,
        action: RecoveryAction,
        reject_ready_destructive: bool,
    ) -> AppResult<RecoveryOutcome> {
        if super::writer::on_writer_thread() {
            return Err(lifecycle_reentrant_error());
        }
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_state().not_ready());
        }
        let _lifecycle = self.try_lifecycle()?;
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_state().not_ready());
        }
        if reject_ready_destructive
            && matches!(
                action,
                RecoveryAction::Repair | RecoveryAction::ResetHistory
            )
            && self.state().is_ready()
        {
            return Err(AppError::InvalidInput {
                what: "recovery action",
                reason:
                    "repair or reset is available only for a corrupt or schema-mismatched database"
                        .to_owned(),
            });
        }
        let (previous, writer, readers) = {
            let mut inner = self.write_inner();
            let previous = inner.state.clone();
            let writer = inner.writer.take();
            let readers = inner.readers.take();
            inner.state = DbState::Recovering;
            (previous, writer, readers)
        };
        #[cfg(test)]
        self.pause_lifecycle_for_test("recover");
        // Joining the writer while holding `inner` can deadlock if a queued
        // job reaches back into this Db. Finish and close connections before
        // recovery touches the file, but after the state fence is visible.
        drop(writer);
        drop(readers);
        let result = recovery::perform(&self.paths, &previous, action);
        let reopened = if self.closed.load(Ordering::Acquire) {
            Ok(unusable(closed_state()))
        } else {
            open_inner(&self.paths, &self.shared)
        };
        let mut inner = self.write_inner();
        let mut discarded = None;
        match reopened {
            Ok(fresh) if self.closed.load(Ordering::Acquire) => {
                discarded = Some(fresh);
                *inner = unusable(closed_state());
            }
            Ok(fresh) => *inner = fresh,
            Err(error) => {
                inner.state = DbState::Corrupt {
                    reason: error.to_string(),
                };
            }
        }
        let state = inner.state.clone();
        drop(inner);
        // A close request may race with reopening. Dispose that unexposed
        // incarnation outside `inner`, preserving writer callback progress.
        drop(discarded);
        result.map(|outcome| outcome.with_state(state))
    }

    /// Replace a healthy history database with a freshly migrated file.
    ///
    /// Privacy deletion is a database lifecycle operation, not a normal
    /// writer job: every reader and the writer must be closed before the old
    /// file is swapped. The replacement helper keeps the old artifact until
    /// the new file passes integrity verification, then this handle reopens
    /// its writer and reader pool.
    pub fn replace_history(
        &self,
        preserved_theme: &str,
        preserved_visualization: &str,
    ) -> AppResult<()> {
        if super::writer::on_writer_thread() {
            return Err(lifecycle_reentrant_error());
        }
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_state().not_ready());
        }
        let _lifecycle = self.try_lifecycle()?;
        let (writer, readers) = {
            let mut inner = self.write_inner();
            if !inner.state.is_ready() {
                return Err(inner.state.not_ready());
            }
            let writer = inner.writer.take();
            let readers = inner.readers.take();
            inner.state = DbState::Recovering;
            (writer, readers)
        };
        drop(writer);
        drop(readers);

        let result =
            super::purge::replace_database(&self.paths, preserved_theme, preserved_visualization);
        let reopened = if self.closed.load(Ordering::Acquire) {
            Ok(unusable(closed_state()))
        } else {
            open_inner(&self.paths, &self.shared)
        };
        let mut inner = self.write_inner();
        match reopened {
            Ok(fresh) if self.closed.load(Ordering::Acquire) => {
                *inner = unusable(closed_state());
                drop(fresh);
            }
            Ok(fresh) => *inner = fresh,
            Err(error) => {
                inner.state = DbState::Corrupt {
                    reason: error.to_string(),
                };
            }
        }
        result
    }

    /// Stop the writer and close every connection. Later calls fail with
    /// `NotReady`. Used by tests that reopen the same files and by shutdown.
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        if super::writer::on_writer_thread() {
            return;
        }
        let _lifecycle = lock(&self.lifecycle);
        let (writer, readers) = {
            let mut inner = self.write_inner();
            let writer = inner.writer.take();
            let readers = inner.readers.take();
            inner.state = DbState::Corrupt {
                reason: "closed".to_owned(),
            };
            (writer, readers)
        };
        drop(writer);
        drop(readers);
    }

    /// Mark the database corrupt after a write reported it, and release the
    /// connections so recovery can move the file.
    fn mark_corrupt(&self, reason: String) {
        if self.closed.load(Ordering::Acquire) || super::writer::on_writer_thread() {
            return;
        }
        let Ok(_lifecycle) = self.try_lifecycle() else {
            // An active close/recovery already owns the old incarnation and
            // will publish the authoritative state after it drains.
            return;
        };
        if self.closed.load(Ordering::Acquire) {
            return;
        }
        let detached = {
            let mut inner = self.write_inner();
            if matches!(inner.state, DbState::Ready { .. }) {
                tracing::error!("the database became corrupt while running");
                let writer = inner.writer.take();
                let readers = inner.readers.take();
                inner.state = DbState::Corrupt { reason };
                Some((writer, readers))
            } else {
                None
            }
        };
        #[cfg(test)]
        self.pause_lifecycle_for_test("mark_corrupt");
        drop(detached);
    }

    fn with_writer<T>(&self, f: impl FnOnce(&WriterHandle) -> AppResult<T>) -> AppResult<T> {
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_state().not_ready());
        }
        let inner = self.read_inner();
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_state().not_ready());
        }
        let writer = inner
            .writer
            .as_ref()
            .ok_or_else(|| inner.state.not_ready())?;
        f(writer)
    }

    fn read_inner(&self) -> std::sync::RwLockReadGuard<'_, Inner> {
        self.inner
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn write_inner(&self) -> std::sync::RwLockWriteGuard<'_, Inner> {
        self.inner
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn try_lifecycle(&self) -> AppResult<MutexGuard<'_, ()>> {
        match self.lifecycle.try_lock() {
            Ok(guard) => Ok(guard),
            Err(TryLockError::Poisoned(poisoned)) => Ok(poisoned.into_inner()),
            Err(TryLockError::WouldBlock) => Err(lifecycle_busy_error()),
        }
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used)]
    fn pause_lifecycle_for_test(&self, operation: &'static str) {
        let probe = {
            let mut slot = lock(&self.lifecycle_probe);
            if slot
                .as_ref()
                .is_some_and(|probe| probe.operation == operation)
            {
                slot.take()
            } else {
                None
            }
        };
        if let Some(probe) = probe {
            probe.entered.send(()).unwrap();
            probe.release.recv().unwrap();
        }
    }
}

fn closed_state() -> DbState {
    DbState::Corrupt {
        reason: "closed".to_owned(),
    }
}

fn lifecycle_busy_error() -> AppError {
    DbError::Recovery {
        action: "start a database lifecycle operation",
        reason: "another database lifecycle operation is already running".to_owned(),
    }
    .into()
}

fn lifecycle_reentrant_error() -> AppError {
    DbError::Recovery {
        action: "start a database lifecycle operation",
        reason: "a writer-thread callback cannot run database lifecycle operations".to_owned(),
    }
    .into()
}

/// Lock a mutex, recovering the value if a holder panicked.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Open, check, migrate, and spin up the writer and readers.
fn open_inner(paths: &DbPaths, shared: &Shared) -> AppResult<Inner> {
    let mut conn = match open_writer(&paths.database) {
        Ok(conn) => conn,
        Err(AppError::Db(DbError::Sqlite { operation, source })) if is_corruption(&source) => {
            tracing::error!(operation, "the database file is not usable");
            return Ok(unusable(DbState::Corrupt {
                reason: source.to_string(),
            }));
        }
        Err(error) => return Err(error),
    };

    match quick_check(&conn) {
        Ok(None) => {}
        Ok(Some(problem)) => {
            tracing::error!("the database failed its integrity check");
            return Ok(unusable(DbState::Corrupt { reason: problem }));
        }
        Err(AppError::Db(DbError::Sqlite { source, .. })) if is_corruption(&source) => {
            return Ok(unusable(DbState::Corrupt {
                reason: source.to_string(),
            }));
        }
        Err(error) => return Err(error),
    }

    // A build problem, not a file problem: fail loudly rather than boot with
    // a schema the library cannot create.
    assert_fts5(&conn)?;

    let report = match migrations::run(&mut conn, MIGRATIONS, &paths.backups) {
        Ok(report) => report,
        Err(AppError::Db(error @ (DbError::Drift { .. } | DbError::Ahead { .. }))) => {
            tracing::error!(%error, "the schema does not match this build");
            return Ok(unusable(DbState::SchemaMismatch {
                reason: error.to_string(),
            }));
        }
        Err(AppError::Db(DbError::Sqlite { source, .. })) if is_corruption(&source) => {
            return Ok(unusable(DbState::Corrupt {
                reason: source.to_string(),
            }));
        }
        Err(error) => return Err(error),
    };
    if report.from != report.to {
        tracing::info!(from = report.from, to = report.to, "schema migrated");
    }

    let readers = ReaderPool::open(&paths.database, READER_POOL_SIZE)?;
    let writer = WriterHandle::spawn(
        conn,
        Arc::clone(&shared.subscribers),
        Arc::clone(&shared.dropped),
        Arc::clone(&shared.failed_commits),
        Arc::clone(&shared.cycle_admission_drops),
        Arc::clone(&shared.failed_cycles),
    )?;
    Ok(Inner {
        state: DbState::Ready {
            schema_version: report.to,
        },
        writer: Some(writer),
        readers: Some(readers),
    })
}

fn unusable(state: DbState) -> Inner {
    Inner {
        state,
        writer: None,
        readers: None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::privacy::Sanitizer;
    use crate::storage::{AtomicCollectorCommit, Baseline};
    use serde_json::json;
    use std::time::Duration;

    fn pause_next_lifecycle(db: &Db, operation: &'static str) -> (Receiver<()>, SyncSender<()>) {
        let (entered_tx, entered_rx) = sync_channel(1);
        let (release_tx, release_rx) = sync_channel(1);
        *lock(&db.lifecycle_probe) = Some(LifecycleProbe {
            operation,
            entered: entered_tx,
            release: release_rx,
        });
        (entered_rx, release_tx)
    }

    fn is_lifecycle_busy(result: &AppResult<RecoveryOutcome>) -> bool {
        matches!(
            result,
            Err(AppError::Db(DbError::Recovery {
                action: "start a database lifecycle operation",
                reason,
            })) if reason.contains("already running")
        )
    }

    fn baseline_only_commit(marker: &str) -> AtomicCollectorCommit {
        let baseline = Sanitizer::new()
            .sanitize(Baseline {
                collector_id: "process".to_owned(),
                entity_kind: "shutdown-test".to_owned(),
                snapshot: json!({ "marker": marker }),
                captured_at: 1,
                truncated: false,
            })
            .0;
        AtomicCollectorCommit::new(Vec::new(), Some(baseline)).unwrap()
    }

    #[test]
    fn the_companion_files_sit_beside_the_database() {
        let paths = DbPaths::under(Path::new(r"C:\data"));
        assert_eq!(paths.database, Path::new(r"C:\data\events.db"));
        assert_eq!(paths.wal(), Path::new(r"C:\data\events.db-wal"));
        assert_eq!(paths.shm(), Path::new(r"C:\data\events.db-shm"));
        assert_eq!(paths.backups, Path::new(r"C:\data\backups"));
    }

    #[test]
    fn a_fresh_open_is_ready_at_the_latest_schema() {
        let dir = crate::testing::temp_dir("db-open");
        let db = Db::open_in(&dir).unwrap();
        assert_eq!(
            db.state(),
            DbState::Ready {
                schema_version: MIGRATIONS.len()
            }
        );
        assert!(db.paths().database.exists());
        assert!(
            db.paths().wal().exists(),
            "WAL mode leaves its log beside the file"
        );
        db.close();
        assert!(!db.state().is_ready());
        assert!(db.flush().is_err());
    }

    #[test]
    fn state_serializes_as_a_tagged_word() {
        let json = serde_json::to_value(DbState::Ready { schema_version: 2 }).unwrap();
        assert_eq!(json["state"], "ready");
        assert_eq!(json["schema_version"], 2);
    }

    #[test]
    fn a_writer_job_cannot_wait_reentrantly_on_the_same_writer() {
        let db = Arc::new(Db::open_in(&crate::testing::temp_dir("db-reentrant-writer")).unwrap());
        let nested = Arc::clone(&db);

        let message = db
            .write(move |_| {
                let error = nested.write(|_| Ok(())).unwrap_err();
                Ok(error.to_string())
            })
            .unwrap();

        assert!(message.contains("cannot wait on the same writer"));
    }

    #[test]
    fn lifecycle_calls_from_a_writer_callback_do_not_wait_on_the_inner_lock() {
        let db =
            Arc::new(Db::open_in(&crate::testing::temp_dir("db-reentrant-lifecycle")).unwrap());
        let callback_db = Arc::clone(&db);

        let recovery_error = db
            .write(move |_| {
                let error = callback_db
                    .recover(RecoveryAction::ExportDiagnostics)
                    .unwrap_err();
                callback_db.close();
                Ok(error.to_string())
            })
            .unwrap();

        assert!(recovery_error.contains("writer-thread callback"));
        assert!(!db.state().is_ready());
        assert!(db.flush().is_err());
        // The callback can make the handle terminal but cannot join itself.
        // A later external close performs the actual connection drain.
        db.close();
    }

    #[test]
    fn close_finishes_an_admitted_ticket_without_holding_the_inner_lock() {
        let db = Arc::new(Db::open_in(&crate::testing::temp_dir("db-close-pending")).unwrap());
        let blocking_db = Arc::clone(&db);
        let (entered_tx, entered_rx) = sync_channel(1);
        let (release_tx, release_rx) = sync_channel(1);
        let blocker = std::thread::spawn(move || {
            blocking_db
                .write(move |_| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    Ok(())
                })
                .unwrap();
        });
        entered_rx.recv().unwrap();

        let ticket = db
            .submit_collector(baseline_only_commit("pending"))
            .unwrap();
        let closing_db = Arc::clone(&db);
        let (closed_tx, closed_rx) = sync_channel(1);
        let closer = std::thread::spawn(move || {
            closing_db.close();
            closed_tx.send(()).unwrap();
        });

        let closed_before_release = closed_rx.recv_timeout(Duration::from_millis(30)).is_ok();
        release_tx.send(()).unwrap();
        blocker.join().unwrap();
        if !closed_before_release {
            closed_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("close is bounded after the finite writer job completes");
        }
        closer.join().unwrap();

        assert!(!closed_before_release);
        assert!(ticket.wait().unwrap().event_ids.is_empty());
        assert!(!db.state().is_ready());
    }

    #[test]
    fn a_second_recovery_is_rejected_before_it_can_touch_shared_files() {
        let db = Arc::new(Db::open_in(&crate::testing::temp_dir("db-recover-exclusive")).unwrap());
        let (entered, release) = pause_next_lifecycle(&db, "recover");
        let first_db = Arc::clone(&db);
        let first = std::thread::spawn(move || first_db.recover(RecoveryAction::ResetHistory));
        entered.recv().unwrap();

        let second = db.recover(RecoveryAction::ResetHistory);
        release.send(()).unwrap();
        let first = first.join().unwrap();

        assert!(is_lifecycle_busy(&second), "second recovery: {second:?}");
        assert!(first.unwrap().succeeded);
        assert!(db.state().is_ready());
    }

    #[test]
    fn command_recovery_rejects_destructive_actions_on_a_healthy_database() {
        let db = Db::open_in(&crate::testing::temp_dir("db-command-recovery-ready")).unwrap();

        for action in [RecoveryAction::Repair, RecoveryAction::ResetHistory] {
            let result = db.recover_for_command(action);
            assert!(result.is_err(), "healthy database accepted {action:?}");
            assert!(db.state().is_ready());
        }
    }

    #[test]
    fn close_waits_for_recovery_and_cannot_be_reopened_behind_it() {
        let db = Arc::new(Db::open_in(&crate::testing::temp_dir("db-recover-close")).unwrap());
        let (entered, release) = pause_next_lifecycle(&db, "recover");
        let recovery_db = Arc::clone(&db);
        let recovery =
            std::thread::spawn(move || recovery_db.recover(RecoveryAction::ExportDiagnostics));
        entered.recv().unwrap();

        let close_db = Arc::clone(&db);
        let (closed_tx, closed_rx) = sync_channel(1);
        let closer = std::thread::spawn(move || {
            close_db.close();
            closed_tx.send(()).unwrap();
        });
        let returned_before_recovery = closed_rx.recv_timeout(Duration::from_millis(30)).is_ok();
        release.send(()).unwrap();
        let _ = recovery.join().unwrap();
        if !returned_before_recovery {
            closed_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        closer.join().unwrap();

        assert!(!returned_before_recovery, "close overtook active recovery");
        assert!(!db.state().is_ready(), "recovery reopened after close");
    }

    #[test]
    fn recovery_cannot_overtake_a_corruption_transition_drain() {
        let db = Arc::new(Db::open_in(&crate::testing::temp_dir("db-corrupt-exclusive")).unwrap());
        let (entered, release) = pause_next_lifecycle(&db, "mark_corrupt");
        let corrupt_db = Arc::clone(&db);
        let marker = std::thread::spawn(move || corrupt_db.mark_corrupt("injected".to_owned()));
        entered.recv().unwrap();

        let recovery = db.recover(RecoveryAction::ResetHistory);
        release.send(()).unwrap();
        marker.join().unwrap();

        assert!(is_lifecycle_busy(&recovery), "recovery: {recovery:?}");
        assert!(matches!(db.state(), DbState::Corrupt { .. }));
    }
}
