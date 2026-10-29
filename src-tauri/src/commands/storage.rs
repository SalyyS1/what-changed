//! Storage commands: what the database costs on disk, and recovery.

use tauri::State;

use crate::app::AppState;
use crate::dto::CommandResult;
use crate::storage::{maintenance, RecoveryAction, RecoveryOutcome, StorageUsage};

/// What the database costs on disk.
#[tauri::command]
pub async fn get_storage_usage(state: State<'_, AppState>) -> CommandResult<StorageUsage> {
    let paths = state.db.paths().clone();
    let mut usage = state
        .db
        .read(|conn| maintenance::storage_usage(conn, &paths))?;
    usage.owner_workspace_bytes = state.owner_store.owned_file_bytes();
    Ok(usage)
}

/// Run a recovery action the user chose on the recovery screen.
///
/// Rebuilding or resetting a database is the longest thing a command does —
/// it copies every readable page — so it must not be the thing the window is
/// waiting on.
#[tauri::command]
pub async fn db_recover(
    state: State<'_, AppState>,
    action: RecoveryAction,
) -> CommandResult<RecoveryOutcome> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.db.recover_for_command(action)?)
}
