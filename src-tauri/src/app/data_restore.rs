//! Closed-store generation backups and recovery. No IPC accepts filesystem paths.
//!
//! Startup must resolve pending purge first, then call recover_before_open,
//! before opening either store. Runtime callers stop/drain producers, await both
//! store closes, then acquire ClosedStores; they must not hold the operation lock.

mod cleanup;
mod files;
mod inventory;
mod journal;
mod locking;
mod manifest;
mod switch;

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;

use serde::{Deserialize, Serialize};

use super::{AppPaths, DataLifecycle};
use crate::error::{AppError, AppResult, DbError};
use crate::owner_workspace::{OwnerStore, OwnerStoreState};
use crate::platform::managed_file_lock as native;
use crate::storage::Db;

pub(crate) const BACKUPS: &str = "data-backups";
pub(crate) const RESTORES: &str = "data-restores";
pub(crate) const JOURNAL: &str = "data-restore.json";
const LOCK: &str = "data-generation.lock";
const MAX_GENERATIONS: usize = 20;
const MAX_FILES: usize = 4096;
const MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const ITEMS: [&str; 9] = [
    "events.db",
    "events.db-wal",
    "events.db-shm",
    "events.db-journal",
    "owner-workspace.sqlite",
    "owner-workspace.sqlite-wal",
    "owner-workspace.sqlite-shm",
    "owner-workspace.sqlite-journal",
    "owner-artifacts",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BackupId(String);

impl BackupId {
    pub fn parse(value: &str) -> AppResult<Self> {
        let parsed = uuid::Uuid::parse_str(value).map_err(|_| error("generation ID is invalid"))?;
        if parsed.hyphenated().to_string() != value {
            return Err(error("generation ID is not canonical"));
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

/// The operation guard and native namespace lock outlive every generation move.
/// This type is intentionally neither Clone nor accepted from a frontend DTO.
pub struct ClosedStores<'a> {
    paths: AppPaths,
    epoch: u64,
    _operation: Option<MutexGuard<'a, ()>>,
    _namespace: File,
    _parents: Vec<File>,
}

impl<'a> ClosedStores<'a> {
    pub fn after_shutdown(
        paths: &AppPaths,
        lifecycle: &'a DataLifecycle,
        legacy: &Db,
        owner: &OwnerStore,
    ) -> AppResult<Self> {
        let operation = lifecycle.lock_operation();
        if !lifecycle.is_closed()
            || !legacy.is_closed()
            || !matches!(owner.state(), OwnerStoreState::Closed)
        {
            return Err(error(
                "close and await both stores and producer admission before restore",
            ));
        }
        lifecycle.wait_for_idle()?;
        let token = Self::acquire(paths, Some(operation))?;
        if token.epoch != lifecycle.epoch() {
            return Err(error("coordinator purge epoch differs"));
        }
        Ok(token)
    }

    fn acquire(paths: &AppPaths, operation: Option<MutexGuard<'a, ()>>) -> AppResult<Self> {
        files::layout(paths)?;
        let parents = files::pin_chain(&paths.data)?;
        let namespace = io(
            &paths.data.join(LOCK),
            "lock data generation namespace",
            native::operation_lock(&paths.data.join(LOCK)),
        )?;
        let fence = journal::purge_fence(paths)?;
        if fence.pending {
            return Err(error("privacy purge must complete before generation work"));
        }
        Ok(Self {
            paths: paths.clone(),
            epoch: fence.epoch,
            _operation: operation,
            _namespace: namespace,
            _parents: parents,
        })
    }

    fn validate_epoch(&self) -> AppResult<()> {
        let fence = journal::purge_fence(&self.paths)?;
        if fence.pending || fence.epoch != self.epoch {
            return Err(error("privacy purge invalidated this generation operation"));
        }
        Ok(())
    }

    fn backup_path(&self, id: &BackupId) -> AppResult<PathBuf> {
        BackupId::parse(id.as_str())?;
        Ok(self.paths.data.join(BACKUPS).join(id.as_str()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreReceipt {
    pub restored: BackupId,
    pub rollback: BackupId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryOutcome {
    Clean,
    RolledBack,
    Completed,
    PurgeRequired,
}

pub fn create_backup(closed: &ClosedStores<'_>) -> AppResult<BackupId> {
    manifest::create_backup(closed)
}

pub fn restore(closed: &mut ClosedStores<'_>, source: &BackupId) -> AppResult<RestoreReceipt> {
    switch::restore(closed, source)
}

/// Restore's old generation remains a normal complete backup. Selecting it uses
/// the same validated transaction and produces a new rollback generation.
pub fn rollback(
    closed: &mut ClosedStores<'_>,
    prior: &RestoreReceipt,
) -> AppResult<RestoreReceipt> {
    restore(closed, &prior.rollback)
}

pub fn recover_before_open(paths: &AppPaths) -> AppResult<RecoveryOutcome> {
    files::layout(paths)?;
    if !files::exists(&paths.data)? {
        return Ok(RecoveryOutcome::Clean);
    }
    // Pending purge wins even if a restore journal is corrupt or incomplete.
    if journal::purge_fence(paths)?.pending {
        purge_owned_generations(paths)?;
        return Ok(RecoveryOutcome::PurgeRequired);
    }
    let mut closed = ClosedStores::acquire(paths, None)?;
    switch::recover(&mut closed)
}

/// Call with global producer admission closed and drained. This removes exact helper namespaces
/// before interrupted purge can finish; exported files and source roots are absent.
pub fn purge_owned_generations(paths: &AppPaths) -> AppResult<()> {
    files::purge_owned(paths)
}

fn error(reason: &str) -> AppError {
    DbError::Recovery {
        action: "coordinate data restore",
        reason: reason.into(),
    }
    .into()
}

fn io<T>(path: &Path, operation: &'static str, result: std::io::Result<T>) -> AppResult<T> {
    result.map_err(|source| AppError::File {
        operation,
        path: path.into(),
        source,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;
