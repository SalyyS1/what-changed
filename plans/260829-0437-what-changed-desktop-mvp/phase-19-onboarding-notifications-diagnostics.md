---
phase: 19
title: "First-run onboarding, restrained notifications, privacy screen, diagnostics"
status: in-progress
priority: P1
effort: 6h
dependencies: [15, 18]
---

## Overview

The trust layer (§42, §50, §53, §58, §33, plus autostart). Onboarding earns the right to watch;
the notification policy earns the right to interrupt; the privacy screen and delete-all make the
"stays on this PC" promise inspectable; diagnostics make a user report actionable without asking for
their database. This lands after the analysis engine so onboarding can honestly describe what the app
will tell them, and so notifications can use real importance and confidence rather than placeholders.

## Requirements

**Functional**

*Onboarding (§42)* — three steps, skippable at any point, shown only when
`settings.onboarding_completed == false`, and re-openable from Settings ("Show the welcome tour again").
1. **What this does** — headline "It worked before. What changed?", one paragraph, and the §72 promise
   line "Your history stays on this PC." No signup, no email field, no telemetry consent because there is
   no telemetry (the screen says so).
2. **What we watch** — the collector list rendered from the same `CollectorDescriptor` data as Settings
   (§49, DRY), each with its "what it watches / why it matters" text and a toggle; everything on by
   default except Docker (off when the pipe is absent) and the watched-folder collector (nothing to watch
   yet). Copy line: "We record that a value changed, never the value itself."
3. **Watch your project folders** — folder picker (P10 command) with 0-or-more selection, an explanation
   that config changes are the highest-value signal, and a "Start with Windows" toggle. Finishing writes
   `onboarding_completed = true`, creates the first automatic checkpoint (P14) so day-one comparisons work,
   and lands on Home.
- A first-run empty Home never looks broken: until events exist, Home shows "We just started watching.
  Come back after something changes." (a §70 state, wired in P07 and asserted here with a fresh DB).

*Autostart*
- `tauri-plugin-autostart` registered with `MacosLauncher`-irrelevant Windows config; **verify the exact
  builder signature and the enable/disable API against `cargo doc -p tauri-plugin-autostart` before
  writing the call** — if the plugin's current API differs from this plan, use the documented one; if the
  plugin cannot be pinned to a Tauri-2-compatible version, fall back to writing a single
  `HKCU\...\CurrentVersion\Run` value through the P11 registry provider (write access to the user's own
  Run key only) and record the substitution as a decision in the phase file.
- Autostart launches with `--minimized`: the window is not shown, only the tray icon, and the first
  window show is lazy. A `settings.start_minimized` toggle exists and defaults to true when autostart is
  enabled.

*Notifications (§53)*
- Policy, implemented in `notify/policy.rs` as a pure function
  `fn should_notify(event, ctx) -> Option<NotificationCopy>`: notify **only** for
  `importance: critical`, `driver/updated`, `network/proxy_changed`, `network/dns_changed`,
  `update/failed`, `service_failed` on a service the user has an entity relationship with, and
  `docker/container_died` with a non-zero exit. Everything else never notifies.
- Rate limits: max 3 notifications per hour and 8 per day; a 30-minute per-category cooldown; nothing
  during the first 60 s after launch (avoids a startup burst); nothing while monitoring is paused; and a
  "quiet hours" setting (default off, 22:00–08:00 when on).
- Copy shape: title = the event's `user_friendly_summary`; body = the §54 "why you might care" line plus,
  when P15's `analyze_window` already links it to a failure, "This may be related to a problem detected
  just now." Clicking a notification focuses the window and opens that event's details drawer.
- Delivery via `tauri-plugin-notification` (Windows toast). If the permission/registration fails, the
  in-app "recent important changes" list is the only surface and one DEBUG line is logged — never a
  modal, never a retry loop.
- `tests/notification_policy.rs` replays a 24-hour synthetic stream of 4,000 events (Chrome updates,
  Spotify restarts, background tasks, 2 driver updates, 1 proxy change) and asserts: exactly 3
  notifications, all from the allowed set, never two of the same category within 30 min, and **zero**
  notifications from the 3,900 routine events. This is the mechanical form of "calm by default".

*Privacy screen (§33, §49)*
- "What's stored" panel with live counts from one command (`get_privacy_summary`): events by category,
  checkpoints + items, investigations, watched folders, config baselines, DB file size, oldest record
  date, log file size and path.
