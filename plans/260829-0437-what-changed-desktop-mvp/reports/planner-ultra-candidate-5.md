# Candidate 5 — Redaction-first spine: prove the toolchain, weld one enforced ingest path, then grow collectors onto it

## Thesis

One ingest path, built once, in this order: **toolchain proof → redaction chokepoint → storage → typed IPC → UI shell → event engine**. Every later collector is a leaf that can only reach disk through `redact::sanitize() -> SanitizedEvent`, which the repository is the only consumer of — making a privacy leak a compile error rather than a review finding. The three blockers get dedicated, dated gates instead of prose: B1 is phase-01 step 1 with a linker-disambiguation fallback, B2 is measured on the bare scaffold *before* 40 KLOC exist to blame, B3 is encoded as a machine-readable `coverage_limits` field that renders into Settings copy and `COLLECTORS.md` so the blind spot cannot be silently forgotten. Collector order follows measured cost (cheap-to-diff first, notification-driven expensive sweeps later) so the §37 CPU budget is never retro-fitted.

## Plan metadata

```yaml
---
title: "What Changed — Windows change recorder (Tauri 2 + Rust + Svelte 5 MVP)"
description: "Local-first desktop black box that records meaningful Windows system changes and deterministically answers 'It worked before. What changed?'"
status: pending
priority: P1
effort: 173h
tags: [desktop, tauri2, rust, svelte5, windows, sqlite, privacy, observability, threejs]
blockedBy: []
blocks: []
created: 2026-08-29
---
```

## PLAN.MD BODY

```markdown
## Overview

"What Changed" is a non-elevated Windows 10/11 tray app that continuously records *meaningful*
system changes to a local SQLite database and answers one question: **"It worked before. What
changed?"** It ships as `WhatChanged.exe` (Tauri 2 + Rust backend, Svelte 5 + TypeScript
frontend, Three.js only for the ambient field, causal graph and system map). No account, no
cloud, no telemetry, no Node/Bun on the user's machine. 17 phases map onto the user's binding
PHASE 0-8 order; nothing is deferred except what the user deferred himself (§26 health checks,
§75-§78 future features — both recorded in `ROADMAP.md` with the schema already in place).

## Goals

| # | Goal | Priority |
|---|------|----------|
| 1 | A normal human sees "Oh. THAT changed." within seconds of opening the app (§1, §91) | P1 |
| 2 | Secrets can never be persisted or logged — enforced by types, not discipline (§34, §60) | P1 |
| 3 | 14 collectors emit one normalized `SystemEvent` shape; one failure never stops the app (§9, §12, §58) | P1 |
| 4 | Deterministic analysis engine ranks the real causal chain above unrelated noise, with visible reasoning (§27-§29, §59) | P1 |
| 5 | Everything works non-elevated; the two measured privilege gaps are documented, not hidden (§36, B3) | P1 |
| 6 | Responsive at 10k / 100k / 1,000,000 events; ~0-1% idle CPU; RAM measured and reported honestly (§37, §61, B2) | P1 |
| 7 | Premium calm dark-first identity; six states on every screen; exact §72-§74 microcopy voice | P1 |
| 8 | Narrow typed IPC only — no shell, no arbitrary file read, no SQL from the frontend (§35, §56) | P1 |
| 9 | Three.js used intentionally for ambient / causal graph / system map, never as the UI (§38, §39) | P2 |
| 10 | Installer, updater architecture and signing documented; upgrades never erase history (§64-§66) | P2 |

## Phases

| # | Phase | Status |
|---|-------|--------|
| 01 | [Toolchain, Scaffold & Architecture Docs](./phase-01-toolchain-scaffold-docs.md) | pending |
| 02 | [Redaction Chokepoint & Privacy Engine](./phase-02-redaction-privacy-chokepoint.md) | pending |
| 03 | [Storage, Migrations, Logging & Recovery](./phase-03-storage-migrations-logging.md) | pending |
| 04 | [Typed IPC, Capabilities & Type Codegen](./phase-04-typed-ipc-codegen.md) | pending |
| 05 | [Design System, App Shell & Six States](./phase-05-design-system-app-shell.md) | pending |
| 06 | [Tray, Lifecycle & Monitoring State](./phase-06-tray-lifecycle-monitoring-state.md) | pending |
| 07 | [Event Engine, Retention & Live Updates](./phase-07-event-engine-live-updates.md) | pending |
| 08 | [Timeline, Details, Search & Virtualization](./phase-08-timeline-search-virtualization.md) | pending |
| 09 | [Collector Framework, Process & Ports](./phase-09-collector-framework-process-ports.md) | pending |
| 10 | [Files, Applications & Services Collectors](./phase-10-files-apps-services-collectors.md) | pending |
| 11 | [Windows Change Intelligence & Snapshots](./phase-11-windows-change-intelligence.md) | pending |
| 12 | [Checkpoints & Comparison](./phase-12-checkpoints-comparison.md) | pending |
| 13 | [Analysis Engine, Scoring & Causal Chains](./phase-13-analysis-engine.md) | pending |
| 14 | [Investigation Experience](./phase-14-investigation-experience.md) | pending |
| 15 | [Ambient Field, Causal Graph & System Map](./phase-15-threejs-visualizations.md) | pending |
| 16 | [Docker & Development Mode](./phase-16-docker-development-mode.md) | pending |
| 17 | [Onboarding, Diagnostics, Installer & Release Audit](./phase-17-release-polish-audit.md) | pending |

## Architecture Summary

```text
Svelte 5 UI (WebView2)                 ← DOM only; never touches an OS API
   │  generated bindings (specta)      ← src/lib/tauri/bindings.ts is committed + drift-guarded
   ▼  12 narrow typed commands + 4 events
