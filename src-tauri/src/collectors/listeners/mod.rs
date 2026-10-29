//! The listener collector: a two-second diff-poll over the machine's listening
//! sockets.
//!
//! It reads the same four tables Windows' own `netstat` reads, and it never
//! resolves an owner by enumerating processes itself. The process collector
//! publishes one snapshot and both collectors read it, so a port is attributed
//! to a process the recorder actually saw rather than to whatever pid happens to
//! be in the socket table by the time the second enumeration runs.

pub mod diff;
pub mod observations;

use serde::{Deserialize, Serialize};

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use crate::collectors::listening_port_map::ListeningPortMap;
use crate::collectors::process_map::{ProcessIdentity, ProcessMap};
use crate::collectors::stateful;
use crate::collectors::{
    AckedState, Collector, CollectorContext, CollectorDescriptor, CollectorSnapshot, CommitToken,
    ObservationClock, RawObservation, RawRelationshipSpec, SnapshotContext, SystemObservationClock,
};
use crate::error::{AppError, AppResult};
use crate::platform::traits::{SocketProvider, SocketRow};
use crate::storage::{Baseline, CollectorCommitReceipt};

/// The catalog id this collector answers to.
pub const ID: &str = "listeners";

/// Endpoint identity already contains protocol/address/port. For an existing
/// endpoint the live collector only surfaces owner-set replacement.
pub(crate) fn checkpoint_fields(
    before: &serde_json::Value,
    after: &serde_json::Value,
) -> Option<Vec<crate::collectors::CheckpointFieldChange>> {
    diff::checkpoint_fields(before, after)
}

/// The durable baseline kind this collector owns.
pub const BASELINE_KIND: &str = "listening_sockets";

/// Normal machines fit comfortably in this caller-owned row allocation.
const SOCKET_ROW_CAPACITY: usize = 128;

/// This collector's descriptor, read from the canonical catalog.
///
/// The catalog owns the copy, the cadence and the cost, so a collector that
/// restated them could drift from what Settings shows.
pub fn catalog_descriptor() -> AppResult<CollectorDescriptor> {
    crate::collectors::registry::CollectorRegistry::production()
        .entries()
        .iter()
        .find(|entry| entry.descriptor.id == ID)
        .map(|entry| entry.descriptor.clone())
        .ok_or_else(|| AppError::InvalidInput {
            what: "collector catalog",
            reason: "the listener collector is not in the canonical catalog".to_owned(),
        })
}

/// The collector's whole durable memory, staged and acknowledged as one value.
///
/// A complete listener target and the process evidence observed with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingListenerTarget {
    /// The complete socket table after folding duplicate owner rows.
    pub sockets: diff::Sockets,
    /// Session-safe process identities captured at the same detection point.
    pub process_identities: BTreeMap<u32, ProcessIdentity>,
}

/// Durable collector memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ListenerState {
    /// What storage has been told, by endpoint id.
    pub acknowledged: diff::Sockets,
    /// A provider target too large for one atomic commit.
    #[serde(default)]
    pub pending: Option<stateful::PendingTarget<PendingListenerTarget>>,
    /// Whether the silent first baseline has been taken.
    pub established: bool,
}

impl ListenerState {
    /// Adopt a durable baseline, and say whether the next diff spans a gap.
    pub fn hydrate(baseline: Option<Baseline>, now_ms: i64) -> (Self, bool) {
        stateful::hydrate(baseline, now_ms, "the listener collector's endpoint table")
    }
}

