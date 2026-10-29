//! Onboarding state contract tests.

#[test]
fn onboarding_command_has_a_reopen_path_and_completion_checkpoint() {
    let source = include_str!("../src/commands/onboarding.rs");
    assert!(source.contains("reopen_onboarding"));
    assert!(source.contains("complete_onboarding"));
    assert!(source.contains("CheckpointKind::Automatic"));
}
