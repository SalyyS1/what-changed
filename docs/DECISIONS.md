# Decisions

**Evidence state: verified here for recorded decisions; proposed decisions are explicitly planned or deferred.**

Durable engineering decisions with the evidence that forced them. One entry per
decision, newest last. A decision is only recorded here once it has been
*measured* on this machine — never from documentation alone.

Format: ID, date, decision, why, evidence, consequence if reversed.

**IDs are allocated here, not in the plan.** A phase that expects to record a
decision reserves its id in this file's reserved block below before using it in a
phase file, so two phases cannot both claim one number. The plan reserves
D-08 through D-17; anything new takes the next free integer.

| Reserved | Owner | Subject |
|---|---|---|
| D-08 | P07 | Timeline virtualization — **settled and recorded below**: retain `@tanstack/svelte-virtual` |
| D-09 | P21 | No automatic/background update check; a future updater is explicit opt-in only |
| D-10 | P02 | `bits-ui` for the five focus-trap primitives — **settled and recorded below**: measured compatible, kept |
| D-11 | P04 | Search index shape — **already settled and recorded below**: FTS5 measured present, one migration body |
| D-12 | P04 | Checkpoint-owned snapshots exempt from retention pruning — **settled and recorded below** |
| D-13 | P05 | Tray Pause/Resume both present, the inapplicable one disabled — **settled and recorded below** |
| D-14 | P05 | `tauri-specta` fallback to `ts-rs` — **settled and recorded below**: 1.0.2 is the Tauri v1 API, so `ts-rs` it is |
| D-15 | user | Memory: both figures reported, neither a gate — **recorded below** |
| D-16 | P02 | No font file shipped; system font chain only — **recorded below** |
| D-17 | P02 | Palette deviates from planned hex values to hold 4.5:1 everywhere — **recorded below** |

---

## D-01 — MSVC toolset and linker resolution

**Date:** 2026-09-03
**Decision:** Build against **MSVC 14.50.35717**, from the Visual Studio Build
Tools install at `<MSVC-install>`. Do not use the VS 2017 install.

**Why:** `vswhere -products * -requires
Microsoft.VisualStudio.Component.VC.Tools.x86.x64` reports exactly two installs
on this machine:

| Install path | Newest toolset | Verdict |
|---|---|---|
| `<MSVC-install>` | 14.50.35717 | chosen — current toolset, has `VC\Auxiliary\Build\vcvars64.bat` |
| `C:\Program Files (x86)\Microsoft Visual Studio\2017\BuildTools` | 14.16.27023 | rejected — 2017-era toolset, predates the C++17/20 runtime surface current crates assume |

The plan anticipated toolsets 14.44 and 14.50; 14.44 is not present, and the
second install is older than expected. Recorded here so the discrepancy is not
rediscovered later.

**Linker shadowing (verified hazard, not hypothetical):** `PATH` on this machine
resolves the bare name `link` to POSIX coreutils, not MSVC:

```
/usr/bin/link          <- Git Bash coreutils
/c/msys64/usr/bin/link <- MSYS2 coreutils
```

`rustc` locates `link.exe` through its own MSVC detection rather than a bare
`PATH` lookup, so this does not break the default build — but it is the reason a
link failure here would look impossible from `vswhere` output alone.
`scripts/verify-toolchain.ps1` reports the resolved `link` and warns when MSVC
is not first, and `scripts/dev-shell.cmd` wraps `vcvars64.bat` as the escape
hatch. No absolute linker path is committed to `.cargo/config.toml`: that would
hard-code one machine's layout into the repository, and the hazard is currently
inert.

**Consequence if reversed:** building against 14.16 risks link errors against
crates compiled for a newer CRT, with a failure signature that points at the
crate rather than at the toolset.

**Escape hatch, verified working.** `scripts\dev-shell.cmd cargo --version`
initializes the 14.50 environment and puts the MSVC `link.exe` first on `PATH`
(confirmed with `where link.exe`: the MSVC one precedes the Git Bash and MSYS2
copies). One cosmetic wrinkle worth knowing so it is not chased later: the run
prints

```
'vswhere.exe' is not recognized as an internal or external command,
```

That message comes from **Microsoft's own `vcvars64.bat`**, which probes for
`vswhere.exe` on a bare `PATH`, not from our script — verified by calling
`vcvars64.bat` directly with nothing else in the shell. It is harmless:
`VCToolsInstallDir` is set correctly and the exit code is 0. The message is left
visible rather than suppressed, because redirecting that stream would also hide a
genuine vcvars failure.

---

## D-02 — TypeScript major version

**Date:** 2026-09-03
**Status:** measured.

The plan pinned `typescript@7.0.2` with a pre-decided fallback to the latest 5.x
if `bun run check` (`svelte-check@4.7.6` + `tsc --noEmit`) fails on the
scaffold. Risk D2 in the plan.

**Result: TypeScript 7.0.2 fails the gate.** `svelte-check` 4.7.6 refuses to
start, from its own `bin/ts-version-check.js`:

> TypeScript 7 support currently requires both TypeScript 7 and TypeScript 6
> installed in your project, and requires using the `--tsgo` or
> `--tsgo-experimental-api` flag.

So the situation is a three-way choice, not the two the plan anticipated:

