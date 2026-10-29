//! Environment, developer-tool and watched-repository branch changes.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::time::Duration;

use crate::collectors::{
    stateful, AckedState, CancelToken, Collector, CollectorContext, CollectorDescriptor,
    CollectorSnapshot, CommitToken, EntityIdentity, ObservationClock, RawObservation,
    SnapshotContext, SystemObservationClock, WakeHandle,
};
use crate::domain::{Category, EventType};
use crate::error::{AppError, AppResult};
use crate::storage::CollectorCommitReceipt;
use serde::{Deserialize, Serialize};

pub const ID: &str = "dev_env";
mod observations;
const BASELINE_KIND: &str = "dev_environment";
const TOOL_RESCAN_INTERVAL_MS: i64 = Duration::from_secs(6 * 60 * 60).as_millis() as i64;

/// Read the current branch from either a normal repository or a Git worktree.
///
/// Settings uses the same bounded parser as the collector so project cards and
/// emitted observations cannot disagree about worktree indirection.
pub(crate) fn branch_for_root(root: &std::path::Path) -> Option<String> {
    observations::read_branch(root)
}

pub trait DevEnvironmentNotificationGuard: Send {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolState {
    pub path: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolScan {
    pub tools: BTreeMap<String, ToolState>,
    pub complete: bool,
}

pub trait DevEnvironmentProvider: Send + Sync + 'static {
    fn environment_variables(&self) -> AppResult<BTreeMap<String, String>>;
    fn probe_tools(&self, path: &str) -> ToolScan;
    fn register_notifications(
        &self,
        wake: WakeHandle,
        cancel: CancelToken,
    ) -> AppResult<Box<dyn DevEnvironmentNotificationGuard>>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct DevEnvironmentState {
    environment: BTreeMap<String, String>,
    // Raw comparison is available only for observations from this process.
    // This whole-snapshot flag never persists or encodes a secret's identity.
    #[serde(skip)]
    environment_values_known: bool,
    path_entries: Vec<String>,
    tools: BTreeMap<String, ToolState>,
    branches: BTreeMap<String, String>,
    last_tool_scan_ms: Option<i64>,
    established: bool,
}

impl DevEnvironmentState {
    fn durable(&self) -> Self {
        let mut durable = self.clone();
        for value in durable.environment.values_mut() {
            *value = crate::privacy::REDACTED.to_owned();
        }
        durable
    }
}

struct PendingDelivery {
    chunks: VecDeque<PendingChunk>,
    captured_at: i64,
}

/// One acknowledgement-sized prefix of a provider diff.
///
/// The state belongs to the prefix, rather than only to the complete provider
/// target.  This is important when a large environment burst is drained over
/// several transactions: after an intermediate commit a recreated collector
/// can hydrate the exact prefix it acknowledged and will not replay the first
/// 200 observations forever.
struct PendingChunk {
    observations: Vec<RawObservation>,
    state: DevEnvironmentState,
}

pub struct DevEnvironmentCollector<P: DevEnvironmentProvider, C = SystemObservationClock> {
    provider: P,
    clock: C,
    descriptor: CollectorDescriptor,
    context: Option<CollectorContext>,
    notifications: Option<Box<dyn DevEnvironmentNotificationGuard>>,
    branch_watcher: Option<notify::RecommendedWatcher>,
    watched_roots: Vec<PathBuf>,
    state: AckedState<DevEnvironmentState>,
    pending: Option<PendingDelivery>,
    downtime: bool,
}

impl<P: DevEnvironmentProvider> DevEnvironmentCollector<P, SystemObservationClock> {
    pub fn new(provider: P, descriptor: CollectorDescriptor) -> Self {
        Self::with_clock(provider, descriptor, SystemObservationClock)
    }
}

impl<P: DevEnvironmentProvider, C: ObservationClock> DevEnvironmentCollector<P, C> {
    pub fn with_clock(provider: P, descriptor: CollectorDescriptor, clock: C) -> Self {
        Self {
            provider,
            clock,
            descriptor,
            context: None,
            notifications: None,
            branch_watcher: None,
            watched_roots: Vec::new(),
            state: AckedState::new(DevEnvironmentState::default()),
            pending: None,
            downtime: false,
        }
    }

    fn roots(context: &CollectorContext) -> AppResult<Vec<PathBuf>> {
        Ok(context
            .state
            .watched_locations()?
            .into_iter()
            .filter(|location| location.enabled)
            .map(|location| PathBuf::from(location.path))
            .collect())
    }

