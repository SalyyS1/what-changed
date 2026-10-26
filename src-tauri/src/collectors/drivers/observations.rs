use serde_json::json;

use crate::collectors::{EntityIdentity, ObservationKind, RawEntity, RawObservation};
use crate::domain::driver_naming;
use crate::domain::{Category, ClassifierContext, Confidence, EventType, Severity};

use super::diff::{Change, VersionDirection};
use super::DriverRecord;

pub(super) fn for_change(
    change: &Change,
    occurred_at: i64,
    uncertain: bool,
) -> Option<RawObservation> {
    match change {
        Change::Installed(row) => Some(observation(
            row,
            EventType::Installed,
            format!("{} installed", name(row)),
            "a present SetupAPI device acquired a driver entry".to_owned(),
            None,
            row.driver_version.as_ref().map(|value| json!(value)),
            None,
            occurred_at,
            uncertain,
            false,
            "installed",
        )),
        Change::Removed(row) => Some(observation(
            row,
            EventType::Uninstalled,
            format!("{} removed", name(row)),
            "a driver-backed device disappeared from the present-device table".to_owned(),
            row.driver_version.as_ref().map(|value| json!(value)),
            None,
            None,
            occurred_at,
            uncertain,
            false,
            "removed",
        )),
        Change::Updated {
            before,
            after,
            direction,
        } => {
            let old = before
                .driver_version
                .as_deref()
                .unwrap_or("unknown version");
            let new = after.driver_version.as_deref().unwrap_or("unknown version");
            let rolled_back = *direction == VersionDirection::Decreased;
            Some(observation(
                after,
                if rolled_back {
                    // The shared P02 vocabulary has no `rolled_back` variant.
                    // Preserve that exact subtype in details without silently
                    // claiming an ordinary forward update.
                    EventType::VersionChanged
                } else {
                    EventType::Updated
                },
                format!(
                    "{} {} {old} → {new}",
                    name(after),
                    if rolled_back {
                        "rolled back"
                    } else {
                        "updated"
                    }
                ),
                format!("driver version changed from {old} to {new}"),
                before.driver_version.as_ref().map(|value| json!(value)),
                after.driver_version.as_ref().map(|value| json!(value)),
                Some("version".to_owned()),
                occurred_at,
                uncertain,
                true,
                if rolled_back {
                    "rolled_back"
                } else {
                    "updated"
                },
            ))
        }
        Change::Metadata(_) => None,
    }
}

fn name(row: &DriverRecord) -> String {
    driver_naming::friendly_name(&row.manufacturer, &row.class_name)
}

#[allow(clippy::too_many_arguments)]
fn observation(
    row: &DriverRecord,
    event_type: EventType,
    user_friendly_summary: String,
    technical_summary: String,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    field: Option<String>,
    occurred_at: i64,
    occurred_at_is_uncertain: bool,
    relevant_driver: bool,
    change_kind: &'static str,
) -> RawObservation {
    RawObservation {
        collector_id: super::ID,
        kind: ObservationKind {
            category: Category::Driver,
            event_type,
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary,
            technical_summary,
            classifier: ClassifierContext {
                relevant_driver,
                ..ClassifierContext::default()
            },
        },
        entity: RawEntity {
            identity: EntityIdentity::Driver {
                device_instance_id: row.device_instance_id.clone(),
            },
            display_name: row.device_description.clone(),
        },
        before,
        after,
        field,
        occurred_at,
        details: Some(json!({
            "change_kind": change_kind,
            "class": row.class_name,
            "manufacturer": row.manufacturer,
            "provider": row.provider_name,
            "driver_date": row.driver_date,
        })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain,
    }
}
