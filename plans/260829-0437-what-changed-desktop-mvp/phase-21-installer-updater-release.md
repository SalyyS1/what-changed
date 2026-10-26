---
phase: 21
title: "NSIS installer, in-place upgrade path, release audit and requirement traceability"
status: in-progress
priority: P1
effort: 7h
dependencies: [20]
---

## Overview

Ship it: a per-user NSIS installer that needs no administrator, an upgrade path that migrates an
existing database instead of orphaning it, an uninstall that asks before deleting a user's history, and
a final audit that maps every requirement §1–§91 to the phase and test that satisfies it. The MVP has
**no automatic or background update check**. D-09 allows only a future, explicitly initiated updater;
until a signed HTTPS feed and release public key are supplied, Settings shows a safe unconfigured
scaffold and downloads remain manual.

## Requirements

**Functional**
- Bundle: `bundle.targets = ["nsis"]` (already set in P01). NSIS config in `tauri.conf.json`:
  `installMode: "currentUser"` (per-user, no UAC), `displayLanguageSelector: false`,
  `installerIcon`, `headerImage`/`sidebarImage` if the assets exist (skip rather than invent art),
  `license` omitted unless a LICENSE file exists, `compression: "lzma"`. Verify every key against
  `src-tauri/gen/schemas/desktop-schema.json` before writing it — schema-verified keys only.
- Artifact naming, both names. Tauri's NSIS bundler produces
  `What Changed_<version>_x64-setup.exe` (measured on this machine: the P01 scaffold produced
  `What Changed_0.1.0_x64-setup.exe`, 1.12 MB). §65 and `plan.md`'s release criterion name
  `WhatChanged-Setup-x64.exe`. Both are satisfied rather than one being redefined: after
  `bun run tauri build`, copy `src-tauri/target/release/bundle/nsis/*-setup.exe` to
  `dist-installer/WhatChanged-Setup-x64.exe` and record **both** filenames plus their SHA-256 in the
  release audit. The copy is the published download; the bundler output is the raw artifact. Version
  comes from `src-tauri/Cargo.toml` (D-03 — `tauri.conf.json` omits `version` so there is one source),
  and `scripts/check-versions.mjs` asserts `package.json` agrees, failing on drift.
- Shortcuts (§65): Start Menu shortcut always, desktop shortcut offered as an installer option rather
  than forced. Both are configured through keys verified against
  `src-tauri/gen/schemas/desktop-schema.json` first, like every other NSIS key here. If the pinned
  Tauri version's schema has no key for one of them, the requirement is met through the generated NSIS
  template's own shortcut section, and the release audit records which mechanism was used — a §65
  clause is never marked satisfied without naming how.
- Data location and upgrade: history lives in `%APPDATA%\dev.whatchanged.app\` (resolved via Tauri's
  `app_data_dir`, never a hardcoded path). Installing a newer version over an older one must leave that
  directory untouched and let migrations run at next start. Verified by an actual two-build test: install
  v0.1.0, seed 10k events, install v0.1.1 (with an added migration `004_test_upgrade.sql` used only in
  this rehearsal branch, then reverted), launch, and assert the event count and the new schema version.
- Uninstall: NSIS uninstaller removes the program files; the app-data directory is **kept by default**,
  and the uninstaller shows a checkbox "Also delete my change history" (unchecked). If the checkbox
  cannot be added within the NSIS template that Tauri generates, the fallback is: never delete data on
  uninstall, and document the folder path in `README.md` plus the in-app privacy screen (which already
  reveals it). Silent deletion of a user's history is not an option.
- First-run-after-install correctness: a fresh install on a machine with no WebView2 runtime must not
  crash. NSIS `webviewInstallMode: { type: "downloadBootstrapper" }` is the Tauri default and requires
  network at install time only — record this as the one install-time network touch, distinct from the
  runtime no-network promise, and state it in `README.md`. (If a fully offline installer is required,
  the alternative is `embedBootstrapper` or a fixed-runtime bundle, which is a size/scope decision for
  the user — raised, not chosen.)
- Unsigned binary reality: there is no code-signing certificate on this machine. The installer will
  trigger SmartScreen ("Unknown publisher"). `README.md` documents this plainly with the exact dialog
  wording and the "More info → Run anyway" path, and `docs/deployment-guide.md` records what signing
  would require (an EV/OV certificate and `signtool`/`tauri signer` wiring) as a future step. No fake
  publisher string, no claim of signing.
- Development vs production signing separation (§66), recorded rather than silently absent: there is no
  development signing identity on this machine and none is created. Production signing would use a
  separate certificate from a separate credential source, supplied to the release run as an environment
  secret and never written to the repository — which the plan already guarantees structurally, because
  it introduces no secret anywhere. `docs/deployment-guide.md` carries this as its signing section, so
  §66 gets a traceability row instead of an inference.

- Open-source release gate: before any public artifact is published, add the canonical **MIT License**
  selected by the user on 2026-09-10, confirm third-party notices/dependency obligations, and make
  package metadata, README and installer agree.
- Release build hygiene:
  - `cargo build --release` with `panic = "abort"`, `lto = "thin"`, `codegen-units = 1`,
    `strip = "symbols"` in `[profile.release]`; `opt-level = "s"` evaluated against the perf gates —
    keep whichever passes P20's budgets and record the choice.
  - `devtools` feature **off**: assert the release binary contains no `simulate_event`, no
    `seed_history`, no `#/dev/` route (symbol/string scan in `tests/release_audit.rs`, run against the
    built binary and the built frontend assets).
  - Frontend built with `bun run build`; no source maps in the shipped bundle; `withGlobalTauri: false`
    confirmed; CSP present in the final `tauri.conf.json`.
