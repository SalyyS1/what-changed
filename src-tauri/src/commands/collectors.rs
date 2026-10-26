//! Collector health and the global pause.
//!
//! Settings and Diagnostics share live supervisor rows. A catalog entry with
//! no platform factory remains `Unavailable`, never falsely described as idle.

use std::collections::BTreeMap;

use tauri::{AppHandle, State};

use crate::app::AppState;
use crate::dto::{CollectorStatusDto, CommandResult, SettingsUpdateDto};
use crate::error::AppResult;

/// Health of every registered collector.
#[tauri::command]
pub async fn get_collector_status(
    state: State<'_, AppState>,
) -> CommandResult<Vec<CollectorStatusDto>> {
    Ok(collector_rows(&state)?)
}

/// Explicit runtime-status spelling used by the collector API.
#[tauri::command]
pub async fn list_collector_status(
    state: State<'_, AppState>,
) -> CommandResult<Vec<CollectorStatusDto>> {
    Ok(state.collectors.list_collector_status())
}

/// The collector table, as a plain function.
///
/// Separate from the command so `get_diagnostics` can build the same rows
/// without calling a command from a command — which stopped compiling the
/// moment either became `async`, and was coupling two command bodies before
/// that.
pub(crate) fn collector_rows(state: &AppState) -> AppResult<Vec<CollectorStatusDto>> {
    Ok(state.collectors.list_collector_status())
}

/// Switch one collector on or off.
#[tauri::command]
pub async fn set_collector_enabled(
    state: State<'_, AppState>,
    collector_id: String,
    enabled: bool,
) -> CommandResult<Vec<CollectorStatusDto>> {
    let mut change = BTreeMap::new();
    change.insert(collector_id, enabled);
    state.update_collector_settings(SettingsUpdateDto {
        collectors: Some(change),
        ..Default::default()
    })?;
    Ok(state.collectors.list_collector_status())
}

/// Pause monitoring for a bounded number of minutes.
#[tauri::command]
pub fn pause_monitoring(
    _app: AppHandle,
    state: State<'_, AppState>,
    minutes: Option<u32>,
) -> CommandResult<crate::dto::MonitoringDto> {
    state.collectors.set_monitoring_paused(true, minutes);
    Ok(state.monitoring.snapshot())
}

/// Resume monitoring immediately.
#[tauri::command]
pub fn resume_monitoring(
    _app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<crate::dto::MonitoringDto> {
    state.collectors.set_monitoring_paused(false, None);
    Ok(state.monitoring.snapshot())
}

/// Current global pause state.
#[tauri::command]
pub fn get_monitor_state(state: State<'_, AppState>) -> CommandResult<crate::dto::MonitoringDto> {
    Ok(state.monitoring.snapshot())
}

/// Pause or resume monitoring. The tray menu calls the same code.
///
/// Synchronous on purpose: it flips an atomic and posts a menu update without
/// waiting, so there is nothing here worth a trip through the async runtime.
#[tauri::command]
pub fn set_monitoring_paused(
    _app: AppHandle,
    state: State<'_, AppState>,
    paused: bool,
    minutes: Option<u32>,
) -> CommandResult<crate::dto::MonitoringDto> {
    state.collectors.set_monitoring_paused(paused, minutes);
    Ok(state.monitoring.snapshot())
}
