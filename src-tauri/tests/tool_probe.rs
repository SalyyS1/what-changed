#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use what_changed_lib::platform::windows::tool_probe::{parse_path, resolve, ToolProbe};

fn serial_tool_probe() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn quoted_path_entries_deduplicate_case_and_keep_semicolons_inside_quotes() {
    let _guard = serial_tool_probe();
    assert_eq!(
        parse_path(r#""C:\tools;stable";C:\bin;;c:\BIN;"#),
        vec![PathBuf::from(r"C:\tools;stable"), PathBuf::from(r"C:\bin")]
    );
}

#[test]
fn a_non_allowlisted_executable_is_never_resolved_or_spawned() {
    let _guard = serial_tool_probe();
    let directory = what_changed_lib::testing::temp_dir("tool-probe-non-allowlisted");
    let source = directory.join("powershell.rs");
    let executable = directory.join("powershell.exe");
    let marker = directory.join("spawned.txt");
    std::fs::write(
        &source,
        r#"
fn main() {
    let marker = std::env::current_exe().unwrap().with_file_name("spawned.txt");
    std::fs::write(marker, "spawned").unwrap();
}
"#,
    )
    .unwrap();
    let compiled = std::process::Command::new("rustc")
        .args(["--edition=2021", "-o"])
        .arg(&executable)
        .arg(&source)
        .status()
        .unwrap();
    assert!(compiled.success());

    assert!(resolve(std::slice::from_ref(&directory), "powershell").is_none());
    let scan = ToolProbe.scan(&directory.to_string_lossy());
    assert!(!scan.tools.contains_key("powershell"));
    assert!(!marker.exists());
}

#[test]
fn cmd_tools_are_detected_but_never_spawned() {
    let _guard = serial_tool_probe();
    let directory = what_changed_lib::testing::temp_dir("tool-probe-cmd-no-spawn");
    let tool = directory.join("node.cmd");
    let marker = directory.join("spawned.txt");
    std::fs::write(
        &tool,
        format!("@echo touched > \"{}\"\r\n", marker.display()),
    )
    .unwrap();
    let version = ToolProbe.probe_resolved("node", &tool, Duration::from_secs(2));
    assert!(version.is_none());
    let scan = ToolProbe.scan(&directory.to_string_lossy());
    assert_eq!(scan.tools["node"].version, None);
    assert!(!marker.exists());
}

#[test]
fn unc_entries_are_skipped_without_a_filesystem_probe() {
    let _guard = serial_tool_probe();
    let started = Instant::now();
    assert!(resolve(&[PathBuf::from(r"\\offline-server\tools")], "node").is_none());
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_millis(50), "elapsed: {elapsed:?}");
}

#[test]
fn path_resolution_is_included_in_the_global_scan_budget() {
    let _guard = serial_tool_probe();
    let root = what_changed_lib::testing::temp_dir("tool-probe-resolution-budget");
    let path = (0..100_000)
        .map(|index| root.join(format!("missing-{index}")))
        .map(|entry| entry.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(";");
    let started = Instant::now();
    let scan = ToolProbe.scan(&path);
    let elapsed = started.elapsed();
    assert!(
        !scan.complete,
        "the fixture should exhaust the bounded scan"
    );
    assert!(
        elapsed <= Duration::from_millis(1_500),
        "elapsed: {elapsed:?}"
    );
}

#[test]
fn reparse_path_entries_are_skipped() {
    let _guard = serial_tool_probe();
    let root = what_changed_lib::testing::temp_dir("tool-probe-reparse");
    let target = root.join("target");
    let link = root.join("linked-tools");
    std::fs::create_dir_all(&target).unwrap();
    if std::os::windows::fs::symlink_dir(&target, &link).is_err() {
        // Creating symlinks requires Developer Mode on some supported Windows
        // installations; the production attribute check remains deterministic.
        return;
    }
    std::fs::write(target.join("node.exe"), b"not executable").unwrap();
    assert!(resolve(&[link], "node").is_none());
}

#[test]
fn a_hanging_executable_is_killed_at_the_probe_deadline() {
    let _guard = serial_tool_probe();
    let directory = what_changed_lib::testing::temp_dir("tool-probe-hanging-exe");
    let source = directory.join("hanging.rs");
    let executable = directory.join("node.exe");
    std::fs::write(
        &source,
        "fn main() { std::thread::sleep(std::time::Duration::from_secs(30)); }",
    )
    .unwrap();
    let compiled = std::process::Command::new("rustc")
        .args(["--edition=2021", "-o"])
        .arg(&executable)
        .arg(&source)
        .status()
        .unwrap();
    assert!(compiled.success());

    let started = Instant::now();
    let version = ToolProbe.probe_resolved("node", &executable, Duration::from_secs(2));
    let elapsed = started.elapsed();
    assert!(version.is_none());
    assert!(
        elapsed >= Duration::from_millis(1_800),
        "elapsed: {elapsed:?}"
    );
    // The kernel deadline is two seconds; allow one second for a heavily
    // contended CI worker to schedule this thread again after termination.
    assert!(elapsed <= Duration::from_secs(3), "elapsed: {elapsed:?}");
}

#[test]
fn an_exited_parent_with_inherited_output_handles_returns_without_waiting_for_eof() {
    let _guard = serial_tool_probe();
    let directory = what_changed_lib::testing::temp_dir("tool-probe-inherited-handles");
    let source = directory.join("inherited_handles.rs");
    let executable = directory.join("node.exe");
    std::fs::write(
        &source,
        r#"
use std::process::{Command, Stdio};
use std::time::Duration;

fn main() {
    if std::env::args().nth(1).as_deref() == Some("child") {
        std::thread::sleep(Duration::from_secs(5));
        return;
    }
    let executable = std::env::current_exe().unwrap();
    let _child = Command::new(executable)
        .arg("child")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    println!("v22.6.0");
}
"#,
    )
    .unwrap();
    let compiled = std::process::Command::new("rustc")
        .args(["--edition=2021", "-o"])
        .arg(&executable)
        .arg(&source)
        .status()
        .unwrap();
    assert!(compiled.success());

    let started = Instant::now();
    let version = ToolProbe.probe_resolved("node", &executable, Duration::from_secs(4));
    let elapsed = started.elapsed();
    assert_eq!(version.as_deref(), Some("22.6.0"), "elapsed: {elapsed:?}");
    assert!(
        elapsed <= Duration::from_millis(4_500),
        "inherited pipe handles outlived the exited parent: {elapsed:?}"
    );
}
