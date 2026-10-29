//! Docker Engine collector lifecycle and reconnect state machine.
//!
//! Only the read-only endpoints represented by [`DockerRequest`] are reachable.
//! `Config.Env` is never requested: container environment values commonly hold
//! credentials and are unnecessary for change history.

mod mapping;
mod stream;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Duration;

use crate::collectors::docker_protocol::{
    parse_containers_response, parse_images_response, parse_version_response, DockerAction,
    DockerApiVersion, DockerDaemonVersion, DockerObjectKind, DockerRequest, ParsedDockerEvent,
    PRIMARY_API_VERSION,
};
use crate::collectors::stateful;
use crate::collectors::{
    AckedState, Collector, CollectorContext, CollectorSnapshot, CommitToken, EntityIdentity,
    RawObservation, RawRelationshipSpec, SnapshotContext, SystemObservationClock,
};
use crate::domain::{
    CollectorDescriptor, CollectorStatusDetails, DockerCollectorState, RelationshipKind,
};
use crate::error::{AppError, AppResult};
use crate::platform::traits::DockerTransport;
use crate::storage::CollectorCommitReceipt;

use mapping::{
    apply_event, baseline_from, compose_observation, diff_baseline, mapped_event, DockerBaseline,
    MappedChange,
};
use stream::{RetryTimer, StreamReader};

pub const ID: &str = "docker";
pub const BASELINE_KIND: &str = "docker_engine";
pub const UNAVAILABLE_MESSAGE: &str =
    "Docker isn't running. We'll start recording container changes when it does.";
pub const COMPOSE_WINDOW_MS: i64 = 30_000;
const MAX_COMPOSE_WINDOWS: usize = 128;
const MAX_COMPOSE_CONTAINER_IDS: usize = 256;
// Failure number five happens only after the capped 30-second delay has been
// observed once (2 -> 4 -> 8 -> 16 -> 30 seconds). Only then do we surface the
// daemon as unavailable; cutting over sooner would make the documented cap
// dead code in the real state machine.
const MAX_RECONNECT_FAILURES: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComposeWindow {
    started_at: i64,
    container_ids: BTreeSet<String>,
    emitted: bool,
}

#[derive(Debug, Clone)]
struct PendingDelivery {
    chunks: VecDeque<PendingChunk>,
    compose_windows: BTreeMap<String, ComposeWindow>,
    clears_resync_uncertainty: bool,
}

#[derive(Debug, Clone)]
struct PendingChunk {
    baseline: DockerBaseline,
    observations: Vec<RawObservation>,
    relationships: Vec<RawRelationshipSpec>,
}

pub const fn reconnect_delay(attempt: u32) -> Duration {
    let shift = if attempt > 4 { 4 } else { attempt };
    let seconds = 2_u64 << shift;
    Duration::from_secs(if seconds > 30 { 30 } else { seconds })
}

pub struct DockerCollector<T, C = SystemObservationClock> {
    transport: T,
    descriptor: CollectorDescriptor,
    clock: C,
    context: Option<CollectorContext>,
    state: AckedState<DockerBaseline>,
    compose_windows: BTreeMap<String, ComposeWindow>,
    pending: Option<PendingDelivery>,
    machine: DockerCollectorState,
    api_version: Option<DockerApiVersion>,
    reader: Option<StreamReader>,
    /// Events read while the initial typed baseline was being fetched. They
    /// are reconciled against that baseline before normal stream processing.
    buffered_events: VecDeque<ParsedDockerEvent>,
    reconcile_against: Option<(DockerBaseline, i64)>,
    retry_timer: Option<RetryTimer>,
    reconnect_attempt: u32,
    retry_not_before_ms: i64,
    transition_count: usize,
    resync_uncertain: bool,
}

impl<T: DockerTransport> DockerCollector<T> {
    pub fn new(transport: T, descriptor: CollectorDescriptor) -> Self {
        Self::with_clock(transport, descriptor, SystemObservationClock)
    }
}

