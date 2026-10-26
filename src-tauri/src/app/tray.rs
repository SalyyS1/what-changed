//! The tray icon and its seven-entry menu.
//!
//! Pause and Resume are both always present and the inapplicable one is
//! disabled rather than removed (D-13). A menu that changes shape under the
//! cursor moves the item below it, so the click that was meant for "Settings"
//! lands on "Quit".
//!
//! Tray construction can fail — a machine with no notification area, a shell
//! that is restarting — and the response is never to hide anyway. The caller
//! learns the tray is absent, the window stays closable, and the shell says so.
//! An invisible process with no way back is the one failure this app cannot
//! have.

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::menu::{Menu, MenuBuilder, MenuEvent, MenuItem, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use tauri::image::Image;
use tauri::{AppHandle, Manager, Runtime, Theme};

use super::window;

/// Menu item ids. Compared against `MenuId`, which is `PartialEq<&str>`.
pub const OPEN: &str = "tray-open";
pub const TODAY: &str = "tray-today";
pub const CHECKPOINT: &str = "tray-checkpoint";
pub const PAUSE: &str = "tray-pause";
pub const RESUME: &str = "tray-resume";
pub const SETTINGS: &str = "tray-settings";
pub const QUIT: &str = "tray-quit";

/// The seven labels, in the product's order and wording.
pub const LABELS: [(&str, &str); 7] = [
    (OPEN, "Open What Changed"),
    (TODAY, "What changed today?"),
    (CHECKPOINT, "Create checkpoint"),
    (PAUSE, "Pause monitoring"),
    (RESUME, "Resume monitoring"),
    (SETTINGS, "Settings"),
    (QUIT, "Quit"),
];

/// The tray icon's own id.
const TRAY_ID: &str = "main";

/// The WebView reports Windows forced-colors changes; Tauri reports the
/// window's light/dark theme. Both feeds select one icon without polling.
#[derive(Default)]
pub struct TrayAppearance {
    forced_colors: AtomicBool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IconVariant {
    Light,
    Dark,
    HighContrast,
}

fn icon_variant(theme: Theme, forced_colors: bool) -> IconVariant {
    if forced_colors {
        IconVariant::HighContrast
    } else if theme == Theme::Light {
        IconVariant::Light
    } else {
        IconVariant::Dark
    }
}

fn themed_icon(variant: IconVariant) -> tauri::Result<Image<'static>> {
    let bytes: &'static [u8] = match variant {
        IconVariant::Light => include_bytes!("../../icons/tray-light.png"),
        IconVariant::Dark => include_bytes!("../../icons/tray-dark.png"),
        IconVariant::HighContrast => include_bytes!("../../icons/tray-high-contrast.png"),
    };
    Image::from_bytes(bytes)
}

/// Apply a theme change only when the tray exists. A decode or shell error
/// leaves the previous icon in place, including the bundled default icon.
fn apply_icon<R: Runtime>(app: &AppHandle<R>, icon: &TrayIcon<R>, theme: Theme) {
    let forced_colors = app
        .try_state::<TrayAppearance>()
        .is_some_and(|appearance| appearance.forced_colors.load(Ordering::Relaxed));
    let variant = icon_variant(theme, forced_colors);
    match themed_icon(variant).and_then(|image| icon.set_icon(Some(image))) {
        Ok(()) => {}
        Err(error) => tracing::debug!(%error, ?variant, "the tray icon could not follow the theme"),
    }
}

/// Called by Tauri for Windows light/dark transitions.
pub fn on_theme_changed<R: Runtime>(app: &AppHandle<R>, theme: Theme) {
    if let Some(icon) = app.tray_by_id(TRAY_ID) {
        apply_icon(app, &icon, theme);
    }
}

