//! Preserve the primary operation error and any observable rollback failure.

use rusqlite::Transaction;
use what_changed_lib::error::{AppError, DbError};

pub(crate) struct GroupFailure {
    causes: Box<FailureCauses>,
}

struct FailureCauses {
    primary: AppError,
    rollback: Option<AppError>,
}

impl GroupFailure {
    /// Only a failure allocates; successful grouped state stays unboxed.
    pub fn new(primary: AppError, rollback: Option<AppError>) -> Self {
        Self {
            causes: Box::new(FailureCauses { primary, rollback }),
        }
    }

    pub fn primary(&self) -> &AppError {
        &self.causes.primary
    }

    pub fn rollback(&self) -> Option<&AppError> {
        self.causes.rollback.as_ref()
    }
}

impl From<AppError> for GroupFailure {
    fn from(primary: AppError) -> Self {
        Self::new(primary, None)
    }
}

impl std::fmt::Display for GroupFailure {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(output, "{}", self.primary())?;
        if let Some(rollback) = self.rollback() {
            write!(output, "; rollback also failed: {rollback}")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for GroupFailure {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, output)
    }
}

pub(super) fn abandon(tx: Transaction<'_>, primary: AppError) -> GroupFailure {
    // SQLite may already have ended the transaction (for example RAISE(ROLLBACK)).
    // Otherwise explicit rollback surfaces errors that Transaction::Drop hides.
    let rollback = if tx.is_autocommit() {
        drop(tx);
        None
    } else {
        tx.rollback().err().map(|source| {
            AppError::Db(DbError::Sqlite {
                operation: "roll back a grouped synthetic seed transaction",
                source,
            })
        })
    };
    GroupFailure::new(primary, rollback)
}
