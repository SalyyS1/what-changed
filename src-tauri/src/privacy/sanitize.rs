//! The chokepoint: a value reaches storage only inside `Sanitized<T>`, and the
//! only way to make one is to go through `Sanitizer`.
//!
//! `Sanitized<T>` has a private field and no `new`, no `From`, no `Deref`. A
//! repository whose signature takes `&[Sanitized<Event>]` therefore cannot be
//! handed an unscrubbed event: it is a compile error, not a review comment.
//! Reading is unrestricted — the guarantee is about what can be constructed,
//! not about what can be looked at.

use serde_json::Value;

use super::keys::is_sensitive_key;
use super::patterns::{Kind, REDACTED};
use super::scrub::scrub_value;
use crate::domain::{Entity, NormalizedEvent};

/// One replacement, for the metadata a user sees when a value is hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redaction {
    /// Field or key path whose value was replaced. Never the value itself.
    pub field: String,
    /// Which rule fired.
    pub kind: &'static str,
}

impl Redaction {
    fn new(field: impl Into<String>, kind: Kind) -> Self {
        Self {
            field: field.into(),
            kind: kind.as_str(),
        }
    }
}

/// A value that has been through redaction.
///
/// The wrapper is the proof. It cannot be constructed outside this module, so
/// possessing one means the scrubber ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sanitized<T>(T);

impl<T> Sanitized<T> {
    /// Borrow the scrubbed value, for assertions and for serialization.
    pub fn get(&self) -> &T {
        &self.0
    }

    /// Take ownership of the scrubbed value. Restricted to this crate: the
    /// repository is the only intended caller, because taking the value out is
    /// how it reaches a sink. That repository does not exist yet, which is why
    /// the method has no caller in this crate today.
    #[allow(dead_code)]
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}

/// Applies the redaction rules to a value of any shape it knows.
///
/// Stateless today; it exists as a value rather than a set of free functions so
/// a future user-configured deny-list has one place to live and every call site
/// already threads it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Sanitizer;

impl Sanitizer {
    /// A sanitizer with the built-in rules.
    pub fn new() -> Self {
        Self
    }

    /// Scrub a value and wrap it. The returned `Vec` names what was replaced,
    /// so a caller can record *why* a field is hidden without keeping any part
    /// of the original.
    pub fn sanitize<T: Scrub>(&self, mut value: T) -> (Sanitized<T>, Vec<Redaction>) {
        let mut report = Vec::new();
        value.scrub_in_place(self, "", &mut report);
        (Sanitized(value), report)
    }
}

/// Types that know how to scrub themselves in place.
///
/// `field` is the path of the value being scrubbed, used for the report and —
/// for a bare string — as the key the name rules see.
pub trait Scrub {
    /// Replace every secret inside `self`, appending one entry per replacement.
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>);
}

impl Scrub for String {
    fn scrub_in_place(&mut self, _sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        let key = if field.is_empty() { None } else { Some(field) };
        let scrubbed = scrub_value(key, self);
        if scrubbed.redacted() {
            for kind in &scrubbed.kinds {
                report.push(Redaction::new(field, *kind));
            }
            *self = scrubbed.text.into_owned();
        }
    }
}

impl<T: Scrub> Scrub for Option<T> {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        if let Some(inner) = self.as_mut() {
            inner.scrub_in_place(sanitizer, field, report);
        }
    }
}

impl<T: Scrub> Scrub for Vec<T> {
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        for (index, item) in self.iter_mut().enumerate() {
            item.scrub_in_place(sanitizer, &format!("{field}[{index}]"), report);
        }
    }
}

