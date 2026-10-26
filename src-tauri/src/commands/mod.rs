//! Tauri command surface.
//!
//! Commands are the *only* way the webview reaches the backend, and they carry
//! no logic: each one validates its input, delegates, and maps the result to a
//! serializable type. There is deliberately no generic escape hatch — no shell
//! execution, no arbitrary file read, no SQL from the frontend.
//!
//! Every command returns `CommandResult<T>`, so the frontend matches on a
//! typed code rather than parsing a sentence. Commands whose implementing
//! phase has not landed return `NotImplemented` naming that phase, and each is
//! listed in `ROADMAP.md` under "Tracked stubs" — a stub is visible, dated and
//! owned, never silently green.
//!
//! **A command that touches the database is `async`.** A synchronous
//! `#[tauri::command]` body runs on the thread that dispatches the IPC
//! message, and that thread is the main one: wry's IPC handler is called from
//! the event loop. So a blocking body freezes the window for as long as it
//! runs, and a database write is a round-trip to the writer thread. Marking
//! the command `async` routes it through `async_runtime::spawn` instead, and
//! the window keeps painting. Commands that only read a value already in
//! memory stay synchronous, because dispatching them would cost more than
//! they do.

pub mod analysis;
pub mod change_map;
pub mod checkpoints;
pub mod collectors;
pub mod dev;
pub mod diagnostics;
pub mod events;
pub mod meta;
pub mod onboarding;
pub mod privacy;
pub mod settings;
pub mod storage;
pub mod system_map;
pub mod tray;

use crate::dto::{AppErrorDto, CommandResult};
use crate::error::AppError;

/// The answer of a command whose phase has not landed.
pub fn not_implemented<T>(phase: &'static str) -> CommandResult<T> {
    Err(AppErrorDto::from(AppError::NotImplemented { phase }))
}

/// Every command with the argument names it expects, read from the source of
/// the modules that declare them.
///
/// Reading the source rather than keeping a hand-written table is the point:
/// a hand-written list of parameter names is a third copy to forget, and the
/// failure it permits — a renamed Rust parameter against an unchanged
/// `client.ts` — is invisible to the compiler on both sides.
///
/// The names are converted to the `camelCase` form Tauri puts on the wire, so
/// they can be compared directly with the object literals the client passes.
pub fn signatures() -> Vec<(&'static str, Vec<String>)> {
    let sources: &[&str] = &[
        include_str!("meta.rs"),
        include_str!("onboarding.rs"),
        include_str!("privacy.rs"),
        include_str!("events.rs"),
        include_str!("checkpoints.rs"),
        include_str!("analysis.rs"),
        include_str!("change_map.rs"),
        include_str!("collectors.rs"),
        include_str!("settings.rs"),
        include_str!("storage.rs"),
        include_str!("system_map.rs"),
        include_str!("diagnostics.rs"),
        include_str!("dev.rs"),
        include_str!("tray.rs"),
    ];
    COMMANDS
        .iter()
        .map(|name| {
            let args = sources
                .iter()
                .find_map(|source| arguments_of(source, name))
                .unwrap_or_default();
            (*name, args)
        })
        .collect()
}

/// The argument names of `name` in `source`, or `None` if it is not there.
///
/// Parameters Tauri injects rather than takes from the caller — the app
/// handle and the managed state — are dropped, because the frontend never
/// sends them.
///
/// Both `pub fn` and `pub async fn` are matched. A command that touches the
/// database is async, and a reader that knew only one form would quietly
/// report "no arguments" for the other — which is the failure this whole
/// mechanism exists to prevent, so it must not be the mechanism's own bug.
fn arguments_of(source: &str, name: &str) -> Option<Vec<String>> {
    let signature = ["pub fn ", "pub async fn "]
        .iter()
        .find_map(|prefix| source.split(&format!("{prefix}{name}(")).nth(1))?;
    let parameters = &signature[..signature.find(')')?];
    let mut names = Vec::new();
    for parameter in parameters.split(',') {
        let Some((binding, ty)) = parameter.split_once(':') else {
            continue;
        };
        // `mut` is a Rust binding modifier, not part of the IPC parameter
        // name. Commands may mutate a validated/clamped DTO before handing it
        // to a service, while Tauri still receives the original field name.
        let binding = binding
            .trim()
            .strip_prefix("mut ")
            .unwrap_or_else(|| binding.trim())
            .trim_start_matches('_');
        if binding.is_empty() || is_injected(ty) {
            continue;
        }
        names.push(to_camel_case(binding));
    }
    Some(names)
}

