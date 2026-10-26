//! What the shell asks for before it can draw anything.
//!
//! One call answers the four questions the frame depends on: is the database
//! usable, is monitoring running, did the tray icon appear, and did the last
//! three starts go badly enough to drop into safe mode. Splitting those across
//! four commands would mean four round trips before first paint and four ways
//! for the shell to be half-informed.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::enums::text_enum;
use crate::storage::DbState;

/// The shell's boot answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct AppStatusDto {
    /// Whether the database is usable, and if not, why.
    pub db: DbState,
    /// Whether monitoring is running.
    pub monitoring: MonitoringDto,
    /// Whether the tray icon was created.
    ///
    /// `false` means closing the window would strand the user with an
    /// invisible process, so the window stays closable and the shell shows a
    /// banner saying so. Never silently hide into a tray that is not there.
    pub tray_available: bool,
    /// Whether this start followed three unclean ones, so collectors stay off.
    pub safe_mode: bool,
    /// Events dropped because the write queue was full, since this start.
    pub dropped_writes: u64,
}

/// Whether the recorder is recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct MonitoringDto {
    /// True while collection is paused.
    pub paused: bool,
    /// When the pause lifts by itself, epoch milliseconds UTC. `null` means it
    /// stays paused until the user resumes.
    pub resume_at: Option<i64>,
}

text_enum! {
    /// What a collector is doing. `Unavailable` means no provider is active;
    /// it must not read as "watching, nothing to report".
    CollectorState, "collector state" {
        Running => "running",
        Paused => "paused",
        Idle => "idle",
        Failed => "failed",
        Degraded => "degraded",
        Disabled => "disabled",
        Unavailable => "unavailable",
    }
}

/// One row of the Diagnostics collector table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CollectorStatusDto {
    /// Self-described Settings copy and scheduling cost.
    pub descriptor: crate::domain::CollectorDescriptorDto,
    /// Collector id, as stored in the `collectors` setting.
    pub id: String,
    /// Whether the user has it switched on.
    pub enabled: bool,
    /// What it is doing.
    pub state: CollectorState,
    /// Last completed run, epoch milliseconds UTC.
    pub last_run_at: Option<i64>,
    /// How long that run took.
    pub last_duration_ms: Option<i64>,
    /// Legacy event rows admitted to the queue, plus atomic rows confirmed
    /// committed, since registration. Legacy commit failures are counted
    /// separately in `failed_commit_rows`; this is not a durable history total.
    pub events_emitted: u64,
    /// Failures since its last success.
    pub consecutive_failures: u64,
    /// Why it last failed, already scrubbed. Never a value it was reading.
    pub last_error: Option<String>,
    /// Collector-specific, bounded diagnostics such as the active driver wake
    /// tier. Platform providers never place raw observed values here.
    pub details: Option<crate::domain::CollectorStatusDetails>,
    /// Stated so the privilege promise is visible in the product, not only in
    /// the documentation. No collector may report `true`.
    pub requires_admin: bool,
    /// Rows rejected before entering the bounded writer queue.
    pub queue_drops: u64,
    /// Rows discarded only after SQLite rejected a complete batch.
    pub failed_commit_rows: u64,
    /// Atomic collector cycles rejected before entering the writer queue.
    /// Kept separate from `queue_drops`, whose unit is event rows.
    pub cycle_admission_drops: u64,
    /// Admitted atomic collector cycles whose SQLite transaction failed.
    /// Kept separate from `failed_commit_rows`, whose unit is event rows.
    pub failed_cycles: u64,
}

/// The Diagnostics screen's payload. Shape owned by phase 19.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DiagnosticsDto {
    /// Version of the running build.
    pub app_version: String,
    /// WebView runtime version. The legacy wire name is retained until the
    /// diagnostics export contract is expanded; this is not the Windows version.
    pub os_version: String,
    /// Database state.
    pub db: DbState,
    /// Database size on disk, bytes.
    pub database_bytes: u64,
    /// Schema version in the file.
    pub schema_version: u32,
    /// Per-collector health.
    pub collectors: Vec<CollectorStatusDto>,
    /// Events dropped because the write queue was full.
    pub dropped_writes: u64,
    /// Rows lost after SQLite rejected an admitted batch.
    pub failed_commit_rows: u64,
    /// Atomic collector cycles rejected before entering the writer queue.
    pub cycle_admission_drops: u64,
    /// Admitted atomic collector cycles whose SQLite transaction failed.
    pub failed_cycles: u64,
    /// Approximate pending writer work. A zero means the queue is idle or the
    /// runtime does not expose a cheap depth counter.
    pub write_queue_depth: u64,
    /// Current process working set, when the platform provider can read it.
    pub current_working_set_bytes: u64,
    /// Peak process working set, when the platform provider can read it.
    pub peak_working_set_bytes: u64,
    /// Total bytes in retained redaction-filtered log files.
    pub log_bytes: u64,
}

/// The answer to "check for updates". Shape owned by phase 21.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct UpdateCheckDto {
    /// Whether a signed updater is configured for this build.
    pub supported: bool,
    /// Whether the user explicitly requested an update check for this call.
    /// This is echoed for the UI; it is never persisted or acted on remotely.
    pub opted_in: bool,
    /// The version that is running.
    pub current_version: String,
    /// What to tell the user instead.
    pub message: String,
}
