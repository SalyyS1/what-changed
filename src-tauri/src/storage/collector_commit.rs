//! One collector cycle committed as one durable fact.
//!
//! A collector may discover an event, the entities at both ends of a proven
//! relationship, the relationship itself, and its next baseline in one pass.
//! Splitting those writes would let a crash or a constraint failure advance
//! the baseline without recording the event. This module keeps that entire
//! unit behind the existing [`Sanitized`] storage chokepoint and one SQLite
//! transaction.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;

use super::{
    baseline_repository, config_snapshot_repository, entity_repository, event_repository, to_json,
    Baseline, ConfigSnapshotMutation, SqlContext,
};
use crate::domain::{ids, Entity, EntityType, NormalizedEvent, Relationship, RelationshipKind};
use crate::error::{AppError, AppResult};
use crate::privacy::Sanitized;

/// Maximum event rows one atomic collector command may carry.
pub const MAX_ATOMIC_OBSERVATIONS: usize = 200;

/// Maximum related entity facts one atomic collector command may carry.
pub const MAX_ATOMIC_RELATED_ENTITIES: usize = 400;

/// Maximum graph edges one atomic collector command may carry.
pub const MAX_ATOMIC_RELATIONSHIPS: usize = 400;

/// Config baselines carried by one debounced filesystem batch.
pub const MAX_ATOMIC_CONFIG_SNAPSHOTS: usize = 200;

/// Serialized sanitized config state across one atomic commit.
pub const MAX_ATOMIC_CONFIG_SNAPSHOT_BYTES: usize = 4 * 1024 * 1024;

/// A proven edge whose evidence id will be assigned by the transaction.
///
/// There is intentionally no `event_id` field. Nesting the spec inside an
/// [`AtomicObservation`] binds it to exactly one event, so a caller cannot
/// accidentally attach an edge to an id from another batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipSpec {
    source_id: String,
    target_id: String,
    kind: RelationshipKind,
    recorded_at: i64,
}

impl RelationshipSpec {
    /// Describe one directed, observed relationship.
    pub fn new(
        source_id: impl Into<String>,
        target_id: impl Into<String>,
        kind: RelationshipKind,
        recorded_at: i64,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            target_id: target_id.into(),
            kind,
            recorded_at,
        }
    }
}

/// One normalized event and every graph fact it proved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicObservation {
    event: Sanitized<NormalizedEvent>,
    related_entities: Vec<Sanitized<Entity>>,
    relationships: Vec<RelationshipSpec>,
}

impl AtomicObservation {
    /// Build an observation after normalization and redaction.
    ///
    /// All relationship endpoints must be the event's primary entity or one
    /// of the supplied related entities. Related entity facts must also be
    /// named by the normalized event, keeping timeline and graph data in
    /// agreement.
    pub fn new(
        event: Sanitized<NormalizedEvent>,
        related_entities: Vec<Sanitized<Entity>>,
        relationships: Vec<RelationshipSpec>,
    ) -> AppResult<Self> {
        let observation = Self {
            event,
            related_entities,
            relationships,
        };
        observation.validate()?;
        Ok(observation)
    }

    pub(crate) fn event(&self) -> &Sanitized<NormalizedEvent> {
        &self.event
    }

