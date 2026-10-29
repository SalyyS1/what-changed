# Release checklist

**Evidence state: available checklist; unchecked rows are known limitations for this workspace, not passes.**

- [ ] `bun run check:versions`
- [ ] Target-resolved no-network graph passes
- [ ] Rust format, clippy and tests pass with measured Cargo parallelism
- [ ] Frontend check, tests, build and binding parity pass
- [ ] Brand source and launch-copy anti-claim checks pass
- [ ] P20 perf corpus, query plans, DB size and soak evidence attached
- [ ] Keyboard, Narrator, contrast, reduced-motion and DPI checks recorded
- [ ] NSIS raw and published installer hashes recorded
- [ ] If enabling the opt-in updater: HTTPS feed, embedded public key and signed `.sig` artifact verified (current scaffold intentionally leaves this unchecked)
- [ ] Upgrade preserves seeded history; uninstall keeps data by default
- [ ] Diagnostics export contains no fixture name, path or secret
- [ ] Privacy purge scans main DB, WAL, SHM and FTS shadow tables
- [ ] Acceptance walkthrough has 15 completed rows
- [ ] Release audit rows have matching evidence
- [ ] Public packet names the exact tag, URLs, hashes and frozen copy
- [ ] User gives one approval for that frozen packet
