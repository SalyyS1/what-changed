# Candidate 1 — Prove the toolchain, install the privacy chokepoint, then earn one honest collector at a time

## Thesis

Three things can silently destroy this build: a Rust toolchain that will not link (B1), a
secret that reaches disk before redaction exists, and a collector that *looks* implemented
but is a stub. So the decomposition front-loads all three: Phase 01 ends only when a real
Tauri bundle links on this machine; Phase 02 makes `RedactedEvent` the only type the
repository will accept, so redaction is a compile-time chokepoint rather than a habit; and
every collector phase carries a controlled-action checklist that must be observed in the
real timeline before the phase closes. After that, each phase is one vertical slice with
one owned file set, ordered exactly along §69, with the expensive/uncertain parts (drivers
2.1 s, WUA COM, wevtapi, Three.js) landing after the cheap measured ones.

## Plan metadata

```yaml
---
title: "What Changed — Windows system change recorder (MVP)"
description: "Local-first Tauri 2 + Rust + Svelte 5 desktop app that continuously records meaningful Windows changes and answers 'It worked before. What changed?'"
status: pending
priority: P1
effort: 254h
branch: main
tags: [tauri2, rust, svelte5, sqlite, windows, privacy, local-first, desktop, observability]
created: 2026-08-29
blockedBy: []
blocks: []
---
```

## PLAN.MD BODY

````markdown
## Overview

"What Changed" is a non-elevated Windows 10/11 tray app that records meaningful local system changes into a local
SQLite history and explains, deterministically, which recent change most likely relates to a problem the user
describes in plain language. Repo root `D:/Project/.WhatChage/` (folder misspelled upstream; product, binary and
package are `What Changed` / `WhatChanged.exe` / `what-changed`, and nothing in code or config depends on the folder
name). Greenfield `main`, zero commits, **Rust not installed**. No Electron, no Go, no second process, no cloud.

## Goals

| # | Goal | Priority |
|---|---|---|
| 1 | Answer "it worked before, what changed?" in seconds, for a non-technical user | P1 |
| 2 | Record 12 collector domains as one normalized `SystemEvent` stream | P1 |
| 3 | Never persist or log a secret — enforced by type, not by discipline | P1 |
| 4 | Deterministic relevance + causal-chain ranking with no LLM in the path | P1 |
| 5 | Run fully non-elevated; degrade honestly where Windows denies a non-admin token | P1 |
| 6 | Stay calm: 4-level importance model, near-silent notifications | P2 |
| 7 | Premium, own-identity dark-first UI; six states on every screen | P2 |
| 8 | Three.js used intentionally (ambient field, causal graph, system map) — never as the UI | P2 |
| 9 | ~0–1% idle CPU, bounded DB, virtualized timeline at 1M events | P2 |
| 10 | Ship a signed-ready NSIS installer that never destroys the event database | P2 |

## Phases

| # | Phase | Status |
|---|---|---|
| 01 | [Foundation & Toolchain](./phase-01-foundation-toolchain.md) | pending |
| 02 | [Redaction & Privacy Core](./phase-02-redaction-privacy-core.md) | pending |
| 03 | [Storage, Domain Model & Migrations](./phase-03-storage-domain-migrations.md) | pending |
| 04 | [App Shell, Design System & Tray](./phase-04-app-shell-design-system.md) | pending |
| 05 | [Packaging & Budget Baseline](./phase-05-packaging-budget-baseline.md) | pending |
| 06 | [Event Engine & Synthetic Feed](./phase-06-event-engine-synthetic-feed.md) | pending |
| 07 | [Timeline, Details & Search](./phase-07-timeline-details-search.md) | pending |
| 08 | [Collector Runtime & Entity Graph](./phase-08-collector-runtime-entity-graph.md) | pending |
| 09 | [Process & Network Listener Collectors](./phase-09-process-network-collectors.md) | pending |
| 10 | [File & Config Collector](./phase-10-file-config-collector.md) | pending |
| 11 | [Application & Service Collectors](./phase-11-application-service-collectors.md) | pending |
| 12 | [Updates, Drivers & Snapshots](./phase-12-updates-drivers-snapshots.md) | pending |
| 13 | [Network, Startup & Event Log Collectors](./phase-13-network-startup-eventlog-collectors.md) | pending |
| 14 | [Checkpoints & Comparison](./phase-14-checkpoints-comparison.md) | pending |
| 15 | [Analysis Engine](./phase-15-analysis-engine.md) | pending |
| 16 | [Investigation Experience](./phase-16-investigation-experience.md) | pending |
| 17 | [Three.js Visualization Layer](./phase-17-three-js-visualization.md) | pending |
| 18 | [Docker & Development Mode](./phase-18-docker-development-mode.md) | pending |
| 19 | [Onboarding, Settings & Notifications](./phase-19-onboarding-settings-notifications.md) | pending |
| 20 | [Diagnostics, Recovery & Release Plumbing](./phase-20-diagnostics-recovery-release-plumbing.md) | pending |
| 21 | [Release Audit & Hardening](./phase-21-release-audit-hardening.md) | pending |

## Architecture Summary

- **Rust owns everything system-facing.** Svelte never touches an OS API. Frontend↔Rust is a fixed list of
  ~30 narrow typed commands (§56) plus 4 events (§57): no shell, no arbitrary file read, no SQL over IPC;
  `tauri-plugin-sql`/`-fs`/`-shell`/`-http` are never installed, and `tests/ipc_contract.rs` fails the
  build if the command list drifts.
- **core vs platform.** `src-tauri/src/{domain,storage,analysis,redact,privacy,collectors}` is
  platform-free; every Win32/COM call lives in `platform/windows/**` and `tests/layering.rs` fails if
  `use windows::` appears outside it. `platform/{linux,macos}` hold only `Unsupported` trait impls.
- **Collectors** implement §5's `Collector` trait, run one supervised panic-isolated Tokio task each with
  backoff restart, and emit `RawEvent` into a bounded channel; the normalizer converts it into a sealed
  `RedactedEvent` (templates, importance, entity + relationship upsert) before any insert is possible.
- **Redaction is a type.** `redact::seal()` is the only constructor of `RedactedEvent` and
  `repo_events::insert` the only writer; logs pass through a scrubbing `MakeWriter`, so persistence *and*
  logging share one chokepoint (§34).
- **SQLite** via `rusqlite` 0.40.2 (`bundled`) + `rusqlite_migration` 2.6.0: WAL, busy timeout, one
  dedicated writer thread with batched transactions, 2 read connections on `spawn_blocking`, keyset
  pagination, incremental vacuum, `VACUUM INTO` pre-migration backup.
- **Analysis** is deterministic: problem parser → candidate retrieval → §28 six-factor scoring with
  per-factor human reasons → relationship traversal → chain builder (Confirmed/Likely/TemporalOnly) →
  `ExplanationProvider`, whose only MVP impl is deterministic; LLM providers stay in ROADMAP (§32).
- **Types**: Rust is the source of truth; `specta` 1.0.5 + `tauri-specta` 1.0.2 generate
  `src/lib/tauri/bindings.ts`, and a test fails if the checked-in file is stale (§81).
- Docs at repo root per §55 (`ARCHITECTURE/SECURITY/PRIVACY/PRODUCT/ROADMAP/COLLECTORS/ANALYSIS/
  CONTRIBUTING/README`), measurements and ADRs under `docs/`; one deviation from §55 — an added
  `src/lib/copy/` dir so §72–§74 microcopy is single-sourced and snapshot-tested.

## Success Criteria

- [ ] `bun run tauri build` produces `WhatChanged-Setup-x64.exe`; it installs and runs non-elevated.
- [ ] Tray works; closing the window hides to tray; monitoring continues; all 7 §7 entries function.
- [ ] Every §9 field is populated for every collector; 12 collectors report health in Diagnostics.
- [ ] Redaction leak suite (§60, 9 secret shapes × 5 sinks) passes; no raw secret in DB or log bytes.
- [ ] Ranking fixtures pass: the §59 config→container→port chain outranks the unrelated Chrome update, and the §73 driver update ranks #1 at `Confidence: High` with no forbidden certainty phrase.
- [ ] Perf matrix recorded at 10k / 100k / 1M events; timeline stays virtualized (≤60 mounted rows).
- [ ] Migration suite proves an upgrade over a seeded DB loses zero rows.
- [ ] Idle CPU ≤1% and RAM measured & reported as backend RSS + WebView2 total (B2 escalation done).
- [ ] Every screen renders loading / empty / error / success / large-data / 900×600 states.
- [ ] Keyboard-only pass on all screens; all token contrast pairs ≥4.5:1; reduced motion honored.
- [ ] `COLLECTORS.md` states each collector's real coverage, including the two measured non-admin gaps.
- [ ] Zero `todo!`/`unimplemented!`/"TODO implement" in shipped paths; `TODO.md` stub list empty.

## Key Risks

| Risk | Signal | Pre-decided response |
|---|---|---|
| B1 Rust absent / 3 MSVC installs | `rustup` fails or `LNK1181` | Explicit `linker=` in `src-tauri/.cargo/config.toml` + ADR; if network blocks rustup, STOP and report |
| B2 RAM <80 MB may be impossible | measured tree >80 MB in Phase 05 | Report backend RSS and WebView2 total separately, escalate as a scope question — never redefine silently |
| B3 non-admin process blind spot | short-lived process missing from timeline | 1 s diff-poll + Application-Error correlation; documented as partial in `COLLECTORS.md` |
| TS 7.0.2 / Vite 8 toolchain gap | `bun run check` fails | Pin TypeScript 5.x (and Vite 7.x if needed), record ADR 0001 |
| Registry/service/device notifications may not fire non-admin | spike returns no callback | Fall back to the measured diff-poll cadences; document latency |
| Driver sweep 2.1 s | CPU spike on timer | Never on a short timer: start + resume + hourly + device-change wake |
| Docker untestable here | no `\\.\pipe\docker_engine` | Verify only the absent path; mark connected path untested-on-this-machine |

## Dependencies

- External: rustup + `stable-x86_64-pc-windows-msvc` (**install in Phase 01**); MSVC VC.Tools, Windows SDK
  10.0.26100, WebView2 Evergreen 151 and Bun 1.3.11 are already present on this machine.
- Ordering is strictly linear (01→21) with disjoint file ownership per phase, so no two phases ever edit the
  same file. §69 mapping: P0=01–05, P1=06–07, P2=08–11, P3=12–13, P4=14, P5=15–16, P6=17, P7=18, P8=19–21.
- Verification baseline **VG-BASE**, run from `/d/Project/.WhatChage` in Git Bash:

```bash
bun install --frozen-lockfile
bun run check
bun run test
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```
````

## PHASE FILES

### FILE: phase-01-foundation-toolchain.md

````markdown
---
phase: 1
title: "Foundation & Toolchain"
status: pending
priority: P1
effort: 8h
dependencies: []
---

## Overview

Resolve blocker B1 (Rust absent), pin the toolchain and every dependency version, scaffold
the Tauri 2 + Svelte 5 + Vite + Tailwind 4 workspace at repo root, settle decision D2
(TypeScript 7 vs 5.x), stand up structured tracing, write the nine root documents (§90), and
land the first commits. No product feature code in this phase.

## Requirements

Functional
- `rustup` + `stable-x86_64-pc-windows-msvc` installed for the current user; exact version
  pinned in `rust-toolchain.toml`; one MSVC install chosen and recorded.
- Workspace scaffolded per §55; binary name `WhatChanged.exe`, window title `What Changed`,
  package name `what-changed`, bundle identifier `com.whatchanged.app`.
- Dependency versions pinned exactly to the research table (§1 of the research report).
- `bun run {dev,build,check,test,tauri}` scripts exist and work (§67).
- Console tracing with `EnvFilter` + a reload handle (file sink lands in Phase 02 together
  with the scrubbing writer — no logging of dynamic values before then).
- Root docs written with real content: `README.md`, `PRODUCT.md`, `ARCHITECTURE.md`,
  `SECURITY.md`, `PRIVACY.md`, `ROADMAP.md`, `CONTRIBUTING.md`, `COLLECTORS.md`, `ANALYSIS.md`.
- `TODO.md` created as the visible stub register required by §87.

Non-functional
- No Node/Bun requirement for the shipped app; no CDN, no remote font (§63).
- `[lints.clippy] unwrap_used = "deny"`, `expect_used = "warn"` in `src-tauri/Cargo.toml` (§79).
- TypeScript `strict: true`, `noUncheckedIndexedAccess: true`, no `any` without a comment.
- Layering rule active from commit 1: no `use windows::` outside `platform/windows/**`.

## Architecture

Repo root `D:/Project/.WhatChage/` is the crate + frontend root. `src/` is a plain Svelte 5
SPA (no SvelteKit: a Tauri window needs no SSR/adapter/router build layer). `src-tauri/` is
one binary crate. `platform/{windows,linux,macos}` exist from day one so the Windows-only
implementation never leaks into core modules. Dev-time API rule for the whole plan: **never
write a Win32/COM/crate symbol from memory** — confirm it first with
`cargo doc -p <crate> --features <...>` or `rg` in the vendored source under
`~/.cargo/registry/src`, and record any symbol that differs from the plan in the phase report.

## Related Code Files

Paths are relative to repo root `D:/Project/.WhatChage/`.

Create
- `.gitignore` (`node_modules/`, `dist/`, `src-tauri/target/`, `*.db*`, `logs/`, `.env*`, `dist-installer/`)
- `.gitattributes` (`* text=auto eol=lf`, `*.ps1 text eol=crlf`, `*.cmd text eol=crlf`)
- `README.md`, `PRODUCT.md`, `ARCHITECTURE.md`, `SECURITY.md`, `PRIVACY.md`, `ROADMAP.md`,
  `CONTRIBUTING.md`, `COLLECTORS.md`, `ANALYSIS.md`, `TODO.md`
- `package.json`, `vite.config.ts`, `svelte.config.js`, `tsconfig.json`, `tsconfig.node.json`, `index.html`
- `src/main.ts`, `src/app/App.svelte`, `src/styles/{tokens.css,base.css,app.css}`
- `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`,
  `src-tauri/capabilities/default.json`, `src-tauri/rustfmt.toml`
- `src-tauri/src/{main.rs,lib.rs}`, `src-tauri/src/app/{mod.rs,logging.rs,state.rs}`
- `src-tauri/src/{domain,storage,analysis,redact,privacy,collectors,commands}/mod.rs` (module roots only)
- `src-tauri/src/platform/{mod.rs,windows/mod.rs,linux/mod.rs,macos/mod.rs}`
- `src-tauri/tests/layering.rs`
- `rust-toolchain.toml`, `docs/decisions/0001-typescript-version.md`,
  `docs/decisions/0002-msvc-toolchain.md`, `docs/decisions/0003-no-sveltekit.md`

Modify — none (greenfield). Delete — none.
## Implementation Steps

1. Install Rust non-elevated:
   `curl -sSfL https://win.rustup.rs/x86_64 -o "$HOME/rustup-init.exe" && "$HOME/rustup-init.exe" -y --default-toolchain stable-x86_64-pc-windows-msvc --profile default --no-modify-path`
   then `export PATH="$HOME/.cargo/bin:$PATH"` for the session (PATH already lists it).
2. Verify `rustc -V`, `cargo -V`, `rustup show`. Write `rust-toolchain.toml` with the exact
   channel string from `rustc -V` and `targets = ["x86_64-pc-windows-msvc"]`.
3. Choose MSVC: `"/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe" -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`.
   Prove linking: `cargo new --bin "$HOME/rustlink" && cargo build --manifest-path "$HOME/rustlink/Cargo.toml"`.
   On `LNK1181`/link.exe-not-found, create `src-tauri/.cargo/config.toml` with
   `[target.x86_64-pc-windows-msvc] linker = "<absolute link.exe from vswhere>"`. Record in ADR 0002. Delete the scratch crate.
4. Write `package.json` by hand with exact pins: deps — `@tauri-apps/api@2.11.1`; devDeps —
   `@tauri-apps/cli@2.11.4`, `svelte@5.56.10`, `@sveltejs/vite-plugin-svelte@7.3.0`,
   `vite@8.2.2`, `typescript@7.0.2`, `svelte-check@4.7.6`, `tailwindcss@4.3.3`,
   `@tailwindcss/vite@4.3.3`, `three@0.185.1`, `@types/three@0.185.4`, `vitest@4.1.11`,
   `@testing-library/svelte@5.4.2`, `jsdom` (resolve via `bun add -d`), `playwright@1.62.1`,
   `@tanstack/svelte-virtual@3.13.36`. `bits-ui` is **not** installed — its primitives are
   copied/adapted by hand per §4. Run `bun install`.
5. Scripts: `dev: vite`, `build: vite build`, `tauri: tauri`,
   `check: svelte-check --tsconfig ./tsconfig.json --fail-on-warnings`,
   `test: vitest run`, `test:rust: cargo test --manifest-path src-tauri/Cargo.toml`,
   `test:perf: cargo test --manifest-path src-tauri/Cargo.toml --release --test perf_events -- --ignored --nocapture`,
   `lint:rust: cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`,
   `verify: bun run check && bun run test && bun run lint:rust && bun run test:rust && bun run build`.
6. `bunx tauri init --help` first, then non-interactively:
   `bunx tauri init --app-name WhatChanged --window-title "What Changed" --frontend-dist ../dist --dev-url http://localhost:1420 --before-dev-command "bun run dev" --before-build-command "bun run build" --ci`.
   Then edit `tauri.conf.json`: `productName: "WhatChanged"`, identifier, `app.windows[0]`
   = `{ title: "What Changed", width: 1280, height: 820, minWidth: 900, minHeight: 600, resizable: true, decorations: true }`,
   `app.withGlobalTauri: false`, `app.security.csp` restrictive (copy the IPC tokens from the
   Tauri-generated default; do not invent them), `bundle.targets: ["nsis"]`.
7. `cargo add` with exact versions inside `src-tauri`: `tauri@2.11.5` (features from the
   template), `tauri-build@2.6.3` (build-dep), `tokio@1.53.1 --features rt-multi-thread,sync,time,macros`,
   `tracing`, `tracing-subscriber@0.3.23 --features env-filter,fmt`, `tracing-appender@0.2.5`,
   `thiserror@2.0.20`, `serde --features derive`, `serde_json`. For crates without a pinned
   version in research §1 (`serde`, `serde_json`, `tracing`, `regex`, `sha2`, `toml`), let
   cargo resolve and **record the resolved version** in the phase report — never type a guessed version.
8. Tailwind 4: `@import "tailwindcss";` in `src/styles/app.css`, `@tailwindcss/vite` plugin in
   `vite.config.ts`, and a first `@theme` block in `tokens.css` with the §41 token *names*
   (dark palette v0; final palette in Phase 04). Confirm `@theme`/`@custom-variant` syntax
   against `node_modules/tailwindcss` for the installed 4.3.3 before writing.
9. `app/logging.rs`: `tracing_subscriber::registry()` + `EnvFilter` (default `info`) +
   `fmt::layer()` to stderr, wrapped in `reload::Layer` so verbosity becomes settable later.
   Log only static startup lines in this phase.
10. `tests/layering.rs`: walk `src-tauri/src`, assert no file outside `platform/windows/`
    contains `use windows::`, `windows::core`, or `extern "system"`; assert `platform/{linux,macos}`
    contain no Win32 references.
11. Decision gate D2: run `bun run check`. On failure that references tsc/TS internals,
    `bun add -d typescript@5` (record the resolved 5.x), re-run, write ADR 0001 with the exact error.
    Same protocol for Vite: if `bun run dev` cannot resolve `@sveltejs/vite-plugin-svelte@7.3.0`
    against Vite 8, pin `vite@7` (resolved version recorded) and note it in ADR 0001.
12. Write the nine root docs with real content: `ARCHITECTURE.md` (layering + core/platform
    rule + IPC contract), `SECURITY.md` (§82 threat table, forbidden-IPC list, non-elevated
    model), `PRIVACY.md` (§33 collected/not-collected table + the two measured non-admin gaps),
    `COLLECTORS.md` (contract, per-collector status = *not implemented*, the §5 sidecar
    protocol sketch), `ANALYSIS.md` (§28 formula, §29 confidence vocabulary), `PRODUCT.md`
    (§1/§72 voice), `ROADMAP.md` (§69 phases + §75–§78 explicitly out of scope), `CONTRIBUTING.md`
    (VG-BASE), `README.md`.
13. Git: if `git config user.email` is unset, ask the user once and set repo-local
    `git config --local user.name/user.email`. Commits: (a) `chore: add tooling config and ignore rules`,
    (b) `docs: add product, architecture, security and privacy baselines`,
    (c) `chore: scaffold tauri 2 shell with svelte 5 and tailwind 4`.
## Todo

- [ ] rustup installed, `rust-toolchain.toml` pinned to the measured version
- [ ] MSVC install selected, link proven, ADR 0002 written
- [ ] `package.json` pinned exactly; `bun install` clean
- [ ] Tauri scaffold + `tauri.conf.json` window/CSP/bundle settings
- [ ] Cargo deps added; resolved versions of unpinned crates recorded
- [ ] Tailwind 4 wired; `tokens.css` has every §41 token name
- [ ] tracing console layer + reload handle
- [ ] `tests/layering.rs` green
- [ ] D2 decided; ADR 0001 written
- [ ] Nine root docs + `TODO.md` written
- [ ] Three commits landed on `main`

## Success Criteria

- [ ] `rustc -V` and `cargo -V` print versions; `rust-toolchain.toml` matches them exactly
- [ ] `bun run check` exits 0
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0
- [ ] `bun run tauri build --debug` produces `src-tauri/target/debug/WhatChanged.exe` and it launches
- [ ] `git log --oneline` shows 3 commits; `git status` clean
- [ ] ADR 0001 records the actual TypeScript version in use with the reason

## Verification Gate

```bash
cd /d/Project/.WhatChage
rustc -V && cargo -V && rustup show
bun install --frozen-lockfile
bun run check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
bun run tauri build --debug
git -C /d/Project/.WhatChage status --porcelain
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| rustup download blocked (proxy/AV) | `rustup-init.exe` exits non-zero, or `rustc` still missing | Retry once; then STOP and report to the user — B1 has no local workaround, and nothing downstream can start |
| Wrong linker among three MSVC installs | `error: linker 'link.exe' not found` or `LNK1181` | Pin absolute `link.exe` in `src-tauri/.cargo/config.toml`, record ADR 0002 |
| TS 7.0.2 unsupported by svelte-check | `bun run check` throws inside tsc | Pin TypeScript 5.x, ADR 0001; do not patch svelte-check |
| Vite 8 ↔ vite-plugin-svelte 7.3 mismatch | `bun run dev` fails plugin resolution/hook error | Pin Vite 7.x, ADR 0001 |
| `tauri init` flags differ in 2.11.4 | unknown-argument error | Read `bunx tauri init --help` and use the actual flags; never guess a flag |
| Git identity missing | commit fails "Please tell me who you are" | Ask the user once; set repo-local config only |
| First `cargo build` of tauri takes very long | >15 min with no output | Expected on cold cache; do not cancel; record the duration in the phase report |

## Security Considerations

- `.gitignore` blocks `.env*`, `*.db*`, `logs/`, `target/` before the first commit, so no
  secret or database can ever enter history.
- `capabilities/default.json` starts with only the core permissions the shell actually uses;
  `tauri-plugin-fs`, `-shell`, `-http`, `-sql` are never added (documented in `SECURITY.md`).
- CSP is set in Phase 01, not retrofitted: `script-src 'self'`, no `unsafe-eval`, no remote origins.
- Signing secrets are never in the repo (§66); Phase 20 documents the env-var release flow.
````

### FILE: phase-02-redaction-privacy-core.md

````markdown
---
phase: 2
title: "Redaction & Privacy Core"
status: pending
priority: P1
effort: 6h
dependencies: [1]
---

## Overview

Build the single redaction chokepoint before anything can persist or log, so §34's "before
persistence and before logging" is a structural property. Ships the pattern engine, the
`RedactedEvent` seal type, the scrubbing log writer with rolling files, the privacy policy
module, and the mandatory §60 leak suite.
## Requirements

Functional
- Key-name redaction for the §13 concept list, matched case-insensitively and
  separator-insensitively (`API_KEY`, `api-key`, `apiKey`, `x-api-key` all match `apikey`):
  `password, passwd, secret, token, apikey, api_key, authorization, cookie, session,
  private_key, client_secret`.
- Value redaction for the §60 corpus: `sk-…` style keys, `Bearer …`, JWT (`eyJ….….…`),
  AWS `AKIA…` + adjacent 40-char secret, PEM `-----BEGIN … PRIVATE KEY-----`, URL userinfo
  credentials (`postgres://u:p@h`, `mongodb://…`), session/cookie values, plus an entropy
  fallback for unknown keys whose value is ≥40 chars, base64/hex charset, Shannon entropy
  ≥3.5 bits/char.
- Known-safe key allowlist so useful diffs survive: `port, api_port, host, hostname, node_env,
  log_level, debug, timezone, version, url` **without** userinfo. §13's target output must hold:
  `API_PORT 3000 → 4000` visible, `OPENAI_API_KEY [REDACTED] → [REDACTED]`.
- `seal(RawEvent) -> RedactedEvent` is the only constructor of `RedactedEvent`; the field is
  private to `redact`. Sealing walks `before`, `after`, `metadata`, `user_friendly_summary`,
  `technical_summary`, `entity_display_name` and sets `sensitive = true` when anything matched.
- Scrubbing writer: `ScrubbingMakeWriter<W>` applies `redact_text` per line before bytes reach
  stderr or the rolling file (`tracing_appender::rolling::daily`) at
  `%LOCALAPPDATA%/WhatChanged/logs/what-changed.log`, keeping 7 files.
- `privacy/policy.rs`: `DataClass` enum with the allowed classes and a `const FORBIDDEN` list
  (keystrokes, clipboard, browser history, messages, screen capture, mic, camera, document
  contents, full command lines); `assert_allowed(class)` is called by the normalizer.

Non-functional
- Per-field input cap 8 KiB and bounded-quantifier regexes only — no unbounded backtracking.
- `redact_text` ≤ 20 µs for a 200-byte line, ≤ 2 ms for an 8 KiB field (asserted in a test).
- Patterns compiled once in a `OnceLock<RedactorSet>`.

## Architecture

`redact/patterns.rs` (pattern table + tests), `redact/engine.rs` (`Redactor`, `redact_text`,
`redact_kv`, `seal`), `redact/config_diff.rs` (redaction-aware key/value diff container used by
Phase 10), `redact/tracing_layer.rs` (writer wrapper). `RedactedEvent` lives in `redact` and is
re-exported for `storage`; `storage::repo_events::insert` accepts `&RedactedEvent` only, so no
code path can write an unsealed event. `regex` is added here (`cargo add regex`, resolved
version recorded) — justified under §80 because hand-rolled secret matching is the highest-risk
place in the product to be clever.

## Related Code Files

Paths relative to `D:/Project/.WhatChage/`.

Create
- `src-tauri/src/redact/{mod.rs,patterns.rs,engine.rs,config_diff.rs,tracing_layer.rs}`
- `src-tauri/src/privacy/{mod.rs,policy.rs,sensitive.rs}`
- `src-tauri/tests/redaction_leak.rs`
- `src-tauri/tests/fixtures/secrets.rs` (the §60 corpus as one shared const table)

Modify
- `src-tauri/src/app/logging.rs` (wrap both layers in `ScrubbingMakeWriter`, add the rolling
  file appender + `WorkerGuard` stored in app state)
- `src-tauri/src/lib.rs` (register `redact`, `privacy` modules)
- `SECURITY.md`, `PRIVACY.md` (document the chokepoint and the pattern classes)

Delete — none.

## Implementation Steps

1. `fixtures/secrets.rs`: `pub const CORPUS: &[(&str, &str)]` with label + raw value for the
   nine §60 shapes plus a realistic `.env` block and a Docker-style env list.
2. `patterns.rs`: `KEY_CONCEPTS: &[&str]`, `SAFE_KEYS: &[&str]`, and a `Vec<(PatternId, Regex)>`
   built in `OnceLock`. Normalizer helper `normalize_key(k)` lowercases and strips `-`, `_`, `.`, spaces.
3. `engine.rs`: `redact_text(&str) -> Cow<str>` (replaces each match with `[REDACTED]`),
   `redact_kv(key, value) -> (Cow<str>, RedactionReason)`, `redact_json(&mut Value)` recursing
   objects/arrays with key-aware logic, and `seal(RawEvent) -> RedactedEvent`.
4. `RedactedEvent`: `pub struct RedactedEvent(SystemEvent);` with `pub fn inner(&self) -> &SystemEvent`
   and no public constructor; doc comment states the invariant.
5. `tracing_layer.rs`: `ScrubbingWriter<W: io::Write>` buffering to newline then scrubbing;
   `ScrubbingMakeWriter<M: MakeWriter>`; verify the `MakeWriter`/`MakeWriter<'a>` signature
   against the installed `tracing-subscriber` 0.3.23 docs before writing.
6. Wire `logging.rs`: stderr layer + daily rolling file layer, both scrubbed; `EnvFilter`
   reload handle kept in `AppState` for the Phase 19 verbosity setting.
7. `policy.rs`: `DataClass`, `FORBIDDEN`, `assert_allowed`; unit test asserting every
   `DataClass` variant maps to a §33-permitted purpose string.
