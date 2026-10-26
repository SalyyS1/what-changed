//! The Tauri capability files are the app's real permission surface. This
//! test makes adding filesystem, shell, HTTP, SQL, process or updater access a
//! build failure rather than something that arrives with a copied snippet —
//! the guarantee SECURITY.md's "Capability allowlist" section promises.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// Permission prefixes that must never appear. Each one would let the webview
/// reach past the typed command surface.
///
/// `updater:` is here for a different reason from the rest: it would work, and
/// it would contradict the product's own promise. An app that says it makes no
/// network calls cannot carry a permission whose purpose is to make one
/// (decision D-09).
const FORBIDDEN_PREFIXES: &[&str] = &["fs:", "shell:", "http:", "sql:", "process:", "updater:"];

/// Exactly what the app is allowed to grant, and why each one is there.
///
/// An exact set rather than a maximum count: a permission swapped for another
/// would keep the count and change the surface.
const ALLOWED: &[&str] = &[
    // Window and application basics.
    "core:default",
    // The titlebar is application-drawn, so dragging and the three buttons are
    // ours to implement.
    "core:window:allow-start-dragging",
    "core:window:allow-minimize",
    "core:window:allow-toggle-maximize",
    "core:window:allow-close",
    // The folder picker and the explicit diagnostics export save dialog.
    // `dialog:default` would grant unrelated message/dialog capabilities.
    "dialog:allow-open",
    "dialog:allow-save",
    // The backend policy is the only caller; the webview receives no generic
    // notification API, but the plugin needs its default registration scope.
    "notification:default",
];

fn capability_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("capabilities")
}

fn permission_identifiers(value: &serde_json::Value) -> Vec<String> {
    value["permissions"]
        .as_array()
        .expect("capability file has a `permissions` array")
        .iter()
        .map(|p| match p {
            serde_json::Value::String(s) => s.clone(),
            // Scoped form: { "identifier": "...", "allow": [...] }
            serde_json::Value::Object(o) => o["identifier"]
                .as_str()
                .expect("scoped permission has an `identifier`")
                .to_owned(),
            other => panic!("unexpected permission entry: {other}"),
        })
        .collect()
}

/// Every permission identifier across every capability file.
fn all_permissions() -> BTreeSet<String> {
    let mut granted = BTreeSet::new();
    let entries = fs::read_dir(capability_dir()).expect("capabilities directory exists");
    for entry in entries {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        granted.extend(permission_identifiers(&value));
    }
    assert!(!granted.is_empty(), "no capability files were checked");
    granted
}

#[test]
fn no_capability_grants_a_forbidden_permission_family() {
    for id in all_permissions() {
        for prefix in FORBIDDEN_PREFIXES {
            assert!(
                !id.starts_with(prefix),
                "`{id}` begins with the forbidden prefix `{prefix}`"
            );
        }
    }
}

#[test]
fn the_granted_set_is_exactly_the_documented_one() {
    let granted = all_permissions();
    let allowed: BTreeSet<String> = ALLOWED.iter().map(|s| (*s).to_owned()).collect();

    let unexpected = undocumented_in(&granted);
    assert!(
        unexpected.is_empty(),
        "these permissions are granted but not documented in SECURITY.md: {unexpected:?}"
    );
    let missing: Vec<&String> = allowed.difference(&granted).collect();
    assert!(
        missing.is_empty(),
        "these permissions are documented but no longer granted: {missing:?}"
    );
}

/// The two checks the gates above perform, as functions, so a deliberately
/// bad capability file can be driven through the same code rather than
/// through a re-implementation of it.
fn forbidden_in(permissions: &BTreeSet<String>) -> Vec<String> {
    permissions
        .iter()
        .filter(|id| FORBIDDEN_PREFIXES.iter().any(|p| id.starts_with(p)))
        .cloned()
        .collect()
}

fn undocumented_in(permissions: &BTreeSet<String>) -> Vec<String> {
    let allowed: BTreeSet<String> = ALLOWED.iter().map(|s| (*s).to_owned()).collect();
    permissions.difference(&allowed).cloned().collect()
}

#[test]
fn the_gates_reject_a_capability_file_that_grants_what_they_forbid() {
    // A gate that cannot fail proves nothing, so a hostile file is parsed by
    // the same reader the gates use and driven through the same predicates.
    let hostile = serde_json::json!({
        "identifier": "hostile",
        "windows": ["main"],
        "permissions": [
            "core:default",
            "fs:allow-read-text-file",
            { "identifier": "shell:allow-execute", "allow": [] },
        ]
    });
    let granted: BTreeSet<String> = permission_identifiers(&hostile).into_iter().collect();

    let caught = forbidden_in(&granted);
    assert_eq!(
        caught,
        vec![
            "fs:allow-read-text-file".to_owned(),
            "shell:allow-execute".to_owned()
        ],
        "both the plain and the scoped form must be caught"
    );
    assert!(
        !undocumented_in(&granted).is_empty(),
        "an undocumented permission must also fail the exact-set gate"
    );
}

#[test]
fn the_gates_pass_the_file_the_app_actually_ships() {
    // The other direction: the real files must survive both predicates, so a
    // rule that rejects everything cannot pass for a working gate.
    let granted = all_permissions();
    assert!(forbidden_in(&granted).is_empty());
    assert!(undocumented_in(&granted).is_empty());
    assert!(
        granted.contains("dialog:allow-open"),
        "the folder picker is granted and must not be caught"
    );
    assert!(granted.contains("dialog:allow-save"));
}

#[test]
fn notification_permission_is_present_only_for_the_policy_adapter() {
    // The plugin is registered and the policy/adapter now exist. The exact
    // permission keeps the webview surface narrow: no fs/shell escape hatch is
    // added, and future notification scopes must update this allow-list test.
    let granted = all_permissions();
    assert!(
        granted.contains("notification:default"),
        "the policy adapter requires the narrowly-scoped notification permission"
    );
}
