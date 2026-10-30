# P05 Typed IPC, Tray & Lifecycle — test-coverage verification

**Mode:** static analysis only. Bash tool dead in this session too (`echo probe`, `pwd && git status`
both returned empty). No cargo/bun run. Gate results below are the lead's, not re-run.

**Gate (lead-supplied, green):** `cargo check`/`fmt`/`clippy` 0; `cargo test --all-features` 186 passed;
`bun run check` 0 errors; `bun run test` 96 passed; `bun run check:bindings` "up to date".

---

## Success Criteria verdicts

| # | Criterion | Verdict |
|---|---|---|
| 1 | `get_settings`/`update_settings` round-trip from UI, persist across restart | **PARTIAL** |
| 2 | Close hides; tray stays; `get_collector_status` still answers | **PARTIAL** |
| 3 | Seven tray items work; Pause/Resume toggle enabled; Quit only exit | **PARTIAL** |
| 4 | Second launch focuses first instance | **UNMET (no test)** |
| 5 | `check:bindings` fails on renamed DTO field, passes after regen | **PARTIAL** |
| 6 | `capability_allowlist` fails if `fs:`/`shell:`/`sql:` added | **MET** |
| 7 | `bun run tauri build` produces NSIS bundle | **UNVERIFIED** (not run — disk) |

### 1 — settings round-trip — PARTIAL

Proven:
- Persistence across DB reopen: `src-tauri/src/storage/settings_repository.rs:236`
  (`an_update_touches_only_what_it_names_and_survives_a_reopen`) — closes and reopens the file, reads
  `Theme::Light` back. Real.
- Partial-update semantics, unknown-collector rejection, retention bounds, garbled-row fallback:
  `settings_repository.rs:265,304,350`.

Not proven, and one is a live defect:

- **The loaded settings are never rendered.** `SettingsStore.current`
  (`src/lib/stores/settings.svelte.ts:22`) has **zero readers** — grep across `src/**` finds only the
  declaration. `SettingsScreen.svelte:41` calls `settings.load()` and discards the result; the theme
  control at `SettingsScreen.svelte:103` binds `theme.mode`, which
  `src/lib/design/theme.svelte.ts:40` reads from `localStorage`. So the round-trip is write-only:
  Rust stores the theme, nothing reads it back. "Persists across restart" currently holds **because of
  localStorage**, not because of `get_settings`. Clear `wc.theme` and the Rust-stored theme is
  ignored. No test would catch this.
- No test calls `get_settings`/`update_settings` as commands. `src-tauri/src/commands/settings.rs:17,23`
  have no test module; no integration test builds an `AppState` and invokes them.
- No store test at all (see Frontend gaps).

Missing test: a `SettingsStore` unit test that stubs `getSettings`/`updateSettings` and asserts
`state` transitions loading→ready, that `save` replaces state with the **backend's** answer, and that a
failed save keeps the previous ready state (`settings.svelte.ts:48` branch is untested). Plus a
component test asserting the Settings screen renders `settings.current.theme`, which would fail today
and should.

### 2 — close hides, tray stays, status answers — PARTIAL

Proven:
- `AppState::tray_available()` false when no tray, and reported through `status()`:
  `src-tauri/src/app/state.rs:266` (`a_state_with_no_tray_says_so_which_is_what_keeps_the_window_closable`).
- Attaching a tray hands it current monitoring state, and `reflect_monitoring` dispatches to it:
  `state.rs:278` via a `TrayReflector` spy. This is the one Tauri-adjacent behaviour with a real seam.

Not proven:
- `on_window_event` (`src-tauri/src/app/window.rs:52`) is untested. Requires `tauri::Window`.
- **The seam is missing.** The decision — hide vs let-close-through — is inline at `window.rs:58-66`.
  A pure `fn close_action(tray_available: bool) -> CloseAction` (or `should_hide(bool) -> bool`) would
  be testable and is not present. Contrast `AppState::tray_available()`, which *was* extracted and *is*
  tested. The predicate is factored out; the branch on it is not.
- One-time balloon: `notify_first_hide` (`window.rs:91`) untested. The flag itself round-trips
  (`settings_repository.rs:342`), but the read→write→show ordering (write first, skip balloon if the
  write fails) has no test. Private fn over `AppHandle`; no seam.
