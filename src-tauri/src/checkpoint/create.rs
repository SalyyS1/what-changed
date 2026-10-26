//! Pure checkpoint capture preparation. Collector orchestration owns timeouts
//! and progress; this module owns naming, manifests and mandatory sanitation.

use chrono::{Local, TimeZone, Timelike};

use crate::domain::{CheckpointCapture, CheckpointItem, CheckpointKind, NewCheckpoint};
use crate::privacy::{Redaction, Sanitized, Sanitizer};

/// Generate the local, human-readable default required by the product brief.
pub fn default_checkpoint_name(created_at: i64) -> String {
    let local = Local
        .timestamp_millis_opt(created_at)
        .single()
        .unwrap_or_else(Local::now);
    let hour = match local.hour() % 12 {
        0 => 12,
        hour => hour,
    };
    let meridiem = if local.hour() < 12 { "AM" } else { "PM" };
    format!(
        "Checkpoint {} {hour}:{:02} {meridiem}",
        local.format("%A"),
        local.minute()
    )
}

/// Assemble and sanitize the one aggregate the repository accepts. Collector
/// manifests are normalized so valid-empty results remain distinguishable
/// from failed/missing collectors.
#[allow(clippy::too_many_arguments)]
pub fn prepare_capture(
    name: Option<String>,
    kind: CheckpointKind,
    reason: Option<String>,
    created_at: i64,
    duration_ms: u64,
    mut captured_collectors: Vec<String>,
    mut missing_collectors: Vec<String>,
    items: Vec<CheckpointItem>,
    sanitizer: &Sanitizer,
) -> (Sanitized<CheckpointCapture>, Vec<Redaction>) {
    captured_collectors.sort();
    captured_collectors.dedup();
    missing_collectors.sort();
    missing_collectors.dedup();
    sanitizer.sanitize(CheckpointCapture {
        checkpoint: NewCheckpoint {
            name: name.unwrap_or_else(|| default_checkpoint_name(created_at)),
            kind,
            reason,
            created_at,
            duration_ms,
            missing_collectors,
            captured_collectors,
        },
        items,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn preparation_sanitizes_state_and_normalizes_manifests() {
        let (capture, report) = prepare_capture(
            Some("Known good".to_owned()),
            CheckpointKind::Manual,
            None,
            1,
            2,
            vec!["services".to_owned(), "services".to_owned()],
            vec!["drivers".to_owned(), "drivers".to_owned()],
            vec![CheckpointItem {
                collector_id: "services".to_owned(),
                entity_kind: "service".to_owned(),
                entity_external_id: "svc".to_owned(),
                entity_name: Some("Example".to_owned()),
                state: json!({"api_key": "sk-abcdefghijklmnopqrstuvwx"}),
            }],
            &Sanitizer::new(),
        );
        assert_eq!(capture.get().checkpoint.captured_collectors, ["services"]);
        assert_eq!(capture.get().checkpoint.missing_collectors, ["drivers"]);
        assert!(!report.is_empty());
        assert!(!capture.get().items[0].state.to_string().contains("sk-"));
    }
}
