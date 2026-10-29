//! The entity identifier grammar.
//!
//! The same thing must get the same id across restarts, or relationships
//! silently rot: a process is keyed by a hash of its executable path rather
//! than by a pid, a port by protocol plus address plus number, a service by
//! its service name. Every constructor here is the only place its prefix is
//! written, so the grammar cannot drift between collectors.
//!
//! Path-based ids hash a canonical form — lowercased, `\\?\` prefix stripped,
//! separators normalised, trailing separator removed — because Windows paths
//! are case-insensitive and the same file arrives spelled several ways.

use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::enums::EntityType;

/// The transport a listening port belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Protocol {
    /// TCP.
    Tcp,
    /// UDP.
    Udp,
}

impl Protocol {
    /// The word used inside a port id.
    pub const fn as_str(self) -> &'static str {
        match self {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        }
    }
}

/// The form of a Windows path that two spellings of one file share.
pub fn canonical_path(path: &str) -> String {
    let stripped = path.strip_prefix(r"\\?\").unwrap_or(path);
    let mut out = stripped.trim().replace('/', "\\").to_lowercase();
    // A drive root keeps its separator; anything longer loses a trailing one.
    while out.len() > 3 && out.ends_with('\\') {
        out.pop();
    }
    out
}

/// The first sixteen hex characters of the SHA-256 of `text`.
///
/// Sixteen is enough that a collision between two paths on one machine is not
/// a realistic event, and short enough to read in a timeline row.
pub fn short_hash(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Whether `text` is exactly the suffix [`short_hash`] writes: sixteen
/// lowercase hexadecimal characters.
///
/// The one shape check behind both the storage trust boundary and
/// [`ProcessId::parse`], so the two cannot drift apart.
pub fn is_short_hash(text: &str) -> bool {
    text.len() == 16
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// A running program, keyed by where its executable lives.
pub fn process(exe_path: &str) -> String {
    format!("process:{}", short_hash(&canonical_path(exe_path)))
}

/// A process identity already reduced to its stable id.
///
/// A collector persists this rather than the executable path, so hydrating a
/// durable baseline never has to recompute an id from text the sanitizer may
/// have rewritten. A path carrying a secret comes back redacted; an id computed
/// from it before redaction does not move, and the process it names therefore
/// does not appear to stop and start across a restart.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ProcessId(String);

/// Deserialization validates, so a durable baseline cannot smuggle an arbitrary
/// string in as an identity.
///
/// The rejection names the rule and not the value: a stored id is a hash, and
/// the string that failed to be one is exactly the kind of text a log line
/// should not carry.
impl<'de> Deserialize<'de> for ProcessId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).ok_or_else(|| {
            serde::de::Error::custom(
                "a stored process id is not `process:` plus sixteen hex digits",
            )
        })
    }
}

impl ProcessId {
    /// The id for one executable. A process whose path a non-elevated reader
    /// cannot see passes its image name, which is a stable identity of a
    /// weaker kind and is reported as such by the collector that used it.
    pub fn from_executable(executable: &str) -> Self {
        Self(process(executable))
    }

    /// Re-adopt a stored id, rejecting anything that is not `process:` plus
    /// sixteen hexadecimal characters.
    pub fn parse(text: &str) -> Option<Self> {
        let suffix = text.strip_prefix("process:")?;
        is_short_hash(suffix).then(|| Self(text.to_owned()))
    }

