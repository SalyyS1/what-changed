//! What Changed — application library.
//!
//! `main.rs` is a shim; everything the app does is wired here so the same code
//! path is reachable from tests.
//!
//! Startup order is a privacy constraint, not a preference: logging is
//! installed first, and it is installed with a redacting writer, so no line
//! emitted by anything that runs afterwards can reach a file unscrubbed. The
//! database opens second, and a database that fails its integrity check is a
//! state the app boots into rather than a reason not to boot.

pub mod analysis;
pub mod app;
pub mod checkpoint;
pub mod collectors;
pub mod commands;
pub mod domain;
pub mod dto;
pub mod error;
pub mod notify;
pub mod observability;
pub mod owner_workspace;
pub mod platform;
pub mod preview_runner;
pub mod privacy;
pub mod services;
pub mod storage;

#[cfg(feature = "devtools")]
mod soak;

// Integration tests compile this library as a normal debug dependency, so
// `cfg(test)` is not set for the library itself. Keep the synthetic helpers in
// debug builds, and require the explicit feature for release builds.
#[cfg(any(test, debug_assertions, feature = "devtools"))]
pub mod testing;

use app::{AppPaths, AppState, EventBus, TrayReflector};
use tauri::{Manager, RunEvent};

struct RuntimeEvents<R: tauri::Runtime>(tauri::AppHandle<R>);

impl<R: tauri::Runtime> collectors::runtime::RuntimeStatusSink for RuntimeEvents<R> {
    fn collector_changed(&self, collector_id: &str) {
        use tauri::Emitter;
        if let Err(error) = self
            .0
            .emit(app::event_bus::EVENT_COLLECTOR_STATUS_CHANGED, collector_id)
        {
            tracing::debug!(%error, "collector status could not be delivered to the window");
        }
    }

    fn monitoring_changed(&self, monitoring: dto::MonitoringDto) {
        if let Some(state) = self.0.try_state::<AppState>() {
            state.reflect_monitoring(monitoring);
        }
        use tauri::Emitter;
        if let Err(error) = self
            .0
            .emit(app::event_bus::EVENT_MONITORING_STATE_CHANGED, monitoring)
        {
            tracing::debug!(%error, "monitoring state could not be delivered to the window");
        }
    }
}