8. `tests/redaction_leak.rs`: for each corpus entry × each sink (sealed event JSON, metadata
   map, before/after diff, `technical_summary`, one `tracing::info!` line captured through the
   scrubbing writer) assert `!output.contains(raw)`. Add a 10k-iteration loop over generated
   secrets (deterministic LCG, no dependency) asserting zero leaks. Add the §13 fixture
   assertion (port survives, key redacted) and the perf assertions.
9. Commits: `feat: add secret redaction engine and sealed event type`,
   `feat: scrub tracing output and add rolling log files`, `test: add secret leak suite`.
## Todo

- [ ] secret corpus fixture shared by all suites
- [ ] key-concept + value patterns + safe-key allowlist
- [ ] `seal()` / `RedactedEvent` with a private field
- [ ] scrubbing writer wired to stderr + rolling daily file
- [ ] privacy policy module with FORBIDDEN list
- [ ] §60 leak suite across 5 sinks + 10k generated secrets
- [ ] `SECURITY.md` / `PRIVACY.md` updated with the chokepoint description

## Success Criteria

- [ ] `cargo test --test redaction_leak` passes with zero leaks across all corpus × sink pairs
- [ ] Grep proves `RedactedEvent(` is constructed in exactly one place (`redact::engine::seal`)
- [ ] A log line containing every corpus value produces a file with no raw value present
- [ ] `.env` fixture diff shows `API_PORT 3000 → 4000` and `OPENAI_API_KEY [REDACTED] → [REDACTED]`
- [ ] Redaction of an 8 KiB field completes in ≤2 ms (test-asserted)

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test redaction_leak -- --nocapture
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
rg -n "RedactedEvent\(" src-tauri/src | grep -v "redact/engine.rs" | wc -l   # must print 0
rg -n "use windows::" src-tauri/src --glob '!platform/windows/**' | wc -l    # must print 0
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Over-redaction destroys diff value | fixture shows `API_PORT [REDACTED]` | Extend `SAFE_KEYS`, raise the entropy floor, re-run the §13 fixture; never disable the key-concept list |
| Under-redaction of an unknown secret shape | leak test fails for a new corpus entry | Add the pattern, add the shape to the corpus permanently, re-run all sinks |
| Regex backtracking stall | redaction perf test exceeds 2 ms | Replace the offending pattern with bounded quantifiers / anchored alternation |
| `MakeWriter` signature differs in 0.3.23 | compile error in `tracing_layer.rs` | Read `cargo doc -p tracing-subscriber`; adapt — do not fall back to unscrubbed logging |
| Scrub cost on hot log paths | idle CPU rises after enabling file logs | Default level `info`, structured fields pre-redacted at the source, never log per-event bodies |

## Security Considerations

- This phase *is* the §34 control. Both sinks (DB and log) are behind it before either exists.
- Log directory is per-user `%LOCALAPPDATA%`; no world-writable path, no temp dir.
- `sensitive = true` propagates to the UI so redacted fields render as `[REDACTED]` rather
  than as an empty value that looks like data loss.
- Diagnostics export (Phase 20) reuses `redact_text` — no second redaction implementation is
  permitted anywhere (DRY is a security property here).
````

### FILE: phase-03-storage-domain-migrations.md

````markdown
---
phase: 3
title: "Storage, Domain Model & Migrations"
status: pending
priority: P1
effort: 14h
dependencies: [2]
---

## Overview

The §9 domain model, the §10 schema with every required table and index, the SQLite access
layer (WAL, one batched writer, two readers), §11 retention, §83 corruption handling, the
search-index decision, and the specta type export that makes Rust the source of truth (§81).

## Requirements

Functional
- `SystemEvent` with exactly the §9 fields; `EventCategory` (15 variants) and `EventType`
  (19 variants) enums exactly as listed; `Importance` = informational|notable|important|critical;
  `Confidence` = low|medium|high; `RelationKind` covering every §30 edge.
- One added column beyond §9: `synthetic INTEGER NOT NULL DEFAULT 0`, so §85-injected and §69
  Phase-1 generated events are permanently distinguishable from real ones (§87 honesty).
- Migrations 001–007 creating: `settings`, `schema_migrations`, `collector_status`, `entities`,
  `entity_relationships`, `events`, `snapshots`, `checkpoints`, `health_states`,
  `watched_locations`, `applications`, `analysis_sessions`, `analysis_results`, plus the search index.
- Indexes: `events(timestamp)`, `events(category,timestamp)`, `events(entity_id,timestamp)`,
  `events(event_type,timestamp)`, `events(importance,timestamp)`,
  `entity_relationships(source_id)`, `entity_relationships(target_id)`, `entities(entity_type)`.
- PRAGMAs at open: `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`,
  `foreign_keys=ON`, `wal_autocheckpoint=1000`; `auto_vacuum=INCREMENTAL` set before first schema.
- Writer: bounded `mpsc` (cap 10 000) → dedicated OS thread with its own connection; flush on
  200 events or 500 ms, one transaction per flush, prepared statements cached.
- Retention: defaults 30 d events / 7 d snapshots / 90 d important; options 7/30/90/custom;
  batched deletes (`LIMIT 5000` per transaction) + `incremental_vacuum(512)`; storage estimate
  from `page_count*page_size` + WAL size.
- Recovery: `PRAGMA quick_check` on open; `state.json` consecutive-failure counter; 3 strikes →
  `AppMode::Recovery`; pre-migration backup via `VACUUM INTO 'backups/pre-<from>-<to>.db'`.
- Bindings: `tauri-specta` export of every command/type to `src/lib/tauri/bindings.ts`, with a
  test that regenerates to a temp path and fails if the checked-in file differs.

Non-functional
- No `unwrap()` in storage paths; every failure is a `thiserror` variant with context.
- All queries are prepared + parameterized; no string-formatted SQL anywhere.
- DB path `%LOCALAPPDATA%/WhatChanged/history.db`; created with the app if absent.
## Architecture

`rusqlite` 0.40.2 with `bundled` (no system SQLite), `rusqlite_migration` 2.6.0 for the
`user_version` ladder plus our own `schema_migrations` audit rows. Synchronous SQLite on a
dedicated writer thread avoids a second async runtime and gives explicit transaction control;
reads run on `tokio::task::spawn_blocking` against a 2-connection pool guarded by a `Mutex`.
`tauri-plugin-sql` is rejected outright (it would expose SQL to the frontend, violating §35/§56).
Entity identity is a deterministic string, not a UUID: `process:<pid>:<start_ms>`,
`port:tcp:<n>`, `service:<name>`, `app:<normalized-name>`, `file:<canonical-path-hash>`,
`driver:<device-instance-id>`, `adapter:<luid>`, `container:<id>`, `image:<repo:tag>`,
`project:<canonical-path-hash>` — so collectors converge on the same entity without a lookup race.
Timestamps are UTC epoch milliseconds (`i64`); no date crate is used. Local-time windows are
computed by the caller and passed explicitly (`now_utc_ms` + `tz_offset_minutes`), which keeps
analysis fixtures deterministic and avoids an unsound local-offset dependency.

## Related Code Files

Paths relative to `D:/Project/.WhatChage/`.

Create
- `src-tauri/src/domain/{mod.rs,event.rs,entity.rs,category.rs,importance.rs,relationship.rs,checkpoint.rs,ids.rs,time.rs,error.rs}`
- `src-tauri/src/storage/{mod.rs,open.rs,pool.rs,writer.rs,migrations.rs,repo_events.rs,repo_entities.rs,repo_settings.rs,repo_status.rs,repo_snapshots.rs,retention.rs,recovery.rs,search_index.rs,estimate.rs}`
- `src-tauri/migrations/{001_core.sql,002_entities.sql,003_events.sql,004_snapshots_checkpoints.sql,005_health.sql,006_watch_applications.sql,007_analysis.sql,008_search.sql}`
- `src-tauri/tests/{migrations.rs,retention.rs,recovery.rs,bindings_up_to_date.rs}`
- `src/lib/tauri/bindings.ts` (generated), `src/lib/types/index.ts` (re-export surface)
- `docs/decisions/0004-search-index-strategy.md`, `docs/decisions/0005-sqlite-access-model.md`

Modify
- `src-tauri/Cargo.toml` (`rusqlite@0.40.2 --features bundled`, `rusqlite_migration@2.6.0`,
  `specta@1.0.5`, `tauri-specta@1.0.2 --features derive,typescript`)
- `src-tauri/src/app/state.rs` (hold `StorageHandle`, `AppMode`), `src-tauri/src/lib.rs`
- `ARCHITECTURE.md` (schema + access model section)

Delete — none.

## Implementation Steps

1. `domain/`: write the enums and `SystemEvent`/`SystemEntity`/`EntityRelationship` structs with
   `serde` + `specta::Type` derives; `RawEvent` is the pre-seal shape (same fields, unsealed).
2. `domain/ids.rs`: typed constructors listed in Architecture + a test that each is stable and
   collision-free for distinct inputs.
3. Write the eight `.sql` migrations. `events.metadata` and `snapshots.payload` are `TEXT` JSON;
   `events.before`/`after` are `TEXT` JSON or NULL; `events.entity_id` FKs `entities(id)`.
   Add `CHECK` constraints for enum columns so a bad write fails loudly.
4. `storage/open.rs`: resolve `%LOCALAPPDATA%/WhatChanged` via `tauri::Manager::path()`
   (`app_local_data_dir`), create dirs, set PRAGMAs in the documented order (auto_vacuum first),
   run `quick_check`, then migrations.
5. `migrations.rs`: `Migrations::new(vec![M::up(include_str!("../../migrations/001_core.sql")), …])`;
   before applying any migration when `user_version > 0`, run `VACUUM INTO` backup, keep 3 newest
   backups; after success insert a `schema_migrations` row (version, name, applied_at, byte length).
6. `writer.rs`: bounded channel + flush policy + `Drop` flush; on channel-full, coalesce
   duplicate `(entity_id,event_type)` within the pending batch and count `dropped_events` in a
   metric exposed later in Diagnostics (never silently discard without counting).
7. `repo_events.rs`: `insert_batch(&[RedactedEvent])`, `query_page(EventQuery) -> Page<EventRow>`
   with keyset pagination `WHERE (timestamp,id) < (?,?) ORDER BY timestamp DESC, id DESC LIMIT ?`,
   `get_details(id)`, `count_by_importance(range)`, `previous_occurrences(entity_id,event_type,range)`.
8. `search_index.rs`: probe `SELECT sqlite_compileoption_used('ENABLE_FTS5')`. If 1 → migration
   008 creates an external-content FTS5 table over `events` with insert/delete triggers. If 0 →
   create `event_terms(event_id, term)` with an index, populated by the writer from
   display name + summaries + port/service tokens. Record the branch taken in ADR 0004.
9. `retention.rs` + `estimate.rs` + `recovery.rs` per Requirements; `AppMode::{Normal,Recovery}`
   in app state, and collectors refuse to start in Recovery (enforced in Phase 08).
10. `tauri-specta` builder in `lib.rs`: export bindings when `cfg!(debug_assertions)`;
    `tests/bindings_up_to_date.rs` regenerates to `$TMP` and diffs against the committed file.
    Resolve the 1.0 builder API from `cargo doc -p tauri-specta` before writing it.
11. Tests: `migrations.rs` (empty→head; stepwise with seeded rows; re-run idempotent; backup file
    exists; 5 000-row v1 DB upgrades with identical row count and checksums), `retention.rs`
    (expiry boundaries, important-event 90 d rule, batched delete under a concurrent reader),
    `recovery.rs` (byte-corrupt a copy → `quick_check` fails → `AppMode::Recovery`, no panic;
    3-strike counter persists in `state.json`).
12. Smoke perf at 100k rows inside `cargo test` (page-1 query ≤40 ms) — the full 10k/100k/1M
    matrix belongs to Phase 06 where the generator exists.
13. Commits: `feat: add system event domain model`, `feat: add sqlite schema and migration runner`,
    `feat: add batched writer and event repository`, `feat: add retention and corruption recovery`,
    `chore: generate typescript bindings from rust types`.
## Todo

- [ ] domain enums/structs with serde + specta derives
- [ ] deterministic entity id constructors + test
- [ ] 8 migration files with indexes and CHECK constraints
- [ ] open path: dirs, PRAGMAs, quick_check, migrate
- [ ] pre-migration `VACUUM INTO` backup + rotation
- [ ] batched writer thread with bounded channel + dropped counter
- [ ] event repository incl. keyset pagination + previous occurrences
- [ ] FTS5 probe + chosen search index branch (ADR 0004)
- [ ] retention, storage estimate, recovery mode
- [ ] specta bindings + staleness test

## Success Criteria

- [ ] `cargo test --test migrations` passes all five scenarios, including zero row loss on upgrade
- [ ] `sqlite3`-free check: a debug command/test prints `journal_mode=wal`, `auto_vacuum=2`
- [ ] Insert of 50 000 events through the writer completes in one-batch multiples with ≥20k/s
- [ ] Page-1 query at 100k rows ≤40 ms (test-asserted)
- [ ] `cargo test --test recovery` proves no panic and `AppMode::Recovery` on a corrupt file
- [ ] `cargo test --test bindings_up_to_date` passes with the committed `bindings.ts`
- [ ] `repo_events::insert_batch` signature accepts only `&[RedactedEvent]` (grep-verified)

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test migrations -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --test retention
cargo test --manifest-path src-tauri/Cargo.toml --test recovery
cargo test --manifest-path src-tauri/Cargo.toml --test bindings_up_to_date
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
bun run check
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| FTS5 absent in bundled rusqlite | probe returns 0 | Take migration-008 branch B (`event_terms`); record ADR 0004; re-run the search perf budget in Phase 06 |
| `auto_vacuum` set too late (after schema) | `PRAGMA auto_vacuum` returns 0 on a fresh DB | Order is asserted in a test on a brand-new file; if it regresses, fix open order — never `VACUUM` a large DB to convert |
| WAL + antivirus file locking | `SQLITE_BUSY` beyond 5 s in tests | Raise `busy_timeout` to 15 s, keep single-writer discipline, log the retry count; document AV interaction |
| `tauri-specta` 1.0 API differs from memory | compile error in the export builder | Read `cargo doc -p tauri-specta`; if the builder cannot export commands, fall back to `specta` type-only export + a hand-written `client.ts` and record the deviation |
| Retention delete blocks the UI | timeline query latency spikes during maintenance | Batch size 5 000 with a 50 ms yield between transactions; maintenance runs on the writer thread only |
| Migration backup fills the disk | backups dir >1 GB | Keep 3 newest, delete older, skip backup when free space <2× DB size and log a warning |

## Security Considerations

- Only parameterized statements; a `tests/no_dynamic_sql.rs`-style grep in Phase 21 asserts no
  `format!` reaches `prepare(`.
- The DB lives under `%LOCALAPPDATA%` (per-user ACL); no shared-machine path.
- `events.metadata` is bounded (32 KiB) and always arrives sealed, so §82's "malicious config
  file" and "large event floods" cannot inflate rows unbounded.
- Backups inherit the same directory ACL and are pruned; they are redacted by construction
  because only sealed events were ever written.
````

### FILE: phase-04-app-shell-design-system.md

````markdown
---
phase: 4
title: "App Shell, Design System & Tray"
status: pending
priority: P1
effort: 16h
dependencies: [3]
---

## Overview

The visual identity and the whole navigational skeleton: §41 tokens across three themes, the
hand-built UI primitive set, the six-state contract, the §7 tray and hide-to-tray lifecycle,
the typed IPC surface (§56) with the contract test, and every screen present with real §72
microcopy. No Three.js yet (§90: after DOM layout is stable).

## Requirements

Functional
- Tokens (CSS variables in `@theme`): `background`, `surface-1/2/3`, `border-subtle`,
  `border-active`, `text-primary/secondary/muted`, `accent`, `success`, `warning`, `danger`,
  `info`, `shadow-sm/md/lg`, `radius-sm/md/lg/xl`, spacing scale (4→64), motion durations
  (`--motion-fast 120ms`, `--motion-base 200ms`, `--motion-slow 320ms`), easings
  (`--ease-out-quart`, `--ease-spring`). Dark / Light / System all implemented; dark is default.
- Primitives in `src/components/ui/`, adapted from shadcn-svelte/Bits UI *concepts* (no
  dependency): Button, IconButton, Card, Badge, Input, Select, Switch, Tabs, Tooltip, Dialog,
  Drawer, Skeleton, EmptyState, ErrorState, Toast, Kbd, ScrollArea, SegmentedControl, Table.
- `ScreenState<T>` union + `<StateSwitch>`: `loading | empty | error | success | large` and a
  `useCompact()` derived store for the ≤1000 px "small window" layout — six states everywhere.
- Shell: in-app header (native window decorations retained — ADR 0006), sidebar nav (Home,
  Timeline, Investigations, Checkpoints, System Map, Search, Settings, Diagnostics, and Dev when
  developer mode is on), status pill `● Active` / `Paused`, window state persisted.
- Tray with exactly §7's entries: Open What Changed / What changed today? / Create checkpoint /
  Pause monitoring / Resume monitoring / Settings / Quit. Close → hide to tray; single instance
  focuses the existing window.
- IPC surface registered and typed (real data where it exists; counts are honest zeros until
  Phase 06): `get_dashboard_summary, get_events, get_event_details, search_events,
  create_checkpoint, rename_checkpoint, delete_checkpoint, list_checkpoints, compare_checkpoint,
  run_investigation, get_collector_status, set_collector_enabled, get_settings, update_settings,
  get_storage_usage, get_diagnostics, export_diagnostics, repair_database, reset_local_history,
  add_watched_location, remove_watched_location, list_watched_locations, add_project,
  remove_project, list_projects, set_monitoring_paused, set_autostart, get_onboarding_state,
  complete_onboarding, check_for_updates, dev_inject_event`.
- Events: `system-event-added`, `collector-status-changed`, `analysis-completed`,
  `monitoring-state-changed` declared with typed payloads.
Non-functional
- 900×600 minimum is usable without horizontal scrolling; 1200–1600 px is the tuned target (§71).
- Keyboard: visible focus ring on every interactive element, `Tab` order matches visual order,
  `Esc` closes overlays, `Ctrl+K` opens search, sidebar reachable by keyboard (§62).
- `prefers-reduced-motion` disables transitions via a single token override, not per-component code.
- No `any` in `src/`; all IPC calls go through the generated bindings (§81).
- Explicitly tracked temporary limitation in `TODO.md`: Pause/Resume persists state and updates
  the UI, but nothing is collecting yet — Phase 08 makes collectors honor it. Not marked done here.

## Architecture

`src/lib/tauri/client.ts` wraps the generated `bindings.ts` with one `invokeSafe` helper that maps
Rust `AppError` into a typed `Result`, so no component ever writes a raw `invoke` string.
`src/lib/stores/*.svelte.ts` hold runes-based state (`$state`, `$derived`) — one store per concern,
no duplicated derived data (§79). Routing is a 60-line typed store (`router.svelte.ts`) with a
`Route` union; no router dependency. All user-visible strings live in `src/lib/copy/strings.ts`
(the single §55 deviation) and are snapshot-tested against §72–§74, so voice cannot drift.
Tray/window/lifecycle logic lives in `src-tauri/src/app/{tray.rs,window.rs,lifecycle.rs}`.

## Related Code Files

Paths relative to `D:/Project/.WhatChage/`.

Create
- `src/app/{App.svelte,router.svelte.ts,routes.ts}`, `src/app/shell/{AppShell.svelte,Sidebar.svelte,HeaderBar.svelte,StatusPill.svelte,CommandHint.svelte}`
- `src/components/ui/*.svelte` (19 primitives listed above) + `src/components/ui/StateSwitch.svelte`
- `src/features/home/HomeScreen.svelte`, `src/features/timeline/TimelineScreen.svelte`,
  `src/features/investigation/InvestigationScreen.svelte`,
  `src/features/checkpoints/CheckpointsScreen.svelte`, `src/features/search/SearchScreen.svelte`,
  `src/features/settings/SettingsScreen.svelte`, `src/features/systemmap/SystemMapScreen.svelte`,
  `src/features/diagnostics/DiagnosticsScreen.svelte`, `src/features/dev/DevPanelScreen.svelte`
- `src/lib/{tauri/client.ts,tauri/events.ts,copy/strings.ts,copy/categories.ts,design/motion.ts,formatting/{time.ts,bytes.ts,importance.ts,diff.ts}}`
- `src/lib/stores/{theme.svelte.ts,monitoring.svelte.ts,settings.svelte.ts,toasts.svelte.ts,router.svelte.ts}`
- `src/styles/tokens.css` (final palette), `src/styles/base.css`
- `src-tauri/src/app/{tray.rs,window.rs,lifecycle.rs}`
- `src-tauri/src/commands/{mod.rs,dashboard.rs,events.rs,search.rs,checkpoints.rs,investigation.rs,collectors.rs,settings.rs,diagnostics.rs,watch.rs,projects.rs,dev.rs,updates.rs}`
- `src-tauri/tests/ipc_contract.rs`
- `src/**/*.states.test.ts` (one per screen), `docs/decisions/0006-native-window-decorations.md`
- `docs/ui-state-matrix.md`

Modify
- `src-tauri/Cargo.toml` (`tauri-plugin-single-instance@2.4.3`, `tauri-plugin-window-state@2.4.1`,
  `tauri-plugin-autostart@2.5.1`, `tauri-plugin-dialog@2.7.2`, `tauri-plugin-notification@2.3.3`)
- `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/lib.rs`
- `src/styles/app.css`, `TODO.md`, `PRODUCT.md` (voice section)

Delete — none.

## Implementation Steps

1. Finalize `tokens.css`: `@theme` block with every token; `[data-theme="light"]` override block;
   `@custom-variant dark` bound to `[data-theme="dark"]` (verify syntax against installed 4.3.3).
   Measure every text/background pair with a small script and record the ratios in `docs/ui-state-matrix.md`.
2. `strings.ts`: exact §72 copy (`"Everything looks normal."`, `"2 recent changes may need attention."`,
   `"What stopped working?"`, `"Example: Discord stopped detecting my microphone this morning"`,
   `"Important changes"`, `"Investigate"`, `"Your history stays on this PC."`), plus the §43
   count row, §45/§73/§74 fragment templates, and one empty/error/loading string per screen.
3. Build the 19 primitives, each with a `*.test.ts` covering keyboard behaviour and disabled state.
   Dialog/Drawer implement focus trap + restore + `aria-modal` + labelled title.
4. `StateSwitch.svelte` + `ScreenState` type; each screen file renders all six states from a
   `?state=` dev override so `*.states.test.ts` can mount them deterministically.
5. `AppShell` + `Sidebar` + `HeaderBar` + `StatusPill`; compact layout at ≤1000 px collapses the
   sidebar to icons with tooltips; test at 900×600 via a Vitest viewport-driven layout assertion.
6. Home screen to the §43 wireframe with the real copy and a placeholder area reserved for the
   ambient scene (empty div now, filled in Phase 17).
7. Rust: `tray.rs` builds the icon + menu with the seven exact labels using `tauri::tray::TrayIconBuilder`
   and `tauri::menu::{MenuBuilder,MenuItem}`; `window.rs` handles `WindowEvent::CloseRequested`
   → `api.prevent_close()` + `window.hide()`; `lifecycle.rs` wires single-instance focus and
   `monitoring-state-changed` emission.
8. Implement all commands with typed request/response structs; unimplemented-domain commands return
   real empty data (`Page::empty()`, zero counts) rather than errors, and each carries a doc comment
   naming the phase that fills it. `dev_inject_event` is gated on `settings.developer_mode`.
9. `tests/ipc_contract.rs`: assert the registered handler list equals the expected 31-name set, and
   assert no name matches `(?i)(shell|exec|run_|read_file|write_file|sql|query_raw|eval)`.
10. Regenerate `bindings.ts`; wire `client.ts` + `events.ts` (typed `listen` wrappers with cleanup).
11. Manual checklist recorded in the phase report: tray menu items all act, close→tray, reopen from
    tray, theme switches instantly, System follows OS, window position restored, 900×600 usable.
12. Commits: `feat: add design tokens and ui primitives`, `feat: add app shell and routing`,
    `feat: add tray, window lifecycle and single instance`, `feat: add typed ipc command surface`,
    `test: add screen state matrix tests`.
## Todo

- [ ] tokens for all three themes + contrast table recorded
- [ ] 19 primitives with keyboard tests
- [ ] StateSwitch + six states on all nine screens
- [ ] shell, sidebar, status pill, compact layout
- [ ] tray with the seven §7 entries, close→tray, single instance
- [ ] 31 typed commands + 4 typed events + contract test
- [ ] `strings.ts` with exact §72 copy + snapshot test
- [ ] TODO.md records the pause/resume temporary limitation

## Success Criteria

- [ ] `bun run test` green including nine `*.states.test.ts` files (6 states each)
- [ ] `cargo test --test ipc_contract` green; forbidden-name regex matches nothing
- [ ] Theme switch changes every surface with no unstyled flash; System follows the OS live
- [ ] Closing the window leaves the process running and the tray icon present
- [ ] All seven tray entries perform their action (manual checklist attached to the report)
- [ ] At 900×600 no screen scrolls horizontally and no text is clipped
- [ ] Contrast table shows every text/surface pair ≥4.5:1

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
bun run tauri dev    # manual checklist: tray, close-to-tray, theme, 900x600, Ctrl+K
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Custom titlebar breaks Windows snap/a11y | dragging or Win+Arrow misbehaves | Decision is already native decorations (ADR 0006); do not revisit without a user request |
| Tray icon invisible on Win11 | no icon after launch | Confirm `bunx tauri icon` generated all sizes; check tray overflow; document that Win11 hides new icons by default |
| Toasts require an installed app on Windows | notification never appears in dev | Verify in the Phase 05 installed build; do not add a second notification mechanism |
| Screen state override leaks into production | `?state=` works in a release build | Gate the override behind `import.meta.env.DEV`; assert in a test |
| Generic-dashboard look creeps in (§70) | screens are plain cards on black | Density/elevation review against §40 before closing the phase; tokens only, no ad-hoc colors |
| Empty commands look like stubs (§87) | reviewer cannot tell scaffold from feature | Every placeholder command has a doc comment naming its phase and a `TODO.md` line |

## Security Considerations

- `capabilities/default.json` grants only the core window/event permissions the shell uses plus
  the five installed plugins' minimal permission sets; after the first build, inspect
  `src-tauri/gen/schemas/*.json` and delete anything unused.
- Every command validates input in Rust (ranges clamped, ids checked, strings length-capped) —
  the frontend is untrusted (§35).
- `dev_inject_event` is runtime-gated on `developer_mode` (default false) and stamps
  `synthetic = 1`, so injected data can never masquerade as observed data.
- No plugin with filesystem/shell/http reach is installed; `SECURITY.md` lists them as banned.
````

### FILE: phase-05-packaging-budget-baseline.md

````markdown
---
phase: 5
title: "Packaging & Budget Baseline"
status: pending
priority: P1
effort: 6h
dependencies: [4]
---

## Overview

Close §69's Phase 0 by proving a release installer builds and installs non-elevated, then take
the first real performance measurements and resolve blocker B2 (the <80 MB RAM target) with data
instead of an assertion.

## Requirements

Functional
- `bun run tauri build` produces an NSIS installer; a post-build script copies it to
  `dist-installer/WhatChanged-Setup-x64.exe` (§65 naming).
- Per-user install (`bundle.windows.nsis.installMode: "currentUser"`), Start Menu shortcut,
  optional desktop shortcut — verified against the installed CLI's config schema before editing.
- WebView2 handling declared explicitly (`bundle.windows.webviewInstallMode`), with the chosen
  mode recorded in ADR 0007 including what happens on a machine without WebView2.
- `scripts/measure-memory.ps1`: reports (M1) `WhatChanged.exe` private working set, (M2) the sum
  of M1 plus every `msedgewebview2.exe` whose parent chain reaches our PID, and (M3) idle CPU
  percent averaged over 120 s normalized by logical core count.
- Measurements written to `docs/measurements/phase-05-baseline.md` with machine context.
- Install → launch → create/verify DB → uninstall → reinstall cycle proves the event DB survives.

Non-functional
- Release binary built with `[profile.release] lto = "thin", codegen-units = 1, panic = "abort"?`
  — `panic = "abort"` is **not** used because collector isolation relies on `catch_unwind`;
  use `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`.
- Installer runs without UAC; app starts non-elevated (§36).

## Architecture

Packaging is configuration plus two scripts; no product code. `scripts/` holds `.ps1`/`.ts`
only (Git Bash heredocs are unreliable here, so scripts are real files invoked by path).
B2 is treated as a measurement with an escalation path, not as an acceptance criterion that can
be quietly failed: M1 is the number this plan commits to (<80 MB backend RSS); M2 is reported to
the user as-is, and if M2 >80 MB the phase output includes a written scope question offering
(a) accept M2 as the documented figure, (b) keep <80 MB as a stretch for backend-only, or
(c) revisit the UI shell — the plan does not choose for the user.
## Related Code Files

Paths relative to `D:/Project/.WhatChage/`.

Create
- `scripts/measure-memory.ps1`, `scripts/measure-idle-cpu.ps1`, `scripts/rename-installer.ts`
- `docs/measurements/phase-05-baseline.md`, `docs/decisions/0007-installer-and-webview2.md`
- `docs/release-checklist.md`

