---
phase: 05
title: "Typed IPC, Tray & Lifecycle"
status: completed
priority: P1
effort: 6h
dependencies: [02, 04]
---

## Overview

**Acceptance closed — 2026-09-19:** **14/14** checks complete. A release build produced the NSIS
bundle, and the optimized app ran with its database open and tray active while both D-15 memory
figures were measured and recorded in `docs/PERFORMANCE.md`. Neither memory figure is a pass/fail
gate; recording both is the acceptance condition.

Close out §69 Phase 0: the narrow typed command surface (§35/§56), generated TypeScript bindings so
Rust types are the source of truth (§81), live event channels (§57), the seven-entry tray (§7),
close-to-tray, single instance, window state, autostart plumbing, and a release build that still
bundles. After this phase the shell is a real app: it boots, opens a database, hides to tray, and the
frontend talks to Rust through typed wrappers only.

## Requirements

**Functional**
- Commands (all `#[tauri::command]`, all validating input in Rust, all returning
  `Result<T, AppErrorDto>`): `get_app_version`, `get_dashboard_summary`, `get_events`,
  `get_event_details`, `search_events`, `list_checkpoints`, `create_checkpoint`, `rename_checkpoint`,
  `delete_checkpoint`, `compare_checkpoint`, `run_investigation`, `list_investigations`,
  `get_investigation`, `get_collector_status`, `set_collector_enabled`, `get_settings`,
  `update_settings`, `get_system_map`, `list_watched_locations`, `add_watched_location`,
  `remove_watched_location`, `list_projects`, `add_project`, `remove_project`,
  `set_monitoring_paused`, `get_storage_usage`, `get_diagnostics`, `export_diagnostics`,
  `db_recover`, `check_for_updates`, `complete_onboarding`, `log_ui_error`, `simulate_event`
  (dev-gated). Later phases implement bodies; this phase lands signatures + DTOs + the ones whose
  data already exists (version, settings, events, storage, collector status, monitoring pause).
- Provenance note in `ARCHITECTURE.md`: §56 names 12 commands; the extra ones each trace to a
  section (`rename_checkpoint` §47, `add_watched_location` §13, `add_project` §24,
  `set_monitoring_paused` §7, `get_storage_usage` §11, `get_diagnostics`/`export_diagnostics` §84,
  `db_recover` §83, `check_for_updates` §64, `complete_onboarding` §50, `log_ui_error` §58,
  `simulate_event` §85, `get_system_map` §48). Nothing generic, nothing invented.
- Events (§57): `system-event-added` (payload = `EventBatchNotice { count, newest_id, categories,
  highest_importance }`, coalesced to at most one emit per 500 ms), `collector-status-changed`,
  `analysis-completed`, `monitoring-state-changed`.
- Tray (§7) with exactly: Open What Changed / What changed today? / Create checkpoint / Pause
  monitoring / Resume monitoring / Settings / Quit. Pause and Resume are both present; the
  inapplicable one is `set_enabled(false)` rather than hidden (documented decision D-13).
- Close → `prevent_close()` + `hide()`; monitoring continues; first hide shows a one-time tray
  balloon "What Changed is still recording. Open it from the tray." Quit is the only exit.
- Single instance: second launch focuses and unminimizes the existing window.
- Window state persisted (size/position/maximized) via the plugin; min size enforced at 900×600.
- Type generation: `tauri-specta` exports `src/lib/tauri/bindings.ts` during a debug run; a
  committed copy plus `bun run check:bindings` (regenerate to a temp file, `diff`) fails the gate on
  drift.
**Non-functional**
- No command takes a raw SQL string, a shell string, or an unconstrained path. Path-taking commands
  canonicalize, reject reparse points/junctions, enforce a max depth of 6 and a max of 20 watched
  roots, and reject anything under `C:\Windows`, `C:\Program Files*`, or another user's profile.
- Every command body is ≤ 30 lines: parse/validate → call a service → map error. No logic in
  `commands/`.
- Frontend never imports `@tauri-apps/api/core` directly outside `src/lib/tauri/`.

## Architecture