Tauri command layer  (commands/*.rs — validate, then delegate; no logic)
   ▼
Application services: QueryService · AnalysisEngine · CheckpointService · SettingsService
   ▼                                        ▲
Repository layer (storage/*.rs) ────────────┘   ← insert_batch(&[SanitizedEvent]) ONLY
   ▲
redact::sanitize(EventDraft) -> SanitizedEvent   ← THE chokepoint (private constructor)
   ▲                                             ← same rules wrap the log writer
Normalizer (dedupe · entity upsert · relationship upsert · cross-collector correlation)
   ▲
CollectorSupervisor  →  14 Collectors (trait: id/start/stop/snapshot), each isolated
   ▲
platform/windows/*   ← every Win32/COM call lives here; core/ stays OS-agnostic
```

SQLite (WAL, `busy_timeout`, `auto_vacuum=INCREMENTAL`, `mmap_size=0`) is written by **one**
dedicated writer thread with 200 ms / 256-event batches; reads use a 4-connection pool from
`spawn_blocking`. Rust domain types are the source of truth; TypeScript is generated. Cheap
collectors diff-poll (process/ports 2 s, services 5 s), expensive ones are notification-driven
with rare sweeps (drivers 60 min, tasks 6 h, network on IP-change notification) per measured cost.

## Success Criteria

- [ ] `bun run verify` (check + vitest + fmt + clippy + rust tests + 5 guards) passes from a clean clone
- [ ] `bun run tauri build` produces an NSIS installer; installing 0.1.0 → 0.1.1 preserves the database
- [ ] Close hides to tray, monitoring continues, all 7 tray entries work, single instance enforced
- [ ] Redaction leak suite: 14 secret fixtures never appear in any persisted column or log file
- [ ] §59 fixture: the port/config chain occupies the top 3 ranks; the unrelated Chrome update is below all 4 chain events
- [ ] 1M-event DB: timeline page p95 < 50 ms, search p95 < 300 ms, investigation < 1.5 s
- [ ] Idle CPU < 1% averaged over 120 s; RAM measured and reported split backend/WebView2 (B2 gate)
- [ ] 36-case six-state matrix green; app usable at 900×600; contrast script reports 0 failures
- [ ] `guard:stubs` reports zero stubs not listed in `docs/COLLECTORS.md` / `docs/ROADMAP.md`
- [ ] Docker absent → "Docker integration — Not installed", zero repeated warnings in 60 s
- [ ] Frontend command surface contains no shell / file-read / SQL command (verified by `guard:types`)
- [ ] `PRODUCT/ARCHITECTURE/SECURITY/PRIVACY/ROADMAP/COLLECTORS/ANALYSIS.md` all pass `guard:docs`

## Key Risks

| Risk | Signal it broke | Pre-decided response |
|------|-----------------|----------------------|
| B1 Rust absent; 3 MSVC installs | `rustc -Vv` missing or `link.exe` error | Install rustup MSVC host, pin `rust-toolchain.toml`, else build through `scripts/dev-shell.cmd` (vcvars64 of the newest `vswhere -latest` install) |
| B2 <80 MB RAM vs WebView2 floor | `measure-memory.ps1` combined > 80 MB on the bare scaffold | Record backend RSS vs WebView2 total in `ARCHITECTURE.md`, apply the 5 documented reductions, then **stop and ask the user** as a scope question — never redefine the metric |
| B3 Non-admin process blind spot | Short-lived processes absent from the timeline | `coverage_limits` on the descriptor renders into Settings + `COLLECTORS.md`; never claim complete coverage; no sub-interval timing claims |
| TypeScript 7.0.2 toolchain lag (unverified) | `bun run check` fails on the scaffold | Pin `typescript@5` latest, record Decision D2 in `ARCHITECTURE.md` |
| Registry/service/IP change notifications non-admin (unverified) | Spike returns denied or never fires | Fall back to the measured poll costs (uninstall 50 ms/60 s, services 3.3 ms/5 s, network sweep 10 min) |
| `tauri-specta` 1.0 API mismatch | Codegen does not compile | Switch to `ts-rs` 12.0.1 + hand-written thin wrappers; `guard:types` keeps drift detection either way |

## Dependencies

External prerequisites: Windows 11 Home 10.0.26200 (present), Windows SDK 10.0.26100 (present),
MSVC v14.44/14.50 (present), WebView2 Evergreen 151.x (present), Bun 1.3.11 (present), Git Bash,
**rustup + `stable-x86_64-pc-windows-msvc` (must be installed — phase 01 step 1)**. No CI runner;
every gate is one local command. No admin. `pwsh` is absent — scripts run under
`powershell -NoProfile -ExecutionPolicy Bypass`. Docker is absent, so only the "not installed"
path is verifiable here. Versions are pinned from `research/verified-facts-report.md` §1; any crate
or npm package not pinned there is added with `cargo add` / `bun add` and its resolved version
recorded in the commit — never hand-guessed.
```

## PHASE FILES

### FILE: phase-01-toolchain-scaffold-docs.md

---
phase: 01
title: "Toolchain, Scaffold & Architecture Docs"
status: pending
priority: P1
effort: 5h
dependencies: []
---

## Overview

Resolve blocker B1 (no Rust), decide D2 (TypeScript 7 vs 5), take the B2 RAM baseline on a bare
scaffold, write the seven §68 documents **before** feature code (§90), and prove the whole
pipeline end to end: `bun run tauri dev` runs, `bun run tauri build` emits an NSIS installer.
Repo root is `D:/Project/.WhatChage` (misspelled folder — never referenced in code or config;
product `What Changed`, binary `WhatChanged.exe`, package `what-changed`, identifier
`com.whatchanged.desktop`).

## Requirements

Functional: rustup + `stable-x86_64-pc-windows-msvc` installed and pinned; Bun workspace with the
§1-pinned versions; Tauri 2 app window 1280×840, `minWidth 900`, `minHeight 600`, `decorations:
false`; one typed IPC round-trip proven; NSIS installer artifact produced; first commit exists.
Non-functional: no Node runtime requirement for the shipped app; all gate commands runnable in Git
Bash non-elevated; every version either pinned from research §1 or resolved by the package manager
and recorded; `pwsh` not used (absent — Windows PowerShell 5.1 only).

## Architecture

Two build graphs joined by Tauri: Bun/Vite 8 builds `src/` → `dist/`; cargo builds `src-tauri/`.
`src-tauri/tauri.conf.json` `build.frontendDist = "../dist"`, `beforeDevCommand = "bun run dev"`,
`beforeBuildCommand = "bun run build"`. Rust module skeleton (empty but real modules, no `todo!()`)
laid out per §55 so later phases only add files, never restructure.

## Related Code Files

Create — root: `package.json`, `bun.lock` (generated), `tsconfig.json`, `vite.config.ts`,
`svelte.config.js`, `index.html`, `.gitignore`, `.gitattributes`, `rust-toolchain.toml`,
`README.md`, `docs/PRODUCT.md`, `docs/ARCHITECTURE.md`, `docs/SECURITY.md`, `docs/PRIVACY.md`,
`docs/ROADMAP.md`, `docs/COLLECTORS.md`, `docs/ANALYSIS.md`, `CONTRIBUTING.md`.
Create — scripts: `scripts/dev-shell.cmd`, `scripts/make-icon.ps1`, `scripts/measure-memory.ps1`,
`scripts/measure-idle-cpu.ps1`, `scripts/guard-stubs.ts`, `scripts/guard-docs.ts`.
Create — frontend: `src/main.ts`, `src/app/App.svelte`, `src/styles/app.css`, `src/vite-env.d.ts`.
Create — backend: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`,
`src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`,
`src-tauri/src/{app,commands,domain,storage,redact,privacy,analysis,collectors,core}/mod.rs`,
`src-tauri/src/platform/{mod.rs,windows/mod.rs,linux/mod.rs,macos/mod.rs}`,
`src-tauri/icons/*` (generated), `assets/brand/icon-source.png` (generated).
Modify: none (greenfield). Delete: none.

## Implementation Steps

1. Install Rust: `powershell -NoProfile -ExecutionPolicy Bypass -Command "Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile $env:TEMP\rustup-init.exe"`, then `"$TEMP/rustup-init.exe" -y --default-host x86_64-pc-windows-msvc --default-toolchain stable --profile default`. Open a new Git Bash (PATH already contains `~/.cargo/bin`). Confirm `rustc -Vv` prints `host: x86_64-pc-windows-msvc`.
2. Write `rust-toolchain.toml`: `[toolchain] channel = "stable"`, `components = ["rustfmt", "clippy"]`, `targets = ["x86_64-pc-windows-msvc"]`. Record the resolved `rustc --version` string in `docs/ARCHITECTURE.md`.
3. Resolve the MSVC ambiguity: `"C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`. Write `scripts/dev-shell.cmd` that calls that install's `VC/Auxiliary/Build/vcvars64.bat` then `%*`. Only used if step 8 fails to link.
4. `bun init -y`, then hand-write `package.json` deps at the research §1 pins: `svelte@5.56.10`, `@sveltejs/vite-plugin-svelte@7.3.0`, `vite@8.2.2`, `typescript@7.0.2`, `tailwindcss@4.3.3`, `@tailwindcss/vite@4.3.3`, `three@0.185.1`, `@types/three@0.185.4`, `vitest@4.1.11`, `@testing-library/svelte@5.4.2`, `svelte-check@4.7.6`, `@tauri-apps/cli@2.11.4`, `@tauri-apps/api@2.11.1`. `bun install`.
5. `tsconfig.json`: `strict: true`, `noUncheckedIndexedAccess`, `noImplicitOverride`, `verbatimModuleSyntax`, `moduleResolution: "bundler"`, `types: ["vite/client"]`, `isolatedModules`. `vite.config.ts`: `plugins: [svelte(), tailwindcss()]`, `server: { port: 1420, strictPort: true }`, `clearScreen: false`, `build: { target: "chrome120", sourcemap: true }`.
6. Scaffold `src-tauri/Cargo.toml` by hand (do not run an interactive template): `[package] name = "what-changed"`, `[lib] name = "what_changed_lib"`, `crate-type = ["staticlib","cdylib","rlib"]`; deps `tauri = { version = "=2.11.5", features = ["tray-icon","image-png"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `thiserror = "=2.0.20"`, `tokio = { version = "=1.53.1", features = ["rt-multi-thread","sync","time","macros"] }`, `tracing = "0.1"`, `tracing-subscriber = { version = "=0.3.23", features = ["env-filter","fmt"] }`, `tracing-appender = "=0.2.5"`; build-dep `tauri-build = "=2.6.3"`. `[profile.release] lto = "thin"`, `codegen-units = 1`, `panic = "abort"`, `strip = "symbols"`.
7. Generate the brand mark: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/make-icon.ps1` (System.Drawing draws a 1024² dark rounded square with an accent Δ — the product mark), then `bunx tauri icon assets/brand/icon-source.png`.
8. `bun run tauri dev` — first cargo build of Tauri 2 (expect several minutes). If linking fails, re-run through `scripts/dev-shell.cmd`.
9. Add one real IPC round-trip: `#[tauri::command] fn get_app_info() -> AppInfo` (name, version from `env!("CARGO_PKG_VERSION")`, webview version via `tauri::webview_version()` — confirm that symbol exists in the vendored `tauri` 2.11.5 source with `grep -rn "pub fn webview_version" ~/.cargo/registry/src/*/tauri-2.11.5/src`; if absent, read the Evergreen registry `pv` value instead, see phase 17 step 5), rendered in `App.svelte`. This proves the frontend↔Rust path required by the §69 PHASE 0 acceptance list.
10. D2 gate: `bun run check`. On failure, `bun add -d typescript@5` and `svelte-check` again; record the outcome under "Decision D2" in `docs/ARCHITECTURE.md` with the exact resolved version.
11. B2 baseline: run `bun run tauri build`, launch the built exe, then `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1` (sums `WorkingSet64`/`PrivateMemorySize64` for `WhatChanged` plus `msedgewebview2` children) and paste the three numbers (backend RSS, WebView2 total, combined) into `docs/ARCHITECTURE.md` → "Measured memory".
12. Write the seven docs with real content (not placeholders): `PRODUCT.md` = §1/§2/§40/§43-§54 voice and UX principles; `ARCHITECTURE.md` = the diagram, module map, IPC table, decisions log (D1-D3, dependency justifications per §80); `SECURITY.md` = §82 threat table (11 threats × mitigation × the test that proves it), §35 capability policy, §36 privilege model incl. the designed-not-built privileged helper, §66 signing; `PRIVACY.md` = the exhaustive collected/not-collected table incl. "no command lines, no environment variables, no file contents"; `COLLECTORS.md` = the 14-collector status table (Planned/Stub/Implemented/Verified-here), the measured non-admin matrix summary, B3, and the external-collector stdio protocol (documented, not built, §5); `ANALYSIS.md` = the §28 scoring formula and §29 confidence language rules; `ROADMAP.md` = phases + §26 health checks + §75-§78 explicitly deferred.
13. `.gitignore`: `node_modules/`, `dist/`, `src-tauri/target/`, `src-tauri/gen/`, `artifacts/`, `*.key`, `.env*`, `!.env.example`, `state.json`. `.gitattributes`: `* text=auto eol=lf`, `*.ps1 text eol=crlf`, `*.cmd text eol=crlf`.
14. Commit: if `git config user.email` is empty, set repo-local `git config --local user.name "what-changed-dev"` / `user.email "dev@localhost"` and note in `CONTRIBUTING.md` that the developer should replace them. Then `git add -A && git commit -m "chore: scaffold tauri 2 + svelte 5 workspace with pinned toolchain"`.

## Todo

- [ ] Commit 1 — `chore: pin rust toolchain and msvc selection`
- [ ] Commit 2 — `docs: product, architecture, security, privacy, collectors, analysis, roadmap`
- [ ] Commit 3 — `chore: scaffold tauri 2 + svelte 5 workspace with pinned toolchain`
- [ ] Commit 4 — `build: brand icon set and nsis bundle configuration`
- [ ] Record D2 decision, resolved rustc version, and B2 baseline numbers in `docs/ARCHITECTURE.md`

## Success Criteria

- [ ] `rustc -Vv` reports `host: x86_64-pc-windows-msvc`; `rustup show` lists the pinned toolchain
- [ ] `bun run check` exits 0 with a recorded TypeScript version (7.0.2 or the 5.x fallback)
- [ ] `bun run tauri dev` opens a 1280×840 window that displays app name, version and WebView2 version fetched over IPC
- [ ] `src-tauri/target/release/bundle/nsis/*-setup.exe` exists after `bun run tauri build`
- [ ] All seven docs exist with substantive content; `bun run guard:docs` exits 0
- [ ] `docs/ARCHITECTURE.md` contains the three measured memory numbers
- [ ] `git log --oneline` shows ≥4 commits; working tree clean

## Verification Gate

```bash
rustc -Vv && cargo -V && rustup show
bun install --frozen-lockfile && bun run check
bun run rust:fmt && bun run rust:lint
bun run tauri build && ls -1 src-tauri/target/release/bundle/nsis/
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1
bun run guard:docs && bun run guard:stubs && git status --porcelain
```

## Risk Assessment

- **B1 rustup install fails / wrong host** — signal: `rustc -Vv` shows a `gnu` host or the command is missing. Response: `rustup toolchain install stable-x86_64-pc-windows-msvc && rustup default stable-x86_64-pc-windows-msvc`; if the installer itself is blocked, stop and report — no Rust means no phase can proceed.
- **Linker picks the wrong MSVC (3 installs)** — signal: `link.exe` not found, `LNK1181`, or a `cl.exe` version mismatch. Response: build via `scripts/dev-shell.cmd` (vcvars64 from `vswhere -latest`) and record the chosen install path in `ARCHITECTURE.md`.
- **D2: TS 7 breaks svelte-check** — signal: `bun run check` fails with compiler-internal errors. Response: `typescript@5`, record the decision. Never proceed with a red `check`.
- **B2: combined RAM > 80 MB on an empty scaffold** — signal: `measure-memory.ps1` combined > 80 MB. Response: record the split, continue the phase, and raise it as an explicit scope question in phase 17's B2 gate; do not silently restate the target.
- **`bunx tauri icon` rejects the generated PNG** — signal: non-square/alpha error. Response: regenerate at 1024×1024 32bpp ARGB in `make-icon.ps1`; if System.Drawing is unavailable, hand-author a 1024² PNG with a Bun script writing raw PNG chunks (no dependency).

## Security Considerations

`capabilities/default.json` starts at `core:default` only — every later permission must be justified
in `SECURITY.md`. `withGlobalTauri: false`. CSP set in `tauri.conf.json`
(`default-src 'self'; script-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline';
connect-src 'self' ipc: http://ipc.localhost`) and validated against the dev console. `.gitignore`
excludes `*.key` and `.env*` before the first commit so no signing key or secret can ever be staged.
No signing thumbprint or token is written into any tracked file.

### FILE: phase-02-redaction-privacy-chokepoint.md

---
phase: 02
title: "Redaction Chokepoint & Privacy Engine"
status: pending
priority: P1
effort: 5h
dependencies: [01]
---

## Overview

Build the single enforced redaction chokepoint **before any code can persist or log a byte** (§34).
`SanitizedEvent` has a private field and exactly one constructor, `redact::sanitize()`; the
repository (phase 03) accepts nothing else, so bypassing redaction is a compile error. The same
rule set wraps the log sink, satisfying "before persistence **and** before logging" structurally
rather than by convention.

## Requirements

Functional: key-name redaction for the §13 concept list; value-shape redaction for the §60 fixture
families; structured map/diff redaction preserving key names and `[REDACTED]` placeholders exactly
as §13 shows; `sensitive: true` stamped on any event that was redacted; a text scrubber used by the
log writer. Non-functional: zero new dependencies (hand-written scanners — no regex crate, no
backtracking risk); < 5 µs per 1 KB value; pure functions, no I/O, fully unit-testable; every
public item documented.

## Architecture

```rust
// redact/sanitized.rs
pub struct SanitizedEvent(SystemEvent);              // field private to module
impl SanitizedEvent { pub(crate) fn inner(&self) -> &SystemEvent { &self.0 } }
// redact/mod.rs — the ONLY way to obtain one
pub fn sanitize(draft: EventDraft) -> SanitizedEvent;
pub fn scrub_text(s: &str) -> String;                // log sink + free text
pub fn scrub_pairs(pairs: &mut Vec<(String, String)>) -> bool;   // returns `sensitive`
```

`keys.rs` holds the case-insensitive key-token list: `password, passwd, pwd, secret, token,
apikey, api_key, authorization, auth, bearer, cookie, session, private_key, client_secret,
credential, access_token, refresh_token, connectionstring, sas, signature`. `shapes.rs` holds
value detectors: `sk-`/`rk-` prefixes, `Bearer <b64>`, JWT (three dot-separated base64url segments
where segment 1 starts `eyJ`), PEM `-----BEGIN ... PRIVATE KEY-----`, URL userinfo
(`scheme://user:pass@host`), AWS `AKIA`/`ASIA` + 16 uppercase-alnum, `ghp_`/`gho_`/`ghs_`/
`github_pat_`, `xoxb-`/`xoxp-`/`xoxa-`, `Set-Cookie` values, and generic high-entropy runs
(≥ 32 chars from a base64/hex alphabet with ≥ 3 character classes). Redacted output is exactly
`[REDACTED]`; the key name and diff structure survive. URL userinfo collapses to
`scheme://user:[REDACTED]@host` so the change is still explainable.

## Related Code Files

Create: `src-tauri/src/redact/{mod.rs,keys.rs,shapes.rs,value.rs,structured.rs,sanitized.rs,log_writer.rs}`,
`src-tauri/src/privacy/{mod.rs,policy.rs}`, `src-tauri/src/domain/{mod.rs,event.rs,category.rs,importance.rs,ids.rs}`,
`src-tauri/tests/redaction_leak.rs`, `src-tauri/tests/fixtures/secrets.rs`, `scripts/guard-redaction.ts`.
Modify: `src-tauri/src/lib.rs` (register modules), `package.json` (`guard:redaction`),
`docs/PRIVACY.md`, `docs/SECURITY.md`. Delete: none.

## Implementation Steps

1. Define §9's domain in `domain/event.rs`: `SystemEvent` with all 18 fields; `EventDraft` = the same minus `id`; `Category` (15 variants), `EventType` (19 variants), `Severity`, `Importance` (`Informational|Notable|Important|Critical`), `Confidence` (`Low|Medium|High`) — all `#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]` with `#[serde(rename_all = "snake_case")]`. `before`/`after`/`metadata` are `Vec<(String,String)>` ordered pairs (diff-friendly, JSON-serialized at the storage edge).
2. `keys.rs`: token list + `pub fn is_sensitive_key(key: &str) -> bool` (lowercase, `contains` any token, word-boundary-tolerant).
3. `shapes.rs`: one `fn detect(value: &str) -> Option<ShapeKind>` per family, all `&str`-scanning, no allocation on the negative path. Unit test each detector's positive and negative cases.
4. `value.rs`: `fn redact_value(key: Option<&str>, value: &str) -> Cow<str>` — key match ⇒ full `[REDACTED]`; shape match ⇒ replace the matched span; else unchanged. Cap any stored value at 200 chars with a `…` suffix.
5. `structured.rs`: `fn redact_pairs`, `fn redact_diff(before, after)` producing the §13 shape (`API_PORT 3000 → 4000`, `OPENAI_API_KEY [REDACTED] → [REDACTED]`), and `fn redact_free_text` for summaries.
6. `sanitized.rs` + `mod.rs::sanitize`: run `redact_pairs` over `before`/`after`/`metadata`, `redact_free_text` over both summaries and `entity_display_name`, set `sensitive = true` if anything changed, then wrap. No other code path constructs the type.
7. `log_writer.rs`: `pub struct RedactingWriter<W>` implementing `std::io::Write` that scrubs each complete line through `scrub_text` before delegating, plus `RedactingMakeWriter` for `tracing_subscriber::fmt`. This file is the **only** place allowed to reference `tracing_appender::rolling`.
8. `privacy/policy.rs`: the machine-readable collection policy — `pub const NEVER_COLLECTED: &[&str]` (keystrokes, clipboard, browser history, messages, screen frames, screenshots, microphone, camera, document contents, command lines, environment variables) — rendered into `PRIVACY.md` by `guard:docs` and into the Settings → Privacy screen (phase 05) so code and copy cannot diverge.
9. `tests/redaction_leak.rs`: 14 fixtures (§60 list + GitHub/Slack/Basic-auth), each asserting the raw value is absent from `serde_json::to_string(sanitize(draft).inner())`, that the key name survives, that `sensitive == true`, and that `scrub_text` of a log line containing it is clean. Table-driven, one `#[test]` per family plus an `all_fixtures_absent` sweep.
10. `scripts/guard-redaction.ts` (Bun): fails if (a) `SanitizedEvent(` appears outside `src-tauri/src/redact/`, (b) `tracing_appender::rolling` appears outside `redact/log_writer.rs`, (c) any file in `collectors/` or `platform/` references `.cmd()`, `.environ()`, `GetCommandLine`, or `QueryFullProcessImageName`-adjacent command-line APIs, (d) `insert_batch` is declared with a parameter type other than `&[SanitizedEvent]`.

## Todo

- [ ] Commit 1 — `feat(domain): normalized system event model`
- [ ] Commit 2 — `feat(redact): key and value-shape redaction scanners`
- [ ] Commit 3 — `feat(redact): sanitized-event chokepoint and redacting log writer`
- [ ] Commit 4 — `test(redact): 14-fixture leak suite and chokepoint guard script`

## Success Criteria

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml redaction` passes; all 14 fixtures leak-free
- [ ] `SanitizedEvent` cannot be constructed outside `redact/` (verified by `guard:redaction` and by a deliberate local experiment that must fail to compile)
- [ ] Redacted diff output matches the §13 sample byte for byte in a snapshot test
- [ ] `docs/PRIVACY.md` "never collected" list is generated from `privacy/policy.rs` and matches it
- [ ] `bun run guard:redaction` exits 0

## Verification Gate

```bash
bun run rust:fmt && bun run rust:lint
cargo test --manifest-path src-tauri/Cargo.toml -- redaction --nocapture
bun run guard:redaction
```

## Risk Assessment

- **Over-redaction destroys useful diffs** — signal: a fixture asserting `API_PORT 3000 → 4000` survives fails, or manual review shows numeric ports redacted. Response: entropy detector requires ≥ 3 character classes and ≥ 32 length; add the failing case as a negative test before touching thresholds.
- **Under-redaction (a novel secret shape)** — signal: a new fixture leaks. Response: fixtures are the contract — add the shape to `shapes.rs`, never to a caller. Any leak found later becomes a permanent fixture.
- **Someone bypasses the chokepoint in a later phase** — signal: `guard:redaction` fails in that phase's gate. Response: the guard is part of `bun run verify`, so the phase cannot pass; fix by routing through `sanitize`.
- **Log lines assembled before the sink** — signal: a scrubbed-line test on a multi-line `{:#?}` payload fails. Response: `RedactingWriter` buffers to a newline before scrubbing; forbid `{:#?}` of event/config types via the guard's denylist.

## Security Considerations

This phase *is* the security boundary for §33/§34. Additional structural guarantees introduced here
and asserted by the guard: `ProcessFacts` (phase 09) will have **no** command-line or environment
field at all; free text from collectors is length-capped before storage (§82 event floods); the
redactor never logs the value it redacted, not even at trace level.

### FILE: phase-03-storage-migrations-logging.md

---
phase: 03
title: "Storage, Migrations, Logging & Recovery"
status: pending
priority: P1
effort: 9h
dependencies: [02]
---

## Overview

The persistence spine: 13 migrated tables (§10), one writer thread with batched transactions, a
4-connection reader pool, retention/maintenance, structured logging through the redacting sink, and
the §83 corruption-recovery state machine that must never crash-loop.

## Requirements

Functional: `rusqlite` 0.40.2 (`bundled`) + `rusqlite_migration` 2.6.0; WAL; all §10 tables and
indexes; `schema_migrations` audit rows in addition to `user_version`; pre-migration `VACUUM INTO`
backup (keep 2); `PRAGMA quick_check` at startup; degraded mode with three recovery actions;
retention defaults 30 d events / 7 d snapshots / 90 d important; rolling daily logs (7 files) with
runtime-settable verbosity. Non-functional: writer never blocks a Tauri command; batches flush at
200 ms or 256 events; bounded 4096-event channel with counted drops; `mmap_size=0` and an 8 MB page
cache to protect the B2 budget; no `unwrap()` outside tests (§79).

## Architecture

DB path `%APPDATA%/com.whatchanged.desktop/what-changed.db` (from `app.path().app_data_dir()`),
logs at `%LOCALAPPDATA%/com.whatchanged.desktop/logs`, sidecar `state.json` beside the DB holding
`{ schema_version, last_clean_shutdown, startup_failures }`.

```text
collectors → mpsc(4096) → WriterThread(owns 1 Connection)
                              ├ sanitize already done (accepts SanitizedEvent only)
                              ├ 200ms/256 batch → BEGIN IMMEDIATE … COMMIT
                              └ post-commit → EventBus (coalesced tauri emit, phase 07)
commands → spawn_blocking → ReaderPool(Mutex<Vec<Connection>>, max 4, read-only flags)
maintenance task (30 min) → chunked deletes (5k rows/txn) → PRAGMA incremental_vacuum(200)
```

Pragmas on every connection: `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`,
`foreign_keys=ON`, `temp_store=MEMORY`, `cache_size=-8000`, `mmap_size=0`;
`auto_vacuum=INCREMENTAL` and `wal_autocheckpoint=512` set on creation before any DDL.

## Related Code Files

Create: `src-tauri/src/storage/{mod.rs,db.rs,pool.rs,migrations.rs,writer.rs,query.rs,maintenance.rs,recovery.rs,repo_events.rs,repo_entities.rs,repo_relationships.rs,repo_snapshots.rs,repo_checkpoints.rs,repo_settings.rs,repo_collector_status.rs,repo_analysis.rs}`,
`src-tauri/migrations/{001_core.sql,002_indexes.sql,003_settings_defaults.sql}`,
`src-tauri/src/domain/{entity.rs,relationship.rs,checkpoint.rs,snapshot.rs,settings.rs}`,
`src-tauri/src/core/{mod.rs,time.rs,errors.rs,fingerprint.rs}`,
`src-tauri/src/app/{mod.rs,state.rs,logging.rs}`,
`src-tauri/tests/{migrations.rs,storage_roundtrip.rs,redaction_persistence.rs,retention.rs,recovery.rs}`.
Modify: `src-tauri/Cargo.toml` (add `rusqlite = { version = "=0.40.2", features = ["bundled"] }`,
`rusqlite_migration = "=2.6.0"`), `src-tauri/src/lib.rs`, `docs/ARCHITECTURE.md`.

## Implementation Steps

1. `migrations/001_core.sql` — `events`(id INTEGER PK AUTOINCREMENT, timestamp INTEGER NOT NULL, category TEXT, event_type TEXT, entity_type TEXT, entity_id TEXT, entity_display_name TEXT, severity TEXT, importance TEXT, source_collector TEXT, before_json TEXT, after_json TEXT, metadata_json TEXT, related_entity_ids TEXT, user_friendly_summary TEXT, technical_summary TEXT, confidence TEXT, sensitive INTEGER, search_text TEXT); `entities`(id TEXT PK, entity_type, display_name, natural_key UNIQUE, first_seen, last_seen, attributes_json); `entity_relationships`(id INTEGER PK, source_id, target_id, kind, confidence, first_seen, last_seen, evidence_collector, UNIQUE(source_id,target_id,kind)); `snapshots`(id, collector_id, taken_at, payload_json, byte_size); `checkpoints`(id, label, created_at, summary_json, snapshot_ids); `health_states`(id, target_kind, target_ref, state, changed_at) — table now, feature deferred per §26; `collector_status`(collector_id PK, enabled, state, last_ok_at, last_error, error_count, maturity); `settings`(key PK, value_json, updated_at); `watched_locations`(id, path, kind, enabled, added_at); `applications`(id, display_name, publisher, version, install_location, registry_source, normalized_key UNIQUE); `analysis_sessions`(id, created_at, problem_text, parsed_json, window_start, window_end); `analysis_results`(id, session_id, event_id, rank, score, factors_json, explanation); `schema_migrations`(version PK, name, applied_at).
2. `migrations/002_indexes.sql` — the six mandated indexes plus `events(importance, timestamp)`, `events(source_collector, timestamp)`, `events(search_text)`, `snapshots(collector_id, taken_at)`, `analysis_results(session_id, rank)`.
3. `storage/migrations.rs` — `Migrations::new(vec![M::up(include_str!(...)), …])`; before applying anything when `user_version < target`, call `recovery::backup_before_migration()` (`VACUUM INTO`), then apply, then insert the `schema_migrations` row inside the same transaction.
4. `storage/db.rs` — `open(path) -> Result<Connection, StorageError>`: apply pragmas, `PRAGMA quick_check(1)`, run migrations, write `state.json`. Map `SQLITE_CORRUPT`/`SQLITE_NOTADB` to `StorageError::Corrupt`. If `user_version > migrations.len()` (an older build opened a newer DB — the downgrade/rollback case), do **not** write: return `StorageError::SchemaTooNew { found, supported }`, which the UI renders as "This history was created by a newer version of What Changed." with Export diagnostics / Reset local history offered and reading disabled. Never silently downgrade or drop rows.
5. `storage/writer.rs` — `std::thread::Builder::new().name("db-writer")` owning the write connection; loop over `mpsc::Receiver<WriteCommand>` where `WriteCommand::Events(Vec<SanitizedEvent>)`, `::Entity(..)`, `::Relationship(..)`, `::Snapshot(..)`, `::Flush(oneshot)`; prepared statements cached with `Connection::prepare_cached`; one `BEGIN IMMEDIATE` per batch; on `SQLITE_BUSY` retry 3× with 50/150/400 ms backoff, then count a drop.
6. `storage/pool.rs` — `ReaderPool { conns: Mutex<Vec<Connection>>, max: 4 }` with `with_read(|c| …)` used exclusively from `tokio::task::spawn_blocking`; connections opened `SQLITE_OPEN_READ_ONLY`.
7. `storage/repo_events.rs` — `insert_batch(&mut Connection, &[SanitizedEvent])` (**the only signature that touches the events table**), `page(filter, cursor, limit)` using keyset pagination `WHERE (timestamp, id) < (?,?) ORDER BY timestamp DESC, id DESC LIMIT ?`, `by_id`, `count_by_importance(range)`. `search_text` is built at insert from display name + summaries + high-value metadata, lowercased, capped 512 chars.
8. `storage/maintenance.rs` — every 30 min (and 60 s after start): delete non-important events older than `retention_days`, important/critical older than `retention_important_days`, snapshots older than `retention_snapshot_days`, orphan entities/relationships; chunk 5 000 rows per transaction with a 50 ms sleep between chunks; then `PRAGMA incremental_vacuum(200)`; `PRAGMA optimize` on shutdown. Expose `estimate_storage()` = `page_count * page_size` + per-table row counts for Settings → Storage (§11).
9. `storage/recovery.rs` — `startup_failures` counter in `state.json`; on `Corrupt` or ≥2 consecutive failures enter `DegradedMode` (events buffered in a 2 000-entry ring, UI banner) and expose `repair_database()` (`VACUUM INTO` a new file, swap on success, else rename to `what-changed.corrupt-<ts>.db` and create fresh), `reset_history()` (typed confirmation), `export_diagnostics()` (phase 17 wires the dialog).
10. `app/logging.rs` — `tracing_subscriber::registry()` + `EnvFilter` behind `reload::Layer` (setting `log_level`) + `fmt::layer().with_writer(RedactingMakeWriter::new(non_blocking_appender))`, appender = `RollingFileAppender::builder().rotation(Rotation::DAILY).filename_prefix("what-changed").max_log_files(7).build(log_dir)`; a console layer only under `cfg!(debug_assertions)`. `tauri-plugin-log` is deliberately **not** used (one logging stack — recorded in `ARCHITECTURE.md` per §80).
11. Tests: `migrations.rs` (fresh DB reaches target `user_version`; a v1 fixture DB migrates forward and keeps its rows; a pre-migration backup file appears); `storage_roundtrip.rs`; `redaction_persistence.rs` (insert all 14 secret fixtures through the repository, then `SELECT` every TEXT column of every table and assert no raw value is present — the persistence-level leak test); `retention.rs` (100k rows pruned in chunks, DB size shrinks after incremental vacuum, important rows survive to 90 d); `recovery.rs` (truncate a copy of the DB file mid-page → `open()` returns `Corrupt`, `repair_database()` yields a usable DB, no panic, no loop).

## Todo

- [ ] Commit 1 — `feat(storage): sqlite schema, migrations and pragmas`
- [ ] Commit 2 — `feat(storage): writer thread, reader pool and event repository`
- [ ] Commit 3 — `feat(storage): retention maintenance and storage estimates`
- [ ] Commit 4 — `feat(app): redacting rolling log sink with runtime verbosity`
- [ ] Commit 5 — `feat(storage): corruption detection and repair paths`
- [ ] Commit 6 — `test(storage): migration, retention, recovery and persistence-leak suites`

## Success Criteria

- [ ] Fresh launch creates the DB with all 13 tables, 11 indexes and `journal_mode=wal` (asserted by test, not by eye)
- [ ] Persistence-leak test finds none of the 14 secret values in any TEXT column
- [ ] 100k-row retention pass completes with no single transaction over 5 000 rows and no UI-blocking call
- [ ] Corrupt-DB test: no panic, no crash loop, `repair_database()` returns a working DB
- [ ] Log file exists under `%LOCALAPPDATA%/com.whatchanged.desktop/logs`, rotates daily, keeps ≤7 files, and contains no fixture secret
- [ ] `cargo clippy -- -D warnings` clean; no `unwrap()` outside `#[cfg(test)]`

## Verification Gate

```bash
bun run rust:fmt && bun run rust:lint
bun run rust:test
cargo test --manifest-path src-tauri/Cargo.toml -- migrations retention recovery redaction_persistence
bun run guard:redaction
```

## Risk Assessment

- **`bundled` SQLite fails to compile (MSVC selection)** — signal: `cc` / `cl.exe` error building `libsqlite3-sys`. Response: build through `scripts/dev-shell.cmd`; do not switch to a system SQLite (would add a runtime dependency).
- **Writer starvation under a burst** — signal: `dropped_events > 0` in Diagnostics during the phase-08 burst test. Response: raise the batch size to 512 and the channel to 8 192; if still dropping, coalesce identical consecutive events in the normalizer (phase 09) rather than growing memory.
- **WAL file growth on a long idle session** — signal: `-wal` > 64 MB. Response: `wal_autocheckpoint=512` plus a `PRAGMA wal_checkpoint(TRUNCATE)` in the 30 min maintenance pass.
- **Migration corrupts data on upgrade** — signal: `migrations.rs` forward test fails or row counts change. Response: pre-migration `VACUUM INTO` backup is restored automatically and the app starts in degraded mode with a "Migration failed — history preserved" banner.
- **`quick_check` cost at startup on a 1 M-row DB** — signal: startup > 500 ms attributable to the check. Response: `quick_check(1)` (single-page) only, and run the full `integrity_check` solely from the Diagnostics screen on demand.

## Security Considerations

`insert_batch` is the sole write path into `events` and accepts only `SanitizedEvent`, so phase 02's
guarantee extends to persistence by type. DB and logs live under per-user AppData (no world-writable
path). Diagnostics export and repair actions never copy raw DB contents into a report — phase 17's
exporter re-runs `scrub_text` over every field. `state.json` holds no user data, only counters.

### FILE: phase-04-typed-ipc-codegen.md

---
phase: 04
title: "Typed IPC, Capabilities & Type Codegen"
status: pending
priority: P1
effort: 5h
dependencies: [03]
---

## Overview

Freeze the frontend↔Rust contract: 12 named commands from §56 (+ diagnostics and dev-only
injectors), 4 events from §57, a typed error envelope, validated inputs, the minimum Tauri
capability set, and generated TypeScript so Rust stays the source of truth (§81). Commands land
per phase; the contract table in `ARCHITECTURE.md` records which phase implements each.

## Requirements

Functional: `src/lib/tauri/bindings.ts` generated from Rust and committed; a drift guard that fails
if regeneration changes the file; every command validates its arguments in Rust; `AppError` is a
serializable typed enum; capabilities enumerate only what the UI actually calls. Non-functional: no
generic command (`run_*`, `read_file`, `exec_*`, raw SQL) may exist — asserted by a guard; command
handlers contain no business logic; all DB work runs in `spawn_blocking`.

## Architecture

Commands implemented in this phase (real backing exists): `get_app_info`, `get_settings`,
`update_settings`, `get_collector_status`, `get_diagnostics`, `get_monitoring_state`. Reserved names
implemented later, each registered only when its backing lands: `get_dashboard_summary`,
`get_events`, `get_event_details`, `search_events` (07/08), `set_collector_enabled` (09),
`create_checkpoint`, `rename_checkpoint`, `delete_checkpoint`, `compare_checkpoint` (12),
`run_investigation` (13), `export_diagnostics`, `check_for_updates` (17),
`dev_seed_synthetic`, `dev_inject_event`, `dev_clear_synthetic` (07, dev-gated).

```rust
#[derive(Debug, thiserror::Error, serde::Serialize, specta::Type)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum AppError {
    #[error("not found")] NotFound,
    #[error("invalid input: {field}")] InvalidInput { field: String, reason: String },
    #[error("storage unavailable")] Storage { recoverable: bool },
    #[error("database degraded")] Degraded,
    #[error("not supported on this system")] Unsupported { reason: String },
    #[error("internal error")] Internal,          // never carries a raw message to the UI
}
```

Codegen: `tauri_specta::Builder::new().commands(tauri_specta::collect_commands![...])
.events(tauri_specta::collect_events![...])`, exported under `cfg!(debug_assertions)` to
`../src/lib/tauri/bindings.ts`. The exact builder/export call is confirmed against the vendored
crate source at `~/.cargo/registry/src/*/tauri-specta-1.0.2/src/lib.rs` before writing it —
**do not guess the API**; if it differs from the above, follow the vendored example verbatim.

## Related Code Files

Create: `src-tauri/src/commands/{mod.rs,app_info.rs,settings.rs,collectors.rs,diagnostics.rs}`,
`src-tauri/src/core/validate.rs`, `src-tauri/src/app/events.rs`,
`src/lib/tauri/{client.ts,events.ts,errors.ts,bindings.ts}` (bindings generated),
`src-tauri/capabilities/default.json` (rewrite), `scripts/guard-types.ts`,
`src-tauri/tests/command_validation.rs`.
Modify: `src-tauri/Cargo.toml` (`specta = "=1.0.5"`, `tauri-specta = "=1.0.2"`, `specta-typescript`
added via `cargo add` with the resolved version recorded), `src-tauri/src/lib.rs`,
`src-tauri/tauri.conf.json`, `docs/ARCHITECTURE.md` (IPC table), `package.json`.

## Implementation Steps

1. Add `specta::Type` derives to every `domain` type that crosses IPC. Keep `#[serde(rename_all = "snake_case")]` consistent so generated TS unions read like the Rust enums.
2. Implement `AppError` with `From<StorageError>`; the `Internal` variant logs the real cause at `error!` level (through the redacting sink) and returns no detail to the frontend (§82 information disclosure).
3. `core/validate.rs`: `page_limit(1..=200)`, `text_len(max)`, `time_range(max_days = 366, start < end)`, `label(1..=80, no control chars)`, `cursor("ts:id" parse)`, `path_in_watched_roots()` (used from phase 10). Every command calls these first; `command_validation.rs` asserts each rejects out-of-range input with `InvalidInput`.
4. Implement the six commands with real backing; `get_diagnostics` returns app version, DB size/event count from `estimate_storage()`, WebView2 version (`tauri::webview_version()`), collector health rows, monitoring state, log directory path.
5. Wire codegen: read the vendored `tauri-specta` source, add the builder in `lib.rs`, run `bun run tauri dev` once to emit `bindings.ts`, commit it.
6. `src/lib/tauri/client.ts` re-exports the generated commands plus a `withError<T>()` helper mapping `AppError` variants to typed UI states; `errors.ts` holds the user-facing copy for each variant (from `src/lib/copy/en.ts` in phase 05); `events.ts` typed `listen` wrappers for `system-event-added`, `collector-status-changed`, `analysis-completed`, `monitoring-state-changed`.
7. Rewrite `capabilities/default.json`: after the first build, enumerate valid identifiers from `src-tauri/gen/schemas/desktop-schema.json` and include only — core defaults, window show/hide/minimize/maximize/start-dragging (custom titlebar), event listen/emit, notification show (phase 17), dialog save + folder pick (phases 10/17), opener open-path for the log folder. Nothing else. Record the final list in `SECURITY.md`.
8. `scripts/guard-types.ts`: (a) regenerate bindings and fail on `git diff --exit-code -- src/lib/tauri/bindings.ts`; (b) fail if any `#[tauri::command]` name matches `/(shell|exec|run_|read_file|write_file|sql|query_raw|eval)/i`; (c) fail if `bindings.ts` exports a command not listed in the `ARCHITECTURE.md` IPC table.

## Todo

- [ ] Commit 1 — `feat(ipc): typed error envelope and input validation helpers`
- [ ] Commit 2 — `feat(ipc): settings, collector-status and diagnostics commands`
- [ ] Commit 3 — `feat(ipc): specta type generation and committed bindings`
- [ ] Commit 4 — `chore(security): minimal tauri capability set + type/command guard`

## Success Criteria

- [ ] `bindings.ts` is committed and `bun run guard:types` exits 0 (no drift, no forbidden command, no undocumented command)
- [ ] Every command returns `Result<T, AppError>`; `command_validation.rs` proves each validator rejects bad input
- [ ] `capabilities/default.json` contains no permission the UI does not call; `SECURITY.md` lists each with a justification
- [ ] Frontend compiles with `strict` and no `any` in `src/lib/tauri/`
- [ ] A deliberate call to an unregistered command fails at TypeScript compile time, not at runtime

## Verification Gate

```bash
bun run tauri dev   # regenerates bindings; close after the window renders
bun run guard:types && bun run check
bun run rust:lint && cargo test --manifest-path src-tauri/Cargo.toml -- command_validation
```

## Risk Assessment

- **`tauri-specta` 1.0 API differs from the sketch above** — signal: codegen does not compile. Response: follow the vendored crate's own example; if still blocked after one attempt, switch to `ts-rs` 12.0.1 (`#[derive(TS)]` + `cargo test export_bindings`) with hand-written command wrappers, and keep `guard:types` drift detection on the exported directory. Record the decision in `ARCHITECTURE.md`.
- **Capability identifier names guessed wrong** — signal: build error "permission not found". Response: identifiers come from the generated `desktop-schema.json` only; never hand-invent one.
- **Generated bindings churn on every build (noise)** — signal: `guard:types` fails on ordering. Response: sort command/event registration lists alphabetically so output is stable.
- **`Internal` error leaks a path or secret** — signal: a test asserting `AppError::Internal` serializes without a message fails. Response: keep the variant field-less; details only go to the redacted log.

## Security Considerations

This is the §35 boundary: the frontend is untrusted. No command takes a path, SQL fragment, command
line, or arbitrary key/value that is written to disk unvalidated; `update_settings` accepts a typed
struct with an allowlisted key set, not a free-form map. Dev-only commands are compiled out of
release builds (`#[cfg(any(debug_assertions, feature = "devtools"))]`) **and** gated at runtime on
`settings.developer_mode`, so §85's panel cannot exist in a shipped binary by accident.

### FILE: phase-05-design-system-app-shell.md

---
phase: 05
title: "Design System, App Shell & Six States"
status: pending
priority: P1
effort: 12h
dependencies: [04]
---

## Overview

Establish the visual identity (§40), the token layer (§41), motion rules (§42), the shell (custom
titlebar + nav rail + page frame), the six-state contract for every screen (§70), 900×600
responsiveness (§71), accessibility baseline (§62), and the centralized microcopy file that carries
the §72-§74 voice. All seven screens exist as real routed pages with real copy and real states —
data arrives in later phases.

## Requirements

Functional: Dark / Light / System themes persisted via `update_settings`; every screen renders
loading, empty, error, success, large-data and small-window states; keyboard reachable end to end;
`prefers-reduced-motion` collapses durations to 0; no lorem ipsum anywhere. Non-functional: no UI
framework beyond Svelte — Bits UI patterns are *copied and restyled*, not installed (§80 dependency
policy); component
files ≤ 200 lines; contrast ≥ 4.5:1 verified by script; zero network requests at runtime (§63),
including fonts.

## Architecture

Identity: near-black desaturated base (`--background: #0A0C0F`), three elevation surfaces, 1 px
borders instead of shadows for structure, a single accent (`--accent: #4C8DFF`), status hues used
only for status. Restrained glass (12 px blur) exists in exactly two places: the command palette and
the drawer scrim. Type: bundled-free system stack `"Segoe UI Variable Display", "Segoe UI",
system-ui` with `font-variant-numeric: tabular-nums` for all timestamps and counters; mono stack
`ui-monospace, "Cascadia Mono", Consolas` for technical details — satisfies §63 with no font
download. Density: 36 px timeline rows, 8 px spacing base, 4 px radius steps.

`src/styles/tokens.css` declares every §41 token as a CSS variable on `:root` and overrides on
`[data-theme="light"]`; `src/styles/app.css` maps them into Tailwind 4 via
`@import "tailwindcss";` + `@theme { --color-background: var(--background); … }` so utilities and
tokens cannot drift. Motion tokens: `--duration-instant 80ms`, `--duration-fast 140ms`,
`--duration-base 220ms`, `--duration-slow 360ms`, `--ease-standard cubic-bezier(.2,0,0,1)`,
`--ease-spring cubic-bezier(.34,1.56,.64,1)`; a `@media (prefers-reduced-motion: reduce)` block sets
all durations to `1ms`.

## Related Code Files

Create — styles: `src/styles/{tokens.css,app.css,base.css}`.
Create — shell: `src/app/{App.svelte,AppFrame.svelte,TitleBar.svelte,NavRail.svelte,PageHeader.svelte}`,
`src/lib/stores/{router.svelte.ts,theme.svelte.ts,settings.svelte.ts,layout.svelte.ts}`.
Create — ui primitives (`src/components/ui/`): `Button.svelte`, `IconButton.svelte`, `Card.svelte`,
`Panel.svelte`, `Chip.svelte`, `Badge.svelte`, `Switch.svelte`, `Select.svelte`, `TextField.svelte`,
`SearchField.svelte`, `Tabs.svelte`, `Tooltip.svelte`, `Dialog.svelte`, `Drawer.svelte`,
`Skeleton.svelte`, `EmptyState.svelte`, `ErrorState.svelte`, `StateShell.svelte`, `Toast.svelte`,
`KeyHint.svelte`, `StatusPill.svelte`, `ImportanceDot.svelte`.
Create — screens (`src/features/`): `home/HomeScreen.svelte`, `timeline/TimelineScreen.svelte`,
`investigation/InvestigationScreen.svelte`, `checkpoints/CheckpointsScreen.svelte`,
`system/SystemMapScreen.svelte`, `settings/SettingsScreen.svelte`,
`diagnostics/DiagnosticsScreen.svelte`, `onboarding/OnboardingFlow.svelte` (shell only; content
in phase 17).
Create — copy/tests: `src/lib/copy/en.ts`, `src/lib/formatting/{time.ts,number.ts,bytes.ts}`,
`src/lib/testing/six-states.ts`, `src/features/__tests__/six-states.test.ts`,
`src/components/ui/__tests__/{dialog-a11y.test.ts,button.test.ts}`, `scripts/guard-contrast.ts`,
`vitest.config.ts`.
Modify: `src/main.ts`, `src-tauri/tauri.conf.json` (window: `decorations: false`, `width 1280`,
`height 840`, `minWidth 900`, `minHeight 600`, `center: true`), `docs/PRODUCT.md`.

## Implementation Steps

1. Write `tokens.css` with all §41 tokens (background, surface-1/2/3, border-subtle, border-active, text-primary/secondary/muted, accent, success, warning, danger, info, shadow-sm/md/lg, radius-sm/md/lg/xl, space-1…10, durations, easings) for dark and light; `theme.svelte.ts` resolves `System` via `window.matchMedia('(prefers-color-scheme: dark)')` and writes `data-theme` on `<html>`.
2. `src/lib/copy/en.ts` — one typed const tree holding **every** user-visible string, seeded with the exact §72 lines ("Everything looks normal.", "2 recent changes may need attention.", "What stopped working?", "Example: Discord stopped detecting my microphone this morning", "Important changes", "Investigate", "Your history stays on this PC."), the §43 counters row, §46 drawer labels, §47 checkpoint actions, §50 onboarding steps, §53 notification texts, and error copy per `AppError` variant. Tests assert screens render these exact strings.
3. `StateShell.svelte` — takes `state: 'loading' | 'empty' | 'error' | 'ready'` plus `empty`/`error`/`children` snippets; loading renders `Skeleton` shapes matched to the real layout (not a spinner); error renders `ErrorState` with the typed message plus a "Try again" action; empty renders purposeful copy (e.g. Timeline empty = "No changes recorded in this range yet." + "Monitoring started 4 minutes ago.").
4. `AppFrame.svelte` — CSS grid: 44 px titlebar (`data-tauri-drag-region`, app mark + title + window controls with `aria-label`s), 64 px nav rail (icon + label, collapses to icons at `compact`), main area with `PageHeader`. `layout.svelte.ts` uses `ResizeObserver` to set `compact` (< 1100 px) and `dense` (< 960 px) classes; hard floor enforced by `minWidth/minHeight`.
5. `NavRail.svelte` — Home, Timeline, Investigations, Checkpoints, System Map, Settings, Diagnostics; roving `tabindex`, Up/Down arrows move, Enter/Space activates, `aria-current="page"`; `router.svelte.ts` is a rune-based store (`$state` route + params) — no router dependency.
6. Build primitives by copying Bits UI *patterns* (focus trap, `aria-modal`, labelled control association) into our own components and styling them from tokens only. `Dialog`/`Drawer` implement focus trap, Escape close, focus restore, scroll lock, and `aria-labelledby`/`describedby`.
7. Screens: each renders `PageHeader` + `StateShell` and static illustrative content using real copy. Home lays out exactly the §43 structure (status line + "● Active" pill, prompt, input with the §72 placeholder, Investigate button, "Today 37 changes / 2 important / 0 critical" counter row, "Important changes" list, privacy line). Data hooks are typed `undefined` placeholders that the later phase replaces — no fake numbers ship: counters read from `get_dashboard_summary` once phase 07 lands, and until then Home renders the `loading` state under a `docs/ROADMAP.md`-tracked stub entry.
8. `vitest.config.ts` — `environment: 'jsdom'` (install `jsdom` via `bun add -d jsdom`, record resolved version), `setupFiles` registering `@testing-library/svelte` cleanup, `globals: true`.
9. `six-states.test.ts` — table-driven over 6 screens × 6 states = 36 cases: loading/empty/error/ready via props, `large-data` by feeding 5 000 synthetic rows, `small-window` by setting the container to 900×600 and asserting no horizontal overflow (`scrollWidth <= clientWidth`) and that nav labels collapse. Each case also asserts the required microcopy string is present.
10. `scripts/guard-contrast.ts` — parses `tokens.css`, computes WCAG 2.1 relative-luminance contrast for the enumerated foreground/background pairs (text-primary/secondary/muted on background and surface-1/2/3, accent on background, status hues on surface-2) in both themes, fails under 4.5:1 (3:1 for ≥ 24 px display text, documented per pair).
11. `dialog-a11y.test.ts` — Tab cycles inside the dialog only, Escape closes, focus returns to the invoker, `aria-modal="true"` present.

## Todo

- [ ] Commit 1 — `feat(ui): design tokens, tailwind 4 theme bridge and base styles`
- [ ] Commit 2 — `feat(ui): primitive component set with accessible dialog/drawer`
- [ ] Commit 3 — `feat(app): custom titlebar, nav rail and responsive app frame`
- [ ] Commit 4 — `feat(app): seven routed screens with six-state shells and real microcopy`
- [ ] Commit 5 — `test(ui): 36-case six-state matrix, dialog a11y and contrast guard`

## Success Criteria

- [ ] 36/36 six-state cases pass; every case asserts a real copy string
- [ ] `bun run guard:contrast` exits 0 for dark and light
- [ ] Theme switch (Dark/Light/System) persists across restart via `settings`
- [ ] Keyboard-only walkthrough reaches every interactive element with a visible focus ring; no focus trap outside dialogs
- [ ] Window at 900×600 shows no horizontal scrollbar and no clipped control (asserted in test + manual screenshot check)
- [ ] `prefers-reduced-motion` reduces all token durations to 1 ms (asserted in test)
- [ ] No `any`, no inline hex colours outside `tokens.css` (grep check in `guard:contrast`)

## Verification Gate

```bash
bun run check && bun run test
bun run guard:contrast && bun run guard:types
bun run tauri dev    # manual: resize to 900x600, toggle theme, tab through Home
```

## Risk Assessment

- **Hand-rolled Dialog/Select fails accessibility** — signal: `dialog-a11y.test.ts` red or keyboard trap escapes. Response: install `bits-ui@2.19.0` and restyle it entirely from tokens (still not a shadcn look); record the decision in `ARCHITECTURE.md`.
- **Tailwind 4 `@theme` bridge does not pick up variables** — signal: utilities render unstyled. Response: verify `@import "tailwindcss"` precedes `@theme`, and that `@tailwindcss/vite` is registered before `svelte()`; if the bridge still fails, use plain token classes in `base.css` and keep Tailwind for layout only.
- **Design drifts into "generic dashboard" (§70)** — signal: a screen consists of equal cards on a flat background with no hierarchy. Response: the §70 unacceptable list is a checklist item in this phase's review; a screen fails review until it has one dominant element, one accent use, and deliberate density.
- **Copy drifts from §72-§74** — signal: a six-state test fails on a string. Response: strings live only in `copy/en.ts`; components never inline user-visible text.
- **jsdom cannot host `ResizeObserver`** — signal: layout test throws. Response: polyfill `ResizeObserver` in `setupFiles` with a minimal stub and assert the class-mapping function directly.

## Security Considerations

CSP forbids remote scripts and fonts; the phase adds no external asset, so §63 offline behaviour is
preserved by construction. The custom titlebar uses `data-tauri-drag-region` only — no extra window
permission beyond the four enumerated in phase 04. All rendered strings pass through Svelte's default
escaping; no `{@html}` is used anywhere (guard: grep in `guard-types.ts` denylist).

### FILE: phase-06-tray-lifecycle-monitoring-state.md

---
phase: 06
title: "Tray, Window Lifecycle & Monitoring State Machine"
status: pending
priority: P1
effort: 6h
dependencies: [05]
---

## Overview

Turn the shell into a resident background app: system tray carrying **exactly** §7's seven entries,
close-to-tray, single instance, restored window geometry, optional autostart, and the monitoring
state machine (`Active | Paused | Starting | Degraded`) that every later collector reports into.
Closes the §69 PHASE 0 acceptance criterion — "app runs, sits in tray, survives close, shows an
empty timeline".

## Requirements

Functional: tray menu contains exactly the §7 entries, in this order and with these labels —
"Open What Changed" / "What changed today?" / "Create checkpoint" / "Pause monitoring" /
"Resume monitoring" / "Settings" / "Quit". Pause and Resume are both present as §7 requires; the one
that does not apply to the current state is `enabled(false)` rather than hidden, so the menu shape never
changes. Visual separators are permitted (they are dividers, not entries); **no additional entry** may
be introduced — the today-count lives in the tray tooltip, not the menu. "What changed today?" opens
the Timeline pre-filtered to today. Tray icon reflects state (normal / paused / attention); left-click
shows+focuses the window. Closing the window hides it and shows the §53 first-time-only balloon
"What Changed is still recording. Find it in the system tray." Second launch focuses the existing
window instead of starting a process. Non-functional: pause must stop *collection*, not just UI
(`MonitoringState` is checked by the supervisor before every poll); resume must not emit a burst of
stale events (see step 4).

## Architecture

`state/monitoring.rs` owns `MonitoringState` in an `Arc<RwLock<…>>` inside `AppState` plus a
`tokio::sync::watch` channel that collectors subscribe to, so a pause takes effect at the next await
point without polling a mutex. Transitions: `Starting` at boot until the first successful collector
tick; `Active` afterwards; `Paused` on user action (persisted to `settings.monitoring_paused` per §49 so
it survives restart); `Degraded` when the supervisor reports ≥1 permanently-failed collector or
storage is in recovery mode. Every transition emits `monitoring-state-changed` and updates the tray
(enable/disable Pause vs Resume, swap the icon).

The tray **tooltip** (`tray.set_tooltip`) carries the live summary — "What Changed — Active · 12
changes today" — refreshed on state change, on checkpoint completion, and on a 60 s tick from a cached
counter in `AppState` (no DB query on the tray path: the counter is incremented by the writer thread
and rolled over at local midnight computed from the OS timezone offset). Keeping the count out of the
menu is what preserves §7's exact entry list.

## Related Code Files

Create: `src-tauri/src/state/monitoring.rs`, `src-tauri/src/app/tray.rs`,
`src-tauri/src/app/window.rs`, `src-tauri/src/app/lifecycle.rs`,
`src-tauri/src/commands/monitoring.rs`, `src-tauri/src/app/counters.rs`,
`src-tauri/tests/monitoring_state.rs`, `src/lib/stores/monitoring.svelte.ts`,
`src/components/ui/MonitoringPill.svelte`, `docs/OPERATIONS.md` (tray + lifecycle section).
Modify: `src-tauri/src/lib.rs` (plugin registration order), `src-tauri/Cargo.toml`
(`tauri-plugin-single-instance` 2.4.3, `tauri-plugin-autostart` 2.5.1,
`tauri-plugin-notification` 2.3.3, `tauri-plugin-window-state` 2.4.1), `src-tauri/tauri.conf.json`,
`src-tauri/capabilities/default.json`, `src/lib/copy/en.ts`, `src/features/settings/SettingsScreen.svelte`.

## Implementation Steps

1. Register plugins in `lib.rs` in this order — `single_instance` **first** (its callback must run before any window is created), then `window_state`, `notification`, `autostart`, `dialog`, `opener`, `updater` (added in phase 17). `single_instance` callback: `if let Some(w) = app.get_webview_window("main") { w.unminimize().ok(); w.show().ok(); w.set_focus().ok(); }`.
2. `tray.rs` — build with `TrayIconBuilder::with_id("main")`, `MenuBuilder` with the seven §7 items (`MenuItemBuilder::with_id("open"|"today"|"checkpoint"|"pause"|"resume"|"settings"|"quit")`, pause/resume toggled via `set_enabled`), `.tooltip(...)`, `.show_menu_on_left_click(false)` plus a `TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }` arm that shows the window. Three ICO variants (`tray-normal.ico`, `tray-paused.ico`, `tray-attention.ico`) generated by extending `scripts/make-icon.ps1`. Read the installed `tauri` 2.11.5 source under `~/.cargo/registry` for the exact `TrayIconBuilder` / `MenuBuilder` / `set_tooltip` method names before writing — do not guess method spellings.
3. `window.rs` — `on_window_event` handler: `WindowEvent::CloseRequested { api, .. } => { api.prevent_close(); window.hide()?; notify_tray_hint_once()?; }`. `notify_tray_hint_once` checks `settings.tray_hint_shown`, sends the §53 notification through `tauri_plugin_notification`, then sets the flag.
4. `monitoring.rs` — enum + `watch::Sender`, `pause()`/`resume()`/`mark_degraded(reason)`/`mark_active()`, each persisting where relevant and emitting the event. `resume()` sets a `resume_at` instant; collectors that resume re-baseline (take a fresh snapshot and **discard** the first diff) so a 3-hour pause does not dump hundreds of events at once — instead a single `MonitoringResumed` event records "Monitoring resumed after 3h 12m. Changes during the pause were not recorded." This is the §54 / §87 honesty rule (state what is unknown; never fabricate), not a shortcut.
5. `commands/monitoring.rs` — `get_monitoring_state`, `set_monitoring_paused(paused: bool)`, `get_dashboard_summary` (returns today's counts + state + `monitoring_started_at`); all added to the specta builder and regenerated bindings.
6. `counters.rs` — `TodayCounters { total, important, critical, day_start_epoch_ms }` updated inside the writer thread commit (single place, no double counting), rolled over when `now >= day_start + 86_400_000` recomputed from `GetTimeZoneInformation`-derived local midnight; restored at boot with one `SELECT COUNT(*) … WHERE timestamp >= ?` per severity bucket.
7. `autostart` — off by default; Settings toggle calls `enable()`/`disable()` and reads `is_enabled()` on mount so the UI reflects reality even if the registry entry was removed externally. Uses `MacosLauncher`-free desktop init (`init(None)`), registry `Run` key — non-elevated, verified by matrix row 21.
8. Frontend: `monitoring.svelte.ts` subscribes to `monitoring-state-changed` and hydrates from `get_monitoring_state`; `MonitoringPill` renders "● Active" / "❚❚ Paused" / "◐ Starting" / "▲ Degraded (1 collector unavailable)" with the exact §43 wording for Active.
9. `monitoring_state.rs` tests: pause→resume transition emits exactly one resume event; `Degraded` set/cleared by supervisor reports; counter rollover at a synthetic midnight; paused state survives a simulated restart (fresh `AppState` from the same settings row).

## Todo

- [ ] Commit 1 — `feat(app): single instance, window state and close-to-tray lifecycle`
- [ ] Commit 2 — `feat(app): system tray menu with live monitoring state and counts`
- [ ] Commit 3 — `feat(core): monitoring state machine with pause-aware re-baselining`
- [ ] Commit 4 — `feat(ui): monitoring pill, settings toggles and autostart control`
- [ ] Commit 5 — `test(app): monitoring transitions, counter rollover and restart persistence`

## Success Criteria

- [ ] §69 PHASE 0 gate: app builds, launches, sits in tray, survives window close, shows an empty timeline with a purposeful empty state
- [ ] All seven §7 tray entries present with the exact labels and no extras; tooltip count updates within 60 s of a new event
- [ ] Second launch focuses the existing window; only one process in Task Manager
- [ ] Window position/size restored after restart (window-state plugin); first run centres at 1280×840
- [ ] Pause survives restart; resume emits one explanatory event and no backlog burst
- [ ] Tray hint notification appears exactly once ever (flag persisted)

## Verification Gate

```bash
cd src-tauri && cargo test --lib && cargo test --test monitoring_state
cd .. && bun run check && bun run tauri dev
# manual: close window -> tray only; relaunch exe -> focuses existing; pause -> resume after 2 min
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1   # B2 checkpoint #2
```

## Risk Assessment

- **Tray API method names differ from memory in 2.11.5** — signal: compile error on `TrayIconBuilder`. Response: step 2 mandates reading the vendored `tauri` source before writing; treat the registry source as the only API authority.
- **`prevent_close` + `hide` leaves a zombie window on quit** — signal: process lingers after "Quit". Response: the Quit handler sets an `is_quitting` flag consulted by the close handler, then calls `app.exit(0)` after flushing the writer queue (bounded 2 s join).
- **Autostart entry written but app starts before the user session is ready** — signal: launch fails at boot. Response: startup tolerates missing WebView2 window creation by retrying once after 3 s; failure path logs and exits non-zero rather than looping.
- **Notification permission denied by Focus Assist** — signal: no balloon. Response: the hint is best-effort; Settings also states the tray behaviour in text so the information is not notification-dependent.
- **Counter drift vs DB** — signal: tray count ≠ timeline count. Response: a nightly rollover recomputes from SQL; a debug assertion compares counter vs `COUNT(*)` in dev builds.

## Security Considerations

Autostart writes only `HKCU\...\Run` with the app's own path and no arguments — no elevation, no
service, no scheduled task, matching the §7 non-goal "No Windows Service in MVP" and the §36 privilege
model. Pausing must genuinely halt collection so the UI's promise is not a lie (§54, §87). The tray
tooltip exposes only an aggregate number, never event text, so a shoulder-surfer sees no change
content.

### FILE: phase-07-event-engine-live-updates.md

---
phase: 07
title: "Event Engine, Query Layer, Clustering & Live Updates"
status: pending
priority: P1
effort: 10h
dependencies: [06]
---

## Overview

Build the event pipeline between the sanitizer and the UI: the `EventBus` every collector will later
publish into, server-side query/filter/pagination, event clustering (§44 "group related events"),
retention enforcement wiring (§11), coalesced live-update events (§57), and a synthetic event
generator so the timeline, search and analysis phases can be built and load-tested before a single
Windows API is called.

## Requirements

Functional: `EventBus::publish(EventDraft)` → sanitize → persist → counters → coalesced UI notify;
queries filter by time range, categories, severities, importance floor, entity, free text, and
`cluster` on/off; clustering groups same-category events within a 60 s window and ≥3 members into one
parent row that expands; retention deletes by `settings.retention_days` on a schedule (§11 defaults:
30 d events, 7 d snapshots, 90 d important); the UI receives `system-event-added` at most every 250 ms
with a batch payload (§57). Non-functional: query for 1 000 rows from a 1 M-row table < 100 ms measured
(§61); no unbounded channel — a full queue drops with a counted warning, never blocks a collector.

## Architecture

```
Collector -> mpsc::Sender<EventDraft>(cap 4096) -> EventBus task
   EventBus: redact::sanitize -> dedupe window -> writer queue (phase 03)
                                             \-> counters.increment
                                             \-> ui_notify.coalesce(250ms)
Frontend  <- emit("system-event-added", { count, latest: EventSummary[8] })
```

Dedupe: an in-memory `HashMap<u64 /*fingerprint*/, i64 /*last_seen_ms*/>` with a 5 s window collapses
the identical repeated event a diff-poller can produce across two consecutive ticks (e.g. a service
flapping mid-poll). Fingerprint = FNV-1a over `(category, entity_key, change_kind, old_value,
new_value)` — the same function already used for `events.fingerprint` in phase 03. Map is swept every
60 s to bound memory.

Clustering is computed in SQL, not in the client: a window function over
`ORDER BY timestamp` assigns a group when `category` matches and gap ≤ 60 s; the query returns either
cluster parents (`is_cluster=1, member_count=N, severity=max`) or raw rows depending on the request
flag, so a 500-event burst renders as one row. Cluster children are fetched on expand via
`get_cluster_members(cluster_key, limit)`.

## Related Code Files

Create: `src-tauri/src/core/bus.rs`, `src-tauri/src/core/dedupe.rs`,
`src-tauri/src/core/query.rs`, `src-tauri/src/core/cluster.rs`,
`src-tauri/src/core/retention.rs`, `src-tauri/src/core/notify_ui.rs`,
`src-tauri/src/dev/synthetic.rs`, `src-tauri/src/commands/events.rs`,
`src-tauri/tests/{event_pipeline.rs,query_filters.rs,clustering.rs,retention.rs}`,
`src-tauri/benches/../tests/query_perf.rs` (as `#[ignore]`d perf test, not a bench crate),
`src/lib/stores/events.svelte.ts`, `src/lib/api/events.ts`.
Modify: `src-tauri/src/storage/repository.rs` (query builder + cluster SQL),
`src-tauri/src/commands/mod.rs`, `src/lib/copy/en.ts`, `docs/ARCHITECTURE.md`.

## Implementation Steps

1. `bus.rs` — `EventBus { tx: mpsc::Sender<EventDraft> }` cloneable handle given to every collector; the consumer task runs `while let Some(draft) = rx.recv().await`, calls `redact::sanitize`, then `dedupe.should_emit(&ev)`, then `writer.enqueue(ev)`. `try_send` on the collector side: on `Full`, increment `metrics.dropped_events` and log at WARN once per 30 s (rate-limited), never `await` — a slow disk must not stall a collector's region.
2. `query.rs` — `EventQuery { range: TimeRange, categories: Vec<Category>, severities: Vec<Severity>, min_importance: u8, entity_key: Option<String>, text: Option<String>, cluster: bool, cursor: Option<Cursor>, limit: u16 }`. Build parameterized SQL with a fixed skeleton and only bound values — never string-interpolate user text. `limit` clamped by `core/validate.rs` (1..=200). Cursor is the keyset pair from phase 03.
3. `cluster.rs` — the grouping SQL plus `get_cluster_members`. Verify the SQLite version bundled by `rusqlite` 0.40.2 supports the window functions used (`SELECT sqlite_version()` printed by a test); if a needed function is unavailable, fall back to a single-pass Rust grouping over the already-paged rows and record the deviation in `ARCHITECTURE.md`.
4. `retention.rs` — a `tokio::time::interval(Duration::from_secs(3600))` task calling the chunked deletion from phase 03 with `retention_days` read fresh each run; skips checkpoint-referenced events (`FOREIGN KEY` + `WHERE id NOT IN (SELECT event_id FROM checkpoint_events)`); emits a single `RetentionRun` log line with rows deleted and duration. Manual "Delete all history" (§33) reuses the same module with `days = 0` inside one transaction plus `VACUUM`.
5. `notify_ui.rs` — coalescer: on first event start a 250 ms timer, accumulate `count` and the last 8 `EventSummary` values, then `app.emit("system-event-added", payload)`. One in-flight timer maximum; if the window closes while hidden, keep coalescing (cheap) — the UI re-queries on show anyway.
6. `commands/events.rs` — `query_events(EventQuery) -> EventPage`, `get_event(id) -> EventDetail`, `get_cluster_members(...)`, `get_dashboard_summary()` (extended), `get_event_categories() -> Vec<CategoryInfo>` (label + description + `coverage_limits` for §54/§87 honesty), `delete_all_history()` (requires an explicit `confirm: "DELETE"` argument so a stray call cannot wipe data).
7. `dev/synthetic.rs` — `#[cfg(any(debug_assertions, feature = "devtools"))]` generator producing a realistic distribution (weights per category taken from the §9 category enum; realistic entity names; timestamps spread over N days) plus `inject_events(count, spread_days)` and `inject_scenario(name)` for the §59 fixture used in phase 13. Guarded additionally by `settings.developer_mode` per phase 04.
8. Frontend `events.svelte.ts` — `$state` page cache, `loadMore()` keyset paging, filter object, `system-event-added` listener that increments a "N new changes" pill rather than reordering the list under the user's cursor (§42 no-jank rule); clicking the pill prepends.
9. `query_perf.rs` (`#[ignore]`) — seeds 1 M synthetic events, asserts: filtered page of 1 000 rows < 100 ms, cluster query < 150 ms, `COUNT` summary < 50 ms; prints timings so the numbers in `docs/PERFORMANCE.md` are measured, never claimed.

## Todo

- [ ] Commit 1 — `feat(core): event bus with sanitize, dedupe window and bounded queue`
- [ ] Commit 2 — `feat(core): parameterized event query, keyset paging and filters`
- [ ] Commit 3 — `feat(core): SQL event clustering with expandable members`
- [ ] Commit 4 — `feat(core): retention scheduler and confirmed history deletion`
- [ ] Commit 5 — `feat(core): 250ms coalesced live updates and dev synthetic generator`
- [ ] Commit 6 — `test(core): pipeline, filters, clustering, retention and 1M-row perf`

## Success Criteria

- [ ] End-to-end test: `publish(draft)` containing a token → row in DB with the token redacted, counter +1, one coalesced UI event
- [ ] 100 drafts published in 50 ms produce exactly one `system-event-added` with `count: 100`
- [ ] Identical draft published twice within 5 s stores one row; after 6 s stores two
- [ ] Every filter dimension asserted independently and in combination (12 cases)
- [ ] Cluster: 40 same-category events in 30 s render as 1 parent with `member_count: 40`
- [ ] Retention deletes older-than-N rows, preserves checkpoint-referenced rows, and reports duration
- [ ] Perf test at 1 M rows meets the three §61 budgets on this machine (recorded in `docs/PERFORMANCE.md`)

## Verification Gate

```bash
cd src-tauri && cargo test --lib && cargo test --test event_pipeline --test query_filters --test clustering --test retention
cargo test --test query_perf -- --ignored --nocapture   # records real timings
cd .. && bun run check && bun run test && bun run guard:redaction && bun run guard:types
```

## Risk Assessment

- **Window-function clustering unsupported by the bundled SQLite** — signal: `no such function` at test time. Response: step 3's Rust-side grouping fallback; identical output shape so no UI change.
- **Coalescing hides genuinely urgent events** — signal: a critical event waits 250 ms. Response: acceptable and documented; severity `Critical` additionally triggers the §53 notification path immediately in phase 17, bypassing the coalescer.
- **Dedupe window swallows real repeats** — signal: a legitimately repeated change (service restarted twice in 5 s) disappears. Response: fingerprint includes `new_value` and a `repeat_count` column increments instead of dropping silently, so the row shows "×3" rather than lying.
- **1 M-row seeding is too slow to run routinely** — signal: perf test takes minutes. Response: it is `#[ignore]`d and run at phase gates only; seeding uses one transaction with `prepare_cached`.
- **Unbounded memory in the dedupe map** — signal: RSS climbs during burst tests. Response: 60 s sweep + hard cap of 50 000 entries with oldest-first eviction, asserted by a test that publishes 200 000 unique drafts.

## Security Considerations

All user-controlled query input reaches SQLite only as bound parameters; the free-text filter is
matched with `LIKE ?` / FTS `MATCH ?` — the frontend can never supply SQL (§35, §56).
`delete_all_history` requires the literal confirm token, is logged, and is the only destructive
command exposed. The synthetic generator is compile-gated *and* runtime-gated, so a release build has
no path to fabricate events (§87).

### FILE: phase-08-timeline-search-virtualization.md

---
phase: 08
title: "Timeline, Details Drawer, Search & Virtualization"
status: pending
priority: P1
effort: 12h
dependencies: [07]
---

## Overview

The primary reading surface (§44): a virtualized, filterable, keyboard-driven timeline with day
grouping and density banding, the event details drawer (§46), and search over recorded history (§52).
This is where §61's "1 M events stay responsive" is proven with real UI, not a benchmark.

## Requirements

Functional: reverse-chronological list with sticky day headers ("Today", "Yesterday", "Tue 26 Aug"),
per-row category glyph, importance dot, time (tabular), one-line summary, entity chip; filter bar
(range presets Last hour / Today / 7 days / 30 days / Custom, category multiselect, severity, "Important
only", search box); cluster rows expand in place; row click opens the drawer with §46's sections (What
changed, When, Category & importance, Technical details as key/value, Related events, Same-entity
history, Actions: copy details / create checkpoint here / start investigation from here); search
debounced 200 ms with match highlighting; `Ctrl+K` command palette jumps to screens, filters and
recent entities; `Ctrl+F` focuses search; `J`/`K` or arrows move selection; `Enter` opens drawer;
`Esc` closes. Non-functional: 60 fps scroll at 100 k rows; constant DOM node count; no layout shift when
new events arrive.

## Architecture

Virtualization via `@tanstack/svelte-virtual` 3.13.36 (unverified assumption A10 — Svelte 5
compatibility). Isolation strategy: all virtualizer usage lives behind
`src/components/list/VirtualList.svelte` exposing `{ items, estimateSize, overscan, children }`. If the
package misbehaves under runes, only that file changes — a hand-rolled fallback
(`VirtualList.manual.svelte`: fixed 36 px row height, `scrollTop`-derived window, absolute-positioned
spacer divs, `ResizeObserver` for viewport height) is written **in this phase**, not deferred, and a
Vitest case runs the shared contract test against both implementations.

Data flow: `events.svelte.ts` holds pages keyed by cursor; the virtualizer's `range.end` crossing
`items.length - 40` triggers `loadMore()`; day headers are computed client-side from the returned
timestamps using the local timezone; the drawer fetches `get_event(id)` for full technical details
rather than fattening the list payload (`EventSummary` stays ≤ 8 fields).

## Related Code Files

Create: `src/components/list/{VirtualList.svelte,VirtualList.manual.svelte,virtual-contract.ts}`,
`src/features/timeline/{TimelineScreen.svelte,FilterBar.svelte,EventRow.svelte,ClusterRow.svelte,DayHeader.svelte,DensityBand.svelte,EventDrawer.svelte,TechnicalDetails.svelte,RelatedEvents.svelte,EntityHistory.svelte}`,
`src/features/search/{SearchScreen.svelte,SearchResultRow.svelte,highlight.ts}`,
`src/components/palette/CommandPalette.svelte`, `src/lib/stores/selection.svelte.ts`,
`src/lib/keyboard/shortcuts.ts`,
`src/features/timeline/__tests__/{timeline.test.ts,filters.test.ts,drawer.test.ts,virtual-both.test.ts}`,
`src/features/search/__tests__/search.test.ts`, `tests/e2e/timeline.spec.ts`.
Modify: `src-tauri/src/storage/repository.rs` (FTS or LIKE search + `search_events` +
`get_entity_history`), `src-tauri/src/commands/events.rs`, `src-tauri/migrations/` (new migration
`0002_search.sql`), `src/lib/copy/en.ts`, `docs/PERFORMANCE.md`.

## Implementation Steps

1. Migration `0002_search.sql` — attempt `CREATE VIRTUAL TABLE events_fts USING fts5(summary, detail_text, entity_name, content='events', content_rowid='id')` plus the three sync triggers. A startup probe (`SELECT 1 FROM pragma_module_list WHERE name='fts5'`) records `search_backend = 'fts5' | 'like'` in `AppState`; the migration is written so that an FTS5-less build still succeeds (the `CREATE VIRTUAL TABLE` lives in its own migration step wrapped by the probe, executed from Rust rather than raw SQL, and skipped when unavailable). Verify FTS5 presence in `rusqlite` 0.40.2's bundled amalgamation with a test that prints the backend — do not assume.
2. `search_events(text, filters, cursor)` — FTS path uses `MATCH ?` with the user string escaped into a quoted phrase (double any `"`), ranks by `bm25(events_fts)` then recency; LIKE path uses `summary LIKE ? ESCAPE '\'` with `%`/`_`/`\` escaped, over an indexed prefix where possible, and hard-limits scanned rows via the time window. Both return the same `EventPage` shape and a `backend` field the UI shows in Diagnostics.
3. `VirtualList.svelte` + `.manual.svelte` + `virtual-contract.ts` (shared assertions: renders ≤ overscan+viewport rows, total scroll height = rows × size, `scrollToIndex` lands within 1 px, no duplicate keys). `virtual-both.test.ts` runs the contract against both components.
4. `EventRow.svelte` — 36 px, grid `[glyph 20px][time 64px][importance 8px][summary 1fr][entity auto]`, `role="option"` inside `role="listbox"`, `aria-selected`, hover reveals a right-aligned "Details" affordance; importance dot colour from severity token with a text label in the `title`/`aria-label` (never colour-only, §62).
5. `DensityBand.svelte` — a 24 px strip above the list showing per-hour event counts for the selected range as thin bars (pure SVG, no Three.js) so a burst is visible before scrolling; clicking a bar scrolls the list to that hour. Counts come from one `get_event_histogram(range, bucket)` command (SQL `GROUP BY timestamp / bucket`).
6. `FilterBar.svelte` — presets + multiselects backed by `get_event_categories()`; the active filter set is reflected in the URL-less router params store and restored on return to the screen; "Clear filters" appears only when a filter is active.
7. `EventDrawer.svelte` — right-side 480 px drawer (full width under 1100 px), sections per §46, `TechnicalDetails` renders `attributes` as a two-column key/value list with a copy-row button and `sensitive: true` values rendered as "[redacted]" (they are already redacted at rest; the flag only drives the label). "Copy details" writes a plain-text block via `navigator.clipboard.writeText` (no clipboard plugin needed — WebView2 supports it in a secure context) and falls back to a hidden textarea + `document.execCommand('copy')` if the promise rejects.
8. `EntityHistory` — `get_entity_history(entity_key, limit 20)`; `RelatedEvents` — `get_related_events(event_id, window_secs 300)` returning same-entity and same-category neighbours (the analysis phase later replaces the ranking, not the shape).
9. `shortcuts.ts` — single global `keydown` listener with a target guard (ignore when `event.target` is an input unless the combo is `Ctrl+`-prefixed), registry mapping combos to actions, `?` opens a shortcut sheet; all combos listed in `docs/PRODUCT.md`.
10. Perf: `timeline.test.ts` asserts DOM node count stays < 120 rows when scrolled through 100 k synthetic items; `tests/e2e/timeline.spec.ts` (Playwright, `#[ignore]`-equivalent via a `@slow` tag) drives the real app if driving Tauri works (assumption A11) — if it does not, delete the spec and record a manual test script in `docs/TESTING.md` instead of leaving a skipped test.

## Todo

- [ ] Commit 1 — `feat(storage): search backend probe, fts5-or-like search and histogram query`
- [ ] Commit 2 — `feat(ui): virtual list with vendored and hand-rolled implementations`
- [ ] Commit 3 — `feat(timeline): day-grouped rows, density band and filter bar`
- [ ] Commit 4 — `feat(timeline): event details drawer with technical details and history`
- [ ] Commit 5 — `feat(app): search screen, command palette and keyboard shortcuts`
- [ ] Commit 6 — `test(timeline): 100k-row DOM bounds, filters, drawer and search`

## Success Criteria

- [ ] 100 k synthetic events: scroll top→bottom keeps DOM rows < 120; measured frame time recorded in `docs/PERFORMANCE.md`
- [ ] 1 M events: filtered first page renders in < 1 s wall clock from click (measured, screenshot of the diagnostics timing panel)
- [ ] Search returns correct rows for both backends; the active backend is displayed in Diagnostics
- [ ] Drawer shows all seven §46 sections with real values; "Copy details" produces a paste-able block
- [ ] Keyboard: `Ctrl+K`, `Ctrl+F`, `J`/`K`, `Enter`, `Esc` all work; `?` lists them
- [ ] New events arriving during scroll never move the selected row (asserted by test)
- [ ] Both virtual implementations pass the identical contract test

## Verification Gate

```bash
cd src-tauri && cargo test --lib && cargo test --test query_filters
cd .. && bun run check && bun run test && bun run guard:types
bun run tauri dev   # inject 100k via dev panel, scroll, filter, search, open drawer
```

## Risk Assessment

- **A10 fails: `@tanstack/svelte-virtual` incompatible with Svelte 5 runes** — signal: reactivity lost or measurement loop. Response: switch `VirtualList.svelte` to re-export the manual implementation (one-line change), drop the dependency, note it in `ARCHITECTURE.md`. Both are already tested, so cost is minutes.
- **FTS5 absent from the bundled amalgamation** — signal: probe reports `like`. Response: LIKE path with time-bounded scans; Diagnostics states the reduced search quality plainly rather than pretending parity.
- **A11 fails: Playwright cannot drive the Tauri window** — signal: WebDriver cannot attach. Response: remove the e2e spec, keep Vitest + a written manual script; do not leave permanently-skipped tests (a stub-completed phase is a disqualifier).
- **Sticky day headers jitter with variable row heights** — signal: header flicker. Response: fixed 36 px rows (cluster rows also 36 px collapsed) keeps the virtualizer in fixed-size mode; expanded clusters render as a separate absolutely-measured block with `measureElement`.
- **Search over 1 M rows on the LIKE path is slow** — signal: > 1 s. Response: the LIKE path requires a time range (defaults to 30 days) and states "searching last 30 days" in the UI; unbounded search is offered only on FTS5.

## Security Considerations

Search text is bound, never interpolated; the FTS phrase quoting is unit-tested with `" OR 1=1` and
`*` inputs. `TechnicalDetails` renders text nodes only — no `{@html}`, so a filename containing markup
cannot inject. Clipboard writes contain already-redacted values; the copy block includes a footer line
"Values may be redacted for privacy." so a user pasting into a support thread is not misled (§34, §72).

### FILE: phase-09-collector-framework-process-ports.md

---
phase: 09
title: "Collector Framework, Supervisor & First Collectors (Process, Ports)"
status: pending
priority: P1
effort: 12h
dependencies: [07]
---

## Overview

Define the collector contract from §5 verbatim, the supervisor that owns lifecycle and panic
isolation, the `CollectorDescriptor` that carries honest coverage limits (§12, §54, §87 — B3), and the two
cheapest measured collectors — processes (6.2 ms warm) and network ports — as the reference
implementations every later collector copies. Runs in parallel with phase 08 (disjoint files).

## Requirements

Functional: `Collector` implements exactly the §5 shape; the supervisor starts enabled collectors,
restarts a failed one with exponential backoff (1 s → 2 → 4 → 8 → 16 → 30 s cap, 5 consecutive
failures ⇒ `Unavailable` + `Degraded` monitoring state), and never lets one collector's panic take the
app down; process collector emits Started/Stopped with name, PID, exe path, version (§14) and
parent-process attribution where obtainable; ports collector emits Opened/Closed with protocol, local
port, state, and owning process (§15). Non-functional: full process poll ≤ 15 ms p95 measured;
combined idle CPU contribution of both collectors < 0.3 % on this machine; every descriptor states its
blind spots in text that reaches the UI.

## Architecture

```rust
// core/collector.rs — §5 contract, unchanged shape
pub trait Collector: Send + Sync {
    fn id(&self) -> &'static str;
    fn start(&self, ctx: CollectorContext) -> Result<(), CollectorError>;
    fn stop(&self) -> Result<(), CollectorError>;
    fn snapshot(&self) -> Result<Vec<SystemEntity>, CollectorError>;
}

pub struct CollectorContext {
    pub bus: EventBus,                       // publish(EventDraft)
    pub monitoring: watch::Receiver<MonitoringState>,
    pub shutdown: CancellationToken,          // tokio-util 0.7.17
    pub handle: tokio::runtime::Handle,
    pub settings: Arc<RwLock<Settings>>,
}

pub struct CollectorDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub category: Category,
    pub mechanism: Mechanism,        // DiffPoll { interval_ms } | Notification | Hybrid
    pub default_enabled: bool,
    pub requires_elevation: bool,    // always false in MVP
    pub coverage_limits: &'static [&'static str],
    pub cost_note: &'static str,     // measured, e.g. "~6 ms per poll"
}
```

Supervisor owns `Vec<Box<dyn Collector>>` + per-collector `CollectorRuntime { state, last_ok,
consecutive_failures, events_emitted, last_error }`. Each collector runs in its own `tokio::task`;
the supervisor wraps each tick body in `std::panic::catch_unwind(AssertUnwindSafe(...))` for
synchronous Win32 sections and treats a `JoinError::is_panic()` as a failure to be restarted, so a
malformed registry blob or a Win32 edge case degrades one category instead of crashing the app.
`panic = "abort"` in the release profile (phase 01) would defeat `catch_unwind`, so the release profile
is changed to `panic = "unwind"` in this phase and `ARCHITECTURE.md` records why (panic isolation
beats the small binary-size win).

Diff-poll pattern (shared helper `core/diff.rs`): `diff_maps<K: Eq+Hash, V: PartialEq>(prev, next) ->
(added, removed, changed)` returning borrowed keys; collectors own their `HashMap` snapshot between
ticks. First tick after start or resume is a baseline: it populates the map and emits nothing (§54 —
the app must not report the entire machine as "changed" at launch).

## Related Code Files

Create: `src-tauri/src/core/{collector.rs,supervisor.rs,diff.rs,descriptor.rs}`,
`src-tauri/src/collectors/mod.rs`, `src-tauri/src/collectors/process.rs`,
`src-tauri/src/collectors/ports.rs`,
`src-tauri/src/platform/windows/{mod.rs,tcp_table.rs,file_version.rs,handles.rs}`,
`src-tauri/src/commands/collectors.rs`,
`src-tauri/tests/{supervisor.rs,diff_engine.rs,collector_process.rs,collector_ports.rs}`,
`src/features/settings/CollectorList.svelte`, `docs/COLLECTORS.md`.
Modify: `src-tauri/Cargo.toml` (`sysinfo` 0.39.6 default-features off + `system` feature,
`windows` 0.62.2 with the exact feature list, `tokio-util` 0.7.17), `src-tauri/src/lib.rs`,
`src-tauri/src/commands/mod.rs`, `src/lib/copy/en.ts`.

## Implementation Steps

1. `collector.rs`, `descriptor.rs` — the contract above; `CollectorError` variants `Unsupported`, `AccessDenied`, `Api { code: u32, call: &'static str }`, `Transient`, `Internal`. `AccessDenied` is terminal-but-honest: the collector reports `Unavailable { reason }` which the UI prints verbatim (B3 requirement).
2. `supervisor.rs` — `register`, `start_all`, `stop_all`, `restart(id)`, `states() -> Vec<CollectorStatus>`; backoff as specified; monitoring `watch` checked at the top of each tick (`if *rx.borrow() == Paused { sleep(500ms); continue; }`); shutdown via `CancellationToken` with a 2 s join budget then detach.
3. `diff.rs` — the generic differ plus a `Baseline` wrapper enforcing "first tick emits nothing"; property test: for random map pairs, `added ∪ changed ∪ unchanged = next.keys()` and `removed = prev \ next`.
4. `process.rs` — `sysinfo::System::new_with_specifics(RefreshKind::nothing().with_processes(ProcessRefreshKind::nothing().with_exe().with_user()))`; poll every 2 s (§14 forbids high-frequency polling; settings-tunable 1–30 s). Key = `(pid, start_time)` so PID reuse cannot alias two processes. Emits `ProcessStarted { name, pid, exe_path, file_version, parent_pid, parent_name }` and `ProcessStopped { name, pid, lifetime_ms }`. **No command line is read** — `sysinfo`'s `cmd()` is forbidden by the phase-02 guard because arguments routinely carry tokens (§34).
5. `file_version.rs` — `GetFileVersionInfoSizeW`/`GetFileVersionInfoW`/`VerQueryValueW` wrapper returning `Option<String>` (`FileVersion` + `ProductName`), with a 512-entry LRU keyed by `(path, size, mtime)` so a repeatedly-launched exe is versioned once. All calls inside `unsafe` blocks with explicit buffer sizing; a fuzz-ish test feeds a non-PE file and a missing path and asserts `None`, not a panic.
6. `ports.rs` — `GetExtendedTcpTable(AF_INET/AF_INET6, TCP_TABLE_OWNER_PID_ALL)` + `GetExtendedUdpTable(UDP_TABLE_OWNER_PID)`; two-call sizing pattern (query size, allocate, query); poll every 5 s; key = `(proto, local_addr, local_port, pid)`; emits `PortOpened/PortClosed` with `process_name` resolved from the process collector's live map when possible (shared read-only snapshot, no second enumeration). Loopback-only listeners are flagged `local_only: true` so the UI can de-emphasise them. Remote addresses of *outbound* connections are recorded as `remote_port` + `remote_scope: Loopback | Private | Public` only — **never the remote IP** (§34 privacy: no network-destination logging).
7. `commands/collectors.rs` — `list_collectors() -> Vec<CollectorInfo>` (descriptor + live status + events_emitted + last_error), `set_collector_enabled(id, enabled)`, `restart_collector(id)`; `CollectorList.svelte` renders name, mechanism, cost note, status pill, and the `coverage_limits` bullets verbatim.
8. `docs/COLLECTORS.md` — one section per collector generated from the descriptors by `scripts/gen-collector-docs.ts` (reads a `list_collectors` JSON dump produced by a `cargo test --test collector_docs -- --nocapture` fixture) so docs cannot drift from code.
9. Tests: `supervisor.rs` — a deliberately panicking fake collector restarts 5× then becomes `Unavailable` and sets `Degraded`, while a healthy fake keeps emitting; `collector_process.rs` — spawns `cmd.exe /c timeout 3` (via `std::process::Command` in the *test only*), asserts a Started event with matching pid/name and a Stopped event with plausible lifetime; `collector_ports.rs` — binds a `TcpListener` on an ephemeral loopback port, asserts a `PortOpened` with that port and `local_only: true`, drops it, asserts `PortClosed`.

## Todo

- [ ] Commit 1 — `feat(core): collector contract, descriptor and coverage-limit metadata`
- [ ] Commit 2 — `feat(core): supervisor with backoff, panic isolation and pause awareness`
- [ ] Commit 3 — `feat(core): generic diff engine with baseline-first semantics`
- [ ] Commit 4 — `feat(collectors): process collector with file-version enrichment`
- [ ] Commit 5 — `feat(collectors): tcp/udp port collector with scope-only remote data`
- [ ] Commit 6 — `feat(ui): collector list with honest coverage limits`
- [ ] Commit 7 — `test(collectors): supervisor, diff properties, process and port round-trips`

## Success Criteria

- [ ] Launching Notepad produces exactly one `ProcessStarted` within 2 s carrying exe path and file version; closing it produces one `ProcessStopped`
- [ ] App start emits zero events from the baseline tick (asserted)
- [ ] Panicking collector is restarted with the documented backoff and never crashes the process; monitoring goes `Degraded` and back
- [ ] Process poll p95 ≤ 15 ms and port poll p95 ≤ 25 ms over 100 ticks (printed by an `#[ignore]` timing test, recorded in `docs/PERFORMANCE.md`)
- [ ] Settings lists both collectors with their measured cost note and B3 blind spot text
- [ ] `docs/COLLECTORS.md` regenerates with no diff after a clean run (drift check in `bun run verify`)
- [ ] No remote IP appears anywhere in the DB after 10 minutes of browsing (SQL scan test over every TEXT column for a dotted-quad/IPv6 pattern, excluding loopback)

## Risk Assessment

- **B3 — diff polling misses processes shorter than the interval** — signal: a known short-lived process never appears. Response: this is a *documented capability limit*, not a bug: `coverage_limits` says "Processes that start and exit between two 2-second polls are not recorded. Push-based process tracing (`Win32_ProcessStartTrace`, ETW) requires administrator rights and is out of scope for the non-elevated MVP." The UI shows it; the analysis engine must never imply completeness (§54, §87). Optional user-visible mitigation: interval configurable down to 1 s with the measured CPU cost stated next to the setting.
- **`panic = "unwind"` change regresses binary size** — signal: release exe grows. Response: accepted and measured; `strip = "symbols"` + `lto = "thin"` retained; the number is recorded in `docs/PERFORMANCE.md`.
- **`sysinfo` 0.39 API differs from expectation** — signal: compile error on `RefreshKind`/`ProcessRefreshKind`. Response: read `~/.cargo/registry/src/*/sysinfo-0.39.6/src/lib.rs` for the exact builder names before writing; do not guess feature or method spellings.
- **`GetExtendedTcpTable` layout mistakes cause UB** — signal: nonsense ports or a crash. Response: use the `windows` crate's generated structs (`MIB_TCPTABLE_OWNER_PID`, `MIB_TCP6ROW_OWNER_PID`) rather than hand-declared ones, iterate via `std::slice::from_raw_parts` over the `table` field with `dwNumEntries`, and assert ports are 1..=65535 in a debug assertion; the ephemeral-listener test proves correctness.
- **Process/port event flood on a busy machine** — signal: thousands of events per minute. Response: the dedupe + cluster layers from phase 07 absorb bursts; additionally the process collector applies an importance floor so routine `svchost` churn scores low and is filtered out of default views (not dropped from storage).

## Security Considerations

Command lines, environment blocks and process memory are never read — the guard script fails the build
if `.cmd(`, `.environ(`, or `OpenProcess` with `VM_READ` appears under `collectors/`. Remote network
addresses are reduced to a scope enum before the `EventDraft` is built, so the raw IP never reaches
`sanitize`, let alone SQLite (§34, §33 "no spyware behavior"). All Win32 calls check their return codes
and translate `ERROR_ACCESS_DENIED` into an honest `Unavailable` status rather than silently reporting
partial data as complete.

### FILE: phase-10-files-apps-services-collectors.md

---
phase: 10
title: "File/Config, Installed Apps & Windows Services Collectors"
status: pending
priority: P1
effort: 12h
dependencies: [09]
---

## Overview

Three collectors on the phase-09 framework: scoped file/config watching with redacted structured diff
(§13), installed applications from Windows installation metadata with duplicate normalization (§17),
and Windows services via SCM (§16). All three are cheap per the measured matrix (uninstall registry
~50 ms, services 3.3 ms) so all three are diff-poll or notification driven with no elevation.

## Requirements

Functional: **no whole-filesystem watching, ever** — roots are only (a) user-selected folders, (b)
detected dev project folders (offered, never auto-added silently), (c) a curated list of config
locations the user enables individually; file events created/deleted/renamed/modified carry `path`,
`size`, `mtime`, and a content hash only when the file is ≤ 1 MB and the user enabled hashing —
**contents are never stored**; recognized text config extensions get a structured before/after diff
rendered exactly as §13's shape (`API_PORT 3000 → 4000`, `OPENAI_API_KEY [REDACTED] → [REDACTED]`);
apps emit installed/removed/version-changed with publisher, version, install location and install date;
services emit installed/removed/started/stopped/restarted/startup-type-changed with the §16 human line
("Windows Audio restarted.") and advanced detail (`Service: Audiosrv`, `State: RUNNING → STOPPED →
RUNNING`). Non-functional: a `node_modules`-heavy project must not flood — default ignore set plus
debouncing; services poll ≤ 10 ms p95; app scan ≤ 150 ms and runs at most every 15 min or on registry
notification.

## Architecture

File watching: `notify` 8.2.0 `RecommendedWatcher` (ReadDirectoryChangesW on Windows) wrapped in
`notify-debouncer-full` 0.7.0 with a 900 ms debounce, which also coalesces the rename
create+delete pair into `EventKind::Modify(ModifyKind::Name(RenameMode::Both))`. Ignore set applied
*before* debounce output is turned into events: `node_modules`, `.git`, `target`, `dist`, `build`,
`.next`, `.svelte-kit`, `__pycache__`, `.venv`, `vendor`, `*.log`, `*.tmp`, `*.swp`, `~$*`, and any
path segment starting with `.` deeper than the root. A per-root event budget (200 events / 10 s)
trips a `RootThrottled` state that emits one honest event — "Watching paused for D:\Project\x: more
than 200 changes in 10 seconds." — instead of thousands of rows.

Config diff needs a *previous* value set without storing file contents. `config_shadow(root_id, path,
key, value_redacted, is_secret, updated_at)` stores only already-sanitized key/value pairs produced by
the phase-02 `scrub_pairs`; secrets are stored as the literal `[REDACTED]` marker plus
`is_secret = 1`, so a secret's *presence and change* is knowable while its value never exists at rest.
Diff = shadow vs freshly parsed pairs; for a secret key the diff reports "changed" only because the
computed FNV-1a of the raw value differs from a stored `value_hash` (hash of value + per-install random
salt kept in `state.json`, so the DB alone cannot be brute-forced against a wordlist). Parsers are
minimal and hand-written per format (`.env`/`.ini`/`.properties` = line `k=v`; `.json` = `serde_json`
flatten to dotted paths; `.toml` = `toml` crate if a pinned version is verified at implementation time,
otherwise treated as opaque "modified" with no key diff; `.yaml`/`.yml` = line-structured diff, no YAML
parser dependency; `.xml`/`.conf` = opaque "modified"). Files > 2 MB or non-UTF-8 are opaque —
never parsed, never read into a diff.

Apps: enumerate `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`,
`HKLM\SOFTWARE\WOW6432Node\...\Uninstall`, `HKCU\...\Uninstall` (all readable non-admin per matrix).
Normalization: drop entries with `SystemComponent=1`, no `DisplayName`, or a `ParentKeyName`
(update rollups); dedupe by `(normalized_display_name, publisher)` keeping the entry with the richest
metadata; `normalized_display_name` = lowercased, trimmed, version-suffix stripped (trailing
`\d+(\.\d+)*` and a trailing parenthesised arch tag). Store `entities` rows keyed
`app:{normalized}:{publisher}` so version-change events attach to a stable entity across upgrades.

Services: `OpenSCManagerW(SC_MANAGER_ENUMERATE_SERVICE | SC_MANAGER_CONNECT)` +
`EnumServicesStatusExW(SC_ENUM_PROCESS_INFO, SERVICE_WIN32|SERVICE_DRIVER, SERVICE_STATE_ALL)` two-call
sizing; `QueryServiceConfigW` per service **only** for services whose state changed or on the 15-min
full sweep (start type + display name + binary path), because per-service config queries are the
expensive part. Push attempt: `NotifyServiceStatusChangeW(SERVICE_NOTIFY_RUNNING | STOPPED | START_PENDING
| STOP_PENDING | DELETE_PENDING)` on the SCM handle (assumption A4 — non-admin viability unverified);
if it returns `ERROR_ACCESS_DENIED` or never fires within a 60 s probe, fall back to the 5 s
`EnumServicesStatusExW` diff (3.3 ms measured, affordable) and record `mechanism: DiffPoll` in the
descriptor so the UI states what is actually happening.

## Related Code Files

Create: `src-tauri/src/collectors/{files.rs,config_diff.rs,apps.rs,services.rs}`,
`src-tauri/src/collectors/config/{env_ini.rs,json_flat.rs,line_diff.rs,mod.rs}`,
`src-tauri/src/platform/windows/{registry.rs,scm.rs,reg_notify.rs}`,
`src-tauri/src/core/watch_roots.rs`, `src-tauri/src/commands/watch_roots.rs`,
`src-tauri/migrations/0003_config_shadow.sql`,
`src-tauri/tests/{collector_files.rs,config_diff_redaction.rs,collector_apps.rs,collector_services.rs,watch_root_throttle.rs}`,
`src/features/settings/{WatchRootsPanel.svelte,ConfigFilesPanel.svelte}`,
`src/features/timeline/ConfigDiffView.svelte`.
Modify: `src-tauri/Cargo.toml` (`notify` 8.2.0 default-features off + `macos_fsevent`-free feature set,
`notify-debouncer-full` 0.7.0), `src-tauri/src/collectors/mod.rs`, `docs/COLLECTORS.md`,
`docs/PRIVACY.md`, `src/lib/copy/en.ts`.

## Implementation Steps

1. `watch_roots.rs` — `WatchRoot { id, path, kind: UserFolder | DevProject | ConfigLocation, enabled, hash_files, include_config_diff, max_depth }` persisted in the `watch_roots` table from phase 03. `add_watch_root` command takes a path **only** from `tauri-plugin-dialog`'s folder picker result echoed back with a one-shot token (the frontend cannot submit an arbitrary path string — see Security), validates existence + that it is not a system root (`C:\`, `C:\Windows`, `C:\Program Files*`, the app's own data dir) and not longer than 240 chars.
2. Dev-project detection: scan the *already-selected* roots (never the whole disk) up to `max_depth 3` for `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `pom.xml`, `build.gradle`, `.git`; surface them in `WatchRootsPanel` as suggestions with an explicit "Watch this project" action. Nothing is watched without a click (§33).
3. `files.rs` — one debouncer per app (multiple `watch()` calls, one event stream), `RecursiveMode::Recursive`, ignore set + budget throttle, then per event: `std::fs::metadata` for size/mtime, optional FNV-1a content hash when `hash_files && len <= 1 MB` (streamed in 64 KB chunks, hash only — contents dropped immediately), then `EventDraft`. Deleted-file events skip metadata gracefully.
4. `config_diff.rs` + `config/` — extension routing, size/encoding gate, parse → `Vec<(String,String)>` → `scrub_pairs` (phase 02, returns `sensitive`) → compare against `config_shadow` → build a `ConfigDiff { added, removed, changed: Vec<{key, old, new, is_secret}> }` capped at 40 entries with an "and N more" tail; write the new shadow in the same transaction as the event so a crash cannot desync (recovery: a missing shadow row makes the next diff "baseline, no event").
5. `registry.rs` — thin `RegOpenKeyExW`/`RegEnumKeyExW`/`RegQueryValueExW` wrappers returning `Result<Option<String>>` with `REG_SZ`/`REG_EXPAND_SZ`/`REG_DWORD` handling, and `reg_notify.rs` — `RegNotifyChangeKeyValue(hkey, TRUE, REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET, event, TRUE)` on an `Event` handle waited by a dedicated thread that signals a `tokio::sync::Notify`. Assumption A3 (does it fire for *subkey creation* under a watched key?) is settled by `collector_apps.rs`'s probe test which creates a throwaway `HKCU\Software\WhatChangedTest\probe` subkey and asserts the notification fires within 2 s; if it does not, the collector uses the 15-min sweep only and the descriptor says "Detected within 15 minutes".
6. `apps.rs` — enumeration + normalization + dedupe as designed; events `AppInstalled { name, publisher, version, install_location }`, `AppRemoved { name, publisher, last_version }`, `AppVersionChanged { name, publisher, old_version, new_version }` producing the §17 line "Discord updated 1.0.9186 → 1.0.9201". Registry-notification-triggered scans are debounced 5 s (an installer touches the key many times).
7. `services.rs` — SCM enumeration + change-scoped `QueryServiceConfigW`; restart detection = a `STOPPED → RUNNING` transition for the same service within 30 s collapsed into one `ServiceRestarted` event carrying the `RUNNING → STOPPED → RUNNING` state trail (§16's advanced detail); display names come from the config query and are cached so the human line reads "Windows Audio restarted." not "Audiosrv restarted."
8. `ConfigDiffView.svelte` — monospace two-column diff, old value struck through in `--text-muted`, new value in `--text-primary`, `[REDACTED]` rendered as a lock chip with the tooltip "This value is a secret. What Changed records that it changed, never what it is."
9. Tests: `config_diff_redaction.rs` — a fixture `.env` with `API_PORT=3000` + `OPENAI_API_KEY=sk-live-<fixture>`; change both; assert the event contains `API_PORT 3000 → 4000`, that the secret row is `[REDACTED] → [REDACTED]` with `is_secret`, and that a full-DB TEXT scan (reusing phase 03's helper) finds neither the old nor the new secret value; `collector_files.rs` — temp dir round trip for create/modify/rename/delete plus an ignored `node_modules/x.js` producing zero events; `watch_root_throttle.rs` — 500 rapid writes produce ≤ 1 throttle event and no more than the budget; `collector_apps.rs` — normalization table test (12 real-world messy DisplayName pairs) + the A3 probe; `collector_services.rs` — parse a captured `EnumServicesStatusExW` buffer fixture and assert diff transitions, plus a live read asserting a known always-present service (`Schedule`) appears with a display name.

## Todo

- [ ] Commit 1 — `feat(core): watch roots with picker-only paths and dev-project suggestions`
- [ ] Commit 2 — `feat(collectors): debounced scoped file watcher with ignore set and throttle`
- [ ] Commit 3 — `feat(collectors): redacted structured config diff with hashed secret shadow`
- [ ] Commit 4 — `feat(collectors): installed apps from uninstall registry with dedupe`
- [ ] Commit 5 — `feat(collectors): scm service collector with restart collapsing`
- [ ] Commit 6 — `feat(ui): watch roots settings and config diff view`
- [ ] Commit 7 — `test(collectors): file, config redaction, apps normalization and service diffs`

## Success Criteria

- [ ] Editing a watched `.env` produces exactly one event within ~1 s rendering exactly the §13 shape
- [ ] Secret values appear nowhere in the DB, WAL, or logs (asserted over every TEXT column + a log-file scan)
- [ ] `node_modules` churn produces zero events; 500 rapid writes produce ≤ 1 throttle event
- [ ] Installing then uninstalling a small MSI/EXE app produces one `AppInstalled` and one `AppRemoved`; an app update produces `AppVersionChanged` with both versions
- [ ] Duplicate registry entries for one app yield one entity (12-case normalization test green)
- [ ] `net stop`/`net start` of a user-controllable service (e.g. `Spooler` if permitted, else a state change observed from a normal restart) produces one `ServiceRestarted` with the state trail
- [ ] A3 and A4 probe results are recorded in `docs/COLLECTORS.md` as measured facts with the fallback that was actually used

## Verification Gate

```bash
cd src-tauri && cargo test --lib && cargo test --test collector_files --test config_diff_redaction --test collector_apps --test collector_services --test watch_root_throttle
cd .. && bun run check && bun run test && bun run guard:redaction
bun run tauri dev   # add a watch root via picker, edit a .env, watch the timeline row
```

## Risk Assessment

- **A3 fails: `RegNotifyChangeKeyValue` does not report new subkeys** — signal: probe test times out. Response: 15-min sweep becomes the only mechanism; descriptor and Settings text change to "Detected within 15 minutes"; no fabricated immediacy (§54).
- **A4 fails: `NotifyServiceStatusChangeW` needs elevation** — signal: `ERROR_ACCESS_DENIED`. Response: 5 s diff poll (measured 3.3 ms) is the documented mechanism; nothing else changes.
- **File watcher misses changes under heavy load (ReadDirectoryChangesW buffer overflow)** — signal: `notify` emits an error event. Response: on overflow, re-stat the root's tracked file set and emit one honest "Some changes in <root> may have been missed" event; never silently continue.
- **Config parse mis-detects a secret and prints it** — signal: redaction test red. Response: the value-shape detectors from phase 02 run on *every* value regardless of key name, so an unknown key holding `sk-…` is still caught; a new fixture is added for each miss found.
- **Path-based privacy leak (a filename itself is sensitive, e.g. `Resume-JohnDoe.pdf`)** — signal: user concern. Response: paths under watched roots are recorded because the user explicitly chose the root; `docs/PRIVACY.md` states this plainly, Settings shows the exact watched list, and removing a root offers "also delete recorded events for this folder" (implemented via `entity_key` prefix delete).
- **Apps scan on a machine with 300 installed apps exceeds budget** — signal: > 150 ms. Response: measured on this machine at ~50 ms for the full key set; if a slower machine appears, the sweep interval widens rather than the data being trimmed.

## Security Considerations

The frontend never supplies a filesystem path: `add_watch_root` accepts only a token issued by the
backend after a dialog-plugin folder pick, which closes the §35 "no arbitrary file read" hole that a
naive `add_watch_root(path: String)` would open. Watched-root validation rejects system directories, so
the app cannot be pointed at `C:\Windows` and turned into a system-wide file monitor. Config contents
are parsed in memory, sanitized, and dropped; only redacted pairs and a salted hash persist, so the
database is not a secret store even if copied (§33, §34). Service and registry reads are read-only —
no `SC_MANAGER_CREATE_SERVICE`, no `RegSetValueEx`, anywhere in the codebase (guard-script denylist).

### FILE: phase-11-windows-change-intelligence.md

---
phase: 11
title: "Windows Change Intelligence — Updates, Drivers, Network, Startup, Event Log, Snapshots"
status: pending
priority: P1
effort: 18h
dependencies: [10]
---

## Overview

The five expensive/high-value Windows sources plus periodic snapshots. Each is cost-shaped by the
measured matrix: cheap sources poll (WUA history 38 ms), expensive ones are notification-driven with a
rare sweep (drivers 2 128 ms, network config 1 441 ms, scheduled tasks 1 326 ms). This is the largest
phase and is split into five independently shippable collectors so a failure in one does not block the
others.

## Requirements

Functional — §18 updates: installed / failed / reboot-related, temporally linked ("Windows installed
KB5065426. Your computer restarted 4 minutes later."); §19 drivers: meaningful version changes with
human names ("NVIDIA graphics driver updated 581.29 → 582.16"), priority GPU/audio/network/storage/input,
**never a bare GUID**; §20 network: adapters, DNS servers, routes, proxy, default gateway, VPN adapter
appear/disappear, IP config where meaningful, and DHCP address rotation must **not** be scored critical;
§21 startup: new/removed/enabled/disabled startup apps + important scheduled tasks ("Adobe Creative
Cloud began launching automatically with Windows today."); §22 event log: only the relevant channels —
app crashes, service failures, driver failures, unexpected shutdown, Windows Update, network failures,
hardware errors — normalized, with raw event IDs under technical details only. Snapshots: periodic
`snapshots` rows for drivers/network/startup so a checkpoint comparison has a base state.
Non-functional: none of these may run on the UI thread or hold the DB writer; the drivers sweep must not
run more than once per 6 h unless a device-change notification fires; total idle CPU for this phase
< 0.5 %.

## Architecture

Cost-shaped mechanisms, each recorded in the descriptor:

| Collector | Mechanism | Measured cost | Notes |
|---|---|---|---|
| Windows Update | Poll 15 min + on `EvtSubscribe` WU event | 38 ms | WUA COM `QueryHistory` |
| Drivers | `CM_Register_Notification` device arrival/removal + 6 h sweep | 2 128 ms warm | SetupAPI enumeration |
| Network config | `NotifyIpInterfaceChange` + `NotifyRouteChange2` + 30 min sweep | 1 441 ms | never poll tight |
| Startup | Registry notify (Run keys) + Task Scheduler COM sweep 30 min | 1 326 ms tasks | tasks are the slow part |
| Event log | `EvtSubscribe` push, per channel | push | no polling at all |

**`Win32_QuickFixEngineering` is not used** — the matrix measured 4 rows from QFE versus 133 from WUA
`QueryHistory` for the same machine, so QFE would silently under-report by 97 %. WUA COM is the only
update source. **The `Security` channel is unreadable non-admin** (measured) and is therefore not in the
channel list; the descriptor says so.

All COM use goes through one `platform/windows/com.rs` helper: `CoInitializeEx(None,
COINIT_APARTMENTTHREADED)` per dedicated thread (never on a tokio worker), RAII guard calling
`CoUninitialize` on drop, and every COM call site inside a `catch_unwind` because a broken WUA agent is
a real-world failure. WUA and Task Scheduler run on their own long-lived threads communicating with the
async world by `mpsc`.

Snapshots: `snapshot_kind` ∈ `drivers | network | startup | services | apps`; each stores a compact,
already-sanitized JSON blob (`serde_json::to_vec`) plus a content fingerprint; a new row is written only
when the fingerprint differs from the previous one (so a stable machine stores ~1 row/kind/week, not
one per sweep), and §11's 7-day snapshot retention applies to *detailed* snapshots while the newest
snapshot per kind is always retained so checkpoint comparison never has an empty base.

## Related Code Files

Create: `src-tauri/src/collectors/{updates.rs,drivers.rs,network.rs,startup.rs,eventlog.rs}`,
`src-tauri/src/platform/windows/{com.rs,wua.rs,setupapi.rs,cm_notify.rs,iphelper.rs,evt.rs,taskscheduler.rs}`,
`src-tauri/src/core/snapshots.rs`, `src-tauri/src/collectors/device_names.rs`,
`src-tauri/tests/{collector_updates.rs,collector_drivers.rs,collector_network.rs,collector_startup.rs,collector_eventlog.rs,snapshots.rs,driver_naming.rs}`,
`src-tauri/tests/fixtures/{evtx_render_samples.json,driver_names.json}`.
Modify: `src-tauri/Cargo.toml` (`windows` 0.62.2 features: `Win32_System_Com`,
`Win32_System_UpdateAgent`, `Win32_Devices_DeviceAndDriverInstallation`,
`Win32_Devices_Properties`, `Win32_NetworkManagement_IpHelper`, `Win32_NetworkManagement_Ndis`,
`Win32_System_EventLog`, `Win32_System_TaskScheduler`, `Win32_System_Registry`,
`Win32_System_Services`, `Win32_Security`, `Win32_Foundation`, `Win32_System_Threading` — resolve the
exact feature names from `~/.cargo/registry/src/*/windows-0.62.2/Cargo.toml` before writing, do not
guess), `src-tauri/src/collectors/mod.rs`, `docs/COLLECTORS.md`, `src/lib/copy/en.ts`.

## Implementation Steps

1. `com.rs` — apartment guard + `ComThread::spawn(name, closure)` helper; a test asserts double-init and drop-order safety and that a panic inside the closure does not poison the process.
2. `wua.rs` — `CoCreateInstance` of `Microsoft.Update.Session` (CLSID resolved via `CLSIDFromProgID`, since the typed `windows` binding for `UpdateSession` may or may not exist in 0.62.2 — read the crate's `UpdateAgent` module first; if typed interfaces exist use them, else use `IDispatch` late binding through `Invoke`, which is verified-working per the matrix's PowerShell probe). Call `CreateUpdateSearcher` → `GetTotalHistoryCount` → `QueryHistory(0, n)` and map each `IUpdateHistoryEntry`: `Title`, `Date`, `ResultCode` (2 = Succeeded, 4 = Failed, 5 = Aborted), `ClientApplicationID`, `UnmappedResultCode`, and the KB number parsed from the title (`KB\d{6,7}`). Dedupe by `(title, date)`; only entries newer than the stored high-water mark emit events.
3. `updates.rs` — `UpdateInstalled { kb, title, result }`, `UpdateFailed { kb, title, code, code_meaning }`, and reboot linkage: read `GetTickCount64` at start plus the `System`/`Kernel-Boot` event-log records (event ID 12 startup / 13 shutdown, provider `Microsoft-Windows-Kernel-General`, and 6008 unexpected shutdown from `System`) to produce `SystemRestarted { boot_time, clean: bool }`; the *link* between update and restart is created by the phase-13 analysis engine, but the raw temporal facts (both timestamps) are stored here so §18's example line is derivable rather than hand-written.
4. `evt.rs` + `eventlog.rs` — `EvtSubscribe(None, None, channel, query, None, None, callback, EvtSubscribeToFutureEvents)` per channel with an XPath query filtering to the enumerated IDs, plus `EvtRender(EvtRenderEventXml)` and a **minimal hand-written XML field extractor** (no XML crate: pull `<EventID>`, `<Provider Name>`, `<TimeCreated SystemTime>`, `<Level>`, and `<Data Name="…">` values by tag scan — the format is machine-generated and stable, and a fixture file of 12 real rendered events guards the parser). Channels/IDs: `Application` (1000 app crash, 1001 WER, 1002 hang), `System` (7000/7001/7009/7011/7023/7031/7034 service failures, 6008 unexpected shutdown, 41 kernel power, 219 driver load failure, 1001 bugcheck), `Microsoft-Windows-WindowsUpdateClient/Operational` (19 install success, 20 install failure, 43 install started), `Microsoft-Windows-NetworkProfile/Operational` (10000/10001 network connect/disconnect), `Microsoft-Windows-Kernel-General` (12/13). Any channel that returns `ERROR_ACCESS_DENIED` (expected for `Security`) is reported as `Unavailable` with the reason shown in Settings, never retried in a loop.
5. `setupapi.rs` + `drivers.rs` + `device_names.rs` — `SetupDiGetClassDevsW(None, None, None, DIGCF_PRESENT | DIGCF_ALLCLASSES)` + `SetupDiEnumDeviceInfo` + `SetupDiGetDevicePropertyW` for `DEVPKEY_Device_DriverVersion`, `DEVPKEY_Device_DriverProvider`, `DEVPKEY_Device_DriverDate`, `DEVPKEY_Device_FriendlyName`, `DEVPKEY_Device_Class`, `DEVPKEY_Device_DeviceDesc`. `device_names.rs` builds the human line: prefer `FriendlyName`, else `DeviceDesc`, else `"{Provider} {Class} device"`; map class → category word (`Display` → "graphics", `MEDIA` → "audio", `Net` → "network", `SCSIAdapter`/`DiskDrive` → "storage", `HIDClass`/`Mouse`/`Keyboard` → "input"); a device with no usable name is **skipped, not emitted with a GUID** (§19). Priority classes are emitted at higher importance; other classes still recorded. `cm_notify.rs` — `CM_Register_Notification` with `CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE` (assumption A5) triggers a targeted re-read of only the arriving device when possible; the 6 h sweep is the guaranteed path.
6. `iphelper.rs` + `network.rs` — `GetAdaptersAddresses(AF_UNSPEC, GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_INCLUDE_ALL_INTERFACES)` for adapters/DNS/gateways, `GetIpForwardTable2` for routes, proxy from `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings` (`ProxyEnable`, `ProxyServer`, `AutoConfigURL`), VPN detection by adapter `IfType` (`IF_TYPE_PPP`, `IF_TYPE_TUNNEL`) or a description match against a small known-vendor list. Push: `NotifyIpInterfaceChange` + `NotifyRouteChange2` callbacks debounced 2 s. Emitted events: `DnsServersChanged { adapter, old: Vec<String>, new: Vec<String> }` (produces §20's "Your DNS server changed 1.1.1.1 → 192.168.1.1"), `DefaultGatewayChanged`, `ProxyChanged`, `AdapterAppeared/Disappeared`, `VpnStateChanged`, `RouteChanged` (default route only; the full table is snapshot-only to avoid noise), `IpAddressChanged` — the last one carries `dhcp_rotation: bool` (same subnet + same gateway + DHCP-enabled adapter ⇒ true) and rotation is scored `Info`, never `Critical` (§20, §54).
7. `taskscheduler.rs` + `startup.rs` — Run-key startup entries from `HKCU`/`HKLM` `...\Run` and `...\RunOnce` plus `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup` and the per-user `StartupApproved` keys for the enabled/disabled bit; scheduled tasks via `ITaskService::Connect(None×4)` → `GetFolder("\\")` → recursive `GetTasks`, keeping only tasks whose triggers include logon/boot (`ITaskDefinition::Triggers` types `TASK_TRIGGER_LOGON`/`BOOT`) — that filter is what makes "important scheduled startup tasks" bounded (§21). Sweep 30 min; registry notify makes Run-key changes near-immediate.
8. `snapshots.rs` — `write_if_changed(kind, value: &SanitizedJson)`; fingerprint = FNV-1a of the canonical JSON bytes; a `newest per kind is never deleted` rule enforced in the retention SQL and covered by a test.
9. Tests: `driver_naming.rs` — 20 real `(FriendlyName, DeviceDesc, Provider, Class, version_old, version_new)` fixture rows asserting the exact human sentences and that no output contains `{` or a GUID pattern; `collector_eventlog.rs` — the 12-event render fixture parsed into normalized events, plus a live subscription smoke test asserting the callback fires for a self-generated `Application` event (write one via `ReportEventW` with our own source, or if source registration needs elevation, assert subscription setup succeeds and document that live-fire is manually verified); `collector_network.rs` — DHCP-rotation classification table (8 cases) and a DNS-change diff from two captured adapter tables; `collector_updates.rs` — history mapping from a captured `QueryHistory` fixture, KB parsing (10 titles), and result-code meaning table; `collector_startup.rs` — Run-key + StartupApproved bit decoding fixture; `snapshots.rs` — unchanged sweep writes no row, changed writes one, newest survives retention.

## Todo

- [ ] Commit 1 — `feat(platform): COM apartment helper and dedicated com threads`
- [ ] Commit 2 — `feat(collectors): windows update history via WUA with reboot facts`
- [ ] Commit 3 — `feat(collectors): event log subscriptions for the relevant channels only`
- [ ] Commit 4 — `feat(collectors): driver version changes with human device naming`
- [ ] Commit 5 — `feat(collectors): network configuration with dhcp-rotation classification`
- [ ] Commit 6 — `feat(collectors): startup entries and logon/boot scheduled tasks`
- [ ] Commit 7 — `feat(core): fingerprinted snapshots with newest-always-retained rule`
- [ ] Commit 8 — `test(collectors): naming, parsing, classification and snapshot fixtures`

## Success Criteria

- [ ] WUA history returns ≥ 100 entries on this machine (matrix measured 133) and the newest maps to a correct `UpdateInstalled` event with its KB
- [ ] Driver events never contain a GUID or a `{`; 20/20 naming fixtures produce the §19 sentence shape
- [ ] Changing the DNS server on an adapter (non-admin per-adapter change or a VPN connect) produces one `DnsServersChanged` with both lists inside 5 s
- [ ] A DHCP lease renewal produces an `Info` event, never `Critical` (classification test + live observation)
- [ ] `Security` channel is reported `Unavailable — administrator rights required`, and no retry loop appears in the log
- [ ] Drivers sweep runs ≤ 1× per 6 h in a 12 h soak; measured wall time recorded (~2.1 s, off the UI path)
- [ ] Snapshot table growth over a 24 h soak ≤ 20 rows total
- [ ] 12 h soak: no COM leak (handle count stable ±50), memory stable, zero collector restarts

## Verification Gate

```bash
cd src-tauri && cargo test --lib && cargo test --test collector_updates --test collector_drivers --test collector_network --test collector_startup --test collector_eventlog --test snapshots --test driver_naming
cd .. && bun run check && bun run test && bun run guard:redaction && bun run guard:types
bun run tauri dev   # observe: connect a VPN, toggle a startup app, check timeline rows
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1
```

## Risk Assessment

- **Typed WUA bindings absent in `windows` 0.62.2** — signal: unresolved `IUpdateSession`. Response: step 2's `IDispatch` late-binding path (ProgID + `Invoke`), which is exactly what the verified PowerShell probe does under the hood; slower but proven. Decide by reading the crate, not by guessing.
- **A5 fails: `CM_Register_Notification` does not report driver *version* changes (only arrival/removal)** — signal: a driver update produces no notification. Response: this is the expected case — the 6 h sweep is the guaranteed mechanism and the descriptor says "Driver updates are detected within 6 hours"; a user-triggered "Check now" button in Settings runs an immediate sweep.
- **`EvtSubscribe` callback runs on an OS thread pool thread and must not block** — signal: dropped events or deadlock. Response: the callback does nothing but `EvtRender` into a `String` and `try_send` onto a bounded channel; all parsing happens on our own thread. A full channel increments a drop counter (surfaced in Diagnostics), never blocks.
- **Task Scheduler enumeration is 1.3 s and touches many tasks** — signal: sweep visible in CPU graph. Response: 30 min interval, logon/boot trigger filter, and the sweep runs on a `spawn_blocking` thread; measured cost is printed in Diagnostics so the user can widen the interval.
- **Event-log message text may embed user data (file paths, URLs)** — signal: redaction test on event-log fixtures. Response: all extracted `<Data>` values pass through `scrub_text` before the `EventDraft` is built, and the fixture set includes a crash record containing a token-shaped string.
- **This phase is 18 h — too big to review in one sitting** — signal: reviewer fatigue. Response: eight commits, each independently testable; the five collectors have disjoint files so they can be implemented and reviewed in five sittings without merge conflicts.

## Security Considerations

Everything here is read-only Windows introspection: no `Security` channel, no ETW session, no elevation
attempt, no service creation, no WMI write. The plan explicitly rejects the elevated push-tracing path
(`Win32_ProcessStartTrace`) rather than asking the user for admin (§36). All extracted strings — event
messages, task action paths, adapter descriptions, driver provider names — pass through
`redact::sanitize`, so a proxy URL containing `user:password@host` is stored with the userinfo stripped
(§34). DNS servers and gateways are configuration values the user owns, not browsing data; `docs/PRIVACY.md`
states what network data is and is not recorded (no destinations, no traffic, no DNS queries).

### FILE: phase-12-checkpoints-comparison.md

---
phase: 12
title: "Checkpoint System & Comparison View"
status: pending
priority: P1
effort: 7h
dependencies: [11]
---

## Overview

The user's manual "known good" marker (§25) and the screen that uses it (§47). A checkpoint captures
timestamp, label, a summary snapshot, and important entity states; comparison answers "what changed
since it worked". This also retires the tray stub from phase 06.

## Requirements

Functional: "Create checkpoint" from tray, Home, timeline drawer ("create checkpoint here" uses the
event's timestamp) and the Checkpoints screen; label input with the §25 suggestions offered as
one-tap chips ("Everything is working", "Before installing driver", "Before changing server config",
"Before Windows update"); cards render "{label} — Aug 29, 1:20 AM" with actions Compare / Rename /
Delete; comparison headline is exactly "12 important changes since this checkpoint" (count
substituted); comparison groups changes by category, lists entity-state deltas (app versions, service
start types, driver versions, DNS servers, startup entries, listening ports), and links each row to its
event. Non-functional: creating a checkpoint must complete in < 400 ms perceived — the summary snapshot
is captured from the *latest existing* per-kind snapshots plus a cheap live read (services 3.3 ms,
processes 6.2 ms, apps from cache), never by triggering a 2 s driver sweep; comparison over a 30-day
window with 50 k events < 300 ms.

## Architecture

`checkpoints(id, created_at, label, note, kind: Manual | Auto, summary_json, deleted_at)` +
`checkpoint_entity_states(checkpoint_id, entity_key, state_json)` (both from the phase-03 schema).
Creation path: `create_checkpoint(label, at: Option<i64>)` → gather = latest `snapshots` row per kind
(reused, not recomputed) + live cheap reads + counters → `redact::sanitize` on every string that came
from the system → one transaction writing the checkpoint, its entity states, and a
`CheckpointCreated` event so the checkpoint itself appears on the timeline.

Comparison is a pure function over data already in SQLite —
`compare_checkpoint(id) -> CheckpointComparison { important_count, total_count, by_category:
Vec<CategoryDelta>, entity_deltas: Vec<EntityDelta>, first_event_at, last_event_at }`. `EntityDelta`
comes from joining `checkpoint_entity_states` against the current `entities.state_json`, so a change
that happened while monitoring was paused still shows up as a *state* difference even though no event
exists — and it is labelled "State differs, but no recorded event explains it" rather than being
attributed to a random event (§54, §87). That distinction is the honest core of this screen.

Deletion is soft (`deleted_at`) for 7 days so an accidental delete is recoverable, then hard-deleted by
the retention task; `checkpoint_events` references are what keep referenced events exempt from
retention (phase 07 step 4).

## Related Code Files

Create: `src-tauri/src/core/checkpoints.rs`, `src-tauri/src/core/compare.rs`,
`src-tauri/src/commands/checkpoints.rs`,
`src-tauri/tests/{checkpoints.rs,compare_checkpoint.rs}`,
`src/features/checkpoints/{CheckpointsScreen.svelte,CheckpointCard.svelte,CreateCheckpointDialog.svelte,ComparisonView.svelte,EntityDeltaTable.svelte,CategoryDeltaList.svelte}`,
`src/lib/api/checkpoints.ts`.
Modify: `src-tauri/src/app/tray.rs` (enable "Create checkpoint"),
`src-tauri/src/core/retention.rs` (hard-delete soft-deleted checkpoints > 7 d),
`src/features/timeline/EventDrawer.svelte` (wire "Create checkpoint here"),
`src/features/home/HomeScreen.svelte`, `src/lib/copy/en.ts`, `docs/PRODUCT.md`.

## Implementation Steps

1. `checkpoints.rs` — `create(label, at, source)` with label validation via `core/validate.rs` (1..=80 chars, trimmed, control chars stripped); `rename(id, label)`, `soft_delete(id)`, `restore(id)`, `list(include_deleted: false)`. `summary_json` shape: `{ counts: {events, important, critical}, apps: u32, services_running: u32, drivers: u32, listening_ports: Vec<u16>, dns: Vec<String>, os_build: String, uptime_ms: u64 }` — small, bounded, and every string sanitized.
2. Entity states captured: apps (name + version), services (name + state + start type), drivers (device + version) from the latest snapshot, startup entries, listening ports with owner, DNS per adapter. Cap at 4 000 entity rows per checkpoint with an overflow marker so a pathological machine cannot create a 50 MB checkpoint.
3. `compare.rs` — event side: one indexed query `WHERE timestamp > ? AND timestamp <= ?` grouped by category and severity; entity side: full outer join in Rust over two `BTreeMap`s (checkpoint states vs current states) producing `Added | Removed | Changed { old, new } | Unexplained`; `important_count` = events with `importance >= 60` (the phase-13 threshold constant, imported not duplicated).
4. `commands/checkpoints.rs` — `create_checkpoint`, `list_checkpoints`, `rename_checkpoint`, `delete_checkpoint`, `restore_checkpoint`, `compare_checkpoint`; all added to the specta builder; bindings regenerated.
5. `CheckpointsScreen.svelte` — card grid (2–3 columns responsive), each card: label, formatted date/time exactly as §47 ("Aug 29, 1:20 AM" via a shared `formatCheckpointDate`), a one-line summary ("37 apps · 84 services running · 3 ports listening"), and Compare / Rename / Delete. Empty state: "No checkpoints yet." + "Create one when everything is working, so you have something to compare against." + a Create button.
6. `CreateCheckpointDialog.svelte` — label field pre-filled with "Everything is working", the four §25 suggestions as chips, optional note, Cmd/Ctrl+Enter to confirm; shows the captured summary after creation as confirmation ("Captured 37 apps, 84 services, 12 drivers.").
7. `ComparisonView.svelte` — headline "{n} important changes since this checkpoint" (with the §54-safe zero case "No important changes since this checkpoint."), then `CategoryDeltaList` (category, count, top three summaries, expand → filtered timeline), then `EntityDeltaTable` (three columns: what, then, now) with `Unexplained` rows visually separated under the sub-heading "Different now, but no recorded change explains it".
8. Tray: "Create checkpoint" becomes enabled and opens the window on the Checkpoints screen with the dialog pre-opened; the tracked stub entry in `docs/ROADMAP.md` is removed in the same commit.
9. Tests: `checkpoints.rs` — create/rename/soft-delete/restore round trip, label validation rejects 0 and 81 chars, `CheckpointCreated` event appears, referenced events survive a retention run that would otherwise delete them; `compare_checkpoint.rs` — a seeded scenario (checkpoint, then 12 important + 40 routine events, one app version change, one service start-type change, one entity changed with events suppressed) asserts `important_count == 12`, the exact headline string, the three entity delta kinds, and one `Unexplained` row.

## Todo

- [ ] Commit 1 — `feat(core): checkpoint creation from reused snapshots with entity states`
- [ ] Commit 2 — `feat(core): checkpoint comparison with event and entity deltas`
- [ ] Commit 3 — `feat(ui): checkpoint cards, create dialog and comparison view`
- [ ] Commit 4 — `feat(app): enable tray checkpoint action and drawer entry point`
- [ ] Commit 5 — `test(core): checkpoint lifecycle, retention exemption and comparison scenario`

## Success Criteria

- [ ] Creating a checkpoint completes in < 400 ms measured (timing logged) and triggers no expensive sweep
- [ ] Card renders exactly the §47 format; Compare / Rename / Delete all work
- [ ] Comparison headline matches §47's wording with the correct count; zero case reads calmly
- [ ] An entity that differs with no explaining event is labelled `Unexplained`, never attributed to an unrelated event
- [ ] Deleted checkpoint is restorable for 7 days, then hard-deleted by retention (test with a backdated `deleted_at`)
- [ ] Events referenced by a checkpoint survive a retention run past their age (asserted)
- [ ] Comparison over 50 k events completes < 300 ms (measured, recorded)

## Verification Gate

```bash
cd src-tauri && cargo test --lib && cargo test --test checkpoints --test compare_checkpoint --test retention
cd .. && bun run check && bun run test && bun run guard:types
bun run tauri dev   # tray -> Create checkpoint -> install something -> Compare
```

## Risk Assessment

- **Checkpoint capture becomes slow as collectors grow** — signal: > 400 ms. Response: capture *reuses* the newest snapshot rows rather than sweeping; the only live reads are the two cheapest collectors. A timing assertion in the test guards against a future collector being added to the capture path.
- **Comparison attributes an unexplained state difference to a nearby event** — signal: a reviewer sees a wrong cause. Response: `Unexplained` is a distinct variant rendered in its own section; the analysis engine (phase 13) is forbidden from consuming `Unexplained` rows as evidence.
- **Storage growth from many checkpoints** — signal: DB size jumps. Response: 4 000-row cap per checkpoint, entity states stored as one JSON blob per entity key rather than a row per field, and Settings shows checkpoint storage separately in the §11 usage estimate.
- **Soft delete confuses the user ("I deleted it, why is it counted?")** — signal: usability. Response: deleted checkpoints vanish from the list immediately; the only surface is an undo toast ("Checkpoint deleted. Undo") plus a documented 7-day grace in `docs/PRIVACY.md`; "Delete all history" purges them immediately.

## Security Considerations

Checkpoint labels and notes are user-authored free text: they are length-validated, control-character
stripped, stored as bound parameters, and rendered as text nodes (no `{@html}`), so a label like
`<img onerror=…>` is inert. Every system-derived string in `summary_json` and `state_json` passes
through `redact::sanitize` before storage, so a checkpoint is not a redaction bypass — the phase-03
`redaction_persistence` test is extended to cover both new tables.

### FILE: phase-13-analysis-engine.md

---
phase: 13
title: "Deterministic Analysis Engine — Scoring, Chains, Explanations"
status: pending
priority: P1
effort: 14h
dependencies: [12]
---

## Overview

The product's intellectual core (§27): given a problem description, a time window and affected
entities, run the eight prescribed stages and return ranked candidate causes with transparent
per-factor scoring (§28), causal chains with honest confidence tiers (§29), and human explanations from
a `DeterministicExplanationProvider` behind the §32 `ExplanationProvider` abstraction. No LLM, no
network. Backend only — the screen is phase 14.

## Requirements

Functional: implement all eight §27 stages in order; score = the exact §28 sum
`temporal + relationship + semantic_category + severity + rarity + failure_proximity`, with every term
retained individually so the UI can print the §28 "Why this is ranked #1" list; relationship traversal
uses the §30 edge types; chains distinguish **Confirmed / Likely / TemporalCorrelation** and never claim
certainty (§29); explanation text may only use the §29 allowed phrasings and must never contain a
forbidden certainty claim; the §59 fixture must rank the config→container→port chain above an unrelated
Chrome update. Non-functional: investigation over a 30-day window with 100 k events returns in < 500 ms
(measured); the engine is a pure function of DB state (same input ⇒ identical output, asserted by
running it twice and comparing).

## Architecture

Stage pipeline in `analysis/mod.rs::investigate(request) -> Investigation`:

```
1 timeframe    parse::extract_time(text) -> TimeWindow { start, end, confidence, source }
2 entities     parse::extract_entities(text) + entity name/alias match -> Vec<EntityRef>
3 traverse     graph::neighbourhood(entities, max_depth 2) -> Vec<(entity, distance, edge_kind)>
4 candidates   repo::candidate_events(window ± padding, entity_keys, categories) -> Vec<Event>
5 score        score::relevance(event, ctx) -> ScoreBreakdown { six named terms, total }
6 group        chains::build(scored) -> Vec<Chain>  (Confirmed | Likely | TemporalCorrelation)
7 rank         rank::order(chains, scored) -> Vec<Candidate>  (stable tie-break: score, then recency, then id)
8 explain      ExplanationProvider::explain(&Investigation) -> Explanation
```

Scoring terms (all `f32`, weights are named constants in `score/weights.rs`, not magic numbers, and
each has a one-line comment stating the reasoning):

- `temporal_score` — 0 outside the window; inside, `W_TEMPORAL * exp(-Δt_minutes / TAU)` with `TAU = 45.0`, so 2 min before the failure ≫ 6 h before. Events *after* the reported failure time get `W_TEMPORAL * 0.25 * decay` (they can be consequences, so they stay visible but rank lower).
- `relationship_score` — `W_REL / (1 + distance)` where distance 0 = the named entity itself, 1 = direct §30 edge, 2 = two hops; `Confirmed` edge kinds (`service → process`, `process → listens_on → port`, `config_file → configures → application`) get a 1.5× multiplier over inferred ones.
- `semantic_category_score` — a static keyword→category relevance table: "microphone/audio/sound" → `{Drivers: 1.0, Services: 0.8, Permissions: 1.0, Apps: 0.6}`, "internet/slow/wifi/dns" → `{Network: 1.0, Drivers: 0.7}`, "won't launch/crash" → `{Apps: 1.0, Files: 0.7, Updates: 0.6}`, "docker/container" → `{Docker: 1.0, Files: 0.8, Ports: 0.9}`, "server/port/connect" → `{Ports: 1.0, Services: 0.8, Network: 0.7}`. Unmatched text scores 0 for this term — never a random guess.
- `severity_score` — `Critical 1.0 / Warning 0.6 / Info 0.25` × `W_SEV`.
- `rarity_score` — `W_RARE * (1 - min(1, occurrences_30d / RARITY_SATURATION))` using one grouped count query per fingerprint prefix; a first-ever event scores full, `svchost` churn scores ~0.
- `failure_proximity_score` — `W_PROX` when the event falls in the tightest band before the reported time (≤ 10 min), scaled down over 60 min; when a §26 health signal exists later this term reads from it (the interface is present, the health source is not built — §26 is the user's own deferral).

Chain building: sort scored events by time; link `a → b` when a rule matches. Rule table with tiers —
`Confirmed`: same entity + known causal edge + b within 120 s of a (e.g. config file written → its
watched project's container restarted; service stopped → its owned port closed, where ownership is
recorded in the graph). `Likely`: related entities within 300 s, or an update event followed by a
restart within 30 min (§18's example). `TemporalCorrelation`: same 300 s band, no edge — this tier is
what carries the §29 "may be connected" language. A chain's confidence is its **weakest** link, which
prevents one confirmed hop from laundering a coincidence into a claim.

`ExplanationProvider` trait: `fn explain(&self, inv: &Investigation) -> Result<Explanation>` plus
`fn id(&self) -> &'static str`. `DeterministicExplanationProvider` composes sentences from templates
keyed by `(top_candidate.category, chain.confidence)`; every template is registered in
`explain/templates.rs` and covered by the language guard test. A `CloudExplanationProvider` is **not
implemented** (§75 deferral) but the trait, the settings flag (`ai_enabled: false`), and the
fact-only context builder (`explain::context::to_facts(&Investigation) -> Vec<FactLine>`) exist so the
later provider can only ever see deterministically retrieved facts (§32).

## Related Code Files

Create: `src-tauri/src/analysis/{mod.rs,parse.rs,graph.rs,candidates.rs,rank.rs,chains.rs}`,
`src-tauri/src/analysis/score/{mod.rs,weights.rs,temporal.rs,relationship.rs,category.rs,rarity.rs}`,
`src-tauri/src/analysis/explain/{mod.rs,provider.rs,deterministic.rs,templates.rs,context.rs,language_guard.rs}`,
`src-tauri/src/core/graph_writer.rs`, `src-tauri/src/commands/analysis.rs`,
`src-tauri/tests/{analysis_fixture_port_chain.rs,analysis_parse.rs,analysis_scoring.rs,analysis_chains.rs,analysis_language.rs,analysis_determinism.rs,analysis_perf.rs}`,
`src-tauri/tests/fixtures/scenarios/{port_chain.json,discord_microphone.json,slow_internet.json,minecraft_server.json,app_wont_launch.json,monitor_flicker.json,docker_something.json}`.
Modify: `src-tauri/src/collectors/*` (emit graph edges alongside events via `graph_writer`),
`src-tauri/src/storage/repository.rs` (candidate query + rarity counts + entity alias lookup),
`docs/ARCHITECTURE.md` (the scoring section, with the weight table), `src/lib/copy/en.ts`.

## Implementation Steps

1. `graph_writer.rs` + collector edits — every collector that already knows a relationship writes it: process→executable, process→parent_process, process→listens_on→port, service→process, application→executable, project→contains→config_file, config_file→configures→application (inferred from the project root + a known filename map: `docker-compose.yml`→docker, `.env`→the project's app, `nginx.conf`→nginx), network_adapter→uses→DNS. Edges are upserted with `first_seen`/`last_seen`; a `confirmed: bool` column marks directly observed edges versus inferred ones. Docker edges land in phase 16 through the same writer.
2. `parse.rs` — time extraction covering the §31 phrasings: "this morning" (local 05:00–12:00 today), "today", "yesterday", "last night" (yesterday 20:00 → today 04:00), "after I restarted" (anchor to the most recent `SystemRestarted` event — a data-driven anchor, not a clock guess), "N minutes/hours/days ago", "since <weekday>", explicit "HH:MM"; no match ⇒ default window = last 24 h with `confidence: Low` and the UI says "Looking at the last 24 hours." Entity extraction: tokenize, strip stopwords, match against `entities.display_name` and an alias table (`discord`→Discord.exe, `minecraft`→java.exe + port 25565, `docker`→docker categories) with case-insensitive contains + a simple bounded edit-distance (≤ 2, hand-written Levenshtein, no crate) for typos.
3. `candidates.rs` — one query with the window padded (start − 2 h, end + 30 min) and an entity-key `IN` clause built from the traversal result, plus category filtering from the semantic table; hard cap 5 000 candidates (widest window first, then trimmed by score) so a pathological range cannot stall the engine.
4. `score/` — the six terms with the constants above; `ScoreBreakdown` keeps each term plus a `Vec<ScoreReason>` of human strings ("occurred 2 minutes before failure", "directly affected Discord", "changed microphone permission", "no similar changes occurred earlier today") whose wording matches §28's example verbatim in shape; the UI prints these, so the reasons are produced by the scorer, never written in the frontend.
5. `chains.rs` — the tier rule table, weakest-link confidence, chain de-duplication (a chain that is a subsequence of a longer one is dropped), max chain length 6, max chains 5.
6. `rank.rs` — order chains by (max member score + 0.5 × chain length bonus for `Confirmed`/`Likely` tiers), then loose events; `#1 Strong match` label applies only when the top candidate's total ≥ `STRONG_MATCH_THRESHOLD` (0.75 of the theoretical max) **and** its chain confidence is `Confirmed` or `Likely`; otherwise the label is "Possible match" — this is what stops the UI from shouting "Strong match" at a coincidence (§54).
7. `explain/` — templates producing the §45 line shapes ("We found 3 changes that may be related.", "Why this matters: Discord depends on microphone permission.", "Timing: 8 minutes before the reported problem."); `language_guard.rs` holds the forbidden-phrase list ("definitely caused", "this caused", "the cause is", "guaranteed", "certainly", "proves") and the allowed-phrase list from §29; it is called in a debug assertion on every generated explanation **and** as a test over all templates × all confidence tiers.
8. `commands/analysis.rs` — `investigate(InvestigateRequest { text, window_override, entity_hint }) -> Investigation`, `get_investigation(id)`, `list_investigations(limit)`; investigations persist to the `investigations` table (request text sanitized first — user free text goes through `scrub_text` because a user may paste a connection string into it).
9. Fixtures + tests: `port_chain.json` is the mandatory §59 scenario (config port 3000→4000, container restart, port 3000 closed, port 4000 opened, HTTP health failure, plus an unrelated Chrome update) asserting the chain ranks #1 and Chrome does not appear in the top 3; the other six fixtures are the §31 user phrasings, each asserting a sensible top-3 without requiring a single "correct" answer (assertions are on *category presence* and on the unrelated-noise event being excluded); `analysis_determinism.rs` runs `investigate` twice over a frozen DB and asserts byte-identical JSON; `analysis_perf.rs` (`#[ignore]`) asserts < 500 ms over 100 k events.

## Todo

- [ ] Commit 1 — `feat(core): entity relationship graph writes from existing collectors`
- [ ] Commit 2 — `feat(analysis): deterministic time and entity parsing for problem text`
- [ ] Commit 3 — `feat(analysis): six-factor transparent relevance scoring with reasons`
- [ ] Commit 4 — `feat(analysis): causal chain builder with weakest-link confidence`
- [ ] Commit 5 — `feat(analysis): ranking, investigation persistence and commands`
- [ ] Commit 6 — `feat(analysis): deterministic explanation provider with language guard`
- [ ] Commit 7 — `test(analysis): mandatory port-chain fixture, six scenarios, determinism, perf`

## Success Criteria

- [ ] §59 fixture: the config→container→port chain ranks #1; the Chrome update is absent from the top 3
- [ ] All six §31 phrasings produce a non-empty, sensibly categorized top 3 with a correct time window
- [ ] Every candidate carries all six score terms; the UI-facing reason list matches §28's shape
- [ ] Language guard: 0 forbidden phrases across every template × tier combination; every explanation contains one of the §29 allowed phrasings
- [ ] "Strong match" appears only above the threshold with a Confirmed/Likely chain (negative test with a pure coincidence asserts "Possible match")
- [ ] Same DB, two runs, identical output bytes
- [ ] 100 k events, 30-day window: < 500 ms measured (recorded in `docs/PERFORMANCE.md`)
- [ ] No network call exists in the analysis path (grep guard for `reqwest`/`http` under `analysis/`)

## Verification Gate

```bash
cd src-tauri && cargo test --lib && cargo test --test analysis_fixture_port_chain --test analysis_parse --test analysis_scoring --test analysis_chains --test analysis_language --test analysis_determinism
cargo test --test analysis_perf -- --ignored --nocapture
cd .. && bun run check && bun run guard:types
```

## Risk Assessment

- **Scoring "feels wrong" on real machine data** — signal: the top candidate is obviously irrelevant during manual use. Response: weights live in one file with a documented rationale; tuning is a data-driven loop using saved real investigations as new fixtures. Never tune by adding a special case for one scenario — a new fixture must pass alongside all existing ones.
- **Time parsing over-claims precision** — signal: "this morning" resolves to an exact minute. Response: `TimeWindow` carries `confidence` and the UI states the window it used; low confidence widens rather than guesses (§54).
- **Chain builder invents causality** — signal: a `TemporalCorrelation` rendered with causal wording. Response: confidence tier is part of the template key, so the wording cannot be selected independently of the tier; the language guard covers every combination.
- **Rarity query is expensive per candidate** — signal: investigation > 500 ms. Response: one grouped `SELECT fingerprint, COUNT(*) … GROUP BY fingerprint` over the 30-day window loaded into a `HashMap` before scoring, not a query per event.
- **Entity matching misses because names differ ("Discord" vs "Discord.exe" vs "Discord Inc.")** — signal: an investigation finds nothing. Response: alias table + normalized-name matching + edit distance ≤ 2; when nothing matches, the engine says so honestly ("I could not match \"<text>\" to anything I monitor.") and falls back to a time-window-only ranking rather than returning an empty screen.
- **§26 health signal is absent, so `failure_proximity` is weaker than designed** — signal: term always uses the reported time. Response: accepted and documented; the term reads from an `Option<HealthTransition>` that is `None` in the MVP, so adding health checks later changes no scoring code.

## Security Considerations

The user's problem text is untrusted free input: it is length-capped (1..=500 via `core/validate.rs`),
passed through `scrub_text` before persistence (users paste connection strings into support boxes),
used only as parsed tokens against bound query parameters, and rendered as a text node. The
`ExplanationProvider` abstraction ships with exactly one local implementation and `ai_enabled: false`;
no HTTP client is linked into the analysis path, so an "accidental cloud call" is not merely disabled
but absent (§32, §63). `explain::context::to_facts` is the only future egress shape and is restricted to
already-persisted, already-redacted fields — file contents can never enter it (§32 "never upload file
contents automatically").

### FILE: phase-14-investigation-experience.md

---
phase: 14
title: "Investigation Experience — Home Input, Results, Evidence"
status: pending
priority: P1
effort: 9h
dependencies: [13]
---

## Overview

The user-facing half of the analysis engine: the §31 natural-language input on Home, the §45
Investigation screen, the §28 "Why this is ranked #1" transparency panel, the "What happened" chain
strip, and the §73/§74 tone rendered from real engine output. This is the moment the product either
feels intelligent or feels like a log viewer.

## Requirements

Functional: prominent Home input with placeholder "Tell me what stopped working…" (§31) — note the §72
Home sample uses "Example: Discord stopped detecting my microphone this morning" as the *example* line,
so both strings appear: the input placeholder is §31's, the example hint sits beneath it; submitting
opens the Investigation screen; the screen shows the echoed problem line, the count line "We found 3
changes that may be related.", ranked candidates with "#1 Strong match" (or "Possible match"), "Why this
matters", "Timing: N minutes before the reported problem", `[ View evidence ]`, the "What happened"
vertical chain with times and the "~10:50 issue observed" anchor, a "Show technical details" affordance,
and a Confidence line; §73's "Why it stands out" bullet list and the closing caveat sentence ("This does
not prove the driver caused the crash, but it is the strongest recent change we found.") are produced
from engine data; §74's development shape (Evidence block + "Possible next step") applies when the top
chain is a config/port chain, and **no config is ever modified** (§74). Non-functional: results render
within 100 ms of the command returning; the screen is usable with the keyboard alone; zero-result and
low-confidence states are calm, not alarming (§54).

## Architecture

One-way data flow: `InvestigationScreen` receives an `Investigation` DTO from `investigate` and renders
it; **no scoring, ranking, thresholding or wording decision happens in TypeScript**. The frontend owns
layout and progressive disclosure only. `Candidate.reasons: string[]` (from phase 13's `ScoreReason`),
`Candidate.match_label: 'strong' | 'possible'`, `Chain.confidence`, `Explanation.headline`,
`Explanation.body_lines`, `Explanation.caveat`, `Explanation.next_step: Option<String>` — every visible
sentence comes from Rust, which is what keeps the §29 language guard authoritative for what users
actually read.

Progressive disclosure ladder (§51 Simple/Advanced): level 1 = headline + why-it-stands-out bullets +
confidence; level 2 = `[ View evidence ]` expands the evidence rows and the chain strip; level 3 =
"Show technical details" reveals the score breakdown table (each of the six §28 terms with its value and
reason), raw event IDs, entity keys and the exact time window used. Level 3 is available to everyone
(§28 "scoring must not be fully hidden") but collapsed by default; Advanced mode in Settings makes
level 2 expanded by default.

`ChainStrip.svelte` renders the §45/§74 vertical sequence with a 1 px connector and a `↓` glyph, times
in tabular figures, the reported-problem anchor rendered as `~10:50 issue observed` in
`--text-secondary` with a dashed connector so it reads as *reported*, not *recorded* — a small honesty
detail that matters (§54).

## Related Code Files

Create: `src/features/investigation/{InvestigationScreen.svelte,ProblemInput.svelte,CandidateCard.svelte,WhyPanel.svelte,ScoreBreakdownTable.svelte,ChainStrip.svelte,EvidenceList.svelte,ConfidenceBadge.svelte,NextStepNote.svelte,InvestigationHistory.svelte,ZeroResults.svelte}`,
`src/lib/api/analysis.ts`, `src/lib/stores/investigation.svelte.ts`,
`src/features/investigation/__tests__/{investigation-render.test.ts,tone.test.ts,disclosure.test.ts,zero-results.test.ts}`,
`src/features/home/__tests__/home-input.test.ts`.
Modify: `src/features/home/HomeScreen.svelte` (wire the input + "Investigate" button + recent
investigations), `src/features/timeline/EventDrawer.svelte` ("start investigation from here" pre-fills
the window and entity), `src/lib/copy/en.ts`, `docs/PRODUCT.md` (screenshots of the three disclosure
levels).

## Implementation Steps

1. `ProblemInput.svelte` — single-line-growing textarea (max 3 rows), placeholder "Tell me what stopped working…", helper line "Example: Discord stopped detecting my microphone this morning", `Enter` submits / `Shift+Enter` newline, 500-char counter appearing after 400, disabled state while a request is in flight with the button label "Investigating…"; on Home it is the visually dominant element (§43 emotional core).
2. `investigation.svelte.ts` — `run(text)` calls `investigate`, stores the result, pushes to history, routes to the Investigation screen; `restore(id)` loads a past investigation; errors map to the typed `AppError` copy from phase 04 and render in `StateShell`'s error state with "Try again".
3. `InvestigationScreen.svelte` — header "Investigation" + echoed problem text + the count line, then candidates, then the chain strip, then the technical-details disclosure. Uses `StateShell` for loading (skeleton shaped like two candidate cards) and error.
4. `CandidateCard.svelte` — rank badge (`#1`), `match_label` chip ("Strong match" / "Possible match"), title, time, `WhyPanel` bullets ("Why it stands out" per §73 when bullets ≥ 2, "Why this matters" single line per §45 when there is exactly one), `Timing:` line, `[ View evidence ]` toggle. Ranks #2/#3 render as compact one-liners exactly as §45 shows, expanding on click.
5. `ConfidenceBadge.svelte` — maps `Chain.confidence` + `match_label` to "High" / "Medium" / "Low" with a tooltip explaining the tier in §29 language; the §73 caveat line renders directly beneath the badge from `Explanation.caveat`.
6. `EvidenceList.svelte` + `NextStepNote.svelte` — the §74 shape: label column (".env", "Container", "Network") with the change on the right, then "Possible next step" as a bordered note with `--info` accent. `NextStepNote` renders only when the engine supplied one; it is advisory text, never an action button (§74 forbids automatic config modification, so no "Fix it" affordance exists at all).
7. `ScoreBreakdownTable.svelte` — six rows (Temporal proximity, Relationship, Category relevance, Severity, Rarity, Failure proximity), each with the numeric contribution, a bar, and the reason string; footer row shows the total and the threshold that produced the label. This is the trust surface (§28) — it renders whatever the engine sent, including zeros.
8. `ZeroResults.svelte` — for the honest empty case: "I did not find a change that explains this." + the window that was searched + "What I monitor" link to Settings + "Create a checkpoint now so the next comparison has a baseline." Never a spinner-forever, never a fabricated candidate.
9. `InvestigationHistory.svelte` on Home — last five investigations as compact rows (problem text, time, top candidate), clicking restores.
10. Tests: `investigation-render.test.ts` — feed the §45 fixture DTO and assert every §45 line appears with the exact strings and order; `tone.test.ts` — feed the §73 and §74 fixture DTOs and assert the exact bullet list, "Confidence High", the caveat sentence, the §74 evidence rows and "Possible next step"; also assert the forbidden §29 phrases are absent from the rendered DOM; `disclosure.test.ts` — level 1 hides evidence and breakdown, `[ View evidence ]` reveals evidence only, "Show technical details" reveals the breakdown, Advanced mode pre-expands level 2; `zero-results.test.ts` — empty candidate list renders the calm copy and no "Strong match" chip; `home-input.test.ts` — placeholder text, Enter submits, 500-char cap, in-flight disabled state.

## Todo

- [ ] Commit 1 — `feat(home): problem input with example hint and recent investigations`
- [ ] Commit 2 — `feat(investigation): screen scaffold, candidates and why panel`
- [ ] Commit 3 — `feat(investigation): chain strip, evidence list and next-step note`
- [ ] Commit 4 — `feat(investigation): score breakdown disclosure and confidence badge`
- [ ] Commit 5 — `test(investigation): §45/§73/§74 tone fixtures, disclosure and zero results`

## Success Criteria

- [ ] The §45 example renders line-for-line from a fixture DTO (asserted string-by-string)
- [ ] The §73 output renders with its four bullets, "Confidence High", and the caveat sentence verbatim
- [ ] The §74 output renders the evidence rows and "Possible next step"; no action button exists anywhere in the investigation UI
- [ ] Rendered DOM contains none of the §29 forbidden phrases in any fixture (asserted)
- [ ] All visible sentences originate from the Rust DTO — a grep test asserts no hard-coded verdict strings in `src/features/investigation/` beyond static labels listed in `copy/en.ts`
- [ ] Zero-result state is calm, states the searched window, and offers the checkpoint next step
- [ ] Keyboard-only: type problem → Enter → arrow through candidates → expand evidence → open the source event
- [ ] Results paint < 100 ms after the command resolves (measured with `performance.now()` in a dev overlay)

## Verification Gate

```bash
bun run check && bun run test && bun run guard:types
cd src-tauri && cargo test --test analysis_language && cd ..
bun run tauri dev   # inject the port-chain scenario, run the §74 phrasing, walk all three disclosure levels
```

## Risk Assessment

- **Frontend starts making judgement calls (thresholds, wording)** — signal: a comparison or template literal appears in a Svelte file. Response: the grep test in the success criteria fails the build; all verdict text is DTO-supplied.
- **Confidence wording feels overconfident to a real user** — signal: manual read-through discomfort. Response: §73's caveat sentence is mandatory on every High-confidence result, and the badge tooltip always names the tier's evidence basis; wording changes happen in the Rust templates so tests catch drift.
- **Long problem text or long entity names break layout** — signal: overflow at 900 px. Response: text clamps at 3 lines with a "show full text" toggle; entity chips truncate with a title attribute; covered by the six-state `small-window` case for this screen.
- **Users expect the app to fix the problem** — signal: "where's the fix button". Response: deliberate product decision (§74) — the app explains, the user decides; `NextStepNote` phrasing is advisory and `docs/PRODUCT.md` states the non-goal.
- **Score breakdown confuses non-technical users** — signal: level-3 panel overwhelming. Response: it is collapsed by default and labelled "Show technical details"; level 1 is a headline plus bullets in plain language, which is what §73 targets.

## Security Considerations

The problem text is echoed back to the screen; it is rendered as a text node (no `{@html}`) and was
already sanitized server-side before persistence, so neither markup injection nor a pasted secret
surviving into the DB is possible. The investigation history stores the sanitized text only. No
investigation path performs a network call; the AI settings section (added in phase 17) renders as
"Off — cloud AI is not enabled" with no key field, because no provider exists to hold a key (§32, §63).

### FILE: phase-15-threejs-visualizations.md

---
phase: 15
title: "Three.js — Ambient Field, Causal Graph, System Map"
status: pending
priority: P1
effort: 12h
dependencies: [14]
---

## Overview

The three legitimate §39 uses, and only those: (a) the Home ambient field, (b) the causal graph — the
primary meaningful 3D view, (c) the ambient→focused-cluster investigation transition. Plus the §48
System Map, which is the causal-graph renderer pointed at the relationship graph instead of one
investigation. Everything else stays DOM (§39).

## Requirements

Functional: ambient field on Home where small particles = ordinary changes, larger nodes = important
changes, thin connections = related events, driven by *real* recent event data (never random noise);
causal graph with hover, click, zoom, careful pan/orbit, focus-node, and dependency-chain highlight;
transition from the ambient field into the focused cluster when an investigation starts; System Map over
Applications, Processes, Services, Ports, Docker containers, Network adapters, Config files answering
"what depends on this?", not required for normal users (§48). Non-functional (§38): 60 fps target,
ambient scene capped at 30 fps, rendering paused when the canvas is not visible
(`IntersectionObserver`) or the window is hidden/minimized (Tauri `Focused`/visibility + Page
Visibility API), fully static under `prefers-reduced-motion`, ≤ 60 draw calls, no post-processing, and
every geometry/material/texture disposed on unmount. Non-WebGL fallback is a real DOM view, not an error.

## Architecture

One shared renderer module — `viz/core/` — used by all three scenes so budget rules exist once:

```
viz/core/renderer.ts     createRenderer(canvas, { alpha, antialias: dpr<2, powerPreference:'low-power' })
viz/core/loop.ts         RenderLoop { targetFps, pause(), resume(), onFrame(dt) }  // rAF + accumulator
viz/core/visibility.ts   binds IntersectionObserver + document.visibilitychange + Tauri window events
viz/core/dispose.ts      DisposalBag { track(obj3d|geometry|material|texture), disposeAll() }
viz/core/capability.ts   detectWebGL(): { ok: boolean, reason?: string }  // one probe, cached
viz/core/labels.ts       DOM label overlay: project world -> screen, absolutely positioned divs
```

Text is **never** rendered in WebGL — labels are DOM elements positioned from projected coordinates
(`labels.ts`), which keeps typography crisp, accessible to screen readers, and consistent with §39's
"text stays DOM-based". The graph therefore has a real accessible fallback: the same node/edge data
renders as a nested DOM tree/list, and that list is what screen readers and the no-WebGL path use.

Geometry budget: one `InstancedMesh` for particles (ambient: ≤ 600 instances), one `InstancedMesh` for
nodes (graph: ≤ 400), one `LineSegments` for all edges (single `BufferGeometry`, colours per-vertex),
one sprite-free selection ring. Materials: `MeshBasicMaterial`/`LineBasicMaterial` only — no lights, no
shadows, no env maps, so the whole scene is ~4 draw calls and a laptop GPU idles.

Layout: the causal graph is a **deterministic** layered layout, not a physics simulation — chains map to
a time axis (x = timestamp normalized), depth = relationship distance (y), category = z band, with a
fixed jitter seeded from the entity key so the same data always draws identically (matches the
determinism promise of phase 13 and makes screenshots reproducible). The System Map uses the same layout
with x = category cluster and a bounded force-free radial arrangement.

## Related Code Files

Create: `src/viz/core/{renderer.ts,loop.ts,visibility.ts,dispose.ts,capability.ts,labels.ts,palette.ts}`,
`src/viz/scenes/{ambient.ts,causal-graph.ts,system-map.ts,transition.ts}`,
`src/viz/data/{to-ambient.ts,to-graph.ts,layout.ts}`,
`src/components/viz/{VizCanvas.svelte,AmbientField.svelte,CausalGraph.svelte,SystemMap.svelte,GraphLegend.svelte,GraphFallback.svelte,NodeTooltip.svelte}`,
`src/viz/__tests__/{layout.test.ts,dispose.test.ts,loop.test.ts,capability.test.ts,to-graph.test.ts}`,
`src/features/system/SystemMapScreen.svelte` (replace the phase-05 placeholder).
Modify: `src/features/home/HomeScreen.svelte`, `src/features/investigation/InvestigationScreen.svelte`
(mount the graph beside the chain strip), `src-tauri/src/commands/analysis.rs`
(`get_system_graph(filter) -> SystemGraph { nodes, edges }` with node/edge caps),
`src/lib/copy/en.ts`, `docs/ARCHITECTURE.md` (viz budget section), `package.json` (`three` 0.185.1,
`@types/three` matching minor — resolve the exact published `@types/three` version with
`bun info @types/three versions` at implementation time; if no matching version exists, use the
bundled types from `three` and record it).

## Implementation Steps

1. `capability.ts` — one `canvas.getContext('webgl2') ?? getContext('webgl')` probe with a `try/catch`, cached in a module singleton, exposing `reason` ("WebGL unavailable", "context creation failed", "software renderer detected" via `WEBGL_debug_renderer_info` when present). `VizCanvas.svelte` renders its `fallback` snippet when `!ok` — no console error, no toast, no retry loop.
2. `renderer.ts` + `loop.ts` — `WebGLRenderer` with `powerPreference: 'low-power'`, `antialias` only when `devicePixelRatio < 2`, `setPixelRatio(Math.min(dpr, 2))`, `setClearAlpha(0)`. `RenderLoop` uses one shared `requestAnimationFrame` driver for *all* mounted scenes (a single rAF in the app, scenes register callbacks) with per-scene `targetFps` throttling — this is what keeps two visible canvases from doubling the frame cost.
3. `visibility.ts` — pause when: `IntersectionObserver` ratio 0, `document.hidden`, Tauri window blurred for > 5 s, or the window is minimized (`window.isMinimized()` on the `Resized`/`Focused` events). Resume restores in ≤ 1 frame. Under `prefers-reduced-motion` the loop renders exactly one frame per data change and never animates.
4. `dispose.ts` + `dispose.test.ts` — `DisposalBag` tracking every created geometry/material/texture; the test creates a scene, mounts, unmounts, and asserts `renderer.info.memory.geometries === 0 && .textures === 0` afterwards (Three's own counters are the ground truth here). `VizCanvas.svelte` calls `disposeAll()` and `renderer.dispose()` in `onDestroy`, plus `forceContextLoss()` so repeated navigation cannot exhaust WebGL contexts (browsers cap ~16).
5. `to-ambient.ts` + `ambient.ts` — last 200 events → particle instances: radius from importance (ordinary 1.0, important 2.2, critical 3.0 units), colour from `palette.ts` (token-derived, read from CSS variables so themes match), y from recency, gentle 0.02 rad/s drift and a slow opacity breath at 30 fps; edges only between events sharing an entity, max 40 lines, opacity 0.25. Empty history ⇒ a still, sparse field, not a fake swarm (§54/§87: the visualization must not imply activity that did not happen).
6. `layout.ts` + `to-graph.ts` + `causal-graph.ts` — deterministic layered layout; interactions: raycast on `pointermove` (throttled to 30 Hz, against the instanced mesh with `instanceId`) → `NodeTooltip` DOM tooltip; click focuses (camera lerps target over 320 ms with `--ease-standard`); focus highlights the node's chain by recolouring edge vertex colours (no geometry rebuild); `OrbitControls` from `three/examples/jsm/controls/OrbitControls.js` with `enablePan: true`, `enableDamping: true`, `minPolarAngle`/`maxPolarAngle` clamped to ±35° from horizontal and `minDistance`/`maxDistance` set so the user cannot get lost or invert the scene ("careful pan/orbit", §39). Double-click resets the view; `R` resets; `Esc` clears focus.
7. `transition.ts` — investigation start: the ambient field's particles matching the investigation's candidate events lerp toward the focused cluster positions over 520 ms while the others fade to 0.05 opacity; implemented as an interpolation between two precomputed instance-matrix arrays (no per-frame allocation). Under reduced motion the transition is an instant swap.
8. `system-map.ts` + `SystemMapScreen.svelte` — nodes from `get_system_graph` (caps: 400 nodes, 1 200 edges, server-side, with "showing the 400 most connected entities" stated in the UI when truncated), category legend, filter chips per entity kind, click a node → side panel listing "Depends on" / "Used by" / "Recent changes" with links into the timeline. `GraphFallback.svelte` renders the same data as a grouped DOM list with disclosure triangles — this is also the a11y path, reachable by a "View as list" toggle available even when WebGL works.
9. Tests: `layout.test.ts` — same input twice ⇒ identical coordinates; a 400-node graph stays inside the bounding box; `to-graph.test.ts` — caps enforced, orphan nodes dropped, edge endpoints always present in nodes; `loop.test.ts` — 30 fps target yields ~30 callbacks/s with a fake clock, pause stops callbacks; `capability.test.ts` — a stubbed failing `getContext` produces `ok: false` with a reason and no throw.

## Todo

- [ ] Commit 1 — `feat(viz): shared renderer, single-rAF loop, visibility gating and disposal bag`
- [ ] Commit 2 — `feat(viz): capability probe with DOM fallback surface`
- [ ] Commit 3 — `feat(viz): home ambient field driven by real recent events`
- [ ] Commit 4 — `feat(viz): deterministic causal graph with focus and chain highlight`
- [ ] Commit 5 — `feat(viz): ambient-to-cluster investigation transition`
- [ ] Commit 6 — `feat(system-map): relationship graph screen with dependency panel and list view`
- [ ] Commit 7 — `test(viz): determinism, caps, disposal, loop throttling and capability`

## Success Criteria

- [ ] Draw calls ≤ 60 and geometries ≤ 8 for each scene (`renderer.info` asserted in a dev overlay and recorded in `docs/PERFORMANCE.md`)
- [ ] Ambient scene runs at ~30 fps, graph at ~60 fps while interacting; both drop to 0 fps when scrolled out of view, window blurred, or minimized (measured via a frame counter in the diagnostics overlay)
- [ ] `prefers-reduced-motion`: no animation, no drift, no transition; scenes still render their content
- [ ] Mount/unmount the System Map 20 times: geometries and textures return to 0 each time; no WebGL context exhaustion warning
- [ ] WebGL disabled (`--disable-gpu`-style stub in tests + a manual run with a forced-fail probe): every screen still works, list fallback renders, no error toast
- [ ] Idle CPU with Home visible ≤ 2 % on this machine; GPU utilization negligible (recorded)
- [ ] Empty database: ambient field renders a still, sparse scene with no invented particles
- [ ] Graph is keyboard-navigable via the list fallback; every node reachable and labelled

## Verification Gate

```bash
bun run check && bun run test
bun run tauri dev   # Home ambient -> run investigation (watch transition) -> System Map -> toggle list view
# manual: scroll Home away, confirm frame counter stops; minimize, confirm 0 fps; enable reduced motion in Windows settings
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1   # B2 checkpoint #4 (WebView2 + GPU)
```

## Risk Assessment

- **Three.js pushes the bundle and memory past budget (B2)** — signal: RSS jumps after mounting a scene. Response: `three` is imported through `viz/` only and code-split by dynamic `import()` so a user who never opens the System Map never loads the graph scene; the ambient field imports a narrow subset (`WebGLRenderer`, `Scene`, `PerspectiveCamera`, `InstancedMesh`, `SphereGeometry`, `LineSegments`, `BufferGeometry`, `MeshBasicMaterial`, `Color`, `Matrix4`) so tree-shaking is effective. Measure before/after and record both numbers.
- **Instanced raycasting is slow at 400 nodes** — signal: pointer lag. Response: 30 Hz throttle plus a coarse bounding-sphere pre-check; if still slow, switch to GPU-free hit testing against projected screen positions (the same projection `labels.ts` already computes).
- **`OrbitControls` import path changes between Three versions** — signal: unresolved module. Response: verify the exact path inside `node_modules/three/examples/jsm/controls/` after install; if the layout differs in 0.185.1, use the `three/addons/` alias that the package's `exports` map declares — read the installed `package.json` exports, do not guess.
- **Scene looks like "particle spam" or a gaming UI (§39 violation)** — signal: review discomfort. Response: hard caps (600 ambient particles, opacity ≤ 0.35, one accent hue plus status hues, no bloom/glow, no post-processing) are written into `docs/ARCHITECTURE.md` as budget rules and checked at review; the ambient field must remain legible behind DOM text at all times.
- **WebGL context lost mid-session (driver update — ironically the app's own subject matter)** — signal: `webglcontextlost` event. Response: listen for it, pause the loop, show the DOM fallback with "Graphics rendering paused after a display driver change. Reload to retry.", and recreate the context on user action. This is a real scenario for this app's users.

## Security Considerations

No shader compiles from user data, no textures load from disk or network (all colours are token
constants), and node labels come from already-sanitized entity names rendered as DOM text — so a
malicious filename cannot reach a shader or inject markup. Dynamic `import()` targets are static string
literals, so the bundle graph is fixed at build time and the CSP needs no `wasm-unsafe-eval` or remote
origin (§35, §63).

### FILE: phase-16-docker-development-mode.md

---
phase: 16
title: "Docker Module & Git / Development Mode"
status: pending
priority: P2
effort: 9h
dependencies: [15]
---

## Overview

Two opt-in modules for the developer half of the audience: the Docker collector (§23) that must never be
required and must never spam when Docker is absent, and Git/development mode (§24) that tracks
repository and manifest state for user-added project directories without parsing enormous repos. This
phase also completes the §74 chain end-to-end (config → container restart → port move) which until now
existed only as a synthetic fixture.

## Requirements

Functional — Docker: enable only when Docker is detected; connect over the local named pipe; track
container created/started/stopped/restarted/removed, image changed, health changed, port mapping
changed, volume mapping changed; support the §23 causal sequence `docker-compose.yml changed →
container api restarted → port 3000 disappeared → port 4000 appeared`; when absent show "Docker
integration — Not installed" with **no error spam** (one line in Settings, one log line at INFO on
startup, nothing else, ever). §24 Git: opt-in per project directory (reusing phase-10 watch roots of
kind `DevProject`); track branch changed, commit changed, working tree became dirty/clean, package
manifest changed, runtime config changed, lockfile changed; never continuously parse a huge repo.
Non-functional: absent-Docker cost = one pipe existence check per 60 s (a `FindFirstFileW` on
`\\.\pipe\*`, measured cheap) with exponential backoff to 10 min after three absences; git status on a
50 k-file repo must not run on every file event — see the debounce/gate design.

## Architecture

Docker detection: `FindFirstFileW(r"\\.\pipe\*")` enumeration looking for `docker_engine` (verified
readable non-admin per the matrix) — no HTTP probe, no shelling out to `docker.exe`, no TCP guess.
Only when the pipe exists does `bollard` 0.21.1 connect (`Docker::connect_with_named_pipe_defaults()`;
the named-pipe transport is behind a cargo feature — read `bollard`'s `Cargo.toml` at implementation time
for the exact feature name and enable exactly that, do not guess) and subscribe to `events()` with a
filtered stream (`type=container`, `type=image`, `type=volume`, `type=network`). Container name, image
tag, port bindings and health status come from the event's `Actor.Attributes` plus a targeted
`inspect_container` **only** on create/start/health-change (never a full `list_containers` loop).
Assumption A9 stands: Docker is not installed on this machine, so this collector is untestable here —
therefore every parsing path is driven by **captured JSON fixtures** committed under
`tests/fixtures/docker/`, the collector is written against those, and `docs/COLLECTORS.md` states
plainly that Docker was verified against fixtures and not against a live daemon. That honesty is
required (§87); pretending it was live-tested would be a fabrication.

Git: `gix` 0.87.1, opened once per project root and cached. Cheap signals are read directly from files
rather than through a repository walk: `HEAD` (branch), `refs/heads/<branch>` or the packed-refs entry
(commit), and the mtimes of `.git/HEAD`, `.git/index`, and the manifest/lockfile set. A *status* call
(the expensive one) runs only when `.git/index` mtime changes, at most once per 10 s per repo, on
`spawn_blocking`, with a hard 3 s timeout and a file-count guard: if `gix` reports > 20 000 tracked
entries, the repo is marked `LargeRepo` and dirty-state tracking is disabled for it with an honest
descriptor note ("Working-tree status is not tracked for very large repositories"). Manifests watched by
name (routed through the phase-10 file watcher, no new watcher): `package.json`, `bun.lock`,
`package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `Cargo.toml`, `Cargo.lock`, `pyproject.toml`,
`poetry.lock`, `requirements.txt`, `go.mod`, `go.sum`, `pom.xml`, `build.gradle`, `composer.json`,
`composer.lock`, `Gemfile.lock`, `docker-compose.yml`/`.yaml`, `Dockerfile`. Manifest *diffs* reuse the
phase-10 config differ (JSON/TOML aware, redacted), so a dependency version bump renders as
`react 18.3.1 → 19.0.0` rather than "file modified".

Graph edges written here (feeding phase 13): `docker_container → exposes → port`,
`docker_container → uses → image`, `project → contains → config_file`,
`config_file → configures → application` (compose file → its containers by service name). These are the
edges that turn §74's evidence into a `Confirmed` chain instead of a temporal coincidence.

## Related Code Files

Create: `src-tauri/src/collectors/{docker.rs,git.rs}`,
`src-tauri/src/collectors/docker/{detect.rs,events.rs,map.rs}`,
`src-tauri/src/collectors/git/{repo.rs,manifests.rs,status_gate.rs}`,
`src-tauri/tests/{collector_docker_fixtures.rs,docker_absent_no_spam.rs,collector_git.rs,git_large_repo.rs}`,
`src-tauri/tests/fixtures/docker/{events_container_restart.json,inspect_api_container.json,events_image_pull.json,events_health_change.json,events_volume_mount.json}`,
`src/features/settings/{DockerPanel.svelte,DevModePanel.svelte}`,
`src/features/timeline/ContainerEventDetail.svelte`.
Modify: `src-tauri/Cargo.toml` (`bollard` 0.21.1 with the named-pipe feature only, `gix` 0.87.1 with
`default-features = false` + the minimal feature set for refs/status — resolve exact feature names from
the crate's `Cargo.toml`), `src-tauri/src/collectors/files.rs` (manifest routing),
`src-tauri/src/core/graph_writer.rs`, `docs/COLLECTORS.md`, `docs/PRIVACY.md`, `src/lib/copy/en.ts`.

## Implementation Steps

1. `docker/detect.rs` — pipe enumeration + `DockerPresence { Present, Absent, Error(String) }`; 60 s → 10 min backoff; a transition Absent→Present emits one `DockerDetected` event and starts the collector; Present→Absent stops it and emits one `DockerUnavailable` event. Exactly one INFO log line per transition — the anti-spam rule is enforced by a test that runs 30 detection cycles with Docker absent and asserts ≤ 1 log line and 0 events.
2. `docker/events.rs` — `bollard` event stream with a reconnect policy (5 s, 15 s, 60 s, then 5 min steady) that logs at DEBUG, not WARN, because a stopped Docker Desktop is normal; the collector's status becomes `Idle — Docker is not running`, which the UI shows plainly.
3. `docker/map.rs` — event → `EventDraft` mapping for the eight tracked change kinds, including port-mapping diffs (compare previous `PortBindings` from the cached inspect against the new one) and volume-mapping diffs; container names are sanitized (a container name can contain a project path fragment).
4. `git/repo.rs` + `status_gate.rs` — cheap-signal reads, the mtime-triggered gate, `spawn_blocking` + timeout + `LargeRepo` guard; events `BranchChanged { old, new }`, `CommitChanged { old_short, new_short, subject }` (subject sanitized and truncated to 120 chars — commit messages can contain tokens), `WorkingTreeDirty { changed_files: u32 }`, `WorkingTreeClean`.
5. `git/manifests.rs` — manifest classification and diff routing into the phase-10 differ; a lockfile diff reports only the count of changed dependencies plus the top 10 by name ("42 dependencies changed") because a full lockfile diff is thousands of rows.
6. `DevModePanel.svelte` — lists the `DevProject` roots with per-project toggles (git tracking, manifest tracking), the `LargeRepo` note where it applies, and a "Detect projects in a folder…" action reusing the phase-10 picker flow. `DockerPanel.svelte` shows one of exactly three states: "Not installed", "Installed — not running", "Connected — N containers tracked".
7. `ContainerEventDetail.svelte` — container name, image, ports before/after, health transition, and a link to the compose file's event when the graph has that edge (this is the visible payoff of §23's causal sequence).
8. Tests: `collector_docker_fixtures.rs` — each captured fixture maps to the expected `EventDraft` set (restart produces exactly one `ContainerRestarted`, not stop+start; the port-mapping fixture produces one `PortMappingChanged` with both maps); `docker_absent_no_spam.rs` — the 30-cycle silence assertion; `collector_git.rs` — a temp repo created with `gix` (or by writing `.git` files directly if `gix` init is heavy): assert branch change, commit change, dirty/clean transitions, and that a commit subject containing `sk-<fixture>` is stored redacted; `git_large_repo.rs` — a synthetic repo with a stubbed entry count > 20 000 flips `LargeRepo` and skips status, with the descriptor note present.

## Todo

- [ ] Commit 1 — `feat(collectors): docker presence detection with backoff and no spam`
- [ ] Commit 2 — `feat(collectors): docker event stream mapping from captured fixtures`
- [ ] Commit 3 — `feat(collectors): git branch, commit and working-tree tracking with status gate`
- [ ] Commit 4 — `feat(collectors): manifest and lockfile change summaries`
- [ ] Commit 5 — `feat(core): docker and project graph edges`
- [ ] Commit 6 — `feat(ui): docker and development mode settings panels`
- [ ] Commit 7 — `test(collectors): docker fixtures, absent-silence, git and large-repo guard`

## Success Criteria

- [ ] With Docker absent (this machine): Settings shows "Docker integration — Not installed", exactly one INFO log line at startup, zero events, zero warnings over a 1 h run
- [ ] All five Docker fixtures map to the exact expected events (restart ≠ stop+start)
- [ ] Switching branches in a watched repo produces one `BranchChanged` within 5 s; committing produces one `CommitChanged` with a redacted, truncated subject
- [ ] `git status` runs at most once per 10 s per repo and never on the async runtime (asserted by a counter + a `spawn_blocking` assertion)
- [ ] A > 20 000-entry repo is marked `LargeRepo`, skips status, and says so in Settings
- [ ] Editing `docker-compose.yml` in a watched project writes a `project → contains → config_file` edge; with fixtures, the §74 chain builds as `Confirmed`
- [ ] Manifest change renders `react 18.3.1 → 19.0.0`; lockfile change renders a counted summary, not thousands of rows

## Verification Gate

```bash
cd src-tauri && cargo test --lib && cargo test --test collector_docker_fixtures --test docker_absent_no_spam --test collector_git --test git_large_repo
cd .. && bun run check && bun run test && bun run guard:redaction
bun run tauri dev   # add this repo as a dev project, switch branches, edit package.json
```

## Risk Assessment

- **A9 — Docker cannot be live-tested on this machine** — signal: no daemon. Response: fixture-driven implementation plus an explicit "verified against fixtures, not a live daemon" note in `docs/COLLECTORS.md` and in the phase report. Do **not** claim live verification. If the user later installs Docker Desktop, a single manual smoke script in `docs/TESTING.md` covers the live path.
- **`bollard`'s named-pipe feature name or connect helper differs** — signal: compile error or missing function. Response: read `bollard` 0.21.1's `Cargo.toml` and `src/docker.rs` before writing; the crate has historically gated Windows pipe support behind a feature, so the exact name must be taken from the source, not memory.
- **`gix` status API shape differs from expectation** — signal: compile error. Response: `gix` moves fast; read the installed crate's docs/source for the status entry point. If the status API is too unstable to use safely, degrade to "index mtime changed ⇒ working tree may have changed" (an honest, weaker signal) and record the deviation rather than shelling out to `git.exe` (a shell call is forbidden by the phase-02 guard).
- **Large monorepo makes dev mode expensive** — signal: CPU spike. Response: the 10 s gate, the 3 s timeout, the 20 000-entry guard, and per-project opt-out; measured cost shown in the panel.
- **Compose-file → container mapping is wrong (service name ≠ container name)** — signal: a wrong `Confirmed` edge. Response: match on the `com.docker.compose.project` / `com.docker.compose.service` labels from the inspect payload, which is the authoritative mapping; when labels are absent, downgrade the edge to inferred (`confirmed: false`) so phase 13 can only build a `Likely` chain from it.

## Security Considerations

Docker access is read-only: `events()` and `inspect_container` only — no `create`, `start`, `stop`,
`exec`, or image pull anywhere in the codebase (guard denylist covers `bollard`'s mutating method
names). Talking to the Docker socket is effectively root-equivalent on the host, so `docs/PRIVACY.md`
and `docs/SECURITY.md` state that the module is opt-in-by-detection, read-only, and never forwards
container environment variables — env is explicitly **not** requested from inspect payloads, and if a
future payload includes it, `scrub_pairs` redacts it before storage. Git commit subjects and branch
names pass through `scrub_text`; repository *contents* are never read, only metadata and manifest
key/value pairs (§34).

### FILE: phase-17-release-polish-audit.md

---
phase: 17
title: "Onboarding, Settings, Notifications, Diagnostics, Installer & Release Audit"
status: pending
priority: P1
effort: 16h
dependencies: [16]
---

## Overview

Everything between "the features work" and "a stranger can install and trust it": §50 onboarding, the
full §49 Settings tree with per-toggle explanations, §53 conservative notifications, §54 importance
calibration, §84 diagnostics with redacted export, §85 dev simulation panel, §64 update checking, §65
NSIS installer with uninstall history prompt, §66 signing documentation, the §82 threat-model audit, the
§68 documentation reconciliation, and the B2 memory decision point.

## Requirements

Functional: onboarding completes in < 1 min with the five §50 steps and the exact default check state
(checked: Apps & updates, Windows settings, Network, Drivers, Services; unchecked: Development tools,
Docker); Settings has the eight §49 sections with all ten monitoring toggles, each stating what is
collected, why, estimated cost (from measured `cost_note`), and whether admin access is needed (all
"No" in this MVP); notifications fire only for the §53 "good" cases and provably never for the four
"bad" cases; Diagnostics shows all nine §84 fields and exports a redacted bundle; "Check for updates"
works with no silent auto-update (§64); the installer produces `WhatChanged-Setup-x64.exe` with Start
Menu shortcut, optional desktop shortcut, optional launch at startup, clean uninstall, no silent data
deletion on upgrade, and an uninstall-time prompt about history (§65). Non-functional: idle budget
verified (§37), cold start measured, first-paint measured, DB size after a 24 h soak recorded, and the
release audit checklist fully green before the phase closes.

## Architecture

Notification gating is a single pure function so "never spam" is testable rather than aspirational:

```rust
// notify/policy.rs
pub fn should_notify(ev: &SystemEvent, ctx: &NotifyContext) -> Option<Notification>
```

Rules, in order: (1) `settings.notifications` must be `Important` or `Critical`-only (default
`CriticalOnly` — "very low volume", §53); (2) category must be in the allow-list
{Drivers, Network, Services, Updates, Docker health, Health}; (3) the event must satisfy at least one
*context* condition — it changed an entity referenced by a checkpoint created in the last 14 days
(produces §53's "A graphics driver changed after you created 'Before driver update' checkpoint."), or it
is a `Critical` failure event, or it is a health transition (interface present, source deferred per §26);
(4) rate limit: ≤ 3 notifications per hour, ≤ 8 per day, and never two for the same entity within 6 h;
(5) an explicit deny-list of routine app-update events (§53's "Chrome updated." / "Spotify updated." /
"Process started." / "Process stopped.") which can never notify regardless of other rules. Every §53
example — good and bad — is a test case.

Importance calibration (§54): `informational | notable | important | critical` mapped from a single
`importance: u8` (0-100) with named bands (0-29 informational, 30-59 notable, 60-84 important, 85-100
critical) assigned by a rule table per event kind, reviewed in this phase against a 24 h real capture:
the acceptance rule is **≤ 5 % of a day's events land in `important` or above** on an ordinary machine.
If more do, the table is retuned — not the definition of "important".

Diagnostics export: builds a JSON bundle from live state + the last 200 log lines, runs the whole bundle
through `redact::scrub_text` a second time (defence in depth — the log sink already redacts), strips
usernames from paths (`C:\Users\<name>` → `C:\Users\<user>`), and writes it via the dialog plugin's save
picker. A test feeds a bundle containing every §60 secret shape plus a real-looking username and asserts
none survive.

Installer: Tauri NSIS bundler (decision D3 — WiX rejected: WiX 3 requires .NET 3.5 and the matrix could
not verify it here; NSIS is bundled and self-contained). `installMode: perMachine` is **not** used —
`currentUser` keeps the whole product non-elevated end to end (§36). NSIS hooks: an
`uninstall.nsh`-style custom section asking "Also delete your recorded history?" (default **No**),
implemented through Tauri's `nsis.installerHooks` template file; the answer drives deletion of
`%LOCALAPPDATA%\WhatChanged`. Upgrade path never touches that directory.

## Related Code Files

Create: `src/features/onboarding/{OnboardingFlow.svelte,StepWelcome.svelte,StepPrivacy.svelte,StepMonitoring.svelte,StepStartup.svelte,StepDone.svelte,ExampleTimeline.svelte}`,
`src/features/settings/{SettingsScreen.svelte (rewrite),GeneralSection.svelte,MonitoringSection.svelte,PrivacySection.svelte,StorageSection.svelte,AppearanceSection.svelte,AdvancedSection.svelte,IntegrationsSection.svelte,AboutSection.svelte,ToggleExplainer.svelte,StorageUsage.svelte}`,
`src/features/diagnostics/{DiagnosticsScreen.svelte,HealthTable.svelte,ExportButton.svelte}`,
`src/features/dev/SimulationPanel.svelte`,
`src-tauri/src/notify/{mod.rs,policy.rs,dispatch.rs}`,
`src-tauri/src/core/importance.rs`, `src-tauri/src/app/diagnostics.rs`,
`src-tauri/src/commands/{diagnostics.rs,updates_check.rs,onboarding.rs}`,
`src-tauri/nsis/installer-hooks.nsh`,
`src-tauri/tests/{notify_policy.rs,importance_bands.rs,diagnostics_redaction.rs,migration_upgrade.rs}`,
`src/features/settings/__tests__/settings-sections.test.ts`,
`src/features/onboarding/__tests__/onboarding.test.ts`,
`docs/{RELEASE.md,TESTING.md,SIGNING.md}`, `scripts/release-audit.ts`.
Modify: `src-tauri/tauri.conf.json` (bundle, NSIS, updater), `src-tauri/Cargo.toml`
(`tauri-plugin-updater` 2.10.1), `src-tauri/src/lib.rs`, all seven §68 docs, `README.md`,
`src/lib/copy/en.ts`.

## Implementation Steps

1. `OnboardingFlow.svelte` — five steps with the §50 copy verbatim, progress dots, Back/Next, `Esc` cannot dismiss mid-flow (it is a first-run gate, not a modal), completion writes `settings.onboarded_at` and starts monitoring. Step 3 lists the ten §49 monitoring groups with the §50 default check state; step 4 offers "Start with Windows? — Recommended" wired to the phase-06 autostart control; step 5 shows `ExampleTimeline.svelte` — a short, clearly-labelled **example** ("Example — this is what your timeline will look like") animated with CSS only, never presented as real data (§87).
2. Settings rewrite into the eight §49 sections; `ToggleExplainer.svelte` renders four fields per toggle from the `CollectorDescriptor`: "What we collect", "Why", "Cost" (measured `cost_note`), "Admin needed: No" — all four come from Rust, so a new collector cannot ship without them (the descriptor fields are non-`Option`). Storage section shows the §11 estimate broken down (events, snapshots, checkpoints, FTS index, WAL) from `page_count * page_size` per table via `dbstat` if available, else a row-count estimate labelled "approximate".
3. `importance.rs` — the band table + `importance_for(kind, context) -> u8`; `importance_bands.rs` asserts the mapping and the ≤ 5 % rule against a committed 24 h capture fixture (exported from a real soak run with `scripts/`-assisted anonymization).
4. `notify/policy.rs` + `dispatch.rs` — the five ordered rules; `dispatch.rs` sends via `tauri-plugin-notification` and records every decision (fired / suppressed + rule id) in a bounded ring buffer surfaced in Diagnostics, so "why didn't I get notified" is answerable.
5. `app/diagnostics.rs` + screen — the nine §84 fields: app version (from `CARGO_PKG_VERSION`), DB size (file + WAL), event count, collector health table (status, last ok, last error, events emitted), last collector error, WebView2 version (`GetAvailableCoreWebView2BrowserVersionString` via the `webview2-com` types already in Tauri's dependency graph — read the available re-export before adding a dependency; if unavailable, read the Evergreen registry version key `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}\pv`), Rust backend health (writer queue depth, dropped events, degraded reasons), monitoring status, plus search backend and DB integrity state. "Export diagnostics" as designed.
6. `SimulationPanel.svelte` (§85) — dev-gated twice (compile + `settings.developer_mode`), injecting the six named scenarios (driver update, port change, process crash, DNS change, Docker restart, config diff) through the phase-07 injectors; a visible red "Simulated" badge on injected events and a `source: Simulated` column value so a simulated event can never masquerade as real (§87) and can be purged with one action.
7. Updater — `tauri-plugin-updater` 2.10.1 configured with `"active": false`-style manual flow: no update check on launch, a "Check for updates" button in About that calls `check()` and shows the result; `dangerousInsecureTransportProtocol` never enabled; pubkey placeholder documented in `docs/SIGNING.md` with the generation command, and **no private key in the repo** (`.gitignore` covers `*.key`, `~/.tauri`). Migration safety (§64): `migration_upgrade.rs` restores a v1-schema DB fixture, runs the app's migrations, and asserts all rows survive and `user_version` advances — the test that proves "updates never erase the event database".
8. Installer — `tauri.conf.json` bundle block: `targets: ["nsis"]`, `nsis: { installMode: "currentUser", installerIcon, headerImage, sidebarImage, installerHooks: "nsis/installer-hooks.nsh", languages: ["English"], displayLanguageSelector: false }`, `shortcutName: "What Changed"`, `createDesktopShortcut` offered as an installer checkbox, `productName: "What Changed"`, `mainBinaryName: "WhatChanged"`, `publisher`, `copyright`, `resources` empty, `windows.webviewInstallMode: { type: "downloadBootstrapper" }` (documented as the only online step, §63 applies to the *running* app). Verify the produced artifact name and rename to `WhatChanged-Setup-x64.exe` if the bundler's default differs; document the exact output path in `docs/RELEASE.md`.
9. `scripts/release-audit.ts` — one command (`bun run audit:release`) that mechanically checks: all seven §68 docs exist and contain no `TODO`/placeholder markers; every `CollectorDescriptor` has non-empty `coverage_limits` and `cost_note`; every command in `bindings.ts` appears in `docs/ARCHITECTURE.md`'s command table; no `TODO`/`unimplemented!`/`todo!()`/`panic!("not implemented")` in `src-tauri/src`; no `console.log` in `src/` outside `src/lib/dev/`; CSP contains no `unsafe-eval` and no remote origin; capabilities list matches the documented set; `.gitignore` covers keys and dotenv; and the §82 threat-model table has a mitigation line for all eleven threats. Exit non-zero on any failure.
10. §82 audit pass — for each of the eleven threats write the mitigation and the *evidence* (test name or file:line) into `docs/SECURITY.md`: malicious frontend script (CSP + no `{@html}` + narrow IPC), malicious config file (size/encoding gates, hand-written parsers, no eval, 40-entry cap), symlink/path traversal (canonicalize + root containment check + reject reparse points on watch roots — add `symlink_guard.rs` test), secret leakage (phase 02+03 tests), DB corruption (phase 03 recovery + §83 three actions), untrusted Docker metadata (sanitize + fixture tests), malformed event-log entries (hand-written extractor + fuzz-ish fixtures with truncated XML), large event floods (bounded channels, dedupe, cluster, throttle, drop counters), IPC abuse (validation limits + no path/SQL/shell surface + dev-command gating), update tampering (signature verification, manual check only), local privilege escalation (no elevation, no service, `currentUser` install, no writable-by-others install dir).
11. B2 decision point — run the final measurement (idle 10 min, Home visible, all collectors on): backend RSS, WebView2 processes, and total. Record all three in `docs/PERFORMANCE.md` against §37's budget. If total > 80 MB, **stop and ask the user** whether to accept the measured number as the documented budget or to cut scope (e.g. ambient viz off by default); do not silently redefine the metric.
12. Docs reconciliation — bring all seven §68 files to final state with measured numbers replacing every estimate, plus `README.md` (what it is, install, the §67 command list, privacy summary) and `docs/TESTING.md` (the manual test scripts for anything not automatable here: live Docker, elevation-requiring paths, installer/uninstall runs).

## Todo

- [ ] Commit 1 — `feat(onboarding): five-step first-run flow with example timeline`
- [ ] Commit 2 — `feat(settings): eight sections with descriptor-driven toggle explanations`
- [ ] Commit 3 — `feat(core): importance bands calibrated against a 24h capture`
- [ ] Commit 4 — `feat(notify): conservative notification policy with rate limits and decision log`
- [ ] Commit 5 — `feat(diagnostics): health surface and redacted diagnostics export`
- [ ] Commit 6 — `feat(dev): gated simulation panel with simulated-source marking`
- [ ] Commit 7 — `feat(release): nsis installer, uninstall history prompt and manual update check`
- [ ] Commit 8 — `test(release): notify policy, importance bands, export redaction, migration upgrade`
- [ ] Commit 9 — `docs: threat model, privacy, collectors, analysis, release and signing`
- [ ] Commit 10 — `chore(release): release audit script and measured performance record`

## Success Criteria

- [ ] Fresh-profile onboarding completes in < 60 s with the exact §50 copy and default check state
- [ ] Every one of the ten §49 monitoring toggles shows what/why/cost/admin, all sourced from Rust
- [ ] §53: all four "bad" examples are provably suppressed (unit tests) and both "good" examples fire; ≤ 3/hour and ≤ 8/day enforced
- [ ] 24 h soak: ≤ 5 % of events are `important` or above; no notification storm; idle CPU and RSS recorded against §37
- [ ] Diagnostics shows all nine §84 fields with live values; exported bundle contains zero §60 secret shapes and no real username
- [ ] `bun run audit:release` exits 0 with every check enumerated in its output
- [ ] `bun run tauri build` produces `WhatChanged-Setup-x64.exe`; install → Start Menu shortcut works, optional desktop shortcut works, optional autostart works
- [ ] Upgrade install over an existing version preserves the database (verified by row count before/after) and `migration_upgrade.rs` is green
- [ ] Uninstall asks about history; choosing "No" leaves `%LOCALAPPDATA%\WhatChanged` intact; choosing "Yes" removes it
- [ ] `docs/SECURITY.md` has a mitigation + evidence line for all eleven §82 threats
- [ ] All seven §68 docs contain measured numbers, no placeholders, and no stale claims
- [ ] B2: measured memory recorded; if over budget, the user has been asked rather than the metric redefined

## Verification Gate

```bash
cd src-tauri && cargo test --lib && cargo test --test notify_policy --test importance_bands --test diagnostics_redaction --test migration_upgrade --test symlink_guard
cargo clippy --all-targets -- -D warnings && cargo fmt --check
cd .. && bun run check && bun run test && bun run guard:redaction && bun run guard:types && bun run guard:contrast
bun run audit:release
bun run tauri build
# manual: install the artifact on a clean profile, run onboarding, soak 24h, then uninstall both ways
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1   # final B2 record
```

## Risk Assessment

- **B2 — total memory exceeds 80 MB (likely with WebView2)** — signal: the final measurement. Response: the pre-decided path is to report the measured split (backend vs WebView2) and ask the user to choose between accepting a documented budget and cutting scope. Never quietly change the target, and never claim the budget was met (§87). Mitigations available if the user wants them: ambient viz off by default, `mmap_size=0` retained, reader pool max 2, Three.js scenes lazily imported (already done in phase 15).
- **NSIS `installerHooks` template syntax differs across Tauri 2 minors** — signal: bundling fails. Response: read the `tauri-bundler` NSIS template shipped in the vendored crate to get the exact hook macro names before writing the file; if hooks prove unusable, the uninstall prompt degrades to a documented manual step in `docs/RELEASE.md` and an in-app "Delete all history" — the §65 wording ("where technically feasible") covers this, but the fallback must be stated honestly, not silently dropped.
- **WebView2 version read needs a dependency that is not present** — signal: no re-export available. Response: the registry `pv` key fallback (matrix-verified readable non-admin).
- **Code signing cannot be completed (no certificate)** — signal: no cert available to a solo dev. Response: `docs/SIGNING.md` documents both dev (self-signed for local testing, clearly marked) and production (EV/OV cert, `signCommand`/`certificateThumbprint` config, timestamp server) paths, with **no secrets in the repo** (§66); the release is documented as unsigned until a certificate exists, and the SmartScreen consequence is stated plainly to the user.
- **The 24 h soak reveals a slow leak** — signal: RSS trend up. Response: soak is part of this phase's gate precisely so this is found before release; the leak is fixed here (handle counts, COM threads, dedupe map, viz disposal are the four suspects, each already instrumented).
- **Audit script becomes a rubber stamp** — signal: it passes while a doc is stale. Response: the script checks *machine-verifiable* facts only (existence, absence of markers, descriptor completeness, command-table parity); the human review items live in a checklist in `docs/RELEASE.md` and are signed off in the phase's final commit message.

## Security Considerations

This phase is where the §82 threat model becomes a documented, evidence-linked artifact rather than an
intention. Three concrete hardening items land here: the symlink/reparse-point guard on watch roots
(preventing a crafted junction from turning a chosen folder into system-wide watching), the diagnostics
export double-redaction plus username scrubbing (the most likely accidental-leak channel, since users
paste exports into forums), and the updater configuration review (signature verification mandatory,
manual check only, no insecure transport). Install stays per-user and non-elevated, so no
writable-by-everyone directory ends up on the machine's PATH or in a service context (§36).

## Phase decomposition rationale

**§69 is binding; this is the mapping.** The packet permits splitting a user phase into multiple plan
phases for reviewability but forbids reordering dependency logic or dropping a phase.

| §69 user phase | Plan phases | Split reason |
|---|---|---|
| PHASE 0 FOUNDATION | 01 toolchain+scaffold+docs, 02 redaction chokepoint, 03 storage+migrations+logging, 04 typed IPC codegen, 05 design system+app shell, 06 tray+lifecycle+monitoring | Six reviewable units; §69's PHASE 0 acceptance list (launch, window, tray, close→tray, DB created, theme, IPC, installer) is fully satisfied by 01+03+05+06 |
| PHASE 1 EVENT ENGINE | 07 event engine+synthetic generator+live updates, 08 timeline+search+virtualization | §69 explicitly asks for the polished timeline on fake data first; 07 delivers the generator, 08 the UI |
| PHASE 2 FIRST REAL COLLECTORS | 09 collector framework+process+ports, 10 files+apps+services | 09 establishes the trait/supervisor/descriptor contract with the two cheapest collectors (6.2 ms, ~3 ms) before the heavier five |
| PHASE 3 WINDOWS CHANGE INTELLIGENCE | 11 updates+drivers+network+startup+event log+snapshots | Single phase, but the largest (18h) — five mechanisms, four of them COM/Win32 |
| PHASE 4 CHECKPOINTS | 12 checkpoints+comparison | — |
| PHASE 5 ANALYSIS ENGINE | 13 analysis engine, 14 investigation experience | Ranking correctness (§59 fixture) is provable headlessly in 13; the screen in 14 |
| PHASE 6 CAUSAL GRAPH + THREE.JS | 15 ambient field+graph+transitions | Deliberately after everything usable, per §38 |
| PHASE 7 DOCKER / DEV MODE | 16 docker+git+project config+runtime detection | — |
| PHASE 8 POLISH / RELEASE | 17 onboarding+settings+notifications+diagnostics+installer+audit | — |

**Three deliberate deviations, each justified:**

1. **The redaction chokepoint (02) is built before storage (03), not in PHASE 8 with the rest of
   privacy.** §69's PHASE 8 lists "privacy" as polish; §33/§34 make redaction a data-model invariant. If
   redaction lands last, every earlier phase writes unredacted rows and the fix is a migration plus a
   trust problem. Making `SanitizedEvent` the only type `insert_batch` accepts turns "we forgot to
   redact" into a compile error for phases 07–16. No dependency is reordered (nothing in PHASE 0 depends
   on privacy work) and nothing is dropped — PHASE 8's privacy *surfaces* (settings copy, threat-model
   doc, export scrubbing, audit) still happen in 17.
2. **Typed IPC codegen (04) is its own phase.** Every later phase adds commands; getting
   generate-then-typecheck wired once (and its fallback decided — specta/tauri-specta, else ts-rs, else
   hand-written `bindings.ts` guarded by `scripts/guard-types.ts`) prevents 13 phases of drift.
3. **Analysis (13) is separated from Investigation UI (14).** The §59 port-chain fixture must rank
   correctly in a headless test before any pixel is spent; a wrong ranking behind a beautiful screen is
   the single worst failure mode for this product (§54, §91).

**Effort:** 5+5+9+5+12+6+10+12+12+12+18+7+14+9+12+9+16 = **173 h**, matching the plan frontmatter.

**Reviewability invariant:** every phase ends with the app runnable and the phase's own verification gate
green. No phase is allowed to close on a stub, a mock, a fake collector, or fabricated data (§87) — the
`audit:release` script's `TODO`/`unimplemented!` scan exists to make that mechanically checkable, and it
is run from phase 09 onward, not only at the end.

**File ownership:** execution is sequential by one agent, so no two phases hold the same file at once.
If the user later parallelizes, the only genuinely disjoint pair is **15 ∥ 16** (15 owns
`src/features/visualization/*` + `src/lib/three/*`; 16 owns `src-tauri/src/collectors/{docker,git}.rs` +
`src/features/dev/*`); both touch `bindings.ts`/`en.ts`/`Settings`, so those three files must be merged
by the controller, not by either phase. Everything else forms a dependency chain.

**Rollback:** each phase is a small commit series, so `git revert` of one commit is the unit of undo.
The only non-revertible artifact is the database schema: migrations are additive (new tables, new
columns with defaults, new indexes — never a destructive `ALTER`), a `VACUUM INTO` backup precedes every
migration run, and an older build that meets a newer `user_version` refuses to write instead of
downgrading (phase 03 step 4). So reverting code never corrupts recorded history; at worst the user is
told to update.

## Risks carried forward

| ID | Risk | Likelihood × Impact | Where it is handled | Residual |
|---|---|---|---|---|
| **B1** | Rust absent; 3 ambiguous MSVC installs → cannot build at all | High × Critical | Phase 01 steps 1–3: `rustup` MSVC toolchain, `vswhere -latest` → `scripts/dev-shell.cmd` (vcvars64), verified by a real `tauri build` | If rustup itself is blocked, **every** phase is blocked — reported, not worked around |
| **B2** | Idle RSS < 80 MB likely unreachable with WebView2 | High × Medium | Measured at phases 01, 06, 15, 17; mitigations pre-listed (lazy Three.js, `mmap_size=0`, reader pool, viz default) | Escalated to the user as a scope question in phase 17 step 11; the metric is never silently redefined |
| **B3** | Non-admin: no reliable process-start push (`Win32_ProcessStartTrace` = Access denied; `__InstanceCreationEvent` captured 0 events in the measured 3 s test) → short-lived processes missed | Certain × Medium | Phase 09: diff-poll at 5 s with machine-readable `coverage_limits` rendered in Settings, Diagnostics and `COLLECTORS.md`; no sub-interval timing claim may be printed unless observed | Permanent product limitation, stated to the user rather than hidden |
| A2/D2 | TypeScript 7.0.2 may break `svelte-check` 4.7.6 / Vite 8 plugin chain | Medium × Medium | Phase 01 decision gate D2: attempt 7.0.2, fall back to the latest 5.x on failure, record the resolved version in `docs/ARCHITECTURE.md` | Fallback is a documented downgrade, not a stack swap |
| A4 | `specta` 1.0.5 + `tauri-specta` 1.0.2 API shape unverified against tauri 2.11.5 | Medium × Medium | Phase 04: build the generator first with two commands; fallbacks ts-rs 12.0.1 → hand-written bindings + `guard:types` | Typed IPC is guaranteed by the guard script even in the worst case |
| A5 | `windows` 0.62.2 feature names / typed WUA bindings differ from expectation | Medium × High | Phases 09/11: read the vendored crate's feature list and generated bindings before writing call sites; WUA falls back to `IDispatch` late binding | COM work is the plan's biggest unknown; every COM site is `catch_unwind`-isolated and degradable |
| A9 | Docker cannot be verified live on this machine | Certain × Low | Phase 16: fixture-driven implementation, and the docs state "verified against recorded fixtures, not a live daemon" | Honest gap; `docs/TESTING.md` carries the manual live-daemon script |
| A11 | Event Log XML shapes vary by provider/build | Medium × Medium | Phase 11: hand-written extractor over `EvtRender` output with truncated/odd-shape fixtures; unknown shapes are skipped, counted, and never guessed | A skipped event is visible in Diagnostics, not silent |
| — | Analysis produces confident nonsense (worst product risk) | Medium × Critical | Phase 13: §59 fixture as a gate, weakest-link chain confidence, threshold-gated "Strong match", `Unexplained` deltas barred from evidence, §54 no-false-panic rules | If the fixture cannot be made to pass, the ranked list ships behind "possible" language rather than shipping a false claim |
| — | Notification fatigue kills trust in week one | Medium × High | Phase 17: pure `should_notify` with the four §53 must-not-fire cases as tests, ≤3/h ≤8/day, default critical-only | Decision log in Diagnostics makes suppression auditable |
| — | Collector cost regression (drivers 2128 ms, network 1441 ms, tasks 1326 ms) freezes the UI | Medium × Medium | Phases 09–11: all snapshots on `spawn_blocking`, notification-driven where an API exists, rare-sweep cadence for the expensive three, `#[ignore]` perf tests recording measured cost | Cost numbers are re-measured in phase 17 and written into `docs/PERFORMANCE.md` |
| — | No CI runner → regressions found late | Certain × Medium | `bun run verify` as the single local gate, run at every phase boundary; guard scripts encode the invariants a reviewer would otherwise have to remember | The gate is only as good as the discipline to run it; each phase's Verification Gate block is copy-pasteable |

## Unresolved questions

1. **B2 memory budget** — if measured idle total exceeds 80 MB (likely: WebView2 alone typically exceeds
   the whole budget), does the user accept the measured figure as the documented budget, or should scope
   be cut (ambient visualization off by default, timeline page size, viz frame cap)? Needed at phase 17;
   default action if unanswered is to document the measurement and ship with visualization enabled.
2. **Retention defaults** — §11 gives a storage estimate but no default retention window. Plan assumes
   90 days for ordinary events / 365 for important+critical / 30 for snapshots, all user-editable. Confirm
   or override at phase 03.
3. **Code signing** — is a signing certificate available? Without one the NSIS artifact is unsigned and
   SmartScreen will warn on every install. Plan documents both paths and ships unsigned by default; no
   signing secret is ever placed in the repo (§66).
4. **Update feed hosting** — the updater needs a static JSON endpoint and a keypair. Unspecified. Plan
   wires manual "Check for updates" against a configurable endpoint and leaves the URL/pubkey as
   documented placeholders in `docs/SIGNING.md` (never committed secrets); if the user has no host, the
   updater UI degrades to "You are on version X — check the release page" with the opener plugin.
5. **§26 health checks** — the user deferred the *checks*; the plan keeps the health *interface* (state
   enum, transitions, notification category) so nothing has to be re-architected. Confirm that keeping
   the interface is wanted, or drop it entirely to reduce surface.
6. **Simple/Advanced default (§51)** — plan defaults to Simple for a first-run user and remembers the
   choice. Confirm; the alternative (Advanced for the solo-dev owner) is a one-line settings default.

Status: DONE — complete 17-phase execution-ready plan for What Changed (Tauri 2.11.5 / Svelte 5.56.10 / rusqlite 0.40.2, 173 h) covering §1–§91 in §69's binding order, with a compile-enforced redaction chokepoint, measured-cost collector cadences, the §59 ranking fixture as an analysis gate, and B1/B2/B3 handled as explicit decision points rather than assumptions.



