| Option | Cost |
|---|---|
| **Pin `typescript@5.9.3`** — chosen | one dependency, one version, no flags |
| Pin TS 6 **and** TS 7 via an npm alias plus `--tsgo` | two type-checkers in `node_modules`, an experimental flag in the gate, and `tsc --noEmit -p` still resolving to whichever binary wins |
| Drop `check` from `verify` | rejected outright — it would hide the failure instead of resolving it |

**Decision:** pin the latest 5.x (`5.9.3`) in `devDependencies` and keep
`bun run check` in `bun run verify` unchanged. Rationale is KISS: the alias
route buys nothing the project needs today (no `tsgo` performance requirement)
in exchange for a dual-toolchain setup whose failure modes are opaque.

**Consequence if reversed:** `bun run check` cannot run at all, so the frontend
loses its only type gate — the failure is a hard refusal to start, not a
degraded check, which is why this is recorded as measured rather than assumed.

**Applied by:** Phase 01 step 6. Until it runs, `package.json` still pins
`7.0.2` and `bun run check` is red by design.

---

## D-03 — Version single source of truth

**Date:** 2026-09-03
**Decision:** `src-tauri/tauri.conf.json` omits `version`; the app version comes
from `src-tauri/Cargo.toml` only. `get_app_version` returns
`app.package_info().version`, which is the same value the bundler and updater
use.

**Why:** two literal version strings drift, and the drift surfaces as an
installer that disagrees with the About screen. Tauri already infers the config
version from the Cargo package when the key is absent.

**Consequence if reversed:** the updater could compare a version the UI never
shows.

---

## D-04 — No `unwrap()` / `expect()` in shipped Rust

**Date:** 2026-09-03
**Decision:** Enforced mechanically in `src-tauri/Cargo.toml` via
`[lints.clippy]` (`unwrap_used`, `expect_used`, `panic`, `todo`,
`unimplemented` = `deny`), with `bun run rs:lint` running clippy under
`-D warnings`. Test modules opt out with an explicit `#[allow(...)]` so every
exception is visible in review.

**Why:** the app runs unattended in the tray all day. A panic is a silent
disappearance from the user's point of view, which is the one failure mode this
product cannot have.

**Consequence if reversed:** panics become invisible reliability debt, since
there is no crash reporter and no telemetry by design.

---

## D-05 — Vite 8 minifies with Oxc, not esbuild

**Date:** 2026-09-03
**Decision:** `vite.config.ts` sets `build.minify = 'oxc'` for release builds.

**Why:** Vite 8 replaced esbuild with Oxc as the default transformer and
minifier, and made `esbuild` an *optional* peer dependency. With
`minify: 'esbuild'` the build fails at the render-chunk stage:

```
Error: Failed to load `transformWithEsbuild`. It is deprecated and it now
requires esbuild to be installed separately.
Caused by: Cannot find package 'esbuild'
```

Measured: `bun run build` fails with the above and succeeds with `'oxc'`
(112 modules, 30.5 kB JS / 12.6 kB CSS, 565 ms). Installing `esbuild` to keep
the old minifier would add a dependency to restore a default that moved.

**Consequence if reversed:** the release frontend cannot be produced at all —
the failure is at bundle time, not a degraded output.

---

## D-06 — Vitest resolves Svelte through the browser condition

**Date:** 2026-09-03
**Decision:** `vite.config.ts` adds `resolve.conditions = ['browser']` when
`process.env.VITEST` is set.

**Why:** Without it, Svelte's package exports resolve to the server build under
Vitest, and `mount()` throws `lifecycle_function_unavailable` — every component
test fails before asserting anything. The app is a client-only SPA with no SSR,
so the browser condition is correct in every environment; it is scoped to Vitest
only because the production build already resolves correctly and narrowing the
change keeps the build config honest about why it exists.

`jsdom` 30.0.1 is pinned as a dev dependency for the same reason — Vitest's
`environment: 'jsdom'` is a hard requirement, not an optional extra, and its
absence fails as `MISSING DEPENDENCY` rather than as a test failure.

**Consequence if reversed:** the frontend test suite cannot mount a component,
so `bun run verify` loses its only frontend behavioural gate.

---

## D-07 — One `tsconfig`, no project reference

**Date:** 2026-09-03
**Decision:** `tsconfig.json` does not reference `tsconfig.node.json`, and
`tsconfig.node.json` is not `composite`.

**Why:** The pair was written as a TypeScript project reference, which requires
the referenced project to emit. Both are type-check-only (`noEmit`), so `tsc`
rejects the combination outright:

```
error TS6310: Referenced project 'tsconfig.node.json' may not disable emit.
```

The reference bought nothing: `tsconfig.node.json` exists only to type-check
`vite.config.ts` and `svelte.config.js` under Node types, and it is invoked
directly when needed. Removing the reference and `composite` makes
`bun run check` green (152 files, 0 errors) without weakening either config.

**Consequence if reversed:** `bun run check` fails on TS6310 regardless of the
source code, which would make the gate untrustworthy and tempting to disable.

---

## D-11 — FTS5 is present, so the search index ships one migration body

**Date:** 2026-09-03
**Status:** measured. Recorded ahead of P04 because it removes a branch from
P04's migration `002` rather than choosing between two shipped paths.