/// Everything the webview may call. The order matches `commands::COMMANDS`,
/// and a test asserts the two agree — a command added to one and not the other
/// would be either unreachable or untyped.
#[cfg(any(debug_assertions, feature = "devtools"))]
macro_rules! command_handler {
    () => {
        tauri::generate_handler![
            commands::meta::get_app_version,
            commands::meta::get_app_status,
            commands::meta::log_ui_error,
            commands::onboarding::complete_onboarding,
            commands::meta::check_for_updates,
            commands::onboarding::get_onboarding_state,
            commands::onboarding::reopen_onboarding,
            commands::onboarding::skip_onboarding,
            commands::onboarding::set_autostart,
            commands::tray::set_tray_forced_colors,
            commands::owner_workspace::get_owner_store_status,
            commands::owner_workspace::save_owner_app,
            commands::owner_workspace::get_owner_app,
            commands::owner_workspace::list_owner_apps,
            commands::owner_workspace::save_owner_capability,
            commands::owner_workspace::list_owner_capabilities,
            commands::owner_workspace::confirm_owner_rule,
            commands::owner_workspace::list_owner_rule_revisions,
            commands::owner_workspace::save_owner_change_draft,
            commands::owner_workspace::get_owner_change_draft,
            commands::owner_workspace::list_owner_change_drafts,
            commands::owner_workspace::get_owner_change_review,
            commands::owner_workspace::save_owner_environment,
            commands::owner_workspace::list_owner_environments,
            commands::owner_workspace::save_owner_candidate,
            commands::owner_workspace::list_owner_candidates,
            commands::owner_workspace::select_owner_candidate,
            commands::owner_workspace::confirm_owner_scenario,
            commands::owner_workspace::list_owner_scenarios,
            commands::owner_workspace::preview_owner_brief,
            commands::preview_runner::prepare_preview_run,
            commands::preview_runner::export_preview_recipe,
            commands::preview_runner::save_preview_recipe,
            commands::preview_runner::capture_preview_surface,
            commands::preview_runner::review_preview_capture,
            commands::preview_runner::list_preview_captures,
            commands::preview_runner::grant_preview_run,
            commands::preview_runner::get_preview_runner_status,
            commands::preview_runner::continue_preview_login,
            commands::preview_runner::cancel_preview_run,
            commands::owner_evidence::start_owner_manual_run,
            commands::owner_evidence::record_owner_manual_result,
            commands::owner_evidence::finish_owner_manual_run,
            commands::owner_evidence::get_owner_manual_run,
            commands::owner_evidence::list_owner_manual_runs,
            commands::owner_evidence::record_owner_evidence_decision,
            commands::owner_evidence::list_owner_evidence_decisions,
            commands::owner_memory::save_project_decision,
            commands::owner_memory::list_project_decisions,
            commands::owner_portability::preview_owner_export,
            commands::owner_portability::export_owner_bundle,
            commands::owner_portability::validate_owner_import,
            commands::owner_portability::import_owner_bundle,
            commands::owner_portability::get_imported_owner_history,
            commands::events::get_dashboard_summary,
            commands::events::get_events,
            commands::events::expand_cluster,
            commands::events::get_event_details,
            commands::events::search_events,
            commands::system_map::get_system_map,
            commands::checkpoints::list_checkpoints,
            commands::checkpoints::create_checkpoint,
            commands::checkpoints::rename_checkpoint,
            commands::checkpoints::delete_checkpoint,
            commands::checkpoints::compare_checkpoint,
            commands::analysis::start_investigation,
            commands::analysis::list_investigations,
            commands::analysis::get_investigation,
            commands::analysis::delete_investigation,
            commands::analysis::analyze_window,
            commands::analysis::get_related_events,
            commands::analysis::export_investigation_context,
            commands::change_map::get_change_map,
            commands::collectors::get_collector_status,
            commands::collectors::list_collector_status,
            commands::collectors::set_collector_enabled,
            commands::collectors::set_monitoring_paused,
            commands::collectors::pause_monitoring,
            commands::collectors::resume_monitoring,
            commands::collectors::get_monitor_state,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::list_watched_locations,
            commands::settings::add_watched_location,
            commands::settings::remove_watched_location,
            commands::settings::list_projects,
            commands::settings::add_project,
            commands::settings::remove_project,
            commands::settings::mark_project_working,
            commands::settings::clear_project_baseline,
            commands::storage::get_storage_usage,
            commands::diagnostics::get_diagnostics,
            commands::diagnostics::export_diagnostics,
            commands::storage::db_recover,
            commands::privacy::request_purge_challenge,
            commands::privacy::get_privacy_summary,
            commands::privacy::purge_category,
            commands::privacy::purge_folder,
            commands::privacy::purge_all,
            commands::privacy::set_retention,
            commands::privacy::set_quiet_hours,
            commands::privacy::get_quiet_hours,
            commands::dev::simulate_event,
            commands::dev::seed_synthetic,
        ]
    };
}