    fn validate(&self) -> AppResult<()> {
        let event = self.event.get();
        if event.entity_id.is_empty() || event.source_collector.is_empty() {
            return invalid("an event identity or collector is empty");
        }
        if !ids::is_valid_entity_id_for(&event.entity_id, event.entity_type) {
            return invalid("an event id prefix does not match its entity type");
        }

        let mut endpoints = BTreeMap::from([(event.entity_id.as_str(), event.entity_type)]);
        let related_ids: BTreeSet<&str> = event
            .related_entity_ids
            .iter()
            .map(String::as_str)
            .collect();
        for sanitized in &self.related_entities {
            let entity = sanitized.get();
            if entity.id.is_empty() || entity.first_seen > entity.last_seen {
                return invalid("a related entity has an invalid identity or time range");
            }
            if !ids::is_valid_entity_id_for(&entity.id, entity.entity_type) {
                return invalid("a related entity id prefix does not match its entity type");
            }
            if !related_ids.contains(entity.id.as_str()) {
                return invalid("a related entity is not named by its event");
            }
            if endpoints
                .insert(entity.id.as_str(), entity.entity_type)
                .is_some()
            {
                return invalid("an observation contains a duplicate entity identity");
            }
        }

        let mut relationships = BTreeSet::new();
        for relationship in &self.relationships {
            if relationship.source_id.is_empty()
                || relationship.target_id.is_empty()
                || relationship.source_id == relationship.target_id
            {
                return invalid("a relationship has invalid endpoints");
            }
            let Some(source_type) = endpoints.get(relationship.source_id.as_str()).copied() else {
                return invalid("a relationship endpoint has no entity fact");
            };
            let Some(target_type) = endpoints.get(relationship.target_id.as_str()).copied() else {
                return invalid("a relationship endpoint has no entity fact");
            };
            if !valid_direction(relationship.kind, source_type, target_type) {
                return invalid("a relationship kind does not match its endpoint types");
            }
            if relationship.source_id != event.entity_id
                && relationship.target_id != event.entity_id
            {
                return invalid("a relationship does not include its evidence event entity");
            }
            if !relationships.insert((
                relationship.source_id.as_str(),
                relationship.target_id.as_str(),
                relationship.kind,
            )) {
                return invalid("an observation contains a duplicate relationship");
            }
            if relationship.recorded_at != event.timestamp {
                return invalid("a relationship timestamp does not match its evidence event");
            }
        }
        Ok(())
    }
}

fn valid_direction(kind: RelationshipKind, source: EntityType, target: EntityType) -> bool {
    match kind {
        RelationshipKind::ListensOn => source == EntityType::Process && target == EntityType::Port,
        RelationshipKind::OpenedBy => source == EntityType::Port && target == EntityType::Process,
        RelationshipKind::Provides => {
            source == EntityType::Application && target == EntityType::Service
        }
        RelationshipKind::Publishes => {
            source == EntityType::Container && target == EntityType::Port
        }
        // Phase 10 has direct evidence paths only for a file configuring a
        // process, listening port, or Windows service. Widen this deliberately
        // when another collector can prove a new target class.
        RelationshipKind::ConfigOf => {
            source == EntityType::File
                && matches!(
                    target,
                    EntityType::Process | EntityType::Port | EntityType::Service
                )
        }
    }
}

/// Every durable write produced by one collector cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicCollectorCommit {
    collector_id: String,
    observations: Vec<AtomicObservation>,
    baseline: Option<Sanitized<Baseline>>,
    config_snapshots: Vec<ConfigSnapshotMutation>,
}

impl AtomicCollectorCommit {
    /// Construct one all-or-nothing collector commit.
    ///
    /// A baseline-only first snapshot is valid. An entirely empty commit is
    /// rejected, as are batches that mix collectors or advance another
    /// collector's baseline.
    pub fn new(
        observations: Vec<AtomicObservation>,
        baseline: Option<Sanitized<Baseline>>,
    ) -> AppResult<Self> {
        let collector_id = observations
            .first()
            .map(|observation| observation.event.get().source_collector.clone())
            .or_else(|| {
                baseline
                    .as_ref()
                    .map(|value| value.get().collector_id.clone())
            })
            .unwrap_or_default();
        let commit = Self {
            collector_id,
            observations,
            baseline,
            config_snapshots: Vec::new(),
        };
        commit.validate()?;
        Ok(commit)
    }

    /// Construct a commit that atomically advances per-file config baselines.
    pub fn with_config_snapshots(
        collector_id: impl Into<String>,
        observations: Vec<AtomicObservation>,
        baseline: Option<Sanitized<Baseline>>,
        config_snapshots: Vec<ConfigSnapshotMutation>,
    ) -> AppResult<Self> {
        let commit = Self {
            collector_id: collector_id.into(),
            observations,
            baseline,
            config_snapshots,
        };
        commit.validate()?;
        Ok(commit)
    }

