//! Privacy summary and destructive history controls.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tauri::State;

use crate::app::{validate, AppState};
use crate::domain::{Category, RetentionPolicy};
use crate::dto::{
    CommandResult, PrivacyCategoryCountDto, PrivacySummaryDto, PurgeReportDto, SettingsDto,
    SettingsUpdateDto,
};
use crate::error::AppError;
use crate::notify::QuietHours;
use crate::storage::{self, settings_repository};

const CHALLENGE_TTL: Duration = Duration::from_secs(120);

#[derive(Debug)]
struct PurgeChallenge {
    token: String,
    issued: Instant,
}

fn challenge_slot() -> &'static Mutex<Option<PurgeChallenge>> {
    static SLOT: OnceLock<Mutex<Option<PurgeChallenge>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Issue a short-lived, one-use, unguessable confirmation challenge. The UI
/// obtains this value immediately before showing the destructive confirmation.
#[tauri::command]
pub fn request_purge_challenge() -> CommandResult<String> {
    // The token must be unguessable because the command is reachable through
    // IPC; a timestamp and process-local counter would let another renderer
    // predict and replay the destructive call.
    let token = format!("wc-{}", uuid::Uuid::new_v4().simple());
    let mut slot = challenge_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *slot = Some(PurgeChallenge {
        token: token.clone(),
        issued: Instant::now(),
    });
    Ok(token)
}

/// Counts and paths shown by the privacy screen. No event content is returned.
#[tauri::command]
pub async fn get_privacy_summary(state: State<'_, AppState>) -> CommandResult<PrivacySummaryDto> {
    privacy_summary(&state)
}

/// Build the privacy payload from application state. Keeping the body in a
/// plain function lets the performance contract exercise the same work as the
/// IPC command without requiring a live Tauri window in the test harness.
pub fn privacy_summary(state: &AppState) -> CommandResult<PrivacySummaryDto> {
    let paths = state.paths.clone();
    // Keep the three read-only pieces under one DB-reader lease. Besides
    // avoiding repeated lifecycle/reader-pool locking, this makes the
    // privacy screen's timing contract measure the same short, consistent
    // snapshot instead of three independently scheduled reads.
    let (retention, aggregates, usage) = state.db.read(|conn| {
        Ok((
            settings_repository::retention(conn)?,
            storage::maintenance::privacy_counts(conn)?,
            storage::maintenance::storage_bytes(conn, &paths.db)?,
        ))
    })?;
    Ok(PrivacySummaryDto {
        categories: aggregates
            .categories
            .into_iter()
            .map(|(category, count)| PrivacyCategoryCountDto { category, count })
            .collect(),
        checkpoints: aggregates.checkpoints,
        checkpoint_items: aggregates.checkpoint_items,
        investigations: aggregates.investigations,
        watched_folders: aggregates.watched_folders,
        config_baselines: aggregates.config_baselines,
        database_bytes: usage.0,
        wal_bytes: usage.1,
        oldest_record_at: aggregates.oldest_record_at,
        log_bytes: directory_bytes(&paths.logs),
        data_path: paths.data.to_string_lossy().into_owned(),
        database_path: paths.db.database.to_string_lossy().into_owned(),
        retention,
    })
}

/// Delete one closed-vocabulary category of event history.
#[tauri::command]
pub async fn purge_category(
    state: State<'_, AppState>,
    category: String,
) -> CommandResult<PurgeReportDto> {
    let category = category
        .parse::<Category>()
        .map_err(|_| AppError::InvalidInput {
            what: "category",
            reason: "the category is not supported".to_owned(),
        })?;
    let counts = state
        .db
        .write(move |conn| storage::purge::category(conn, category.as_str()))?;
    Ok(PurgeReportDto {
        events_deleted: counts.events_deleted,
        ..PurgeReportDto::default()
    })
}