/// Called only at WebView mount and when `(forced-colors: active)` changes.
/// Missing tray or window remains a cosmetic degradation, never an IPC error.
pub fn set_forced_colors<R: Runtime>(app: &AppHandle<R>, active: bool) {
    let Some(appearance) = app.try_state::<TrayAppearance>() else {
        return;
    };
    if appearance.forced_colors.swap(active, Ordering::Relaxed) == active {
        return;
    }
    let Some(icon) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let theme = window::main_window(app)
        .and_then(|window| window.theme().ok())
        .unwrap_or(Theme::Dark);
    apply_icon(app, &icon, theme);
}

/// What the frontend is told to show when a menu item asks for a screen.
pub const EVENT_TRAY_NAVIGATE: &str = "tray-navigate";

/// The two items whose enabled state tracks monitoring.
pub struct PauseResume<R: Runtime> {
    app: AppHandle<R>,
    pause: MenuItem<R>,
    resume: MenuItem<R>,
}

impl<R: Runtime> PauseResume<R> {
    /// Enable exactly the one that applies.
    ///
    /// Posted to the main thread and *not* waited on. `MenuItem::set_enabled`
    /// is `run_item_main_thread!`, which posts a task and then blocks on a
    /// channel until the event loop drains it — so calling it directly costs
    /// a round-trip through the event loop when we are already on the main
    /// thread, and a cross-thread block when we are not. Neither is worth
    /// paying to grey out a menu item: the state the menu reflects is already
    /// authoritative in `Monitoring`, and the command the item stands for
    /// works whether or not the item is drawn correctly yet.
    pub fn reflect(&self, monitoring: crate::dto::MonitoringDto) {
        let pause = self.pause.clone();
        let resume = self.resume.clone();
        let resume_label = resume_label(monitoring);
        let posted = self.app.run_on_main_thread(move || {
            // A menu item that cannot be updated is a cosmetic failure, not a
            // reason to stop.
            for (item, enabled) in [(&pause, !monitoring.paused), (&resume, monitoring.paused)] {
                if let Err(error) = item.set_enabled(enabled) {
                    tracing::debug!(%error, "the tray menu could not be updated");
                }
            }
            if let Err(error) = resume.set_text(resume_label) {
                tracing::debug!(%error, "the timed resume label could not be updated");
            }
        });
        if let Err(error) = posted {
            tracing::debug!(%error, "the tray menu update could not be scheduled");
        }
    }
}

/// The built tray, kept alive for the life of the app.
pub struct Tray<R: Runtime> {
    /// Dropping the icon removes it from the notification area.
    _icon: TrayIcon<R>,
    /// Handles for the two items that toggle.
    pub pause_resume: PauseResume<R>,
}

/// Build the menu and the icon.
///
/// `paused` is the state at build time, so the menu is correct on its first
/// display rather than after the first toggle.
pub fn build<R: Runtime>(app: &AppHandle<R>, paused: bool) -> tauri::Result<Tray<R>> {
    let items = build_items(app)?;
    let menu = build_menu(app, &items)?;
    let pause_resume = PauseResume {
        app: app.clone(),
        pause: items[3].clone(),
        resume: items[4].clone(),
    };
    pause_resume.reflect(crate::dto::MonitoringDto {
        paused,
        resume_at: None,
    });

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("What Changed")
        .menu(&menu)
        // Left click opens the window; the menu belongs on right click, which
        // is where Windows users look for it.
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_icon_event);

    // Begin with the bundled icon as a known-good fallback. The small themed
    // mark is installed after build; if decoding or shell update fails, the
    // bundled icon still gives the user a way back to the window.
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    } else {
        tracing::warn!("no application icon was available for the tray");
    }

    let icon = builder.build(app)?;
    if let Some(theme) = window::main_window(app).and_then(|window| window.theme().ok()) {
        apply_icon(app, &icon, theme);
    }
    Ok(Tray {
        _icon: icon,
        pause_resume,
    })
}

/// The seven items, in order.
fn build_items<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Vec<MenuItem<R>>> {
    LABELS
        .iter()
        .map(|(id, label)| MenuItemBuilder::with_id(*id, *label).build(app))
        .collect()
}