    /// Number of event rows this command admits to the bounded queue.
    pub fn event_count(&self) -> usize {
        self.observations.len()
    }

    /// Collector that owns this cycle, including a baseline-only cycle.
    pub fn collector_id(&self) -> &str {
        &self.collector_id
    }

    pub(crate) fn observations(&self) -> &[AtomicObservation] {
        &self.observations
    }

    fn validate(&self) -> AppResult<()> {
        if self.observations.is_empty()
            && self.baseline.is_none()
            && self.config_snapshots.is_empty()
        {
            return invalid("the commit contains no durable collector state");
        }
        if self.collector_id.is_empty() {
            return invalid("the commit collector is empty");
        }
        if self.observations.len() > MAX_ATOMIC_OBSERVATIONS {
            return invalid("the commit contains too many observations");
        }

        let mut collector: Option<&str> = None;
        let mut relationships = BTreeSet::new();
        let mut related_entity_count = 0usize;
        let mut relationship_count = 0usize;
        let mut latest_event_at: Option<i64> = None;
        for observation in &self.observations {
            related_entity_count =
                related_entity_count.saturating_add(observation.related_entities.len());
            relationship_count = relationship_count.saturating_add(observation.relationships.len());
            if related_entity_count > MAX_ATOMIC_RELATED_ENTITIES {
                return invalid("the commit contains too many related entities");
            }
            if relationship_count > MAX_ATOMIC_RELATIONSHIPS {
                return invalid("the commit contains too many relationships");
            }
            observation.validate()?;
            latest_event_at = Some(
                latest_event_at.map_or(observation.event.get().timestamp, |latest| {
                    latest.max(observation.event.get().timestamp)
                }),
            );
            let found = observation.event.get().source_collector.as_str();
            if found != self.collector_id {
                return invalid("an observation belongs to another collector");
            }
            match collector {
                Some(expected) if expected != found => {
                    return invalid("one commit mixes more than one collector");
                }
                None => collector = Some(found),
                Some(_) => {}
            }
            for relationship in &observation.relationships {
                if !relationships.insert((
                    relationship.source_id.as_str(),
                    relationship.target_id.as_str(),
                    relationship.kind,
                )) {
                    return invalid("one commit assigns the same relationship to two events");
                }
            }
        }
        if let Some(baseline) = &self.baseline {
            let baseline = baseline.get();
            if baseline.collector_id.is_empty() || baseline.entity_kind.is_empty() {
                return invalid("the baseline identity is empty");
            }
            if !baseline.truncated
                && to_json(&baseline.snapshot)?.len() > baseline_repository::MAX_SNAPSHOT_BYTES
            {
                return invalid("the baseline snapshot is larger than the atomic commit limit");
            }
            if collector.is_some_and(|expected| expected != baseline.collector_id) {
                return invalid("the baseline belongs to another collector");
            }
            if baseline.collector_id != self.collector_id {
                return invalid("the baseline belongs to another collector");
            }
            if latest_event_at.is_some_and(|latest| baseline.captured_at < latest) {
                return invalid("the baseline predates an event in its collector commit");
            }
        }
        if !self.config_snapshots.is_empty() && self.collector_id != "files" {
            return invalid("config snapshots belong to the files collector");
        }
        if self.config_snapshots.len() > MAX_ATOMIC_CONFIG_SNAPSHOTS {
            return invalid("the commit contains too many config snapshots");
        }
        let mut config_bytes = 0usize;
        for mutation in &self.config_snapshots {
            if !mutation.validate() {
                return invalid("a config snapshot mutation is invalid");
            }
            config_bytes = config_bytes.saturating_add(mutation.encoded_bytes()?);
            if config_bytes > MAX_ATOMIC_CONFIG_SNAPSHOT_BYTES {
                return invalid("the config snapshots exceed the atomic commit limit");
            }
        }
        Ok(())
    }
}