**Decision:** `002_search_index.sql` creates an **FTS5 external-content table**
and nothing else. The `search_terms` inverted-index fallback is documented as a
future path and **not implemented**.

**Why:** the plan had `002` branch at runtime on `PRAGMA compile_options`, which
would give one migration version two valid bodies and two valid checksums. That
defeats the drift checksum P04 stores per `schema_migrations` row and the
"assert the new schema version" upgrade proof in P21. And the branch cannot
actually vary: `rusqlite` is pinned with `bundled`, so FTS5 availability is a
property of the vendored amalgamation, fixed at build time.

**Evidence** — a probe crate built against the pinned `rusqlite 0.40.2`
with `features = ["bundled"]` on this machine:

```
SQLITE_VERSION=3.53.2
FTS5=true            <- ENABLE_FTS5 present in PRAGMA compile_options
FTS3=true
FTS4=false
CREATE_FTS5_TABLE=ok
```

and against an `content=''` external-content table holding one row
(`'port 3000 stopped listening'` / `'tcp:0.0.0.0:3000'`), all three query shapes
§51's search needs return the row:

| Query | Rows |
|---|---:|
| `e MATCH '3000'` (numeric token — port search) | 1 |
| `e MATCH 'port*'` (prefix) | 1 |
| `e MATCH '"0.0.0.0:3000"'` (quoted phrase) | 1 |

One incidental finding worth recording so it is not rediscovered as a bug:
`compile_options` does **not** list `ENABLE_JSON1`, yet `json_extract` and
`json_each` both work (verified: `json_extract('{"a":{"b":42}}','$.a.b')` → 42).
JSON functions are compiled in by default from SQLite 3.38 onward, so the
option is simply no longer reported. A gate that asserts `ENABLE_JSON1` would
fail on a build that fully supports JSON.

**Consequence if reversed:** shipping the conditional migration means version 2
no longer identifies a schema, so the checksum cannot detect drift and the
upgrade rehearsal can pass against a database whose search table is the other
shape. Shipping the unused second implementation instead means an unexercised
code path presented as real, which §87 forbids.

**Regression guard rather than a branch:** P04 keeps a startup assertion that
`ENABLE_FTS5` is present. If a future dependency bump drops it, the app fails
loudly at open and the fallback gets written then — against a test that
constructs the other schema explicitly.

---

## D-15 — Memory is reported as two figures and gated on neither

**Date:** 2026-09-03
**Decided by:** the user, on the measured numbers.
**Decision:** At every performance checkpoint, record **both** the total working
set (the process tree Task Manager shows) and the private working set (pages that
exist because this app is running), each against §37's 80 MB as a reference
line. **Neither figure is a pass/fail gate.**

**Why:** the brief says "under 80 MB" without saying which figure, and the two
land on opposite sides of it on an *empty* shell — 361.7 MB total, 77.1 MB private
at idle, with six WebView2 processes accounting for ~92% of the total. Choosing
total makes the target unmeetable by any application-level work; choosing private
makes it already met before a line of feature code. The user chose to keep the
measurement honest rather than pick the figure that produces a verdict.

**Evidence:** the measured rows in `docs/PERFORMANCE.md`. The two P01 rows established the decision;
the P05 row records the later full-app checkpoint with the database open and tray active. All were
captured by `scripts/measure-memory.ps1` on optimized builds.

**Stated consequence:** memory is a measurement in this release, not a budget.
The plan says so in G6, risk B2, the release success criterion and phase 20,
rather than leaving a target on paper that nothing can fail. What is still
enforced: both figures must appear at every checkpoint (a report naming one
fails), the WebView2 share is broken out, and no collector is ever disabled and no
metric silently redefined to move a number. The P01 → P05 → P20 trend rows are
what a future scope conversation stands on.

**Consequence if reversed:** picking total as a gate reintroduces a criterion that
fails on a WebView2 property no phase can change; picking private drops the
figure users actually see and complain about.

---

## D-10 — `bits-ui` stays as the headless layer for the focus-trap primitives

**Date:** 2026-09-03
**Decision:** Keep `bits-ui` 2.19.0 as the dependency behind Dialog, Drawer,
DropdownMenu, Select, Tooltip, Popover, Switch and Tabs. Every visual, class
name, animation and the composition API is local; screens import from
`src/components/ui/index.ts`, never from `bits-ui`, so the dependency can be
replaced behind that seam.

**Why:** the plan's risk row allowed dropping the dep if it did not type-check
against Svelte 5.56 or failed to trap focus. It does both: `svelte-check` on the
wrapped primitives reports zero errors, and a mounted Dialog renders
`aria-modal="true"` with a working Escape/outside-click path while Switch reports
`aria-checked`. Hand-rolling focus management is the class of code that is
subtly wrong for years (§80), and one maintained dependency replaces it.

**Evidence:** `bun run check` — 791 files, 0 errors, 0 warnings — with all eight
wrappers compiled; `src/app/App.test.ts` renders the shell with the primitive
layer mounted.

**Consequence if reversed:** the eight wrappers keep their props and test ids;
only their internals change. Focus-trap unit tests would then become mandatory
because nothing else would be guaranteeing the behaviour.

---

## D-16 — No font file is shipped; the UI uses the Windows system faces

