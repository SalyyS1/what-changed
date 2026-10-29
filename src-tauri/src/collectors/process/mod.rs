//! The process collector: a one-second diff-poll over the machine's processes.
//!
//! Detection is a diff rather than a subscription because both event-tracing
//! routes were measured and both need elevation this app never asks for. The
//! consequence is stated where a user reads it — in this collector's descriptor
//! copy, which the canonical catalog owns — rather than hidden: a program that
//! lives less than a second can be missed.
//!
//! What the diff may and may not claim is the rest of this module. A pid is a
//! map key, never an identity. A path a non-elevated reader cannot see degrades
//! the event instead of failing it. And a program nobody watched start is never
//! reported as having started.

pub mod churn;
pub mod diff;
pub mod observations;
pub mod state;

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use crate::collectors::process_map::ProcessMap;
use crate::collectors::stateful;
use crate::collectors::{
    AckedState, AutomaticCheckpointTrigger, Collector, CollectorContext, CollectorDescriptor,
    CollectorSnapshot, CommitToken, ObservationClock, RawObservation, SnapshotContext,
    SystemObservationClock,
};
use crate::domain::Confidence;
use crate::error::{AppError, AppResult};
use crate::platform::traits::{clear_process_rows, ProcessProvider, ProcessRow};
use crate::storage::{CollectorCommitReceipt, MAX_ATOMIC_OBSERVATIONS};

use state::{ProcessState, TrackedInstance};

/// The catalog id this collector answers to.
pub const ID: &str = "process";

/// Confidence in a process instance requires both a stable executable identity
/// and the creation time that distinguishes this use of a pid from the next.
pub(super) fn identity_confidence(instance: &TrackedInstance) -> Confidence {
    if instance.identity_path_based && instance.path_available && instance.creation_available {
        Confidence::High
    } else {
        Confidence::Low
    }
}

/// How often the aggregate path-denial count is reported.
const DENIAL_LOG_INTERVAL_MS: i64 = 60 * 60 * 1_000;

/// The development machine reports about 370 processes. Owning this allocation
/// in the collector lets the production provider refill it every second.
const PROCESS_ROW_CAPACITY: usize = 512;

/// This collector's descriptor, read from the canonical catalog.
///
/// The catalog owns the copy, the cadence and the cost, so a collector that
/// restated them could drift from what Settings shows. Failing here is the right
/// outcome: a collector the catalog does not describe has no product identity
/// to run under, and silently inventing one is how two descriptions of the same
/// source start competing.
pub fn catalog_descriptor() -> AppResult<CollectorDescriptor> {
    crate::collectors::registry::CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == ID)
        .map(|entry| entry.descriptor.clone())
        .ok_or_else(|| AppError::InvalidInput {
            what: "collector catalog",
            reason: "the process collector is not in the canonical catalog".to_owned(),
        })
}

/// Diffs the machine's processes once per cadence tick.
pub struct ProcessCollector<P, C = SystemObservationClock> {
    provider: P,
    clock: C,
    map: ProcessMap,
    descriptor: CollectorDescriptor,
    state: AckedState<ProcessState>,
    context: Option<CollectorContext>,
    /// Whether the next diff is against a baseline captured before a gap.
    ///
    /// Held outside the staged state on purpose: it describes this instance's
    /// first cycle, not the machine. Clearing it only on a durable
    /// acknowledgement means a rejected gap-diff is retried as uncertain rather
    /// than quietly promoted to a fact nobody observed.
    downtime: bool,
    /// Fresh-provider candidate whose durable acknowledgement may clear a
    /// hydrated downtime gap. Pending backlog acknowledgements never do.
    downtime_clear_token: Option<CommitToken>,
    denials_since_log: u64,
    last_denial_log_ms: i64,
    /// Caller-owned provider output. Paths are scrubbed immediately after the
    /// tracked snapshot has taken the few values it needs.
    rows: Vec<ProcessRow>,
    /// Reused whole-table pid index for parent lookup.
    row_index: HashMap<u32, usize>,
}

impl<P: ProcessProvider> ProcessCollector<P, SystemObservationClock> {
    /// A collector against the production clock.
    pub fn new(provider: P, map: ProcessMap, descriptor: CollectorDescriptor) -> Self {
        Self::with_clock(provider, map, descriptor, SystemObservationClock)
    }
}

impl<P: ProcessProvider, C: ObservationClock> ProcessCollector<P, C> {
    /// A collector against an injected clock, so a sixty-second churn window is
    /// testable without spending a minute of wall clock on it.
    pub fn with_clock(
        provider: P,
        map: ProcessMap,
        descriptor: CollectorDescriptor,
        clock: C,
    ) -> Self {
        Self {
            provider,
            clock,
            map,
            descriptor,
            state: AckedState::new(ProcessState::default()),
            context: None,
            downtime: false,
            downtime_clear_token: None,
            denials_since_log: 0,
            last_denial_log_ms: 0,
            rows: Vec::with_capacity(PROCESS_ROW_CAPACITY),
            row_index: HashMap::with_capacity(PROCESS_ROW_CAPACITY),
        }
    }

    /// One aggregate line an hour about how many processes denied the path read.
    ///
    /// A line per denial would be dozens a second saying nothing new: a
    /// non-elevated reader cannot see into about half the processes on a normal
    /// machine, and that is the design rather than an incident.
    fn report_path_denials(&mut self, denials: u64, snapshot_processes: usize, now_ms: i64) {
        self.denials_since_log = self.denials_since_log.saturating_add(denials);
        if now_ms.saturating_sub(self.last_denial_log_ms) < DENIAL_LOG_INTERVAL_MS {
            return;
        }
        let denied = std::mem::take(&mut self.denials_since_log);
        self.last_denial_log_ms = now_ms;
        tracing::debug!(
            denied,
            snapshot_processes,
            "executable-path reads denied to a non-elevated caller"
        );
    }

