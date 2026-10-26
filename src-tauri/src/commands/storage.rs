//! Storage commands: what the database costs on disk, and recovery.

use tauri::State;

use crate::app::AppState;
use crate::dto::CommandResult;
use crate::storage::{maintenance, RecoveryAction, RecoveryOutcome, StorageUsage};

/// What the database costs on disk.
#[tauri::command]
pub async fn get_storage_usage(state: State<'_, AppState>) -> CommandResult<StorageUsage> {
    let paths = state.db.paths().clone();
    Ok(state
        .db
        .read(|conn| maintenance::storage_usage(conn, &paths))?)
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
    Ok(state.db.recover_for_command(action)?)
}