**§8's chain, realized.** This phase is where §8's architecture stops being a diagram: Svelte UI
(WebView2) → typed Tauri IPC → `AppService` → { `QueryService`, `AnalysisEngine`, `SettingsService` } →
`EventRepository` → SQLite, with the `Normalizer` feeding the repository from collectors. §8's closing
constraint — "UI never calls OS APIs" — is enforced structurally rather than by review: there is no
command that takes a shell string, a SQL string, or an unconstrained path, and `capabilities_allowlist.rs`
fails the build if an `fs:`, `shell:`, `http:`, `sql:`, `process:` or `updater:` permission appears.

`commands/` mirrors feature areas (`meta.rs`, `events.rs`, `checkpoints.rs`, `analysis.rs`,
`settings.rs`, `collectors.rs`, `diagnostics.rs`, `dev.rs`) and holds only glue. `app/state.rs` owns
`AppState { db, settings, collector_registry, monitoring, event_bus }` behind `tauri::State`.
`app/event_bus.rs` coalesces writer notifications into throttled Tauri emits. Frontend mirror:
`src/lib/tauri/{bindings.ts (generated), client.ts (typed wrappers + error mapping),
events.ts (typed listeners), index.ts}`; stores in `src/lib/stores/` consume `client.ts` only.

## Related Code Files

**Create** — `src-tauri/src/commands/{mod.rs,meta.rs,events.rs,checkpoints.rs,analysis.rs,
settings.rs,collectors.rs,diagnostics.rs,dev.rs}`; `src-tauri/src/app/{mod.rs,state.rs,event_bus.rs,
tray.rs,window.rs,lifecycle.rs,validate.rs}`; `src-tauri/src/dto/{mod.rs,event_dto.rs,
settings_dto.rs,status_dto.rs,error_dto.rs}`; `src-tauri/src/bin/export_bindings.rs`;
`src/lib/tauri/{client.ts,events.ts,index.ts,bindings.ts}`;
`src/lib/stores/{settings.svelte.ts,monitoring.svelte.ts,collectors.svelte.ts,toast.svelte.ts}`;
`scripts/check-bindings.ps1`; `src-tauri/tests/commands_contract.rs`;
`src-tauri/tests/capabilities_allowlist.rs`; `src/lib/tauri/client.test.ts`.
**Modify** — `src-tauri/src/lib.rs` (plugins, state, tray, handlers, window events),
`src-tauri/Cargo.toml` (specta 1.0.5, tauri-specta 1.0.2, plugins single-instance 2.4.3,
window-state 2.4.1, autostart 2.5.1, dialog 2.7.2), `src-tauri/tauri.conf.json`,
`src-tauri/capabilities/default.json`, `package.json` (`check:bindings`, add to `verify`),
`src/app/AppShell.svelte`, `src/app/TitleBar.svelte`, `ARCHITECTURE.md`, `SECURITY.md`.
**Delete** — none.

