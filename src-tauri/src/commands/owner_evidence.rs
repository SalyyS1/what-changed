//! Offline manual observations. No command grants browser or network authority.
use crate::app::AppState;
use crate::dto::{owner_evidence_dto::*, CommandResult};
use crate::owner_workspace::{OwnerPageDto, OwnerPageInput};
use tauri::State;

#[tauri::command]
pub async fn start_owner_manual_run(
    state: State<'_, AppState>,
    input: StartOwnerManualRunInput,
) -> CommandResult<OwnerManualRunDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.start_manual_run(input)?)
}
#[tauri::command]
pub async fn record_owner_manual_result(
    state: State<'_, AppState>,
    input: RecordOwnerManualResultInput,
) -> CommandResult<OwnerManualResultDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.record_manual_result(input)?)
}
#[tauri::command]
pub async fn finish_owner_manual_run(
    state: State<'_, AppState>,
    input: FinishOwnerManualRunInput,
) -> CommandResult<OwnerManualRunDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.finish_manual_run(input)?)
}
#[tauri::command]
pub async fn get_owner_manual_run(
    state: State<'_, AppState>,
    app_id: String,
    run_id: String,
) -> CommandResult<OwnerEvidenceReviewDto> {
    Ok(state.owner_store.get_manual_run(&app_id, &run_id)?)
}
#[tauri::command]
pub async fn list_owner_manual_runs(
    state: State<'_, AppState>,
    app_id: String,
    change_id: String,
    page: OwnerPageInput,
) -> CommandResult<OwnerPageDto<OwnerEvidenceReviewDto>> {
    Ok(state
        .owner_store
        .list_manual_runs(&app_id, &change_id, page)?)
}
#[tauri::command]
pub async fn record_owner_evidence_decision(
    state: State<'_, AppState>,
    input: RecordOwnerEvidenceDecisionInput,
) -> CommandResult<OwnerEvidenceDecisionDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.record_owner_decision(input)?)
}
#[tauri::command]
pub async fn list_owner_evidence_decisions(
    state: State<'_, AppState>,
    app_id: String,
    change_id: String,
    page: OwnerPageInput,
) -> CommandResult<OwnerPageDto<OwnerEvidenceDecisionDto>> {
    Ok(state
        .owner_store
        .list_owner_decisions(&app_id, &change_id, page)?)
}
