//! Persistence. The only module allowed to speak SQL, and the only one that
//! opens the database.
//!
//! One dedicated writer thread owns the single read-write connection; every
//! query runs on a read-only connection from a small pool. Writes accept only
//! `Sanitized<_>`, so an unredacted row is a compile error. Corruption is a
//! state the `Db` can be in, not a reason to fail startup: the application
//! boots, and the user chooses a recovery action.

pub mod baseline_repository;
#[cfg(feature = "devtools")]
pub mod checkpoint_compare_plan_capture;
pub mod checkpoint_repository;
#[cfg(feature = "devtools")]
pub mod checkpoint_window_plan_capture;
mod collector_commit;
pub mod config_snapshot_repository;
pub mod connection;
pub mod db;
pub mod entity_repository;
pub mod event_repository;
pub mod investigations;
pub mod maintenance;
pub mod migrations;
#[cfg(feature = "devtools")]
pub mod page_plan_capture;
pub mod project_repository;
pub mod purge;
pub mod reader_pool;
pub mod recovery;
pub mod search_index;
pub mod settings_repository;
pub mod snapshot_repository;
pub mod system_map_repository;
pub mod watched_location_repository;
pub mod writer;

pub use baseline_repository::Baseline;
pub use collector_commit::{
    AtomicCollectorCommit, AtomicObservation, CollectorCommitReceipt, RelationshipSpec,
    MAX_ATOMIC_CONFIG_SNAPSHOTS, MAX_ATOMIC_CONFIG_SNAPSHOT_BYTES, MAX_ATOMIC_OBSERVATIONS,
    MAX_ATOMIC_RELATED_ENTITIES, MAX_ATOMIC_RELATIONSHIPS,
};
pub use config_snapshot_repository::{ConfigSnapshot, ConfigSnapshotMutation};
pub use db::{Db, DbPaths, DbState};
pub use event_repository::ImportanceCounts;
pub use investigations::{
    AnalysisResultRecord, AnalysisSessionRecord, InvestigationListItem, InvestigationPersistence,
    StoredInvestigation,
};
pub use maintenance::{MaintenanceHandle, MaintenanceReport, StorageUsage};
pub use recovery::{RecoveryAction, RecoveryOutcome};
pub use search_index::{Fts5Index, SearchIndex};
pub use snapshot_repository::Snapshot;
pub use writer::{BatchNotice, CollectorCommitTicket};

use crate::error::{AppResult, DbError};

/// Attaches the operation name to a SQLite failure so the error says what was
/// being done without saying what was being stored.
pub(crate) trait SqlContext<T> {
    /// Map a `rusqlite` failure into the typed error.
    fn during(self, operation: &'static str) -> AppResult<T>;
}

impl<T> SqlContext<T> for rusqlite::Result<T> {
    fn during(self, operation: &'static str) -> AppResult<T> {
        self.map_err(|source| DbError::Sqlite { operation, source }.into())
    }
}

/// Encode a value as a JSON column.
pub(crate) fn to_json<T: serde::Serialize>(value: &T) -> AppResult<String> {
    serde_json::to_string(value).map_err(|error| {
        DbError::Encoding {
            reason: error.to_string(),
        }
        .into()
    })
}

/// Decode a JSON column, naming the column when it does not parse.
pub(crate) fn from_json<T: serde::de::DeserializeOwned>(
    column: &'static str,
    text: &str,
) -> AppResult<T> {
    serde_json::from_str(text).map_err(|_| {
        DbError::Decoding {
            reason: format!("column {column} is not valid JSON"),
        }
        .into()
    })
}

/// A COUNT(*) or pragma value. SQLite integers are signed; a count is not.
pub(crate) fn count_column(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(index)?;
    Ok(u64::try_from(value).unwrap_or(0))
}

/// Parse an enum column, naming the column when the word is unknown.
pub(crate) fn parse_word<T: std::str::FromStr>(column: &'static str, text: &str) -> AppResult<T> {
    text.parse().map_err(|_| {
        DbError::Decoding {
            reason: format!("column {column} holds an unknown word"),
        }
        .into()
    })
}
