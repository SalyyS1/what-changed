//! The one error shape that crosses IPC.
//!
//! Three fields, each doing one job. `code` is a closed vocabulary the
//! frontend can branch on — a caller that has to match on a sentence cannot
//! recover. `message` is the sentence a person reads, and it goes through
//! `scrub_line` on the way out, so an error can never become the path a value
//! takes past redaction. `retryable` says whether pressing the button again
//! could plausibly work, which is the only thing an error screen has to decide.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::enums::text_enum;
use crate::error::{AppError, DbError};
use crate::privacy::scrub_line;

text_enum! {
    /// What kind of failure this was. Closed, so TypeScript can exhaust it.
    ErrorCode, "error code" {
        /// A directory the app owns could not be prepared.
        Directory => "directory",
        /// A file the app owns could not be read or written.
        File => "file",
        /// The logging stack.
        Logging => "logging",
        /// The database refused a statement, or a row would not decode.
        Database => "database",
        /// The database is corrupt, mismatched, or being recovered.
        NotReady => "not_ready",
        /// The write queue was full and the write was dropped.
        QueueFull => "queue_full",
        /// A parameter did not pass validation.
        InvalidInput => "invalid_input",
        /// The command is registered but its phase has not landed.
        NotImplemented => "not_implemented",
        /// The command was called too often.
        RateLimited => "rate_limited",
        /// An operating-system call the product depends on failed.
        Platform => "platform",
    }
}

/// A failure, as the webview sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct AppErrorDto {
    /// What kind of failure this was.
    pub code: ErrorCode,
    /// One sentence for the user. Scrubbed.
    pub message: String,
    /// Whether repeating the call could plausibly succeed.
    pub retryable: bool,
}

/// What every command returns.
pub type CommandResult<T> = Result<T, AppErrorDto>;

impl AppErrorDto {
    /// Build the wire error from a backend error.
    pub fn new(code: ErrorCode, message: &str, retryable: bool) -> Self {
        Self {
            code,
            message: scrub_line(message).into_owned(),
            retryable,
        }
    }
}

impl From<AppError> for AppErrorDto {
    fn from(error: AppError) -> Self {
        let (code, retryable) = classify(&error);
        AppErrorDto::new(code, &error.to_string(), retryable)
    }
}

/// The code and retryability of a backend error.
///
/// Retryable means *the same call might work next time without the user
/// changing anything*: a full queue drains, a database being recovered comes
/// back, an operating-system call that failed on buffer sizing or a transient
/// access denial usually succeeds on the next one. A rejected path and an
/// unbuilt phase never fix themselves, so offering "Try again" for them would
/// be a lie the error screen tells.
fn classify(error: &AppError) -> (ErrorCode, bool) {
    match error {
        AppError::Directory { .. } => (ErrorCode::Directory, false),
        AppError::File { .. } => (ErrorCode::File, false),
        AppError::Logging { .. } | AppError::LogLevel { .. } => (ErrorCode::Logging, false),
        AppError::InvalidInput { .. } => (ErrorCode::InvalidInput, false),
        AppError::NotImplemented { .. } => (ErrorCode::NotImplemented, false),
        AppError::RateLimited { .. } => (ErrorCode::RateLimited, true),
        AppError::Platform { .. } | AppError::DockerHttpStatus { .. } => {
            (ErrorCode::Platform, true)
        }
        AppError::Db(db) => match db {
            DbError::NotReady { .. } => (ErrorCode::NotReady, true),
            DbError::QueueFull => (ErrorCode::QueueFull, true),
            DbError::WriterGone => (ErrorCode::NotReady, false),
            _ => (ErrorCode::Database, false),
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_message_that_carries_a_secret_is_scrubbed_before_it_crosses() {
        let error = AppError::InvalidInput {
            what: "path",
            reason: "DB_PASSWORD=hunter2NotReal is not a folder".to_owned(),
        };
        let dto = AppErrorDto::from(error);
        assert!(!dto.message.contains("hunter2NotReal"));
        assert_eq!(dto.code, ErrorCode::InvalidInput);
        assert!(!dto.retryable);
    }

    #[test]
    fn only_the_failures_that_can_pass_next_time_are_retryable() {
        let full = AppErrorDto::from(AppError::from(DbError::QueueFull));
        assert!(full.retryable);
        assert_eq!(full.code, ErrorCode::QueueFull);

        let stub = AppErrorDto::from(AppError::NotImplemented { phase: "phase-14" });
        assert!(!stub.retryable);
        assert!(stub.message.contains("phase-14"));
    }

    #[test]
    fn the_code_vocabulary_is_closed_and_reads_as_snake_case() {
        assert_eq!(ErrorCode::ALL.len(), 10);
        for code in ErrorCode::ALL {
            assert_eq!(code.as_str(), code.as_str().to_lowercase());
        }
    }

    #[test]
    fn a_platform_failure_names_the_operation_and_is_retryable() {
        let dto = AppErrorDto::from(AppError::Platform {
            operation: "enumerate listening sockets",
            reason: "insufficient buffer after three attempts".to_owned(),
        });
        assert_eq!(dto.code, ErrorCode::Platform);
        assert!(dto.retryable);
        assert!(dto.message.contains("enumerate listening sockets"));
        // The message goes through the same scrub as every other error; the
        // leak suite owns proving that, not this vocabulary test.
        assert!(!dto.message.is_empty());
    }

    #[test]
    fn a_docker_status_exposes_only_the_closed_status_code() {
        let dto = AppErrorDto::from(AppError::DockerHttpStatus { status: 400 });
        assert_eq!(dto.code, ErrorCode::Platform);
        assert!(dto.retryable);
        assert_eq!(
            dto.message,
            "Docker Engine rejected a request with HTTP 400"
        );
    }
}
