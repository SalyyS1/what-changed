//! The updater scaffold is deliberately I/O free until a signed release feed
//! and a release key are available.

use what_changed_lib::commands::meta::update_check_response;

#[test]
fn opted_in_request_is_explicitly_not_configured_and_does_not_check_network() {
    let response = update_check_response("0.1.0".to_owned(), true);

    assert!(!response.supported);
    assert!(response.opted_in);
    assert_eq!(response.current_version, "0.1.0");
    assert!(response.message.contains("No network request was made"));
}

#[test]
fn update_check_requires_opt_in() {
    let response = update_check_response("0.1.0".to_owned(), false);

    assert!(!response.supported);
    assert!(!response.opted_in);
    assert!(response.message.contains("opt-in"));
}