impl<T: DockerTransport, C: crate::collectors::ObservationClock> DockerCollector<T, C> {
    pub fn with_clock(transport: T, descriptor: CollectorDescriptor, clock: C) -> Self {
        Self {
            transport,
            descriptor,
            clock,
            context: None,
            state: AckedState::new(DockerBaseline::default()),
            compose_windows: BTreeMap::new(),
            pending: None,
            machine: DockerCollectorState::Probing,
            api_version: None,
            reader: None,
            buffered_events: VecDeque::new(),
            reconcile_against: None,
            retry_timer: None,
            reconnect_attempt: 0,
            retry_not_before_ms: 0,
            transition_count: 0,
            resync_uncertain: false,
        }
    }

    pub const fn machine_state(&self) -> DockerCollectorState {
        self.machine
    }

    pub const fn transition_count(&self) -> usize {
        self.transition_count
    }

    pub const fn next_reconnect_delay(&self) -> Duration {
        reconnect_delay(self.reconnect_attempt)
    }

    pub const fn retry_not_before_ms(&self) -> i64 {
        self.retry_not_before_ms
    }

    fn update_status(&self) -> AppResult<()> {
        if let Some(context) = &self.context {
            context
                .status_details
                .replace(Some(CollectorStatusDetails {
                    wake_tier: None,
                    docker_state: Some(self.machine),
                    docker_api_version: self.api_version.map(|version| version.as_str().to_owned()),
                }))?;
        }
        Ok(())
    }

    fn transition(&mut self, next: DockerCollectorState) -> AppResult<()> {
        if self.machine == next {
            return Ok(());
        }
        let previous = self.machine;
        self.machine = next;
        self.transition_count = self.transition_count.saturating_add(1);
        tracing::info!(
            collector = ID,
            from = previous.as_str(),
            to = next.as_str(),
            "Docker collector state changed"
        );
        self.update_status()
    }

    fn connect(&mut self, token: CommitToken) -> AppResult<CollectorSnapshot> {
        let now_ms = self.clock.now_ms();
        if self.machine == DockerCollectorState::Reconnecting && now_ms < self.retry_not_before_ms {
            return Ok(CollectorSnapshot::Noop);
        }
        match self
            .transport
            .get(&DockerRequest::Ping.path(PRIMARY_API_VERSION))
        {
            Ok(body) if body == b"OK" => {}
            Ok(_) | Err(_) => return self.connection_failed(now_ms),
        }

        let (version, daemon) = match self.negotiate_version() {
            Ok(negotiated) => negotiated,
            Err(_) => return self.connection_interrupted(),
        };
        let stream_started_at_ms = now_ms;
        let stream = match self.transport.stream(&DockerRequest::Events.path(version)) {
            Ok(stream) => stream,
            Err(_) => return self.connection_interrupted(),
        };
        let Some(context) = self.context.as_ref() else {
            return Ok(CollectorSnapshot::Noop);
        };
        let mut reader = StreamReader::start(stream, context.wake.clone());
        // Subscribe before the list snapshots. Events arriving while Docker
        // builds the typed baseline are queued by the reader instead of
        // falling into a snapshot-then-subscribe gap.
        let next = match self.fetch_baseline(version, daemon.engine_version) {
            Ok(next) => next,
            Err(_) => {
                reader.stop();
                return self.connection_interrupted();
            }
        };
        let (startup_events, _) = reader.drain();
        self.buffered_events.extend(startup_events);
        self.reader = Some(reader);
        self.reconcile_against = Some((next.clone(), stream_started_at_ms));
        if let Some(mut timer) = self.retry_timer.take() {
            timer.stop();
        }
        self.api_version = Some(version);
        self.reconnect_attempt = 0;
        self.retry_not_before_ms = 0;
        self.transition(DockerCollectorState::Available)?;

        if !self.state.committed().established {
            return self.stage_new(
                token,
                vec![MappedChange {
                    baseline: next,
                    observations: Vec::new(),
                    relationships: Vec::new(),
                }],
                self.compose_windows.clone(),
                true,
            );
        }
        let mut changes = diff_baseline(self.state.committed(), &next, now_ms);
        if self.resync_uncertain {
            for change in &mut changes {
                for observation in &mut change.observations {
                    observation.occurred_at_is_uncertain = true;
                }
            }
        }
        if changes.is_empty() {
            self.resync_uncertain = false;
            return Ok(CollectorSnapshot::Noop);
        }
        self.stage_new(token, changes, self.compose_windows.clone(), true)
    }

