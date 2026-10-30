//! Starting up, pausing, and quitting.
//!
//! Quit is the only exit, and it is a clean one: the crash-loop counter is
//! reset, the write queue is drained, and only then does the process end. A
//! recorder that loses its last batch on the way out is a recorder whose
//! history has a hole exactly where the interesting thing happened.

use tauri::{AppHandle, Manager, Runtime};

use super::state::AppState;

/// Pause or resume monitoring, updating the tray and telling the window.
///
/// The tray menu and the `set_monitoring_paused` command both land here, so
/// the two paths cannot diverge.
pub fn set_monitoring_paused<R: Runtime>(app: &AppHandle<R>, paused: bool) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Ok(_mutation) = state.data_lifecycle.admit() else {
        return;
    };
    state.collectors.set_monitoring_paused(paused, None);
}

/// Quit: flush, record a clean shutdown, then exit.
///
/// `AppHandle::exit` runs the exit path, which reaches the `RunEvent::Exit`
/// arm in `lib.rs`; the flush happens here as well so the data is safe even if
/// the event loop is torn down before that arm runs.
pub fn quit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<AppState>() {
        state.shutdown_cleanly();
    }
    app.exit(0);
}

/// Everything that must happen exactly once before the process ends.
///
/// Called from both the quit path and `RunEvent::Exit`, so it must be
/// idempotent — which it is: flushing an empty queue is a no-op, and writing
/// the marker twice writes the same bytes.
pub fn shutdown(state: &AppState) {
    let _operation = state.data_lifecycle.lock_operation();
    state.data_lifecycle.begin_shutdown();
    state.preview_runner.stop_all();
    state
        .preview_retention
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
    state.stop_resume_monitor();
    state.stop_maintenance();
    // Close admission first. Collector shutdown then wakes any checkpoint
    // capture already waiting on workers; the automation thread can therefore
    // join without holding shutdown hostage. Only flush after every detached
    // checkpoint caller has returned.
    state.checkpoints.begin_shutdown();
    state.collectors.stop_all();
    state.stop_checkpoint_automation();
    state.checkpoints.wait_for_idle();
    let mut clean = true;
    if state.collectors.shutdown_incomplete() {
        tracing::warn!("collector workers did not all join at shutdown");
        clean = false;
    }
    if let Err(error) = state.data_lifecycle.wait_for_idle() {
        tracing::warn!(%error, "mutation workers did not drain at shutdown");
        clean = false;
    }
    if let Err(error) = state.db.close_write_admission(state.data_lifecycle.epoch()) {
        tracing::warn!(%error, "the legacy mutation barrier could not drain at shutdown");
        clean = false;
    }
    state.owner_portability.clear();
    if let Err(error) = super::data_lifecycle::DataLifecycle::flush_owner(&state.owner_store) {
        tracing::debug!(%error, "the owner-workspace write queue could not be flushed at shutdown");
        clean = false;
    }
    if state.db.state().is_ready() {
        if let Err(error) = state.db.flush() {
            tracing::debug!(%error, "the write queue could not be flushed at shutdown");
            clean = false;
        }
    }
    if let Err(error) = super::data_lifecycle::DataLifecycle::shutdown_owner(&state.owner_store) {
        tracing::debug!(%error, "the owner-workspace writer could not stop at shutdown");
        clean = false;
    }
    super::data_lifecycle::DataLifecycle::shutdown_legacy(&state.db);
    if clean {
        state.mark_clean_shutdown();
    }
}
