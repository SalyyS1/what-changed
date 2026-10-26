//! Development-only commands.
//!
//! The simulation commands write synthetic events through the real write path
//! so a screen can be exercised before any collector exists. Their accepting
//! bodies compile only in debug builds or with the `devtools` feature, and a
//! release audit asserts that neither command is registered in the shipped
//! handler. A synthetic row is labelled `source_collector = "synthetic"`
//! wherever it appears, so it cannot be mistaken for a real machine event.

use tauri::State;

use crate::app::{validate, AppState};
use crate::dto::CommandResult;

#[cfg(any(test, debug_assertions, feature = "devtools"))]
fn load_developer_mode(state: &AppState) -> CommandResult<bool> {
    use crate::storage::settings_repository;

    Ok(state
        .db
        .read(|conn| settings_repository::load(conn).map(|settings| settings.developer_mode))?)
}

#[cfg(any(test, debug_assertions, feature = "devtools"))]
pub(crate) fn build_simulation(
    kind: &str,
    developer_mode: bool,
) -> CommandResult<crate::privacy::Sanitized<crate::domain::NormalizedEvent>> {
    if !developer_mode {
        return Err(crate::error::AppError::InvalidInput {
            what: "kind",
            reason: "developer mode is disabled".to_owned(),
        }
        .into());
    }
    Ok(crate::testing::synthetic::one(kind)?)
}

/// Write one development event through the same sanitized repository path used
/// by the IPC command and the headless soak harness.
#[cfg(any(test, debug_assertions, feature = "devtools"))]
pub(crate) fn simulate_event_for_state(state: &AppState, kind: &str) -> CommandResult<i64> {
    validate::text("kind", kind, 40)?;
    let developer_mode = load_developer_mode(state)?;
    let event = build_simulation(kind, developer_mode)?;
    let ids = state.db.insert_events(vec![event])?;
    ids.into_iter().next_back().ok_or_else(|| {
        crate::error::AppError::InvalidInput {
            what: "kind",
            reason: "the synthetic event was not written".to_owned(),
        }
        .into()
    })
}

/// Produce one synthetic event of the named kind.
#[cfg(any(test, debug_assertions, feature = "devtools"))]
#[tauri::command]
pub async fn simulate_event(state: State<'_, AppState>, kind: String) -> CommandResult<i64> {
    simulate_event_for_state(&state, &kind)
}

/// Seed a named deterministic corpus for the developer states panel.
#[cfg(any(test, debug_assertions, feature = "devtools"))]
#[tauri::command]
pub async fn seed_synthetic(
    state: State<'_, AppState>,
    profile: String,
    relations: bool,
) -> CommandResult<u64> {
    validate::text("profile", &profile, 16)?;
    if !load_developer_mode(&state)? {
        return Err(crate::error::AppError::InvalidInput {
            what: "profile",
            reason: "developer mode is disabled".to_owned(),
        }
        .into());
    }
    let ids = crate::testing::synthetic::seed(
        &state.db,
        &profile,
        0xD3E0_0001,
        crate::domain::now_ms(),
        relations,
    )?;
    u64::try_from(ids.len()).map_err(|_| {
        crate::error::AppError::InvalidInput {
            what: "profile",
            reason: "the synthetic corpus is too large".to_owned(),
        }
        .into()
    })
}

/// Defense-in-depth fallback for non-development builds. The release command
/// handler does not register this function, and this body still refuses if an
/// internal call is introduced accidentally.
#[cfg(not(any(test, debug_assertions, feature = "devtools")))]
#[tauri::command]
pub async fn simulate_event(_state: State<'_, AppState>, kind: String) -> CommandResult<i64> {
    validate::text("kind", &kind, 40)?;
    Err(crate::error::AppError::InvalidInput {
        what: "kind",
        reason: "synthetic events exist only in a development build".to_owned(),
    }
    .into())
}

#[cfg(not(any(test, debug_assertions, feature = "devtools")))]
#[tauri::command]
pub async fn seed_synthetic(
    _state: State<'_, AppState>,
    profile: String,
    _relations: bool,
) -> CommandResult<u64> {
    validate::text("profile", &profile, 16)?;
    Err(crate::error::AppError::InvalidInput {
        what: "profile",
        reason: "synthetic events exist only in a development build".to_owned(),
    }
    .into())
}