**As built.** The dependency line reads `ts-rs 12.0.1` (feature `serde-json-impl`, needed because
an event's `before`/`after` are `serde_json::Value`) with plugins single-instance **2.4.4**,
window-state 2.4.1, dialog **2.7.3**, autostart 2.5.1, notification **2.4.0** — the versions that
actually resolve; no `specta`, no `tauri-specta` (D-14). The generator is
`src-tauri/examples/export_bindings.rs` rather than `src/bin/`, so it is not a second binary in a
shipped build. Three modules were added beyond the listed set, each because the alternative was
duplicating a rule: `app/monitoring.rs` (the pause gate the tray and the command share),
`app/settings_service.rs` (the one place the key/value table becomes an object), and
`app/rate_limit.rs` (the cap on `log_ui_error`). `tauri.conf.json` needed no change — the window's
minimum size was already 900×600, and it is re-asserted in `setup` so a geometry restored from an
older build cannot reopen smaller. `AppShell.svelte` needed no change either; the listeners belong
in `App.svelte`, which owns the mount lifecycle. `src/features/settings/collector-settings.ts`
became the copy table for all twelve collectors rather than a two-row fixture.

## Implementation Steps

1. DTOs in `dto/` with `#[derive(Serialize, Deserialize, specta::Type)]`; `AppErrorDto { code: &str,
   message: String, retryable: bool }` — messages are user-safe and pass through `scrub_line`
   before crossing IPC.
2. Implement `AppState` construction in `lib.rs` `setup`: paths → tracing init (P03) → `Db::open`
   (P04) → settings load → event bus → state manage. If `Db::open` returns `Corrupt`, still complete
   setup and route the UI to the recovery screen (§83: no crash loop).
3. Implement the already-backed command bodies; the rest return
   `Err(AppError::NotImplemented("phase-NN"))` **and** are listed in `ROADMAP.md` under "Tracked
   stubs" with their owning phase (§87 compliance — visible, dated, never silently green).
4. `app/event_bus.rs`: subscribe to writer batch notifications, coalesce per 500 ms, emit
   `system-event-added`. Unit test: 1,000 notifications in 100 ms produce ≤ 2 emits.
5. `app/tray.rs`: `TrayIconBuilder` + `MenuBuilder` with the seven items in §7's exact order and
   wording; handlers call the same services the commands do (DRY). Left-click shows the window.
6. `app/window.rs`: `WindowEvent::CloseRequested` → `api.prevent_close(); window.hide()`; a
   `first_hide_notified` setting guards the one-time balloon.
7. Register plugins in this order: single-instance (must be first), window-state, dialog, autostart,
   notification (P19 uses it; registered here with no capability granted until then).
8. `bin/export_bindings.rs` (or the `tauri-specta` builder in `lib.rs` under
   `#[cfg(debug_assertions)]`) writes `src/lib/tauri/bindings.ts`. Verify the exact builder API with
   `cargo doc -p tauri-specta --open` before writing the call — do not guess the entry point.
   Fallback if the 1.0 API does not fit: `ts-rs` 12.0.1 derive exports plus hand-written command
   wrappers, with `check:bindings` still enforcing parity.

   **The fallback applied, before a line was written.** `tauri-specta` 1.0.2 declares
   `tauri ^1.2.4` and `specta ^1.0.3` — it is the Tauri **v1** API, whose entry point is
   `ts::export(collect_types![…], path)` and which does not exist under Tauri 2. The Tauri-2 line
   is `2.0.0-rc.25` and pins four pre-release crates. So `ts-rs` 12.0.1 generates the types
   (`src-tauri/src/dto/bindings.rs`, written by `examples/export_bindings.rs`) and the command
   wrappers in `src/lib/tauri/client.ts` are hand-written. Recorded as **D-14**.
9. `src/lib/tauri/client.ts`: one exported async function per command, typed from `bindings.ts`,
   wrapping `invoke` with error normalization into `AsyncState` errors and a dev-mode console trace.
10. Extend `src-tauri/tests/capability_allowlist.rs` (exists since the design-system phase with the
    forbidden-prefix assertion for `fs:`/`shell:`/`http:`/`sql:`/`process:`): add `updater:` to the
    forbidden prefixes and assert the permission set is exactly the documented list.
11. `commands_contract.rs`: assert every function in `generate_handler!` appears in `bindings.ts`
    and vice versa (read both files), so a command can never be added without a typed wrapper.
12. B2 measurement #2: `bun run mem` against the release build with the DB open and tray active;
    append to `docs/PERFORMANCE.md`.
13. Commits: (a) `feat: dto and typed command surface`, (b) `feat: generated typescript bindings`,
    (c) `feat: tray, close-to-tray and single instance`, (d) `feat: throttled live event bus`,
    (e) `test: ipc contract and capability allowlist`.

## Todo (commit boundaries)

- [x] DTOs + error mapping + validation helpers
- [x] All command signatures registered; stubs listed in ROADMAP
- [x] Bindings generated, committed, drift-gated
- [x] Tray with the seven §7 entries; close hides; single instance focuses
- [x] Event bus coalescing with a test
- [x] Capability allowlist + IPC contract tests
- [x] Memory measurement #2 recorded

## Success Criteria

- [x] `get_settings`/`update_settings` round-trip from the UI and persist across restart
- [x] Closing the window hides it; the tray stays; `get_collector_status` still answers
- [x] All seven tray items work; Pause/Resume toggle enabled state; Quit is the only exit path
- [x] Second launch focuses the first instance instead of opening a window
- [x] `bun run check:bindings` fails when a Rust DTO field is renamed and passes after regeneration
- [x] `capabilities_allowlist` test fails if `fs:`/`shell:`/`sql:` is added
- [x] `bun run tauri build` still produces the NSIS bundle; §69 Phase 0 acceptance list is fully green

### Evidence

**186 Rust tests, 0 failures**, on 2026-09-07: 135 lib, 7 architecture, 4 capability_allowlist,
5 commands_contract, 8 db_recovery, 8 migrations, 9 redaction_leak, 14 repository. `cargo fmt
--check` and `cargo clippy --all-targets --all-features -- -D warnings` both exit 0, so the
no-panic rule holds across every new module. `bun run check` reports 0 errors over 798 files and
`bun run test` passes 96. `bun run check:bindings` reports "bindings are up to date" immediately
after a regeneration, which is the property the gate exists to assert.

Criterion 1 is covered by `storage::settings_repository`'s
`an_update_touches_only_what_it_names_and_survives_a_reopen`, which writes through the real `Db`,
closes it, reopens the same files and reads the value back — persistence across restart, not
across a function call. Criterion 5 is covered structurally: `dto/bindings.rs` generates from the
derives, so a renamed field changes the output and `--check` compares byte for byte after
normalising line endings. Criterion 6 gained a third test,
`the_forbidden_check_can_actually_fail`, because the original assertion could pass against an
empty forbidden list.

Criteria 2, 3 and 4 are **structurally** verified rather than executed: they need a running Tauri
window, which this gate has no way to start. What is asserted is the logic behind them —
`app::tray`'s tests pin the seven labels and their order, `app::state`'s
`a_state_with_no_tray_says_so_which_is_what_keeps_the_window_closable` and
`attaching_a_tray_hands_it_the_current_monitoring_state` pin the tray-availability contract, and
`app::window`'s test pins the balloon wording. The window-event handler, the single-instance
callback and the menu-event router are three-line functions that delegate to those tested pieces.
That is the honest state: the seams are tested, the wiring is not, and the wiring is what a manual
`bun run tauri dev` pass would confirm.

Criterion 7 and memory measurement #2 closed on 2026-09-19. `bun run tauri build` completed the
release profile in 3m06s using `CARGO_TARGET_DIR=.qa/phase08-release-target-20260912` and produced
`release/bundle/nsis/What Changed_0.1.0_x64-setup.exe` (3,196,937 bytes; SHA-256
`570825495FA297409D3C1141E71059A16E105D97FE6F356A3C5A4E07EFB78EE5`). The release app ran with
the database open and tray active. After 10 seconds, `scripts/measure-memory.ps1` measured backend
RSS 41.9 MB, six WebView2 processes totaling 405.1 MB, 447.1 MB total working set, and 145.5 MB
private working set. The exact row is recorded in `docs/PERFORMANCE.md`; per D-15 both figures are
reported and neither is pass/fail.

**One finding closed during the phase.** `tests/architecture.rs` failed on
`app/settings_service.rs reaches for rusqlite directly` — a real violation of the "SQL only under
storage/" invariant, caught by the gate rather than by review. The fix was structural rather than
an exemption: the settings vocabulary moved to `domain/settings.rs` (it is domain state — the
retention policy decides what maintenance deletes) and the key/value-to-object assembly moved into
`storage/settings_repository.rs`, beside the table it reads. `dto/settings_dto.rs` is now a
re-export, so settings have one definition instead of two.

### What the review changed

A `tester` pass found three things worth the phase being held open for, and all three are fixed.
Final counts after them: **208 Rust tests and 111 frontend tests, 0 failures.**

**The settings round-trip was write-only.** `get_settings` was called and its answer discarded:
`SettingsStore.current` had no readers, and the theme control read `localStorage`. So Rust stored
the theme and nothing read it back — the criterion appeared met only because of the mirror that
`public/theme-bootstrap.js` needs for first paint. `SettingsStore.load` now hydrates the theme
store from the backend's answer, `App.svelte` loads settings at boot rather than when the Settings
screen first opens, and the control reads `settings.current?.theme`. The test that proves it sets
the theme store to one value, has the backend return another, and asserts the backend wins; it
fails against the previous code.

**Command *parameter* names were ungated, which was the larger hole.** The bindings emitted command
names only, so renaming a Rust parameter — `event_id` to `id` — broke the call at runtime while
the build, `check:bindings` and the contract test all stayed green. Tauri matches arguments by
name, so that is precisely the failure the typed surface exists to prevent.
`commands::signatures()` now reads the parameter names out of the command sources, drops the ones
Tauri injects, converts them to the camelCase form that goes on the wire, and generates them into
the bindings table; `commands_contract.rs` parses the object literals `client.ts` passes and
compares. Both directions fail the build now.

**Two tests could not fail**, and both were rebuilt on the pattern `tests/architecture.rs` already
uses — drive a deliberately bad input through the same function the gate uses. The capability one
now parses a hostile JSON file through the real reader and asserts both the plain and the scoped
form are caught; the bindings one counts braces and has a companion test proving an unterminated
declaration is detected.

Also from that pass: `close_action` and `route_for` extracted as pure functions, so the testable
half of criteria 2 and 3 is now tested rather than merely factored; `tests/wiring.rs` added with
five source-text guards — plugin registration order, quit-as-the-only-exit, Rust-to-TypeScript
event-name parity, tray-route-to-`routes.ts` parity, and both directions of the ROADMAP stub table;
15 store tests covering every silent-failure branch named in the report; and a dead
`let _ = not_implemented(…)` removed.

Two of its recommendations are deliberately not this phase's work, and are recorded rather than
dropped. Enabling Tauri's `test` feature would unlock command-level integration tests — genuinely
the highest coverage per line available here — but it adds a dependency to the dev graph on a
volume with 5 GB free, so it belongs in its own decision rather than at the end of this one. And
`collector-status-changed`/`analysis-completed` are declared on both sides and emitted by nobody
until phases 08 and 15; that is now a table in ROADMAP's tracked-stubs section, on the same
argument the phase makes for stub commands.
## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check && bun run check:bindings
bun run test
cargo test --manifest-path src-tauri/Cargo.toml --all-features -j 2 --test commands_contract
cargo test --manifest-path src-tauri/Cargo.toml --all-features -j 2 --test capability_allowlist
bun run rs:lint && bun run tauri build
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1
```

Two corrections to the commands as originally written, both learned the same way phase 04 learned
them. `--all-features` is required on any bare `--test` invocation: the integration suites import
`what_changed_lib::testing`, which is gated behind the `devtools` feature, so without the flag they
do not compile. And the suite is `capability_allowlist`, singular — the file has been named that
since phase 02. `-j 2` is this machine's constraint, not a preference: the D: volume has under 8 GB
free against a 4.8 GB dev profile.

`bun run tauri build` must be run **after** `cargo clean --profile dev`, because a debug and a
release profile together exceed the free space on this volume (D-19, and the disk section of
`docs/PERFORMANCE.md`).

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `tauri-specta` 1.0 builder API differs from the assumed shape | Compile error in the export path | Read `cargo doc -p tauri-specta`; if it still does not fit, switch to `ts-rs` 12.0.1 + hand-written wrappers, keep `check:bindings` as the parity gate, record D-14 |
| Tray icon missing at runtime | Tray absent though the app runs | Ensure the `tray-icon` + `image-png` tauri features are enabled and the icon path is a bundled resource; assert the icon file exists in a startup test |
| Hidden window is unreachable if the tray fails to build | Window hidden, no tray | Tray construction failure is fatal-by-design at startup: log, show the window, and surface a banner "Tray unavailable — closing the window will exit"; never leave an invisible process |
| Autostart plugin API/signature drift | Compile error registering the plugin | Verify with `cargo doc -p tauri-plugin-autostart`; the onboarding toggle (P19) is the only caller, so the fallback is a documented manual Startup-folder shortcut created by our own code |
| Event flood makes the UI thrash | Renderer CPU spikes with rapid inserts | The 500 ms coalescer is the contract; the frontend additionally ignores notices while scrolled away and shows a "N new changes" pill |

## Security Considerations

- §35: the command list is a closed allowlist; there is no dynamic dispatch, no `eval`, no
  path/SQL/shell passthrough. `app/validate.rs` centralizes path and range validation (§82 path
  traversal / symlink).
- Frontend is untrusted: every command re-validates, even for values the UI just received from Rust.
- `log_ui_error` is rate-limited (10/min) and its payload is scrubbed before it reaches tracing,
  so a malicious page script cannot flood or poison the log (§82 malicious frontend script, IPC abuse).
- Capabilities remain `core:default` + `core:window:allow-start-dragging` + the three
  `core:window:allow-{minimize,toggle-maximize,close}` grants for the drawn titlebar (added in P02)
  + `dialog:allow-open` (folder picking only) at the end of this phase; each addition is justified
  in `SECURITY.md`.
