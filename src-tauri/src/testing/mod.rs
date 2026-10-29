//! Test support. Compiled for unit tests and, behind the `devtools` feature,
//! for integration tests and development tooling — never into a shipped
//! build.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Once;
use std::time::{Duration, SystemTime};

use serde_json::json;

use crate::domain::{
    classify, Category, ClassifierContext, Confidence, EntityType, EventType, NormalizedEvent,
    Severity,
};

pub mod synthetic;

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// The prefix every directory this helper creates carries, so the sweep below
/// recognises its own leftovers and nothing else in the temporary directory.
const PREFIX: &str = "what-changed-test-";

/// How long a leftover directory survives: long enough to open the files from
/// the run that just failed, short enough that a week of runs cannot fill a
/// volume. One full suite leaves roughly sixty megabytes behind.
const KEEP: Duration = Duration::from_secs(60 * 60);

static SWEPT: Once = Once::new();

/// The original deterministic corpus vocabulary.
///
/// `churn` is a collector-produced summary rather than a primitive synthetic
/// fact. Keeping the Phase 06 sequence stable prevents adding that event type
/// from silently changing every seeded performance and UI fixture.
const SYNTHETIC_EVENT_TYPES: [EventType; 19] = [
    EventType::Created,
    EventType::Deleted,
    EventType::Modified,
    EventType::Started,
    EventType::Stopped,
    EventType::Restarted,
    EventType::Installed,
    EventType::Uninstalled,
    EventType::Updated,
    EventType::Enabled,
    EventType::Disabled,
    EventType::Connected,
    EventType::Disconnected,
    EventType::Opened,
    EventType::Closed,
    EventType::Failed,
    EventType::Recovered,
    EventType::ConfigurationChanged,
    EventType::VersionChanged,
];

/// Delete this helper's directories from earlier runs, once per process and
/// best effort: a directory another process still holds open simply stays until
/// the next sweep.
fn sweep_stale() {
    let Ok(entries) = std::fs::read_dir(std::env::temp_dir()) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(PREFIX) {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .map(|changed| {
                now.duration_since(changed)
                    .map(|age| age > KEEP)
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// A fresh, empty directory under the system temporary directory. Unique per
/// call so tests can run in parallel, and kept after the test so a failure
/// leaves its files to look at; a later run sweeps what is older than an hour.
pub fn temp_dir(label: &str) -> PathBuf {
    SWEPT.call_once(sweep_stale);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!(
        "{PREFIX}{label}-{}-{n}-{nanos}",
        std::process::id()
    ));
    // A directory that cannot be created will surface at the first file
    // write, with a better error than an unwrap here would give.
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// A realistic event with deterministic content derived from `seed`.
///
/// Cycles through the categories and event types so a corpus built from
/// consecutive seeds exercises every index the schema has.
pub fn synthetic_event(seed: u64, timestamp: i64) -> NormalizedEvent {
    let category = Category::ALL[(seed % Category::ALL.len() as u64) as usize];
    let event_type = SYNTHETIC_EVENT_TYPES[(seed % SYNTHETIC_EVENT_TYPES.len() as u64) as usize];
    let entity_index = seed % 37;
    let entity_type = match category {
        Category::Application => EntityType::Application,
        Category::Process => EntityType::Process,
        Category::Filesystem | Category::Configuration | Category::Git => EntityType::File,
        Category::Network => EntityType::Adapter,
        Category::Service => EntityType::Service,
        Category::Driver => EntityType::Driver,
        Category::Update => EntityType::Update,
        Category::Docker => EntityType::Container,
        _ => EntityType::System,
    };
    let display_name = match seed % 12 {
        0 => "Discord.exe",
        1 => "NVIDIA nvlddmkm display driver",
        2 => "nginx",
        3 => "DNS resolver",
        4 => "Minecraft server 25565",
        5 => "Audiosrv",
        6 => "docker-compose.yml",
        _ => "Synthetic Service",
    };
    let mut event = NormalizedEvent {
        timestamp,
        category,
        event_type,
        entity_type,
        entity_id: format!("{}:Synthetic{entity_index}", entity_type.as_str()),
        entity_display_name: format!("{display_name} {entity_index}"),
        severity: Severity::Info,
        importance: crate::domain::Importance::Informational,
        source_collector: "synthetic".to_owned(),
        before: Some(json!({ "state": "running", "seed": seed })),
        after: Some(json!({ "state": "stopped", "seed": seed })),
        field: None,
        metadata: Some(json!({ "port": 25565 + (seed % 100), "path": "C:\\Users\\dev\\project" })),
        related_entity_ids: vec![format!("port:tcp:0.0.0.0:{}", 25565 + (seed % 100))],
        user_friendly_summary: format!("{display_name} {}", event_type.as_str()),
        technical_summary: format!("{display_name}: {} (seed {seed})", event_type.as_str()),
        confidence: Confidence::High,
        sensitive: false,
        occurred_at_is_uncertain: false,
    };
    let context = ClassifierContext {
        automatic_service: category == Category::Service
            && event_type == EventType::Stopped
            && seed % 97 == 0,
        relevant_driver: category == Category::Driver
            && event_type == EventType::Updated
            && seed % 89 == 0,
        checkpoint_port: entity_type == EntityType::Port && event_type == EventType::Closed,
        unexpected_shutdown: false,
        // One synthetic event is not a browser subprocess; the damping rule is
        // covered where the rule lives.
        browser_subprocess: false,
    };
    event.importance = classify(&event, &context);
    event
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn adding_collector_summaries_does_not_resequence_legacy_corpora() {
        assert_eq!(synthetic_event(18, 0).event_type, EventType::VersionChanged);
        assert_eq!(synthetic_event(19, 0).event_type, EventType::Created);
        assert!((0..SYNTHETIC_EVENT_TYPES.len() as u64)
            .all(|seed| synthetic_event(seed, 0).event_type != EventType::Churn));
    }
}