**Date:** 2026-09-03
**Decision:** `src/styles/typography.css` declares system font chains only —
Segoe UI Variable Text / Display for UI and display text, Cascadia Mono for
monospace — with generic-family fallbacks. No `@font-face`, no `src/assets/fonts/`,
no preload in `index.html`.

**Why:** the plan asked for one self-hosted variable subset. The only candidate
faces on the build machine are the Microsoft-supplied builds of Segoe UI Variable
and Cascadia Code, whose embedded licence text forbids redistribution, and no
OFL-licensed face is present locally. Fetching one at build or run time would
break the CSP (`font-src 'self'`) and the no-network promise. The product targets
Windows 10/11 only (§6), where both families are already installed, so the
self-hosted copy would have duplicated bytes the OS provides.

**Evidence:** licence strings inspected in the installed font files; CSP in
`src-tauri/tauri.conf.json`; the no-raw-asset walk in
`src/lib/design/contrast.test.ts` finds no font reference outside the stylesheet.

**Consequence if reversed:** shipping a font means finding an OFL face, checking
it into `src/assets/fonts/`, adding `@font-face` with `font-display: swap`, and
extending the release-size budget in `docs/PERFORMANCE.md` by the subset's size.
The type scale and every component are unchanged either way.

---

## D-17 — Palette deviates from the planned values to hold AA contrast everywhere

**Date:** 2026-09-03
**Decision:** six token values differ from the hex list in the design-system
phase file: dark `--text-muted` `#828b9a` (planned `#6e7682`) and `--danger`
`#ea5b5f` (planned `#e0575b`); light `--text-muted` `#606775`, `--accent`
`#2d5fdc`, `--success` `#1a754c`, `--warning` `#866110`, `--danger` `#bc383c`,
`--info` `#1c6f8d`. Every text tier and status colour now clears **4.5:1 on
every surface it can sit on**, in both themes, and
`src/lib/design/tokens.ts` pins `minRatio: 4.5` for all pairs.

**Why:** the planned values conflicted with the phase's own success criterion.
`#6e7682` on dark `surface-3` is 3.48:1, and the muted tier is rendered at
11–13.5px body size in twenty call sites (timestamps, evidence lines, the
privacy line), where WCAG requires 4.5:1 — the 3:1 large-text level would only
apply to text ≥18.66px bold, which does not exist in the app. The plan's list
was a starting sketch; the criterion, not the sketch, is the contract. A
mutation test (`darken 40% must fail`) keeps the table honest.

**Evidence:** ratios computed by `src/lib/design/contrast.test.ts`, which
asserts every documented pair in both themes and fails on any future darkening.

**Consequence if reversed:** restoring any planned value re-fails the contrast
suite; the hue relationships are preserved, so no component or screen changes —
only the token values would move, and the test would name each violation.

---

## D-12 — A checkpoint's snapshots outlive the snapshot retention window

**Date:** 2026-09-04
**Decision:** Retention deletes detailed snapshots after 7 days **only when no
checkpoint owns them**. `snapshot_repository::delete_expired` filters
`checkpoint_id IS NULL`; a snapshot attached to a checkpoint is never pruned by
age and goes away only with its checkpoint, through the foreign key.

**Why:** a checkpoint is a marker the user created and named, and its whole
purpose is "compare with how it was then". Pruning its snapshots on the same
7-day clock as ambient detail would break "Compare with checkpoint" exactly one
week after the user thought they had saved something — the feature would work in
every demo and fail in every real use.

**Evidence:** `tests/repository.rs`, the retention test seeds three snapshots at
1 and 30 days old, one of them owned by a checkpoint, and asserts that the pass
deletes exactly the unowned expired one and that the owned one survives. The
count of deleted events in the same pass is asserted separately, so a change
that widened the delete would fail on the number rather than pass quietly.

**Consequence if reversed:** the growth is bounded by what the user themselves
created — checkpoints are explicit, few, and deletable — so the cost is small and
visible in Settings → Storage, while the alternative silently voids a feature.

---

## D-18 — The data folder comes from `dirs::data_dir()`, not Tauri's resolver

**Date:** 2026-09-04
**Decision:** `AppPaths::resolve()` builds every path from
`dirs::data_dir()/dev.whatchanged.app`. Tauri's `PathResolver` is not used for
the data or log folder.

**Why:** the plan's dependency list preferred Tauri's own path resolver, and it
does not fit where the paths are actually needed. Logging is initialized and the
database is opened **before** `tauri::Builder::build()` returns, because a
corrupt database has to be a state the first window can render rather than a
failure during setup, and because a crash during setup should still be in the
log file. Tauri's resolver needs an initialized `App`, so using it would mean
either booting the storage layer twice or logging nothing about the boot itself.
The same call also keeps `AppPaths` constructible in a test with no Tauri
runtime, which every storage suite depends on.

**Evidence:** `dirs` 6.0.0 is already in the Windows build graph through
`tauri` — `cargo tree -i dirs --target x86_64-pc-windows-msvc` shows both
`tauri v2.11.5` and this crate depending on it — so naming it directly adds a
line to `Cargo.toml` and nothing to the binary. `dirs::data_dir()` is the
roaming application-data folder, `%APPDATA%`, which is the location `README.md`
and `PRIVACY.md` already promise, and the fallback to the temporary directory
logs a warning rather than failing silently on a broken profile.