Modify
- `src-tauri/tauri.conf.json` (bundle section), `src-tauri/Cargo.toml` (`[profile.release]`)
- `package.json` (`bundle: "tauri build && bun scripts/rename-installer.ts"`)
- `README.md` (install/run instructions), `TODO.md`

Delete — none.

## Implementation Steps

1. Read the config schema referenced by `$schema` in `tauri.conf.json` (under
   `node_modules/@tauri-apps/cli/`) and confirm the exact keys for `bundle.targets`,
   `bundle.windows.nsis.installMode`, `bundle.windows.webviewInstallMode`,
   `bundle.windows.nsis.installerIcon`, and shortcut options. Only then edit the config.
2. Set `[profile.release] lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`; keep unwinding.
3. `bun run tauri build`; record wall-clock, exe size, installer size.
4. `scripts/rename-installer.ts` (Bun): find the produced `*-setup.exe` under
   `src-tauri/target/release/bundle/nsis/`, copy to `dist-installer/WhatChanged-Setup-x64.exe`,
   print both paths; fail loudly if zero or multiple matches.
5. Install from the installer as the current user. Verify: Start Menu entry launches; app starts
   non-elevated (`whoami /groups` shows no elevation; task manager shows no shield); DB created at
   `%LOCALAPPDATA%/WhatChanged/history.db`.
6. `scripts/measure-memory.ps1`: locate our PID by process name, walk `Win32_Process.ParentProcessId`
   to collect WebView2 children, sum `Get-Counter '\Process(<name>)\Working Set - Private'`; print
   M1/M2 in MiB. Run twice: 60 s after launch (idle, window open) and after hiding to tray.
7. `scripts/measure-idle-cpu.ps1`: sample `\Process(*)\% Processor Time` for our tree every 2 s for
   120 s with the window hidden; report average / total-core-normalized value (§37 target 0–1%).
8. Write `docs/measurements/phase-05-baseline.md`: M1, M2, M3 for window-open and tray-hidden,
   installer/exe sizes, WebView2 runtime version, and the B2 verdict + scope question if M2 >80 MB.