/// Whether Tauri supplies this parameter rather than the caller.
fn is_injected(ty: &str) -> bool {
    let ty = ty.trim();
    ty.starts_with("State<") || ty.starts_with("AppHandle") || ty.starts_with("tauri::AppHandle")
}

/// `collector_id` becomes `collectorId`, which is what arrives over IPC.
fn to_camel_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut capitalize = false;
    for c in name.chars() {
        if c == '_' {
            capitalize = true;
        } else if capitalize {
            out.extend(c.to_uppercase());
            capitalize = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// The names registered with `generate_handler!`, in registration order.
///
/// Kept beside the handler so the contract test can compare the registered
/// surface against the generated bindings without parsing a macro. The test
/// also asserts this list matches the macro invocation itself, so the two
/// cannot drift.
pub const COMMANDS: &[&str] = &[
    // meta
    "get_app_version",
    "get_app_status",
    "log_ui_error",
    "complete_onboarding",
    "check_for_updates",
    // onboarding and startup
    "get_onboarding_state",
    "reopen_onboarding",
    "skip_onboarding",
    "set_autostart",
    "set_tray_forced_colors",
    // events
    "get_dashboard_summary",
    "get_events",
    "expand_cluster",
    "get_event_details",
    "search_events",
    "get_system_map",
    // checkpoints
    "list_checkpoints",
    "create_checkpoint",
    "rename_checkpoint",
    "delete_checkpoint",
    "compare_checkpoint",
    // analysis
    "start_investigation",
    "list_investigations",
    "get_investigation",
    "delete_investigation",
    "analyze_window",
    "get_related_events",
    "export_investigation_context",
    "get_change_map",
    // collectors
    "get_collector_status",
    "list_collector_status",
    "set_collector_enabled",
    "set_monitoring_paused",
    "pause_monitoring",
    "resume_monitoring",
    "get_monitor_state",
    // settings
    "get_settings",
    "update_settings",
    "list_watched_locations",
    "add_watched_location",
    "remove_watched_location",
    "list_projects",
    "add_project",
    "remove_project",
    "mark_project_working",
    "clear_project_baseline",
    // storage and diagnostics
    "get_storage_usage",
    "get_diagnostics",
    "export_diagnostics",
    "db_recover",
    // privacy and destructive history controls
    "request_purge_challenge",
    "get_privacy_summary",
    "purge_category",
    "purge_folder",
    "purge_all",
    "set_retention",
    "set_quiet_hours",
    "get_quiet_hours",
    // This entry, like the handler registration, disappears from release
    // builds that do not opt into the development tools feature.
    #[cfg(any(debug_assertions, feature = "devtools"))]
    "simulate_event",
    #[cfg(any(debug_assertions, feature = "devtools"))]
    "seed_synthetic",
];

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_surface_matches_the_documented_typed_commands() {
        assert_eq!(COMMANDS.len(), 60);
        assert!(COMMANDS.contains(&"simulate_event"));
        assert!(COMMANDS.contains(&"seed_synthetic"));
        assert!(COMMANDS.contains(&"mark_project_working"));
        assert!(COMMANDS.contains(&"clear_project_baseline"));
        assert!(COMMANDS.contains(&"export_investigation_context"));
        assert_eq!(
            COMMANDS
                .iter()
                .filter(|c| !matches!(**c, "simulate_event" | "seed_synthetic"))
                .count(),
            58
        );
    }

    #[test]
    fn no_command_is_registered_twice() {
        let mut sorted = COMMANDS.to_vec();
        sorted.sort_unstable();
        let before = sorted.len();
        sorted.dedup();
        assert_eq!(sorted.len(), before);
    }

    #[test]
    fn a_stub_names_the_phase_that_owns_it() {
        let error = not_implemented::<()>("phase-14").unwrap_err();
        assert!(error.message.contains("phase-14"));
        assert!(!error.retryable);
    }

    #[test]
    fn every_command_signature_is_found_in_the_source() {
        let signatures = signatures();
        assert_eq!(signatures.len(), COMMANDS.len());
        // A command whose source could not be parsed would silently get an
        // empty argument list, so the ones that take arguments are named.
        let by_name: std::collections::BTreeMap<_, _> = signatures.into_iter().collect();
        assert_eq!(by_name["get_event_details"], vec!["eventId"]);
        assert_eq!(
            by_name["set_collector_enabled"],
            vec!["collectorId", "enabled"]
        );
        assert_eq!(by_name["set_monitoring_paused"], vec!["paused", "minutes"]);
        assert_eq!(by_name["add_watched_location"], vec!["path", "maxDepth"]);
        assert_eq!(by_name["log_ui_error"], vec!["message", "context"]);
        assert_eq!(by_name["update_settings"], vec!["update"]);
    }

    #[test]
    fn the_injected_parameters_are_dropped_because_the_frontend_never_sends_them() {
        let by_name: std::collections::BTreeMap<_, _> = signatures().into_iter().collect();
        // `get_app_version(app: AppHandle)` and `get_settings(state: State<…>)`
        // both take a parameter; neither takes one from the caller.
        assert!(by_name["get_app_version"].is_empty());
        assert!(by_name["get_settings"].is_empty());
        assert!(by_name["get_diagnostics"].is_empty(), "takes both");
    }

    #[test]
    fn snake_case_parameters_are_converted_the_way_tauri_converts_them() {
        assert_eq!(to_camel_case("collector_id"), "collectorId");
        assert_eq!(to_camel_case("max_depth"), "maxDepth");
        assert_eq!(to_camel_case("paused"), "paused");
        assert_eq!(to_camel_case("investigation_id"), "investigationId");
    }

    #[test]
    fn mutable_bindings_keep_their_wire_name() {
        let source =
            "pub async fn get_events(mut query: EventQuery, state: State<'_, AppState>) {}";
        assert_eq!(
            arguments_of(source, "get_events"),
            Some(vec!["query".to_owned()])
        );
    }

    #[test]
    fn the_signature_reader_can_actually_fail() {
        // A gate that cannot fail proves nothing: the parser must return
        // `None` for a command that is not in the source it was given.
        assert!(arguments_of("pub fn something_else(a: i64) {}", "get_events").is_none());
        assert_eq!(
            arguments_of("pub fn get_events(query: EventQuery) {}", "get_events"),
            Some(vec!["query".to_owned()])
        );
    }

    #[test]
    fn an_async_command_reads_the_same_as_a_synchronous_one() {
        // Marking a command `async` must not silently empty its argument
        // list. It did once: the reader matched only `pub fn`, so every
        // database-touching command reported no arguments the moment it moved
        // off the UI thread.
        assert_eq!(
            arguments_of(
                "pub async fn update_settings(state: State<'_, AppState>, update: Dto) {}",
                "update_settings"
            ),
            Some(vec!["update".to_owned()])
        );
        // And the real sources agree: the async commands carry their names.
        let by_name: std::collections::BTreeMap<_, _> = signatures().into_iter().collect();
        assert_eq!(by_name["update_settings"], vec!["update"]);
        assert_eq!(by_name["db_recover"], vec!["action"]);
        assert_eq!(by_name["simulate_event"], vec!["kind"]);
        assert_eq!(by_name["seed_synthetic"], vec!["profile", "relations"]);
    }
}