impl Scrub for NormalizedEvent {
    /// Every text field an event carries, by its own field name, so a
    /// redaction report reads `metadata.env.DB_PASSWORD` rather than a
    /// position. The `sensitive` flag is set when anything was replaced,
    /// which is how the timeline can say a value was hidden.
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        let seen = report.len();
        let path = |name: &str| {
            if field.is_empty() {
                name.to_owned()
            } else {
                format!("{field}.{name}")
            }
        };
        self.entity_display_name
            .scrub_in_place(sanitizer, &path("entity_display_name"), report);
        self.before
            .scrub_in_place(sanitizer, &path("before"), report);
        self.after.scrub_in_place(sanitizer, &path("after"), report);
        self.field.scrub_in_place(sanitizer, &path("field"), report);
        self.metadata
            .scrub_in_place(sanitizer, &path("metadata"), report);
        self.related_entity_ids
            .scrub_in_place(sanitizer, &path("related_entity_ids"), report);
        self.user_friendly_summary.scrub_in_place(
            sanitizer,
            &path("user_friendly_summary"),
            report,
        );
        self.technical_summary
            .scrub_in_place(sanitizer, &path("technical_summary"), report);
        if report.len() > seen {
            self.sensitive = true;
        }
    }
}

impl Scrub for Entity {
    /// Stable identity, type and observation times are structural facts. Only
    /// the person-facing name and collector metadata may carry values that
    /// need redaction before an endpoint reaches storage.
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        let path = |name: &str| {
            if field.is_empty() {
                name.to_owned()
            } else {
                format!("{field}.{name}")
            }
        };
        self.display_name
            .scrub_in_place(sanitizer, &path("display_name"), report);
        self.metadata
            .scrub_in_place(sanitizer, &path("metadata"), report);
    }
}

impl Scrub for Value {
    /// Recursive and key-aware. A sensitive key replaces whatever sits under
    /// it — string, number, object or array — because the shape of a secret is
    /// not the point once its name has given it away.
    fn scrub_in_place(&mut self, sanitizer: &Sanitizer, field: &str, report: &mut Vec<Redaction>) {
        match self {
            Value::String(text) => {
                let mut owned = std::mem::take(text);
                owned.scrub_in_place(sanitizer, field, report);
                *text = owned;
            }
            Value::Array(items) => {
                for (index, item) in items.iter_mut().enumerate() {
                    let path = format!("{field}[{index}]");
                    item.scrub_in_place(sanitizer, &path, report);
                }
            }
            Value::Object(map) => {
                for (key, value) in map.iter_mut() {
                    let path = if field.is_empty() {
                        key.clone()
                    } else {
                        format!("{field}.{key}")
                    };
                    if is_sensitive_key(key) {
                        if *value != Value::String(REDACTED.to_owned()) {
                            report.push(Redaction::new(&path, Kind::KeyName));
                            *value = Value::String(REDACTED.to_owned());
                        }
                    } else {
                        value.scrub_in_place(sanitizer, &path, report);
                    }
                }
            }
            // Numbers, booleans and null carry no text to scrub. A number is
            // never redacted: ports, sizes and counts are the evidence.
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_bare_string_is_scrubbed_by_shape() {
        let (sanitized, report) =
            Sanitizer::new().sanitize(String::from("token sk-abcdefghijklmnopqrstuvwx"));
        assert_eq!(sanitized.get(), &format!("token {REDACTED}"));
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].kind, "api_key");
    }

    #[test]
    fn an_absent_option_is_left_alone() {
        let (sanitized, report) = Sanitizer::new().sanitize(Option::<String>::None);
        assert_eq!(sanitized.get(), &None);
        assert!(report.is_empty());
    }

    #[test]
    fn json_redacts_by_key_at_any_depth_and_names_the_path() {
        let (sanitized, report) = Sanitizer::new().sanitize(json!({
            "service": "docker",
            "env": { "DB_PASSWORD": "hunter2", "API_PORT": 8080 },
            "args": ["--token", "ghp_abcdefghijklmnopqrstuvwxyz0123456789"]
        }));
        let value = sanitized.get();
        assert_eq!(value["env"]["DB_PASSWORD"], json!(REDACTED));
        assert_eq!(value["env"]["API_PORT"], json!(8080), "a port is evidence");
        assert_eq!(value["service"], json!("docker"));
        assert_eq!(value["args"][1], json!(REDACTED));

        let fields: Vec<&str> = report.iter().map(|r| r.field.as_str()).collect();
        assert!(fields.contains(&"env.DB_PASSWORD"), "{fields:?}");
        assert!(fields.contains(&"args[1]"), "{fields:?}");
    }

