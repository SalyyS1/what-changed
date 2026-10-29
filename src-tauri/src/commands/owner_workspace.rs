//! Offline app ownership and immutable product-memory commands.

use crate::app::AppState;
use crate::dto::CommandResult;
use crate::owner_workspace::*;
use tauri::State;

#[tauri::command]
pub async fn get_owner_store_status(state: State<'_, AppState>) -> CommandResult<OwnerStoreState> {
    Ok(state.owner_store.state())
}

#[tauri::command]
pub async fn save_owner_app(
    state: State<'_, AppState>,
    input: SaveOwnerAppInput,
) -> CommandResult<OwnerAppDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.save_app(input)?)
}

#[tauri::command]
pub async fn get_owner_app(
    state: State<'_, AppState>,
    app_id: String,
) -> CommandResult<OwnerAppDto> {
    Ok(state
        .owner_store
        .get_app(&app_id)?
        .ok_or_else(|| crate::error::AppError::InvalidInput {
            what: "app",
            reason: "does not exist".into(),
        })?)
}

#[tauri::command]
pub async fn list_owner_apps(
    state: State<'_, AppState>,
    include_archived: bool,
) -> CommandResult<Vec<OwnerAppDto>> {
    Ok(state.owner_store.list_apps(include_archived)?)
}

#[tauri::command]
pub async fn save_owner_capability(
    state: State<'_, AppState>,
    input: SaveOwnerCapabilityInput,
) -> CommandResult<OwnerCapabilityDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.save_capability(input)?)
}

#[tauri::command]
pub async fn list_owner_capabilities(
    state: State<'_, AppState>,
    app_id: String,
    include_archived: bool,
) -> CommandResult<Vec<OwnerCapabilityDto>> {
    Ok(state
        .owner_store
        .list_capabilities(&app_id, include_archived)?)
}

#[tauri::command]
pub async fn confirm_owner_rule(
    state: State<'_, AppState>,
    input: ConfirmOwnerRuleInput,
) -> CommandResult<OwnerRuleRevisionDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.confirm_rule(input)?)
}

#[tauri::command]
pub async fn list_owner_rule_revisions(
    state: State<'_, AppState>,
    app_id: String,
    page: OwnerPageInput,
) -> CommandResult<OwnerPageDto<OwnerRuleRevisionDto>> {
    Ok(state.owner_store.list_rule_revisions(&app_id, page)?)
}

#[tauri::command]
pub async fn save_owner_change_draft(
    state: State<'_, AppState>,
    input: SaveOwnerChangeInput,
) -> CommandResult<OwnerChangeDraftDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.save_change_draft(input)?)
}

#[tauri::command]
pub async fn get_owner_change_draft(
    state: State<'_, AppState>,
    app_id: String,
    change_id: String,
) -> CommandResult<OwnerChangeDraftDto> {
    Ok(state
        .owner_store
        .get_change_draft(&app_id, &change_id)?
        .ok_or_else(|| crate::error::AppError::InvalidInput {
            what: "change",
            reason: "does not exist".into(),
        })?)
}

#[tauri::command]
pub async fn list_owner_change_drafts(
    state: State<'_, AppState>,
    app_id: String,
    page: OwnerPageInput,
) -> CommandResult<OwnerPageDto<OwnerChangeDraftDto>> {
    Ok(state.owner_store.list_change_drafts(&app_id, page)?)
}

#[tauri::command]
pub async fn get_owner_change_review(
    state: State<'_, AppState>,
    app_id: String,
    change_id: String,
) -> CommandResult<OwnerChangeReviewDto> {
    Ok(state.owner_store.get_change_review(&app_id, &change_id)?)
}

#[tauri::command]
pub async fn save_owner_environment(
    state: State<'_, AppState>,
    input: SaveOwnerEnvironmentInput,
) -> CommandResult<OwnerEnvironmentRevisionDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.save_environment(input)?)
}

#[tauri::command]
pub async fn list_owner_environments(
    state: State<'_, AppState>,
    app_id: String,
    page: OwnerPageInput,
) -> CommandResult<OwnerPageDto<OwnerEnvironmentRevisionDto>> {
    Ok(state
        .owner_store
        .list_environment_revisions(&app_id, page)?)
}

#[tauri::command]
pub async fn save_owner_candidate(
    state: State<'_, AppState>,
    input: SaveOwnerCandidateInput,
) -> CommandResult<OwnerCandidateRevisionDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.save_candidate(input)?)
}

#[tauri::command]
pub async fn list_owner_candidates(
    state: State<'_, AppState>,
    app_id: String,
    page: OwnerPageInput,
) -> CommandResult<OwnerPageDto<OwnerCandidateRevisionDto>> {
    Ok(state.owner_store.list_candidates(&app_id, page)?)
}

#[tauri::command]
pub async fn select_owner_candidate(
    state: State<'_, AppState>,
    input: SelectOwnerCandidateInput,
) -> CommandResult<OwnerAppDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.select_candidate(input)?)
}

#[tauri::command]
pub async fn confirm_owner_scenario(
    state: State<'_, AppState>,
    input: ConfirmOwnerScenarioInput,
) -> CommandResult<ScenarioRevisionDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.confirm_scenario(input)?)
}

#[tauri::command]
pub async fn list_owner_scenarios(
    state: State<'_, AppState>,
    app_id: String,
    page: OwnerPageInput,
) -> CommandResult<OwnerPageDto<ScenarioRevisionDto>> {
    Ok(state.owner_store.list_scenarios(&app_id, page)?)
}

/// Build a deterministic, redacted Markdown preview. Copying is a frontend
/// decision after the owner reviews this value; the backend never reads or
/// writes the system clipboard.
#[tauri::command]
pub async fn preview_owner_brief(
    state: State<'_, AppState>,
    app_id: String,
    change_id: String,
    selection: OwnerBriefSelectionInput,
) -> CommandResult<OwnerBriefPreviewDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state
        .owner_store
        .render_brief(&app_id, &change_id, selection)?)
}
