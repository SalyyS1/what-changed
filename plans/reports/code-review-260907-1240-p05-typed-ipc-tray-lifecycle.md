# Code review — P05 typed IPC, tray, lifecycle

Date: 2026-09-07
Reviewer: p05-review
Scope: static review only (see Method)

## Method

The Bash tool is dead in this environment too — every call, including `echo test`,
returns empty with no exit status. **This is a static review.** No build, test,
lint or binary check was run by me. The gate results quoted in the task brief
(cargo check/fmt/clippy clean, 186 Rust tests, 96 vitest, bindings current) are
taken as given and not re-verified.

Files read in full: `src-tauri/src/app/*.rs` (9), `src-tauri/src/commands/*.rs` (10),
`src-tauri/src/dto/{mod,error_dto,settings_dto,status_dto,bindings}.rs`,
`src-tauri/src/{lib,error}.rs`, `src-tauri/src/domain/settings.rs`,
`src-tauri/src/storage/{mod,db,writer,reader_pool,maintenance,recovery,settings_repository}.rs`,
`src-tauri/tests/{commands_contract,capability_allowlist,architecture}.rs`,
`src-tauri/{Cargo.toml,capabilities/default.json,examples/export_bindings.rs}`,
`src/lib/tauri/{client,events,index,window-controls}.ts`,
`src/lib/stores/*.svelte.ts`, `src/app/{App.svelte,TitleBar.svelte,routes.ts}`,
`src/features/settings/{SettingsScreen.svelte,collector-settings.ts}`,
`src/lib/stores/stores.test.ts`, `src/lib/tauri/client.test.ts`.
Third-party behaviour was verified against the vendored sources in
`~/.cargo/registry/src/index.crates.io-*/` (tauri 2.11.5, tauri-runtime-wry 2.11.4,
tauri-macros 2.6.3) rather than from memory.

`bindings.ts` content not reviewed (generated), only its provenance.

---

## Findings

### BLOCKER 1 — `set_monitoring_paused` deadlocks the app when called from the UI

`src-tauri/src/commands/collectors.rs:77-86` → `src-tauri/src/app/lifecycle.rs:26-33`
→ `src-tauri/src/app/state.rs:174-178` → `src-tauri/src/app/tray.rs:54-63`.

`#[tauri::command] pub fn set_monitoring_paused(...)` is **not** `async`, so
tauri-macros generates the blocking body
(`tauri-macros-2.6.3/src/command/wrapper.rs:404-434`, `ExecutionContext::Blocking`
is the default at line 50). A blocking command body runs synchronously on the
thread that dispatches the IPC message — on Windows that is the webview's
message handler, driven by the main/event-loop thread.

That body calls `lifecycle::broadcast_monitoring`, which calls
`state.reflect_monitoring` → `PauseResume::reflect` → `MenuItem::set_enabled`.
`set_enabled` is `run_item_main_thread!`
(`tauri-2.11.5/src/menu/normal.rs:105-107`), which expands to
(`tauri-2.11.5/src/menu/mod.rs:25-39`):

```rust
let (tx, rx) = channel();
$self.app_handle().run_on_main_thread(task)
  .and_then(|_| rx.recv().map_err(|_| crate::Error::FailedToReceiveMessage))
```

`run_on_main_thread` is `send_user_message`
(`tauri-runtime-wry-2.11.4/src/lib.rs:1982-1984` and `235-255`). It only runs the
closure inline when `current_thread().id() == context.main_thread_id`; otherwise
it posts `Message::Task` to the event-loop proxy and the caller **blocks on
`rx.recv()` until the event loop drains it**.

So the outcome depends on which thread the sync command body lands on:

- If it is the main/event-loop thread, the send is inline and this is fine.
- If it is any other thread while the event loop is *not* pumping (it cannot pump,
  because on the close/menu paths it is the thread that is busy), `rx.recv()`
  blocks forever.

The tray path is provably safe: `on_menu_event` (`tray.rs:148-173`) already runs
on the main thread, so its `set_enabled` is the inline branch. The **command**
path is the one at risk, and it is the path the UI uses on every pause/resume
toggle from `MonitoringStore.setPaused` (`src/lib/stores/monitoring.svelte.ts:59-66`).

