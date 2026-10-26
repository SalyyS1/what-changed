//! Facts about the running build, the shell's boot call, and the two commands
//! the frontend uses to report about itself.

use tauri::{AppHandle, State};

use crate::app::AppState;
use crate::dto::{AppStatusDto, CommandResult, UpdateCheckDto};
use crate::error::AppError;
use crate::privacy::scrub_line;

/// Longest frontend error report accepted, in characters.
const MAX_REPORT: usize = 1_000;

/// Version of the running application.
///
/// Read from Tauri's own resolved package info — the same value the bundler and
/// updater use — so the UI can never report a version that disagrees with the
/// installed artifact.
#[tauri::command]
pub fn get_app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Everything the shell needs before it can draw its first frame.
#[tauri::command]
pub fn get_app_status(state: State<'_, AppState>) -> AppStatusDto {
    state.status()
}

/// Record an error the frontend hit.
///
/// Rate-limited and scrubbed: this is the one command whose purpose is to put
/// text chosen by the page into a log file, so a malicious script must not be
/// able to flood the log or smuggle a value into it.
#[tauri::command]
pub fn log_ui_error(
    state: State<'_, AppState>,
    message: String,
    context: Option<String>,
) -> CommandResult<()> {
    if !state.ui_errors.allow() {
        return Err(AppError::RateLimited {
            what: "log_ui_error",
        }
        .into());
    }
    let bounded: String = message.chars().take(MAX_REPORT).collect();
    let where_it_happened = context
        .map(|c| c.chars().take(120).collect::<String>())
        .unwrap_or_else(|| "unknown".to_owned());
    tracing::warn!(
        context = %scrub_line(&where_it_happened),
        "the interface reported a problem: {}",
        scrub_line(&bounded)
    );
    Ok(())
}

/// Answer "check for updates" without contacting anything.
///
/// The command exists because the Settings screen asks the question; the
/// answer is that this preview has no configured feed or signing key. The
/// explicit opt-in is accepted so a future signed updater can be added without
/// changing the Settings interaction; it does not enable network access here.
#[tauri::command]
pub fn check_for_updates(app: AppHandle, opt_in: bool) -> CommandResult<UpdateCheckDto> {
    Ok(update_check_response(
        app.package_info().version.to_string(),
        opt_in,
    ))
}

/// Build the updater response without any I/O. Keeping this separate makes
/// the no-network contract directly testable outside a Tauri runtime.
pub fn update_check_response(current_version: String, opted_in: bool) -> UpdateCheckDto {
    UpdateCheckDto {
        supported: false,
        opted_in,
        current_version,
        message: if opted_in {
            "Update checks are not configured in this preview. No network request was made."
        } else {
            "Updates are opt-in. Enable the check to ask, when a signed feed is configured."
        }
        .to_owned(),
    }
}
