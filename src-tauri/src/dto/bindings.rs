//! Generates `src/lib/tauri/bindings.ts` from the Rust types.
//!
//! `ts-rs` writes one file per type by default. This project wants one file,
//! because thirty imports of thirty one-line modules is a build-time cost and
//! a reading cost for nothing. So the declarations are collected here and
//! concatenated in a fixed order, and the command signatures — which no
//! derive can know about — are written from `COMMANDS` alongside them.
//!
//! The generated file is committed. `bun run check:bindings` regenerates it to
//! a temporary file and compares, so a renamed Rust field fails the build
//! instead of failing the UI at runtime.

use std::fmt::Write as _;

use ts_rs::{Config, TS};

use crate::domain::{
    Category, ChainStep, CheckpointKind, CollectorCadenceDto, CollectorCost,
    CollectorDescriptorDto, CollectorStatusDetails, CollectorWakeTier, Confidence, ConfidenceClass,
    DockerCollectorState, Entity, EntityType, EventQuery, EventSignature, EventSummary, EventType,
    Importance, InvestigationWindow, MonitoringProfile, NoCauseReason, NormalizedEvent, Page,
    Relationship, RelationshipKind, ResolvedInvestigationWindow, RetentionPolicy, ScoreBreakdown,
    Settings, SettingsUpdate, Severity, SystemEvent, Theme, TimeRange, VisualizationMode,
};
use crate::dto::{
    AppErrorDto, AppStatusDto, ChangeMapData, ChangeMapEdge, ChangeMapEdgeKind, ChangeMapLane,
    ChangeMapLaneData, ChangeMapNode, ChangeMapRequest, CheckpointCategoryDiffDto,
    CheckpointDiffDto, CheckpointDto, CheckpointRef, CollectorState, CollectorStatusDto,
    DashboardCountsDto, DashboardSummaryDto, DiagnosticsDto, DiffItemDto, EntityKindAvailability,
    EntityRef, ErrorCode, EventDetailsDto, InvestigationDto, InvestigationRequestDto,
    MonitoringDto, OnboardingStateDto, PrivacyCategoryCountDto, PrivacySummaryDto, ProjectDto,
    PurgeReportDto, SearchResultDto, SuspectDto, SystemMapData, SystemMapEdge, SystemMapNode,
    SystemMapRecentEvent, UpdateCheckDto, WatchedLocationDto,
};
use crate::notify::QuietHours;
use crate::services::cluster::{ClusterRef, TimelineItem};
use crate::services::query_service::TimelinePage;
use crate::storage::{
    BatchNotice, DbState, MaintenanceReport, RecoveryAction, RecoveryOutcome, StorageUsage,
};

/// The header. Says where the file comes from and how to regenerate it,
/// because the first instinct on seeing a generated file is to edit it.
///
/// ASCII only, deliberately. The file is compared byte for byte by
/// `check:bindings`, and a non-ASCII character is one more thing that can
/// differ between two checkouts for a reason that has nothing to do with types.
const HEADER: &str = "\
/**
 * GENERATED FILE - do not edit.
 *
 * Rust is the source of truth for these shapes. Regenerate with:
 *
 *     bun run gen:bindings
 *
 * `bun run check:bindings` fails the build when this file disagrees with the
 * Rust types, so a renamed field breaks the build rather than the interface.
 */
";

/// The one type `ts-rs` names but does not declare.
///
/// An event's `before`, `after` and `metadata` are arbitrary JSON, because a
/// collector decides what is worth keeping about the thing it watched. The
/// recursive definition is the honest one: narrowing it to `object` would be a
/// type that lies about half the values that reach it.
const JSON_VALUE: &str = "
// ---- JSON ----

/** Arbitrary JSON, as a collector recorded it. Already redacted. */
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
";

/// One exported declaration, in the order the file lists them.
macro_rules! declarations {
    ($config:expr, $out:expr, $( $ty:ty ),* $(,)?) => {
        $(
            let _ = writeln!($out, "export {}", <$ty as TS>::decl($config));
        )*
    };
}