Failure scenario: user clicks pause in the window. If the IPC dispatch thread is
not the main thread, the command never returns; the promise never settles;
`setPaused` hangs; the window's pause control is dead for the rest of the
session, and the IPC dispatch thread is now permanently parked.

I could not run the app to determine empirically which thread Tauri 2.11.5
dispatches a sync command on, so I am reporting this as a blocker on the strength
of the source above rather than a reproduction. Two things make it worth fixing
regardless of which way that resolves:

1. If dispatch *is* the main thread, then the blocking body is doing a database
   write (`state.db.write` in `update_settings`, `complete_onboarding`,
   `set_collector_enabled`) on the UI thread, which stalls the window for the
   duration of a writer round-trip. That is a real, separate, cheaper-to-fix
   problem.
2. If dispatch is *not* the main thread, this is a hang.

Fix: make `set_monitoring_paused` `async` (which routes the body through
`async_runtime::spawn`, `tauri-2.11.5/src/ipc/mod.rs:329`) **or** make
`reflect_monitoring` fire-and-forget by wrapping the `set_enabled` call in
`app.run_on_main_thread(...)` itself and not waiting. The second is the smaller
change and also removes the UI-thread stall.

**Corollary, same root cause:** `AppState::attach_tray` (`state.rs:158-161`)
calls `tray.reflect(...)` from `setup`, which *is* the main thread — safe. No
change needed there.

---

### BLOCKER 2 — `commands/meta.rs:84` discards a `Result` to no purpose, and the line is dead

```rust
pub fn check_for_updates(app: AppHandle) -> CommandResult<UpdateCheckDto> {
    let _ = not_implemented::<()>("phase-21");
    Ok(UpdateCheckDto { ... })
}
```

`not_implemented::<()>("phase-21")` constructs an `AppErrorDto`, runs it through
`scrub_line`, and throws it away. The value is never observed. This is not a
stub marker the roadmap can see — it compiles to nothing observable and reads as
if the command is tracked when the command is in fact *complete and final* by
D-09 (it deliberately never checks for updates).

Impact is not a runtime failure; it is that a maintainer grepping for
`not_implemented` gets a false positive on a command that is finished, and the
"Tracked stubs" list in `ROADMAP.md` (per `commands/mod.rs:9-12`) is therefore
wrong about this command. Downgrade candidate — I am calling it a blocker only
because the module doc makes "a stub is visible, dated and owned, never silently
green" a stated invariant, and this line inverts it: a non-stub is silently
marked as a stub.

Fix: delete line 84. If the intent was to record the phase that owns a *future*
update mechanism, that belongs in `ROADMAP.md`, not in a discarded expression.

---

### SHOULD-FIX 3 — `notify_first_hide` can hide the window and never explain why

`src-tauri/src/app/window.rs:91-112`.

The docstring says the flag is written before the notification is sent, and that
a failed write means no balloon. That is what the code does. But the *read* has
the same fallback in the wrong direction for the failure that matters:

```rust
let already_shown = state.db.read(settings_repository::first_hide_notified).unwrap_or(true);
```

`.unwrap_or(true)` means: if the database cannot be read, assume the balloon has
already been shown, and stay silent. Combined with `hide_to_tray` having already
hidden the window (`window.rs:76-84`, the hide happens *before* this function is
called), the user gets: window vanishes, no tray notification, no explanation.

This is the exact scenario the module docstring calls "the one failure this app
cannot have" — except it is reached through a corrupt database rather than a
missing tray, so the `tray_is_available` guard does not catch it.

Note `settings_repository::first_hide_notified` (`settings_repository.rs:197-199`)
*already* has a safe internal fallback: it uses `get_or(..., false)`, i.e. "not
shown yet", and its own docstring says "the worst consequence of getting that
wrong is one balloon". The `.unwrap_or(true)` at the call site overrides that
deliberate choice with the opposite one. The two disagree.

The only way `read` returns `Err` here is a `DbState` that is not `Ready`
(`db.rs:239-246`) — corrupt, mismatched, recovering, or closed. In every one of
those states the user is *more* likely to need the balloon, not less.

Fix: `.unwrap_or(false)`, matching the repository's own documented stance. One
extra balloon in a corrupt-database session is strictly better than a silent
disappearance.

---