- Release audit (`docs/release-audit.md`, generated by hand in this phase):
  1. **Requirement traceability table**: every requirement §1–§91 → the phase that implements it → the
     test or manual check that proves it → status (Done / Deferred-by-user / Not-verifiable-here). The
     only permitted feature deferrals are the user's own: §26 health checks and the §75–§78 future
     feature bundle (the brief groups it under one "architect for, do **NOT** build" heading, and
     `plan.md` G9 carries the architecture-readiness commitment). Do not infer narrower feature
     numbering from that digest. Docker's live path is marked Not-verifiable-here (D3); accepted
     §39 Home-ambient omission and irreversible §90 first-task sequencing miss remain explicit
     Not-verifiable-here limitations, never passes or new feature deferrals.
     **Each citation is checked against what it claims to implement, not merely that the number
     appears.** The audit's own precedent is §48: three phases cited it while describing the privacy
     screen, so a citation count alone would have reported the System Map covered when no phase built
     it (red-team Y-6). A row whose named evidence does not match the requirement's subject is a
     blocking finding exactly like a missing row.
  2. **Non-negotiables checklist**: no Electron, no Go, no second process, no service, no account, no
     cloud, no telemetry, no elevation, offline at runtime — each with the evidence that proves it
     (dependency list, process tree description, capability allowlist test, and the absence of any HTTP
     client **in the Windows build graph**). `scripts/check-no-network.mjs` must resolve
     `cargo tree --target x86_64-pc-windows-msvc --edges normal` and fail if `reqwest`, `hyper`, `ureq`
     or `isahc` appears there — **not** grep `Cargo.lock`. Measured on this machine: `reqwest` and
     `hyper` are present in `Cargo.lock` because Tauri depends on them for non-Windows targets, and
     absent from the resolved Windows graph. A lockfile grep therefore fails on a clean scaffold forever
     and would be disabled by the first person who hit it, taking the real guarantee with it.
  3. **Measured numbers**: the P20 baseline table plus the B2 verdict.
  4. **Known gaps**: B3 short-lived processes, WinHTTP machine proxy, XML config diffing if it degraded,
     Docker live path, unsigned installer, §26 and §75–§78 deferrals.
- Manual acceptance script (`docs/acceptance-walkthrough.md`): 15 steps a human runs on a clean install —
  onboarding, watch a folder, change a `.env`, see the event, open details, run an investigation on a
  seeded break, view the map, create and compare a checkpoint, pause/resume from the tray, delete a
  category, export diagnostics, restart the app and confirm history survives, check the 900×600 window,
  keyboard-only pass, and confirm no network activity (Resource Monitor, app row shows zero bytes).

