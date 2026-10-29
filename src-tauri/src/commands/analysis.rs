//! Deterministic investigation commands.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::app::event_bus::EVENT_ANALYSIS_COMPLETED;
use crate::app::{validate, AppState};
use crate::domain::{ids, EntityType};
use crate::domain::{EventSummary, InvestigationWindow};
use crate::dto::{CommandResult, InvestigationDto, InvestigationRequestDto};
use crate::error::AppError;
use crate::services::context_pack;
use crate::services::investigation_service::InvestigationService;
use crate::storage::project_repository;

#[tauri::command]
pub async fn start_investigation(
    app: AppHandle,
    state: State<'_, AppState>,
    request: InvestigationRequestDto,
) -> CommandResult<InvestigationDto> {
    let admission = state.data_lifecycle.admit()?;
    let problem = validate::text("problem", &request.problem, validate::MAX_PROBLEM)?;
    if let Some(window) = &request.window {
        validate_window(window)?;
    }
    let project_id = request
        .project_id
        .map(|value| validate_project(&state, value))
        .transpose()?;
    let service = InvestigationService::new(Arc::clone(&state.db));
    let requested_window = request.window;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _admission = admission;
        service.start_scoped(problem, requested_window, project_id)
    })
    .await
    .map_err(|_| background_failed())??;
    let _ = app.emit(EVENT_ANALYSIS_COMPLETED, result.id);
    Ok(result.into())
}

#[tauri::command]
pub async fn list_investigations(
    state: State<'_, AppState>,
    limit: u16,
) -> CommandResult<Vec<InvestigationDto>> {
    validate::in_range("result count", limit, 1, 100)?;
    let service = InvestigationService::new(Arc::clone(&state.db));
    let rows = tauri::async_runtime::spawn_blocking(move || service.list(limit))
        .await
        .map_err(|_| background_failed())??;
    Ok(rows.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn get_investigation(
    state: State<'_, AppState>,
    investigation_id: i64,
) -> CommandResult<InvestigationDto> {
    validate::in_range("investigation id", investigation_id, 1, i64::MAX)?;
    let service = InvestigationService::new(Arc::clone(&state.db));
    let row = tauri::async_runtime::spawn_blocking(move || service.get(investigation_id))
        .await
        .map_err(|_| background_failed())??;
    Ok(row.into())
}

#[tauri::command]
pub async fn delete_investigation(
    state: State<'_, AppState>,
    investigation_id: i64,
) -> CommandResult<()> {
    let admission = state.data_lifecycle.admit()?;
    validate::in_range("investigation id", investigation_id, 1, i64::MAX)?;
    let service = InvestigationService::new(Arc::clone(&state.db));
    tauri::async_runtime::spawn_blocking(move || {
        let _admission = admission;
        service.delete(investigation_id)
    })
    .await
    .map_err(|_| background_failed())??;
    Ok(())
}

#[tauri::command]
pub async fn analyze_window(
    state: State<'_, AppState>,
    window: InvestigationWindow,
) -> CommandResult<InvestigationDto> {
    let admission = state.data_lifecycle.admit()?;
    validate_window(&window)?;
    let service = InvestigationService::new(Arc::clone(&state.db));
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _admission = admission;
        service.analyze_window(window)
    })
    .await
    .map_err(|_| background_failed())??;
    Ok(result.into())
}

#[tauri::command]
pub async fn get_related_events(
    state: State<'_, AppState>,
    event_id: i64,
) -> CommandResult<Vec<EventSummary>> {
    validate::in_range("event id", event_id, 1, i64::MAX)?;
    let service = InvestigationService::new(Arc::clone(&state.db));
    Ok(
        tauri::async_runtime::spawn_blocking(move || service.related_events(event_id))
            .await
            .map_err(|_| background_failed())??,
    )
}

/// Return a bounded, redacted Markdown handoff for any external AI assistant.
/// The backend reads only the persisted investigation snapshot and optional
/// registered-project metadata; the frontend owns clipboard delivery.
#[tauri::command]
pub fn export_investigation_context(
    state: State<'_, AppState>,
    investigation_id: i64,
) -> CommandResult<String> {
    validate::in_range("investigation id", investigation_id, 1, i64::MAX)?;
    let service = InvestigationService::new(Arc::clone(&state.db));
    let investigation = service.get(investigation_id)?;
    let project = investigation
        .project_id
        .as_deref()
        .map(|project_id| {
            state
                .db
                .read(|conn| project_repository::by_id(conn, project_id))
        })
        .transpose()?
        .flatten();
    Ok(context_pack::investigation_markdown(
        &investigation,
        project.as_ref(),
    ))
}

fn validate_project(state: &AppState, value: String) -> CommandResult<String> {
    let project_id = validate::text("project", &value, validate::MAX_TEXT)?;
    if !ids::is_valid_entity_id_for(&project_id, EntityType::Project) {
        return Err(AppError::InvalidInput {
            what: "project",
            reason: "the id is not a canonical project id".to_owned(),
        }
        .into());
    }
    state
        .db
        .read(|conn| project_repository::by_id(conn, &project_id))?
        .ok_or_else(|| AppError::InvalidInput {
            what: "project",
            reason: "project was not found".to_owned(),
        })?;
    Ok(project_id)
}

fn validate_window(window: &InvestigationWindow) -> crate::error::AppResult<()> {
    match window {
        InvestigationWindow::Custom { from_ms, to_ms } => {
            validate::time_range(*from_ms, *to_ms)?;
        }
        InvestigationWindow::SinceCheckpoint { checkpoint_id } => {
            validate::in_range("checkpoint id", *checkpoint_id, 1, i64::MAX)?;
        }
        _ => {}
    }
    Ok(())
}

fn background_failed() -> crate::dto::AppErrorDto {
    AppError::InvalidInput {
        what: "analysis",
        reason: "the background task stopped before returning a result".to_owned(),
    }
    .into()
}