    /// Stage one candidate state and describe the commit that carries it.
    fn stage(
        &mut self,
        token: CommitToken,
        next: ProcessState,
        now_ms: i64,
        observations: Vec<RawObservation>,
        clears_downtime: bool,
    ) -> AppResult<CollectorSnapshot> {
        let baseline = stateful::baseline_for(state::BASELINE_KIND, &next, now_ms)?;
        self.state.stage(token, next)?;
        self.downtime_clear_token = clears_downtime.then_some(token);
        Ok(CollectorSnapshot::Atomic {
            token,
            observations,
            relationships: Vec::new(),
            baseline: Some(baseline),
            config_snapshots: Vec::new(),
        })
    }
}

impl<P: ProcessProvider, C: ObservationClock> Collector for ProcessCollector<P, C> {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let now_ms = self.clock.now_ms();
        let baseline = context.state.baseline(state::BASELINE_KIND)?;
        let (state, downtime) = ProcessState::hydrate(baseline, now_ms);
        self.state = AckedState::new(state);
        self.downtime = downtime;
        self.downtime_clear_token = None;
        self.denials_since_log = 0;
        self.last_denial_log_ms = now_ms;
        self.context = Some(context);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(context) = &self.context {
            self.map.invalidate(&context.cancel);
        }
        self.context = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Err(AppError::InvalidInput {
            what: "process collector",
            reason: "a stateful collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        let Some(cancel) = self.context.as_ref().map(|held| held.cancel.clone()) else {
            return Ok(CollectorSnapshot::Noop);
        };
        let now_ms = self.clock.now_ms();
        self.provider.enumerate_into(&mut self.rows)?;
        if cancel.is_cancelled() {
            clear_process_rows(&mut self.rows);
            return Ok(CollectorSnapshot::Noop);
        }

        let snapshot_processes = self.rows.len();
        let quiet_rows = quiet_rows(&self.rows, self.state.committed(), now_ms, self.downtime);
        if quiet_rows {
            let denials = self
                .rows
                .iter()
                .filter(|row| row.executable_path.is_none())
                .count() as u64;
            self.report_path_denials(denials, snapshot_processes, now_ms);
            clear_process_rows(&mut self.rows);
            self.map.refresh(&cancel, Instant::now());
            self.downtime = false;
            return Ok(CollectorSnapshot::Noop);
        }
        let (mut current, denials) =
            state::instances_with_index(&self.rows, self.state.committed(), &mut self.row_index);
        clear_process_rows(&mut self.rows);
        // Readers attribute sockets to this, so it receives the complete
        // successful snapshot and never a persistence chunk: a listener that
        // named an owner the publisher had not confirmed would be writing a
        // relationship nobody proved.
        self.map.publish_instances(
            current.iter().map(|(pid, instance)| {
                (
                    *pid,
                    &instance.entity_id,
                    instance.image.as_str(),
                    identity_confidence(instance),
                )
            }),
            &cancel,
            Instant::now(),
        );
        self.report_path_denials(denials, snapshot_processes, now_ms);

        if !self.state.committed().established {
            // A new session takes its first snapshot without inventing a start
            // for every program that was already running.
            for instance in current.values_mut() {
                // No start event will consume these paths, and the id already
                // carries the stable identity. Do not retain raw paths in the
                // acknowledged in-memory baseline.
                instance.executable = None;
            }
            let next = ProcessState {
                acknowledged: current,
                established: true,
                ..ProcessState::default()
            };
            return self.stage(context.token(), next, now_ms, Vec::new(), true);
        }

        // The provider still performs the complete observation above, but a
        // stable machine should not pay for cloning the acknowledged map,
        // rebuilding the diff and re-evaluating churn every tick. Preserve the
        // evidence-refresh rules by requiring the availability bits and
        // creation identity to match before taking this path.
        if self.state.committed().pending.is_none()
            && quiet_snapshot(
                &self.state.committed().acknowledged,
                &current,
                &self.state.committed().episodes,
                now_ms,
                self.downtime,
            )
        {
            self.downtime = false;
            return Ok(CollectorSnapshot::Noop);
        }

        let pending = self.state.committed().pending.clone();
        let mut acknowledged = self.state.committed().acknowledged.clone();
        if pending.is_none() {
            // Persist current evidence for continuing instances even though it
            // is not an event. An upgrade may provide the first authoritative
            // creation time needed to catch the next pid reuse; a downgrade
            // must keep a later stop from borrowing stale High confidence.
            // Do this only when there is no older pending target: current
            // provider evidence must never rewrite history still being drained.
            for (pid, instance) in &current {
                let Some(existing) = acknowledged.get_mut(pid) else {
                    continue;
                };
                if diff::is_continuation(existing, instance) {
                    existing.creation = instance.creation;
                    existing.creation_available = instance.creation_available;
                    existing.path_available = instance.path_available;
                }
            }
        }
        let (mut target, observed_at, uncertain) = pending
            .as_ref()
            .map(|pending| {
                (
                    pending.target.clone(),
                    pending.observed_at,
                    pending.occurred_at_is_uncertain,
                )
            })
            .unwrap_or_else(|| (current, now_ms, self.downtime));
        let mut episodes = self.state.committed().episodes.clone();
        // Finished windows close first: a summary describes a window that has
        // already ended, and queueing it behind this cycle's changes would
        // delay a fact that is already complete.
        let closure_frontier = if pending.is_some() {
            observed_at
        } else {
            now_ms
        };
        let summaries = churn::close_due(&mut episodes, closure_frontier, MAX_ATOMIC_OBSERVATIONS);
        let changes = diff::diff(&self.state.committed().acknowledged, &target);
        let budget = MAX_ATOMIC_OBSERVATIONS.saturating_sub(summaries.len());
        let drained = diff::fitting(&changes, budget);

        if changes[..drained].iter().any(installer_started) {
            if let Some(context) = &self.context {
                let _ = context
                    .automatic_checkpoint
                    .try_request(AutomaticCheckpointTrigger::InstallerProcessStarted);
            }
        }

        let mut observations = Vec::with_capacity(summaries.len() + drained + 1);
        observations.extend(summaries.iter().map(observations::for_churn));
        for change in &changes[..drained] {
            observations.extend(observations::for_change(change, observed_at, uncertain));
            if !uncertain {
                if let Some(instance) = completed_pair(change) {
                    churn::record_pair(
                        &mut episodes,
                        &instance.entity_id,
                        &instance.image,
                        observed_at,
                        identity_confidence(instance),
                    );
                }
            }
        }
        let fully_drained = drained == changes.len();
        for change in &changes[..drained] {
            diff::apply(&mut acknowledged, change);
        }
        let next_pending = if fully_drained {
            None
        } else {
            // The structural id has already captured the path-derived
            // identity. Raw paths must not live for several polling cycles or
            // survive only accidentally until serialization; deferred events
            // disclose a null path after this privacy boundary.
            for instance in target.values_mut() {
                instance.executable = None;
            }
            Some(stateful::PendingTarget {
                target,
                observed_at,
                occurred_at_is_uncertain: uncertain,
            })
        };
        let next = ProcessState {
            acknowledged,
            pending: next_pending,
            episodes,
            established: true,
        };

        if observations.is_empty() && next == *self.state.committed() {
            // A quiet cycle has nothing to say and nothing to persist. Writing
            // the baseline once a second anyway would be disk churn for a fact
            // that has not changed.
            self.downtime = false;
            return Ok(CollectorSnapshot::Noop);
        }
        self.stage(
            context.token(),
            next,
            now_ms,
            observations,
            pending.is_none(),
        )
    }