    /// The id exactly as stored.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A listening socket.
pub fn port(protocol: Protocol, address: &str, port: u16) -> String {
    format!("port:{}:{address}:{port}", protocol.as_str())
}

/// A Windows service, by its service name (not its display name).
pub fn service(name: &str) -> String {
    format!("service:{name}")
}

/// An installed application, by the registry key it was enumerated from.
pub fn app(hive: &str, registry_key: &str) -> String {
    format!("app:{hive}:{registry_key}")
}

/// A device driver, by the device instance it is bound to.
pub fn driver(device_instance_id: &str) -> String {
    format!("driver:{device_instance_id}")
}

/// A file inside a watched folder, keyed by its canonical path.
pub fn file(path: &str) -> String {
    format!("file:{}", short_hash(&canonical_path(path)))
}

/// A network adapter, by its GUID.
pub fn adapter(guid: &str) -> String {
    format!("adapter:{guid}")
}

/// A Docker container, by the twelve-character short id Docker itself shows.
pub fn container(docker_id: &str) -> String {
    let short: String = docker_id.chars().take(12).collect();
    format!("container:{short}")
}

/// A Docker image, by `repo:tag`.
pub fn image(repo_tag: &str) -> String {
    format!("image:{repo_tag}")
}

/// A Docker volume, by its engine-owned name.
pub fn docker_volume(name: &str) -> String {
    format!("docker_volume:{name}")
}

/// A Docker network, by its engine-owned name.
pub fn docker_network(name: &str) -> String {
    format!("docker_network:{name}")
}

/// A registered project folder, keyed by its canonical path.
pub fn project(path: &str) -> String {
    format!("project:{}", short_hash(&canonical_path(path)))
}

/// A Windows update, by KB number or update id.
pub fn update(kb_or_update_id: &str) -> String {
    format!("update:{kb_or_update_id}")
}

/// A scheduled task, by its task path.
pub fn task(task_path: &str) -> String {
    format!("task:{task_path}")
}

/// The machine itself, for boot and shutdown events.
pub fn system() -> String {
    "system:local".to_owned()
}

/// The entity type an id's prefix names, if the prefix is one of ours.
pub fn entity_type_of(id: &str) -> Option<EntityType> {
    let (prefix, _) = id.split_once(':')?;
    EntityType::from_str(prefix).ok()
}

/// Whether an id follows the canonical grammar emitted by this module.
/// Prefix-only checks are insufficient: `service:` names no service and must
/// never cross an IPC boundary as though it were a real entity.
pub fn is_valid_entity_id(id: &str) -> bool {
    let Some(entity_type) = entity_type_of(id) else {
        return false;
    };
    is_valid_entity_id_for(id, entity_type)
}

/// Validate an id and its declared type through the one canonical grammar
/// shared by collector commits and IPC lookups.
pub fn is_valid_entity_id_for(id: &str, entity_type: EntityType) -> bool {
    if entity_type_of(id) != Some(entity_type) {
        return false;
    }
    let Some((_, suffix)) = id.split_once(':') else {
        return false;
    };
    if suffix.is_empty() || suffix.chars().any(char::is_control) {
        return false;
    }
    match entity_type {
        EntityType::Process | EntityType::File | EntityType::Project => is_short_hash(suffix),
        EntityType::Port => {
            let Some((protocol, endpoint)) = suffix.split_once(':') else {
                return false;
            };
            let Some((address, port)) = endpoint.rsplit_once(':') else {
                return false;
            };
            matches!(protocol, "tcp" | "udp") && !address.is_empty() && port.parse::<u16>().is_ok()
        }
        EntityType::System => suffix == "local",
        EntityType::Container => suffix.chars().count() <= 64,
        EntityType::Service
        | EntityType::Application
        | EntityType::Driver
        | EntityType::Adapter
        | EntityType::Image
        | EntityType::DockerVolume
        | EntityType::DockerNetwork
        | EntityType::Update
        | EntityType::Task => true,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn canonical_form_ignores_case_prefix_and_trailing_separator() {
        let expected = canonical_path(r"C:\Program Files\Docker\docker.exe");
        assert_eq!(expected, r"c:\program files\docker\docker.exe");
        assert_eq!(
            canonical_path(r"\\?\C:\Program Files\Docker\docker.exe"),
            expected
        );
        assert_eq!(
            canonical_path("C:/Program Files/Docker/docker.exe"),
            expected
        );
        assert_eq!(
            canonical_path(r"C:\Program Files\Docker\docker.exe\"),
            expected
        );
        assert_eq!(
            canonical_path(r"C:\"),
            r"c:\",
            "a drive root keeps its separator"
        );
    }

    #[test]
    fn two_spellings_of_one_executable_share_an_id() {
        let a = process(r"C:\Users\dev\api\API.EXE");
        let b = process(r"\\?\c:\users\dev\api\api.exe");
        assert_eq!(a, b);
        assert!(a.starts_with("process:"));
        assert_eq!(a.len(), "process:".len() + 16);
    }

    #[test]
    fn every_prefix_parses_back_to_its_entity_type() {
        let cases = [
            (process("x.exe"), EntityType::Process),
            (port(Protocol::Tcp, "0.0.0.0", 3000), EntityType::Port),
            (service("Spooler"), EntityType::Service),
            (app("HKLM", "Uninstall\\Docker"), EntityType::Application),
            (driver("PCI\\VEN_10DE"), EntityType::Driver),
            (file(r"C:\p\.env"), EntityType::File),
            (adapter("{6D809377}"), EntityType::Adapter),
            (container("abcdef0123456789"), EntityType::Container),
            (image("nginx:latest"), EntityType::Image),
            (docker_volume("postgres-data"), EntityType::DockerVolume),
            (docker_network("api-default"), EntityType::DockerNetwork),
            (project(r"C:\p"), EntityType::Project),
            (update("KB5031356"), EntityType::Update),
            (task(r"\Microsoft\Windows\Defrag"), EntityType::Task),
            (system(), EntityType::System),
        ];
        for (id, entity_type) in cases {
            assert_eq!(entity_type_of(&id), Some(entity_type), "{id}");
        }
        assert_eq!(entity_type_of("weather:today"), None);
        assert_eq!(entity_type_of("no separator"), None);
    }

    #[test]
    fn a_port_id_reads_as_protocol_address_port() {
        assert_eq!(
            port(Protocol::Tcp, "0.0.0.0", 3000),
            "port:tcp:0.0.0.0:3000"
        );
        assert_eq!(port(Protocol::Udp, "::", 53), "port:udp::::53");
    }

    #[test]
    fn a_container_id_is_the_twelve_character_short_form() {
        assert_eq!(
            container("3f2a9c8e7d6b5a4f3e2d1c0b9a8f7e6d"),
            "container:3f2a9c8e7d6b"
        );
        assert_eq!(container("short"), "container:short");
    }

    #[test]
    fn a_stored_process_id_is_adopted_rather_than_recomputed() {
        let computed = ProcessId::from_executable(r"C:\Users\dev\api\API.EXE");
        assert_eq!(computed, ProcessId::parse(computed.as_str()).unwrap());
        assert_eq!(
            computed,
            ProcessId::from_executable(r"\\?\c:\users\dev\api\api.exe")
        );

        for rejected in [
            "",
            "process:",
            "process:0123456789abcde",
            "process:0123456789abcdefg",
            "process:0123456789ABCDEF",
            "process:sk-abcdefghijkl",
            "file:0123456789abcdef",
        ] {
            assert_eq!(ProcessId::parse(rejected), None, "{rejected}");
        }
    }

    #[test]
    fn canonical_validation_rejects_prefixes_without_an_identity() {
        for rejected in [
            "service:",
            "process:not-a-hash",
            "file:0123",
            "port:tcp:3000",
            "port:http:0.0.0.0:3000",
            "port:tcp:0.0.0.0:70000",
            "system:remote",
            "unknown:value",
            " service:api ",
            "service:api\nspoof",
        ] {
            assert!(!is_valid_entity_id(rejected), "{rejected}");
        }
        for accepted in [
            "service:api",
            "process:0123456789abcdef",
            "file:0123456789abcdef",
            "port:tcp:0.0.0.0:3000",
            "port:udp::::53",
            "system:local",
        ] {
            assert!(is_valid_entity_id(accepted), "{accepted}");
        }
    }
}
