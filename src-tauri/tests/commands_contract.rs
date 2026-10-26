//! The typed IPC surface, checked from both ends.
//!
//! Two lists describe the same thing: the commands Rust registers, and the
//! wrappers TypeScript exposes. This suite fails when they disagree, so a
//! command can never be reachable without a typed wrapper and a wrapper can
//! never point at a command that does not exist.
//!
//! It reads the committed `bindings.ts` rather than regenerating, because the
//! committed file is what ships. `bun run check:bindings` covers the other
//! direction — that the committed file still matches the Rust types.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use what_changed_lib::commands::{signatures, COMMANDS};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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

/// The generated `COMMANDS` table in `bindings.ts`, name to argument names.
fn bindings_table() -> BTreeMap<String, Vec<String>> {
    let text = read("src/lib/tauri/bindings.ts");
    let start = text
        .find("export const COMMANDS = {")
        .expect("bindings.ts declares the COMMANDS table");
    let end = text[start..]
        .find("} as const")
        .map(|offset| start + offset)
        .expect("the COMMANDS table is closed");
    let mut table = BTreeMap::new();
    for line in text[start..end].lines() {
        let Some((name, args)) = line.trim().split_once(": [") else {
            continue;
        };
        let args = args.trim_end_matches(&[',', ']'][..]);
        table.insert(name.to_owned(), quoted_names(args));
    }
    table
}

/// Every `'name'` in a fragment, in order.
fn quoted_names(fragment: &str) -> Vec<String> {
    fragment
        .split(',')
        .filter_map(|part| {
            let part = part.trim().trim_end_matches(']');
            part.strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
                .map(str::to_owned)
        })
        .collect()
}

fn names_in_bindings() -> BTreeSet<String> {
    bindings_table().into_keys().collect()
}

/// What `client.ts` invokes: command name to the argument names it passes.
///
/// Every call goes through one helper, so one marker finds them all, and the
/// object literal after the name is what Tauri will match against the Rust
/// parameters.
fn client_calls() -> BTreeMap<String, Vec<String>> {
    let text = read("src/lib/tauri/client.ts");
    let mut found = BTreeMap::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("call('") {
        let after = &rest[at + "call('".len()..];
        let Some(end) = after.find('\'') else {
            break;
        };
        let name = after[..end].to_owned();
        found.insert(name, argument_keys(&after[end..]));
        rest = after;
    }
    found
}

/// The keys of the object literal a call passes, if it passes one.
///
/// `call('get_events', { query })` yields `["query"]`; a call with no second
/// argument yields an empty list. Shorthand and `key: value` both appear in
/// this file, so the key is whatever precedes a `:` or closes the entry.
fn argument_keys(after_name: &str) -> Vec<String> {
    // The object literal, if any, starts after the comma that follows the
    // name and ends at the matching brace. Calls without arguments reach the
    // closing paren first.
    let Some(open) = after_name.find('{') else {
        return Vec::new();
    };
    let paren = after_name.find(')').unwrap_or(usize::MAX);
    if paren < open {
        return Vec::new();
    }
    let Some(close) = after_name[open..].find('}').map(|o| open + o) else {
        return Vec::new();
    };
    after_name[open + 1..close]
        .split(',')
        .filter_map(|entry| {
            let key = entry.split(':').next()?.trim();
            (!key.is_empty()).then(|| key.to_owned())
        })
        .collect()
}

fn names_invoked_by_client() -> BTreeSet<String> {
    client_calls().into_keys().collect()
}

fn registered() -> BTreeSet<String> {
    COMMANDS.iter().map(|c| (*c).to_owned()).collect()
}

#[test]
fn every_registered_command_has_a_generated_binding() {
    let missing: Vec<String> = registered()
        .difference(&names_in_bindings())
        .cloned()
        .collect();
    assert!(
        missing.is_empty(),
        "registered in Rust but absent from bindings.ts: {missing:?}"
    );
}

#[test]
fn every_generated_binding_names_a_registered_command() {
    let extra: Vec<String> = names_in_bindings()
        .difference(&registered())
        .cloned()
        .collect();
    assert!(
        extra.is_empty(),
        "present in bindings.ts but not registered in Rust: {extra:?}"
    );
}

#[test]
fn every_command_has_a_typed_wrapper_in_the_client() {
    let invoked = names_invoked_by_client();
    let missing: Vec<String> = registered().difference(&invoked).cloned().collect();
    assert!(
        missing.is_empty(),
        "no typed wrapper calls these commands: {missing:?}"
    );
    let unknown: Vec<String> = invoked.difference(&registered()).cloned().collect();
    assert!(
        unknown.is_empty(),
        "the client calls commands that are not registered: {unknown:?}"
    );
}

