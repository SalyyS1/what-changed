//! Bounded, format-aware config parsing and privacy-preserving key diffs.

mod env;
mod flatten;
mod ini;
mod json;
mod toml;
mod xml;
mod yaml;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use zeroize::{Zeroize, Zeroizing};

use crate::privacy::{scrub_value, REDACTED};

use super::ignore_set::MAX_FILE_BYTES;

/// Exact per-key events emitted before a bounded summary replaces the tail.
pub const MAX_KEY_EVENTS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Env,
    Json,
    Yaml,
    Toml,
    Ini,
    Properties,
    Xml,
}

impl ConfigFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Ini => "ini",
            Self::Properties => "properties",
            Self::Xml => "xml",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedConfig {
    pub format: ConfigFormat,
    /// Values are sanitized before this map exists outside the parser.
    pub keys: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOutcome {
    Unsupported,
    TooLarge,
    Parsed(ParsedConfig),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    ReadFailed,
    InvalidUtf8(ConfigFormat),
    ParseSkipped(ConfigFormat),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFailed => formatter.write_str("config file could not be read"),
            Self::InvalidUtf8(format) => {
                write!(formatter, "{} config is not valid UTF-8", format.as_str())
            }
            Self::ParseSkipped(format) => {
                write!(formatter, "{} config could not be parsed", format.as_str())
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Compatibility helper for parser tests and callers without a watched root.
/// The production file collector must use [`read_under`].
pub fn read(path: &Path) -> Result<ReadOutcome, ConfigError> {
    let root = path.parent().ok_or(ConfigError::ReadFailed)?;
    read_under(root, path)
}

/// Read through an OS-validated handle whose final object is contained by the
/// approved watched root. The same handle supplies every byte to the parser.
pub fn read_under(root: &Path, path: &Path) -> Result<ReadOutcome, ConfigError> {
    let mut bytes = Zeroizing::new(Vec::new());
    crate::platform::safe_file::read_under(root, path, MAX_FILE_BYTES, &mut bytes)
        .map_err(|_| ConfigError::ReadFailed)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Ok(ReadOutcome::TooLarge);
    }
    parse(path, &bytes)
}

/// Parse caller-owned bytes and return only sanitized values. Callers that
/// read plaintext themselves remain responsible for overwriting that buffer;
/// [`read_under`] is the production entry point that enforces it automatically.
pub fn parse(path: &Path, bytes: &[u8]) -> Result<ReadOutcome, ConfigError> {
    let Some(format) = detect(path, bytes) else {
        return Ok(ReadOutcome::Unsupported);
    };
    let raw = match format {
        ConfigFormat::Json => json::parse(bytes),
        ConfigFormat::Xml => xml::parse(bytes),
        ConfigFormat::Env
        | ConfigFormat::Yaml
        | ConfigFormat::Toml
        | ConfigFormat::Ini
        | ConfigFormat::Properties => {
            let text = std::str::from_utf8(bytes).map_err(|_| ConfigError::InvalidUtf8(format))?;
            match format {
                ConfigFormat::Env => Ok(env::parse(text)),
                ConfigFormat::Yaml => yaml::parse(text),
                ConfigFormat::Toml => toml::parse(text),
                ConfigFormat::Ini => Ok(ini::parse(text, true)),
                ConfigFormat::Properties => Ok(ini::parse(text, false)),
                ConfigFormat::Json | ConfigFormat::Xml => unreachable!(),
            }
        }
    }
    .map_err(|_| ConfigError::ParseSkipped(format))?;

    Ok(ReadOutcome::Parsed(ParsedConfig {
        format,
        keys: sanitize(raw),
    }))
}

fn sanitize(raw: BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut sanitized = BTreeMap::new();
    for (mut key, mut value) in raw {
        let safe_key = scrub_value(None, &key).text.into_owned();
        let safe = scrub_value(Some(&key), &value).text.into_owned();
        key.zeroize();
        value.zeroize();
        sanitized.insert(safe_key, safe);
    }
    sanitized
}

fn detect(path: &Path, bytes: &[u8]) -> Option<ConfigFormat> {
    let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    if name == ".env" || name.starts_with(".env.") {
        return Some(ConfigFormat::Env);
    }
    let extension = path
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase());
    match extension.as_deref() {
        Some("json") => Some(ConfigFormat::Json),
        Some("yaml" | "yml") => Some(ConfigFormat::Yaml),
        Some("toml") => Some(ConfigFormat::Toml),
        Some("ini" | "conf") => Some(ConfigFormat::Ini),
        Some("properties") => Some(ConfigFormat::Properties),
        Some("xml") => Some(ConfigFormat::Xml),
        _ if matches!(name.as_str(), "dockerfile" | "caddyfile" | "hosts") => sniff(bytes),
        _ => None,
    }
}

fn sniff(bytes: &[u8]) -> Option<ConfigFormat> {
    let text = std::str::from_utf8(bytes)
        .ok()?
        .trim_start_matches('\u{feff}')
        .trim_start();
    if text.starts_with('{') || text.starts_with('[') {
        Some(ConfigFormat::Json)
    } else if text.starts_with('<') {
        Some(ConfigFormat::Xml)
    } else if text.lines().any(|line| {
        let line = line.trim();
        line.starts_with('[') && line.ends_with(']')
    }) {
        Some(ConfigFormat::Ini)
    } else if text.lines().any(|line| line.contains('=')) {
        Some(ConfigFormat::Env)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigChange {
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigDiffItem {
    Change(ConfigChange),
    Summary { omitted: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigDiff {
    pub changes: Vec<ConfigChange>,
    pub omitted: usize,
    /// A retained sensitive key is `[REDACTED]` on both sides. Its value may
    /// have changed, but the persisted comparison state intentionally cannot
    /// prove that, so only a file-level uncertainty event is honest.
    pub sensitive_uncertain: bool,
}

impl ConfigDiff {
    pub fn between(before: &BTreeMap<String, String>, after: &BTreeMap<String, String>) -> Self {
        let fields: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
        let mut exact = Vec::new();
        let mut sensitive_uncertain = false;
        for field in fields {
            let old = before.get(field);
            let new = after.get(field);
            if old.map(String::as_str) == Some(REDACTED)
                && new.map(String::as_str) == Some(REDACTED)
            {
                sensitive_uncertain = true;
                continue;
            }
            if old != new {
                exact.push(ConfigChange {
                    field: field.clone(),
                    before: old.cloned(),
                    after: new.cloned(),
                });
            }
        }
        let omitted = exact.len().saturating_sub(MAX_KEY_EVENTS);
        exact.truncate(MAX_KEY_EVENTS);
        Self {
            changes: exact,
            omitted,
            sensitive_uncertain,
        }
    }

    /// The bounded event-spec sequence: at most 20 keyed changes plus one
    /// summary for every exact change beyond the cap.
    pub fn items(&self) -> Vec<ConfigDiffItem> {
        let mut items: Vec<_> = self
            .changes
            .iter()
            .cloned()
            .map(ConfigDiffItem::Change)
            .collect();
        if self.omitted > 0 {
            items.push(ConfigDiffItem::Summary {
                omitted: self.omitted,
            });
        }
        items
    }
}
