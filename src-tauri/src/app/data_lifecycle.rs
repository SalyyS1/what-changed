//! One admission barrier and durable deletion intent for both SQLite stores.
//!
//! A mutation lease belongs to the actual worker, not its IPC future. Purge
//! waits for those workers before touching files, and a completed journal
//! retains the non-sensitive epoch so old work cannot revive after restart.

mod inventory;
mod journal;

use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::domain::{SettingsUpdate, Theme, VisualizationMode};
use crate::error::{AppError, AppResult, DbError};
use crate::owner_workspace::{OwnerStore, OwnerStoreState};
use crate::storage::{settings_repository, Db};

use super::paths::AppPaths;
use journal::PurgeJournal;

const DRAIN_TIMEOUT: Duration = Duration::from_secs(30);

struct AdmissionState {
    record: PurgeJournal,
    closed: bool,
    shutting_down: bool,
    active: usize,
}

pub struct DataLifecycle {
    journal: PathBuf,
    state: Mutex<AdmissionState>,
    idle: Condvar,
    operation: Mutex<()>,
}

/// Owned and Send, so cancelling an IPC future cannot untrack its detached job.
pub struct MutationLease {
    lifecycle: Arc<DataLifecycle>,
    epoch: u64,
}

impl MutationLease {
    /// Recheck immediately before promoting a file or committing prepared work.
    /// Holding this lease also prevents deletion until the worker has returned.
    pub fn validate(&self) -> AppResult<()> {
        let state = lock(&self.lifecycle.state);
        if state.closed || state.record.epoch != self.epoch {
            return Err(recovery_error(
                "mutation admission is closed or its epoch expired",
            ));
        }
        Ok(())
    }
}

impl Drop for MutationLease {
    fn drop(&mut self) {
        let mut state = lock(&self.lifecycle.state);
        state.active = state.active.saturating_sub(1);
        if state.active == 0 {
            self.lifecycle.idle.notify_all();
        }
    }
}

impl DataLifecycle {
    pub(crate) fn runner_profile_guard(data_root: &std::path::Path) -> AppResult<std::fs::File> {
        let root = data_root.join("owner-temp-profiles");
        inventory::directory_chain(&root)?;
        std::fs::create_dir_all(&root)
            .map_err(|_| recovery_error("runner profile root unavailable"))?;
        let _pin = crate::platform::managed_file_lock::pin_directory(&root)
            .map_err(|_| recovery_error("runner profile root unavailable"))?;
        crate::platform::managed_file_lock::operation_lock(&root.join("session.lock"))
            .map_err(|_| recovery_error("another preview process owns the temporary profiles"))
    }
    pub(crate) fn recover_runner_profiles(data_root: &std::path::Path) -> AppResult<()> {
        let root = data_root.join("owner-temp-profiles");
        for entry in std::fs::read_dir(&root)
            .map_err(|_| recovery_error("runner profile recovery unavailable"))?
        {
            let path = entry
                .map_err(|_| recovery_error("runner profile recovery unavailable"))?
                .path();
            if path.file_name().and_then(|name| name.to_str()) == Some("session.lock") {
                continue;
            }
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_none_or(|name| uuid::Uuid::parse_str(name).is_err())
            {
                return Err(recovery_error(
                    "unknown temporary runner profile entry; inspect app data",
                ));
            }
            inventory::remove_tree(&path)?;
        }
        Ok(())
    }
    pub(crate) fn prepare_runner_artifacts(root: &std::path::Path) -> AppResult<()> {
        inventory::directory_chain(root)?;
        std::fs::create_dir_all(root)
            .map_err(|_| recovery_error("runner artifact directory could not be created"))?;
        inventory::directory_chain(root)
    }
    /// Same app-owned profile inventory as privacy deletion, with a UUID leaf.
    pub(crate) fn prepare_runner_profile(
        data_root: &std::path::Path,
        run_id: &str,
    ) -> AppResult<()> {
        crate::owner_workspace::validation::uuid(run_id, "runner profile id")?;
        inventory::directory_chain(data_root)?;
        let parent = data_root.join("owner-temp-profiles");
        inventory::directory_chain(&parent)?;
        std::fs::create_dir_all(&parent)
            .map_err(|_| recovery_error("runner profile root could not be created"))?;
        let _pin = crate::platform::managed_file_lock::pin_directory(&parent)
            .map_err(|_| recovery_error("runner profile root could not be pinned"))?;
        std::fs::create_dir(parent.join(run_id)).map_err(|_| {
            recovery_error("one-use runner profile already exists or could not be created")
        })
    }
    pub(crate) fn cleanup_runner_profile(
        data_root: &std::path::Path,
        run_id: &str,
    ) -> AppResult<()> {
        crate::owner_workspace::validation::uuid(run_id, "runner profile id")?;
        inventory::directory_chain(data_root)?;
        inventory::remove_tree(&data_root.join("owner-temp-profiles").join(run_id))
    }
    /// Raw recovery precedes SQLite open, integrity inspection and migration.
    pub fn prepare(paths: &AppPaths) -> AppResult<Self> {
        inventory::validate_layout(paths)?;
        let journal = paths.purge_journal();
        let record = journal::load(&journal)?.unwrap_or_default();
        // Resolve a whole restore generation before either SQLite file opens.
        // Pending deletion wins and removes only managed generation copies.
        super::data_restore::recover_before_open(paths)?;
        if record.pending {
            let _logs = crate::observability::begin_privacy_purge()?;
            inventory::remove_owned(paths, true)?;
        }
        Ok(Self {
            journal,
            state: Mutex::new(AdmissionState {
                record,
                closed: true,
                shutting_down: false,
                active: 0,
            }),
            idle: Condvar::new(),
            operation: Mutex::new(()),
        })
    }