    fn start_branch_watcher(&mut self, context: &CollectorContext) {
        use notify::Watcher;
        let wake = context.wake.clone();
        let heads = self
            .watched_roots
            .iter()
            .filter_map(|root| observations::git_head_path(root))
            .collect::<Vec<_>>();
        let mut watcher = match notify::recommended_watcher(
            move |result: notify::Result<notify::Event>| {
                if result.is_ok_and(|event| {
                    event
                        .paths
                        .iter()
                        .any(|path| heads.iter().any(|head| observations::same_path(path, head)))
                }) {
                    let _ = wake.request_snapshot();
                }
            },
        ) {
            Ok(watcher) => watcher,
            Err(error) => {
                tracing::debug!(reason = %error, "git HEAD notifications unavailable; using fallback sweep");
                return;
            }
        };
        for root in &self.watched_roots {
            if let Some(parent) =
                observations::git_head_path(root).and_then(|head| head.parent().map(PathBuf::from))
            {
                let _ = watcher.watch(&parent, notify::RecursiveMode::NonRecursive);
            }
        }
        self.branch_watcher = Some(watcher);
    }

    fn current_state(&self, now_ms: i64) -> AppResult<DevEnvironmentState> {
        let environment =
            observations::normalize_environment(self.provider.environment_variables()?);
        let path_entries = environment
            .get("PATH")
            .map(|value| crate::domain::tool::path_entries(value))
            .unwrap_or_default();
        let path_changed = path_entries != self.state.committed().path_entries;
        let tool_scan_due = !self.state.committed().established
            || path_changed
            || self
                .state
                .committed()
                .last_tool_scan_ms
                .is_none_or(|last| now_ms.saturating_sub(last) >= TOOL_RESCAN_INTERVAL_MS);
        let mut tools = if tool_scan_due {
            let scan = self
                .provider
                .probe_tools(environment.get("PATH").map(String::as_str).unwrap_or(""));
            let mut tools = scan.tools;
            if !scan.complete {
                for (name, previous) in &self.state.committed().tools {
                    tools
                        .entry(name.clone())
                        .or_insert_with(|| previous.clone());
                }
            }
            tools
        } else {
            self.state.committed().tools.clone()
        };
        for (name, current) in &mut tools {
            if current.version.is_none() {
                let previous = self.state.committed().tools.get(name);
                if previous
                    .is_some_and(|previous| previous.path.eq_ignore_ascii_case(&current.path))
                {
                    current.version = previous.and_then(|previous| previous.version.clone());
                }
            }
        }
        Ok(DevEnvironmentState {
            environment,
            environment_values_known: true,
            path_entries,
            tools,
            branches: observations::read_branches(&self.watched_roots),
            last_tool_scan_ms: if tool_scan_due {
                Some(now_ms)
            } else {
                self.state.committed().last_tool_scan_ms
            },
            established: true,
        })
    }

    fn reconcile_roots(&mut self) -> AppResult<()> {
        let Some(context) = self.context.as_ref().cloned() else {
            return Ok(());
        };
        let roots = Self::roots(&context)?;
        if roots != self.watched_roots {
            self.branch_watcher = None;
            self.watched_roots = roots;
            self.start_branch_watcher(&context);
        }
        Ok(())
    }

    fn begin_delivery(
        &mut self,
        token: CommitToken,
        target: DevEnvironmentState,
        captured_at: i64,
        observations: Vec<RawObservation>,
    ) -> AppResult<CollectorSnapshot> {
        let chunks = self.chunk_delivery(&target, observations);
        self.pending = Some(PendingDelivery {
            chunks,
            captured_at,
        });
        self.stage_pending(token)
    }

