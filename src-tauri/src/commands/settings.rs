//! Settings, watched folders and projects.
//!
//! Settings are backed now. Watched folders and projects validate their paths
//! now — the rule that a junction into `C:\Windows` is refused belongs with
//! the command that accepts the path, not with the collector that would later
//! read it — and persist in the phases that own their tables.

use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::app::{validate, AppState};
use crate::commands::checkpoints::create_manual_with_progress;
use crate::domain::{ids, EntityType};
use crate::dto::{CommandResult, ProjectDto, SettingsDto, SettingsUpdateDto, WatchedLocationDto};
use crate::error::AppError;
use crate::storage::{project_repository, watched_location_repository};

fn watched_dto(
    row: watched_location_repository::WatchedLocation,
    app_data: &std::path::Path,
) -> WatchedLocationDto {
    let available = validate::watched_folder(&row.path, app_data).is_ok();
    WatchedLocationDto {
        id: row.id,
        path: row.path,
        enabled: row.enabled,
        available,
        max_depth: row.max_depth,
        added_at: row.added_at,
    }
}

fn project_dto(row: project_repository::Project) -> ProjectDto {
    let branch = read_branch(std::path::Path::new(&row.path));
    ProjectDto {
        id: row.id,
        path: row.path,
        name: row.name,
        branch,
        baseline_checkpoint_id: row.baseline_checkpoint_id,
        baseline_checkpoint_name: row.baseline_checkpoint_name,
        baseline_checkpoint_created_at: row.baseline_checkpoint_created_at,
    }
}

fn read_branch(root: &std::path::Path) -> Option<String> {
    crate::collectors::dev_env::branch_for_root(root)
}

/// Everything the Settings screen renders.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> CommandResult<SettingsDto> {
    Ok(state.settings()?)
}

/// Apply a partial update and return the settings as they now stand.
#[tauri::command]
pub async fn update_settings(
    state: State<'_, AppState>,
    update: SettingsUpdateDto,
) -> CommandResult<SettingsDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.update_collector_settings(update)?)
}

/// Folders the user chose to watch.
#[tauri::command]
pub async fn list_watched_locations(
    state: State<'_, AppState>,
) -> CommandResult<Vec<WatchedLocationDto>> {
    let rows = state.db.read(watched_location_repository::list)?;
    Ok(rows
        .into_iter()
        .map(|row| watched_dto(row, &state.paths.data))
        .collect())
}

/// Add a folder, after proving it is one this app may watch.
#[tauri::command]
pub async fn add_watched_location(
    state: State<'_, AppState>,
    path: String,
    max_depth: Option<u8>,
) -> CommandResult<WatchedLocationDto> {
    let _admission = state.data_lifecycle.admit()?;
    let canonical = validate::watched_folder(&path, &state.paths.data)?;
    let max_depth = validate::watch_depth(max_depth.unwrap_or(validate::MAX_WATCH_DEPTH))?;
    let shown = validate::display_path(&canonical);
    let row = state
        .collectors
        .apply_sources_change(&["files", "dev_env"], || {
            state.db.write(move |conn| {
                if let Some(existing) = watched_location_repository::by_path(conn, &shown)? {
                    return Ok(existing);
                }
                validate::watch_capacity(watched_location_repository::count(conn)?)?;
                watched_location_repository::insert(
                    conn,
                    &shown,
                    max_depth,
                    crate::domain::now_ms(),
                )
            })
        })?;
    Ok(watched_dto(row, &state.paths.data))
}

/// Stop watching a folder.
#[tauri::command]
pub async fn remove_watched_location(
    state: State<'_, AppState>,
    location_id: i64,
) -> CommandResult<()> {
    let _admission = state.data_lifecycle.admit()?;
    validate::in_range("folder id", location_id, 1, i64::MAX)?;
    state
        .collectors
        .apply_sources_change(&["files", "dev_env"], || {
            state.db.write(move |conn| {
                watched_location_repository::remove_with_snapshots(conn, location_id)
            })
        })?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::read_branch;

    #[test]
    fn branch_reader_is_bounded_and_does_not_label_a_detached_head_as_a_branch() {
        let root = crate::testing::temp_dir("project-branch-reader");
        std::fs::create_dir(root.join(".git")).unwrap();
        std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/feature/auth\n").unwrap();
        assert_eq!(read_branch(&root).as_deref(), Some("feature/auth"));

        std::fs::write(root.join(".git/HEAD"), "0123456789abcdef\n").unwrap();
        assert_eq!(read_branch(&root), None);

        std::fs::write(root.join(".git/HEAD"), "x".repeat(4 * 1024 + 1)).unwrap();
        assert_eq!(read_branch(&root), None);
    }

    #[test]
    fn branch_reader_supports_git_worktree_indirection() {
        let root = crate::testing::temp_dir("project-worktree-branch-reader");
        let worktree = root.join("worktree");
        let metadata = root.join("metadata");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::create_dir_all(&metadata).unwrap();
        std::fs::write(worktree.join(".git"), "gitdir: ../metadata\n").unwrap();
        std::fs::write(metadata.join("HEAD"), "ref: refs/heads/feature/worktree\n").unwrap();

        assert_eq!(read_branch(&worktree).as_deref(), Some("feature/worktree"));
    }
}