/// The shipped handler omits the synthetic command entirely. Keeping a
/// separate list makes the cargo-feature half of the double gate structural:
/// there is no release IPC route that can reach the generator.
#[cfg(not(any(debug_assertions, feature = "devtools")))]
macro_rules! command_handler {
    () => {
        tauri::generate_handler![
            commands::meta::get_app_version,
            commands::meta::get_app_status,
            commands::meta::log_ui_error,
            commands::onboarding::complete_onboarding,
            commands::meta::check_for_updates,
            commands::onboarding::get_onboarding_state,
            commands::onboarding::reopen_onboarding,
            commands::onboarding::skip_onboarding,
            commands::onboarding::set_autostart,
            commands::tray::set_tray_forced_colors,
            commands::owner_workspace::get_owner_store_status,
            commands::owner_workspace::save_owner_app,
            commands::owner_workspace::get_owner_app,
            commands::owner_workspace::list_owner_apps,
            commands::owner_workspace::save_owner_capability,
            commands::owner_workspace::list_owner_capabilities,
            commands::owner_workspace::confirm_owner_rule,
            commands::owner_workspace::list_owner_rule_revisions,
            commands::owner_workspace::save_owner_change_draft,
            commands::owner_workspace::get_owner_change_draft,
            commands::owner_workspace::list_owner_change_drafts,
            commands::owner_workspace::get_owner_change_review,
            commands::owner_workspace::save_owner_environment,
            commands::owner_workspace::list_owner_environments,
            commands::owner_workspace::save_owner_candidate,
            commands::owner_workspace::list_owner_candidates,
            commands::owner_workspace::select_owner_candidate,
            commands::owner_workspace::confirm_owner_scenario,
            commands::owner_workspace::list_owner_scenarios,
            commands::owner_workspace::preview_owner_brief,
            commands::preview_runner::prepare_preview_run,
            commands::preview_runner::export_preview_recipe,
            commands::preview_runner::save_preview_recipe,
            commands::preview_runner::capture_preview_surface,
            commands::preview_runner::review_preview_capture,
            commands::preview_runner::list_preview_captures,
            commands::preview_runner::grant_preview_run,
            commands::preview_runner::get_preview_runner_status,
            commands::preview_runner::continue_preview_login,
            commands::preview_runner::cancel_preview_run,
            commands::owner_evidence::start_owner_manual_run,
            commands::owner_evidence::record_owner_manual_result,
            commands::owner_evidence::finish_owner_manual_run,
            commands::owner_evidence::get_owner_manual_run,
            commands::owner_evidence::list_owner_manual_runs,
            commands::owner_evidence::record_owner_evidence_decision,
            commands::owner_evidence::list_owner_evidence_decisions,
            commands::owner_memory::save_project_decision,
            commands::owner_memory::list_project_decisions,
            commands::owner_portability::preview_owner_export,
            commands::owner_portability::export_owner_bundle,
            commands::owner_portability::validate_owner_import,
            commands::owner_portability::import_owner_bundle,
            commands::owner_portability::get_imported_owner_history,
            commands::events::get_dashboard_summary,
            commands::events::get_events,
            commands::events::expand_cluster,
            commands::events::get_event_details,
            commands::events::search_events,
            commands::system_map::get_system_map,
            commands::checkpoints::list_checkpoints,
            commands::checkpoints::create_checkpoint,
            commands::checkpoints::rename_checkpoint,
            commands::checkpoints::delete_checkpoint,
            commands::checkpoints::compare_checkpoint,
            commands::analysis::start_investigation,
            commands::analysis::list_investigations,
            commands::analysis::get_investigation,
            commands::analysis::delete_investigation,
            commands::analysis::analyze_window,
            commands::analysis::get_related_events,
            commands::analysis::export_investigation_context,
            commands::change_map::get_change_map,
            commands::collectors::get_collector_status,
            commands::collectors::list_collector_status,
            commands::collectors::set_collector_enabled,
            commands::collectors::set_monitoring_paused,
            commands::collectors::pause_monitoring,
            commands::collectors::resume_monitoring,
            commands::collectors::get_monitor_state,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::list_watched_locations,
            commands::settings::add_watched_location,
            commands::settings::remove_watched_location,
            commands::settings::list_projects,
            commands::settings::add_project,
            commands::settings::remove_project,
            commands::settings::mark_project_working,
            commands::settings::clear_project_baseline,
            commands::storage::get_storage_usage,
            commands::diagnostics::get_diagnostics,
            commands::diagnostics::export_diagnostics,
            commands::storage::db_recover,
            commands::privacy::request_purge_challenge,
            commands::privacy::get_privacy_summary,
            commands::privacy::purge_category,
            commands::privacy::purge_folder,
            commands::privacy::purge_all,
            commands::privacy::set_retention,
            commands::privacy::set_quiet_hours,
            commands::privacy::get_quiet_hours,
        ]
    };
}

/// Adapts the built tray into the runtime-free handle `AppState` keeps.
struct TrayHandle<R: tauri::Runtime>(app::tray::Tray<R>);

impl<R: tauri::Runtime> TrayReflector for TrayHandle<R> {
    fn reflect(&self, monitoring: dto::MonitoringDto) {
        self.0.pause_resume.reflect(monitoring);
    }
}