    /// Split a diff while advancing a matching in-memory state for every
    /// prefix.  A provider can legitimately report hundreds of independent
    /// environment/tool/branch changes in one sweep; each prefix must be a
    /// complete, replay-safe state rather than a batch with the final target
    /// hidden behind an uncommitted tail.
    fn chunk_delivery(
        &self,
        target: &DevEnvironmentState,
        observations: Vec<RawObservation>,
    ) -> VecDeque<PendingChunk> {
        let mut chunks = VecDeque::new();
        let mut prefix = self.state.committed().clone();
        let mut current = Vec::with_capacity(crate::storage::MAX_ATOMIC_OBSERVATIONS);
        for observation in observations {
            apply_observation_state(&mut prefix, target, &observation);
            current.push(observation);
            if current.len() == crate::storage::MAX_ATOMIC_OBSERVATIONS {
                chunks.push_back(PendingChunk {
                    observations: std::mem::take(&mut current),
                    state: prefix.clone(),
                });
            }
        }
        if current.is_empty() {
            if let Some(last) = chunks.back_mut() {
                apply_target_metadata(&mut last.state, target);
            } else {
                chunks.push_back(PendingChunk {
                    observations: Vec::new(),
                    state: target.clone(),
                });
            }
        } else {
            // Metadata that is not represented by an observation (for
            // example a completed tool scan timestamp) advances only with the
            // final prefix.  This keeps a crash between chunks from making a
            // later scan appear acknowledged when its observations were not.
            apply_target_metadata(&mut prefix, target);
            chunks.push_back(PendingChunk {
                observations: current,
                state: prefix,
            });
        }
        chunks
    }

    fn stage_pending(&mut self, token: CommitToken) -> AppResult<CollectorSnapshot> {
        let pending = self
            .pending
            .as_ref()
            .ok_or_else(|| AppError::InvalidInput {
                what: "developer environment delivery",
                reason: "no pending batch exists".to_owned(),
            })?;
        let chunk = pending
            .chunks
            .front()
            .ok_or_else(|| AppError::InvalidInput {
                what: "developer environment delivery",
                reason: "the pending delivery has no chunk".to_owned(),
            })?;
        let baseline = Some(durable_baseline(&chunk.state, pending.captured_at)?);
        self.state.stage(token, chunk.state.clone())?;
        Ok(CollectorSnapshot::Atomic {
            token,
            observations: chunk.observations.clone(),
            relationships: Vec::new(),
            baseline,
            config_snapshots: Vec::new(),
        })
    }
}

impl<P: DevEnvironmentProvider, C: ObservationClock> Collector for DevEnvironmentCollector<P, C> {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let now = self.clock.now_ms();
        let (stored, downtime) =
            stateful::hydrate(context.state.baseline(BASELINE_KIND)?, now, BASELINE_KIND);
        self.state = AckedState::new(stored);
        self.pending = None;
        self.downtime = downtime;
        self.watched_roots = Self::roots(&context)?;
        self.notifications = match self
            .provider
            .register_notifications(context.wake.clone(), context.cancel.clone())
        {
            Ok(guard) => Some(guard),
            Err(error) => {
                tracing::debug!(reason = %error, "environment notifications unavailable; using fallback sweep");
                None
            }
        };
        self.start_branch_watcher(&context);
        self.context = Some(context);
        Ok(())
    }