### SHOULD-FIX 4 — a timed pause can silently outlive its deadline for up to `INTERVAL`

`src-tauri/src/app/monitoring.rs:40-50`.

`is_paused()` expires the deadline lazily — reading is what resumes. The comment
argues this avoids a timer thread, and for the collector hot path that is right.

But nothing polls it. Grepping the tree, `is_paused` is called from
`state.rs:159` (tray attach), `lib.rs:183` (setup), and `snapshot()`
(`monitoring.rs:78`), which is reached from `AppState::status()` and from
`set_monitoring_paused`. All of those are user- or startup-driven. There is no
periodic reader.

Failure scenario: user selects "pause for 60 minutes" and closes the window to
the tray. At T+60min the pause has expired in principle. Nothing reads the flag,
so:

- The tray menu still shows Pause disabled / Resume enabled — `reflect` is only
  called from `attach_tray` and `reflect_monitoring`, neither of which runs on a
  timer.
- `EVENT_MONITORING_STATE_CHANGED` is never emitted, so a window opened later
  gets the correct answer only because `refresh()` calls `get_app_status` →
  `status()` → `snapshot()` → `is_paused()`, which expires it *at that moment*.

So the state is correct whenever anyone looks, and wrong on the tray — the one
surface visible while the window is closed — until something else happens to
look. The tray is the primary UI for a tray app.

This is latent rather than broken today (phase 05 has no collector runtime, so
nothing is being suppressed), but the tray menu is shipped now and it will show
a stale "Resume monitoring" enabled state.

Fix options, cheapest first: (a) have the tray's `on_menu_event` call
`broadcast_monitoring` before showing state — does not help, the menu is built
before the event; (b) add a low-frequency tick that calls `broadcast_monitoring`
when `snapshot().paused` transitions; (c) document that timed pause reconciles
lazily and accept the stale tray until phase 08 adds the collector loop that
polls `is_paused()` every cycle. If (c), say so in `monitoring.rs` — the current
comment claims no timer is *needed*, which is only true once a poller exists.

---

### SHOULD-FIX 5 — `AppState::status()` mutates state from a `&self` read path

`src-tauri/src/app/state.rs:181-189` → `monitoring.rs:77-88` → `monitoring.rs:40-50`.

`status()` looks like a getter and is used as one (`get_app_status`,
`MonitoringStore.refresh`). It calls `snapshot()`, which calls `is_paused()`,
which calls `self.resume()` on an expired deadline — a write.

Additionally there is a benign-but-real race inside `snapshot()`:

```rust
let paused = self.is_paused();                              // may call resume()
let deadline = self.resume_at.load(Ordering::Relaxed);      // separate load
```

Two separate atomics read non-atomically with `Relaxed` ordering. A concurrent
`pause(Some(n))` between the two lines yields `paused: false, resume_at: <new>`,
and the `if paused && ...` guard then reports `resume_at: None` — losing the
deadline the caller just set. Conversely a concurrent `resume()` yields
`paused: true` with a stale deadline, suppressed correctly by the same guard.

The window is microseconds and both racers are user-driven, so this is unlikely
to be hit. But `pause()` writing `resume_at` *before* `paused` (`monitoring.rs:57-58`)
and `is_paused()` reading `paused` before `resume_at` (`41-44`) is the ordering
that makes the interleaving possible at all, and `Relaxed` gives no
happens-before to lean on.

Fix: make the snapshot a single read of a packed state, or take both under one
`Ordering::Acquire`/`Release` pair, or simply document that `status()` is not a
pure read. The mutation-in-a-getter is the part worth fixing regardless of the
race.

---

### SHOULD-FIX 6 — `is_other_users_profile` fails open on a UNC or unset `USERPROFILE`

`src-tauri/src/app/validate.rs:212-224`. The task brief asked specifically about
this; here is what the code actually does.

```rust
let Ok(profile) = std::env::var("USERPROFILE") else { return false; };
```