/// Diffs the machine's listening sockets once per cadence tick.
pub struct ListenerCollector<P, C = SystemObservationClock> {
    provider: P,
    clock: C,
    map: ProcessMap,
    port_map: ListeningPortMap,
    descriptor: CollectorDescriptor,
    state: AckedState<ListenerState>,
    context: Option<CollectorContext>,
    /// Whether the next diff is against a baseline captured before a gap.
    ///
    /// Cleared only on a durable acknowledgement, so a rejected gap-diff is
    /// retried as uncertain rather than quietly promoted to a fact nobody
    /// observed.
    downtime: bool,
    /// Fresh-provider candidate whose acknowledgement may clear downtime.
    /// Draining a persisted target is historical work and never clears it.
    downtime_clear_token: Option<CommitToken>,
    /// Caller-owned provider output, refilled instead of reallocated each poll.
    rows: Vec<SocketRow>,
    /// Reusable current endpoint index. Durable ordered maps are materialized
    /// only for the first baseline or a changed target.
    current_index: diff::SocketIndex,
}

impl<P: SocketProvider> ListenerCollector<P, SystemObservationClock> {
    /// A collector against the production clock.
    pub fn new(provider: P, map: ProcessMap, descriptor: CollectorDescriptor) -> Self {
        Self::with_clock(provider, map, descriptor, SystemObservationClock)
    }

    /// A production collector that publishes the complete live port table.
    pub fn with_port_map(
        provider: P,
        map: ProcessMap,
        port_map: ListeningPortMap,
        descriptor: CollectorDescriptor,
    ) -> Self {
        Self::with_maps_and_clock(provider, map, port_map, descriptor, SystemObservationClock)
    }
}

impl<P: SocketProvider, C: ObservationClock> ListenerCollector<P, C> {
    /// A collector against an injected clock.
    pub fn with_clock(
        provider: P,
        map: ProcessMap,
        descriptor: CollectorDescriptor,
        clock: C,
    ) -> Self {
        Self::with_maps_and_clock(provider, map, ListeningPortMap::new(), descriptor, clock)
    }

    /// A collector with explicit shared evidence maps and clock.
    pub fn with_maps_and_clock(
        provider: P,
        map: ProcessMap,
        port_map: ListeningPortMap,
        descriptor: CollectorDescriptor,
        clock: C,
    ) -> Self {
        Self {
            provider,
            clock,
            map,
            port_map,
            descriptor,
            state: AckedState::new(ListenerState::default()),
            context: None,
            downtime: false,
            downtime_clear_token: None,
            rows: Vec::with_capacity(SOCKET_ROW_CAPACITY),
            current_index: diff::SocketIndex::with_capacity(SOCKET_ROW_CAPACITY),
        }
    }

    /// Process facts relevant to this socket target, captured at the same time.
    fn relevant_process_identities(
        &self,
        sockets: &diff::Sockets,
    ) -> BTreeMap<u32, ProcessIdentity> {
        let owners: BTreeSet<u32> = sockets
            .values()
            .flat_map(|socket| socket.owners.iter().copied())
            .collect();
        self.map.snapshot_for(&owners)
    }

    /// Stage one candidate state and describe the commit that carries it.
    fn stage(
        &mut self,
        token: CommitToken,
        next: ListenerState,
        now_ms: i64,
        observations: Vec<RawObservation>,
        relationships: Vec<RawRelationshipSpec>,
        clears_downtime: bool,
    ) -> AppResult<CollectorSnapshot> {
        let baseline = stateful::baseline_for(BASELINE_KIND, &next, now_ms)?;
        self.state.stage(token, next)?;
        self.downtime_clear_token = clears_downtime.then_some(token);
        Ok(CollectorSnapshot::Atomic {
            token,
            observations,
            relationships,
            baseline: Some(baseline),
            config_snapshots: Vec::new(),
        })
    }
}

