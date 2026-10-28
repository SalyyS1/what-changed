//! Native file dialogs grant exact file selection; IPC never accepts a path.
use crate::app::AppState;
use crate::dto::{owner_portability_dto::*, CommandResult};
use crate::error::{AppError, AppResult};
use crate::owner_workspace::{
    archive::MAX_ARCHIVE_BYTES,
    portability::{cache::ReviewData, *},
};
use std::io::Read;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

pub(crate) mod save;

fn invalid(reason: &str) -> AppError {
    AppError::InvalidInput {
        what: "portable memory",
        reason: reason.into(),
    }
}

#[tauri::command]
pub async fn preview_owner_export(
    state: State<'_, AppState>,
    app_id: String,
    selection: OwnerExportSelectionInput,
) -> CommandResult<OwnerBundlePreviewDto> {
    let admission = state.data_lifecycle.admit()?;
    let snapshot = state.owner_store.portable_snapshot(&app_id)?;
    let (bytes, preview) = prepare_bundle(snapshot, &selection)?;
    admission.validate()?;
    state.owner_portability.put(
        state.data_lifecycle.epoch(),
        ReviewData::Export {
            bytes,
            preview: preview.clone(),
            selection,
        },
    );
    Ok(preview)
}

#[tauri::command]
pub async fn export_owner_bundle(
    app: AppHandle,
    state: State<'_, AppState>,
    app_id: String,
    digest: String,
) -> CommandResult<Option<String>> {
    // Recheck the exact reviewed digest after the dialog. A purge while the
    // dialog is open clears the preview and cannot export old data afterward.
    let Some(selected) = app
        .dialog()
        .file()
        .set_title("Save reviewed project memory")
        .set_file_name("project-memory.zip")
        .add_filter("Ownvero memory bundle", &["zip"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let destination = selected
        .into_path()
        .map_err(|_| invalid("choose a local file"))?;
    let replace = match std::fs::symlink_metadata(&destination) {
        Ok(_) => {
            if !app
                .dialog()
                .message("Replace the selected file with this reviewed project memory bundle?")
                .title("Replace project memory file")
                .buttons(MessageDialogButtons::OkCancel)
                .blocking_show()
            {
                return Ok(None);
            }
            true
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return Err(invalid("cannot inspect the selected file").into()),
    };
    let admission = state.data_lifecycle.admit()?;
    let current = state.owner_store.portable_snapshot(&app_id)?;
    let bytes =
        state
            .owner_portability
            .export(state.data_lifecycle.epoch(), &app_id, &digest, current)?;
    admission.validate()?;
    save::bundle(&destination, &bytes, replace)?;
    Ok(Some(destination.to_string_lossy().into_owned()))
}

fn read_selected(path: &std::path::Path) -> AppResult<Vec<u8>> {
    let input =
        std::fs::File::open(path).map_err(|_| invalid("cannot open the selected bundle"))?;
    let metadata = input
        .metadata()
        .map_err(|_| invalid("cannot inspect the selected bundle"))?;
    if !metadata.is_file() || metadata.len() > MAX_ARCHIVE_BYTES as u64 {
        return Err(invalid("select a regular bundle no larger than 16 MiB"));
    }
    let mut bytes = Vec::new();
    input
        .take(MAX_ARCHIVE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid("cannot read the selected bundle"))?;
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(invalid("bundle exceeds 16 MiB"));
    }
    Ok(bytes)
}

#[tauri::command]
pub async fn validate_owner_import(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Option<OwnerImportPreviewDto>> {
    let Some(selected) = app
        .dialog()
        .file()
        .set_title("Review a project memory bundle")
        .add_filter("Ownvero memory bundle", &["zip"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|_| invalid("choose a local bundle"))?;
    let admission = state.data_lifecycle.admit()?;
    let bytes = read_selected(&path)?;
    let source_digest = format!("{:x}", sha2::Sha256::digest(&bytes));
    let raw = validate_bundle(&bytes)?;
    let (sanitized, export_preview) = prepare_bundle(
        raw,
        &OwnerExportSelectionInput {
            include_target_details: false,
        },
    )?;
    let data = validate_bundle(&sanitized)?;
    let existing = state.owner_store.list_apps(true)?;
    let preview=OwnerImportPreviewDto{review_id:uuid::Uuid::new_v4().to_string(),digest:source_digest,app_name:data.records.app.name.clone(),proposed_app_id:uuid::Uuid::new_v4().to_string(),rules:export_preview.rules,markdown:export_preview.markdown,conflicts:existing.iter().filter(|a|a.name.eq_ignore_ascii_case(&data.records.app.name)).map(|_|"An app with this name already exists. Import creates a separate app; choose a distinct name.".into()).collect(),limitations:IMPORT_LIMITATIONS.iter().map(|s|s.to_string()).collect()};
    admission.validate()?;
    state.owner_portability.put(
        state.data_lifecycle.epoch(),
        ReviewData::Import {
            data: Box::new(data),
            preview: preview.clone(),
        },
    );
    Ok(Some(preview))
}

#[tauri::command]
pub async fn import_owner_bundle(
    state: State<'_, AppState>,
    input: ApplyOwnerImportInput,
) -> CommandResult<OwnerImportResultDto> {
    let admission = state.data_lifecycle.admit()?;
    let (data, new_app) = state.owner_portability.import(
        state.data_lifecycle.epoch(),
        &input.review_id,
        &input.digest,
    )?;
    admission.validate()?;
    Ok(state
        .owner_store
        .apply_portable_import(data, input, new_app)?)
}

#[tauri::command]
pub async fn get_imported_owner_history(
    state: State<'_, AppState>,
    app_id: String,
) -> CommandResult<serde_json::Value> {
    serde_json::to_value(state.owner_store.imported_owner_history(&app_id)?)
        .map_err(|_| invalid("cannot encode historical memory").into())
}
use sha2::Digest;