    fn negotiate_version(&mut self) -> AppResult<(DockerApiVersion, DockerDaemonVersion)> {
        let primary_path = DockerRequest::Version.path(PRIMARY_API_VERSION);
        match self.transport.get(&primary_path) {
            Ok(body) => Ok((
                PRIMARY_API_VERSION,
                parse_version_response(&body).map_err(protocol_error)?,
            )),
            Err(error) if is_version_too_new(&error) => {
                let fallback = crate::collectors::docker_protocol::FALLBACK_API_VERSION;
                let body = self.transport.get(&DockerRequest::Version.path(fallback))?;
                Ok((
                    fallback,
                    parse_version_response(&body).map_err(protocol_error)?,
                ))
            }
            Err(error) => Err(error),
        }
    }

    fn fetch_baseline(
        &mut self,
        version: DockerApiVersion,
        daemon_version: String,
    ) -> AppResult<DockerBaseline> {
        let containers = self
            .transport
            .get(&DockerRequest::Containers.path(version))?;
        let images = self.transport.get(&DockerRequest::Images.path(version))?;
        Ok(baseline_from(
            parse_containers_response(&containers).map_err(protocol_error)?,
            parse_images_response(&images).map_err(protocol_error)?,
            daemon_version,
        ))
    }

    fn drain_stream(&mut self, token: CommitToken) -> AppResult<CollectorSnapshot> {
        let (stream_events, disconnected) = self
            .reader
            .as_ref()
            .map_or_else(|| (Vec::new(), true), StreamReader::drain);
        let mut events: Vec<ParsedDockerEvent> = self.buffered_events.drain(..).collect();
        events.extend(stream_events);
        if let Some((baseline, cutoff_ms)) = self.reconcile_against.take() {
            // Only discard events that predate stream subscription. Events
            // occurring during/after the list request remain authoritative,
            // even if the typed snapshot happened to observe their result.
            events.retain(|event| {
                event.occurred_at_ms > cutoff_ms || !baseline.reconciles_event(event)
            });
        }
        if disconnected {
            self.enter_reconnecting()?;
        }
        if events.is_empty() {
            return Ok(CollectorSnapshot::Noop);
        }

        let mut next = self.state.committed().clone();
        let refresh_needed = events.iter().any(|event| {
            event.object_kind == DockerObjectKind::Container
                && (event.action == DockerAction::Created || !next.has_container_instance(event))
        });
        let typed_containers = if refresh_needed {
            self.api_version
                .and_then(|version| self.fetch_typed_containers(version).ok())
        } else {
            None
        };
        let mut compose = self.compose_windows.clone();
        let mut changes = Vec::new();
        for event in events {
            let qualifier = typed_containers
                .as_ref()
                .filter(|baseline| baseline.has_container_instance(&event))
                .unwrap_or(&next);
            let event = qualifier.qualify_event(&event);
            apply_event(&mut next, &event);
            let (mut observations, relationships) = mapped_event(&event);
            append_compose_event(&mut compose, &event, &mut observations);
            changes.push(MappedChange {
                baseline: next.clone(),
                observations,
                relationships,
            });
        }
        self.stage_new(token, changes, compose, false)
    }

    fn fetch_typed_containers(&mut self, version: DockerApiVersion) -> AppResult<DockerBaseline> {
        let containers = self
            .transport
            .get(&DockerRequest::Containers.path(version))?;
        Ok(baseline_from(
            parse_containers_response(&containers).map_err(protocol_error)?,
            Vec::new(),
            String::new(),
        ))
    }

    fn enter_reconnecting(&mut self) -> AppResult<()> {
        if let Some(mut reader) = self.reader.take() {
            reader.stop();
        }
        self.reconnect_attempt = 0;
        self.resync_uncertain = true;
        self.retry_not_before_ms = self
            .clock
            .now_ms()
            .saturating_add(duration_ms(reconnect_delay(0)));
        self.schedule_retry(reconnect_delay(0));
        self.transition(DockerCollectorState::Reconnecting)
    }