    fn stop(&mut self) {
        self.branch_watcher = None;
        self.notifications = None;
        self.context = None;
        self.watched_roots.clear();
        self.pending = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Err(AppError::InvalidInput {
            what: "developer environment collector",
            reason: "the collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        if self.context.is_none() {
            return Ok(CollectorSnapshot::Noop);
        }
        if self.pending.is_some() {
            return self.stage_pending(context.token());
        }
        self.reconcile_roots()?;
        let now = self.clock.now_ms();
        let next = self.current_state(now)?;
        let observations = if self.state.committed().established {
            observations::between(self.state.committed(), &next, now, self.downtime)
        } else {
            Vec::new()
        };
        self.begin_delivery(context.token(), next, now, observations)
    }

    fn commit_succeeded(
        &mut self,
        token: CommitToken,
        _receipt: &CollectorCommitReceipt,
    ) -> AppResult<()> {
        self.state.ack(token)?;
        let completed = if let Some(pending) = self.pending.as_mut() {
            pending.chunks.pop_front();
            pending.chunks.is_empty()
        } else {
            false
        };
        if completed {
            self.pending = None;
            self.downtime = false;
        } else if let Some(context) = &self.context {
            // A large deterministic burst is drained without waiting for the
            // fallback interval. Rejected chunks stay at the front and retry.
            let _ = context.wake.request_snapshot();
        }
        Ok(())
    }

    fn commit_failed(&mut self, token: CommitToken) -> AppResult<()> {
        // An empty establishment snapshot carries no observations that could
        // be lost, so retry it from the provider (and re-run the tool probe).
        // For a real burst retain the front chunk: re-reading the provider
        // could miss a short-lived change and would defeat acknowledgement
        // safety.
        let retry_provider = self
            .pending
            .as_ref()
            .and_then(|pending| pending.chunks.front())
            .is_some_and(|chunk| chunk.observations.is_empty());
        let result = self.state.reject(token);
        if result.is_ok() && retry_provider {
            self.pending = None;
        }
        result
    }
}

/// Advance only the fields proven by one raw observation.  This lets each
/// atomic chunk carry a prefix baseline without ever persisting the complete
/// target before its tail has been acknowledged.
fn apply_observation_state(
    state: &mut DevEnvironmentState,
    target: &DevEnvironmentState,
    observation: &RawObservation,
) {
    match observation.kind.category {
        Category::Configuration => {
            let Some(field) = observation.field.as_deref() else {
                return;
            };
            if field.eq_ignore_ascii_case("PATH") {
                apply_path_observation(state, target, observation);
            } else if let Some(value) = target.environment.get(field) {
                state.environment.insert(field.to_owned(), value.clone());
                state.environment_values_known = true;
            } else {
                state.environment.remove(field);
                state.environment_values_known = true;
            }
        }
        Category::Runtime => {
            let Some(field) = observation.field.as_deref() else {
                return;
            };
            if let Some(tool) = target.tools.get(field) {
                state.tools.insert(field.to_owned(), tool.clone());
            } else {
                state.tools.remove(field);
            }
            state.last_tool_scan_ms = target.last_tool_scan_ms;
        }
        Category::Git => {
            let EntityIdentity::Project { path } = &observation.entity.identity else {
                return;
            };
            if let Some(branch) = target.branches.get(path) {
                state.branches.insert(path.clone(), branch.clone());
            } else {
                state.branches.remove(path);
            }
        }
        _ => {}
    }
}

fn apply_path_observation(
    state: &mut DevEnvironmentState,
    target: &DevEnvironmentState,
    observation: &RawObservation,
) {
    let value = match observation.kind.event_type {
        EventType::Created => observation
            .after
            .as_ref()
            .and_then(serde_json::Value::as_str),
        EventType::Deleted => observation
            .before
            .as_ref()
            .and_then(serde_json::Value::as_str),
        _ => None,
    };
    let Some(value) = value else {
        return;
    };
    let key = path_key(value);
    match observation.kind.event_type {
        EventType::Created => {
            if !state
                .path_entries
                .iter()
                .any(|entry| path_key(entry) == key)
            {
                state.path_entries.push(value.to_owned());
            }
        }
        EventType::Deleted => {
            state.path_entries.retain(|entry| path_key(entry) != key);
        }
        _ => {}
    }
    // Keep the prefix deterministic even when the provider reported a PATH
    // addition in an order different from the effective Windows search order.
    let present = state
        .path_entries
        .iter()
        .map(|entry| path_key(entry))
        .collect::<std::collections::BTreeSet<_>>();
    state.path_entries = target
        .path_entries
        .iter()
        .filter(|entry| present.contains(&path_key(entry)))
        .cloned()
        .collect();
}

fn path_key(value: &str) -> String {
    value.replace('/', "\\").to_ascii_lowercase()
}

fn apply_target_metadata(state: &mut DevEnvironmentState, target: &DevEnvironmentState) {
    state.environment_values_known = target.environment_values_known;
    state.last_tool_scan_ms = target.last_tool_scan_ms;
    state.established = target.established;
}

/// Build a durable baseline, degrading to an explicit unestablished marker if
/// an unusually large developer environment cannot fit the storage bound.
///
/// Without this fallback a machine with thousands of PATH/tool/branch entries
/// would return the same baseline-size error forever, so no later snapshot
/// could be acknowledged.  The marker causes one quiet re-baseline after a
/// restart; it never stores raw environment values or a hash of them.
fn durable_baseline(
    state: &DevEnvironmentState,
    captured_at: i64,
) -> AppResult<crate::collectors::RawBaseline> {
    match stateful::baseline_for(BASELINE_KIND, &state.durable(), captured_at) {
        Ok(baseline) => Ok(baseline),
        Err(error) => {
            tracing::warn!(reason = %error, "developer environment baseline exceeded the durable bound; scheduling a quiet re-baseline");
            stateful::baseline_for(BASELINE_KIND, &DevEnvironmentState::default(), captured_at)
        }
    }
}

pub fn catalog_descriptor() -> AppResult<CollectorDescriptor> {
    crate::collectors::registry::CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == ID)
        .map(|entry| entry.descriptor.clone())
        .ok_or_else(|| AppError::InvalidInput {
            what: "collector catalog",
            reason: "the developer environment collector is not in the canonical catalog"
                .to_owned(),
        })
}
