//! Logging and diagnostics plumbing.
//!
//! Kept apart from `privacy` so the dependency runs one way: observability
//! needs the scrubber, the scrubber knows nothing about logging.

mod log_epoch;
pub mod redacting_writer;
pub mod tracing_setup;

pub(crate) use log_epoch::begin_privacy_purge;
pub use redacting_writer::{RedactingMakeWriter, RedactingWriter};
pub use tracing_setup::{init, LogControl};