- `get_collector_status` answering while hidden: nothing calls the command in a test. It reads settings
  and maps rows (`src-tauri/src/commands/collectors.rs:19,30`) — the `row()` mapper (enabled →
  `Unavailable`, disabled → `Disabled`, `requires_admin: false` always) is pure and trivially testable
  yet **has no test**. `requires_admin: false` is a product promise (`collectors.rs:46`) with nothing
  pinning it.

Genuinely untestable without a running app: the actual hide, and that monitoring keeps running after it.
Everything else above is testable as written or one extraction away.

### 3 — seven tray items — PARTIAL

Proven:
- Exact seven labels in product order: `src-tauri/src/app/tray.rs:189`. Real pinning test — a reworded
  or reordered `LABELS` fails it.
- Both Pause and Resume present, unknown id → `None`: `tray.rs:206`. Ids distinct: `tray.rs:213`.

Not proven:
- `PauseResume::reflect` (`tray.rs:54`) — the actual `set_enabled(!paused)`/`set_enabled(paused)` pair —
  is untested. Only the *dispatch* to a reflector is (`state.rs:278`). A swap of the two calls would
  pass every test and ship a menu that enables the wrong item.
- `on_menu_event` (`tray.rs:148`) untested; no seam. The route table is inline at `tray.rs:157-162`.
  Extract `fn route_for(id: &str) -> Option<&'static str>` beside the existing `label_for`
  (`tray.rs:176`) and it becomes a three-line test.
- **Cross-language route drift is ungated.** `tray.rs` emits `"timeline"`, `"settings"`,
  `"checkpoints"`; `src/app/routes.ts:10` owns `RouteId`. `App.svelte:29` does
  `routeById(route as RouteId)` — the cast erases the mismatch, and `routeById` returns `undefined`
  silently. Rename a route id and three tray items become no-ops with no failing test. A
  `commands_contract.rs`-style file-reading test could assert the three literals appear in `routes.ts`.
- "Quit is the only exit path": nothing asserts it. `lifecycle::quit` (`lifecycle.rs:40`) untested.
  Current state is clean — `app.exit`/`process::exit` appear only at `lifecycle.rs:44` and the two
  startup-failure arms `lib.rs:110,142` — but an added exit path would be caught by nobody. Cheap
  architecture-test candidate (`architecture.rs` already greps sources for forbidden strings).
- Idempotent clean shutdown *is* tested: `state.rs:304`, and it is a good test — it proves the second
  call is a no-op by observing that an intervening `arm()` still counts.

### 4 — single instance focuses — UNMET

Callback registered `lib.rs:120` → `app::window::show_main`. `show_main` (`window.rs:35`) untested;
unminimize→show→focus ordering is a documented requirement with no assertion.

Truly untestable without two processes. But the *registration order* is testable and is not tested:
the plugin must be first (`lib.rs:120` comment, phase step 7). `lib.rs:218` already proves the file can
be read as source text with `include_str!` — the same trick would pin
`single_instance` before `window_state`/`dialog`/`autostart`/`notification`. Missing.

### 5 — bindings drift gate — PARTIAL

Passing direction verified by the lead's run ("bindings are up to date").

Failing direction is unproven and, more importantly, **incomplete in scope**:

- No test injects drift. `examples/export_bindings.rs:47` compares `generate_full()` to the committed
  file byte-for-byte after CRLF normalisation, so a renamed `ts_rs::TS` field *will* differ — but that
  is reasoning, not a test. `dto/bindings.rs:202-283` tests only assert properties of the generated
  text, never that a mismatch is detected.
- **Command parameter names and arity are not covered by any gate.** `COMMAND_SECTION`
  (`dto/bindings.rs:176`) emits names only. `commands_contract.rs` matches on `call('name'`. So
  renaming a Rust command parameter — e.g. `event_id` → `id` at `commands/events.rs:46` — breaks
  `client.ts:130`'s `{ eventId }` at runtime with a green build, green bindings check and green
  contract test. This is the largest hole in the typed-surface promise.
- `commands_contract.rs:126` (`invoke_is_reached_only_through_the_tauri_directory`) checks for
  `@tauri-apps/api/core` and `/event` only. A screen importing `@tauri-apps/api/window` directly
  passes — and that is exactly what `src/lib/tauri/window-controls.ts:15` imports, so the module family
  is in use and unguarded.