    #[test]
    fn a_sensitive_key_hides_a_whole_subtree() {
        let (sanitized, report) = Sanitizer::new().sanitize(json!({
            "credentials": { "user": "app", "pass": "hunter2" }
        }));
        assert_eq!(sanitized.get()["credentials"], json!(REDACTED));
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].kind, "key_name");
    }

    #[test]
    fn an_event_is_scrubbed_field_by_field_and_marked_sensitive() {
        use crate::domain::{Category, Confidence, EntityType, EventType, Importance, Severity};
        let event = NormalizedEvent {
            timestamp: 1,
            category: Category::Configuration,
            event_type: EventType::ConfigurationChanged,
            entity_type: EntityType::File,
            entity_id: "file:abc".to_owned(),
            entity_display_name: ".env".to_owned(),
            severity: Severity::Info,
            importance: Importance::Notable,
            source_collector: "files".to_owned(),
            before: Some(json!({ "OPENAI_API_KEY": "sk-abcdefghijklmnopqrstuvwx" })),
            after: Some(json!({ "OPENAI_API_KEY": "sk-zyxwvutsrqponmlkjihgfedc" })),
            field: Some("OPENAI_API_KEY".to_owned()),
            metadata: Some(json!({ "path": "C:\\p\\.env", "changed": true })),
            related_entity_ids: vec!["port:tcp:0.0.0.0:3000".to_owned()],
            user_friendly_summary: "OPENAI_API_KEY changed in .env".to_owned(),
            technical_summary: "token sk-abcdefghijklmnopqrstuvwx rotated".to_owned(),
            confidence: Confidence::High,
            sensitive: false,
            occurred_at_is_uncertain: false,
        };
        let (sanitized, report) = Sanitizer::new().sanitize(event);
        let event = sanitized.get();
        assert!(event.sensitive);
        assert_eq!(
            event.before.as_ref().unwrap()["OPENAI_API_KEY"],
            json!(REDACTED)
        );
        assert_eq!(
            event.after.as_ref().unwrap()["OPENAI_API_KEY"],
            json!(REDACTED)
        );
        assert_eq!(event.technical_summary, format!("token {REDACTED} rotated"));
        assert_eq!(event.metadata.as_ref().unwrap()["changed"], json!(true));
        assert_eq!(event.related_entity_ids, vec!["port:tcp:0.0.0.0:3000"]);
        let fields: Vec<&str> = report.iter().map(|r| r.field.as_str()).collect();
        assert!(fields.contains(&"before.OPENAI_API_KEY"), "{fields:?}");
        assert!(fields.contains(&"technical_summary"), "{fields:?}");
    }

    #[test]
    fn scrubbing_is_idempotent() {
        let sanitizer = Sanitizer::new();
        let (once, _) = sanitizer.sanitize(json!({ "API_KEY": "sk-abcdefghijklmnopqrstuvwx" }));
        let (twice, report) = sanitizer.sanitize(once.get().clone());
        assert_eq!(once.get(), twice.get());
        assert!(
            report.is_empty(),
            "a second pass must find nothing left to do"
        );
    }

    #[test]
    fn an_entity_scrubs_only_its_display_and_metadata() {
        use crate::domain::EntityType;

        let original_id = "process:stable:sk-abcdefghijklmnopqrstuvwx";
        let (entity, report) = Sanitizer::new().sanitize(Entity {
            id: original_id.to_owned(),
            entity_type: EntityType::Process,
            display_name: "token sk-abcdefghijklmnopqrstuvwx".to_owned(),
            first_seen: 10,
            last_seen: 20,
            metadata: Some(serde_json::json!({ "DB_PASSWORD": "hunter2" })),
        });

        assert_eq!(entity.get().id, original_id);
        assert_eq!(entity.get().entity_type, EntityType::Process);
        assert_eq!(entity.get().first_seen, 10);
        assert_eq!(entity.get().last_seen, 20);
        assert_eq!(entity.get().display_name, format!("token {REDACTED}"));
        assert_eq!(
            entity.get().metadata.as_ref().unwrap()["DB_PASSWORD"],
            serde_json::json!(REDACTED)
        );
        let fields: Vec<&str> = report.iter().map(|item| item.field.as_str()).collect();
        assert!(fields.contains(&"display_name"));
        assert!(fields.contains(&"metadata.DB_PASSWORD"));
    }
}
