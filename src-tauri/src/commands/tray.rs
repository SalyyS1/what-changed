//! The one WebView-to-tray appearance signal Tauri does not expose natively.

use tauri::AppHandle;

/// Report Windows forced-colors state. Only a boolean crosses IPC: no theme
/// strings, image paths, arbitrary bytes or shell operations are accepted.
#[tauri::command]
pub fn set_tray_forced_colors(app: AppHandle, active: bool) {
    crate::app::tray::set_forced_colors(&app, active);
}