    /// Called before any maintenance thread or producer starts. Unavailable
    /// stores remain isolated on ordinary starts; pending deletion requires
    /// both fresh stores to be usable before the journal can be completed.
    pub fn finish_startup(&self, legacy: &Db, owner: &OwnerStore) -> AppResult<()> {
        let record = lock(&self.state).record.clone();
        if record.pending {
            if !legacy.state().is_ready() || !matches!(owner.state(), OwnerStoreState::Ready) {
                return Err(recovery_error(
                    "a fresh store is unavailable after interrupted deletion",
                ));
            }
            legacy.write(move |conn| {
                settings_repository::update(
                    conn,
                    &SettingsUpdate {
                        theme: Some(record.theme),
                        visualization: Some(record.visualization),
                        ..Default::default()
                    },
                )
            })?;
        }
        legacy.close_write_admission(record.epoch)?;
        if matches!(owner.state(), OwnerStoreState::Ready) {
            owner.synchronize_epoch(record.epoch)?;
        }
        if record.pending {
            self.complete_record(record.epoch)?;
        }
        self.reopen_stores(legacy, owner, record.epoch)?;
        lock(&self.state).closed = false;
        Ok(())
    }

    pub fn admit(self: &Arc<Self>) -> AppResult<MutationLease> {
        let mut state = lock(&self.state);
        if state.closed {
            return Err(recovery_error("application mutation admission is closed"));
        }
        state.active = state
            .active
            .checked_add(1)
            .ok_or_else(|| recovery_error("too many active mutations"))?;
        Ok(MutationLease {
            lifecycle: Arc::clone(self),
            epoch: state.record.epoch,
        })
    }

    pub fn epoch(&self) -> u64 {
        lock(&self.state).record.epoch
    }

    pub fn is_closed(&self) -> bool {
        lock(&self.state).closed
    }

    pub(super) fn reopen_global(&self) -> AppResult<()> {
        let mut state = lock(&self.state);
        if state.record.pending || state.shutting_down {
            return Err(recovery_error(
                "data lifecycle cannot reopen before completion",
            ));
        }
        state.closed = false;
        Ok(())
    }