/// Separators group the menu the way the product describes it: open and look,
/// then act, then leave.
fn build_menu<R: Runtime>(app: &AppHandle<R>, items: &[MenuItem<R>]) -> tauri::Result<Menu<R>> {
    let mut builder = MenuBuilder::new(app);
    for (index, item) in items.iter().enumerate() {
        if matches!(index, 3 | 5 | 6) {
            builder = builder.separator();
        }
        builder = builder.item(item);
    }
    builder.build()
}

/// Left click brings the window back. That is the only gesture that does, so a
/// user who lost the window has one obvious thing to try.
fn on_icon_event<R: Runtime>(tray: &TrayIcon<R>, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
    } = event
    {
        window::show_main(tray.app_handle());
    }
}

/// The screen a menu item opens, if it opens one.
///
/// The route ids are the frontend's, from `src/app/routes.ts`. A test asserts
/// each one appears there, because a renamed route would otherwise turn three
/// tray items into silent no-ops: the frontend's lookup returns `undefined`
/// and nothing complains.
pub fn route_for(id: &str) -> Option<&'static str> {
    match id {
        TODAY => Some("timeline"),
        SETTINGS => Some("settings"),
        CHECKPOINT => Some("checkpoints"),
        _ => None,
    }
}

/// Route a menu click. Every arm calls the same code the equivalent command
/// calls, so the tray and the UI can never drift apart.
fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let id = event.id();
    if id == QUIT {
        super::lifecycle::quit(app);
    } else if id == PAUSE || id == RESUME {
        super::lifecycle::set_monitoring_paused(app, id == PAUSE);
    } else if id == OPEN {
        window::show_main(app);
    } else if id == CHECKPOINT {
        window::show_main(app);
        use tauri::Emitter;
        let _ = app.emit(EVENT_TRAY_NAVIGATE, "checkpoints");
        if let Some(state) = app.try_state::<crate::app::AppState>() {
            let service = state.checkpoints.clone();
            let handle = app.clone();
            let total = state
                .collectors
                .list_collector_status()
                .into_iter()
                .filter(|collector| {
                    collector.enabled && collector.state != crate::dto::CollectorState::Unavailable
                })
                .count();
            tauri::async_runtime::spawn_blocking(move || {
                match crate::commands::checkpoints::create_manual_with_progress(
                    handle.clone(),
                    service,
                    None,
                    Some("Created from the tray".to_owned()),
                    total,
                ) {
                    Ok(_) => show_checkpoint_saved(&handle),
                    Err(error) => tracing::warn!(%error, "tray checkpoint could not be created"),
                }
            });
        }
    } else if let Some(route) = route_for(id.as_ref()) {
        // These three open a screen. The window comes forward first, then the
        // frontend is told where to go — one event with the destination rather
        // than three events the router would have to learn about.
        window::show_main(app);
        use tauri::Emitter;
        if let Err(error) = app.emit(EVENT_TRAY_NAVIGATE, route) {
            tracing::debug!(%error, "the window could not be told where to go");
        }
    }
}

fn show_checkpoint_saved<R: Runtime>(app: &AppHandle<R>) {
    use tauri_plugin_notification::NotificationExt;
    if let Err(error) = app
        .notification()
        .builder()
        .title("What Changed")
        .body("Checkpoint saved. You can compare against this later.")
        .show()
    {
        tracing::debug!(%error, "the checkpoint notification could not be shown");
    }
}

/// Whether a label is one of the seven, for the tests that pin the wording.
pub fn label_for(id: &str) -> Option<&'static str> {
    LABELS
        .iter()
        .find(|(item, _)| *item == id)
        .map(|(_, label)| *label)
}