- Plain statements: no account, no network calls, no telemetry, no crash reporting; the list of
  categories recorded; the redaction promise with two concrete examples ("`DATABASE_URL` changed in
  `.env`" and "`AWS_SECRET_ACCESS_KEY` changed", both showing `[REDACTED]` values).
- Controls: retention sliders (events 7/30/90 days, default 30), "Delete everything now" with a typed
  confirmation ("delete") that replaces the history database with a freshly migrated database,
  restores only an explicit allowlist of non-history preferences, clears logs, and shows "Your history
  has been deleted. We're watching again from now.", plus per-category "Delete this category's history"
  and "Stop watching and delete <folder>'s history". Decide and test the settings allowlist explicitly:
  appearance/accessibility preferences may survive; watched folders/projects, collector baselines,
  onboarding/checkpoint state and anything that reveals observed history do not survive unless the
  privacy contract names them. `db_recover(Repair|ResetHistory)` is accepted only while the database is
  `Corrupt` or
  `SchemaMismatch`; healthy-history deletion uses the dedicated destructive command and a backend-
  validated one-use random confirmation challenge. The challenge prevents
  guessing/replay, while the native command surface cannot prove that a caller
  displayed the visible confirmation dialog; that IPC trust-boundary limitation
  remains documented.
  Scoped purges rebuild FTS5, checkpoint/truncate WAL, run `VACUUM`, and byte-scan the main DB/WAL/SHM
  plus FTS shadow tables for a unique fixture. Full purge closes connections and atomically replaces
  the database rather than trusting ordinary deletes to erase FTS5 forensic remnants; the old artifact
  is removed only after the replacement passes integrity/migration checks. The UI and privacy docs state
  the limits of best-effort secure erasure rather than promise cryptographic deletion on unencrypted
  storage.
- A "Verify for yourself" affordance: a button that opens the data folder in Explorer (via the
  `opener` plugin scoped to exactly the app-data directory — the only shell-adjacent capability in the
  app, justified because it is what makes the privacy claim checkable) and shows the DB path as
  selectable text.

*Diagnostics (§58, §33)*
- Diagnostics screen: app + OS version, DB state and size, migration version, per-collector status table
  (state, cadence tier, last run, last duration, events emitted, consecutive failures, chosen wake tier
  from P12/P13), write-queue depth, dropped-event counters, and the B2 memory reading (current working
  set + peak) taken from `GetProcessMemoryInfo`.
- Degraded-mode banner (§33): when any collector is `Failed` or `Unavailable`, a dismissible banner reads
  "Some things aren't being watched right now." with a link to Diagnostics. Never a modal, never a
  blocking error, and the rest of the app keeps working.
- `export_diagnostics()` writes a single `.zip`-free plain text file (no archive dependency) to a
  user-chosen path containing: versions, settings (redacted), collector status, the last 500 log lines
  (already redaction-filtered by P03's writer), counts by event type, and the last 20 error entries.
  It contains **no event content, no paths, no entity names** — the header states this and a test asserts
  it (grep the export for a fixture entity name and a fixture secret; both must be absent).
- §26 health checks stay deferred (the user's own deferral): the `health_states` table remains unused and
  `ROADMAP.md` carries the entry with its owning future phase.

**Non-functional**
- Onboarding is reachable and completable with the keyboard only; every step passes the P02 contrast test.
- `get_privacy_summary` ≤ 60 ms at 100k events (count queries hit indexes; DB size from file metadata).
- `export_diagnostics` ≤ 300 ms and never larger than 512 KB.
- Notification evaluation adds ≤ 1 ms per event and runs off the write path (it consumes the same
  coalesced batch the frontend does).

## Architecture

`src-tauri/src/notify/{mod.rs,policy.rs,copy.rs,rate_limit.rs}` — pure policy + a thin
`tauri-plugin-notification` adapter, so the policy is unit-testable with no OS involvement.
`src-tauri/src/commands/{privacy.rs,diagnostics.rs,onboarding.rs}`;
`src-tauri/src/storage/purge.rs` (per-category and full deletion, reusing the P04 retention machinery);
`src-tauri/src/platform/windows/memory.rs` (`GetProcessMemoryInfo`).
Frontend: `features/onboarding/{OnboardingFlow.svelte,StepIntro.svelte,StepCollectors.svelte,
StepFolders.svelte}`, `features/privacy/{PrivacyScreen.svelte,StoredDataPanel.svelte,
RetentionControls.svelte,DeleteEverythingDialog.svelte}`,
`features/diagnostics/{DiagnosticsScreen.svelte,CollectorStatusTable.svelte,ExportDiagnostics.svelte}`,
`components/feedback/{DegradedBanner.svelte,Toast.svelte}` (Toast created in P02 if absent),
`lib/stores/{onboarding.svelte.ts,privacy.svelte.ts,diagnostics.svelte.ts}`.

## Related Code Files

**Create** — `src-tauri/src/notify/{mod.rs,policy.rs,copy.rs,rate_limit.rs}`;
`src-tauri/src/commands/{privacy.rs,diagnostics.rs,onboarding.rs}`;
`src-tauri/src/storage/purge.rs`; `src-tauri/src/platform/windows/memory.rs`;
`src-tauri/tests/notification_policy.rs`, `src-tauri/tests/privacy_purge.rs`,
`src-tauri/tests/diagnostics_export.rs`, `src-tauri/tests/onboarding_state.rs`;
the frontend files listed above; `src/features/onboarding/copy.ts`.
**Modify** — `src-tauri/src/lib.rs` (register notification + autostart + opener plugins; consume the
event batch for notification evaluation), `src-tauri/src/commands/mod.rs` + `dto/bindings.rs`
(`get_privacy_summary`, `purge_category`, `purge_all`, `set_retention`, `get_diagnostics`,
`export_diagnostics`, `complete_onboarding`, `set_autostart`), `src-tauri/capabilities/default.json`
(`notification:default`, `opener:allow-reveal-item-in-dir` scoped to the app data dir — and update
`tests/capabilities_allowlist.rs` to allow exactly these two additions and nothing else),
`src-tauri/src/domain/settings.rs`, `src/app/AppShell.svelte` (onboarding gate + degraded banner),
`src/features/settings/SettingsScreen.svelte` (autostart, quiet hours, re-run tour),
`ROADMAP.md` (§26 health checks), `docs/deployment-guide.md`, `PRODUCT.md`.
**Delete** — none.

## Implementation Steps

1. `notify/policy.rs` + `rate_limit.rs` + the 24-hour replay test first — the policy is the interruption
   contract and is cheapest to get right in isolation.
2. Notification adapter + click routing (window focus + drawer open by event id).
3. `storage/purge.rs`: per-category, per-folder, and full purge in transactions; full purge recreates the
   schema at the current migration version and runs `VACUUM`.
4. `commands/privacy.rs` + the privacy screen with the typed confirmation dialog.
5. `platform/windows/memory.rs` + `commands/diagnostics.rs` + the diagnostics screen and the degraded
   banner; export writer with the "no event content" header and its test.
6. Onboarding flow with the exact copy in `copy.ts`, keyboard navigation, and the completion side effects
   (settings write, first automatic checkpoint).
7. Autostart: verify the plugin API, wire the toggle, add `--minimized` handling in `lib.rs`'s setup
   (window created hidden when the flag is present), and confirm the tray-only start path.
8. B2 measurement #3: record working set at idle after 10 minutes with all collectors running, with and
   without the window shown, into the phase file. If < 80 MB is unreachable, do **not** redefine the
   target — surface it as a scope question with the three numbers (headline: WebView2 baseline dominates).
9. Tests: policy replay, purge (counts before/after, settings preserved, checkpoints removed by full
   purge but not by category purge), diagnostics export content assertions, onboarding state machine
   (fresh DB → flow shown; completed → skipped; re-run works).
10. Commits: (a) `feat: notification policy and rate limits`, (b) `feat: toast delivery and click routing`,
    (c) `feat: purge operations`, (d) `feat: privacy screen`, (e) `feat: diagnostics screen and export`,
    (f) `feat: degraded mode banner`, (g) `feat: onboarding flow`, (h) `feat: autostart and minimized start`,
    (i) `test: policy, purge, export and onboarding suites`.

## Todo (commit boundaries)

- [x] Notification policy + rate limits + 24-hour replay test
- [x] Toast delivery and click-to-event routing
- [x] Purge operations (category, folder, all)
- [x] Privacy screen with live counts and verify-for-yourself
- [x] Diagnostics screen, export, degraded banner
- [x] Onboarding flow with first checkpoint
- [x] Autostart + minimized start
- [ ] B2 measurement #3 recorded

## Success Criteria

- [x] `notification_policy.rs`: 4,000-event day → exactly 3 notifications, none from the 3,900 routine
      events, no two of one category within 30 min, none in the first 60 s, none while paused
- [ ] Clicking a notification focuses the window and opens the correct event's drawer (manual, recorded)
- [x] `purge_all` preserves exactly the reviewed non-history preference allowlist, empties every history
      table including checkpoints/investigations/baselines/watched roots, and the UI shows the exact
      §33/§49 confirmation copy
- [x] `purge_category("network")` removes only network events and leaves others' counts unchanged
- [x] `diagnostics_export.rs`: the export contains no fixture entity name, no fixture path, and no
      fixture secret; size ≤ 512 KB
- [x] `get_privacy_summary` ≤ 60 ms at 100k events
- [x] Fresh DB → onboarding shows; completing writes the completion flag, creates one automatic checkpoint, and
      Home shows "We just started watching."
- [x] Onboarding completable with keyboard only; contrast test passes on all three steps
- [ ] Autostart enabled → the registry/plugin state is set and a `--minimized` launch shows only the tray
      (manual, recorded)
- [x] Killing one collector shows the degraded banner and Diagnostics names the failed collector
- [ ] B2 numbers (idle working set, window shown / window hidden, peak) recorded in the phase file

## Verification evidence — 2026-09-22

`notification_policy` 4/4, `privacy_purge` 2/2, `onboarding_state` 1/1 and
`diagnostics_export` 1/1 passed. Frontend automated coverage exercises the
degraded diagnostics state and keyboard/contrast contracts. Backend delivery
emits the event id and the UI routes it to the timeline drawer; the native
Windows toast-click callback is not exposed reliably by the installed plugin,
so that manual row remains open. Byte-level FTS/WAL/SHM scans, the 100k
privacy-summary timing, fresh installed-app onboarding, autostart minimized
launch and D-15 B2 readings remain open. The backend notification event is a
delivery signal; the native toast click callback is not exposed by the installed
plugin, so automatic drawer routing remains open.

## Verification evidence — 2026-09-23 continuation

- `get_quiet_hours` now reads the persisted notification policy with a disabled
  default for older settings rows; the backend round-trip test passes.
- Diagnostics now reports the live admitted writer-command depth via the same
  bounded queue counter used by the writer, instead of a hardcoded zero.
- The exact synchronous body used by `get_privacy_summary` is exercised against
  100,000 persisted events; the isolated rerun measured p95 **28.1072 ms** against the ≤60 ms gate
  (21 samples; the test passed).
- The diagnostics export now renders through a pure backend report builder shared by the command;
  its unit gate injects fixture entity/path/secret log content and passes with all three absent and
  report size ≤512 KB.
- Fresh-DB onboarding completion is exercised in the backend with the real `AppState`: the flag is
  persisted, exactly one automatic `First day` checkpoint is created, repeated completion is idempotent,
  and the Home empty state test passes 5/5 with the canonical "We just started watching" copy.
- Full-purge byte verification now seeds a unique fixture, replaces the database, and scans the main
  DB plus WAL/SHM companions; the marker is absent from every existing SQLite artifact and the test passes.
- Settings loads the stored quiet-hours window on mount and uses the generated
  binding/client command for updates. `bun run check:bindings`, Settings screen
  tests (2/2), frontend typecheck and targeted onboarding, diagnostics,
  notification and privacy suites pass.
- This closes the persisted-settings/UI mismatch but does not close native
  toast-click/drawer routing, installed onboarding, minimized autostart or D-15
  manual rows.

## Verification evidence — 2026-10-14 current-source memory snapshot

- A current-source release binary (`cargo build --release --locked --no-default-features`) was launched
  under the current Windows account and measured with `scripts/measure-memory.ps1` after a 5-second
  settle. Window shown: backend working set **48.8 MB**, WebView2 descendants **366.1 MB** across six
  processes, total working set **414.9 MB**, private working set **132.7 MB**. This is one engineering
  checkpoint only; hidden-window and after-four-hour readings remain open, and D-15 means no pass/fail
  verdict is attached to either memory figure.
- A second engineering snapshot with the window hidden measured backend working set **50.7 MB**, six
  WebView2 descendants **359.8 MB**, total working set **410.5 MB**, and private working set **128.9 MB**.
  The four-hour checkpoint and installed tray/autostart walkthrough remain open.
- A current-source `what-changed.exe --minimized` smoke stayed alive and responsive for 10 seconds with
  a native window handle. This confirms argument acceptance only; it does not prove the autostart registry,
  tray-only visibility or installed-app behavior, which remain manual rows.

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test notification_policy --test privacy_purge --test diagnostics_export \
           --test onboarding_state --test capabilities_allowlist
cargo clippy --all-targets -- -D warnings
cd /d/Project/.WhatChage && bun run check && bun run check:bindings && bun run test
bun run tauri dev   # manual: fresh-DB onboarding, notification click, --minimized start, banner
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Notification fatigue destroys trust (§53) | More than 3/hour in the replay test, or a routine event notifying | The policy is an allowlist, not a filter — a new event type notifies only if it is added explicitly, and the replay test is a build gate |
| `tauri-plugin-autostart` API differs from the plan | `cargo doc` shows a different builder/API | Use the documented API; if no Tauri-2-compatible version pins, write the user's own `Run` key via the P11 provider and record the decision. Never a scheduled task (that is the elevation-flavoured path) |
| B2 (<80 MB) unreachable | Measurement #3 exceeds 80 MB | Report the three measurements and the WebView2 share as a scope question to the user. Do not silently redefine the target, and do not ship a "memory saver" hack that breaks the tray/lifecycle |
| Full/scoped purge leaves recoverable FTS or WAL content | A unique fixture remains in FTS shadow tables, main DB, `-wal` or `-shm` | Scoped purge rebuilds FTS then checkpoints/VACUUMs; full purge closes all handles and atomically replaces the database. Byte-scan all artifacts and document that unencrypted-storage erasure is best effort, not cryptographic |
| Database replacement loses preferences or resurrects history | restored rows are broader/narrower than the reviewed allowlist | copy the allowlist into memory only after the confirmation challenge, migrate a fresh DB, restore those keys transactionally, verify, atomically swap; keep the old DB until verification, then remove it |
| Diagnostics export leaks history | The export test finds an entity name | The export is built from an explicit field allowlist (counts and versions only), not by serializing a struct; the test is the gate |
| Adding the `opener` plugin widens the attack surface | `capabilities_allowlist` test fails | Scoped to `reveal-item-in-dir` for the app data dir only; the allowlist test is updated to permit exactly that one permission and still reject `fs:`, `shell:`, `http:`, `process:` |
| Onboarding blocks a user who wants to look first | Support-style complaint / dead end | Every step is skippable, nothing is required, and the tour is re-runnable from Settings |

## Security Considerations

- Two new capabilities only: `notification:default` and a path-scoped `opener` reveal. The allowlist test
  is tightened to name them explicitly so a future broad `fs:`/`shell:` addition fails the build.
- `export_diagnostics` is an allowlist-built text file with no event content; log lines inside it already
  went through the P03 redacting writer, so it is doubly filtered.
- `purge_all` requires both the typed UI phrase and a short-lived, single-use random backend challenge;
  the destructive command refuses absent, expired, reused or mismatched challenges. The challenge is not
  bound to a renderer session, so a direct IPC caller that first requests a challenge can still invoke the
  command; the UI confirmation remains a user-flow control rather than a cryptographic boundary.
  Recovery reset is separately state-gated to an unusable DB.
  The dialog says the operation is irreversible and the Rust test calls IPC semantics directly, so the
  guarantee does not depend on an honest frontend.
- Notification bodies contain `user_friendly_summary` text, which is sanitized; a toast therefore cannot
  surface a secret on a lock screen.
- Autostart writes only the current user's own `Run` entry (or the plugin equivalent); no service, no
  scheduled task, no elevation.

## Verification evidence — 2026-09-24 targeted P19 run

- `CARGO_BUILD_JOBS=1 cargo test --locked --test notification_policy --test privacy_purge --test diagnostics_export --test onboarding_state --test capability_allowlist` passed: notification 4/4, privacy purge 4/4 (including 100k summary p95 gate), diagnostics export 1/1, onboarding 1/1, capability allowlist 5/5.
- Frontend diagnostics plus accessibility contracts passed with `bun run test -- src/features/diagnostics/DiagnosticsScreen.test.ts src/lib/a11y/contrast.test.ts src/lib/a11y/reduced-motion.test.ts src/lib/a11y/focus-order.test.ts`: 12/12 tests across 4 files.
- Remaining P19 rows are installed-app/manual gates: native toast click-to-drawer, fresh installed onboarding, minimized autostart, and B2 readings. They remain intentionally open.