    fn connection_failed(&mut self, now_ms: i64) -> AppResult<CollectorSnapshot> {
        self.reader = None;
        self.api_version = None;
        if self.machine == DockerCollectorState::Reconnecting {
            self.reconnect_attempt = self.reconnect_attempt.saturating_add(1);
            if self.reconnect_attempt >= MAX_RECONNECT_FAILURES {
                self.retry_not_before_ms = 0;
                if let Some(mut timer) = self.retry_timer.take() {
                    timer.stop();
                }
                self.transition(DockerCollectorState::Unavailable)?;
                return Ok(CollectorSnapshot::Noop);
            }
            self.retry_not_before_ms =
                now_ms.saturating_add(duration_ms(reconnect_delay(self.reconnect_attempt)));
            self.schedule_retry(reconnect_delay(self.reconnect_attempt));
            self.update_status()?;
        } else {
            self.transition(DockerCollectorState::Unavailable)?;
        }
        Ok(CollectorSnapshot::Noop)
    }

    fn connection_interrupted(&mut self) -> AppResult<CollectorSnapshot> {
        self.enter_reconnecting()?;
        Ok(CollectorSnapshot::Noop)
    }

    fn schedule_retry(&mut self, delay: Duration) {
        if let Some(mut timer) = self.retry_timer.take() {
            timer.stop();
        }
        if let Some(context) = &self.context {
            self.retry_timer = Some(RetryTimer::start(delay, context.wake.clone()));
        }
    }

    fn stage_new(
        &mut self,
        token: CommitToken,
        changes: Vec<MappedChange>,
        compose_windows: BTreeMap<String, ComposeWindow>,
        clears_resync_uncertainty: bool,
    ) -> AppResult<CollectorSnapshot> {
        let chunks = delivery_chunks(changes)?;
        self.pending = Some(PendingDelivery {
            chunks,
            compose_windows,
            clears_resync_uncertainty,
        });
        self.stage_pending(token)
    }

    fn stage_pending(&mut self, token: CommitToken) -> AppResult<CollectorSnapshot> {
        let pending = self
            .pending
            .as_ref()
            .ok_or_else(|| AppError::InvalidInput {
                what: "Docker collector delivery",
                reason: "no pending batch exists".to_owned(),
            })?;
        let chunk = pending
            .chunks
            .front()
            .ok_or_else(|| AppError::InvalidInput {
                what: "Docker collector delivery",
                reason: "the pending delivery has no chunk".to_owned(),
            })?;
        let captured_at = chunk
            .observations
            .iter()
            .map(|observation| observation.occurred_at)
            .fold(self.clock.now_ms(), i64::max);
        let baseline =
            stateful::baseline_for_or_marker(BASELINE_KIND, &chunk.baseline, captured_at)?;
        self.state.stage(token, chunk.baseline.clone())?;
        Ok(CollectorSnapshot::Atomic {
            token,
            observations: chunk.observations.clone(),
            relationships: chunk.relationships.clone(),
            baseline: Some(baseline),
            config_snapshots: Vec::new(),
        })
    }
}

impl<T: DockerTransport, C: crate::collectors::ObservationClock> Collector
    for DockerCollector<T, C>
{
    fn id(&self) -> &'static str {
        ID
    }

    fn descriptor(&self) -> CollectorDescriptor {
        self.descriptor.clone()
    }

    fn start(&mut self, context: CollectorContext) -> AppResult<()> {
        let (baseline, downtime) = stateful::hydrate(
            context.state.baseline(BASELINE_KIND)?,
            self.clock.now_ms(),
            "Docker baseline",
        );
        self.state = AckedState::new(baseline);
        self.resync_uncertain = downtime;
        self.context = Some(context);
        self.machine = DockerCollectorState::Probing;
        self.update_status()
    }

    fn stop(&mut self) {
        if let Some(mut reader) = self.reader.take() {
            reader.stop();
        }
        if let Some(mut timer) = self.retry_timer.take() {
            timer.stop();
        }
        self.context = None;
    }

    fn snapshot(&mut self) -> AppResult<Vec<RawObservation>> {
        Err(AppError::InvalidInput {
            what: "Docker collector",
            reason: "a stateful collector commits through snapshot_batch".to_owned(),
        })
    }

    fn snapshot_batch(&mut self, context: SnapshotContext) -> AppResult<CollectorSnapshot> {
        if self
            .context
            .as_ref()
            .is_none_or(|held| held.cancel.is_cancelled())
        {
            self.stop();
            return Ok(CollectorSnapshot::Noop);
        }
        if self.pending.is_some() {
            return self.stage_pending(context.token());
        }
        match self.machine {
            DockerCollectorState::Probing | DockerCollectorState::Unavailable => {
                self.connect(context.token())
            }
            DockerCollectorState::Reconnecting => self.connect(context.token()),
            DockerCollectorState::Available => self.drain_stream(context.token()),
        }
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
            if let Some(pending) = self.pending.take() {
                self.compose_windows = pending.compose_windows;
                if pending.clears_resync_uncertainty {
                    self.resync_uncertain = false;
                }
            }
        }
        if let Some(context) = &self.context {
            // Each chunk is a complete atomic commit. Keep draining immediately
            // rather than waiting for the periodic collector sweep. The same
            // trailing wake preserves stream/reconnect work that raced replay.
            let _ = context.wake.request_snapshot();
        }
        Ok(())
    }

    fn commit_failed(&mut self, token: CommitToken) -> AppResult<()> {
        self.state.reject(token)
    }
}

