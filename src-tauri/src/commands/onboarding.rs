//! First-run tour and startup preference commands.

use tauri::{AppHandle, State};

use crate::app::AppState;
use crate::domain::CheckpointKind;
use crate::dto::{CommandResult, OnboardingStateDto, SettingsUpdateDto};
use crate::error::AppError;
use crate::storage::{checkpoint_repository, settings_repository};

/// Read the one setting that gates the tour.
#[tauri::command]
pub fn get_onboarding_state(state: State<'_, AppState>) -> CommandResult<OnboardingStateDto> {
    let completed = state
        .db
        .read(settings_repository::load)?
        .onboarding_completed;
    Ok(OnboardingStateDto {
        completed,
        can_skip: true,
    })
}

/// Mark onboarding complete and capture the first known-good checkpoint.
#[tauri::command]
pub async fn complete_onboarding(state: State<'_, AppState>) -> CommandResult<()> {
    let _admission = state.data_lifecycle.admit()?;
    complete_onboarding_inner(&state)
}

fn complete_onboarding_inner(state: &AppState) -> CommandResult<()> {
    let _config = state.lock_collector_config();
    // Apply the persisted choices while the onboarding fence is still closed.
    // This makes the first checkpoint capture the user's actual day-one
    // sources instead of an empty boundary from the pre-consent runtime.
    let settings = state.db.read(settings_repository::load)?;
    let mut enabled: Vec<String> = Vec::new();
    if !state.startup.safe_mode {
        for (id, should_enable) in &settings.collectors {
            if *should_enable {
                if let Err(error) = state.collectors.set_enabled(id, true) {
                    for rollback in &enabled {
                        let _ = state.collectors.set_enabled(rollback, false);
                    }
                    return Err(error.into());
                }
                enabled.push(id.clone());
            }
        }
    }

    // Establish the first known-good boundary before committing the completion
    // flag. Re-running the tour must be idempotent: history is retained when
    // the tour is reopened, so do not add duplicate "First day" checkpoints.
    let has_first_checkpoint = match state.db.read(|conn| {
        checkpoint_repository::list(conn).map(|rows| rows.iter().any(|row| row.name == "First day"))
    }) {
        Ok(value) => value,
        Err(error) => {
            for rollback in &enabled {
                let _ = state.collectors.set_enabled(rollback, false);
            }
            return Err(error.into());
        }
    };
    let checkpoint_id = if has_first_checkpoint {
        None
    } else {
        match state.checkpoints.create(
            Some("First day".to_owned()),
            CheckpointKind::Automatic,
            Some("Created when onboarding finished".to_owned()),
            |_| {},
        ) {
            Ok(checkpoint) => Some(checkpoint.id),
            Err(error) => {
                for rollback in &enabled {
                    let _ = state.collectors.set_enabled(rollback, false);
                }
                return Err(error.into());
            }
        }
    };
    if let Err(error) = state.db.write(|conn| {
        settings_repository::update(
            conn,
            &SettingsUpdateDto {
                onboarding_completed: Some(true),
                ..Default::default()
            },
        )
        .map(|_| ())
    }) {
        if let Some(id) = checkpoint_id {
            let _ = state
                .db
                .write(move |conn| checkpoint_repository::delete(conn, id));
        }
        for rollback in &enabled {
            let _ = state.collectors.set_enabled(rollback, false);
        }
        return Err(error.into());
    }
    Ok(())
}

/// Re-open the tour from Settings. Existing history remains intact.
#[tauri::command]
pub async fn reopen_onboarding(state: State<'_, AppState>) -> CommandResult<()> {
    let _admission = state.data_lifecycle.admit()?;
    let _config = state.lock_collector_config();
    let settings = state.db.read(settings_repository::load)?;
    let mut disabled: Vec<String> = Vec::new();
    for id in settings.collectors.keys() {
        if !settings.collectors.get(id).copied().unwrap_or(false) {
            continue;
        }
        if let Err(error) = state.collectors.set_enabled(id, false) {
            for rollback in &disabled {
                let _ = state.collectors.set_enabled(rollback, true);
            }
            return Err(error.into());
        }
        disabled.push(id.clone());
    }
    if let Err(error) = state.db.write(|conn| {
        settings_repository::update(
            conn,
            &SettingsUpdateDto {
                onboarding_completed: Some(false),
                ..Default::default()
            },
        )
        .map(|_| ())
    }) {
        for rollback in &disabled {
            let _ = state.collectors.set_enabled(rollback, true);
        }
        return Err(error.into());
    }
    Ok(())
}

/// Explicit skip uses the same completion side effects as Finish.
#[tauri::command]
pub async fn skip_onboarding(state: State<'_, AppState>) -> CommandResult<()> {
    complete_onboarding(state).await
}

/// Toggle the per-user autostart entry supplied by the Tauri plugin.
#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> CommandResult<bool> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|_| AppError::Platform {
        operation: "change autostart preference",
        reason: "the per-user startup entry could not be changed".to_owned(),
    })?;
    Ok(manager.is_enabled().map_err(|_| AppError::Platform {
        operation: "read autostart preference",
        reason: "the per-user startup entry could not be read".to_owned(),
    })?)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::complete_onboarding_inner;
    use crate::app::{AppPaths, AppState};
    use crate::domain::CheckpointKind;
    use crate::storage::{checkpoint_repository, settings_repository};
    use crate::testing::temp_dir;

    #[test]
    fn fresh_db_completion_persists_consent_and_one_automatic_checkpoint() {
        let dir = temp_dir("onboarding-fresh-db");
        let state = AppState::open(AppPaths::under(&dir)).unwrap();
        assert!(
            !state
                .db
                .read(settings_repository::load)
                .unwrap()
                .onboarding_completed
        );
        assert!(state
            .db
            .read(checkpoint_repository::list)
            .unwrap()
            .is_empty());

        complete_onboarding_inner(&state).unwrap();

        let settings = state.db.read(settings_repository::load).unwrap();
        assert!(settings.onboarding_completed);
        let checkpoints = state.db.read(checkpoint_repository::list).unwrap();
        assert_eq!(checkpoints.len(), 1);
        assert_eq!(checkpoints[0].name, "First day");
        assert_eq!(checkpoints[0].kind, CheckpointKind::Automatic);

        // Reopening and completing the tour again must not create a second
        // day-one boundary or duplicate the persisted history.
        complete_onboarding_inner(&state).unwrap();
        assert_eq!(state.db.read(checkpoint_repository::list).unwrap().len(), 1);
        state.shutdown_cleanly();
    }
}
