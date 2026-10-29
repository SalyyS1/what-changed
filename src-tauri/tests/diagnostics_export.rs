//! The report contract is intentionally fixture-free: the command's header
//! states that event content, paths and entity names are not exported.

#[test]
fn diagnostics_contract_is_explicit_in_the_command_source() {
    let source = include_str!("../src/commands/diagnostics.rs");
    assert!(source.contains("no event content, paths or entity names"));
    assert!(source.contains("512 * 1024"));
    assert!(source.contains("recent_logs_redacted"));
}
