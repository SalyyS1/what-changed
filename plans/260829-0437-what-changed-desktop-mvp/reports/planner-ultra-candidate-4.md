# Candidate 4 — Prove the toolchain, weld the privacy chokepoint shut, scale the UI on synthetic data, then land collectors in measured-cost order

## Thesis

Three inversions de-risk this build. **(1) Trust before data:** the redaction engine and a
`Sanitized<T>` newtype whose only constructor lives in `privacy::sanitize` land *before* the
repository exists, so the write path physically cannot compile without passing the chokepoint —
§34 becomes a type, not a code review. **(2) Scale before reality:** the full event/query/timeline
stack is built and stress-proven to 1M rows against a synthetic generator before the first Win32
call, so §61 is settled while collectors are still unwritten and UI work never waits on OS work.
**(3) Cost-ordered collectors:** the measured snapshot table (research §3) dictates landing order —
cheap-diff collectors (process 6 ms, services 3.3 ms, listeners sub-ms from Rust) first, then the
notification-driven expensive ones (drivers 2.1 s, tasks 1.3 s, network 1.4 s), each behind a
supervisor so one failure is contained. B1 (Rust absent) is Phase 01 step 2 with a hello-world
bundle as its gate; B2 (RAM) is measured three times with a pre-decided escalation, never quietly
redefined; B3 (process blind spot) is written into `COLLECTORS.md` as a named limitation.

## Plan metadata

```yaml
---
title: "What Changed — Windows change recorder & causal investigator (MVP)"
description: "Non-elevated Tauri 2 + Svelte 5 tray app that records meaningful Windows change locally and deterministically answers 'It worked before. What changed?'"
status: pending
priority: P1
effort: 131h
branch: main
tags: [tauri2, rust, svelte5, sqlite, windows, privacy, observability, desktop, mvp]
created: 2026-08-29
blockedBy: []
blocks: []
---
```

## PLAN.MD BODY

```markdown
## Overview

"What Changed" is a **non-elevated** Windows 10/11 x64 tray app that continuously records
meaningful system change into a local SQLite history and answers **"It worked before. What
changed?"** in seconds, for a non-technical user, with progressive technical depth for an
advanced one. Stack is fixed: Tauri 2 + Rust + Svelte 5 (runes) + TypeScript + Vite + Bun
(dev only) + Tailwind 4 + Three.js + SQLite. No Electron, no Go, no second process, no
account, no cloud, no telemetry, fully offline.

Repo root (`$ROOT`) = `D:\Project\.WhatChage` — the folder name is misspelled upstream and
**nothing in code or config may depend on it**; product = `What Changed`, binary =
`WhatChanged.exe`, package = `what-changed`. Repo is a true greenfield: only `.git/` and
`plans/` exist, `main` has zero commits, and **Rust is not installed** (verified).

Research measured 34 of 35 troubleshooting capabilities working under a plain `BUILTIN\Users`
token, so §36's non-admin privilege model is achievable. The two measured gaps —
`Win32_ProcessStartTrace` (Access denied) and the `Security` event-log channel (unauthorized) —
are designed around and documented, not worked around with elevation.

## Goals

| # | Goal | Priority |
|---|------|----------|
| G1 | A normal human describes a symptom and within seconds sees the most likely related change, with plain-language evidence | P1 |
| G2 | 12 collectors record meaningful change continuously, non-elevated, each independently failable | P1 |
| G3 | Deterministic transparent ranking: the port/config causal chain outranks an unrelated Chrome update, with a visible "why" | P1 |
| G4 | Secrets are never persisted and never logged — enforced at one typed chokepoint, proven by leak tests | P1 |
| G5 | Premium calm desktop identity: all six states on every screen, exact §72–§74 microcopy, 900×600 min, a11y | P1 |
| G6 | Idle CPU ≈0–1%; memory measured and honestly reported against the <80 MB target | P2 |
| G7 | Three.js used intentionally (ambient field, causal graph, system map) and never required for core use | P2 |
| G8 | Installs, runs offline, upgrades without ever destroying the event database | P1 |
| G9 | Architecture ready for a future sidecar collector, AI explanation provider, and Impact — none built now | P3 |

## Phases

| # | Phase | Status |
|---|-------|--------|
| 01 | [Foundation & Toolchain](./phase-01-foundation-toolchain.md) | pending |
| 02 | [Design System & App Shell](./phase-02-design-system-app-shell.md) | pending |
| 03 | [Redaction & Privacy Chokepoint](./phase-03-redaction-privacy-chokepoint.md) | pending |
| 04 | [Domain Model & Persistence](./phase-04-domain-model-persistence.md) | pending |
| 05 | [Typed IPC, Tray & Lifecycle](./phase-05-typed-ipc-tray-lifecycle.md) | pending |
| 06 | [Event Engine & Queries](./phase-06-event-engine-queries.md) | pending |
| 07 | [Timeline & Search UI](./phase-07-timeline-search-ui.md) | pending |
| 08 | [Collector Runtime](./phase-08-collector-runtime.md) | pending |
| 09 | [Process & Port Collectors](./phase-09-process-port-collectors.md) | pending |
| 10 | [File & Config Collector](./phase-10-file-config-collector.md) | pending |
| 11 | [Service & Application Collectors](./phase-11-service-application-collectors.md) | pending |
| 12 | [Update & Driver Collectors](./phase-12-update-driver-collectors.md) | pending |
| 13 | [Network, Startup & Event Log Collectors](./phase-13-network-startup-eventlog-collectors.md) | pending |
| 14 | [Checkpoints](./phase-14-checkpoints.md) | pending |
| 15 | [Analysis Engine & Investigation](./phase-15-analysis-engine-investigation.md) | pending |
| 16 | [Three.js Visualization](./phase-16-three-js-visualization.md) | pending |
| 17 | [Docker & Development Mode](./phase-17-docker-development-mode.md) | pending |
| 18 | [Onboarding, Notifications & Diagnostics](./phase-18-onboarding-notifications-diagnostics.md) | pending |
| 19 | [Performance & Accessibility Hardening](./phase-19-performance-accessibility-hardening.md) | pending |
| 20 | [Installer, Updater & Release Audit](./phase-20-installer-updater-release.md) | pending |

## Architecture Summary

Svelte UI (WebView2) → **narrow typed Tauri commands** → `AppService` → { `QueryService`,
`AnalysisEngine`, `SettingsService`, `CheckpointService` } → `EventRepository` → SQLite (WAL).
Collectors → `mpsc` → `Normalizer` → `Sanitizer` → batched writer task → repository. The UI never
touches an OS API; there is no `execute_shell`, no arbitrary file read, no SQL from the frontend.

Rust module split (`$ROOT/src-tauri/src/`): `app/` (wiring, state, lifecycle), `commands/` (IPC
only, no logic), `domain/` (§9 types — **source of truth**, exported to TS), `storage/`
(repository + migrations + maintenance + recovery), `analysis/` (parser, scoring, chains,
explanations), `privacy/` + `redact/` (chokepoint), `collectors/*` (platform-agnostic diff→event
logic against provider traits), `platform/windows/*` (the only place Win32/COM lives).
Collectors depend on provider traits, so every collector is unit-testable with fake snapshots and
`core/` stays free of `#[cfg(windows)]`.

Frontend (`$ROOT/src/`): `app/` shell+router, `components/{ui,timeline,investigation,graph,system}`,
`features/{home,timeline,investigation,checkpoints,search,settings,onboarding}`,
`lib/{tauri,stores,types,formatting,design}`, `three/{engine,ambient,causal-graph,system-map}`,
`styles/`. Rust types generate `src/lib/tauri/bindings.ts`; a parity gate fails the build if
generated types drift from committed ones.

## Success Criteria

- [ ] `bun run verify && bun run tauri build` is green from a clean clone on this machine
- [ ] Close button hides to tray; monitoring continues; tray has exactly the seven §7 entries
- [ ] 12 collectors run non-elevated; `get_collector_status` shows per-collector health; killing one collector never stops the app
- [ ] Redaction leak suite (§60: `sk-`, `Bearer`, JWT, AWS, PEM, `postgres://user:pass@`, `mongodb://`, cookie, session) proves no raw secret in DB **or** log files
- [ ] §59 fixture: the config→restart→port chain ranks #1 and strictly outranks the unrelated Chrome update, including when the Chrome update is moved nearer in time
- [ ] 10k / 100k / 1M event suites pass their recorded budgets; timeline virtualizes; no query returns unbounded rows
- [ ] Every major screen demonstrably renders loading, empty, error, success, large-data and small-window (900×600) states
- [ ] Migrations apply forward from empty and from v1 with data; a pre-migration backup exists; a deliberately corrupted DB yields the recovery screen, not a crash loop
- [ ] Memory measured (backend RSS vs WebView2 total) and recorded in `docs/PERFORMANCE.md`; any miss of §37 reported to the user as a scope question
- [ ] Notification suite: a stream of Chrome/Spotify updates and process start/stop produces **zero** notifications
- [ ] `WhatChanged-Setup-x64.exe` installs, upgrades over an existing install without touching `events.db`, and uninstalls cleanly
- [ ] No `unwrap()`/`expect()` in non-test Rust (enforced by clippy); `-D warnings` clean; TS strict with no undocumented `any`

## Key Risks

| # | Risk | Signal | Pre-decided response |
|---|------|--------|----------------------|
| B1 | Rust absent; 3 MSVC installs → linker ambiguity | `rustup-init` fails, or `link.exe not found` | winget fallback for rustup; `scripts/dev-shell.cmd` wraps the chosen `vcvars64.bat`; pin choice in `docs/DECISIONS.md` |
| B2 | <80 MB RAM likely unreachable with WebView2 | `scripts/measure-memory.ps1` total >80 MB in P01/P05/P19 | Optimize the listed levers, then report measured backend RSS vs WebView2 total to the user as a scope question; never redefine the metric silently |
| B3 | Non-admin process events are diff-polled → blind spot | Short-lived process absent from timeline in the P09 fixture | Document in `COLLECTORS.md` "Known limitations"; classify exits `unexpected` only with Event Log corroboration; 1h ETW spike, then stop |
| D2 | TypeScript 7.0.2 vs `svelte-check` 4.7.6 / Vite 8 | `bun run check` fails on the P01 scaffold | Pin the latest 5.x, record resolved version + reason in `docs/DECISIONS.md` |
| R5 | Driver/network/task enumeration is 1.3–2.3 s | Idle CPU >1% in P19 sampling | Notification-driven wake + slow sweep only; never a short timer; accept and document detection latency |
| R6 | Notification APIs for registry/service/device change unproven non-admin | Spike returns error or never fires | Fall back to the measured slow diff-poll for that collector and record the latency in `COLLECTORS.md` |
| R7 | Docker connected path untestable here | No `\\.\pipe\docker_engine` | Verify only the absent path; mark connected path "untested on this machine" in `COLLECTORS.md` |
| R8 | Analysis over-claims causality | Any UI string asserts a cause | Language allowlist test over explanation templates; forbidden phrases fail the suite |

## Dependencies

- **Toolchain (P01 blocks everything):** `rustup` + `stable-x86_64-pc-windows-msvc`, pinned in
  `$ROOT/rust-toolchain.toml`; MSVC 14.44/14.50 already present; Windows SDK 10.0.26100.0;
  WebView2 Evergreen 151.0.4129.107 present; Bun 1.3.11; Git 2.52.0. Git identity is **not**
  configured — P01 asks the user once and sets it repo-locally.
- **Crates (pinned):** tauri 2.11.5, tauri-build 2.6.3, rusqlite 0.40.2 (`bundled`),
  rusqlite_migration 2.6.0, notify 8.2.0, notify-debouncer-full 0.7.0, windows 0.62.2,
  specta 1.0.5, tauri-specta 1.0.2, tokio 1.53.1, tracing-subscriber 0.3.23,
  tracing-appender 0.2.5, thiserror 2.0.20, bollard 0.21.1, gix 0.87.1; plugins
  single-instance 2.4.3, autostart 2.5.1, notification 2.3.3, updater 2.10.1,
  window-state 2.4.1, dialog 2.7.2.
- **npm (pinned):** svelte 5.56.10, @sveltejs/vite-plugin-svelte 7.3.0, vite 8.2.2,
  typescript 7.0.2 (D2 gate), tailwindcss + @tailwindcss/vite 4.3.3, three 0.185.1 +
  @types/three 0.185.4, vitest 4.1.11, @testing-library/svelte 5.4.2, @tauri-apps/cli 2.11.4,
  @tauri-apps/api 2.11.1, svelte-check 4.7.6, playwright 1.62.1,
  @tanstack/svelte-virtual 3.13.36, bits-ui 2.19.0.
- **Not pinned by research** (resolve with `cargo add` / `bun add`, record the resolved version in
  `Cargo.lock` / `bun.lock` and `docs/DECISIONS.md`; do not invent a number): `tracing`, `serde`,
  `serde_json`, `regex`, `dirs`-equivalent path helper (prefer Tauri's own path resolver).
- **Rejected on purpose:** `tauri-plugin-sql` (would expose SQL to the frontend, violates
  §35/§56), `tauri-plugin-log` (duplicate logging stack; `tracing` + `tracing-appender` covers
  §4 and is redaction-wrapped), `sysinfo` (two Win32 calls replace it, §80),
  `Win32_QuickFixEngineering` (measured 4 rows vs WUA's 133), Electron/Go (§4/§5).
```

## PHASE FILES

> Path convention in every phase file: `$ROOT` = `D:\Project\.WhatChage` (POSIX form
> `/d/Project/.WhatChage` in Git Bash). All commands run from `$ROOT` unless stated.

### FILE: phase-01-foundation-toolchain.md

```markdown
---
phase: 01
title: "Foundation & Toolchain"
status: pending
priority: P1
effort: 5h
dependencies: []
---

## Overview

Resolve blocker B1 (Rust is not installed), decision D2 (TypeScript 7 vs 5.x), and decision D1
(never depend on the misspelled folder name). Write the §90-mandated documents *before* app code.
Hand-scaffold a Tauri 2 + Svelte 5 + Vite 8 + Tailwind 4 app (no interactive template generator),
prove it builds debug **and** release **and** an NSIS bundle, and take the first B2 memory
measurement. Nothing else in this plan can start until `bun run tauri build` succeeds here.

## Requirements

**Functional**
- `rustup` installed per-user, default toolchain `stable-x86_64-pc-windows-msvc`, pinned in
  `$ROOT/rust-toolchain.toml` with the *actually installed* version recorded (never a guessed one).
- One MSVC install selected and recorded; a repeatable escape hatch if `link.exe` is not found.
- `$ROOT/package.json` scripts: `dev, build, check, test, tauri, rs:fmt, rs:lint, rs:test, verify, mem`.
- App identity: `productName "What Changed"`, `mainBinaryName "WhatChanged"`,
  `identifier "dev.whatchanged.app"`; window 1280×820, `minWidth 900`, `minHeight 600`,
  `decorations: false` (custom chrome in P02).
- Docs (§55/§68/§90) authored with real content, not placeholders: `README.md`, `PRODUCT.md`,
  `ARCHITECTURE.md`, `SECURITY.md`, `PRIVACY.md`, `ROADMAP.md`, `CONTRIBUTING.md`,
  `COLLECTORS.md`, `ANALYSIS.md`, plus `docs/DECISIONS.md`, `docs/PERFORMANCE.md`.
- One `get_app_version` command + one Svelte call, proving IPC end-to-end (§69 P0 acceptance).

**Non-functional**
- No Node/Bun requirement in the shipped app; `withGlobalTauri: false`; CSP set from day one.
- No bash heredocs in any script (Git Bash constraint) — `.ps1`/`.cmd`/`bun` scripts only.
- Release binary must not open a console window (`windows_subsystem = "windows"`).

## Architecture

`src-tauri/src/main.rs` is a 3-line shim calling `what_changed_lib::run()`; `lib.rs` owns the
builder. Frontend is a plain Svelte 5 SPA (no SvelteKit: no SSR, no adapter, no router build
layer — research §4.4). Tailwind 4 is wired via `@tailwindcss/vite`, not PostCSS.

## Related Code Files

**Create** — `$ROOT/`: `.gitignore`, `.gitattributes`, `rust-toolchain.toml`, `README.md`,
`PRODUCT.md`, `ARCHITECTURE.md`, `SECURITY.md`, `PRIVACY.md`, `ROADMAP.md`, `CONTRIBUTING.md`,
`COLLECTORS.md`, `ANALYSIS.md`, `docs/DECISIONS.md`, `docs/PERFORMANCE.md`, `package.json`,
`tsconfig.json`, `tsconfig.node.json`, `vite.config.ts`, `svelte.config.js`, `index.html`,
`src/main.ts`, `src/app/App.svelte`, `src/styles/app.css`, `src/vite-env.d.ts`,
`assets/brand/icon-source.svg`, `scripts/verify-toolchain.ps1`, `scripts/measure-memory.ps1`,
`scripts/dev-shell.cmd`, `src-tauri/Cargo.toml`, `src-tauri/build.rs`,
`src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`,
`src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`, `src-tauri/src/commands/meta.rs`,
`src-tauri/icons/*` (generated).
**Modify** — none (greenfield). **Delete** — none.
## Implementation Steps

1. **Git identity** (packet: none configured). Ask the user once for name + email — this is a §86
   "genuinely impossible to infer" case — then `git config --local user.name/user.email`. Never
   touch global config.
2. **Install Rust (B1).** `powershell -NoProfile -Command "Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile $env:TEMP\rustup-init.exe"`
   then `"$TEMP/rustup-init.exe" -y --default-toolchain stable-x86_64-pc-windows-msvc --profile default`
   (per-user, no elevation). Open a fresh shell; verify `rustc -vV` prints
   `host: x86_64-pc-windows-msvc` and `cargo --version` works.
3. **Pin it.** Write `rust-toolchain.toml` with `channel = "<exact version from rustc -vV>"`,
   `targets = ["x86_64-pc-windows-msvc"]`, `components = ["rustfmt", "clippy"]`.
4. **Pin MSVC.** `scripts/verify-toolchain.ps1` runs `vswhere -latest -products * -requires
   Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`, prints all
   matches, writes the chosen one to `docs/DECISIONS.md` (D-01). `scripts/dev-shell.cmd` calls that
   install's `VC\Auxiliary\Build\vcvars64.bat` then `%*` — used only if step 10 fails.
5. **Frontend deps.** Hand-write `package.json` with the pinned versions from the plan Dependencies
   list, then `bun install`. Scripts: `check` = `svelte-check --tsconfig ./tsconfig.json && tsc --noEmit -p tsconfig.json`;
   `verify` = `bun run check && bun run test && bun run rs:fmt && bun run rs:lint && bun run rs:test`;
   `rs:*` use `--manifest-path src-tauri/Cargo.toml` (if `cargo fmt` rejects that flag, switch the
   script body to `cd src-tauri && cargo fmt --all -- --check` and note it in DECISIONS).
6. **D2 gate.** Run `bun run check` on the scaffold with `typescript@7.0.2`. On failure, `bun add -d
   typescript@^5` (record the resolved version), re-run, and write the outcome to `docs/DECISIONS.md`
   (D-02). Do not proceed with a red `check`.
7. **Tailwind 4.** `src/styles/app.css` starts with `@import "tailwindcss";` and an empty `@theme`
   block (tokens arrive in P02). Register `@tailwindcss/vite` in `vite.config.ts` alongside
   `svelte()`; set `server.port = 5183`, `server.strictPort = true`, `clearScreen: false`.
8. **Tauri config.** `src-tauri/tauri.conf.json`: identity keys from Requirements; `app.withGlobalTauri:
   false`; `app.security.csp = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
   img-src 'self' data:; font-src 'self'; connect-src 'self' ipc: http://ipc.localhost"`;
   `build.frontendDist = "../dist"`, `build.devUrl = "http://localhost:5183"`,
   `build.beforeDevCommand = "bun run dev"`, `build.beforeBuildCommand = "bun run build"`;
   `bundle.targets = ["nsis"]`. Any key the CLI rejects is corrected against
   `src-tauri/gen/schemas/desktop-schema.json` (generated on first run) — the schema, not memory,
   is the authority.
9. **Icons.** Author `assets/brand/icon-source.svg` (a minimal two-state diff glyph, no gradients),
   rasterize with the installed ImageMagick: `magick -background none assets/brand/icon-source.svg
   -resize 1024x1024 assets/brand/icon-1024.png`, then `bunx tauri icon assets/brand/icon-1024.png`.
10. **Minimal Rust app.** `Cargo.toml` with `tauri = { version = "2.11.5", features = ["tray-icon",
    "image-png"] }`, `tauri-build = "2.6.3"`, `serde`/`serde_json`, `thiserror = "2.0.20"`.
    `lib.rs`: `#[tauri::command] get_app_version()` in `commands/meta.rs`, `tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![commands::meta::get_app_version]).run(...)`.
    `main.rs`: `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` + `run()`.
11. **Capabilities.** `capabilities/default.json` — `windows: ["main"]`, `permissions:
    ["core:default", "core:window:allow-start-dragging"]` and nothing else. No `fs:`, `shell:`,
    `http:`, `sql:` permission may ever appear.
12. **Prove it.** `bun run tauri dev` → window shows the version string fetched over IPC. Then
    `bun run tauri build` → `src-tauri/target/release/bundle/nsis/*.exe` exists.
13. **B2 measurement #1.** Run the release exe, then `bun run mem`; append the baseline (backend RSS,
    WebView2 child sum, total) to `docs/PERFORMANCE.md` with the date and build hash.
14. **Commits:** (a) `chore: repo hygiene and product docs`, (b) `chore: pin rust and node toolchains`,
    (c) `feat: tauri 2 + svelte 5 application scaffold`.
## Todo (commit boundaries)

- [ ] Git identity asked + set repo-locally; `.gitignore`/`.gitattributes` committed
- [ ] `rustup` installed, MSVC selected, `rust-toolchain.toml` pinned, `docs/DECISIONS.md` D-01
- [ ] Nine §55/§68 docs written with real content; `ROADMAP.md` lists all 20 plan phases
- [ ] `bun install` clean; `bun run check` green (D-02 recorded)
- [ ] Tauri scaffold runs; IPC round-trip visible in the window
- [ ] `bun run tauri build` produces an NSIS installer
- [ ] `docs/PERFORMANCE.md` has the baseline memory row

## Success Criteria

- [ ] `rustc -vV` host is `x86_64-pc-windows-msvc`; `cargo build --manifest-path src-tauri/Cargo.toml` succeeds with no elevation
- [ ] `bun run check` exits 0; `bun run rs:lint` exits 0 with `-D warnings`
- [ ] `bun run tauri dev` opens a 1280×820 window titled "What Changed" that cannot be resized below 900×600
- [ ] `src-tauri/target/release/bundle/nsis/` contains an installer; the installed exe is named `WhatChanged.exe`
- [ ] Release run shows no console window
- [ ] `capabilities/default.json` contains exactly the two permitted permissions
- [ ] `git log --oneline` shows the three phase commits; no secret-bearing file is tracked

## Verification Gate

```bash
cd /d/Project/.WhatChage
rustc -vV && cargo --version
bun install
bun run check
bun run rs:fmt && bun run rs:lint
bun run tauri build
ls -1 src-tauri/target/release/bundle/nsis/
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1
git status --porcelain && git log --oneline
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| B1: `rustup-init` download blocked | `Invoke-WebRequest` non-200 / TLS error | `winget install --id Rustlang.Rustup -e --accept-source-agreements`; if that also fails, stop and report BLOCKED — no other phase can proceed |
| B1: linker ambiguity across 3 MSVC installs | `error: linker 'link.exe' not found` or LNK1181 | Re-run the failing cargo command through `scripts/dev-shell.cmd`; record the required env in DECISIONS D-01 and route `rs:*` scripts through it |
| D2: TS 7 breaks `svelte-check` | `bun run check` errors inside `svelte-check`/`tsc` | Pin latest 5.x, record resolved version, re-run; do not silently drop `check` from `verify` |
| Tauri config key rejected | `unknown field` / schema error from the CLI | Fix against `src-tauri/gen/schemas/desktop-schema.json`; note the corrected key in DECISIONS |
| `bunx tauri icon` rejects the PNG | CLI error about size/alpha | Re-export at exactly 1024×1024 RGBA with ImageMagick `-define png:color-type=6` |
| First `cargo build` takes very long | >10 min with no output | Expected (cold crate compile); do not kill. Record wall time in `docs/PERFORMANCE.md` |

## Security Considerations

- `.gitignore` blocks `node_modules/`, `dist/`, `src-tauri/target/`, `.env*`, `*.pfx`, `*.p12`,
  `*.key`, `*.pem`, `*.db`, `*.db-wal`, `*.db-shm`, `logs/`, `diagnostics-*.json`, `*.local`.
- CSP and `withGlobalTauri: false` are set at scaffold time so no phase inherits a permissive default.
- `capabilities/default.json` starts minimal; every later phase must justify each added permission
  in `SECURITY.md`.
- No signing key material enters the repo (P20 documents production signing via environment only).
```

### FILE: phase-02-design-system-app-shell.md

```markdown
---
phase: 02
title: "Design System & App Shell"
status: pending
priority: P1
effort: 6h
dependencies: [01]
---

## Overview

Build the visual identity and the navigation shell so every later screen inherits it: §41 design
tokens as CSS variables, Dark/Light/System, custom window chrome, sidebar, the seven routes as
real (empty-state) screens, the six-state primitive, motion tokens, and the a11y baseline. This is
the phase that decides the app does not look like a generic dashboard (§40/§70).

## Requirements

**Functional**
- Tokens: `background`, `surface-1/2/3`, `border-subtle`, `border-active`,
  `text-primary/secondary/muted`, `accent`, `success`, `warning`, `danger`, `info`,
  `shadow-sm/md/lg`, `radius-sm/md/lg/xl`, spacing scale, motion durations, easing curves — all in
  `src/styles/tokens.css`, re-exported to Tailwind via `@theme` in `src/styles/app.css`.
- Dark default (documented as the stronger MVP surface), Light complete, System follows
  `matchMedia('(prefers-color-scheme: dark)')`; theme applied via `data-theme` on `<html>`.
- Custom chrome: 36px draggable titlebar (`data-tauri-drag-region`), minimize/maximize/close
  buttons, close → hide (wired in P05), traffic-light-free Windows layout, keyboard reachable.
- Sidebar nav with seven destinations: Home, Timeline, Investigations, Checkpoints, System Map,
  Settings, Diagnostics; collapses to icon rail below 1040px; active-route indicator animates.
- Hash-based typed router in `src/lib/stores/router.svelte.ts` (no SvelteKit).
- `StateShell.svelte` renders exactly one of `loading | empty | error | content` from an
  `AsyncState<T>` union; `large-data` and `small-window` are handled by virtualization and
  container queries and are demonstrated in the dev states harness.
- Primitive layer in `src/components/ui/`: `Button, IconButton, Card, Badge, Chip, Input, Textarea,
  Switch, Tabs, Tooltip, Popover, DropdownMenu, Dialog, Drawer, Select, Separator, Skeleton,
  EmptyState, ErrorState, ProgressRing, KeyHint` — styling 100% ours; overlay/menu/select/tooltip/
  switch semantics delegate to `bits-ui` 2.19.0 headless primitives (see Architecture note).
- Dev-only route `#/dev/states` enumerating every screen × six states (§70 checkable).

**Non-functional**
- Zero remote assets: one variable font subset shipped in `src/assets/fonts/` (self-hosted,
  `font-display: swap`), no CDN, no Google Fonts (§63).
- Contrast: every token pair used for text ≥ 4.5:1 (3:1 for ≥18.66px bold), asserted by a unit test.
- All motion honours `prefers-reduced-motion: reduce` via a single `--motion-scale` variable.

## Architecture

Token flow: `tokens.css` (raw values per theme) → `@theme` (Tailwind utility names) → components use
utilities only; no component declares a raw hex. Decision D-03 recorded in `docs/DECISIONS.md`:
`bits-ui` is a dependency for the five primitives with hard focus-trap/ARIA semantics rather than
hand-rolling them (§80: one maintained dep replaces risky bespoke focus management); all visuals,
composition and naming are local, satisfying §3's "must not look like a generic shadcn dashboard".

## Related Code Files

**Create** — `src/styles/{tokens.css,typography.css,motion.css}`;
`src/lib/design/{tokens.ts,contrast.ts,theme.svelte.ts}`;
`src/lib/stores/{router.svelte.ts,ui.svelte.ts}`;
`src/app/{App.svelte,AppShell.svelte,TitleBar.svelte,Sidebar.svelte,routes.ts}`;
`src/components/ui/*.svelte` (21 files, one per primitive) + `src/components/ui/index.ts`;
`src/components/state/{StateShell.svelte,EmptyState.svelte,ErrorState.svelte,LoadingBlock.svelte}`;
`src/features/{home,timeline,investigation,checkpoints,search,settings,onboarding}/…Screen.svelte`
(seven placeholder screens with real microcopy, no lorem);
`src/features/dev/DevStatesHarness.svelte`; `src/lib/types/async-state.ts`;
`src/assets/fonts/*`; tests `src/lib/design/contrast.test.ts`,
`src/components/state/state-shell.test.ts`.
**Modify** — `src/styles/app.css` (add `@theme`), `src/main.ts`, `src/app/App.svelte`,
`index.html` (font preload, `data-theme` bootstrap script that reads a synchronous
`localStorage` mirror to avoid a theme flash), `package.json` (add bits-ui, @tanstack/svelte-virtual
is deferred to P07).
**Delete** — none.

## Implementation Steps

1. Author `tokens.css` with the dark palette (`background #0B0C0E`, `surface-1 #121417`,
   `surface-2 #171A1F`, `surface-3 #1E2228`, `border-subtle #22262D`, `border-active #323945`,
   `text-primary #E8EAED`, `text-secondary #A5ABB6`, `text-muted #6E7682`, `accent #5B8CFF`,
   `success #37B37E`, `warning #D9A32B`, `danger #E0575B`, `info #4BA3C7`) and the light palette;
   radii 6/10/14/20px; spacing 2–72 on a 4px base; durations `--dur-instant 90ms`, `--dur-fast
   140ms`, `--dur-base 200ms`, `--dur-slow 320ms`; easings `--ease-spring cubic-bezier(.2,.8,.2,1)`,
   `--ease-standard cubic-bezier(.4,0,.2,1)`; three low-alpha shadows plus a 1px inset top highlight.
2. Mirror tokens into `@theme` (`--color-surface-1: var(--surface-1)` …) so utilities like
   `bg-surface-1` exist; forbid raw hex in components via an ESLint-free grep test in
   `src/lib/design/contrast.test.ts` (scans `src/**/*.svelte` for `#[0-9a-fA-F]{6}`, allowlist = the
   two token files).