**Consequence if reversed:** the path would have to be resolved after the app is
built, which means the database opens after the first window exists and the
recovery screen loses the state it is meant to display. On a roaming domain
profile `%APPDATA%` syncs, so a very large history would travel with the user;
that is the documented location and changing it is a user-visible move, not an
implementation detail.

---

## D-19 — Dependencies compile without debug info in the dev profile

**Date:** 2026-09-04
**Decision:** `[profile.dev] debug = false, incremental = false` in
`src-tauri/Cargo.toml`. The whole default development graph omits line-level
native debug symbols and duplicate incremental objects; a developer who needs
them can opt in locally with Cargo profile environment overrides.

**2026-09-11 extension:** binding generation began failing while archiving the
application crate itself, so D-19 now covers the whole dev profile rather than
dependencies only. The test profile also sets `debug = false` and
`incremental = false`. The Phase 06 full gate reached Rust tests only after
TypeScript, bindings, Vitest, formatting and Clippy had populated the target
directory, then failed creating `what_changed_lib.lib` with OS error 112.
Test assertions and optimization are unchanged; only debug-symbol and duplicate
incremental artifacts are omitted. The measured gate runs with two Cargo jobs
so resource exhaustion is not misreported as a product failure (R15).

**Why:** disk, not speed. The dev profile reached 4.7 GB on a volume that ran
out of space four times during phases 01–04, and the failures do not look like
disk failures — `cargo clippy` reported
`failed to build archive … There is not enough space on the disk (os error 112)`
and the linker exited 1180, both of which read as toolchain faults. Dependency
debug info was most of that 4.7 GB, and nobody steps through `rusqlite`'s
internals here.

**Evidence:** the disk section of `docs/PERFORMANCE.md`, which records the
profile sizes and the three failures that led to this.

**Consequence if reversed:** the dev profile roughly doubles and a debug plus a
release profile together exceed the free space on this machine, so the next
`bun run verify` fails with an error that points at the linker. What is lost by
keeping it: a backtrace through a dependency frame shows the function but not
the line, which has not yet cost anything, since every panic so far has been in
this crate's own code or in a test.

---

## D-14 — TypeScript bindings come from `ts-rs`, not `tauri-specta`

**Date:** 2026-09-07
**Decision:** `src/lib/tauri/bindings.ts` is generated by `ts-rs` 12.0.1
derives, collected into one file by `src-tauri/src/dto/bindings.rs` and written
by the `export_bindings` example. The command *names* are emitted from
`commands::COMMANDS` and the typed wrappers in `src/lib/tauri/client.ts` are
written by hand. `tauri-specta` is not a dependency.

**Why:** the pinned `tauri-specta` 1.0.2 is the Tauri **v1** API. Its own
dependency list requires `tauri ^1.2.4` and `specta ^1.0.3`, and its entry
point is `ts::export(collect_types![…], path)` — a shape that does not exist
under Tauri 2. The Tauri-2 line is `2.0.0-rc.25`, still a release candidate,
and it pins `specta =2.0.0-rc.25` plus three `0.0.12` support crates; adopting
it would put four pre-release crates on the path that generates the frontend's
type definitions. The plan pre-decided this fallback for exactly this signal
("compile error in the export path"), and the signal arrived before a line was
written.

**Evidence:** crates.io's dependency listing for `tauri-specta` 1.0.2 —
`"crate_id":"tauri"`, `"req":"^1.2.4"` — and for 2.0.0-rc.25,
`"crate_id":"specta"`, `"req":"=2.0.0-rc.25"`. `ts-rs` 12.0.1 resolved and
compiled into this crate's Windows graph with the `serde-json-impl` feature,
which is required because an event's `before`/`after` are `serde_json::Value`.

**What is lost, stated plainly:** `tauri-specta` would have generated the
command signatures too. `ts-rs` knows nothing about `#[tauri::command]`, so the
wrapper functions are hand-written, and a hand-written wrapper can disagree
with its command. That is why the parity gate is two-sided rather than one:
`commands_contract.rs` asserts the registered handler, the generated
`COMMANDS` array and the functions `client.ts` actually invokes are the same
set, and `bun run check:bindings` regenerates the file and fails on any
difference. A command with no wrapper, a wrapper for no command, and a renamed
Rust field all break the build.

**Consequence if reversed:** when the Tauri-2 `tauri-specta` reaches a stable
release, the derives change and `dto/bindings.rs` is deleted. The two contract
tests stay either way — they test the property, not the generator.

---

## D-13 — Pause and Resume are both always in the tray menu

**Date:** 2026-09-07
**Decision:** the tray menu has seven items, always the same seven, in the same
order. `Pause monitoring` and `Resume monitoring` are both present at all
times; the one that does not apply is `set_enabled(false)` rather than removed.

**Why:** a menu that changes shape under the cursor moves the items below the
change. If Pause were removed while paused, `Settings` and `Quit` would each
shift up one row between one opening of the menu and the next — so a user who
knows where `Settings` is clicks `Quit`. The cost of the alternative is one
greyed-out row; the cost of this failure is quitting a recorder by accident,
and not noticing, because the thing it does when it quits is stop being
visible.

