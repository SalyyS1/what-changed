//! Typed application errors.
//!
//! One error enum for the whole backend: a caller that has to guess from a
//! string what went wrong cannot recover, and a command that returns a string
//! cannot be matched on from TypeScript. Variants carry the operation that
//! failed, never the value that failed — an error message is a log line, and a
//! log line must not become a way to smuggle a secret past redaction.

use std::path::PathBuf;

/// Anything the backend can fail at.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// A path the app owns could not be created or written.
    #[error("could not prepare the directory {path}: {source}")]
    Directory {
        /// The directory the app tried to prepare.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },

    /// A file the app owns could not be read, written, renamed or removed.
    #[error("could not {operation} {path}: {source}")]
    File {
        /// What was being done to it.
        operation: &'static str,
        /// The file. Paths under the app's own data directory are not secrets.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: std::io::Error,
    },

    /// The logging stack could not be installed.
    #[error("could not start logging: {reason}")]
    Logging {
        /// What failed, in terms a maintainer can act on.
        reason: String,
    },

    /// A log level string from settings or the environment was not a level.
    #[error("'{value}' is not a log level")]
    LogLevel {
        /// The rejected value. Log levels are a closed set of public words, so
        /// echoing one back leaks nothing.
        value: String,
    },

    /// A command rejected its input before doing anything with it.
    ///
    /// `what` names the parameter, `reason` explains the rule that was broken.
    /// Neither ever echoes the value: a rejected path is still a path.
    #[error("{what} was rejected: {reason}")]
    InvalidInput {
        /// Which parameter.
        what: &'static str,
        /// Which rule it broke, in words a person can act on.
        reason: String,
    },

    /// A command exists and is registered, but the phase that implements it has
    /// not landed. Listed in `ROADMAP.md` under "Tracked stubs" — never silent.
    #[error("this is not built yet; it lands in {phase}")]
    NotImplemented {
        /// The phase that owns the implementation, e.g. `phase-06`.
        phase: &'static str,
    },

    /// A command the frontend may call freely was called too often.
    #[error("{what} was called too often; try again shortly")]
    RateLimited {
        /// Which command.
        what: &'static str,
    },

    /// An operating-system call the product depends on failed.
    ///
    /// The operation is named and the platform's own code is reported. What the
    /// call was about and what it would have returned are not, because both can
    /// carry a path or a secret, and an error message is a log line.
    #[error("{operation} failed: {reason}")]
    Platform {
        /// Which operating-system operation.
        operation: &'static str,
        /// What the platform said, reduced to a code or a fixed phrase.
        reason: String,
    },

    /// Docker returned a non-success HTTP status over its local named pipe.
    ///
    /// The response body is deliberately absent: daemon error text is
    /// untrusted and must never become a log message. Collector policy may
    /// match the closed numeric status to negotiate an older API version.
    #[error("Docker Engine rejected a request with HTTP {status}")]
    DockerHttpStatus {
        /// The three-digit HTTP response status.
        status: u16,
    },

    /// The database, its migrations, its writer or its recovery.
    #[error("database: {0}")]
    Db(#[from] DbError),
}

/// A database failure. Each variant names the operation, never a row.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// SQLite refused a statement.
    #[error("{operation}: {source}")]
    Sqlite {
        /// The repository operation that was running.
        operation: &'static str,
        /// What SQLite said. Its messages name columns and constraints, not
        /// values.
        #[source]
        source: rusqlite::Error,
    },

    /// A migration could not be applied.
    #[error("migration {version} failed: {reason}")]
    Migration {
        /// The schema version that failed.
        version: usize,
        /// Why, from the migration runner.
        reason: String,
    },

    /// The SQL recorded for an applied migration differs from what this build
    /// ships. Someone edited a migration after it had run somewhere.
    #[error("migration {version} on disk was applied from different SQL than this build ships")]
    Drift {
        /// The schema version whose checksum disagrees.
        version: usize,
    },

    /// The database was written by a newer build.
    #[error("the database is at schema version {found}; this build knows up to {known}")]
    Ahead {
        /// Version found in the file.
        found: usize,
        /// Latest version this build defines.
        known: usize,
    },

    /// The vendored SQLite was built without a feature the schema needs.
    #[error("the bundled SQLite lacks {option}")]
    MissingCompileOption {
        /// The compile option that should have been present.
        option: &'static str,
    },

    /// The database is corrupt, unavailable, or being recovered.
    #[error("the database is not available: {state}")]
    NotReady {
        /// The state it is in, for the message.
        state: String,
    },

    /// The write queue was full; the write was dropped and counted.
    #[error("the write queue is full; the write was dropped")]
    QueueFull,

    /// The writer thread has stopped, so nothing can be written.
    #[error("the database writer has stopped")]
    WriterGone,

    /// A value could not be turned into a column.
    #[error("could not encode a row: {reason}")]
    Encoding {
        /// From the serializer. It names a type or a field, not a value.
        reason: String,
    },

    /// A column could not be turned back into a value.
    #[error("a stored row could not be read: {reason}")]
    Decoding {
        /// What did not parse, by name.
        reason: String,
    },

    /// A recovery action did not complete.
    #[error("recovery could not {action}: {reason}")]
    Recovery {
        /// The action.
        action: &'static str,
        /// Why.
        reason: String,
    },
}

/// Result type used across the backend.
pub type AppResult<T> = Result<T, AppError>;