**Non-functional**
- Installer ≤ 25 MB (Tauri's typical range; a larger number is recorded, not hidden).
- Install → first launch → visible window ≤ 5 s on a clean machine.
- The whole release can be produced by one documented command sequence with no manual file edits.

## Architecture

No new runtime code beyond the audit test. Artifacts: `scripts/{check-versions.mjs,check-no-network.mjs,
release.ps1}`, `src-tauri/tests/release_audit.rs`, `docs/{release-audit.md,deployment-guide.md,
acceptance-walkthrough.md}`, and the NSIS section of `tauri.conf.json`. `release.ps1` runs the gates in
order (versions → clippy → cargo test → bun check/test/build → no-network check → tauri build → release
audit test against the artifact) and stops at the first failure.

## Related Code Files

**Create** — `scripts/{check-versions.mjs,check-no-network.mjs,release.ps1}`;
`src-tauri/tests/release_audit.rs`; `docs/{release-audit.md,acceptance-walkthrough.md}`;
`src-tauri/tests/upgrade_rehearsal.rs`.
**Modify** — `src-tauri/tauri.conf.json` (nsis block, version), `src-tauri/Cargo.toml`
(`[profile.release]`), `package.json` (`release`, `check:versions`, `check:no-network` scripts),
`README.md` (install instructions, SmartScreen note, data folder, install-time WebView2 note),
`docs/deployment-guide.md`, `ROADMAP.md` (signing, offline installer, auto-update decision),
`docs/project-overview-pdr.md` (final scope statement).
**Delete** — nothing in the repo; the rehearsal migration `004_test_upgrade.sql` is created and reverted
within step 4 and must not exist in the final tree (asserted by the migration test's file list).

## Implementation Steps

1. `scripts/check-versions.mjs` and `scripts/check-no-network.mjs` first — they are cheap gates that must
   be true from now on.
2. NSIS config keys verified against `gen/schemas/desktop-schema.json`, then `bun run tauri build`;
   inspect the produced installer name, size, and that it installs without a UAC prompt.
3. Install into a clean user context (a second Windows user account, or a VM if available; if neither is
   available, record that the clean-machine install was verified only on this account and list it as a
   gap rather than claiming otherwise).
4. Upgrade rehearsal: build v0.1.0, install, seed 10k, add `004_test_upgrade.sql`, bump to v0.1.1, build,
   install over the top, launch, assert count + schema version, then revert the rehearsal migration and
   re-run the migration test suite.
5. Uninstall behaviour: attempt the "Also delete my change history" checkbox via Tauri's NSIS template
   hooks; if unavailable, take the documented keep-data path.
6. `tests/release_audit.rs`: scan the release binary and `dist/` for dev-only strings/symbols; assert the
   CSP and `withGlobalTauri: false` in the built config; assert no source maps in `dist/`.
7. Write `docs/release-audit.md` with the four sections, walking §1–§91 one by one. Any requirement whose
   evidence cannot be named is a **blocking finding** — it goes back to its phase, it does not get a
   check mark.
8. Run the 15-step manual acceptance walkthrough on the installed build and record the result of each step.
9. `release.ps1` wiring, then a full clean run of it end to end.
10. Commits: (a) `chore: version and network guards`, (b) `feat: nsis installer configuration`,
    (c) `test: release artifact audit`, (d) `test: upgrade rehearsal`,
    (e) `docs: deployment guide and smartscreen note`, (f) `docs: release audit and traceability`,
    (g) `docs: acceptance walkthrough results`, (h) `chore: one-command release script`.

## Todo (commit boundaries)

- [x] Version-drift and target-resolved no-network guards
- [x] Schema-verified NSIS configuration, installer built
- [x] Release-artifact audit test (no devtools, CSP, no source maps)
- [ ] Upgrade rehearsal with a real migration
- [x] Uninstall data decision implemented and documented
- [x] `docs/release-audit.md` traceability §1–§91 (91 unique rows; 87 cited files verified to exist on 2026-10-07)
- [ ] Manual acceptance walkthrough executed and recorded
- [x] One-command release script green end to end

## Success Criteria

- [ ] Installer builds, installs with **no UAC prompt**, and launches to a visible window ≤ 5 s
- [x] Both installer filenames exist and are recorded with their SHA-256: the bundler's
      `What Changed_<version>_x64-setup.exe` and the published `WhatChanged-Setup-x64.exe`
- [ ] A Start Menu shortcut exists after install, and the desktop shortcut appears only when the user
      chose it; the audit names which mechanism produced each
- [x] `check-no-network.mjs` resolves the Windows target graph (not `Cargo.lock`) and finds no HTTP
      client in it
- [x] `check-versions.mjs` passes with all version sources equal
- [x] `release_audit.rs`: zero dev-only strings/symbols in the release artifact; CSP present;
      `withGlobalTauri: false`; no `.map` files in `dist/`
- [ ] Upgrade rehearsal: 10k events survive a version-over-version install and the new migration applies
- [ ] Uninstall does not delete history unless the user explicitly opts in
- [x] `docs/release-audit.md` has a row for every requirement §1–§91 with a named test or manual check;
      the only Deferred feature rows are §26 and §75–§78; Not-verifiable rows include Docker-live,
      recorded clean-machine gaps, the accepted §39 Home-ambient omission and historical §90 sequence miss
- [ ] All 15 acceptance steps pass, including Resource Monitor showing zero network bytes for the app
      during a 10-minute session
- [ ] `powershell -File scripts/release.ps1` completes green from a clean tree

## Verification Gate

## Verification evidence — 2026-09-22

- `scripts/release.ps1 -AllowDirty -SkipInstaller -SkipPerf` completed green after the release-gate
  fixes: version agreement, Windows target dependency graph, no-network graph, fmt, strict Clippy,
  444 Rust tests / 1 ignored, bindings parity, 57 frontend files / 314 tests, production frontend
  build, bundle budget (137,890 gzip bytes), docs links and upgrade-rehearsal guard.
- After the Phase 20 storage changes, the same rehearsal was re-run with a D:-backed temp directory.
  The functional Rust suite remained 444 passed / 1 ignored; two attempts stopped on live timing/order
  flakes (`perf_system_map` under concurrent load and one dev-environment timestamp ordering test),
  while both tests passed in isolated reruns. This is recorded as a reproducibility caveat, not a green
  public-release gate.
- The rehearsal intentionally skipped the real Tauri installer and 1M performance run. Therefore the
  installer artifact, SHA-256 pair, clean-machine install, NSIS shortcut/uninstall behavior, release
  binary audit and full manual acceptance remain open and are not marked complete.
- The release script now validates `dist` as a directory, and diagnostics export has the narrowly
  scoped `dialog:allow-save` permission covered by the exact capability test.

## Verification evidence — 2026-09-23 continuation

- `scripts/verify-installer.ps1` validates exactly two checksum-manifest entries
  (raw and published) and the preserved 0.1.0 fixture passes with both files at
  3,196,937 bytes and SHA-256
  `570825495fa297409d3c1141e71059a16e105d97fe6f356a3c5a4e07efb78ee5`.
- `upgrade_rehearsal` passes 2/2 and now asserts the stable app identifier;
  the release audit compiles and its synthetic-config/dist scan passes 1/1.
- These are source/fixture gates only. A newly built installer, clean-machine
  install/upgrade/uninstall and GUI SmartScreen/WebView2 evidence remain open.
- The release script labels its optional perf invocation as a 10k smoke gate;
  the 100k/1M corpus runs and four-hour soak remain explicit Phase 20/P25 work.

## Verification evidence — 2026-09-24 release candidate build

- `scripts/release.ps1 -AllowDirty -SkipPerf` completed green end to end: version/no-network,
  formatting/lint/tests, bindings, frontend typecheck/tests/build/bundle, docs/brand/launch,
  Tauri build, checksum verification, release-artifact audit and upgrade guard. The optional
  10k performance smoke was intentionally skipped; the explicit P20 perf gate remains separate.
- The final `bun run tauri build` within that rehearsal produced the fresh x64 NSIS artifact
  `What Changed_0.1.0_x64-setup.exe` (4,422,587 bytes). It was copied to the
  published name `dist-installer/WhatChanged-Setup-x64.exe`; both files have SHA-256
  `d49d29ff233511c4cab6816bb16c9b0d68e6ac649cfad6908231a24b0840f9a9`.
- `scripts/verify-installer.ps1` passed against the raw/published pair and the two-entry
  `dist-installer/SHA256SUMS.txt`. The artifact audit passed 1/1 against the newly built
  `target/release/WhatChanged.exe` and current `dist/` (no dev-only markers/source maps,
  CSP and `withGlobalTauri: false`). `upgrade_rehearsal --include-ignored` passed 2/2,
  but its two-version install migration test remains explicitly manual/ignored.
- `scripts/check-nsis-script.mjs` now runs against the generated `installer.nsi` and proves the
  per-user execution level, Start Menu/desktop shortcut sections, and the NSIS
  `Delete app data` checkbox guard around both roaming and local app-data removal. This closes
  the implementation/documentation decision; it does not claim a successful installed uninstall.
- A local per-user smoke install returned exit 0, created the HKCU uninstall entry and Start Menu/
  desktop shortcuts, and launched a visible window in 2,096 ms. The silent uninstaller could not
  launch its extracted helper because this account's `%LOCALAPPDATA%\What Changed` files are EFS
  encrypted (`NSIS Error: Error launching installer`); the app data remained intact. This is an
  environment-specific limitation, so clean-machine uninstall/data-preservation evidence remains
  open. No GUI SmartScreen/WebView2 walkthrough or 15-step manual acceptance is claimed.

## Verification evidence — 2026-09-26 release rehearsal

- `scripts/release.ps1 -AllowDirty -SkipPerf` reached the installer and all post-build artifact gates
  after the brand, tray, explanation-provider and updater-scaffold changes. The source gates included
  454 Rust tests passed / 1 ignored, 319 frontend tests passed, typecheck, bindings, bundle, docs,
  brand and launch checks.
- The fresh NSIS raw and published artifacts are 4,458,019 bytes each and both verify with SHA-256
  `6a5aa3963b5e5bfbe5dd9a8d1e2d8a02e5b97d24654092de432bbc97a71aaa9e`. `check-nsis-script.mjs`
  passes against the generated script and confirms all five installer artwork binaries match the
  configured brand sources, in addition to the per-user and shortcut/data-deletion guards.
- `release_audit` passed 1/1 against the built binary and current `dist/`; `upgrade_rehearsal`
  passed 2/2 including its ignored migration-contract test. This is still a dirty-tree rehearsal with
  `-SkipPerf`, not a clean release or installed-app acceptance result.
- `scripts/release.ps1` now resolves the release artifact directory from `CARGO_TARGET_DIR` when set,
  so a D:-backed Cargo target is audited instead of being mistaken for a stale default-target file.
- The Settings updater is an explicit opt-in, no-I/O scaffold. It reports no configured signed feed or
  public key and makes no request; production updater enablement remains blocked on those release inputs.

## Verification evidence — 2026-10-14 engineering rehearsal

- `powershell -File scripts/release.ps1 -AllowDirty -SkipInstaller -SkipPerf` completed green. The run
  passed the target-resolved no-network graph, rustfmt, strict lint, 486 Rust unit/integration test
  binaries (483 passed / 3 ignored in the library suite), bindings parity, 65 frontend files / 339
  tests, typecheck, production build, 142,076-byte gzip bundle budget, documentation links, brand
  manifest and launch-copy claim checks, plus the upgrade-contract guard.
- The same source was attempted with the installer enabled; it reached the legal payload gate and was
  correctly blocked by `generate-third-party-notices.mjs --prepare` because exact-version license/text
  review is still pending. No notice text was invented and no public artifact is claimed from this run.
- Phase 20's 10k/100k/1M performance matrix and all installer/native/manual acceptance rows remain open;
  the engineering rehearsal intentionally skipped them and does not change the public-v1 NO-GO packet.
- A non-packaged current-source engineering binary was built with `cargo build --release --locked
  --no-default-features` after the frontend build. A 12-second smoke launch under the current Windows
  account stayed alive with window title `What Changed` and `Responding=True`, then the test-owned PID
  was stopped cleanly. This proves binary startup only; it is not installer/UAC/shortcut/uninstall or
  15-step acceptance evidence, and no current-profile history migration or purge was performed.

```bash
cd /d/Project/.WhatChage
bun run check:versions && bun run check:no-network
cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test
cd /d/Project/.WhatChage && bun run check && bun run test && bun run build
bun run tauri build
cd src-tauri && cargo test --test release_audit --test upgrade_rehearsal -- --include-ignored
cd /d/Project/.WhatChage && powershell -ExecutionPolicy Bypass -File scripts/release.ps1
```

## Risk Assessment

Source collection on 2026-10-05 independently verifies bundled SQLite's official
source hash and all three WebView2 SDK candidate byte identities, and collects
complete pending license/NOTICE transcriptions. Immutable package manifests and
the frontend release tag/publication metadata support eleven missing-text
candidate sources. The raw Cargo metadata snapshot differs from the preserved
draft despite matching runtime identities/manifests; no cause or reconciliation
is claimed. Review choices/obligations, final payload and installed notice gates
remain open; manifest status/approval fields are unchanged.
[Native source correspondence](../reports/qa-261005-native-notice-source-correspondence.md),
[full-text collection](../reports/qa-261005-missing-license-source-collection.md).

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| An updater could silently broaden the network promise (D-09) | A future build checks on startup or accepts an unsigned feed | Keep checks explicitly user-initiated; require a pinned HTTPS feed, embedded public key, signature verification and no-background-call tests before enabling the updater. The current scaffold makes no request and remains manual |
| SmartScreen blocks users on an unsigned installer | "Windows protected your PC" on a clean machine | Documented in `README.md` with the exact click path; signing is a recorded future step with its concrete requirements. No pretend-signing, no publisher spoofing |
| WebView2 bootstrapper needs network at install time | Install fails offline | Documented as install-time-only; the alternative bundle modes and their size cost are presented to the user as a decision, not chosen here |
| NSIS config key invented and silently ignored | Build succeeds but the setting has no effect | Every key is checked against `gen/schemas/desktop-schema.json` first; unknown keys are not written |
| Uninstall deletes a user's history | Data gone after uninstall | Default is keep; the opt-in checkbox is attempted, and the fallback is keep-always. This is a hard rule, not a preference |
| Upgrade orphans the database (identifier or path change) | Post-upgrade event count is 0 | Identifier `dev.whatchanged.app` is frozen from P01 and asserted by the upgrade rehearsal; changing it later would require a data-migration phase |
| A requirement has no evidence at audit time | A traceability row with an empty test column | The row blocks the release; the work returns to its phase. The audit is not allowed to mark it Done on the basis of "looks implemented" |
| Release script hides a failure | Green run with a skipped step | `release.ps1` uses `$ErrorActionPreference = "Stop"`, echoes each step, and prints a final summary listing every gate with its exit code |

## Security Considerations

### Source notice approval — 2026-10-27

The owner approved the [frozen packet](../reports/approval-261027-0059-notice-packet.md):
232 package candidates, two native components and 263 referenced texts. Review
manifest SHA-256 `8f1cf708cf950e86bc86eedcbb562138475b34c380a16d074d61b23600508f87`.
Normal generator prepare/check passed; notice payload SHA-256
`aea1d355d6864858959b2a9e4c8ee628acd56718d1e6a8b2620961f344f8c6a5`.
Existing notice suites: 96/96 passed. The pending source proposal remains frozen
as the object approved. This supersedes the pending-review status in the dated
2026-10-05 source collection above. Installer/installed binding, native acceptance
and publication approval remain open; no phase checkbox closes from source review.

- Per-user install (`currentUser`) means no elevation at install time and no writes outside the user's
  profile — consistent with the non-elevated runtime promise.
- The no-network guard is the mechanical proof of the offline claim, and it resolves the Windows target
  graph rather than the lockfile — a lockfile grep would fail on Tauri's non-Windows transitive
  dependencies and get disabled, which is how a real guarantee becomes a disabled script. It runs on
  every release.
- Release binaries exclude the `devtools` feature, so `simulate_event`, the seeder, and the `#/dev/*`
  routes cannot be reached in a shipped build.
- The unsigned-binary limitation is disclosed rather than worked around; users are told what they will
  see and why.
- `docs/release-audit.md` contains no machine identifiers, and the acceptance walkthrough instructs the
  operator to redact paths that contain a real username before pasting results.
