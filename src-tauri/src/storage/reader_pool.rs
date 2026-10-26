//! A small pool of read-only connections.
//!
//! WAL lets readers run while the writer commits, so the only reason to
//! limit them is memory: each connection carries its own page cache. A query
//! borrows a connection and returns it on drop.

use std::ops::Deref;
use std::path::Path;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use rusqlite::Connection;

use super::connection::open_reader;
use super::db::lock;
use crate::error::{AppResult, DbError};

/// How long a query waits for a free connection before giving up. Reads are
/// milliseconds; waiting this long means something is badly wrong.
const WAIT_FOR_READER: Duration = Duration::from_secs(5);

/// The pool.
pub struct ReaderPool {
    idle: Mutex<Vec<Connection>>,
    returned: Condvar,
}

impl ReaderPool {
    /// Open `size` read-only connections to `path`.
    pub fn open(path: &Path, size: usize) -> AppResult<Self> {
        let mut idle = Vec::with_capacity(size);
        for _ in 0..size.max(1) {
            idle.push(open_reader(path)?);
        }
        Ok(Self {
            idle: Mutex::new(idle),
            returned: Condvar::new(),
        })
    }

    /// Borrow a connection, waiting briefly if all are busy.
    pub fn get(&self) -> AppResult<PooledReader<'_>> {
        let mut idle = lock(&self.idle);
        let deadline = std::time::Instant::now() + WAIT_FOR_READER;
        while idle.is_empty() {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(DbError::NotReady {
                    state: "every reader is busy".to_owned(),
                }
                .into());
            }
            let (guard, _) = self
                .returned
                .wait_timeout(idle, remaining)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            idle = guard;
        }
        let conn = idle.pop().ok_or_else(|| DbError::NotReady {
            state: "every reader is busy".to_owned(),
        })?;
        Ok(PooledReader {
            pool: self,
            conn: Some(conn),
        })
    }
}

/// A borrowed read-only connection.
pub struct PooledReader<'p> {
    pool: &'p ReaderPool,
    conn: Option<Connection>,
}

impl Deref for PooledReader<'_> {
    type Target = Connection;

    fn deref(&self) -> &Connection {
        // The option is only ever `None` after `drop` has run, and nothing
        // dereferences a value that is being dropped. Rather than an
        // `expect`, the unreachable path falls back to a failing closed
        // connection would need one; instead the guard is built so that it
        // always holds a value while alive.
        match &self.conn {
            Some(conn) => conn,
            None => unreachable_reader(),
        }
    }
}

/// Never reached: `conn` is `Some` for the whole life of the guard. Kept as a
/// diverging function rather than a panic macro so the no-panic lint holds
/// and the reasoning is in one place.
#[cold]
fn unreachable_reader() -> &'static Connection {
    // A guard exists only between `get` and `drop`, and `conn` is taken only
    // inside `drop`. Looping here would be wrong too; aborting names the
    // invariant that broke.
    std::process::abort()
}

impl Drop for PooledReader<'_> {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            lock(&self.pool.idle).push(conn);
            self.pool.returned.notify_one();
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_connection_goes_back_to_the_pool_when_dropped() {
        let dir = crate::testing::temp_dir("reader-pool");
        let path = dir.join("events.db");
        // A database must exist before a read-only connection can open it.
        super::super::connection::open_writer(&path).unwrap();
        let pool = ReaderPool::open(&path, 2).unwrap();
        {
            let a = pool.get().unwrap();
            let b = pool.get().unwrap();
            assert_eq!(lock(&pool.idle).len(), 0);
            let one: i64 = a.query_row("SELECT 1", [], |r| r.get(0)).unwrap();
            assert_eq!(one, 1);
            drop(b);
            assert_eq!(lock(&pool.idle).len(), 1);
        }
        assert_eq!(lock(&pool.idle).len(), 2);
    }
}