**Evidence:** `app/tray.rs` builds all seven from a fixed `LABELS` table and
calls `PauseResume::reflect` on build and on every state change, so the enabled
state is derived from monitoring rather than tracked separately. Its tests pin
the seven labels, their order, and that the ids are distinct.

**Consequence if reversed:** hiding the inapplicable item would make the menu
one row shorter half the time, which is not a benefit anyone asked for, against
a mis-click whose result is silent.

---

## D-20 — Event pages use opaque keyset cursors and exact cluster references

**Date:** 2026-09-11
**Decision:** The public event query accepts a semantic `TimeRange` plus filters,
an opaque URL-safe base64 cursor encoding `(timestamp, id)`, and a server-clamped
limit of 1–200. Clustering happens after one bounded page is fetched. A cluster
can be expanded only with the backend-issued `ClusterRef`, which carries its
category, coarse entity, minute bucket and exact oldest/newest timestamp/id
boundaries. Search is also bounded and returns a tagged union: either an event
summary or a checkpoint reference.

**Why:** Offset pagination becomes slower deep in a long history and shifts when
a new event arrives. A visible cursor tuple would invite callers to depend on
storage details. A cluster key alone is too broad at page edges and could expand
rows that were never represented by the item. Tagged search hits let the global
palette route a checkpoint and an event without guessing from optional fields.

**Security boundary:** The backend resolves and validates ranges, rejects forged
or malformed cursors and inconsistent cluster references, caps search text at
128 characters, and clamps result limits regardless of the frontend request.
Synthetic corpus and single-event commands have a separate double gate: build
registration (`debug_assertions` or explicit `devtools`) and the persisted
`developer_mode` setting. Normal release handlers register neither command.

**Evidence:** `tests/query_service.rs` covers range/filter plans, cursor failure,
checkpoint search and event details; `tests/cluster.rs` proves a 40-row burst
expands to exactly 40; `tests/commands_contract.rs` checks the release handler;
the Phase 06 measurements are recorded in `docs/PERFORMANCE.md`.

**Consequence if reversed:** deep pages regress toward linear work, live inserts
can move rows between offset pages, cluster expansion can leak adjacent events,
and the frontend must infer search-result kinds or trust caller-controlled
boundaries.

---

## D-08 — Retain TanStack Svelte Virtual with a bounded page window

**Date:** 2026-09-11
**Decision:** Keep `@tanstack/svelte-virtual` 3.13.36 for the Timeline. Event rows have a fixed 72px
estimate, group headers 32px and overscan eight. Independently of DOM virtualization, the Timeline store
retains at most three backend pages (600 raw event payloads); evicted adjacent pages are refetched from their
recorded keyset cursors.

**Why:** The planned Svelte 5 runes spike passed, including reactive virtualizer option updates, so the
hand-written `WindowedList` fallback would duplicate a tested library without buying compatibility. DOM
virtualization alone is not a memory bound: keeping every fetched page would still grow the store during a
long scroll, hence the separate three-page window.

**Evidence:** The Phase 07 frontend suite passed 156 tests. Store tests exercise the 600-event ceiling,
eviction, bidirectional refetch and atomic replacement of a refreshed page-zero cursor chain; component tests
prove a six-figure total does not mount the complete list. The 900×600 browser state passed visual inspection,
and final review scored 9.6/10 with no critical findings.

**Deferred evidence:** Native Tauri/WebView smoke remains for live backend events and focus behavior. The
100k-corpus scroll-FPS measurement remains Phase 20's gate; this decision claims compatibility and bounded
memory, not a measured frame rate.

**Consequence if reversed:** a custom virtualizer becomes another scroll/focus implementation to maintain,
while removing the page window makes client memory grow with browsing even if the DOM stays small.

---

## D-21 — Release uses unwind to preserve collector failure isolation

**Date:** 2026-09-13
**Status:** measured; Phase 08 accepted.
**Decision:** `[profile.release] panic = "unwind"` in `src-tauri/Cargo.toml`. Collector callbacks
remain protected by runtime panic boundaries; poisoned instances are discarded and recreated under
the existing failure/backoff policy. Retain the devtools-gated optimized executable probe.

**Why:** the previous `panic = "abort"` terminated the entire process before collector-level
`catch_unwind` could isolate the failure. A green Rust test harness does not prove the production
panic strategy, because the harness uses unwind behavior independently of an ordinary release target.

**Evidence:** `src-tauri/examples/collector_panic_probe.rs`, built as an optimized non-test binary,
passed with exit 0: the deliberately panicking collector was contained while its healthy neighbour
continued. The build took 9 min 12 s on this machine. The corrected runtime/storage paths passed
their focused regressions and final scoped review scored 9.6/10 with no blocking findings.

```bash
cargo run --manifest-path src-tauri/Cargo.toml --release --features devtools --example collector_panic_probe -j 2
```

**Consequences:** unwinding makes the documented collector isolation boundary effective in release;
it does not make arbitrary native faults recoverable. No binary-size or steady-state performance
overhead comparison was measured, so none is claimed. Current query/idle measurements are recorded in
`docs/PERFORMANCE.md`, not presented as an abort-versus-unwind comparison.