/// The whole binding file, as text.
pub fn generate() -> String {
    // Large integers are `number`, not `bigint`. Every one of them here is a
    // row id or an epoch millisecond count, both far inside the range a double
    // holds exactly, and `bigint` would infect every arithmetic expression in
    // the frontend for a precision problem this data cannot have.
    let config = Config::new().with_large_int("number");
    let mut out = String::with_capacity(8 * 1024);
    out.push_str(HEADER);

    out.push_str("\n// ---- Domain vocabularies ----\n\n");
    declarations!(
        &config,
        out,
        Category,
        EventType,
        EntityType,
        Severity,
        Importance,
        Confidence,
        ConfidenceClass,
        InvestigationWindow,
        ResolvedInvestigationWindow,
        NoCauseReason,
        ScoreBreakdown,
        ChainStep,
        CheckpointKind,
        RelationshipKind,
        CollectorCost,
        CollectorCadenceDto,
        CollectorDescriptorDto,
    );

    // `ts-rs` renders `serde_json::Value` as `JsonValue` and never declares
    // it — the type is a leaf as far as the derive is concerned. Without this
    // the generated file references a name TypeScript has never heard of, and
    // `bun run check` fails on seven lines nobody wrote.
    out.push_str(JSON_VALUE);

    out.push_str("\n// ---- Events and entities ----\n\n");
    declarations!(
        &config,
        out,
        NormalizedEvent,
        SystemEvent,
        EventSignature,
        EventSummary,
        ClusterRef,
        TimelineItem,
        TimelinePage,
        Entity,
        Relationship,
        EventQuery,
        TimeRange,
        RetentionPolicy,
    );

    // Cursor has a custom serde representation. ts-rs otherwise sees no fields
    // (they are intentionally skipped) and emits `{}`; the wire value is text.
    out.push_str("\nexport type Cursor = string;\n");

    let _ = writeln!(out, "export {}", <Page<EventSummary> as TS>::decl(&config));

    out.push_str("\n// ---- Storage ----\n\n");
    declarations!(
        &config,
        out,
        DbState,
        StorageUsage,
        MaintenanceReport,
        RecoveryAction,
        RecoveryOutcome,
        BatchNotice,
    );

    out.push_str("\n// ---- Transfer types ----\n\n");
    declarations!(
        &config,
        out,
        ErrorCode,
        AppErrorDto,
        Theme,
        VisualizationMode,
        MonitoringProfile,
        Settings,
        SettingsUpdate,
        MonitoringDto,
        AppStatusDto,
        CollectorState,
        CollectorWakeTier,
        DockerCollectorState,
        CollectorStatusDetails,
        CollectorStatusDto,
        DiagnosticsDto,
        UpdateCheckDto,
        DashboardCountsDto,
        CheckpointRef,
        DashboardSummaryDto,
        EventDetailsDto,
        SearchResultDto,
        WatchedLocationDto,
        ProjectDto,
        CheckpointDto,
        DiffItemDto,
        CheckpointCategoryDiffDto,
        CheckpointDiffDto,
        InvestigationRequestDto,
        SuspectDto,
        InvestigationDto,
        ChangeMapRequest,
        ChangeMapLane,
        ChangeMapLaneData,
        ChangeMapNode,
        ChangeMapEdgeKind,
        ChangeMapEdge,
        ChangeMapData,
        EntityRef,
        SystemMapNode,
        SystemMapEdge,
        SystemMapRecentEvent,
        EntityKindAvailability,
        SystemMapData,
        OnboardingStateDto,
        PrivacyCategoryCountDto,
        PrivacySummaryDto,
        PurgeReportDto,
        QuietHours,
    );

    out.push_str(COMMAND_SECTION);
    out
}

/// The command names and their argument names, as values the typed client
/// and the contract test index.
///
/// Derived from the `#[tauri::command]` signatures by reading the source, not
/// hand-written: a hand-written list is a third place to forget. Names alone
/// would not be enough — Tauri matches arguments by name, so renaming a Rust
/// parameter breaks the call at runtime while the build, the bindings check
/// and a name-only contract test all stay green. That is the one runtime
/// failure this whole surface exists to prevent, so the argument names are
/// generated and compared too.
const COMMAND_SECTION: &str = r#"
// ---- Commands ----

/**
 * Every command the backend registers, with the argument names it expects.
 * `client.ts` exposes one typed function per entry; `commands_contract.rs`
 * asserts this table, the Rust handler and the object literals `client.ts`
 * passes all agree — so neither a missing command nor a misspelled argument
 * can reach a running window.
 *
 * Tauri converts `snake_case` parameters to `camelCase` on the wire, so the
 * names here are the camelCase form the frontend actually sends.
 */
