use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::collectors::stateful;
use crate::privacy::{Sanitizer, Scrub};
use crate::storage::Baseline;

/// Trigger families retained from Task Scheduler. Only the first three are
/// startup triggers; all other tasks remain visible as informational history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskTriggerKind {
    Boot,
    Logon,
    Idle,
    Schedule,
    Event,
    Other,
}

impl TaskTriggerKind {
    pub const fn is_startup(self) -> bool {
        matches!(self, Self::Boot | Self::Logon | Self::Idle)
    }
}

/// Platform-neutral scheduled-task row. COM objects never cross this boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub name: String,
    pub path: String,
    pub enabled: bool,
    pub triggers: Vec<TaskTriggerKind>,
    pub action_path: Option<String>,
    pub last_run_ms: Option<i64>,
    pub last_result: i32,
}

impl ScheduledTask {
    pub fn is_startup(&self) -> bool {
        self.triggers.iter().any(|trigger| trigger.is_startup())
    }

    pub(super) fn key(&self) -> String {
        self.path.to_ascii_lowercase()
    }
}

/// One of the exact autostart locations collected by this phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupLocation {
    CurrentUserRun,
    CurrentUserRunOnce,
    LocalMachineRun,
    LocalMachineRunOnce,
    CurrentUserStartupFolder,
    CommonStartupFolder,
}

impl StartupLocation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CurrentUserRun => "HKCU Run",
            Self::CurrentUserRunOnce => "HKCU RunOnce",
            Self::LocalMachineRun => "HKLM Run",
            Self::LocalMachineRunOnce => "HKLM RunOnce",
            Self::CurrentUserStartupFolder => "current-user Startup folder",
            Self::CommonStartupFolder => "common Startup folder",
        }
    }
}

/// A named command in one of the four Run/RunOnce keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEntry {
    pub location: StartupLocation,
    pub name: String,
    pub command: String,
    /// StartupApproved is read-only and only exists for current-user entries.
    pub approved: Option<bool>,
}

impl RunEntry {
    pub(super) fn key(&self) -> String {
        format!("{:?}:{}", self.location, self.name.to_ascii_lowercase())
    }

    pub(super) fn identity(&self) -> String {
        format!("run:{:?}:{}", self.location, self.name)
    }
}

/// One direct child of either Startup folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupFile {
    pub location: StartupLocation,
    pub path: String,
    pub name: String,
    pub approved: Option<bool>,
}

impl StartupFile {
    pub(super) fn key(&self) -> String {
        format!("{:?}:{}", self.location, self.path.to_ascii_lowercase())
    }

    pub(super) fn identity(&self) -> String {
        format!("startup-file:{:?}:{}", self.location, self.path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartupInventory {
    pub tasks: Vec<ScheduledTask>,
    pub run_entries: Vec<RunEntry>,
    pub startup_files: Vec<StartupFile>,
    /// False means Task Scheduler could not provide a complete root walk. The
    /// collector overlays readable rows onto its last acknowledged tasks,
    /// detecting safe additions/changes while suppressing uncertain removals.
    pub task_store_available: bool,
}

impl Default for StartupInventory {
    fn default() -> Self {
        Self {
            tasks: Vec::new(),
            run_entries: Vec::new(),
            startup_files: Vec::new(),
            task_store_available: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(super) struct StartupState {
    pub acknowledged: IndexedInventory,
    #[serde(default)]
    pub pending: Option<stateful::PendingTarget<IndexedInventory>>,
    pub established: bool,
}

impl StartupState {
    pub(super) fn hydrate(baseline: Option<Baseline>, now_ms: i64) -> (Self, bool) {
        stateful::hydrate(baseline, now_ms, "the startup/task table")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(super) struct IndexedInventory {
    pub tasks: BTreeMap<String, ScheduledTask>,
    pub run_entries: BTreeMap<String, RunEntry>,
    pub startup_files: BTreeMap<String, StartupFile>,
}

impl From<StartupInventory> for IndexedInventory {
    fn from(value: StartupInventory) -> Self {
        Self {
            tasks: value
                .tasks
                .into_iter()
                .map(|task| (task.key(), task))
                .collect(),
            run_entries: value
                .run_entries
                .into_iter()
                .map(|entry| (entry.key(), entry))
                .collect(),
            startup_files: value
                .startup_files
                .into_iter()
                .map(|file| (file.key(), file))
                .collect(),
        }
    }
}

/// Canonicalize sensitive command/path material before both diffing and
/// baseline serialization. The storage chokepoint sanitizes again, but doing
/// it here prevents a sanitized durable baseline from looking different from
/// the same live command after every restart.
pub(super) fn sanitize(mut inventory: StartupInventory) -> StartupInventory {
    let sanitizer = Sanitizer::new();
    let mut report = Vec::new();
    for task in &mut inventory.tasks {
        task.action_path
            .scrub_in_place(&sanitizer, "action_path", &mut report);
    }
    for entry in &mut inventory.run_entries {
        entry
            .command
            .scrub_in_place(&sanitizer, "command", &mut report);
    }
    for file in &mut inventory.startup_files {
        file.path.scrub_in_place(&sanitizer, "path", &mut report);
    }
    inventory
}
