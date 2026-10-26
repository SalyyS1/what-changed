use std::path::Path;

use serde_json::json;

use crate::collectors::{EntityIdentity, ObservationKind, RawEntity, RawObservation};
use crate::domain::{Category, ClassifierContext, Confidence, EventType, Severity};

use super::ID;

pub(super) fn generic(
    path: &Path,
    event_type: EventType,
    size: Option<u64>,
    modified_at: Option<i64>,
    occurred_at: i64,
    uncertain: bool,
) -> RawObservation {
    let action = match event_type {
        EventType::Created => "created",
        EventType::Deleted => "deleted",
        EventType::Renamed => "renamed",
        _ => "modified",
    };
    observation(
        path,
        Category::Filesystem,
        event_type,
        Confidence::High,
        format!("{} was {action}", name(path)),
        format!("{}: {}", action, path.display()),
        None,
        None,
        None,
        Some(json!({ "path": path.to_string_lossy(), "size": size, "modified_at": modified_at })),
        occurred_at,
        uncertain,
    )
}

pub(super) fn renamed(
    from: &Path,
    to: &Path,
    size: Option<u64>,
    modified_at: Option<i64>,
    occurred_at: i64,
) -> RawObservation {
    let mut row = generic(
        to,
        EventType::Renamed,
        size,
        modified_at,
        occurred_at,
        false,
    );
    row.kind.user_friendly_summary = format!("{} was renamed", name(to));
    row.kind.technical_summary = format!("renamed {} to {}", from.display(), to.display());
    row.details = Some(json!({
        "path": to.to_string_lossy(),
        "from": from.to_string_lossy(),
        "to": to.to_string_lossy(),
        "size": size,
        "modified_at": modified_at,
    }));
    row
}

pub(super) fn config_change(
    path: &Path,
    field: String,
    before: Option<String>,
    after: Option<String>,
    occurred_at: i64,
    uncertain: bool,
) -> RawObservation {
    observation(
        path,
        Category::Configuration,
        EventType::ConfigurationChanged,
        Confidence::High,
        format!("{} changed in {}", field, name(path)),
        format!("configuration key {} changed in {}", field, path.display()),
        before.map(serde_json::Value::String),
        after.map(serde_json::Value::String),
        Some(field),
        Some(json!({ "path": path.to_string_lossy() })),
        occurred_at,
        uncertain,
    )
}

pub(super) fn config_summary(
    path: &Path,
    omitted: usize,
    occurred_at: i64,
    uncertain: bool,
) -> RawObservation {
    observation(
        path,
        Category::Configuration,
        EventType::ConfigurationChanged,
        Confidence::High,
        format!(
            "{omitted} more configuration keys changed in {}",
            name(path)
        ),
        format!("and {omitted} more keys changed in {}", path.display()),
        None,
        None,
        None,
        Some(json!({ "path": path.to_string_lossy(), "omitted_key_changes": omitted })),
        occurred_at,
        uncertain,
    )
}

pub(super) fn sensitive_uncertain(
    path: &Path,
    occurred_at: i64,
    uncertain: bool,
) -> RawObservation {
    observation(
        path,
        Category::Configuration,
        EventType::ConfigurationChanged,
        Confidence::Low,
        format!("Sensitive configuration may have changed in {}", name(path)),
        format!(
            "a retained redacted value may have changed in {}",
            path.display()
        ),
        None,
        None,
        None,
        Some(json!({ "path": path.to_string_lossy(), "sensitive_change_unprovable": true })),
        occurred_at,
        uncertain,
    )
}

#[allow(clippy::too_many_arguments)]
fn observation(
    path: &Path,
    category: Category,
    event_type: EventType,
    confidence: Confidence,
    user_friendly_summary: String,
    technical_summary: String,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    field: Option<String>,
    details: Option<serde_json::Value>,
    occurred_at: i64,
    uncertain: bool,
) -> RawObservation {
    RawObservation {
        collector_id: ID,
        kind: ObservationKind {
            category,
            event_type,
            severity: Severity::Info,
            confidence,
            user_friendly_summary,
            technical_summary,
            classifier: ClassifierContext::default(),
        },
        entity: file_entity(path),
        before,
        after,
        field,
        occurred_at,
        details,
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: uncertain,
    }
}

pub(super) fn file_entity(path: &Path) -> RawEntity {
    RawEntity {
        identity: EntityIdentity::File {
            path: path.to_string_lossy().into_owned(),
        },
        display_name: name(path),
    }
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}
