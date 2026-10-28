//! Local immutable project decisions and filtered history.
use crate::app::AppState;
use crate::dto::{owner_memory_dto::*, CommandResult};
use crate::owner_workspace::OwnerPageDto;
use tauri::State;

#[tauri::command]
pub async fn save_project_decision(
    state: State<'_, AppState>,
    input: SaveProjectDecisionInput,
) -> CommandResult<ProjectDecisionRevisionDto> {
    let _admission = state.data_lifecycle.admit()?;
    Ok(state.owner_store.save_project_decision(input)?)
}
#[tauri::command]
pub async fn list_project_decisions(
    state: State<'_, AppState>,
    app_id: String,
    filter: ProjectMemoryFilterInput,
) -> CommandResult<OwnerPageDto<ProjectDecisionRevisionDto>> {
    Ok(state.owner_store.list_project_decisions(&app_id, filter)?)
}