fn resume_label(monitoring: crate::dto::MonitoringDto) -> String {
    let Some(resume_at) = monitoring.resume_at else {
        return "Resume monitoring".to_owned();
    };
    let Some(utc) = chrono::DateTime::from_timestamp_millis(resume_at) else {
        return "Resume monitoring".to_owned();
    };
    format!(
        "Resume monitoring (at {})",
        utc.with_timezone(&chrono::Local).format("%H:%M")
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn system_theme_selects_the_matching_tray_mark() {
        assert_eq!(icon_variant(Theme::Light, false), IconVariant::Light);
        assert_eq!(icon_variant(Theme::Dark, false), IconVariant::Dark);
    }

    #[test]
    fn forced_colors_overrides_either_system_theme() {
        assert_eq!(icon_variant(Theme::Light, true), IconVariant::HighContrast);
        assert_eq!(icon_variant(Theme::Dark, true), IconVariant::HighContrast);
    }

    #[test]
    fn every_embedded_tray_variant_decodes_as_a_small_png() {
        for variant in [
            IconVariant::Light,
            IconVariant::Dark,
            IconVariant::HighContrast,
        ] {
            let image = themed_icon(variant).expect("the exported tray PNG must decode");
            assert_eq!((image.width(), image.height()), (32, 32), "{variant:?}");
        }
    }

    #[test]
    fn the_menu_is_exactly_the_seven_entries_in_the_products_order() {
        let labels: Vec<&str> = LABELS.iter().map(|(_, label)| *label).collect();
        assert_eq!(
            labels,
            vec![
                "Open What Changed",
                "What changed today?",
                "Create checkpoint",
                "Pause monitoring",
                "Resume monitoring",
                "Settings",
                "Quit",
            ]
        );
    }

    #[test]
    fn pause_and_resume_are_both_present_so_the_menu_never_changes_shape() {
        assert!(label_for(PAUSE).is_some());
        assert!(label_for(RESUME).is_some());
        assert_eq!(label_for("tray-nothing"), None);
    }

    #[test]
    fn every_id_is_distinct() {
        let mut ids: Vec<&str> = LABELS.iter().map(|(id, _)| *id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count);
    }

    #[test]
    fn exactly_the_three_screen_items_carry_a_route() {
        assert_eq!(route_for(TODAY), Some("timeline"));
        assert_eq!(route_for(SETTINGS), Some("settings"));
        assert_eq!(route_for(CHECKPOINT), Some("checkpoints"));
        for id in [OPEN, PAUSE, RESUME, QUIT] {
            assert_eq!(route_for(id), None, "{id} does not open a screen");
        }
    }

    #[test]
    fn a_timed_pause_labels_the_resume_deadline() {
        let label = resume_label(crate::dto::MonitoringDto {
            paused: true,
            resume_at: Some(1_700_000_000_000),
        });
        assert!(label.starts_with("Resume monitoring (at "));
        assert!(label.ends_with(')'));
        assert_eq!(
            resume_label(crate::dto::MonitoringDto {
                paused: true,
                resume_at: None,
            }),
            "Resume monitoring"
        );
    }

    #[test]
    fn every_route_the_tray_emits_exists_in_the_frontends_route_table() {
        // A renamed route id would turn three tray items into no-ops: the
        // frontend's lookup returns `undefined` and says nothing. This is the
        // only place the two languages can be compared.
        let routes = include_str!("../../../src/app/routes.ts");
        for id in [TODAY, SETTINGS, CHECKPOINT] {
            let route = route_for(id).unwrap_or_default();
            assert!(
                routes.contains(&format!("id: '{route}'")),
                "the tray opens '{route}', which src/app/routes.ts does not define"
            );
        }
    }

    #[test]
    fn checkpoint_menu_uses_the_same_application_operation_as_the_command() {
        let tray = include_str!("tray.rs");
        let command = include_str!("../commands/checkpoints.rs");
        let shared = "create_manual_with_progress";
        assert!(
            tray.contains(&format!("checkpoints::{shared}")),
            "the tray must route through the shared checkpoint operation"
        );
        assert!(
            command.matches(shared).count() >= 2,
            "the command must define and call the shared checkpoint operation"
        );
    }
}