impl<P: SocketProvider, C: ObservationClock> Collector for ListenerCollector<P, C> {
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let now_ms = self.clock.now_ms();
        let baseline = context.state.baseline(BASELINE_KIND)?;
        let (state, downtime) = ListenerState::hydrate(baseline, now_ms);
        self.state = AckedState::new(state);
        self.downtime = downtime;
        self.downtime_clear_token = None;
        self.context = Some(context);
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(context) = &self.context {
            self.port_map.invalidate(&context.cancel);
        }
        self.context = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Err(AppError::InvalidInput {
            what: "listener collector",
            reason: "a stateful collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        let Some(cancel) = self.context.as_ref().map(|held| held.cancel.clone()) else {
            return Ok(CollectorSnapshot::Noop);
        };
        let now_ms = self.clock.now_ms();
        self.provider.listeners_into(&mut self.rows)?;
        if cancel.is_cancelled() {
            self.rows.clear();
            return Ok(CollectorSnapshot::Noop);
        }
        diff::socket_index_into(&self.rows, &mut self.current_index);
        self.rows.clear();
        if !self.state.committed().established {
            // A new session takes its first snapshot without inventing an open
            // for every socket that was already listening.
            let next = ListenerState {
                acknowledged: diff::ordered_sockets(&self.current_index),
                pending: None,
                established: true,
            };
            return self.stage(context.token(), next, now_ms, Vec::new(), Vec::new(), true);
        }

        let pending = self.state.committed().pending.clone();
        let (target, observed_at, uncertain, changes) = if let Some(pending) = pending.as_ref() {
            let target = pending.target.clone();
            let changes = diff::diff(&self.state.committed().acknowledged, &target.sockets);
            (
                target,
                pending.observed_at,
                pending.occurred_at_is_uncertain,
                changes,
            )
        } else {
            if diff::unchanged(&self.state.committed().acknowledged, &self.current_index) {
                // The common path retains both the row allocation and the hash
                // buckets, and builds no second whole-table map.
                self.port_map.publish(
                    self.current_index.values().cloned(),
                    &cancel,
                    Instant::now(),
                );
                self.downtime = false;
                return Ok(CollectorSnapshot::Noop);
            }
            let changes =
                diff::diff_index(&self.state.committed().acknowledged, &self.current_index);
            let sockets = diff::ordered_sockets(&self.current_index);
            let target = PendingListenerTarget {
                process_identities: self.relevant_process_identities(&sockets),
                sockets,
            };
            (target, now_ms, self.downtime, changes)
        };
        let planned = observations::plan_with_identities(
            &changes,
            &target.process_identities,
            observed_at,
            uncertain,
        )?;
        if planned.is_empty() && pending.is_none() {
            // A quiet cycle has nothing to say and nothing to persist.
            self.downtime = false;
            return Ok(CollectorSnapshot::Noop);
        }

        let drained = planned.len();
        let mut observations = Vec::with_capacity(drained);
        let mut relationships = Vec::new();
        for row in planned {
            relationships.extend(row.relationships);
            observations.push(row.observation);
        }

        let mut acknowledged = self.state.committed().acknowledged.clone();
        for change in &changes[..drained] {
            diff::apply(&mut acknowledged, change);
        }

        let next_pending = (drained != changes.len()).then_some(stateful::PendingTarget {
            target,
            observed_at,
            occurred_at_is_uncertain: uncertain,
        });

        let next = ListenerState {
            acknowledged,
            pending: next_pending,
            established: true,
        };
        self.stage(
            context.token(),
            next,
            now_ms,
            observations,
            relationships,
            pending.is_none(),
        )
    }