/// Whether a data root is reserved for synthetic QA rather than the app.
fn reserved_qa_data_dir(path: &std::path::Path) -> bool {
    // The devtools seeder owns these directory names before it writes a
    // sentinel. Resolve junctions too, so an alias cannot make the shipped
    // executable open a QA store while seed/purge is running.
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    resolved
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.starts_with("what-changed-perf-") || name.starts_with("what-changed-seed-")
        })
}

/// Build and run the Tauri application.
///
/// Returns only when the event loop exits. A startup failure is reported on
/// stderr and exits non-zero: a builder that cannot start has nothing to
/// recover into.
pub fn run() {
    #[cfg(feature = "devtools")]
    if std::env::args().any(|argument| argument == "--soak") {
        std::process::exit(soak::run(std::env::args().skip(1)));
    }
    let paths = AppPaths::resolve();
    if reserved_qa_data_dir(&paths.db.root) {
        // This fixed string carries no data-directory path or user content.
        // Tests can open their isolated fixture through AppState directly;
        // the user-facing executable must never observe or modify it.
        eprintln!("fatal: What Changed cannot start from a QA-only data directory");
        std::process::exit(1);
    }

    // Held for the lifetime of the process: dropping it stops the background
    // log writer and loses whatever it had buffered.
    let logging = match observability::init(&paths.logs) {
        Ok(control) => Some(control),
        Err(error) => {
            // Logging is the thing that failed, so this one message goes to
            // stderr directly. It names the failure, never a value.
            eprintln!("warning: continuing without a log file: {error}");
            None
        }
    };

    tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting");

    // The shipped app composes its collector catalog through the platform
    // boundary; `AppState::open` keeps the all-unavailable canonical catalog so
    // tests stay deterministic on any host.
    let state = match AppState::open_with_registry(paths, platform::production_registry()) {
        Ok(state) => state,
        Err(error) => {
            tracing::error!(%error, "the data directory could not be prepared");
            eprintln!("fatal: What Changed could not start: {error}");
            drop(logging);
            std::process::exit(1);
        }
    };
    // Subscribed before the app is built, so a write that happens during setup
    // is not a notice nobody was listening for.
    let notices = state.db.subscribe();

    let app = tauri::Builder::default()
        // Registered first, by the plugin's own requirement: its callback has
        // to run before a second window could be created.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if !argv.iter().any(|argument| argument == "--minimized") {
                app::window::show_main(app);
            }
        }))
        .plugin(window_state_plugin())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg("--minimized")
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .manage(app::tray::TrayAppearance::default())
        .on_window_event(app::window::on_window_event)
        .invoke_handler(command_handler!())
        .setup(move |app| {
            setup(app, notices);
            Ok(())
        })
        .build(tauri::generate_context!());

    let app = match app {
        Ok(app) => app,
        Err(error) => {
            tracing::error!(%error, "the application could not start");
            eprintln!("fatal: What Changed could not start: {error}");
            drop(logging);
            std::process::exit(1);
        }
    };

    // Quitting through the tray is the only exit, and it is a clean one: the
    // write queue is drained and the crash-loop counter is reset here, so the
    // next start is an ordinary start.
    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<AppState>() {
                state.shutdown_cleanly();
            }
        }
    });

    drop(logging);
}

/// Restore size, position and maximized state; never visibility.
///
/// Restoring visibility would mean an app that was hidden to the tray when it
/// last quit comes back invisible, with the user having asked for a window.
fn window_state_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    use tauri_plugin_window_state::StateFlags;
    tauri_plugin_window_state::Builder::new()
        .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
        .build()
}