- Two commands violate the stated contract "all returning `Result<T, AppErrorDto>`":
  `get_app_version` returns `String` (`commands/meta.rs:22`) and `get_app_status` returns
  `AppStatusDto` (`meta.rs:28`). Nothing asserts the return shape.

### 6 — capability allowlist — MET

`src-tauri/tests/capability_allowlist.rs:78` iterates the real files in `capabilities/` and rejects any
`fs:`/`shell:`/`http:`/`sql:`/`process:`/`updater:` prefix; `:90` asserts the granted set is *exactly*
the six documented entries, so a swap keeps the count and still fails. Adding `fs:allow-read` to
`capabilities/default.json:6` fails both. `:126` additionally pins that `notification:` is not granted
yet though the plugin is registered (`lib.rs:126`). Genuinely load-bearing.

Caveat — see Trivially-true tests below re `:107`.

### 7 — NSIS bundle — UNVERIFIED

Not run (disk; requires `cargo clean --profile dev` first per D-19). `tauri.conf.json:36` targets
`["nsis"]`, `installMode: currentUser`. Nothing changed in the bundle config this phase, so risk is low,
but the criterion is unproven.

---

## Todo (commit boundaries)

| Item | Verdict | Evidence |
|---|---|---|
| DTOs + error mapping + validation helpers | **MET** | `dto/error_dto.rs:102,114,125` (scrubbing, retryability, closed vocab); `app/validate.rs:248-348` — 11 tests incl. the junction/forbidden-root/no-echo cases |
| All signatures registered; stubs in ROADMAP | **PARTIAL** | `lib.rs:219` proves macro == `COMMANDS`; `commands/mod.rs:89,101` pin count 34 and no dupes. **But nothing asserts the ROADMAP table lists every `not_implemented` command.** Counted by hand: 20 stubs in code, 20 rows at `ROADMAP.md:68-87` — matches today, unenforced tomorrow |
| Bindings generated, committed, drift-gated | **PARTIAL** | see criterion 5 |
| Tray seven entries; close hides; single instance | **PARTIAL** | see criteria 2–4 |
| Event bus coalescing with a test | **MET** | `app/event_bus.rs:181` — 1,000 notices → ≤2 emits, asserts the sender was fast, asserts nothing swallowed, asserts count sums to 1,000 and newest_id == 999. `:216` pins merge semantics incl. that category order is the enum's not the alphabet's. `:249` covers the single-notice case. Best-tested unit in the phase |
| Capability allowlist + IPC contract tests | **MET** | `capability_allowlist.rs`, `commands_contract.rs` |
| Memory measurement #2 recorded | **UNMET** | `docs/PERFORMANCE.md:53-56` still holds only the two P01 rows. `docs/PERFORMANCE.md:85` explicitly commits to re-measuring at P05. Not done |

---

## Tests that cannot fail

1. `src-tauri/src/dto/bindings.rs:203` `every_declaration_is_exported_and_terminated` — the guard is
   `line.ends_with(';') || line.ends_with('{') || line.contains('=')`. `ts-rs` emits only
   `export type X = …` (grep: 51 `export type`, 0 `export interface` in `bindings.ts`), and every such
   line contains `=`. The assertion is a tautology. The two lines after it (`starts_with("/**")`,
   `contains("do not edit")`) are real.
2. `src-tauri/tests/capability_allowlist.rs:107` `the_forbidden_check_can_actually_fail` — builds
   `format!("{prefix}allow-everything")` from `FORBIDDEN_PREFIXES` then asserts some prefix in
   `FORBIDDEN_PREFIXES` matches it. True by construction; it exercises `str::starts_with`, not the
   gate. It does not prove that a forbidden entry *in a capability file* fails. The real gate at `:78`
   does work, so this is a misleading self-test rather than a coverage hole. Contrast
   `tests/architecture.rs:190` `the_table_assertion_can_fail`, which feeds a bad string through the
   same `missing_tables()` the gate uses — that one is a genuine negative test and is the pattern to
   copy.
3. Weak but not tautological: `src/app/App.test.ts:96` (monitoring pill "Active") — `paused` defaults
   `false`, so the assertion mostly proves the shell survives a failing `getAppStatus()`. Worth keeping,
   worth not mistaking for coverage of the pill.
