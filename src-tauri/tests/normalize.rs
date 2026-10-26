#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::json;
use what_changed_lib::collectors::normalize::Normalizer;
use what_changed_lib::collectors::{EntityIdentity, ObservationKind, RawEntity, RawObservation};
use what_changed_lib::domain::{Category, ClassifierContext, Confidence, EventType, Severity};
use what_changed_lib::privacy::patterns::REDACTED;
use what_changed_lib::storage::{event_repository, Db};

#[test]
fn entity_grammar_and_secrets_are_normalized_before_storage() {
    let raw = RawObservation {
        collector_id: "files",
        kind: ObservationKind {
            category: Category::Configuration,
            event_type: EventType::Modified,
            severity: Severity::Info,
            confidence: Confidence::High,
            user_friendly_summary: "API token changed".to_owned(),
            technical_summary: "token sk-abcdefghijklmnopqrstuvwx changed".to_owned(),
            classifier: ClassifierContext::default(),
        },
        entity: RawEntity {
            identity: EntityIdentity::File {
                path: r"C:\work\.env".to_owned(),
            },
            display_name: ".env".to_owned(),
        },
        before: Some(json!({"API_KEY":"sk-abcdefghijklmnopqrstuvwx"})),
        after: Some(json!({"API_KEY":"sk-zyxwvutsrqponmlkjihgfedc"})),
        field: None,
        occurred_at: 1_700_000_000_000,
        details: Some(json!({"note":"sk-abcdefghijklmnopqrstuvwx"})),
        related_entity_ids: Vec::new(),
        occurred_at_is_uncertain: false,
    };
    let event = Normalizer::new().normalize(raw);
    assert!(event.get().entity_id.starts_with("file:"));
    assert!(event.get().sensitive);
    let db = Db::open_in(&what_changed_lib::testing::temp_dir("normalize-storage")).unwrap();
    let ids = db.insert_events(vec![event]).unwrap();
    let stored = db
        .read(|conn| event_repository::by_id(conn, ids[0]))
        .unwrap()
        .unwrap();
    assert_eq!(stored.event.before.unwrap()["API_KEY"], json!(REDACTED));
    assert_eq!(stored.event.metadata.unwrap()["note"], json!(REDACTED));
    assert!(!stored.event.technical_summary.contains("sk-"));
}
