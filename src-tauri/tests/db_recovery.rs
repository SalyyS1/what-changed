//! Corruption is a state, not a crash. A damaged file opens into
//! `DbState::Corrupt`, the app keeps running, and each of the three recovery
//! actions does what its button says.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::{Seek, SeekFrom, Write};

use what_changed_lib::app::{AppPaths, AppState};
use what_changed_lib::domain::NormalizedEvent;
use what_changed_lib::privacy::{Sanitized, Sanitizer};
use what_changed_lib::storage::{
    event_repository, project_repository, Db, DbState, RecoveryAction,
};
use what_changed_lib::testing::{synthetic_event, temp_dir};

fn seeded(label: &str) -> std::path::PathBuf {
    let dir = temp_dir(label);
    let db = Db::open_in(&dir).unwrap();
    let sanitizer = Sanitizer::new();
    let events: Vec<Sanitized<NormalizedEvent>> = (0..200)
        .map(|seed| {
            sanitizer
                .sanitize(synthetic_event(seed, 1_000 + seed as i64))
                .0
        })
        .collect();
    db.insert_events(events).unwrap();
    db.flush().unwrap();
    // Fold the log into the main file so the damage below lands on real pages.
    db.write(|conn| {
        let _: (i64, i64, i64) = conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .unwrap();
        Ok(())
    })
    .unwrap();
    db.close();
    dir
}

/// Overwrite the middle half of the file with garbage. The header on page
/// one survives, so the file still opens; the pages that hold rows and
/// index entries do not.
fn damage(dir: &std::path::Path) {
    let path = dir.join("events.db");
    let size = fs::metadata(&path).unwrap().len();
    let mut file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(size / 4)).unwrap();
    file.write_all(&vec![0xAB; usize::try_from(size / 2).unwrap()])
        .unwrap();
}

/// Replace the whole file with text that is not a database at all.
fn replace_with_text(dir: &std::path::Path) {
    fs::write(
        dir.join("events.db"),
        b"this is not a database\n".repeat(64),
    )
    .unwrap();
    let _ = fs::remove_file(dir.join("events.db-wal"));
    let _ = fs::remove_file(dir.join("events.db-shm"));
}

#[test]
fn a_damaged_file_opens_into_the_corrupt_state_instead_of_failing() {
    let dir = seeded("recovery-open");
    damage(&dir);
    let db = Db::open_in(&dir).unwrap();
    assert!(
        matches!(db.state(), DbState::Corrupt { .. }),
        "{:?}",
        db.state()
    );
    assert!(db.flush().is_err(), "writes are refused, not attempted");
    assert!(db.read(event_repository::count).is_err());
}

#[test]
fn a_file_that_is_not_a_database_opens_into_the_corrupt_state() {
    let dir = seeded("recovery-not-db");
    replace_with_text(&dir);
    let db = Db::open_in(&dir).unwrap();
    assert!(
        matches!(db.state(), DbState::Corrupt { .. }),
        "{:?}",
        db.state()
    );
}

#[test]
fn reset_history_moves_the_file_aside_and_starts_fresh() {
    let dir = seeded("recovery-reset");
    replace_with_text(&dir);
    let db = Db::open_in(&dir).unwrap();
    let outcome = db.recover(RecoveryAction::ResetHistory).unwrap();
    assert!(outcome.succeeded);
    assert!(outcome.state.is_ready(), "{:?}", outcome.state);
    assert!(db.state().is_ready());

    let quarantined = outcome.artifact.expect("the damaged file was kept");
    let name = quarantined
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(
        name.starts_with("events.corrupt-") && name.ends_with(".db"),
        "{name}"
    );
    assert!(quarantined.exists(), "renamed, never deleted");

    assert_eq!(db.read(event_repository::count).unwrap(), 0);
    let sanitizer = Sanitizer::new();
    db.insert_events(vec![sanitizer.sanitize(synthetic_event(1, 1)).0])
        .unwrap();
    assert_eq!(db.read(event_repository::count).unwrap(), 1);
}

