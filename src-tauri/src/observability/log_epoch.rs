//! Privacy generations for the asynchronous log queue.
//!
//! Capture an epoch when formatting starts, not when a delayed fragment finally
//! reaches the queue. The file-side gate rejects stale/paused frames and flushes
//! each accepted frame before releasing its lock. A purge can then truncate
//! files without an older queued or partially formatted line reviving later.

use std::io::{self, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use tracing_subscriber::fmt::MakeWriter;

use super::{RedactingMakeWriter, RedactingWriter};
use crate::error::{AppError, AppResult};

const MAGIC: &[u8; 8] = b"WCLOGv1\0";
const HEADER_BYTES: usize = 16;
static ACTIVE: OnceLock<Arc<LogEpoch>> = OnceLock::new();

#[derive(Debug, Default)]
struct EpochState {
    epoch: u64,
    paused: bool,
    flush_error: Option<String>,
    discarded_frames: u64,
}

#[derive(Debug, Default)]
pub(super) struct LogEpoch {
    state: Mutex<EpochState>,
    /// Formatting never waits for disk I/O held under the file-side gate.
    published_epoch: AtomicU64,
}

impl LogEpoch {
    fn snapshot(&self) -> u64 {
        self.published_epoch.load(Ordering::Acquire)
    }

    fn pause(self: &Arc<Self>) -> AppResult<PurgeLogGuard> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| logging_error("the file log gate is poisoned"))?;
        if state.paused {
            return Err(logging_error("another privacy log barrier is active"));
        }
        if let Some(reason) = state.flush_error.as_ref() {
            return Err(logging_error(&format!(
                "the file log writer has not flushed; retry deletion after recovery: {reason}"
            )));
        }
        // Reserve the second increment for Drop, which resumes a fresh epoch
        // on success and failure alike. Logging is never permanently disabled.
        if state.epoch > u64::MAX - 2 {
            return Err(logging_error("the file log privacy epoch is exhausted"));
        }
        state.epoch += 1;
        state.paused = true;
        self.published_epoch.store(state.epoch, Ordering::Release);
        Ok(PurgeLogGuard {
            epoch: Arc::clone(self),
        })
    }
}

pub(crate) struct PurgeLogGuard {
    epoch: Arc<LogEpoch>,
}

impl Drop for PurgeLogGuard {
    fn drop(&mut self) {
        let mut state = self
            .epoch
            .state
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.epoch = state.epoch.saturating_add(1);
        state.paused = false;
        self.epoch
            .published_epoch
            .store(state.epoch, Ordering::Release);
    }
}

pub(super) fn register(epoch: Arc<LogEpoch>) -> AppResult<()> {
    ACTIVE
        .set(epoch)
        .map_err(|_| logging_error("file log lifecycle was already initialized"))
}

/// Unit-test AppStates have no global subscriber. A real initialized file
/// logger always participates; an unhealthy gate returns an explicit error.
pub(crate) fn begin_privacy_purge() -> AppResult<Option<PurgeLogGuard>> {
    ACTIVE.get().map(LogEpoch::pause).transpose()
}

#[derive(Clone)]
pub(super) struct EpochMakeWriter<W: Write + Clone> {
    queue: W,
    epoch: Arc<LogEpoch>,
}

impl<W: Write + Clone> EpochMakeWriter<W> {
    pub(super) fn new(queue: W, epoch: Arc<LogEpoch>) -> Self {
        Self { queue, epoch }
    }
}

impl<'a, W: Write + Clone + 'a> MakeWriter<'a> for EpochMakeWriter<W> {
    type Writer = RedactingWriter<FramedQueue<W>>;

    fn make_writer(&'a self) -> Self::Writer {
        RedactingMakeWriter::new(FramedQueue {
            queue: self.queue.clone(),
            epoch: self.epoch.snapshot(),
            pending: Vec::new(),
        })
        .make_writer()
    }
}

/// One queue message per complete, already-redacted line prevents lines from
/// different formatter instances interleaving their text/newline fragments.
pub(super) struct FramedQueue<W: Write> {
    queue: W,
    epoch: u64,
    pending: Vec<u8>,
}

impl<W: Write> Write for FramedQueue<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(bytes);
        while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
            let mut frame = Vec::with_capacity(HEADER_BYTES + end + 1);
            frame.extend_from_slice(MAGIC);
            frame.extend_from_slice(&self.epoch.to_le_bytes());
            frame.extend(self.pending.drain(..=end));
            self.queue.write_all(&frame)?;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.queue.flush()
    }
}

pub(super) struct EpochSink<W: Write> {
    writer: W,
    epoch: Arc<LogEpoch>,
}

impl<W: Write> EpochSink<W> {
    pub(super) fn new(writer: W, epoch: Arc<LogEpoch>) -> Self {
        Self { writer, epoch }
    }
}

impl<W: Write> Write for EpochSink<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() < HEADER_BYTES || &bytes[..MAGIC.len()] != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unframed file log buffer",
            ));
        }
        let mut encoded = [0; 8];
        encoded.copy_from_slice(&bytes[MAGIC.len()..HEADER_BYTES]);
        let captured = u64::from_le_bytes(encoded);
        let mut state = self
            .epoch
            .state
            .lock()
            .map_err(|_| io::Error::other("file log gate poisoned"))?;
        if state.paused || captured != state.epoch {
            state.discarded_frames = state.discarded_frames.saturating_add(1);
            return Ok(bytes.len());
        }
        // Flush while still fenced, including any buffering in the rolling
        // appender. A successful pause proves no old buffered bytes remain.
        match self
            .writer
            .write_all(&bytes[HEADER_BYTES..])
            .and_then(|_| self.writer.flush())
        {
            Ok(()) => {
                state.flush_error = None;
                Ok(bytes.len())
            }
            Err(error) => {
                state.flush_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut state = self
            .epoch
            .state
            .lock()
            .map_err(|_| io::Error::other("file log gate poisoned"))?;
        if state.paused {
            return Ok(());
        }
        match self.writer.flush() {
            Ok(()) => {
                state.flush_error = None;
                Ok(())
            }
            Err(error) => {
                state.flush_error = Some(error.to_string());
                Err(error)
            }
        }
    }
}

fn logging_error(reason: &str) -> AppError {
    AppError::Logging {
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
#[path = "log_epoch_tests.rs"]
mod tests;