    fn commit_succeeded(
        &mut self,
        token: CommitToken,
        _receipt: &CollectorCommitReceipt,
    ) -> AppResult<()> {
        self.state.ack(token)?;
        if self.downtime_clear_token == Some(token) {
            self.downtime = false;
        }
        self.downtime_clear_token = None;
        Ok(())
    }

    fn commit_failed(&mut self, token: CommitToken) -> AppResult<()> {
        let result = self.state.reject(token);
        if self.downtime_clear_token == Some(token) {
            self.downtime_clear_token = None;
        }
        result
    }
}

/// Whether a fully observed cycle can skip the state/diff machinery without
/// changing any durable evidence. This is deliberately stricter than the
/// process continuity rule: a path or creation read becoming unavailable must
/// still stage a confidence downgrade for the next acknowledgement.
fn quiet_snapshot(
    acknowledged: &BTreeMap<u32, TrackedInstance>,
    current: &BTreeMap<u32, TrackedInstance>,
    episodes: &churn::Episodes,
    now_ms: i64,
    downtime: bool,
) -> bool {
    if downtime
        || acknowledged.len() != current.len()
        || episodes
            .values()
            .any(|episode| now_ms.saturating_sub(episode.anchor_ms) >= churn::WINDOW_MS)
    {
        return false;
    }
    acknowledged.iter().all(|(pid, previous)| {
        let Some(now) = current.get(pid) else {
            return false;
        };
        diff::is_continuation(previous, now)
            && previous.creation == now.creation
            && previous.creation_available == now.creation_available
            && previous.path_available == now.path_available
    })
}

/// Prove a quiet cycle directly from provider rows before allocating the
/// ordered tracked-instance view. Every identity and confidence bit that can
/// affect evidence is checked; anything uncertain falls through to the full
/// reconciliation path.
fn quiet_rows(rows: &[ProcessRow], state: &ProcessState, now_ms: i64, downtime: bool) -> bool {
    if downtime
        || !state.established
        || state.pending.is_some()
        || state
            .episodes
            .values()
            .any(|episode| now_ms.saturating_sub(episode.anchor_ms) >= churn::WINDOW_MS)
        || rows.len() != state.acknowledged.len()
    {
        return false;
    }
    rows.iter().all(|row| {
        let Some(previous) = state.acknowledged.get(&row.pid) else {
            return false;
        };
        state::same_instance(previous, row)
            && previous.creation_available == row.creation_time.is_some()
            && previous.path_available == row.executable_path.is_some()
    })
}

fn installer_started(change: &diff::Change) -> bool {
    let instance = match change {
        diff::Change::Started(instance) => instance,
        diff::Change::Reused { started, .. } => started,
        diff::Change::Stopped(_) => return false,
    };
    let image = instance.image.to_ascii_lowercase();
    image == "msiexec.exe"
        || (image.ends_with(".exe") && (image.contains("setup") || image.contains("installer")))
}

