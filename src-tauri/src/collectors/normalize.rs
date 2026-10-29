//! Converts platform-neutral observations into redacted storage facts.

use std::collections::{BTreeMap, BTreeSet};

use crate::collectors::{
    EntityIdentity, RawBaseline, RawConfigSnapshotMutation, RawEntity, RawObservation,
    RawRelationshipSpec,
};
use crate::domain::{classify, ids, Entity, EntityType, NormalizedEvent};
use crate::error::{AppError, AppResult};
use crate::privacy::{Sanitized, Sanitizer};
use crate::storage::{
    AtomicCollectorCommit, AtomicObservation, Baseline, ConfigSnapshot, ConfigSnapshotMutation,
    RelationshipSpec,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct Normalizer {
    sanitizer: Sanitizer,
}

impl Normalizer {
    pub fn new() -> Self {
        Self {
            sanitizer: Sanitizer::new(),
        }
    }

    pub fn normalize(&self, raw: RawObservation) -> Sanitized<NormalizedEvent> {
        self.sanitizer.sanitize(self.normalize_event(raw)).0
    }

    fn normalize_event(&self, raw: RawObservation) -> NormalizedEvent {
        let (entity_id, entity_type) = entity_id(&raw.entity.identity);
        let mut event = NormalizedEvent {
            timestamp: raw.occurred_at,
            category: raw.kind.category,
            event_type: raw.kind.event_type,
            entity_type,
            entity_id,
            entity_display_name: raw.entity.display_name,
            severity: raw.kind.severity,
            importance: crate::domain::Importance::Informational,
            source_collector: raw.collector_id.to_owned(),
            before: raw.before,
            after: raw.after,
            field: raw.field,
            metadata: raw.details,
            related_entity_ids: raw.related_entity_ids,
            user_friendly_summary: raw.kind.user_friendly_summary,
            technical_summary: raw.kind.technical_summary,
            confidence: raw.kind.confidence,
            sensitive: false,
            occurred_at_is_uncertain: raw.occurred_at_is_uncertain,
        };
        event.importance = classify(&event, &raw.kind.classifier);
        event
    }

    pub fn normalize_all(
        &self,
        observations: Vec<RawObservation>,
    ) -> Vec<Sanitized<NormalizedEvent>> {
        observations
            .into_iter()
            .map(|raw| self.normalize(raw))
            .collect()
    }

    /// Prepare raw atomic collector output for the storage transaction.
    ///
    /// Relationship evidence is resolved by observation index here so storage
    /// receives normalized ids, redacted endpoint facts and no collector-layer
    /// identity material.
    #[cfg(test)]
    pub(crate) fn prepare_atomic(
        &self,
        collector_id: &'static str,
        observations: Vec<RawObservation>,
        relationships: Vec<RawRelationshipSpec>,
        baseline: Option<RawBaseline>,
    ) -> AppResult<AtomicCollectorCommit> {
        self.prepare_atomic_with_config(
            collector_id,
            observations,
            relationships,
            baseline,
            Vec::new(),
        )
    }

    pub(crate) fn prepare_atomic_with_config(
        &self,
        collector_id: &'static str,
        observations: Vec<RawObservation>,
        relationships: Vec<RawRelationshipSpec>,
        baseline: Option<RawBaseline>,
        config_snapshots: Vec<RawConfigSnapshotMutation>,
    ) -> AppResult<AtomicCollectorCommit> {
        if observations
            .iter()
            .any(|observation| observation.collector_id != collector_id)
        {
            return Err(atomic_input_error(
                "an observation belongs to another collector",
            ));
        }

        let mut relationships_by_observation: Vec<Vec<RawRelationshipSpec>> =
            (0..observations.len()).map(|_| Vec::new()).collect();
        for relationship in relationships {
            let Some(bucket) =
                relationships_by_observation.get_mut(relationship.evidence_observation)
            else {
                return Err(atomic_input_error(
                    "a relationship names a missing evidence observation",
                ));
            };
            bucket.push(relationship);
        }

        let observations = observations
            .into_iter()
            .zip(relationships_by_observation)
            .map(|(raw, relationships)| self.prepare_observation(raw, relationships))
            .collect::<AppResult<Vec<_>>>()?;
        let baseline = baseline.map(|raw| {
            let truncated = raw
                .snapshot
                .get("truncated")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
                && raw
                    .snapshot
                    .get("bytes")
                    .and_then(serde_json::Value::as_u64)
                    .is_some();
            self.sanitizer
                .sanitize(Baseline {
                    collector_id: collector_id.to_owned(),
                    entity_kind: raw.entity_kind,
                    snapshot: raw.snapshot,
                    captured_at: raw.captured_at,
                    truncated,
                })
                .0
        });
        let config_snapshots = config_snapshots
            .into_iter()
            .map(|mutation| match mutation {
                RawConfigSnapshotMutation::Upsert {
                    path,
                    keys,
                    captured_at,
                    format,
                } => {
                    let keys = serde_json::to_value(keys).map_err(|_| {
                        atomic_input_error("config snapshot keys could not be encoded")
                    })?;
                    let snapshot = self
                        .sanitizer
                        .sanitize(ConfigSnapshot {
                            path,
                            keys,
                            captured_at,
                            format,
                        })
                        .0;
                    Ok(ConfigSnapshotMutation::upsert(snapshot))
                }
                RawConfigSnapshotMutation::Delete { path } => {
                    Ok(ConfigSnapshotMutation::delete(path))
                }
            })
            .collect::<AppResult<Vec<_>>>()?;
        AtomicCollectorCommit::with_config_snapshots(
            collector_id,
            observations,
            baseline,
            config_snapshots,
        )
    }

    fn prepare_observation(
        &self,
        raw: RawObservation,
        relationships: Vec<RawRelationshipSpec>,
    ) -> AppResult<AtomicObservation> {
        let mut event = self.normalize_event(raw);
        let mut related_ids: BTreeSet<String> = std::mem::take(&mut event.related_entity_ids)
            .into_iter()
            .collect();
        related_ids.remove(&event.entity_id);
        let mut related_entities = BTreeMap::<String, Entity>::new();
        let mut normalized_relationships = Vec::with_capacity(relationships.len());

        for relationship in relationships {
            let source = endpoint(relationship.source, event.timestamp);
            let target = endpoint(relationship.target, event.timestamp);
            if source.id != event.entity_id && target.id != event.entity_id {
                return Err(atomic_input_error(
                    "a relationship does not include its evidence observation entity",
                ));
            }
            add_related_endpoint(
                &event,
                source.clone(),
                &mut related_ids,
                &mut related_entities,
            )?;
            add_related_endpoint(
                &event,
                target.clone(),
                &mut related_ids,
                &mut related_entities,
            )?;
            normalized_relationships.push(RelationshipSpec::new(
                source.id,
                target.id,
                relationship.kind,
                event.timestamp,
            ));
        }
        event.related_entity_ids = related_ids.into_iter().collect();
        let event = self.sanitizer.sanitize(event).0;
        let related_entities = related_entities
            .into_values()
            .map(|entity| self.sanitizer.sanitize(entity).0)
            .collect();
        AtomicObservation::new(event, related_entities, normalized_relationships)
    }
}

fn endpoint(raw: RawEntity, timestamp: i64) -> Entity {
    let (id, entity_type) = entity_id(&raw.identity);
    Entity {
        id,
        entity_type,
        display_name: raw.display_name,
        first_seen: timestamp,
        last_seen: timestamp,
        metadata: None,
    }
}

fn add_related_endpoint(
    event: &NormalizedEvent,
    entity: Entity,
    related_ids: &mut BTreeSet<String>,
    related_entities: &mut BTreeMap<String, Entity>,
) -> AppResult<()> {
    if entity.id == event.entity_id {
        if entity.entity_type != event.entity_type {
            return Err(atomic_input_error(
                "a primary relationship endpoint has the wrong entity type",
            ));
        }
        return Ok(());
    }
    related_ids.insert(entity.id.clone());
    if let Some(existing) = related_entities.get(&entity.id) {
        if existing.entity_type != entity.entity_type {
            return Err(atomic_input_error(
                "one relationship endpoint has conflicting entity types",
            ));
        }
    } else {
        related_entities.insert(entity.id.clone(), entity);
    }
    Ok(())
}

fn atomic_input_error(reason: &'static str) -> AppError {
    AppError::InvalidInput {
        what: "collector atomic snapshot",
        reason: reason.to_owned(),
    }
}

fn entity_id(identity: &EntityIdentity) -> (String, EntityType) {
    match identity {
        EntityIdentity::Process { executable } => (ids::process(executable), EntityType::Process),
        EntityIdentity::ProcessById { id } => (id.as_str().to_owned(), EntityType::Process),
        EntityIdentity::Port {
            protocol,
            address,
            port,
        } => (ids::port(*protocol, address, *port), EntityType::Port),
        EntityIdentity::Service { name } => (ids::service(name), EntityType::Service),
        EntityIdentity::Application { hive, registry_key } => {
            (ids::app(hive, registry_key), EntityType::Application)
        }
        EntityIdentity::Driver { device_instance_id } => {
            (ids::driver(device_instance_id), EntityType::Driver)
        }
        EntityIdentity::File { path } => (ids::file(path), EntityType::File),
        EntityIdentity::Adapter { guid } => (ids::adapter(guid), EntityType::Adapter),
        EntityIdentity::Container { docker_id } => {
            (ids::container(docker_id), EntityType::Container)
        }
        EntityIdentity::Image { repository_tag } => (ids::image(repository_tag), EntityType::Image),
        EntityIdentity::DockerVolume { name } => {
            (ids::docker_volume(name), EntityType::DockerVolume)
        }
        EntityIdentity::DockerNetwork { name } => {
            (ids::docker_network(name), EntityType::DockerNetwork)
        }
        EntityIdentity::Project { path } => (ids::project(path), EntityType::Project),
        EntityIdentity::Update { id } => (ids::update(id), EntityType::Update),
        EntityIdentity::Task { path } => (ids::task(path), EntityType::Task),
        EntityIdentity::System => (ids::system(), EntityType::System),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::collectors::{ObservationKind, RawRelationshipSpec};
    use crate::domain::{
        Category, ClassifierContext, Confidence, EventType, RelationshipKind, Severity,
    };

    fn process(name: &str) -> RawEntity {
        RawEntity {
            identity: EntityIdentity::Process {
                executable: format!("{name}.exe"),
            },
            display_name: name.to_owned(),
        }
    }

    fn port() -> RawEntity {
        RawEntity {
            identity: EntityIdentity::Port {
                protocol: ids::Protocol::Tcp,
                address: "127.0.0.1".to_owned(),
                port: 8_080,
            },
            display_name: "listener".to_owned(),
        }
    }

    fn observation(collector_id: &'static str) -> RawObservation {
        RawObservation {
            collector_id,
            kind: ObservationKind {
                category: Category::Process,
                event_type: EventType::Started,
                severity: Severity::Info,
                confidence: Confidence::High,
                user_friendly_summary: "process started".to_owned(),
                technical_summary: "process started".to_owned(),
                classifier: ClassifierContext::default(),
            },
            entity: process("primary"),
            before: None,
            after: Some(json!({ "token": "secret-value" })),
            field: None,
            occurred_at: 1_700_000_000_000,
            details: None,
            related_entity_ids: Vec::new(),
            occurred_at_is_uncertain: false,
        }
    }

    #[test]
    fn atomic_input_rejects_foreign_collectors_and_missing_evidence() {
        let foreign = Normalizer::new()
            .prepare_atomic("expected", vec![observation("foreign")], Vec::new(), None)
            .unwrap_err();
        assert!(foreign.to_string().contains("another collector"));

        let missing = Normalizer::new()
            .prepare_atomic(
                "expected",
                vec![observation("expected")],
                vec![RawRelationshipSpec {
                    evidence_observation: 1,
                    source: process("primary"),
                    target: port(),
                    kind: RelationshipKind::ListensOn,
                }],
                None,
            )
            .unwrap_err();
        assert!(missing.to_string().contains("missing evidence"));
    }

    #[test]
    fn atomic_relationship_must_include_its_primary_observation_entity() {
        let error = Normalizer::new()
            .prepare_atomic(
                "expected",
                vec![observation("expected")],
                vec![RawRelationshipSpec {
                    evidence_observation: 0,
                    source: process("different"),
                    target: port(),
                    kind: RelationshipKind::ListensOn,
                }],
                None,
            )
            .unwrap_err();
        assert!(error.to_string().contains("does not include"));
    }
}
