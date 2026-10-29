//! Paging vocabulary: what a timeline query asks for and what a page carries.
//!
//! Pagination is keyset, never offset: a cursor is the `(timestamp, id)` of
//! the last row consumed, so page five hundred costs what page one costs and
//! a row inserted while the user scrolls can neither be shown twice nor push
//! another row out of view.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;

use super::enums::{Category, Confidence, EntityType, EventType, Importance, Severity};

/// The most rows one page may carry, whatever the caller asked for.
pub const MAX_PAGE_SIZE: u16 = 200;

/// The page size when the caller does not say.
pub const DEFAULT_PAGE_SIZE: u16 = 100;

/// A user-visible range. Custom bounds are inclusive UTC epoch milliseconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimeRange {
    Today,
    Yesterday,
    Last7Days,
    SinceLastRestart,
    Custom { from: i64, to: i64 },
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::Custom {
            from: i64::MIN,
            to: i64::MAX,
        }
    }
}

/// Maximum accepted custom range (one year).
pub const MAX_RANGE_MS: i64 = 366 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorDecodeError {
    InvalidEncoding,
    InvalidLength,
    InvalidParts,
}
impl fmt::Display for CursorDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidEncoding => "cursor encoding is invalid",
            Self::InvalidLength => "cursor length is invalid",
            Self::InvalidParts => "cursor parts are invalid",
        })
    }
}
impl std::error::Error for CursorDecodeError {}

/// Where the previous page stopped. Serialized as opaque URL-safe base64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TS)]
pub struct Cursor {
    /// Timestamp of the last row consumed.
    #[ts(skip)]
    pub timestamp: i64,
    /// Id of the last row consumed, which breaks ties within one millisecond.
    #[ts(skip)]
    pub id: i64,
}
impl Cursor {
    pub fn encode(self) -> String {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&self.timestamp.to_be_bytes());
        bytes[8..].copy_from_slice(&self.id.to_be_bytes());
        encode_base64(&bytes)
    }
    pub fn decode(value: &str) -> Result<Self, CursorDecodeError> {
        let bytes = decode_base64(value)?;
        if bytes.len() != 16 {
            return Err(CursorDecodeError::InvalidLength);
        }
        let mut ts = [0; 8];
        let mut id = [0; 8];
        ts.copy_from_slice(&bytes[..8]);
        id.copy_from_slice(&bytes[8..]);
        let timestamp = i64::from_be_bytes(ts);
        let id = i64::from_be_bytes(id);
        if timestamp < 0 || id <= 0 {
            return Err(CursorDecodeError::InvalidParts);
        }
        Ok(Self { timestamp, id })
    }
}
impl Serialize for Cursor {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.encode())
    }
}
impl<'de> Deserialize<'de> for Cursor {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::decode(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
fn encode_base64(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        out.push(B64[(a >> 2) as usize] as char);
        out.push(B64[((a & 3) << 4 | b >> 4) as usize] as char);
        if chunk.len() > 1 {
            out.push(B64[((b & 15) << 2 | c >> 6) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(B64[(c & 63) as usize] as char);
        }
    }
    out
}
fn decode_base64(value: &str) -> Result<Vec<u8>, CursorDecodeError> {
    if value.is_empty() || value.len() > 24 || value.len() % 4 == 1 {
        return Err(CursorDecodeError::InvalidEncoding);
    }
    let mut out = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for byte in value.bytes() {
        let Some(index) = B64.iter().position(|candidate| *candidate == byte) else {
            return Err(CursorDecodeError::InvalidEncoding);
        };
        buffer = (buffer << 6) | index as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// A request for one page of events, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EventQuery {
    /// Canonical range selected by the caller.
    #[serde(default)]
    pub range: TimeRange,
    /// Resolved inclusive lower bound, retained for repository compatibility.
    #[serde(default, skip_serializing)]
    #[ts(skip)]
    pub from: i64,
    /// Resolved inclusive upper bound, retained for repository compatibility.
    #[serde(default, skip_serializing)]
    #[ts(skip)]
    pub to: i64,
    /// Only these categories; empty means all.
    #[serde(default)]
    pub categories: Vec<Category>,
    /// Only rows at least this important.
    #[serde(default)]
    pub importance_min: Option<Importance>,
    /// Only rows about this entity.
    #[serde(default)]
    pub entity_id: Option<String>,
    /// Continue after this row.
    #[serde(default)]
    pub cursor: Option<Cursor>,
    /// Rows wanted. Clamped to `MAX_PAGE_SIZE` by the repository, not trusted.
    pub limit: u16,
}

impl Default for EventQuery {
    fn default() -> Self {
        Self {
            range: TimeRange::default(),
            from: i64::MIN,
            to: i64::MAX,
            categories: Vec::new(),
            importance_min: None,
            entity_id: None,
            cursor: None,
            limit: DEFAULT_PAGE_SIZE,
        }
    }
}

impl EventQuery {
    /// The limit the repository will actually apply: at least one row, never
    /// more than `MAX_PAGE_SIZE`. No query may return unbounded rows.
    pub fn effective_limit(&self) -> u16 {
        self.limit.clamp(1, MAX_PAGE_SIZE)
    }
}

/// One timeline row: enough to render and to fetch details for, no more.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EventSummary {
    /// Database id.
    pub id: i64,
    /// Epoch milliseconds UTC.
    pub timestamp: i64,
    /// What part of the machine.
    pub category: Category,
    /// What happened.
    pub event_type: EventType,
    /// What kind of thing.
    pub entity_type: EntityType,
    /// Stable id of the thing.
    pub entity_id: String,
    /// The name a person recognises.
    pub entity_display_name: String,
    /// How bad.
    pub severity: Severity,
    /// Whether to care.
    pub importance: Importance,
    /// How sure.
    pub confidence: Confidence,
    /// Which collector.
    pub source_collector: String,
    /// State before the change, already redacted. Never file contents.
    pub before: Option<serde_json::Value>,
    /// State after the change, already redacted. Never file contents.
    pub after: Option<serde_json::Value>,
    /// Structured config key for a provable key-level change.
    pub field: Option<String>,
    /// One sentence in the product's voice.
    pub user_friendly_summary: String,
    /// The timestamp is a detection time, not an occurrence time.
    pub occurred_at_is_uncertain: bool,
}

/// One page of results and where the next one starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct Page<T> {
    /// The rows, newest first.
    pub items: Vec<T>,
    /// Cursor for the next page, or `None` when this was the last one.
    pub next: Option<Cursor>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_limit_is_clamped_whatever_was_asked_for() {
        let mut query = EventQuery::default();
        assert_eq!(query.effective_limit(), DEFAULT_PAGE_SIZE);
        query.limit = 0;
        assert_eq!(query.effective_limit(), 1);
        query.limit = u16::MAX;
        assert_eq!(query.effective_limit(), MAX_PAGE_SIZE);
    }
}