#[test]
fn the_generated_argument_names_match_the_rust_signatures() {
    let generated = bindings_table();
    for (name, expected) in signatures() {
        let found = generated
            .get(name)
            .unwrap_or_else(|| panic!("{name} is missing from the generated table"));
        assert_eq!(
            found, &expected,
            "{name}'s argument names in bindings.ts disagree with its Rust signature"
        );
    }
}

/// The property the whole typed surface exists for.
///
/// Tauri matches arguments by name. A renamed Rust parameter against an
/// unchanged `client.ts` compiles on both sides, generates clean bindings and
/// fails only when a user clicks the thing — so it is checked here, against
/// the object literal the client actually passes.
#[test]
fn every_client_call_passes_exactly_the_arguments_its_command_declares() {
    let declared: BTreeMap<String, Vec<String>> = signatures()
        .into_iter()
        .map(|(name, args)| (name.to_owned(), args))
        .collect();

    for (name, passed) in client_calls() {
        let Some(expected) = declared.get(&name) else {
            continue; // covered by the registration tests above
        };
        let mut passed = passed;
        let mut expected = expected.clone();
        passed.sort();
        expected.sort();
        assert_eq!(
            passed, expected,
            "client.ts calls {name} with {passed:?} but Rust declares {expected:?}"
        );
    }
}

#[test]
fn the_argument_reader_can_actually_fail() {
    // A gate that cannot fail proves nothing. The parser must find the keys
    // of a real call and report none for a call that passes no object.
    assert_eq!(
        argument_keys("get_event_details', { eventId });"),
        vec!["eventId".to_owned()]
    );
    assert_eq!(
        argument_keys("set_collector_enabled', { collectorId, enabled });"),
        vec!["collectorId".to_owned(), "enabled".to_owned()]
    );
    assert!(argument_keys("get_settings');").is_empty());
    assert_eq!(
        argument_keys("create_checkpoint', { name: name ?? null });"),
        vec!["name".to_owned()],
        "a `key: value` entry contributes its key, not its expression"
    );
}

#[test]
fn the_bindings_file_is_generated_and_says_so() {
    let text = read("src/lib/tauri/bindings.ts");
    assert!(
        text.contains("GENERATED FILE"),
        "the generated file must say it is generated"
    );
    assert!(text.contains("bun run gen:bindings"));
}

#[test]
fn invoke_is_reached_only_through_the_tauri_directory() {
    // The one architectural rule this suite can check cheaply: a screen that
    // calls `invoke` directly is a screen with an untyped, unvalidated call in
    // it. `bindings.ts` documents the rule in prose, so the check looks for
    // the import rather than the word.
    let root = repo_root().join("src");
    let mut offenders = Vec::new();
    visit(&root, &mut |path, text| {
        let relative = path
            .strip_prefix(repo_root())
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        if relative.starts_with("src/lib/tauri/") {
            return;
        }
        if text.contains("@tauri-apps/api/core") || text.contains("@tauri-apps/api/event") {
            offenders.push(relative);
        }
    });
    assert!(
        offenders.is_empty(),
        "these files reach for the Tauri API outside src/lib/tauri/: {offenders:?}"
    );
}

#[test]
fn synthetic_simulation_has_both_a_build_gate_and_a_setting_gate() {
    let lib = read("src-tauri/src/lib.rs");
    let release = lib
        .split("The shipped handler omits the synthetic command entirely")
        .nth(1)
        .expect("lib.rs declares the shipped handler")
        .split("struct TrayHandle")
        .next()
        .expect("the shipped handler has a bounded source section");
    assert!(release.contains("not(any(debug_assertions, feature = \"devtools\"))"));
    assert!(
        !release.contains("commands::dev::simulate_event"),
        "the normal release handler must not register synthetic IPC"
    );
    assert!(
        !release.contains("commands::dev::seed_synthetic"),
        "the normal release handler must not register synthetic corpus IPC"
    );

    let dev = read("src-tauri/src/commands/dev.rs");
    assert!(dev.contains("any(test, debug_assertions, feature = \"devtools\")"));
    assert!(dev.contains("if !developer_mode"));
    assert!(dev.contains("developer mode is disabled"));
}

/// Walk every `.ts` and `.svelte` file under `dir`.
fn visit(dir: &std::path::Path, f: &mut impl FnMut(&std::path::Path, &str)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit(&path, f);
            continue;
        }
        let is_source = matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("ts") | Some("svelte")
        );
        if is_source {
            if let Ok(text) = fs::read_to_string(&path) {
                f(&path, &text);
            }
        }
    }
}