**`USERPROFILE` unset** → returns `false` → the check is skipped entirely. The
docstring acknowledges this ("when they are not, the answer is 'no' and the
forbidden-root list is what protects the path"). But the forbidden-root list
(`validate.rs:120-139`) contains `SystemRoot`, `ProgramFiles*`, `ProgramData`
and the app's own data dir — **not** `C:\Users`. So with `USERPROFILE` unset,
`C:\Users\someone-else\Documents` passes every check and is accepted. The
docstring's claim that the forbidden-root list covers this case is wrong.

**UNC `USERPROFILE`** (roaming profile, `\\server\profiles\alice`): `Path::parent()`
on `\\server\profiles\alice` gives `\\server\profiles`, so `users_dir` becomes
`\\server\profiles` and the logic works — *provided* the candidate path is also
UNC and lowercases identically. A local path under `C:\Users\bob` is not inside
`\\server\profiles`, so `is_other_users_profile` returns `false` and, again, only
the forbidden-root list applies, which does not cover `C:\Users`. So a machine
with a roaming profile loses the other-user check for local paths.

**Drive-root `USERPROFILE`** (pathological, `C:\`): `parent()` returns `None`
→ `false` → check skipped.

How reachable is this? `USERPROFILE` is set by Windows for interactive logons, so
the unset case is not the common one. But this is a *defence-in-depth* check
against a deliberately hostile input (the user picking, or a junction pointing
at, another profile), and it silently becomes a no-op in three environments
without any log line saying so.

Also note the ordering is otherwise sound and the brief's concern about it is
unfounded: `exists()` → `is_reparse_point()` (via `symlink_metadata`, which does
not follow, `validate.rs:97-106`) → `is_dir()` → `canonicalize()`. Rejecting the
reparse point outright before canonicalizing is the right call and the comment
explaining why (`validate.rs:8-13`) is accurate. There *is* a TOCTOU window
between the reparse check and `canonicalize` — an attacker who can replace the
directory with a junction in that window wins — but exploiting it requires write
access to the parent directory of a path the user just picked in a dialog, at
which point the attacker already has the user's privileges. Not worth fixing.

Fix: add `C:\Users` (derived from `USERPROFILE`'s parent, or from
`SystemDrive\Users` as a fallback) to `forbidden_roots` when the profile cannot
be determined, and `tracing::warn!` once when the check is skipped.

---

### SHOULD-FIX 7 — `EventBus` `Drop` ordering can drop the final batch notice

`src-tauri/src/app/state.rs:120-129` and `event_bus.rs:92-99`.

`AppState`'s field declaration order is `paths, db, startup, monitoring,
ui_errors, tray, shutdown_recorded, bus, _maintenance`. Rust drops fields in
declaration order, so on `AppState` drop: `db` (field 2) is dropped **before**
`bus` (field 8) and `_maintenance` (field 9).

`db` is `Arc<Db>` and `_maintenance` holds another `Arc<Db>` clone
(`state.rs:143`, `MaintenanceHandle::spawn(Arc::clone(&db))`), so dropping field
2 does not actually close the database — the maintenance thread's clone keeps it
alive until field 9 drops. That is load-bearing and accidental: it works because
of the `Arc` clone, not because the ordering was chosen.

The `EventBus` side is correct on its own terms: `run_loop` handles
`Disconnected` by flushing `pending` before breaking (`event_bus.rs:130-135`),
so a notice collected inside an open window is delivered when the writer's
sender is dropped. And `EventBus::drop` sets `stop` then joins.

But `EventBus::drop` sets `stop = true` and joins **without** draining. If the
stop flag is observed at the top of the loop (`event_bus.rs:108-110`) while
`pending` holds a merged notice whose 500 ms window has not yet elapsed, that
notice is **dropped on the floor** — `break` at line 109 skips the flush at
lines 138-144.

Failure scenario: events are written, coalesced into `pending`, and the user
quits within 500 ms. The final batch notice never reaches the webview. In
practice the webview is being torn down anyway, so nobody would see it — the
real cost is that the code claims "notices that arrive inside a window are
merged rather than dropped" (`event_bus.rs:8-11`) and that claim is false on the
shutdown path.

Fix: in `Drop`, flush before joining, or have the loop flush `pending` on the
stop-flag branch the same way it does on `Disconnected`.

---

### SHOULD-FIX 8 — `lib.rs:208` is a no-op that looks like a lifetime anchor

```rust
let _ = Arc::clone(&state.db);
```

Clones the `Arc`, increments the refcount, binds to `_`, drops immediately,
decrements. Net effect: nothing. It reads as though it is keeping the database
alive past the end of `setup`, which is what a reader would assume from an
otherwise-unexplained `Arc::clone`, and it does not do that. `state` is managed
by Tauri and outlives `setup` regardless.

Fix: delete it.

---

### NIT 9 — `commands/dev.rs:26` uses `unwrap_or_default()` to mask an impossible empty vec

```rust
Ok(ids.into_iter().next_back().unwrap_or_default())
```

`insert_events` with a one-element input returns one id; the `unwrap_or_default()`
returns `0` for an empty result, and `0` is not a valid row id — a caller would
treat it as a real event id. Behind `devtools`, so shipped builds are unaffected.
Prefer returning an explicit error, or `ids.last().copied().ok_or(...)`.

### NIT 10 — `diagnostics.rs:35` re-enters a command function

`get_diagnostics` calls `collectors::get_collector_status(state)?` — a
`#[tauri::command]` function called directly as a helper. It works (the macro
generates a separate wrapper and leaves the original callable), and it does
correctly avoid duplicating the mapping. But it couples two command bodies, and
`get_collector_status` also consumes the `State` guard, which is why it must be
last in the struct literal. Extracting the shared body into a plain function
would make the ordering constraint disappear.

### NIT 11 — `commands/mod.rs:92` asserts a hardcoded 34

`assert_eq!(COMMANDS.len(), 34)` plus a filtered count of 33. This is a real
assertion (it fails if a command is added without updating the count), but the
comment "33 named commands plus `simulate_event`" plus two derived counts is
three statements of one fact. The `lib.rs:219` test that compares the macro
against `COMMANDS` is the one carrying weight.

### NIT 12 — `capability_allowlist.rs:126` names a phase in a test name

`the_notification_permission_is_not_granted_before_the_phase_that_uses_it` — the
project's own rule (`review-audit-self-decision.md`, "Stable Code Artifacts")
says not to put phase references in test names. The invariant is "no
notification permission is granted"; the phase framing will be stale the moment
that phase lands.

---

## Checked and clean

These were on the brief's list and I found nothing wrong:

- **Event-bus coalescing timing.** `run_loop` (`event_bus.rs:103-146`) is
  correct for the steady state. The `wait` computation shrinks to the remaining
  window while one is open, `merge` is order-independent for all four fields,
  `newest_id` uses `max` not last-write, and the disconnect path flushes. Only
  the stop-flag path (finding 7) loses anything.
- **`merge` category ordering.** `categories.sort_unstable()` after the union
  gives declaration order, matching what `writer::publish` produces
  (`writer.rs:256-258` also sorts) — so a merged and unmerged notice are
  ordered identically. The test at `event_bus.rs:216-246` asserts this and
  additionally asserts the ordering *is* the enum's, which is the part that
  would silently change if someone reordered `Category`.
- **Close-to-tray.** `on_window_event` (`window.rs:52-67`) checks
  `tray_is_available` **before** `api.prevent_close()`, so a missing tray means
  close-means-close. `tray_available()` reads the `Mutex<Option<...>>` that is
  only ever populated on the success branch of `tray::build`
  (`lib.rs:183-190`). There is no path that hides with no tray. Confirmed.
- **`Mutex` poisoning.** Every lock in the reviewed code recovers with
  `unwrap_or_else(|p| p.into_inner())` (`state.rs:210-214`, `db.rs:320-324`,
  `rate_limit.rs:55-57`, `reader_pool.rs` via `db::lock`). Consistent.
- **`shutdown_recorded`.** `swap(true, SeqCst)` (`state.rs:194`) is the correct
  idempotence primitive, and the test at `state.rs:304-320` proves the second
  call is a no-op by a mechanism that would actually catch a regression.
- **`get_db_state` → `get_app_status` migration.** No reference to
  `get_db_state` or `getDbState` survives anywhere in the tree (grepped).
  `COMMANDS` (`commands/mod.rs:38-81`), the handler macro (`lib.rs:34-69`) and
  `client.ts:106` all agree.
- **`ui.svelte.ts` losing `monitoring`.** No caller reads `ui.monitoring` or
  `ui.paused` (grepped). The two consumers — `App.svelte:27` and
  `TitleBar.svelte:42,66,67` — both import the new store.
- **`windowControls.close()` meaning change.** One call site
  (`TitleBar.svelte:91`), and its label is derived from
  `monitoring.closeWouldExit` (`TitleBar.svelte:41-43`), so the button tells the
  truth in both modes. Covered by `stores.test.ts:143-167`.
- **Frontend import boundary.** `@tauri-apps/api` appears in exactly three files,
  all under `src/lib/tauri/` (grepped), and `commands_contract.rs:126-150`
  enforces it as a build gate.
- **Command bodies.** All 34 are glue. Longest is `get_diagnostics`
  (`diagnostics.rs:15-37`, 23 lines) and it is assembly, not logic. None exceeds
  30 lines. Validation-then-delegate is consistent.
- **No `unwrap`/`expect`/`panic` in shipped Rust.** The only hits under
  `src/app/` outside `#[cfg(test)]` are in test modules with explicit
  `#[allow]`. `architecture.rs:79-93` enforces this by reading source text, and
  `production_code()` correctly cuts at `#[cfg(test)]` and strips comments.
  `reader_pool.rs:94-100` avoids a panic via `std::process::abort()` in a `#[cold]`
  diverging fn — ugly but honest and within the rule.
- **SQL containment.** `rusqlite::` appears only under `storage/` and in
  `error.rs` (grepped), matching the exemption in `architecture.rs:61`.
- **Capability surface.** `default.json` grants six permissions, exactly matching
  the `ALLOWED` set in `capability_allowlist.rs:25-37`. No forbidden prefix. The
  `the_forbidden_check_can_actually_fail` test (line 107) is a genuine
  anti-vacuity check, as is `the_table_assertion_can_fail`
  (`architecture.rs:190`). Both would fail if the predicate were broken.
- **Error scrubbing.** `AppErrorDto::new` (`error_dto.rs:57-63`) runs every
  message through `scrub_line` unconditionally, and `AppError`'s variants carry
  `&'static str` operation names rather than values (`error.rs`), so there are
  two independent reasons a value cannot escape. `validate.rs:340-348` tests the
  path-not-echoed property specifically.
- **Retryability classification.** `classify` (`error_dto.rs:79-94`) marks
  exactly `RateLimited`, `NotReady` and `QueueFull` retryable. `WriterGone` maps
  to `NotReady` but **non**-retryable, which is correct — a stopped writer does
  not restart itself.
- **Bindings provenance.** `export_bindings.rs` normalises CRLF before comparing
  (line 45, 69-71), which is the right call on Windows.

---

## Recommended actions

1. Resolve finding 1 — either make `set_monitoring_paused` async or make
   `reflect_monitoring` non-blocking. Do not ship the pause toggle until this is
   settled one way or the other.
2. Fix finding 3 (`.unwrap_or(true)` → `.unwrap_or(false)` in `window.rs:99`).
   One character, removes a silent-disappearance path.
3. Delete `meta.rs:84` and `lib.rs:208` (findings 2, 8).
4. Decide on finding 4 — either add a reconciliation tick or correct the comment
   in `monitoring.rs` to say the lazy expiry depends on a future poller.
5. Add `C:\Users` to `forbidden_roots` and warn when the profile check is skipped
   (finding 6).
6. Flush `pending` on the stop-flag branch of the event-bus loop (finding 7).
7. Nits 9-12 at leisure.

## Unresolved questions

1. **Which thread does Tauri 2.11.5 dispatch a synchronous command body on, on
   Windows?** Finding 1 is a hang if it is not the main thread and a UI stall if
   it is. I could not run anything to determine this. Someone with a working
   shell should confirm before deciding how much to change.
2. Is the stale tray state during an expired timed pause (finding 4) acceptable
   until the phase-08 collector loop lands, or is it a shipped-now defect? That
   is a product call.
3. `MonitoringStore.resumeAt` is stored and tested (`stores.test.ts:129-141`) but
   nothing renders it — the titlebar shows only Active/Paused. Is a
   "paused until HH:MM" surface planned, or is the field speculative?
4. Is `Db::close()` ever called on the shutdown path? `lifecycle::shutdown`
   flushes but does not close, and `AppState` has no `Drop`. On process exit this
   is harmless (WAL is durable), but it means the `-wal` and `-shm` files are
   left behind on every quit rather than checkpointed.