/// Durable event ids, in the same order as the submitted observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorCommitReceipt {
    pub event_ids: Vec<i64>,
}

/// Execute a previously validated commit on the writer connection.
pub(crate) fn insert(
    conn: &Connection,
    commit: &AtomicCollectorCommit,
) -> AppResult<CollectorCommitReceipt> {
    // Validate again at the trust boundary. Fields are private today, but
    // keeping the repository defensive prevents a future constructor from
    // weakening the transaction silently.
    commit.validate()?;
    let tx = conn
        .unchecked_transaction()
        .during("begin a collector commit")?;
    let mut event_ids = Vec::with_capacity(commit.observations.len());

    for observation in &commit.observations {
        let event_id = event_repository::insert_or_merge(&tx, observation.event())?;
        entity_repository::touch(&tx, observation.event.get())?;
        for entity in &observation.related_entities {
            entity_repository::upsert_sanitized(&tx, entity)?;
        }
        for spec in &observation.relationships {
            entity_repository::insert_relationship(
                &tx,
                &Relationship {
                    source_id: spec.source_id.clone(),
                    target_id: spec.target_id.clone(),
                    kind: spec.kind,
                    event_id: Some(event_id),
                    recorded_at: spec.recorded_at,
                },
            )?;
        }
        event_ids.push(event_id);
    }

    for (observation, event_id) in commit.observations.iter().zip(&event_ids) {
        event_repository::reconcile_inserted_event(&tx, observation.event().get(), *event_id)?;
    }

    if let Some(baseline) = &commit.baseline {
        baseline_repository::store_monotonic(&tx, baseline)?;
    }
    for mutation in &commit.config_snapshots {
        config_snapshot_repository::apply(&tx, mutation)?;
    }
    tx.commit().during("commit a collector transaction")?;
    Ok(CollectorCommitReceipt { event_ids })
}