/// Registered project roots.
#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> CommandResult<Vec<ProjectDto>> {
    Ok(state
        .db
        .read(project_repository::list)?
        .into_iter()
        .map(project_dto)
        .collect())
}

/// Register a project root. Same path rules as a watched folder.
#[tauri::command]
pub fn add_project(state: State<'_, AppState>, path: String) -> CommandResult<ProjectDto> {
    let _admission = state.data_lifecycle.admit()?;
    let canonical = validate::watched_folder(&path, &state.paths.data)?;
    let shown = validate::display_path(&canonical);
    let name = canonical
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| shown.clone());
    let row = state.collectors.apply_source_change("files", || {
        state.db.write(move |conn| {
            project_repository::insert(conn, &shown, &name, crate::domain::now_ms())
        })
    })?;
    Ok(project_dto(row))
}

/// Unregister a project root.
#[tauri::command]
pub fn remove_project(state: State<'_, AppState>, project_id: String) -> CommandResult<()> {
    let _admission = state.data_lifecycle.admit()?;
    let project_id = validate::text("project", &project_id, validate::MAX_TEXT)?;
    if !ids::is_valid_entity_id_for(&project_id, EntityType::Project) {
        return Err(AppError::InvalidInput {
            what: "project",
            reason: "the id is not a canonical project id".to_owned(),
        }
        .into());
    }
    state.collectors.apply_source_change("files", || {
        state
            .db
            .write(move |conn| project_repository::remove(conn, &project_id))
    })?;
    Ok(())
}

/// Capture the current machine state and attach the committed checkpoint to a
/// registered project. The checkpoint remains global if the project is later
/// removed; only the pointer is project-owned.
#[tauri::command]
pub async fn mark_project_working(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
) -> CommandResult<ProjectDto> {
    let admission = state.data_lifecycle.admit()?;
    let project_id = validate::text("project", &project_id, validate::MAX_TEXT)?;
    if !ids::is_valid_entity_id_for(&project_id, EntityType::Project) {
        return Err(AppError::InvalidInput {
            what: "project",
            reason: "the id is not a canonical project id".to_owned(),
        }
        .into());
    }
    let project = state
        .db
        .read(|conn| project_repository::by_id(conn, &project_id))?
        .ok_or_else(|| AppError::InvalidInput {
            what: "project",
            reason: "project was not found".to_owned(),
        })?;
    let total = state
        .collectors
        .list_collector_status()
        .into_iter()
        .filter(|collector| {
            collector.enabled && collector.state != crate::dto::CollectorState::Unavailable
        })
        .count();
    let checkpoint_service = state.checkpoints.clone();
    let db = Arc::clone(&state.db);
    let reason = Some(format!("Workspace baseline: {}", project.name));
    let project_id_for_write = project_id.clone();
    let baseline = tauri::async_runtime::spawn_blocking(move || {
        let _admission = admission;
        let checkpoint = create_manual_with_progress(
            app,
            checkpoint_service,
            Some(format!("Working: {}", project.name)),
            reason,
            total,
        )?;
        if checkpoint.partial {
            return Err(AppError::InvalidInput {
                what: "workspace baseline",
                reason: format!(
                    "checkpoint {} was saved, but the baseline was not advanced because some collectors were unavailable",
                    checkpoint.id
                ),
            });
        }
        db.write(move |conn| {
            project_repository::set_baseline(conn, &project_id_for_write, checkpoint.id)
        })
    })
    .await
    .map_err(|_| AppError::InvalidInput {
        what: "workspace baseline",
        reason: "the background task stopped before returning a result".to_owned(),
    })??;
    Ok(project_dto(baseline))
}

#[tauri::command]
pub fn clear_project_baseline(
    state: State<'_, AppState>,
    project_id: String,
) -> CommandResult<ProjectDto> {
    let _admission = state.data_lifecycle.admit()?;
    let project_id = validate::text("project", &project_id, validate::MAX_TEXT)?;
    if !ids::is_valid_entity_id_for(&project_id, EntityType::Project) {
        return Err(AppError::InvalidInput {
            what: "project",
            reason: "the id is not a canonical project id".to_owned(),
        }
        .into());
    }
    Ok(project_dto(state.db.write(move |conn| {
        project_repository::clear_baseline(conn, &project_id)
    })?))
}