**Consequence if reversed:** collector panics can terminate the recorder, contradicting Phase 08's
acceptance contract even if unit tests remain green.

---

## D-22 — One already-resolved `windows` crate, behind a provider seam, and a partial read is an error

**Date:** 2026-09-13
**Status:** accepted; Phase 09 gates passed.
**Decision:** `windows = "0.61"` as a `[target.'cfg(windows)'.dependencies]` entry in
`src-tauri/Cargo.toml`, with exactly five named features — `Win32_Foundation`,
`Win32_NetworkManagement_IpHelper`, `Win32_Networking_WinSock`,
`Win32_System_Diagnostics_ToolHelp`, `Win32_System_Threading`. Every call sits under
`src/platform/windows/` and reaches collectors only through `ProcessProvider` and `SocketProvider` in
`src/platform/traits.rs`. A provider that cannot read the whole table returns an error; it never
returns a short list.

**Why:** the plan named `windows` 0.62. The resolved graph already carried **0.61.3** through `tauri`,
`tao`, `wry` and `webview2-com`, so declaring 0.62 would have put two versions of the same crate in one
shipped binary to gain nothing this phase uses. The provider seam exists because the alternative is a
collector that can only be tested on a machine in a particular state. The partial-read rule is the one
that is easy to get wrong quietly: a collector diffs whatever it is handed, so a truncated process walk
reports a stop for every process past the failure point, and an unreadable address family reports every
socket in it as closed. Both are fabricated history rather than missing history.

**Evidence:** `Cargo.lock` carried `windows 0.61.3` before this change and still carries exactly one
entry after it. Each signature used was read out of the resolved local crate source before the call was
written — `CreateToolhelp32Snapshot`, `Process32FirstW`/`NextW`, `OpenProcess`,
`QueryFullProcessImageNameW`, `GetProcessTimes`, `GetProcessHandleCount`, `GetExtendedTcpTable`,
`GetExtendedUdpTable` and the eight `MIB_*` row and table structs. `tests/architecture.rs` gained a
gate failing the build on a Win32 import outside `src/platform/windows/`, beside the existing gate that
keeps a target `cfg` out of `domain/`, `analysis/` and `collectors/`. Port and address conversions are
pinned by tests written from a fixed byte pattern, because reading a network-order port as a host
integer turns 4000 into a different plausible port rather than an obviously broken one.

Phase acceptance exercised the real providers as well as scripted tables: the strict 30.05-second
idle gate passed `events < 5` (the passing output did not expose an exact count), and fifty real cycles
measured process p95 at 11.8723 ms and listener p95 at 353.5 µs, both within the 15 ms budget. A
disposable development-profile smoke recorded Notepad and Python starts, `Port 4000 opened by:
python.exe`, both evidence directions between that process and `tcp:127.0.0.1:4000`, then the matching
process and port stops. The full Rust all-features gate passed 481 tests with one separately exercised
ignored idle test; final scoped review approved at 9.5/10 with no findings. These are acceptance
measurements, not evidence of a published release.

**Consequences:** the release no-network gate still resolves one dependency graph, and the target gate
keeps the crate out of a non-Windows build entirely. The privilege surface stays one directory wide, so
the non-elevation promise remains auditable by reading it: `PROCESS_QUERY_LIMITED_INFORMATION` is the
widest access asked for anywhere, and no command line or environment block is read at all.

**Consequence if reversed:** a second `windows` version in the binary, collectors that cannot be tested
without a live machine, and — the expensive one — a truncated enumeration silently rewritten into
events about programs that never stopped.

---

## D-23 — Persist a sanitized investigation snapshot, not a recipe for later re-explanation

**Date:** 2026-09-20
**Status:** accepted; Phase 15 gates passed.
**Decision:** A completed investigation is persisted as one immutable, sanitized presentation snapshot,
alongside its analysis session and score-term audit rows. The input corpus is assembled inside one
consistent SQLite read transaction, and investigation history has its own fixed 90-day retention class.
The unsent problem draft remains in frontend memory only; the non-sensitive window preference may be
stored locally.

**Why:** The phase plan originally proposed persisting ids, scores and classes, then regenerating wording
on read. That makes a reopened investigation depend on source events that can expire earlier and lets a
copy or scoring change rewrite what the user previously saw. An investigation is a historical answer, so
its displayed evidence must remain stable and reopenable after event retention. The transaction boundary
similarly prevents one answer from combining events from one database moment with relationship or rarity
data from another while collectors continue writing. Keeping the draft out of browser storage limits the
exposure of credentials pasted into the symptom field before the backend sanitizer runs.

**Evidence:** [`investigation_service.rs`](../src-tauri/src/services/investigation_service.rs) owns the
single read transaction and sanitization handoff;
[`storage/investigations.rs`](../src-tauri/src/storage/investigations.rs) owns the atomic snapshot write,
90-day pruning and delete cascade. [`investigation_service.rs` tests](../src-tauri/tests/investigation_service.rs)
delete all source events and still reopen the byte-equivalent result. Repository tests cover sanitization,
rollback and retention. [`investigation.test.ts`](../src/lib/stores/investigation.test.ts) asserts that the
draft never appears in `localStorage`.

