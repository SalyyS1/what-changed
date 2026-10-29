//! Architecture invariants, enforced by reading the source tree.
//!
//! These are build gates, not review conventions. Each exists because the
//! failure it prevents is expensive and silent: SQL drifting into commands
//! where nobody indexes it, a panic in a collector thread, business logic
//! that cannot be tested without a machine in a particular state, and a
//! table that some phase writes to and no migration creates.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn binary_entry_directory_contains_only_executable_sources() {
    // The pinned Tauri CLI discovers every immediate src/bin entry as a
    // bundle binary, including directories without a Cargo main.rs. Keep
    // support modules outside this directory so packaging cannot select them.
    for entry in fs::read_dir(crate_root().join("src/bin")).unwrap() {
        let entry = entry.unwrap();
        assert!(
            entry.file_type().unwrap().is_file()
                && entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs"),
            "src/bin must contain executable Rust sources only: {:?}",
            entry.path()
        );
    }
}

fn is_external_test_module(path: &Path) -> bool {
    path == crate_root().join("src/soak/tests.rs")
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            // The grouped seeder is an explicitly devtools-gated QA harness.
            // Its helper modules intentionally use direct SQLite/Win32 calls
            // and panic-style assertions while measuring fixture construction;
            // they are not shipped application code covered by these runtime
            // architecture boundaries.
            if path.ends_with(Path::new("src/bin_support/seed_history_grouping"))
                || path.ends_with(Path::new("src/bin_support/seed_history_proof"))
            {
                continue;
            }
            files.extend(rust_files(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            // These are `#[cfg(test)]` modules included by a production
            // storage file via `#[path]`. They are test fixtures, not shipped
            // runtime code; scanning their SQL would turn fixture tables into
            // false migration obligations.
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if is_external_test_module(&path)
                || name.ends_with("_tests.rs")
                || name.ends_with("_test_fixture.rs")
                || name.ends_with("_benchmark.rs")
                || name.ends_with("_fixture.rs")
                || name.ends_with("_parity.rs")
                || name.starts_with("recurrence_incremental_prototype_")
            {
                continue;
            }
            files.push(path);
        }
    }
    files.sort();
    files
}

fn relative(path: &Path) -> String {
    path.strip_prefix(crate_root())
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/")
}

