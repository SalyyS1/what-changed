//! Pure Docker baseline diffing and event-to-domain mapping.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::collectors::docker_protocol::{
    DockerAction, DockerContainerSummary, DockerImageSummary, DockerObjectKind, ParsedDockerEvent,
    PortPublication, DOCKER_PROTOCOL_SCHEMA_VERSION,
};
use crate::collectors::{
    EntityIdentity, ObservationKind, RawEntity, RawObservation, RawRelationshipSpec,
};
use crate::domain::ids::Protocol;
use crate::domain::{
    Category, ClassifierContext, Confidence, EventType, RelationshipKind, Severity,
};

use super::ID;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(super) struct DockerBaseline {
    pub(super) established: bool,
    pub(super) daemon_version: Option<String>,
    containers: BTreeMap<String, ContainerState>,
    images: BTreeMap<String, ImageState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ContainerState {
    id: String,
    name: String,
    image: String,
    state: String,
    compose_project: Option<String>,
    #[serde(default)]
    event_exit_code_trusted: bool,
    ports: Vec<StoredPort>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredPort {
    host_address: String,
    host_port: u16,
    container_port: u16,
    protocol: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ImageState {
    id: String,
    repository_tags: BTreeSet<String>,
}

pub(super) fn baseline_from(
    containers: Vec<DockerContainerSummary>,
    images: Vec<DockerImageSummary>,
    daemon_version: String,
) -> DockerBaseline {
    let mut result = DockerBaseline {
        established: true,
        daemon_version: Some(scrub_text(daemon_version)),
        ..DockerBaseline::default()
    };
    for container in containers {
        let name = container
            .names
            .first()
            .map(|name| name.trim_start_matches('/').to_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| container.id.clone());
        result.containers.insert(
            container.id.clone(),
            ContainerState {
                id: container.id,
                name: scrub_text(name),
                image: scrub_text(container.image),
                state: scrub_text(container.state),
                compose_project: container.compose_project.map(scrub_text),
                event_exit_code_trusted: !container.exit_code_label_present,
                ports: container
                    .published_ports
                    .into_iter()
                    .map(StoredPort::from)
                    .collect(),
            },
        );
    }
    for image in images {
        let repository_tags = image
            .repository_tags
            .into_iter()
            .filter(|tag| tag != "<none>:<none>")
            .map(scrub_text)
            .collect();
        result.images.insert(
            image.id.clone(),
            ImageState {
                id: image.id,
                repository_tags,
            },
        );
    }
    result
}

impl DockerBaseline {
    /// Qualify flattened event attributes against the typed container list.
    /// Moby mixes user labels into Actor.Attributes, so only baseline ports and
    /// an exit code whose reserved key is not shadowed may reach domain data.
    pub(super) fn qualify_event(&self, event: &ParsedDockerEvent) -> ParsedDockerEvent {
        let mut qualified = event.clone();
        qualified.published_ports.clear();
        qualified.exit_code = None;
        if event.object_kind != DockerObjectKind::Container {
            return qualified;
        }
        let Some(container) = self.containers.get(&event.actor_id) else {
            return qualified;
        };
        qualified.published_ports = container
            .ports
            .iter()
            .map(|port| PortPublication {
                host_address: port.host_address.clone(),
                host_port: port.host_port,
                container_port: port.container_port,
                protocol: port.protocol.clone(),
            })
            .collect();
        if container.event_exit_code_trusted {
            qualified.exit_code = event.exit_code;
        }
        qualified
    }

    pub(super) fn has_container_instance(&self, event: &ParsedDockerEvent) -> bool {
        event.object_kind == DockerObjectKind::Container
            && self.containers.contains_key(&event.actor_id)
    }

    /// Whether a stream event is already reflected by a typed snapshot taken
    /// after the event was queued. The event stream is opened before the list
    /// endpoints to close the loss window; this reconciliation prevents the
    /// overlap from duplicating ordinary lifecycle events.
    pub(super) fn reconciles_event(&self, event: &ParsedDockerEvent) -> bool {
        match event.object_kind {
            DockerObjectKind::Container => {
                let Some(container) = self.containers.get(&event.actor_id) else {
                    return event.action == DockerAction::Removed;
                };
                match event.action {
                    DockerAction::Created => true,
                    DockerAction::Started => container.state == "running",
                    DockerAction::Stopped | DockerAction::Died => container.state == "exited",
                    DockerAction::Removed => false,
                    DockerAction::Pulled => false,
                }
            }
            DockerObjectKind::Image => {
                let tag = scrub_text(event.display_name.clone());
                let contains = self
                    .images
                    .get(&event.actor_id)
                    .is_some_and(|image| image.repository_tags.contains(&tag));
                match event.action {
                    DockerAction::Pulled => contains,
                    DockerAction::Removed => !contains,
                    _ => false,
                }
            }
            // The list endpoints do not provide authoritative volume/network
            // state, so retain those events even when they overlap the list.
            DockerObjectKind::Volume | DockerObjectKind::Network => false,
        }
    }
}

impl From<PortPublication> for StoredPort {
    fn from(value: PortPublication) -> Self {
        Self {
            host_address: value.host_address,
            host_port: value.host_port,
            container_port: value.container_port,
            protocol: value.protocol,
        }
    }
}

pub(super) fn apply_event(baseline: &mut DockerBaseline, event: &ParsedDockerEvent) {
    match event.object_kind {
        DockerObjectKind::Container => {
            let key = event.actor_id.clone();
            if event.action == DockerAction::Removed {
                baseline.containers.remove(&key);
            } else {
                if event.action == DockerAction::Created {
                    baseline.containers.retain(|id, state| {
                        id == &event.actor_id || state.name != event.display_name
                    });
                }
                let previous = baseline.containers.get(&key).cloned();
                let same_instance = previous.as_ref();
                let state = match event.action {
                    DockerAction::Started => "running",
                    DockerAction::Stopped | DockerAction::Died => "exited",
                    DockerAction::Created => "created",
                    DockerAction::Removed | DockerAction::Pulled => "unknown",
                };
                baseline.containers.insert(
                    key.clone(),
                    ContainerState {
                        id: event.actor_id.clone(),
                        name: scrub_text(event.display_name.clone()),
                        image: event
                            .image
                            .clone()
                            .or_else(|| same_instance.map(|state| state.image.clone()))
                            .map(scrub_text)
                            .unwrap_or_default(),
                        state: state.to_owned(),
                        compose_project: event.compose_project.clone().map(scrub_text).or_else(
                            || same_instance.and_then(|state| state.compose_project.clone()),
                        ),
                        event_exit_code_trusted: if event.action == DockerAction::Created {
                            !event.exit_code_label_present
                        } else {
                            same_instance.is_some_and(|state| state.event_exit_code_trusted)
                        },
                        // `qualify_event` supplies only typed baseline ports.
                        // A recreated name never inherits its predecessor's
                        // mappings because `same_instance` also checks the ID.
                        ports: if event.published_ports.is_empty() {
                            same_instance.map_or_else(Vec::new, |state| state.ports.clone())
                        } else {
                            event
                                .published_ports
                                .iter()
                                .cloned()
                                .map(StoredPort::from)
                                .collect()
                        },
                    },
                );
            }
        }
        DockerObjectKind::Image => {
            if event.action == DockerAction::Removed {
                if let Some(image) = baseline.images.get_mut(&event.actor_id) {
                    image
                        .repository_tags
                        .remove(&scrub_text(event.display_name.clone()));
                    if image.repository_tags.is_empty() {
                        baseline.images.remove(&event.actor_id);
                    }
                }
            } else {
                let image = baseline
                    .images
                    .entry(event.actor_id.clone())
                    .or_insert_with(|| ImageState {
                        id: event.actor_id.clone(),
                        repository_tags: BTreeSet::new(),
                    });
                image
                    .repository_tags
                    .insert(scrub_text(event.display_name.clone()));
            }
        }
        DockerObjectKind::Volume | DockerObjectKind::Network => {}
    }
}

pub(super) fn mapped_event(
    event: &ParsedDockerEvent,
) -> (Vec<RawObservation>, Vec<RawRelationshipSpec>) {
    let entity = event_entity(event);
    let (event_type, severity, subtype) = event_semantics(event);
    let related_entity_ids = event
        .published_ports
        .iter()
        .filter_map(|port| {
            protocol(&port.protocol).map(|protocol| {
                crate::domain::ids::port(protocol, &port.host_address, port.host_port)
            })
        })
        .collect();
    let observation = RawObservation {
        collector_id: ID,
        kind: ObservationKind {
            category: Category::Docker,
            event_type,
            severity,
            confidence: Confidence::High,
            user_friendly_summary: summary(event, event_type),
            technical_summary: format!("{subtype}: {}", event.display_name),
            classifier: ClassifierContext::default(),
        },
        entity: entity.clone(),
        before: None,
        after: None,
        field: None,
        occurred_at: event.occurred_at_ms,
        details: Some(json!({
            "subtype": subtype,
            "exit_code": event.exit_code,
            "image": event.image,
            "compose_project": event.compose_project,
        })),
        related_entity_ids,
        occurred_at_is_uncertain: false,
    };
    let relationships = event
        .published_ports
        .iter()
        .filter_map(|port| {
            let protocol = protocol(&port.protocol)?;
            Some(RawRelationshipSpec {
                evidence_observation: 0,
                source: entity.clone(),
                target: RawEntity {
                    identity: EntityIdentity::Port {
                        protocol,
                        address: port.host_address.clone(),
                        port: port.host_port,
                    },
                    display_name: format!(
                        "{} {}:{}",
                        port.protocol, port.host_address, port.host_port
                    ),
                },
                kind: RelationshipKind::Publishes,
            })
        })
        .collect();
    (vec![observation], relationships)
}

fn event_semantics(event: &ParsedDockerEvent) -> (EventType, Severity, &'static str) {
    match (event.object_kind, event.action) {
        (DockerObjectKind::Container, DockerAction::Created) => (
            EventType::Created,
            Severity::Info,
            "docker/container_created",
        ),
        (DockerObjectKind::Container, DockerAction::Started) => (
            EventType::Started,
            Severity::Info,
            "docker/container_started",
        ),
        (DockerObjectKind::Container, DockerAction::Stopped) => (
            EventType::Stopped,
            Severity::Info,
            "docker/container_stopped",
        ),
        (DockerObjectKind::Container, DockerAction::Died) => (
            if event.exit_code.unwrap_or_default() == 0 {
                EventType::Stopped
            } else {
                EventType::Failed
            },
            if event.exit_code.unwrap_or_default() == 0 {
                Severity::Info
            } else {
                Severity::Error
            },
            "docker/container_died",
        ),
        (DockerObjectKind::Container, DockerAction::Removed) => (
            EventType::Deleted,
            Severity::Info,
            "docker/container_removed",
        ),
        (DockerObjectKind::Image, DockerAction::Pulled) => {
            (EventType::Created, Severity::Info, "docker/image_pulled")
        }
        (DockerObjectKind::Image, DockerAction::Removed) => {
            (EventType::Deleted, Severity::Info, "docker/image_removed")
        }
        (DockerObjectKind::Volume, DockerAction::Created) => {
            (EventType::Created, Severity::Info, "docker/volume_created")
        }
        (DockerObjectKind::Volume, DockerAction::Removed) => {
            (EventType::Deleted, Severity::Info, "docker/volume_removed")
        }
        (DockerObjectKind::Network, DockerAction::Created) => {
            (EventType::Created, Severity::Info, "docker/network_created")
        }
        (DockerObjectKind::Network, DockerAction::Removed) => {
            (EventType::Deleted, Severity::Info, "docker/network_removed")
        }
        _ => (EventType::Modified, Severity::Info, "docker/object_changed"),
    }
}

fn event_entity(event: &ParsedDockerEvent) -> RawEntity {
    let identity = match event.object_kind {
        DockerObjectKind::Container => EntityIdentity::Container {
            docker_id: event.actor_id.clone(),
        },
        DockerObjectKind::Image => EntityIdentity::Image {
            repository_tag: event.actor_id.clone(),
        },
        DockerObjectKind::Volume if safe_external_identity(&event.display_name) => {
            EntityIdentity::DockerVolume {
                name: event.display_name.clone(),
            }
        }
        DockerObjectKind::Volume => EntityIdentity::System,
        DockerObjectKind::Network if safe_external_identity(&event.actor_id) => {
            EntityIdentity::DockerNetwork {
                name: event.actor_id.clone(),
            }
        }
        DockerObjectKind::Network => EntityIdentity::System,
    };
    RawEntity {
        identity,
        display_name: event.display_name.clone(),
    }
}

fn summary(event: &ParsedDockerEvent, event_type: EventType) -> String {
    if event.action == DockerAction::Died {
        if let Some(code) = event.exit_code {
            return format!("{} exited with code {code}", event.display_name);
        }
    }
    format!(
        "{} {}",
        event.display_name,
        event_type.as_str().replace('_', " ")
    )
}

pub(super) fn compose_observation(project: &str, occurred_at: i64) -> RawObservation {
    RawObservation {
        collector_id: ID,
        kind: ObservationKind {
            category: Category::Docker,
            event_type: EventType::Modified,
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary: format!("Docker Compose project {project} changed"),
            technical_summary: format!("docker/compose_project_changed: {project}"),
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            // Compose project names are user-controlled labels. They remain in
            // scrubbed display text, never in a stable id or a derived hash.
            identity: EntityIdentity::System,
            display_name: "Docker Compose".to_owned(),
        },
        before: None,
        after: None,
        field: None,
        occurred_at,
        details: Some(json!({ "subtype": "docker/compose_project_changed" })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: false,
    }
}

#[derive(Debug, Clone)]
pub(super) struct MappedChange {
    pub(super) baseline: DockerBaseline,
    pub(super) observations: Vec<RawObservation>,
    pub(super) relationships: Vec<RawRelationshipSpec>,
}

pub(super) fn diff_baseline(
    previous: &DockerBaseline,
    current: &DockerBaseline,
    occurred_at: i64,
) -> Vec<MappedChange> {
    let mut working = previous.clone();
    let mut changes = Vec::new();
    if previous.daemon_version != current.daemon_version {
        working.daemon_version.clone_from(&current.daemon_version);
        changes.push(MappedChange {
            baseline: working.clone(),
            observations: vec![daemon_observation(
                previous.daemon_version.as_deref(),
                current.daemon_version.as_deref(),
                occurred_at,
            )],
            relationships: Vec::new(),
        });
    }

    // Remove disappeared instances first. A same-name replacement is therefore
    // represented as remove(old id) then create(new id), with every persisted
    // intermediate baseline matching the observations already committed.
    for (id, old) in &previous.containers {
        if !current.containers.contains_key(id) {
            push_event_change(
                &mut working,
                &event_from_container(old, DockerAction::Removed, occurred_at),
                &mut changes,
            );
        }
    }
    for (id, container) in &current.containers {
        let old = previous.containers.get(id);
        let action = match old {
            None => Some(DockerAction::Created),
            Some(old) if old.state != container.state && container.state == "running" => {
                Some(DockerAction::Started)
            }
            Some(old) if old.state != container.state => Some(DockerAction::Stopped),
            Some(old) if old.image != container.image || old.ports != container.ports => {
                Some(DockerAction::Created)
            }
            Some(_) => None,
        };
        if let Some(action) = action {
            push_event_change(
                &mut working,
                &event_from_container(container, action, occurred_at),
                &mut changes,
            );
        }
    }

    // Remove tags before adding replacements so a stable tag changing digest
    // remains visible as one removal and one pull keyed by opaque image ids.
    for (id, image) in &previous.images {
        let current_tags = current
            .images
            .get(id)
            .map(|current| &current.repository_tags);
        for tag in image
            .repository_tags
            .iter()
            .filter(|tag| current_tags.is_none_or(|current_tags| !current_tags.contains(*tag)))
        {
            push_event_change(
                &mut working,
                &simple_event(
                    DockerObjectKind::Image,
                    DockerAction::Removed,
                    id,
                    tag,
                    occurred_at,
                ),
                &mut changes,
            );
        }
    }
    for (id, image) in &current.images {
        let old_tags = previous.images.get(id).map(|old| &old.repository_tags);
        for tag in image
            .repository_tags
            .iter()
            .filter(|tag| old_tags.is_none_or(|old_tags| !old_tags.contains(*tag)))
        {
            push_event_change(
                &mut working,
                &simple_event(
                    DockerObjectKind::Image,
                    DockerAction::Pulled,
                    id,
                    tag,
                    occurred_at,
                ),
                &mut changes,
            );
        }
    }

    if let Some(last) = changes.last_mut() {
        last.baseline = current.clone();
    } else if previous != current {
        changes.push(MappedChange {
            baseline: current.clone(),
            observations: Vec::new(),
            relationships: Vec::new(),
        });
    }
    changes
}

fn push_event_change(
    working: &mut DockerBaseline,
    event: &ParsedDockerEvent,
    changes: &mut Vec<MappedChange>,
) {
    apply_event(working, event);
    let (observations, relationships) = mapped_event(event);
    changes.push(MappedChange {
        baseline: working.clone(),
        observations,
        relationships,
    });
}

fn daemon_observation(
    before: Option<&str>,
    after: Option<&str>,
    occurred_at: i64,
) -> RawObservation {
    let before_display = before.unwrap_or("unknown");
    let after_display = after.unwrap_or("unknown");
    RawObservation {
        collector_id: ID,
        kind: ObservationKind {
            category: Category::Docker,
            event_type: EventType::VersionChanged,
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary: format!(
                "Docker Engine changed {before_display} → {after_display}"
            ),
            technical_summary: format!(
                "docker/daemon_changed: {before_display} -> {after_display}"
            ),
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            identity: EntityIdentity::System,
            display_name: "Docker Engine".to_owned(),
        },
        before: before.map(|value| json!(value)),
        after: after.map(|value| json!(value)),
        field: Some("version".to_owned()),
        occurred_at,
        details: Some(json!({ "subtype": "docker/daemon_changed" })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: false,
    }
}

fn event_from_container(
    container: &ContainerState,
    action: DockerAction,
    occurred_at: i64,
) -> ParsedDockerEvent {
    ParsedDockerEvent {
        schema_version: DOCKER_PROTOCOL_SCHEMA_VERSION,
        object_kind: DockerObjectKind::Container,
        action,
        actor_id: container.id.clone(),
        display_name: container.name.clone(),
        image: Some(container.image.clone()),
        compose_project: container.compose_project.clone(),
        exit_code: None,
        exit_code_label_present: !container.event_exit_code_trusted,
        published_ports: container
            .ports
            .iter()
            .cloned()
            .map(|port| PortPublication {
                host_address: port.host_address,
                host_port: port.host_port,
                container_port: port.container_port,
                protocol: port.protocol,
            })
            .collect(),
        occurred_at_ms: occurred_at,
    }
}

fn simple_event(
    kind: DockerObjectKind,
    action: DockerAction,
    actor_id: &str,
    display_name: &str,
    occurred_at: i64,
) -> ParsedDockerEvent {
    ParsedDockerEvent {
        schema_version: DOCKER_PROTOCOL_SCHEMA_VERSION,
        object_kind: kind,
        action,
        actor_id: actor_id.to_owned(),
        display_name: display_name.to_owned(),
        image: None,
        compose_project: None,
        exit_code: None,
        exit_code_label_present: false,
        published_ports: Vec::new(),
        occurred_at_ms: occurred_at,
    }
}

fn safe_external_identity(value: &str) -> bool {
    let (sanitized, redactions) = crate::privacy::Sanitizer::new().sanitize(value.to_owned());
    redactions.is_empty() && sanitized.get() == value
}

/// Baselines are serialized directly by the stateful collector, before the
/// normalizer gets a chance to scrub user-facing text. Keep Docker labels,
/// names, tags, and daemon strings redaction-safe at that boundary too.
fn scrub_text(value: String) -> String {
    crate::privacy::Sanitizer::new()
        .sanitize(value)
        .0
        .get()
        .clone()
}

fn protocol(value: &str) -> Option<Protocol> {
    match value {
        "tcp" => Some(Protocol::Tcp),
        "udp" => Some(Protocol::Udp),
        _ => None,
    }
}