fn invalid<T>(reason: &'static str) -> AppResult<T> {
    Err(AppError::InvalidInput {
        what: "collector commit",
        reason: reason.to_owned(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::domain::{Category, EntityType};
    use crate::privacy::Sanitizer;

    fn event(source: &str) -> Sanitized<NormalizedEvent> {
        let mut event = crate::testing::synthetic_event(1, 123);
        event.source_collector = source.to_owned();
        event.category = Category::Process;
        event.entity_type = EntityType::Process;
        event.entity_id = "process:0123456789abcdef".to_owned();
        event.related_entity_ids = vec!["port:tcp:127.0.0.1:3000".to_owned()];
        Sanitizer::new().sanitize(event).0
    }

    fn endpoint(id: &str) -> Sanitized<Entity> {
        Sanitizer::new()
            .sanitize(Entity {
                id: id.to_owned(),
                entity_type: EntityType::Port,
                display_name: "localhost:3000".to_owned(),
                first_seen: 123,
                last_seen: 123,
                metadata: None,
            })
            .0
    }

    fn bounded_observation(
        index: usize,
        related_count: usize,
        bidirectional_edges: bool,
    ) -> AtomicObservation {
        let timestamp = 1_000 + index as i64;
        let process_id = format!("process:{index:016x}");
        let port_ids = (0..related_count)
            .map(|offset| {
                format!(
                    "port:tcp:127.0.0.1:{}",
                    1_000 + index.saturating_mul(3) + offset
                )
            })
            .collect::<Vec<_>>();

        let mut event = crate::testing::synthetic_event(index as u64 + 1, timestamp);
        event.source_collector = "process".to_owned();
        event.category = Category::Process;
        event.entity_type = EntityType::Process;
        event.entity_id.clone_from(&process_id);
        event.related_entity_ids.clone_from(&port_ids);
        let event = Sanitizer::new().sanitize(event).0;

        let related_entities = port_ids
            .iter()
            .map(|id| {
                Sanitizer::new()
                    .sanitize(Entity {
                        id: id.clone(),
                        entity_type: EntityType::Port,
                        display_name: id.clone(),
                        first_seen: timestamp,
                        last_seen: timestamp,
                        metadata: None,
                    })
                    .0
            })
            .collect::<Vec<_>>();
        let relationships = if bidirectional_edges {
            port_ids
                .iter()
                .flat_map(|port_id| {
                    [
                        RelationshipSpec::new(
                            process_id.clone(),
                            port_id.clone(),
                            RelationshipKind::ListensOn,
                            timestamp,
                        ),
                        RelationshipSpec::new(
                            port_id.clone(),
                            process_id.clone(),
                            RelationshipKind::OpenedBy,
                            timestamp,
                        ),
                    ]
                })
                .collect()
        } else {
            Vec::new()
        };

        AtomicObservation::new(event, related_entities, relationships).unwrap()
    }

    #[test]
    fn relationship_mapping_is_structural_and_endpoints_are_checked() {
        let relationship = RelationshipSpec::new(
            "process:0123456789abcdef",
            "port:tcp:127.0.0.1:3000",
            RelationshipKind::ListensOn,
            123,
        );
        assert!(AtomicObservation::new(
            event("process"),
            vec![endpoint("port:tcp:127.0.0.1:3000")],
            vec![relationship]
        )
        .is_ok());

        let missing = RelationshipSpec::new(
            "process:0123456789abcdef",
            "port:tcp:127.0.0.1:9999",
            RelationshipKind::ListensOn,
            123,
        );
        assert!(AtomicObservation::new(
            event("process"),
            vec![endpoint("port:tcp:127.0.0.1:3000")],
            vec![missing]
        )
        .is_err());
    }

    #[test]
    fn entity_prefixes_and_relationship_direction_are_checked() {
        let wrong_direction = RelationshipSpec::new(
            "port:tcp:127.0.0.1:3000",
            "process:0123456789abcdef",
            RelationshipKind::ListensOn,
            123,
        );
        assert!(AtomicObservation::new(
            event("process"),
            vec![endpoint("port:tcp:127.0.0.1:3000")],
            vec![wrong_direction]
        )
        .is_err());

        let mut wrong_type = endpoint("port:tcp:127.0.0.1:3000").get().clone();
        wrong_type.entity_type = EntityType::Process;
        let wrong_type = Sanitizer::new().sanitize(wrong_type).0;
        assert!(AtomicObservation::new(event("process"), vec![wrong_type], Vec::new()).is_err());

        let mut token_id = event("process").get().clone();
        token_id.entity_id = "process:sk-abcdefghijklmnopqrstuvwx".to_owned();
        let token_id = Sanitizer::new().sanitize(token_id).0;
        assert!(AtomicObservation::new(token_id, Vec::new(), Vec::new()).is_err());

        let mut file_event = event("files").get().clone();
        file_event.entity_type = EntityType::File;
        file_event.entity_id = "file:0123456789abcdef".to_owned();
        file_event.related_entity_ids = vec!["driver:PCI\\VEN_10DE".to_owned()];
        let file_event = Sanitizer::new().sanitize(file_event).0;
        let driver = Sanitizer::new()
            .sanitize(Entity {
                id: "driver:PCI\\VEN_10DE".to_owned(),
                entity_type: EntityType::Driver,
                display_name: "Display adapter".to_owned(),
                first_seen: 123,
                last_seen: 123,
                metadata: None,
            })
            .0;
        let unsupported_config_edge = RelationshipSpec::new(
            "file:0123456789abcdef",
            "driver:PCI\\VEN_10DE",
            RelationshipKind::ConfigOf,
            123,
        );
        assert!(
            AtomicObservation::new(file_event, vec![driver], vec![unsupported_config_edge])
                .is_err()
        );
    }

    #[test]
    fn a_commit_cannot_mix_event_and_baseline_owners() {
        let observation = AtomicObservation::new(event("process"), Vec::new(), Vec::new()).unwrap();
        let baseline = Sanitizer::new()
            .sanitize(Baseline {
                collector_id: "listeners".to_owned(),
                entity_kind: "port".to_owned(),
                snapshot: serde_json::json!({}),
                captured_at: 123,
                truncated: false,
            })
            .0;
        assert!(AtomicCollectorCommit::new(vec![observation], Some(baseline)).is_err());
    }

    #[test]
    fn one_commit_cannot_assign_the_same_edge_to_two_evidence_events() {
        let relationship = RelationshipSpec::new(
            "process:0123456789abcdef",
            "port:tcp:127.0.0.1:3000",
            RelationshipKind::ListensOn,
            123,
        );
        let observation = AtomicObservation::new(
            event("process"),
            vec![endpoint("port:tcp:127.0.0.1:3000")],
            vec![relationship],
        )
        .unwrap();
        assert!(AtomicCollectorCommit::new(vec![observation.clone(), observation], None).is_err());
    }

    #[test]
    fn atomic_command_weight_limits_accept_boundaries_and_reject_one_more() {
        let mut observations = (0..MAX_ATOMIC_OBSERVATIONS)
            .map(|index| bounded_observation(index, 0, false))
            .collect::<Vec<_>>();
        assert!(AtomicCollectorCommit::new(observations.clone(), None).is_ok());
        observations.push(bounded_observation(MAX_ATOMIC_OBSERVATIONS, 0, false));
        assert!(AtomicCollectorCommit::new(observations, None).is_err());

        assert!(AtomicCollectorCommit::new(
            vec![bounded_observation(0, MAX_ATOMIC_RELATED_ENTITIES, false)],
            None,
        )
        .is_ok());
        assert!(AtomicCollectorCommit::new(
            vec![bounded_observation(
                0,
                MAX_ATOMIC_RELATED_ENTITIES + 1,
                false,
            )],
            None,
        )
        .is_err());

        let mut edge_observations = (0..MAX_ATOMIC_OBSERVATIONS)
            .map(|index| bounded_observation(index, 1, true))
            .collect::<Vec<_>>();
        assert!(AtomicCollectorCommit::new(edge_observations.clone(), None).is_ok());
        edge_observations[0] = bounded_observation(0, 2, true);
        assert!(AtomicCollectorCommit::new(edge_observations, None).is_err());
    }

    #[test]
    fn baseline_size_and_event_time_are_bounded_before_admission() {
        let exact_limit =
            Sanitizer::new()
                .sanitize(Baseline {
                    collector_id: "process".to_owned(),
                    entity_kind: "weight".to_owned(),
                    snapshot: serde_json::json!(
                        "x".repeat(baseline_repository::MAX_SNAPSHOT_BYTES - 2)
                    ),
                    captured_at: 123,
                    truncated: false,
                })
                .0;
        assert_eq!(
            to_json(&exact_limit.get().snapshot).unwrap().len(),
            baseline_repository::MAX_SNAPSHOT_BYTES
        );
        assert!(AtomicCollectorCommit::new(Vec::new(), Some(exact_limit)).is_ok());

        let oversized =
            Sanitizer::new()
                .sanitize(Baseline {
                    collector_id: "process".to_owned(),
                    entity_kind: "weight".to_owned(),
                    snapshot: serde_json::json!(
                        "x".repeat(baseline_repository::MAX_SNAPSHOT_BYTES - 1)
                    ),
                    captured_at: 123,
                    truncated: false,
                })
                .0;
        assert!(AtomicCollectorCommit::new(Vec::new(), Some(oversized)).is_err());

        let observation = AtomicObservation::new(event("process"), Vec::new(), Vec::new()).unwrap();
        let older_baseline = Sanitizer::new()
            .sanitize(Baseline {
                collector_id: "process".to_owned(),
                entity_kind: "weight".to_owned(),
                snapshot: serde_json::json!({ "marker": "explicit" }),
                captured_at: 122,
                truncated: true,
            })
            .0;
        assert!(AtomicCollectorCommit::new(vec![observation], Some(older_baseline)).is_err());
    }
}
