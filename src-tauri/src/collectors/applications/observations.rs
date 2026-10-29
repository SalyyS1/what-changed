use serde_json::json;

use crate::collectors::{EntityIdentity, ObservationKind, RawEntity, RawObservation};
use crate::domain::{Category, ClassifierContext, Confidence, EventType, Severity};

use super::diff::Change;
use super::TrackedApplication;

pub(super) fn for_change(
    change: &Change,
    occurred_at: i64,
    uncertain: bool,
) -> Option<RawObservation> {
    match change {
        Change::Installed(app) => Some(observation(
            app,
            EventType::Installed,
            format!("{} was installed", app.display_name),
            "an uninstall registry entry appeared".to_owned(),
            None,
            app.display_version.as_ref().map(|version| json!(version)),
            None,
            occurred_at,
            uncertain,
        )),
        Change::Removed(app) => Some(observation(
            app,
            EventType::Uninstalled,
            format!("{} was removed", app.display_name),
            "an uninstall registry entry disappeared".to_owned(),
            app.display_version.as_ref().map(|version| json!(version)),
            None,
            None,
            occurred_at,
            uncertain,
        )),
        Change::Updated { before, after } => {
            let old = before
                .display_version
                .as_deref()
                .unwrap_or("unknown version");
            let new = after
                .display_version
                .as_deref()
                .unwrap_or("unknown version");
            Some(observation(
                after,
                EventType::Updated,
                format!("{} updated {old} → {new}", after.display_name),
                format!("application version changed from {old} to {new}"),
                before
                    .display_version
                    .as_ref()
                    .map(|version| json!(version)),
                after.display_version.as_ref().map(|version| json!(version)),
                Some("version".to_owned()),
                occurred_at,
                uncertain,
            ))
        }
        Change::Metadata(_) => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn observation(
    app: &TrackedApplication,
    event_type: EventType,
    user_friendly_summary: String,
    technical_summary: String,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
    field: Option<String>,
    occurred_at: i64,
    occurred_at_is_uncertain: bool,
) -> RawObservation {
    RawObservation {
        collector_id: super::ID,
        kind: ObservationKind {
            category: Category::Application,
            event_type,
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary,
            technical_summary,
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            identity: EntityIdentity::Application {
                hive: app.hive.identity().to_owned(),
                registry_key: app.subkey_name.clone(),
            },
            display_name: app.display_name.clone(),
        },
        before,
        after,
        field,
        occurred_at,
        details: Some(json!({
            "publisher": app.publisher,
            "install_date": app.install_date,
        })),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain,
    }
}
