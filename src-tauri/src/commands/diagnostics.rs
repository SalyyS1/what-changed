//! The Diagnostics screen. Its full payload lands in phase 19; the parts that
//! already exist — build version, database state and size, collector list —
//! are answered now, because a diagnostics screen that says nothing while the
//! database is corrupt is exactly the screen a user needs at that moment.

use std::fs;
use std::path::Path;

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use super::collectors;
use crate::app::AppState;
use crate::dto::{CommandResult, DiagnosticsDto};
use crate::error::AppError;
use crate::privacy::scrub_line;
use crate::storage::{maintenance, DbState};

/// What the local service is doing right now.
#[tauri::command]
pub async fn get_diagnostics(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<DiagnosticsDto> {
    let db = state.db.state();
    let paths = state.db.paths().clone();
    // A corrupt database has no readable size; that is a fact to report, not a
    // reason for the screen to fail.
    let database_bytes = state
        .db
        .read(|conn| maintenance::storage_usage(conn, &paths))
        .map(|usage| usage.database_bytes)
        .unwrap_or(0);
    Ok(DiagnosticsDto {
        app_version: app.package_info().version.to_string(),
        os_version: tauri::webview_version().unwrap_or_else(|_| "unknown".to_owned()),
        schema_version: schema_version(&db),
        db,
        database_bytes,
        dropped_writes: state.db.dropped_writes(),
        failed_commit_rows: state.db.failed_commit_rows().values().copied().sum(),
        cycle_admission_drops: state
            .db
            .collector_cycle_admission_drops()
            .values()
            .copied()
            .sum(),
        failed_cycles: state.db.failed_collector_cycles().values().copied().sum(),
        write_queue_depth: state.db.write_queue_depth(),
        current_working_set_bytes: crate::platform::memory::current_working_set_bytes(),
        peak_working_set_bytes: crate::platform::memory::peak_working_set_bytes(),
        log_bytes: directory_bytes(&state.paths.logs),
        collectors: collectors::collector_rows(&state)?,
        owner_store: state.owner_store.state(),
        owner_workspace_bytes: state.owner_store.owned_file_bytes(),
    })
}

/// The schema version, or zero when the file is not in a state that has one.
fn schema_version(state: &DbState) -> u32 {
    match state {
        DbState::Ready { schema_version } => u32::try_from(*schema_version).unwrap_or(0),
        _ => 0,
    }
}

fn directory_bytes(path: &Path) -> u64 {
    fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

/// Write a diagnostics report the user can paste into a bug report.
///
/// The native save dialog is opened by the backend so an arbitrary IPC caller
/// cannot supply a path that overwrites another user file. The report is
/// assembled from an explicit allow-list and intentionally contains no event
/// bodies, entity names or paths.
#[tauri::command]
pub async fn export_diagnostics(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<String> {
    let _admission = state.data_lifecycle.admit()?;
    let destination = app
        .dialog()
        .file()
        .set_title("Save What Changed diagnostics")
        .set_file_name("what-changed-diagnostics.txt")
        .add_filter("Text report", &["txt"])
        .blocking_save_file()
        .ok_or_else(|| AppError::InvalidInput {
            what: "diagnostics export",
            reason: "choose a file path".to_owned(),
        })?
        .into_path()
        .map_err(|_| AppError::InvalidInput {
            what: "diagnostics export",
            reason: "the selected path is not a local file".to_owned(),
        })?;
    if destination.as_os_str().is_empty() || destination.is_dir() {
        return Err(AppError::InvalidInput {
            what: "diagnostics export",
            reason: "choose a file path".to_owned(),
        }
        .into());
    }
    let db = state.db.state();
    let paths = state.db.paths().clone();
    let usage = state
        .db
        .read(|conn| maintenance::storage_usage(conn, &paths))
        .unwrap_or_default();
    let settings = state
        .db
        .read(crate::storage::settings_repository::all)
        .unwrap_or_default();
    let collectors = collectors::collector_rows(&state).unwrap_or_default();
    let event_type_counts = state
        .db
        .read(maintenance::event_type_counts)
        .unwrap_or_default();
    let recent_logs = recent_log_lines(&state.paths.logs);
    let app_version = app.package_info().version.to_string();
    let webview_version = tauri::webview_version().unwrap_or_else(|_| "unknown".to_owned());
    let report = build_diagnostics_report(DiagnosticsReportInput {
        app_version: &app_version,
        webview_version: &webview_version,
        database_state: safe_db_state(&db),
        database_bytes: usage.database_bytes,
        wal_bytes: usage.wal_bytes,
        settings: &settings,
        collectors: &collectors,
        table_rows: &usage.table_rows,
        event_type_counts: &event_type_counts,
        recent_logs: &recent_logs,
    });
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|source| AppError::Directory {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(&destination, report).map_err(|source| AppError::File {
        operation: "write diagnostics report",
        path: destination.clone(),
        source,
    })?;
    Ok(destination.to_string_lossy().into_owned())
}

struct DiagnosticsReportInput<'a> {
    app_version: &'a str,
    webview_version: &'a str,
    database_state: &'a str,
    database_bytes: u64,
    wal_bytes: u64,
    settings: &'a [(String, serde_json::Value)],
    collectors: &'a [crate::dto::CollectorStatusDto],
    table_rows: &'a [(String, u64)],
    event_type_counts: &'a [(String, u64)],
    recent_logs: &'a [String],
}

fn build_diagnostics_report(input: DiagnosticsReportInput<'_>) -> String {
    let DiagnosticsReportInput {
        app_version,
        webview_version,
        database_state,
        database_bytes,
        wal_bytes,
        settings,
        collectors,
        table_rows,
        event_type_counts,
        recent_logs,
    } = input;
    let mut report = String::new();
    report.push_str("What Changed diagnostics\n");
    report.push_str("This report contains versions, health and aggregate counts only; it contains no event content, paths or entity names.\n\n");
    report.push_str(&format!("app_version={app_version}\n"));
    report.push_str(&format!("webview_version={webview_version}\n"));
    report.push_str(&format!("database_state={database_state}\n"));
    report.push_str(&format!(
        "database_bytes={}\nwal_bytes={}\n",
        database_bytes, wal_bytes
    ));
    report.push_str("\nsettings_keys:\n");
    for (key, value) in settings {
        // Values are represented only by their JSON type. This preserves
        // enough shape for support without exporting collector paths or user
        // preferences verbatim.
        report.push_str(&format!("- {} ({})\n", safe_word(key), json_kind(value)));
    }
    report.push_str("\ncollectors:\n");
    for collector in collectors {
        report.push_str(&format!(
            "- id={} state={} enabled={} failures={} queue_drops={} failed_cycles={}\n",
            safe_word(&collector.id),
            collector.state,
            collector.enabled,
            collector.consecutive_failures,
            collector.queue_drops,
            collector.failed_cycles
        ));
    }
    report.push_str("\ntable_counts:\n");
    for (table, count) in table_rows {
        report.push_str(&format!("- {}={}\n", safe_word(table), count));
    }
    report.push_str("\nevent_type_counts:\n");
    for (kind, count) in event_type_counts {
        report.push_str(&format!("- {}={}\n", safe_word(kind), count));
    }
    append_sanitized_log_lines_from_lines(&mut report, recent_logs);
    if report.len() > 512 * 1024 {
        report.truncate(512 * 1024);
    }
    report
}

fn safe_db_state(state: &DbState) -> &'static str {
    match state {
        DbState::Ready { .. } => "ready",
        DbState::Corrupt { .. } => "corrupt",
        DbState::SchemaMismatch { .. } => "schema_mismatch",
        DbState::Recovering => "recovering",
    }
}

fn safe_word(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        .take(80)
        .collect()
}

fn json_kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn recent_log_lines(logs: &Path) -> Vec<String> {
    let mut files = fs::read_dir(logs)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    let mut lines = Vec::new();
    for file in files.into_iter().rev().take(7) {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        lines.extend(text.lines().map(str::to_owned));
        if lines.len() >= 500 {
            break;
        }
    }
    let start = lines.len().saturating_sub(500);
    lines.into_iter().skip(start).collect()
}

fn append_sanitized_log_lines_from_lines(report: &mut String, lines: &[String]) {
    report.push_str("\nrecent_logs_redacted:\n");
    for (index, line) in lines.iter().enumerate() {
        // The writer has already applied the secret scrubber. We additionally
        // retain only a bounded, generic marker for each line so diagnostics
        // cannot become an entity/path export channel.
        let scrubbed = scrub_line(line);
        let level = if scrubbed.contains(" ERROR ") || scrubbed.contains("error") {
            "error"
        } else if scrubbed.contains(" WARN ") || scrubbed.contains("warn") {
            "warn"
        } else {
            "info"
        };
        report.push_str(&format!(
            "- line={} level={} message=[REDACTED]\n",
            index + 1,
            level
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{build_diagnostics_report, DiagnosticsReportInput};

    #[test]
    fn report_rendering_excludes_fixture_content_paths_and_secrets() {
        let report = build_diagnostics_report(DiagnosticsReportInput {
            app_version: "0.1.0",
            webview_version: "webview-test",
            database_state: "ready",
            database_bytes: 123,
            wal_bytes: 0,
            settings: &[("theme".to_owned(), serde_json::json!("dark"))],
            collectors: &[],
            table_rows: &[("events".to_owned(), 4)],
            event_type_counts: &[("updated".to_owned(), 4)],
            recent_logs: &[
                "ERROR fixture-entity C:\\fixture\\secret.env sk-test-secret".to_owned(),
            ],
        });
        assert!(!report.contains("fixture-entity"));
        assert!(!report.contains("C:\\fixture\\secret.env"));
        assert!(!report.contains("sk-test-secret"));
        assert!(report.len() <= 512 * 1024);
    }
}
