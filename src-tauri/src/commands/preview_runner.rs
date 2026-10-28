//! Fixed preview-session capabilities; no executable, shell or profile argument.
use crate::app::AppState;
use crate::dto::CommandResult;
use crate::preview_runner::*;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn prepare_preview_run(
    state: State<'_, AppState>,
    input: PreparePreviewRunInput,
) -> CommandResult<PreviewRunPlanDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.preview_runner.prepare(&state.owner_store, input)?)
}
#[tauri::command]
pub async fn grant_preview_run(
    app: AppHandle,
    state: State<'_, AppState>,
    input: GrantPreviewRunInput,
) -> CommandResult<PreviewRunnerStatusDto> {
    Ok(state.preview_runner.start(
        Arc::clone(&state.owner_store),
        Arc::clone(&state.data_lifecycle),
        PreviewRunner::resource_root(&app)?,
        state.paths.data.clone(),
        input,
    )?)
}
#[tauri::command]
pub async fn get_preview_runner_status(
    state: State<'_, AppState>,
    app_id: String,
    run_id: String,
) -> CommandResult<PreviewRunnerStatusDto> {
    Ok(state.preview_runner.status(&app_id, &run_id)?)
}
#[tauri::command]
pub async fn continue_preview_login(
    state: State<'_, AppState>,
    app_id: String,
    run_id: String,
) -> CommandResult<()> {
    Ok(state.preview_runner.continue_login(&app_id, &run_id)?)
}
#[tauri::command]
pub async fn cancel_preview_run(
    state: State<'_, AppState>,
    app_id: String,
    run_id: String,
) -> CommandResult<()> {
    Ok(state.preview_runner.cancel(&app_id, &run_id)?)
}

#[tauri::command]
pub async fn capture_preview_surface(
    state: State<'_, AppState>,
    app_id: String,
    run_id: String,
    role: String,
) -> CommandResult<()> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.preview_runner.capture(&app_id, &run_id, &role)?)
}
#[tauri::command]
pub async fn review_preview_capture(
    state: State<'_, AppState>,
    app_id: String,
    run_id: String,
    capture_id: String,
    retain: bool,
) -> CommandResult<()> {
    let admission = state.data_lifecycle.admit()?;
    state.owner_store.get_manual_run(&app_id, &run_id)?;
    admission.validate()?;
    Ok(state.preview_runner.review_capture(
        &state.paths.data,
        &app_id,
        &run_id,
        &capture_id,
        retain,
    )?)
}
#[tauri::command]
pub async fn list_preview_captures(
    state: State<'_, AppState>,
    app_id: String,
    run_id: String,
) -> CommandResult<Vec<PreviewCaptureDto>> {
    let admission = state.data_lifecycle.admit()?;
    state.owner_store.get_manual_run(&app_id, &run_id)?;
    let captures = crate::preview_runner::captures::list(&state.paths.data, &app_id, &run_id)?;
    admission.validate()?;
    Ok(captures)
}

#[tauri::command]
pub async fn export_preview_recipe(
    state: State<'_, AppState>,
    input: PreparePreviewRunInput,
) -> CommandResult<PreviewRecipeExportDto> {
    let admission = state.data_lifecycle.admit()?;
    let preview = export_recipe(&state.owner_store, input)?;
    admission.validate()?;
    Ok(preview)
}

#[tauri::command]
pub async fn save_preview_recipe(
    app: AppHandle,
    state: State<'_, AppState>,
    input: PreparePreviewRunInput,
    digest: String,
) -> CommandResult<Option<String>> {
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
    let Some(selected) = app
        .dialog()
        .file()
        .set_title("Save reviewed preview test")
        .set_file_name("ownvero-preview-test.zip")
        .add_filter("Preview test ZIP", &["zip"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let destination = selected
        .into_path()
        .map_err(|_| crate::preview_runner::invalid("Choose a local test archive"))?;
    let replace = match std::fs::symlink_metadata(&destination) {
        Ok(_) => {
            if !app
                .dialog()
                .message("Replace the selected file with this reviewed test archive?")
                .title("Replace preview test")
                .buttons(MessageDialogButtons::OkCancel)
                .blocking_show()
            {
                return Ok(None);
            }
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => {
            return Err(crate::preview_runner::invalid(
                "The selected destination could not be inspected",
            )
            .into())
        }
    };
    let admission = state.data_lifecycle.admit()?;
    let preview = export_recipe(&state.owner_store, input)?;
    if preview.digest != digest {
        return Err(crate::preview_runner::invalid(
            "The test changed after review; prepare and review its export again",
        )
        .into());
    }
    let bytes = recipe_archive(&preview)?;
    admission.validate()?;
    super::owner_portability::save::bundle(&destination, &bytes, replace)?;
    Ok(Some(destination.to_string_lossy().into_owned()))
}