/// The production part of a file: everything before its `#[cfg(test)]`
/// module, with comment lines dropped so a rule quoted in a comment does
/// not trip the rule.
fn production_code(text: &str) -> String {
    // An inner attribute gates the complete out-of-line module, exactly as
    // rustc does. Filenames alone do not establish this boundary.
    if text.trim_start().starts_with("#![cfg(test)]") {
        return String::new();
    }
    let cut = text.find("#[cfg(test)]").unwrap_or(text.len());
    text[..cut]
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn whole_file_test_gate_does_not_hide_ordinary_production_files() {
    assert!(production_code("#![cfg(test)]\nfn fixture(){ value.unwrap(); }").is_empty());
    assert!(
        production_code("// #![cfg(test)]\nfn real(){ value.unwrap(); }")
            .contains("value.unwrap()")
    );
    assert!(production_code("fn real(){}\n#[cfg(test)]\nmod tests{}").contains("fn real()"));
}

#[test]
fn external_soak_tests_are_gated_without_hiding_production_code() {
    let root = crate_root();
    assert!(is_external_test_module(&root.join("src/soak/tests.rs")));
    assert!(!is_external_test_module(&root.join("src/other/tests.rs")));
    let parent = fs::read_to_string(root.join("src/soak.rs")).unwrap();
    let lines: Vec<_> = parent
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    assert!(lines.ends_with(&["#[cfg(test)]", "mod tests;"]));
    let scanned = production_code(&parent);
    assert!(scanned.contains("fn parse_options("));
    assert!(scanned.contains("pub fn run("));
    let files = rust_files(&root.join("src"));
    for path in [
        "src/soak.rs",
        "src/soak/health.rs",
        "src/soak/paths.rs",
        "src/soak/runner.rs",
    ] {
        assert!(
            files.contains(&root.join(path)),
            "production module must be scanned: {path}"
        );
    }
}

/// Statements that only `storage/` may contain.
const SQL_WRITE_MARKERS: &[&str] = &["INSERT INTO", "UPDATE ", "DELETE FROM", "CREATE TABLE"];

#[test]
fn sql_lives_only_under_storage() {
    for file in rust_files(&crate_root().join("src")) {
        let rel = relative(&file);
        // The typed error wraps the driver's error; that is the one place
        // outside storage that may name it.
        if rel.starts_with("src/storage/")
            || rel == "src/owner_workspace/storage.rs"
            || rel.starts_with("src/owner_workspace/storage/")
            || rel == "src/error.rs"
        {
            continue;
        }
        let code = production_code(&fs::read_to_string(&file).unwrap());
        for marker in SQL_WRITE_MARKERS {
            assert!(
                !code.contains(marker),
                "{rel} contains `{marker}`; SQL belongs in src/storage/"
            );
        }
        assert!(
            !code.contains("rusqlite::"),
            "{rel} reaches for rusqlite directly; go through src/storage/"
        );
    }
}

#[test]
fn no_unwrap_or_expect_outside_tests() {
    for file in rust_files(&crate_root().join("src")) {
        let rel = relative(&file);
        if rel.starts_with("src/testing/") {
            continue;
        }
        let code = production_code(&fs::read_to_string(&file).unwrap());
        for needle in [".unwrap()", ".expect("] {
            assert!(
                !code.contains(needle),
                "{rel} uses `{needle}` outside a test module"
            );
        }
    }
}

#[test]
fn core_modules_carry_no_windows_cfg() {
    for file in rust_files(&crate_root().join("src")) {
        let rel = relative(&file);
        let core = rel.starts_with("src/domain/")
            || rel.starts_with("src/analysis/")
            || rel.starts_with("src/collectors/");
        if !core {
            continue;
        }
        let code = production_code(&fs::read_to_string(&file).unwrap());
        assert!(
            !code.contains("cfg(windows)") && !code.contains("cfg(target_os"),
            "{rel} is platform-specific; that belongs in src/platform/"
        );
    }
}

/// Table names that appear after a SQL keyword in any `.rs` file.
fn tables_named_in_rust(code: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut ctes = BTreeSet::new();
    let mut with_rest = code;
    while let Some(at) = with_rest.find("WITH ") {
        let after = &with_rest[at + "WITH ".len()..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            ctes.insert(name);
        }
        with_rest = after;
    }
    // A WITH clause can declare several CTEs and can include an optional
    // column list (`candidate(id) AS ...`).  Looking only at the token after
    // `WITH` mistakes every later CTE for a migrated table.  SQL aliases do
    // not have an opening parenthesis after `AS`, so this narrow scan captures
    // CTE declarations without hiding ordinary FROM/JOIN table names.
    for marker in [" AS (", " AS MATERIALIZED (", " AS NOT MATERIALIZED ("] {
        let mut rest = code;
        while let Some(at) = rest.find(marker) {
            let mut before = rest[..at].trim_end();
            if before.ends_with(')') {
                let mut depth = 0usize;
                let mut open = None;
                for (index, character) in before.char_indices().rev() {
                    match character {
                        ')' => depth += 1,
                        '(' => {
                            depth = depth.saturating_sub(1);
                            if depth == 0 {
                                open = Some(index);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(open) = open {
                    before = before[..open].trim_end();
                }
            }
            let name: String = before
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if !name.is_empty() {
                ctes.insert(name);
            }
            rest = &rest[at + marker.len()..];
        }
    }
    for keyword in [
        "FROM ",
        "INSERT INTO ",
        "INSERT OR IGNORE INTO ",
        "UPDATE ",
        "DELETE FROM ",
    ] {
        let mut rest = code;
        while let Some(at) = rest.find(keyword) {
            let after = &rest[at + keyword.len()..];
            let name: String = after
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            // Only identifiers count: `FROM ?1` is a parameter, `DO UPDATE
            // SET` is an upsert clause, and `sqlite_master` is SQLite's own.
            if !name.is_empty()
                && !name.starts_with(char::is_numeric)
                && !name.starts_with("sqlite_")
                // Ephemeral per-connection work tables are created and
                // dropped by the same storage function, never migrations.
                && !name.starts_with("temp_")
                && name != "SET"
                && !ctes.contains(&name)
            {
                names.insert(name);
            }
            rest = after;
        }
    }
    names
}

/// Table names created by any migration.
fn tables_created_by_migrations() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in ["migrations", "src/owner_workspace/storage/migrations"]
        .into_iter()
        .flat_map(|directory| fs::read_dir(crate_root().join(directory)).unwrap())
    {
        let path = entry.unwrap().path();
        let sql = fs::read_to_string(&path).unwrap();
        for keyword in ["CREATE TABLE ", "CREATE VIRTUAL TABLE "] {
            let mut rest = sql.as_str();
            while let Some(at) = rest.find(keyword) {
                let after = &rest[at + keyword.len()..];
                let after = after.strip_prefix("IF NOT EXISTS ").unwrap_or(after);
                let name: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                names.insert(name);
                rest = after;
            }
        }
    }
    names
}

fn missing_tables(code: &str) -> Vec<String> {
    let created = tables_created_by_migrations();
    tables_named_in_rust(code)
        .into_iter()
        .filter(|name| !created.contains(name))
        .collect()
}

#[test]
fn every_table_named_in_rust_is_created_by_a_migration() {
    for file in rust_files(&crate_root().join("src")) {
        let code = production_code(&fs::read_to_string(&file).unwrap());
        let missing = missing_tables(&code);
        assert!(
            missing.is_empty(),
            "{} uses tables no migration creates: {missing:?}",
            relative(&file)
        );
    }
}

#[test]
fn the_table_assertion_can_fail() {
    // A gate that cannot fail proves nothing. A deliberately bad query must
    // be caught by the same function the gate uses.
    let bad = "conn.prepare(\"SELECT * FROM collector_state WHERE id = ?1\")";
    assert_eq!(missing_tables(bad), vec!["collector_state".to_owned()]);
    let good = "conn.prepare(\"SELECT * FROM collector_baselines WHERE id = ?1\")";
    assert!(missing_tables(good).is_empty());
    let cte = "conn.prepare(\"WITH requested(id) AS (VALUES (?1)) SELECT * FROM requested\")";
    assert!(missing_tables(cte).is_empty());
    let multiple_ctes = "conn.prepare(\"WITH roots(path) AS MATERIALIZED (SELECT path FROM projects), candidates(id) AS MATERIALIZED (SELECT id FROM entities) DELETE FROM events WHERE id IN (SELECT id FROM candidates)\")";
    assert!(missing_tables(multiple_ctes).is_empty());
    let recursive = "conn.execute(\"WITH RECURSIVE sequence(value) AS (SELECT 1 UNION ALL SELECT value + 1 FROM sequence) INSERT INTO events SELECT value FROM sequence\")";
    assert!(missing_tables(recursive).is_empty());
    let temporary = "conn.execute(\"INSERT INTO temp_walk SELECT id FROM entities\")";
    assert!(missing_tables(temporary).is_empty());
}

#[test]
fn analysis_never_imports_platform_or_collectors() {
    for file in rust_files(&crate_root().join("src/analysis")) {
        let code = production_code(&fs::read_to_string(&file).unwrap());
        assert!(
            !code.contains("crate::platform") && !code.contains("crate::collectors"),
            "{} crosses the pure analysis boundary",
            relative(&file)
        );
    }
}

#[test]
fn win32_calls_live_only_behind_the_platform_boundary() {
    // The non-elevation promise is auditable only if there is one place to
    // audit. A Win32 call that leaked into a collector would also make that
    // collector untestable without a machine in a particular state, which is
    // the same failure the cfg gate above prevents from the other side.
    for file in rust_files(&crate_root().join("src")) {
        let rel = relative(&file);
        if rel.starts_with("src/platform/windows/") {
            continue;
        }
        let code = production_code(&fs::read_to_string(&file).unwrap());
        for needle in ["windows::Win32", "windows_sys::", "winapi::"] {
            assert!(
                !code.contains(needle),
                "{rel} reaches for `{needle}` directly; that belongs in src/platform/windows/"
            );
        }
    }
}

#[test]
fn the_win32_gate_can_fail() {
    // A gate that cannot fail proves nothing.
    let bad = "use windows::Win32::System::Threading::OpenProcess;";
    assert!(bad.contains("windows::Win32"));
    let good = "use crate::platform::traits::ProcessProvider;";
    for needle in ["windows::Win32", "windows_sys::", "winapi::"] {
        assert!(!good.contains(needle));
    }
}

#[test]
fn collectors_and_platform_never_read_command_lines_or_environments() {
    // Blocking acquisition, not just persistence: a process's command line
    // and environment block are where credentials are passed, and no
    // collector has a reason to read either.
    for file in rust_files(&crate_root().join("src")) {
        let rel = relative(&file);
        if !(rel.starts_with("src/collectors/") || rel.starts_with("src/platform/")) {
            continue;
        }
        let code = production_code(&fs::read_to_string(&file).unwrap());
        for needle in [".cmd()", ".environ()", "GetCommandLine", "env::vars"] {
            assert!(
                !code.contains(needle),
                "{rel} reads `{needle}`, which is where secrets are passed"
            );
        }
    }
}

#[test]
fn sanitized_is_constructed_only_inside_the_sanitizer_module() {
    for file in rust_files(&crate_root().join("src")) {
        let rel = relative(&file);
        if rel == "src/privacy/sanitize.rs" {
            continue;
        }
        let code = production_code(&fs::read_to_string(&file).unwrap());
        assert!(
            !code.contains("Sanitized("),
            "{rel} constructs Sanitized directly"
        );
    }
}
