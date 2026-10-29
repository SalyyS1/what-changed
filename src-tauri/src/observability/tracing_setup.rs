//! Tracing setup: the one place the logging stack is assembled.
//!
//! Two sinks, both redacting: a daily rolling file in the app's log directory,
//! and the console in development only. Verbosity is a reloadable filter so it
//! can be raised from settings while the app runs — a change recorder that has
//! to be restarted to be diagnosed is a change recorder you cannot diagnose.

use std::fs;
#[cfg(debug_assertions)]
use std::io;
use std::path::Path;
use std::sync::Arc;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Registry;

use super::log_epoch::{self, EpochMakeWriter, EpochSink, LogEpoch};
#[cfg(debug_assertions)]
use super::redacting_writer::RedactingMakeWriter;
use crate::error::{AppError, AppResult};

/// Environment variable that overrides verbosity for one run.
const FILTER_ENV: &str = "WHAT_CHANGED_LOG";

/// Base name of the rolling log file.
const LOG_FILE_PREFIX: &str = "what-changed";

/// How many daily files to keep. A week is enough to explain a Monday problem
/// reported on a Friday, and small enough that logs never become the biggest
/// thing on disk.
const KEPT_FILES: usize = 7;
/// Bound queued complete lines without blocking application threads on disk.
const QUEUED_LOG_LINES: usize = 1024;

/// Handle for changing verbosity after startup.
pub struct LogControl {
    handle: reload::Handle<EnvFilter, Registry>,
    /// Flushes the non-blocking file writer when dropped; the app must hold it
    /// for as long as it wants logs.
    _guard: WorkerGuard,
}

impl LogControl {
    /// Replace the active filter, e.g. `"info,what_changed_lib=trace"`.
    pub fn set_filter(&self, directives: &str) -> AppResult<()> {
        let filter = EnvFilter::try_new(directives).map_err(|_| AppError::LogLevel {
            value: directives.to_owned(),
        })?;
        self.handle
            .reload(filter)
            .map_err(|error| AppError::Logging {
                reason: error.to_string(),
            })
    }
}

/// Default verbosity: this crate at debug in a development build, everything
/// else at info, and nothing from dependencies below that.
fn default_directives() -> &'static str {
    if cfg!(debug_assertions) {
        "info,what_changed_lib=debug"
    } else {
        "info"
    }
}

/// Install the logging stack. Call once, before anything that might log.
///
/// The console layer exists only in development builds: a tray application has
/// no console to read, and stdout in a release build is a sink nobody sees.
pub fn init(log_dir: &Path) -> AppResult<LogControl> {
    fs::create_dir_all(log_dir).map_err(|source| AppError::Directory {
        path: log_dir.to_path_buf(),
        source,
    })?;

    let directives = std::env::var(FILTER_ENV).unwrap_or_else(|_| default_directives().to_owned());
    let filter =
        EnvFilter::try_new(&directives).map_err(|_| AppError::LogLevel { value: directives })?;
    let (filter, handle) = reload::Layer::new(filter);

    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix("log")
        .max_log_files(KEPT_FILES)
        .build(log_dir)
        .map_err(|error| AppError::Logging {
            reason: error.to_string(),
        })?;
    let epoch = Arc::new(LogEpoch::default());
    let (file, guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
        .buffered_lines_limit(QUEUED_LOG_LINES)
        .finish(EpochSink::new(appender, Arc::clone(&epoch)));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_writer(EpochMakeWriter::new(file, Arc::clone(&epoch)));

    let registry = Registry::default().with(filter).with(file_layer);

    // The console sink is a second redacting writer rather than a plain one:
    // the rule is about the writer, so every writer has to obey it.
    #[cfg(debug_assertions)]
    let registry = registry.with(
        tracing_subscriber::fmt::layer()
            .with_target(false)
            .with_writer(RedactingMakeWriter::new(WriteAdapter(io::stdout()))),
    );

    registry.try_init().map_err(|error| AppError::Logging {
        reason: error.to_string(),
    })?;
    log_epoch::register(epoch)?;

    Ok(LogControl {
        handle,
        _guard: guard,
    })
}

/// Adapts a writer that is only `Write` by value into the shared-sink shape the
/// redacting writer holds. Nothing here changes the bytes.
#[derive(Debug)]
#[cfg(debug_assertions)]
struct WriteAdapter<W: io::Write>(W);

#[cfg(debug_assertions)]
impl<W: io::Write> io::Write for WriteAdapter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_directives_name_this_crate_in_development() {
        let directives = default_directives();
        assert!(directives.starts_with("info"));
        if cfg!(debug_assertions) {
            assert!(directives.contains("what_changed_lib"));
        }
    }

    #[test]
    fn a_filter_string_that_is_not_a_filter_is_rejected() {
        assert!(EnvFilter::try_new("info,what_changed_lib=debug").is_ok());
        assert!(EnvFilter::try_new("=not a level=").is_err());
    }
}
