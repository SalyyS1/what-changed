# Tauri binary discovery repair

The approved-notice installer attempt compiled Rust successfully in 13m46s,
then failed while renaming nonexistent `seed_history_grouping.exe`. No installer
or native acceptance was claimed from that failed attempt.

Cause verified against [pinned Tauri CLI 2.11.4 source](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-cli/src/interface/rust.rs#L885):
binary discovery scans all immediate `src/bin` entries, including directories
without Cargo entry points. Cargo metadata lists only the real application,
seed_history and resource_probe; the latter two require devtools. `default-run`
alone would leave bogus support binaries in the bundle list.

Changed: moved seeder support groups to `src/bin_support/`, kept 22 file bodies
byte-identical, updated three module paths and the two existing exact architecture
scan exemptions. The third moved directory was empty. A narrow architecture
regression gate requires immediate `src/bin` entries to be executable Rust source
files. No dummy binary, dependency, lockfile, feature or CLI-version change.

Move boundaries and source/post-move hashes are recorded privately in
`.qa/binary-support-relocation-20261027.json`. Current input recapture matches all
seven approved hashes, including raw Cargo metadata. Normal notice check retains
SHA-256 `aea1d355d6864858959b2a9e4c8ee628acd56718d1e6a8b2620961f344f8c6a5`.
Source approval remains applicable; it was not silently refreshed.

Validation: formatting passed; seeder 23 passed / 3 intentional ignored proof
profiles. Architecture first exposed a separate false-positive on the externally
declared `soak/tests.rs`. Its parent also placed the cfg(test) declaration before
production functions, causing the text scanner to omit them. The fix uses one
exact test-file exemption with a cfg-only ownership regression, and moves the
declaration after production. All 12 architecture tests then passed; actual soak
production functions and helper modules remain covered. No broad test-name or
directory exemption was added.

Normal Tauri packaging now passes (exit 0); its generated NSIS has the intended
application binary and reviewed notice payload. [Candidate evidence](qa-261027-0059-installed-candidate.md).
Failed log: `.qa/installer-build-20261027.log`. Focused checks:
`.qa/binary-support-relocation-tests-20261027.log` and
`.qa/architecture-external-test-module-20261027.log`.

Unresolved: installed/native acceptance and full release gates.
