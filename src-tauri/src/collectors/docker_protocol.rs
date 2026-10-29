//! Pure, bounded Docker Engine wire-format parsing.
//!
//! Docker metadata is untrusted local input. This module deliberately exposes
//! only the few attributes the collector needs; the original attribute map is
//! dropped so labels and container environment material cannot accidentally
//! reach logs or persistence.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, BufRead};

use serde::Deserialize;

pub const DOCKER_PROTOCOL_SCHEMA_VERSION: u16 = 1;
pub const PRIMARY_API_VERSION: DockerApiVersion = DockerApiVersion::V1_43;
pub const FALLBACK_API_VERSION: DockerApiVersion = DockerApiVersion::V1_41;
pub const MAX_EVENT_LINE_BYTES: usize = 1024 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_EVENTS_PER_BATCH: usize = 10_000;
pub const MAX_BASELINE_ITEMS: usize = 10_000;

const MAX_ID_CHARS: usize = 256;
const MAX_NAME_CHARS: usize = 256;
const MAX_VERSION_CHARS: usize = 128;
const MAX_ATTRIBUTES: usize = 128;
const EVENT_FILTERS: &str =
    "%7B%22type%22%3A%5B%22container%22%2C%22image%22%2C%22volume%22%2C%22network%22%5D%7D";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerApiVersion {
    V1_43,
    V1_41,
}

impl DockerApiVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1_43 => "v1.43",
            Self::V1_41 => "v1.41",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerRequest {
    Ping,
    Version,
    Events,
    Containers,
    Images,
}

impl DockerRequest {
    /// The complete read-only request target. No arbitrary path can cross the
    /// transport boundary.
    pub fn path(self, version: DockerApiVersion) -> String {
        match self {
            Self::Ping => "/_ping".to_owned(),
            Self::Version => format!("/{}/version", version.as_str()),
            Self::Events => format!("/{}/events?filters={EVENT_FILTERS}", version.as_str()),
            Self::Containers => format!("/{}/containers/json?all=1", version.as_str()),
            Self::Images => format!("/{}/images/json", version.as_str()),
        }
    }
}

/// Errors contain only fixed protocol facts. Docker-supplied values are never
/// included because these messages may be written to the application log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerProtocolError {
    LineTooLarge,
    ResponseTooLarge,
    TooManyItems,
    InvalidUtf8,
    MalformedJson,
    InvalidSchema(&'static str),
    Io,
}

impl fmt::Display for DockerProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LineTooLarge => f.write_str("Docker event exceeded the protocol size limit"),
            Self::ResponseTooLarge => {
                f.write_str("Docker response exceeded the protocol size limit")
            }
            Self::TooManyItems => f.write_str("Docker response exceeded the protocol item limit"),
            Self::InvalidUtf8 => f.write_str("Docker response was not valid UTF-8"),
            Self::MalformedJson => f.write_str("Docker response was not valid JSON"),
            Self::InvalidSchema(field) => write!(f, "Docker response had an invalid {field} field"),
            Self::Io => f.write_str("Docker response could not be read"),
        }
    }
}

