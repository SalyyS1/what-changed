//! One bounded writer and two read-only connections for the separate owner file.

mod configuration;
mod evidence;
mod lifecycle;
mod migrations;
mod portability;
mod project_memory;
pub(crate) mod repository;
mod usage;
pub(crate) use repository::brief_snapshot;
mod rules;
mod scenario;
mod workspace;

use crate::error::{AppResult, DbError};
use crate::storage::{connection, reader_pool::ReaderPool, SqlContext};
use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use ts_rs::TS;

const WRITE_QUEUE_CAPACITY: usize = 16;
type Job = Box<dyn FnOnce(&mut Connection) + Send>;

/// Opaque admission captured by backend producers; never accepted over IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmissionEpoch(u64);
impl AdmissionEpoch {
    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum OwnerStoreState {
    Ready,
    Unavailable { reason: String },
    Closed,
}

struct Writer {
    sender: mpsc::SyncSender<Job>,
    handle: JoinHandle<()>,
}
struct Runtime {
    writer: Writer,
    readers: Arc<ReaderPool>,
}

/// AppState owns this store. Shutdown joins all admitted work before releasing files.
pub struct OwnerStore {
    path: PathBuf,
    runtime: RwLock<Option<Runtime>>,
    failure: Mutex<Option<String>>,
}

pub(super) fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|p| p.into_inner())
}

fn not_ready(message: &str) -> crate::error::AppError {
    DbError::NotReady {
        state: message.to_owned(),
    }
    .into()
}

fn epoch_value(raw: i64) -> AppResult<u64> {
    u64::try_from(raw).map_err(|_| not_ready("owner epoch is invalid"))
}

impl Runtime {
    fn open(path: &Path) -> AppResult<Self> {
        let existed = path.exists();
        if existed {
            // Inspect through a read-only handle before WAL pragmas can alter any file.
            let reader = connection::open_reader(path)?;
            migrations::inspect(&reader)?;
        }
        let mut connection = connection::open_writer(path)?;
        migrations::initialize(&mut connection, path, existed)?;
        let readers = Arc::new(ReaderPool::open(path, 2)?);
        let (sender, receiver) = mpsc::sync_channel::<Job>(WRITE_QUEUE_CAPACITY);
        let handle = std::thread::Builder::new()
            .name("owner-workspace-writer".into())
            .spawn(move || {
                for job in receiver {
                    job(&mut connection);
                }
            })
            .map_err(|_| not_ready("could not start owner writer"))?;
        Ok(Self {
            writer: Writer { sender, handle },
            readers,
        })
    }

    fn run<T: Send + 'static>(
        &self,
        action: impl FnOnce(&mut Connection) -> AppResult<T> + Send + 'static,
    ) -> AppResult<T> {
        let (reply, answer) = mpsc::sync_channel(1);
        self.writer
            .sender
            .try_send(Box::new(move |conn| {
                let _ = reply.send(action(conn));
            }))
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => DbError::QueueFull,
                mpsc::TrySendError::Disconnected(_) => DbError::WriterGone,
            })?;
        answer.recv().map_err(|_| DbError::WriterGone)?
    }

    fn stop(self) -> AppResult<()> {
        drop(self.readers);
        drop(self.writer.sender);
        self.writer
            .handle
            .join()
            .map_err(|_| DbError::WriterGone.into())
    }
}

impl OwnerStore {
    /// Failure is isolated in `state`; an existing unusable database is preserved.
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| {
                crate::error::AppError::Directory {
                    path: parent.to_path_buf(),
                    source,
                }
            })?;
        }
        let (runtime, failure) = match Runtime::open(path) {
            Ok(runtime) => (Some(runtime), None),
            Err(error) => (None, Some(error.to_string())),
        };
        Ok(Self {
            path: path.to_path_buf(),
            runtime: RwLock::new(runtime),
            failure: Mutex::new(failure),
        })
    }

    pub fn state(&self) -> OwnerStoreState {
        let runtime = self.runtime.read().unwrap_or_else(|p| p.into_inner());
        if runtime.is_some() {
            OwnerStoreState::Ready
        } else if let Some(reason) = lock(&self.failure).clone() {
            OwnerStoreState::Unavailable { reason }
        } else {
            OwnerStoreState::Closed
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The closure runs on an OS-enforced read-only connection and is not an IPC API.
    pub fn read<T>(&self, action: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let guard = self.runtime.read().unwrap_or_else(|p| p.into_inner());
        let runtime = guard
            .as_ref()
            .ok_or_else(|| not_ready("owner store is closed or unavailable"))?;
        let reader = runtime.readers.get()?;
        action(&reader)
    }

    fn run<T: Send + 'static>(
        &self,
        action: impl FnOnce(&mut Connection) -> AppResult<T> + Send + 'static,
    ) -> AppResult<T> {
        let guard = self.runtime.read().unwrap_or_else(|p| p.into_inner());
        guard
            .as_ref()
            .ok_or_else(|| not_ready("owner store is closed or unavailable"))?
            .run(action)
    }

    pub fn capture_epoch(&self) -> AppResult<AdmissionEpoch> {
        self.read(|conn| {
            let (epoch, open): (i64, bool) = conn
                .query_row(
                    "SELECT epoch,admission_open FROM owner_meta WHERE singleton=1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .during("capture owner admission")?;
            if !open {
                return Err(not_ready("owner admission is closed"));
            }
            Ok(AdmissionEpoch(epoch_value(epoch)?))
        })
    }

    /// Deferred producers must retain this token through final commit/promotion.
    /// The repository is the only caller that supplies domain data to this primitive.
    pub fn write_at_epoch<T: Send + 'static>(
        &self,
        epoch: AdmissionEpoch,
        action: impl FnOnce(&Transaction<'_>) -> AppResult<T> + Send + 'static,
    ) -> AppResult<T> {
        self.run(move |conn| {
            let tx = conn.transaction().during("begin owner transaction")?;
            Self::check_epoch(&tx, epoch)?;
            let result = action(&tx)?;
            Self::check_epoch(&tx, epoch)?;
            tx.commit().during("commit owner transaction")?;
            Ok(result)
        })
    }

    fn check_epoch(conn: &Connection, expected: AdmissionEpoch) -> AppResult<()> {
        let (epoch, open): (i64, bool) = conn
            .query_row(
                "SELECT epoch,admission_open FROM owner_meta WHERE singleton=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .during("check owner admission epoch")?;
        if !open || epoch_value(epoch)? != expected.0 {
            return Err(not_ready("owner operation admission expired"));
        }
        Ok(())
    }
}

impl Drop for OwnerStore {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
