#![cfg(test)]

use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use rusqlite::Connection;
use what_changed_lib::error::{AppError, AppResult, DbError};
use what_changed_lib::owner_workspace::OwnerStore;

/// A real acquired reader or active writer transaction. Channels establish the
/// acquisition order; Drop releases and joins on assertion failure as well.
pub struct HeldOperation {
    release: mpsc::Sender<()>,
    worker: Option<JoinHandle<AppResult<i64>>>,
}

impl HeldOperation {
    pub fn writer(store: Arc<OwnerStore>) -> Self {
        let epoch = store.capture_epoch().unwrap();
        Self::start(move |entered, release| {
            store.write_at_epoch(epoch, move |tx| hold(tx, entered, release))
        })
    }

    pub fn reader(store: Arc<OwnerStore>) -> Self {
        Self::start(move |entered, release| store.read(|conn| hold(conn, entered, release)))
    }

    fn start(
        action: impl FnOnce(mpsc::Sender<()>, mpsc::Receiver<()>) -> AppResult<i64> + Send + 'static,
    ) -> Self {
        let (entered, acquired) = mpsc::channel();
        let (release, waiting) = mpsc::channel();
        let held = Self {
            release,
            worker: Some(thread::spawn(move || action(entered, waiting))),
        };
        acquired.recv_timeout(Duration::from_secs(30)).unwrap();
        held
    }

    pub fn finish(mut self) -> AppResult<i64> {
        let _ = self.release.send(());
        self.worker.take().unwrap().join().unwrap()
    }
}

impl Drop for HeldOperation {
    fn drop(&mut self) {
        let _ = self.release.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn hold(
    conn: &Connection,
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
) -> AppResult<i64> {
    let count = conn
        .query_row("SELECT count(*) FROM owner_apps", [], |row| row.get(0))
        .map_err(|source| DbError::Sqlite {
            operation: "read capacity fixture app count",
            source,
        })?;
    let synchronization_error = || AppError::InvalidInput {
        what: "capacity fixture",
        reason: "controlled operation synchronization failed".into(),
    };
    entered.send(()).map_err(|_| synchronization_error())?;
    release
        .recv_timeout(Duration::from_secs(60))
        .map_err(|_| synchronization_error())?;
    Ok(count)
}