3. `theme.svelte.ts`: `$state` theme mode, `matchMedia` listener, `data-theme` writer, and a
   `localStorage` mirror for flash-free boot (authoritative value lives in Rust settings from P05).
4. `TitleBar.svelte`: drag region, window buttons via `@tauri-apps/api/window`
   (`getCurrentWindow().minimize()/toggleMaximize()/close()`), `aria-label`s, focus ring, and a
   status pill on the right ("● Active" / "Paused") bound to a store stubbed until P05.
5. `Sidebar.svelte` + `routes.ts`: seven typed routes, icon+label, `aria-current="page"`,
   `Ctrl+1..7` shortcuts, collapse behaviour via `@container` query at 1040px.
6. Build the 21 primitives. Wrap `bits-ui` for Dialog/Drawer/DropdownMenu/Select/Tooltip/Popover/
   Switch; hand-write the rest. Every interactive primitive gets: visible focus ring
   (`outline: 2px solid var(--border-active); outline-offset: 2px`), disabled styling, and a
   `data-testid`.
7. `StateShell.svelte` + `AsyncState<T>` union (`{status:'loading'} | {status:'empty'} |
   {status:'error', message, retry?} | {status:'ready', data}`); every screen must consume it.
8. Seven placeholder screens using the exact §72 voice where applicable ("Everything looks normal.",
   "What stopped working?", "Your history stays on this PC.") — no lorem ipsum anywhere.
9. Dev states harness route iterating `[screen × state]` in a grid at 1200px and again inside a
   900×600 iframe, so the §70 six-state rule is inspectable in one place.
10. Write `contrast.ts` (WCAG relative-luminance ratio) + tests asserting every documented
    text-on-surface pair passes, for both themes.
11. Commits: (a) `feat: design tokens and theming`, (b) `feat: ui primitive layer`,
    (c) `feat: application shell, router and screen placeholders`.
## Todo (commit boundaries)

- [ ] Tokens + `@theme` + theme switching + no-flash boot
- [ ] 21 primitives with focus rings and ARIA semantics
- [ ] Shell: titlebar, sidebar, router, seven screens with real microcopy
- [ ] `StateShell` + dev states harness
- [ ] Contrast + no-raw-hex tests green

## Success Criteria

- [ ] Switching Dark/Light/System repaints without reload and without a flash on cold start
- [ ] `src/lib/design/contrast.test.ts` passes for both themes and fails if a token is darkened
- [ ] No `.svelte` file outside `src/styles/` contains a raw hex colour (asserted by test)
- [ ] Keyboard-only pass reaches every nav item, titlebar button and primitive; focus is always visible
- [ ] At 900×600 nothing clips, scrolls horizontally, or overlaps the titlebar
- [ ] `#/dev/states` renders 7 screens × 4 shell states plus the 900×600 frame
- [ ] `prefers-reduced-motion: reduce` (toggled in DevTools emulation) removes all transitions

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test -- src/lib/design src/components/state
bun run tauri dev      # manual: theme cycle, Ctrl+1..7, resize to 900x600, tab-through
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `bits-ui` 2.19.0 incompatible with svelte 5.56 | `bun run check` errors in bits-ui types, or a dialog fails to trap focus | Drop the dep; hand-roll the five primitives using the native `<dialog>` element + `inert` for the rest; record in DECISIONS D-03 and add focus-trap unit tests |
| Custom chrome loses drag or snap behaviour | Window cannot be dragged or Win+Arrow snapping breaks | Keep `decorations: false` but add `data-tauri-drag-region` to the full titlebar background and a double-click maximize handler; if snapping still breaks, re-enable native decorations and record the trade-off |
| Tailwind 4 `@theme` does not emit expected utilities | `bg-surface-1` renders no style | Fall back to plain CSS variable usage (`style="background: var(--surface-1)"` via small `.surface-1` classes in `app.css`); tokens stay the single source |
| Token palette reads as generic dark dashboard | Self-review against §40's avoid-list | Iterate on depth cues (1px inset highlight, single accent, restrained glass on overlays only) before adding any new colour; never add a second accent hue |

## Security Considerations

- Fonts and icons are local files; no network request is possible under the P01 CSP.
- The `localStorage` theme mirror holds a single enum value — never settings, never user data.
- Dev states harness route is compiled out of release builds via
  `import.meta.env.DEV` guards **and** a route-table filter, so it cannot be reached in production.
```

### FILE: phase-03-redaction-privacy-chokepoint.md

```markdown
---
phase: 03
title: "Redaction & Privacy Chokepoint"
status: pending
priority: P1
effort: 4h
dependencies: [01]
---

## Overview

Build the secret-redaction engine and make it structurally unavoidable **before** any persistence
or logging code exists (§34: "redaction runs before persistence and before logging"). Two
enforcement mechanisms: a `Sanitized<T>` newtype whose only constructor lives in
`privacy::sanitize`, and a redacting `MakeWriter` through which every byte of every log line passes.

## Requirements

**Functional**
- Key-name redaction (§13/§34): normalized key (lowercase, strip `_ - . space`) matched against
  `password, passwd, pwd, secret, token, apikey, authorization, cookie, session, privatekey,
  clientsecret, accesskey, refreshtoken, bearer, credential, passphrase`, plus suffix rules
  `*key`, `*token`, `*secret`.
- Value-pattern redaction: `sk-[A-Za-z0-9_-]{16,}`, `Bearer\s+\S{12,}`, JWT
  `eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}`, `AKIA|ASIA[0-9A-Z]{16}`,
  `-----BEGIN [A-Z ]*PRIVATE KEY-----`, `gh[pousr]_[A-Za-z0-9]{20,}`, `xox[baprs]-\S+`,
  URI userinfo `(?i)\b[a-z][a-z0-9+.-]*://[^\s/@:]+:[^\s/@]+@` (redacts credentials, keeps host —
  covers `postgres://`, `mysql://`, `mongodb+srv://`, `redis://`, `amqp://`), and a gated
  high-entropy rule for `[A-Za-z0-9+/]{40,}={0,2}` that **excludes pure hex and pure digits** so
  file hashes, versions, ports and PIDs survive.
- Redacted output is the literal `[REDACTED]`. For a config diff, `changed: bool` is computed in
  memory from the pre-redaction values and the values are then dropped — never a digest, never a
  salt, never a prefix. This is what makes §13's `OPENAI_API_KEY [REDACTED] → [REDACTED]` honest.
- `Scrub` trait implemented for the event payload types' string/JSON fields; `Sanitizer::sanitize<T:
  Scrub>(T) -> Sanitized<T>`; `Sanitized<T>` exposes `into_inner()` only to `storage`.
- `scrub_line(&str) -> Cow<str>` applied by `RedactingMakeWriter` to stdout **and** the rolling file.
- Tracing setup: `EnvFilter` with a runtime-reloadable handle (§4 configurable verbosity), rolling
  daily file in the app log dir via `tracing-appender`, JSON-ish structured fields, console layer
  in dev only.
**Non-functional**
- Redaction cost < 20 µs per 1 KB payload (benchmarked); regexes compiled once via
  `std::sync::LazyLock` (no `once_cell` dependency).
- False-positive discipline: a fixture of 40 benign values (paths, versions, GUIDs, sha256 hashes,
  ports, KB numbers, driver versions, container ids) must pass through **unchanged**.

## Architecture

```
collector → NormalizedEvent ──► Sanitizer::sanitize ──► Sanitized<NormalizedEvent> ──► EventRepository
tracing macro → subscriber → RedactingMakeWriter::scrub_line → {stdout, rolling file}
```
`Sanitized<T>(T)` has a private field; the module exposes no `new`, no `From`, no `Deref`. The
repository signature `fn insert_batch(&self, events: &[Sanitized<NormalizedEvent>])` therefore makes
an unsanitized write a compile error. `diagnostics` (P18) and `commands` reuse `scrub_line` — one
implementation, three consumers (DRY).

## Related Code Files

**Create** — `src-tauri/src/privacy/mod.rs`, `src-tauri/src/privacy/sanitize.rs` (newtype +
`Sanitizer`), `src-tauri/src/privacy/scrub.rs` (`scrub_line`, `scrub_value`),
`src-tauri/src/privacy/patterns.rs`, `src-tauri/src/privacy/keys.rs`,
`src-tauri/src/privacy/entropy.rs`, `src-tauri/src/observability/mod.rs`,
`src-tauri/src/observability/tracing_setup.rs`,
`src-tauri/src/observability/redacting_writer.rs`, `src-tauri/src/error.rs` (typed
`AppError` via `thiserror`), `src-tauri/tests/redaction_leak.rs`,
`src-tauri/tests/fixtures/secrets.rs`, `src-tauri/tests/fixtures/benign_values.rs`.
**Modify** — `src-tauri/src/lib.rs` (init tracing first, before anything else),
`src-tauri/Cargo.toml` (`tracing`, `tracing-subscriber` 0.3.23 + `env-filter`,
`tracing-appender` 0.2.5, `regex`), `SECURITY.md`, `PRIVACY.md`.
**Delete** — none.

## Implementation Steps

1. `patterns.rs`: one `LazyLock<Vec<(Kind, Regex)>>`; `Kind` is an enum used for the redaction
   reason recorded in metadata (`{"redacted": {"field": "OPENAI_API_KEY", "kind": "api_key"}}`).
2. `keys.rs`: `normalize_key()` + `is_sensitive_key()`; exhaustive unit tests including
   `API_KEY`, `api-key`, `Api.Key`, `X_AUTH_TOKEN`, `db_password`, `SESSION_ID`, plus negatives
   (`API_PORT`, `KEYBOARD_LAYOUT`, `TOKENIZER_PATH`, `SESSION_TIMEOUT_MS` → must NOT redact the
   value but *may* redact on suffix rules; assert the exact intended behaviour per name and
   document any deliberate over-redaction).
3. `entropy.rs`: Shannon entropy over the candidate; require ≥ 3.6 bits/char AND mixed case or
   symbols AND not-all-hex before the generic base64 rule fires.
4. `scrub.rs`: `scrub_value(key: Option<&str>, value: &str)` (key rule first, then pattern rules)
   and `scrub_line(line: &str)` (pattern rules + `key=value` / `"key": "value"` detection).
5. `sanitize.rs`: `Scrub` trait (`fn scrub_in_place(&mut self, s: &Sanitizer)`), impls for
   `String`, `Option<String>`, `serde_json::Value` (recursive, key-aware), and a derive-free manual
   impl for the P04 event struct (added in P04 as a one-line impl).
6. `redacting_writer.rs`: `RedactingMakeWriter<W>` implementing `MakeWriter`, buffering a line and
   calling `scrub_line` before `write_all`. Wire in `tracing_setup.rs` for both sinks.
7. `tracing_setup.rs`: `init(log_dir: &Path, level: Level) -> ReloadHandle`; daily rolling file
   `logs/what-changed.log`, 7-file retention, `EnvFilter` default `info,what_changed=debug` in dev.
8. `redaction_leak.rs` (§60, mandatory): for each of the 12 secret fixtures — (a) sanitize an event
   whose `before`/`after`/`metadata` carry it and assert the serialized JSON contains `[REDACTED]`
   and does **not** contain any 8+ char substring of the secret; (b) `tracing::error!(payload = %s)`
   it, flush, read the log file back, and assert the same. Then the benign fixture must round-trip
   byte-identical.
9. Write the "What we never collect" section of `PRIVACY.md` and the secret-handling section of
   `SECURITY.md` from the implemented rules (not aspirationally).
10. Commits: (a) `feat: secret redaction engine`, (b) `feat: redaction-wrapped tracing`,
    (c) `test: secret leak suite`.

## Todo (commit boundaries)

- [ ] Key + pattern + entropy rules with unit tests
- [ ] `Sanitized<T>` newtype with single constructor
- [ ] Redacting writer + tracing init + rolling files
- [ ] §60 leak suite (DB-bound assertions added in P04) + benign no-op suite
- [ ] PRIVACY.md / SECURITY.md sections written from code

## Success Criteria

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml redaction` passes with 12 secret shapes × 2 sinks
- [ ] Attempting to construct `Sanitized` outside `privacy::sanitize` fails to compile (a
      `tests/compile_fail/` note documents the attempt; the private field is the guarantee)
- [ ] All 40 benign values survive unchanged
- [ ] Log file after the suite contains zero fixture secret substrings (grep-asserted in the test)
- [ ] Redaction bench under 20 µs / KB recorded in `docs/PERFORMANCE.md`

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run rs:fmt && bun run rs:lint
cargo test --manifest-path src-tauri/Cargo.toml redaction -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --test redaction_leak
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Over-redaction destroys useful evidence | Benign fixture fails, or a config diff shows `[REDACTED]` for `API_PORT` | Tighten the entropy gate and the suffix rules; the benign suite is the regression guard and must never be weakened to make a pattern pass |
| Under-redaction (a new secret shape) | Leak suite fails / manual review finds a raw token | Add the shape to `patterns.rs` with a test in the same commit; the leak suite is append-only |
| A future collector bypasses the chokepoint by writing SQL directly | Architecture test in P04 detects `INSERT INTO` outside the repository | Fail the build; the repository is the only writer |
| Log line splitting hides a secret across two writes | Secret appears intact in the file | `RedactingMakeWriter` buffers until `\n` before scrubbing; test asserts a split-write case |
## Security Considerations

- The engine is the §82 mitigation for "secret leakage" and part of the mitigation for "malicious
  config file" (a config crafted to smuggle a token into metadata is scrubbed on the same path).
- Redaction happens in-process before the value can reach SQLite, a log file, a Tauri event, or a
  diagnostics export — all four consumers sit downstream of `Sanitizer`/`scrub_line`.
- The leak suite is a *security control*, not a nicety: it may never be `#[ignore]`d.
- No secret-derived material (hash, prefix, length-preserving mask) is stored; only a `changed` flag.
```

### FILE: phase-04-domain-model-persistence.md

```markdown
---
phase: 04
title: "Domain Model & Persistence"
status: pending
priority: P1
effort: 7h
dependencies: [03]
---

## Overview

Define the §9 normalized domain (the single event shape every collector emits), the §10 SQLite
schema with migrations and indexes, the repository that is the *only* SQL site in the codebase, §11
retention/maintenance, and §83 database safety including corruption recovery. Writes accept only
`Sanitized<_>`.

## Requirements

**Functional**
- `SystemEvent` with exactly the §9 fields: `id, timestamp, category, event_type, entity_type,
  entity_id, entity_display_name, severity, importance, source_collector, before, after, metadata,
  related_entity_ids, user_friendly_summary, technical_summary, confidence, sensitive`.
- Enums: `Category` (15 variants exactly as §9), `EventType` (19 variants exactly as §9),
  `Severity` (`info, warning, error`), `Importance` (`informational, notable, important, critical`
  per §54), `Confidence` (`low, medium, high`) — `snake_case` serde, stored as TEXT for query
  readability.
- Tables (§10, migration `001_initial.sql`): `events, entities, entity_relationships, snapshots,
  checkpoints, health_states, collector_status, settings, watched_locations, applications,
  analysis_sessions, analysis_results, schema_migrations`. `health_states` is created now and left
  unused until the §26 Phase-2 feature — noted in `ROADMAP.md` so it is not mistaken for a stub.
- Indexes (§10): `events(timestamp)`, `events(category, timestamp)`, `events(entity_id, timestamp)`,
  `events(event_type, timestamp)`, `entity_relationships(source_id)`,
  `entity_relationships(target_id)`, plus `events(importance, timestamp)` (retention + Home counts)
  and `entities(entity_type, display_name)` (search).
- Stable entity-id grammar, documented in `ARCHITECTURE.md`: `process:<exe_path_lower_sha256_16>`,
  `port:<tcp|udp>:<addr>:<port>`, `service:<ServiceName>`, `app:<hive>:<registry_key>`,
  `driver:<device_instance_id>`, `file:<canonical_path_lower_sha256_16>`, `adapter:<guid>`,
  `container:<docker_id_12>`, `image:<repo:tag>`, `project:<canonical_path_sha256_16>`,
  `update:<KB or update_id>`, `task:<task_path>`.
- PRAGMAs on open: `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`,
  `foreign_keys=ON`, `auto_vacuum=INCREMENTAL`, `temp_store=MEMORY`, `cache_size=-8000` (8 MB).
- Retention (§11): defaults events 30d, detailed snapshots 7d, important events 90d; options
  7/30/90/custom; **checkpoint-owned snapshots are exempt** from the 7d rule (documented decision
  D-05 — otherwise "Compare with checkpoint" breaks after a week).
- Maintenance task: at startup + every 6h; deletes in ≤500-row batches with a 25 ms yield between
  batches; `PRAGMA incremental_vacuum(256)` after a delete pass; full `VACUUM` only from
  Settings → Storage on explicit user action; never on the UI thread.
- Storage usage: `page_count*page_size` + `-wal` file size + per-table row counts.
- Recovery (§83): `PRAGMA quick_check` on open; on failure enter `DbState::Corrupt` and expose
  `db_recover(action)` with `Repair` (`VACUUM INTO` a new file, swap on success), `ExportDiagnostics`,
  `ResetHistory` (rename to `events.corrupt-<epoch>.db`, create fresh). Crash-loop guard: a
  `startup.json` attempt counter; 3 unclean starts → boot with all collectors off ("Safe mode").
- Pre-migration backup: before any migration that is not purely additive, `VACUUM INTO
  backups/events-pre-v<n>.db`; keep the two most recent.

**Non-functional**
- One dedicated writer thread owning the `Connection` (SQLite single-writer discipline); readers use
  a small pool of read-only connections (`Connection::open_with_flags` READ_ONLY, WAL allows
  concurrent readers). No async SQLite, no second runtime (research §4.1).
- Every statement prepared and cached (`Connection::prepare_cached`); every batch in one transaction.
- No `unwrap()`; all errors flow through `AppError` with a `DbError` variant.

## Architecture

`storage/` layout: `mod.rs` (open/init/pragmas/state), `migrations.rs` (runner + `schema_migrations`
bookkeeping + checksums), `event_repository.rs`, `entity_repository.rs`, `settings_repository.rs`,
`snapshot_repository.rs`, `checkpoint_repository.rs` (P14), `analysis_repository.rs` (P15),
`maintenance.rs`, `recovery.rs`, `writer.rs` (owns the write connection + batching loop),
`search_index.rs`. `rusqlite_migration::Migrations` is built from `include_str!`-ed SQL files so
migrations are reviewable diffs; after `to_latest` the runner inserts a `schema_migrations` row
(version, applied_at, sha256 of the SQL) — §10 asks for the table explicitly even though the crate
tracks `user_version`.
## Related Code Files

**Create** — `src-tauri/migrations/001_initial.sql`, `.../002_search_index.sql`;
`src-tauri/src/domain/{mod.rs,event.rs,entity.rs,ids.rs,enums.rs,importance.rs,page.rs}`;
`src-tauri/src/storage/{mod.rs,migrations.rs,writer.rs,event_repository.rs,entity_repository.rs,
settings_repository.rs,snapshot_repository.rs,maintenance.rs,recovery.rs,search_index.rs}`;
`src-tauri/src/app/{state.rs,paths.rs}`; `src-tauri/tests/migrations.rs`,
`src-tauri/tests/repository.rs`, `src-tauri/tests/architecture.rs`,
`src-tauri/tests/db_recovery.rs`, `src-tauri/src/testing/mod.rs` (temp-DB helper, `cfg(test)` +
`cfg(feature = "devtools")`).
**Modify** — `src-tauri/Cargo.toml` (rusqlite 0.40.2 `features = ["bundled", "backup"]`,
rusqlite_migration 2.6.0), `src-tauri/src/lib.rs`, `src-tauri/src/privacy/sanitize.rs` (add the
`Scrub` impl for `NormalizedEvent`), `ARCHITECTURE.md`, `docs/DECISIONS.md`.
**Delete** — none.

## Implementation Steps

1. `domain/event.rs`: `NormalizedEvent` (what collectors emit — no `id`) and `SystemEvent` (what the
   repository returns — with `id`). `before`/`after`/`metadata` are `Option<serde_json::Value>`;
   `related_entity_ids: Vec<String>` stored as a JSON array column. Timestamps are
   `i64` epoch **milliseconds UTC**; the UI localizes.
2. `domain/enums.rs`: the exact §9 variant lists with `as_str()` / `FromStr` and an exhaustive
   round-trip test (a test asserting the variant count catches accidental additions/removals).
