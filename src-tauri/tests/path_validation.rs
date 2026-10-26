#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use what_changed_lib::app::validate;
use what_changed_lib::testing::temp_dir;

#[test]
fn real_folder_is_canonicalized_and_app_data_is_denied() {
    let app_data = temp_dir("path-validation-app-data");
    let watched = temp_dir("path-validation-watched");
    let canonical = validate::watched_folder(&watched.to_string_lossy(), &app_data).unwrap();
    assert!(canonical.is_absolute());
    let own_error = validate::watched_folder(&app_data.to_string_lossy(), &app_data).unwrap_err();
    assert!(own_error.to_string().contains("never watched"));
}

#[test]
fn all_of_program_data_remains_denied_not_only_the_microsoft_subtree() {
    let Ok(program_data) = std::env::var("ProgramData") else {
        return;
    };
    if !std::path::Path::new(&program_data).is_dir() {
        return;
    }
    let app_data = temp_dir("path-validation-program-data");
    let error = validate::watched_folder(&program_data, &app_data).unwrap_err();
    assert!(error.to_string().contains("system folders"));
}

#[cfg(windows)]
#[test]
fn a_directory_reparse_point_is_denied_when_the_host_can_create_one() {
    use std::os::windows::fs::symlink_dir;

    let app_data = temp_dir("path-validation-reparse-data");
    let target = temp_dir("path-validation-reparse-target");
    let parent = temp_dir("path-validation-reparse-link");
    let link = parent.join("linked-folder");
    if symlink_dir(&target, &link).is_err() {
        // Windows permits this without elevation only when Developer Mode is
        // enabled. The production check itself is also covered by unit tests.
        return;
    }
    let error = validate::watched_folder(&link.to_string_lossy(), &app_data).unwrap_err();
    assert!(error.to_string().contains("shortcut"));
}