#[test]
fn export_diagnostics_writes_a_report_with_structure_and_no_rows() {
    let dir = seeded("recovery-diagnostics");
    damage(&dir);
    let db = Db::open_in(&dir).unwrap();
    let outcome = db.recover(RecoveryAction::ExportDiagnostics).unwrap();
    assert!(outcome.succeeded);
    let report = outcome.artifact.expect("a report was written");
    let text = fs::read_to_string(&report).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["state"]["state"], "corrupt");
    assert!(json["sqlite_version"].as_str().unwrap().starts_with('3'));
    assert!(json["database_bytes"].as_u64().unwrap() > 0);
    assert!(
        !text.contains("Synthetic Service"),
        "the report must carry no history"
    );
    // Exporting is not repairing.
    assert!(matches!(db.state(), DbState::Corrupt { .. }));
}

#[test]
fn repair_of_a_file_that_is_not_a_database_reports_failure_and_changes_nothing() {
    // Nothing can be read out of this file, so the outcome is not a matter of
    // which pages happened to survive: repair must say so, and must leave the
    // file exactly where the other two actions expect to find it.
    let dir = seeded("recovery-repair-hopeless");
    replace_with_text(&dir);
    let before = fs::read(dir.join("events.db")).unwrap();
    let db = Db::open_in(&dir).unwrap();
    let outcome = db.recover(RecoveryAction::Repair).unwrap();
    assert!(!outcome.succeeded, "{}", outcome.message);
    assert!(matches!(outcome.state, DbState::Corrupt { .. }));
    assert!(outcome.artifact.is_none(), "nothing was moved aside");
    assert!(
        outcome.message.contains("could not be rebuilt"),
        "{}",
        outcome.message
    );
    assert_eq!(
        fs::read(dir.join("events.db")).unwrap(),
        before,
        "the damaged file is untouched, so reset and diagnostics still work"
    );
    assert!(
        !dir.join("events.repair.db").exists(),
        "the half-built rebuild is cleaned up"
    );
}

#[test]
fn repair_of_a_partly_damaged_file_either_rebuilds_or_reports_and_never_loses_it() {
    // How much survives half a file of garbage is SQLite's call, so what is
    // asserted is the invariant that holds either way: the verdict agrees with
    // the state, and a database exists afterwards whichever way it went.
    let dir = seeded("recovery-repair");
    damage(&dir);
    let db = Db::open_in(&dir).unwrap();
    let outcome = db.recover(RecoveryAction::Repair).unwrap();
    assert_eq!(
        outcome.succeeded,
        outcome.state.is_ready(),
        "the verdict and the state must agree: {outcome:?}"
    );
    assert!(dir.join("events.db").exists(), "there is still a database");
    if outcome.succeeded {
        let original = outcome.artifact.expect("the damaged copy was kept");
        assert!(original.exists());
        // Whatever survived is readable and the schema is intact.
        db.read(event_repository::count).unwrap();
    } else {
        // Damage that cannot be read past is reported, and the file is left
        // exactly where it was for the other two actions.
        assert!(outcome.artifact.is_none());
        assert!(outcome.message.contains("could not be rebuilt"));
    }
}

#[test]
fn repair_on_an_intact_file_is_a_clean_rebuild_that_keeps_every_row() {
    let dir = seeded("recovery-repair-intact");
    let db = Db::open_in(&dir).unwrap();
    db.write(|connection| {
        project_repository::insert(connection, r"C:\dev\repair-v8", "repair-v8", 1).map(|_| ())
    })
    .unwrap();
    let before = db.read(event_repository::count).unwrap();
    let outcome = db.recover(RecoveryAction::Repair).unwrap();
    assert!(outcome.succeeded);
    assert_eq!(db.read(event_repository::count).unwrap(), before);
    assert_eq!(db.read(project_repository::count).unwrap(), 1);
}

#[test]
fn the_app_state_boots_over_a_corrupt_database() {
    let dir = seeded("recovery-app-state");
    replace_with_text(&dir);
    let state = AppState::open(AppPaths::under(&dir)).unwrap();
    assert!(matches!(state.db.state(), DbState::Corrupt { .. }));
    assert!(!state.startup.safe_mode);
}
