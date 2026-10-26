#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;

use what_changed_lib::collectors::config_diff::{
    self, ConfigDiff, ConfigDiffItem, ConfigError, ReadOutcome, MAX_KEY_EVENTS,
};
use what_changed_lib::collectors::ignore_set::MAX_FILE_BYTES;
use what_changed_lib::privacy::REDACTED;
use what_changed_lib::testing::temp_dir;

fn parsed(path: &str, bytes: &[u8]) -> config_diff::ParsedConfig {
    match config_diff::parse(Path::new(path), bytes).unwrap() {
        ReadOutcome::Parsed(parsed) => parsed,
        other => panic!("expected parsed config, got {other:?}"),
    }
}

#[test]
fn every_supported_format_flattens_to_the_same_key_shape() {
    let cases: &[(&str, &[u8], &str, &str)] = &[
        (
            "nested.json",
            include_bytes!("fixtures/config/nested.json"),
            "server.host",
            "server.port",
        ),
        (
            "sample.yaml",
            include_bytes!("fixtures/config/sample.yaml"),
            "server.host",
            "server.port",
        ),
        (
            "sample.toml",
            include_bytes!("fixtures/config/sample.toml"),
            "server.host",
            "server.port",
        ),
        (
            "sample.ini",
            include_bytes!("fixtures/config/sample.ini"),
            "server.host",
            "server.port",
        ),
        (
            "sample.properties",
            include_bytes!("fixtures/config/sample.properties"),
            "server.host",
            "server.port",
        ),
        (
            "sample.xml",
            include_bytes!("fixtures/config/sample.xml"),
            "server.host.#text",
            "server.port.#text",
        ),
    ];
    for (path, bytes, host_key, port_key) in cases {
        let config = parsed(path, bytes);
        assert_eq!(
            config.keys.get(*host_key).map(String::as_str),
            Some("localhost"),
            "{path}: {:?}",
            config.keys
        );
        assert_eq!(
            config.keys.get(*port_key).map(String::as_str),
            Some("4000"),
            "{path}: {:?}",
            config.keys
        );
    }
}

#[test]
fn values_are_sanitized_before_the_caller_can_compare_them() {
    let before = parsed(".env.before", include_bytes!("fixtures/config/before.env"));
    let after = parsed(".env.after", include_bytes!("fixtures/config/after.env"));
    assert_eq!(before.keys["DATABASE_URL"], REDACTED);
    assert_eq!(after.keys["DATABASE_URL"], REDACTED);
    assert!(!format!("{before:?}{after:?}").contains("alice:"));

    let diff = ConfigDiff::between(&before.keys, &after.keys);
    assert!(diff.sensitive_uncertain);
    assert_eq!(
        diff.changes
            .iter()
            .map(|change| change.field.as_str())
            .collect::<Vec<_>>(),
        ["FEATURE", "PORT"]
    );
}

#[test]
fn sensitive_presence_changes_are_exact_but_redacted_to_redacted_is_not() {
    let empty = BTreeMap::new();
    let present = BTreeMap::from([("DATABASE_URL".to_owned(), REDACTED.to_owned())]);
    let added = ConfigDiff::between(&empty, &present);
    assert_eq!(added.changes.len(), 1);
    assert_eq!(added.changes[0].field, "DATABASE_URL");
    assert_eq!(added.changes[0].after.as_deref(), Some(REDACTED));
    assert!(!added.sensitive_uncertain);

    let retained = ConfigDiff::between(&present, &present);
    assert!(retained.changes.is_empty());
    assert!(retained.sensitive_uncertain);
}

#[test]
fn exact_key_events_are_capped_at_twenty_then_one_summary() {
    let before: BTreeMap<_, _> = (0..25)
        .map(|index| (format!("KEY_{index:02}"), "before".to_owned()))
        .collect();
    let after: BTreeMap<_, _> = (0..25)
        .map(|index| (format!("KEY_{index:02}"), "after".to_owned()))
        .collect();
    let diff = ConfigDiff::between(&before, &after);
    assert_eq!(diff.changes.len(), MAX_KEY_EVENTS);
    assert_eq!(diff.omitted, 5);
    let items = diff.items();
    assert_eq!(items.len(), 21);
    assert_eq!(items.last(), Some(&ConfigDiffItem::Summary { omitted: 5 }));
}

#[test]
fn malformed_yaml_is_a_parse_skip_without_file_content_in_the_error() {
    let secret = include_bytes!("fixtures/config/malformed.yaml");
    let error = config_diff::parse(Path::new("bad.yaml"), secret).unwrap_err();
    assert_eq!(
        error,
        ConfigError::ParseSkipped(config_diff::ConfigFormat::Yaml)
    );
    assert!(!error.to_string().contains("unterminated"));
}

#[test]
fn an_oversized_json_is_not_parsed_but_remains_eligible_for_a_file_event() {
    let dir = temp_dir("config-diff-large");
    let path = dir.join("large.json");
    std::fs::write(&path, vec![b'x'; MAX_FILE_BYTES as usize + 1]).unwrap();
    assert_eq!(config_diff::read(&path).unwrap(), ReadOutcome::TooLarge);
}