4. Dead statement, not a test: `commands/meta.rs:84` `let _ = not_implemented::<()>("phase-21");`
   constructs an error and discards it. No effect; reads as a leftover.

---

## Frontend gaps

**No test file exists for any store.** `src/lib/stores/{settings,monitoring,collectors,toast,ui,router}.svelte.ts`
— only `App.test.ts`, `client.test.ts`, `state-shell.test.ts`, `contrast.test.ts`,
`theme-bootstrap.test.ts` exist. This is a real gap; each of these breaks silently:

- `settings.svelte.ts:48` — the "a failed save must not blank the screen" branch. Invert the condition
  and every test still passes; the Settings screen goes to an error face on a transient save failure.
- `collectors.svelte.ts:44` — same branch, same silence.
- `collectors.svelte.ts:40` — `setEnabled` replaces the whole list with the backend's answer. If that
  ever returned a partial list, the toggles would vanish with no failing test.
- `monitoring.svelte.ts:37` — `apply()` maps `resume_at` → `resumeAt`. A snake/camel slip here is
  invisible: the field is `number | null` and `undefined` reads as "indefinite pause", which is a
  plausible-looking wrong answer.
- `monitoring.svelte.ts:31` — `closeWouldExit` requires `status !== null && !tray_available`. It drives
  the close button's label (`TitleBar.svelte:42`), i.e. whether the user is told the window will quit.
  Untested. `App.test.ts:91` asserts the button is named `'Close to tray'` — the *default* branch only.
- `monitoring.svelte.ts:46` — the empty `catch` that makes the shell render outside Tauri. Covered
  incidentally by `App.test.ts:96`.
- `toast.svelte.ts:33` — `MAX_VISIBLE = 4` drop-oldest, and `:42` `fromError` tone mapping
  (retryable → warning, else danger). Pure functions, zero tests. The tone mapping is the thing a user
  sees on every backend failure.
- `toast.svelte.ts:35` — `setTimeout(dismiss, ttlMs)` fires against a store that may have been cleared;
  `dismiss` filters so it is safe, but nothing pins that.

Also untested on the frontend: `events.ts` `subscribe()`'s no-backend and listen-throws fallbacks
(`events.ts:31,36`), and `App.svelte:26-41`'s unmount-before-subscription-resolves guard — the
comment at `App.svelte:18` says this exists specifically to stop a listener leaking into the next mount,
and no test exercises it.

Store singletons (`settings`, `monitoring`, `collectors`, `toasts`, `ui`, `theme`) are module-level and
never reset between tests. Nothing mutates them across files today, so no failure yet; it is a latent
isolation problem for the store tests recommended above.

---

## `log_ui_error` rate limit

**Wired: yes. Asserted: no.**

- Wiring: `src-tauri/src/commands/meta.rs:43` — `if !state.ui_errors.allow()` → `AppError::RateLimited`
  before anything is logged. `AppState.ui_errors` is `RateLimiter::default()` = 10/60 s
  (`app/state.rs:118,149`; `app/rate_limit.rs:14,17,33`). Payload is bounded to 1,000 chars and context
  to 120, both through `scrub_line` (`meta.rs:49-57`). `RateLimited` classifies as retryable
  (`dto/error_dto.rs:86`), which is correct.
- Asserted: only the limiter in isolation — `rate_limit.rs:76` (10 pass, 11th refused) and `:86`
  (window resets). Nothing asserts the **command** consults it. Delete line 43 of `meta.rs` and the
  full suite stays green.
- Missing test: call `log_ui_error` 11× against one `AppState` and assert the 11th returns
  `ErrorCode::RateLimited`. Needs a `State<'_, AppState>`, which is the same obstacle as every other
  command test — see the recommendation below.
- Minor flake risk: `rate_limit.rs:86` sleeps 70 ms against a 50 ms window. Fine normally; would flake
  if the two preceding `allow()` calls ever took >50 ms on a loaded machine.

---

## Other findings

- `collector-status-changed` and `analysis-completed` are declared on both sides
  (`app/event_bus.rs:26,29`; `src/lib/tauri/events.ts:18,19`) and **emitted by nothing**. Grep for
  `EVENT_COLLECTOR_STATUS_CHANGED` finds only its definition. Consequence today:
  `collectors.attach()` (`collectors.svelte.ts:34`) subscribes to a dead channel, so the Settings
  collector list never refreshes live. Expected for this phase (later phases emit), but nothing records
  that it is expected, and no test would notice if a later phase emitted the wrong name.