export const COMMANDS = {
"#;

/// The generated file, with the command table filled in.
pub fn generate_full() -> String {
    let mut out = generate();
    for (name, args) in crate::commands::signatures() {
        let rendered: Vec<String> = args.iter().map(|a| format!("'{a}'")).collect();
        let _ = writeln!(out, "  {name}: [{}],", rendered.join(", "));
    }
    out.push_str(
        "} as const;\n\
         \n\
         export type CommandName = keyof typeof COMMANDS;\n",
    );
    let had_trailing_newline = out.ends_with('\n');
    let mut clean = out
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    if had_trailing_newline {
        clean.push('\n');
    }
    clean
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_declaration_is_exported_and_the_braces_balance() {
        let text = generate_full();
        let declarations = text
            .lines()
            .filter(|line| line.starts_with("export type"))
            .count();
        assert!(
            declarations >= 40,
            "only {declarations} declarations were generated; the list is incomplete"
        );
        // A truncated declaration is the failure worth catching — it type-checks
        // as far as `starts_with` is concerned and breaks the file. Counting
        // braces over the whole output catches it wherever it happens.
        let opened = text.matches('{').count();
        let closed = text.matches('}').count();
        assert_eq!(opened, closed, "a generated declaration is unterminated");
        assert!(text.starts_with("/**"), "the header comes first");
        assert!(text.contains("do not edit"));
    }

    #[test]
    fn a_truncated_declaration_would_be_caught() {
        // The check above is only worth having if it fails on the thing it
        // describes, so the failure is constructed here.
        let broken = format!("{}\nexport type Broken = {{ a: number,\n", generate_full());
        assert_ne!(
            broken.matches('{').count(),
            broken.matches('}').count(),
            "an unterminated declaration must not balance"
        );
    }

    #[test]
    fn the_vocabularies_arrive_as_string_unions_a_screen_can_switch_on() {
        let text = generate_full();
        assert!(
            text.contains(r#""configuration""#),
            "categories are their stored words"
        );
        assert!(text.contains(r#""critical""#));
        assert!(text.contains(r#""not_implemented""#), "error codes too");
    }

    #[test]
    fn ids_and_timestamps_are_numbers_rather_than_bigints() {
        let text = generate_full();
        assert!(
            !text.contains("bigint"),
            "a row id in a bigint would infect every arithmetic expression in the UI"
        );
    }

    #[test]
    fn every_registered_command_appears_in_the_generated_list() {
        let text = generate_full();
        for name in crate::commands::COMMANDS {
            assert!(
                text.contains(&format!("  {name}: [")),
                "{name} is registered but absent from the bindings"
            );
        }
        assert!(text.contains("export type CommandName"));
    }

    #[test]
    fn the_generated_table_carries_the_argument_names_the_frontend_must_send() {
        let text = generate_full();
        // Tauri matches arguments by name, so these are the strings that
        // decide whether a call arrives with its parameters or without them.
        assert!(text.contains("get_event_details: ['eventId'],"));
        assert!(text.contains("expand_cluster: ['cluster'],"));
        assert!(text.contains("set_collector_enabled: ['collectorId', 'enabled'],"));
        assert!(
            text.contains("get_app_version: [],"),
            "no arguments is empty"
        );
    }

    #[test]
    fn the_one_type_ts_rs_names_but_does_not_declare_is_declared_here() {
        let text = generate_full();
        assert!(
            text.contains("export type JsonValue ="),
            "an event's before/after reference JsonValue, so it must be declared"
        );
        assert!(text.contains("before: JsonValue | null"));
    }

    #[test]
    fn a_partial_settings_update_omits_rather_than_nulls() {
        let text = generate_full();
        assert!(
            text.contains("theme?: Theme | null"),
            "a toggle sends the one field it changed; the rest are absent"
        );
        assert!(
            text.contains("export type SettingsUpdate ="),
            "the update shape is the domain type, not a copy of it"
        );
    }

    #[test]
    fn the_status_shape_carries_what_the_shell_boots_on() {
        let text = generate_full();
        for field in [
            "tray_available",
            "safe_mode",
            "dropped_writes",
            "monitoring",
        ] {
            assert!(text.contains(field), "AppStatusDto is missing {field}");
        }
    }
}