3. `domain/ids.rs`: the entity-id grammar constructors + tests (path canonicalization is
   lowercase + `\\?\` prefix stripped + trailing separator removed before hashing).
4. `001_initial.sql`: all 13 tables + the 8 indexes + `settings` seeded with defaults (theme
   `system`, retention `30/7/90`, every collector default per §50: apps, updates, network, drivers,
   services **on**; processes on; files/config on with an empty watch list; dev tools and docker
   **off**), and `collector_status` rows created lazily.
5. `storage/mod.rs`: `Db::open(app_data_dir)` → PRAGMAs → `quick_check` → migrations →
   `schema_migrations` bookkeeping; returns `Db { writer_tx, reader_pool, state }`.
6. `storage/writer.rs`: dedicated `std::thread` owning the write connection; receives
   `WriteCommand::InsertEvents(Vec<Sanitized<NormalizedEvent>>)` etc. over an `mpsc` with capacity
   4096; coalesces for up to 250 ms or 200 events, then one transaction; on channel-full, increments
   a dropped counter and emits one throttle notice per 10 s (§82 event floods).
7. `event_repository.rs`: `insert_batch(&[Sanitized<NormalizedEvent>]) -> Vec<i64>`,
   `page(EventQuery) -> Page<EventSummary>` (keyset: `WHERE (timestamp, id) < (?, ?) ORDER BY
   timestamp DESC, id DESC LIMIT ?`), `by_id`, `counts_for_range`, `prior_occurrences(signature,
   since)`, `delete_expired(...) -> usize`. Every method uses `prepare_cached`.
8. `search_index.rs`: run `PRAGMA compile_options` at open; if `ENABLE_FTS5` is present, migration
   `002` creates an FTS5 external-content table over the search columns; otherwise `002` creates
   `search_terms(term TEXT, event_id INTEGER, PRIMARY KEY(term, event_id)) WITHOUT ROWID` plus a
   shared tokenizer (lowercase, split on non-alphanumeric, keep numeric tokens for ports, min len 2,
   cap 24 terms/event). Record the selected path in `docs/DECISIONS.md` (D-04). Both paths are
   implemented behind `trait SearchIndex`.
9. `maintenance.rs` + `recovery.rs` per Requirements; `recovery.rs` gets a test that writes garbage
   over a copy of a valid DB and asserts `DbState::Corrupt` + a successful `ResetHistory`.
10. `tests/architecture.rs`: read every `.rs` under `src-tauri/src/`, assert (a) no `INSERT INTO`
    / `UPDATE ` / `DELETE FROM` string outside `src/storage/`, (b) no `.unwrap()` or `.expect(`
    outside `#[cfg(test)]` blocks and `src/testing/`, (c) no `#[cfg(windows)]` inside `src/domain/`,
    `src/analysis/`, `src/collectors/` (only `src/platform/`).
11. `tests/migrations.rs`: empty→latest; latest→latest idempotent; v1-with-data→latest preserves row
    counts; `schema_migrations` has one row per applied migration with a matching checksum; a
    corrupted checksum is reported, not silently ignored.
12. Extend the P03 leak suite with the persisted assertion: insert a secret-bearing event through the
    real repository, then read the raw DB file bytes and assert no fixture secret substring appears.
13. Commits: (a) `feat: domain event and entity model`, (b) `feat: sqlite schema and migrations`,
    (c) `feat: event repository and batched writer`, (d) `feat: retention, maintenance and recovery`,
    (e) `test: migration, repository and architecture suites`.

## Todo (commit boundaries)

- [ ] Domain types + enum round-trip tests + entity-id grammar
- [ ] Migration 001 with 13 tables, 8 indexes, seeded settings
- [ ] Writer thread + repository with keyset pagination
- [ ] Search index path chosen (D-04) and implemented
- [ ] Retention/maintenance + corruption recovery + safe mode
- [ ] Migration, repository, architecture, recovery, persisted-leak suites green

## Success Criteria

- [ ] `events.db` is created under the Tauri app data dir with WAL files present and `quick_check` ok
- [ ] All 13 §10 tables and all 8 indexes exist (asserted from `sqlite_master` in a test)
- [ ] Inserting 10,000 synthetic events in batches completes in one measured run and the recorded
      throughput is written to `docs/PERFORMANCE.md`
- [ ] Keyset pagination returns stable, non-overlapping pages under concurrent inserts (test)
- [ ] Retention deletes only expired rows, keeps `important`+ for 90d, and never removes a
      checkpoint-owned snapshot (test)
- [ ] A deliberately corrupted DB produces `DbState::Corrupt` and all three recovery actions work
- [ ] Architecture test forbids SQL outside `storage/`, `unwrap()` in production paths, and
      `#[cfg(windows)]` in core modules
- [ ] Raw DB bytes contain no fixture secret after the leak suite

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run rs:fmt && bun run rs:lint
cargo test --manifest-path src-tauri/Cargo.toml --test migrations
cargo test --manifest-path src-tauri/Cargo.toml --test repository
cargo test --manifest-path src-tauri/Cargo.toml --test architecture
cargo test --manifest-path src-tauri/Cargo.toml --test db_recovery
cargo test --manifest-path src-tauri/Cargo.toml --test redaction_leak
```
## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| FTS5 absent from the `bundled` SQLite | `PRAGMA compile_options` lacks `ENABLE_FTS5` | Use the `search_terms` inverted index (already implemented behind the same trait); record D-04 and the extra DB size in `docs/PERFORMANCE.md` |
| WAL growth on a long-running writer | `-wal` file > 64 MB in the perf run | `PRAGMA wal_autocheckpoint=1000` plus an explicit `wal_checkpoint(TRUNCATE)` after each maintenance pass |
| Write amplification from retention (§11) | Maintenance pass exceeds 500 ms of CPU or spikes disk | Batch size stays 500 with yields; switch to `incremental_vacuum(64)` per pass; never full-VACUUM automatically |
| `rusqlite_migration` 2.6 API differs from expectation | Compile error on `M::up`/`to_latest` | `cargo doc -p rusqlite_migration --open` and adapt; if the crate cannot express the pre-migration backup hook, run the backup in `migrations.rs` before calling the runner |
| Unclean shutdown leaves a hot journal | App start slow or `quick_check` warns | WAL recovery is automatic; the startup attempt counter routes a third failure into safe mode instead of a crash loop |
| Enum drift between Rust and DB text | Round-trip test fails | Enum text is the storage contract; adding a variant requires a migration note in the same commit |

## Security Considerations

- The repository is the single write path and only accepts `Sanitized<_>` — §34 is enforced by types.
- `foreign_keys=ON` plus `ON DELETE CASCADE` on `entity_relationships`, `snapshots`,
  `analysis_results` and search rows prevents orphan growth used as a disk-fill vector.
- §10's "never store arbitrary full file contents" is enforced by shape: `before`/`after` for files
  carry only `path`, `size`, `mtime`, `hash`, and for recognized config files a per-key diff whose
  values are already scrubbed.
- Recovery actions are explicit user choices; nothing deletes history automatically. `ResetHistory`
  renames rather than deletes so the corrupt file can still be inspected.
- The DB lives in the per-user app data dir (no shared/world-writable location); paths come from
  Tauri's path resolver, never from a frontend-supplied string.
```

### FILE: phase-05-typed-ipc-tray-lifecycle.md

```markdown
---
phase: 05
title: "Typed IPC, Tray & Lifecycle"
status: pending
priority: P1
effort: 6h
dependencies: [02, 04]
---

## Overview

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
  inapplicable one is `set_enabled(false)` rather than hidden (documented decision D-06).
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
   notification (P18 uses it; registered here with no capability granted until then).
8. `bin/export_bindings.rs` (or the `tauri-specta` builder in `lib.rs` under
   `#[cfg(debug_assertions)]`) writes `src/lib/tauri/bindings.ts`. Verify the exact builder API with
   `cargo doc -p tauri-specta --open` before writing the call — do not guess the entry point.
   Fallback if the 1.0 API does not fit: `ts-rs` 12.0.1 derive exports plus hand-written command
   wrappers, with `check:bindings` still enforcing parity.
9. `src/lib/tauri/client.ts`: one exported async function per command, typed from `bindings.ts`,
   wrapping `invoke` with error normalization into `AsyncState` errors and a dev-mode console trace.
10. `capabilities_allowlist.rs`: parse `capabilities/*.json` and assert no permission identifier
    starts with `fs:`, `shell:`, `http:`, `sql:`, `process:`, `updater:` (updater is added in P20
    with a note), and that the set is exactly the documented list.
11. `commands_contract.rs`: assert every function in `generate_handler!` appears in `bindings.ts`
    and vice versa (read both files), so a command can never be added without a typed wrapper.
12. B2 measurement #2: `bun run mem` against the release build with the DB open and tray active;
    append to `docs/PERFORMANCE.md`.
13. Commits: (a) `feat: dto and typed command surface`, (b) `feat: generated typescript bindings`,
    (c) `feat: tray, close-to-tray and single instance`, (d) `feat: throttled live event bus`,
    (e) `test: ipc contract and capability allowlist`.

## Todo (commit boundaries)

- [ ] DTOs + error mapping + validation helpers
- [ ] All command signatures registered; stubs listed in ROADMAP
- [ ] Bindings generated, committed, drift-gated
- [ ] Tray with the seven §7 entries; close hides; single instance focuses
- [ ] Event bus coalescing with a test
- [ ] Capability allowlist + IPC contract tests
- [ ] Memory measurement #2 recorded

## Success Criteria

- [ ] `get_settings`/`update_settings` round-trip from the UI and persist across restart
- [ ] Closing the window hides it; the tray stays; `get_collector_status` still answers
- [ ] All seven tray items work; Pause/Resume toggle enabled state; Quit is the only exit path
- [ ] Second launch focuses the first instance instead of opening a window
- [ ] `bun run check:bindings` fails when a Rust DTO field is renamed and passes after regeneration
- [ ] `capabilities_allowlist` test fails if `fs:`/`shell:`/`sql:` is added
- [ ] `bun run tauri build` still produces the NSIS bundle; §69 Phase 0 acceptance list is fully green
## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check && bun run check:bindings
bun run test
cargo test --manifest-path src-tauri/Cargo.toml --test commands_contract
cargo test --manifest-path src-tauri/Cargo.toml --test capabilities_allowlist
bun run rs:lint && bun run tauri build
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `tauri-specta` 1.0 builder API differs from the assumed shape | Compile error in the export path | Read `cargo doc -p tauri-specta`; if it still does not fit, switch to `ts-rs` 12.0.1 + hand-written wrappers, keep `check:bindings` as the parity gate, record D-07 |
| Tray icon missing at runtime | Tray absent though the app runs | Ensure the `tray-icon` + `image-png` tauri features are enabled and the icon path is a bundled resource; assert the icon file exists in a startup test |
| Hidden window is unreachable if the tray fails to build | Window hidden, no tray | Tray construction failure is fatal-by-design at startup: log, show the window, and surface a banner "Tray unavailable — closing the window will exit"; never leave an invisible process |
| Autostart plugin API/signature drift | Compile error registering the plugin | Verify with `cargo doc -p tauri-plugin-autostart`; the onboarding toggle (P18) is the only caller, so the fallback is a documented manual Startup-folder shortcut created by our own code |
| Event flood makes the UI thrash | Renderer CPU spikes with rapid inserts | The 500 ms coalescer is the contract; the frontend additionally ignores notices while scrolled away and shows a "N new changes" pill |

## Security Considerations

- §35: the command list is a closed allowlist; there is no dynamic dispatch, no `eval`, no
  path/SQL/shell passthrough. `app/validate.rs` centralizes path and range validation (§82 path
  traversal / symlink).
- Frontend is untrusted: every command re-validates, even for values the UI just received from Rust.
- `log_ui_error` is rate-limited (10/min) and its payload is scrubbed before it reaches tracing,
  so a malicious page script cannot flood or poison the log (§82 malicious frontend script, IPC abuse).
- Capabilities remain `core:default` + `core:window:allow-start-dragging` + `dialog:allow-open`
  (folder picking only) at the end of this phase; each addition is justified in `SECURITY.md`.
```

### FILE: phase-06-event-engine-queries.md

```markdown
---
phase: 06
title: "Event Engine & Queries"
status: pending
priority: P1
effort: 6h
dependencies: [05]
---

## Overview

§69 Phase 1 backend half: the query service (ranges, filters, clustering, counts), search, the
synthetic event generator that unblocks all UI work, the §85 developer simulation panel backend, and
the 10k-scale responsiveness proof. No OS APIs yet — deliberately.

## Requirements

**Functional**
- `EventQuery { range: TimeRange, categories: Vec<Category>, importance_min: Option<Importance>,
  entity_id: Option<String>, cursor: Option<Cursor>, limit: u16 (≤200) }`;
  `TimeRange = Today | Yesterday | Last7Days | SinceLastRestart | Custom{from,to}` — `SinceLastRestart`
  answers a §1 question and resolves from the recorded boot event (falls back to
  `GetTickCount64`-derived boot time once P13 lands; until then, midnight).
- `get_dashboard_summary` → `{ today: {total, notable, important, critical}, status_line, top_important:
  Vec<EventSummary>, monitoring: MonitoringState, last_checkpoint: Option<CheckpointRef> }`;
  `status_line` is computed, not hardcoded: "Everything looks normal." when
  `important + critical == 0`, else "N recent changes may need attention." (§72 voice, exact strings).
- Clustering (§44): after fetching a keyset page of ≤200 rows, group runs sharing
  `(category, coarse entity key, 60 s bucket)` with count > 4 into
  `TimelineItem::Cluster { key, label, count, first_ts, last_ts, highest_importance }`; label from a
  template table ("12 Docker-related changes", "8 file changes in `api/`"). Expanding a cluster
  issues a scoped `get_events` with the cluster key + window.
- `search_events` over the P04 index: application names, event descriptions, file names, ports,
  services, drivers, container names, checkpoint labels (§52). Numeric queries (`25565`) match port
  entities and metadata port fields.
- `get_event_details` → human explanation, timestamp, importance, confidence, before/after, related
  events (by `related_entity_ids` + ±5 min same-entity), affected entities, source collector,
  technical metadata, raw identifiers, and `prior_occurrences` ("This happened 2 times before" — §1).
- Synthetic generator (`src-tauri/src/testing/synthetic.rs`, compiled in release only behind the
  `devtools` cargo feature): deterministic seeded streams producing realistic events across all 15
  categories, with a `--profile {day, week, burst, 10k, 100k, 1m}` selector, used by both the dev
  panel and the perf tests. Clearly labelled `source_collector = "synthetic"` so no synthetic row can
  ever be mistaken for a real one (§87).
- `simulate_event(kind)` (§85) supports: driver update, port change, process crash, DNS change,
  Docker restart, config diff — each producing a correctly shaped normalized event through the real
  write path.
**Non-functional**
- No query may return unbounded rows; `limit` is clamped in Rust regardless of the request.
- p95 budgets recorded at 10k: page ≤ 10 ms, summary ≤ 15 ms, search ≤ 40 ms (measured, written to
  `docs/PERFORMANCE.md`; 100k/1M gates are P19's job).
- The generator writes through the real `Sanitized` path — no test-only shortcut into SQLite.

## Architecture

`services/` is introduced here: `query_service.rs` (ranges, filters, clustering, details),
`search_service.rs`, `summary_service.rs`, plus `importance.rs` in `domain/` (the §54 classifier used
by the generator now and by collectors later — one table, one behaviour). Clustering lives in Rust
over a fetched page (deterministic page size, variable item count) rather than in SQL: simpler,
index-friendly, and it keeps §44's "never render 500 tiny events" promise without a second query
planner.

## Related Code Files

**Create** — `src-tauri/src/services/{mod.rs,query_service.rs,search_service.rs,summary_service.rs,
cluster.rs}`; `src-tauri/src/domain/importance.rs`; `src-tauri/src/testing/synthetic.rs`;
`src-tauri/src/dto/{query_dto.rs,summary_dto.rs}`; `src-tauri/tests/query_service.rs`;
`src-tauri/tests/cluster.rs`; `src-tauri/tests/importance_calm.rs`;
`src-tauri/tests/perf_scale.rs` (10k active now, 100k/1M `#[ignore]`d until P19).
**Modify** — `src-tauri/src/commands/{events.rs,dev.rs}` (real bodies), `src-tauri/src/domain/mod.rs`,
`src-tauri/src/storage/event_repository.rs` (cluster-scoped and search queries),
`src-tauri/Cargo.toml` (`[features] devtools = []`), `src/lib/tauri/bindings.ts` (regenerate),
`ROADMAP.md` (remove the stubs this phase implements), `ANALYSIS.md` (importance table).
**Delete** — none.

## Implementation Steps

1. `domain/importance.rs`: `classify(&NormalizedEvent, &ClassifierCtx) -> Importance` from an explicit
   `(category, event_type) → base` table plus escalations: automatic-start service stopped →
   `important`; GPU/audio/network/storage/input driver `updated` → `important`; proxy or DNS change →
   `important`; a port that a checkpoint recorded as open closing → `important`; Windows update
   installed → `notable`, update **failed** → `important`; unexpected shutdown → `critical`;
   application `updated` (Chrome, Spotify, Discord …) → `informational` **always** (§53/§54).
2. `tests/importance_calm.rs`: over a 2,000-event realistic synthetic day, assert ≥ 80% are
   `informational`, ≤ 2% `critical`, and that no `application/updated` row is above `informational`.
3. `query_service.rs`: range resolution (local-day boundaries via the OS timezone, stored ms are UTC),
   filter mapping to the §10 indexes, keyset cursor encode/decode (opaque base64 of `(ts,id)`),
   `EXPLAIN QUERY PLAN` assertions in tests that each filter path uses an index (no `SCAN events`).
4. `cluster.rs` + tests: cluster/no-cluster boundaries, label templates, stable ordering, and
   expansion returning exactly the clustered members.
5. `search_service.rs`: tokenize the query the same way the index tokenizes documents (shared
   function), rank by `importance desc, timestamp desc`, cap 100 results, group by category for the
   palette UI.
6. `summary_service.rs`: today's counts via `events(importance, timestamp)`, status line strings
   exactly as §72, top 3 important events.
7. `synthetic.rs`: seeded `SmallRng`-free deterministic LCG (no new dependency), profiles as
   specified, realistic names (`nvlddmkm`, `Audiosrv`, `Discord.exe`, `api.exe`, `docker-compose.yml`,
   `1.1.1.1 → 192.168.1.1`), and a `--relations` flag that also writes entity relationships so P15 has
   graph data to traverse.
8. Wire `simulate_event` to the generator's single-event constructors; gate the command with
   `#[cfg(any(debug_assertions, feature = "devtools"))]` **and** a `developer_mode` setting check.
9. `perf_scale.rs`: `seed(profile)` then assert the 10k budgets; print measured numbers; write them to
   `docs/PERFORMANCE.md` manually in the same commit.
10. Commits: (a) `feat: importance classifier`, (b) `feat: query service with keyset paging and
    clustering`, (c) `feat: search service`, (d) `feat: synthetic event generator and simulation
    commands`, (e) `test: query, cluster, calmness and 10k performance`.

## Todo (commit boundaries)

- [ ] Importance table + calmness test
- [ ] Query service + cursor + index-plan assertions
- [ ] Clustering + labels + expansion
- [ ] Search service + shared tokenizer
- [ ] Dashboard summary with exact §72 strings
- [ ] Synthetic generator + `simulate_event`
- [ ] 10k budgets measured and recorded

## Success Criteria

- [ ] 10,000 synthetic events: page/summary/search all inside the recorded budgets, numbers in `docs/PERFORMANCE.md`
- [ ] `EXPLAIN QUERY PLAN` shows an index for every filter combination the UI can produce (test)
- [ ] Clustering collapses a 40-event Docker burst into one item and expands back to exactly 40
- [ ] Search finds `Discord`, `25565`, `NVIDIA`, `nginx`, `DNS` in the synthetic corpus
- [ ] `get_event_details` returns related events and a correct `prior_occurrences` count
- [ ] Every synthetic row is labelled `source_collector = "synthetic"`; a test asserts no other
      collector id can be produced by the generator
- [ ] `simulate_event` is unreachable in a release build without the `devtools` feature *and*
      `developer_mode` enabled (test asserts the double gate)

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run rs:fmt && bun run rs:lint
cargo test --manifest-path src-tauri/Cargo.toml --test query_service --test cluster --test importance_calm
cargo test --manifest-path src-tauri/Cargo.toml --release --test perf_scale -- --nocapture
bun run check:bindings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Rust-side clustering breaks page continuity | Cursor paging skips or repeats items around a cluster edge | Cursor is always the last *row* consumed, never the last item; a dedicated test pages through a clustered corpus and asserts the union equals the raw set |
| Search too slow without FTS5 | Search > 40 ms at 10k | The inverted index is already the fallback; if it is the chosen path and still slow, cap candidate terms and add `search_terms(term)` prefix range scans instead of `LIKE` |
| Synthetic data quietly shipping to users | A `synthetic` row appears in a normal run | Double gate (cargo feature + setting) plus a startup assertion that release builds without `devtools` cannot register the generator; a "Developer mode" badge is visible in the titlebar whenever it is on |
| Local-day boundaries wrong across DST | "Today" count jumps at 23:00 | Resolve day boundaries with the OS local offset at query time (not a cached offset) and add a DST-boundary unit test with fixed offsets |
## Security Considerations

- Cursors are opaque but not trusted: decode failures return a typed error and reset to page one; a
  forged cursor cannot express a different table or predicate.
- `limit` and `range` clamped server-side; a hostile frontend cannot request 1M rows (§82 IPC abuse).
- Search input is parameterized (never string-concatenated into SQL) and length-capped at 128 chars.
- The generator is a development tool: §87 forbids presenting it as real monitoring, so its output is
  labelled at the data layer, not just in the UI.
```

### FILE: phase-07-timeline-search-ui.md

```markdown
---
phase: 07
title: "Timeline & Search UI"
status: pending
priority: P1
effort: 8h
dependencies: [06]
---

## Overview

§69 Phase 1 frontend half: the polished Home and Timeline screens plus the event details drawer and
global search — built on synthetic data so the UI is finished and fast before any collector exists.
This is where the product's voice becomes visible (§43, §44, §46, §72).

## Requirements

**Functional**
- Home (§43/§72): status pill ("● Active"/"Paused"), headline "Everything looks normal." or "N recent
  changes may need attention.", prompt "What stopped working?", input placeholder "Example: Discord
  stopped detecting my microphone this morning", primary button "Investigate", today's counts
  ("37 changes · 2 important · 0 critical"), "Important changes" list, privacy line "Your history
  stays on this PC.", plus quick chips: Today / Since last restart / Yesterday.
- Timeline (§44): range tabs Today / Yesterday / Last 7 days / Custom; filter chips All / Apps /
  Windows / Network / Drivers / Files / Development / Docker / Important only; grouped by local hour
  with sticky headers; each row shows time, human title, `before → after`, an optional relative note
  ("34 minutes before first game crash" when a correlation exists), and "View details".
- Clusters render as one row with a count and "[Expand]" that loads members inline.
- Virtualized list: `@tanstack/svelte-virtual` 3.13.36 with fixed 72 px rows; **fallback** to a
  hand-rolled windowed list (`src/components/timeline/WindowedList.svelte`) if the runes spike fails
  (research §5.10). Never hold more than 3 pages (600 rows) in memory; older pages are dropped and
  refetched.
- Live updates: on `system-event-added`, if the user is at scroll-top on a live range, prepend with a
  140 ms enter animation; otherwise show a "N new changes" pill that scrolls to top on click. Never
  re-render the whole list (§57).
- Details drawer (§46): human explanation, timestamp (absolute + relative), importance chip,
  confidence, before/after table, related events (clickable), affected entities, "Detected by
  <collector>", and a collapsed "Show technical details" section with raw metadata and identifiers
  (§51 progressive disclosure). Keyboard: `Esc` closes, focus returns to the invoking row.
- Global search (§52): `Ctrl+K` palette, debounced 120 ms, grouped results (Applications, Files,
  Ports, Services, Drivers, Docker, Checkpoints, Events), arrow-key navigation, `Enter` opens the
  details drawer or the timeline filtered to that entity.
- Six states on Home, Timeline, and Search (§70): skeleton loading; empty ("Nothing has changed yet.
  What Changed started recording at 1:20 AM." / "No changes match these filters."); error with Retry;
  success; large-data (virtualized + "Showing 200 of 128,431"); small-window at 900×600 (filters
  collapse into a dropdown, drawer becomes full-width).

**Non-functional**
- 60 FPS scroll at 100k rows (measured in P19); no layout thrash (fixed row height, `content-visibility`
  on group headers).
- No business logic in components: formatting lives in `src/lib/formatting/`, data access in stores.
- All interactive rows are real `<button>`/`<a>` elements with `aria-expanded`/`aria-controls`.

## Architecture

Stores: `timeline.svelte.ts` (query state, pages, cursor, live pill), `eventDetails.svelte.ts`,
`search.svelte.ts` — each a rune-based class exposing `AsyncState`. Components:
`components/timeline/{TimelineList,TimelineRow,ClusterRow,GroupHeader,RangeTabs,FilterChips,
LivePill,WindowedList}.svelte`, `components/event/{DetailsDrawer,BeforeAfterTable,RelatedEvents,
TechnicalDetails}.svelte`, `components/search/{SearchPalette,ResultGroup}.svelte`.
Formatting: `formatting/{time.ts,bytes.ts,eventCopy.ts,relative.ts}` — `eventCopy.ts` is the only
place UI strings for event titles are assembled, from the Rust-provided
`user_friendly_summary`/`technical_summary` (never re-derived in TS).
## Related Code Files

**Create** — `src/features/home/{HomeScreen.svelte,StatusHeadline.svelte,TodayCounts.svelte,
ImportantChanges.svelte,ProblemInput.svelte}`;
`src/features/timeline/{TimelineScreen.svelte,TimelineToolbar.svelte,CustomRangePicker.svelte}`;
`src/components/timeline/{TimelineList,TimelineRow,ClusterRow,GroupHeader,RangeTabs,FilterChips,
LivePill,WindowedList}.svelte`; `src/components/event/{DetailsDrawer,BeforeAfterTable,
RelatedEvents,TechnicalDetails,ImportanceChip,ConfidenceBadge}.svelte`;
`src/components/search/{SearchPalette,ResultGroup,ResultRow}.svelte`;
`src/lib/stores/{timeline.svelte.ts,eventDetails.svelte.ts,search.svelte.ts,liveEvents.svelte.ts}`;
`src/lib/formatting/{time.ts,bytes.ts,eventCopy.ts,relative.ts}`; tests
`src/lib/formatting/time.test.ts`, `src/lib/stores/timeline.test.ts`,
`src/components/timeline/timeline-list.test.ts`, `src/components/event/details-drawer.test.ts`,
`src/components/search/search-palette.test.ts`.
**Modify** — `src/features/home/HomeScreen.svelte` and
`src/features/timeline/TimelineScreen.svelte` (replace P02 placeholders), `src/app/AppShell.svelte`
(mount palette + drawer portals), `src/features/dev/DevStatesHarness.svelte` (wire real screens),
`package.json` (`@tanstack/svelte-virtual`), `PRODUCT.md` (screen copy inventory).
**Delete** — none.

## Implementation Steps

1. Spike `@tanstack/svelte-virtual` with 100k synthetic rows in a scratch route; if runes
   incompatibility appears (research §5.10), implement `WindowedList.svelte` (fixed row height,
   `scrollTop`-derived window + 8-row overscan) and record D-08. Do the spike **first** — the rest of
   the timeline depends on the choice.
2. `formatting/time.ts`: absolute (`10:42 AM`), relative (`34 minutes before …`), day headers
   ("Today", "Yesterday", "Friday, Aug 22"), and the §44 duration phrasing; unit-tested with fixed
   timestamps.
3. `timeline.svelte.ts`: query state → `getEvents` → page list; `loadMore` on scroll threshold;
   `applyFilter`/`setRange` reset the cursor; page eviction beyond 3 pages; `liveEvents` integration.
4. Build `TimelineRow` to the §44 shape exactly, `ClusterRow` with "[Expand]", `GroupHeader` sticky
   per hour.
5. `DetailsDrawer`: right-side sheet (480 px, full-width below 1040 px), spring transition
   (`--ease-spring`, `--dur-base`), focus trap via the P02 primitive, `Esc` close, technical section
   collapsed by default with a persisted per-user preference (§51).
6. Home screen assembled from `get_dashboard_summary`; the problem input stores its draft and routes
   to Investigations (P15) — until then the button is disabled with the tooltip "Investigation arrives
   in the next phase" **and** the stub is listed in `ROADMAP.md` (§87).
7. `SearchPalette`: `Ctrl+K` global handler, `role="dialog"` + `aria-modal`, `role="listbox"` results,
   `aria-activedescendant` navigation, empty state "No matches. Try an app name, a port, or a file
   name."
8. Six-state pass: add each screen's four shell states to the dev harness, verify the 900×600 frame,
   and add a large-data state fed by the 100k profile.
9. Vitest + Testing Library tests: filter/range transitions, cluster expand, live-pill behaviour when
   scrolled, drawer focus return, palette keyboard nav, and empty/error rendering.
10. Commits: (a) `feat: virtualization decision and windowed list`, (b) `feat: timeline screen`,
    (c) `feat: event details drawer`, (d) `feat: home screen`, (e) `feat: global search palette`,
    (f) `test: timeline, drawer and palette suites`.

## Todo (commit boundaries)

- [ ] Virtualization spike resolved (D-08)
- [ ] Timeline list, rows, clusters, group headers, toolbar
- [ ] Details drawer with progressive disclosure
- [ ] Home screen with exact §72 copy
- [ ] Search palette
- [ ] Six states wired into the dev harness + component tests

## Success Criteria

- [ ] Scrolling a 100k synthetic corpus stays smooth with ≤ 600 rows in memory (asserted by a store test)
- [ ] Filters and ranges combine correctly and the URL hash reflects state (deep-linkable)
- [ ] Cluster expand shows exactly the clustered members and collapses back
- [ ] While scrolled down, new events never move the viewport; the pill appears instead
- [ ] Drawer shows every §46 field for a synthetic event with `before`/`after`
- [ ] `Ctrl+K` → typing `25565` finds the port entity; `Enter` filters the timeline to it
- [ ] All six states verifiable at `#/dev/states`, including 900×600
- [ ] `bun run test` green; `bun run check` green

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test -- src/lib/formatting src/lib/stores src/components
bun run tauri dev   # manual: simulate_event x20 while scrolled down -> pill only; Ctrl+K; 900x600
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `@tanstack/svelte-virtual` incompatible with runes | Spike errors or rows fail to recycle | Ship `WindowedList.svelte` (fixed-height rows make this ~120 lines); record D-08; drop the dependency entirely |
| Variable-height rows break virtualization | Rows with long paths wrap and misalign | Row height is fixed at 72 px; long values truncate with a tooltip and full value in the drawer — never wrap in the list |
| Live prepend causes scroll jump | Viewport moves on insert | Prepend only at `scrollTop < 4px`; otherwise pill. Store test covers both branches |
| Drawer + palette both open trap focus incorrectly | Tab escapes to the background | Single portal stack with an ordered focus-trap registry; palette closes the drawer's trap rather than nesting |
| Copy drifts from §72/§44 wording | Manual diff against the packet strings | The exact strings live in `src/lib/formatting/eventCopy.ts` + `src/features/**/copy.ts` and are asserted by a snapshot test, so a wording change is a visible diff |

## Security Considerations

- The drawer renders all values as text nodes; no `{@html}` anywhere in the codebase (asserted by a
  grep test in `src/components/**` — protects against a malicious config/Docker label reaching the DOM).
- Raw identifiers shown in technical details are already scrubbed by P03; the UI never re-fetches raw
  file contents (there is no command that could).
- Deep-link hash state is validated against the typed query schema before use.
---

### FILE: phase-08-collector-runtime.md

```yaml
---
phase: 8
title: "Collector runtime: trait, supervisor, scheduler, failure isolation, settings"
status: pending
priority: P1
effort: 5h
dependencies: [5]
---
```

## Overview

The chassis every collector plugs into: the `Collector` trait (§5), a supervisor that runs each
collector independently so one failure cannot stop the others (§6, §33), a cost-aware scheduler, the
normalizer that turns raw observations into `SystemEvent`s through the P03 sanitizer, and the §49
settings surface driven from collector self-description. No OS-specific code lands here — the phase
ships with two fake collectors so the runtime is fully tested before any Win32 call exists.

## Requirements

**Functional**
- `Collector` trait: `id() -> &'static str`, `descriptor() -> CollectorDescriptor`,
  `start(&mut self, ctx: CollectorContext) -> Result<()>`, `stop(&mut self)`,
  `snapshot(&mut self) -> Result<Vec<RawObservation>>` (§5).
- `CollectorDescriptor { id, display_name, what_it_watches, why_it_matters, cost_hint: Cheap|Moderate|Expensive, cadence: Poll(Duration)|Notify|NotifyPlusSweep, requires_admin: false }` — the single source for §49 toggle copy ("what it watches, why it matters").
- Supervisor: spawns one OS thread per collector (not async — the Win32 calls are blocking), owns a
  `Vec<CollectorHandle>`, and exposes `list_collector_status()` → `CollectorStatus { id, enabled,
  state: Running|Paused|Failed{reason}|Disabled, last_run_at, last_duration_ms, events_emitted,
  consecutive_failures }`.
- Failure isolation: a panic in a collector thread is caught (`catch_unwind` around the loop body),
  recorded, and the collector is restarted with exponential backoff 5 s → 30 s → 5 min, capped; after
  5 consecutive failures the collector enters `Failed` and reports through §33's degraded-mode
  surface. The app keeps running and the other collectors are untouched.
- Global pause: `pause_monitoring(minutes)` / `resume_monitoring()` set a shared `MonitorState`
  (`AtomicU8` + `Instant` deadline); collectors check it at the top of each cycle and skip work
  (threads stay alive so restart is instant). Auto-resume when the deadline passes; the tray label
  reflects the remaining time.
- Normalizer: `RawObservation { collector_id, kind, entity, before, after, occurred_at, details }` →
  `SystemEvent`; assigns `event_type`/`category`/`importance` (P06 classifier), builds the entity id
  by the P04 grammar, and passes every text field through `Sanitizer::sanitize` before the event is
  handed to the P04 batched writer. The writer accepts only `Sanitized<SystemEvent>`, so this is
  compiler-enforced, not review-enforced.
- Settings (§49): `get_settings`/`update_settings` persist per-collector `enabled` flags and the
  monitoring profile (`Balanced` default, `Minimal`, `Thorough`) to the `settings` table; toggling a
  collector off stops its thread within one cycle and toggling on starts it without an app restart.
- Scheduler honours `cost_hint`: `Expensive` collectors never run on the fast tick, are staggered so
  two never sweep in the same 30 s window, and are skipped entirely while a user-visible query is in
  flight (a shared `QueryBusy` gate) so the UI never waits behind a 2 s driver sweep.

**Non-functional**
- Runtime module is `#![cfg(any())]`-free and compiles/tests on any host: no `windows` crate import
  in `collectors/mod.rs`, `collectors/runtime.rs`, or `collectors/normalize.rs` (enforced by
  `tests/architecture.rs` from P04).
- Idle CPU contribution of the runtime itself (no collectors registered) ≈ 0: threads block on
  `Condvar`/`recv_timeout`, never spin.

## Architecture

`collectors/mod.rs` (trait + descriptor + `RawObservation`), `collectors/runtime.rs` (supervisor,
threads, backoff, pause gate), `collectors/schedule.rs` (tick wheel, cost staggering, `QueryBusy`),
`collectors/normalize.rs` (raw → `SystemEvent` → `Sanitized`), `collectors/registry.rs` (the one place
that lists concrete collectors; `#[cfg(windows)]` only here and in `platform/windows/`).
Provider-trait pattern: each future collector takes a `&dyn XProvider` (e.g. `ProcessProvider`) whose
Windows implementation lives in `platform/windows/`; tests inject a scripted fake. That is what makes
collectors 09–13 unit-testable on any machine, and it is established here with
`collectors/testing.rs` (`ScriptedCollector`, `FlakyCollector`, `PanickingCollector` behind
`#[cfg(test)]`).

Data flow: `Collector::snapshot` → `Vec<RawObservation>` → normalizer → `Sanitized<SystemEvent>` →
`storage::writer` mpsc → coalesced transaction → `EventBatchNotice` to the frontend (P05).

## Related Code Files

**Create** — `src-tauri/src/collectors/{mod.rs,runtime.rs,schedule.rs,normalize.rs,registry.rs,testing.rs}`;
`src-tauri/src/domain/collector.rs` (descriptor/status types, specta-exported);
`src-tauri/src/commands/collectors.rs` (`list_collector_status`, `set_collector_enabled`,
`pause_monitoring`, `resume_monitoring`, `get_monitor_state`);
`src-tauri/tests/collector_runtime.rs`, `src-tauri/tests/collector_isolation.rs`,
`src-tauri/tests/normalize.rs`; `src/features/settings/CollectorList.svelte`,
`src/lib/stores/collectors.svelte.ts`.
**Modify** — `src-tauri/src/lib.rs` (build supervisor in `setup`, store in managed state, stop on
exit), `src-tauri/src/commands/mod.rs` + the specta builder (5 new commands),
`src-tauri/src/domain/settings.rs` (collector flags, profile), `src/features/settings/SettingsScreen.svelte`,
`ROADMAP.md`, `docs/system-architecture.md` (collector runtime section).
**Delete** — none.

## Implementation Steps

1. Define `Collector`, `CollectorDescriptor`, `RawObservation`, `CollectorContext { monitor: MonitorHandle, emit: EmitHandle, cancel: CancelToken }` in `collectors/mod.rs`.
2. `runtime.rs`: `CollectorSupervisor::new(registry, emit)`, `spawn_all()`, per-collector loop
   (`check pause → check cancel → snapshot → emit → sleep by cadence`), `catch_unwind` + backoff,
   `stop_all()` joining with a 2 s timeout then detaching.
3. `schedule.rs`: cadence wheel driven by `Instant`; stagger `Expensive` collectors by hashing their
   id into a 0–29 s offset; `QueryBusy` is an `AtomicUsize` incremented by the P05 command middleware.
4. `normalize.rs`: entity-id construction per P04 grammar, `before`/`after` JSON assembly, category
   mapping, then `Sanitizer::sanitize`. Every field that can contain user data goes through it —
   including `entity_name` and `details`.
5. Wire commands, then the settings UI: one row per collector with name, "what it watches", "why it
   matters", cost badge, live state, last-run time, and a toggle.
6. Tests: `collector_runtime.rs` (start/stop/pause/resume/enable/disable, cadence honoured within
   ±20%), `collector_isolation.rs` (a panicking collector is restarted with backoff, a permanently
   failing one reaches `Failed` after 5 tries, and a healthy neighbour keeps emitting throughout),
   `normalize.rs` (entity grammar + a secret-bearing observation is redacted before reaching the writer).
7. Commits: (a) `feat: collector trait and descriptor`, (b) `feat: collector supervisor with failure isolation`,
   (c) `feat: cost-aware scheduler and pause gate`, (d) `feat: observation normalizer`,
   (e) `feat: collector settings surface`, (f) `test: runtime, isolation and normalizer suites`.

## Todo (commit boundaries)

- [ ] Trait, descriptor, context, raw observation
- [ ] Supervisor with panic isolation and backoff
- [ ] Scheduler with cost staggering and query gate
- [ ] Normalizer routed through the sanitizer
- [ ] Settings surface from descriptors
- [ ] Three test suites green

## Success Criteria

- [ ] A `PanickingCollector` panics 3 times; the test asserts backoff timings, `Failed` state after 5,
      and that a neighbour `ScriptedCollector` emitted every expected event during the whole episode
- [ ] `set_collector_enabled(id, false)` stops the thread within one cadence and `true` restarts it —
      no app restart, asserted by event counts before/after
- [ ] `pause_monitoring(15)` → zero events emitted; auto-resume fires at the deadline
- [ ] A `RawObservation` containing `sk-live-xxxx` is stored redacted (queries the DB in the test)
- [ ] `list_collector_status()` returns one row per registered collector with `requires_admin: false`
      for all of them
- [ ] `tests/architecture.rs` still passes (no Win32 import outside `platform/`)
- [ ] Idle CPU with two fake collectors at 1 s cadence < 0.3% over a 60 s measurement

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test collector_runtime --test collector_isolation --test normalize --test architecture
cargo clippy --all-targets -- -D warnings
cd /d/Project/.WhatChage && bun run check:bindings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `catch_unwind` cannot cover a collector holding a `&mut self` across the boundary | Compile error on `UnwindSafe` | Wrap the collector in `AssertUnwindSafe` and treat post-panic state as poisoned: drop and re-`new()` the collector from the registry factory rather than resuming it |
| Thread-per-collector costs too much memory | RSS grows > 2 MB per collector | Stack size set explicitly to 256 KB per thread at spawn; if still heavy, merge all `Cheap` poll collectors onto one shared tick thread (the trait already permits it — only `registry.rs` changes) |
| Blocking `snapshot()` overruns its cadence | `last_duration_ms > cadence` in status | Scheduler skips the next tick instead of queueing; a `Moderate`/`Expensive` collector that overruns 3× is downgraded to `NotifyPlusSweep` and the overrun is logged once |
| Pause gate checked too rarely | Events arrive after pause | Cancel token + pause flag are also checked inside long sweeps at each provider-page boundary; the isolation test asserts ≤ 1 s to quiesce |
| Settings write races the supervisor | Toggle appears to do nothing | Settings mutate through the supervisor's command channel only; the DB row is written by the supervisor after the state change succeeds |

## Security Considerations

- The normalizer is the only path into storage, and it takes `RawObservation` and returns
  `Sanitized<SystemEvent>` — a collector cannot bypass redaction (no `pub fn` accepts an unsanitized
  event downstream).
- `CollectorContext` gives collectors no filesystem, network, or shell handle; capability is passed as
  narrow provider traits.
- No collector requests elevation, and `requires_admin` is hardcoded `false` for the MVP set; a future
  collector that needs it must surface the fact in its descriptor before the toggle can be shown.

---

### FILE: phase-09-process-port-collectors.md

```yaml
---
phase: 9
title: "Process and network-listener collectors (diff-polled, path-degrading)"
status: pending
priority: P1
effort: 6h
dependencies: [8]
---
```

## Overview

First real data. Process start/stop by 1 s diff-polling (§11) and TCP/UDP listener open/close by 2 s
diff-polling (§15), both non-elevated. Measured costs: process snapshot 6.2 ms warm, listener
enumeration is in the same cheap class — so both are `Cheap`/`Poll`. This phase also establishes the
`process → listens_on → port` relationship rows that the P15 analysis engine walks, and produces the
§15 example output verbatim.

## Requirements

**Functional**
- Process collector: `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)` +
  `Process32FirstW`/`Process32NextW` for pid, ppid, exe name; `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`
  + `QueryFullProcessImageNameW` for the full path. Diff two consecutive snapshots keyed by
  `(pid, creation_time)` — never pid alone (pid reuse would forge a stop+start pair). Creation time via
  `GetProcessTimes` when the handle opens; when it does not, key on `(pid, exe_name)` and mark the
  event `identity_confidence: Low`.
- Measured reality: ~46% of processes deny `QueryFullProcessImageNameW` non-elevated. Those events are
  emitted with `entity_name` = image name, `path: null`, and `details.path_unavailable = true`; the UI
  shows the name without a path and never renders "unknown". This is normal operation, not an error —
  no log line per occurrence (one aggregate count per hour at DEBUG).
- Events: `process/started`, `process/stopped`. `process/stopped` carries `exit_kind:
  Normal|Unexpected|Unknown`; `Unexpected` is only ever set when the Event Log collector (P13) has a
  corroborating Application Error / 1000 / 1026 entry for the same image within ±5 s — otherwise
  `Unknown`. **B3 is documented in the UI**: the collector descriptor's `what_it_watches` says
  "Programs starting and stopping (checked every second — programs that live less than a second may be
  missed)".
- Noise control (§53): a process that starts and stops more than 5 times in 60 s is collapsed into one
  `process/churn` event with a count; browser/helper subprocess trees (parent is a known browser and
  the image matches the parent) are recorded at `importance: informational` and excluded from
  notifications entirely.
- Listener collector: `GetExtendedTcpTable(TCP_TABLE_OWNER_PID_LISTENER)` and
  `GetExtendedUdpTable(UDP_TABLE_OWNER_PID)` for AF_INET and AF_INET6; diff on
  `(protocol, local_addr, local_port)`. Events `port/opened`, `port/closed` with `owning_process`
  resolved from the live process map (not a second enumeration).
- Relationships: on `port/opened`, insert `relationships` rows `process:<id> --listens_on--> port:<id>`
  and the reverse `port:<id> --opened_by--> process:<id>`, both with the event id, so P15's graph walk
  finds them from either direction. §15's output shape — "Port 4000 opened by: api.exe" and "These
  events occurred 300ms apart" — is produced from the relationship row plus the two timestamps.
- Both collectors expose a `snapshot()` that is pure w.r.t. the injected provider so tests drive
  scripted process/port tables.

**Non-functional**
- Process cycle ≤ 15 ms p95 on this machine (measured baseline 6.2 ms warm); listener cycle ≤ 15 ms.
- Steady-state allocation: snapshots reuse two pre-sized `Vec`s + a `HashMap` swap (no per-tick
  allocation of the whole table where avoidable).
- Zero events when nothing changed (diff-empty is the common case) — verified by a 30 s idle test.

## Architecture

`collectors/process.rs` + `collectors/listeners.rs` hold the diff logic and take
`&dyn ProcessProvider` / `&dyn SocketProvider`. `platform/windows/process.rs` and
`platform/windows/sockets.rs` hold every Win32 call (`windows` 0.62 features
`Win32_System_Diagnostics_ToolHelp`, `Win32_System_Threading`, `Win32_NetworkManagement_IpHelper`,
`Win32_Networking_WinSock`). A shared `ProcessMap` (`Arc<RwLock<HashMap<Pid, ProcessInfo>>>`) is owned
by the process collector and read by the listener collector for owner resolution — one enumeration
serves both.

## Related Code Files

**Create** — `src-tauri/src/collectors/{process.rs,listeners.rs}`;
`src-tauri/src/platform/windows/{process.rs,sockets.rs}`;
`src-tauri/src/platform/{mod.rs,traits.rs}` (if not already created in P08);
`src-tauri/tests/collector_process.rs`, `src-tauri/tests/collector_listeners.rs`,
`src-tauri/tests/relationships_process_port.rs`.
**Modify** — `src-tauri/src/collectors/registry.rs` (register both), `src-tauri/Cargo.toml`
(`windows` features listed above), `src-tauri/src/domain/event.rs` (`exit_kind`,
`identity_confidence`, `process/churn`), `src-tauri/src/storage/relationships.rs` (insert helper),
`docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. `platform/traits.rs`: `ProcessProvider { fn enumerate(&self) -> Result<Vec<ProcessRow>> }`,
   `SocketProvider { fn listeners(&self) -> Result<Vec<SocketRow>> }`.
2. Implement `platform/windows/process.rs`: ToolHelp enumeration; then a second pass that opens each
   *new* pid only (not every pid every tick) to fetch path + creation time; close every handle in a
   guard type so an early return cannot leak (`struct Handle(HANDLE)` with `Drop`).
3. Implement `platform/windows/sockets.rs`: call `GetExtendedTcpTable` with a null buffer to size,
   allocate, call again; handle `ERROR_INSUFFICIENT_BUFFER` retry loop with a 3-attempt cap; parse the
   `MIB_TCPTABLE_OWNER_PID` rows (ports are big-endian — `u16::from_be`), and the same for UDP/IPv6.
4. `collectors/process.rs`: diff, churn collapse, browser-subtree damping, `ProcessMap` publish.
5. `collectors/listeners.rs`: diff, owner resolution, relationship emission.
6. Verify each Win32 signature against `cargo doc --open -p windows` before writing the call; if a
   name differs from this plan, use the documented one and note the correction in the phase file
   (do not invent a fallback).
7. Tests with scripted providers: start/stop detection, pid-reuse does not forge events, path-denied
   rows degrade rather than error, churn collapse, listener open/close, relationship rows present in
   both directions, 30 s idle emits nothing.
8. Manual: run `bun run tauri dev`, start Notepad and a `python -m http.server 4000`, confirm both
   appear in the timeline within ~2 s with the port linked to the process.
9. Commits: (a) `feat: process provider and win32 enumeration`, (b) `feat: process diff collector`,
   (c) `feat: socket provider and listener collector`, (d) `feat: process-port relationships`,
   (e) `test: process and listener suites`.

## Todo (commit boundaries)

- [ ] Provider traits + Win32 process enumeration with handle guards
- [ ] Process diff collector with pid-reuse safety and path degradation
- [ ] Churn collapse + browser-subtree damping
- [ ] Listener enumeration (TCP/UDP, v4/v6) and diff collector
- [ ] Relationship rows both directions
- [ ] Test suites + manual smoke

## Success Criteria

- [ ] Scripted test: pid 1234 exits and a new pid 1234 appears with a different creation time → exactly
      one stop and one start, correctly ordered
- [ ] Scripted test: 20 of 40 processes deny their path → 40 events, 20 with `path: null` and
      `details.path_unavailable = true`, zero errors logged
- [ ] Scripted test: 8 start/stop pairs in 40 s for one image → one `process/churn` event with `count: 8`
- [ ] Scripted test: TCP 4000 opens owned by `api.exe` → `port/opened` + 2 relationship rows; the
      timeline row reads "Port 4000 opened by: api.exe"
- [ ] 30 s idle run against the real OS emits 0 events for both collectors (allowing for genuine OS churn: assert < 5)
- [ ] `cargo test` p95 cycle assertion: process ≤ 15 ms, listeners ≤ 15 ms (measured over 50 cycles)

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test collector_process --test collector_listeners --test relationships_process_port
cargo clippy --all-targets -- -D warnings
cargo test --test architecture
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| B3: sub-second processes missed | A known short command never appears | Documented in the descriptor copy and `docs/`; **not** solved by ETW (`Win32_ProcessStartTrace` measured Access-denied non-elevated). Do not add an elevation prompt |
| `GetProcessTimes` denied for the same ~46% | Many `identity_confidence: Low` events | Accept and surface: `Low` events still record, and P15 refuses to build a `Confirmed` link off a `Low`-confidence process identity |
| Handle leak in the per-new-pid pass | Handle count climbs in Task Manager over an hour | `Drop` guard on every handle + a test that opens/closes 10k times and asserts a stable `GetProcessHandleCount` for our own process |
| Big-endian port parsing wrong | Ports appear as 4000 → 43855 | Unit test with a fixed `MIB_TCPROW_OWNER_PID` byte pattern asserting 4000 |
| IPv6 table sizing differs | `GetExtendedTcpTable` returns error 122 repeatedly | Retry-with-grown-buffer loop already handles it; after 3 attempts the cycle is skipped and logged once, collector stays `Running` |
| Chrome/Teams flood the timeline | Timeline unusable during normal browsing | Damping rules above; the P06 calmness test (≥80% informational) runs against a captured 10-minute real stream, so regressions are caught mechanically |

## Security Considerations

- `PROCESS_QUERY_LIMITED_INFORMATION` only — never `PROCESS_VM_READ`, never command lines. Command
  lines are the highest-risk secret carrier (tokens in argv) and are **out of scope for the MVP**;
  the descriptor says so.
- Process paths pass through the P03 sanitizer (a path can contain a username or a token-named folder).
- No handle is retained across cycles.

---

### FILE: phase-10-file-config-collector.md

```yaml
---
phase: 10
title: "Watched-folder file collector with structured config diffing"
status: pending
priority: P1
effort: 7h
dependencies: [8]
---
```

## Overview

Watch user-chosen folders (§12) and produce *meaning*, not noise: for known config formats, a
key-level before/after diff (§13) with values scrubbed by P03; for everything else, a
created/modified/deleted/renamed record. This is the highest privacy-risk collector in the app, so it
is opt-in per folder, path-validated by the P05 rules, size-capped, and its diff output is the primary
target of the §60 leak tests.

## Requirements

**Functional**
- `add_watched_path(path)` / `remove_watched_path(path)` / `list_watched_paths()` (P05 commands, already
  allowlisted). Validation on add: canonicalize, must be a directory, reject reparse points/junctions,
  reject `C:\Windows`, `C:\Program Files*`, `C:\ProgramData\Microsoft`, any other user's profile,
  and the app's own data dir; max 20 roots; recursion depth cap 6 enforced during walk, not by notify.
- Watcher: `notify` 8.2.0 `RecommendedWatcher` (ReadDirectoryChangesW on Windows) wrapped in
  `notify-debouncer-full` 0.7.0 with a 500 ms debounce so editor save patterns (write-temp → rename →
  delete) collapse into a single logical change. Verify the debouncer constructor arity against
  `cargo doc -p notify-debouncer-full` before use.
- Ignore rules (built-in, non-configurable in MVP): `.git/`, `node_modules/`, `target/`, `dist/`,
  `build/`, `.next/`, `__pycache__/`, `.venv/`, `vendor/`, `*.log`, `*.tmp`, `*.swp`, `*~`,
  `*.lock` except `package-lock.json`/`bun.lock`/`Cargo.lock` (those are meaningful), and any file
  > 2 MB. Rules are a single `IgnoreSet` with unit tests.
- Structured diff for: `.env`/`.env.*`, `.json`, `.yaml`/`.yml`, `.toml`, `.ini`/`.conf`,
  `.properties`, `.xml` (attribute+text level). Parse old and new, flatten to `dotted.key = value`,
  compare, and emit **one event per changed key** (capped at 20 events per file change, then one
  summary event "and 14 more keys changed"). Format detection by extension first, then by content
  sniff for extensionless files named `Dockerfile`/`Caddyfile`/`hosts`.
- The "old" side comes from a shadow copy: on watch registration and after every processed change, the
  file's parsed key-map (values already redacted) is stored in the `config_snapshots` table keyed by
  path. So a diff never needs to re-read a previous version from disk, and the app holds no plaintext
  history. First observation of a file records a baseline with no event.
- Event shape matches §13 exactly: `entity_name` = file name, `field` = the dotted key,
  `before`/`after` = redacted values, `user_friendly_summary` = `DATABASE_URL changed in .env`,
  `technical_summary` includes the absolute path.
- Non-config files: `file/created`, `file/modified`, `file/deleted`, `file/renamed` with size and mtime
  in `details`; no content is read.
- Content is read **only** to parse a supported config format, only when ≤ 2 MB, and the plaintext
  buffer is zeroized (`drop` + explicit overwrite of the `String`'s bytes via `zeroize` 1.8 if the
  crate resolves; otherwise a manual `fill(0)` on a `Vec<u8>` buffer we own) before the function returns.

**Non-functional**
- A `bun install` inside a watched folder (thousands of files under `node_modules/`) produces ≤ 3
  events and ≤ 5% CPU for ≤ 10 s — the ignore set must be applied before any parse or DB touch.
- Watcher survives folder deletion/recreation and drive disconnect: the collector re-registers on
  `notify` error and marks the root `Unavailable` in `list_watched_paths()` instead of failing.

## Architecture

`collectors/files.rs` (watcher lifecycle, event → logical change), `collectors/config_diff/` module
(`mod.rs` dispatcher, `env.rs`, `json.rs`, `yaml.rs`, `toml.rs`, `ini.rs`, `xml.rs`, `flatten.rs`),
`storage/config_snapshots.rs` (baseline get/put), `domain/watched_path.rs`. Parsers: `serde_json`
(already present), `serde_yaml_ng` or `yaml-rust2` — **choose by `cargo add` resolution and record the
resolved name+version; do not assume `serde_yaml` (deprecated/unmaintained)**; `toml` for TOML;
hand-written line parsers for `.env`/`.ini`/`.properties` (~60 lines each, fewer deps than a crate and
tolerant of the malformed files real users have); `quick-xml` for XML if it resolves, else XML falls
back to whole-file `file/modified` and that limitation is listed in `ROADMAP.md` (§87).

## Related Code Files

**Create** — `src-tauri/src/collectors/files.rs`;
`src-tauri/src/collectors/config_diff/{mod.rs,flatten.rs,env.rs,json.rs,yaml.rs,toml.rs,ini.rs,xml.rs}`;
`src-tauri/src/collectors/ignore_set.rs`; `src-tauri/src/storage/config_snapshots.rs`;
`src-tauri/src/domain/watched_path.rs`; `src-tauri/src/commands/watched_paths.rs`;
`src-tauri/tests/collector_files.rs`, `src-tauri/tests/config_diff.rs`,
`src-tauri/tests/path_validation.rs`, `src-tauri/tests/redaction_leak_config.rs`;
`src-tauri/tests/fixtures/config/` (paired before/after files for each format, including a
secret-laden `.env`, a malformed YAML, a BOM-prefixed JSON, a CRLF `.ini`, a 3 MB JSON);
`src/features/settings/WatchedFolders.svelte`.
**Modify** — `src-tauri/src/collectors/registry.rs`, `src-tauri/Cargo.toml` (notify,
notify-debouncer-full, toml, yaml crate, quick-xml, zeroize), `src-tauri/src/commands/mod.rs` +
specta builder, `src/features/settings/SettingsScreen.svelte`, `ROADMAP.md`, `docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. `ignore_set.rs` first, with tests — it is the performance and privacy gate for everything else.
2. `path_validation` in `commands/watched_paths.rs` reusing the P05 validator; add reparse-point
   rejection via `std::fs::symlink_metadata().file_type().is_symlink()` **and** the
   `FILE_ATTRIBUTE_REPARSE_POINT` bit (junctions are not symlinks to Rust).
3. `flatten.rs`: `Value`-agnostic flattener producing `Vec<(String, String)>` sorted by key; arrays
   flatten as `key[0]`, `key[1]`.
4. Per-format parsers returning `BTreeMap<String,String>`; each tolerant of malformed input (return
   `Err(ParseSkipped)` → fall back to `file/modified`, never panic, never poison the watcher).
5. `config_snapshots.rs`: `get(path) -> Option<BTreeMap>`, `put(path, map)`, both storing **redacted**
   values only; `delete(path)` on unwatch/remove so removing a folder erases its baselines.
6. `collectors/files.rs`: debouncer wiring, logical-change classification (created/modified/deleted/
   renamed from the debounced event kinds), dispatch to config diff, event emission, root health.
7. Settings UI: folder list with add (native dialog via `tauri-plugin-dialog` — **only** the dialog
   plugin, no `fs` plugin), remove, status badge, and the copy "We record what changed, never the
   secret values."
8. Tests: `config_diff.rs` over every fixture pair (exact expected event lists);
   `redaction_leak_config.rs` asserts that for the secret `.env` fixture, no fixture secret substring
   occurs anywhere in the DB file bytes or the log file; `collector_files.rs` drives a temp dir through
   editor-save, rename, delete, and a 5000-file `node_modules` creation asserting ≤ 3 events.
9. Commits: (a) `feat: ignore set`, (b) `feat: watched path validation and commands`,
   (c) `feat: config parsers and flattener`, (d) `feat: config baseline storage`,
   (e) `feat: file watcher collector`, (f) `feat: watched folders settings ui`,
   (g) `test: config diff and file collector suites`.

## Todo (commit boundaries)

- [ ] Ignore set with tests
- [ ] Path validation incl. junction rejection
- [ ] Flattener + 6 format parsers, malformed-tolerant
- [ ] Redacted baseline snapshots in SQLite
- [ ] Debounced watcher + logical change classification
- [ ] Watched-folders settings UI
- [ ] Diff, leak, and stress suites

## Success Criteria

- [ ] `.env` fixture: `DATABASE_URL` changes → exactly one event titled `DATABASE_URL changed in .env`
      with both values `[REDACTED]`
- [ ] `.env` fixture with 25 changed keys → 20 key events + 1 "and 5 more keys changed"
- [ ] JSON fixture with a nested change → event key is `server.port`, before `3000`, after `4000`
      (non-secret values are preserved verbatim — redaction is key/pattern-driven, not blanket)
- [ ] Malformed YAML → one `file/modified` event, no panic, watcher still alive for the next change
- [ ] 3 MB JSON → `file/modified` only, never parsed
- [ ] `node_modules` creation of 5000 files → ≤ 3 events
- [ ] Removing a watched folder deletes its `config_snapshots` rows (asserted by count)
- [ ] `redaction_leak_config.rs` finds zero secret substrings in `history.db` and the log file
- [ ] Adding `C:\Windows` is rejected with a specific error message; adding a junction is rejected

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test config_diff --test collector_files --test path_validation \
           --test redaction_leak --test redaction_leak_config
cargo clippy --all-targets -- -D warnings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| A secret reaches the DB through a config value the pattern set misses | Leak test fails, or manual review finds plaintext | Layer two: key-name rules already redact `*_KEY`, `*_TOKEN`, `*_SECRET`, `*_PASSWORD`, `*_URL` **for the value**, so an unrecognized *shape* under a recognized *name* is still caught. Fixtures include a plain-looking high-entropy value under a benign key name; if that one leaks, the entropy gate is tightened and the fixture stays as a regression test |
| Chosen YAML crate is unmaintained/absent | `cargo add` fails or emits a deprecation | Record the resolved crate+version in the phase file. If none resolves cleanly, YAML degrades to `file/modified` and the gap is listed in `ROADMAP.md` — never a hand-rolled YAML parser |
| Debouncer collapses two genuinely different changes | Two edits 200 ms apart show as one | Accept: 500 ms is the editor-noise threshold; the event `details.debounced_paths` lists every path in the batch so nothing is silently lost |
| Watched folder on a removable drive disappears | Watcher thread errors repeatedly | Root marked `Unavailable`, re-registration attempted every 60 s, no event storm, collector stays `Running` |
| Plaintext lingers in memory | Not directly observable | Buffers are owned `Vec<u8>` overwritten before drop; no `String` from file content is cloned into an error message (parse errors carry line/col only, never content) |
| `ReadDirectoryChangesW` buffer overflow on huge churn | notify emits an overflow/rescan event | On overflow: skip the batch, log once, and re-baseline the affected root's config files (correct end state, no event storm) |

## Security Considerations

- Highest-risk collector: opt-in per folder, deny-listed system paths, depth and size caps, and content
  read only for parsing.
- There is no command that returns file content to the frontend — the drawer shows only diffed keys and
  redacted values (§56 stays narrow).
- Baselines in `config_snapshots` are redacted at write time, so even a stolen `history.db` has no
  secret values.
- `tauri-plugin-dialog` is added for folder picking; the `fs` plugin is **not** added, and
  `tests/capabilities_allowlist.rs` (P05) keeps it out.

---

### FILE: phase-11-service-application-collectors.md

```yaml
---
phase: 11
title: "Windows service collector and installed-application collector"
status: pending
priority: P1
effort: 5h
dependencies: [8]
---
```

## Overview

Services (§14) by 5 s diff-poll of `EnumServicesStatusExW` (measured 3.3 ms — cheap) with
`NotifyServiceStatusChangeW` used as an immediate wake for faster reaction; installed applications
(§17) by diffing the three uninstall registry hives (measured 88 + 148 + 38 keys, ~50 ms total) woken
by `RegNotifyChangeKeyValue` with a 5-minute fallback poll. Both are read-only, non-elevated, and
measured-PASS in the capability matrix.

## Requirements

**Functional**
- Service collector: `OpenSCManagerW(NULL, NULL, SC_MANAGER_ENUMERATE_SERVICE)` (read-only rights —
  never `SC_MANAGER_ALL_ACCESS`), `EnumServicesStatusExW(SC_ENUM_PROCESS_INFO, SERVICE_WIN32|SERVICE_DRIVER,
  SERVICE_STATE_ALL)` with the size-then-fill buffer pattern. Diff on service name; emit
  `service/started`, `service/stopped`, `service/installed`, `service/removed`,
  `service/start_type_changed`. Start type + display name via `QueryServiceConfigW` on an
  `OpenServiceW(SERVICE_QUERY_CONFIG)` handle, fetched only for services that are new or changed.
- `NotifyServiceStatusChangeW(SERVICE_NOTIFY_CREATED|DELETED)` on the SCM handle wakes the 5 s loop
  early; **treat it as an optimization**: if registration fails (research §5 lists this as unverified),
  log once at DEBUG and continue on the pure 5 s poll. The collector's correctness never depends on it.
- Application collector: read `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`,
  `HKLM\SOFTWARE\WOW6432Node\...\Uninstall`, `HKCU\SOFTWARE\...\Uninstall`; per subkey capture
  `DisplayName`, `DisplayVersion`, `Publisher`, `InstallDate`, `InstallLocation`, `SystemComponent`.
  Skip entries with `SystemComponent = 1` or an empty `DisplayName` (that is how the "148 keys" become
  a human-sized list). Diff on `(hive, subkey_name)`; emit `application/installed`,
  `application/removed`, `application/updated` (with `before`/`after` = versions).
- §17 phrasing: `Docker Desktop updated 4.35.1 → 4.36.0`. `application/updated` is capped at
  `importance: informational` (P06 rule + test).
- Registry watch: one thread per hive with `RegNotifyChangeKeyValue(hKey, TRUE,
  REG_NOTIFY_CHANGE_NAME|REG_NOTIFY_CHANGE_LAST_SET, event, TRUE)` waiting on an event handle with a
  300 s timeout — so the timeout *is* the fallback poll; no separate timer. Re-arm after every signal
  (the notification is one-shot).
- Both collectors publish to the shared entity store so `service:<Name>` and
  `application:<display_name_normalized>` ids are stable across restarts, and the app collector emits a
  `relationships` row `application:<x> --provides--> service:<y>` when a new service's binary path lives
  under a known `InstallLocation` (this is what lets P15 say "Docker Desktop update → com.docker.service
  restart → port change").

**Non-functional**
- Service cycle ≤ 12 ms p95 (baseline 3.3 ms); registry sweep ≤ 150 ms p95 (baseline ~50 ms) and runs
  off the fast tick (`cost_hint: Moderate`).
- Boot-time storm control: on app start, the first cycle of each collector is a baseline only (no
  events), so launching the app never floods the timeline with "everything started".

## Architecture

`collectors/services.rs`, `collectors/applications.rs` with `ServiceProvider` /
`RegistryUninstallProvider` traits; `platform/windows/services.rs` (advapi32 via `windows` 0.62
`Win32_System_Services`), `platform/windows/registry.rs` (`Win32_System_Registry`, plus the notify
thread). Registry values are read with `RegGetValueW`/`RegEnumKeyExW` directly rather than adding
`winreg` — one dependency fewer and the `windows` crate is already present. `SC_HANDLE`/`HKEY` wrapped
in `Drop` guards.

## Related Code Files

**Create** — `src-tauri/src/collectors/{services.rs,applications.rs}`;
`src-tauri/src/platform/windows/{services.rs,registry.rs}`;
`src-tauri/tests/collector_services.rs`, `src-tauri/tests/collector_applications.rs`,
`src-tauri/tests/baseline_no_storm.rs`.
**Modify** — `src-tauri/src/collectors/registry.rs`, `src-tauri/src/platform/traits.rs`,
`src-tauri/Cargo.toml` (`Win32_System_Services`, `Win32_System_Registry`),
`src-tauri/src/domain/event.rs` (new event types), `src-tauri/src/storage/entities.rs`,
`docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. `platform/windows/services.rs`: SCM open with minimum rights, size-then-fill enumeration,
   `QueryServiceConfigW` for new/changed only, handle guards, `NotifyServiceStatusChangeW` behind a
   `try_register_notify() -> bool`.
2. `platform/windows/registry.rs`: enumerate the three hives into `Vec<UninstallEntry>`; a notify
   thread per hive that signals an `mpsc` the app collector selects on.
3. `collectors/services.rs`: diff + baseline-first-cycle + start-type change detection.
4. `collectors/applications.rs`: diff + version compare + `InstallLocation` → service relationship.
5. Human naming: reuse P02's `display_name` normalizer for `DisplayName` (strip trailing version
   numbers and "(x64)" so `Docker Desktop 4.36.0` and `Docker Desktop` are the same entity).
6. Tests with scripted providers: install/remove/update/version-downgrade; service start/stop/install/
   remove/start-type; `SystemComponent` filtering; baseline-no-storm for both; relationship inference.
7. Manual: `bun run tauri dev`, then `net stop`/`net start` a user-controllable service (e.g. `Spooler`
   requires admin — use a Docker/VMware service if present, otherwise start any user-startable service)
   and confirm one event pair; install a small MSI/portable app and confirm `application/installed`.
8. Commits: (a) `feat: service provider and collector`, (b) `feat: registry uninstall provider`,
   (c) `feat: application collector with version diffing`, (d) `feat: application to service
   relationships`, (e) `test: service and application suites`.

## Todo (commit boundaries)

- [ ] Service enumeration with minimum SCM rights + handle guards
- [ ] Service diff collector with optional notify wake
- [ ] Registry uninstall enumeration + notify threads
- [ ] Application diff collector with version phrasing
- [ ] Install-location → service relationship inference
- [ ] Test suites incl. baseline-no-storm

## Success Criteria

- [ ] Scripted: 200 services, one stops → exactly one `service/stopped`, others silent
- [ ] Scripted: start type `Automatic → Manual` → one `service/start_type_changed` with before/after
- [ ] Scripted: 148 uninstall keys of which 60 are `SystemComponent = 1` → 88 tracked entities
- [ ] Scripted: version `4.35.1 → 4.36.0` → summary exactly `Docker Desktop updated 4.35.1 → 4.36.0`,
      importance `informational`
- [ ] First cycle after start emits zero events for both collectors (baseline-no-storm test)
- [ ] Real-OS run: service cycle ≤ 12 ms p95, registry sweep ≤ 150 ms p95 over 20 sweeps
- [ ] Notify-registration failure path leaves the collector `Running` (fault-injected test)

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test collector_services --test collector_applications --test baseline_no_storm
cargo clippy --all-targets -- -D warnings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `NotifyServiceStatusChangeW` unusable non-elevated (research §5, unverified) | `try_register_notify()` returns false | Already the designed fallback: pure 5 s poll. Worst case is 5 s latency, which the §14 requirement tolerates |
| `RegNotifyChangeKeyValue` misses `HKCU` changes | An app install shows up only at the 300 s fallback | Accept: the fallback bounds staleness at 5 minutes, and the requirement is "notices installs", not "notices instantly". Documented in the descriptor's cadence text |
| Registry sweep on a machine with 1000+ entries | Sweep > 500 ms | Sweep is `Moderate`/staggered and gated by `QueryBusy`; if p95 > 500 ms the hives are swept one per tick (3-tick rotation) rather than all at once |
| MSI repair/self-update churn | Repeated `application/updated` for the same version | Diff compares normalized versions; equal versions emit nothing. A same-version key rewrite is ignored |
| Handle leak on the early-return paths | Handle count grows | `Drop` guards + the same 10k open/close assertion used in P09 |
| Two collectors both claim the same service entity | Duplicate entity rows | `entities` has a unique index on `(kind, external_id)` from P04; inserts are upserts |

## Security Considerations

- Read-only rights everywhere: `SC_MANAGER_ENUMERATE_SERVICE`, `SERVICE_QUERY_CONFIG`, `KEY_READ`. The
  app cannot start, stop, or reconfigure a service even if compromised.
- `HKCU` reading is limited to the current user's own uninstall hive; no other user's hive is opened.
- Service binary paths and install locations pass through the P03 sanitizer (paths embed usernames).

---

### FILE: phase-12-update-driver-collectors.md

```yaml
---
phase: 12
title: "Windows Update collector (WUA COM) and driver collector (notify + rare sweep)"
status: pending
priority: P1
effort: 7h
dependencies: [9, 11]
---
```

## Overview

The two collectors that most often answer "what changed?" for a machine that broke overnight: Windows
Update history (§16) and driver installs/updates (§19). Both are measured-PASS non-elevated, but they
have opposite cost profiles — WUA `QueryHistory` is 38 ms and returned 133 entries; a full driver
enumeration is 2128 ms warm, which is far too slow to poll. So updates are polled on a slow timer plus
event-log corroboration, and drivers are notification-driven with a rare sweep.

## Requirements

**Functional**
- Update collector: COM `Microsoft.Update.Session` → `IUpdateSearcher` →
  `GetTotalHistoryCount()` + `QueryHistory(0, n)` → `IUpdateHistoryEntry` fields `Title`, `Date`,
  `ResultCode`, `Operation`, `ClientApplicationID`, `SupportUrl`, `UpdateIdentity`. Poll every 15 min
  and on resume-from-sleep. Diff on `(UpdateIdentity.UpdateID, Date)`. Emit `update/installed`,
  `update/failed` (`ResultCode` 4 = failed, 5 = aborted), `update/uninstalled`
  (`Operation` = `uoUninstallation`).
- **`Win32_QuickFixEngineering` is forbidden** — measured 4 rows vs WUA's 133; a comment in the code
  states why so nobody "simplifies" it back.
- COM lifecycle: the update collector owns a dedicated thread that calls
  `CoInitializeEx(COINIT_APARTMENTTHREADED)` once at start and `CoUninitialize` at stop; the COM
  pointers never cross the thread boundary — only plain `UpdateHistoryRow` structs do. All COM objects
  held in `windows` crate RAII wrappers.
- KB-number extraction from `Title` via `\(KB\d{6,7}\)` for the entity id
  (`update:KB5034123`), falling back to a hash of the `UpdateID` when there is no KB number
  (driver updates delivered through WU often have none).
- §16 phrasing: `Windows Update installed KB5034123` / `Windows Update failed: KB5034123 (error 0x80070002)`.
  `update/failed` is `importance: important` and is one of the §53 notification-worthy categories.
- Event-log corroboration: `Microsoft-Windows-WindowsUpdateClient/Operational` is subscribed in P13;
  when both sources describe the same KB within ±10 min, the WUA event is authoritative and the log
  entry only contributes `details.evidence`.
- Driver collector: `SetupDiGetClassDevs(NULL, NULL, NULL, DIGCF_ALLCLASSES | DIGCF_PRESENT)` +
  `SetupDiEnumDeviceInfo` + `SetupDiGetDeviceRegistryPropertyW(SPDRP_DEVICEDESC, SPDRP_CLASS,
  SPDRP_MFG, SPDRP_DRIVER)` and `SetupDiGetDeviceInstanceIdW`; driver version + date from the
  `SPDRP_DRIVER` key under `HKLM\SYSTEM\CurrentControlSet\Control\Class\{...}\<n>` (`DriverVersion`,
  `DriverDate`, `ProviderName`). Entity id = device instance id (P04 grammar).
- Cadence: `CM_Register_Notification(CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE, ...)` for
  arrival/removal wake, **plus** a sweep every 6 hours and one on resume-from-sleep. If
  `CM_Register_Notification` cannot be used (research §5 flags the callback ABI as unverified), fall
  back to `RegNotifyChangeKeyValue` on the Class key tree as the wake source, and if that also fails,
  pure 6-hour sweep. Three tiers, each independently sufficient; the chosen tier is recorded in
  `CollectorStatus.details` so the diagnostics screen shows it.
- §19 human naming: `NVIDIA graphics driver updated 566.36 → 566.45` — device description + class map
  to a friendly noun (`Display` → "graphics driver", `Net` → "network driver", `AudioEndpoint`/`MEDIA`
  → "audio driver", `USB` → "USB driver", else "<Class> driver"), with the manufacturer's first token as
  the brand. Mapping table is data, not code branches, and is unit-tested against the §73 example.
- Driver events: `driver/installed`, `driver/updated`, `driver/removed`, `driver/rolled_back`
  (version decreased). `driver/updated` is `importance: important` (§54) — this is the class of change
  that silently breaks audio and video.

**Non-functional**
- Driver sweep never runs while a query is in flight (`QueryBusy` gate from P08) and never on the fast
  tick; measured 2128 ms is budgeted as ≤ 3500 ms p95 with a hard 10 s timeout after which the sweep is
  abandoned and retried at the next interval.
- Update poll ≤ 200 ms p95 (baseline 38 ms).
- Neither collector blocks app startup: both run their first (baseline) cycle after the window is shown.

## Architecture

`collectors/updates.rs` + `collectors/drivers.rs` (diff + naming, provider-injected);
`platform/windows/wua.rs` (COM thread, `IUpdateSession`/`IUpdateSearcher`/`IUpdateHistoryEntry` via
`windows` 0.62 features `Win32_System_Com` + the WUA interfaces — verify the exact feature/module path
with `cargo doc -p windows` and, if WUA interfaces are absent from the crate, generate them with
`windows-bindgen` or declare them via `#[interface]` from `windows-core`; record which route was used);
`platform/windows/devices.rs` (SetupAPI via `Win32_Devices_DeviceAndDriverInstallation`,
`Win32_Devices_Properties`); `domain/driver_naming.rs` (the class→noun table).

Data flow is unchanged: provider → collector diff → normalizer → sanitizer → writer. Both collectors
persist their last-seen set in a `collector_state` row (P04) so a restart does not re-emit history —
critical for updates, where `QueryHistory` returns 133 entries every single time.

## Related Code Files

**Create** — `src-tauri/src/collectors/{updates.rs,drivers.rs}`;
`src-tauri/src/platform/windows/{wua.rs,devices.rs}`; `src-tauri/src/domain/driver_naming.rs`;
`src-tauri/tests/collector_updates.rs`, `src-tauri/tests/collector_drivers.rs`,
`src-tauri/tests/driver_naming.rs`, `src-tauri/tests/collector_state_restart.rs`.
**Modify** — `src-tauri/src/collectors/registry.rs`, `src-tauri/src/platform/traits.rs`,
`src-tauri/Cargo.toml` (windows features above), `src-tauri/src/storage/collector_state.rs`,
`src-tauri/src/domain/event.rs`, `docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. **Verify the WUA binding route first** (`cargo doc -p windows` / search the crate for
   `IUpdateSearcher`). Do not write COM calls before the route is confirmed; if bindgen is required,
   commit the generated module under `src-tauri/src/platform/windows/wua_bindings.rs` with a header
   comment naming the generator and the source metadata.
2. `platform/windows/wua.rs`: dedicated thread, `CoInitializeEx`, session creation, `QueryHistory`
   paging in chunks of 50, mapping to `UpdateHistoryRow`, `CoUninitialize` on stop. `ResultCode` and
   `Operation` mapped through explicit `match` on the documented integer values (2 = succeeded,
   3 = succeeded-with-errors, 4 = failed, 5 = aborted; 1 = installation, 2 = uninstallation).
3. `collectors/updates.rs`: load `collector_state`, diff, emit, persist state in the same transaction as
   the events (P04 writer supports a state piggyback — add it if absent).
4. `platform/windows/devices.rs`: enumeration with a `Drop` guard on the `HDEVINFO`
   (`SetupDiDestroyDeviceInfoList`), per-device property reads with the size-then-fill pattern, driver
   key read for version/date/provider.
5. `domain/driver_naming.rs`: the class→noun table + brand extraction + the exact §73 assertion.
6. `collectors/drivers.rs`: tiered wake registration (`CM_Register_Notification` → registry notify →
   timer only), sweep with the 10 s timeout, diff, rollback detection via semver-ish numeric compare
   with a lexical fallback for non-numeric versions.
7. Resume-from-sleep hook: subscribe to `WM_POWERBROADCAST`/`PBT_APMRESUMEAUTOMATIC` on the Tauri
   window (or poll `GetTickCount64` vs wall clock for a > 60 s jump as a fallback that needs no window
   message plumbing) and trigger both collectors' sweeps.
8. Tests with scripted providers: WUA history diffing (including "same 133 entries again → zero events"),
   failed-update phrasing with the hex error, uninstall operation, KB-less update ids, driver
   install/update/rollback/removal, naming table, restart-does-not-replay.
9. Manual: run `bun run tauri dev`; confirm the historical update list is *not* dumped into the timeline
   on first run (baseline) and that `driver/…` sweeps appear in the diagnostics screen with the chosen
   wake tier.
10. Commits: (a) `feat: wua binding route`, (b) `feat: update history collector`,
    (c) `feat: setupapi device provider`, (d) `feat: driver collector with tiered wake`,
    (e) `feat: driver friendly naming`, (f) `feat: resume from sleep sweeps`,
    (g) `test: update and driver suites`.

## Todo (commit boundaries)

- [ ] WUA binding route verified and committed
- [ ] Update history collector + persisted state
- [ ] SetupAPI provider with list guard
- [ ] Driver collector with three-tier wake and sweep timeout
- [ ] Friendly driver naming table
- [ ] Resume-from-sleep trigger
- [ ] Test suites

## Success Criteria

- [ ] Scripted WUA: 133 entries on first run → baseline, 0 events; 134th entry appears → exactly 1
      `update/installed` with summary `Windows Update installed KB5034123`
- [ ] Scripted WUA: `ResultCode = 4` → `Windows Update failed: KB5034123 (error 0x80070002)`,
      importance `important`
- [ ] Restart test: process restarts, `QueryHistory` returns the same 133 → 0 events
- [ ] `driver_naming.rs` test asserts the §73 string `NVIDIA graphics driver updated 566.36 → 566.45`
- [ ] Scripted drivers: version decrease → `driver/rolled_back`, not `driver/updated`
- [ ] Real-OS sweep: driver enumeration completes ≤ 3500 ms p95 over 5 sweeps; a forced 10 s stall is
      abandoned cleanly and the collector stays `Running`
- [ ] `list_collector_status()` reports which wake tier the driver collector obtained
- [ ] `rg -n "QuickFixEngineering" src-tauri/src` returns only the explanatory comment

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test collector_updates --test collector_drivers --test driver_naming \
           --test collector_state_restart
cargo clippy --all-targets -- -D warnings
cargo test --test architecture
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| WUA interfaces absent from `windows` 0.62 | `cargo doc` search finds no `IUpdateSearcher` | Generate bindings with `windows-bindgen` from the WUA metadata and commit them; the collector code above is unchanged because it only touches `platform/windows/wua.rs`. Do **not** shell out to PowerShell as a substitute |
| COM apartment misuse crashes the app | `RPC_E_*` HRESULT or a hard crash on stop | COM is confined to one owned thread with no pointer escape; the isolation test from P08 covers the panic path, and a failed `CoInitializeEx` disables the collector rather than aborting the process |
| `CM_Register_Notification` callback ABI wrong (research §5 unverified) | Registration returns non-zero, or a callback crash in the fault test | Tier down to registry notify, then to timer. The tier is chosen at start and never mid-flight, so there is no partial-registration state |
| 2.1 s driver sweep stalls the UI | A visible hitch when the sweep runs | Sweep runs on its own thread behind `QueryBusy`; a UI-latency test measures `get_events` p95 while a sweep is deliberately running and asserts ≤ 20 ms |
| Update history replayed after a DB reset | Timeline floods with old updates | `collector_state` is reset with the DB, so a reset *does* re-baseline (0 events) rather than re-emit — asserted by the restart test with a fresh DB |
| Driver version strings non-numeric (`31.0.15.5222` vs `1.2.3-hp`) | Rollback misdetected | Numeric-tuple compare when both parse, lexical compare otherwise, and `Unknown` direction → emit `driver/updated` without a rollback claim |

## Security Considerations

- Both collectors are read-only: WUA is used for history only (never `IUpdateInstaller`), SetupAPI for
  property reads only (never `SetupDiCallClassInstaller`).
- Update titles and driver provider strings pass through the P03 sanitizer; `SupportUrl` is stored but
  never made clickable in the UI (no `shell:open` capability exists — P05 allowlist).
- The `Security` event-log channel is not touched here; corroboration uses only channels measured
  readable non-elevated.

---

### FILE: phase-13-network-startup-eventlog-collectors.md

```yaml
---
phase: 13
title: "Network config, startup/scheduled-task, and Windows event-log collectors"
status: pending
priority: P1
effort: 8h
dependencies: [12]
---
```

## Overview

The last three collectors, and the three most expensive to enumerate: network configuration (§18,
measured 1440 ms), scheduled tasks and startup entries (§20, measured 1325 ms for 194 tasks of which 28
are logon-triggered), and Windows event logs (§21) which are a *subscription*, not a sweep. All three
are notification-driven with rare sweeps, and the event-log collector is the corroboration source that
P09's `exit_kind` and P12's update evidence depend on.

## Requirements

**Functional**
- Network collector: `GetAdaptersAddresses(AF_UNSPEC, GAA_FLAG_INCLUDE_GATEWAYS |
  GAA_FLAG_INCLUDE_ALL_INTERFACES)` for interfaces/IPs/DNS/gateways, `GetIpForwardTable2` for the
  default route, and the WinINET/WinHTTP proxy configuration read from
  `HKCU\...\Internet Settings` (`ProxyEnable`, `ProxyServer`, `ProxyOverride`, `AutoConfigURL`) rather
  than a COM call. Wake via `NotifyIpInterfaceChange` + `NotifyRouteChange2` + a
  `RegNotifyChangeKeyValue` on Internet Settings; sweep every 30 min and on resume.
  Events: `network/adapter_changed`, `network/ip_changed`, `network/dns_changed`,
  `network/gateway_changed`, `network/proxy_changed`, `network/vpn_detected` (adapter with a tunnel
  `IfType` — 131/`IF_TYPE_TUNNEL` — or a description matching a small known-VPN token list).
  `network/proxy_changed` and `network/dns_changed` are `important` (§54): they are top causes of "it
  worked yesterday".
- Startup collector: Task Scheduler COM (`Schedule.Service` → `ITaskService::Connect(NULL…)` →
  `GetFolder("\\")` → recursive `ITaskFolder::GetTasks(TASK_ENUM_HIDDEN)`) capturing name, path,
  enabled, triggers, action path, last run/result; plus the four run keys
  (`HKCU`/`HKLM` × `Run`/`RunOnce`), the two Startup folders, and (read-only) the current user's
  `StartupApproved` state. Events: `startup/task_added|removed|changed|disabled|enabled`,
  `startup/run_key_added|removed`. Cadence: `RegNotifyChangeKeyValue` on the run keys (instant), notify
  watcher on the two Startup folders (reuse the P10 notify machinery with a private root), and a
  60-minute sweep for the task store (1325 ms is too slow for anything faster). Only logon/boot/idle
  triggered tasks are treated as "startup"; the rest are recorded but at `informational` and excluded
  from §53 notifications (this is what keeps 194 tasks from becoming 194 alerts).
- Event-log collector: `EvtSubscribe(NULL, NULL, channel, query, NULL, ctx, callback,
  EvtSubscribeToFutureEvents)` on exactly four channels — `Application`, `System`,
  `Microsoft-Windows-WindowsUpdateClient/Operational`, `Setup`. **The `Security` channel is not
  subscribed**: measured unauthorized non-elevated. A code comment states this so it is never "fixed"
  by adding it back.
- Event-log XPath filters keep volume sane: `Application` → `*[System[(Level<=2)]]` plus
  `EventID` 1000/1001/1002/1026; `System` → `Level<=2` plus 7000/7001/7009/7011/7023/7024/7031/7034
  (service failures), 6008 (unexpected shutdown), 41 (kernel power), 1074 (planned restart/shutdown);
  `Setup` → all; WU/Operational → all. Emit `system/error_logged`, `system/service_failed`,
  `system/unexpected_shutdown`, `system/restart`, `system/boot`.
- Rendering: `EvtRender(EvtRenderEventXml)` then a *targeted* XML read (provider, event id, level,
  time, and the `<Data>` children) — no full XML blob stored. `details.message` is built from the
  `<Data>` values, not `EvtFormatMessage` (which needs provider metadata that is often missing);
  when `<Data>` is empty the event still records with provider + id and a generic phrase.
- Dedup: an identical `(provider, event_id, first_data_value)` repeated within 60 s increments a count
  on the existing event instead of inserting a new one (log-storm control, §53).
- Corroboration API: `analysis::evidence::find_log_evidence(image_or_service, window)` used by P09's
  `exit_kind` and P12's update evidence; implemented here as a repository query over the persisted log
  events (not a live log re-read).
- **§1 "since last restart"** is now fully sourced: `system/boot` + 6005/6006/6008/1074 give the true
  boot boundary, replacing the P06 fallback of "earliest event after the longest gap". P06's
  `TimeRange::SinceLastRestart` is switched to this source in this phase and its test updated.

**Non-functional**
- Network sweep ≤ 2500 ms p95 (baseline 1440 ms), task sweep ≤ 2500 ms p95 (baseline 1325 ms); both
  behind `QueryBusy`, staggered so they never overlap, hard 15 s timeout.
- Event-log callback returns in < 1 ms: it copies the handle-rendered fields into a channel and does no
  DB work on the callback thread (a slow callback throttles the whole channel).
- A 500-error log burst produces ≤ 20 stored events (dedup + filters), measured by a fault-injection test.

## Architecture

`collectors/network.rs`, `collectors/startup.rs`, `collectors/eventlog.rs` with
`NetworkProvider`/`TaskProvider`/`EventLogSource` traits; `platform/windows/network.rs`
(`Win32_NetworkManagement_IpHelper`), `platform/windows/tasks.rs` (`Win32_System_TaskScheduler` COM on
its own initialized thread, same discipline as P12's WUA), `platform/windows/eventlog.rs`
(`Win32_System_EventLog` — `EvtSubscribe`/`EvtRender`; callback is an `extern "system"` fn with a
`Box`ed context pointer whose lifetime outlives the subscription, released only after `EvtClose`).
`analysis/evidence.rs` is created here as the shared corroboration query.

## Related Code Files

**Create** — `src-tauri/src/collectors/{network.rs,startup.rs,eventlog.rs}`;
`src-tauri/src/platform/windows/{network.rs,tasks.rs,eventlog.rs}`;
`src-tauri/src/analysis/evidence.rs`; `src-tauri/tests/collector_network.rs`,
`src-tauri/tests/collector_startup.rs`, `src-tauri/tests/collector_eventlog.rs`,
`src-tauri/tests/eventlog_storm.rs`, `src-tauri/tests/since_last_restart.rs`;
`src-tauri/tests/fixtures/eventlog/*.xml` (10 real rendered samples captured from this machine's
readable channels, secrets stripped).
**Modify** — `src-tauri/src/collectors/registry.rs`, `src-tauri/src/platform/traits.rs`,
`src-tauri/Cargo.toml` (windows features above), `src-tauri/src/collectors/process.rs` (wire
`exit_kind` corroboration), `src-tauri/src/collectors/updates.rs` (wire evidence),
`src-tauri/src/domain/query.rs` (`SinceLastRestart` source switch),
`src-tauri/tests/event_queries.rs`, `docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. Capture the event-log fixtures first: `wevtutil qe Application /c:5 /f:xml` (read-only, non-elevated,
   already proven readable) into `tests/fixtures/eventlog/`, then hand-strip anything machine-identifying.
   The parser is written against real XML, not imagined XML.
2. `platform/windows/eventlog.rs`: subscription per channel, `extern "system"` callback that renders and
   pushes to an `mpsc::SyncSender` with `try_send` (drop + count on full, never block the callback),
   `EvtClose` on stop before the context box is dropped.
3. `collectors/eventlog.rs`: channel drain, XPath-filtered mapping, dedup window, event emission.
4. `analysis/evidence.rs` + wire P09 `exit_kind` and P12 update evidence; update both phases' tests to
   the corroborated behaviour.
5. `platform/windows/network.rs`: `GetAdaptersAddresses` size-then-fill, route table via
   `GetIpForwardTable2` + `FreeMibTable` guard, proxy from the registry; then `collectors/network.rs`
   diff with per-field events.
6. `platform/windows/tasks.rs`: COM thread, recursive folder walk, trigger-type extraction
   (`ITrigger::Type` → logon/boot/idle/schedule); then `collectors/startup.rs` diff over tasks + run keys
   + startup folders.
7. Switch `TimeRange::SinceLastRestart` to the boot-event source and update `since_last_restart.rs`.
8. Tests: scripted providers for network (IP change, DNS change, proxy on/off, VPN adapter appearing),
   startup (task added/disabled, run key added, startup folder file added), event log (fixture parsing
   for all 10 samples, dedup, 500-event storm → ≤ 20 stored), and `exit_kind` corroboration
   (crash log present → `Unexpected`; absent → `Unknown`).
9. Manual: toggle a system proxy setting and change DNS on one adapter; confirm two `important` events
   appear with correct before/after; add a Startup-folder shortcut and confirm one event.
10. Commits: (a) `feat: event log subscription and rendering`, (b) `feat: event log collector with dedup`,
    (c) `feat: log evidence corroboration`, (d) `feat: network provider and collector`,
    (e) `feat: task scheduler provider`, (f) `feat: startup collector`,
    (g) `refactor: since-last-restart uses boot events`, (h) `test: network, startup and eventlog suites`.

## Todo (commit boundaries)

- [ ] Real event-log fixtures captured
- [ ] EvtSubscribe + callback + rendering with no callback DB work
- [ ] Event-log collector with XPath filters and dedup
- [ ] Evidence API wired into P09/P12
- [ ] Network provider + per-field diff collector
- [ ] Task Scheduler provider + startup collector
- [ ] SinceLastRestart switched to boot events
- [ ] Test suites incl. storm control

## Success Criteria

- [ ] All 10 event-log fixtures parse to the expected `(provider, event_id, level, message)` tuples
- [ ] 500 identical Application errors in 60 s → ≤ 20 stored events, one with a count ≥ 480
- [ ] `Security` channel is never subscribed (`rg -n '"Security"' src-tauri/src/collectors` → comment only)
- [ ] Scripted: proxy `off → 127.0.0.1:8888` → one `network/proxy_changed`, importance `important`,
      before/after present
- [ ] Scripted: DNS `192.168.1.1 → 8.8.8.8` → one `network/dns_changed`
- [ ] Scripted: 194 tasks with 28 logon-triggered → 28 startup entities, 166 recorded at `informational`
- [ ] `exit_kind` test: with a matching 1000 error → `Unexpected`; without → `Unknown` (never guessed)
- [ ] `SinceLastRestart` test: a fixture stream with a 6005 boot event returns exactly the post-boot slice
- [ ] Real-OS: network sweep ≤ 2500 ms p95, task sweep ≤ 2500 ms p95; `get_events` p95 ≤ 20 ms while a
      sweep runs

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test collector_network --test collector_startup --test collector_eventlog \
           --test eventlog_storm --test since_last_restart --test event_queries
cargo clippy --all-targets -- -D warnings
cargo test --test architecture --test importance_calm
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `EvtSubscribe` callback ABI/lifetime mistake → crash | Access violation shortly after subscribing | Context is a `Box::into_raw` held in the subscription struct and only freed after `EvtClose`; the callback does nothing but render + `try_send`. A dedicated stress test subscribes/unsubscribes 200 times |
| Event-log volume swamps the DB | `history.db` grows > 100 MB in a day | XPath level filters + dedup + the P04 30-day retention; if growth still exceeds budget, `Application` narrows to Level<=2 only (drop the explicit EventID list) — a one-line change with a test |
| Task Scheduler COM enumeration hangs on a corrupt task | Sweep hits the 15 s timeout repeatedly | Timeout abandons the sweep, the failing folder path is remembered and skipped next time, and the skip is surfaced in diagnostics |
| `NotifyIpInterfaceChange` callback fires on its own thread while the collector is stopping | Use-after-free on shutdown | Cancel handle is closed before the collector's state is dropped; the notify callback only sets an `AtomicBool` |
| Proxy read from the wrong place (per-user vs WinHTTP machine proxy) | A proxy change made in Settings is missed | MVP reads the per-user WinINET keys (that is what browsers and most tooling use); the WinHTTP machine proxy is listed in `ROADMAP.md` as a known gap rather than silently claimed |
| VPN detection false positives | Hyper-V/loopback adapters reported as VPN | Detection requires tunnel `IfType` **or** a known-vendor token; anything else is `network/adapter_changed`. Fixture test includes a Hyper-V adapter asserting no VPN event |

## Security Considerations

- Only the four measured-readable channels are subscribed; the unreadable `Security` channel is
  explicitly excluded with a comment, so the app never appears to need elevation.
- Event-log `<Data>` values frequently contain paths and usernames → every rendered field goes through
  the P03 sanitizer before storage, and the leak test corpus gains an event-log case.
- Network data (IPs, DNS servers, SSIDs, proxy hosts) is local-only and never leaves the machine; the
  privacy screen (§48) lists these categories by name.
- Task action paths and run-key command lines are stored **redacted** — run keys are a known place for
  embedded credentials.

---

### FILE: phase-14-checkpoints.md

```yaml
---
phase: 14
title: "Checkpoints: known-good snapshots and diff-since-checkpoint"
status: pending
priority: P1
effort: 5h
dependencies: [11]
---
```

## Overview

A checkpoint is a named "it worked here" marker plus a full state snapshot, so the user can later ask
"what changed since then?" and get an answer that does not depend on event retention or on the
collectors having been running the whole time (§23–§25). Checkpoints are created manually, from the
tray, and automatically before high-risk changes. This is the feature that converts the app from a log
viewer into a debugging tool, and it is deliberately built before the analysis engine so the engine can
use checkpoint boundaries as evidence.

## Requirements

**Functional**
- `create_checkpoint(name: Option<String>, kind: Manual|Automatic, reason: Option<String>)`: calls
  `snapshot()` on every enabled `Cheap`/`Moderate` collector plus a *forced* sweep of the `Expensive`
  ones, serializes each collector's state set into `checkpoint_items` (collector_id, entity_kind,
  entity_external_id, entity_name, state_json), and writes one `checkpoints` row (id, name, kind,
  reason, created_at, item_count, duration_ms, partial: bool).
- Default name when none given: `Checkpoint <weekday> <h:mm am/pm>` (e.g. `Checkpoint Friday 3:14 PM`).
- All values in `state_json` pass through the P03 sanitizer — a checkpoint is a durable artifact and is
  covered by an extra §60 leak assertion.
- Expected cost: sum of measured snapshots ≈ 6.2 + 3.3 + 50 + 38 + 1440 + 1325 + 2128 ms ≈ 5.0 s. The UI
  therefore shows a determinate progress list ("Programs ✓ / Services ✓ / Drivers …") driven by a
  `CheckpointProgress` event, and the command returns only when finished. A collector that fails or
  times out (8 s per collector) marks the checkpoint `partial: true` and records which collectors are
  missing — a partial checkpoint is still useful and is labelled everywhere it appears.
- `list_checkpoints()` → newest first with name, kind, age, item count, partial flag.
- `delete_checkpoint(id)` (cascades items), `rename_checkpoint(id, name)`.
- `compare_checkpoint(id, to: Now | Checkpoint(id2))` → `CheckpointDiff { added: Vec<DiffItem>,
  removed: Vec<DiffItem>, changed: Vec<DiffItem { entity, field, before, after }>, grouped_by_category,
  event_ids_in_window }`. `changed` is computed by the same per-collector comparison functions the
  collectors use for their live diffs — one implementation, two callers (DRY): each collector exposes
  `fn diff_states(old: &StateSet, new: &StateSet) -> Vec<StateChange>` and the live loop is
  `diff_states(prev, current)`.
- `event_ids_in_window` links the structural diff to the recorded event stream between the two
  timestamps so the UI can show "27 changes recorded in this window" and jump to the timeline filtered
  to that range.
- Automatic checkpoints: created before a detected Windows Update install (WU/Operational "installation
  started"), before an `application/installed` that is still in progress (best-effort: on the first
  `msiexec`/`*setup*.exe`/`*installer*.exe` process start, debounced to at most one per 10 min), and
  once every 7 days if none exists. Automatic checkpoints are capped at 10 (oldest auto pruned; manual
  ones are never auto-pruned) and are exempt from the P04 retention purge (D-05).
- Tray: "Create checkpoint" is one of the seven §7 entries and calls the same command; a success balloon
  reads "Checkpoint saved. You can compare against this later."
- UI (§24, §25): Checkpoints screen — list with primary action `Compare to now`; comparison view with
  three sections (Added / Removed / Changed), category grouping, a `Show technical details` toggle
  (§51), and the empty state "No checkpoints yet. Create one while things work, so you can compare
  later." Six states covered in the dev harness.

**Non-functional**
- `create_checkpoint` p95 ≤ 8 s on this machine with all collectors enabled; the UI stays responsive
  (progress events at least every 500 ms) and no other query is blocked for more than 20 ms.
- `compare_checkpoint` p95 ≤ 150 ms for a 3000-item checkpoint (two indexed reads + an in-memory
  hash-join, no N+1 queries).
- A checkpoint of a 3000-item machine occupies ≤ 2 MB in SQLite (state_json is per-item, compact keys).

## Architecture

`checkpoint/{mod.rs,create.rs,compare.rs,auto.rs}` under `src-tauri/src/`;
`storage/checkpoints.rs` for the two tables; `commands/checkpoints.rs` for the six commands.
`StateSet` = `BTreeMap<String /*external_id*/, serde_json::Value>` produced by each collector's
`state_set()` (added to the `Collector` trait in this phase as a defaulted method returning an empty
set, so no collector is forced to implement it before it can). Comparison is collector-agnostic:
`compare.rs` hash-joins two `StateSet`s per collector id and delegates field-level differences to the
collector's `diff_states`.

## Related Code Files

**Create** — `src-tauri/src/checkpoint/{mod.rs,create.rs,compare.rs,auto.rs}`;
`src-tauri/src/storage/checkpoints.rs`; `src-tauri/src/commands/checkpoints.rs`;
`src-tauri/src/domain/checkpoint.rs`; `src-tauri/tests/checkpoint_create.rs`,
`src-tauri/tests/checkpoint_compare.rs`, `src-tauri/tests/checkpoint_auto.rs`,
`src-tauri/tests/checkpoint_leak.rs`; `src/features/checkpoints/{CheckpointsScreen.svelte,
CheckpointList.svelte,CreateCheckpointDialog.svelte,CheckpointProgress.svelte,
ComparisonView.svelte,DiffSection.svelte}`; `src/lib/stores/checkpoints.svelte.ts`.
**Modify** — `src-tauri/src/collectors/mod.rs` (`state_set`, `diff_states` on the trait),
`src-tauri/src/collectors/{process.rs,listeners.rs,files.rs,services.rs,applications.rs,updates.rs,
drivers.rs,network.rs,startup.rs}` (implement `state_set`; refactor the live loop to call the shared
`diff_states`), `src-tauri/src/commands/mod.rs` + specta builder (6 commands),
`src-tauri/src/app/tray.rs` (wire "Create checkpoint"),
`src/features/dev/DevStatesHarness.svelte`, `docs/system-architecture.md`, `ROADMAP.md`.
**Delete** — none.

## Implementation Steps

1. Add `state_set()` and `diff_states()` to the `Collector` trait with defaults; refactor each existing
   collector so its live loop uses `diff_states` (this is a behaviour-preserving refactor — the P09–P13
   suites must stay green and are the guard).
2. `storage/checkpoints.rs`: insert checkpoint + bulk-insert items in one transaction with a prepared
   statement reused across items; indexes `checkpoint_items(checkpoint_id)` and
   `checkpoint_items(checkpoint_id, collector_id)`.
3. `create.rs`: per-collector 8 s timeout, progress emission, partial flag, sanitizer pass, and the
   `QueryBusy` cooperation so a checkpoint does not stall the timeline.
4. `compare.rs`: hash-join, `DiffItem` construction, category grouping, event-window query.
5. `auto.rs`: the three automatic triggers with their debounces + the 10-auto cap prune + the retention
   exemption.
6. Frontend: list, create dialog (name field, "What are you doing right now?" optional reason), the
   determinate progress panel, and the comparison view with the three sections.
7. Tests: create with scripted collectors (3000 items, verifies count/duration/partial-on-timeout),
   compare (added/removed/changed with a field-level change), auto-trigger debounce + cap + retention
   exemption, and `checkpoint_leak.rs` (a secret in a scripted state set never lands in
   `checkpoint_items`).
8. Manual: create a checkpoint, install/uninstall a small app, compare to now, confirm the app appears
   under Added/Removed with the right category.
9. Commits: (a) `refactor: shared state diffing on the collector trait`, (b) `feat: checkpoint storage`,
   (c) `feat: checkpoint creation with progress and partial handling`, (d) `feat: checkpoint comparison`,
   (e) `feat: automatic checkpoints`, (f) `feat: checkpoints ui`, (g) `test: checkpoint suites`.

## Todo (commit boundaries)

- [ ] Trait-level `state_set`/`diff_states` + collector refactor (P09–P13 suites still green)
- [ ] Checkpoint tables and bulk insert
- [ ] Creation with progress, timeouts, partial flag
- [ ] Comparison with category grouping and event window
- [ ] Automatic triggers, cap, retention exemption
- [ ] Checkpoints UI with six states
- [ ] Four test suites

## Success Criteria

- [ ] Scripted 3000-item checkpoint: created in one transaction, `item_count = 3000`, DB growth ≤ 2 MB
- [ ] One collector times out → `partial: true`, missing collector listed, UI shows "Partial — Drivers
      could not be captured"
- [ ] Compare with 5 added, 3 removed, 2 field changes → exactly those, grouped by category
- [ ] `compare_checkpoint` p95 ≤ 150 ms at 3000 items (asserted in the test)
- [ ] Automatic checkpoint fires once for a burst of 5 installer starts within 10 min
- [ ] 11th automatic checkpoint prunes the oldest automatic one and leaves all manual ones
- [ ] Retention purge with a 1-day cutoff leaves checkpoint rows intact
- [ ] `checkpoint_leak.rs`: no secret substring anywhere in `checkpoint_items`
- [ ] Tray "Create checkpoint" produces the same result as the in-app button

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test checkpoint_create --test checkpoint_compare --test checkpoint_auto \
           --test checkpoint_leak --test redaction_leak
cargo test --test collector_process --test collector_services --test collector_applications
cd /d/Project/.WhatChage && bun run check && bun run check:bindings && bun run test
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| The `diff_states` refactor changes live collector behaviour | A P09–P13 test fails | The refactor commit lands **before** any checkpoint feature so the failure is unambiguous and revertible on its own |
| 5 s creation feels broken | Users click twice / the button appears dead | Determinate per-collector progress + the button disables during creation + a duplicate call returns the in-flight checkpoint id instead of starting a second one |
| Expensive sweeps during checkpoint creation stall the UI | Timeline queries slow while creating | Sweeps run on collector threads; the command awaits a completion channel; a UI-latency assertion runs inside `checkpoint_create.rs` |
| Checkpoint bloat | DB grows faster than the event stream | `state_json` stores only fields used by `diff_states`; a size assertion in the test fails loudly if a collector starts dumping whole objects |
| Automatic checkpoints fire on unrelated `*setup*.exe` | Noise in the checkpoint list | Automatic checkpoints are visually distinct, capped at 10, and prunable; a false positive costs a row, not correctness |
| A partial checkpoint is compared as if complete | Phantom "removed" items for the missing collector | Comparison skips collectors absent from *either* side and states so in the UI ("Drivers not compared — missing from this checkpoint") |

## Security Considerations

- Checkpoints are the longest-lived artifact in the app (exempt from retention), so they get their own
  leak test in addition to the general §60 suite.
- `delete_checkpoint` and the §50 delete-all both remove items via `ON DELETE CASCADE`; the §48 privacy
  screen counts checkpoint items in its "what is stored" figures.
- No checkpoint export in the MVP (nothing to leak by sharing); the §58 diagnostics export includes
  checkpoint *counts* only, never item contents.

---

### FILE: phase-15-analysis-engine-investigation.md

```yaml
---
phase: 15
title: "Deterministic analysis engine: correlation, causal ranking, investigation flow"
status: pending
priority: P1
effort: 12h
dependencies: [13, 14]
---
```

## Overview

The product thesis lands here. Everything before this phase records; this phase *answers*. The engine
takes a user problem statement plus a time window and returns a ranked, explained list of suspect
changes (§27–§32, §45, §47, §59, §74). It is fully deterministic — a scoring function over recorded
events and relationships, no ML, no network, no LLM — so the same history always produces the same
ranking, and the ranking is testable with fixtures. Confidence language is capped by evidence class so
the app never claims causation it cannot support (§32).

## Requirements

**Functional**

*Investigation entry (§45)*
- `start_investigation(problem: String, window: InvestigationWindow)` where
  `InvestigationWindow = LastHour | Today | SinceYesterday | SinceLastRestart | SinceCheckpoint(id) | Custom{from,to}`.
  Default window resolution when the user gives none: if the problem text contains a time hint
  ("this morning", "yesterday", "after the update") map it; else `Today`.
- Problem-text parsing is **keyword extraction, not NLU**: lowercase, strip stopwords, and match
  against (a) known entity names from the `entities` table (fuzzy: case-insensitive substring +
  normalized display name), (b) a small domain lexicon mapping symptom words to categories
  (`microphone|audio|sound|mic → [driver:Audio, application, service]`,
  `internet|network|wifi|dns|vpn|proxy|offline → [network, driver:Net, service]`,
  `port|localhost|connect|refused|3000|8080 → [port, process, config]`,
  `slow|freeze|lag → [driver, update, startup, service]`,
  `crash|closes|quit|error → [application, update, driver]`,
  `build|compile|test → [config, application, update]`,
  `docker|container → [docker, service, application]`,
  `login|sign in|password → [config, network, application]`). The lexicon is a data table with a unit
  test per row; unmatched text simply yields no category boost (never an error, never a wrong guess).
- Result: `Investigation { id, problem, window, resolved_window, suspects: Vec<Suspect>, total_events_in_window, created_at }`
  persisted to the `investigations` table so it appears in history and can be reopened.

*Correlation graph (§28, §29)*
- `analysis/graph.rs` builds an in-memory graph from the window's events (≤ 5000 events; older windows
  are downsampled by dropping `informational` events first, and the drop is disclosed in the UI as
  "Analysed the 5,000 most important changes in this window").
- Edge sources, in decreasing evidence strength:
  1. `relationships` rows recorded by collectors (P09 `listens_on`/`opened_by`, P11 `provides`,
     P10 `config_of`) — **structural**, the only edge class that can reach `Confirmed`.
  2. Same-entity edges: two events on the same `entity_id` — **structural**.
  3. Temporal-proximity edges: events within 5 min, weighted by closeness — **circumstantial**.
  4. Category-affinity edges from a fixed table (`config → service`, `update → driver`,
     `driver → device`, `service → port`, `startup → process`) — **circumstantial**.
- Chain detection: depth-limited DFS (max depth 4, max 200 paths) over structural edges ordered by
  time, producing `Chain { events: Vec<EventId>, span_ms, edge_kinds }`. A chain of ≥ 2 structural
  edges is the §30 "cascade" that the UI renders as a sequence.

*Scoring (§31)*
- `score = w_t·temporal + w_r·relationship + w_c·category + w_s·severity + w_x·rarity + w_p·failure_proximity`,
  computed in `analysis/score.rs` with named constants in one `const` block:
  `W_TEMPORAL = 1.0`, `W_RELATIONSHIP = 3.0`, `W_CATEGORY = 1.5`, `W_SEVERITY = 1.0`,
  `W_RARITY = 1.25`, `W_FAILURE_PROXIMITY = 2.0`.
  - `temporal` = `exp(-minutes_before_symptom / 90.0)` clamped to `[0,1]`, and `0.0` for anything
    *after* the symptom time (a change after the failure cannot have caused it — hard rule, tested).
  - `relationship` = `1.0` if the event is on an entity structurally linked to a symptom-matched
    entity, `0.6` if two structural hops away, `0.0` otherwise.
  - `category` = `1.0` if the event's category is in the lexicon-derived category set, `0.35` if it is
    in a category-affinity neighbour of one, else `0.0`.
  - `severity` = `{critical: 1.0, important: 0.6, informational: 0.15}` from the P06 classifier.
  - `rarity` = `1.0 - min(1.0, prior_occurrences_90d / 20.0)` (a first-ever driver update scores 1.0;
    a Chrome update that happened 18 times scores 0.1). Uses the P06 `prior_occurrences` query.
  - `failure_proximity` = `1.0` if a `system/error_logged` / `service_failed` / `process/stopped`
    (`exit_kind: Unexpected`) event exists within 10 min *after* this event **and** shares an entity or
    a structural edge with it; `0.5` for the same within 60 min; else `0.0`.
- Chain bonus: an event that is the *head* of a detected structural chain gets
  `score += 0.75 · (chain_len - 1)`, capped at `+2.25`, and the chain's members are attached to the
  suspect instead of listed separately — so the timeline of a cascade is presented as one suspect with
  its steps (§30), not three competing suspects.
- Ranking is total-ordered and stable: sort by `score` desc, then `severity` desc, then `timestamp`
  desc, then `event_id` asc. Two runs over the same data are byte-identical (asserted).

*Confidence classes (§32)*
- `Confirmed` — requires a structural chain reaching a symptom-matched entity **and** a corroborating
  failure signal (`failure_proximity == 1.0`). Copy: "This change directly affected <entity>."
- `Likely` — structural edge without a corroborating failure signal, or a chain whose corroboration is
  only within 60 min. Copy: "This change likely affected <entity>."
- `Correlated` — temporal/category evidence only. Copy: "This changed around the same time."
- The class is computed from evidence, not from the score, and the copy template is selected by class in
  `analysis/explain.rs`. A `Correlated` suspect can never render `Confirmed` copy — enforced by the type
  (`Explanation::for_class(class, ...)` is the only constructor) and by the forbidden-phrase test.
- Forbidden phrases anywhere in generated explanation strings for non-`Confirmed` classes: "caused",
  "because of", "the problem is", "this broke", "definitely", "root cause". `tests/no_causal_overclaim.rs`
  generates explanations for 200 fixture suspects across all three classes and greps the output.

*Explanations (§31, §74)*
- Each suspect renders: title (the event's `user_friendly_summary`), timestamp + relative phrasing
  ("34 minutes before you noticed the problem"), the confidence sentence, a `why_ranked` bullet list
  built from the contributing score terms in plain language ("Happened 34 minutes before the problem",
  "Changed a file that this service reads", "First time this driver changed in 90 days"), the chain
  steps when present, and a `What to try` line drawn from a fixed remediation table keyed by event type
  (e.g. `network/proxy_changed` → "Check your proxy setting under Windows Settings → Network → Proxy").
  The remediation table contains no commands the app runs — it is advice text only.
- §74 worked example is a mandatory fixture: a `.env` `PORT 3000 → 4000` change, then a
  `service/stopped`+`service/started` for the API service, then `port/closed 3000` + `port/opened 4000`.
  With the problem "my app can't connect to the api", the top suspect must be the `.env` change,
  `Confirmed`, with a 3-step chain, and the explanation must contain "Port 4000" and the two other
  steps — asserted string-by-string.

*Ranking fixture (§59)*
- `tests/fixtures/analysis/scenarios/*.json` — 8 hand-built scenarios with a declared expected top-3.
  The mandated one: the config→restart→port chain must outrank a Chrome update **even when the Chrome
  update is moved to 2 minutes before the symptom and the chain is 40 minutes before**. This is the
  test that proves relationship + rarity dominate raw recency, and it fails loudly if anyone rebalances
  the weights carelessly. Other scenarios: driver update vs unrelated app install; proxy change vs
  service restart; two competing drivers where only one is on the symptom's device; a window with
  nothing but `informational` events (expect "No obvious cause" — see below); a 5000+ event window
  (downsampling still returns the planted cause); a cascade whose head is outside the window (expect
  `Likely`, never `Confirmed`); an event *after* the symptom (must never rank).
- No-answer path (§47): if the top score is below `MIN_REPORTABLE_SCORE = 1.2`, the result is
  `suspects: []` plus `no_cause_reason: NothingChanged | OnlyRoutineChanges | WindowTooNarrow |
  NotWatchingYet`, each with its own copy ("Nothing meaningful changed in this window." /
  "Only routine changes here — updates and background tasks." / "Try widening the time range." /
  "We started watching after this window. Try a more recent range."). Guessing is never preferred over
  saying nothing.

*Commands (already allowlisted in P05)*
- `start_investigation`, `get_investigation`, `list_investigations`, `delete_investigation`,
  `analyze_window(window)` (dashboard "what changed recently" without a problem statement — same engine,
  empty symptom set, category boost disabled), `get_related_events(event_id)` (single-event graph
  neighbourhood for the P07 drawer's "Related events" section, retro-filled here).

**Non-functional**
- `start_investigation` p95 ≤ 400 ms for a 5000-event window on this machine; ≤ 1.5 s at the 100k-event
  DB size (window queries are index-bound, the graph is capped).
- Engine is pure and OS-free: `analysis/` imports nothing from `platform/` or `collectors/`
  (`tests/architecture.rs` extended with this rule). It takes `Vec<SystemEvent>` +
  `Vec<Relationship>` and returns `Investigation` — that is why it is fixture-testable.
- Zero allocation of the whole event table: windows are fetched by the P04 keyset query with a cap.

## Architecture

`analysis/{mod.rs,window.rs,symptom.rs,graph.rs,chain.rs,score.rs,confidence.rs,explain.rs,
remediation.rs,downsample.rs}` plus the existing `analysis/evidence.rs` from P13.
`symptom.rs` owns the lexicon + entity matching and returns `SymptomModel { entities: Vec<EntityId>,
categories: Vec<Category>, symptom_time: DateTime }`. `graph.rs` builds `Graph { nodes: Vec<EventId>,
edges: Vec<Edge { from, to, kind: Structural|Temporal|CategoryAffinity, weight }>`. `score.rs` is a pure
function `fn score(event, ctx: &ScoreContext) -> ScoreBreakdown` where `ScoreBreakdown` keeps every term
separately — that breakdown is what `explain.rs` turns into `why_ranked` bullets, so the explanation can
never disagree with the score (one source of truth, DRY).
`storage/investigations.rs` persists the investigation and its suspects (`suspects` stored as ids +
scores + class, re-explained on read so copy changes do not need a migration).
Frontend: `features/investigate/{InvestigateScreen.svelte,ProblemForm.svelte,WindowPicker.svelte,
SuspectList.svelte,SuspectCard.svelte,ChainSteps.svelte,NoCauseState.svelte,InvestigationHistory.svelte}`,
store `lib/stores/investigation.svelte.ts`.

## Related Code Files

**Create** — `src-tauri/src/analysis/{window.rs,symptom.rs,graph.rs,chain.rs,score.rs,confidence.rs,
explain.rs,remediation.rs,downsample.rs}`; `src-tauri/src/storage/investigations.rs`;
`src-tauri/src/commands/investigate.rs`; `src-tauri/src/domain/investigation.rs`;
`src-tauri/tests/analysis_scoring.rs`, `src-tauri/tests/analysis_ranking_fixtures.rs`,
`src-tauri/tests/analysis_chains.rs`, `src-tauri/tests/no_causal_overclaim.rs`,
`src-tauri/tests/analysis_determinism.rs`, `src-tauri/tests/analysis_perf.rs`,
`src-tauri/tests/symptom_lexicon.rs`; `src-tauri/tests/fixtures/analysis/scenarios/{port-config-chain,
driver-vs-install,proxy-vs-restart,two-drivers,routine-only,large-window,head-outside-window,
after-symptom}.json`; the frontend files listed above.
**Modify** — `src-tauri/src/analysis/mod.rs`, `src-tauri/src/commands/mod.rs` + specta builder
(6 commands), `src-tauri/src/domain/event.rs` (`ScoreBreakdown`, `ConfidenceClass` exports),
`src-tauri/tests/architecture.rs` (new purity rule), `src/features/home/ProblemInput.svelte`
(remove the P07 stub, route to Investigate), `src/components/event/RelatedEvents.svelte`
(wire `get_related_events`), `ROADMAP.md` (clear the P07 stub entry), `docs/system-architecture.md`,
`PRODUCT.md`.
**Delete** — none.

## Implementation Steps

1. Write the 8 scenario fixtures **first**, with their expected top-3 declared. They are the spec; the
   engine is written to satisfy them. Each fixture is a JSON list of events + relationships + a problem
   string + a symptom time.
2. `window.rs`: window resolution (incl. `SinceLastRestart` via P13 boot events and `SinceCheckpoint`
   via P14 timestamps), event fetch with the 5000 cap, `downsample.rs` dropping `informational` first
   and reporting the drop count.
3. `symptom.rs`: lexicon table + entity fuzzy match + symptom-time inference; `symptom_lexicon.rs` test
   asserts one row per mapping and that an unmatched problem yields an empty category set.
4. `graph.rs` + `chain.rs`: edge construction from the four sources with `kind` recorded; DFS with the
   depth/path caps; chain span computation.
5. `score.rs`: the const block and the six terms, each with its own unit test at boundary values
   (0 minutes, 90 minutes, after-symptom, 0 vs 20 prior occurrences, all three severities).
6. `confidence.rs` + `explain.rs` + `remediation.rs`: class derivation from evidence only,
   `Explanation::for_class` as the single constructor, bullets generated from `ScoreBreakdown`,
   remediation table keyed by event type with a default of "Compare against a checkpoint from when this
   worked."
7. Wire `start_investigation` end-to-end with persistence, then `analyze_window` and
   `get_related_events` reusing the same graph builder.
8. Frontend: problem form (§45 copy, the P07 placeholder text preserved), window picker, ranked suspect
   cards with confidence badge + `why_ranked` + chain steps + `What to try`, the four no-cause states,
   and investigation history. Six states in the dev harness, including the 900×600 frame.
9. Run the fixture suite; tune only by adjusting a *term definition* with a justification comment, never
   by special-casing a fixture. If a fixture cannot be satisfied without special-casing, stop and record
   it in the phase file as an open design question rather than fudging the test.
10. `analysis_perf.rs`: 5000-event window ≤ 400 ms p95; then seed 100k events and assert ≤ 1.5 s.
11. Commits: (a) `test: analysis ranking fixtures`, (b) `feat: window resolution and downsampling`,
    (c) `feat: symptom model and lexicon`, (d) `feat: correlation graph and chain detection`,
    (e) `feat: suspect scoring`, (f) `feat: confidence classes and explanations`,
    (g) `feat: investigation commands and persistence`, (h) `feat: investigate ui`,
    (i) `test: determinism, overclaim and performance suites`.

## Todo (commit boundaries)

- [ ] 8 scenario fixtures with declared expectations
- [ ] Window resolution + downsampling with disclosure
- [ ] Symptom model (lexicon + entity match)
- [ ] Graph + chain detection with caps
- [ ] Six-term scorer with named constants
- [ ] Confidence classes + explanation generation + remediation table
- [ ] Commands, persistence, related-events retro-fill
- [ ] Investigate UI with four no-cause states
- [ ] Determinism, overclaim, performance suites

## Success Criteria

- [ ] §59 fixture passes: with the Chrome update 2 min before the symptom and the config→restart→port
      chain 40 min before, the chain's head is rank 1 and the Chrome update is not in the top 3
- [ ] §74 fixture passes: top suspect is the `.env` `PORT` change, class `Confirmed`, 3 chain steps, and
      the explanation contains "Port 4000"
- [ ] An event timestamped after the symptom never appears in `suspects` (any fixture, any window)
- [ ] `no_causal_overclaim.rs`: zero forbidden phrases across 200 generated `Likely`/`Correlated`
      explanations
- [ ] A chain whose head is outside the window yields at most `Likely`, never `Confirmed`
- [ ] Routine-only window yields `suspects: []` with `no_cause_reason: OnlyRoutineChanges` and the exact
      §47 copy
- [ ] `analysis_determinism.rs`: two runs over the same fixture produce identical suspect ids, scores,
      classes, and explanation strings
- [ ] `analysis_perf.rs`: 5000-event window ≤ 400 ms p95; 100k-event DB ≤ 1.5 s
- [ ] `tests/architecture.rs`: no `platform/` or `collectors/` import inside `analysis/`
- [ ] Every score term has a boundary unit test; `rg -n "if event.id ==" src-tauri/src/analysis` is empty
      (no fixture special-casing)

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test analysis_ranking_fixtures --test analysis_scoring --test analysis_chains \
           --test no_causal_overclaim --test analysis_determinism --test symptom_lexicon \
           --test architecture
cargo test --test analysis_perf -- --include-ignored
cargo clippy --all-targets -- -D warnings
cd /d/Project/.WhatChage && bun run check && bun run check:bindings && bun run test
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Weights tuned until fixtures pass, producing a ranker that only works on fixtures | Fixtures pass but a manual real-machine investigation returns nonsense | Weights may only change with a written justification in `score.rs` and all 8 fixtures re-run; additionally a manual acceptance run on this machine's real 7-day history is part of the phase's Done, and its top-5 must be defensible by inspection |
| Over-claiming causation damages trust (§32) | A `Correlated` suspect reads like a diagnosis | Class-gated constructor + forbidden-phrase test + the `What to try` line is advice, never an assertion. If a reviewer finds one over-claiming string, the phrase joins the forbidden list as a regression test |
| Symptom keyword matching produces a wrong entity match and skews everything | Suspect list is dominated by an unrelated app | Entity matches require a normalized token match of length ≥ 4 (no 2-letter substring hits); unmatched → category-only scoring; the "two-drivers" fixture guards the entity-specificity path |
| Graph blowup on a busy window | Investigation takes seconds, CPU spike | Hard caps: 5000 events, depth 4, 200 paths, and temporal edges only within 5 min. `analysis_perf.rs` fails the build if exceeded |
| Downsampling drops the actual cause | Planted cause missing from a large-window fixture | Downsampling drops `informational` first and *never* drops an event with a structural edge or a `failure_proximity` contribution; the `large-window` fixture asserts the planted cause survives |
| Rarity term punishes legitimately frequent causes (e.g. a nightly job that really does break things) | A recurring true cause ranks low | Rarity is weighted 1.25, not dominant, and `failure_proximity` (2.0) can still lift a frequent event; the `proxy-vs-restart` fixture covers a recurring-but-causal case |
| Persisted suspects go stale when copy changes | Reopened investigations show old wording | Only ids/scores/classes are persisted; explanations are regenerated on read |

## Security Considerations

- The engine reads only already-redacted events, so explanations cannot leak a secret value; the
  `.env` fixture's explanation is asserted to contain the *key name* and `[REDACTED]`, never a value.
- `remediation.rs` returns advice text only — no command execution, no `shell:open`, no auto-fix. The
  app never modifies the system it observes (§4), and there is no IPC command that could.
- Problem statements are user free text: stored via the P03 sanitizer (a user may paste a connection
  string into "what stopped working"), rendered as a text node, never interpolated into SQL (the
  keyword matcher builds parameterized `LIKE` clauses with escaped `%`/`_`).
- Investigation persistence is subject to the §50 delete-all and the 90-day retention class.

---

### FILE: phase-16-three-js-visualization.md

```yaml
---
phase: 16
title: "Change Map: purposeful 3D causal/temporal visualization with 2D fallback"
status: pending
priority: P2
effort: 9h
dependencies: [15]
---
```

## Overview

One visualization, one job: make the *shape* of a cascade visible in a way a list cannot (§35–§39).
The Change Map plots the events of an investigation or a time window in three meaningful axes — X =
time, Y = category lane, Z = chain depth — draws the P15 structural edges as tubes and the temporal
edges as faint lines, and lets the user click a node to open the same details drawer the timeline uses.
Three.js is used because the third axis carries data (chain depth / branching), not because 3D looks
impressive; if the visualization cannot answer "which change started the chain and what followed",
it is cut rather than decorated. A full 2D canvas fallback ships in the same phase for machines without
usable WebGL and for `prefers-reduced-motion` users.

## Requirements

**Functional**
- Entry points: (1) an investigation's "View as map" action showing that investigation's suspects,
  chains, and the symptom marker; (2) the Timeline's "Map" toggle showing the current window/filters.
  Both feed the same component with the same typed payload — no second data path.
- Layout is deterministic and computed in TypeScript from the Rust payload (`ChangeMapData { nodes, edges,
  symptom_time, lanes }`): X = `(timestamp - window_start) / window_span` scaled to `[-halfW, halfW]`;
  Y = one lane per category in a fixed order (process, port, config, service, application, update,
  driver, network, startup, system) so lane position is stable across sessions; Z = chain depth from
  P15's chains (0 for unchained events). Node size encodes importance (informational 0.6, important 1.0,
  critical 1.4); colour comes from the §41 token palette by category (read from CSS variables at scene
  init so theme switches stay consistent).
- Rendering: one `THREE.InstancedMesh` (sphere, 12×8 segments) for all nodes with per-instance colour
  and matrix; one `LineSegments` with a `BufferGeometry` for temporal edges; structural edges as a second
  `LineSegments` with a brighter material and 2 px `linewidth`-independent visual weight achieved by
  drawing them as short quads only if a spike shows lines are too faint (default: two line materials with
  different opacity — no third-party line library).
- Interaction: orbit-style camera implemented directly with pointer events and a small damping loop
  (~70 lines) instead of adding `OrbitControls` from the examples bundle — keeps the dependency surface at
  the `three` package only, as decided in research §4. Constraints: azimuth free, polar clamped to
  `[0.15π, 0.85π]`, zoom clamped `[0.4, 4]` of the fit distance, pan clamped to the data bounds; a
  `Reset view` button restores the fitted camera.
- Picking: `Raycaster` against the instanced mesh (`instanceId`) on pointer-move throttled to 30 Hz and
  only while the pointer is inside the canvas; hover shows a DOM tooltip (not a WebGL label) with the
  event summary + relative time; click emits `select(eventId)` → the P07 details drawer opens over the
  map; the selected node is highlighted by writing an emphasis colour into the instance colour buffer
  (no extra mesh, no re-layout).
- Chain emphasis: hovering or selecting a node dims all instances not in its chain to 25% opacity via the
  instance colour buffer, and draws the chain's structural edges at full strength. That is the one
  interaction that justifies the third dimension — a cascade becomes a visible ridge.
- Symptom marker: a vertical translucent plane at the symptom time with the label "You noticed the
  problem here" rendered as a DOM overlay pinned by projecting the world point each frame.
- Accessibility and equivalence (§62): the canvas has `role="img"` with an `aria-label` summarizing
  node/edge counts and the top chain; a visually-present `Switch to list` control returns to the ranked
  list (the map is never the only way to reach the information); keyboard `Tab` focuses a hidden
  ordered list of nodes with `Enter` selecting one (same handler as click), so a keyboard user can reach
  every node; `prefers-reduced-motion: reduce` disables camera damping/auto-fit animation and the render
  loop becomes on-demand (render only after an interaction).
- Fallback: `ChangeMap2D.svelte` (plain 2D canvas, X = time, Y = lane, chains drawn as polylines, same
  picking/selection API) is used when `WebGLRenderingContext` creation fails, when
  `renderer.getContext().isContextLost()` after init, on `webglcontextlost`, or when the user sets
  `visualization: "2d"` in settings. Switching is transparent to the parent component (both implement the
  same props/events contract).
- Data caps: max 3000 nodes and 6000 edges. Above that, the payload is aggregated server-side by the
  existing P15 downsampler and the map shows a chip "Showing the 3,000 most important changes".

**Non-functional**
- 60 FPS at 3000 nodes on this machine's GPU, measured with a 300-frame timing harness; if the median
  frame exceeds 16.6 ms, node geometry drops to 8×6 segments and temporal edges are thinned by stride —
  both switches are in one `qualityTier` constant.
- No allocation inside the render loop: matrices, colours, and vectors are pre-allocated; the loop does
  `instanceMatrix.needsUpdate` only when something changed.
- Full teardown on unmount: `renderer.dispose()`, `renderer.forceContextLoss()`, every geometry/material
  `.dispose()`, `setAnimationLoop(null)`, pointer/resize listeners removed, and the canvas element
  dropped. A mount/unmount×20 test asserts `renderer.info.memory.geometries === 0` for a fresh renderer
  and that JS heap does not grow monotonically (heap check is advisory; the geometry/texture counters are
  the hard assertion).
- Render loop pauses when the window is hidden/minimized (`document.visibilitychange`) and when the map
  is not the active route — no GPU burn behind the tray.
- The map never blocks the main data path: it is a leaf consumer of an existing command's result.

## Architecture

`src/three/` is the only place `three` is imported (enforced by an import-boundary test):
`three/changeMap/{createScene.ts,layout.ts,nodes.ts,edges.ts,camera.ts,picking.ts,quality.ts,
dispose.ts,palette.ts}` — all plain TS, framework-free, unit-testable with a null renderer.
`src/components/visualization/{ChangeMap.svelte,ChangeMap2D.svelte,ChangeMapFrame.svelte,
MapLegend.svelte,MapTooltip.svelte,MapNodeList.svelte}`. `ChangeMapFrame.svelte` owns capability
detection and chooses 3D or 2D; it is what features import.
Svelte integration per research §4: `$effect` creates the scene on mount and returns a teardown closure;
reactive prop changes call `scene.update(data)` rather than recreating the scene; the store is read once
into a local, so a re-render never rebuilds the WebGL context.
Rust side: `analysis/change_map.rs` shapes `ChangeMapData` from an existing `Investigation` or window
result (specta-exported), so the frontend performs no causal logic — it only positions what Rust already
decided (§81, and it keeps the ranking single-sourced).

## Related Code Files

**Create** — `src-tauri/src/analysis/change_map.rs`; `src-tauri/src/commands/change_map.rs`
(`get_change_map(investigation_id | window)`); `src-tauri/tests/change_map_payload.rs`;
`src/three/changeMap/{createScene.ts,layout.ts,nodes.ts,edges.ts,camera.ts,picking.ts,quality.ts,
dispose.ts,palette.ts}`; `src/components/visualization/{ChangeMap.svelte,ChangeMap2D.svelte,
ChangeMapFrame.svelte,MapLegend.svelte,MapTooltip.svelte,MapNodeList.svelte}`;
`src/three/changeMap/layout.test.ts`, `src/three/changeMap/picking.test.ts`,
`src/three/changeMap/dispose.test.ts`, `src/components/visualization/change-map-frame.test.ts`;
`src/lib/stores/changeMap.svelte.ts`.
**Modify** — `src/features/investigate/InvestigateScreen.svelte` (View as map),
`src/features/timeline/TimelineToolbar.svelte` (Map toggle),
`src/features/settings/SettingsScreen.svelte` (`visualization: auto|3d|2d`),
`src-tauri/src/commands/mod.rs` + specta builder (1 command),
`src-tauri/src/domain/settings.rs`, `src/features/dev/DevStatesHarness.svelte`,
`package.json` (`three` 0.185.1, `@types/three`), `docs/system-architecture.md`, `PRODUCT.md`.
**Delete** — none.

## Implementation Steps

1. `analysis/change_map.rs` + command + payload test first: nodes/edges/lanes/symptom time derived from
   the P15 result, capped and aggregated. The frontend cannot invent structure it is not given.
2. `layout.ts` as a pure function `(data, viewport) => {positions: Float32Array, colors: Float32Array,
   laneIndex, bounds}` with unit tests (stable lane order, deterministic output for identical input,
   correct scaling at 1-node and 3000-node extremes).
3. `palette.ts`: read the §41 CSS variables via `getComputedStyle(document.documentElement)` once and map
   category → `THREE.Color`; a theme change re-reads and rewrites the instance colour buffer.
4. `createScene.ts`: renderer (`antialias: true`, `powerPreference: 'low-power'`, DPR
   `Math.min(devicePixelRatio, 2)`), orthographic-ish perspective camera fitted to bounds, ambient +
   one directional light, instanced node mesh, two line meshes, symptom plane.
5. `camera.ts`: pointer-drag orbit with damping, wheel zoom, clamps, `reset()`, and a
   `reducedMotion` flag that disables damping and switches to on-demand rendering.
6. `picking.ts`: throttled raycast, `instanceId → eventId` map, hover/select state, chain dimming via the
   colour buffer.
7. `ChangeMap.svelte`: `$effect` lifecycle, resize observer, visibility pause, tooltip/overlay
   positioning, `role="img"` + `aria-label`, hidden `MapNodeList` for keyboard access.
8. `ChangeMap2D.svelte`: same contract on a 2D canvas (lane rows, time X, chain polylines, hit-testing by
   nearest-point within 8 px).
9. `ChangeMapFrame.svelte`: capability probe (create a throwaway context, check for loss), settings
   override, `webglcontextlost` → swap to 2D at runtime with a one-time toast "Switched to the simple map
   because 3D stopped working."
10. `dispose.test.ts`: mount/unmount 20 times in jsdom with a stubbed WebGL context, asserting dispose
    calls and listener removal counts; `quality.ts` frame-timing harness behind `#/dev/perf`.
11. Six states in the dev harness: loading skeleton, empty ("Nothing to map in this window."), error,
    populated (3000 nodes), 2D fallback, and the 900×600 frame.
12. Commits: (a) `feat: change map payload`, (b) `feat: deterministic map layout`,
    (c) `feat: three scene with instanced nodes and edges`, (d) `feat: map camera and picking`,
    (e) `feat: chain emphasis and symptom marker`, (f) `feat: 2d fallback map`,
    (g) `feat: map frame with capability detection`, (h) `test: layout, picking, dispose suites`.

## Todo (commit boundaries)

- [ ] Rust `ChangeMapData` payload + test
- [ ] Pure layout with unit tests
- [ ] Scene with instanced nodes + two edge classes
- [ ] Camera, picking, chain emphasis, symptom marker
- [ ] 2D fallback with identical contract
- [ ] Capability detection + settings override + context-loss recovery
- [ ] Dispose/lifecycle tests + six states

## Success Criteria

- [ ] The §74 investigation rendered as a map shows a 3-node chain as a connected ridge; selecting any
      member dims the rest and highlights the chain's edges
- [ ] Clicking a node opens the same drawer as the timeline row for that event id
- [ ] 3000 nodes render with a median frame ≤ 16.6 ms at `#/dev/perf` on this machine (recorded number in
      the phase file); the quality tier drop is exercised by forcing `qualityTier = 'low'`
- [ ] `layout.test.ts`: identical input → byte-identical `Float32Array` output; lane order matches the
      documented category order
- [ ] `dispose.test.ts`: after 20 mount/unmount cycles, geometries/materials disposed, `setAnimationLoop(null)`
      called, and zero remaining pointer/resize/visibility listeners
- [ ] Forcing `WebGLRenderingContext` creation to fail renders `ChangeMap2D` with no console error, and the
      2D map supports hover/select
- [ ] `prefers-reduced-motion: reduce` → no continuous render loop (frame counter stays flat while idle)
- [ ] Keyboard: `Tab` reaches the node list, `Enter` selects, focus returns to the map control after the
      drawer closes
- [ ] Minimizing the window stops rendering (frame counter flat), restoring resumes
- [ ] `rg -n "from 'three'" src --glob '!src/three/**'` returns nothing

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test -- src/three src/components/visualization
cd src-tauri && cargo test --test change_map_payload && cargo clippy --all-targets -- -D warnings
cd /d/Project/.WhatChage && bun run tauri dev   # manual: #/dev/perf frame timing, forced 2D, 900x600
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| 3D adds no insight over the list (decorative) | Reviewer cannot name a question the map answers faster | The map's justification is chain shape + branching; if the chain-emphasis interaction does not read clearly at 3000 nodes, drop the Z axis and ship the 2D map as the only visualization (the fallback already exists, so this is a deletion, not a rewrite) |
| WebGL unavailable or context lost | Blank canvas / `webglcontextlost` | Capability probe at mount + runtime swap to the 2D map with a toast; the 2D path is a shipped, tested component, not a placeholder |
| GPU/VRAM leak across route changes | `renderer.info.memory` counters climb; RAM grows per visit | Dispose test with 20 cycles is a build gate; `forceContextLoss()` on teardown |
| Instanced picking wrong under DPR ≠ 1 | Hover highlights the wrong node on a scaled display | Raycaster uses NDC computed from `getBoundingClientRect`, never `offsetWidth`; `picking.test.ts` covers DPR 1, 1.5, 2 |
| 100k-event windows overwhelm the map | Freeze on open | Hard 3000-node cap enforced in Rust (the frontend cannot request more), with the disclosure chip |
| Theme switch leaves stale colours | Dark-mode map in light theme | Palette re-read on theme change writes the colour buffer; covered by a component test toggling the theme class |
| Reduced-motion users get a moving camera anyway | Motion persists with the OS flag set | `matchMedia` checked at init *and* subscribed for changes; assertion in the frame-counter test |

## Security Considerations

- The map renders only fields already present in the redacted event payload; node labels use
  `user_friendly_summary` (sanitized) and tooltips are DOM text nodes, never `{@html}`.
- No shader receives user-controlled strings; there is no runtime shader compilation from data.
- `three` is pinned at 0.185.1 with no examples/addons imported, keeping the audited dependency surface to
  one package.
- The visualization performs no IPC of its own beyond `get_change_map`, which is read-only and typed.

---

### FILE: phase-17-docker-development-mode.md

```yaml
---
phase: 17
title: "Docker collector and developer-environment collector (dev mode)"
status: pending
priority: P2
effort: 6h
dependencies: [8]
---
```

## Overview

Developer-specific change sources: Docker container/image lifecycle and the developer environment
itself (environment variables, PATH entries, toolchain versions, git branch of watched repos). Docker
Desktop is **not installed on this machine** (decision D3), so the live path cannot be verified here —
therefore the collector is written against a transport trait with a scripted fake covering the wire
protocol, and the only paths verifiable locally (absence, unavailability, reconnection) are the ones
this phase's gate asserts. The Docker-present path is explicitly listed as an unverified surface in the
phase file and in `ROADMAP.md` rather than being claimed as tested.

## Requirements

**Functional**

*Docker collector (§dev-tooling)*
- Detection: `\\.\pipe\docker_engine` exists → `Available`; missing → collector state `Unavailable`
  with the §33 copy "Docker isn't running. We'll start recording container changes when it does."
  Re-probe every 60 s; no error log per probe (one line on transition only).
- Transport: `DockerTransport` trait (`fn get(&mut self, path: &str) -> Result<Vec<u8>>`,
  `fn stream(&mut self, path: &str) -> Result<Box<dyn BufRead>>`). Windows implementation opens the
  named pipe with `CreateFileW(GENERIC_READ|GENERIC_WRITE, ...)` and speaks HTTP/1.1 by hand
  (`GET /v1.43/events?filters=... HTTP/1.1\r\nHost: docker\r\n\r\n`), parsing chunked responses — this
  avoids adding an HTTP client and a pipe-transport crate for two endpoints. API version is requested as
  `/v1.43/` with a fallback to `/v1.41/` when the daemon replies 400 with a version message; the
  negotiated version is recorded in `CollectorStatus.details`.
- Endpoints used: `GET /_ping` (health), `GET /version` (daemon version → `docker/daemon_changed`),
  `GET /events` (streamed), `GET /containers/json?all=1` and `GET /images/json` (baseline + resync after
  a stream drop). Nothing else — no exec, no create, no start, no stop (read-only by construction).
- Events mapped from the stream: `docker/container_started`, `docker/container_stopped`,
  `docker/container_died` (`exitCode != 0` → `importance: important` and the exit code in `details`),
  `docker/container_created`, `docker/container_removed`, `docker/image_pulled`,
  `docker/image_removed`, `docker/volume_created`, `docker/volume_removed`,
  `docker/network_created`, `docker/network_removed`, `docker/compose_project_changed` (derived from
  the `com.docker.compose.project` label when ≥ 2 containers of a project change within 30 s — one
  event instead of ten).
- Entity ids: `container:<name_or_short_id>` (name preferred — it survives recreation, which is the
  whole point when a compose service is rebuilt), `image:<repo:tag>`, `docker_volume:<name>`,
  `docker_network:<name>`. Port mappings from the event's `Actor.Attributes` create
  `container --publishes--> port` relationship rows, so P15 can link "container restarted" to
  "port 5432 closed".
- Stream resilience: on read error/EOF, mark `Reconnecting`, resync from `containers/json` +
  `images/json` (diffing against the last state set — no duplicate events for containers that never
  changed), and reconnect with 2 s → 30 s backoff. A daemon restart therefore produces the real
  transitions, not a flood.
- Container labels and env are **not** stored: labels are read only for the compose project name;
  `Config.Env` is never requested (that is where `POSTGRES_PASSWORD` lives). A code comment states this.

*Developer-environment collector*
- Environment variables: `HKCU\Environment` and
  `HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment` read via the P11 registry
  provider, woken by `RegNotifyChangeKeyValue`. Events `env/variable_added|removed|changed` with
  **values redacted by the P03 sanitizer** (env vars are the single most common secret store), and a
  dedicated `PATH` handling: `env/path_entry_added|removed` with one event per entry (not a diff of a
  2000-character string), because "something disappeared from PATH" is a classic silent breakage.
- Toolchain versions: a fixed allowlist of executable *names* (`node`, `bun`, `npm`, `pnpm`, `yarn`,
  `python`, `pip`, `java`, `dotnet`, `go`, `rustc`, `cargo`, `git`, `docker`, `kubectl`) resolved by
  scanning PATH directories for `<name>.exe`/`.cmd` — **no shell, no `PATHEXT` guessing beyond those two,
  no user-supplied names**. Version obtained by `std::process::Command` with the resolved absolute path,
  a single `--version` argument, `CREATE_NO_WINDOW`, stdin null, a 2 s timeout, and stdout truncated to
  256 bytes. Runs only (a) once at first start, (b) when a PATH entry changed, (c) every 6 hours.
  Events: `tool/version_changed` (`Node.js changed 20.11.0 → 22.6.0`), `tool/added`, `tool/removed`
  (resolved path disappeared).
- Git branch of watched folders: for each watched root (P10) that contains `.git`, read `.git/HEAD` on
  change and emit `dev/branch_switched` (`Switched branch in my-api: main → feature/auth`). Requires a
  narrow exception in the P10 ignore set for exactly `.git/HEAD` (the `.git/` directory stays ignored
  otherwise); implemented as an explicit allowlist entry, unit-tested so a future ignore-set edit cannot
  silently drop it.
- All three sub-sources live in one collector (`dev_env`) with one settings toggle, because a user
  thinks of them as one thing ("my dev setup"). Docker is a separate toggle.

**Non-functional**
- Docker event stream costs ~0 when idle (blocking read); the probe when absent is a single
  `GetFileAttributesW`-class check per 60 s.
- Toolchain scan ≤ 1.5 s total for 15 tools with the 2 s per-tool cap, runs off the fast tick
  (`cost_hint: Expensive`, gated by `QueryBusy`), and never on startup's critical path.
- Environment registry read ≤ 20 ms.
- Both collectors are absent-safe: no Docker, no git, and no toolchains present must all yield a
  `Running` collector emitting nothing, with zero error-level logs.

## Architecture

`collectors/docker.rs` (state machine: `Probing → Available{stream} → Reconnecting → Unavailable`),
`platform/windows/named_pipe.rs` (pipe open + HTTP/1.1 + chunked decoding),
`collectors/docker_protocol.rs` (pure parsing of the JSON event objects → `RawObservation`, fully
unit-testable), `collectors/dev_env.rs` (env vars + PATH + tools + git HEAD),
`platform/windows/tool_probe.rs` (PATH resolution + bounded process spawn),
`domain/tool.rs` (the tool allowlist as data). `DockerTransport` implementations:
`platform/windows/named_pipe.rs` (live) and `collectors/testing.rs::ScriptedTransport` (replays a captured
event fixture, including a mid-stream disconnect).

## Related Code Files

**Create** — `src-tauri/src/collectors/{docker.rs,docker_protocol.rs,dev_env.rs}`;
`src-tauri/src/platform/windows/{named_pipe.rs,tool_probe.rs}`; `src-tauri/src/domain/tool.rs`;
`src-tauri/tests/collector_docker.rs`, `src-tauri/tests/docker_protocol.rs`,
`src-tauri/tests/collector_dev_env.rs`, `src-tauri/tests/tool_probe.rs`;
`src-tauri/tests/fixtures/docker/{events-stream.ndjson,containers.json,images.json,
events-with-disconnect.ndjson,version.json}` (hand-written from the documented Docker Engine API event
shape, with a header comment stating they are protocol fixtures, not captures from a live daemon);
`src-tauri/tests/fixtures/dev_env/{path-before.txt,path-after.txt,env-before.json,env-after.json}`.
**Modify** — `src-tauri/src/collectors/registry.rs`, `src-tauri/src/collectors/ignore_set.rs`
(`.git/HEAD` allowlist), `src-tauri/src/collectors/testing.rs` (`ScriptedTransport`),
`src-tauri/src/platform/traits.rs`, `src-tauri/src/domain/event.rs` (new event types),
`src-tauri/Cargo.toml` (`Win32_Storage_FileSystem` for the pipe),
`src/features/settings/SettingsScreen.svelte` (two new toggles with §49 copy),
`ROADMAP.md` (unverified Docker-live path), `docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. `docker_protocol.rs` first: parse the fixture NDJSON into `RawObservation`s with full tests. This is
   the part that can be verified without a daemon, so it is where the correctness lives.
2. `named_pipe.rs`: pipe open with `Drop`-guarded handle, minimal HTTP/1.1 request writer, response
   header parse, chunked-body decoder with a 1 MB per-chunk sanity cap, and a `stream()` returning a
   `BufReader` over the pipe. Version negotiation with the 400-fallback.
3. `collectors/docker.rs`: the four-state machine, baseline from `containers/json`+`images/json`,
   diff-on-resync, compose-project coalescing, relationship emission, backoff.
4. `tool_probe.rs`: PATH split (respect quotes and trailing `;`), per-directory existence check for
   `<name>.exe` then `<name>.cmd`, bounded spawn, version-string extraction by a per-tool regex in
   `domain/tool.rs` (e.g. `node` → `^v?(\d+\.\d+\.\d+)`), and a hard refusal to spawn anything whose
   resolved path is not one of the allowlisted file names.
5. `collectors/dev_env.rs`: env diff (values redacted), PATH entry diff, tool version diff, git HEAD
   watch; all four emit through the normal normalizer.
6. Settings toggles: "Docker — containers, images and volumes starting, stopping and being rebuilt"
   and "Developer environment — environment variables, PATH, installed tool versions, and the branch of
   folders you watch. Values are never stored."
7. Tests: protocol fixtures; a scripted-transport run including a mid-stream disconnect asserting
   resync produces no duplicates; absence path (`Unavailable`, no errors); env/PATH/tool/git diffs;
   `tool_probe.rs` asserting a non-allowlisted name is never spawned and that a hanging fake tool is
   killed at 2 s.
8. Manual (limited by D3): confirm `Unavailable` state and the exact settings copy render; confirm a
   real PATH change and a real branch switch each produce one event; record in the phase file that the
   Docker-present path was **not** exercised on this machine.
9. Commits: (a) `feat: docker event protocol parsing`, (b) `feat: named pipe http transport`,
   (c) `feat: docker collector state machine`, (d) `feat: environment and path collector`,
   (e) `feat: toolchain version probing`, (f) `feat: git branch watching`,
   (g) `test: docker protocol and dev env suites`.

## Todo (commit boundaries)

- [ ] Docker protocol parsing with fixtures
- [ ] Named-pipe HTTP transport with chunked decoding
- [ ] Docker collector state machine + resync + compose coalescing
- [ ] Env var / PATH entry collector with redaction
- [ ] Bounded toolchain version probing
- [ ] Git HEAD watching with the ignore-set exception
- [ ] Test suites + absence-path verification

## Success Criteria

- [ ] `docker_protocol.rs`: every fixture event maps to the expected event type, entity id, and
      importance; a `die` with `exitCode: 137` yields `docker/container_died`, `important`, code in details
- [ ] Scripted transport with a mid-stream disconnect: resync emits events only for containers that
      actually changed (asserted count), and the collector returns to `Available`
- [ ] Compose fixture: 6 containers of one project restarting within 30 s → 1
      `docker/compose_project_changed` plus per-container events at `informational`
- [ ] No Docker present: collector reaches `Unavailable`, emits nothing, logs exactly one transition line
      in 5 minutes of running
- [ ] `Config.Env` is never requested (`rg -n "Env" src-tauri/src/collectors/docker*.rs` → comment only)
- [ ] Env fixture: `AWS_SECRET_ACCESS_KEY` changes → one event, both values `[REDACTED]`
- [ ] PATH fixture: one entry removed, one added → exactly two events naming the entries
- [ ] `tool_probe.rs`: a hanging fake tool is killed at 2 s and reported as `Unknown`, not an error; a
      name outside the allowlist is never spawned (asserted by a spawn-recording fake)
- [ ] Branch switch in a watched repo → `Switched branch in my-api: main → feature/auth`
- [ ] `ignore_set` test: `.git/HEAD` allowed, `.git/index` and `.git/objects/**` still ignored

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test docker_protocol --test collector_docker --test collector_dev_env --test tool_probe
cargo test --test collector_files --test redaction_leak
cargo clippy --all-targets -- -D warnings
cd /d/Project/.WhatChage && bun run check && bun run check:bindings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| D3: the Docker-present path is unverifiable here | Cannot be observed locally at all | Correctness is concentrated in the pure protocol parser (fixture-tested); the live path is a thin transport. The phase file and `ROADMAP.md` record "Docker live path unverified — needs a machine with Docker Desktop" instead of claiming it works. No test is written that pretends to exercise a daemon |
| Hand-rolled HTTP/chunked parsing is subtly wrong | Stream stalls or events are truncated after the first chunk | Chunk decoder is unit-tested against fixtures containing multi-chunk bodies, a chunk split across reads, and a trailing `0\r\n\r\n`. If it still misbehaves against a real daemon, the fallback is to poll `containers/json` every 3 s (cheap, already implemented for resync) and drop the stream — a config constant, not a rewrite |
| Docker API version mismatch on an older daemon | 400 with "client version too new" | Negotiation with a documented fallback version; the negotiated version is visible in diagnostics so a mismatch is diagnosable from a user report |
| Spawning tools is a code-execution surface | An unexpected process appears in P09's own event stream | Allowlisted file names only, absolute resolved paths, no shell, no user input, single fixed argument, `CREATE_NO_WINDOW`, 2 s timeout. The spawn-recording test is the guard. Self-events from these spawns are tagged and excluded from the timeline |
| A malicious PATH entry shadows `node.exe` | We execute an attacker binary | We only run tools the user's own shell would already run; the risk is not increased by observing. Mitigation: the resolved absolute path is stored in the event, so a shadowing entry is *visible* — which is the product's whole purpose |
| Env var redaction misses a value under an unusual name | Leak test on the env fixture fails | The fixture set includes benign-named high-entropy values; P03's entropy gate covers them, and any miss becomes a permanent fixture |
| `.git/HEAD` exception widens over time | Ignore-set test fails | The exception is a single exact-path allowlist entry with its own test; broadening it fails the test |

## Security Considerations

- Docker access is strictly read-only (`_ping`, `version`, `events`, `containers/json`, `images/json`);
  no endpoint that can create, start, stop, exec, or remove anything is ever called, and the named pipe is
  opened for read/write only because the protocol requires writing the request line.
- Container environment variables and secrets are never requested, so a compromised app cannot exfiltrate
  them from Docker.
- Environment-variable values are redacted before persistence; the settings copy states this so the user
  can trust the toggle.
- Process spawning is the only place the app executes anything: allowlisted names, absolute paths, no
  shell, no user-controlled arguments — and no IPC command can trigger it (it is timer/PATH-driven only).

---

### FILE: phase-18-onboarding-notifications-diagnostics.md

```yaml
---
phase: 18
title: "First-run onboarding, restrained notifications, privacy screen, diagnostics"
status: pending
priority: P1
effort: 6h
dependencies: [15]
---
```

## Overview

The trust layer (§42, §48, §50, §53, §58, §33, plus autostart). Onboarding earns the right to watch;
the notification policy earns the right to interrupt; the privacy screen and delete-all make the
"stays on this PC" promise inspectable; diagnostics make a user report actionable without asking for
their database. This lands after the analysis engine so onboarding can honestly describe what the app
will tell them, and so notifications can use real importance and confidence rather than placeholders.

## Requirements

**Functional**

*Onboarding (§42)* — three steps, skippable at any point, shown only when
`settings.onboarding_completed_at IS NULL`, and re-openable from Settings ("Show the welcome tour again").
1. **What this does** — headline "It worked before. What changed?", one paragraph, and the §72 promise
   line "Your history stays on this PC." No signup, no email field, no telemetry consent because there is
   no telemetry (the screen says so).
2. **What we watch** — the collector list rendered from the same `CollectorDescriptor` data as Settings
   (§49, DRY), each with its "what it watches / why it matters" text and a toggle; everything on by
   default except Docker (off when the pipe is absent) and the watched-folder collector (nothing to watch
   yet). Copy line: "We record that a value changed, never the value itself."
3. **Watch your project folders** — folder picker (P10 command) with 0-or-more selection, an explanation
   that config changes are the highest-value signal, and a "Start with Windows" toggle. Finishing writes
   `onboarding_completed_at`, creates the first automatic checkpoint (P14) so day-one comparisons work,
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

*Privacy screen (§48, §50)*
- "What's stored" panel with live counts from one command (`get_privacy_summary`): events by category,
  checkpoints + items, investigations, watched folders, config baselines, DB file size, oldest record
  date, log file size and path.
- Plain statements: no account, no network calls, no telemetry, no crash reporting; the list of
  categories recorded; the redaction promise with two concrete examples ("`DATABASE_URL` changed in
  `.env`" and "`AWS_SECRET_ACCESS_KEY` changed", both showing `[REDACTED]` values).
- Controls: retention sliders (events 7/30/90 days, default 30), "Delete everything now" with a typed
  confirmation ("delete") that drops and recreates the schema, clears logs, keeps settings, and shows
  "Your history has been deleted. We're watching again from now.", plus per-category "Delete this
  category's history" and "Stop watching and delete <folder>'s history".
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
event batch for notification evaluation), `src-tauri/src/commands/mod.rs` + specta builder
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

- [ ] Notification policy + rate limits + 24-hour replay test
- [ ] Toast delivery and click-to-event routing
- [ ] Purge operations (category, folder, all)
- [ ] Privacy screen with live counts and verify-for-yourself
- [ ] Diagnostics screen, export, degraded banner
- [ ] Onboarding flow with first checkpoint
- [ ] Autostart + minimized start
- [ ] B2 measurement #3 recorded

## Success Criteria

- [ ] `notification_policy.rs`: 4,000-event day → exactly 3 notifications, none from the 3,900 routine
      events, no two of one category within 30 min, none in the first 60 s, none while paused
- [ ] Clicking a notification focuses the window and opens the correct event's drawer (manual, recorded)
- [ ] `purge_all` leaves settings intact, empties every history table including checkpoints and
      investigations, and the UI shows the exact §50 confirmation copy
- [ ] `purge_category("network")` removes only network events and leaves others' counts unchanged
- [ ] `diagnostics_export.rs`: the export contains no fixture entity name, no fixture path, and no
      fixture secret; size ≤ 512 KB
- [ ] `get_privacy_summary` ≤ 60 ms at 100k events
- [ ] Fresh DB → onboarding shows; completing writes the timestamp, creates one automatic checkpoint, and
      Home shows "We just started watching."
- [ ] Onboarding completable with keyboard only; contrast test passes on all three steps
- [ ] Autostart enabled → the registry/plugin state is set and a `--minimized` launch shows only the tray
      (manual, recorded)
- [ ] Killing one collector shows the degraded banner and Diagnostics names the failed collector
- [ ] B2 numbers (idle working set, window shown / window hidden, peak) recorded in the phase file

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
| `purge_all` leaves a WAL with old data | The `-wal` file still contains fixture strings after purge | Purge does `wal_checkpoint(TRUNCATE)` then `VACUUM`; the purge test greps the DB, `-wal`, and `-shm` files for a fixture string |
| Diagnostics export leaks history | The export test finds an entity name | The export is built from an explicit field allowlist (counts and versions only), not by serializing a struct; the test is the gate |
| Adding the `opener` plugin widens the attack surface | `capabilities_allowlist` test fails | Scoped to `reveal-item-in-dir` for the app data dir only; the allowlist test is updated to permit exactly that one permission and still reject `fs:`, `shell:`, `http:`, `process:` |
| Onboarding blocks a user who wants to look first | Support-style complaint / dead end | Every step is skippable, nothing is required, and the tour is re-runnable from Settings |

## Security Considerations

- Two new capabilities only: `notification:default` and a path-scoped `opener` reveal. The allowlist test
  is tightened to name them explicitly so a future broad `fs:`/`shell:` addition fails the build.
- `export_diagnostics` is an allowlist-built text file with no event content; log lines inside it already
  went through the P03 redacting writer, so it is doubly filtered.
- `purge_all` requires a typed confirmation in the UI and is irreversible — the dialog says so plainly,
  and no command can trigger it without that user action (there is no scripting surface).
- Notification bodies contain `user_friendly_summary` text, which is sanitized; a toast therefore cannot
  surface a secret on a lock screen.
- Autostart writes only the current user's own `Run` entry (or the plugin equivalent); no service, no
  scheduled task, no elevation.

---

### FILE: phase-19-performance-accessibility-hardening.md

```yaml
---
phase: 19
title: "Performance at 10k/100k/1M events, resource budgets, accessibility conformance"
status: pending
priority: P1
effort: 7h
dependencies: [16, 17, 18]
---
```

## Overview

Turn the standing budgets into enforced gates (§61, §62, §63). Three seeded corpora (10k, 100k, 1M
events) drive query, memory, and DB-size assertions; a soak run proves the collectors do not leak over
hours; and a full accessibility pass makes every screen keyboard-operable and screen-reader-sane. This
phase adds no features — it either passes, or it produces a specific, measured optimization commit.

## Requirements

**Functional (as gates)**
- Seeder: `cargo run --bin seed_history --features devtools -- --events 1000000 --days 90` builds a
  realistic corpus (category mix matching the P06 calmness distribution, 8% clustered bursts, 3000
  entities, 15% with relationships, 40 checkpoints, 20 investigations) writing through the real
  `Sanitized` path and tagged `source_collector = "synthetic"` so it can be purged in one statement.
  Deterministic with a seed so numbers are comparable between runs.
- §61 query gates at each size (p95 over 30 runs, warm, asserted in `tests/perf_queries.rs` which is
  `#[ignore]`d by default and run explicitly in this phase's gate):

  | Query | 10k | 100k | 1M |
  |---|---|---|---|
  | `get_events` first page (200 rows) | ≤ 10 ms | ≤ 20 ms | ≤ 45 ms |
  | `get_events` page 10 (keyset) | ≤ 10 ms | ≤ 20 ms | ≤ 45 ms |
  | `get_dashboard_summary` | ≤ 15 ms | ≤ 30 ms | ≤ 80 ms |
  | `search_events` (2-token) | ≤ 40 ms | ≤ 90 ms | ≤ 250 ms |
  | `get_event_details` + prior occurrences | ≤ 10 ms | ≤ 15 ms | ≤ 30 ms |
  | `start_investigation` (5k window) | ≤ 400 ms | ≤ 600 ms | ≤ 1500 ms |
  | `compare_checkpoint` (3k items) | ≤ 150 ms | ≤ 200 ms | ≤ 300 ms |
  | `get_privacy_summary` | ≤ 30 ms | ≤ 60 ms | ≤ 200 ms |

- Every query in the table has an `EXPLAIN QUERY PLAN` assertion proving it uses an index and never
  scans `events` (`tests/perf_query_plans.rs`); a plan containing `SCAN TABLE events` fails the test.
  A failing budget is fixed by an index or a query change, never by relaxing the number without the
  user's decision.
- DB-size gates: ≤ 12 MB at 10k, ≤ 110 MB at 100k, ≤ 1.1 GB at 1M (≈ 1.1 KB/event), plus a
  `page_count`/`freelist_count` check after retention purge proving `incremental_vacuum` reclaims space.
  If 1M exceeds the budget, the response is column-level: drop redundant text from `details`, not a
  schema redesign.
- Resource gates measured with all collectors running:
  - Idle CPU ≤ 1% average over 10 minutes (sampled from `GetProcessTimes` in a `#[ignore]`d test binary
    and cross-checked once manually in Task Manager).
  - Working set: recorded at the three B2 checkpoints (window shown, window hidden, after 4 hours) and
    compared to the 80 MB target — pass/fail reported honestly with the WebView2 share broken out
    (`get_working_set` for the main process and the WebView2 child processes enumerated by parent pid).
  - Cold start to visible window ≤ 2.0 s; warm start ≤ 1.2 s; tray-only autostart ≤ 900 ms to tray icon.
  - 4-hour soak (`scripts/soak.ps1` driving `simulate_event` plus real collectors): working set growth
    ≤ 15%, handle count flat ±5%, thread count flat, no `Failed` collectors, zero dropped write batches.
- Frontend gates: 60 FPS scroll on a 100k timeline (measured with a `#/dev/perf` harness recording
  `requestAnimationFrame` deltas over 600 frames; p95 ≤ 20 ms), Change Map median frame ≤ 16.6 ms at
  3000 nodes, initial JS bundle ≤ 400 KB gzipped excluding `three`, `three` lazily imported so the
  Timeline route never pays for it (`bun run build` + a bundle-size assertion script).
- §62 accessibility conformance:
  - Keyboard: every interactive element reachable in a logical order; visible focus ring on all of them
    (P02 token); `Esc` closes every overlay; `Ctrl+K` search; arrow-key navigation in the timeline list
    and the search results; no keyboard trap (verified per screen).
  - Screen reader: one `<h1>` per screen, landmark regions (`banner`/`navigation`/`main`), `aria-live="polite"`
    for the live-events pill and toast, `aria-live="assertive"` reserved for the degraded banner only,
    accessible names on all icon-only buttons, table semantics for the before/after and collector-status
    tables. Verified manually with Narrator on Home, Timeline, Details drawer, Investigate, Checkpoints,
    Privacy, Diagnostics, and Settings — findings recorded per screen in the phase file.
  - Contrast: automated WCAG AA check over every token pair used by a component (extends the P02 test to
    walk a manifest of actual pairs, so a new component cannot introduce an untested pair).
  - Motion: `prefers-reduced-motion` removes transitions/animations app-wide (one CSS media block plus the
    Change Map's on-demand rendering) — asserted by a test that computes styles with the media feature
    forced.
  - Zoom/scale: usable at 200% browser zoom and at Windows 150% DPI scaling at 900×600 (no clipped
    controls, no horizontal scroll on the primary column).
- Fixes discovered here land as their own commits in this phase; anything that would change behaviour or
  scope is written up instead of silently applied.

**Non-functional**
- The perf suite must be runnable in one command and must clean up its corpora (`--purge-synthetic`),
  so it can be re-run before release without polluting real history.
- Measured numbers are recorded in the phase file next to the budget, with the machine's identity kept
  generic (no serials, no hostnames).

## Architecture

`src-tauri/src/bin/seed_history.rs` (devtools feature), `src-tauri/tests/perf_queries.rs`,
`tests/perf_query_plans.rs`, `tests/perf_db_size.rs`, `src-tauri/src/bin/resource_probe.rs`
(idle CPU / working set / handles sampler), `scripts/soak.ps1`, `scripts/bundle-size.mjs`,
`src/features/dev/PerfHarness.svelte` (`#/dev/perf`: scroll-FPS recorder, map frame timer, route bundle
report). Accessibility work is spread across existing components; the new artifacts are
`src/lib/a11y/contrast-pairs.ts` (the manifest), `src/lib/a11y/contrast.test.ts`,
`src/lib/a11y/reduced-motion.test.ts`, and `docs/accessibility-audit.md` (the per-screen Narrator record).

## Related Code Files

**Create** — `src-tauri/src/bin/{seed_history.rs,resource_probe.rs}`;
`src-tauri/tests/{perf_queries.rs,perf_query_plans.rs,perf_db_size.rs,soak_support.rs}`;
`scripts/{soak.ps1,bundle-size.mjs}`; `src/features/dev/PerfHarness.svelte`;
`src/lib/a11y/{contrast-pairs.ts,contrast.test.ts,reduced-motion.test.ts,focus-order.test.ts}`;
`docs/accessibility-audit.md`; `docs/performance-baseline.md`.
**Modify** — `src-tauri/Cargo.toml` (`devtools` feature, two bins), `src-tauri/src/storage/migrations/`
(only if an index is added — as `002_perf_indexes.sql`, never by editing `001_initial.sql`),
`src/styles/motion.css` (reduced-motion block), any component with an a11y finding,
`src/three/changeMap/createScene.ts` (lazy import boundary),
`src/app/router.ts` (dynamic import for the map route), `package.json` (`test:perf`, `check:bundle`),
`ROADMAP.md`, `docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. `seed_history.rs` with the deterministic generator and `--purge-synthetic`; verify the 10k corpus looks
   like real history in the UI (spot check) before trusting the bigger ones.
2. `perf_query_plans.rs` before `perf_queries.rs` — a plan regression is cheaper to diagnose than a
   timing regression, and it explains any timing failure that follows.
3. Run the 10k gate; fix anything failing; then 100k; then 1M. Record every number in
   `docs/performance-baseline.md` as it is measured, not at the end.
4. Add `002_perf_indexes.sql` only if a plan assertion demands it, with a migration test proving it
   applies to an existing 100k DB in ≤ 5 s.
5. `resource_probe.rs` + the 10-minute idle CPU run + the three working-set readings (main + WebView2
   children) → B2 verdict written into the phase file and `docs/performance-baseline.md`.
6. `scripts/soak.ps1`: 4 hours, samples every 60 s to a CSV, asserts the growth/flatness rules at the end
   and prints a summary. Run it once for real; attach the summary numbers.
7. Frontend: `#/dev/perf` harness, scroll-FPS at 100k, map frames, then `bundle-size.mjs` gate and the
   lazy-import split for `three`.
8. Accessibility sweep, screen by screen: keyboard order and traps, focus rings, landmarks, live regions,
   accessible names, table semantics; then the contrast-pair manifest test and the reduced-motion test;
   then the Narrator pass with findings recorded in `docs/accessibility-audit.md`.
9. Fix commits, each naming the measured before/after.
10. Commits: (a) `feat: deterministic history seeder`, (b) `test: query plan assertions`,
    (c) `test: query performance gates`, (d) `perf: <specific fix>` (as needed),
    (e) `test: db size and vacuum gates`, (f) `feat: resource probe and soak script`,
    (g) `feat: perf harness route`, (h) `perf: lazy load three on the map route`,
    (i) `fix: accessibility findings <screen>` (one per screen with findings),
    (j) `test: contrast manifest and reduced motion`, (k) `docs: performance baseline and a11y audit`.

## Todo (commit boundaries)

- [ ] Deterministic seeder with purge
- [ ] Query-plan assertions green
- [ ] 10k / 100k / 1M query gates green (with any needed index migration)
- [ ] DB-size and vacuum gates green
- [ ] Idle CPU, working set (B2 verdict), start-time numbers recorded
- [ ] 4-hour soak run completed and summarized
- [ ] Frontend FPS, bundle-size gate, lazy `three`
- [ ] Accessibility sweep + contrast manifest + reduced-motion tests + Narrator record

## Success Criteria

- [ ] Every cell of the §61 table passes at its size, or has a recorded, user-approved exception
- [ ] `perf_query_plans.rs`: zero `SCAN TABLE events` across all measured queries
- [ ] DB size within budget at all three sizes; post-purge `freelist_count` shrinks after
      `incremental_vacuum`
- [ ] Idle CPU ≤ 1% over 10 minutes with all collectors enabled
- [ ] B2: the three working-set numbers and the WebView2 share are recorded, with an explicit
      pass/fail against 80 MB and no redefinition of the target
- [ ] Cold start ≤ 2.0 s, warm ≤ 1.2 s, tray-only ≤ 900 ms
- [ ] Soak: working set growth ≤ 15%, handles ±5%, threads flat, zero dropped batches, no failed collectors
- [ ] 100k timeline scroll p95 frame ≤ 20 ms; map median ≤ 16.6 ms at 3000 nodes
- [ ] Initial bundle ≤ 400 KB gzipped excluding `three`; the Timeline route does not load `three`
      (asserted by `bundle-size.mjs`)
- [ ] Keyboard-only traversal of all eight screens with no trap and a visible focus ring throughout
- [ ] Contrast manifest test covers every pair used by a component and passes AA
- [ ] `prefers-reduced-motion` removes all transitions and stops the map's continuous loop
- [ ] Usable at 900×600 with Windows 150% scaling and at 200% zoom
- [ ] `docs/accessibility-audit.md` records a Narrator result per screen

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo run --bin seed_history --features devtools -- --events 10000 --days 30 --seed 42
cargo test --test perf_query_plans -- --include-ignored
cargo test --test perf_queries --test perf_db_size -- --include-ignored
cargo run --bin seed_history --features devtools -- --purge-synthetic
cargo clippy --all-targets --all-features -- -D warnings
cd /d/Project/.WhatChage
bun run check && bun run test && bun run build && bun run check:bundle
powershell -ExecutionPolicy Bypass -File scripts/soak.ps1 -Hours 4   # run once, attach summary
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| A 1M-event budget is unreachable on this hardware | `perf_queries` fails at 1M only | Diagnose with the query plan first (usually a missing composite index); if the shape is right and it is simply data volume, present the measured number to the user as a scope question. Never relax the assertion silently, and never cap history below the user's chosen retention without their decision |
| B2 fails and pressure builds to hack around it | Working set > 80 MB | Report the split (main process vs WebView2 children) and the three readings. Options presented, not chosen unilaterally: accept a higher target, hide the window aggressively, or defer the UI to a lighter shell. Do not disable collectors to make a number |
| Seeded corpus is unrealistically uniform, so gates pass but real use is slow | Real-machine timeline feels slower than the 100k gate | Seeder mirrors the P06 category distribution and includes burst clusters and entity reuse; a spot-check against this machine's real history (a week of events) is part of step 1 |
| An added index slows writes | Write batch duration rises in the soak | Batch-write timing is part of the soak assertions; an index that costs > 10% write throughput is replaced by a query change instead |
| 1M seeding takes so long it is skipped | The 1M column is never actually run | Seeder writes in batched transactions (target ≤ 4 minutes for 1M); the gate command is in the phase's Verification Gate so skipping it fails the phase |
| Accessibility fixes cause visual regressions | Component tests or the contrast test fail | Each fix is its own commit with the failing/passing test named, so a regression is bisectable |
| Narrator findings are subjective | Disagreement about "sane" output | The record is per-screen and concrete (announced name, role, and state for each control); ambiguous items are listed as follow-ups in `ROADMAP.md` rather than silently passed |

## Security Considerations

- The seeder and probes are behind the `devtools` cargo feature and are not compiled into the release
  binary (asserted by a release-build symbol check in P20).
- `docs/performance-baseline.md` and the soak CSV contain no hostnames, serials, usernames, or paths —
  only counts and timings (the same allowlist discipline as the diagnostics export).
- The perf harness route lives under `#/dev/*`, which is compiled out of release builds along with the
  rest of the dev harness.
- Synthetic events are labelled and purgeable, so a perf run cannot be mistaken for real history in an
  investigation.

---

### FILE: phase-20-installer-updater-release.md

```yaml
---
phase: 20
title: "NSIS installer, in-place upgrade path, release audit and requirement traceability"
status: pending
priority: P1
effort: 6h
dependencies: [19]
---
```

## Overview

Ship it: a per-user NSIS installer that needs no administrator, an upgrade path that migrates an
existing database instead of orphaning it, an uninstall that asks before deleting a user's history, and
a final audit that maps every requirement §1–§91 to the phase and test that satisfies it. The MVP
deliberately ships **no automatic update check** — an app whose core promise is "no network calls, your
history stays on this PC" cannot silently contact an update server. That is recorded as decision D-09
with its consequence (manual download), and it is raised as an open question rather than assumed.

## Requirements

**Functional**
- Bundle: `bundle.targets = ["nsis"]` (already set in P01). NSIS config in `tauri.conf.json`:
  `installMode: "currentUser"` (per-user, no UAC), `displayLanguageSelector: false`,
  `installerIcon`, `headerImage`/`sidebarImage` if the assets exist (skip rather than invent art),
  `license` omitted unless a LICENSE file exists, `compression: "lzma"`. Verify every key against
  `src-tauri/gen/schemas/desktop-schema.json` before writing it — schema-verified keys only.
- Artifact naming: `What Changed_<version>_x64-setup.exe`; version comes from `tauri.conf.json`
  `version`, which is the single source (Cargo.toml and package.json read it or are asserted equal by
  `scripts/check-versions.mjs`, failing the build on drift).
- Data location and upgrade: history lives in `%APPDATA%\dev.whatchanged.app\` (resolved via Tauri's
  `app_data_dir`, never a hardcoded path). Installing a newer version over an older one must leave that
  directory untouched and let migrations run at next start. Verified by an actual two-build test: install
  v0.1.0, seed 10k events, install v0.1.1 (with an added migration `003_test_upgrade.sql` used only in
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
- Release build hygiene:
  - `cargo build --release` with `panic = "abort"`, `lto = "thin"`, `codegen-units = 1`,
    `strip = "symbols"` in `[profile.release]`; `opt-level = "s"` evaluated against the perf gates —
    keep whichever passes P19's budgets and record the choice.
  - `devtools` feature **off**: assert the release binary contains no `simulate_event`, no
    `seed_history`, no `#/dev/` route (symbol/string scan in `tests/release_audit.rs`, run against the
    built binary and the built frontend assets).
  - Frontend built with `bun run build`; no source maps in the shipped bundle; `withGlobalTauri: false`
    confirmed; CSP present in the final `tauri.conf.json`.
- Release audit (`docs/release-audit.md`, generated by hand in this phase):
  1. **Requirement traceability table**: every requirement §1–§91 → the phase that implements it → the
     test or manual check that proves it → status (Done / Deferred-by-user / Not-verifiable-here). The
     only permitted deferrals are the user's own: §26 health checks and §75 future features. Docker's
     live path is marked Not-verifiable-here (D3).
  2. **Non-negotiables checklist**: no Electron, no Go, no second process, no service, no account, no
     cloud, no telemetry, no elevation, offline at runtime — each with the evidence that proves it
     (dependency list, process tree screenshot description, capability allowlist test, absence of any
     HTTP client in `Cargo.lock`, verified by `scripts/check-no-network.mjs` grepping the lockfile for
     `reqwest`/`hyper`/`ureq`/`isahc` and failing if any appears outside a dev-dependency).
  3. **Measured numbers**: the P19 baseline table plus the B2 verdict.
  4. **Known gaps**: B3 short-lived processes, WinHTTP machine proxy, XML config diffing if it degraded,
     Docker live path, unsigned installer, §26/§75 deferrals.
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
**Delete** — nothing in the repo; the rehearsal migration `003_test_upgrade.sql` is created and reverted
within step 4 and must not exist in the final tree (asserted by the migration test's file list).

## Implementation Steps

1. `scripts/check-versions.mjs` and `scripts/check-no-network.mjs` first — they are cheap gates that must
   be true from now on.
2. NSIS config keys verified against `gen/schemas/desktop-schema.json`, then `bun run tauri build`;
   inspect the produced installer name, size, and that it installs without a UAC prompt.
3. Install into a clean user context (a second Windows user account, or a VM if available; if neither is
   available, record that the clean-machine install was verified only on this account and list it as a
   gap rather than claiming otherwise).
4. Upgrade rehearsal: build v0.1.0, install, seed 10k, add `003_test_upgrade.sql`, bump to v0.1.1, build,
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

- [ ] Version-drift and no-network lockfile guards
- [ ] Schema-verified NSIS configuration, installer built
- [ ] Release-artifact audit test (no devtools, CSP, no source maps)
- [ ] Upgrade rehearsal with a real migration
- [ ] Uninstall data decision implemented and documented
- [ ] `docs/release-audit.md` traceability §1–§91
- [ ] Manual acceptance walkthrough executed and recorded
- [ ] One-command release script green end to end

## Success Criteria

- [ ] Installer builds, installs with **no UAC prompt**, and launches to a visible window ≤ 5 s
- [ ] `check-no-network.mjs` finds no HTTP client in `Cargo.lock` runtime dependencies
- [ ] `check-versions.mjs` passes with all three version sources equal
- [ ] `release_audit.rs`: zero dev-only strings/symbols in the release artifact; CSP present;
      `withGlobalTauri: false`; no `.map` files in `dist/`
- [ ] Upgrade rehearsal: 10k events survive a version-over-version install and the new migration applies
- [ ] Uninstall does not delete history unless the user explicitly opts in
- [ ] `docs/release-audit.md` has a row for every requirement §1–§91 with a named test or manual check;
      the only Deferred rows are §26 and §75; the only Not-verifiable rows are Docker-live and any
      recorded clean-machine gap
- [ ] All 15 acceptance steps pass, including Resource Monitor showing zero network bytes for the app
      during a 10-minute session
- [ ] `powershell -File scripts/release.ps1` completes green from a clean tree

## Verification Gate

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

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| An auto-updater would contradict the no-network promise (D-09) | Requirement review asks "where is update checking?" | Ship none; document the manual-download path; raise it as an open question with three options (none / manual "check for updates" link that only opens the browser / opt-in updater with an explicit consent screen). Do not add a silent update check under any circumstance |
| SmartScreen blocks users on an unsigned installer | "Windows protected your PC" on a clean machine | Documented in `README.md` with the exact click path; signing is a recorded future step with its concrete requirements. No pretend-signing, no publisher spoofing |
| WebView2 bootstrapper needs network at install time | Install fails offline | Documented as install-time-only; the alternative bundle modes and their size cost are presented to the user as a decision, not chosen here |
| NSIS config key invented and silently ignored | Build succeeds but the setting has no effect | Every key is checked against `gen/schemas/desktop-schema.json` first; unknown keys are not written |
| Uninstall deletes a user's history | Data gone after uninstall | Default is keep; the opt-in checkbox is attempted, and the fallback is keep-always. This is a hard rule, not a preference |
| Upgrade orphans the database (identifier or path change) | Post-upgrade event count is 0 | Identifier `dev.whatchanged.app` is frozen from P01 and asserted by the upgrade rehearsal; changing it later would require a data-migration phase |
| A requirement has no evidence at audit time | A traceability row with an empty test column | The row blocks the release; the work returns to its phase. The audit is not allowed to mark it Done on the basis of "looks implemented" |
| Release script hides a failure | Green run with a skipped step | `release.ps1` uses `$ErrorActionPreference = "Stop"`, echoes each step, and prints a final summary listing every gate with its exit code |

## Security Considerations

- Per-user install (`currentUser`) means no elevation at install time and no writes outside the user's
  profile — consistent with the non-elevated runtime promise.
- The no-network lockfile guard is the mechanical proof of the offline claim; it runs on every release.
- Release binaries exclude the `devtools` feature, so `simulate_event`, the seeder, and the `#/dev/*`
  routes cannot be reached in a shipped build.
- The unsigned-binary limitation is disclosed rather than worked around; users are told what they will
  see and why.
- `docs/release-audit.md` contains no machine identifiers, and the acceptance walkthrough instructs the
  operator to redact paths that contain a real username before pasting results.

---

## Phase decomposition rationale

**§69 binding order → plan phases.** The user's Phase 0–8 order is preserved; a user phase is split
where it exceeds one reviewable unit, and no dependency logic is reordered.

| §69 phase | Plan phases | Why split / note |
|---|---|---|
| P0 Foundation | 01 toolchain, 02 design system + shell, 03 redaction, 04 domain + persistence, 05 typed IPC + tray | Five commit-sized units; each independently verifiable. Splitting keeps a failed toolchain install from blocking design work |
| P1 Event engine + UI | 06 event engine + queries, 07 timeline/search UI | Rust query layer and the Svelte surface are different review surfaces and different test kinds |
| P2 First collectors | 08 collector runtime, 09 process + ports, 10 files + config, 11 services + applications | The runtime chassis is its own unit so collector phases are pure additions; the four cheap/measured collectors then land in cost order |
| P3 Remaining collectors | 12 updates + drivers, 13 network + startup + event log | Grouped by cadence class: 12 is notification+rare-sweep, 13 is subscription+rare-sweep and also supplies the corroboration source 09/12 need |
| P4 Checkpoints | 14 | Single unit; depends on collectors exposing `state_set` |
| P5 Analysis | 15 | Deliberately one large unit (12h) — splitting the scorer from the explainer would let over-claiming copy ship before the confidence classes exist |
| P6 Visualization | 16 | Consumes P15 output only |
| P7 Docker + dev mode | 17 | Independent of 12–16; depends only on the P08 runtime, so it could run in parallel on another machine |
| P8 Polish + release | 18 onboarding/notifications/diagnostics, 19 performance + a11y, 20 installer + audit | Trust surfaces, then measured hardening, then the artifact and the §1–§91 traceability audit |

**Why redaction (03) lands before persistence (04).** If the schema and the writer exist first, the
natural next move is "store events now, add scrubbing later" — and every event written in between is a
permanent leak in a 30-day database. Building `Sanitized<T>` first makes the writer's signature
`fn write(&self, event: Sanitized<SystemEvent>)`, so *unredacted data cannot compile* into storage. The
§60 leak tests then exist before there is anything to leak, and every later collector inherits the
guarantee for free rather than being individually audited.

**Why the toolchain is a phase, not a preamble.** Rust is absent and three MSVC installs are ambiguous
(B1). Discovering a broken linker while debugging a collector conflates two failures. Phase 01 ends with
a proven `cargo build` + `tauri dev` window and a pinned `rust-toolchain.toml`, so every later failure is
about the code.

**Why the UI is built on synthetic data (06→07) before real collectors (08→13).** The timeline must
survive 100k events and six states. Waiting for real collectors to produce volume means the scale
problems surface last, when they are most expensive. `simulate_event` + the seeder let 07 be finished and
measured, so collectors land against a UI that already works.

**Why collectors land in measured-cost order.** 09 (6.2 ms) and 11 (3.3 ms / 50 ms) prove the runtime on
cheap sources; 10 proves the privacy path on the riskiest source; only then do 12–13 introduce the 1.3–2.1 s
enumerations that require notification-driven cadence and the `QueryBusy` gate. The scheduler is therefore
exercised by real cost pressure only after it has been unit-proven.

**Effort:** 131h total (01:5, 02:6, 03:4, 04:7, 05:6, 06:6, 07:8, 08:5, 09:6, 10:7, 11:5, 12:7, 13:8,
14:5, 15:12, 16:9, 17:6, 18:6, 19:7, 20:6). Each phase is 4–12h — one to three focused sessions, with
commit boundaries listed per phase so review never faces a 3000-line diff (§89).

---

## Risks carried forward

**Research §5 unverified assumptions — each is a live risk with a settling phase, a signal, and a
pre-decided response. None is restated in this plan as an established fact.**

| # | Unverified assumption | Settled in | Signal it is false | Pre-decided response |
|---|---|---|---|---|
| 1 | `tauri-specta` 1.0.2 builder API is as documented | 05 | `cargo doc` shows a different builder; `export_bindings` will not compile | Use the documented API; if binding export is unworkable, switch to `ts-rs` 12.0.1 for types + a hand-written command signature module, with `check:bindings` still gating drift (D-07) |
| 2 | `rusqlite_migration` `M::up` signature and `to_latest` semantics | 04 | Migration setup does not compile or applies out of order | Hand-rolled migration runner over `user_version` (≈60 lines, one `PRAGMA` + a match on version) — the schema files and tests are unchanged |
| 3 | FTS5 is compiled into `rusqlite` `bundled` | 04 | `PRAGMA compile_options` lacks `ENABLE_FTS5` | Use the `search_terms` column + `LIKE`/prefix index path already specified as D-04; the search budget in §61 was set for that path |
| 4 | `notify-debouncer-full` 0.7.0 constructor arity | 10 | Constructor call does not compile | Use the documented constructor; if the debouncer is unusable, debounce in `collectors/files.rs` with a 500 ms coalescing map keyed by path (the behaviour the debouncer provides) |
| 5 | `@tanstack/svelte-virtual` works with Svelte 5 runes (§5.10) | 07 | Spike errors or rows do not recycle | Ship `WindowedList.svelte`; drop the dependency (D-08) |
| 6 | `tauri-plugin-autostart` has a Tauri-2-compatible pinned version | 18 | No version resolves, or the API differs | Write the current user's own `HKCU\...\Run` value via the P11 registry provider; never a scheduled task |
| 7 | WUA COM interfaces are exposed by `windows` 0.62 | 12 | `IUpdateSearcher` absent from the crate | Generate bindings with `windows-bindgen` and commit them under `platform/windows/`; never shell out to PowerShell |
| 8 | `NotifyServiceStatusChangeW` is usable non-elevated | 11 | Registration returns a failure HRESULT | Pure 5 s poll (already the designed fallback); worst case is 5 s latency |
| 9 | `CM_Register_Notification` callback ABI works from Rust | 12 | Non-zero return or a crash in the fault test | Tier down to `RegNotifyChangeKeyValue` on the Class key tree, then to the 6-hour sweep only; the active tier is shown in Diagnostics |
| 10 | B2: < 80 MB idle RAM is reachable with WebView2 | 01, 05, 18, 19 (four measurements) | Any measurement exceeds 80 MB | Report all readings with the WebView2 child-process share broken out and escalate as a scope question with three options. Never silently redefine the target, never disable collectors to make the number |
| 11 | D2: TypeScript 7.0.2 works with `svelte-check` and the Vite plugin | 01 | `bun run check` fails or emits plugin-version errors | Pin back to the latest 5.x that passes and record the pinned version + the reason in the phase file |

**Blockers and decisions carried into execution**

| Item | Status entering execution | Response if it bites |
|---|---|---|
| B1 — Rust absent; 3 ambiguous MSVC installs | Verified absent this session (`rustc`, `cargo`, `rustup` all missing) | Phase 01 installs via `win.rustup.rs` with an explicit MSVC toolchain, then `scripts/verify-toolchain.ps1` (vswhere) proves the linker. If linking still fails, builds run through `scripts/dev-shell.cmd` (vcvars64) and that becomes the documented build entry point |
| B3 — non-admin process events are diff-polled | Accepted limitation, not solvable non-elevated (`Win32_ProcessStartTrace` measured Access-denied) | Documented in the collector descriptor, the timeline copy, and `docs/release-audit.md` known gaps. No elevation prompt is added, and no plan text claims sub-second process coverage |
| D1 — repo folder is misspelled `.WhatChage` | Confirmed: `$ROOT` contains only `.git` and `plans`, zero commits | Nothing derives an identifier, path, or product string from the folder name; `identifier` is `dev.whatchanged.app` and `productName` is "What Changed", both frozen in phase 01 |
| D3 — Docker Desktop absent | Live Docker path cannot be exercised here | Correctness concentrated in the fixture-tested protocol parser; the live transport is marked Not-verifiable-here in the release audit. No test pretends to talk to a daemon |
| Analysis over-claim | Structural risk of the product, not of a library | Class-gated explanation constructor + forbidden-phrase test + the §59 fixtures. A new over-claiming phrase found in review joins the forbidden list permanently |
| 1M-event budgets | Unmeasured until phase 19 | Query-plan assertions first, then index or query fixes; a genuinely unreachable budget is escalated with measured numbers, never silently relaxed |
| Unsigned installer / no auto-update | Known and deliberate | Disclosed in `README.md` and the release audit; signing and any update mechanism are recorded future steps, with the update question raised below |

---

## Unresolved questions

1. **B2 memory target (80 MB).** WebView2 alone typically dominates this figure. If the phase-19
   measurement lands above 80 MB, which do you want: accept a revised target with the measured number,
   aggressively unload the webview when the window is hidden (adds a re-init cost and a visible relaunch
   delay), or keep the target and cut UI scope? I will not pick for you, and I will not redefine the
   target silently.
2. **Update mechanism (D-09).** The MVP ships no update check because that would contradict "no network
   calls". Options: (a) none — manual download only; (b) a "Check for updates" button that opens a
   browser to a release page (the app itself still makes no request); (c) an opt-in updater behind an
   explicit consent screen. Default in this plan is (a).
3. **Offline installer.** The default WebView2 `downloadBootstrapper` needs network *at install time*.
   Do you want the larger `embedBootstrapper`/fixed-runtime bundle instead (bigger installer, fully
   offline install)?
4. **Uninstall data.** Default is to keep history on uninstall and document the folder. Do you want the
   opt-in "also delete my history" checkbox pursued even if it requires an NSIS template hook?
5. **Command-line capture.** Process command lines would materially improve the analysis engine
   ("`node server.js --port 4000`") but are the single richest secret carrier. This plan excludes them.
   Do you want them behind an off-by-default toggle with redaction, or excluded entirely?
6. **XML config diffing.** If `quick-xml` does not resolve cleanly, XML files degrade to
   `file/modified` (no key-level diff). Acceptable, or should XML be a hard requirement?
7. **WinHTTP machine proxy.** Phase 13 reads the per-user WinINET proxy keys (what browsers and most
   tooling use). Machine-level WinHTTP proxy changes would be missed. Add it, or leave it as a
   documented gap?
8. **Retention default.** 30 days for events / 7 for verbose / 90 for investigations, with checkpoints
   exempt. At 1M events that is ~1.1 GB. Confirm, or prefer a size-based cap as the primary control?
9. **Clean-machine verification.** Phase 20 verifies the installer on this account; a second user
   account or VM would prove the clean-install path properly. Is one available, or should that stay a
   documented gap?

---

Status: DONE
Summary: Candidate 4 is complete — 20 commit-sized phases (131h) that install and prove the toolchain
first, weld redaction shut as a typed chokepoint before any storage exists, scale the UI on synthetic
data, land collectors in measured-cost order, and gate the deterministic analysis engine on §59/§74
ranking fixtures plus an over-claim test; B1/B2/B3, D1–D3, and all 11 research §5 unverified assumptions
are carried as named risks with signals and pre-decided fallbacks, and nine scope questions are left to
the user rather than answered unilaterally.







































