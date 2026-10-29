//! Upgrade rehearsal guard. The actual two-version install is a Windows GUI
//! operation; this ignored test prevents the temporary rehearsal migration
//! from accidentally becoming part of the shipped tree.

#![allow(clippy::expect_used)]

use std::{fs, path::PathBuf};

fn config() -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let text = fs::read_to_string(&path).expect("release config is readable");
    serde_json::from_str(&text).expect("release config is valid JSON")
}

#[test]
fn upgrade_contract_keeps_stable_application_identity() {
    assert_eq!(
        config()["identifier"],
        "dev.whatchanged.app",
        "changing the Tauri identifier would orphan the per-user app-data directory"
    );
    assert!(
        !PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("migrations/004_test_upgrade.sql")
            .exists(),
        "temporary upgrade migration must not ship"
    );
}

#[test]
#[ignore = "requires two installed NSIS versions"]
fn rehearsal_migration_is_not_part_of_the_final_tree() {
    assert!(
        !PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("migrations/004_test_upgrade.sql")
            .exists(),
        "temporary upgrade migration must be reverted before release"
    );
}