    fn commit_succeeded(
        &mut self,
        token: CommitToken,
        _receipt: &CollectorCommitReceipt,
    ) -> AppResult<()> {
        self.state.ack(token)?;
        if let Some(context) = &self.context {
            self.port_map.publish(
                self.state.committed().acknowledged.values().cloned(),
                &context.cancel,
                Instant::now(),
            );
        }
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::collectors::process_map::ProcessIdentity;
    use crate::collectors::testing::{live_context, LiveContext};
    use crate::collectors::CancelToken;
    use crate::domain::ids::{ProcessId, Protocol};
    use crate::domain::{Confidence, EventType};
    use crate::platform::traits::SocketRow;
    use crate::storage::MAX_ATOMIC_RELATIONSHIPS;

    #[derive(Clone, Default)]
    struct ScriptedProvider {
        tables: Arc<Mutex<VecDeque<Vec<SocketRow>>>>,
    }

    impl ScriptedProvider {
        fn new(tables: Vec<Vec<SocketRow>>) -> Self {
            Self {
                tables: Arc::new(Mutex::new(tables.into())),
            }
        }

        fn next_table(&self) -> Vec<SocketRow> {
            let mut tables = self.tables.lock().unwrap();
            // The last scripted table is held rather than consumed, so a cycle
            // that was not given a new one reads the machine as unchanged
            // instead of as empty.
            if tables.len() > 1 {
                return tables.pop_front().unwrap_or_default();
            }
            tables.front().cloned().unwrap_or_default()
        }
    }

    impl SocketProvider for ScriptedProvider {
        fn listeners(&self) -> AppResult<Vec<SocketRow>> {
            Ok(self.next_table())
        }

        fn listeners_into(&self, output: &mut Vec<SocketRow>) -> AppResult<()> {
            output.clear();
            output.extend(self.next_table());
            Ok(())
        }
    }

    struct ManualClock(Arc<AtomicI64>);

    impl ObservationClock for ManualClock {
        fn now_ms(&self) -> i64 {
            self.0.load(Ordering::Acquire)
        }
    }

    type UnderTest = ListenerCollector<ScriptedProvider, ManualClock>;

    fn socket(protocol: Protocol, address: &str, port: u16, pid: u32) -> SocketRow {
        SocketRow {
            protocol,
            address: address.to_owned(),
            port,
            owning_pid: pid,
        }
    }

    /// A started collector whose map resolves one pid to `api.exe`.
    fn started(tables: Vec<Vec<SocketRow>>, now_ms: i64) -> (UnderTest, ProcessMap, LiveContext) {
        let map = ProcessMap::new();
        map.publish(
            [(
                4_242u32,
                ProcessIdentity {
                    entity_id: ProcessId::from_executable(r"C:\dev\api.exe"),
                    display_name: "api.exe".to_owned(),
                    confidence: Confidence::High,
                },
            )]
            .into_iter()
            .collect(),
            &CancelToken::default(),
            std::time::Instant::now(),
        );
        let mut collector = ListenerCollector::with_clock(
            ScriptedProvider::new(tables),
            map.clone(),
            catalog_descriptor().unwrap(),
            ManualClock(Arc::new(AtomicI64::new(now_ms))),
        );
        let live = live_context(ID).unwrap();
        collector.start(live.context.clone()).unwrap();
        (collector, map, live)
    }

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

    fn script(collector: &mut UnderTest, table: Vec<SocketRow>) {
        collector.provider = ScriptedProvider::new(vec![table]);
    }

    fn atomic(snapshot: &CollectorSnapshot) -> (&[RawObservation], &[RawRelationshipSpec]) {
        match snapshot {
            CollectorSnapshot::Atomic {
                observations,
                relationships,
                ..
            } => (observations, relationships),
            CollectorSnapshot::Noop => (&[], &[]),
            other => panic!("expected an atomic snapshot, got {other:?}"),
        }
    }

    fn state_of(snapshot: &CollectorSnapshot) -> ListenerState {
        match snapshot {
            CollectorSnapshot::Atomic {
                baseline: Some(stored),
                ..
            } => serde_json::from_value(stored.snapshot.clone()).unwrap(),
            other => panic!("expected an atomic snapshot with a baseline, got {other:?}"),
        }
    }

    #[test]
    fn the_catalog_owns_the_descriptor() {
        let descriptor = catalog_descriptor().unwrap();
        assert_eq!(descriptor.id, ID);
        assert!(!descriptor.requires_admin);
    }

    #[test]
    fn a_first_snapshot_records_a_baseline_and_invents_no_opens() {
        let table = vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 4_242)];
        let (mut collector, _, _live) = started(vec![table], 1_000);
        let snapshot = cycle(&mut collector, 1);
        let (observations, relationships) = atomic(&snapshot);
        assert!(observations.is_empty());
        assert!(relationships.is_empty());
        let state = state_of(&snapshot);
        assert!(state.established);
        assert_eq!(state.acknowledged.len(), 1);
    }

    #[test]
    fn a_quiet_cycle_is_a_noop() {
        let table = vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 4_242)];
        let (mut collector, _, _live) = started(vec![table], 1_000);
        cycle(&mut collector, 1);
        assert_eq!(cycle(&mut collector, 2), CollectorSnapshot::Noop);
    }

    #[test]
    fn quiet_polls_reuse_row_storage_and_the_endpoint_index() {
        let table = vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 4_242)];
        let (mut collector, _, _live) = started(vec![table], 1_000);
        let row_allocation = (collector.rows.as_ptr(), collector.rows.capacity());
        let index_capacity = collector.current_index.capacity();

        cycle(&mut collector, 1);
        assert!(collector.rows.is_empty());
        assert_eq!(
            (collector.rows.as_ptr(), collector.rows.capacity()),
            row_allocation
        );
        assert_eq!(collector.current_index.capacity(), index_capacity);

        assert_eq!(cycle(&mut collector, 2), CollectorSnapshot::Noop);
        assert!(collector.rows.is_empty());
        assert_eq!(
            (collector.rows.as_ptr(), collector.rows.capacity()),
            row_allocation
        );
        assert_eq!(collector.current_index.capacity(), index_capacity);
    }

    #[test]
    fn an_opened_port_lands_with_both_directions_of_its_edge() {
        let (mut collector, _, _live) = started(vec![Vec::new()], 1_000);
        cycle(&mut collector, 1);
        script(
            &mut collector,
            vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 4_242)],
        );

        let snapshot = cycle(&mut collector, 2);
        let (observations, relationships) = atomic(&snapshot);
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].kind.event_type, EventType::Opened);
        assert_eq!(
            observations[0].kind.user_friendly_summary,
            "Port 4000 opened by: api.exe"
        );
        assert_eq!(relationships.len(), 2, "both directions of one edge");
        assert!(state_of(&snapshot)
            .acknowledged
            .contains_key("port:tcp:0.0.0.0:4000"));

        script(&mut collector, Vec::new());
        let closed = cycle(&mut collector, 3);
        let (observations, relationships) = atomic(&closed);
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].kind.event_type, EventType::Closed);
        assert!(relationships.is_empty());
        assert!(state_of(&closed).acknowledged.is_empty());
    }

    #[test]
    fn an_owner_change_is_one_modification_and_not_a_close_and_an_open() {
        let (mut collector, _, _live) = started(
            vec![vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 4_242)]],
            1_000,
        );
        cycle(&mut collector, 1);
        script(
            &mut collector,
            vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 7)],
        );

        let snapshot = cycle(&mut collector, 2);
        let (observations, _) = atomic(&snapshot);
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].kind.event_type, EventType::Modified);
        assert_eq!(
            observations[0].kind.confidence,
            Confidence::Low,
            "pid 7 is not in the published process snapshot"
        );
        assert_eq!(
            observations[0].details.as_ref().unwrap()["owner_unresolved"],
            true
        );
    }

    #[test]
    fn a_stale_or_absent_publisher_yields_no_edges_rather_than_a_guess() {
        let (mut collector, map, _live) = started(vec![Vec::new()], 1_000);
        cycle(&mut collector, 1);
        // Republish under a token that is then cancelled, which is exactly what
        // a paused or replaced process collector leaves behind.
        let token = CancelToken::default();
        map.publish(
            [(
                4_242u32,
                ProcessIdentity {
                    entity_id: ProcessId::from_executable("api.exe"),
                    display_name: "api.exe".to_owned(),
                    confidence: Confidence::High,
                },
            )]
            .into_iter()
            .collect(),
            &token,
            std::time::Instant::now(),
        );
        token.cancel();
        script(
            &mut collector,
            vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 4_242)],
        );

        let snapshot = cycle(&mut collector, 2);
        let (observations, relationships) = atomic(&snapshot);
        assert_eq!(observations.len(), 1, "the port still opened");
        assert!(relationships.is_empty(), "but nobody proved who owns it");
        assert_eq!(observations[0].kind.confidence, Confidence::Low);
    }

    #[test]
    fn a_burst_larger_than_one_commit_arrives_whole_over_several_cycles() {
        const BURST: u16 = 500;
        let (mut collector, _, _live) = started(vec![Vec::new()], 1_000);
        cycle(&mut collector, 1);
        // Every endpoint is owned by the pid the map resolves, so each one
        // costs two graph edges and the relationship bound is what bites.
        script(
            &mut collector,
            (0..BURST)
                .map(|index| socket(Protocol::Tcp, "0.0.0.0", 1_000 + index, 4_242))
                .collect(),
        );

        let mut delivered = 0usize;
        let mut widest = 0usize;
        let mut widest_edges = 0usize;
        for token in 2..12u64 {
            let snapshot = cycle(&mut collector, token);
            let (observations, relationships) = atomic(&snapshot);
            widest = widest.max(observations.len());
            widest_edges = widest_edges.max(relationships.len());
            delivered += observations.len();
            assert!(
                relationships.len() <= MAX_ATOMIC_RELATIONSHIPS,
                "no batch overshot the edge bound"
            );
            if delivered >= usize::from(BURST) {
                break;
            }
        }
        assert_eq!(delivered, usize::from(BURST), "nothing truncated");
        assert!(widest > 1, "the burst was actually drained in batches");
        assert_eq!(
            widest_edges, MAX_ATOMIC_RELATIONSHIPS,
            "the drain filled the bound instead of guessing under it"
        );
        assert_eq!(cycle(&mut collector, 30), CollectorSnapshot::Noop);
    }

    #[test]
    fn a_deferred_short_lived_listener_survives_recreation_with_original_edges_and_time() {
        const BURST: u16 = 450;
        let (mut collector, _map, live) = started(vec![Vec::new()], 1_000);
        cycle(&mut collector, 1);
        collector.clock.0.store(2_000, Ordering::Release);
        script(
            &mut collector,
            (0..BURST)
                .map(|index| socket(Protocol::Tcp, "0.0.0.0", 1_000 + index, 4_242))
                .collect(),
        );

        let first = cycle(&mut collector, 2);
        let (first_events, first_edges) = atomic(&first);
        assert_eq!(first_events.len(), 200);
        assert_eq!(first_edges.len(), MAX_ATOMIC_RELATIONSHIPS);
        assert!(first_events.iter().all(|event| event.occurred_at == 2_000));
        let durable = state_of(&first);
        assert!(durable.pending.is_some());
        let stored = Baseline {
            collector_id: ID.to_owned(),
            entity_kind: BASELINE_KIND.to_owned(),
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
        let mut resumed = ListenerCollector::with_clock(
            ScriptedProvider::new(vec![Vec::new()]),
            ProcessMap::new(),
            catalog_descriptor().unwrap(),
            ManualClock(Arc::new(AtomicI64::new(
                2_000 + stateful::DOWNTIME_GAP_MS + 1,
            ))),
        );
        resumed.start(live.context.clone()).unwrap();

        let second = cycle(&mut resumed, 3);
        let third = cycle(&mut resumed, 4);
        let (second_events, second_edges) = atomic(&second);
        let (third_events, third_edges) = atomic(&third);
        assert_eq!(second_events.len(), 200);
        assert_eq!(second_edges.len(), MAX_ATOMIC_RELATIONSHIPS);
        assert_eq!(third_events.len(), 50);
        assert_eq!(third_edges.len(), 100);
        assert!(second_events
            .iter()
            .chain(third_events)
            .all(|event| event.kind.event_type == EventType::Opened
                && event.occurred_at == 2_000
                && !event.occurred_at_is_uncertain));
        assert!(state_of(&third).pending.is_none());
        assert_eq!(
            third_events.last().unwrap().entity.display_name,
            "TCP 0.0.0.0:1449",
            "the endpoint vanished from live tables but remained in the durable target"
        );

        let later = cycle(&mut resumed, 5);
        let (later_events, later_edges) = atomic(&later);
        assert!(
            later_events
                .iter()
                .all(|event| event.kind.event_type == EventType::Closed
                    && event.occurred_at_is_uncertain),
            "draining the historical pending target must not clear fresh-gap uncertainty"
        );
        assert!(later_edges.is_empty());
    }

    #[test]
    fn an_oversized_single_listener_transition_errors_without_state_advance() {
        let pids: Vec<u32> = (1..=201).collect();
        let map = ProcessMap::new();
        let token = CancelToken::default();
        map.publish(
            pids.iter()
                .map(|pid| {
                    (
                        *pid,
                        ProcessIdentity {
                            entity_id: ProcessId::from_executable(&format!("app-{pid}.exe")),
                            display_name: format!("app-{pid}.exe"),
                            confidence: Confidence::High,
                        },
                    )
                })
                .collect(),
            &token,
            std::time::Instant::now(),
        );
        let mut collector = ListenerCollector::with_clock(
            ScriptedProvider::new(vec![Vec::new()]),
            map,
            catalog_descriptor().unwrap(),
            ManualClock(Arc::new(AtomicI64::new(1_000))),
        );
        let live = live_context(ID).unwrap();
        collector.start(live.context.clone()).unwrap();
        cycle(&mut collector, 1);
        script(
            &mut collector,
            pids.iter()
                .map(|pid| socket(Protocol::Tcp, "0.0.0.0", 4_000, *pid))
                .collect(),
        );

        let error = collector
            .snapshot_batch(SnapshotContext::new(CommitToken::mint(2)))
            .unwrap_err();
        assert!(matches!(
            error,
            AppError::InvalidInput {
                what: "listener transition",
                ..
            }
        ));
        assert!(collector.state.committed().acknowledged.is_empty());
        assert!(collector.state.committed().pending.is_none());
    }

    #[test]
    fn a_downtime_diff_marks_its_timestamps_uncertain_and_then_stops() {
        let table = vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 4_242)];
        let (mut collector, _, live) = started(vec![table], 1_000);
        let first = cycle(&mut collector, 1);
        let state = state_of(&first);
        // Rebuild the durable baseline far enough in the past that the next
        // diff is a gap, then start a fresh collector over the same database.
        let stored = Baseline {
            collector_id: ID.to_owned(),
            entity_kind: BASELINE_KIND.to_owned(),
            snapshot: serde_json::to_value(&state).unwrap(),
            captured_at: 1_000,
            truncated: false,
        };
        live.db
            .write(move |connection| {
                crate::storage::baseline_repository::store(
                    connection,
                    &crate::privacy::Sanitizer::new().sanitize(stored.clone()).0,
                )
            })
            .unwrap();

        let mut resumed = ListenerCollector::with_clock(
            ScriptedProvider::new(vec![Vec::new()]),
            ProcessMap::new(),
            catalog_descriptor().unwrap(),
            ManualClock(Arc::new(AtomicI64::new(
                1_000 + stateful::DOWNTIME_GAP_MS + 1,
            ))),
        );
        resumed.start(live.context.clone()).unwrap();
        let snapshot = cycle(&mut resumed, 2);
        let (observations, _) = atomic(&snapshot);
        assert_eq!(observations.len(), 1);
        assert!(
            observations[0].occurred_at_is_uncertain,
            "a change found across a gap has a detection time, not an occurrence time"
        );

        script(
            &mut resumed,
            vec![socket(Protocol::Tcp, "0.0.0.0", 4_000, 4_242)],
        );
        let later = cycle(&mut resumed, 3);
        let (observations, _) = atomic(&later);
        assert_eq!(observations.len(), 1);
        assert!(
            !observations[0].occurred_at_is_uncertain,
            "the gap was closed by the acknowledged diff"
        );
    }
}
