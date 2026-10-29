//! Wiring invariants that no type can express.
//!
//! Each of these is a rule that holds today, would break silently, and has no
//! other place to be checked: an ordering between plugin registrations, a
//! string that must be identical in two languages, a table in a Markdown file
//! that must list what the code refuses to do. They are checked by reading the
//! source, which is the same trick `tests/architecture.rs` already uses — a
//! cheap gate is worth more than a rule nobody can enforce.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_root() -> PathBuf {
    crate_root()
        .parent()
        .expect("the crate sits inside the repository")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("{} could not be read: {error}", path.display());
    })
}

#[test]
fn the_single_instance_plugin_is_registered_before_every_other_plugin() {
    // Tauri runs plugins in registration order, and this one's whole job is to
    // stop a second window before anything else reacts to it. Registered
    // later, a second launch would create a window and then be told not to.
    let source = read("src-tauri/src/lib.rs");
    let first = source
        .find("tauri_plugin_single_instance::init")
        .expect("the single-instance plugin is registered");
    for other in [
        "window_state_plugin()",
        "tauri_plugin_dialog::init",
        "tauri_plugin_autostart::Builder",
        "tauri_plugin_notification::init",
    ] {
        let at = source
            .find(other)
            .unwrap_or_else(|| panic!("{other} is registered"));
        assert!(
            first < at,
            "single-instance must be registered before {other}"
        );
    }
}

#[test]
fn quitting_is_the_only_exit_path() {
    // "Quit is the only way out" is a product promise. An `exit` added
    // anywhere else would keep every test green and break it.
    let allowed = [
        "src/app/lifecycle.rs",
        "src/lib.rs",
        // Development-only QA binaries use an exit code for CLI parse
        // failures; they are not application lifecycle paths.
        "src/bin/seed_history.rs",
        "src/bin/resource_probe.rs",
    ];
    let mut offenders = Vec::new();
    for file in rust_files(&crate_root().join("src")) {
        let relative = relative(&file);
        if allowed.contains(&relative.as_str()) {
            continue;
        }
        let code = fs::read_to_string(&file).unwrap();
        for needle in ["process::exit", ".exit("] {
            if code.contains(needle) {
                offenders.push(format!("{relative} calls {needle}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "an exit outside the quit path: {offenders:?}"
    );

    // The GUI exits only through Quit or its three startup failures. The
    // feature-gated backend diagnostic returns its CLI exit status before
    // opening a profile or building the GUI.
    let lifecycle = read("src-tauri/src/app/lifecycle.rs");
    assert_eq!(
        lifecycle.matches(".exit(").count(),
        1,
        "lifecycle exits exactly once, from quit"
    );
    let lib = read("src-tauri/src/lib.rs");
    let diagnostic_dispatch = concat!(
        "    #[cfg(feature = \"devtools\")]\n",
        "    if std::env::args().any(|argument| argument == \"--soak\") {\n",
        "        std::process::exit(soak::run(std::env::args().skip(1)));\n",
        "    }\n",
    );
    let lib = lib.replace("\r\n", "\n");
    assert_eq!(
        lib.matches(diagnostic_dispatch).count(),
        1,
        "the diagnostic exit must remain behind the devtools feature and explicit CLI flag"
    );
    let gui = lib.replacen(diagnostic_dispatch, "", 1);
    assert_eq!(
        gui.matches("process::exit").count(),
        3,
        "the GUI exits only on startup failures"
    );
}

#[test]
fn every_event_name_rust_emits_is_the_name_typescript_listens_for() {
    // The two sides name the same channel in two languages. A typo in either
    // is a feature that silently never updates.
    let rust = read("src-tauri/src/app/event_bus.rs");
    let tray = read("src-tauri/src/app/tray.rs");
    let typescript = read("src/lib/tauri/events.ts");

    // Only the `EVENT_*` constants: the tray's menu-item ids are kebab-case
    // too, and they are not events — they never cross to the frontend.
    let mut declared = BTreeSet::new();
    for source in [&rust, &tray] {
        for line in source.lines() {
            let Some(declaration) = line.split("pub const EVENT_").nth(1) else {
                continue;
            };
            let Some(value) = declaration.split(" = \"").nth(1) else {
                continue;
            };
            let Some(name) = value.split('"').next() else {
                continue;
            };
            declared.insert(name.to_owned());
        }
    }
    assert_eq!(
        declared.len(),
        6,
        "expected the five backend events plus tray navigation, found {declared:?}"
    );
    for name in &declared {
        assert!(
            typescript.contains(&format!("'{name}'")),
            "Rust emits '{name}' and src/lib/tauri/events.ts does not listen for it"
        );
    }
}

#[test]
fn every_unimplemented_command_is_listed_in_the_roadmap() {
    // A registered command with no body type-checks, appears in the bindings
    // and looks finished. The ROADMAP table is what makes it visible, so the
    // table has to be complete rather than merely present.
    let roadmap = read("ROADMAP.md");
    let mut missing = Vec::new();
    for file in rust_files(&crate_root().join("src/commands")) {
        let code = fs::read_to_string(&file).unwrap();
        for (index, line) in code.lines().enumerate() {
            if !line.trim_start().starts_with("not_implemented(") {
                continue;
            }
            // The command is the nearest `pub fn` above the stub body.
            let name = code
                .lines()
                .take(index)
                .filter_map(|l| l.trim().strip_prefix("pub fn "))
                .filter_map(|l| l.split('(').next())
                .last()
                .unwrap_or("<unknown>");
            if !roadmap.contains(&format!("`{name}`")) {
                missing.push(name.to_owned());
            }
        }
    }
    assert!(
        missing.is_empty(),
        "these commands return not_implemented and are not in ROADMAP's tracked stubs: {missing:?}"
    );
}

#[test]
fn the_roadmap_lists_no_stub_that_has_since_been_implemented() {
    // The other direction: a phase that lands a body must remove its row, or
    // the table becomes a list of things that are already done.
    let roadmap = read("ROADMAP.md");
    let start = roadmap
        .find("## Tracked stubs")
        .expect("the tracked-stubs section exists");
    let end = roadmap[start..]
        .find("\n## ")
        .map(|offset| start + offset)
        .unwrap_or(roadmap.len());
    let section = &roadmap[start..end];

    let mut implemented = Vec::new();
    let sources: String = rust_files(&crate_root().join("src/commands"))
        .iter()
        .map(|f| fs::read_to_string(f).unwrap())
        .collect();
    for line in section.lines() {
        let Some(name) = line.strip_prefix("| `") else {
            continue;
        };
        let Some(name) = name.split('`').next() else {
            continue;
        };
        let declared = sources.contains(&format!("pub fn {name}("));
        let still_a_stub = sources
            .split(&format!("pub fn {name}("))
            .nth(1)
            .is_some_and(|body| {
                body.split("pub fn ")
                    .next()
                    .is_some_and(|b| b.contains("not_implemented("))
            });
        if declared && !still_a_stub {
            implemented.push(name.to_owned());
        }
    }
    assert!(
        implemented.is_empty(),
        "these are listed as tracked stubs but have bodies: {implemented:?}"
    );
}

/// Every `.rs` file under `dir`.
fn rust_files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn relative(path: &std::path::Path) -> String {
    path.strip_prefix(crate_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