/// Pack an ordered sequence of state transitions into storage-sized atomic
/// commits. Every chunk carries the exact baseline reached by its last event;
/// a crash after an intermediate acknowledgement therefore resumes at the
/// acknowledged transition instead of replaying the whole burst against the
/// final snapshot.
fn delivery_chunks(changes: Vec<MappedChange>) -> AppResult<VecDeque<PendingChunk>> {
    if changes.is_empty() {
        return Ok(VecDeque::new());
    }

    let mut chunks = VecDeque::new();
    let mut capacity = stateful::CommitCapacity::full();
    let mut chunk_baseline: Option<DockerBaseline> = None;
    let mut chunk_observations = Vec::new();
    let mut chunk_relationships = Vec::new();
    let mut seen_edges: Vec<(EntityIdentity, EntityIdentity, RelationshipKind)> = Vec::new();

    for change in changes {
        if change.observations.is_empty() {
            // A baseline-only transition cannot be folded into an observation
            // chunk: the empty commit is what makes that state durable.
            if !chunk_observations.is_empty() {
                let Some(baseline) = chunk_baseline.take() else {
                    return Err(invalid_delivery("an observation chunk lost its baseline"));
                };
                chunks.push_back(PendingChunk {
                    baseline,
                    observations: std::mem::take(&mut chunk_observations),
                    relationships: std::mem::take(&mut chunk_relationships),
                });
                capacity = stateful::CommitCapacity::full();
            }
            chunks.push_back(PendingChunk {
                baseline: change.baseline,
                observations: Vec::new(),
                relationships: Vec::new(),
            });
            continue;
        }

        let mut relationships_by_observation: Vec<Vec<RawRelationshipSpec>> =
            (0..change.observations.len()).map(|_| Vec::new()).collect();
        for mut relationship in change.relationships {
            let Some(bucket) =
                relationships_by_observation.get_mut(relationship.evidence_observation)
            else {
                return Err(AppError::InvalidInput {
                    what: "Docker collector delivery",
                    reason: "a relationship names a missing observation".to_owned(),
                });
            };
            let edge = (
                relationship.source.identity.clone(),
                relationship.target.identity.clone(),
                relationship.kind,
            );
            // Repeated publications (especially when a stream reconnects) are
            // one graph edge, not an excuse to overflow the atomic relationship
            // budget and retry forever.
            if seen_edges.contains(&edge) {
                continue;
            }
            seen_edges.push(edge);
            relationship.evidence_observation = 0;
            bucket.push(relationship);
        }

        let change_relationships = relationships_by_observation
            .iter()
            .map(Vec::len)
            .sum::<usize>();
        let change_related_entities = change
            .observations
            .iter()
            .zip(&relationships_by_observation)
            .map(|(observation, edges)| unique_related_endpoint_count(observation, edges))
            .sum::<usize>();
        if !capacity.fits(
            change.observations.len(),
            change_relationships,
            change_related_entities,
        ) && !chunk_observations.is_empty()
        {
            let Some(baseline) = chunk_baseline.take() else {
                return Err(invalid_delivery("an observation chunk lost its baseline"));
            };
            chunks.push_back(PendingChunk {
                baseline,
                observations: std::mem::take(&mut chunk_observations),
                relationships: std::mem::take(&mut chunk_relationships),
            });
            capacity = stateful::CommitCapacity::full();
        }
        if !capacity.fits(
            change.observations.len(),
            change_relationships,
            change_related_entities,
        ) {
            return Err(AppError::InvalidInput {
                what: "Docker collector delivery",
                reason: "one Docker event exceeds the atomic commit limit".to_owned(),
            });
        }
        let observation_offset = chunk_observations.len();
        let _ = capacity.spend(
            change.observations.len(),
            change_relationships,
            change_related_entities,
        );
        for (index, mut edges) in relationships_by_observation.into_iter().enumerate() {
            for edge in &mut edges {
                edge.evidence_observation = observation_offset + index;
            }
            chunk_relationships.extend(edges);
        }
        chunk_observations.extend(change.observations);
        chunk_baseline = Some(change.baseline);
    }

    if !chunk_observations.is_empty() {
        let Some(baseline) = chunk_baseline else {
            return Err(invalid_delivery("an observation chunk lost its baseline"));
        };
        chunks.push_back(PendingChunk {
            baseline,
            observations: chunk_observations,
            relationships: chunk_relationships,
        });
    }
    Ok(chunks)
}