impl std::error::Error for DockerProtocolError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerObjectKind {
    Container,
    Image,
    Volume,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerAction {
    Created,
    Started,
    Stopped,
    Died,
    Removed,
    Pulled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortPublication {
    pub host_address: String,
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDockerEvent {
    pub schema_version: u16,
    pub object_kind: DockerObjectKind,
    pub action: DockerAction,
    pub actor_id: String,
    pub display_name: String,
    pub image: Option<String>,
    pub compose_project: Option<String>,
    pub exit_code: Option<i32>,
    /// On a create event, whether a user label occupied Docker's reserved
    /// `exitCode` event-attribute key. Moby flattens labels into Attributes,
    /// so later die-event values are trusted only when this is known false.
    pub exit_code_label_present: bool,
    pub published_ports: Vec<PortPublication>,
    pub occurred_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerDaemonVersion {
    pub engine_version: String,
    pub api_version: String,
    pub minimum_api_version: String,
    pub operating_system: String,
    pub architecture: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerContainerSummary {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub state: String,
    pub status: String,
    pub compose_project: Option<String>,
    pub exit_code_label_present: bool,
    pub published_ports: Vec<PortPublication>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerImageSummary {
    pub id: String,
    pub repository_tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WireEvent {
    #[serde(rename = "Type")]
    object_type: String,
    action: Option<String>,
    status: Option<String>,
    actor: WireActor,
    #[serde(default)]
    #[serde(rename = "time")]
    time: Option<i64>,
    #[serde(default)]
    #[serde(rename = "timeNano")]
    time_nano: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WireActor {
    #[serde(rename = "ID")]
    id: String,
    #[serde(default)]
    attributes: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WireVersion {
    version: String,
    api_version: String,
    #[serde(rename = "MinAPIVersion")]
    min_api_version: String,
    os: String,
    arch: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WireContainer {
    id: String,
    #[serde(default)]
    names: Vec<String>,
    image: String,
    state: String,
    status: String,
    #[serde(default)]
    labels: BTreeMap<String, String>,
    #[serde(default)]
    ports: Vec<WirePort>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WirePort {
    #[serde(default)]
    #[serde(rename = "IP")]
    ip: String,
    private_port: u16,
    #[serde(default)]
    public_port: Option<u16>,
    #[serde(rename = "Type")]
    protocol: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct WireImage {
    id: String,
    #[serde(default)]
    repo_tags: Vec<String>,
}

pub fn parse_event_line(line: &[u8]) -> Result<Option<ParsedDockerEvent>, DockerProtocolError> {
    if line.len() > MAX_EVENT_LINE_BYTES {
        return Err(DockerProtocolError::LineTooLarge);
    }
    let text = std::str::from_utf8(line).map_err(|_| DockerProtocolError::InvalidUtf8)?;
    let text = text.trim();
    if text.is_empty() || text.starts_with('#') {
        return Ok(None);
    }
    let event: WireEvent =
        serde_json::from_str(text).map_err(|_| DockerProtocolError::MalformedJson)?;
    if event.actor.attributes.len() > MAX_ATTRIBUTES {
        return Err(DockerProtocolError::InvalidSchema("attributes"));
    }
    validate_text(&event.actor.id, MAX_ID_CHARS, "actor id")?;

    let object_kind = match event.object_type.as_str() {
        "container" => DockerObjectKind::Container,
        "image" => DockerObjectKind::Image,
        "volume" => DockerObjectKind::Volume,
        "network" => DockerObjectKind::Network,
        _ => return Ok(None),
    };
    if matches!(
        object_kind,
        DockerObjectKind::Container | DockerObjectKind::Image | DockerObjectKind::Network
    ) {
        validate_engine_id(&event.actor.id, "actor id")?;
    }
    let wire_action = event.action.as_deref().or(event.status.as_deref());
    let Some(action) = action_for(object_kind, wire_action.unwrap_or_default()) else {
        return Ok(None);
    };

    let attributes = &event.actor.attributes;
    let preferred_name = match object_kind {
        DockerObjectKind::Container => attributes.get("name"),
        DockerObjectKind::Image => attributes.get("name").or_else(|| attributes.get("image")),
        DockerObjectKind::Volume | DockerObjectKind::Network => {
            attributes.get("name").or(Some(&event.actor.id))
        }
    };
    let display_name = bounded_text(
        preferred_name
            .map(String::as_str)
            .unwrap_or(&event.actor.id),
        MAX_NAME_CHARS,
        "display name",
    )?;
    let image = optional_bounded(attributes.get("image"), MAX_NAME_CHARS, "image")?;
    let compose_project = optional_bounded(
        attributes.get("com.docker.compose.project"),
        MAX_NAME_CHARS,
        "compose project",
    )?;
    // Moby copies user labels into Actor.Attributes and labels can overwrite
    // daemon-supplied keys. Parse defensively here; the collector qualifies
    // this value against the authoritative container-list label set before it
    // can affect severity or persistence.
    let exit_code = attributes
        .get("exitCode")
        .and_then(|value| value.parse::<i32>().ok());
    let exit_code_label_present =
        action == DockerAction::Created && attributes.contains_key("exitCode");
    let occurred_at_ms = timestamp_ms(event.time, event.time_nano)?;

    Ok(Some(ParsedDockerEvent {
        schema_version: DOCKER_PROTOCOL_SCHEMA_VERSION,
        object_kind,
        action,
        actor_id: event.actor.id,
        display_name: display_name.trim_start_matches('/').to_owned(),
        image,
        compose_project,
        exit_code,
        exit_code_label_present,
        // Event Attributes are a flattened label map, and the Engine API does
        // not define port keys there. Authoritative ports come only from the
        // typed /containers/json baseline and are attached by the collector.
        published_ports: Vec::new(),
        occurred_at_ms,
    }))
}

pub fn parse_event_stream<R: BufRead>(
    mut reader: R,
) -> Result<Vec<ParsedDockerEvent>, DockerProtocolError> {
    let mut events = Vec::new();
    let mut line = Vec::new();
    loop {
        let read = read_event_frame(&mut reader, &mut line).map_err(|error| match error {
            EventFrameReadError::LineTooLarge => DockerProtocolError::LineTooLarge,
            EventFrameReadError::Io(_) => DockerProtocolError::Io,
        })?;
        if read == 0 {
            break;
        }
        if let Some(event) = parse_event_line(&line)? {
            if events.len() == MAX_EVENTS_PER_BATCH {
                return Err(DockerProtocolError::TooManyItems);
            }
            events.push(event);
        }
        line.clear();
    }
    Ok(events)
}

pub(super) enum EventFrameReadError {
    Io(io::Error),
    LineTooLarge,
}

/// Reads one NDJSON frame without ever growing `frame` beyond the protocol
/// limit. Docker owns the local pipe but its bytes are still untrusted input.
/// An idle I/O timeout preserves the partial frame; the caller clears it only
/// after successfully consuming the complete frame returned by this function.
pub(super) fn read_event_frame<R: BufRead>(
    reader: &mut R,
    frame: &mut Vec<u8>,
) -> Result<usize, EventFrameReadError> {
    loop {
        let (take, complete) = {
            let available = reader.fill_buf().map_err(EventFrameReadError::Io)?;
            if available.is_empty() {
                return Ok(frame.len());
            }
            let take = available
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(available.len(), |index| index + 1);
            if frame.len().saturating_add(take) > MAX_EVENT_LINE_BYTES {
                return Err(EventFrameReadError::LineTooLarge);
            }
            frame.extend_from_slice(&available[..take]);
            (take, available[take - 1] == b'\n')
        };
        reader.consume(take);
        if complete {
            return Ok(frame.len());
        }
    }
}

pub fn parse_version_response(body: &[u8]) -> Result<DockerDaemonVersion, DockerProtocolError> {
    let wire: WireVersion = parse_bounded_json(body)?;
    Ok(DockerDaemonVersion {
        engine_version: bounded_text(&wire.version, MAX_VERSION_CHARS, "engine version")?,
        api_version: bounded_text(&wire.api_version, MAX_VERSION_CHARS, "API version")?,
        minimum_api_version: bounded_text(
            &wire.min_api_version,
            MAX_VERSION_CHARS,
            "minimum API version",
        )?,
        operating_system: bounded_text(&wire.os, MAX_NAME_CHARS, "operating system")?,
        architecture: bounded_text(&wire.arch, MAX_NAME_CHARS, "architecture")?,
    })
}

pub fn parse_containers_response(
    body: &[u8],
) -> Result<Vec<DockerContainerSummary>, DockerProtocolError> {
    let wire: Vec<WireContainer> = parse_bounded_json(body)?;
    if wire.len() > MAX_BASELINE_ITEMS {
        return Err(DockerProtocolError::TooManyItems);
    }
    wire.into_iter()
        .map(|container| {
            validate_text(&container.id, MAX_ID_CHARS, "container id")?;
            if container.labels.len() > MAX_ATTRIBUTES || container.ports.len() > MAX_ATTRIBUTES {
                return Err(DockerProtocolError::TooManyItems);
            }
            let names = bounded_vec(container.names, MAX_NAME_CHARS, "container name")?;
            let compose_project = optional_bounded(
                container.labels.get("com.docker.compose.project"),
                MAX_NAME_CHARS,
                "compose project",
            )?;
            let mut published_ports = Vec::new();
            for port in container.ports {
                let Some(host_port) = port.public_port else {
                    continue;
                };
                let host_address = if port.ip.is_empty() {
                    "0.0.0.0".to_owned()
                } else {
                    bounded_text(&port.ip, MAX_NAME_CHARS, "host address")?
                };
                let protocol = bounded_text(&port.protocol, MAX_NAME_CHARS, "port protocol")?;
                // Docker can publish SCTP (and future protocols) even though
                // the domain graph currently models only TCP/UDP. Keep the
                // typed container baseline usable and simply omit protocols we
                // cannot represent instead of rejecting every container.
                if !matches!(protocol.as_str(), "tcp" | "udp") {
                    continue;
                }
                published_ports.push(PortPublication {
                    host_address,
                    host_port,
                    container_port: port.private_port,
                    protocol,
                });
            }
            Ok(DockerContainerSummary {
                id: container.id,
                names,
                image: bounded_text(&container.image, MAX_NAME_CHARS, "container image")?,
                state: bounded_text(&container.state, MAX_NAME_CHARS, "container state")?,
                status: bounded_text(&container.status, MAX_NAME_CHARS, "container status")?,
                compose_project,
                exit_code_label_present: container.labels.contains_key("exitCode"),
                published_ports,
            })
        })
        .collect()
}

pub fn parse_images_response(body: &[u8]) -> Result<Vec<DockerImageSummary>, DockerProtocolError> {
    let wire: Vec<WireImage> = parse_bounded_json(body)?;
    if wire.len() > MAX_BASELINE_ITEMS {
        return Err(DockerProtocolError::TooManyItems);
    }
    wire.into_iter()
        .map(|image| {
            validate_text(&image.id, MAX_ID_CHARS, "image id")?;
            Ok(DockerImageSummary {
                id: image.id,
                repository_tags: bounded_vec(image.repo_tags, MAX_NAME_CHARS, "repository tag")?,
            })
        })
        .collect()
}

fn action_for(kind: DockerObjectKind, action: &str) -> Option<DockerAction> {
    match (kind, action) {
        (DockerObjectKind::Container, "create") => Some(DockerAction::Created),
        (DockerObjectKind::Container, "start") => Some(DockerAction::Started),
        (DockerObjectKind::Container, "stop" | "kill") => Some(DockerAction::Stopped),
        (DockerObjectKind::Container, "die") => Some(DockerAction::Died),
        (DockerObjectKind::Container, "destroy" | "remove") => Some(DockerAction::Removed),
        (DockerObjectKind::Image, "pull") => Some(DockerAction::Pulled),
        (DockerObjectKind::Image, "delete" | "remove") => Some(DockerAction::Removed),
        (DockerObjectKind::Volume | DockerObjectKind::Network, "create") => {
            Some(DockerAction::Created)
        }
        (DockerObjectKind::Volume | DockerObjectKind::Network, "destroy" | "remove") => {
            Some(DockerAction::Removed)
        }
        _ => None,
    }
}

fn timestamp_ms(seconds: Option<i64>, nanos: Option<i64>) -> Result<i64, DockerProtocolError> {
    if let Some(nanos) = nanos.filter(|value| *value > 0) {
        let milliseconds = nanos / 1_000_000;
        if milliseconds > 0 {
            return Ok(milliseconds);
        }
    }
    let Some(seconds) = seconds.filter(|value| *value > 0) else {
        return Err(DockerProtocolError::InvalidSchema("timestamp"));
    };
    seconds
        .checked_mul(1_000)
        .filter(|value| *value > 0)
        .ok_or(DockerProtocolError::InvalidSchema("timestamp"))
}

/// Engine object ids are opaque identifiers supplied by Docker. They are
/// allowed to be digest-shaped (we must not derive another hash), but labels,
/// URLs, paths, and credential-shaped values are not valid structural ids and
/// must never become durable entity keys.
fn validate_engine_id(value: &str, field: &'static str) -> Result<(), DockerProtocolError> {
    let digest_body = value.strip_prefix("sha256:").unwrap_or(value);
    let is_hex_id = digest_body.len() >= 12 && digest_body.chars().all(|ch| ch.is_ascii_hexdigit());
    let has_structural_delimiter = value
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '/' | '\\' | '=' | '?' | '&'));
    if has_structural_delimiter {
        return Err(DockerProtocolError::InvalidSchema(field));
    }
    if !is_hex_id {
        let (_, redactions) = crate::privacy::Sanitizer::new().sanitize(value.to_owned());
        if !redactions.is_empty() {
            return Err(DockerProtocolError::InvalidSchema(field));
        }
    }
    Ok(())
}

fn parse_bounded_json<T: serde::de::DeserializeOwned>(
    body: &[u8],
) -> Result<T, DockerProtocolError> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(DockerProtocolError::ResponseTooLarge);
    }
    let text = std::str::from_utf8(body).map_err(|_| DockerProtocolError::InvalidUtf8)?;
    serde_json::from_str(text).map_err(|_| DockerProtocolError::MalformedJson)
}

fn bounded_vec(
    values: Vec<String>,
    limit: usize,
    field: &'static str,
) -> Result<Vec<String>, DockerProtocolError> {
    if values.len() > MAX_ATTRIBUTES {
        return Err(DockerProtocolError::TooManyItems);
    }
    values
        .into_iter()
        .map(|value| bounded_text(&value, limit, field))
        .collect()
}

fn optional_bounded(
    value: Option<&String>,
    limit: usize,
    field: &'static str,
) -> Result<Option<String>, DockerProtocolError> {
    value
        .map(|value| bounded_text(value, limit, field))
        .transpose()
}

fn bounded_text(
    value: &str,
    limit: usize,
    field: &'static str,
) -> Result<String, DockerProtocolError> {
    validate_text(value, limit, field)?;
    Ok(value.to_owned())
}

fn validate_text(
    value: &str,
    limit: usize,
    field: &'static str,
) -> Result<(), DockerProtocolError> {
    if value.is_empty() || value.chars().count() > limit || value.chars().any(char::is_control) {
        return Err(DockerProtocolError::InvalidSchema(field));
    }
    Ok(())
}