**Consequences:** History consumes more space than an id-only recipe, but is bounded by retention and the
20-suspect result cap. Explanation improvements apply to future investigations only. Reversing this would
require an explicit product decision that historical answers may drift or become unreadable when their
source events expire.

---

## D-24 — Change Map is contextual, bounded, and equivalent without WebGL

**Decision:** Change Map is a view of the current Timeline query or one Investigation, not a top-level
destination. Rust supplies one typed payload capped at 3,000 nodes and 6,000 edges. The 3D scene uses chain
depth as its third axis; automatic mode selects the 2D surface for reduced motion and unavailable or lost
WebGL. Both renderers hand selection to the existing event-details surface through one bounded keyboard
navigator.

**Why:** A global visualization would have no query context and would compete with the seven task-oriented
destinations. Three dimensions are justified only where recorded chain depth exposes cascade shape; they
must not make investigation evidence inaccessible on machines or to users that cannot use WebGL motion.
Backend caps keep this optional leaf consumer from becoming an unbounded memory path, independent of caller
behaviour.

**Evidence:** [`change_map.rs`](../src-tauri/src/analysis/change_map.rs) owns payload shaping and caps;
[`ChangeMapFrame.svelte`](../src/components/visualization/ChangeMapFrame.svelte) owns renderer choice and the
shared accessibility surface; [`change-map.ts`](../src/lib/navigation/change-map.ts) owns contextual URL
preservation. Lifecycle, fallback and bounded-navigation tests live beside those sources. The retained
3,000-node browser evidence uses end-to-end frame intervals on Intel Iris Xe / Direct3D11 and is scoped in
[`PERFORMANCE.md`](PERFORMANCE.md#change-map-rendering--2026-09-20).

**Consequence if reversed:** making the map a destination loses the user's investigative context; deriving
chains in the frontend creates a second causal authority; requiring WebGL makes the same evidence conditional
on graphics capability or motion tolerance; removing caps lets an optional view monopolize memory.

---

## D-25 — System Map traversal stays storage-owned and evidence-literal

**Decision:** The System Map asks what is connected through persisted relationship evidence. Its bounded
breadth-first traversal and exact connected count stay at the storage boundary; the display reuses the
shared renderer but does not share the Change Map's temporal or causal meaning. An edge exposes the latest
retained `recorded_at` proof and is omitted when its endpoints cannot be resolved. No executable or
process relationship is synthesized to make the graph appear richer.

**Why:** Relationship sparsity is an observation about collector evidence, not a visualization defect.
Moving traversal into the frontend would duplicate database authority and make caps caller-dependent;
moving SQL into the pure analysis boundary would break the repository's storage isolation. Reusing renderer
mechanics is safe only while each map retains its own graph semantics.

**Evidence:** [`system_map_repository.rs`](../src-tauri/src/storage/system_map_repository.rs) owns the
bounded bidirectional walk and disclosure count; [`system_map.rs`](../src-tauri/src/analysis/system_map.rs)
owns the typed payload and caps; [`system_map_payload.rs`](../src-tauri/tests/system_map_payload.rs) covers
the five persisted edge kinds and exact truncation disclosure. The dense query and hardware-browser
measurements are scoped in
[`PERFORMANCE.md`](PERFORMANCE.md#system-map-query-and-rendering--2026-09-20).

**Consequence if reversed:** frontend traversal creates a second query engine, inferred edges violate the
product's evidence-only promise, and duplicated renderer plumbing lets capability or disposal behavior drift
between the two maps.

## D-26 — Bound history by size and expire informational process noise sooner

**Decision:** Retention keeps the user's lifetimes (30/7/90 days by default) and adds two storage-owned
rules in the maintenance pass. Informational events from the process collector expire after 3 days, or after
the user's own Events lifetime if that is shorter. Independently, when live database pages exceed 1 GiB the
oldest informational events, from any collector, are deleted in batches until the database is under 90% of
the cap. Notable, important and critical events are never deleted to satisfy the cap; if no informational
events remain the pass logs a warning and stops.

**Why:** A real profile on a developer machine with agent tooling recorded 25,330 events in about 70 hours
(22,393 in one hour, 92% from six helper programs, 25,324 informational). At roughly 930 bytes per stored
event the 30-day default alone would grow without bound on that workload. The raw events are still appended
in full and the churn summary is unchanged; this only bounds how long, and how much, informational history is
retained. Applying the short lifetime to the process collector rather than to all informational events keeps
port, service and driver evidence available for a week-old symptom.

**Evidence:** [`retention.rs`](../src-tauri/src/domain/retention.rs) holds the constants and cutoff;
[`maintenance.rs`](../src-tauri/src/storage/maintenance.rs) applies the expiry and the cap;
[`event_repository.rs`](../src-tauri/src/storage/event_repository.rs) owns the two deletes;
[`repository.rs`](../src-tauri/tests/repository.rs) covers noise expiry, a shorter user lifetime, oldest-first
cap pruning that spares notable and important rows, and a protected over-cap database. The numbers are from
one machine and one session; the 3-day lifetime and 1 GiB cap are proposed defaults, not measured optima.

**Consequence if reversed:** an uncapped history on a busy developer machine grows by hundreds of megabytes a
day, slowing every query and risking the 1M-event budgets; deleting by size without the importance guard
could discard the evidence an investigation depends on.