/// Everything that needs a live `App`: the tray, and the coalescing thread
/// that feeds the window.
///
/// Nothing here may fail the setup. A tray that will not build is a degraded
/// app that still records and still shows a window; refusing to start would
/// turn a cosmetic failure into a total one.
fn setup(app: &mut tauri::App, notices: std::sync::mpsc::Receiver<storage::BatchNotice>) {
    let handle = app.handle().clone();
    let Some(state) = handle.try_state::<AppState>() else {
        return;
    };

    let tray_available = match app::tray::build(&handle, state.monitoring.is_paused()) {
        Ok(tray) => {
            state.attach_tray(Box::new(TrayHandle(tray)));
            true
        }
        Err(error) => {
            // The window stays closable while this is true, which is what
            // stops the app from becoming an unreachable process.
            tracing::error!(%error, "the tray icon could not be created; closing the window will exit");
            false
        }
    };

    // Autostart passes --minimized, but hiding is safe only after the tray
    // actually exists. On a shell/tray failure the visible window is the only
    // way to recover or quit without Task Manager.
    if should_hide_at_autostart(
        std::env::args().any(|argument| argument == "--minimized"),
        tray_available,
    ) {
        if let Some(window) = handle.get_webview_window("main") {
            if let Err(error) = window.hide() {
                tracing::debug!(%error, "the minimized startup window could not be hidden");
            }
        }
    }

    state.attach_event_bus(EventBus::spawn(notices, handle.clone()));
    state
        .collectors
        .attach_status_sink(std::sync::Arc::new(RuntimeEvents(handle.clone())));
    if let Err(error) = state.collectors.spawn_all() {
        tracing::error!(%error, "one or more collector threads could not start");
    }
    if let Err(error) = state.start_checkpoint_automation(std::sync::Arc::new(handle.clone())) {
        tracing::error!(%error, "automatic checkpoint scheduling could not start");
    }
    match platform::start_resume_monitor(state.collectors.resume_wake_handle()) {
        Ok(monitor) => state.attach_resume_monitor(monitor),
        Err(error) => tracing::debug!(%error, "resume monitoring could not start"),
    }

    // The minimum size is set in the configuration; re-asserting it here means
    // a restored geometry from an older build cannot reopen the window smaller
    // than the layout supports.
    if let Some(window) = app::window::main_window(&handle) {
        let minimum = tauri::LogicalSize::new(900.0, 600.0);
        if let Err(error) = window.set_min_size(Some(minimum)) {
            tracing::debug!(%error, "the minimum window size could not be applied");
        }
    }

    let db_state = state.db.state();
    if !db_state.is_ready() {
        tracing::error!(?db_state, "starting with an unusable database");
    }
}

fn should_hide_at_autostart(minimized_requested: bool, tray_available: bool) -> bool {
    minimized_requested && tray_available
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    #[test]
    fn executable_rejects_reserved_qa_directory_names() {
        use std::path::Path;

        assert!(super::reserved_qa_data_dir(Path::new(
            "what-changed-perf-123"
        )));
        assert!(super::reserved_qa_data_dir(Path::new(
            "what-changed-seed-123"
        )));
        assert!(!super::reserved_qa_data_dir(Path::new("WhatChanged")));
        assert!(!super::reserved_qa_data_dir(Path::new("what-changed-perf")));
    }

    #[test]
    fn autostart_never_hides_the_only_window_when_the_tray_fails() {
        assert!(!super::should_hide_at_autostart(true, false));
        assert!(super::should_hide_at_autostart(true, true));
        assert!(!super::should_hide_at_autostart(false, true));
    }

    /// The handler macro and `commands::COMMANDS` are two lists of the same
    /// thing, and the second is what the binding generator and the contract
    /// test read. This asserts they agree by counting the paths in the macro's
    /// own source text — the file is the source of truth for both.
    #[test]
    fn the_registered_handler_matches_the_documented_command_list() {
        let source = include_str!("lib.rs");
        let start = source
            .find("tauri::generate_handler![")
            .expect("the handler macro is in this file");
        let end = source[start..]
            .find(']')
            .map(|offset| start + offset)
            .expect("the handler macro is closed");
        let registered: Vec<String> = source[start..end]
            .lines()
            .filter_map(|line| line.trim().strip_suffix(','))
            .filter_map(|path| path.rsplit("::").next())
            .map(str::to_owned)
            .collect();

        assert_eq!(
            registered.len(),
            crate::commands::COMMANDS.len(),
            "registered {registered:?}"
        );
        for (registered, documented) in registered.iter().zip(crate::commands::COMMANDS) {
            assert_eq!(
                registered, documented,
                "the two lists are in the same order"
            );
        }
    }
}
