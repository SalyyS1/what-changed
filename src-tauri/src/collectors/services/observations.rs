use serde_json::json;

use super::diff::Change;
use super::{ServiceRecord, ID};
use crate::collectors::{EntityIdentity, ObservationKind, RawEntity, RawObservation};
use crate::domain::{Category, ClassifierContext, Confidence, EventType, Severity};

pub fn for_change(change: &Change, now_ms: i64, uncertain: bool) -> RawObservation {
    match change {
        Change::Installed(record) => observation(
            record,
            EventType::Installed,
            Severity::Info,
            format!("{} was installed", record.display_name),
            format!("service {} installed", record.name),
            None,
            Some(snapshot(record)),
            None,
            now_ms,
            uncertain,
        ),
        Change::Removed(record) => observation(
            record,
            EventType::Uninstalled,
            Severity::Info,
            format!("{} was removed", record.display_name),
            format!("service {} removed", record.name),
            Some(snapshot(record)),
            None,
            None,
            now_ms,
            uncertain,
        ),
        Change::Started { before, after } => observation(
            after,
            EventType::Started,
            Severity::Info,
            format!("{} started", after.display_name),
            format!("service {}: stopped -> running", after.name),
            Some(json!(before.state.label())),
            Some(json!(after.state.label())),
            Some("state"),
            now_ms,
            uncertain,
        ),
        Change::Stopped { before, after } => observation(
            after,
            EventType::Stopped,
            Severity::Warning,
            format!("{} stopped", after.display_name),
            format!("service {}: running -> stopped", after.name),
            Some(json!(before.state.label())),
            Some(json!(after.state.label())),
            Some("state"),
            now_ms,
            uncertain,
        ),
        Change::StartTypeChanged { before, after } => observation(
            after,
            EventType::ConfigurationChanged,
            Severity::Info,
            format!(
                "{} start type changed {} → {}",
                after.display_name,
                before.start_type.label(),
                after.start_type.label()
            ),
            format!(
                "service {} start type: {} -> {}",
                after.name,
                before.start_type.label(),
                after.start_type.label()
            ),
            Some(json!(before.start_type.label())),
            Some(json!(after.start_type.label())),
            Some("start_type"),
            now_ms,
            uncertain,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn observation(
    record: &ServiceRecord,
    event_type: EventType,
    severity: Severity,
    user_friendly_summary: String,
    technical_summary: String,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    field: Option<&str>,
    now_ms: i64,
    uncertain: bool,
) -> RawObservation {
    RawObservation {
        collector_id: ID,
        kind: ObservationKind {
            category: Category::Service,
            event_type,
            severity,
            confidence: Confidence::High,
            user_friendly_summary,
            technical_summary,
            classifier: ClassifierContext {
                automatic_service: event_type == EventType::Stopped
                    && record.start_type == super::ServiceStartType::Automatic,
                ..Default::default()
            },
        },
        entity: RawEntity {
            identity: EntityIdentity::Service {
                name: record.name.clone(),
            },
            display_name: record.display_name.clone(),
        },
        before,
        after,
        field: field.map(str::to_owned),
        occurred_at: now_ms,
        details: Some(json!({
            "service_name": record.name,
            "process_id": record.process_id,
            "binary_path": record.binary_path,
            "state": record.state.label(),
            "start_type": record.start_type.label(),
            "config_available": record.config_available,
        })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: uncertain,
    }
}

fn snapshot(record: &ServiceRecord) -> serde_json::Value {
    json!({
        "state": record.state.label(),
        "start_type": record.start_type.label(),
    })
}
