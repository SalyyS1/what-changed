//! Release-artifact assertions. The test is ignored by default because it
//! requires a completed Windows release build; the release script supplies the
//! artifact paths when it runs the gate.

#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::path::PathBuf;

fn required_file_from_env(name: &str) -> PathBuf {
    let value = std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"));
    let path = PathBuf::from(value);
    assert!(
        path.is_file(),
        "{name} must point to a regular file: {path:?}"
    );
    path
}

fn required_directory_from_env(name: &str) -> PathBuf {
    let value = std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"));
    let path = PathBuf::from(value);
    assert!(path.is_dir(), "{name} must point to a directory: {path:?}");
    path
}

fn assert_no_forbidden_marker(bytes: &[u8], subject: &str) {
    let text = String::from_utf8_lossy(bytes);
    for forbidden in ["simulate_event", "seed_history", "#/dev/"] {
        assert!(
            !text.contains(forbidden),
            "{subject} contains dev-only marker {forbidden}"
        );
    }
}

fn audit_dist(dist: PathBuf) {
    let mut stack = vec![dist];
    let mut file_count = 0;
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path).expect("release dist is readable") {
            let entry = entry.expect("release dist entry is readable");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            file_count += 1;
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default();
            assert!(
                !extension.eq_ignore_ascii_case("map"),
                "source maps are not shipped: {path:?}"
            );
            let bytes = fs::read(&path).expect("release dist file is readable");
            assert_no_forbidden_marker(&bytes, &format!("release dist file {path:?}"));
            let text = String::from_utf8_lossy(&bytes);
            assert!(
                !text.contains("sourceMappingURL") && !text.contains("//# sourceURL="),
                "release dist file contains a source-map/debug URL: {path:?}"
            );
        }
    }
    assert!(
        file_count > 0,
        "release dist must contain built frontend files"
    );
}

#[test]
#[ignore = "requires a built Windows release artifact"]
fn release_artifact_has_no_dev_only_surface() {
    let binary = required_file_from_env("WHAT_CHANGED_RELEASE_BINARY");
    let bytes = fs::read(&binary).expect("release binary is readable");
    assert_no_forbidden_marker(&bytes, "release artifact");

    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let config = fs::read_to_string(&config_path)
        .unwrap_or_else(|error| panic!("release config is readable ({config_path:?}): {error}"));
    let config_json: serde_json::Value = serde_json::from_str(&config)
        .unwrap_or_else(|error| panic!("release config is valid JSON ({config_path:?}): {error}"));
    assert!(
        config_json["app"]["withGlobalTauri"] == serde_json::Value::Bool(false),
        "release config must keep the global Tauri bridge disabled"
    );
    assert!(
        config_json["app"]["security"]["csp"]
            .as_str()
            .is_some_and(|csp| !csp.trim().is_empty()),
        "release config must declare a non-empty content-security policy"
    );
    assert!(
        config_json["bundle"]["targets"]
            .as_array()
            .is_some_and(|targets| targets.iter().any(|target| target == "nsis")),
        "release config must produce an NSIS artifact"
    );
    assert_eq!(
        config_json["bundle"]["windows"]["nsis"]["installMode"], "currentUser",
        "release installer must remain per-user"
    );

    let dist = required_directory_from_env("WHAT_CHANGED_DIST_DIR");
    audit_dist(dist);
}