- Event-name parity Rust↔TS is ungated, same class of problem as the tray routes.
- `commands/collectors.rs:30` `row()` and `commands/diagnostics.rs:40` `schema_version()` are pure
  helpers with no tests. `schema_version` swallows a `try_from` failure into `0`.
- `get_app_status` is registered and documented (`ARCHITECTURE.md:215`) but does not appear in the phase
  file's command list or its provenance note (`phase-05…md:24-36`). Surface is 34 (33 named + dev) vs
  the phase's 33; the extra one is `get_app_status`. Provenance note not updated.
- `src-tauri/tests/redaction_leak.rs` is modified in the working tree (uncommitted per the session's
  git status). Not read as part of this phase; flagging that the tree is dirty.

---

## Recommendations, in priority order

1. **Fix the settings read path** (criterion 1 defect). Make `SettingsScreen` render
   `settings.current.theme` and reconcile it with the localStorage mirror on load, then add the
   component test that would have caught it.
2. **Gate command parameter names.** Extend `dto/bindings.rs` to emit an arg-name list per command
   (hand-written beside `COMMANDS`, same rationale), and extend `commands_contract.rs` to parse the
   object literals in `client.ts` and compare. Closes the one runtime failure the typed surface
   currently permits.
3. **Add a command-level integration test harness.** Every "no test calls the command" finding above
   has the same root cause: no way to build a `State<'_, AppState>` in a test. `tauri`'s `test` feature
   (`mock_builder`) is not enabled in `src-tauri/Cargo.toml:21`. Enabling it under the existing
   `devtools` feature would unlock tests for `get_settings`/`update_settings` round-trip,
   `log_ui_error` rate limiting, `set_monitoring_paused` broadcast, and `get_collector_status` rows.
   Highest coverage-per-line available in this phase.
4. **Extract two seams in `app/`**: `route_for(id)` in `tray.rs` beside `label_for`, and a
   `close_action(tray_available: bool)` predicate in `window.rs`. Both then get direct tests, covering
   the testable half of criteria 2 and 3.
5. **Store tests** for `settings`, `monitoring`, `collectors`, `toast` — stub the `client.ts` module,
   assert the failure branches named above. Four small files.
6. **Source-text guards**, following the `lib.rs:219` pattern already in the codebase: plugin
   registration order (single-instance first); tray route literals present in `routes.ts`; event-name
   constants matching `events.ts`; every `not_implemented("phase-NN")` present in `ROADMAP.md`; no
   `app.exit`/`process::exit` outside `lifecycle::quit` and the two startup arms.
7. **Repair the two tautological tests** (`dto/bindings.rs:203`, `capability_allowlist.rs:107`) using
   the `architecture.rs:190` pattern — drive a deliberately bad input through the same function the gate
   uses. For bindings, that means calling the comparison with a mutated string and asserting it reports
   drift, which also discharges criterion 5's failing direction.
8. **Record memory measurement #2** in `docs/PERFORMANCE.md` (needs the release build, so it pairs with
   criterion 7).
9. Delete the no-op `let _ = not_implemented::<()>("phase-21");` at `commands/meta.rs:84`.

---

## Unresolved questions

1. Is the localStorage theme mirror intended to be authoritative over Rust settings, or is
   `settings.current` going unread a genuine miss? `theme.svelte.ts:6-9` says the key "remains as a
   synchronous mirror" once Rust owns the value — implying Rust *should* be the source and the
   hydration step is missing.
2. Should enabling tauri's `test` feature (item 3) be this phase's work or deferred? It changes
   `Cargo.toml` on a disk-constrained machine and adds a dependency to the dev graph.
3. Criterion 7 stays unverified until someone runs `cargo clean --profile dev` + `bun run tauri build`.
   Who owns that, and does it block phase sign-off?
4. Are `collector-status-changed` / `analysis-completed` intentionally unemitted until phases 08/15? If
   so, worth a line in ROADMAP's tracked-stubs section — the same argument the phase file makes for
   stub commands applies to stub events.
5. `src-tauri/tests/redaction_leak.rs` is uncommitted-modified. Is that in scope for anyone?