    /// Purge and shutdown cannot interleave their handle/file transitions.
    pub fn lock_operation(&self) -> MutexGuard<'_, ()> {
        lock(&self.operation)
    }

    fn begin_purge(&self, theme: Theme, visualization: VisualizationMode) -> AppResult<u64> {
        let mut state = lock(&self.state);
        if state.shutting_down {
            return Err(recovery_error("the application is shutting down"));
        }
        state.closed = true;
        if state.record.pending {
            // Retry the same intent, including its reviewed preferences.
            journal::save(&self.journal, &state.record)?;
            return Ok(state.record.epoch);
        }
        let epoch = state
            .record
            .epoch
            .checked_add(1)
            .filter(|value| *value <= i64::MAX as u64)
            .ok_or_else(|| recovery_error("the purge epoch is exhausted"))?;
        let record = PurgeJournal {
            epoch,
            pending: true,
            theme,
            visualization,
            ..state.record.clone()
        };
        journal::save(&self.journal, &record)?;
        state.record = record;
        Ok(epoch)
    }

    fn complete_record(&self, epoch: u64) -> AppResult<()> {
        let mut state = lock(&self.state);
        if !state.closed || !state.record.pending || state.record.epoch != epoch {
            return Err(recovery_error("the purge epoch is not active"));
        }
        let record = PurgeJournal {
            pending: false,
            ..state.record.clone()
        };
        journal::save(&self.journal, &record)?;
        state.record = record;
        Ok(())
    }

    /// Shutdown does not create a pending journal or change the durable epoch.
    pub fn begin_shutdown(&self) {
        let mut state = lock(&self.state);
        state.closed = true;
        state.shutting_down = true;
    }

    pub fn wait_for_idle(&self) -> AppResult<()> {
        self.wait_for_idle_with_timeout(DRAIN_TIMEOUT)
    }

    fn wait_for_idle_with_timeout(&self, timeout: Duration) -> AppResult<()> {
        let began = Instant::now();
        let mut state = lock(&self.state);
        while state.active > 0 {
            let remaining = timeout.saturating_sub(began.elapsed());
            if remaining.is_zero() {
                return Err(recovery_error(
                    "active mutation workers did not drain; deletion remains pending",
                ));
            }
            (state, _) = self
                .idle
                .wait_timeout(state, remaining)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        Ok(())
    }

    fn reopen_stores(&self, legacy: &Db, owner: &OwnerStore, epoch: u64) -> AppResult<()> {
        if matches!(owner.state(), OwnerStoreState::Ready) {
            owner.reopen_admission()?;
        }
        if let Err(error) = legacy.reopen_write_admission(epoch) {
            if matches!(owner.state(), OwnerStoreState::Ready)
                && owner.synchronize_epoch(epoch).is_err()
            {
                let _ = owner.shutdown();
            }
            return Err(error);
        }
        Ok(())
    }

    pub fn flush_owner(owner: &OwnerStore) -> AppResult<()> {
        match owner.state() {
            OwnerStoreState::Ready => owner.flush(),
            OwnerStoreState::Unavailable { .. } | OwnerStoreState::Closed => Ok(()),
        }
    }

    pub fn shutdown_owner(owner: &OwnerStore) -> AppResult<()> {
        owner.shutdown()
    }

    pub fn shutdown_legacy(db: &Db) {
        db.close();
    }
}

/// The app owns producer handles, while this coordinator owns durable intent.
/// The callback must stop background admission, drain command leases, and only
/// then reset collector instances. Purge itself must never hold a mutation lease.
pub fn purge_stores(
    lifecycle: &DataLifecycle,
    paths: &AppPaths,
    legacy: &Db,
    owner: &OwnerStore,
    stop_and_drain: impl FnOnce() -> AppResult<()>,
    preserved_theme: &str,
    preserved_visualization: &str,
) -> AppResult<()> {
    let theme = serde_json::from_str(preserved_theme)
        .map_err(|_| recovery_error("preserved theme is not a supported appearance value"))?;
    let visualization = serde_json::from_str(preserved_visualization).map_err(|_| {
        recovery_error("preserved visualization is not a supported appearance value")
    })?;
    // AppState holds the operation lock through producer resumption as well.
    let begun = lifecycle.begin_purge(theme, visualization);
    let epoch = begun
        .as_ref()
        .copied()
        .unwrap_or_else(|_| lifecycle.epoch());
    let owner_closed = if matches!(owner.state(), OwnerStoreState::Ready) {
        owner.synchronize_epoch(epoch)
    } else {
        Ok(())
    };
    if owner_closed.is_err() {
        // A full/broken writer queue must not leave owner admission open when
        // the global operation fails. Removing its runtime is the fallback.
        let _ = owner.shutdown();
    }
    let legacy_closed = legacy.close_write_admission(epoch);
    let producers_stopped = stop_and_drain();
    // Even a journal or store-fence error first closes every other admission
    // and stops producers. No deletion occurs unless all four steps succeeded.
    begun?;
    owner_closed?;
    legacy_closed?;
    producers_stopped?;
    lifecycle.wait_for_idle()?;
    let _logs = crate::observability::begin_privacy_purge()?;
    inventory::validate_owned(paths, true)?;
    owner.reset_after_purge(epoch)?;
    let record = lock(&lifecycle.state).record.clone();
    let theme = journal::appearance_json(&record.theme)?;
    let visualization = journal::appearance_json(&record.visualization)?;
    legacy.replace_history(&theme, &visualization)?;
    if !legacy.state().is_ready() {
        return Err(recovery_error(
            "the replacement legacy database is not usable",
        ));
    }
    // Live stores have just reopened. Remove only obsolete generations here.
    inventory::remove_owned(paths, false)?;
    lifecycle.complete_record(epoch)?;
    lifecycle.reopen_stores(legacy, owner, epoch)?;
    Ok(())
}

fn recovery_error(reason: &str) -> AppError {
    DbError::Recovery {
        action: "coordinate application data lifecycle",
        reason: reason.to_owned(),
    }
    .into()
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests;