fn invalid_delivery(reason: &str) -> AppError {
    AppError::InvalidInput {
        what: "Docker collector delivery",
        reason: reason.to_owned(),
    }
}

fn unique_related_endpoint_count(
    observation: &RawObservation,
    relationships: &[RawRelationshipSpec],
) -> usize {
    let mut related = Vec::<EntityIdentity>::new();
    for relationship in relationships {
        for endpoint in [&relationship.source.identity, &relationship.target.identity] {
            if endpoint != &observation.entity.identity && !related.contains(endpoint) {
                related.push(endpoint.clone());
            }
        }
    }
    related.len()
}

fn append_compose_event(
    windows: &mut BTreeMap<String, ComposeWindow>,
    event: &ParsedDockerEvent,
    observations: &mut Vec<RawObservation>,
) {
    if event.object_kind != DockerObjectKind::Container {
        return;
    }
    let Some(project) = event.compose_project.as_ref() else {
        return;
    };
    if !windows.contains_key(project) && windows.len() >= MAX_COMPOSE_WINDOWS {
        // Compose labels are provider-controlled and can be unbounded. Drop
        // the oldest correlation window when the bounded cache is full; the
        // underlying container event remains durable and no identity is
        // fabricated to represent the omitted correlation.
        if let Some(oldest) = windows
            .iter()
            .min_by_key(|(_, window)| window.started_at)
            .map(|(key, _)| key.clone())
        {
            windows.remove(&oldest);
        }
    }
    let window = windows.entry(project.clone()).or_insert(ComposeWindow {
        started_at: event.occurred_at_ms,
        container_ids: BTreeSet::new(),
        emitted: false,
    });
    if event.occurred_at_ms.saturating_sub(window.started_at) > COMPOSE_WINDOW_MS {
        *window = ComposeWindow {
            started_at: event.occurred_at_ms,
            container_ids: BTreeSet::new(),
            emitted: false,
        };
    }
    if window.emitted {
        return;
    }
    if window.container_ids.len() < MAX_COMPOSE_CONTAINER_IDS {
        window.container_ids.insert(event.actor_id.clone());
    }
    if window.container_ids.len() >= 2 {
        window.emitted = true;
        observations.push(compose_observation(project, event.occurred_at_ms));
    }
}

fn duration_ms(duration: Duration) -> i64 {
    i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
}

fn is_version_too_new(error: &AppError) -> bool {
    matches!(error, AppError::DockerHttpStatus { status: 400 })
}

fn protocol_error(_error: crate::collectors::docker_protocol::DockerProtocolError) -> AppError {
    AppError::Platform {
        operation: "parse a Docker Engine response",
        reason: "the response did not match the bounded Docker schema".to_owned(),
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
            reason: "the Docker collector is not in the canonical catalog".to_owned(),
        })
}