/// Stop watching and delete the history associated with one approved folder.
#[tauri::command]
pub async fn purge_folder(
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<PurgeReportDto> {
    let canonical = validate::watched_folder(&path, &state.paths.data)?;
    let shown = validate::display_path(&canonical);
    let counts = state
        .collectors
        .apply_sources_change(&["files", "dev_env"], || {
            state.db.write(move |conn| {
                if storage::watched_location_repository::by_path(conn, &shown)?.is_none() {
                    return Err(AppError::InvalidInput {
                        what: "path",
                        reason: "the folder is not currently watched".to_owned(),
                    });
                }
                storage::purge::folder(conn, &shown)
            })
        })?;
    Ok(PurgeReportDto {
        events_deleted: counts.events_deleted,
        baselines_deleted: counts.baselines_deleted,
        watched_folders_deleted: counts.watched_folders_deleted,
        ..PurgeReportDto::default()
    })
}

/// Delete every history table after a visible phrase and one-use challenge.
#[tauri::command]
pub async fn purge_all(
    state: State<'_, AppState>,
    confirmation: String,
    challenge: String,
) -> CommandResult<PurgeReportDto> {
    if !confirmation.trim().eq_ignore_ascii_case("delete") {
        return Err(AppError::InvalidInput {
            what: "confirmation",
            reason: "type delete to confirm".to_owned(),
        }
        .into());
    }
    {
        let mut slot = challenge_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let valid = slot.as_ref().is_some_and(|candidate| {
            candidate.token == challenge && candidate.issued.elapsed() <= CHALLENGE_TTL
        });
        if !valid {
            return Err(AppError::InvalidInput {
                what: "challenge",
                reason: "the confirmation has expired; open the dialog again".to_owned(),
            }
            .into());
        }
        // Consume before the write. A retry after a crashed/failed operation
        // must obtain a fresh visible confirmation instead of replaying one.
        *slot = None;
    }
    // Full deletion is a lifecycle operation. Read the report and the two
    // explicitly reviewed appearance preferences before closing every SQLite
    // handle, then atomically replace the history database with a fresh
    // migrated copy.
    let settings = state.db.read(settings_repository::load)?;
    let counts = state.db.read(storage::purge::counts)?;
    let theme = serde_json::to_string(&settings.theme).map_err(|error| AppError::Platform {
        operation: "encode preserved theme preference",
        reason: error.to_string(),
    })?;
    let visualization =
        serde_json::to_string(&settings.visualization).map_err(|error| AppError::Platform {
            operation: "encode preserved visualization preference",
            reason: error.to_string(),
        })?;
    // Cancel and discard every active collector instance before replacing the
    // database. The fresh onboarding state then prevents collection until the
    // user explicitly starts watching again.
    state.reset_collectors_after_history_purge()?;
    if let Err(error) = state.db.replace_history(&theme, &visualization) {
        // The lifecycle helper restores the old database when staging or
        // verification fails. Restore the runtime to the same persisted
        // collector choices so a failed destructive action is not a silent
        // monitoring outage.
        for (id, enabled) in settings.collectors {
            if enabled {
                let _ = state.collectors.set_enabled(&id, true);
            }
        }
        return Err(error.into());
    }
    clear_log_files(&state.paths.logs);
    Ok(PurgeReportDto {
        events_deleted: counts.events_deleted,
        checkpoints_deleted: counts.checkpoints_deleted,
        investigations_deleted: counts.investigations_deleted,
        baselines_deleted: counts.baselines_deleted,
        watched_folders_deleted: counts.watched_folders_deleted,
    })
}

/// Update retention using the same validation as the Settings command.
#[tauri::command]
pub async fn set_retention(
    state: State<'_, AppState>,
    retention: RetentionPolicy,
) -> CommandResult<SettingsDto> {
    Ok(state.update_collector_settings(SettingsUpdateDto {
        retention: Some(retention),
        ..Default::default()
    })?)
}

/// Persist the optional quiet-hours window used by notification policy.
#[tauri::command]
pub async fn set_quiet_hours(
    state: State<'_, AppState>,
    quiet_hours: QuietHours,
) -> CommandResult<QuietHours> {
    if quiet_hours.start_hour > 23 || quiet_hours.end_hour > 23 {
        return Err(AppError::InvalidInput {
            what: "quiet_hours",
            reason: "hours must be between 0 and 23".to_owned(),
        }
        .into());
    }
    Ok(state.db.write(move |conn| {
        crate::storage::settings_repository::put(conn, "quiet_hours", &quiet_hours)
            .map(|_| quiet_hours)
    })?)
}

/// Read the persisted quiet-hours policy used by notification delivery.
///
/// The setting is optional in older databases, so a missing row resolves to
/// the documented disabled default rather than making Settings render an
/// incorrect local value or fail to load.
#[tauri::command]
pub async fn get_quiet_hours(state: State<'_, AppState>) -> CommandResult<QuietHours> {
    Ok(state
        .db
        .read(|conn| settings_repository::get::<QuietHours>(conn, "quiet_hours"))?
        .unwrap_or_else(QuietHours::default_off))
}

fn directory_bytes(path: &std::path::Path) -> u64 {
    std::fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
        .sum()
}

/// Truncate app-owned logs after a confirmed full history purge. The active
/// tracing file may be held open by the logger on Windows, so a failure to
/// truncate one file is deliberately best-effort and never turns a successful
/// database purge into a misleading retry prompt.
fn clear_log_files(path: &std::path::Path) {
    let entries = std::fs::read_dir(path)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok);
    for entry in entries {
        let file = entry.path();
        if !file.is_file() {
            continue;
        }
        if let Err(error) = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&file)
        {
            tracing::debug!(%error, "a log file could not be truncated after privacy purge");
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::storage::Db;
    use crate::testing::temp_dir;

    #[test]
    fn quiet_hours_read_defaults_when_older_database_has_no_row_and_round_trips() {
        let db = Db::open_in(&temp_dir("quiet-hours-command")).unwrap();
        assert_eq!(
            db.read(|conn| settings_repository::get::<QuietHours>(conn, "quiet_hours"))
                .unwrap()
                .unwrap_or_else(QuietHours::default_off),
            QuietHours::default_off()
        );

        let saved = QuietHours {
            enabled: true,
            start_hour: 21,
            end_hour: 7,
        };
        let saved_for_db = saved;
        db.write(move |conn| settings_repository::put(conn, "quiet_hours", &saved_for_db))
            .unwrap();
        assert_eq!(
            db.read(|conn| settings_repository::get::<QuietHours>(conn, "quiet_hours"))
                .unwrap()
                .unwrap_or_else(QuietHours::default_off),
            saved
        );
    }
}
