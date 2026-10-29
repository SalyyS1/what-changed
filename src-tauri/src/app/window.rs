//! The main window's lifecycle: showing it, and hiding it instead of closing.
//!
//! Closing the window hides it and monitoring continues. That is a promise the
//! product makes, and it has one dangerous edge: if the tray icon does not
//! exist, hiding would leave a process the user cannot reach and cannot quit
//! without Task Manager. So the hide is conditional on the tray being there,
//! and when it is not, close means close.
//!
//! The first hide shows one balloon saying the app is still recording. Once,
//! ever — a notification that repeats teaches people to dismiss notifications.

use tauri::{AppHandle, Manager, Runtime, WebviewWindow, WindowEvent};

use super::state::AppState;
use super::tray;
use crate::storage::settings_repository;

/// The label in `tauri.conf.json`.
pub const MAIN: &str = "main";

/// What the balloon says on the first hide. Fixed wording.
pub const FIRST_HIDE_TITLE: &str = "What Changed";
pub const FIRST_HIDE_BODY: &str = "What Changed is still recording. Open it from the tray.";

/// The main window, if it exists.
pub fn main_window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window(MAIN)
}

/// Bring the window back: unminimize, show, focus, in that order.
///
/// All three are needed. A hidden window that is also minimized stays
/// minimized when shown, and a shown window that is not focused appears
/// behind whatever the user was doing — which reads as "the tray icon did
/// nothing".
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = main_window(app) else {
        tracing::warn!("the main window is gone; nothing to show");
        return;
    };
    for (what, result) in [
        ("unminimize", window.unminimize()),
        ("show", window.show()),
        ("focus", window.set_focus()),
    ] {
        if let Err(error) = result {
            tracing::debug!(what, %error, "the window could not be brought forward");
        }
    }
}

/// What closing the window should do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseAction {
    /// Keep the process alive with only a tray icon.
    HideToTray,
    /// Let the close through, which exits the application.
    Exit,
}

/// The decision, as a function of the one thing it depends on.
///
/// Extracted so the rule is testable without a window: hiding a window the
/// user cannot get back is the failure this guards against, and a rule that
/// can only be checked by running the app is a rule nobody checks.
pub const fn close_action(tray_available: bool) -> CloseAction {
    if tray_available {
        CloseAction::HideToTray
    } else {
        CloseAction::Exit
    }
}

/// Handle close and system-theme events for the main window.
pub fn on_window_event<R: Runtime>(window: &tauri::Window<R>, event: &WindowEvent) {
    if window.label() != MAIN {
        return;
    }
    match event {
        WindowEvent::ThemeChanged(theme) => tray::on_theme_changed(window.app_handle(), *theme),
        WindowEvent::CloseRequested { api, .. } => {
            let app = window.app_handle();
            if let Some(state) = app.try_state::<AppState>() {
                state.preview_runner.stop_all();
            }
            match close_action(tray_is_available(app)) {
                CloseAction::Exit => {
                    // Hiding would strand the user with a process they cannot
                    // reach. Let the close happen, and say why in the log.
                    tracing::warn!("closing the window exits because the tray is unavailable");
                }
                CloseAction::HideToTray => {
                    api.prevent_close();
                    hide_to_tray(app);
                }
            }
        }
        _ => {}
    }
}

/// Whether a tray icon exists to get the window back from.
fn tray_is_available<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.try_state::<AppState>()
        .is_some_and(|state| state.tray_available())
}

/// Hide the window and, the first time only, say that recording continues.
fn hide_to_tray<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = main_window(app) {
        if let Err(error) = window.hide() {
            tracing::warn!(%error, "the window could not be hidden");
            return;
        }
    }
    notify_first_hide(app);
}

/// Show the one-time balloon, and record that it has been shown.
///
/// The flag is written before the notification is sent. If the write fails the
/// balloon is skipped entirely: showing a "one-time" message that will be
/// shown again on every hide is worse than not showing it at all.
fn notify_first_hide<R: Runtime>(app: &AppHandle<R>) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    // An unreadable flag means "not shown yet", matching the repository's own
    // deliberate fallback. The window has already been hidden by the time this
    // runs, so treating a corrupt database as "already told them" produces the
    // exact failure this app cannot have: the window vanishes and nothing says
    // where it went. One extra balloon is the cheaper mistake.
    let already_shown = state
        .db
        .read(settings_repository::first_hide_notified)
        .unwrap_or(false);
    if already_shown {
        return;
    }
    if let Err(error) = state
        .db
        .write(settings_repository::mark_first_hide_notified)
    {
        tracing::debug!(%error, "the first-hide flag could not be recorded; no balloon shown");
        return;
    }
    show_balloon(app);
}

/// Send the balloon through the notification plugin.
fn show_balloon<R: Runtime>(app: &AppHandle<R>) {
    use tauri_plugin_notification::NotificationExt;
    let sent = app
        .notification()
        .builder()
        .title(FIRST_HIDE_TITLE)
        .body(FIRST_HIDE_BODY)
        .show();
    if let Err(error) = sent {
        tracing::debug!(%error, "the first-hide notification could not be shown");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_balloon_says_recording_continues_and_where_to_look() {
        // The wording is a product commitment, so it is pinned here rather
        // than only in the document that states it.
        assert_eq!(
            FIRST_HIDE_BODY,
            "What Changed is still recording. Open it from the tray."
        );
    }

    #[test]
    fn the_window_only_hides_when_there_is_a_tray_to_get_it_back_from() {
        assert_eq!(close_action(true), CloseAction::HideToTray);
        assert_eq!(
            close_action(false),
            CloseAction::Exit,
            "hiding with no tray would leave a process the user cannot reach"
        );
    }
}