/// The instance whose exit completed a matched pair, if this change is one.
///
/// A stop whose start this collector also observed is a pair. A program that
/// was already running when recording began is not, and must not open a churn
/// episode on the strength of an exit nobody watched begin.
fn completed_pair(change: &diff::Change) -> Option<&TrackedInstance> {
    let stopped = match change {
        diff::Change::Stopped(instance) => instance,
        diff::Change::Reused { stopped, .. } => stopped,
        diff::Change::Started(_) => return None,
    };
    stopped.observed_start.then_some(stopped)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::collectors::testing::{live_context, LiveContext};
    use crate::domain::EventType;
    use crate::platform::traits::ProcessRow;

    /// A provider that hands back one scripted table per call, repeating the
    /// last one once the script runs out, and can be told to fail first.
    #[derive(Clone, Default)]
    struct ScriptedProvider {
        tables: Arc<Mutex<VecDeque<Vec<ProcessRow>>>>,
        fail_next: Arc<AtomicBool>,
    }

    impl ScriptedProvider {
        fn new(tables: Vec<Vec<ProcessRow>>) -> Self {
            Self {
                tables: Arc::new(Mutex::new(tables.into())),
                fail_next: Arc::new(AtomicBool::new(false)),
            }
        }

        fn failing_first(tables: Vec<Vec<ProcessRow>>) -> Self {
            let provider = Self::new(tables);
            provider.fail_next.store(true, Ordering::Release);
            provider
        }

        fn next_table(&self) -> AppResult<Vec<ProcessRow>> {
            if self.fail_next.swap(false, Ordering::AcqRel) {
                return Err(AppError::Platform {
                    operation: "take a process snapshot",
                    reason: "0x80070005".to_owned(),
                });
            }
            let mut tables = self.tables.lock().unwrap();
            // The last scripted table is held rather than consumed, so a cycle
            // that was not given a new one reads the machine as unchanged
            // instead of as empty.
            if tables.len() > 1 {
                return Ok(tables.pop_front().unwrap_or_default());
            }
            Ok(tables.front().cloned().unwrap_or_default())
        }
    }

    impl ProcessProvider for ScriptedProvider {
        fn enumerate(&self) -> AppResult<Vec<ProcessRow>> {
            self.next_table()
        }

        fn enumerate_into(&self, output: &mut Vec<ProcessRow>) -> AppResult<()> {
            clear_process_rows(output);
            output.extend(self.next_table()?);
            Ok(())
        }
    }

    struct ManualClock(Arc<AtomicI64>);

    impl ManualClock {
        fn at(now_ms: i64) -> Self {
            Self(Arc::new(AtomicI64::new(now_ms)))
        }

        fn advance(&self, by_ms: i64) {
            self.0.fetch_add(by_ms, Ordering::AcqRel);
        }
    }

    impl ObservationClock for ManualClock {
        fn now_ms(&self) -> i64 {
            self.0.load(Ordering::Acquire)
        }
    }

    type UnderTest = ProcessCollector<ScriptedProvider, ManualClock>;

    fn process(pid: u32, image: &str, creation: u64) -> ProcessRow {
        ProcessRow {
            pid,
            parent_pid: 0,
            image_name: image.to_owned(),
            executable_path: Some(format!(r"C:\dev\{image}")),
            creation_time: Some(creation),
        }
    }

    fn denied_process(pid: u32, image: &str, creation: u64) -> ProcessRow {
        ProcessRow {
            executable_path: None,
            ..process(pid, image, creation)
        }
    }

    /// A started collector over a real database, plus the clock that drives it.
    fn started(tables: Vec<Vec<ProcessRow>>, now_ms: i64) -> (UnderTest, ManualClock, LiveContext) {
        let clock = ManualClock::at(now_ms);
        let mut collector = ProcessCollector::with_clock(
            ScriptedProvider::new(tables),
            ProcessMap::new(),
            catalog_descriptor().unwrap(),
            ManualClock(Arc::clone(&clock.0)),
        );
        let live = live_context(ID).unwrap();
        collector.start(live.context.clone()).unwrap();
        (collector, clock, live)
    }

    /// Run one cycle, acknowledge it, and return what it wanted to write.
    fn cycle(collector: &mut UnderTest, token: u64) -> CollectorSnapshot {
        let token = CommitToken::mint(token);
        let snapshot = collector
            .snapshot_batch(SnapshotContext::new(token))
            .unwrap();
        if let CollectorSnapshot::Atomic { .. } = &snapshot {
            collector
                .commit_succeeded(
                    token,
                    &CollectorCommitReceipt {
                        event_ids: Vec::new(),
                    },
                )
                .unwrap();
        }
        snapshot
    }

    /// Replace the table the next cycle will read.
    fn script(collector: &mut UnderTest, table: Vec<ProcessRow>) {
        collector.provider = ScriptedProvider::new(vec![table]);
    }

    #[test]
    fn quiet_rows_requires_every_identity_and_availability_bit_to_match() {
        let rows = vec![process(7, "api.exe", 10)];
        let acknowledged = state::instances(&rows, &ProcessState::default()).0;
        let state = ProcessState {
            acknowledged,
            established: true,
            ..ProcessState::default()
        };
        assert!(quiet_rows(&rows, &state, 1_000, false));
        assert!(!quiet_rows(&rows, &state, 1_000, true));
        assert!(!quiet_rows(&[], &state, 1_000, false));

        let mut path_denied = rows.clone();
        path_denied[0].executable_path = None;
        assert!(!quiet_rows(&path_denied, &state, 1_000, false));

        let mut pid_reused = rows.clone();
        pid_reused[0].creation_time = Some(11);
        assert!(!quiet_rows(&pid_reused, &state, 1_000, false));

        let pending = ProcessState {
            pending: Some(stateful::PendingTarget {
                target: state.acknowledged.clone(),
                observed_at: 1_000,
                occurred_at_is_uncertain: false,
            }),
            ..state.clone()
        };
        assert!(!quiet_rows(&rows, &pending, 1_000, false));
    }

    #[test]
    fn quiet_cycle_returns_noop_without_fabricating_observations() {
        let (mut collector, _clock, _live) = started(vec![vec![process(7, "api.exe", 10)]], 0);
        let first = cycle(&mut collector, 1);
        assert!(matches!(first, CollectorSnapshot::Atomic { .. }));
        let second = cycle(&mut collector, 2);
        assert!(matches!(second, CollectorSnapshot::Noop));
    }

    fn events(snapshot: &CollectorSnapshot) -> &[RawObservation] {
        match snapshot {
            CollectorSnapshot::Atomic { observations, .. } => observations,
            CollectorSnapshot::Noop => &[],
            other => panic!("expected an atomic snapshot, got {other:?}"),
        }
    }

    fn count(snapshot: &CollectorSnapshot, event_type: EventType) -> usize {
        events(snapshot)
            .iter()
            .filter(|event| event.kind.event_type == event_type)
            .count()
    }

    fn state_of(snapshot: &CollectorSnapshot) -> ProcessState {
        match snapshot {
            CollectorSnapshot::Atomic {
                baseline: Some(stored),
                ..
            } => serde_json::from_value(stored.snapshot.clone()).unwrap(),
            other => panic!("expected an atomic snapshot with a baseline, got {other:?}"),
        }
    }

    #[test]
    fn the_catalog_owns_the_descriptor_and_states_the_blind_spot() {
        let descriptor = catalog_descriptor().unwrap();
        assert_eq!(descriptor.id, ID);
        assert!(!descriptor.requires_admin);
        assert!(
            descriptor.what_it_watches.contains("less than a second"),
            "the blind spot is stated where a user reads it: {}",
            descriptor.what_it_watches
        );
    }

    #[test]
    fn a_first_snapshot_records_a_baseline_and_invents_no_starts() {
        let (mut collector, _, _live) = started(vec![vec![process(1, "api.exe", 10)]], 1_000);
        let snapshot = cycle(&mut collector, 1);
        assert!(events(&snapshot).is_empty());
        let state = state_of(&snapshot);
        assert!(state.established);
        assert_eq!(state.acknowledged.len(), 1);
        assert!(
            !state.acknowledged[&1].observed_start,
            "a program that was already running was not seen to start"
        );
        assert_eq!(
            state.acknowledged[&1].executable, None,
            "a silent baseline has no event that needs to retain the path"
        );
        assert_eq!(
            collector.state.committed().acknowledged[&1].executable,
            None,
            "the live acknowledged state also drops the path"
        );
    }

    #[test]
    fn collector_reuses_and_scrubs_its_process_row_buffer() {
        let table = vec![process(1, "api.exe", 10)];
        let (mut collector, _, _live) = started(vec![table], 1_000);
        let allocation = (collector.rows.as_ptr(), collector.rows.capacity());
        cycle(&mut collector, 1);
        assert!(
            collector.rows.is_empty(),
            "paths do not outlive normalization"
        );
        assert_eq!(
            (collector.rows.as_ptr(), collector.rows.capacity()),
            allocation
        );
        cycle(&mut collector, 2);
        assert!(collector.rows.is_empty());
        assert_eq!(
            (collector.rows.as_ptr(), collector.rows.capacity()),
            allocation
        );
        assert!(collector.row_index.capacity() >= PROCESS_ROW_CAPACITY);
    }

    #[test]
    fn a_quiet_cycle_is_a_noop_and_does_not_rewrite_the_baseline() {
        let (mut collector, _, _live) = started(vec![vec![process(1, "api.exe", 10)]], 1_000);
        assert!(matches!(
            cycle(&mut collector, 1),
            CollectorSnapshot::Atomic { .. }
        ));
        assert_eq!(cycle(&mut collector, 2), CollectorSnapshot::Noop);
        assert_eq!(cycle(&mut collector, 3), CollectorSnapshot::Noop);
    }

    #[test]
    fn a_failed_cycle_leaves_nothing_staged_so_the_next_one_can_proceed() {
        let (mut collector, _, _live) = started(vec![Vec::new()], 1_000);
        assert!(matches!(
            cycle(&mut collector, 1),
            CollectorSnapshot::Atomic { .. }
        ));

        collector.provider = ScriptedProvider::failing_first(vec![vec![process(2, "new.exe", 5)]]);
        let error = collector
            .snapshot_batch(SnapshotContext::new(CommitToken::mint(2)))
            .unwrap_err();
        assert!(matches!(error, AppError::Platform { .. }));

        // The provider's failure is spent, so this cycle reads the table the
        // failed one was going to. It can only stage if the failure left no
        // candidate behind.
        let recovered = cycle(&mut collector, 3);
        assert_eq!(count(&recovered, EventType::Started), 1);
    }

    #[test]
    fn an_oversized_baseline_fails_before_acked_state_advances() {
        let huge = "x".repeat(crate::storage::baseline_repository::MAX_SNAPSHOT_BYTES + 1);
        let row = ProcessRow {
            pid: 7,
            parent_pid: 0,
            image_name: huge,
            executable_path: None,
            creation_time: Some(1),
        };
        let (mut collector, _, _live) = started(vec![vec![row]], 1_000);
        let error = collector
            .snapshot_batch(SnapshotContext::new(CommitToken::mint(1)))
            .unwrap_err();
        assert!(matches!(
            error,
            AppError::InvalidInput {
                what: "collector baseline",
                ..
            }
        ));
        assert_eq!(collector.state.committed(), &ProcessState::default());
    }

    #[test]
    fn a_start_and_a_stop_land_in_the_cycle_that_sees_them() {
        let (mut collector, _, _live) = started(vec![vec![process(1, "api.exe", 10)]], 1_000);
        cycle(&mut collector, 1);

        script(
            &mut collector,
            vec![process(1, "api.exe", 10), process(2, "new.exe", 20)],
        );
        let started = cycle(&mut collector, 2);
        assert_eq!(count(&started, EventType::Started), 1);
        assert_eq!(events(&started)[0].entity.display_name, "new.exe");
        assert!(!events(&started)[0].occurred_at_is_uncertain);
        assert!(state_of(&started).acknowledged[&2].observed_start);

        script(&mut collector, vec![process(1, "api.exe", 10)]);
        let stopped = cycle(&mut collector, 3);
        assert_eq!(count(&stopped, EventType::Stopped), 1);
        assert_eq!(events(&stopped)[0].entity.display_name, "new.exe");
        assert!(!state_of(&stopped).acknowledged.contains_key(&2));
    }

    #[test]
    fn a_burst_larger_than_one_commit_arrives_whole_over_several_cycles() {
        const BURST: u32 = 500;
        let (mut collector, _, _live) = started(vec![Vec::new()], 1_000);
        cycle(&mut collector, 1);
        script(
            &mut collector,
            (0..BURST).map(|pid| process(pid, "x.exe", 5)).collect(),
        );

        let mut delivered = 0usize;
        let mut widest = 0usize;
        for token in 2..10u64 {
            let snapshot = cycle(&mut collector, token);
            let batch = events(&snapshot).len();
            widest = widest.max(batch);
            delivered += batch;
            if delivered >= usize::try_from(BURST).unwrap() {
                break;
            }
        }
        assert_eq!(
            delivered,
            usize::try_from(BURST).unwrap(),
            "nothing truncated"
        );
        assert!(
            widest <= MAX_ATOMIC_OBSERVATIONS,
            "no batch overshot the storage bound: {widest}"
        );
        assert!(widest > 1, "the burst was actually drained in batches");
        assert_eq!(cycle(&mut collector, 20), CollectorSnapshot::Noop);
    }

    #[test]
    fn a_deferred_short_lived_process_survives_recreation_without_timestamp_drift() {
        const BURST: u32 = 450;
        let (mut collector, clock, live) = started(vec![Vec::new()], 1_000);
        cycle(&mut collector, 1);
        clock.advance(1_000);
        script(
            &mut collector,
            (0..BURST)
                .map(|pid| process(pid, "short.exe", u64::from(pid) + 1))
                .collect(),
        );

        let first = cycle(&mut collector, 2);
        assert_eq!(events(&first).len(), MAX_ATOMIC_OBSERVATIONS);
        assert!(events(&first)
            .iter()
            .all(|event| event.occurred_at == 2_000));
        assert!(
            collector
                .state
                .committed()
                .pending
                .as_ref()
                .is_some_and(|pending| pending
                    .target
                    .values()
                    .all(|instance| instance.executable.is_none())),
            "raw paths do not remain in the in-memory deferred target"
        );
        let durable = state_of(&first);
        assert!(durable.pending.is_some(), "the complete target is durable");
        let stored = crate::storage::Baseline {
            collector_id: ID.to_owned(),
            entity_kind: state::BASELINE_KIND.to_owned(),
            snapshot: serde_json::to_value(&durable).unwrap(),
            captured_at: 2_000,
            truncated: false,
        };
        live.db
            .write(move |connection| {
                crate::storage::baseline_repository::store(
                    connection,
                    &crate::privacy::Sanitizer::new().sanitize(stored).0,
                )
            })
            .unwrap();

        collector.stop();
        let mut resumed = ProcessCollector::with_clock(
            ScriptedProvider::new(vec![Vec::new()]),
            ProcessMap::new(),
            catalog_descriptor().unwrap(),
            ManualClock::at(2_000 + stateful::DOWNTIME_GAP_MS + 1),
        );
        resumed.start(live.context.clone()).unwrap();

        let second = cycle(&mut resumed, 3);
        let third = cycle(&mut resumed, 4);
        assert_eq!(events(&second).len(), MAX_ATOMIC_OBSERVATIONS);
        assert_eq!(events(&third).len(), 50);
        assert!(events(&second)
            .iter()
            .chain(events(&third))
            .all(|event| event.kind.event_type == EventType::Started
                && event.occurred_at == 2_000
                && !event.occurred_at_is_uncertain));
        assert!(
            events(&second).iter().chain(events(&third)).all(|event| {
                event.details.as_ref().is_some_and(|details| {
                    details["path"].is_null()
                        && details["path_unavailable"] == true
                        && details["path_omitted_after_defer"] == true
                })
            }),
            "deferred raw paths are omitted consistently across recreation"
        );
        assert!(state_of(&third).pending.is_none());
        assert_eq!(
            events(&third).last().unwrap().entity.display_name,
            "short.exe",
            "a process absent from every post-recreation provider view still lands"
        );

        let later = cycle(&mut resumed, 5);
        assert!(
            events(&later)
                .iter()
                .all(|event| event.kind.event_type == EventType::Stopped
                    && event.occurred_at_is_uncertain),
            "draining the historical pending target must not clear fresh-gap uncertainty"
        );
    }

    #[test]
    fn a_recreated_backlog_closes_one_complete_churn_window_after_every_pair_drains() {
        const PAIRS: u32 = 250;
        let (mut collector, clock, live) = started(vec![Vec::new()], 0);
        cycle(&mut collector, 1);

        let first_generation: Vec<_> = (0..PAIRS)
            .map(|pid| process(pid + 1, "flaky.exe", u64::from(pid) + 1))
            .collect();
        script(&mut collector, first_generation);
        assert_eq!(events(&cycle(&mut collector, 2)).len(), 200);
        assert_eq!(events(&cycle(&mut collector, 3)).len(), 50);

        clock.advance(1_000);
        let second_generation: Vec<_> = (0..PAIRS)
            .map(|pid| process(pid + 1, "flaky.exe", u64::from(pid) + 1_001))
            .collect();
        script(&mut collector, second_generation.clone());
        let first_reuse_chunk = cycle(&mut collector, 4);
        assert_eq!(events(&first_reuse_chunk).len(), 200);
        assert_eq!(count(&first_reuse_chunk, EventType::Churn), 0);
        let durable = state_of(&first_reuse_chunk);
        assert!(durable.pending.is_some());
        let stored = crate::storage::Baseline {
            collector_id: ID.to_owned(),
            entity_kind: state::BASELINE_KIND.to_owned(),
            snapshot: serde_json::to_value(&durable).unwrap(),
            captured_at: 1_000,
            truncated: false,
        };
        live.db
            .write(move |connection| {
                crate::storage::baseline_repository::store(
                    connection,
                    &crate::privacy::Sanitizer::new().sanitize(stored).0,
                )
            })
            .unwrap();

        collector.stop();
        let mut resumed = ProcessCollector::with_clock(
            ScriptedProvider::new(vec![second_generation]),
            ProcessMap::new(),
            catalog_descriptor().unwrap(),
            ManualClock::at(1_000 + churn::WINDOW_MS + 1),
        );
        resumed.start(live.context.clone()).unwrap();

        let second_reuse_chunk = cycle(&mut resumed, 5);
        let final_reuse_chunk = cycle(&mut resumed, 6);
        assert_eq!(events(&second_reuse_chunk).len(), 200);
        assert_eq!(events(&final_reuse_chunk).len(), 100);
        assert_eq!(count(&second_reuse_chunk, EventType::Churn), 0);
        assert_eq!(count(&final_reuse_chunk, EventType::Churn), 0);
        assert!(state_of(&final_reuse_chunk).pending.is_none());

        let closed = cycle(&mut resumed, 7);
        assert_eq!(count(&closed, EventType::Churn), 1);
        let summary = events(&closed)
            .iter()
            .find(|event| event.kind.event_type == EventType::Churn)
            .unwrap();
        assert_eq!(summary.details.as_ref().unwrap()["count"], PAIRS);
        assert_eq!(cycle(&mut resumed, 8), CollectorSnapshot::Noop);
    }

    #[test]
    fn stopping_the_process_collector_invalidates_its_published_owner_map() {
        let table = vec![process(7, "api.exe", 10)];
        let (mut collector, _, _live) = started(vec![table], 1_000);
        cycle(&mut collector, 1);
        assert!(collector.map.resolve(7).is_some());
        collector.stop();
        assert!(collector.map.resolve(7).is_none());
    }

    #[test]
    fn a_published_owner_without_creation_time_is_low_confidence() {
        let row = ProcessRow {
            pid: 7,
            parent_pid: 0,
            image_name: "api.exe".to_owned(),
            executable_path: Some(r"C:\dev\api.exe".to_owned()),
            creation_time: None,
        };
        let (mut collector, _, _live) = started(vec![vec![row]], 1_000);
        cycle(&mut collector, 1);
        assert_eq!(
            collector.map.resolve(7).map(|owner| owner.confidence),
            Some(Confidence::Low)
        );
    }

    #[test]
    fn a_temporary_access_denial_lowers_the_live_owner_confidence() {
        let (mut collector, _, _live) = started(vec![vec![process(7, "api.exe", 10)]], 1_000);
        cycle(&mut collector, 1);
        assert_eq!(
            collector.map.resolve(7).map(|owner| owner.confidence),
            Some(Confidence::High)
        );

        script(
            &mut collector,
            vec![ProcessRow {
                pid: 7,
                parent_pid: 0,
                image_name: "api.exe".to_owned(),
                executable_path: None,
                creation_time: None,
            }],
        );
        let denied = cycle(&mut collector, 2);
        assert!(events(&denied).is_empty());
        let denied_state = state_of(&denied);
        assert_eq!(denied_state.acknowledged[&7].creation, Some(10));
        assert!(!denied_state.acknowledged[&7].creation_available);
        assert!(!denied_state.acknowledged[&7].path_available);
        assert_eq!(
            collector.map.resolve(7).map(|owner| owner.confidence),
            Some(Confidence::Low),
            "the map must use current evidence rather than the baseline's old access"
        );
    }

    #[test]
    fn a_stop_after_access_was_denied_cannot_borrow_old_high_confidence() {
        let (mut collector, _, _live) = started(vec![vec![process(7, "api.exe", 10)]], 1_000);
        cycle(&mut collector, 1);
        script(
            &mut collector,
            vec![ProcessRow {
                pid: 7,
                parent_pid: 0,
                image_name: "api.exe".to_owned(),
                executable_path: None,
                creation_time: None,
            }],
        );
        cycle(&mut collector, 2);

        script(&mut collector, Vec::new());
        let stopped = cycle(&mut collector, 3);
        assert_eq!(count(&stopped, EventType::Stopped), 1);
        assert_eq!(events(&stopped)[0].kind.confidence, Confidence::Low);
        assert_eq!(
            events(&stopped)[0].details.as_ref().unwrap()["path_unavailable"],
            true
        );
    }

    #[test]
    fn creation_learned_after_a_denial_is_durable_and_exposes_the_next_pid_reuse() {
        let weak = ProcessRow {
            pid: 7,
            parent_pid: 0,
            image_name: "api.exe".to_owned(),
            executable_path: None,
            creation_time: None,
        };
        let (mut collector, _, _live) = started(vec![vec![weak]], 1_000);
        cycle(&mut collector, 1);

        script(&mut collector, vec![process(7, "api.exe", 10)]);
        let learned = cycle(&mut collector, 2);
        assert!(
            events(&learned).is_empty(),
            "learning evidence is not an event"
        );
        assert_eq!(state_of(&learned).acknowledged[&7].creation, Some(10));
        assert!(state_of(&learned).acknowledged[&7].creation_available);

        script(&mut collector, vec![process(7, "api.exe", 11)]);
        let reused = cycle(&mut collector, 3);
        assert_eq!(events(&reused).len(), 2);
        assert_eq!(events(&reused)[0].kind.event_type, EventType::Stopped);
        assert_eq!(events(&reused)[1].kind.event_type, EventType::Started);
        assert_eq!(state_of(&reused).acknowledged[&7].creation, Some(11));
    }

    #[test]
    fn a_weak_image_identity_never_becomes_a_confirmed_owner_mid_instance() {
        let weak = ProcessRow {
            pid: 7,
            parent_pid: 0,
            image_name: "api.exe".to_owned(),
            executable_path: None,
            creation_time: Some(10),
        };
        let (mut collector, _, _live) = started(vec![vec![weak]], 1_000);
        cycle(&mut collector, 1);
        let weak_id = collector.map.resolve(7).unwrap().entity_id;

        script(&mut collector, vec![process(7, "api.exe", 10)]);
        cycle(&mut collector, 2);
        let promoted = collector.map.resolve(7).unwrap();
        assert_eq!(
            promoted.entity_id, weak_id,
            "identity stays stable mid-instance"
        );
        assert_eq!(
            promoted.confidence,
            Confidence::Low,
            "a path observed later cannot strengthen an image-derived identity"
        );
    }

    #[test]
    fn eight_pairs_produce_sixteen_prompt_raw_events_and_one_summary_at_window_close() {
        let (mut collector, clock, _live) = started(vec![Vec::new()], 0);
        cycle(&mut collector, 1);

        let mut token = 2u64;
        let mut starts = 0usize;
        let mut stops = 0usize;
        for creation in 1..=8u64 {
            script(&mut collector, vec![process(9, "flaky.exe", creation)]);
            let started = cycle(&mut collector, token);
            starts += count(&started, EventType::Started);
            token += 1;
            script(&mut collector, Vec::new());
            let stopped = cycle(&mut collector, token);
            stops += count(&stopped, EventType::Stopped);
            token += 1;
        }
        assert_eq!(starts, 8, "every raw start is written when it is seen");
        assert_eq!(
            stops, 8,
            "and every raw stop, so the evidence stays complete"
        );
        let early = cycle(&mut collector, token);
        assert_eq!(
            count(&early, EventType::Churn),
            0,
            "nothing is summarized before the window closes"
        );

        clock.advance(churn::WINDOW_MS);
        let summary = cycle(&mut collector, token + 1);
        let churn = events(&summary)
            .iter()
            .filter(|event| event.kind.event_type == EventType::Churn)
            .collect::<Vec<_>>();
        assert_eq!(churn.len(), 1, "one summary, not one per pair");
        let details = churn[0].details.as_ref().unwrap();
        assert_eq!(details["pairs"], 8);
        assert_eq!(details["count"], 8);
        assert_eq!(details["summary_of_window"], true);
        assert_eq!(
            churn[0].occurred_at,
            churn::WINDOW_MS,
            "the timestamp is the window's close, not a process start"
        );
        let after = cycle(&mut collector, token + 2);
        assert_eq!(
            count(&after, EventType::Churn),
            0,
            "a closed episode does not publish twice"
        );
    }

    #[test]
    fn denied_image_identities_cannot_produce_a_high_confidence_churn_summary() {
        let (mut collector, clock, _live) = started(vec![Vec::new()], 0);
        cycle(&mut collector, 1);

        let mut token = 2u64;
        for creation in 1..=6u64 {
            script(
                &mut collector,
                vec![denied_process(9, "shared.exe", creation)],
            );
            cycle(&mut collector, token);
            token += 1;
            script(&mut collector, Vec::new());
            cycle(&mut collector, token);
            token += 1;
        }
        clock.advance(churn::WINDOW_MS);
        let summary = cycle(&mut collector, token);
        let churn = events(&summary)
            .iter()
            .find(|event| event.kind.event_type == EventType::Churn)
            .expect("six pairs earn a churn summary");
        assert_eq!(churn.kind.confidence, Confidence::Low);
    }

    #[test]
    fn an_exit_nobody_watched_begin_is_not_a_matched_pair() {
        // The establishing baseline already contains this image, so the
        // collector never saw it start and its exit completes nothing.
        let (mut collector, clock, _live) = started(vec![vec![process(9, "flaky.exe", 1)]], 0);
        cycle(&mut collector, 1);
        script(&mut collector, Vec::new());
        let stopped = cycle(&mut collector, 2);
        assert_eq!(count(&stopped, EventType::Stopped), 1);
        assert!(
            state_of(&stopped).episodes.is_empty(),
            "an unmatched exit opens no episode"
        );
        clock.advance(churn::WINDOW_MS);
        let after = cycle(&mut collector, 3);
        assert_eq!(count(&after, EventType::Churn), 0);
    }
}
