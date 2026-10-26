#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::collectors::runtime::RuntimeStatusSink;
use crate::dto::{CollectorState, CollectorStatusDto};
use std::fs;
use std::path::Path;

fn root() -> PathBuf {
    crate::testing::temp_dir("soak-paths").join("what-changed-perf-soak-test")
}

fn create(path: &Path) -> crate::error::AppResult<paths::OwnedRoot> {
    paths::OwnedRoot::create(path, Path::new("trigger.txt"), Path::new("ack.txt"))
}

#[test]
fn invalid_paths_are_rejected_before_any_directory_creation() {
    let parent = crate::testing::temp_dir("soak-invalid");
    let invalid = parent.join("unreserved-name");
    assert!(create(&invalid).is_err());
    assert!(!invalid.exists());
    let root = parent.join("what-changed-perf-soak-valid");
    let outside = parent.join("outside").join("trigger.txt");
    assert!(paths::OwnedRoot::create(&root, &outside, Path::new("ack.txt")).is_err());
    assert!(!root.exists());
    assert!(!outside.parent().unwrap().exists());
    assert!(
        paths::OwnedRoot::create(&root, Path::new("../trigger.txt"), Path::new("ack.txt")).is_err()
    );
    assert!(paths::OwnedRoot::create(&root, Path::new("ACK.txt"), Path::new("ack.txt")).is_err());
    let trigger = root.join("same.txt");
    let alternate = PathBuf::from(root.join("SAME.txt").to_string_lossy().replace('\\', "/"));
    assert!(paths::OwnedRoot::create(&root, &trigger, &alternate).is_err());
    assert!(paths::OwnedRoot::create(&root, &trigger, &root.join(".").join("SAME.txt")).is_err());
    assert!(paths::OwnedRoot::create(&root, Path::new("CON.txt"), Path::new("ack.txt")).is_err());
    assert!(!root.exists());
}

#[test]
fn existing_root_and_outputs_are_never_overwritten() {
    let root = root();
    fs::create_dir(&root).unwrap();
    let marker = root.join("settings-sentinel");
    fs::write(&marker, "keep").unwrap();
    assert!(create(&root).is_err());
    assert_eq!(fs::read_to_string(marker).unwrap(), "keep");
    let fresh = self::root();
    let owned = create(&fresh).unwrap();
    assert!(create(&fresh).is_err());
    paths::write_new(&owned.ack, "first").unwrap();
    assert!(paths::write_new(&owned.ack, "replacement").is_err());
    assert_eq!(fs::read_to_string(&owned.ack).unwrap(), "first");
    assert!(fs::read_dir(&fresh).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".tmp")));
}

#[test]
fn normal_application_data_is_rejected_even_with_forward_slashes() {
    let root = crate::app::AppPaths::resolve()
        .data
        .join("what-changed-perf-soak-forbidden");
    assert!(create(&root).is_err());
    let alternate = PathBuf::from(root.to_string_lossy().replace('\\', "/"));
    assert!(create(&alternate).is_err());
}

#[test]
fn malformed_or_oversize_trigger_is_not_silently_ignored() {
    let owned = create(&root()).unwrap();
    assert!(paths::read_trigger(&owned.trigger).unwrap().is_none());
    fs::write(&owned.trigger, vec![b'x'; 129]).unwrap();
    assert!(paths::read_trigger(&owned.trigger).is_err());
    fs::write(&owned.trigger, [0xff]).unwrap();
    assert!(paths::read_trigger(&owned.trigger).is_err());
}

#[cfg(windows)]
#[test]
fn junction_ancestor_is_rejected_without_touching_its_target() {
    let parent = crate::testing::temp_dir("soak-junction");
    let target = parent.join("target");
    fs::create_dir(&target).unwrap();
    let alias = parent.join("alias");
    // NTFS junction creation does not require developer mode or elevation.
    let command = format!("$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path '{}' -Target '{}' | Out-Null",
        alias.to_string_lossy().replace('\'', "''"), target.to_string_lossy().replace('\'', "''"));
    let status = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &command])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(create(&alias.join("what-changed-perf-soak-test")).is_err());
    assert!(!target.join("what-changed-perf-soak-test").exists());
}

fn status() -> CollectorStatusDto {
    let descriptor = crate::platform::production_registry().entries()[0]
        .descriptor
        .clone();
    CollectorStatusDto {
        id: descriptor.id.to_owned(),
        descriptor: (&descriptor).into(),
        enabled: true,
        state: CollectorState::Idle,
        last_run_at: None,
        last_duration_ms: None,
        events_emitted: 0,
        consecutive_failures: 0,
        last_error: None,
        details: None,
        requires_admin: false,
        queue_drops: 0,
        failed_commit_rows: 0,
        cycle_admission_drops: 0,
        failed_cycles: 0,
    }
}

#[test]
fn readiness_requires_every_success_and_retains_failure_after_recovery() {
    let row = status();
    let expected = vec![row.id.clone()];
    let evidence = health::Evidence::default();
    assert!(
        !health::inspect_rows(std::slice::from_ref(&row), &expected, &evidence.snapshot()).unwrap()
    );
    evidence.collector_outcome_observed(&row.id, true);
    assert!(
        health::inspect_rows(std::slice::from_ref(&row), &expected, &evidence.snapshot()).unwrap()
    );
    evidence.collector_outcome_observed(&row.id, false);
    evidence.collector_outcome_observed(&row.id, true);
    assert!(health::inspect_rows(&[row], &expected, &evidence.snapshot()).is_err());
}

#[test]
fn health_rejects_missing_duplicate_unavailable_paused_and_loss_rows() {
    let row = status();
    let expected = vec![row.id.clone()];
    let evidence = health::Evidence::default();
    evidence.collector_outcome_observed(&row.id, true);
    assert!(health::inspect_rows(&[], &expected, &evidence.snapshot()).is_err());
    assert!(
        health::inspect_rows(&[row.clone(), row.clone()], &expected, &evidence.snapshot()).is_err()
    );
    for state in [
        CollectorState::Unavailable,
        CollectorState::Paused,
        CollectorState::Degraded,
        CollectorState::Failed,
        CollectorState::Disabled,
    ] {
        let mut bad = row.clone();
        bad.state = state;
        assert!(health::inspect_rows(&[bad], &expected, &evidence.snapshot()).is_err());
    }
    let mut lost = row;
    lost.queue_drops = 1;
    assert!(health::inspect_rows(&[lost], &expected, &evidence.snapshot()).is_err());
}

#[test]
fn options_bound_duration_and_require_explicit_paths() {
    assert!(parse_options(["--soak".to_owned()]).is_err());
    for seconds in ["0", "86401", "not-a-number"] {
        assert!(parse_options(["--seconds".to_owned(), seconds.to_owned()]).is_err());
    }
}