9. Data-preservation cycle: seed 100 synthetic events (dev panel), uninstall, reinstall, confirm the
   DB and its rows still exist; record the result (this is the §64/§65 guarantee's first proof).
10. Commits: `chore: configure nsis bundle and release profile`,
    `chore: add memory and idle cpu measurement scripts`, `docs: record phase 0 baseline measurements`.

## Todo

- [ ] bundle config verified against the shipped schema
- [ ] release build + installer produced and renamed
- [ ] per-user install verified non-elevated
- [ ] M1/M2/M3 measured in both window states and recorded
- [ ] B2 verdict written; scope question raised if needed
- [ ] uninstall/reinstall preserves the database
- [ ] release checklist drafted

## Success Criteria

- [ ] `dist-installer/WhatChanged-Setup-x64.exe` exists and installs with no UAC prompt
- [ ] Installed app launches, shows the shell, creates the DB, and hides to tray
- [ ] `docs/measurements/phase-05-baseline.md` contains M1, M2, M3 with timestamps
- [ ] M1 (backend private working set) < 80 MB
- [ ] M3 idle CPU ≤1% averaged over 120 s with the window hidden
- [ ] Reinstall preserves all seeded rows (row count identical)
- [ ] §69 Phase 0 acceptance list is fully checked in the phase report

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run tauri build
bun scripts/rename-installer.ts
ls -l dist-installer/WhatChanged-Setup-x64.exe
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/measure-idle-cpu.ps1
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| B2: WebView2 pushes total RAM over 80 MB | M2 >80 MB in the recorded measurement | Report M1 and M2 separately in the phase output and raise the scope question verbatim; do not redefine the metric, do not silently pass |
| NSIS option names differ from the schema | `tauri build` config validation error | Use the schema's actual keys; record the difference in ADR 0007 |
| SmartScreen blocks the unsigned installer | "Windows protected your PC" dialog | Expected without signing; document in `SECURITY.md`, keep production signing for Phase 20 |
| Release build fails only in release mode | link or LTO error absent in debug | Drop to `lto = false` first, then `codegen-units = 16`; record which setting was responsible |
| WebView2 absent on a target machine | app window blank on a clean machine | The chosen `webviewInstallMode` is documented in ADR 0007 with its network implication; note it in README |
| Idle CPU >1% with no collectors yet | M3 >1% | Investigate the WebView (animation/timer) first; suspend the render loop when hidden before adding collectors in Phase 08 |

## Security Considerations

- Per-user install means no service, no admin, no writable `Program Files` target (§36).
- The installer is unsigned in dev; signing configuration and secret handling are Phase 20 and
  documented as env-var-only (§66) — no key material ever enters the repo.
- The measurement scripts read only our own process tree; they collect no user data and write only
  to `docs/measurements/`.
````

### FILE: phase-06-event-engine-synthetic-feed.md

````markdown
---
phase: 6
title: "Event Engine & Synthetic Feed"
status: pending
priority: P1
effort: 12h
dependencies: [5]
---

## Overview

§69 Phase 1 backend: real query/pagination/grouping/summary APIs, throttled live event delivery,
the deterministic synthetic event generator that makes UI work possible before collectors exist
(explicitly flagged `synthetic = 1`), and the full §61 performance matrix at 10k/100k/1M events.

## Requirements

Functional
- `get_events(EventQuery)` — range (`from_ms`,`to_ms`), categories, event types, importance floor,
  entity id, text filter, `cursor: Option<(i64,i64)>`, `limit ≤ 200`; returns
  `Page<EventSummary>{ items, next_cursor, total_estimate }`.
- `get_event_group(EventQuery)` — server-side clustering: key = `(source_collector, category,
  entity_root, 60 s bucket)`; returns `EventGroup{ key, label, count, representative, importance_max,
  first_ms, last_ms }` so the frontend never receives 500 raw rows (§44).
- `get_dashboard_summary(range)` — total / notable / important / critical counts + the top N
  important events + monitoring state + collector health rollup (§43's `37 changes 2 important 0 critical`).
- `search_events(SearchQuery)` — over the Phase-03 index; matches app names, descriptions, file
  names, ports, services, drivers, containers (checkpoint labels join in Phase 14).
- `get_event_details(id)` — full event + related events + affected entities + previous occurrences.
- Live delivery: a coalescer emits `system-event-added` at most every 250 ms with
  `{ added_count, latest: [EventSummary; ≤20], dropped_count }`.
- Synthetic generator: seeded, deterministic, covering all 15 categories with realistic names and
  before/after values; bulk mode (`seed_events(n)`, used by tests) and drip mode (developer panel).
  Every generated row has `synthetic = 1`.
Non-functional (asserted budgets, release mode, this machine)
- Page-1 query: ≤25 ms @10k, ≤40 ms @100k, ≤60 ms @1M.
- Filtered (category+importance) page: ≤60 / ≤120 / ≤250 ms.
- Group query for one day: ≤80 / ≤200 / ≤500 ms.
- Search single term: ≤150 / ≤300 / ≤700 ms.
- Batched insert throughput ≥20 000 events/s; 1M seed ≤120 s.
- Burst: 5 000 events in 5 s never blocks a concurrent read >200 ms; overflow is counted, not silent.
- Retention pass over 1M rows ≤60 s while a reader keeps querying.
- A failing budget is optimized or escalated with the measurement — never relaxed silently.

## Architecture

Query construction is one builder in `storage/query.rs` producing a parameterized statement plus a
bound-parameter vector; no query strings are assembled anywhere else. Clustering happens in SQL
(`GROUP BY source_collector, category, entity_root, timestamp/60000`) so row volume never crosses
the IPC boundary. The coalescer lives in `app/live.rs`: a Tokio task holding a 250 ms interval and a
small ring buffer, emitting via `AppHandle::emit`. The synthetic generator lives in
`collectors/synthetic.rs` and implements the same `RawEvent` shape as real collectors, but is
registered as a *source* rather than a `Collector`, so it can never be mistaken for one in
`collector_status`.

## Related Code Files

Paths relative to `D:/Project/.WhatChage/`.

Create
- `src-tauri/src/storage/query.rs`, `src-tauri/src/storage/repo_groups.rs`
- `src-tauri/src/app/live.rs`
- `src-tauri/src/collectors/synthetic.rs`
- `src-tauri/tests/perf_events.rs`, `src-tauri/tests/query_semantics.rs`, `src-tauri/tests/live_coalescer.rs`
- `docs/measurements/phase-06-perf-matrix.md`

Modify
- `src-tauri/src/commands/{events.rs,dashboard.rs,search.rs,dev.rs}` (real implementations)
- `src-tauri/src/storage/{repo_events.rs,search_index.rs}` (group + search queries)
- `src-tauri/src/lib.rs` (spawn the coalescer), `src/lib/tauri/bindings.ts` (regenerate)
- `TODO.md` (remove the "counts are zeros" placeholder line)

Delete — none.

## Implementation Steps

1. `storage/query.rs`: `EventQuery` → `(sql, params)` with keyset pagination, an
   `importance >= ?` floor, `IN (…)` lists built from bound parameters only, and a hard `LIMIT` clamp.
2. `repo_groups.rs`: the grouping query + a `representative` pick (highest importance, then latest).
   `entity_root` is a stored column on `events` derived by the normalizer (`app:discord`, `port:tcp:3000`).
3. `commands/events.rs` + `dashboard.rs` + `search.rs`: thin, validating wrappers; clamp `limit`,
   reject inverted ranges, cap `text` at 128 chars.
4. `app/live.rs`: coalescer with a 250 ms tick; drops nothing silently — `dropped_count` accumulates
   from the writer's overflow counter and is surfaced in the payload and in Diagnostics.
5. `collectors/synthetic.rs`: deterministic LCG seeded by a constant; templates per category using
   realistic values (`NVIDIA graphics driver updated 581.29 → 582.11`, `Port 3000 stopped listening`,
   `Windows Audio restarted`, `.env API_PORT 3000 → 4000`), a configurable time spread, and
   `synthetic = 1` on every row. Bulk seeding writes directly through `insert_batch` in chunks of 5 000.
6. `tests/perf_events.rs` (`#[ignore]`, release): three tiers (10k/100k/1M), each measuring the six
   budgets with `Instant`, printing a markdown table, asserting the thresholds, and recording DB size.
7. Burst test: spawn a producer pushing 5 000 events in 5 s while a reader loops page-1 queries;
   assert max read latency ≤200 ms and `dropped_count` is reported when the channel saturates.
8. `tests/query_semantics.rs`: pagination has no duplicates/gaps across 20 pages with equal
   timestamps; importance floor; category filter; range boundaries are inclusive/exclusive as documented.
9. `tests/live_coalescer.rs`: 1 000 events in 1 s produce ≤5 emissions with correct `added_count`.
10. Paste the perf table into `docs/measurements/phase-06-perf-matrix.md` with the machine context.
11. Commits: `feat: add event query builder and pagination`, `feat: add server-side event grouping`,
    `feat: add throttled live event channel`, `feat: add deterministic synthetic event source`,
    `test: add 10k/100k/1M performance matrix`.

## Todo

- [ ] query builder + keyset pagination
- [ ] grouping query + `entity_root` column populated
- [ ] dashboard summary + search commands real
- [ ] live coalescer with dropped-count honesty
- [ ] synthetic generator (bulk + drip), all rows flagged
- [ ] perf matrix at three tiers recorded
- [ ] burst + pagination + coalescer tests

## Success Criteria

- [ ] `bun run test:perf` prints the matrix and every asserted budget passes
- [ ] 1M-row DB size recorded; retention pass ≤60 s with a concurrent reader
- [ ] Pagination test proves no duplicate or skipped row across 20 pages
- [ ] `system-event-added` fires ≤4×/s under a 1 000 events/s load
- [ ] `get_dashboard_summary` returns the §43 numbers from real rows
- [ ] Every synthetic row has `synthetic = 1` (SQL-verified in the test)

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test query_semantics
cargo test --manifest-path src-tauri/Cargo.toml --test live_coalescer
bun run test:perf
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| 1M-row search misses the budget | search tier-3 assertion fails | If on the `event_terms` branch, add a covering index on `(term, event_id)`; if still failing, default search to the retention window and document the scope of a search |
| Keyset pagination breaks on ties | duplicate rows across pages | Composite cursor `(timestamp, id)` is mandatory — fix the comparator, never switch to OFFSET |
| Seeding 1M takes far longer than 120 s | seed step exceeds the budget | Raise batch size to 20 000, `PRAGMA synchronous=OFF` **for the test DB only**, and record the production setting is unchanged |
| Coalescer starves under sustained load | UI event lag >2 s | Increase the window to 500 ms and switch to count-only payloads above 200 events/s |
| Synthetic data mistaken for real | timeline shows plausible fake events with no marker | UI renders a persistent "Synthetic" badge for `synthetic = 1` rows and Diagnostics shows the synthetic count |
| Group query hides an important event | important event only visible after expanding | `importance_max` is part of the group row and any `important`/`critical` member is always rendered ungrouped |

## Security Considerations

- Query parameters are bound; the `text` filter never becomes SQL, only a bound `LIKE`/FTS argument.
- `dev_inject_event` remains gated by `developer_mode`; bulk seeding is test-only code paths.
- Event floods (§82) are bounded by the writer channel cap plus a per-collector rate ceiling added
  in Phase 08; the overflow counter makes suppression observable instead of invisible.
````

### FILE: phase-07-timeline-details-search.md

````markdown
---
phase: 7
title: "Timeline, Details & Search"
status: pending
priority: P1
effort: 16h
dependencies: [6]
---

## Overview

§69 Phase 1 frontend: the polished virtualized Timeline against synthetic data, the §46 event
details drawer with §51 progressive disclosure, and the §52 global search — each with all six
states and real microcopy. This is where the product's visual identity has to become undeniable.

## Requirements

Functional
- Ranges: Today / Yesterday / Last 7 days / Custom (date-time range picker, local time computed in
  the frontend and sent as UTC ms).
- Filters exactly §44: All / Apps / Windows / Network / Drivers / Files / Development / Docker /
  Important only, as a segmented control + overflow; filter state is URL-free but persisted per session.
- Row shape per §44: time, human title, `before → after` line, an optional relation line
  (`34 minutes before first game crash`), `[ View details ]`.
- Clustering: groups render as `12 Docker-related changes  [Expand]`; expanding fetches the group's
  rows lazily; `important`/`critical` members always render individually.
- Details drawer per §46: human explanation, timestamp, importance, confidence, before, after,
  related events, affected entities, source collector, technical details (raw ids, paths, PID,
  registry path, port, exe, hash), previous occurrences ("seen 3 times in the last 30 days").
- Progressive disclosure (§51): "Show technical details" reveals the raw block; state remembered.
- Search screen: single input (`Ctrl+K` from anywhere), debounce 150 ms, grouped results by kind,
  example-query chips (`Discord`, `Java`, `25565`, `NVIDIA`, `nginx`, `Docker`, `DNS`).
- Live insertion: new events animate in at the top when the range includes "now"; when the user has
  scrolled away, show a `3 new changes` pill instead of moving content.
Non-functional
- ≤60 mounted DOM rows at any scroll offset with 100 000 events; verified by counting nodes.
- Scroll stays ≥50 fps at 100k (frame-time sampling in the harness, not a vibe check).
- Every interactive element reachable by keyboard; drawer traps focus and restores it on close (§62).
- All strings come from `src/lib/copy/strings.ts`; no literal user-facing text in components.
- 900×600 minimum: timeline, filters, and drawer all usable without horizontal scroll (§71).

## Architecture

Virtualization: attempt `@tanstack/svelte-virtual@3.13.36` first (research assumption #10 — Svelte 5
runes compatibility is unverified). The spike is a timeboxed 60 minutes in step 1; the fallback is
`src/lib/components/timeline/windowed-list.svelte`, a hand-rolled fixed-height window (measured row
height, `translateY` spacer, overscan 8). Row height is fixed per row *variant* (single vs grouped),
so both paths use a variant-height table rather than dynamic measurement.

Data flow: `timelineStore` (runes) owns `{ range, filters, cursor, rows, state }`. It calls
`get_events` through the typed bindings, appends pages on `loadMore`, and reacts to
`system-event-added` by prepending when `range` includes now and `scrollTop < 40`. The drawer calls
`get_event_details(id)` on open — the list payload deliberately does not carry technical details, so
the wire stays small.

## Related Code Files

Create
- `src/lib/stores/timeline-store.svelte.ts`, `src/lib/stores/search-store.svelte.ts`
- `src/lib/components/timeline/{timeline-view.svelte,event-row.svelte,event-group-row.svelte,range-picker.svelte,filter-bar.svelte,new-events-pill.svelte,windowed-list.svelte}`
- `src/lib/components/details/{event-drawer.svelte,technical-details.svelte,related-events.svelte,previous-occurrences.svelte}`
- `src/lib/components/search/{search-panel.svelte,result-group.svelte,query-chips.svelte}`
- `src/routes/{timeline.svelte,search.svelte}` (replacing the phase-04 placeholders)
- `src/lib/utils/{time-format.ts,relative-time.ts}`
- `tests/timeline-virtualization.test.ts`, `tests/timeline-store.test.ts`, `tests/copy-coverage.test.ts`

Modify
- `src/lib/copy/strings.ts` (timeline/search/drawer strings), `src/app.css` (row/drawer tokens only)
- `src/lib/router.ts` (register `Ctrl+K`), `TODO.md`

Delete — none.

## Implementation Steps

1. Spike (60 min hard stop): render 100 000 synthetic rows with `@tanstack/svelte-virtual` in a
   scratch route. Success = correct row recycling under Svelte 5 runes + ≤60 mounted nodes. Record
   the outcome in `docs/adr/0008-virtualization-choice.md`; on failure, delete the dependency from
   `package.json` and implement `windowed-list.svelte`.
2. `time-format.ts` / `relative-time.ts`: `HH:MM`, `HH:MM:SS`, `Yesterday 14:32`, and the
   §44 relation phrasing (`34 minutes before first game crash`) — pure functions, unit-tested,
   no locale library.
3. `filter-bar.svelte` + `range-picker.svelte`: the §44 filter set; custom range validates
   start < end and clamps to the retention window with the copy "No data before <date>".
4. `event-row.svelte`: time, title, `before → after` (truncated with a tooltip carrying the full
   value), relation line, `[ View details ]`. `event-group-row.svelte`: `N <category>-related changes`
   plus `[Expand]`, expanding to an indented child list fetched via `get_event_group`.
5. `timeline-view.svelte`: composes the chosen list implementation, sentinel-driven `loadMore`,
   and the six `ScreenState` branches — Loading skeleton rows, Empty ("No changes recorded yet."),
   Error with `[ Retry ]`, Partial (a banner naming which collector is degraded), Ready, Dense
   (grouping switched on above 200 rows in view range).
6. `new-events-pill.svelte`: appears when events arrive while scrolled; clicking scrolls to top and
   flushes the buffer.
7. `event-drawer.svelte`: right-side drawer, `Escape` closes, focus trap, sectioned per §46,
   `technical-details.svelte` behind the §51 "Show technical details" toggle persisted in settings.
8. `previous-occurrences.svelte`: renders "This has happened 3 times in the last 30 days." from the
   command's occurrence rows, and nothing at all when the count is 1 (no empty section noise).
9. Search: `search-panel.svelte` with the `Ctrl+K` global handler (ignored while an input is focused
   inside the drawer), 150 ms debounce, grouped results, chips, and the §52 empty state
   "No changes match that. Try a shorter search term."
10. `tests/timeline-virtualization.test.ts`: mount with 100 000 mocked rows, assert mounted-row
    count ≤60 at three scroll offsets and after a range change.
11. `tests/copy-coverage.test.ts`: scan `src/lib/components/**` for user-visible literals
    (regex over text nodes and `title=`/`aria-label=`) and fail on anything not sourced from `strings.ts`.
12. Commits: `feat: add virtualized timeline with range and category filters`,
    `feat: add event details drawer with progressive disclosure`,
    `feat: add global search panel`, `test: add virtualization and copy coverage tests`.

## Todo

- [ ] virtualization spike decided + ADR 0008
- [ ] time/relative formatting utilities + tests
- [ ] filter bar + range picker with validation
- [ ] event row, group row, expand
- [ ] six states on timeline
- [ ] details drawer + technical disclosure + previous occurrences
- [ ] search panel + `Ctrl+K` + chips + empty state
- [ ] virtualization and copy-coverage tests

## Success Criteria

- [ ] With 100 000 synthetic events, mounted rows ≤60 and scrolling holds ≥50 fps (recorded)
- [ ] All six states render on Timeline, Search, and the drawer (state-matrix screenshot list in TODO-free docs)
- [ ] Keyboard-only: reach any row, open the drawer, toggle technical details, close, focus restored
- [ ] Zero user-visible string literals outside `strings.ts` (test-enforced)
- [ ] At 900×600 nothing clips or scrolls horizontally
- [ ] Grouped rows never hide an `important`/`critical` event

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test
bun run tauri dev   # manual: 6-state sweep, Ctrl+K, keyboard-only drawer, 900x600 resize
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `@tanstack/svelte-virtual` incompatible with runes | spike shows duplicated/blank rows or reactivity loss | Drop the dependency at the 60-minute mark and ship `windowed-list.svelte`; record in ADR 0008 |
| Variable row heights break the window | rows overlap or the scrollbar jumps | Fixed height per row variant is mandatory; wrap long values with truncation instead of growing rows |
| Live prepending fights the user's scroll | content shifts under the cursor | Prepend only when `scrollTop < 40`; otherwise the pill |
| Drawer fetch latency feels broken | visible blank drawer | Drawer opens with skeleton sections instantly and fills in; never gate the open on the request |
| Search debounce feels laggy on short queries | user types and waits | 150 ms debounce with an immediate local filter over already-loaded rows as a preview |
| Copy drifts from §72–§74 | reviewer finds a paraphrase | `strings.ts` is the single source and `copy-coverage.test.ts` fails the build on literals |

## Security Considerations

- Technical details show paths that the redaction layer already sealed; the frontend never
  re-fetches raw values from disk and has no filesystem capability.
- Tooltips render text nodes only (Svelte escapes by default); no `{@html}` anywhere in this phase —
  enforced by grepping for `{@html` in the gate of Phase 21.
- Search input is length-capped client-side and re-validated in Rust; it never reaches SQL unbound.
````

### FILE: phase-08-collector-runtime-entity-graph.md

````markdown
---
phase: 8
title: "Collector Runtime & Entity Graph"
status: pending
priority: P1
effort: 10h
dependencies: [7]
---

## Overview

§69 Phase 2 opens here: the supervised collector runtime (§5 trait, lifecycle, panic isolation,
health), the normalizer that turns `RawEvent` into human sentences with importance and dedupe, and
the entity/relationship graph with bounded traversal (§30). Every collector in Phases 09–13 plugs
into this without touching runtime code.

## Requirements

Functional
- `Collector` trait exactly as §5 specifies: stable `id()`, `start(ctx)`, `stop()`, and an on-demand
  `snapshot()`; async via `async_trait` (resolve the crate version with `cargo add async-trait`
  and record it) or a hand-written boxed-future signature if that keeps the trait object simpler.
- `CollectorContext`: event sink (`mpsc::Sender<RawEvent>`), a settings snapshot, a health reporter,
  and a `tokio::sync::watch::Receiver<bool>` cancellation signal.
- `CollectorRegistry`: registers collectors, starts only enabled ones, stops them in reverse order,
  and exposes `get_collector_status()` rows: id, display name, state
  (`Running | Stopped | Degraded | Failed | Unsupported`), last event time, error count, last error
  (redacted), events emitted, and a `restarts` counter.
- Panic isolation: each collector body runs inside `catch_unwind(AssertUnwindSafe(...))`; a panic
  marks `Failed`, logs a redacted message, and schedules a restart with backoff 30 s → 2 min → 10 min
  (capped); after 5 consecutive failures the collector stays `Failed` until a manual retry.
- Per-collector rate ceiling: a token bucket (default 200 events/10 s) that, once exceeded, switches
  the collector to `Degraded` with the reason "Suppressed a burst of changes" and counts suppressions.
- Normalizer: templates per event type producing §12/§14/§15/§16 style sentences, importance rules
  (§27 conservative default), dedupe by `(event_type, entity_id, before, after)` inside a 5 s window,
  and rules for known-noise sources (documented, not silently dropped).
- Entity upsert (`entities`) + relationship upsert (`relationships`) with the §30 edge kinds; edges
  carry `first_seen`, `last_seen`, and a confidence.
- Traversal: `related_entities(entity_id, depth ≤ 3)` BFS with a visited set, a 500-node cap, and
  deterministic ordering (by edge confidence then id) so results are reproducible.
- Global pause/resume: `set_monitoring_paused(bool)` stops every collector and records a
  `monitoring_paused` / `monitoring_resumed` system event. This removes the Phase 04 stub.
- Sidecar protocol documented in `docs/sidecar-protocol.md`: newline-delimited JSON over stdio,
  a versioned `hello` handshake, `event` frames matching `RawEvent`, `health` frames, and the
  explicit statement that no sidecar ships in the MVP (§34 stays a documented seam).
Non-functional
- Registry start/stop is idempotent; a double `stop()` cannot panic or double-decrement counters.
- Stopping all collectors completes within 2 s (cancellation is cooperative; blocking snapshots run
  on `spawn_blocking` and are abandoned, not awaited, past 2 s).
- Normalizing 10 000 raw events ≤200 ms (pure CPU, no I/O) — asserted in a test.
- Traversal at depth 3 over a 50 000-edge graph ≤50 ms.
- No collector can reach `storage` directly: `tests/layering.rs` forbids `use crate::storage` inside
  `collectors/`, so persistence stays behind normalizer → `seal` → repository.

## Architecture

```
Collector ──RawEvent──▶ normalizer ──SystemEvent──▶ redact::seal ──RedactedEvent──▶ writer thread
    │                        │                                                          │
    └── health reporter ──▶ collector_status                    entities/relationships ─┘
```

The supervisor owns one `JoinHandle` per collector plus its backoff state. Health is a
`Arc<RwLock<HashMap<CollectorId, CollectorHealth>>>` updated by the reporter; the status command
reads a clone. The normalizer is a pure module (`fn normalize(RawEvent, &NormalizerCtx) -> Vec<SystemEvent>`)
so it is fully unit-testable with fixtures and cannot perform I/O.

## Related Code Files

Create
- `src-tauri/src/collectors/{mod.rs,traits.rs,registry.rs,supervisor.rs,rate_limit.rs,health.rs}`
- `src-tauri/src/core/normalize/{mod.rs,templates.rs,importance.rs,dedupe.rs}`
- `src-tauri/src/core/graph/{mod.rs,upsert.rs,traverse.rs}`
- `src-tauri/tests/{collector_lifecycle.rs,normalizer_fixtures.rs,graph_traversal.rs}`
- `src-tauri/tests/fixtures/raw_events/*.json`
- `docs/sidecar-protocol.md`, `docs/adr/0009-collector-supervision-model.md`

Modify
- `src-tauri/src/commands/{collectors.rs,monitoring.rs}` (real status + pause/resume)
- `src-tauri/src/storage/{repo_entities.rs,repo_relationships.rs}`
- `src-tauri/src/lib.rs` (build and own the registry in managed state), `src/lib/tauri/bindings.ts`
- `TODO.md` (remove the pause/resume stub entry recorded in Phase 04)

Delete — none.

## Implementation Steps

1. `traits.rs`: the §5 trait verbatim plus `CollectorContext`, `CollectorHealth`, `CollectorState`.
   Add a doc comment stating the trait is the extension seam for §34 sidecars.
2. `supervisor.rs`: spawn loop with `catch_unwind`, backoff table, restart counter, and cooperative
   cancel. A collector that returns `Err(Unsupported)` is marked `Unsupported` and never restarted.
3. `rate_limit.rs`: token bucket keyed by collector; on trip, emit exactly one `Degraded` health
   update and one system event, then suppress silently while counting.
4. `normalize/templates.rs`: one function per event type returning `{ title, description,
   before, after, entity_id, entity_root, category, importance_hint }`. Titles use the §12–§16
   phrasing; unknown types fall back to a neutral factual sentence, never to "unknown event".
5. `importance.rs`: rules producing `background | notable | important | critical`, defaulting one
   level lower than instinct (§27) and documenting each rule inline with the invariant it protects.
6. `dedupe.rs`: a bounded LRU (2 048 entries) over `(event_type, entity_id, before, after)` with a
   5 s window; a suppressed duplicate bumps an occurrence counter rather than vanishing.
7. `graph/upsert.rs`: `entities` upsert on the deterministic id from Phase 03; relationship upsert
   keyed by `(from, to, kind)` updating `last_seen` and confidence.
8. `graph/traverse.rs`: BFS with depth/nodes caps and deterministic ordering.
9. `commands/monitoring.rs`: `set_monitoring_paused` → registry stop/start + a recorded system event;
   the tray "Pause monitoring" label toggles to "Resume monitoring" from the returned state.
10. `tests/collector_lifecycle.rs`: a `PanicCollector` (panics on the 3rd tick) must not kill the
    process, must reach `Failed`, must restart on the 30 s backoff (time is injected), and must stop
    restarting after 5 failures. A `FloodCollector` must trip the rate ceiling and become `Degraded`.
11. `tests/normalizer_fixtures.rs`: JSON fixtures → expected sentence + importance, one per event
    type; a test asserts every `EventType` variant has a fixture (exhaustive `match` over the enum).
12. `tests/graph_traversal.rs`: build 50 000 edges, assert depth-3 latency and identical results across
    two runs; assert a cycle cannot loop forever.
13. Write `docs/sidecar-protocol.md` and ADR 0009.
14. Commits: `feat: add collector trait and supervised runtime`,
    `feat: add event normalizer with importance and dedupe rules`,
    `feat: add entity graph upsert and bounded traversal`,
    `feat: implement global monitoring pause`, `docs: document sidecar protocol`.

## Todo

- [ ] `Collector` trait + context + health types
- [ ] supervisor with panic isolation and backoff
- [ ] per-collector rate ceiling
- [ ] normalizer templates for every event type
- [ ] importance rules (conservative) + dedupe
- [ ] entity/relationship upsert + BFS traversal
- [ ] pause/resume real (Phase 04 stub removed from `TODO.md`)
- [ ] lifecycle, fixture, and traversal tests

## Success Criteria

- [ ] A panicking collector leaves the app running and shows `Failed` in Collector Status
- [ ] Restart backoff observed at 30 s / 2 min / 10 min, capped after 5 failures
- [ ] Every `EventType` variant has a normalizer fixture (exhaustive-match test)
- [ ] Normalizing 10 000 events ≤200 ms; depth-3 traversal over 50k edges ≤50 ms
- [ ] Pause stops all collectors and records `monitoring_paused`; resume restores them
- [ ] `tests/layering.rs` passes with `collectors/` unable to import `storage`

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test collector_lifecycle
cargo test --manifest-path src-tauri/Cargo.toml --test normalizer_fixtures
cargo test --manifest-path src-tauri/Cargo.toml --test graph_traversal
cargo test --manifest-path src-tauri/Cargo.toml --test layering
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `catch_unwind` ineffective because `panic = "abort"` crept into a profile | a test panic aborts the process | Phase 05 forbids `panic = "abort"`; a test asserts a caught panic returns `Err` — if it aborts, remove the profile setting before continuing |
| A blocking Win32 call ignores cancellation | shutdown exceeds 2 s | Blocking work runs in `spawn_blocking` with a deadline; the supervisor abandons the handle and logs `AbandonedOnShutdown` |
| Rate ceiling hides a real storm of changes | `Degraded` with high suppression count | Suppression count is surfaced in Collector Status and Diagnostics with the wording "Suppressed N changes" so it is never invisible |
| Importance rules become a dumping ground | many events marked `important` | A test asserts that a fixture corpus of 200 events yields ≤10% at `important` or higher (§54 calm-importance guard) |
| Graph grows unbounded | `relationships` row count climbs monotonically | Retention (Phase 03) prunes edges whose `last_seen` falls outside the window; a test seeds stale edges and asserts pruning |
| Deterministic ids collide across collectors | two different things share an entity id | Id prefixes are namespaced per kind and a test asserts uniqueness across a mixed 10 000-entity corpus |

## Security Considerations

- Collector error strings pass through the redaction scrubber before storage and logging; a test
  feeds a fake secret into a collector error and asserts it never appears in `collector_status`.
- The sidecar protocol document explicitly forbids executing sidecar-supplied paths and requires a
  fixed argv when a sidecar is eventually introduced.
- Rate ceiling plus writer channel cap together bound the §82 "event flood" threat.
````

### FILE: phase-09-process-network-collectors.md

````markdown
---
phase: 9
title: "Process & Network Listener Collectors"
status: pending
priority: P1
effort: 12h
dependencies: [8]
---

## Overview

The first two real collectors, chosen because together they answer the flagship §15 story ("Port 3000
stopped listening because the process holding it exited"). Both must be honest about the measured
non-admin blind spots (B3, ~46% exe-path denial) rather than pretending completeness.

## Requirements

Functional
- Process collector: 1 s diff-poll via `sysinfo` 0.39.6 (`Win32_ProcessStartTrace` is **denied**
  without elevation per the measured matrix — no ETW, no WMI start trace in the MVP).
  Emits `process_started` / `process_stopped` with name, pid, parent pid, start time, and — when
  readable — executable path, file version, and signature status.
- Executable metadata: `GetFileVersionInfoSizeW` / `GetFileVersionInfoW` / `VerQueryValueW` for
  version and product name; signature via `WinVerifyTrust` with a cache keyed by
  `(path, size, mtime)` and a 500-entry LRU so no path is verified twice.
- Access-denied degradation: when the path is unreadable (~46% of processes measured), the event still
  records name/pid and sets `detail_level = "name_only"`; the UI shows "Path not available (requires
  administrator)". The collector never logs a failure storm — one summary line per minute.
- **Command lines are never captured.** Only the executable name/path. This is a hard requirement
  (§13/§82 privacy) and is enforced by a test that greps the collector for `cmd`/`CommandLine` access.
- Network listener collector: 2 s diff-poll of `GetExtendedTcpTable(TCP_TABLE_OWNER_PID_LISTENER)`
  and `GetExtendedUdpTable(UDP_TABLE_OWNER_PID)` for both `AF_INET` and `AF_INET6`; emits
  `port_opened` / `port_closed` with port, protocol, address family, owning pid and process name.
- Relationship emission: `process --listens_on--> port`, `process --child_of--> process`.
- Correlation: when a `port_closed` and a `process_stopped` for the owning pid land within 5 s, the
  port event's description uses the §15 phrasing and a `caused_by` relationship is written, with the
  literal timing sentence "These events occurred 300ms apart." rendered from the measured delta.
- Snapshot: full process list and listener table on demand (used by Checkpoints in Phase 14).
Non-functional
- Process poll ≤15 ms per tick at ~250 processes (measured baseline 6 ms); listener poll ≤10 ms.
- Combined steady-state CPU contribution from both collectors ≤0.5% (measured over 120 s).
- Noise floor: short-lived processes that start and stop inside one poll interval are invisible by
  construction — this limitation is documented in `docs/limitations.md`, not hidden.
- Signature verification never blocks the poll loop: it runs on `spawn_blocking` and the event is
  emitted with `signature = "checking"` upgraded by a follow-up update row only for `important`+ events.

## Architecture

Both collectors follow the same shape: a `tokio::time::interval` loop, a previous-state
`HashMap`, a diff step producing `RawEvent`s, and cooperative cancellation. Win32 access lives in
`platform/windows/{process_info.rs,tcp_table.rs,file_version.rs,trust.rs}`; the collectors themselves
contain no `unsafe`. Every Win32 symbol is confirmed against `cargo doc -p windows --features ...`
before it is written — the `windows` crate feature list is extended per module and recorded in
`docs/windows-api-surface.md` (symbol → feature → module), so a reviewer can re-verify each call.

## Related Code Files

Create
- `src-tauri/src/platform/windows/{process_info.rs,tcp_table.rs,file_version.rs,trust.rs}`
- `src-tauri/src/collectors/{process.rs,network_listener.rs}`
- `src-tauri/examples/portholder.rs` (binds a port, holds it, exits on keypress — controlled-action test aid)
- `src-tauri/tests/{process_collector.rs,listener_correlation.rs,no_command_lines.rs}`
- `docs/windows-api-surface.md`, `docs/limitations.md`

Modify
- `src-tauri/Cargo.toml` (`sysinfo = "0.39.6"`, `windows` features for `Win32_System_ProcessStatus`,
  `Win32_NetworkManagement_IpHelper`, `Win32_Storage_FileSystem`, `Win32_Security_WinTrust` — confirm
  exact feature names with `cargo doc` before adding)
- `src-tauri/src/collectors/registry.rs` (register both), `src-tauri/src/core/normalize/templates.rs`
- `src/lib/copy/strings.ts` (§15 sentences), `src/lib/components/details/technical-details.svelte`

Delete — none.

## Implementation Steps

1. `platform/windows/process_info.rs`: `sysinfo::System` wrapper returning a `ProcessSnapshot`
   (`pid, ppid, name, exe: Option<PathBuf>, start_ms`). Explicitly select
   `ProcessRefreshKind` without command-line data — verify the exact builder name via
   `cargo doc -p sysinfo` before writing it.
2. `file_version.rs`: version-info extraction; handles the "no version resource" case as `None`,
   not an error. Bounded buffer, no `unwrap`.
3. `trust.rs`: `WinVerifyTrust` with `WINTRUST_DATA`/`WINTRUST_FILE_INFO`; resolve the exact struct
   field names and the `WINTRUST_ACTION_GENERIC_VERIFY_V2` GUID from `cargo doc` — do not write them
   from memory. Returns `Signed { subject } | Unsigned | Unknown`, cached by `(path, size, mtime)`.
4. `collectors/process.rs`: diff loop, degradation handling, one-per-minute denial summary, snapshot.
5. `tcp_table.rs`: `GetExtendedTcpTable` / `GetExtendedUdpTable` with the two-call size pattern
   (query required size, allocate, re-query), parsing both the IPv4 and IPv6 row structs; ports are
   converted from network byte order explicitly with a unit test on a known value.
6. `collectors/network_listener.rs`: diff loop, `listens_on` relationship, and the 5 s correlation
   window that produces the §15 causal phrasing and `caused_by` edge.
7. Copy: add the §15 strings, including the exact "These events occurred 300ms apart." template where
   the number is substituted from the measured delta and rounded to whole milliseconds.
8. `tests/no_command_lines.rs`: `rg` the collector and platform sources for
   `(?i)cmd_line|command_line|CommandLine|\.cmd\(\)` and assert zero matches.
9. `tests/process_collector.rs`: feed two synthetic snapshots into the pure diff function and assert
   the emitted start/stop events, including the `name_only` degradation path.
10. `tests/listener_correlation.rs`: inject a `port_closed` + `process_stopped` pair 300 ms apart and
    assert the produced description matches the §15 sentence exactly and a `caused_by` edge exists.
11. Manual controlled-action verification, recorded in `docs/measurements/phase-09-manual.md`:
    run `cargo run --example portholder -- 3000`, confirm `port_opened` appears within 3 s, exit it,
    confirm `port_closed` plus the causal sentence within 5 s.
12. Write `docs/limitations.md` (sub-second processes, ~46% path denial, no command lines by design,
    B3 elevation note) and `docs/windows-api-surface.md`.
13. Commits: `feat: add process collector with executable metadata`,
    `feat: add tcp and udp listener collector`,
    `feat: correlate port closures with process exits`,
    `docs: document non-admin monitoring limitations`.

## Todo

- [ ] process snapshot wrapper without command lines
- [ ] file version + cached signature verification
- [ ] process collector with denial degradation
- [ ] TCP/UDP listener tables (IPv4 + IPv6)
- [ ] listener collector + `listens_on` edges
- [ ] 5 s port/process correlation with §15 copy
- [ ] `portholder` example + manual verification record
- [ ] limitations + Windows API surface docs

## Success Criteria

- [ ] Starting/stopping Notepad produces `process_started`/`process_stopped` within 2 s
- [ ] `cargo run --example portholder -- 3000` produces `port_opened`, and exiting produces
      `port_closed` with the §15 causal sentence and a measured "N ms apart" phrase
- [ ] `tests/no_command_lines.rs` passes (zero command-line access)
- [ ] Processes with unreadable paths still produce events marked `name_only`
- [ ] Both collectors' combined idle CPU ≤0.5% over 120 s (recorded)
- [ ] `docs/limitations.md` states the sub-second and privilege blind spots plainly

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test process_collector
cargo test --manifest-path src-tauri/Cargo.toml --test listener_correlation
cargo test --manifest-path src-tauri/Cargo.toml --test no_command_lines
cargo run --manifest-path src-tauri/Cargo.toml --example portholder -- 3000
powershell -ExecutionPolicy Bypass -File scripts/measure-idle-cpu.ps1
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| A `windows` crate symbol or feature name is wrong | compile error on the Win32 module | Resolve from `cargo doc -p windows --features <f>` or `rg` in the registry source; never guess a second time. Record the resolved path in `docs/windows-api-surface.md` |
| `sysinfo` refresh pulls command lines by default | `no_command_lines` test fails or memory grows | Use the explicit refresh-kind builder that excludes them; if the API cannot exclude them, strip the field at the wrapper boundary and assert the struct never carries it |
| Signature verification stalls on network paths | poll tick exceeds 15 ms | Verification is off-thread and skipped entirely for UNC paths; `Unknown` is a valid honest result |
| 1 s polling misses meaningful processes | user reports a missing app launch | Documented limitation plus the Phase 11 Application-Error correlation; do not raise the poll rate (CPU budget) |
| Port byte-order bug shows port 30720 instead of 3000 | wrong port numbers in the UI | Unit test on a known-endianness value is part of this phase's gate |
| Correlation produces false causality | "caused by" on unrelated pairs | Correlation requires pid identity, not just timing; timing-only pairs get `TemporalOnly` confidence and the §29 hedged wording |

## Security Considerations

- No command lines, no process memory, no handle enumeration — the collector reads only what the
  matrix confirms is available non-elevated.
- Executable paths pass through `redact::seal`, which replaces the user profile segment with
  `%USERPROFILE%` before persistence (Phase 02 rule), so timelines do not leak the account name.
- `examples/portholder.rs` binds to `127.0.0.1` only, never `0.0.0.0`, so the test aid never exposes a
  listener to the network.
````

### FILE: phase-10-file-config-collector.md

````markdown
---
phase: 10
title: "File & Config Collector"
status: pending
priority: P2
effort: 11h
dependencies: [9]
---

## Overview

Watched-location file monitoring with config-aware diffing (§13) — the highest-privacy-risk collector
in the product, and the reason the redaction chokepoint exists before any collector was written. File
*contents are never stored*; only key-level before/after for recognized config formats, after redaction.

## Requirements

Functional
- Watch user-selected locations only (default: none until onboarding adds them). Recursive watching via
  `notify` 8.2.0 with `notify-debouncer-full` 0.7.0 at a 500 ms debounce.
- Location validation before watching: canonicalize; reject drive roots, `C:\Windows`,
  `C:\Program Files*`, `%ProgramData%`, any path containing a reparse point/junction, and any path
  outside the user profile unless the user explicitly confirms; caps of 25 locations, depth 8, and
  50 000 tracked files (exceeding the cap marks the collector `Degraded` with a specific reason).
- Ignore rules: `node_modules`, `.git/objects`, `target`, `dist`, `build`, `__pycache__`, `.venv`,
  `*.log`, `*.tmp`, `*.lock~`, and files >8 MB (hashed metadata only, never diffed).
- Event types: `file_created`, `file_modified`, `file_deleted`, `file_renamed`, `config_changed`.
- Recognized config formats and the differ used for each:
  `.env`/`.ini`/`.conf`/`.properties`/`.yaml`/`.yml` → line-key differ (key = text before the first
  `=`/`:` at the shallowest indent); `.json` → `serde_json` recursive key-path diff;
  `.toml` → `toml` crate key-path diff; `.xml` → metadata-only change with the limitation documented.
- Config diff output: a list of `{ key_path, before, after, kind: Added|Removed|Changed }`, each value
  passed through `redact::seal`'s value rules **before** it can be persisted or logged.
- Content hashing: `sha2` SHA-256 for files ≤8 MB to detect no-op writes (same hash ⇒ no event).
- Snapshot: hash + mtime + size of every tracked file, used by Checkpoints.
- Settings integration: add/remove locations at runtime without restarting the app; removal stops the
  watcher and clears its tracked state.
Non-functional
- A 10 000-file tree is indexed in ≤5 s at startup and adds ≤10 MB RSS (measured, recorded).
- A burst of 500 file writes in 2 s produces ≤50 events after debounce and never blocks the UI.
- Config diff of a 500-line `.env` ≤5 ms.
- Reading a file for diffing uses a 2 MB read cap and a 3 s deadline; a locked file yields a
  metadata-only event with reason `file_locked`, never a hang.
- Zero file contents in the database: enforced by a test that seeds a file with a unique 40-char
  marker string and asserts the marker appears nowhere in the DB or logs unless it is a config value
  that survived redaction as a `SAFE_KEYS` entry.

## Architecture

`platform/windows/fs_watch.rs` owns the debouncer and translates its events into a
`FsChange { path, kind, at }`. `core/diff/` holds the format-detecting dispatcher plus the three
differs, all pure functions over `&str` (no filesystem access), which makes them exhaustively testable
with fixtures. The collector reads the file (bounded), computes the hash, calls the differ against a
cached previous snapshot of *keys and redacted values only*, and emits. Because the cache holds
redacted values, even the in-memory state never carries a secret past the chokepoint.

## Related Code Files

Create
- `src-tauri/src/platform/windows/fs_watch.rs`
- `src-tauri/src/core/diff/{mod.rs,detect.rs,line_key.rs,json_diff.rs,toml_diff.rs}`
- `src-tauri/src/collectors/file_config.rs`
- `src-tauri/src/core/paths/validate_watch_location.rs`
- `src-tauri/tests/{config_diff.rs,watch_validation.rs,no_file_contents.rs}`
- `src-tauri/tests/fixtures/configs/{sample.env,sample.json,sample.toml,sample.yaml,sample.ini}`

Modify
- `src-tauri/Cargo.toml` (`notify = "8.2.0"`, `notify-debouncer-full = "0.7.0"`, `sha2`, `toml` —
  record the resolved `sha2`/`toml` versions after `cargo add`)
- `src-tauri/src/redact/engine.rs` (wire the config-value path; no new public constructor)
- `src-tauri/src/collectors/registry.rs`, `src-tauri/src/commands/settings.rs` (location add/remove)
- `src/lib/components/settings/watched-locations.svelte`, `src/lib/copy/strings.ts`
- `docs/limitations.md` (XML metadata-only, locked files, >8 MB files)

Delete — none.

## Implementation Steps

1. `validate_watch_location.rs`: canonicalize with `std::fs::canonicalize`, strip the `\\?\` prefix,
   reject the denylist, and detect reparse points via `std::fs::symlink_metadata().file_type().is_symlink()`
   plus the `FILE_ATTRIBUTE_REPARSE_POINT` bit (confirm the constant's module in the `windows` crate docs).
   Returns a typed rejection reason so the UI can explain it in words.
2. `fs_watch.rs`: build the debouncer (verify the 8.2.0 / 0.7.0 constructor signature via
   `cargo doc -p notify-debouncer-full` — the builder shape changed across majors), one watcher per
   location, and a channel of `FsChange`.
3. `core/diff/detect.rs`: extension + first-line sniffing (e.g. a `.conf` that parses as TOML still
   uses the line-key differ — deterministic by extension first, sniffing only for extensionless files).
4. `line_key.rs`: parse into ordered `(key, value)` honoring comments (`#`, `;`) and blank lines;
   report Added/Removed/Changed by key, and fall back to "N lines changed" when a file has no
   recognizable keys.
5. `json_diff.rs` / `toml_diff.rs`: recursive key-path diff (`server.port`), arrays compared by index
   with a length-change note, depth cap 32.
6. `collectors/file_config.rs`: bounded read, hash short-circuit, differ dispatch, redaction, event
   emission, cap enforcement with `Degraded` reasons, and the runtime add/remove path.
7. Redaction integration test using the exact §13 fixture: a `.env` where `API_PORT` changes 3000→4000
   and `OPENAI_API_KEY` changes — assert the event shows `API_PORT 3000 → 4000` and
   `OPENAI_API_KEY [REDACTED] → [REDACTED]`, and that neither secret value exists in the DB or log file.
8. `tests/watch_validation.rs`: create a junction with `cmd //c mklink /J` inside a temp dir, point a
   watch location through it, and assert the validator rejects it; assert `C:\Windows` and `C:\` are rejected.
9. `tests/no_file_contents.rs`: write a file containing a unique marker, let the collector process it,
   then assert the marker is absent from every DB table and from the log file.
10. `tests/config_diff.rs`: fixture-driven, one case per format plus the no-op-write case
    (same content ⇒ zero events) and the locked-file case.
11. Measure the 10 000-file index and record it in `docs/measurements/phase-10-fs.md`.
12. Commits: `feat: add watched location validation`, `feat: add filesystem watcher collector`,
    `feat: add config format diffing`, `test: prove file contents never reach storage`.

## Todo

- [ ] watch-location validator with typed rejections
- [ ] debounced recursive watcher
- [ ] format detection + three differs
- [ ] collector with caps, hashing, locked-file handling
- [ ] §13 redaction fixture green
- [ ] junction escape + no-contents tests
- [ ] 10k-file index measurement recorded

## Success Criteria

- [ ] Editing a watched `.env` produces `API_PORT 3000 → 4000` and `OPENAI_API_KEY [REDACTED] → [REDACTED]`
- [ ] Saving a file with unchanged content produces zero events
- [ ] Watch locations pointing through a junction or at `C:\Windows` are rejected with a readable reason
- [ ] A 40-char marker written into a watched file appears nowhere in the DB or logs
- [ ] 10 000-file tree indexed ≤5 s, ≤10 MB added RSS (recorded)
- [ ] Removing a location stops its events immediately without an app restart

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test config_diff
cargo test --manifest-path src-tauri/Cargo.toml --test watch_validation
cargo test --manifest-path src-tauri/Cargo.toml --test no_file_contents
cargo test --manifest-path src-tauri/Cargo.toml --test redaction_leak
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| A secret survives into a diff value | `redaction_leak` or `no_file_contents` fails | Treat as a release blocker: fix the pattern, add the shape to the leak-test corpus, and re-run before any other work in this phase continues |
| `notify` 8.x/debouncer 0.7 API differs from expectation | compile error in `fs_watch.rs` | Resolve the builder from `cargo doc -p notify-debouncer-full`; if the crate pair is incompatible, pin `notify` to the version the debouncer depends on and record it in an ADR |
| Watching a huge tree exhausts handles | `Degraded` with cap-exceeded, or watcher errors | Caps are enforced before watching; the UI names the offending location and suggests a narrower one |
| Editor atomic-save churn (rename dance) creates triple events | 3 events per save | Debounce plus hash short-circuit plus rename pairing inside the 500 ms window collapse them; asserted in a fixture test |
| Locked file blocks the collector | event latency spike | 3 s deadline + metadata-only event with `file_locked` |
| XML diffing silently produces nothing useful | user sees only "file modified" | Documented in `docs/limitations.md` and surfaced in the drawer as "Detailed comparison not supported for this format" |

## Security Considerations

- This is the collector §82 flags hardest: the invariant "file contents are never persisted" is
  enforced by construction (only differ output is passed on) and proven by `no_file_contents.rs`.
- Path canonicalization plus reparse-point rejection prevents a symlink from redirecting the watcher
  outside the user's chosen scope.
- Read caps (2 MB, 3 s) bound a hostile-file denial-of-service against the collector thread.
- The in-memory previous-value cache stores redacted values only, so a crash dump cannot leak secrets.
````

### FILE: phase-11-application-service-collectors.md

````markdown
---
phase: 11
title: "Application & Service Collectors"
status: pending
priority: P2
effort: 9h
dependencies: [10]
---

## Overview

Installed-application tracking across the three uninstall hives (§17) and Windows service state and
configuration monitoring (§16), including the restart-collapse rule and the §54 discipline that
background app updaters must never become notifications.

## Requirements

Functional
- Application inventory from three registry locations (measured counts on this machine: 88 / 148 / 38
  keys): `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`,
  `HKLM\SOFTWARE\WOW6432Node\...\Uninstall`, `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`.
  Fields: `DisplayName`, `DisplayVersion`, `Publisher`, `InstallDate`, `InstallLocation`,
  `SystemComponent`, `ParentKeyName`. Rows with `SystemComponent = 1` or an empty `DisplayName` are skipped.
- Dedupe across hives by `(normalized DisplayName, Publisher)` keeping the 64-bit entry; the source
  hive is retained in technical details.
- Events: `app_installed`, `app_updated` (version before → after), `app_removed`, with §17 phrasing.
- Change detection: `RegNotifyChangeKeyValue` on each hive with
  `REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET` and `REG_NOTIFY_THREAD_AGNOSTIC`
  (research assumption #3 — unverified for HKLM non-elevated; a spike decides). Fallback: a 5-minute
  diff-poll, which measured 50 ms per full scan and is therefore an acceptable permanent fallback.
- Service collector: `EnumServicesStatusExW` (state + type) plus `QueryServiceConfigW` for start type
  and binary path, at a 5 s diff-poll (measured 3.3 ms per scan). Events: `service_started`,
  `service_stopped`, `service_restarted`, `service_config_changed` (start type or binary path).
- Restart collapse: a `stopped` followed by `running` for the same service within 30 s becomes a
  single `service_restarted` event; the two raw transitions are not emitted.
- Change-notification spike: `NotifyServiceStatusChangeW` (research assumption #4) is attempted; if it
  requires elevation or proves unreliable, the 5 s diff-poll stands and the result is recorded in an ADR.
- Update noise policy: applications matching a known-updater list (Chrome, Edge, Spotify, Discord,
  Teams, Zoom, NVIDIA GeForce Experience, plus a documented pattern for `*Update*` publishers) are
  recorded at `background` importance and are **never** notification-eligible (§53 denylist).
- Snapshot: full application inventory and service table for Checkpoints.
Non-functional
- Full application scan ≤120 ms (measured baseline ~50 ms) and runs off the async runtime via
  `spawn_blocking`; service scan ≤20 ms.
- Registry and service handles are always closed on every path including error paths (RAII wrappers).
- 274 uninstall keys and ~300 services produce zero events on an unchanged system — a stability test
  runs two consecutive scans and asserts an empty diff (guards against unstable field ordering).
- App-updater traffic contributes ≤5% of `notable`+ events over a 24 h dogfood run (recorded).

## Architecture

`platform/windows/registry.rs` provides a thin RAII `RegKey` (open, read string/dword, enumerate
subkeys, notify) so no collector writes `unsafe` directly; the exact `windows` crate symbols
(`RegOpenKeyExW`, `RegQueryValueExW`, `RegEnumKeyExW`, `RegNotifyChangeKeyValue`, `RegCloseKey`) are
confirmed from `cargo doc -p windows --features Win32_System_Registry` before use.
`platform/windows/services.rs` wraps the SCM handle with the same discipline
(`OpenSCManagerW` with `SC_MANAGER_ENUMERATE_SERVICE | SC_MANAGER_CONNECT` — read-only rights only,
which the matrix measured as available non-elevated).

## Related Code Files

Create
- `src-tauri/src/platform/windows/{registry.rs,services.rs}`
- `src-tauri/src/collectors/{application.rs,service.rs}`
- `src-tauri/src/core/policy/updater_denylist.rs`
- `src-tauri/tests/{app_inventory.rs,service_restart_collapse.rs,updater_noise.rs}`
- `docs/adr/0010-registry-and-service-change-detection.md`

Modify
- `src-tauri/Cargo.toml` (`windows` features: `Win32_System_Registry`, `Win32_System_Services` —
  confirm names first), `src-tauri/src/collectors/registry.rs`
- `src-tauri/src/core/normalize/{templates.rs,importance.rs}`, `src/lib/copy/strings.ts`
- `docs/windows-api-surface.md`, `docs/limitations.md`

Delete — none.

## Implementation Steps

1. `platform/windows/registry.rs`: RAII `RegKey` with `Drop` calling `RegCloseKey`; wide-string
   helpers; `read_string` tolerating `REG_SZ`/`REG_EXPAND_SZ` and returning `None` for missing values.
2. Spike (45 min): `RegNotifyChangeKeyValue` against `HKLM\...\Uninstall` non-elevated with
   `REG_NOTIFY_THREAD_AGNOSTIC`. Record the result (works / requires elevation / unreliable) in
   ADR 0010 and choose notify-plus-poll-verify or poll-only accordingly.
3. `collectors/application.rs`: three-hive scan, filter, dedupe, diff against the previous inventory,
   version-comparison for `app_updated`, and the §17 sentences.
4. `updater_denylist.rs`: the named updaters plus documented patterns; a function
   `is_background_updater(publisher, name) -> bool` used by both importance rules and notification eligibility.
5. `platform/windows/services.rs`: SCM open with read-only rights, `EnumServicesStatusExW` two-call
   size pattern, `QueryServiceConfigW` per changed service only (not for all ~300 every tick).
6. `collectors/service.rs`: 5 s diff-poll, pending-state handling (`START_PENDING`/`STOP_PENDING` are
   not events), and the 30 s restart-collapse buffer.
7. Spike (30 min): `NotifyServiceStatusChangeW`; on failure keep the poll and note it in ADR 0010.
8. `tests/app_inventory.rs`: fixture inventories → expected install/update/remove events, including
   the cross-hive dedupe case and the `SystemComponent` filter.
9. `tests/service_restart_collapse.rs`: stop+start within 30 s ⇒ exactly one `service_restarted`;
   stop with no start within 30 s ⇒ one `service_stopped`; a start 31 s later ⇒ two separate events.
10. `tests/updater_noise.rs`: a Chrome `app_updated` event is `background` importance and
    `notification_eligible == false`.
11. Stability test: two consecutive real scans produce an empty diff.
12. Manual verification recorded in `docs/measurements/phase-11-manual.md`: install a small portable
    app via its installer (user-scope, e.g. an MSI-free per-user installer already on the machine) or
    create a synthetic HKCU uninstall key with `reg add`, and confirm the event; `sc query`-observable
    restart of a user-controllable service (e.g. `Spooler` is admin-only — use a per-user service or
    the `net stop`-able one available non-elevated; if none exists, document that service events were
    verified only via injected fixtures and say so plainly).
13. Commits: `feat: add application inventory collector`,
    `feat: add service state and config collector`,
    `feat: suppress background updater noise`, `docs: record registry notification findings`.

## Todo

- [ ] RAII registry wrapper
- [ ] registry-notify spike + ADR 0010
- [ ] three-hive inventory with dedupe and filters
- [ ] updater denylist wired to importance + notifications
- [ ] SCM wrapper with read-only rights
- [ ] service collector + restart collapse
- [ ] service-notify spike outcome recorded
- [ ] inventory / collapse / noise tests + stability test

## Success Criteria

- [ ] Creating a synthetic HKCU uninstall key produces `app_installed` within one scan interval
- [ ] Changing its `DisplayVersion` produces `app_updated` with `before → after`
- [ ] A stop+start within 30 s yields exactly one `service_restarted` event
- [ ] Chrome/Spotify updates are `background` and never notification-eligible
- [ ] Two consecutive scans on an unchanged system produce zero events
- [ ] Application scan ≤120 ms, service scan ≤20 ms (recorded)

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test app_inventory
cargo test --manifest-path src-tauri/Cargo.toml --test service_restart_collapse
cargo test --manifest-path src-tauri/Cargo.toml --test updater_noise
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `RegNotifyChangeKeyValue` on HKLM needs elevation (assumption #3 unverified) | spike returns access denied | Ship the 5-minute poll (50 ms cost is negligible) and state the detection latency in `docs/limitations.md` |
| `NotifyServiceStatusChangeW` unreliable (assumption #4) | spike misses transitions | Keep the 5 s poll; document that sub-5-second service flaps may be missed |
| Handle leak in the enumeration loop | handle count climbs across scans | RAII wrappers plus a test that runs 200 scans and asserts a stable handle count via `GetProcessHandleCount` |
| Version strings not comparable (`1.2.3` vs `1.2.3.0` vs `2024.5`) | spurious `app_updated` events | Compare normalized dotted-numeric segments; if unparseable, compare raw strings and mark importance `background` |
| Per-user services differ across machines, making manual verification impossible | no admin-free service to toggle | Fall back to injected fixtures and state honestly in the measurements doc that live verification was not possible non-elevated |
| Updater denylist becomes stale | notification noise from a new updater | Denylist plus the generic pattern rule, and the §53 rate limit as a second line of defense |

## Security Considerations

- SCM is opened with enumeration/connect rights only — the app never requests service control rights,
  so it cannot start or stop a service even if compromised.
- Registry access is read-only; no write API is imported in `registry.rs` (asserted by a grep test for
  `RegSetValue` / `RegCreateKey` returning zero matches).
- `InstallLocation` and service binary paths are sealed (user-profile substitution) before persistence.
````

### FILE: phase-12-updates-drivers-snapshots.md

````markdown
---
phase: 12
title: "Updates, Drivers & Snapshots"
status: pending
priority: P2
effort: 14h
dependencies: [11]
---

## Overview

The §18/§19 collectors that carry the product's most compelling story (the §73 NVIDIA driver answer),
plus the shared Windows Event Log wrapper they need and the snapshot service that later phases reuse.
Every cadence here is dictated by measured cost: the driver sweep costs ~2.1 s and therefore must
never sit on a short timer.

## Requirements

Functional
- Windows Update history via WUA COM on a dedicated thread with
  `CoInitializeEx(None, COINIT_APARTMENTTHREADED)`: `IUpdateSession` → `CreateUpdateSearcher()` →
  `GetTotalHistoryCount()` → `QueryHistory(start, count)`. Measured working non-elevated (133 entries,
  38 ms). **`Win32_QuickFixEngineering` is explicitly forbidden** (returns only 4 rows — a trap).
- Fields per entry: `Title`, `Date`, `ResultCode`, `HResult`, `Operation`, `SupportUrl`,
  `UpdateIdentity.UpdateID`. KB extraction with `regex` `KB\d{6,7}` from the title.
- Cadence: every 15 minutes plus on resume; dedupe against the stored `UpdateID` + operation so a
  re-read of the same 133 entries produces no events.
- Event log wrapper (built here, reused by Phase 13): `EvtSubscribe` in pull mode with an XPath filter,
  `EvtNext`, `EvtRender(EvtRenderEventXml)`, and a `windows` crate `Evt*` handle wrapper closing via
  `EvtClose`. Hard caps: 64 KiB per rendered event, 256 events per pull, malformed XML skipped with a
  counter (never a panic).
- Update-client channel: subscribe to `Microsoft-Windows-WindowsUpdateClient/Operational` for
  install-start/success/failure. **Only event IDs empirically confirmed on this machine may be filtered
  on**; any ID not confirmed is left out and the reason recorded.
- Reboot correlation: System channel 6005/6006/6008/41/1074 (all measured present) plus
  `GetTickCount64` to compute uptime; produces `system_rebooted` with `clean | unexpected` and links
  pending-update entries to the following boot.
- Driver inventory via SetupAPI: `SetupDiGetClassDevsW(None, None, None, DIGCF_ALLCLASSES | DIGCF_PRESENT)`,
  then per device `SetupDiGetDevicePropertyW` with `DEVPKEY_Device_DriverVersion`,
  `DEVPKEY_Device_DriverProvider`, `DEVPKEY_Device_DriverDate`, `DEVPKEY_Device_InstanceId`,
  `DEVPKEY_NAME`, `DEVPKEY_Device_Class` (resolve each DEVPKEY's exact constant path from
  `cargo doc -p windows`; do not write them from memory).
- Driver cadence: on app start (deferred 30 s), on system resume, hourly, and on a device-arrival
  wake from `CM_Register_Notification` (research assumption #5 — if unavailable, hourly + resume only).
  The sweep runs on `spawn_blocking` at below-normal thread priority via `SetThreadPriority`.
- Driver events: `driver_installed`, `driver_updated` (`581.29 → 582.11` phrasing per §73),
  `driver_removed`, `device_added`, `device_removed`, each at `important` when the device class is
  Display/Net/Audio/Storage/System, otherwise `notable`.
- SnapshotService: `create_snapshot(reason)` invokes every collector's `snapshot()` concurrently with a
  10 s total deadline, stores a compact JSON payload per collector in `snapshots`, caps each payload at
  512 KiB and the total at 4 MiB, and retains snapshots 7 days (checkpoint-owned snapshots are exempt).
Non-functional
- The driver sweep never runs on the UI thread and never blocks the writer; a sweep in progress must
  not delay event ingestion by more than 50 ms (asserted by running a sweep while the burst producer runs).
- WUA COM lives on exactly one thread for the process lifetime; `CoUninitialize` runs on shutdown.
  A test asserts no COM call happens from a Tokio worker thread (the wrapper takes a thread-bound token).
- Sweep CPU: the hourly sweep may spike, but the 120 s idle-CPU measurement (M3) must still pass with
  the sweep excluded, and a separate measurement records the sweep's own cost.
- Malformed event-log XML, oversized events, and provider-not-found are all non-fatal and counted.

## Architecture

Three isolated OS surfaces, three wrappers, one shared discipline:
`platform/windows/wua.rs` (COM, thread-bound), `platform/windows/evtlog.rs` (handles + XPath + caps),
`platform/windows/setupapi.rs` (device enumeration). Collectors (`update.rs`, `driver.rs`) hold no
`unsafe`. The `evtlog` wrapper is deliberately built in this phase rather than Phase 13 because the
update collector needs it first, and Phase 13 then consumes a proven wrapper instead of writing its own.
`core/snapshot/service.rs` orchestrates snapshots and is the only writer of the `snapshots` table.

## Related Code Files

Create
- `src-tauri/src/platform/windows/{wua.rs,evtlog.rs,setupapi.rs,power_notify.rs}`
- `src-tauri/src/collectors/{update.rs,driver.rs,event_log_system.rs}`
- `src-tauri/src/core/snapshot/{mod.rs,service.rs}`
- `src-tauri/tests/{wua_history.rs,evtlog_render.rs,driver_diff.rs,snapshot_caps.rs}`
- `src-tauri/tests/fixtures/evtlog/{valid_6005.xml,malformed.xml,oversized.xml}`
- `docs/adr/0011-wua-and-setupapi-cadence.md`

Modify
- `src-tauri/Cargo.toml` (`windows` features for `Win32_System_Com`, `Win32_System_EventLog`,
  `Win32_Devices_DeviceAndDriverInstallation`, `Win32_System_Power` — confirm each), `regex`
- `src-tauri/src/collectors/registry.rs`, `src-tauri/src/core/normalize/templates.rs`
- `src/lib/copy/strings.ts` (§18/§19/§73 phrasing), `docs/windows-api-surface.md`, `docs/limitations.md`

Delete — none.

## Implementation Steps

1. `wua.rs`: a `WuaThread` owning a dedicated `std::thread` with an mpsc request channel; requests
   return via oneshot. Inside, `CoInitializeEx` once, then `IUpdateSession` created per request batch.
   Resolve every interface and method name from `cargo doc -p windows --features Win32_System_UpdateAgent`
   (confirm the feature name; if the interfaces are not in the `windows` crate, use `windows-core`
   `IDispatch` late binding via `CLSID_UpdateSession` progid `Microsoft.Update.Session` — decide in
   step 1 and record which route in ADR 0011).
2. `collectors/update.rs`: 15-min cadence, `UpdateID`+operation dedupe, KB extraction, result-code
   mapping (`2 = Succeeded`, `4 = Failed`, `5 = Aborted` — confirm against the enumeration docs), and
   §18 phrasing including the failure case with the HRESULT shown in technical details only.
3. `evtlog.rs`: `EvtSubscribe` (pull), `EvtNext` with a 256 cap, `EvtRender` into a bounded buffer
   (query size, reject >64 KiB), `EvtClose` in `Drop`, and a minimal XML field extractor
   (`EventID`, `TimeCreated`, `Provider`, `Level`, `EventData/Data`) implemented with a small
   hand-rolled scanner over the rendered XML — no XML crate, no regex over untrusted structure.
4. `collectors/event_log_system.rs`: System-channel subscription for 6005/6006/6008/41/1074 with the
   XPath filter, producing `system_rebooted` and the shutdown-reason description from 1074's data.
   Uptime from `GetTickCount64` distinguishes "since last boot" phrasing.
5. `power_notify.rs`: resume detection. Prefer a hidden-window `WM_POWERBROADCAST`/`PBT_APMRESUMEAUTOMATIC`
   handler if Tauri exposes a message hook; otherwise use a monotonic-clock gap detector (a >90 s jump
   between 1 s ticks means the machine slept) — the gap detector is the guaranteed path and ships
   regardless, with the message hook as an optional enhancement recorded in ADR 0011.
6. `setupapi.rs`: `SetupDiGetClassDevsW` + `SetupDiEnumDeviceInfo` + `SetupDiGetDevicePropertyW`
   two-call size pattern, `SetupDiDestroyDeviceInfoList` in `Drop`.
7. `collectors/driver.rs`: cadence per requirements, `spawn_blocking` + below-normal priority, diff by
   device instance id, §19/§73 phrasing (`NVIDIA graphics driver updated 581.29 → 582.11`).
8. `core/snapshot/service.rs`: concurrent `snapshot()` with per-collector 5 s and total 10 s deadlines,
   size caps, `snapshots` rows, 7-day retention with checkpoint exemption.
9. `tests/evtlog_render.rs`: the three fixtures — valid parses, malformed increments the skip counter
   and returns `Ok`, oversized is rejected without allocating past the cap.
10. `tests/wua_history.rs`: against the real WUA (this machine has 133 entries) assert ≥1 entry is read,
    each has a non-empty title, and a second read produces zero new events (dedupe). Marked `#[ignore]`
    only if WUA is unavailable, with the reason printed rather than silently skipped.
11. `tests/driver_diff.rs`: fixture device lists → expected install/update/remove events, and a timing
    test asserting the real sweep completes ≤4 s (2× the measured 2.1 s) on `spawn_blocking`.
12. `tests/snapshot_caps.rs`: a collector returning 10 MiB is truncated to 512 KiB with a recorded
    `truncated = true`; a hanging collector is abandoned at 5 s and the snapshot still completes.
13. Record the sweep cost and idle-CPU impact in `docs/measurements/phase-12-costs.md`.
14. Commits: `feat: add windows update history collector`,
    `feat: add event log wrapper with hard caps`, `feat: add reboot correlation`,
    `feat: add driver and device inventory collector`, `feat: add snapshot service`.

## Todo

- [ ] WUA thread + interface route decided (ADR 0011)
- [ ] update collector with dedupe and KB extraction
- [ ] evtlog wrapper with caps and malformed handling
- [ ] System-channel reboot correlation + uptime
- [ ] resume detection (gap detector guaranteed)
- [ ] SetupAPI enumeration + driver collector cadence
- [ ] snapshot service with deadlines and caps
- [ ] evtlog / WUA / driver / snapshot tests + cost measurements

## Success Criteria

- [ ] Update history reads real entries once and produces zero duplicates on the next cycle
- [ ] `Win32_QuickFixEngineering` appears nowhere in the codebase (grep-asserted)
- [ ] A malformed event-log XML fixture increments a counter and never panics
- [ ] Driver sweep completes ≤4 s, runs off the async runtime, and does not delay ingestion >50 ms
- [ ] A driver version change fixture renders exactly `NVIDIA graphics driver updated 581.29 → 582.11`
- [ ] Reboot events appear with `clean`/`unexpected` classification from real System-channel data
- [ ] Snapshots respect the 512 KiB per-collector and 4 MiB total caps

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test evtlog_render
cargo test --manifest-path src-tauri/Cargo.toml --test wua_history -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --test driver_diff -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --test snapshot_caps
rg -n "QuickFixEngineering" src-tauri/src | wc -l   # must print 0
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| WUA interfaces absent from the `windows` crate feature set | no `IUpdateSession` in `cargo doc` | Switch to `IDispatch` late binding on `Microsoft.Update.Session` (documented in ADR 0011) rather than inventing a symbol |
| COM called from the wrong thread | `RPC_E_WRONG_THREAD` at runtime | The thread-bound token type makes it a compile error to call from elsewhere; the test asserting thread affinity must pass before the collector is registered |
| `CM_Register_Notification` unavailable (assumption #5) | spike fails or no callbacks | Ship hourly + resume + start cadence only, and state the detection latency in `docs/limitations.md` |
| Driver sweep spikes CPU and users notice | idle-CPU measurement rises above 1% | Below-normal thread priority plus hourly cadence; if still visible, drop to every 4 hours and record the change |
| Event-log XPath filter rejected by the provider | subscription returns an error | Fall back to an unfiltered subscription with in-process ID filtering, and log the provider's error once |
| Unconfirmed event IDs produce nothing or wrong data | a filter matches zero events forever | Only measured IDs ship; each additional ID requires an empirical confirmation step recorded in the measurements doc before it is added |
| Snapshot hangs on a stuck collector | snapshot never completes | Per-collector 5 s deadline; the snapshot records `collector_timeout` for that entry |

## Security Considerations

- The update collector reads history only; it never calls `IUpdateInstaller` or any WUA write API
  (grep-asserted absence of `IUpdateInstaller`, `Download`, `Install`).
- Event-log rendering is fully bounded (64 KiB, 256 per pull) so a hostile or corrupt event cannot
  drive unbounded allocation (§82 malformed-data threat).
- Support URLs from WUA are stored as text and are never opened automatically; only the narrow
  enum-based reveal command exists, and it cannot take a URL.
- Snapshot payloads pass through the same seal path as events, so a driver `InfPath` under the user
  profile is normalized before persistence.
````

### FILE: phase-13-network-startup-eventlog-collectors.md

````markdown
---
phase: 13
title: "Network, Startup & Event Log Collectors"
status: pending
priority: P2
effort: 13h
dependencies: [12]
---

## Overview

The last collector phase: network configuration (§20 — DNS, IP, gateway, adapters, proxy, VPN, routes),
startup programs (§21 — Run keys, StartupApproved, Startup folder, scheduled tasks), and the
Application/System/Setup event-log channels (§22) including the crash correlation that upgrades a
process `stopped` event to `failed`.

## Requirements

Functional
- Network configuration via `GetAdaptersAddresses` (unicast addresses, DNS servers, gateways, adapter
  name/description, operational status, `IfType`) and `GetIpForwardTable2` for the default route.
  Change detection via `NotifyIpInterfaceChange`, `NotifyUnicastIpAddressChange`, and
  `NotifyRouteChange2` callbacks (research assumption #6 — callback safety from Rust is unverified:
  the callback must do nothing but push to a lock-free/`std::sync::mpsc` sender and return), with a
  5-minute reconcile poll as the safety net (measured full read ~1.4 s, so the poll is not free —
  reconcile at 5 minutes, never on a short timer).
- Events: `dns_changed`, `ip_changed`, `gateway_changed`, `adapter_state_changed`, `route_changed`,
  `proxy_changed`, `vpn_connected`, `vpn_disconnected` with §20 phrasing.
- DHCP-rotation suppression: an `ip_changed` where the new address is in the same /24 and the lease
  source is DHCP is recorded at `background` importance with description "IP address renewed"; a
  subnet change is `important`.
- VPN detection: adapter `IfType` in the tunnel set (`IF_TYPE_TUNNEL`, `IF_TYPE_PROP_VIRTUAL` —
  confirm constant names) or a description matching a documented pattern list; the pattern list is
  documented as heuristic in `docs/limitations.md`.
- Proxy: watch `HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings`
  (`ProxyEnable`, `ProxyServer`, `AutoConfigURL`) with the Phase 11 registry notify/poll mechanism.
- Startup programs: `HKCU`/`HKLM` `...\CurrentVersion\Run` and `RunOnce`, the Startup folder via
  `SHGetKnownFolderPath(FOLDERID_Startup)` and `FOLDERID_CommonStartup`, and
  `...\Explorer\StartupApproved\{Run,StartupFolder,Run32}` for enabled/disabled state. **The
  StartupApproved binary blob encoding is [UNVERIFIED]** — step 6 determines it empirically by toggling
  a test entry in Task Manager and dumping both blobs before any decoding logic is written.
- Scheduled tasks via Task Scheduler COM (`ITaskService` → `GetFolder` → `GetTasks` recursive),
  measured 194 tasks / 1.3 s ⇒ cadence: start (deferred 60 s) + hourly only. Events:
  `scheduled_task_created`, `scheduled_task_removed`, `scheduled_task_changed` (enabled state,
  trigger set, or action path).
- Event-log channels: Application (1000 application-crash and 1002 hang — **1000 is measured, 1002 is
  not; confirm empirically or omit**), System (already subscribed in Phase 12, extended with service
  control manager entries only if their IDs are empirically confirmed), and Setup.
  Security channel is **excluded** (measured unauthorized non-elevated).
- Crash correlation: an Application 1000 event naming an executable within 10 s of a
  `process_stopped` for the same executable upgrades that event to `process_failed` with the fault
  module in technical details, and writes a `caused_by`-style relationship. This removes the Phase 09
  temporary limitation registered in `TODO.md`.
Non-functional
- The 1.4 s network read and the 1.3 s task enumeration both run on `spawn_blocking` and never on a
  short timer; a reconcile in progress must not delay ingestion >50 ms.
- Win32 notification callbacks perform no allocation-heavy work, take no locks that a collector holds,
  and never call back into the runtime — enforced by a comment-documented invariant plus a review item.
- All notification registrations are cancelled on shutdown (`CancelMibChangeNotify2`) before the
  process exits, verified by a shutdown test that runs 20 start/stop cycles without a crash.
- Event-log subscriptions survive a channel that is temporarily unavailable and retry with backoff.

## Architecture

`platform/windows/{net_config.rs,net_notify.rs,startup.rs,taskscheduler.rs}` plus the Phase 12
`evtlog.rs`. `net_notify.rs` is the only place with an `extern "system"` callback; it holds a
`OnceLock<Sender<NetChangeHint>>` and its body is three lines. Hints are coalesced by
`collectors/network_config.rs` with a 2 s quiet period before a reconcile read, so a burst of
adapter events costs one 1.4 s read, not ten.

## Related Code Files

Create
- `src-tauri/src/platform/windows/{net_config.rs,net_notify.rs,startup.rs,taskscheduler.rs}`
- `src-tauri/src/collectors/{network_config.rs,startup.rs,event_log_app.rs}`
- `src-tauri/src/core/correlate/crash_correlation.rs`
- `src-tauri/tests/{net_diff.rs,startup_approved.rs,crash_correlation.rs,notify_shutdown.rs}`
- `docs/adr/0012-network-change-notification-strategy.md`

Modify
- `src-tauri/Cargo.toml` (`windows` features `Win32_NetworkManagement_IpHelper`,
  `Win32_UI_Shell`, `Win32_System_TaskScheduler` — confirm names)
- `src-tauri/src/collectors/registry.rs`, `src-tauri/src/core/normalize/{templates.rs,importance.rs}`
- `src/lib/copy/strings.ts` (§20/§21/§22), `docs/limitations.md`, `docs/windows-api-surface.md`
- `TODO.md` (remove the Phase 09 `process_stopped` upgrade limitation)

Delete — none.

## Implementation Steps

1. `net_config.rs`: `GetAdaptersAddresses` with the two-call size pattern and
   `GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_INCLUDE_ALL_INTERFACES`; walk the linked lists safely into an
   owned `Vec<AdapterInfo>` before any diffing (no pointer escapes the unsafe block).
   `GetIpForwardTable2` for the default route; free with `FreeMibTable`.
2. `net_notify.rs`: register the three notifications, store handles, cancel on drop. The callback pushes
   a `NetChangeHint` and returns immediately.
3. `collectors/network_config.rs`: hint coalescing with a 2 s quiet period, 5-minute reconcile,
   field-level diff, DHCP-rotation rule, VPN detection, proxy registry watch.
4. `startup.rs`: Run/RunOnce enumeration (HKCU + HKLM), Startup folder listing via
   `SHGetKnownFolderPath` (free with `CoTaskMemFree`), and raw StartupApproved blob reads.
5. Empirical StartupApproved determination (before writing any decoder): add a test entry, dump the
   blob hex for enabled and disabled states with a tiny `examples/dump_startup_approved.rs`, record
   both in `docs/measurements/phase-13-startup-approved.md`, then implement the decoder against the
   recorded bytes. If the encoding cannot be determined confidently, ship presence-only tracking and
   state that enabled/disabled state is not detected.
6. `taskscheduler.rs`: `ITaskService::Connect(empty variants)` → `GetFolder("\\")` → recursive
   `GetFolders`/`GetTasks`; read `Name`, `Path`, `Enabled`, `LastRunTime`, and the XML definition's
   trigger/action summary. Runs on the WUA-style dedicated COM thread pattern (reuse the helper).
7. `collectors/startup.rs`: hourly + start cadence, unified diff across the four sources, §21 phrasing.
8. `collectors/event_log_app.rs`: Application-channel subscription with an XPath filter for confirmed
   IDs only; extract `Faulting application name`, `Faulting module name`, and the exception code from
   `EventData` positions (documented as position-based with a fixture test).
9. `crash_correlation.rs`: a 10 s ring buffer of recent `process_stopped` events keyed by executable
   name; on a matching crash event, write the upgrade. If the stop event has already been persisted,
   write a correction row (`event_type = process_failed`) linked to the original rather than mutating
   history — history is append-only, and the timeline renders the correction in place of the original.
10. `tests/net_diff.rs`: fixture adapter lists → expected events, including the same-/24 DHCP case
    (background) vs a subnet change (important) and a VPN adapter appearing.
11. `tests/startup_approved.rs`: decode the recorded enabled/disabled blobs correctly; an unknown blob
    shape yields `Unknown` rather than a wrong answer.
12. `tests/crash_correlation.rs`: inject a stop + a 1000 event 2 s apart ⇒ a `process_failed`
    correction linked to the original; 30 s apart ⇒ no correlation.
13. `tests/notify_shutdown.rs`: 20 register/cancel cycles without a crash or handle growth.
14. Manual verification recorded in `docs/measurements/phase-13-manual.md`: toggle a DNS server on the
    active adapter (user-changeable non-elevated? if it requires admin, use a VPN connect/disconnect or
    Wi-Fi network switch instead and say which was used), add a Startup-folder shortcut, and confirm events.
15. Commits: `feat: add network configuration collector`, `feat: add startup program collector`,
    `feat: add application event log collector`, `feat: correlate crashes with process exits`.

## Todo

- [ ] adapter/route reads with owned data
- [ ] three change notifications + cancel on drop
- [ ] coalesced reconcile + DHCP/VPN/proxy rules
- [ ] startup sources incl. empirically determined StartupApproved
- [ ] scheduled tasks at hourly cadence
- [ ] Application channel + crash correlation (Phase 09 limitation removed)
- [ ] net/startup/crash/shutdown tests + manual record

## Success Criteria

- [ ] Connecting/disconnecting a VPN or switching networks produces the §20 events within 10 s
- [ ] An `ip_changed` inside the same /24 is `background`; a subnet change is `important`
- [ ] Adding a shortcut to the Startup folder produces `startup_item_added` within one cycle
- [ ] StartupApproved enabled/disabled decoding matches the recorded blobs, or presence-only is
      shipped with the limitation documented
- [ ] A crash 2 s after a process exit yields a `process_failed` correction linked to the original
- [ ] 20 register/cancel cycles leave handle count stable
- [ ] Security channel appears nowhere in the subscription list (grep-asserted)

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test net_diff
cargo test --manifest-path src-tauri/Cargo.toml --test startup_approved
cargo test --manifest-path src-tauri/Cargo.toml --test crash_correlation
cargo test --manifest-path src-tauri/Cargo.toml --test notify_shutdown
rg -n '"Security"' src-tauri/src | wc -l   # must print 0
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Win32 notification callback from Rust is unsafe in practice (assumption #6) | crash or deadlock during network changes | Callback body is send-and-return only; if instability persists, drop the callbacks entirely and rely on the 5-minute reconcile, documented in ADR 0012 |
| StartupApproved blob misdecoded | items shown enabled when disabled | Empirical determination precedes any decoder; unknown shapes report `Unknown`, never a guess |
| Task Scheduler COM enumeration throws on a protected folder | collector errors hourly | Per-folder error is caught, counted, and skipped; the collector reports `Degraded` with the folder count skipped |
| Position-based EventData extraction breaks on a locale or version variant | crash correlation silently stops | Fixture test plus a fallback that matches on the `<Data>` element order *and* a name attribute when present; a miss produces the crash event without correlation, never a wrong pairing |
| Reconcile poll cost (1.4 s) accumulates | idle CPU rises | 5-minute cadence with coalescing; measured contribution recorded in `docs/measurements/phase-13-costs.md` |
| Mutating history to upgrade an event corrupts the append-only model | audit finds updated rows | Corrections are new linked rows; a test asserts `events` rows are never `UPDATE`d (grep for `UPDATE events` returning only the retention/redaction-free columns) |
| DNS change needs admin, blocking manual verification | cannot reproduce the §20 story | Use a VPN or Wi-Fi switch and record which action was used; never claim a verification that did not happen |

## Security Considerations

- Adapter data, proxy strings, and task action paths are all sealed before persistence; proxy URLs can
  carry credentials (`http://user:pass@host`) and are covered by the Phase 02 URL-userinfo pattern —
  a leak-test case for exactly that shape is added here.
- The Security event-log channel stays excluded, which is both a measured limitation and a privacy
  benefit (it carries logon and audit data the product does not need).
- Scheduled-task XML is parsed with the same bounded scanner as event XML; no XML entity expansion is
  possible because no XML parser with entity support is used.
````

### FILE: phase-14-checkpoints-comparison.md

````markdown
---
phase: 14
title: "Checkpoints & Comparison"
status: pending
priority: P2
effort: 11h
dependencies: [13]
---

## Overview

§69 Phase 4: the "it worked here" marker (§24, §47). A checkpoint is a named snapshot plus a time
anchor; comparison answers "what changed since then" using the real collector snapshots from Phase 12
and the event history from Phase 03.

## Requirements

Functional
- `create_checkpoint(name, note)`: captures a full snapshot set (all collectors, Phase 12 service),
  stores `checkpoints` row with `created_at`, `name`, `note`, `snapshot_id`, and an `auto` flag.
- Automatic checkpoints: on first run, and before a detected Windows-update reboot when one is
  observable — both marked `auto = 1`, both counted against a cap of 10 auto checkpoints (oldest pruned).
- Manual checkpoints: unlimited, never pruned by retention; their snapshots are retention-exempt (§25).
- `rename_checkpoint(id, name)`, `delete_checkpoint(id)` (with a confirm step in the UI and snapshot
  cleanup), `list_checkpoints()` returning §47 card data: name, relative age, and the
  "12 important changes since this checkpoint" count computed at `notable`+ importance.
- `compare_checkpoint(id)` and `compare_checkpoints(a, b)`: returns per-category groups of
  `{ added, removed, changed }` derived from snapshot diffing, plus the event list in the interval,
  plus a `summary` sentence.
- Tray "Create checkpoint" performs a one-click checkpoint with an auto-generated name
  (`Checkpoint <date> <time>`) and shows the §53-compliant confirmation notification
  ("Checkpoint created. You can compare against this later.").
- Checkpoint names join the search index so searching a checkpoint name finds it.
- UI: Checkpoints screen with the §47 card list, a create dialog, a compare view with category groups
  and expandable entries, and all six states.

Non-functional
- Checkpoint creation completes ≤12 s worst case (10 s snapshot deadline + write) and shows progress;
  the UI stays responsive throughout.
- Comparison of two snapshots ≤300 ms at the 4 MiB cap; the interval event count query ≤200 ms at 1M rows.
- Deleting a checkpoint never deletes events, only its snapshot rows.
- A checkpoint referencing a pruned snapshot degrades to event-only comparison with the explanation
  "Detailed comparison is unavailable for this checkpoint." rather than an error.
## Architecture

`core/checkpoint/service.rs` owns creation and deletion; `core/checkpoint/compare.rs` is a pure
function over two snapshot payloads plus an event slice, which makes the whole comparison logic
testable from fixtures without touching the OS. Snapshot payloads are typed per collector
(`SnapshotPayload` enum), so the differ matches exhaustively and a new collector cannot be forgotten —
adding a variant breaks the match at compile time. That exhaustiveness is the guard against a
comparison that silently ignores half the system.

## Related Code Files

Create
- `src-tauri/src/core/checkpoint/{mod.rs,service.rs,compare.rs}`
- `src-tauri/src/commands/checkpoints.rs`
- `src-tauri/tests/{checkpoint_compare.rs,checkpoint_retention.rs}`
- `src-tauri/tests/fixtures/snapshots/{before.json,after.json}`
- `src/lib/stores/checkpoint-store.svelte.ts`
- `src/lib/components/checkpoints/{checkpoint-card.svelte,create-dialog.svelte,compare-view.svelte,category-group.svelte}`
- `src/routes/checkpoints.svelte` (replacing the Phase 04 placeholder)

Modify
- `src-tauri/src/core/snapshot/service.rs` (retention exemption for checkpoint-owned snapshots)
- `src-tauri/src/storage/{repo_checkpoints.rs,search_index.rs}`, `src-tauri/src/lib.rs` (tray handler)
- `src/lib/copy/strings.ts` (§24/§47 strings), `src/lib/tauri/bindings.ts` (regenerate)

Delete — none.

## Implementation Steps

1. `SnapshotPayload` enum with one variant per collector; snapshot service serializes into it (this
   replaces the opaque JSON blob from Phase 12 with a typed payload while keeping the same storage row).
2. `service.rs`: create (with progress reporting via a `checkpoint-progress` event reusing the existing
   typed event set — no new event name beyond the four unless the count is updated in `ipc_contract.rs`),
   rename, delete with snapshot cleanup, auto-checkpoint cap of 10.
3. `compare.rs`: exhaustive per-variant differ returning `CategoryDiff { category, added, removed, changed }`
   plus a summary sentence built from the largest three categories.
4. `commands/checkpoints.rs`: the five commands, each validating name length (1–64 chars) and rejecting
   duplicate names with a readable error.
5. Search index: insert checkpoint names on create/rename, remove on delete.
6. `checkpoint-card.svelte`: name, "created 3 days ago", the `notable`+ change count, and actions
   (Compare / Rename / Delete) with the delete confirm dialog.
7. `compare-view.svelte`: category groups collapsed by default with counts, expanding to entries;
   the six states including Partial ("Detailed comparison is unavailable for this checkpoint.").
8. Tray "Create checkpoint" wiring plus the §53 notification (subject to the rate limit and denylist).
9. `tests/checkpoint_compare.rs`: fixture before/after snapshots produce the expected per-category
   diff; a missing collector in the older snapshot is reported as "not tracked at that time", not as removed.
10. `tests/checkpoint_retention.rs`: a manual checkpoint's snapshot survives a retention pass that
    prunes a 30-day-old ad-hoc snapshot; an 11th auto checkpoint prunes the oldest auto one only.
11. Commits: `feat: add typed snapshot payloads`, `feat: add checkpoint create and delete`,
    `feat: add checkpoint comparison`, `feat: add checkpoint UI and tray shortcut`.

## Todo

- [ ] typed `SnapshotPayload` per collector
- [ ] checkpoint create/rename/delete + auto cap
- [ ] exhaustive comparison differ
- [ ] five commands with validation
- [ ] checkpoint names searchable
- [ ] cards + compare view + six states
- [ ] tray quick-create + notification
- [ ] compare + retention tests

## Success Criteria

- [ ] Creating a checkpoint completes ≤12 s with visible progress and the UI stays responsive
- [ ] A card shows "N important changes since this checkpoint" matching a SQL-verified count
- [ ] Comparison groups changes by category and expands to individual entries
- [ ] Adding a collector variant without updating the differ fails to compile (exhaustiveness proven)
- [ ] Manual checkpoint snapshots survive retention; auto checkpoints cap at 10
- [ ] Searching a checkpoint name finds it
- [ ] Deleting a checkpoint leaves all events intact (row count asserted)

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test checkpoint_compare
cargo test --manifest-path src-tauri/Cargo.toml --test checkpoint_retention
cargo test --manifest-path src-tauri/Cargo.toml --test ipc_contract
bun run check && bun run test
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Snapshot capture blocks the UI | frozen window during creation | Creation runs entirely off-thread with progress events; a manual check during the gate confirms responsiveness |
| Comparison drowns the user in noise | hundreds of "changed" rows | Groups are collapsed by default, ordered by importance-weighted count, and background-updater entries are folded into a single "N background updates" row |
| Missing snapshot makes compare look broken | error state on an old checkpoint | Documented Partial state with the exact copy; event-only comparison still works |
| Auto checkpoints crowd out manual ones | user's named checkpoints disappear | Pruning touches `auto = 1` rows only; asserted by the retention test |
| A new collector is added later and comparison ignores it | silently incomplete diffs | The exhaustive match makes it a compile error — this is the mitigation, and the test documents it |
| Duplicate checkpoint names confuse comparison | two "before update" cards | Duplicate names rejected with "A checkpoint with that name already exists." |

## Security Considerations

- Snapshot payloads contain inventory data (app names, service paths, adapter names) already sealed by
  the redaction chokepoint; the compare path adds no new ingestion route.
- `delete_checkpoint` takes an id, never a path, and the snapshot cleanup deletes rows only — no
  filesystem deletion happens anywhere in this phase.
- Checkpoint names are user input rendered as text nodes and stored as bound parameters.
````

### FILE: phase-15-analysis-engine.md

````markdown
---
phase: 15
title: "Analysis Engine"
status: pending
priority: P1
effort: 16h
dependencies: [14]
---

## Overview

§69 Phase 5 and the intellectual core of the product: turn "Minecraft server won't start" into a
ranked, explained, honest set of candidate causes (§26–§31). Fully deterministic, no network, no
model. The §59 ranking fixture and the §73 NVIDIA fixture are the acceptance tests that decide whether
this product is credible.

## Requirements

Functional
- `analyze_problem(ProblemInput)` where input is `{ text, since_ms: Option<i64>, tz_offset_minutes: i32 }`.
- Problem parser: extracts (a) a time window from natural phrases ("this morning", "yesterday",
  "last night", "an hour ago", "since Friday", "3 days ago") using the caller's `tz_offset_minutes`
  (no timezone crate); (b) entity mentions matched against `entities` (exact, then case-insensitive
  substring, then token overlap ≥0.6); (c) a `ProblemKind` from a keyword table
  (`WontStart, Crashes, Slow, NoNetwork, PortConflict, BuildFails, AuthFails, AudioVideo, Unknown`).
  Default window when nothing is extractable: 48 hours, stated in the UI as "Looking at the last 48 hours".
- Candidate retrieval: events in the window at `notable`+ importance, plus events touching mentioned
  entities or entities within traversal depth 2 of them, capped at 2 000 candidates ordered by
  importance then recency; the cap being hit is reported in the result (`truncated: true`).
- Scoring — the §28 six factors, each producing a `ScoreContribution { factor, points, human_reason }`:
  1. Temporal proximity to the problem window (max 30) — closer to the reported onset scores higher,
     events after the onset score 0 unless the onset is fuzzy.
  2. Relationship strength to a mentioned entity (max 25) — direct edge > depth-2 > none.
  3. Semantic category match to `ProblemKind` (max 20) — e.g. `NoNetwork` ↔ network/DNS categories.
  4. Severity/importance (max 10).
  5. Rarity (max 10) — how unusual this event type is for this machine over 30 days.
  6. Failure proximity (max 5) — an observed failure/crash event near it in time.
- Confidence: `High ≥70`, `Medium 45–69`, `Low <45`, mapped to the §29 hedged language; `Low` results
  are labelled "Possibly unrelated" and are never described as causes.
- Causal chains: build from the top candidates via entity relationships; each link is
  `Confirmed` (shared entity identity, e.g. the port and the process that held it), `Likely`
  (edge + tight timing), or `TemporalOnly` (timing alone). A chain containing a `TemporalOnly` link
  can never present as `Confirmed`.
- `ExplanationProvider` trait with `DeterministicExplanationProvider` as the only MVP implementation
  (§32 keeps AI out of the MVP); the trait exists so a provider can be added later without touching
  the engine, and a test asserts no network call exists in the crate (grep for `reqwest`/`http`).
- Result shape: `{ candidates: Vec<RankedCause>, chains: Vec<CausalChain>, window, truncated,
  previous_occurrences }` where each `RankedCause` carries the event, score, confidence,
  contributions, and the rendered explanation strings.
- "Has this happened before": count matching past occurrences of the top candidate's event signature
  in the retained history and render §31 phrasing.

Non-functional
- Full analysis ≤1 s at 100 000 events, ≤2 s at 1M (asserted, release mode).
- Deterministic: the same DB and input produce byte-identical results across runs (asserted by hashing
  the serialized result twice with a shuffled candidate order in between).
- No user-visible output may contain a forbidden overclaiming phrase; `tests/no_forbidden_phrases.rs`
  scans generated explanations for "definitely", "certainly", "the cause is", "guaranteed", "will fix".
- Scoring is explainable: every point awarded has a `human_reason`; a test asserts the sum of
  contributions equals the total score for 500 generated cases.
## Architecture

```
ProblemInput ─▶ parse::problem ─▶ retrieve::candidates ─▶ score::six_factors ─▶ chain::build
                    │                    │                        │                  │
              window+entities+kind   ≤2000 events        contributions+total    typed links
                                                                   └──▶ ExplanationProvider ──▶ AnalysisResult
```

Every stage is a pure function taking data in and returning data out; only `retrieve` touches the
repository, and it does so through a read connection passed in. That shape is what makes the §59
fixture possible: the test seeds a DB, calls the pipeline, and asserts on ranking without any mocking.
Weights live in one `const` table in `score/weights.rs` with a comment per weight explaining the
behavior it protects, so tuning is a single-file, reviewable change.

## Related Code Files

Create
- `src-tauri/src/core/analysis/{mod.rs,parse.rs,retrieve.rs,chain.rs}`
- `src-tauri/src/core/analysis/score/{mod.rs,weights.rs,temporal.rs,relational.rs,semantic.rs,severity.rs,rarity.rs,failure.rs}`
- `src-tauri/src/core/analysis/explain/{mod.rs,provider.rs,deterministic.rs}`
- `src-tauri/src/commands/analysis.rs`
- `src-tauri/tests/{analysis_ranking.rs,analysis_determinism.rs,no_forbidden_phrases.rs,problem_parser.rs,analysis_perf.rs}`
- `src-tauri/tests/fixtures/scenarios/{minecraft_port.rs,nvidia_driver.rs,negative_control.rs}`
- `docs/analysis-scoring.md` (the weight table and its rationale, for user-facing transparency)

Modify
- `src-tauri/src/storage/repo_events.rs` (signature-occurrence query), `src-tauri/src/core/graph/traverse.rs`
- `src/lib/tauri/bindings.ts` (regenerate), `src/lib/copy/strings.ts` (§29/§31 hedged phrasing)

Delete — none.

## Implementation Steps

1. `parse.rs`: time-phrase table → `(start_ms, end_ms)` computed from `now_ms` and
   `tz_offset_minutes` (local midnight = `now - ((now + offset) mod 86_400_000)`); entity matching
   in three passes; `ProblemKind` keyword table. Returns a `ParsedProblem` with an
   `interpretation` string the UI shows back ("Looking at changes since yesterday 18:00").
2. `retrieve.rs`: the candidate query (window + importance + entity expansion) with the 2 000 cap and
   `truncated` flag; uses the Phase 06 query builder, adding no new SQL string assembly.
3. `score/weights.rs`: the six maxima above as named consts, each with a rationale comment.
4. One module per factor, each `fn score(&Candidate, &ParsedProblem, &Ctx) -> ScoreContribution`.
   `rarity.rs` uses a 30-day count of the same `event_type` + entity kind, computed once per analysis
   into a `HashMap` (not per candidate) to hold the latency budget.
5. `chain.rs`: from each top-5 candidate, walk relationships to the mentioned entity or to a failure
   event; classify each link; reject chains longer than 4 nodes; deduplicate chains sharing a head.
6. `explain/deterministic.rs`: renders §45 strings — "Why this matters", "Timing", the chain arrows,
   and the confidence sentence — from `RankedCause` data only, never from raw event text.
7. `commands/analysis.rs`: `analyze_problem`, `get_analysis_result(id)` if results are cached
   (cache in memory only, keyed by input hash, 8 entries), and `explain_ranking(id)` returning the
   contributions for the "Why this is ranked #1" panel.
8. `fixtures/scenarios/minecraft_port.rs`: seed the §59/§74 scenario — a `.env` config change, a Docker
   container restart taking port 25565, and, 40 minutes earlier, a Chrome update. Assert the config →
   container → port chain occupies rank #1 and that the Chrome update ranks below every chain member.
9. `fixtures/scenarios/nvidia_driver.rs`: seed the §73 scenario — an NVIDIA driver update 581.29 →
   582.11 three hours before a reported game crash, plus five background events. Assert rank #1,
   `Confidence::High`, and that the rendered explanation contains the driver version transition.
10. `fixtures/scenarios/negative_control.rs`: a window with only background updater noise must return
    zero `High`/`Medium` candidates and the §26 honest empty answer
    ("We didn't find anything that clearly explains this.").
11. `tests/analysis_determinism.rs`: run twice with a shuffled retrieval order; serialized results must
    be byte-identical.
12. `tests/problem_parser.rs`: 20 phrasings → expected windows, including a DST-agnostic check that the
    offset is applied additively and a "no time phrase" case defaulting to 48 h.
13. `tests/analysis_perf.rs` (`#[ignore]`, release): 100k and 1M seeds, assert ≤1 s and ≤2 s, print timings.
14. Write `docs/analysis-scoring.md` from the weight table (kept in sync by a test that greps the consts
    and asserts each appears in the doc).
15. Commits: `feat: add problem parser`, `feat: add candidate retrieval`,
    `feat: add six-factor scoring with contributions`, `feat: add causal chain builder`,
    `feat: add deterministic explanation provider`, `test: add ranking and determinism fixtures`.

## Todo

- [ ] problem parser (time, entities, kind) + interpretation string
- [ ] candidate retrieval with cap and truncation flag
- [ ] six factor modules + single weight table
- [ ] confidence mapping + hedged language
- [ ] chain builder with typed link confidence
- [ ] `ExplanationProvider` + deterministic impl
- [ ] §59 Minecraft fixture, §73 NVIDIA fixture, negative control
- [ ] determinism, parser, forbidden-phrase, perf tests
- [ ] `docs/analysis-scoring.md` in sync with the consts

## Success Criteria

- [ ] §59 fixture: the config → container → port chain ranks #1 above the Chrome update
- [ ] §73 fixture: the NVIDIA driver update ranks #1 with `Confidence::High` and the version transition
      appears in the explanation
- [ ] Negative control returns the honest "nothing clearly explains this" answer, not a weak guess
- [ ] Contributions sum to the total score for 500 generated cases
- [ ] No forbidden overclaiming phrase appears in any generated explanation
- [ ] Two identical runs produce byte-identical results
- [ ] ≤1 s at 100k events, ≤2 s at 1M (recorded)
- [ ] No HTTP/network dependency exists in the crate (grep-asserted)

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test analysis_ranking -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --test analysis_determinism
cargo test --manifest-path src-tauri/Cargo.toml --test problem_parser
cargo test --manifest-path src-tauri/Cargo.toml --test no_forbidden_phrases
cargo test --manifest-path src-tauri/Cargo.toml --release --test analysis_perf -- --ignored --nocapture
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Weight tuning turns into whack-a-mole | fixing one fixture breaks another | Both scenario fixtures plus the negative control run in the same gate; a change must keep all three green. Weights live in one file so a tuning commit is reviewable in isolation |
| Ranking looks arbitrary to the user | "why is this #1" has no answer | Contributions with `human_reason` are mandatory and surfaced in Phase 16's panel; the sum test prevents hidden adjustments |
| Time parsing is wrong across midnight or DST | window off by hours | Offset supplied by the caller and applied additively; parser tests include boundary cases; the interpretation string always shows the resolved window so a user can see a mistake |
| Candidate cap hides the real cause | `truncated: true` on a busy window | The UI states "We looked at the 2000 most relevant changes" and offers narrowing by category; the cap is never silent |
| Rarity factor punishes a fresh install | everything is "rare" in week 1 | Rarity scores 0 when the history is shorter than 7 days, and the reason says "not enough history to judge rarity" |
| Chain builder invents causality | `Confirmed` on a timing-only pair | `Confirmed` requires entity identity in code; a test asserts a timing-only fixture never yields `Confirmed` |
| 1M-event analysis exceeds 2 s | perf assertion fails | Precompute the rarity map, add the covering index used by retrieval, and if still failing, narrow the default window to 24 h and say so in the interpretation string |

## Security Considerations

- The analysis engine reads sealed data only; it cannot re-read the filesystem or registry, so no new
  ingestion path exists here (enforced by `tests/layering.rs`: `core/analysis` may not import `platform`).
- Problem text is treated as untrusted input: length-capped at 500 chars, used only for parsing and
  bound-parameter matching, never interpolated into SQL and never rendered as HTML.
- The `ExplanationProvider` trait's MVP implementation performs zero I/O, which keeps §32's "no data
  leaves the machine" promise structurally true rather than merely intended.
````

### FILE: phase-16-investigation-experience.md

````markdown
---
phase: 16
title: "Investigation Experience"
status: pending
priority: P1
effort: 12h
dependencies: [15]
---

## Overview

The §43 home input and §45 answer layout — the screen that has to make a stranger trust the product in
ten seconds. Exact microcopy from §72–§74, honest confidence language from §29, and the "Why this is
ranked #1" transparency panel that turns the scoring engine into something a skeptic can audit.

## Requirements

Functional
- Home (§43): the large question input with placeholder "What stopped working?", example chips
  ("Minecraft server won't start", "Docker container can't bind port", "Game crashes after update"),
  a "since" control (Auto / Today / Yesterday / Last 7 days / Custom), and a summary strip showing
  today's counts (total changes, important changes, last activity time).
- Submitting shows a Loading state with the §72 wording, then the answer layout.
- Answer layout (§45) in this order: the interpretation line ("Looking at changes since yesterday
  18:00."), the headline "We found 3 changes that may be related.", then per candidate:
  "#1 Strong match" / "#2 Possible match" / "#3 Weaker match" derived from confidence,
  the human title, "Why this matters" paragraph, "Timing" line, "[ View evidence ]",
  the causal chain rendered with ↓ arrows and per-link confidence labels, and
  "Show technical details" progressive disclosure.
- "Why this is ranked #1": a toggle revealing the six contributions as
  `factor — points — human_reason` rows, plus the total and the confidence threshold it crossed.
- Previous occurrences: "This has happened 3 times in the last 30 days." with a link that opens the
  Timeline filtered to those occurrences.
- Honest empty answer (§26): "We didn't find anything that clearly explains this." plus two concrete
  next steps (widen the window, add a watched location) — never a fabricated weak cause.
- Truncation notice when `truncated`: "We looked at the 2000 most relevant changes."
- `[ View evidence ]` opens the Phase 07 details drawer for that event, so evidence has one
  implementation, not two.
- All six states on Home and on the Answer view.

Non-functional
- Perceived latency: the answer view renders its skeleton within 100 ms of submit; the analysis result
  fills in when ready, with a progress hint after 1 s.
- Keyboard: `Enter` submits, `Escape` clears, `Tab` order follows visual order, every disclosure is a
  real `<button>` with `aria-expanded` (§62).
- Reduced motion: no chain-drawing animation when `prefers-reduced-motion` is set; content appears directly.
- Copy parity test: every §72–§74 string used here is asserted character-for-character against
  `strings.ts` in a test, so a reviewer can diff the spec against the code.
## Architecture

`investigationStore` (runes) holds `{ input, since, state, result, expanded }` and calls
`analyze_problem` through the typed bindings. Rendering is a pure function of `AnalysisResult` — the
frontend performs no scoring, no re-ranking, and no threshold logic; confidence labels come from the
Rust `Confidence` enum mapped through one lookup in `confidence-label.ts`. That keeps the Rust types the
single source of truth (§35) and makes a mismatch impossible to introduce accidentally in the UI.

## Related Code Files

Create
- `src/lib/stores/investigation-store.svelte.ts`
- `src/lib/components/investigate/{question-input.svelte,example-chips.svelte,since-control.svelte,summary-strip.svelte,answer-view.svelte,candidate-card.svelte,causal-chain.svelte,ranking-panel.svelte,empty-answer.svelte}`
- `src/lib/utils/confidence-label.ts`
- `src/routes/home.svelte` (replacing the Phase 04 placeholder)
- `tests/investigation-copy.test.ts`, `tests/investigation-states.test.ts`

Modify
- `src/lib/copy/strings.ts` (§43/§45/§72/§73/§74 strings), `src/lib/components/details/event-drawer.svelte`
  (open by id from a candidate), `src/lib/router.ts` (home is the default route)

Delete — none.

## Implementation Steps

1. `question-input.svelte`: autofocus on route entry, 500-char cap with a counter past 400,
   `Enter` submits, `Escape` clears; the placeholder and chips come from `strings.ts`.
2. `since-control.svelte`: Auto by default; a non-Auto choice passes `since_ms` explicitly and the
   interpretation line reflects the user's choice rather than the parser's guess.
3. `summary-strip.svelte`: `get_dashboard_summary` counts with the §43 phrasing; zero state reads
   "No changes recorded yet." rather than "0 changes".
4. `answer-view.svelte`: skeleton-first render, then the sections in §45 order; a 1 s progress hint
   ("Still looking…") that disappears on completion.
5. `candidate-card.svelte`: rank label from confidence, title, "Why this matters", "Timing",
   `[ View evidence ]` (opens the shared drawer), and the technical disclosure.
6. `causal-chain.svelte`: vertical chain with ↓ separators, per-link confidence chips
   (`Confirmed` / `Likely` / `Timing only`), and a static render under reduced motion.
7. `ranking-panel.svelte`: contribution rows, total, and the crossed threshold; collapsed by default,
   `aria-expanded` wired, remembered per session.
8. `empty-answer.svelte`: the §26 honest answer plus the two next-step actions, each a real button that
   performs the action (widen window re-runs the analysis; add-location opens Settings at that section).
9. `tests/investigation-copy.test.ts`: assert every §72–§74 string constant matches the spec text
   exactly (the expected strings are inlined in the test as the assertion source).
10. `tests/investigation-states.test.ts`: mount with each of the six states and assert the distinguishing
    element is present; assert the empty-answer path never renders a candidate card.
11. Manual pass recorded in `docs/measurements/phase-16-manual.md`: run the §73 and §59 seeded scenarios
    through the real UI at 900×600, keyboard-only, and with reduced motion enabled.
12. Commits: `feat: add investigation home input`, `feat: add ranked answer layout`,
    `feat: add ranking transparency panel`, `feat: add honest empty answer`,
    `test: assert investigation copy matches spec`.

## Todo

- [ ] question input + chips + since control
- [ ] summary strip from real counts
- [ ] answer view in §45 order with skeleton-first render
- [ ] candidate cards + evidence via the shared drawer
- [ ] causal chain with per-link confidence
- [ ] ranking transparency panel
- [ ] honest empty answer with working next steps
- [ ] copy parity + state tests + manual pass

## Success Criteria

- [ ] Typing "Minecraft server won't start" on the seeded §59 DB shows the config → container → port
      chain as "#1 Strong match"
- [ ] The §73 scenario shows the NVIDIA driver update at #1 with High confidence
- [ ] "Why this is ranked #1" lists six contributions whose points sum to the shown total
- [ ] With only background noise, the honest empty answer appears and no candidate card renders
- [ ] Copy parity test passes for every §72–§74 string
- [ ] Keyboard-only investigation is possible end to end; reduced motion removes all animation
- [ ] At 900×600 the answer view is fully readable without horizontal scroll

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test
bun run tauri dev   # manual: seeded scenarios, keyboard-only, reduced motion, 900x600
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| UI re-derives confidence and drifts from Rust | a card labelled Strong with a Medium result | One lookup table maps the Rust enum; a test asserts the mapping is total over the enum's variants exported by specta |
| Answer feels slow even at 900 ms | user perceives a hang | Skeleton within 100 ms plus the 1 s progress hint; latency is perceived, not just measured |
| Hedged language reads as evasive | "may be related" everywhere | High confidence uses direct-but-bounded phrasing ("This is the most likely cause") while still avoiding forbidden absolutes; the forbidden-phrase test remains the boundary |
| The empty answer looks like a failure | users think the app is broken | Empty answer includes the two working next steps and the count of changes actually examined |
| Chain rendering breaks with a single-node chain | empty arrows | Single-node chains render as a plain candidate with no chain section (asserted in a state test) |
| Copy drift during polish | reviewer finds a paraphrase | `investigation-copy.test.ts` is the mechanical guard and runs in the gate |

## Security Considerations

- The problem text is never persisted by default; if a future feature stores it, it must pass the seal
  path — a comment at the store boundary states this, and no persistence call exists in this phase.
- Evidence rendering reuses the Phase 07 drawer, which renders text nodes only (no `{@html}`).
- The "add watched location" next step opens Settings; it never receives a path from the analysis result,
  so an analyzed value cannot become a watched path without explicit user selection.
````

### FILE: phase-17-three-js-visualization.md

````markdown
---
phase: 17
title: "Three.js Visualization Layer"
status: pending
priority: P2
effort: 20h
dependencies: [16]
---

## Overview

§69 Phase 6 and §39's constraint: Three.js appears in exactly three places where spatial structure
communicates something text cannot — the ambient background field, the causal graph, and the system
map. Every one of them has a DOM equivalent that is always present, so the product never depends on
WebGL to be usable.

## Requirements

Functional
- Shared renderer lifecycle (`three` 0.186.0): one `WebGLRenderer` per canvas, created with
  `{ antialias: false, powerPreference: "low-power", alpha: true }`, device-pixel-ratio capped at 1.5,
  and a render loop that stops on `document.hidden`, window blur, minimize, and route change.
- Full disposal on unmount: geometries, materials, textures, render targets, controls, and
  `renderer.dispose()` + `forceContextLoss()`; a test asserts `renderer.info.memory.geometries === 0`
  and `textures === 0` after teardown.
- Reduced motion: render exactly one static frame and stop; no `requestAnimationFrame` loop at all.
- WebGL absent or context lost: show the DOM equivalent with the copy "Visual view unavailable on this
  system. Showing the list instead." — never a blank canvas, never an error dialog.
- Ambient field (background, Home + Timeline): ≤1 200 `Points` in a single `BufferGeometry` plus ≤64
  `LineSegments`, drifting slowly; density is driven by recent change volume (§40 — the visual carries
  information: more recent activity, denser field), capped so it never becomes noise.
- Causal graph (Investigation answer, §37): nodes as one `InstancedMesh` (≤300 instances), edges as one
  `LineSegments` buffer, hit-testing via `Raycaster` + `instanceId`, labels as a DOM overlay positioned
  from projected coordinates (no text geometry, no sprite atlas), and constrained `OrbitControls`
  (rotate + zoom clamped, no pan beyond bounds, damping on).
- System map (§38): deterministic layered layout (layer per entity kind: processes, ports, services,
  apps, files, containers, adapters) with stable positions across renders given the same entity set, so
  the map does not reshuffle between visits; clicking an entity opens an `EntityInspector` panel with
  its recent events and relationships.
- Always-present DOM equivalent: each visualization has a sibling `<details>`-based list with the same
  nodes and edges as text, rendered server-of-truth-first (visible by default when WebGL is
  unavailable; collapsed but present otherwise) so keyboard and screen-reader users lose nothing.
- Entity selection is shared state: selecting in the graph highlights in the DOM list and vice versa.

Non-functional
- ≤8 ms per frame and ≤12 draw calls per visualization at the stated caps (measured via
  `renderer.info.render.calls` and a frame-time sampler, recorded).
- Idle GPU/CPU: with the window visible and no interaction, the ambient field's contribution to process
  CPU is ≤1% (measured); with the window hidden, exactly 0 frames are rendered (asserted by a counter).
- No memory growth across 20 mount/unmount cycles (asserted: geometries/textures return to 0 each time).
- Bundle: `three` is imported through a single module (`src/lib/three/index.ts`) with named imports only,
  so tree-shaking works; the built bundle size is recorded and a >1.5 MB gzip total fails the gate.
## Architecture

`src/lib/three/` owns every Three.js touchpoint: `renderer.ts` (creation + capability probe),
`loop.ts` (visibility-aware RAF driver shared by all canvases so there is never more than one RAF
callback), `dispose.ts` (the teardown contract + assertions), `layout/layered.ts` (deterministic
positions), and one module per scene. Svelte components own no Three.js objects directly — they call
`mountScene(canvas, data)` and `destroyScene(handle)`, which keeps lifecycle bugs confined to one file.
Data comes from existing commands (`get_related_entities`, `get_system_map`, the analysis result); this
phase adds no new OS access and no new persistence.

## Related Code Files

Create
- `src/lib/three/{index.ts,renderer.ts,loop.ts,dispose.ts,capability.ts}`
- `src/lib/three/scenes/{ambient-field.ts,causal-graph.ts,system-map.ts}`
- `src/lib/three/layout/layered.ts`
- `src/lib/components/viz/{ambient-canvas.svelte,causal-graph-view.svelte,system-map-view.svelte,graph-dom-fallback.svelte,entity-inspector.svelte,label-overlay.svelte}`
- `src/routes/system-map.svelte` (replacing the Phase 04 placeholder)
- `tests/three-dispose.test.ts`, `tests/three-layout.test.ts`, `tests/viz-fallback.test.ts`
- `docs/measurements/phase-17-render.md`

Modify
- `src-tauri/src/commands/entities.rs` (`get_system_map` returning capped nodes/edges)
- `src/routes/{home.svelte,timeline.svelte}` (mount the ambient canvas), `src/lib/components/investigate/answer-view.svelte`
- `src/app.css` (canvas layering tokens only), `src/lib/copy/strings.ts`

Delete — none.

## Implementation Steps

1. `capability.ts`: probe by creating a throwaway canvas and requesting `webgl2` then `webgl`; cache the
   result; expose `hasWebGL(): boolean`. On `webglcontextlost`, flip a store so every canvas swaps to
   the DOM equivalent without a reload.
2. `renderer.ts`: the constrained renderer factory; explicit `setPixelRatio(Math.min(devicePixelRatio, 1.5))`
   and `setSize` from the container's `ResizeObserver`.
3. `loop.ts`: one RAF driver with a registry of scene tick functions; stops on `visibilitychange`,
   `blur`, and when the registry is empty; exposes a rendered-frame counter for tests.
4. `dispose.ts`: `disposeScene(scene, renderer, extras)` walking the object graph disposing geometries,
   materials (including material maps), and render targets, then `renderer.dispose()` +
   `forceContextLoss()`. Returns the post-teardown `renderer.info.memory` snapshot for assertions.
5. `ambient-field.ts`: one `BufferGeometry` with ≤1 200 positions, a `PointsMaterial` with a soft size,
   and a slow drift in a single attribute update per frame; density from a `changeVolume` prop.
6. `causal-graph.ts`: `InstancedMesh` (sphere geometry, 12 segments) with per-instance color for
   confidence, one `LineSegments` for edges, `Raycaster` hit-testing to `instanceId`,
   `OrbitControls` with `enablePan = false`, clamped `minDistance`/`maxDistance` and polar angle.
7. `label-overlay.svelte`: projects node positions to screen space each frame (only while the graph is
   visible) and renders absolutely positioned DOM labels — accessible, selectable, and zero GPU cost.
8. `layout/layered.ts`: deterministic layout — sort entities by `(kind, id)`, assign a layer per kind and
   an evenly spaced slot within the layer, seeded jitter from a hash of the id so the map looks organic
   but is reproducible. Unit-tested for stability across two runs with shuffled input.
9. `graph-dom-fallback.svelte`: the always-present `<details>` list of nodes with their edges as text,
   visible by default when WebGL is unavailable and collapsed (but in the DOM) otherwise.
10. `entity-inspector.svelte`: on selection, fetch the entity's recent events and relationships and show
    them with the same row component as the Timeline.
11. `system-map.svelte`: the route, with all six states (Loading, Empty "Nothing to map yet.", Error,
    Partial "Showing 300 of 812 entities.", Ready, Dense).
12. `tests/three-dispose.test.ts` (jsdom + a stubbed WebGL context, or `headless-gl`-free approach:
    assert against a fake renderer implementing the `info.memory` contract): 20 mount/unmount cycles
    leave geometries and textures at 0 and the RAF registry empty. If a real GL context is unavailable in
    the test environment, the test asserts the disposal *calls* against a spy renderer and the real
    `renderer.info` check is performed in the manual pass and recorded — stated plainly, not implied.
13. `tests/viz-fallback.test.ts`: with `hasWebGL()` stubbed false, the DOM equivalent is visible and no
    canvas is created.
14. Measure frame time and draw calls for all three scenes at their caps; record in
    `docs/measurements/phase-17-render.md` along with the gzip bundle size.
15. Commits: `feat: add shared three.js renderer lifecycle`, `feat: add ambient field visualization`,
    `feat: add causal graph with instanced nodes`, `feat: add deterministic system map`,
    `feat: add dom equivalents for visualizations`, `test: assert full three.js disposal`.

## Todo

- [ ] capability probe + context-loss handling
- [ ] constrained renderer + single shared RAF loop
- [ ] disposal contract + assertions
- [ ] ambient field driven by change volume
- [ ] causal graph (instanced + raycast + DOM labels + clamped controls)
- [ ] deterministic layered system map + entity inspector
- [ ] always-present DOM equivalents
- [ ] dispose / layout / fallback tests + render measurements

## Success Criteria

- [ ] Hiding the window renders exactly 0 frames (counter-asserted)
- [ ] 20 mount/unmount cycles leave geometries and textures at 0
- [ ] `prefers-reduced-motion` yields one static frame and no RAF loop
- [ ] With WebGL stubbed unavailable, every visualization shows its DOM equivalent and the app is fully usable
- [ ] ≤8 ms/frame and ≤12 draw calls per scene at the caps (recorded)
- [ ] System map positions are identical across two visits with the same entity set
- [ ] Clicking a graph node opens the inspector with that entity's real events
- [ ] Gzip bundle total ≤1.5 MB (recorded)

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test
bun run build && ls -la dist/assets | head -20
bun run tauri dev   # manual: hide/restore, reduced motion, 20 route switches, draw-call readout
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| WebGL contexts leak across route changes | context-lost warnings, rising GPU memory | One renderer per canvas with the disposal contract; the 20-cycle test is the gate. If leaks persist, switch to a single shared renderer with multiple viewports and record the change |
| Three.js pushes the bundle over budget | gzip total >1.5 MB | Named imports through one module; if still over, lazy-import the scene modules per route (dynamic `import()`), keeping the ambient field in the main chunk only |
| Ambient field becomes decorative noise (§39 violation) | reviewer cannot say what it shows | Density is bound to change volume and the legend line states "Density reflects recent change volume"; if that cannot be made legible, the field is removed rather than kept as decoration |
| WebGL unavailable in the test environment | dispose test cannot assert real `info.memory` | Assert against a spy renderer in CI-less local tests and perform the real check in the recorded manual pass — the limitation is stated in the measurements doc, not hidden |
| `OrbitControls` fights trackpad users | erratic zoom | Damping on, zoom speed reduced, clamped distances; keyboard alternatives (arrow keys rotate, +/- zoom) are wired for a11y |
| Large graphs exceed instance caps | Partial state at 300 nodes | Partial state names the truncation ("Showing 300 of 812 entities.") and the DOM equivalent lists all of them |
| Labels desync from nodes during zoom | text drifts off nodes | Labels project from the same matrix in the same frame as the render; a manual zoom check is in the gate |

## Security Considerations

- Visualizations consume already-sealed data through existing commands; no new IPC surface beyond
  `get_system_map`, which is added to `ipc_contract.rs` in the same commit.
- Entity labels are rendered as DOM text nodes (Svelte-escaped); no shader receives user-derived strings.
- No external asset, font, or texture is fetched at runtime — everything is generated procedurally, so
  the app makes zero network requests to render (consistent with §32).
````

### FILE: phase-18-docker-development-mode.md

````markdown
---
phase: 18
title: "Docker & Development Mode"
status: pending
priority: P2
effort: 13h
dependencies: [17]
---

## Overview

§69 Phase 7 (§23): Docker container tracking and development-project awareness — the features that make
the product feel written for the person who says "it worked in the container yesterday". Docker's real
behavior is untestable on this machine (verified absent, decision D3), so the plan verifies the absent
path for real and the present path through an injector, and says so honestly.

## Requirements

Functional
- Detection: check for the named pipe `\\.\pipe\docker_engine` existence before any connection attempt.
  Absent ⇒ the Docker section shows "Docker integration — Not installed", no error events, no log spam,
  no retry loop. This is the path that is actually verifiable here and it must be provably quiet.
- When present: connect via `bollard` 0.21.1 using its named-pipe constructor (the exact function name
  is **[UNVERIFIED]** — resolve from `cargo doc -p bollard` before writing it; do not guess), subscribe
  to the event stream, and reconcile with a full `list_containers` on connect and every 5 minutes.
- Events: `container_started`, `container_stopped`, `container_created`, `container_removed`,
  `container_port_changed`, `image_pulled`, `image_removed`, `compose_project_changed`.
- Container facts captured: id (short), name, image (repo:tag), state, published port mappings,
  compose project/service labels, and **environment variable key names only — never values** (a test
  asserts a seeded `SECRET=...` value never appears anywhere).
- Port linkage: a published container port creates a `container --publishes--> port` relationship, which
  is what lets the §23/§74 chain (config → container → port conflict) resolve.
- Reconnection: on stream error, backoff 5 s → 15 s → 1 m → 5 m (capped), with the collector reporting
  `Degraded` and exactly one log line per transition, never per attempt.
- Development projects: user-designated project roots (from Settings). Per project, detect
  git state by reading `.git/HEAD`, `.git/refs/heads/*`, and `.git/packed-refs` directly (no `gix`);
  branch changes and commit-hash changes produce `git_branch_changed` / `git_commit_changed`.
- Dirty-state via `git status --porcelain` with a **fixed argv** (`["status", "--porcelain"]`, `cwd` set
  to the project root, no shell), a 5 s timeout, and automatic disable after 2 consecutive timeouts with
  the reason surfaced in Collector Status.
- Manifest/lockfile awareness: `package.json`, `bun.lock`, `package-lock.json`, `Cargo.toml`,
  `Cargo.lock`, `requirements.txt`, `pyproject.toml`, `go.mod`, `pom.xml`, `build.gradle` — a change
  produces `dependency_manifest_changed` with the changed dependency keys (values redacted through the
  Phase 10 differ, which is reused, not reimplemented).
- Runtime version probes: `node`, `bun`, `python`, `java`, `go`, `rustc`, `docker` — fixed argv with
  `--version`, 3 s timeout, results cached 1 hour; a version change produces `runtime_version_changed`
  (this is how "it worked before I updated Node" becomes answerable).
- Dev-mode section in Settings lists detected projects with their detected stack and last activity.
Non-functional
- Docker absent: zero events, zero log lines beyond one startup line, zero CPU beyond the pipe check
  (a test asserts the log file contains exactly one Docker-related line after 60 s).
- Every external process is spawned with a fixed argv, `cwd` set explicitly, `stdin` null, output capped
  at 256 KiB, and a timeout — no `cmd /c`, no shell string anywhere (grep-asserted).
- Git file reads never exceed 1 MiB (`packed-refs` can be large; it is read line-wise with a cap).
- Project scanning is bounded: ≤20 projects, and manifest polling is 60 s (file watching for projects
  reuses the Phase 10 watcher when the project is inside a watched location).

## Architecture

`platform/docker/` (pipe probe + `bollard` client + event mapping) and `core/devmode/`
(git reader, manifest detection, runtime probes) are separate: Docker is an optional external service,
dev-mode is local file reading. `core/exec/guarded_command.rs` is the single place any child process is
spawned — fixed argv, timeout, output cap, and a denylist assertion that the program is one of the seven
known probes or `git`. Everything else in the codebase is forbidden from using `std::process::Command`
(enforced by `tests/layering.rs`).

## Related Code Files

Create
- `src-tauri/src/platform/docker/{mod.rs,probe.rs,client.rs,events.rs}`
- `src-tauri/src/core/devmode/{mod.rs,git_state.rs,manifests.rs,runtime_probe.rs,projects.rs}`
- `src-tauri/src/core/exec/guarded_command.rs`
- `src-tauri/src/collectors/{docker.rs,devmode.rs}`
- `src-tauri/tests/{docker_absent.rs,docker_injected_chain.rs,git_state.rs,guarded_command.rs,no_shell_exec.rs,docker_env_keys_only.rs}`
- `src-tauri/tests/fixtures/git/{head_branch,head_detached,packed-refs}`
- `src/lib/components/settings/{docker-section.svelte,projects-section.svelte}`
- `docs/adr/0013-docker-verification-strategy.md`

Modify
- `src-tauri/Cargo.toml` (`bollard = "0.21.1"` with the named-pipe feature — confirm the feature name),
  `src-tauri/src/collectors/registry.rs`, `src-tauri/src/core/normalize/templates.rs`
- `src/lib/copy/strings.ts` (§23 strings), `docs/limitations.md`

Delete — none.

## Implementation Steps

1. `core/exec/guarded_command.rs`: `run(program: KnownProgram, args: &[&str], cwd: Option<&Path>) -> Result<Output>`
   where `KnownProgram` is an enum — an arbitrary program name is not representable. Timeout via
   `tokio::time::timeout` over `Command::output()`; output truncated at 256 KiB.
2. `tests/no_shell_exec.rs`: grep for `cmd /c`, `cmd.exe`, `powershell`, `sh -c`, `Command::new(` outside
   `guarded_command.rs` — all must be zero.
3. `docker/probe.rs`: pipe existence check via `std::fs::metadata(r"\\.\pipe\docker_engine")` (verify this
   works for named pipes; if not, use `CreateFileW` with `OPEN_EXISTING` and close immediately — resolve
   which works empirically in this step and record it).
4. `docker/client.rs`: resolve `bollard`'s named-pipe constructor from `cargo doc -p bollard`
   (candidates include a `connect_with_named_pipe*` family — **confirm before writing**), then event
   subscription and `list_containers` reconciliation.
5. `docker/events.rs`: map Docker event types to the product's event types; extract port mappings and
   labels; env handling extracts **key names only** at the mapping boundary so a value never enters a
   `RawEvent`.
6. `collectors/docker.rs`: absent path (quiet), present path (stream + reconcile), backoff, health.
7. `core/devmode/git_state.rs`: parse `.git/HEAD` (`ref: refs/heads/main` or a detached hash),
   resolve the ref from `refs/heads/*` or `packed-refs`, all with caps; no external process for this.
8. `runtime_probe.rs`: the seven probes through `guarded_command`, parsed with a tolerant version
   extractor (first `\d+\.\d+(\.\d+)?` in the output), cached 1 hour, absent tools recorded as `None`
   without an error event.
9. `manifests.rs`: reuse the Phase 10 differ for JSON/TOML/text manifests; emit changed dependency keys.
10. `collectors/devmode.rs`: project polling, git-state diffing, dirty-state with the timeout/disable
    rule, manifest and runtime events.
11. `tests/docker_absent.rs`: with the pipe absent (the real state of this machine), run the collector for
    60 s (compressed via injected time), assert zero events and exactly one log line.
12. `tests/docker_injected_chain.rs`: feed a recorded Docker event sequence (hand-written fixture JSON
    matching the Docker events API shape) through `events.rs` into the normalizer, and assert the §23/§74
    chain resolves: a config change, then a container restart, then a port publish conflicting with an
    existing listener. This is how the feature is verified without Docker installed; ADR 0013 records that
    the real-daemon path remains **unverified on this machine** and lists what to re-run when Docker exists.
13. `tests/docker_env_keys_only.rs`: an injected container with `SECRET=hunter2` yields an event
    containing `SECRET` and never `hunter2` (DB + log asserted).
14. `tests/git_state.rs`: the three fixtures resolve correctly; a corrupt `HEAD` yields `Unknown`, not an error.
15. `tests/guarded_command.rs`: a probe that sleeps past the timeout is killed and reported; two timeouts
    disable dirty-state checks.
16. Commits: `feat: add guarded external command runner`, `feat: add docker presence probe`,
    `feat: add docker collector with reconnection`, `feat: add git and manifest awareness`,
    `feat: add runtime version tracking`, `test: verify docker chain via injected events`.

## Todo

- [ ] guarded command runner + no-shell test
- [ ] pipe probe (empirically chosen mechanism)
- [ ] `bollard` constructor resolved from docs, event stream + reconcile
- [ ] env keys only (test-proven)
- [ ] git state reader (no external process)
- [ ] manifest diffing via the Phase 10 differ
- [ ] runtime version probes with caching
- [ ] absent-path quiet test + injected chain test + ADR 0013

## Success Criteria

- [ ] With Docker absent (this machine), 60 s of runtime produces zero events and one log line
- [ ] Settings shows "Docker integration — Not installed" with no error styling
- [ ] The injected §23/§74 sequence produces the config → container → port chain
- [ ] An injected container env value never appears in the DB or logs; its key does
- [ ] `no_shell_exec.rs` passes (no shell invocation anywhere)
- [ ] A hung `git status` is killed at 5 s and disables itself after the second timeout
- [ ] Switching a project's git branch produces `git_branch_changed` within one poll
- [ ] Changing the Node version produces `runtime_version_changed` on the next probe

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test docker_absent
cargo test --manifest-path src-tauri/Cargo.toml --test docker_injected_chain
cargo test --manifest-path src-tauri/Cargo.toml --test docker_env_keys_only
cargo test --manifest-path src-tauri/Cargo.toml --test git_state
cargo test --manifest-path src-tauri/Cargo.toml --test no_shell_exec
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `bollard` named-pipe constructor differs from expectation | compile error in `client.rs` | Resolve from `cargo doc -p bollard` and the crate's Windows example; if named-pipe support requires a feature flag, add it and record the resolved name in ADR 0013 |
| Real Docker behavior differs from the injected fixtures (D3) | untestable here | ADR 0013 states the daemon path is unverified, lists the exact re-verification steps, and `docs/limitations.md` repeats it. The absent path — the only one testable here — is covered by a real test |
| Docker absence produces retry spam | log grows during a 60 s idle run | The probe gates all connection attempts; the absent test is the guard |
| `git status` hangs on a huge repo | dirty-state latency | 5 s timeout, disable after 2, reason shown in Collector Status |
| Env values leak through labels or command fields | `docker_env_keys_only` fails | Mapping extracts a fixed allowlist of fields; `Cmd`/`Entrypoint` are dropped entirely, consistent with the no-command-lines rule from Phase 09 |
| Runtime probes spawn on every tick | CPU spikes every poll | 1-hour cache with an explicit "probe now" action in Settings only |
| `packed-refs` in a large monorepo is huge | memory spike | Line-wise read with a 1 MiB cap; over the cap, branch resolution degrades to `Unknown` with a documented reason |

## Security Considerations

- `KnownProgram` makes arbitrary execution unrepresentable in the type system — the strongest available
  mitigation for the §82 "shell injection through a project path" threat, and the reason `Command::new`
  is banned outside one file.
- Container environment values are dropped at the mapping boundary, before `RawEvent` construction, so
  they cannot reach even the pre-seal stage.
- The Docker pipe is a local IPC endpoint; the app only reads events and lists containers — no create,
  start, stop, or exec API is imported (grep-asserted absence of `create_container`, `start_container`,
  `exec`).
- Project roots are validated with the Phase 10 watch-location validator, so a project path cannot
  redirect reads outside the user's chosen scope.
````

### FILE: phase-19-onboarding-settings-notifications.md

````markdown
---
phase: 19
title: "Onboarding, Settings & Notifications"
status: pending
priority: P1
effort: 12h
dependencies: [18]
---

## Overview

§69 Phase 8 begins: the surfaces that decide whether a first-run user trusts the app — §50 onboarding
with honest cost and privilege disclosure, §49 settings with per-toggle what/why/cost/admin columns
sourced from the measured capability matrix, and §53 conservative notifications.

## Requirements

Functional
- Onboarding, exactly five steps with the §50 copy and default check states:
  1. Welcome — what the app does in one sentence, and the promise "Everything stays on this computer."
  2. What we watch — the collector list with per-item what/why/cost, defaults checked for the cheap
     collectors (process, services, apps, updates, network, startup, drivers, event log) and
     **unchecked** for file watching (no locations yet) and Docker/dev mode (opt-in).
  3. Watched locations — optional; an add-location picker using the Tauri dialog plugin
     (`tauri-plugin-dialog` 2.4.0 — confirm the pinned version from the research table before adding),
     with the validator's rejection reasons shown inline.
  4. What requires administrator — the honest list from the measured matrix: full process paths
     (~46% unavailable), process start/stop precision, and the Security event log (excluded). Wording
     states the app runs without administrator by design and what that costs.
  5. Ready — "Monitoring is on. Come back when something breaks." with a "Create first checkpoint" action.
- Onboarding completion is persisted; re-running it is possible from Settings.
- Settings, eight sections per §49: Monitoring (per-collector toggles with what/why/cost/admin),
  Watched locations, Development (projects, Docker), Notifications (rules + denylist view), Data
  (retention options 7/30/90 days + "keep important events longer" + a live storage estimate + Export +
  Reset), Appearance (Dark/Light/System, density, reduced motion override), Advanced (developer mode,
  verbosity, diagnostics link), AI assistance.
- AI section: a single explanatory panel stating no AI provider ships in the MVP (§32 "later"), the
  toggle is present but disabled with the reason, and the copy promises that enabling it in a future
  version will be explicit and off by default. **Zero network code exists** (the Phase 15 grep test
  covers the crate).
- Storage estimate: computed from actual average row size × retained events, refreshed on section open.
- Notifications (§53): `tauri-plugin-notification` 2.4.0 (confirm the pin); rules — only
  `important`/`critical` events, at most one notification per hour, never during the first 10 minutes
  after app start, never for events matching the updater denylist, and never more than one per collector
  per hour. A notification click opens the app at the relevant event.
- Notification settings: master toggle (default on), per-category opt-out, and a "Test notification" button.
- Toggling a collector off stops it immediately and records a system event; toggling on starts it.

Non-functional
- Settings changes apply without an app restart, and every change is persisted before the UI confirms.
- The what/why/cost/admin data is generated from a single Rust-side table (`collector_catalog.rs`) so the
  UI cannot drift from reality; the cost column carries the measured numbers from the research report.
- Notification rate limiting is enforced in Rust, not the UI, and is unit-tested with injected time.
- All six states on Onboarding and Settings (Settings' Error state covers a failed persist).
## Architecture

`core/settings/` owns a typed `Settings` struct persisted as one JSON row in `settings` (not a
key-value soup), versioned with a `schema_version` so a future field addition has a defined default.
`core/catalog/collector_catalog.rs` is the single table of `{ id, display_name, what, why,
cost_note, requires_admin, default_enabled }`, exported to TypeScript by specta — the onboarding and
settings UIs render from it, so a new collector appears in both automatically. `core/notify/rules.rs`
holds the rate limiter and eligibility check, both pure functions over `(event, state, now)`.

## Related Code Files

Create
- `src-tauri/src/core/settings/{mod.rs,model.rs,store.rs}`
- `src-tauri/src/core/catalog/collector_catalog.rs`
- `src-tauri/src/core/notify/{mod.rs,rules.rs,dispatch.rs}`
- `src-tauri/src/commands/{onboarding.rs,notifications.rs}`
- `src-tauri/tests/{settings_roundtrip.rs,notification_rules.rs,catalog_completeness.rs}`
- `src/lib/stores/settings-store.svelte.ts`
- `src/lib/components/onboarding/{onboarding-flow.svelte,step-welcome.svelte,step-collectors.svelte,step-locations.svelte,step-admin.svelte,step-ready.svelte}`
- `src/lib/components/settings/{settings-nav.svelte,monitoring-section.svelte,notifications-section.svelte,data-section.svelte,appearance-section.svelte,advanced-section.svelte,ai-section.svelte,collector-toggle.svelte}`
- `src/routes/{onboarding.svelte,settings.svelte}` (replacing the Phase 04 placeholders)
- `tests/onboarding-copy.test.ts`, `tests/settings-persist.test.ts`

Modify
- `src-tauri/Cargo.toml` + `src-tauri/capabilities/default.json` (dialog + notification plugin permissions,
  scoped to exactly what is needed), `src-tauri/src/collectors/registry.rs` (enable/disable at runtime)
- `src/lib/copy/strings.ts` (§49/§50 strings), `src/lib/tauri/bindings.ts` (regenerate)

Delete — none.

## Implementation Steps

1. `settings/model.rs`: the typed struct with `schema_version`, `Default` implementing the §50 default
   check states; `store.rs` loads, migrates (version bump = a named migration function), and saves atomically.
2. `collector_catalog.rs`: one entry per collector with the measured cost note (e.g. driver inventory
   "~2.1 s per sweep, runs hourly", process "6 ms per second") and the `requires_admin` flag drawn from
   the capability matrix. `tests/catalog_completeness.rs` asserts every registered collector id has a
   catalog entry and vice versa — a missing entry is a compile-time-adjacent failure.
3. Add the two plugins with minimal capabilities: dialog restricted to `open` (directory selection only),
   notification restricted to `notify`. No `-fs`, `-shell`, `-http` plugin is added (they remain banned).
4. `onboarding-flow.svelte`: five steps with progress, back/next, skip on optional steps, and the exact
   §50 copy from `strings.ts`; completing writes `onboarding_completed_at`.
5. `step-admin.svelte`: renders the honest privilege list from the catalog's `requires_admin` entries plus
   the two named exclusions (process start trace, Security channel) — the numbers come from the matrix,
   not from prose invented in the UI.
6. Settings sections, each reading and writing through `settings-store.svelte.ts`; `collector-toggle.svelte`
   shows what/why/cost and an "Administrator required for full detail" note where applicable.
7. `data-section.svelte`: retention radio group, "keep important events longer" checkbox, the storage
   estimate (bytes → human), Export, and Reset (Reset shows a typed-confirmation dialog).
8. `notify/rules.rs`: eligibility (`importance >= important`, not updater-denylisted, not within 10 min of
   start) plus the limiter (≤1/hour global, ≤1/hour per collector); `dispatch.rs` sends through the plugin
   and records that a notification was sent (so the limiter survives a restart).
9. `notifications-section.svelte`: master toggle, per-category opt-out, denylist display (read-only list of
   the suppressed updaters, so the behavior is visible rather than mysterious), and "Test notification".
10. `ai-section.svelte`: the explanatory panel with the disabled toggle and the §32 wording.
11. `tests/notification_rules.rs`: with injected time — a `critical` event 5 min after start is suppressed;
    two `important` events 10 min apart yield one notification; a Chrome update never notifies; the limiter
    state persists across a simulated restart.
12. `tests/settings_roundtrip.rs`: save → load → equal; an unknown future field is preserved (forward
    compatibility) and a missing field takes its default; a corrupt settings row falls back to defaults
    with a recorded warning rather than a crash.
13. `tests/onboarding-copy.test.ts`: the five steps' strings match §50 exactly.
14. Commits: `feat: add typed settings store`, `feat: add collector catalog`,
    `feat: add onboarding flow`, `feat: add settings sections`,
    `feat: add conservative notification rules`, `test: assert notification rate limiting`.

## Todo

- [ ] typed settings + versioned persistence
- [ ] collector catalog with measured costs + completeness test
- [ ] dialog + notification plugins with minimal capabilities
- [ ] five onboarding steps with exact copy and defaults
- [ ] eight settings sections incl. storage estimate
- [ ] AI section (disabled, honest)
- [ ] notification eligibility + limiter in Rust
- [ ] rules / roundtrip / copy tests

## Success Criteria

- [ ] First run shows the five §50 steps with the specified default check states
- [ ] The admin step lists exactly the measured limitations (no invented privilege claims)
- [ ] Toggling a collector off stops it immediately and persists across restart
- [ ] Storage estimate matches a SQL-computed size within 10%
- [ ] A `critical` event within 10 min of start produces no notification
- [ ] Chrome/Spotify updates never notify (test-proven)
- [ ] The AI toggle is present, disabled, and explains why; zero network code exists in the crate
- [ ] Every registered collector has a catalog entry (test-enforced)

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test notification_rules
cargo test --manifest-path src-tauri/Cargo.toml --test settings_roundtrip
cargo test --manifest-path src-tauri/Cargo.toml --test catalog_completeness
bun run check && bun run test
bun run tauri dev   # manual: first-run onboarding, toggle a collector, test notification
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Notification plugin version differs from the research pin | `cargo add` resolves a different version | Use the version the research report verified; if unavailable, take the nearest 2.x, record the resolved version in the phase notes, and re-check the permission identifiers against the plugin's shipped schema |
| Capability file grants more than needed | reviewer finds a broad permission | Capabilities list exact permission identifiers (`dialog:allow-open`, `notification:default` equivalents — confirm names from the plugin schema) and Phase 21 audits the file |
| Users enable everything and the app gets heavy | idle CPU rises after onboarding | Cost notes are shown per toggle *before* enabling, and the driver/task collectors state their sweep cost explicitly |
| Settings drift from collector reality | a toggle for a collector that no longer exists | `catalog_completeness.rs` fails the build on mismatch |
| Notification limiter resets on restart and spams | multiple notifications after repeated restarts | Limiter state is persisted; the roundtrip test covers the restart case |
| The disabled AI toggle looks like a bug | user tries to click it | The panel explains it in words and the toggle carries `aria-disabled` with the reason as its description |
| Storage estimate is wildly wrong | user distrusts the Data section | Estimate is computed from real average row size and validated against `page_count * page_size` within 10% in a test |

## Security Considerations

- Only two plugins are added, each with the narrowest permission set; `-fs`, `-shell`, and `-http` remain
  absent (Phase 21 asserts this by grepping `Cargo.toml` and the capability files).
- The dialog plugin returns a directory path chosen by the user; that path still passes the watch-location
  validator before anything is watched, so the picker is not a bypass.
- Reset requires typed confirmation and deletes the database only — never a user directory.
- The AI section's promise ("nothing leaves this computer") is backed by the crate-level absence of any
  HTTP client, which Phase 21 re-verifies as a release gate rather than trusting this phase's word.
````

### FILE: phase-20-diagnostics-recovery-release-plumbing.md

````markdown
---
phase: 20
title: "Diagnostics, Recovery & Release Plumbing"
status: pending
priority: P1
effort: 11h
dependencies: [19]
---

## Overview

The operational surfaces §83/§84 require: a diagnostics screen a user can safely share, the database
recovery experience (Repair / Export / Reset) wired to the Phase 03 engine, the updater with auto-check
off, and installer polish. This is the phase that makes the app supportable without a support team.

## Requirements

Functional
- Diagnostics screen (§84): app version, build profile, `tauri::webview_version()` (confirm the exact
  function name from `cargo doc -p tauri` before use — if absent, read the WebView2 client version from
  the registry and say which source was used), OS build, database path (shown as
  `%LOCALAPPDATA%\...` never the literal profile path), DB size, page count, WAL size, event/entity/
  relationship counts, per-collector health rows, dropped-event count, suppression counts, last error
  per collector (redacted), and the redaction engine's pattern-version.
- "Copy diagnostics" and "Save diagnostics to file" produce a redacted text report: no secrets (the
  redaction engine is applied to the whole document as a final pass), no raw user profile paths, no
  machine name, no account name. A test seeds a secret into a collector error and a user path into a
  watched location and asserts neither appears in the export.
- Recovery UI (§83): when startup detects corruption (`PRAGMA quick_check` failure), show a dedicated
  recovery screen — not the normal shell — with three actions: **Repair** (attempt `VACUUM INTO` a fresh
  file and swap), **Export** (copy the damaged file to a user-chosen location), **Reset** (rename the
  damaged DB aside and start fresh). Each action reports success or failure in words.
- Three-strike rule: `state.json` counts consecutive failed startups; on the third, the app boots straight
  into recovery mode with the message "The app had trouble starting three times. Choose how to continue."
  A successful startup resets the counter. This is what prevents a crash loop.
- Updater: `tauri-plugin-updater` 2.10.1 with **auto-check off**; a "Check for updates" button in Settings
  → Advanced, a visible current-version line, and an explicit user-initiated download+install.
- DB-preserving upgrade: an install over an existing version keeps the database. Verified by seeding 5 000
  events, running the migration path from the previous schema version to current, and asserting all rows
  and the settings row survive.
- Installer polish: Start-menu shortcut, optional desktop shortcut, "launch at login" as an opt-in
  written to `HKCU\...\Run` only when the user chooses it (and removed on uninstall), a clean uninstall,
  and an uninstall-time prompt asking whether to keep or delete the recorded history (NSIS
  `installerHooks` — confirm the key name and hook file contract against the shipped Tauri config schema
  before writing it).
- Log file management: daily rolling files in `%LOCALAPPDATA%\WhatChanged\logs`, 7-day retention, and an
  "Open logs folder" action using the narrow enum-based reveal command (no `-shell`, no `-opener`).

Non-functional
- Diagnostics generation ≤300 ms at 1M events (counts come from cheap queries or cached values, never a
  full scan).
- Recovery actions never destroy data silently: Reset renames the file with a timestamp suffix and tells
  the user where it went.
- The updater performs no network request unless the user clicks the button (asserted by the absent-network
  check during a 60 s idle run in Phase 21).
## Architecture

Recovery mode is a separate render path chosen in `lib.rs` before the main shell is built: if
`quick_check` fails or the strike count is 3, the window loads with a `mode=recovery` flag and the
frontend mounts `recovery.svelte` instead of the router. That separation matters — a corrupt database must
not be handled by UI that expects a working database. Diagnostics assembly lives in
`core/diag/report.rs` as a pure function over collected metrics, then passes through the redaction engine
as a final whole-document scrub, so the export path shares the Phase 02 chokepoint rather than
re-implementing sanitization.

## Related Code Files

Create
- `src-tauri/src/core/diag/{mod.rs,metrics.rs,report.rs}`
- `src-tauri/src/core/recovery/{mod.rs,actions.rs,strikes.rs}`
- `src-tauri/src/commands/{diagnostics.rs,recovery.rs,updater.rs}`
- `src-tauri/tests/{diagnostics_redaction.rs,recovery_actions.rs,upgrade_preserves_db.rs,strike_counter.rs}`
- `src/lib/components/diagnostics/{diagnostics-view.svelte,metric-row.svelte,collector-health-table.svelte}`
- `src/lib/components/recovery/{recovery-screen.svelte,action-card.svelte}`
- `src/routes/{diagnostics.svelte,recovery.svelte}`
- `src-tauri/installer/uninstall-hooks.nsh` (only if the schema confirms the hook contract)

Modify
- `src-tauri/Cargo.toml` + `src-tauri/tauri.conf.json` (updater plugin + endpoints placeholder + NSIS hooks),
  `src-tauri/capabilities/default.json` (updater permissions, user-initiated only)
- `src-tauri/src/lib.rs` (recovery-mode branch), `src-tauri/src/storage/recovery.rs` (wire the UI actions)
- `src/lib/copy/strings.ts` (§83/§84 strings), `docs/deployment-guide.md` or the repo's equivalent
  release doc (signing steps, env-var-only secrets)

Delete — none.

## Implementation Steps

1. `core/diag/metrics.rs`: cheap metric collection — `PRAGMA page_count`/`page_size`, cached row counts
   maintained by the writer (not `COUNT(*)` at 1M), WAL file size from the filesystem, collector health
   snapshot, dropped/suppression counters.
2. `report.rs`: assemble a plain-text report with sections; the final line of the function passes the whole
   string through the redaction engine's text scrubber and replaces the profile path with `%LOCALAPPDATA%`.
3. `commands/diagnostics.rs`: `get_diagnostics()` (structured, for the screen) and
   `export_diagnostics(target: DiagnosticsTarget)` where the target is an enum (`Clipboard | File`), with
   the file written through the dialog plugin's user-chosen path only.
4. `core/recovery/strikes.rs`: `state.json` with `consecutive_failed_starts`; incremented before DB open,
   reset after a successful open plus 30 s of running (so a crash 5 s in still counts).
5. `core/recovery/actions.rs`: `repair()` (`VACUUM INTO` a temp file, `quick_check` it, swap on success),
   `export_db(path)` (file copy of the main DB + WAL), `reset()` (rename to
   `events.corrupt-<epoch>.db` and create a fresh DB). Each returns a typed outcome the UI renders in words.
6. `lib.rs` recovery branch + `recovery.svelte` with the three action cards, the honest explanation, and a
   "Try starting normally" escape hatch.
7. Updater: add the plugin, set `"active": false` for auto-check in the config (confirm the exact config key
   from the plugin's shipped schema), and implement the Settings → Advanced button with progress and a
   failure path that says what went wrong.
8. `tests/upgrade_preserves_db.rs`: build a DB at migration version N-1 with 5 000 events and a settings
   row, run the migration path to current, assert row counts, a sample of event field values, and the
   settings content are intact; assert the pre-migration `VACUUM INTO` backup file exists.
9. `tests/diagnostics_redaction.rs`: seed a collector error containing `sk-ABC…`, a watched location under
   the real user profile, and a proxy URL with credentials; assert the exported report contains none of
   them and does contain the `%LOCALAPPDATA%` form.
10. `tests/recovery_actions.rs`: corrupt a DB file by overwriting bytes mid-file; assert `quick_check`
    detects it, `repair` either succeeds or fails cleanly, `export` produces a byte-identical copy, and
    `reset` renames rather than deletes (the old file still exists on disk afterwards).
11. `tests/strike_counter.rs`: three simulated failed starts route to recovery mode; a successful start
    plus 30 s resets the counter.
12. Installer: verify the NSIS hook contract against the shipped config schema
    (`bunx tauri --help` / the schema JSON in the tauri crate) before adding `uninstall-hooks.nsh`; if the
    contract cannot be confirmed, ship without the uninstall prompt and record it in `TODO.md` with the
    named follow-up rather than guessing a hook name.
13. Manual verification recorded in `docs/measurements/phase-20-manual.md`: install over the Phase 05
    build, confirm the DB survived, run each recovery action against a copied corrupt DB, and check the
    uninstall behavior.
14. Commits: `feat: add diagnostics report with redaction`, `feat: add database recovery actions`,
    `feat: add startup strike counter and recovery mode`, `feat: add user-initiated updater`,
    `feat: polish installer shortcuts and uninstall`.

## Todo

- [ ] cheap diagnostics metrics (no full scans)
- [ ] redacted diagnostics export (clipboard + file)
- [ ] strike counter + recovery-mode boot path
- [ ] repair / export / reset actions with typed outcomes
- [ ] updater with auto-check off + user-initiated flow
- [ ] DB-preserving upgrade test (5 000 events)
- [ ] installer shortcuts, opt-in autostart, uninstall prompt (schema-confirmed)
- [ ] diagnostics / recovery / strike tests + manual record

## Success Criteria

- [ ] Diagnostics generates ≤300 ms at 1M events and shows every listed metric
- [ ] The exported report contains no secret, no raw profile path, no machine or account name
- [ ] A corrupted DB boots into recovery mode, and Reset preserves the damaged file under a new name
- [ ] Three failed starts route to recovery; a good start resets the count
- [ ] Installing over the previous build preserves all 5 000 seeded events and the settings row
- [ ] The updater makes no request until the button is clicked
- [ ] Uninstall leaves no Run-key entry and asks about history (or the omission is recorded in `TODO.md`
      with a named follow-up)

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml --test diagnostics_redaction
cargo test --manifest-path src-tauri/Cargo.toml --test recovery_actions
cargo test --manifest-path src-tauri/Cargo.toml --test upgrade_preserves_db
cargo test --manifest-path src-tauri/Cargo.toml --test strike_counter
bun run tauri build && ls -la dist-installer/
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `tauri::webview_version()` does not exist under that name | compile error | Resolve from `cargo doc -p tauri`; if absent, read `pv` from `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F01...}` and label the source in the report |
| NSIS hook key guessed wrong | build fails or the hook is silently ignored | Confirm against the shipped schema; if unconfirmable, ship without the prompt and record the follow-up — never guess an installer hook |
| `VACUUM INTO` repair fails on a badly damaged file | repair returns an error | Repair failure is expected and handled: the UI then recommends Export + Reset, in that order, with the file location shown |
| Diagnostics counts require full scans at 1M | 300 ms budget missed | Counts are maintained incrementally by the writer and reconciled at startup; the test measures at 1M |
| An upgrade migration loses data | upgrade test fails | The pre-migration `VACUUM INTO` backup plus the test is the gate; a failing migration aborts startup into recovery mode rather than proceeding |
| Autostart written without consent | a Run-key entry appears unasked | Autostart is opt-in in onboarding/settings only; a test asserts no registry write happens with the default settings |
| Updater endpoint misconfiguration leaks a request | network traffic at idle | Auto-check disabled in config and re-verified in Phase 21's idle network check |

## Security Considerations

- The diagnostics export is the highest-risk sharing surface in the product; it passes through the same
  redaction engine as persistence and has its own dedicated leak test (§60 discipline applied to §84).
- Recovery actions operate only on the app's own data directory and a user-chosen export path; no
  arbitrary path is accepted from the frontend (the export target is a dialog result, not a string).
- Updater signing keys and any release credential live in environment variables only, documented in the
  release doc with placeholder names — no key material enters the repository.
- The "Open logs folder" action uses a fixed enum target (`LogsFolder`), so it cannot be turned into an
  arbitrary-path opener.
````

### FILE: phase-21-release-audit-hardening.md

````markdown
---
phase: 21
title: "Release Audit & Hardening"
status: pending
priority: P1
effort: 12h
dependencies: [20]
---

## Overview

The final gate: re-run every mechanical check on the assembled product, sweep the human dimensions
(keyboard, contrast, 900×600, calm importance), re-measure the budgets, verify the §82 threat model
line by line, and confirm §87 — no stubs, no `todo!`, no dev commands in release, empty `TODO.md`.
Nothing new is built here; everything is proven or fixed.

## Requirements

Functional
- Full mechanical re-run: `bun run check`, `bun run test`, `cargo fmt --check`, `cargo clippy -D warnings`,
  the complete `cargo test` suite, the `#[ignore]` perf suites at 10k/100k/1M, the analysis fixtures, the
  redaction leak suite, the migration/upgrade tests, and the state-matrix frontend tests.
- Accessibility sweep (§62): keyboard-only traversal of all nine screens with a recorded checklist; visible
  focus on every interactive element; contrast ratios measured for every token pair in both themes
  (≥4.5:1 body, ≥3:1 large text and UI boundaries); `prefers-reduced-motion` honored everywhere;
  screen-reader labels on icon-only controls.
- Layout sweep (§71): every screen at 900×600, 1280×800, and 1920×1080; no clipping, no horizontal scroll,
  no text truncation that loses meaning.
- Calm-importance audit (§54): over a 24-hour dogfood run, report the distribution of importance levels and
  assert ≤10% at `important`+ and ≤1 notification/hour observed; if the distribution is noisier, tune the
  importance rules (single-file change in `importance.rs`) and re-run.
- Budget re-measurement: M1 backend RSS, M2 process-tree total, M3 idle CPU over 120 s, DB size after the
  24-hour run, and startup time to first paint. Every number is recorded in
  `docs/measurements/release-baseline.md` next to its Phase 05 baseline.
- §82 threat-model verification table: for each threat — secret exfiltration via events, secret via logs,
  secret via diagnostics export, arbitrary file read from the frontend, shell injection, SQL injection,
  event flood, malformed OS data, corrupt database, unauthorized IPC command, network exfiltration — a row
  naming the mitigation and the specific test that proves it, with the test result.
- Network-silence check: run the app for 60 s at idle with a socket monitor
  (`netstat -ano | findstr <pid>`) and assert no outbound connection; repeat with the updater button
  clicked to confirm the only network path is user-initiated.
- §87 stub audit: zero `todo!`/`unimplemented!`/`unreachable!(` without a proof comment, zero
  `#[allow(dead_code)]` outside generated files, all dev-only commands behind
  `#[cfg(debug_assertions)]` or the `developer_mode` gate (verified in a release build by asserting the
  command is absent from the release binary's registered command list), and `TODO.md` empty.
- Docs non-stale pass: every doc claim re-checked against code — pinned versions match `Cargo.lock` and
  `bun.lock`, the capability matrix in the docs matches the catalog, `docs/limitations.md` lists every
  known limitation encountered during the build, and every ADR's decision is still what the code does.
- Final release build plus a §88/§90 completion report: what was built, what was measured, what was
  deferred (with the user's own deferrals named), and what remains unverified on this machine.
Non-functional
- Every failure found here is fixed in this phase or explicitly recorded as a known limitation with the
  reason — the phase cannot complete with a red check and a silent excuse.
- The audit is scripted where possible (`scripts/audit-release.ts`) so it is repeatable for future releases
  rather than a one-off manual effort.

## Architecture

`scripts/audit-release.ts` (Bun) orchestrates the mechanical checks and writes a machine-readable result
plus a markdown summary; the human sweeps (keyboard, contrast, layout) use a checklist in
`docs/release-checklist.md` filled in by hand with the observed result per line. Contrast ratios are
computed by `scripts/check-contrast.ts` from the `@theme` token values parsed out of `src/app.css`, so the
check is mechanical rather than eyeballed.

## Related Code Files

Create
- `scripts/audit-release.ts`, `scripts/check-contrast.ts`, `scripts/check-stubs.ts`
- `src-tauri/tests/{release_surface.rs,threat_model.rs}`
- `docs/release-checklist.md`, `docs/threat-model.md`, `docs/measurements/release-baseline.md`
- `docs/release-report.md` (the §88/§90 completion report)

Modify
- `package.json` (`audit:release` script), `src-tauri/src/core/normalize/importance.rs` (tuning only, if the
  distribution audit requires it), `docs/limitations.md`, `TODO.md` (must end empty)
- Any file where the audit finds a defect (scope: fixes only, no new features)

Delete
- Any leftover scratch route, unused placeholder component, or dead example not referenced by a test
  (enumerated by the audit before deletion, never bulk-deleted).

## Implementation Steps

1. `scripts/check-stubs.ts`: fail on `todo!(`, `unimplemented!(`, `unreachable!(` without an adjacent
   `// proof:` comment, on `#[allow(dead_code)]` outside `bindings`/generated files, on `{@html`, and on any
   non-empty `TODO.md`.
2. `scripts/check-contrast.ts`: parse the `@theme` block, compute WCAG contrast for the documented
   foreground/background pairs in both themes, and fail below the thresholds.
3. `tests/release_surface.rs`: assert the registered command list in a release-configured build contains
   exactly the production command names and none of the dev commands; assert
   `tauri.conf.json` capabilities reference no `fs`, `shell`, `http`, or `sql` permission; assert
   `Cargo.toml` has no `tauri-plugin-{fs,shell,http,sql}` dependency.
4. `tests/threat_model.rs`: one test function per §82 threat, each either running the proving assertion
   directly or asserting the existence and pass of the named dedicated test (a registry mapping threat →
   test name, so a deleted test breaks the threat model).
5. `scripts/audit-release.ts`: run the full mechanical set in order, capture output, and emit
   `docs/measurements/release-baseline.md` plus a pass/fail table. Non-zero exit on any failure.
6. Run the 24-hour dogfood: leave the app running with all default collectors enabled, then compute the
   importance distribution, the notification count, the DB growth, and the event count by collector; record
   them and tune `importance.rs` if the ≤10% guard is exceeded.
7. Perform the keyboard, screen-reader-label, and layout sweeps at the three resolutions, filling in
   `docs/release-checklist.md` with the observed result for each of the nine screens.
8. Network-silence check with `netstat -ano | findstr <pid>` at idle and after clicking "Check for updates";
   record both outcomes.
9. Re-measure M1/M2/M3 and startup-to-first-paint; put them beside the Phase 05 baselines. If M2 still
   exceeds 80 MB, the B2 escalation from Phase 05 is restated in the release report with the final measured
   number and the three options — the metric is never quietly redefined.
10. Docs pass: verify pinned versions against `Cargo.lock`/`bun.lock`, the catalog against the matrix, the
    ADR list against the code, and `docs/limitations.md` against everything recorded during Phases 09–20.
11. Write `docs/threat-model.md` (the verification table) and `docs/release-report.md` (§88/§90: built,
    measured, deferred, unverified).
12. Final `bun run tauri build`; install the artifact on a clean user profile if one is available,
    otherwise record that the clean-profile install was not exercised.
13. Commits: `chore: add release audit scripts`, `test: add release surface and threat model tests`,
    `docs: record release measurements and threat model`, `fix: <per defect found>`,
    `chore: prepare release build`.

## Todo

- [ ] stub/contrast/surface audit scripts
- [ ] threat-model test registry (§82 rows all proven)
- [ ] full mechanical re-run green
- [ ] 24-hour dogfood + importance distribution audit
- [ ] a11y + layout sweeps recorded per screen
- [ ] network-silence check recorded
- [ ] M1/M2/M3 + startup re-measured beside baselines
- [ ] docs non-stale pass + limitations complete
- [ ] `TODO.md` empty + release report written

## Success Criteria

- [ ] `bun run audit:release` exits 0 with every check green
- [ ] Perf matrix, redaction suite, analysis fixtures, and migration tests all pass on the release build
- [ ] All nine screens pass keyboard-only traversal and the 900×600 layout check (recorded per screen)
- [ ] Every token pair meets its contrast threshold in both themes (script-verified)
- [ ] Importance distribution ≤10% at `important`+ over 24 hours; ≤1 notification/hour observed
- [ ] Zero outbound connections at idle; the only network path is the user-initiated update check
- [ ] Every §82 threat row names a mitigation and a passing test
- [ ] No `todo!`/`unimplemented!` without proof, no dev command in the release surface, `TODO.md` empty
- [ ] M1/M2/M3 recorded beside the Phase 05 baselines, with the B2 outcome stated plainly
- [ ] `docs/release-report.md` lists what was built, deferred, and left unverified

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run audit:release
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --release -- --ignored --nocapture
bun run check && bun run test
powershell -ExecutionPolicy Bypass -File scripts/measure-memory.ps1
powershell -ExecutionPolicy Bypass -File scripts/measure-idle-cpu.ps1
bun run tauri build
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| The audit finds a defect too large to fix in 12 h | a failing check with a multi-hour fix | Record it as a release blocker with an estimate and escalate to the user with options (delay, ship with a documented limitation, or descope a feature) — never mark the phase complete with a red check |
| M2 memory still exceeds 80 MB | measurement above budget | Restate the B2 escalation with the final number and the three options; the budget is the user's decision, not the plan's |
| Importance distribution too noisy | >10% at `important`+ | Tune `importance.rs` (one file) and re-run the distribution; if tuning cannot reach the guard, document the observed distribution and the reason |
| Contrast failures in the dark theme | script fails on token pairs | Adjust the token values (Phase 04 tokens are the single source) and re-run all three sweeps |
| A `#[ignore]` perf test was never run in earlier phases | first real failure appears here | Each phase's gate runs its own perf test; this phase's job is confirmation, and any surprise is treated as a defect with its phase named in the report |
| Docs drifted during 20 phases | doc claims contradict code | The docs pass is mechanical where possible (version pins, catalog, ADR list) and the report states which docs were re-verified |
| A clean-profile install is impossible on this machine | cannot test first-run cleanly | Use a renamed data directory to simulate first run, and record that a true clean-profile install was not exercised |

## Security Considerations

- This phase is where the security claims stop being intentions: each §82 row must point at a test that
  ran and passed, and `tests/threat_model.rs` makes deleting such a test a build failure.
- The release surface test is the structural guarantee that no banned plugin, raw SQL command, or dev-only
  command reaches users.
- The network-silence check converts §32's "nothing leaves this computer" from a promise into an observed
  measurement, recorded with the method used so it can be re-run.
````

## Phase decomposition rationale

**Why 21 phases inside §69's nine.** §69 fixes the order (foundation → events → collectors → …) but not
the granularity. Twenty-one phases map onto it strictly linearly — P0 = 01–05, P1 = 06–07, P2 = 08–11,
P3 = 12–13, P4 = 14, P5 = 15–16, P6 = 17, P7 = 18, P8 = 19–21 — and each is 6–20 h, which is the size a
reviewer can actually hold in their head (§89). No two phases own the same file, so a failed phase is
revertible by dropping its commits without touching a neighbour.

**Why redaction is Phase 02, before storage exists.** A redaction layer added after collectors is a
redaction layer that has already been bypassed. Making `RedactedEvent` the only type the writer accepts
means every later phase inherits the guarantee for free, and the leak suite grows as new shapes appear
instead of being retrofitted once at the end.

**Why the synthetic feed (06) precedes every real collector.** The Timeline, drawer, search, perf matrix,
and analysis fixtures all need volume and variety that real collectors cannot produce on demand. Building
the frontend against synthetic data also proves the IPC contract before any OS-specific complexity can be
blamed for a UI bug. It is not a stub: it is a permanent, flagged developer facility (§55/§87).

**Why the collector runtime (08) is its own phase.** Supervision, panic isolation, rate limiting, and
normalization are cross-cutting. Written once before the first real collector, they make Phases 09–13
about OS APIs only. Written later, every collector would have grown its own ad-hoc retry logic.

**Why process+network (09) is the first real collector pair.** They are the cheapest (6 ms / 10 ms), they
exercise the hardest privilege reality (B3, ~46% path denial), and together they produce the §15 causal
story — so the first real collector phase also proves the causality model end to end.

**Why the event-log wrapper is built in 12, not 13.** The update collector needs it first. Building it
once, with caps and malformed-input defenses, means Phase 13 consumes a proven wrapper instead of writing
a second one — the DRY choice that also concentrates the malformed-data threat in one audited module.

**Why analysis (15) comes after all collectors.** Ranking quality depends on real event shapes,
categories, and entity edges. Scoring written against imagined data would need re-tuning the moment real
collectors landed; the §59 and §73 fixtures are only meaningful once the events they reference can
actually exist.

**Why Three.js is a single late phase (17).** §39 says intentional-only. Isolating it means it can be cut
or reduced under time pressure without touching a single feature, and the always-present DOM equivalents
are what make that true rather than aspirational.

**Why the audit is a phase, not a checklist item.** Twelve hours of dedicated re-verification with scripted
checks is the difference between "we believe it is secure and fast" and "here are the numbers and the tests
that produced them" (§88/§90).

## Risks carried forward

| ID | Risk | Carried into | Current status |
|---|---|---|---|
| B1 | Rust toolchain absent; three MSVC installs, linker ambiguity | Phase 01 resolves; `rust-toolchain.toml` pins it | Mitigated by construction — Phase 01 cannot pass without a linked binary |
| B2 | <80 MB RAM likely unreachable with WebView2 | Measured in 05, re-measured in 21 | **Open scope question** — M1 committed, M2 escalated to the user with three options; never silently redefined |
| B3 | No non-elevated process start/stop precision; ~46% path denial | Phase 09 documents; Phase 13 adds crash correlation | Partial coverage, stated in `docs/limitations.md` and the onboarding admin step |
| A3 | `RegNotifyChangeKeyValue` on HKLM non-elevated unverified | Phase 11 spike | Fallback (5-min poll, 50 ms cost) is production-acceptable, so the risk is bounded |
| A4 | `NotifyServiceStatusChangeW` reliability unverified | Phase 11 spike | Fallback 5 s poll (3.3 ms) ships regardless |
| A5 | `CM_Register_Notification` availability unverified | Phase 12 | Fallback: start + resume + hourly |
| A6 | Win32 change-notification callbacks from Rust unverified | Phase 13 | Callback is send-and-return; 5-min reconcile is the safety net; ADR 0012 records the outcome |
| A9 | Docker daemon path untestable here (D3) | Phase 18 | Absent path really tested; present path tested via injected events; ADR 0013 lists re-verification steps |
| A10 | `@tanstack/svelte-virtual` + Svelte 5 runes unverified | Phase 07 | 60-minute timeboxed spike with a hand-rolled fallback already specified |
| D2 | TypeScript 7.0.2 vs 5.x | Phase 01 gate, ADR 0001 | Decided at the gate with a measured fallback |
| — | StartupApproved blob encoding [UNVERIFIED] | Phase 13 | Determined empirically before any decoder is written; presence-only fallback if undeterminable |
| — | Unconfirmed event-log IDs (1002, SCM IDs) | Phases 12–13 | Only measured IDs ship; each addition requires a recorded empirical confirmation |
| — | WUA interface route (`windows` crate vs `IDispatch`) | Phase 12, ADR 0011 | Decided from `cargo doc` in step 1; both routes are real, neither is invented |
| — | NSIS uninstall-hook contract | Phase 20 | Confirmed against the shipped schema or omitted with a named follow-up |

## Unresolved questions

1. **B2 memory budget.** If the measured process-tree total (M2, including `msedgewebview2.exe`) lands
   above 80 MB — which the research report suggests is likely — which do you want: (a) keep 80 MB as the
   backend-only (M1) target and publish the tree total as an observed number, (b) raise the stated budget
   to the measured value, or (c) treat it as a blocker and reconsider the WebView-based shell? Phase 05
   stops for this answer rather than choosing for you.
2. **Watched-location defaults.** Onboarding ships file watching *off* with no default locations, because
   guessing a user's project directory is both privacy-risky and likely wrong. Would you rather it propose
   a specific default (for example a detected `Documents`/dev folder) with a clear opt-out?
3. **Notification frequency.** §53 gives "at most one per hour" as the ceiling; the plan also suppresses
   notifications for the first 10 minutes after start and enforces one-per-collector-per-hour. Is the
   10-minute quiet period welcome, or would you prefer notifications from the first moment?
4. **Dogfood window.** Phase 21's importance-distribution audit assumes a 24-hour run. If the release needs
   to happen sooner, the alternative is a 4-hour run with the distribution marked provisional — which do
   you want as the default?
5. **Clean-profile install.** A true first-run test needs a second Windows user profile. If creating one is
   not acceptable on this machine, Phase 21 simulates it by renaming the data directory and records that the
   real clean-profile path was not exercised. Confirm that substitute is acceptable.

Status: DONE
Summary: Complete 21-phase candidate plan written to the single designated report file, covering §1–§91 in the §69-mandated order with pinned versions, exact commands, per-phase verification gates, and the B1/B2/B3 risks handled explicitly.
