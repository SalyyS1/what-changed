# Candidate 2 — Seal the contracts first: one sealed write path, one typed IPC allowlist, one platform boundary — then 12 collectors become additive, reviewable increments

## Thesis

Every hard requirement in this build is a **contract that must be impossible to violate later**: redaction before persistence and logging (§34), no raw SQL/shell/file surface (§35/§56), Rust types as source of truth (§81), Win32 confined to `platform/windows/` (§6). Phases 01–10 install those contracts as *compile-time and grep-time gates* — `SealedEvent` has a private constructor reachable only through the redactor, the command allowlist is asserted by a test, `windows::` imports outside `platform/windows/` fail a test. After that, each of the 12 collectors is a small, uniform, independently-failing increment that physically cannot bypass redaction or leak an API to the frontend. Blockers are front-loaded: B1 (Rust absent) and D2 (TS 7) are settled in Phase 02 before any product code exists; B2 (RAM) is measured on a smoke build in Phase 02 and re-measured in Phase 24 with a pre-decided escalation, never a redefinition; B3 (process blind spot) is mitigated with a measured second source (Event Log 1000, probe #16 PASS) and documented as partial.

## Plan metadata

```yaml
---
title: "What Changed — Windows change recorder (MVP)"
description: "Local-first Tauri 2 + Rust + Svelte 5 flight recorder for Windows that records meaningful system changes and answers 'It worked before. What changed?' — 12 collectors, deterministic analysis engine, Three.js causal graph, zero cloud."
status: pending
priority: P1
effort: 176h
tags: [tauri2, rust, svelte5, sqlite, windows, privacy, observability, greenfield]
blockedBy: []
blocks: []
created: 2026-08-29
---
```

## PLAN.MD BODY

```markdown
## Overview

"What Changed" continuously records meaningful local system changes and answers one question:
*"It worked before. What changed?"* (§1, §91). Local-first, non-elevated, no account, no cloud,
no telemetry (§33). Windows 10/11 x64 only in this MVP; `core/` is platform-free and every Win32
call lives in `platform/windows/` (§6).

**Repo root:** `D:\Project\.WhatChage` (folder name misspelled — D1; product/binary/package names are
`What Changed` / `WhatChanged.exe` / `what-changed`, and no code or config may reference the folder name).
All file paths in this plan are repo-root-relative unless absolute. Greenfield: `main`, zero commits.
**Rust is not installed** — Phase 02 owns that, nothing compiles before it.

Scope mode: **HOLD SCOPE**. Everything in §1–§91 ships. The only deferrals are the user's own:
§26 health checks and §75–§78 future features, both parked in `ROADMAP.md` (the §10-mandated
`health_states` table is still created in Phase 04).

## Goals

| # | Goal | Priority |
|---|------|----------|
| 1 | Record normalized system change events from 12 collectors, each independently failing (§9, §12, §58) | P1 |
| 2 | Redaction that cannot be bypassed: enforced before persistence *and* before logging (§34, §60) | P1 |
| 3 | Deterministic analysis engine that ranks probable causes with a transparent score (§27, §28, §29) | P1 |
| 4 | A calm, premium desktop UI that a non-technical user understands in seconds (§40–§54, §70–§74) | P1 |
| 5 | Narrow typed IPC only — no shell, no raw SQL, no arbitrary file read (§35, §56) | P1 |
| 6 | Non-elevated operation with honest degradation where Windows requires admin (§36) | P1 |
| 7 | SQLite durability: WAL, migrations, retention, corruption recovery (§10, §11, §83) | P1 |
| 8 | Checkpoints and checkpoint comparison (§25, §47) | P1 |
| 9 | Three.js used intentionally: ambient field, causal graph, system map — never a canvas app (§38, §39, §48) | P2 |
| 10 | Optional Docker + development-mode collectors that never spam when absent (§23, §24) | P2 |
| 11 | Installer, updater, signing documentation, upgrade that never erases history (§64–§66) | P1 |
| 12 | Documentation that stays true: 9 living docs plus per-decision records (§68, §87) | P1 |

## Phases

| # | Phase | Status | §69 |
|---|-------|--------|-----|
| 01 | [Repo Genesis & Product Docs](./phase-01-repo-genesis-product-docs.md) | pending | 0 |
| 02 | [Toolchain & Decision Spikes](./phase-02-toolchain-decision-spikes.md) | pending | 0 |
| 03 | [App Scaffold & Typed IPC Spine](./phase-03-app-scaffold-typed-ipc.md) | pending | 0 |
| 04 | [Storage Foundation & Migrations](./phase-04-storage-foundation-migrations.md) | pending | 0 |
| 05 | [Windows Shell, Tray & Installer](./phase-05-windows-shell-tray-installer.md) | pending | 0 |
| 06 | [Design System & Skeleton Screens](./phase-06-design-system-skeleton-screens.md) | pending | 0 |
| 07 | [Domain Model & Redaction Chokepoint](./phase-07-domain-model-redaction-chokepoint.md) | pending | 1 |
| 08 | [Event Repository, Ingest & Retention](./phase-08-event-repository-ingest-retention.md) | pending | 1 |
| 09 | [Timeline Experience & Live Events](./phase-09-timeline-experience-live-events.md) | pending | 1 |
| 10 | [Collector Runtime & Supervisor](./phase-10-collector-runtime-supervisor.md) | pending | 2 |
| 11 | [Process & Network Listener Collectors](./phase-11-process-network-collectors.md) | pending | 2 |
| 12 | [File & Config Collector](./phase-12-file-config-collector.md) | pending | 2 |
| 13 | [Applications & Services Collectors](./phase-13-applications-services-collectors.md) | pending | 2 |
| 14 | [Windows Update & Event Log Collectors](./phase-14-updates-eventlog-collectors.md) | pending | 3 |
| 15 | [Drivers, Network Config, Startup & Snapshots](./phase-15-drivers-network-startup-snapshots.md) | pending | 3 |
| 16 | [Checkpoints & Notification Policy](./phase-16-checkpoints-notifications.md) | pending | 4 |
| 17 | [Analysis Engine](./phase-17-analysis-engine.md) | pending | 5 |
| 18 | [Investigation Experience](./phase-18-investigation-experience.md) | pending | 5 |
| 19 | [Three.js Runtime & Ambient Field](./phase-19-three-runtime-ambient-field.md) | pending | 6 |
| 20 | [Causal Graph & System Map](./phase-20-causal-graph-system-map.md) | pending | 6 |
| 21 | [Docker & Development Mode](./phase-21-docker-development-mode.md) | pending | 7 |
| 22 | [Onboarding, Settings & Diagnostics](./phase-22-onboarding-settings-diagnostics.md) | pending | 8 |
| 23 | [States, Accessibility & Motion Audit](./phase-23-states-accessibility-motion-audit.md) | pending | 8 |
| 24 | [Release: Performance, Update & Security Audit](./phase-24-release-performance-security-audit.md) | pending | 8 |

## Architecture Summary

Svelte 5 UI (WebView2) → generated typed bindings → **narrow Tauri command allowlist** → Application
Service → { Query Service, Analysis Engine, Settings } → Event Repository → SQLite. Collectors →
platform probes → Normalizer → **Redactor (seal)** → bounded queue → batching writer → SQLite.
The UI never touches an OS API and never sees SQL, a shell, or arbitrary file bytes (§8, §35).

Four enforced invariants, each covered by a test in `src-tauri/tests/architecture-gates.rs`:

1. **Sealed write path** — `SealedEvent` is constructed only by `redact::Redactor::seal`; the repository
   accepts nothing else. Redaction therefore precedes persistence by construction (§34).
2. **Redacted logging** — `RawEvent`/`ConfigDiff` implement `Debug` manually as redacted output, plus a
   `tracing` scrubbing layer. Secrets cannot reach the rolling log file (§34).
3. **Platform isolation** — no `windows::`/`windows_sys::` path outside `src/platform/windows/` (§6).
4. **IPC allowlist** — the registered command set equals the §56 list plus the explicitly justified
   additions in `docs/IPC-SURFACE.md`; no generic command exists (§35).

Rust domain types are the source of truth; `specta` + `tauri-specta` generate `src/lib/tauri/bindings.ts`
and the gate fails on drift (§81). Each collector is `collectors/<name>/{mod.rs,normalize.rs}` (core) plus
`platform/windows/<name>.rs` (probe), supervised in its own Tokio task so one failure never kills the app.

## Success Criteria

- [ ] `bun run gate` passes: prettier, `svelte-check` (warnings = errors), `tsc --noEmit`, `vitest run`, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`.
- [ ] `bun run tauri build` produces a working NSIS per-user installer; installing, launching, closing to tray, and uninstalling all work non-elevated.
- [ ] All 12 collectors run non-elevated; `COLLECTORS.md` states the two measured gaps (push process events, `Security` channel) as limitations, not features.
- [ ] Redaction leak suite: 12 secret specimens never appear as raw bytes in the DB file, the WAL, the log files, or a diagnostics export.
- [ ] §59 fixture: the `.env` port → container restart → 3000 closed → 4000 opened → health-fail chain ranks #1; the unrelated Chrome update ranks strictly lower.
- [ ] 10k / 100k / 1M event runs meet the thresholds recorded in `docs/perf-baseline.md`; the timeline virtualizes and never loads all events.
- [ ] Migration suite passes from empty and from every intermediate version; a corrupted DB opens into recovery UI without a crash loop.
- [ ] Every one of the 9 screens renders all six states with real microcopy; the §72 copy strings are asserted verbatim by a test.
- [ ] Idle CPU ≤1% average over 30 min; RAM measured and reported with backend RSS and WebView2 total split (B2).
- [ ] `docs/STUBS.md` is empty at the end of Phase 24; no `todo!()`/`unimplemented!()` remains in shipped paths (§87).

## Key Risks

| Risk | Signal it broke | Pre-decided response |
|---|---|---|
| **B1** Rust absent; 3 MSVC installs; coreutils `link.exe` shadows MSVC in Git Bash | `rustup`/`cargo` missing, or `LNK1181` / "link: extra operand" | Phase 02 installs `stable-x86_64-pc-windows-msvc`, pins `rust-toolchain.toml` to the observed version, and runs cargo via `powershell -NoProfile` if the coreutils shadow appears |
| **B2** <80 MB RAM may be unreachable with WebView2 | Phase 02/24 measurement exceeds 80 MB total | Report both numbers, never redefine the metric; raise a three-option scope question to the user (accept measured target / cut a feature / continue optimizing) |
| **B3** Non-admin process events are diff-polled | Short-lived process invisible in a 1 s window | Documented in `COLLECTORS.md`; crash-exits recovered via Event Log 1000 (probe #16 PASS); optional ETW path only if the Phase 10 spike passes |
| **D2** TypeScript 7.0.2 toolchain support unproven | `svelte-check`/`tsc` fails on the scaffold | Pin down to the highest working 6.x, then 5.x; record in `docs/decisions/0002-toolchain-pins.md` |
| Notification APIs unverified (research §5 #3/#4/#5) | Spike test reports FAIL | Fall back to the measured poll costs (uninstall 50 ms, services 3.3 ms, drivers hourly sweep) and state the detection latency |
| `@tanstack/svelte-virtual` Svelte-5 support unverified (#10) | 100k-row smoke test errors or drops frames | Replace with the hand-rolled fixed-height windowed list already specified in Phase 09 |
| Playwright cannot drive Tauri (#11) | `tauri-driver` session fails | Keep E2E at Vitest/component level, say so in `README.md`; do not claim E2E coverage |

## Dependencies

- **External, must exist before Phase 03:** `rustup` + `stable-x86_64-pc-windows-msvc` (Phase 02 installs), MSVC VC.Tools (present), Windows SDK 10.0.26100.0 (present), WebView2 Evergreen 151.x (present), Bun 1.3.11 (present).
- **Never a runtime dependency of the shipped app:** Node, Bun, npm/pnpm/yarn, Docker, network access (§63, §67).
- **Version pins:** all Rust and npm versions come from `research/verified-facts-report.md` §1 and are recorded in `docs/decisions/0002-toolchain-pins.md`; any package absent from that table is pinned to the exact version the installer resolves, with no caret range.
- **Sequencing:** phase *N* depends on *N-1*; §69's user-phase order is preserved (column §69 above is monotonic).
```

## PHASE FILES

### FILE: phase-01-repo-genesis-product-docs.md

---
phase: 01
title: "Repo Genesis & Product Docs"
status: pending
priority: P1
effort: 3h
dependencies: []
---

## Overview

§90 requires the product/architecture/security/privacy/roadmap documents *before* implementation code.
This phase deliberately needs **zero Rust**, so it is productive while B1 is still open. It also fixes a
measured blocker: `git config user.name` is **unset** in this environment (`user.email` is set globally) —
the first commit fails with "empty ident name" unless set repo-locally first.

## Requirements

**Functional**
- Repo hygiene: `.gitignore`, `.gitattributes`, `LICENSE` placeholder decision recorded, first commit on `main`.
- The nine living docs of §68 plus §55's `README.md` and `CONTRIBUTING.md`.
- `docs/decisions/` ADR directory; `docs/STUBS.md` stub registry (§87); `docs/IPC-SURFACE.md` command allowlist doc (§56).
- `ROADMAP.md` records the user's own deferrals: §26 health checks, §75–§78 future features. Nothing else is deferred.
- `SECURITY.md` + `PRIVACY.md` state the two **measured** privilege gaps and the full "never collected" list (§33).
- `COLLECTORS.md` lists all 12 collectors with columns: source API, admin required (measured), cadence (measured cost), status.

**Non-functional**
- No claim in any doc may exceed the evidence: privilege statements cite the probe row; unverified items are labelled "unverified".
- No secrets, no email addresses, no machine-specific absolute paths in committed docs.

## Architecture

Docs-as-contract: `ARCHITECTURE.md` fixes the module boundaries that Phases 03–10 turn into enforced gates
(sealed write path, platform isolation, IPC allowlist, Rust-as-source-of-truth). `ANALYSIS.md` fixes the §28
score formula and the §29 confidence vocabulary before any code can drift from it.

## Related Code Files

**Create:** `.gitignore`, `.gitattributes`, `README.md`, `PRODUCT.md`, `ARCHITECTURE.md`, `SECURITY.md`,
`PRIVACY.md`, `ROADMAP.md`, `CONTRIBUTING.md`, `COLLECTORS.md`, `ANALYSIS.md`, `docs/IPC-SURFACE.md`,
`docs/STUBS.md`, `docs/decisions/0000-record-format.md`, `docs/copy-deck.md`, `docs/ui-states-matrix.md`
**Modify:** none. **Delete:** none.

## Implementation Steps

1. `git -C "D:/Project/.WhatChage" config user.name "Salyyy"` (repo-local; do not touch global config).
2. Write `.gitignore`: `/node_modules/`, `/dist/`, `/src-tauri/target/`, `/.tmp/`, `*.db`, `*.db-wal`, `*.db-shm`, `/spikes/*/target/`, `.env*`, `/src-tauri/gen/`, `*.bak`.
3. Write `.gitattributes`: `* text=auto eol=lf`, `*.ps1 text eol=crlf`, `*.cmd text eol=crlf`, `*.rs text eol=lf`.
4. `PRODUCT.md`: §1 vision, §2 dual audience, §43/§45/§47 screen intents, §54 calm importance model, §72–§74 voice, §91 final principle.
5. `ARCHITECTURE.md`: the §8 layer chain, the §55 tree, the four enforced invariants, the §5 `Collector` trait verbatim, the future-sidecar protocol note (documented, not built).
6. `SECURITY.md`: §82's 11 threats as a table (threat → mitigation → phase that implements it → test that proves it); §35 capability policy; §36 privilege model; the two measured gaps.
7. `PRIVACY.md`: exactly what is collected per collector and the §33 never-list; "no telemetry" statement; where data lives (`%APPDATA%\WhatChanged`).
8. `COLLECTORS.md`: 12 rows seeded with the measured matrix (process, ports, files, apps, services, updates, eventlog, drivers, network, startup, docker, git) + B3 blind-spot paragraph + "Docker connected path untested on this machine".
9. `ANALYSIS.md`: the §28 six-term formula, each term's definition, the §29 three confidence tiers, and the forbidden-language list.
10. `docs/copy-deck.md`: every §72/§73/§74/§43/§45/§46/§47/§50 string verbatim, with the §31/§43/§72 placeholder conflict resolved explicitly (see `docs/decisions/0003-microcopy.md`, written in Phase 06).
11. `git add -A && git commit -m "chore: repo genesis and product documentation"`.

## Todo

- [ ] Repo-local git identity set; commit succeeds
- [ ] 11 root docs + 5 `docs/` files written, no lorem ipsum, no unverified claim stated as fact
- [ ] `COLLECTORS.md` privilege column matches the measured matrix row-for-row
- [ ] `ROADMAP.md` contains §26 and §75–§78 and nothing else deferred

## Success Criteria

- [ ] `git -C "D:/Project/.WhatChage" log --oneline` shows exactly one commit and `git status --porcelain` is empty
- [ ] `grep -c "Security" SECURITY.md` > 0 and the file names both measured gaps
- [ ] Every doc link resolves (checked by step 11's manual pass; automated in Phase 03's gate)
- [ ] No file contains the string `.WhatChage` outside a quoted note about D1

## Verification Gate

```bash
cd "D:/Project/.WhatChage" && git log --oneline && git status --porcelain
grep -rl "TODO\|lorem" --include="*.md" "D:/Project/.WhatChage" | grep -v plans/ || echo "no placeholder text"
grep -c "Win32_ProcessStartTrace\|Security channel" "D:/Project/.WhatChage/SECURITY.md"
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Commit fails on missing identity | `*** Please tell me who you are` | Step 1 already sets it repo-locally; never write global git config |
| Docs overstate capability | A privilege claim with no probe row behind it | Reject the sentence; move it to an "unverified" subsection with the settle-it action |
| Docs go stale by Phase 24 | Phase gate finds a doc contradicting code | Every later phase's Todo ends with "update the doc it changed"; Phase 24 runs a de-stale pass |

## Security Considerations

Docs must not embed the machine's username, email, or absolute user paths. `PRIVACY.md` is a public
commitment — write only what the code will actually do. Threat-model rows stay unchecked until the phase
that implements the mitigation lands its test.

### FILE: phase-02-toolchain-decision-spikes.md

---
phase: 02
title: "Toolchain & Decision Spikes"
status: pending
priority: P1
effort: 4h
dependencies: [01]
---

## Overview

Settles B1 (Rust absent), D2 (TypeScript 7), B2 (RAM baseline) and D-installer (NSIS/WiX, WebView2
bootstrapper) on a throwaway smoke app, before one line of product code exists. Nothing after this phase
is allowed to guess a toolchain fact.

## Requirements

**Functional**
- `rustup` + `stable-x86_64-pc-windows-msvc` installed non-elevated; `rust-toolchain.toml` pinned to the *observed* version.
- A minimal Tauri 2.11.5 window builds in release and opens, proving MSVC linking + WebView2 + `tauri-build`.
- Measured RAM: Rust backend RSS **and** the sum of `msedgewebview2` children, recorded separately (B2).
- TS 7.0.2 verdict: `svelte-check` 4.7.6 + `tsc --noEmit` on a scratch Svelte 5 file, PASS/FAIL recorded.
- Installer capability read from the *installed* CLI (`bunx @tauri-apps/cli@2.11.4 build --help`), not from memory.

**Non-functional**
- No global PATH edits: `$HOME/.cargo/bin` is **already** on PATH (verified) and becomes live once the directory exists, so install with `--no-modify-path`.
- Spike crates live in `spikes/` and are deleted in Phase 03; both are registered in `docs/STUBS.md` on creation.

## Architecture

Decision records only. `docs/decisions/0001-rust-toolchain.md`, `0002-toolchain-pins.md`,
`0004-installer-and-webview2.md`, `docs/perf-baseline.md`. Every later phase reads pins from `0002`.

## Related Code Files

**Create:** `rust-toolchain.toml`, `spikes/tauri-smoke/{Cargo.toml,build.rs,tauri.conf.json,src/main.rs,dist/index.html}`,
`spikes/ts7-check/{package.json,tsconfig.json,svelte.config.js,src/Probe.svelte}`,
`docs/decisions/0001-rust-toolchain.md`, `docs/decisions/0002-toolchain-pins.md`,
`docs/decisions/0004-installer-and-webview2.md`, `docs/perf-baseline.md`, `scripts/measure-memory.ps1`
**Modify:** `docs/STUBS.md`. **Delete:** none in this phase.

## Implementation Steps

1. `mkdir -p "D:/Project/.WhatChage/.tmp" && curl -fsSLo "D:/Project/.WhatChage/.tmp/rustup-init.exe" https://win.rustup.rs/x86_64`
2. `"D:/Project/.WhatChage/.tmp/rustup-init.exe" -y --profile minimal --default-toolchain stable-x86_64-pc-windows-msvc --no-modify-path` — fallback if the download is blocked: `winget install --id Rustlang.Rustup --scope user` (winget verified present).
3. `rustc -V && cargo -V && rustup show`; write `rust-toolchain.toml` with `[toolchain] channel = "<observed x.y.z>"`, `targets = ["x86_64-pc-windows-msvc"]`, `components = ["rustfmt","clippy"]`.
4. Smoke crate: `cargo new --bin spikes/tauri-smoke`, add `tauri = "2.11.5"`, `tauri-build = "2.6.3"`, a 15-line `main.rs` opening one window, `tauri.conf.json` with `build.frontendDist = "./dist"`, and a one-line `dist/index.html`.
5. `cargo build --release --manifest-path "D:/Project/.WhatChage/spikes/tauri-smoke/Cargo.toml" -v 2>&1 | tee "D:/Project/.WhatChage/.tmp/link.log"`; record the resolved `link.exe` path from the log into `0001`.
6. Run the smoke binary; with the window open run `scripts/measure-memory.ps1` (`Get-Process` → `WorkingSet64`/`PrivateMemorySize64` for the app process and every `msedgewebview2` child) and write both totals to `docs/perf-baseline.md`.
7. `bun add -D --cwd "D:/Project/.WhatChage/spikes/ts7-check" typescript@7.0.2 svelte@5.56.10 svelte-check@4.7.6 @sveltejs/vite-plugin-svelte@7.3.0 vite@8.2.2`; run `bunx svelte-check --tsconfig ./tsconfig.json` and `bunx tsc --noEmit`; record verdict in `0002`.
8. `bunx @tauri-apps/cli@2.11.4 build --help` and `bunx @tauri-apps/cli@2.11.4 --version`; record available bundle targets and the WebView2 install-mode options actually offered into `0004`; decide `nsis` + per-user install + `downloadBootstrapper`, with `embedBootstrapper`/`offlineInstaller` named as the offline alternative if the CLI offers them.
9. Commit: `chore: pin rust toolchain and record toolchain decisions`.

## Todo

- [ ] `rustc`/`cargo` resolve in a fresh Git Bash session without PATH edits
- [ ] Smoke window opens from a `--release` binary
- [ ] `docs/perf-baseline.md` has three numbers: backend RSS, WebView2 total, combined
- [ ] `0002` records the TS verdict and every pinned version, caret-free
- [ ] `0004` records bundle targets copied from the installed CLI's own output

## Success Criteria

- [ ] `rustc -V` prints a `-x86_64-pc-windows-msvc` host triple (not `-gnu`)
- [ ] `cargo build --release` on the smoke crate exits 0 and the window renders HTML
- [ ] TS decision is one of: keep 7.0.2 / pin 6.x / pin 5.x — with the failing command output pasted into `0002` if not 7.0.2
- [ ] B2 numbers exist as measurements, with an explicit sentence stating whether <80 MB looks reachable

## Verification Gate

```bash
rustc -V; cargo -V; rustup show
cargo build --release --manifest-path "D:/Project/.WhatChage/spikes/tauri-smoke/Cargo.toml"
powershell -NoProfile -File "D:/Project/.WhatChage/scripts/measure-memory.ps1"
bunx svelte-check --tsconfig "D:/Project/.WhatChage/spikes/ts7-check/tsconfig.json"
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Git Bash coreutils `link.exe` shadows MSVC's (both exist at `/usr/bin/link.exe`, ahead of MSVC on PATH) | `link: extra operand` or `LNK1181` | Re-run the build as `powershell -NoProfile -Command "cargo build --release"`; if it then succeeds, record in `0001` that all cargo invocations go through PowerShell and update `scripts/gate.ts` accordingly |
| Wrong MSVC selected among the three installs (VS2017 BuildTools, `D:\Work\c++`, VS18 Insiders) | Missing `kernel32.lib`/CRT headers | Launch the build from the chosen install's `Launch-VsDevShell.ps1`, record the chosen `installationPath` in `0001` |
| TS 7 breaks `svelte-check` (unverified #1) | Non-zero exit with parser/host errors | Pin down one major at a time (6.x, then 5.x); record the failure output; do not silence with `skipLibCheck` alone |
| B2 already exceeded on a blank app | Combined RAM > 80 MB with one empty window | Do **not** adjust the target: write the number, note that a feature-complete app can only be larger, and raise the scope question now rather than in Phase 24 |
| `rustup-init` download blocked offline | curl exits non-zero | Use the winget fallback in step 2; if both fail, the phase is BLOCKED and reported — no code can proceed |

## Security Considerations

`rustup-init.exe` is fetched over HTTPS from the official host; verify the printed toolchain hash/version in
`rustup show` before use. Spike crates never touch user data or the app database. `.tmp/` is git-ignored so
no downloaded binary is committed.

### FILE: phase-03-app-scaffold-typed-ipc.md

---
phase: 03
title: "App Scaffold & Typed IPC Spine"
status: pending
priority: P1
effort: 6h
dependencies: [02]
---

## Overview

Hand-build the real project (no `create-tauri-app` template, so every pin and every config key is
deliberate): Bun workspace, Vite 8 + Svelte 5 runes + Tailwind 4, strict TS, the Rust app skeleton,
structured tracing to rolling files, the generated TypeScript bindings, and the single `bun run gate`
command every later phase reuses.

## Requirements

**Functional**
- `package.json` scripts exactly per §67: `dev`, `build`, `check`, `test`, `tauri`, `tauri:dev`, `tauri:build`, plus `gate` and `fmt`.
- Rust workspace `src-tauri/` with modules `app/ commands/ domain/ storage/ analysis/ privacy/ redact/ platform/ collectors/` created as real modules (empty `mod.rs` files are structure, not stubs).
- `specta` 1.0.5 + `tauri-specta` 1.0.2 export `src/lib/tauri/bindings.ts`; gate fails if the file is stale.
- First real command: `get_diagnostics` (§84 subset available now — app version, `tauri::webview_version()`, monitoring status, log directory) — no placeholder command.
- Tracing: `tracing-subscriber` 0.3.23 (`env-filter,fmt,json,time`) + `tracing-appender` 0.2.5 daily rolling files under `%APPDATA%\WhatChanged\logs\`, verbosity from settings/`WHATCHANGED_LOG` (default `info`).
- Strict CSP, `withGlobalTauri: false`, `assetProtocol.enable: false`, minimal `capabilities/default.json`.

**Non-functional**
- TS: `strict`, `noUncheckedIndexedAccess`, `verbatimModuleSyntax`, `moduleResolution: bundler`, no `any`.
- Cargo profile: `[profile.release] lto = true, codegen-units = 1, strip = true` and **`panic` left at `unwind`** — collector isolation depends on unwinding a panicking task (§58).
- `build.target = "chrome120"` in Vite (WebView2 Evergreen is 151 here and auto-updates; 120 keeps older Evergreen safe).

## Architecture

`lib.rs` → `app::run()` builds the Tauri app: settings → logging → (later) storage → (later) collector
supervisor → commands. `commands/` holds only thin validators that call services; no logic. `platform/mod.rs`
carries `#[cfg(not(target_os = "windows"))] compile_error!("What Changed MVP is Windows-only — see ROADMAP.md")`.

## Related Code Files

**Create:** `package.json`, `bun.lock` (generated), `vite.config.ts`, `svelte.config.js`, `tsconfig.json`,
`.prettierrc`, `index.html`, `src/app/main.ts`, `src/app/App.svelte`, `src/lib/tauri/{client.ts,bindings.ts}`,
`src/styles/app.css`, `scripts/gate.ts`, `src-tauri/Cargo.toml`, `src-tauri/build.rs`,
`src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/lib.rs`,
`src-tauri/src/main.rs`, `src-tauri/src/app/{mod.rs,logging.rs,error.rs}`,
`src-tauri/src/commands/{mod.rs,diagnostics.rs}`, `src-tauri/src/{domain,storage,analysis,privacy,redact,collectors}/mod.rs`,
`src-tauri/src/platform/{mod.rs,windows/mod.rs}`, `src-tauri/tests/architecture-gates.rs`
**Modify:** `docs/STUBS.md`, `docs/IPC-SURFACE.md`. **Delete:** `spikes/tauri-smoke/`, `spikes/ts7-check/`.

## Implementation Steps

1. `bun init -y` in the repo root; rewrite `package.json` with the exact pins from `docs/decisions/0002` (svelte 5.56.10, @sveltejs/vite-plugin-svelte 7.3.0, vite 8.2.2, tailwindcss 4.3.3, @tailwindcss/vite 4.3.3, typescript per D2, svelte-check 4.7.6, vitest 4.1.11, @testing-library/svelte 5.4.2, @tauri-apps/cli 2.11.4, @tauri-apps/api 2.11.1, three 0.185.1, @types/three 0.185.4, @tanstack/svelte-virtual 3.13.36) — all exact, no `^`. `prettier` + `prettier-plugin-svelte` pinned to whatever `bun add -D --exact` resolves, recorded in `0002`.
2. `vite.config.ts`: `svelte()`, `tailwindcss()`, `server: { port: 1420, strictPort: true }`, `build: { target: "chrome120", sourcemap: true }`, `clearScreen: false`.
3. `svelte.config.js`: `vitePreprocess()`, `compilerOptions: { runes: true }`; treat compiler a11y warnings as errors via the `check` script's `--fail-on-warnings`.
4. `src-tauri/Cargo.toml`: `tauri = { version = "2.11.5", features = ["tray-icon"] }`, `tauri-build = "2.6.3"`, `serde`/`serde_json`/`tracing` pinned exactly as `cargo add` resolves, `thiserror = "2.0.20"`, `tokio = { version = "1.53.1", features = ["rt-multi-thread","sync","time","macros"] }`, `tracing-subscriber = "0.3.23"`, `tracing-appender = "0.2.5"`, `specta = "1.0.5"`, `tauri-specta = "1.0.2"`.
5. `app/error.rs`: `thiserror` `AppError` enum (`Storage`, `Collector`, `Analysis`, `InvalidInput`, `NotAvailable`, `Degraded`) + `impl serde::Serialize` so commands return typed errors to TS (§79).
6. Wire `tauri_specta` in `build`/`app::run` to export `src/lib/tauri/bindings.ts`; confirm the 1.0 builder API against `cargo doc -p tauri-specta` at this step — **if the builder shape differs from expectation, fall back to `ts-rs` 12.0.1 derives plus hand-written wrappers in `src/lib/tauri/client.ts`** and record the choice in `docs/decisions/0005-type-generation.md`.
7. `tauri.conf.json`: `productName: "What Changed"`, `identifier: "dev.whatchanged.app"`, `mainBinaryName: "WhatChanged"`, window `width: 1440, height: 900, minWidth: 900, minHeight: 600, visible: true, decorations: false` (custom chrome, Phase 06), `app.security.csp = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self' ipc: http://ipc.localhost"`, `app.security.assetProtocol.enable = false`, `app.withGlobalTauri = false`. Validate by running `bunx tauri dev` — unknown keys fail fast.
8. `capabilities/default.json`: `windows: ["main"]`, `permissions: ["core:default"]` only; every future permission is added by the phase that needs it with a justification line in `docs/IPC-SURFACE.md`.
9. `scripts/gate.ts` (Bun): runs `prettier --check .`, `svelte-check --fail-on-warnings`, `tsc --noEmit`, `vitest run`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, then `git diff --exit-code -- src/lib/tauri/bindings.ts` to catch binding drift. Uses `--manifest-path`/absolute paths, and routes cargo through PowerShell if `0001` said so.
10. `tests/architecture-gates.rs`: source scans — (a) no `windows::`/`windows_sys::` outside `src/platform/windows/`, (b) no `std::process::Command` anywhere in `src/`, (c) every `#[tauri::command]` name appears in `docs/IPC-SURFACE.md`, (d) every `todo!()`/`unimplemented!()`/`STUB:` marker is listed in `docs/STUBS.md`.
11. Delete both spike crates; remove their `docs/STUBS.md` rows. Commit: `feat: scaffold tauri app with typed ipc and gate`.

## Todo

- [ ] `bun install` clean; `bun run tauri dev` opens a window that calls `get_diagnostics` and renders the real values
- [ ] `bindings.ts` generated and committed; gate detects drift
- [ ] Rolling log file created under `%APPDATA%\WhatChanged\logs\` with a JSON line per event
- [ ] All four architecture gates pass on the empty skeleton
- [ ] Spikes deleted; `docs/STUBS.md` back to empty

## Success Criteria

- [ ] `bun run gate` exits 0
- [ ] `bunx tauri build --no-bundle` produces `src-tauri/target/release/WhatChanged.exe`
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml architecture_gates` passes with 4 assertions
- [ ] Grep proves `capabilities/default.json` contains no `shell`, `fs`, or `sql` permission

## Verification Gate

```bash
cd "D:/Project/.WhatChage" && bun install && bun run gate
cd "D:/Project/.WhatChage" && bunx tauri build --no-bundle
grep -E '"(shell|fs|sql)' "D:/Project/.WhatChage/src-tauri/capabilities/default.json" && echo "FAIL: broad permission" || echo "OK"
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `tauri-specta` 1.0 builder API differs from assumption | Compile error in `app::run` at the export call | Step 6's recorded fallback: `ts-rs` 12.0.1 + hand-written typed wrappers; the gate's drift check still applies |
| Vite 8 / Tailwind 4 / Svelte 5 plugin incompatibility | `bun run dev` fails to transform CSS or `.svelte` | Drop `@tailwindcss/vite` for the PostCSS entry path (`tailwindcss` + `@tailwindcss/postcss`), still Tailwind 4, recorded in `0002` |
| Strict CSP breaks Vite HMR in dev | Blank window in `tauri dev`, CSP errors in console | Use `app.security.devCsp` for the dev-only relaxation; production CSP stays strict |
| Clippy `-D warnings` blocks progress on empty modules | `dead_code` denials | Allow `dead_code` only on the not-yet-populated module declarations, with a `docs/STUBS.md` row that Phase 07/08 removes |

## Security Considerations

The capability file starts at `core:default` and grows only by justified addition — this is the primary
defense against a malicious frontend (§82). `withGlobalTauri: false` keeps `window.__TAURI__` out of the
page. Logging is configured **before** any data-carrying module exists, so no unredacted payload type can
predate the logging policy. `get_diagnostics` returns no user paths beyond the app's own data directory.

### FILE: phase-04-storage-foundation-migrations.md

---
phase: 04
title: "Storage Foundation & Migrations"
status: pending
priority: P1
effort: 6h
dependencies: [03]
---

## Overview

SQLite becomes real: one writer connection on a dedicated blocking thread, a small read pool, all 13
§10 tables with the six mandated indexes, migrations with a pre-migration backup, and the §83 corruption
path that must never crash-loop. Settings land here because everything later reads them.

## Requirements

**Functional**
- `rusqlite` 0.40.2 (`features = ["bundled","backup"]`) + `rusqlite_migration` 2.6.0.
- Pragmas in the correct order at creation: `page_size` and `auto_vacuum = INCREMENTAL` **before** the first table, then `journal_mode = WAL`, `synchronous = NORMAL`, `busy_timeout = 5000`, `foreign_keys = ON`, `wal_autocheckpoint = 512`.
- Tables (§10): `events, entities, entity_relationships, snapshots, checkpoints, health_states, collector_status, settings, watched_locations, applications, analysis_sessions, analysis_results, schema_migrations`. `health_states` is created but unused — §26 is the user's own deferral, noted in `ROADMAP.md`.
- Indexes (§10): `events(timestamp)`, `events(category,timestamp)`, `events(entity_id,timestamp)`, `events(event_type,timestamp)`, `entity_relationships(source_id)`, `entity_relationships(target_id)`; plus `events(importance,timestamp)` for "Important only" and `entities(entity_type,display_name)` for search.
- `schema_migrations` rows (version, name, applied_at, sql_sha256) written by the runner in addition to `user_version`.
- Backup before any migration that is not the initial create: copy to `%APPDATA%\WhatChanged\backups\events.pre-v<N>.db` via `rusqlite::backup`.
- Open sequence: `PRAGMA quick_check` → on failure enter `DegradedMode`; a `app-state.json` failure counter forces degraded mode after 3 consecutive failed opens instead of retrying.
- Recovery operations (§83): `repair_database` (backup-copy into a fresh file, then row-salvage scan skipping unreadable pages), `export_diagnostics` (Phase 22 completes the UI), `reset_local_history` (rename to `events.corrupt-<epoch>.db`, create fresh).
- Commands `get_settings` / `update_settings` (§56) with typed setting keys — no free-form key/value from the frontend.

**Non-functional**
- No `unwrap()`/`expect()` in `storage/` outside tests (clippy `-D clippy::unwrap_used` for that module).
- All writes go through prepared statements held in a per-connection cache; every multi-row write is one transaction.

## Architecture

`storage/db.rs` owns `Database { writer: mpsc::Sender<WriteJob>, readers: ReaderPool }`. The writer thread is
sync `rusqlite` inside `std::thread` with a bounded channel; readers are 4 read-only connections leased via
`tokio::task::spawn_blocking`. No async SQLite crate, no second runtime (research §4.1). `tauri-plugin-sql`
is explicitly rejected — it would put SQL in the frontend (§35/§56).

## Related Code Files

**Create:** `src-tauri/src/storage/{mod.rs,db.rs,pragmas.rs,migrations.rs,settings.rs,recovery.rs,paths.rs}`,
`src-tauri/migrations/{0001_init.sql,0002_indexes.sql}`, `src-tauri/src/commands/settings.rs`,
`src-tauri/tests/{migrations.rs,storage_recovery.rs}`, `src-tauri/tests/fixtures/corrupt-db-recipe.md`
**Modify:** `src-tauri/Cargo.toml`, `src-tauri/src/app/mod.rs`, `src-tauri/src/lib.rs`, `docs/IPC-SURFACE.md`, `ARCHITECTURE.md`
**Delete:** none.

## Implementation Steps

1. `cargo add rusqlite --features bundled,backup --manifest-path src-tauri/Cargo.toml` (pin `0.40.2`), `cargo add rusqlite_migration@2.6.0`.
2. `storage/paths.rs`: resolve `%APPDATA%\WhatChanged\` via `tauri::Manager::path().app_data_dir()`; subdirs `db/`, `logs/`, `backups/`. Never derive a path from the repo folder name (D1).
3. `storage/pragmas.rs`: apply pragmas in the documented order; assert `journal_mode` returns `wal` and fail startup loudly if not.
4. `migrations/0001_init.sql`: all 13 tables. `events` columns mirror §9 exactly (`id INTEGER PRIMARY KEY, timestamp INTEGER NOT NULL, category TEXT, event_type TEXT, entity_type TEXT, entity_id TEXT, entity_display_name TEXT, severity TEXT, importance TEXT, source_collector TEXT, before_json TEXT, after_json TEXT, metadata_json TEXT, user_friendly_summary TEXT, technical_summary TEXT, confidence REAL, sensitive INTEGER NOT NULL DEFAULT 0`); `related_entity_ids` is normalized into `entity_relationships` + an `event_entities` join written in Phase 08 — record that in `ARCHITECTURE.md` as the one intentional §9 normalization.
5. `migrations/0002_indexes.sql`: the eight indexes.
6. `storage/migrations.rs`: `Migrations::new(vec![M::up(include_str!(...)), ...])`, `to_latest`, then insert the `schema_migrations` audit row per applied version.
7. `storage/recovery.rs`: `quick_check`, degraded-mode state machine, the three recovery ops, and the `app-state.json` failure counter.
8. `storage/settings.rs`: typed `Settings` struct (theme, retention_days, important_retention_days, snapshot_retention_days, collector toggles, poll intervals, notifications, developer_mode, autostart, log_level, ai_provider = `None`) serialized as one JSON row plus per-key rows for cheap partial updates; defaults exactly per §11/§50.
9. `commands/settings.rs`: `get_settings`, `update_settings` with server-side validation (retention ∈ {7,30,90,custom 1..3650}, poll interval ∈ 500..5000 ms).
10. Tests: migrate-from-empty; migrate from each intermediate version using checked-in snapshot DBs generated by a helper; idempotent re-run; pragma assertions; corruption test (copy a good DB, zero 4 KB at offset 8192, assert `open()` yields `Degraded` and does not panic); backup-file existence.
11. Commit: `feat(storage): sqlite foundation, migrations and recovery`.

## Todo

- [ ] 13 tables + 8 indexes exist after `to_latest` on an empty file
- [ ] `schema_migrations` has one row per migration with a sha256
- [ ] Corrupted DB opens into `DegradedMode`, writes no panic, and the failure counter caps retries at 3
- [ ] `get_settings`/`update_settings` reject out-of-range input with `AppError::InvalidInput`

## Success Criteria

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml migrations storage_recovery` green
- [ ] `PRAGMA journal_mode` returns `wal` and `PRAGMA auto_vacuum` returns `2` (incremental) on a fresh DB
- [ ] `sqlite3`-free verification: a test asserts index names via `PRAGMA index_list('events')`
- [ ] Deleting the DB file and restarting recreates it silently (first-run path)

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test migrations -- --nocapture
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test storage_recovery -- --nocapture
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `auto_vacuum` set after table creation silently does nothing | `PRAGMA auto_vacuum` returns 0 on a fresh DB | Pragma-order assertion in step 3 fails the test; fix order (it cannot be changed later without a full `VACUUM`) |
| WAL files left behind after unclean shutdown | `.db-wal` grows unbounded | `wal_autocheckpoint = 512` plus `PRAGMA wal_checkpoint(TRUNCATE)` on graceful shutdown; recovery path tolerates a stale WAL |
| Writer thread panics and drops the channel | Send errors on every subsequent write | Writer wrapped in a supervisor that restarts it once, re-opens the connection, and flips to `DegradedMode` on a second failure — surfaced in Diagnostics |
| `bundled` SQLite lacks FTS5 | `SELECT sqlite_compileoption_used('ENABLE_FTS5')` returns 0 | Phase 08's search uses the prefix-index path instead; probe result recorded in `docs/decisions/0006-search-strategy.md` |

## Security Considerations

The database lives under the user's roaming profile, not the repo. No frontend-supplied SQL, table name, or
column name is ever accepted — `update_settings` takes a typed struct. Backup files inherit the same
directory ACLs. Migration SQL is `include_str!`-embedded, so it cannot be swapped on disk at runtime.

### FILE: phase-05-windows-shell-tray-installer.md

---
phase: 05
title: "Windows Shell, Tray & Installer"
status: pending
priority: P1
effort: 5h
dependencies: [04]
---

## Overview

The §7 process lifecycle: one instance, tray with the seven exact entries, close-to-tray, pause/resume
monitoring state, optional autostart, and a per-user NSIS installer that survives upgrades without touching
the database. This closes the §69 Phase-0 acceptance list except for the UI (Phase 06).

## Requirements

**Functional**
- Plugins: `tauri-plugin-single-instance` 2.4.3, `tauri-plugin-autostart` 2.5.1, `tauri-plugin-dialog` 2.7.2, `tauri-plugin-notification` 2.3.3, `tauri-plugin-updater` 2.10.1, `tauri-plugin-opener` 2.5.4. Each justified in `docs/IPC-SURFACE.md`; `tauri-plugin-log` and `tauri-plugin-window-state` are explicitly **rejected** (§80: tracing already covers logging; window persistence is unrequested).
- Tray menu, exactly and in this order (§7): `Open What Changed`, `What changed today?`, `Create checkpoint`, `Pause monitoring`, `Resume monitoring`, `Settings`, `Quit`. Pause/Resume enable-state reflects the monitoring state.
- `WindowEvent::CloseRequested` → `api.prevent_close()` + `window.hide()`; monitoring continues (§7). Quit is the only exit path.
- Single instance: second launch focuses and un-hides the existing window.
- Monitoring state machine (`Running | Paused | Degraded`) persisted, broadcast via the `monitoring-state-changed` event (§57), exposed by `set_monitoring_paused`.
- Autostart: off until onboarding/Settings enables it; uses the plugin's per-user registry Run entry (non-elevated).
- NSIS bundle: `targets: ["nsis"]`, per-user install mode, Start Menu shortcut, optional desktop shortcut, `webviewInstallMode` per `docs/decisions/0004`.
- Upgrade safety (§65): installer must never remove `%APPDATA%\WhatChanged\db`; verified by an install → upgrade → event-count test in Phase 24.

**Non-functional**
- Tray icon assets bundled locally (§63): no CDN, no runtime download.
- Tray click handling must not block the async runtime; menu handlers dispatch commands and return immediately.

## Architecture

`app/tray.rs` builds `TrayIconBuilder` with a `Menu`; handlers post messages onto an `AppCommand` channel
consumed by `app/controller.rs`, which owns the monitoring state and (from Phase 10) the supervisor. Tray →
controller → services, never tray → storage directly.

## Related Code Files

**Create:** `src-tauri/src/app/{tray.rs,controller.rs,lifecycle.rs,autostart.rs}`,
`src-tauri/src/commands/monitoring.rs`, `src-tauri/icons/` (tray + app icons, 16/32/128/256 + `.ico`),
`src-tauri/installer/hooks.nsh` (only if the CLI supports installer hooks), `docs/decisions/0007-installer.md`
**Modify:** `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/Cargo.toml`,
`src-tauri/src/app/mod.rs`, `docs/IPC-SURFACE.md`, `README.md`
**Delete:** none.

## Implementation Steps

1. Add the six plugins at their pinned versions; add only the specific permissions each needs to `capabilities/default.json` (`autostart:allow-enable|disable|is-enabled`, `dialog:allow-open` folder-only, `notification:default`, `updater:allow-check`, `opener:allow-open-path` **scoped to the app data dir**), each with a one-line justification in `docs/IPC-SURFACE.md`.
2. Generate icons from a single 1024px source with `bunx tauri icon` and commit the outputs (offline requirement §63).
3. `app/tray.rs`: menu with the seven items; `MenuEvent` match arms → `AppCommand::{ShowWindow, ShowToday, CreateCheckpoint, Pause, Resume, ShowSettings, Quit}`. `ShowToday`/`ShowSettings` show the window and emit a `navigate` payload the frontend consumes.
4. `app/lifecycle.rs`: `on_window_event` close interception + hide; `RunEvent::ExitRequested` prevention unless the quit flag is set; graceful shutdown flushes the writer, checkpoints the WAL, and stops collectors.
5. `app/controller.rs`: monitoring state, persistence in settings, `monitoring-state-changed` emit, and a `MonitoringGate` handle later consumed by the supervisor.
6. `commands/monitoring.rs`: `set_monitoring_paused(paused: bool)`, `get_monitoring_state()`.
7. `app/autostart.rs`: wrap the plugin behind `set_autostart(enabled)` inside `update_settings` (no separate command).
8. `tauri.conf.json` bundle block: `targets`, `bundle.windows.nsis` per `0004` (per-user, shortcuts, languages `["English"]`); if the installed CLI exposes an installer-hooks key, add `installer/hooks.nsh` with an uninstall-time "also remove history?" prompt; **if it does not, implement the equivalent as Settings → Storage → "Delete all history"** and state the limitation in `README.md` (§65 says "where technically feasible").
9. `bun run tauri build`; install the produced setup executable non-elevated; verify Start Menu entry, launch, close-to-tray, tray items, uninstall.
10. Commit: `feat(app): tray, lifecycle, autostart and nsis installer`.

## Todo

- [ ] Seven tray items present, in order, with working handlers
- [ ] Closing the window hides it; the process stays alive; Quit exits cleanly
- [ ] Second launch focuses the first instance instead of starting a second
- [ ] `bun run tauri build` produces a per-user setup executable that installs and uninstalls without elevation
- [ ] `docs/IPC-SURFACE.md` lists every added capability permission with a justification

## Success Criteria

- [ ] Task Manager shows exactly one `WhatChanged.exe` after a second launch attempt
- [ ] After install → uninstall → reinstall, `%APPDATA%\WhatChanged\db\events.db` still holds its rows (manual check with the file size/mtime, automated in Phase 24)
- [ ] `capabilities/default.json` still contains no `shell`, `fs`, or `sql` permission (gate assertion)
- [ ] Tray "Pause monitoring" flips the state, emits `monitoring-state-changed`, and survives a restart

## Verification Gate

```bash
cd "D:/Project/.WhatChage" && bun run gate
cd "D:/Project/.WhatChage" && bun run tauri build
ls -la "D:/Project/.WhatChage/src-tauri/target/release/bundle/nsis/"
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| NSIS installer-hook key absent in this CLI version | `tauri build` rejects the config key | Fall back to the in-app "Delete all history" action (step 8) and document; do not fake an uninstall prompt |
| WebView2 bootstrapper download path unavailable offline during install | Installer stalls on WebView2 step | Switch `webviewInstallMode` to the offline/embedded option the CLI reports in `0004`; note the installer size increase |
| Autostart entry written to the wrong hive | Entry missing from `HKCU\...\Run` after enabling | Verify with a registry read (probe #24 shows HKCU Run is readable non-elevated); if the plugin writes elsewhere, record the actual location in `0007` |
| Hide-to-tray leaks a WebView2 process per hide | RAM climbs after repeated hide/show | Hide (not destroy) the window and reuse it; add the hide/show loop to the Phase 24 RAM protocol |

## Security Considerations

Per-user install means no elevation at any point (§36). Only six plugin permissions exist, each scoped; the
`opener` scope is restricted to the app data directory so a compromised frontend cannot open arbitrary paths.
The updater's public key is configured but no auto-download is enabled (§64). Tray handlers accept no
frontend input.

### FILE: phase-06-design-system-skeleton-screens.md

---
phase: 06
title: "Design System & Skeleton Screens"
status: pending
priority: P1
effort: 8h
dependencies: [05]
---

## Overview

The visual identity and the six-state discipline are installed **before** any data screen exists, so no
screen can later be built without them. Hand-written primitives (shadcn/Bits UI *concepts* only, §4) with
the app's own identity: dark, calm, precise, premium (§40). §90's polished initial shell lands here.

## Requirements

**Functional**
- Tokens (§41) as CSS variables in `src/styles/tokens.css`, exposed to Tailwind 4 via `@theme`: `background`, `surface-1/2/3`, `border-subtle`, `border-active`, `text-primary/secondary/muted`, `accent`, `success`, `warning`, `danger`, `info`, `shadow-sm/md/lg`, `radius-sm/md/lg/xl`, spacing scale (4/8/12/16/20/24/32/40/56/72), motion durations (`fast 120ms`, `base 200ms`, `slow 320ms`), easings (`standard`, `emphasis`, `spring`).
- Themes: Dark (MVP default), Light, System — `data-theme` on `<html>`, `prefers-color-scheme` listener, persisted in settings.
- Primitives in `src/components/ui/`: Button, IconButton, Card, Badge, Pill, Input, Textarea, Select, Switch, Checkbox, Tabs, Tooltip, Dialog, Drawer, Table, Separator, Kbd, Spinner, Skeleton, EmptyState, ErrorState, StateShell.
- `StateShell` takes `state: 'loading'|'empty'|'error'|'success'|'large'|'small'` snippets so **every screen structurally declares all six states** (§70).
- App shell: custom titlebar (drag region, minimize/maximize/close, "● Active" status pill), left sidebar nav (Home, Timeline, Investigations, Checkpoints, System Map, Settings), content region, command-palette-free (unrequested).
- Nine routes registered by a typed store router: `home`, `timeline`, `investigations`, `checkpoints`, `system-map`, `settings`, `onboarding`, `diagnostics`, `recovery` (§83 degraded screen).
- Copy deck as code: `src/lib/copy/index.ts` typed const tree holding every §72/§73/§74/§43/§45/§46/§47/§50 string verbatim; components import from it — no inline literals in screens.
- Layout support down to 900×600 with desktop density (§71): sidebar collapses to icons under 1100px; two-column detail layouts stack under 1000px.

**Non-functional**
- Semantic HTML, visible focus rings from `border-active`, all interactive elements keyboard reachable (§62).
- `prefers-reduced-motion` disables transitions globally via a single token override.
- No screen file over ~200 lines; sections extracted to `features/<screen>/components/`.

## Architecture

`src/app/App.svelte` mounts `AppShell` + the router store. Feature folders per §55 own their screens;
`components/ui` owns primitives; `lib/design/` holds token TS mirrors (for Three.js colors in Phase 19) and
the contrast test's token table. Screens whose data arrives later render the real empty state, not fake data —
each is registered in `docs/STUBS.md` with the phase that wires it.

## Related Code Files

**Create:** `src/styles/{tokens.css,app.css,typography.css}`, `src/components/ui/*.svelte` (21 files),
`src/components/system/{AppShell.svelte,TitleBar.svelte,Sidebar.svelte,StatusPill.svelte}`,
`src/lib/stores/{router.svelte.ts,theme.svelte.ts,settings.svelte.ts}`, `src/lib/design/tokens.ts`,
`src/lib/copy/index.ts`, `src/features/{home,timeline,investigation,checkpoints,search,settings,onboarding}/`
screen entry files, `src/features/system-map/SystemMapScreen.svelte`, `src/features/diagnostics/DiagnosticsScreen.svelte`,
`src/features/recovery/RecoveryScreen.svelte`, `src/routes/dev-states-gallery.svelte`,
`src/lib/copy/copy.test.ts`, `src/lib/design/contrast.test.ts`, `docs/decisions/0003-microcopy.md`
**Modify:** `src/app/App.svelte`, `docs/ui-states-matrix.md`, `docs/STUBS.md`, `PRODUCT.md`
**Delete:** none.

## Implementation Steps

1. `src/styles/tokens.css`: `:root[data-theme="dark"]` and `[data-theme="light"]` blocks with every §41 token; `@theme inline` maps them to Tailwind utilities so classes read `bg-surface-2 text-text-secondary`.
2. Typography: one bundled variable font (`src/assets/fonts/`, `@font-face` local only — §63 forbids an online font); scale 12/13/14/16/20/24/32 with tabular numerals for timestamps and ports.
3. Build the 21 primitives with snippet-based composition; each gets `aria-*` wiring and a focus-visible style; Dialog/Drawer trap focus and restore it on close.
4. `StateShell.svelte`: renders the snippet for the active state; `large` and `small` are layout variants (dense list / stacked) selected by a `container-query`-driven store, not by user choice.
5. `AppShell` + `TitleBar` with `data-tauri-drag-region`, window controls via `getCurrentWindow()`, and the status pill bound to monitoring state from Phase 05.
6. Router store: `type Route = 'home' | ... ` union, `navigate(route, params)`, back/forward via an internal stack, deep-link entry from the tray `navigate` payload.
7. Write all nine screens as shells: real headings, real empty-state copy from the copy deck, six states declared, zero fake data. Register the five data-dependent screens in `docs/STUBS.md`.
8. `docs/decisions/0003-microcopy.md`: resolve the placeholder conflict — heading `What stopped working?` and placeholder `Example: Discord stopped detecting my microphone this morning` (§72 wins as the explicit "exact voice" section); record §31's `Tell me what stopped working…` and §43's `Tell me what happened…` as alternates kept in the copy deck under `alternates` so the user can switch without a code change.
9. `copy.test.ts`: assert the §72 strings byte-for-byte (`Everything looks normal.`, `2 recent changes may need attention.`, `What stopped working?`, `Important changes`, `Investigate`, `Your history stays on this PC.`).
10. `contrast.test.ts`: compute WCAG contrast for every text-on-surface token pair in both themes; fail under 4.5:1 for body text and 3:1 for large text/borders.
11. `dev-states-gallery.svelte`: reachable only when `developer_mode` is on; renders every screen in all six states for visual review.
12. Commit as three chunks: `feat(ui): design tokens and theming`, `feat(ui): primitive component library`, `feat(ui): app shell and screen skeletons`.

## Todo

- [ ] Every §41 token exists in both themes and is used by at least one primitive
- [ ] 21 primitives render, are keyboard operable, and have no inline color literals
- [ ] Nine routes reachable; five registered in `docs/STUBS.md` as data-pending
- [ ] `docs/ui-states-matrix.md` filled in with the six states per screen (owner phase per cell)
- [ ] Theme switch (Dark/Light/System) persists across restart

## Success Criteria

- [ ] `bun run test` passes `copy.test.ts` and `contrast.test.ts`
- [ ] `svelte-check --fail-on-warnings` reports zero a11y warnings
- [ ] At 900×600 no screen horizontally scrolls and no control is clipped (manual, recorded in the states matrix)
- [ ] Grep finds no hard-coded hex color in `src/features/` or `src/components/`

## Verification Gate

```bash
cd "D:/Project/.WhatChage" && bun run gate
grep -rEn "#[0-9a-fA-F]{3,8}\b" "D:/Project/.WhatChage/src/features" "D:/Project/.WhatChage/src/components" && echo "FAIL: raw color" || echo "OK: tokens only"
cd "D:/Project/.WhatChage" && bun run tauri dev   # manual: resize to 900x600, tab through every control
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Result looks like a generic shadcn dashboard (§70 explicitly unacceptable) | Screens read as card-grid-on-black | Identity checklist in `PRODUCT.md`: single accent, one elevation ladder, hairline borders over shadows, dense left rail, no gradient blobs — reviewed at the end of this phase and again in Phase 23 |
| Custom titlebar breaks native window behaviors (snap, double-click maximize) | Window cannot be snapped or maximized by gesture | Keep `decorations: false` but implement drag region + double-click maximize handler; if snapping cannot be restored, switch to `decorations: true` and keep the in-app header — recorded as a decision |
| Tailwind 4 `@theme` cannot consume runtime-swapped CSS variables | Theme switch changes nothing | Tokens stay plain CSS variables consumed via `var()` in a thin utility layer; Tailwind used for layout/spacing only |
| Six-state discipline decays as screens grow | A screen renders data without an empty branch | `StateShell` is the only way a screen composes content; Phase 23's suite asserts all 54 cells |

## Security Considerations

The states gallery and any dev affordance are gated behind `developer_mode` (§85) and are not reachable in a
default install. No remote assets, fonts, or icon CDNs (§63). Copy deck strings are static — no user-supplied
string is ever rendered as HTML (`{@html}` is banned by a grep in the gate).

### FILE: phase-07-domain-model-redaction-chokepoint.md

---
phase: 07
title: "Domain Model & Redaction Chokepoint"
status: pending
priority: P1
effort: 7h
dependencies: [06]
---

## Overview

The single most security-critical phase. The §9 domain types are defined once in Rust and exported to TS,
and the **only** way to produce a persistable event is `Redactor::seal()`. Because no repository insert
exists yet (Phase 08 adds it against `SealedEvent`), no collector can ever predate redaction.

## Requirements

**Functional**
- `SystemEvent` with the exact §9 field set; `EventCategory` (15 variants), `EventType` (19 variants), `Importance` (`Informational|Notable|Important|Critical`, §54), `Severity`, `Confidence` (`Confirmed|Likely|Correlated`, §29), `EntityType`, `RelationKind` (the 11 §30 relations).
- `RawEvent` (pre-redaction, produced by normalizers) and `SealedEvent` (post-redaction, private field, no public constructor). `Redactor::seal(RawEvent) -> SealedEvent` is the sole producer.
- `Redactor` detects (§34): `sk-`-style API keys, `Bearer <token>`, 3-segment JWTs, AWS-style `AKIA/ASIA` + 40-char secrets, PEM private-key blocks, `postgres|mysql|mongodb|redis|amqp://user:pass@host` credentials, `Set-Cookie`/session tokens, and any value whose **key name** matches the §13 concept list (`password, passwd, secret, token, apikey, api_key, authorization, cookie, session, private_key, client_secret`) case/separator-insensitively. Redacted values become the literal `[REDACTED]`; the key name is preserved.
- Key-name matching is the primary rule (a weak-entropy password must still be redacted); value-shape matching is the secondary rule (an unnamed `sk-...` in free text must still be caught).
- Manual `Debug`/`Display` for `RawEvent`, `ConfigDiff`, and `MetadataMap` emitting redacted output only; a `RedactingLayer` for `tracing` scrubs recorded string fields as defense in depth.
- `sensitive: bool` on the event is set true whenever any redaction fired, so the UI can label it.

**Non-functional**
- Redaction is pure, allocation-conscious, and runs in the collector's task (not the writer) so a flood cannot serialize on it.
- False-positive policy documented: a redacted-but-harmless value is acceptable; a leaked secret is not.
- Every type derives `Serialize, Deserialize, specta::Type`; `bindings.ts` regenerates in the same commit.

## Architecture

```
collector probe → normalize.rs → RawEvent → Redactor::seal → SealedEvent → (Phase 08) ingest queue → SQLite
                                     └── Debug = redacted ──→ tracing → rolling log file
```

`redact/` is the only module that can name `SealedEvent`'s constructor; `tests/architecture-gates.rs` asserts
that no other file contains a `SealedEvent {` struct literal and that `storage::events::insert` takes
`&SealedEvent`.

## Related Code Files

**Create:** `src-tauri/src/domain/{event.rs,category.rs,event_type.rs,importance.rs,severity.rs,entity.rs,relationship.rs,confidence.rs,metadata.rs,normalize.rs}`,
`src-tauri/src/redact/{mod.rs,patterns.rs,key_names.rs,value_shapes.rs,seal.rs,tracing_layer.rs}`,
`src-tauri/tests/redaction-leak.rs`, `src-tauri/tests/fixtures/secret-specimens.json`,
`src-tauri/src/privacy/{mod.rs,policy.rs}`
**Modify:** `src-tauri/src/domain/mod.rs`, `src-tauri/src/redact/mod.rs`, `src-tauri/src/app/logging.rs`,
`src/lib/tauri/bindings.ts` (generated), `src-tauri/tests/architecture-gates.rs`, `SECURITY.md`, `PRIVACY.md`
**Delete:** none.

## Implementation Steps

1. Define the enums as `#[serde(rename_all = "snake_case")]` so the DB and TS use the §9 vocabulary literally.
2. `domain/event.rs`: `RawEvent` with all §9 fields; `metadata: MetadataMap` (`BTreeMap<String, MetaValue>` where `MetaValue` is `Str|Int|Bool|List|Map`) so redaction can walk it structurally instead of regexing JSON.
3. `redact/key_names.rs`: normalize a key by lowercasing and stripping `_-. ` then match against the concept list plus common variants (`pwd`, `passphrase`, `access_token`, `refresh_token`, `auth`, `x-api-key`).
4. `redact/value_shapes.rs`: compiled matchers for the eight §60 shapes. Implement with hand-written scanners over `&str` (no regex crate — §80: `regex` would be a large dep for fixed prefixes/segment counts); document the choice.
5. `redact/seal.rs`: walk every string in `before`, `after`, `metadata`, `technical_summary`, `user_friendly_summary`, `entity_display_name`; on any hit replace with `[REDACTED]`, set `sensitive = true`, and count hits per rule for Diagnostics.
6. `tracing_layer.rs`: a `Layer` that visits recorded string fields and applies `value_shapes` before they reach the formatter.
7. `privacy/policy.rs`: compile-time list of things never collected (§33) with a `debug_assert` helper that collectors call for the categories they emit; also the §14 rule that command lines are never captured.
8. `fixtures/secret-specimens.json`: at least 12 specimens — `sk-live-…`, `Bearer eyJ…`, a full 3-part JWT, `postgres://admin:hunter2@db:5432/app`, `mongodb+srv://u:p@cluster/db`, `AKIA…` + 40-char secret, a `-----BEGIN RSA PRIVATE KEY-----` block, `Set-Cookie: session=…`, `PASSWORD=hunter2`, `client_secret: abc…`, `X-API-Key: …`, `https://user:pass@host/path`.
9. `tests/redaction-leak.rs`: for each specimen, build a `RawEvent` carrying it in five places (before, after, metadata leaf, metadata nested map, technical_summary), seal it, serialize it, **and additionally scan the raw bytes of a temp DB file and the rolling log file after a full ingest round-trip once Phase 08 exists** (this phase asserts the serialization + log paths; Phase 08's gate re-runs the file-byte scan).
10. Regenerate bindings; update `SECURITY.md`/`PRIVACY.md` with the final rule list.
11. Commit: `feat(domain): system event model` then `feat(redact): sealed redaction chokepoint`.

## Todo

- [ ] All §9 fields and all 15/19/4 enum variants present and exported to TS
- [ ] `SealedEvent` unconstructable outside `redact` (gate test)
- [ ] 12 specimens × 5 placements redacted; `sensitive` set
- [ ] `RawEvent`'s `Debug` output contains no specimen substring
- [ ] Per-rule hit counters exposed for Diagnostics

## Success Criteria

- [ ] `cargo test --test redaction-leak` green with ≥60 assertions
- [ ] `cargo test --test architecture-gates` proves the sealed-constructor and insert-signature rules
- [ ] `bindings.ts` diff shows the new types and the gate's drift check passes
- [ ] A deliberate negative test: removing one `key_names` entry makes the suite fail (proves the tests bite)

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test redaction-leak -- --nocapture
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test architecture-gates
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Over-redaction destroys useful diffs (e.g. a port named `API_TOKEN_PORT`) | Users see `[REDACTED]` where a number belonged | Key-name rule wins by design; add a narrow numeric exception only when the whole value is a 1–5 digit integer **and** the key ends in `_port`, with a test proving `API_TOKEN` is still redacted |
| A secret hides in a shape no rule matches | Leak test passes but a real key persists | Add a high-entropy fallback: ≥32 chars, base64/hex alphabet, no whitespace → redact; documented as intentionally aggressive |
| Hand-written scanners have edge-case bugs | Specimen variants slip through | Property test: mutate each specimen (case, padding, surrounding punctuation, CRLF) and assert redaction holds |
| Redaction cost on a burst | Ingest latency spike under a file-event flood | Redaction happens per event in the collector task; Phase 24's burst test measures it and the bounded queue absorbs the rest |

## Security Considerations

This is the §34 chokepoint and the mitigation for the "secret leakage" row of the §82 threat model. Two
independent mechanisms cover the two exits (persistence via the sealed type, logging via `Debug` + layer), so a
single mistake cannot open both. `privacy/policy.rs` encodes the §33 never-list as code rather than prose.

### FILE: phase-08-event-repository-ingest-retention.md

---
phase: 08
title: "Event Repository, Ingest & Retention"
status: pending
priority: P1
effort: 8h
dependencies: [07]
---

## Overview

The write path and the read path. A bounded queue absorbs bursts, a batching writer commits in transactions,
keyset pagination keeps the timeline O(page) at 1M events, retention bounds the database, and the perf harness
establishes the numbers `docs/perf-baseline.md` will be judged against.

## Requirements

**Functional**
- `EventSink` (clonable handle) → bounded `mpsc` (capacity 8192) → writer batches on **256 events or 500 ms**, one transaction per batch (§37 batched non-blocking writes).
- Overflow policy: drop **oldest** and increment `dropped_events`, surfaced in Diagnostics and logged at most once per minute (§82 event floods).
- Coalescing: identical `(entity_id, event_type)` within 2 s collapse into one event with an `occurrences` metadata count.
- Entity upsert + relationship upsert on ingest; `event_entities` join rows for §9 `related_entity_ids`.
- Queries: `get_events(filter, cursor, limit≤200)` using keyset pagination (`timestamp < ?, id < ?`), `get_event_details(id)` (event + related events + affected entities + source collector), `get_dashboard_summary(range)` (counts by importance for §43's "37 changes / 2 important / 0 critical"), `search_events(query, limit)`.
- Search strategy decided by probe: `SELECT sqlite_compileoption_used('ENABLE_FTS5')` → if 1, an FTS5 table over summaries + entity names kept in sync by triggers; else a normalized `search_index(term, event_id)` table with prefix indexes. Result recorded in `docs/decisions/0006-search-strategy.md`.
- Retention job: hourly, deletes in ≤5000-row transactions, respects three horizons (events 30 d, snapshots 7 d, important 90 d; options 7/30/90/custom), then `PRAGMA incremental_vacuum(200)`; yields if the ingest queue is >50% full (§11 never blocks, avoids write amplification).
- Storage estimate for Settings: `page_count × page_size` + WAL size + backups size.
- Synthetic generator (`domain/synthetic.rs`, §69 Phase 1) producing realistic events with `metadata.synthetic = true` and `source_collector = "simulator"` — the flag is written to the DB so a simulated event is never mistakable for a real one (§87).

**Non-functional**
- Perf targets (set here, re-baselined by measurement): page query p95 <25 ms at 1M events; `get_dashboard_summary` <50 ms; search <150 ms at 1M; ingest 10k events <5 s; retention pass over 100k expired rows <10 s without blocking a concurrent page query.
- No `SELECT *`; explicit column lists; every hot query prepared and cached.

## Architecture

`storage/events.rs` (insert/query), `storage/entities.rs`, `storage/relationships.rs`, `storage/ingest.rs`
(queue + batcher), `storage/retention.rs`, `storage/search.rs`. `insert_batch(&[SealedEvent])` is the only
insert; the compiler therefore guarantees redaction ran (§34). Commands live in `commands/events.rs` and only
validate + delegate.

## Related Code Files

**Create:** `src-tauri/src/storage/{events.rs,entities.rs,relationships.rs,ingest.rs,retention.rs,search.rs,stats.rs}`,
`src-tauri/src/domain/synthetic.rs`, `src-tauri/src/commands/events.rs`,
`src-tauri/migrations/0003_search.sql`, `src-tauri/tests/{repository.rs,retention.rs,perf-events.rs,ingest-flood.rs}`,
`docs/decisions/0006-search-strategy.md`
**Modify:** `src-tauri/src/storage/mod.rs`, `src-tauri/src/app/mod.rs`, `docs/IPC-SURFACE.md`,
`docs/perf-baseline.md`, `src-tauri/tests/redaction-leak.rs` (add the DB-byte scan), `ARCHITECTURE.md`
**Delete:** none.

## Implementation Steps

1. `storage/events.rs`: `insert_batch(&mut Transaction, &[SealedEvent])` with cached prepared statements; returns assigned ids for the live-event emit.
2. `storage/ingest.rs`: `EventSink::send(SealedEvent)`; batcher task with `tokio::time::interval` + size trigger; drop-oldest ring on overflow; counters (`accepted`, `dropped`, `coalesced`, `batches`, `last_flush_ms`).
3. `storage/entities.rs`: deterministic entity ids — `process:<lowercased exe path>` (or `process:name:<name>` when the path is denied, ~46% of processes per probe #3), `port:tcp/3000`, `service:Audiosrv`, `app:<normalized display name>`, `driver:<device instance id>`, `file:<canonical lowercase path>`, `adapter:<guid>`, `container:<id>`, `image:<repo:tag>`, `project:<canonical path>`. No UUID dependency.
4. `storage/relationships.rs`: upsert `(source_id, target_id, kind, first_seen, last_seen, confidence)`; the 11 §30 kinds as an enum.
5. `storage/search.rs`: run the FTS5 probe once at startup, pick the strategy, expose one `search(query, limit)` API so callers never branch.
6. `storage/retention.rs`: horizon resolution from settings, batched deletes ordered by `timestamp`, importance-aware (`importance IN ('important','critical')` keeps 90 d), `incremental_vacuum`, backpressure check against the ingest queue depth.
7. `commands/events.rs`: `get_events`, `get_event_details`, `get_dashboard_summary`, `search_events` with validation (limit ≤200, range ≤366 d, query length ≤128, filter category values must parse to the enum).
8. `domain/synthetic.rs`: seeded generator (`StdRng::seed_from_u64`) producing a realistic mix (70% informational, 20% notable, 8% important, 2% critical) across all 15 categories, plus named scenarios reused by Phase 09's dev panel and Phase 17's fixtures.
9. `tests/perf-events.rs`: `#[ignore]`d tests reading `WC_PERF_EVENTS` (10000/100000/1000000); generate → ingest → time page queries, filtered queries, search, and `get_dashboard_summary`; print a table and append it to `docs/perf-baseline.md`. No `criterion` dependency (§80).
10. `tests/ingest-flood.rs`: push 50k events as fast as possible into a capacity-8192 queue; assert no panic, `dropped_events > 0` accounted exactly, and that a concurrent page query still returns under 100 ms.
11. Extend `tests/redaction-leak.rs`: after a full ingest round-trip, read the `.db`, `.db-wal`, and log file as bytes and assert no specimen substring occurs (case-insensitive, UTF-8 and UTF-16LE forms).
12. Commit in three chunks: `feat(storage): event repository and queries`, `feat(storage): batching ingest pipeline`, `feat(storage): retention and search`.

## Todo

- [ ] `insert_batch` is the only insert path and takes `&[SealedEvent]`
- [ ] Keyset pagination returns stable pages with no duplicates/holes across inserts
- [ ] Retention respects all three horizons and yields under ingest pressure
- [ ] Search strategy probed and recorded; one API regardless of branch
- [ ] Perf table for 10k/100k/1M written into `docs/perf-baseline.md`
- [ ] DB/WAL/log byte scan finds no secret specimen

## Success Criteria

- [ ] `WC_PERF_EVENTS=1000000 cargo test --release --test perf-events -- --ignored --nocapture` completes and every printed number is at or below the target (or the miss is recorded with a cause)
- [ ] `cargo test --test repository --test retention --test ingest-flood` green
- [ ] 1M-row DB file size recorded; retention brings a 30-day-expired DB down without blocking a query
- [ ] `cargo test --test redaction-leak` green including the file-byte scan

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test repository --test retention --test ingest-flood
WC_PERF_EVENTS=10000 cargo test --release --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test perf-events -- --ignored --nocapture
WC_PERF_EVENTS=1000000 cargo test --release --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test perf-events -- --ignored --nocapture
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test redaction-leak
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| 1M-event page query misses 25 ms | Perf table shows p95 >25 ms | Add the covering index the query plan asks for (`EXPLAIN QUERY PLAN` in the test output), then re-measure; if still short, record the real number as the baseline and note it in `docs/perf-baseline.md` rather than hiding it |
| FTS5 absent in the bundled build | Probe returns 0 | Prefix-index path (already specified); search target relaxes to <250 ms at 1M and is recorded |
| Writer starves under a flood, WAL balloons | `.db-wal` > 200 MB during the flood test | Force a `wal_checkpoint(PASSIVE)` every 64 batches; flood test asserts WAL stays bounded |
| Retention deletes an event the UI is paging through | Page returns fewer rows than requested mid-scroll | Keyset pagination tolerates missing rows by design; the UI treats a short page as "continue from cursor", asserted in a test |
| Synthetic events pollute real history | A user sees simulated events as real | `metadata.synthetic = true` + `source_collector = "simulator"` are persisted and rendered as a "Simulated" badge (Phase 09); a test asserts real collectors never set the flag |

## Security Considerations

Every command validates bounds before touching SQL; no frontend value reaches a query as anything but a bound
parameter. The overflow counter prevents a flood from becoming an OOM (§82). The DB-byte scan added here is the
strongest available proof that the §34 chokepoint holds end-to-end.

### FILE: phase-09-timeline-experience-live-events.md

---
phase: 09
title: "Timeline Experience & Live Events"
status: pending
priority: P1
effort: 9h
dependencies: [08]
---

## Overview

The first screen a user will actually live in. Synthetic data (Phase 08) makes the whole timeline — ranges,
filters, clustering, virtualization, details drawer, live append — buildable and testable before a single real
collector exists, which is exactly why §69 puts it in user-phase 1.

## Requirements

**Functional**
- §12 ranges: Last hour, Today, Yesterday, Last 7 days, Last 30 days, Custom range. Selection persists in settings.
- §12 filters (all nine): category, importance, entity, source collector, text search, has-relationships, synthetic-only, time-of-day bucket, changed-value-present. Filters compose; active filters render as removable chips with a "Clear all".
- §12 clustering: ≥5 events sharing a category within a 60 s window collapse to one row reading exactly `12 Docker-related changes [Expand]` (count + category label + `[Expand]`), expanding in place without a refetch.
- §46 details drawer: summary, exact timestamp (absolute + relative), category/importance badges, before → after value block (redacted values render as `••••••` with a "redacted" tooltip), affected entities as chips that pivot the filter, related events list, source collector, and a "Copy details" action producing redacted text.
- Live append: `system-event-added` payloads are buffered in the frontend and flushed on a 250 ms animation frame tick; if the user is scrolled away from the top, a `N new changes` pill appears instead of shifting the viewport (§41 never move content under the cursor).
- §85 dev simulation panel (dev builds only, behind `#[cfg(debug_assertions)]` on the command and `import.meta.env.DEV` on the UI): buttons to inject each named scenario, flood 5000 events, and clear synthetic events only.
- Simulated events show a "Simulated" badge sourced from the persisted flag.
- Empty state (§39): the exact §72 first-run copy path is Phase 22's onboarding; here the empty timeline reads "No changes recorded in this range." with a range-widening action.

**Non-functional**
- 60 fps scroll at 10k rendered rows; virtualization keeps DOM nodes <120.
- Keyboard: `↑/↓` move row focus, `Enter` opens the drawer, `Esc` closes, `/` focuses search, `Tab` order follows visual order.
- No layout shift on live append (§41).

## Architecture

`src/lib/state/timeline.svelte.ts` holds a rune store: `filters`, `range`, `pages`, `cursor`, `pending` (live
buffer), `selected`. It is the only module that calls the four event commands. Components are dumb:
`TimelineView.svelte` → `TimelineList.svelte` → `TimelineRow.svelte` / `ClusterRow.svelte`, plus
`EventDetailsDrawer.svelte`, `FilterBar.svelte`, `RangePicker.svelte`. Virtualization via
`@tanstack/svelte-virtual` 3.13.12; the store exposes a plain windowing fallback (`start`/`end` from scrollTop ÷
row height) selected by one flag so a virtualizer API mismatch cannot block the phase.

## Related Code Files

**Create:** `src/lib/state/timeline.svelte.ts`, `src/lib/components/timeline/{TimelineView,TimelineList,TimelineRow,ClusterRow,FilterBar,RangePicker,EventDetailsDrawer,LiveEventPill,SimulatedBadge}.svelte`,
`src/lib/components/dev/SimulationPanel.svelte`, `src/lib/utils/{cluster.ts,relative-time.ts,virtual-window.ts}`,
`src-tauri/src/commands/dev.rs`, `tests/e2e/timeline.spec.ts`, `src/lib/utils/cluster.test.ts`
**Modify:** `src/routes.ts`, `src/lib/state/app.svelte.ts`, `src-tauri/src/app/events.rs` (batched emit),
`docs/IPC-SURFACE.md`, `STUBS.md`
**Delete:** the Phase 06 timeline placeholder screen body.

## Implementation Steps

1. Add the batched emitter in `app/events.rs`: collect ids from `insert_batch`, emit one `system-event-added` per batch (payload = array), never per event.
2. Build `timeline.svelte.ts`: `load()` (first page), `loadMore()` (cursor), `applyFilters()` (reset + reload), `ingestLive(batch)` (prepend if `atTop`, else buffer + count), `select(id)` → `get_event_details`.
3. `cluster.ts`: pure function `cluster(events, windowMs=60000, min=5)` → `(Event | Cluster)[]`; unit-tested on boundary cases (exactly 4 → no cluster, exactly 5 → cluster, mixed categories, events spanning the window edge).
4. `TimelineRow.svelte`: importance stripe using the §35 semantic tokens, category icon, summary, relative time with an absolute `title`, relationship-count affordance.
5. `ClusterRow.svelte`: renders the exact string `{count} {categoryLabel}-related changes` plus an `[Expand]` control; expanding swaps to the member rows in place.
6. `FilterBar.svelte` + `RangePicker.svelte`: nine filters, chips, `Clear all`, persisted selection.
7. `EventDetailsDrawer.svelte`: §46 sections; redacted values as `••••••`; entity chips dispatch a filter pivot; "Copy details" builds text from the already-redacted payload only.
8. Virtualization: wire `@tanstack/svelte-virtual`; if its Svelte 5 runes integration does not bind cleanly, flip to `virtual-window.ts` and record the reason in `docs/decisions/0007-virtualization.md`.
9. `SimulationPanel.svelte` + `commands/dev.rs` (`dev_inject_scenario`, `dev_flood_events`, `dev_clear_synthetic`), all `#[cfg(debug_assertions)]` so they do not exist in the release binary — asserted by a release-build symbol check in Phase 24.
10. Playwright `timeline.spec.ts` against `bun run dev` (web-only; Tauri commands stubbed by a `window.__WC_MOCK__` injection the store honours in dev): filter composition, cluster expand, drawer open/close, keyboard path, live-append-with-pill, no-scroll-jump assertion via `scrollTop` before/after.
11. Commit: `feat(ui): timeline with filters, clustering and details drawer`, `feat(ui): live event append`, `feat(dev): simulation panel`.

## Todo

- [ ] All six ranges and all nine filters functional and composable
- [ ] Cluster row text matches the §12 wording exactly
- [ ] Drawer shows all §46 sections; redacted values never show plaintext
- [ ] Live append never shifts the viewport when scrolled away from top
- [ ] Dev panel absent from a release build
- [ ] Playwright timeline spec green

## Success Criteria

- [ ] With 100k synthetic events loaded, scrolling is smooth and DOM node count stays <120 (asserted in the spec)
- [ ] `bun run gate` green (tsc strict, svelte-check, lint, vitest, playwright)
- [ ] A flood of 5000 injected events keeps the UI responsive and the pill count accurate
- [ ] Keyboard-only operation reaches every timeline control

## Verification Gate

```bash
cd "D:/Project/.WhatChage" && bun run check && bun run test:unit
cd "D:/Project/.WhatChage" && bun run test:e2e -- tests/e2e/timeline.spec.ts
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test architecture-gates
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `@tanstack/svelte-virtual` does not integrate with runes | Type error or blank list | Ship `virtual-window.ts` (already specified, ~60 lines) and drop the dependency per §80; record in `docs/decisions/0007-virtualization.md` |
| Live emit floods the webview | UI jank during flood test | Batched emit (one event per DB batch) + 250 ms frontend flush is the design; if still janky, cap the pill at 999+ and stop buffering payloads, refetching page 1 on the user's click |
| Clustering hides an important event | An `important`/`critical` event inside a cluster | Never cluster `important` or `critical` events — they always render as their own row; asserted in `cluster.test.ts` |
| Playwright cannot reach Tauri commands | `invoke` undefined in browser | `window.__WC_MOCK__` fixture path is the design; Tauri-runtime behaviour is covered by the Rust command tests instead, and this split is documented in `docs/TESTING.md` |
| Drawer leaks a raw value | Plaintext secret visible in the UI | Values arrive already sealed; a spec asserts a seeded secret event renders `••••••` and that `document.body.innerText` contains no specimen |

## Security Considerations

The frontend never receives an unredacted value, so there is nothing to leak client-side; the drawer's "Copy
details" therefore cannot exfiltrate a secret. Dev commands are compile-time excluded from release, so the
simulation surface is not an attack surface in shipped builds.

### FILE: phase-10-collector-runtime-supervisor.md

---
phase: 10
title: "Collector Runtime & Supervisor"
status: pending
priority: P1
effort: 6h
dependencies: [09]
---

## Overview

One place that owns every collector's lifetime, so a single misbehaving Win32 call can never take the app down
and every collector reports the same status shape. This phase also runs the notification-API spike battery that
decides, once, whether the later collectors are event-driven or polled — a decision that would otherwise be
re-litigated in five phases.

## Requirements

**Functional**
- `Collector` trait exactly as §5 specifies: `fn name(&self) -> &'static str`, `fn category(&self) -> EventCategory`, `async fn initialize(&mut self, ctx: &CollectorContext) -> Result<()>`, `async fn poll(&mut self, ctx: &CollectorContext) -> Result<Vec<RawEvent>>`, `fn interval(&self) -> Duration`, `async fn shutdown(&mut self) -> Result<()>`. Collectors emit `RawEvent`; only the supervisor seals (§34).
- `CollectorContext` gives read access to settings, the entity/relationship repositories (for previous-state lookups), a cancellation token, and a `tracing` span — never the raw `EventSink`.
- Supervisor: one Tokio task per collector; `poll` wrapped so a `Result::Err` increments `consecutive_errors` and backs off (1×, 2×, 4×, 8×, capped at 16× the interval), and a **panic** is captured via `JoinError::is_panic()` and turns into `CollectorState::Failed { reason }` — the app keeps running (§38 graceful degradation).
- Per-collector status persisted in `collector_status`: `enabled`, `state` (Idle/Running/Backoff/Failed/Disabled/Unavailable), `last_run_at`, `last_duration_ms`, `events_last_run`, `consecutive_errors`, `last_error` (redacted), `total_events`.
- Commands: `get_collector_status()` → `Vec<CollectorStatusDto>`, `set_collector_enabled(name, bool)` (takes effect without restart: disable stops the task and marks Disabled; enable spawns and re-initializes).
- Poll-time budget: if a `poll` exceeds 5× its interval it is logged once and the collector moves to Backoff; if `initialize` returns `Unavailable` (e.g. Docker pipe absent) the collector is marked `Unavailable` and never retried more than once per 5 minutes — no error spam (§21).
- Global switches honoured here so no collector re-implements them: master "recording paused", per-category enable, and a `snapshot_only` startup mode.

**Spike battery (decides research §5 items #2–#5), each a small `#[ignore]`d integration test that must print a verdict:**
1. `RegNotifyChangeKeyValue` on `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall` from a non-elevated process — does it signal on a subkey write?
2. `NotifyServiceStatusChangeW` — does a non-elevated caller receive SCM notifications?
3. `CM_Register_Notification` for device arrival/removal without a window handle.
4. `NotifyIpInterfaceChange` / `NotifyRouteChange2` callback delivery.
5. Optional: ETW for process start/stop without admin (research §5 #1). If it fails, the §5-documented diff-poll blind spot stands and nothing else changes.
Each verdict is written into `docs/decisions/0005-notification-apis.md` as PASS/FAIL + the measured latency, and every later collector reads its cadence from that file.

**Non-functional**
- Idle cost of the supervisor itself (no collectors registered) must be unmeasurable: no busy loops, all waits on timers or notification handles.
- No `unwrap()`/`expect()` in supervisor paths; `catch_unwind` boundaries documented.

## Architecture

```
Supervisor ──spawn──> [task: Collector A] ──RawEvent──┐
           ──spawn──> [task: Collector B] ────────────┼──> Redactor::seal ──> EventSink ──> batcher (Phase 08)
           ──spawn──> [task: Collector C] ────────────┘
                 ▲
                 └── status table + cancellation token + settings snapshot
```

Collector tasks never see `SealedEvent` or the DB writer. The sealing hop lives in the supervisor's collect loop,
which is the single place a reviewer must read to trust §34.

## Related Code Files

**Create:** `src-tauri/src/collectors/{mod.rs,traits.rs,supervisor.rs,context.rs,status.rs}`,
`src-tauri/src/commands/collectors.rs`, `src-tauri/tests/{supervisor.rs,spike-notifications.rs}`,
`docs/decisions/0005-notification-apis.md`, `docs/COLLECTORS.md`
**Modify:** `src-tauri/src/app/mod.rs` (start/stop supervisor with the app lifecycle), `src-tauri/src/lib.rs`,
`docs/IPC-SURFACE.md`, `ARCHITECTURE.md`, `src-tauri/migrations/0004_collector_status.sql` (if not already in 0001)
**Delete:** none.

## Implementation Steps

1. Write `traits.rs` with the §5 trait verbatim plus `CollectorHealth` and a `CollectorError` enum (`Unavailable`, `PermissionDenied`, `Transient`, `Fatal`) so the supervisor can react by class rather than by string.
2. `supervisor.rs`: registry `Vec<Box<dyn Collector>>`, spawn loop, backoff state machine, panic capture, cancellation-aware `select!` on interval vs. shutdown, status writes debounced to once per run.
3. `context.rs`: read-only settings snapshot (refreshed on change via a watch channel), repository handles, span factory.
4. `status.rs` + `commands/collectors.rs`: status DTOs and the two commands; `set_collector_enabled` validates the name against the registry (no arbitrary strings).
5. Register a `HeartbeatCollector` (interval 60 s, emits nothing, only touches its status row) purely so the supervisor is testable and observable before real collectors land — it stays in the shipped app as the Diagnostics liveness proof.
6. `tests/supervisor.rs`: a collector that always errors reaches Backoff with the expected schedule; a collector that panics is marked Failed while a sibling keeps polling; disable stops polling within one interval; enable re-initializes.
7. `tests/spike-notifications.rs`: the five probes above, each `#[ignore]`d, each printing `SPIKE <n> <name>: PASS|FAIL <detail>`.
8. Run the battery, then write `docs/decisions/0005-notification-apis.md` with a table: API, verdict, latency, chosen cadence, affected phase.
9. Write `docs/COLLECTORS.md`: the collector inventory table with columns *collector, mechanism, cadence, admin required (from the measured matrix), known blind spots, degradation behaviour*. The §5 B3 process blind spot is recorded here.
10. Commit: `feat(collectors): supervisor runtime and status surface`, `docs: notification API spike verdicts`.

## Todo

- [ ] §5 trait implemented verbatim; sealing happens only in the supervisor
- [ ] Panic in one collector cannot stop another or the app
- [ ] Backoff schedule and Unavailable throttle implemented and tested
- [ ] Five spikes executed with verdicts recorded
- [ ] `COLLECTORS.md` created with the blind-spot column populated

## Success Criteria

- [ ] `cargo test --test supervisor` green, including the panic-isolation test
- [ ] `docs/decisions/0005-notification-apis.md` contains a PASS/FAIL verdict for all five spikes, with a cadence chosen for each affected collector
- [ ] `get_collector_status` returns the heartbeat collector with a `last_run_at` that advances
- [ ] Idle CPU with only the heartbeat collector registered is indistinguishable from the Phase 05 baseline

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test supervisor
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test spike-notifications -- --ignored --nocapture
cargo clippy --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --all-targets -- -D warnings
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| All four notification APIs FAIL non-elevated | Spike output all FAIL | Every affected collector falls back to its measured-cost poll cadence (registry 60 s, SCM 5 s diff, devices hourly, network 30 s) — all four costs are already measured, so the fallback is costed, not guessed |
| A spike hangs (callback never fires, no timeout) | Test does not return | Every spike has a hard 10 s `tokio::time::timeout`; a timeout is recorded as FAIL with "no callback within 10s" |
| Panic capture misses a panic in a blocking Win32 call | App exits during a later phase | Blocking Win32 work runs inside `spawn_blocking`, whose `JoinError` also reports panics; a test panics inside `spawn_blocking` to prove it |
| Status writes thrash the DB | High write volume with no events | Status is written once per run per collector and only when a field changed; the heartbeat test asserts ≤1 status write per interval |
| Enable/disable races the poll loop | Duplicate tasks or a zombie task | Each collector's task is tracked by `JoinHandle`; enable is a no-op if a handle exists, disable awaits the handle with a 2 s timeout then aborts; asserted in a test |

## Security Considerations

`set_collector_enabled` accepts only registry-known names (no reflection into arbitrary code paths).
`last_error` is stored through the redactor because Win32 error strings can contain file paths and user names
(§33). The supervisor never gains write access to the system — every collector is read-only by construction, and
`COLLECTORS.md` states that explicitly so §57's "read-only" promise is auditable.

### FILE: phase-11-process-network-collectors.md

---
phase: 11
title: "Process & Network Listener Collectors"
status: pending
priority: P1
effort: 8h
dependencies: [10]
---

## Overview

The two collectors the product's headline story depends on: something started listening on a port, or a process
appeared/vanished. Both are measured-PASS non-elevated (probes #1–#5), both have a known partial-denial mode, and
their join produces the `process → listens_on → port` relationship that Phase 17 scores.

## Requirements

**Functional — process collector**
- Enumerate via ToolHelp (`CreateToolhelp32Snapshot` + `Process32FirstW/NextW`), resolve the image path with `QueryFullProcessImageNameW` on a `PROCESS_QUERY_LIMITED_INFORMATION` handle.
- Cadence 1 s (measured 6.2 ms warm; 0.6% of one core), diff against the previous snapshot keyed by `(pid, start_time)` so PID reuse cannot fake continuity.
- Path denial is expected for ~46% of processes (probe #3, measured): fall back to the ToolHelp `szExeFile` name, set `metadata.path_resolved = false`, and never log an error per process (one aggregate count per run).
- Emit `ProcessStarted` / `ProcessStopped`; importance is `informational` by default and `notable` when the process is a known long-lived server shape (listens on a port within 10 s, per the join below) or when it is a service host started outside SCM.
- §5 B3 blind spot: processes that start and exit inside one interval are missed. Documented in `COLLECTORS.md`, partially recovered by the Phase 14 Event Log 1000 (crash) correlation.
- Exclusions: the app's own process and its `msedgewebview2` children are never reported as user-visible changes (they are still counted in Diagnostics).

**Functional — network listener collector**
- `GetExtendedTcpTable(TCP_TABLE_OWNER_PID_ALL)` (measured 0.31 ms, 47 rows) + `GetExtendedUdpTable(UDP_TABLE_OWNER_PID)`, IPv4 and IPv6.
- Cadence 2 s; diff on `(protocol, local_addr, local_port, pid)`.
- Emit `PortOpened` / `PortClosed` with the §15 exact microcopy: summary `Port 3000 is now in use by node.exe`, and on close `Port 3000 was released by node.exe`. When the owning image path is denied, the name-only form is used and `metadata.path_resolved = false`.
- Create/refresh entities `process:<path>` and `port:tcp/3000`, plus the relationship `process → listens_on → port` (confidence Confirmed — it comes from the kernel table, not inference).
- Loopback vs. all-interfaces distinction is carried in metadata (`0.0.0.0` / `::` → `metadata.exposure = "all-interfaces"`), because §82 cares about it and §17's port-conflict story needs it.
- Port-conflict signal: if a `PortOpened` is preceded within 30 s by a `PortClosed` for the same port from a different process, mark both `notable` and relate them `conflicts_with`.

**Non-functional**
- Combined CPU of both collectors ≤1% of one core at their cadences (derived from measured 6.2 ms + 0.31 ms); verified by sampling in Phase 24.
- Zero allocations per unchanged process beyond the snapshot vector (reuse buffers across runs).

## Architecture

`collectors/process.rs` and `collectors/network_listeners.rs` share `platform/win/process_snapshot.rs` (safe
wrapper: handle RAII, wide-string decoding, error classification) and `platform/win/tcp_table.rs`. All `unsafe`
lives in `platform/win/*` behind safe signatures with `// SAFETY:` comments; collectors contain no `unsafe`.
The join lives in `collectors/join_process_port.rs`, a pure function over the two diffs, so it is unit-testable
without Win32.

## Related Code Files

**Create:** `src-tauri/src/platform/win/{process_snapshot.rs,tcp_table.rs,wide.rs,handle.rs}`,
`src-tauri/src/collectors/{process.rs,network_listeners.rs,join_process_port.rs}`,
`src-tauri/tests/{collector-process.rs,collector-ports.rs,join-process-port.rs}`,
`scripts/verify-phase-11.ps1`
**Modify:** `src-tauri/src/collectors/mod.rs` (register both), `src-tauri/Cargo.toml` (add the `windows` features
`Win32_System_Diagnostics_ToolHelp`, `Win32_System_Threading`, `Win32_NetworkManagement_IpHelper`,
`Win32_Networking_WinSock`), `docs/COLLECTORS.md`, `src-tauri/src/platform/mod.rs`
**Delete:** none.

## Implementation Steps

1. `platform/win/handle.rs`: an `OwnedHandle` newtype closing on drop; `wide.rs`: `PCWSTR`→`String` with lossy fallback and NUL trimming.
2. `platform/win/process_snapshot.rs`: `snapshot() -> Result<Vec<ProcessInfo>>` where `ProcessInfo { pid, parent_pid, name, path: Option<String>, start_time_ms: Option<u64>, session_id }`. Path denial returns `None`, not an error. Include a `denied_count` in the return so the collector can log one aggregate line.
3. `platform/win/tcp_table.rs`: `tcp_listeners()` + `udp_bindings()` returning `Vec<Endpoint { protocol, addr, port, pid, state }>`; the two-call size-probe pattern (`GetExtendedTcpTable` with a null buffer, then the sized allocation) with a bounded retry (3 attempts) for the race where the table grows between calls.
4. `collectors/process.rs`: previous-snapshot map, diff → `RawEvent`s, buffer reuse, aggregate denial logging, self-exclusion.
5. `collectors/network_listeners.rs`: diff, exact §15 summaries, entity + relationship emission, exposure metadata, conflict detection window.
6. `join_process_port.rs`: pure `fn correlate(process_diff, port_diff, now) -> Vec<Relationship>` plus the "server-shape" importance bump; unit tests cover started-then-listened, listened-then-started (out-of-order arrival), and the PID-reuse case.
7. `scripts/verify-phase-11.ps1`: starts a `System.Net.Sockets.TcpListener` on 3000, waits, stops it, starts another on 4000 from a differently-named host process, and prints the timestamps — the operator (or the agent) compares them against the app's timeline. Script is idempotent and releases the listeners in a `finally`.
8. `tests/collector-ports.rs`: an in-test `TcpListener` binds an ephemeral port; assert the collector's next poll reports that exact port with the test process's own pid and that the summary string matches the §15 template character-for-character.
9. `tests/collector-process.rs`: spawn `cmd /c ping -n 3 127.0.0.1`, assert `ProcessStarted` then `ProcessStopped` with matching `(pid, start_time)`; assert the collector reports zero errors when some paths are denied.
10. Update `docs/COLLECTORS.md` rows for both collectors with mechanism, measured cost, admin=No (probes #1/#3/#4/#5), and blind spots.
11. Commit: `feat(platform): win32 process and tcp table wrappers`, `feat(collectors): process collector`, `feat(collectors): network listener collector`.

## Todo

- [ ] All `unsafe` confined to `platform/win/*` with SAFETY comments
- [ ] Path denial degrades silently with an aggregate count, never per-process errors
- [ ] §15 port microcopy matches exactly, asserted by test
- [ ] `process → listens_on → port` relationship written with Confirmed confidence
- [ ] `(pid, start_time)` keying prevents PID-reuse false continuity
- [ ] Self/webview processes excluded from user-visible events

## Success Criteria

- [ ] `cargo test --test collector-process --test collector-ports --test join-process-port` green
- [ ] Running `scripts/verify-phase-11.ps1` produces timeline entries for ports 3000 and 4000 within 3 s of the script's printed timestamps
- [ ] Sampling both collectors for 60 s shows CPU ≤1% of one core (recorded in `docs/perf-baseline.md`)
- [ ] `cargo clippy -- -D warnings` clean, including `unsafe_op_in_unsafe_fn`

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test collector-process --test collector-ports --test join-process-port
cargo clippy --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File "D:/Project/.WhatChage/scripts/verify-phase-11.ps1"
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `GetExtendedTcpTable` size race loops | Repeated `ERROR_INSUFFICIENT_BUFFER` | Bounded 3-attempt retry with a 1.5× overshoot on the allocation; on exhaustion, skip this poll and count it (no error event) |
| 1 s process polling is more expensive on a busy machine than the 6.2 ms measurement | `last_duration_ms` >50 ms sustained | Adaptive cadence: if the trailing average exceeds 25 ms, back the interval off to 2 s and record it in Diagnostics; the blind-spot note in `COLLECTORS.md` is updated to say the window widened |
| Path-denied processes make entity ids unstable | Same program appears as two entities | Entity id falls back to `process:name:<name>`; when a later poll resolves the path, the name-entity is merged and a `same_as` relationship is written; covered by a test |
| Chatty processes flood the timeline | Hundreds of `ProcessStarted` per minute | Phase 08's coalescing collapses repeats; additionally, `informational` process events are excluded from notifications entirely (§53) and are clustered by Phase 09 |
| IPv6 rows misparsed | Ports reported with wrong addresses | `tcp_table.rs` handles the v4 and v6 row structs separately with a test over both; a malformed row is skipped, not guessed |

## Security Considerations

Process paths and command-relevant names go through the redactor before persistence — a path can contain a user
name (§33 never-list). The collectors open handles with `PROCESS_QUERY_LIMITED_INFORMATION` only, never
`PROCESS_VM_READ`, so the app cannot read another process's memory even by accident; that limitation is stated in
`COLLECTORS.md` as part of the §57 read-only promise. Command lines are **not** collected (they routinely carry
tokens), which is recorded as a deliberate privacy decision rather than an omission.

### FILE: phase-12-file-config-collector.md

---
phase: 12
title: "File & Configuration Change Collector"
status: pending
priority: P1
effort: 7h
dependencies: [11]
---

## Overview

The collector that turns "someone edited a config" into a diff a human can read. It is also the highest-risk
privacy surface in the app — config files are where secrets live — so the structured differ is written to hand the
redactor a *key-shaped* view of the change instead of a blob.

## Requirements

**Functional**
- Watch a user-managed set of locations (`watched_locations` table: path, recursive, enabled, added_at, label). Defaults offered at onboarding (§50): the user profile's dev directories are **not** auto-added; the user picks. Nothing is watched until the user opts in (§57).
- `notify` 8.2.0 with `notify-debouncer-full` 0.7.0, 500 ms debounce, so a save-with-temp-file (write → rename) produces one event, not three.
- Interest filter before any read: extension allowlist (`.env`, `.ini`, `.conf`, `.cfg`, `.properties`, `.json`, `.toml`, `.yaml`, `.yml`, `.xml`, plus exact names `hosts`, `Dockerfile`, `docker-compose.yml`, `.npmrc`, `.gitconfig`), size cap **256 KB**, and an ignore list (`node_modules`, `.git/objects`, `target`, `dist`, `.venv`, `__pycache__`, anything under a `Temp` directory).
- Path validation before every read: canonicalize, refuse if the canonical path escapes a watched root, refuse reparse points/symlinks (`FILE_ATTRIBUTE_REPARSE_POINT`), refuse device paths (`\\.\`, `\\?\` prefixes after canonicalization), refuse if not a regular file. A refusal is counted, not reported as a user event.
- Structured diff by format:
  - `env` / `properties` / `ini` / `conf` / `cfg`: line-oriented `key = value` parser (comments and blank lines preserved as non-keys); diff = added / removed / changed keys.
  - `json`: parsed with `serde_json`, flattened to dotted paths (`a.b[0].c`), diff on the flattened map.
  - `toml`: parsed with `toml`, same flattening.
  - `yaml`: **in-house** indentation-based flattener (§80 — no new dependency) handling maps, sequences, scalars, quoted strings and `#` comments; documented limits: no anchors/aliases, no multi-document, no block scalars — a file using them falls back to line-diff mode with `metadata.diff_mode = "line"`.
  - `xml` and unknown allowlisted text: line-diff with a 200-changed-line cap, then `metadata.diff_truncated = true`.
- Emit `FileModified` / `FileCreated` / `FileDeleted` with the §13 event shape: summary `Configuration changed in .env`, `changed_key`, `previous_value`, `new_value` (both sealed), `metadata.diff_mode`, and one event per changed key when ≤10 keys changed, otherwise one summary event listing the key names only.
- Previous content: a per-file **key/value fingerprint cache** (`file_states` table: path, hash of the file, flattened key→value-hash map, last_seen) — the app never stores file contents, so a diff is computed from hashes plus the current read, and a first sighting yields "now tracked" with no values.
- Entities: `file:<canonical lowercase path>`; relationship `file → configures → <process|service|container>` when the filename maps to a known consumer (`docker-compose.yml` → containers, `.npmrc` → node processes) — confidence Correlated, never Confirmed.

**Non-functional**
- Watching 5 roots with ~20k files costs no measurable idle CPU (notify uses `ReadDirectoryChangesW`; verify with the idle sample).
- A 256 KB file diff completes in <20 ms.

## Architecture

`collectors/file_config.rs` owns the watcher and the debouncer; `diff/` holds the pure format modules
(`env.rs`, `json_flat.rs`, `toml_flat.rs`, `yaml_flat.rs`, `line.rs`, `mod.rs` dispatch). Everything in `diff/` is
pure `&str -> BTreeMap<String, String>` (or line vectors), so the entire differ is unit-tested with no filesystem.
`storage/file_states.rs` holds the fingerprint cache. Path validation is a single function,
`platform/win/safe_path.rs::validate_watched_read()`, called by exactly one call site.

## Related Code Files

**Create:** `src-tauri/src/collectors/file_config.rs`, `src-tauri/src/diff/{mod.rs,env.rs,json_flat.rs,toml_flat.rs,yaml_flat.rs,line.rs}`,
`src-tauri/src/platform/win/safe_path.rs`, `src-tauri/src/storage/file_states.rs`,
`src-tauri/src/commands/watched_locations.rs`, `src-tauri/migrations/0005_file_states.sql`,
`src-tauri/tests/{diff-formats.rs,collector-file.rs,path-validation.rs}`,
`src-tauri/fixtures/configs/` (paired before/after files per format, including a YAML file using anchors to prove the fallback)
**Modify:** `src-tauri/Cargo.toml` (`notify`, `notify-debouncer-full`, `toml`), `src-tauri/src/collectors/mod.rs`,
`docs/COLLECTORS.md`, `docs/IPC-SURFACE.md`, `src/lib/components/settings/WatchedLocations.svelte` (stub wired in Phase 22)
**Delete:** none.

## Implementation Steps

1. `diff/env.rs` first (the `.env` case is the product's canonical demo), then `json_flat.rs`, `toml_flat.rs`, `line.rs`, and finally `yaml_flat.rs` with its documented limits and fallback signal.
2. `diff/mod.rs`: `fn diff(path: &Path, before: &FileFingerprint, after: &str) -> DiffResult` returning `Vec<KeyChange>` + `mode` + `truncated`; format chosen by extension then by content sniff (a `.conf` that parses as JSON is treated as JSON).
3. `platform/win/safe_path.rs`: `validate_watched_read(candidate, roots) -> Result<PathBuf, PathRefusal>`; unit tests cover `..` traversal, a junction pointing outside a root, a symlink to `C:\Windows\System32\config`, a device path, a directory, and a 300 KB file.
4. `storage/file_states.rs`: fingerprint upsert; key-value hashes are `blake3`-free — use SQLite's own `hex(randomblob)`-independent approach: a small in-repo FNV-1a/xxhash-style 64-bit hash in `util/hash.rs` (no dependency, §80) with a test proving stability across runs.
5. `collectors/file_config.rs`: build the watcher from enabled `watched_locations`, rebuild on change (watch channel from settings), debounce, interest filter, validate, read, diff, emit; count refusals and skips.
6. `commands/watched_locations.rs`: `list_watched_locations`, `add_watched_location(path, recursive)` (validated: must exist, must be a directory, must not be a system root like `C:\Windows` or the drive root, ≤10 roots), `remove_watched_location(id)`, `set_watched_location_enabled`.
7. `tests/diff-formats.rs`: fixture-driven — for each pair, assert the exact `KeyChange` set; assert the anchors YAML falls back to line mode.
8. `tests/collector-file.rs`: create a temp root, add it, write a `.env` with `PASSWORD=hunter2`, modify it, and assert (a) one event, (b) `changed_key = "PASSWORD"`, (c) the persisted values are redacted, (d) the summary is exactly `Configuration changed in .env`.
9. `tests/path-validation.rs`: the refusal matrix from step 3 as explicit assertions.
10. Update `COLLECTORS.md`: mechanism `ReadDirectoryChangesW` via notify, admin=No (probe #21 PASS for user paths; note that watching a protected system path fails and is refused up front), blind spots (files changed while the app is closed are seen as one change on next start, with the previous fingerprint as the baseline).
11. Commit: `feat(diff): pure structured config differs`, `feat(security): watched-path validation`, `feat(collectors): file and configuration collector`.

## Todo

- [ ] Nothing is watched without explicit user opt-in
- [ ] All six diff modes implemented; YAML limits documented with a working fallback
- [ ] Path validation refuses traversal, reparse points, device paths, oversized and non-regular files
- [ ] File contents are never persisted — only hashes and sealed key/value pairs
- [ ] `PASSWORD=hunter2` round-trip proves redaction on the real collector path
- [ ] `add_watched_location` rejects system roots and enforces the 10-root cap

## Success Criteria

- [ ] `cargo test --test diff-formats --test collector-file --test path-validation` green
- [ ] Editing a fixture `.env` in a watched folder produces exactly one timeline event within 1 s of the debounce window, with the key named and the value masked
- [ ] Watching 5 roots adds no measurable idle CPU (recorded alongside the Phase 24 idle sample)
- [ ] A YAML file with anchors produces a line-mode diff and no panic

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test diff-formats --test collector-file --test path-validation
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test redaction-leak
cargo clippy --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --all-targets -- -D warnings
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| A config value leaks despite key-level redaction | `redaction-leak` DB scan finds a specimen | The differ hands the redactor `(key, value)` pairs, so both the §60 value shapes and the §33 key names apply; if a specimen still lands, the fix is a new matcher plus a fixture — never disabling the diff |
| notify misses changes on a network or virtual drive | Edits produce no events | Detect the drive type at `add_watched_location` (`GetDriveTypeW`); for non-fixed drives, add a 30 s polling fallback for that root and label it "polled" in Settings |
| Watcher handle exhaustion with recursive roots | notify returns an error on start | Cap at 10 roots (enforced in the command); on watcher error the collector reports `Unavailable` with the root named, and the UI shows a per-root error rather than failing the collector |
| Huge JSON flattening blows memory | Diff of a 256 KB deep JSON allocates excessively | 256 KB read cap plus a 5000-key flatten cap; exceeding it sets `diff_truncated` and lists key names only |
| A rename storm (build output inside a watched root) floods events | Hundreds of events from `dist/` | Ignore list covers build dirs; additionally the interest filter runs before any read, so ignored churn costs one path comparison |

## Security Considerations

This is the phase where §65 and §82 meet: file reads are bounded by an allowlist, a canonicalized root check, a
reparse-point refusal, and a size cap — enumerated as a refusal matrix with tests. The frontend can never name a
file to read; it can only add a *root* (validated) and the collector decides what to open, so there is no
arbitrary-file-read primitive exposed over IPC (an explicit disqualifier). Contents are never stored, which means
a stolen database cannot reconstruct a config file.

### FILE: phase-13-applications-services-collectors.md

---
phase: 13
title: "Application & Service Collectors"
status: pending
priority: P1
effort: 6h
dependencies: [12]
---

## Overview

Two registry/SCM collectors whose cadences are dictated by measurement, not preference: the uninstall hives are
cheap to read but change rarely, while `EnumServicesStatusExW` is 3.3 ms but `QueryServiceConfigW` across all
services is 392 ms — so service *config* is read only when a service's state changed or every 15 minutes, never on
the fast diff tick.

## Requirements

**Functional — application collector**
- Read all three uninstall hives (measured: `HKLM\...\Uninstall` 88 entries, `HKLM\...\WOW6432Node\...\Uninstall` 148, `HKCU\...\Uninstall` 38; probes #6/#7/#8 all PASS non-elevated).
- Dedupe across hives by `(DisplayName normalized, Publisher)`, keeping the richest record; a 32-bit and 64-bit entry for the same product is one application.
- Fields: DisplayName, DisplayVersion, Publisher, InstallDate, InstallLocation, UninstallString presence (never the string itself — it can contain tokens).
- Diff → `ApplicationInstalled` / `ApplicationRemoved` / `ApplicationUpdated` (version changed). Summaries: `Node.js was updated from 22.11.0 to 24.4.0`, `Docker Desktop was installed`, `Foo was removed`.
- Cadence: `RegNotifyChangeKeyValue` on all three keys if spike #1 PASSed (Phase 10), else a 60 s poll. Either way a full re-read is ~3 ms of work.
- Importance: `ApplicationUpdated` for a dev-relevant publisher/name set (node, python, docker, git, jdk, dotnet, go, rust, postgres, mysql, redis, nginx) is `important`; everything else `notable`; browser updates are explicitly `informational` (§53 — "Chrome updated" must never notify).
- Entities `app:<normalized name>`; relationship `app → provides → process:<exe>` when the InstallLocation is a prefix of an observed process path (Correlated).

**Functional — service collector**
- `EnumServicesStatusExW` over all services (measured 3.3 ms) every 5 s → diff on `(name, current_state, start_type_cached)`.
- `QueryServiceConfigW` (measured 392 ms for all) is called **per changed service only**, plus a full sweep every 15 minutes to catch start-type changes that produce no state change.
- If spike #2 (`NotifyServiceStatusChangeW`, non-elevated) PASSed, the 5 s diff becomes event-driven with the 15-minute config sweep retained; the fallback cadence is already costed.
- Emit `ServiceStarted` / `ServiceStopped` / `ServiceStartTypeChanged` / `ServiceInstalled` / `ServiceRemoved`.
- Restart coalescing: a Stopped followed by a Started for the same service within 30 s becomes one `ServiceRestarted` event whose summary is exactly `Windows Audio restarted.` (display name + " restarted."). This is a §16 requirement and a correctness test, not cosmetic.
- Importance: services whose start type changed → `important`; a stop with no restart inside 30 s → `notable`; a restart → `notable`; routine `informational` for known-transient services (e.g. `Windows Modules Installer` while updates run).
- Entities `service:<service name>` with the display name stored on the entity; relationship `service → runs_as → process:<exe>` from the service's ProcessId when available (Confirmed) and `service → depends_on → service` from the config's dependency list (Confirmed).

**Non-functional**
- Neither collector may add more than 0.1% average CPU: application ~3 ms/60 s, service 3.3 ms/5 s + 392 ms/15 min → both trivially inside budget; the 392 ms sweep runs on `spawn_blocking` so it cannot stall the runtime.

## Architecture

`platform/win/registry.rs` (safe `RegOpenKeyEx`/`RegEnumKeyEx`/`RegQueryValueEx` wrappers + a
`watch_key(hkey, path) -> notify handle` helper) and `platform/win/services.rs` (SCM enumeration, config read,
optional notification). Collectors `collectors/applications.rs`, `collectors/services.rs`. The dedupe and the
restart-coalescing are pure functions in `collectors/dedupe_apps.rs` and `collectors/coalesce_services.rs`,
unit-tested without Win32.

## Related Code Files

**Create:** `src-tauri/src/platform/win/{registry.rs,services.rs}`,
`src-tauri/src/collectors/{applications.rs,services.rs,dedupe_apps.rs,coalesce_services.rs}`,
`src-tauri/tests/{collector-apps.rs,collector-services.rs,coalesce-services.rs,dedupe-apps.rs}`
**Modify:** `src-tauri/Cargo.toml` (`windows` features `Win32_System_Registry`, `Win32_System_Services`),
`src-tauri/src/collectors/mod.rs`, `docs/COLLECTORS.md`, `docs/decisions/0005-notification-apis.md` (record which branch was taken)
**Delete:** none.

## Implementation Steps

1. `platform/win/registry.rs`: enumerate-subkeys and read-string-value helpers with `ERROR_FILE_NOT_FOUND` treated as "absent" not "error"; `REG_SZ`/`REG_EXPAND_SZ`/`REG_DWORD` handled, other types skipped with a count.
2. `collectors/applications.rs`: read three hives into `Vec<AppRecord>`, `dedupe_apps::dedupe()`, diff against the previous set held in memory *and* rehydrated from entities on startup (so a restart does not re-announce 274 apps as new — asserted by a test).
3. `platform/win/services.rs`: `enum_services()` (name, display name, state, process id), `query_config(name)` (start type, binary path presence, dependencies), and `subscribe(name?)` behind the spike verdict.
4. `collectors/services.rs`: 5 s diff; changed-only config reads; 15-minute full sweep on `spawn_blocking`; startup rehydration from entities.
5. `coalesce_services.rs`: pure `fn coalesce(events, window=30s) -> Vec<RawEvent>`; tests assert stop+start→`ServiceRestarted` with the exact `"<DisplayName> restarted."` summary, stop-alone stays a stop after the window, and three rapid restarts become one restart with `occurrences = 3`.
6. `tests/collector-apps.rs`: seed a fake application record under `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\WhatChangedTest` (HKCU is writable non-elevated — probe #8 read PASS, and the test writes only under its own test key and removes it in a guard), then assert install/update/remove events and the exact update summary.
7. `tests/collector-services.rs`: assert enumeration returns >50 services, that the collector produces no events on two consecutive unchanged polls, and that a synthetic diff (injected previous-state) yields the expected event set. Do **not** start/stop a real service (requires admin — probe #12 measured FAIL for control operations); state that in `COLLECTORS.md`.
8. Update `COLLECTORS.md` with both rows: mechanism, cadence, measured cost, admin=No for read, admin=Yes for control (which the app never does), blind spots (a service that stops and starts inside one 5 s tick is reported as a restart only if both edges are seen; otherwise nothing).
9. Commit: `feat(platform): registry and SCM wrappers`, `feat(collectors): application collector`, `feat(collectors): service collector`.

## Todo

- [ ] All three uninstall hives read and deduped
- [ ] Service config reads are change-driven plus a 15-minute sweep, never per tick
- [ ] `"Windows Audio restarted."` produced by the coalescer, asserted verbatim
- [ ] Startup rehydration prevents a re-announce storm after a restart
- [ ] Browser updates classified `informational` so they cannot notify
- [ ] 392 ms sweep runs on `spawn_blocking`

## Success Criteria

- [ ] `cargo test --test collector-apps --test collector-services --test coalesce-services --test dedupe-apps` green
- [ ] Restarting the app twice with no system changes produces zero application/service events
- [ ] Measured: application poll <10 ms, service diff <10 ms, config sweep <500 ms (recorded in `docs/perf-baseline.md`)
- [ ] The test-key application fixture produces the exact `was updated from X to Y` summary

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test collector-apps --test collector-services --test coalesce-services --test dedupe-apps
cargo clippy --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --all-targets -- -D warnings
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Restart floods the timeline with "installed" events | 274 app events after a restart | Startup rehydration from persisted entities is the design; a test restarts the collector against a populated DB and asserts zero events |
| `RegNotifyChangeKeyValue` fires continuously on a noisy hive | CPU rises, repeated identical reads | Re-arm with a 2 s minimum gap and diff before emitting; identical reads produce no events by construction |
| A service enumeration transiently fails during shutdown | `ERROR_SHUTDOWN_IN_PROGRESS` | Classified `Transient`; backoff handles it; no user-visible error |
| Publisher/name normalization merges two distinct apps | Two products appear as one, versions flap | Normalization is conservative (trim, case-fold, collapse whitespace, strip a trailing version); a flap detector logs when the same entity changes version more than 3× in an hour and demotes to `informational` pending investigation |
| The 15-minute config sweep coincides with the retention job | A 1 s stall | Both run on `spawn_blocking` with a shared "heavy work" semaphore of 1, so they serialize instead of contending |

## Security Considerations

`UninstallString` and service binary paths are deliberately **not** persisted (they can embed tokens and always
embed user paths); only their presence is recorded. All registry access is read-only — the only write in the whole
phase is inside a test, under a test-owned HKCU key, removed by a drop guard. Service *control* is never
implemented, so the app cannot be used to stop a security service (§57, §82).

### FILE: phase-14-updates-eventlog-collectors.md

---
phase: 14
title: "Windows Update & Event Log Collectors"
status: pending
priority: P1
effort: 7h
dependencies: [13]
---

## Overview

The "what did Windows do to me overnight" collectors. Both have a measured trap: `Win32_QuickFixEngineering`
returns **4** entries where WUA `QueryHistory` returns **133** (measured), and the `Security` channel is a measured
**FAIL** non-elevated while Application/System/Setup/WindowsUpdateClient all PASS. Both traps are encoded as
constraints here so a later implementer cannot reintroduce them.

## Requirements

**Functional — Windows Update collector**
- WUA COM: `IUpdateSession` → `CreateUpdateSearcher` → `GetTotalHistoryCount` + `QueryHistory(0, n)` (measured 133 entries in 38 ms, probe #9 PASS non-elevated).
- **Never** `Win32_QuickFixEngineering` / `wmic qfe` — it under-reports by ~97% on this machine. A comment in the code and a row in `COLLECTORS.md` state this with both numbers.
- Cadence: every 30 minutes, plus an immediate run at startup and on resume-from-sleep.
- Diff by `(UpdateIdentity or Title, Date, ResultCode)`; emit `WindowsUpdateInstalled` with `ResultCode` mapped (2 = Succeeded, 3 = SucceededWithErrors, 4 = Failed, 5 = Aborted). Failures are `important`, successes `notable`, and a security update (`Title` contains a KB and "Security") is `important`.
- Summary: `Windows installed update KB5041585 (Security Update)`; failed: `Windows update KB5041585 failed to install`.
- Entity `update:KB5041585`; relationship `update → may_have_changed → driver|service` is **not** invented here — Phase 17 derives temporal correlations instead (§29 honesty).

**Functional — Event Log collector**
- `EvtSubscribe` (pull mode) on exactly four channels: `Application`, `System`, `Setup`, `Microsoft-Windows-WindowsUpdateClient/Operational` (all measured PASS, probes #14–#18). `Security` is **excluded** — measured FAIL non-elevated (probe #19); Settings shows it greyed with "Requires administrator" (§54 admin column), never a silent failure.
- Event-ID allowlist per channel, each with a mapped meaning, e.g. System: 1074 (initiated shutdown/restart), 6005/6006 (log start/stop), 6008 (unexpected shutdown), 41 (kernel power), 7000/7001/7011 (service start failure/dependency/timeout), 7045 (new service installed); Application: 1000 (app crash), 1001 (fault bucket), 1002 (app hang); Setup: 1/2/4 (servicing operations); WindowsUpdateClient: 19/20/43/44. Anything not on the list is ignored — the app is not a log viewer (§88 scope).
- Rendering: `EvtRender(EvtRenderEventXml)` for structured fields plus `EvtFormatMessage` for the human string; if `EvtFormatMessage` fails (missing provider resources — common), fall back to the XML fields and set `metadata.message_rendered = false`. A malformed/oversized (>64 KB) record is skipped and counted.
- Bookmarks: persist an `EvtCreateBookmark` XML string per channel in `collector_state` so a restart resumes exactly where it stopped, with a first-run backfill limited to the last 24 hours (and a `?` count cap of 500 per channel per run to bound a flood).
- Crash recovery for §5 B3: an Application 1000 for an image the process collector saw start but never saw stop upgrades the missing `ProcessStopped` into a `ProcessCrashed` event with confidence Correlated and `metadata.recovered_from = "eventlog:1000"`.
- Reboot correlation: 1074/6006/6008/41 produce a `SystemRebooted` event carrying `metadata.clean = true|false`; Phase 17 treats a reboot as a chain boundary marker.

**Non-functional**
- Event Log polling every 15 s (pull-mode `EvtNext` with a 0 ms timeout) with a bounded batch; measured `EvtQuery` costs (probes #14–#18) put a full pull well under 50 ms.
- All COM work on a dedicated thread with `CoInitializeEx(COINIT_APARTMENTTHREADED)` per thread and matching `CoUninitialize`; COM is never initialized on a Tokio worker.

## Architecture

`platform/win/wua.rs` (COM lifetime + searcher + history mapping), `platform/win/eventlog.rs`
(`EvtSubscribe`/`EvtNext`/`EvtRender`/`EvtFormatMessage`/bookmarks, all handles RAII-wrapped),
`collectors/windows_update.rs`, `collectors/event_log.rs`, and `collectors/recover_process_exit.rs` (the pure
correlation from a 1000 record + open process set → `ProcessCrashed`). A `ComApartment` guard type makes the
init/uninit pairing a compile-visible scope.

## Related Code Files

**Create:** `src-tauri/src/platform/win/{wua.rs,eventlog.rs,com.rs}`,
`src-tauri/src/collectors/{windows_update.rs,event_log.rs,recover_process_exit.rs}`,
`src-tauri/src/storage/collector_state.rs`, `src-tauri/migrations/0006_collector_state.sql`,
`src-tauri/tests/{collector-updates.rs,collector-eventlog.rs,recover-process-exit.rs}`,
`docs/decisions/0008-wua-binding.md`
**Modify:** `src-tauri/Cargo.toml` (`windows` features `Win32_System_Com`, `Win32_System_EventLog` /
`Win32_System_Diagnostics_Etw` as required by the actual binding, plus `Win32_System_UpdateAgent` **if it exists in
0.62.2**), `src-tauri/src/collectors/mod.rs`, `docs/COLLECTORS.md`
**Delete:** none.

## Implementation Steps

1. `platform/win/com.rs`: `ComApartment::new()` → RAII guard; a `ComThread` helper that owns a thread, initializes COM once, and services requests over a channel (WUA and Task Scheduler in Phase 15 both use it).
2. WUA binding decision, executed and recorded in `docs/decisions/0008-wua-binding.md`: first attempt typed bindings from `windows` 0.62.2 feature `Win32_System_UpdateAgent`; if that feature/module is absent in this version, use late-bound COM via `CoCreateInstance` on the `Microsoft.Update.Session` CLSID with `IDispatch::Invoke`. **Do not assume either shape** — check `cargo doc`/`cargo tree` output first and write down which was true.
3. `platform/win/wua.rs`: `history(max) -> Vec<UpdateHistoryEntry>` with title, KB extraction (parse `KB\d{6,7}` from the title by hand — no `regex`, §80), date (epoch ms), result code, operation, and support URL presence.
4. `collectors/windows_update.rs`: 30-minute interval, startup + resume run, diff via `collector_state` (last seen max date + a set of seen identities capped at 500), event mapping and summaries.
5. `platform/win/eventlog.rs`: channel open with `EvtSubscribe` (`EvtSubscribeStartAfterBookmark` / `EvtSubscribeStartAtOldestRecord` for backfill), `EvtNext` batching (≤100), `EvtRender` XML into a small hand-written field extractor (no XML crate: pull `<EventID>`, `<TimeCreated SystemTime=>`, `<Provider Name=>`, `<Data Name=>` — bounded, tested), `EvtFormatMessage` with graceful failure, bookmark save.
6. `collectors/event_log.rs`: the allowlist table as a `const` array of `(channel, event_id, meaning, event_type, importance)`; per-channel bookmarks; 500/run cap; skip counters.
7. `recover_process_exit.rs`: pure function over (open-process set, 1000 records) → `ProcessCrashed` events; tests cover a match, a name-only match when the path was denied, and no match (no event invented).
8. `tests/collector-eventlog.rs`: query the real `Application` channel for the last 24 h and assert the collector returns only allowlisted IDs, that a synthetic malformed XML string is skipped without panic, and that a saved bookmark round-trips (no duplicate events on a second run).
9. `tests/collector-updates.rs`: assert ≥1 history entry is returned on this machine (measured 133), that KB parsing extracts the KB from a fixture title set, and that a second run with an unchanged history yields zero events.
10. Update `COLLECTORS.md`: the QFE-vs-WUA numbers, the `Security`-channel exclusion with its measured verdict, and the per-channel admin column.
11. Commit: `feat(platform): COM apartment and WUA history`, `feat(collectors): windows update collector`, `feat(platform): event log reader`, `feat(collectors): event log collector with crash recovery`.

## Todo

- [ ] `Win32_QuickFixEngineering` appears nowhere in the codebase (asserted by an architecture-gate grep)
- [ ] `Security` channel is not subscribed and is shown as admin-gated in Settings
- [ ] COM initialized only on its own thread with a matching uninit
- [ ] Bookmarks prevent duplicate events across restarts
- [ ] `EvtFormatMessage` failure degrades to XML fields, not to an error event
- [ ] Crash recovery never invents a stop event without a matching 1000 record

## Success Criteria

- [ ] `cargo test --test collector-updates --test collector-eventlog --test recover-process-exit` green
- [ ] WUA returns ≥100 history entries on this machine in <200 ms (measured 133 in 38 ms)
- [ ] Two consecutive runs with no new log records produce zero events
- [ ] `docs/decisions/0008-wua-binding.md` states which binding shape was actually found in `windows` 0.62.2
- [ ] The architecture-gate test fails the build if `QuickFixEngineering`, `wmic`, or `"Security"` as a channel literal appears in `src-tauri/src`

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test collector-updates --test collector-eventlog --test recover-process-exit
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test architecture-gates
cargo clippy --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --all-targets -- -D warnings
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `Win32_System_UpdateAgent` does not exist in `windows` 0.62.2 | Feature rejected by cargo | Late-bound `IDispatch` path on `Microsoft.Update.Session` (specified above), recorded in `docs/decisions/0008-wua-binding.md`. Neither shape is asserted in advance — this is exactly the "never invent an API" rule applied |
| WUA COM call blocks for seconds on a machine mid-update | `last_duration_ms` spikes | The whole WUA collector runs on the dedicated COM thread with a 20 s cap; a timeout marks the run Transient and retries at the next 30-minute tick |
| Event Log XML extraction mis-parses a record | Wrong event id or timestamp | The hand-written extractor is fixture-tested against real captured XML for every allowlisted ID; an extraction failure skips the record and increments a counter rather than guessing |
| A log flood (thousands of 1000 events) | 500/run cap hit repeatedly | Cap is per channel per run; the collector emits one aggregate `metadata.suppressed = N` note on the last event and Diagnostics surfaces the counter |
| `EvtSubscribe` unavailable/permission-denied for a channel at runtime | Open fails for one channel | Per-channel state: that channel is marked `Unavailable` with a reason while the other three keep working (§38); Settings shows the per-channel state |

## Security Considerations

Event Log messages routinely contain user names and paths, so every rendered message goes through the redactor
before persistence; the extractor keeps only allowlisted fields, which bounds what can be stored in the first
place. The `Security` channel is excluded both because it needs admin and because it is the channel most likely to
contain sensitive authentication detail — that dual rationale is written into `COLLECTORS.md` so a future
implementer with admin rights does not casually enable it.

### FILE: phase-15-drivers-network-startup-snapshots.md

---
phase: 15
title: "Driver, Network Config & Startup Collectors + Snapshots"
status: pending
priority: P1
effort: 9h
dependencies: [14]
---

## Overview

The three remaining §14/§18/§19 collectors, all of them cadence-constrained by measurement (driver enumeration is
**2128 ms warm** — an hourly sweep, never a timer), plus the `snapshots` table that gives every change a
before/after system context so a user can answer "what did my machine look like when it worked?".

## Requirements

**Functional — driver collector**
- SetupAPI: `SetupDiGetClassDevsW(NULL, NULL, NULL, DIGCF_ALLCLASSES | DIGCF_PRESENT)` + `SetupDiEnumDeviceInfo`, reading `DEVPKEY_Device_DriverVersion`, `DEVPKEY_Device_DriverDate`, `DEVPKEY_Device_DriverProvider`, `DEVPKEY_Device_FriendlyName`/`DEVPKEY_NAME`, `DEVPKEY_Device_Class`, and the device instance id (probe #10 PASS non-elevated, 2128 ms warm).
- Cadence: **hourly** sweep on `spawn_blocking`, plus an immediate sweep on device-arrival/removal if spike #3 (`CM_Register_Notification`) PASSed, plus one at startup. The 2128 ms cost is why a short timer is explicitly forbidden — stated in the code comment and `COLLECTORS.md`.
- Diff on `(instance_id, driver_version, driver_date)` → `DriverInstalled` / `DriverUpdated` / `DriverRemoved`. Summary: `Display driver updated from 32.0.101.5333 to 32.0.101.6314 (Intel Corporation)`.
- Importance: display/network/storage/audio class drivers → `important` (they break things); everything else `notable`.
- Entity `driver:<instance id>` with class + friendly name; relationship `driver → affects → adapter:<guid>` for network-class drivers (Confirmed via the adapter's device instance id).

**Functional — network configuration collector**
- `GetAdaptersAddresses` (measured 0.20 ms, 18 adapters — probe #11 PASS) every 30 s, or event-driven if spike #4 (`NotifyIpInterfaceChange`) PASSed; plus `GetIpForwardTable2` for the default-route/gateway view.
- Diff → `NetworkAdapterChanged` (up/down, media state), `IpAddressChanged`, `DnsServersChanged`, `DefaultGatewayChanged`, `ProxyConfigChanged` (from `HKCU\...\Internet Settings`: `ProxyEnable`, `ProxyServer`, `AutoConfigURL`).
- DHCP lease renewals that change nothing but the lease timestamp are demoted to `informational` and suppressed from the timeline unless the IP actually changed (measured DHCP data is available but noisy — research §5 flagged it; it is recorded as informational-only rather than dropped).
- Summaries: `DNS servers changed on Wi-Fi: 1.1.1.1 → 192.168.1.1`, `Proxy enabled: 127.0.0.1:8080`. Proxy values are treated as potentially sensitive (a proxy URL can embed credentials) and pass through the redactor's URL matcher.
- Entities `adapter:<guid>`; a change to DNS/gateway/proxy is `important` (classic "it worked before" cause).

**Functional — startup/scheduled-task collector**
- Run keys (`HKLM`/`HKCU` `\...\Run`, `\...\RunOnce`), `StartupApproved\Run` (enabled/disabled state), and both Startup folders (per-user + all-users), all read-only, 5-minute cadence (or registry notification where available).
- Task Scheduler 2.0 COM (`ITaskService::Connect` + folder walk; measured 194 tasks, probe #13 PASS non-elevated) on the shared COM thread, 15-minute cadence; read task name, path, enabled, triggers summary (types only), last run result. **Not** the action command line (token risk) — only the executable's file name.
- Emit `StartupItemAdded` / `StartupItemRemoved` / `StartupItemDisabled` / `ScheduledTaskChanged`. Startup additions are `important` (§18 rationale: this is how machines silently change behaviour at boot).
- Tasks under `\Microsoft\Windows\` that Windows itself churns are classified `informational` unless enabled-state changed.

**Functional — snapshots**
- `snapshots` table: `id`, `taken_at`, `reason` (`startup` | `pre_change` | `post_change` | `checkpoint` | `manual`), `payload_json` (sealed), `size_bytes`.
- Payload: counts + compact lists — running process names (deduped, no paths), listening ports with owner names, running services, adapter IP/DNS/gateway summary, app count + versions of the dev-relevant set, driver versions of the important classes, startup item names, and (Phase 21) container names/images. Nothing that is not already an entity.
- Cadence: one at startup, one every 6 hours, one on checkpoint creation (Phase 16), and a before/after pair around any `important`/`critical` event (debounced so a burst produces one pair).
- Retention: 7 days by default (§11), configurable, and always keeps the snapshots referenced by a checkpoint.
- Command `get_snapshot_diff(a, b)` returning grouped added/removed/changed lists — this is what powers Phase 16's "compare to checkpoint" and Phase 20's System Map.

**Non-functional**
- The hourly driver sweep must not be visible: run on `spawn_blocking` under the shared heavy-work semaphore (Phase 13), and assert in a test that a concurrent timeline page query still completes <100 ms during a sweep.
- A snapshot payload stays under 256 KB; if it would exceed that, lists are truncated with counts retained.

## Architecture

`platform/win/{setupapi.rs,adapters.rs,taskscheduler.rs}` + `collectors/{drivers.rs,network_config.rs,startup.rs}`
+ `storage/snapshots.rs` + `domain/snapshot.rs` (the payload struct, `specta`-exported so the diff view is typed).
`snapshot_diff.rs` is a pure function over two payloads — the entire compare feature is unit-testable from two
JSON fixtures.

## Related Code Files

**Create:** `src-tauri/src/platform/win/{setupapi.rs,adapters.rs,taskscheduler.rs}`,
`src-tauri/src/collectors/{drivers.rs,network_config.rs,startup.rs}`,
`src-tauri/src/{domain/snapshot.rs,storage/snapshots.rs,domain/snapshot_diff.rs,commands/snapshots.rs}`,
`src-tauri/migrations/0007_snapshots.sql`,
`src-tauri/tests/{collector-drivers.rs,collector-network.rs,collector-startup.rs,snapshot-diff.rs}`,
`src-tauri/fixtures/snapshots/{before.json,after.json}`
**Modify:** `src-tauri/Cargo.toml` (`windows` features `Win32_Devices_DeviceAndDriverInstallation`,
`Win32_Devices_Properties`, `Win32_NetworkManagement_IpHelper`, `Win32_System_TaskScheduler`),
`src-tauri/src/collectors/mod.rs`, `docs/COLLECTORS.md`, `docs/IPC-SURFACE.md`
**Delete:** none.

## Implementation Steps

1. `platform/win/setupapi.rs`: device-info-set RAII (`SetupDiDestroyDeviceInfoList` on drop), `SetupDiGetDevicePropertyW` wrapper handling `DEVPROP_TYPE_STRING`, `_FILETIME`, `_UINT32`; return `Vec<DeviceInfo>` plus a `skipped` count for devices whose properties are absent (normal).
2. `collectors/drivers.rs`: hourly sweep, startup sweep, optional CM notification trigger; previous state rehydrated from entities so a restart does not re-announce every driver.
3. `platform/win/adapters.rs`: `adapters()` via the two-call size pattern; extract friendly name, description, operational status, unicast IPs, DNS servers, gateways, DHCP enabled + server; `default_routes()` via `GetIpForwardTable2`.
4. `collectors/network_config.rs`: diff, proxy registry read, DHCP demotion rule, exact summaries.
5. `platform/win/taskscheduler.rs`: on the shared COM thread, walk folders recursively (depth cap 8, count cap 2000), collect name/path/enabled/state/last-result/trigger-type list; never read `Exec.Arguments`.
6. `collectors/startup.rs`: Run keys + StartupApproved decoding (the first byte of the binary value indicates enabled/disabled — decode defensively, unknown → `metadata.approval_unknown = true`) + Startup folder listing + task diff.
7. `domain/snapshot.rs` + `storage/snapshots.rs`: build, seal, store, retain; `commands/snapshots.rs`: `list_snapshots(range)`, `get_snapshot(id)`, `get_snapshot_diff(a,b)`, `create_manual_snapshot()`.
8. `domain/snapshot_diff.rs` + `tests/snapshot-diff.rs`: grouped diff over the two fixtures asserting exact added/removed/changed sets, including the "nothing changed" case producing an empty diff (not a diff of noise like timestamps).
9. `tests/collector-drivers.rs`: assert the sweep returns >50 devices, completes in <5 s, and that two consecutive sweeps produce zero events; assert a concurrent page query stays <100 ms during a sweep.
10. `tests/collector-startup.rs`: write a test value under `HKCU\...\Run\WhatChangedTest` (removed by a guard), assert `StartupItemAdded` with `important` importance, then removal.
11. Update `COLLECTORS.md` with all three rows, including the explicit "2128 ms → hourly only" note and the "task action arguments are not collected" privacy note.
12. Commit: `feat(platform): setupapi, adapters, task scheduler`, `feat(collectors): driver collector`, `feat(collectors): network configuration collector`, `feat(collectors): startup and scheduled task collector`, `feat(storage): system snapshots and diff`.

## Todo

- [ ] Driver sweep is hourly + event-triggered, never a short timer
- [ ] Snapshot payload contains no data that is not already an entity, and stays <256 KB
- [ ] `get_snapshot_diff` on identical snapshots returns an empty diff
- [ ] Task action arguments are never read or stored
- [ ] DHCP-only lease churn does not reach the timeline
- [ ] Proxy values pass the URL redaction matcher
- [ ] Restart produces zero re-announced drivers/startup items

## Success Criteria

- [ ] `cargo test --test collector-drivers --test collector-network --test collector-startup --test snapshot-diff` green
- [ ] Measured and recorded in `docs/perf-baseline.md`: driver sweep (expect ~2.1 s), adapters (<1 ms), tasks (expect ~194 tasks), snapshot build time and payload size
- [ ] A timeline page query during a driver sweep stays <100 ms
- [ ] All 15 §7 categories now have at least one producing collector except health checks (deferred) — verified by a test that enumerates registered collectors against the category enum and lists the intentional gaps

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test collector-drivers --test collector-network --test collector-startup --test snapshot-diff
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test architecture-gates
cargo clippy --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --all-targets -- -D warnings
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Driver sweep costs far more than 2128 ms on a loaded machine | Sweep >5 s | Sweep already runs on `spawn_blocking` under the heavy-work semaphore; if it exceeds 5 s twice, the cadence drops to every 4 hours and the change is recorded in Diagnostics and `COLLECTORS.md` |
| Task Scheduler COM enumeration is slow or throws on a protected folder | Partial results or an error | Per-folder error is caught, that folder is skipped with a counter, and the rest is reported (§38); depth and count caps bound the walk |
| `StartupApproved` binary decoding is wrong | Items reported disabled when enabled | Unknown byte patterns set `approval_unknown` and the event omits the enabled/disabled claim entirely rather than guessing (§29 honesty); fixtures cover the known 12-byte enabled/disabled forms |
| Snapshot payloads dominate the database | DB grows faster than events | 7-day snapshot retention + 256 KB cap + counts-not-lists on overflow; the storage estimate in Settings breaks out snapshot bytes separately so the user can see it |
| `GetIpForwardTable2` unavailable or empty | No gateway data | Gateway falls back to the per-adapter gateway list from `GetAdaptersAddresses`; the route table is an enhancement, not a requirement |

## Security Considerations

Driver provider/version and adapter data are not sensitive, but adapter friendly names and DNS suffixes can carry
organization names, and proxy strings can carry credentials — all three pass the redactor. Scheduled-task action
arguments are deliberately excluded, and that exclusion is asserted by an architecture-gate grep for
`Arguments` in the task scheduler module, so a future "small improvement" cannot quietly start collecting command
lines. Snapshots are sealed before persistence like every other payload (§34).

### FILE: phase-16-checkpoints-notifications.md

---
phase: 16
title: "Checkpoints & Conservative Notifications"
status: pending
priority: P1
effort: 7h
dependencies: [15]
---

## Overview

Checkpoints turn "it worked before" into a named point in time the user chose, and they are the highest-value
feature per line of code in the product. Notifications are the opposite: the feature most likely to make the app
uninstallable, so §53's conservative policy is implemented as a *default-deny* rule set with a test that proves a
Chrome update can never produce a toast.

## Requirements

**Functional — checkpoints (§24, §25)**
- `checkpoints` table: `id`, `label`, `created_at`, `snapshot_id`, `note`, `auto_created`.
- Create: from the tray, from the header button, and from the keyboard (`Ctrl+K`). Creating one takes a snapshot (Phase 15) synchronously so the checkpoint is a real system state, and shows the §25 suggested-label chips: `Everything works`, `Before update`, `Before install`, `Clean state`, `Working build` — chips fill the input, the user can type anything, and the label is required (empty label → the create button stays disabled with the reason shown).
- Rename, delete (with a confirm dialog naming the label), and list (newest first, with the change count since each).
- "Since this checkpoint" view: exact §24 copy `12 important changes since this checkpoint`, plus the full filtered timeline below and a `Compare system state` action that renders the Phase 15 snapshot diff.
- Auto-checkpoint: one on first run (label `First run`, `auto_created = true`) so the very first investigation has a baseline; nothing else auto-creates (no surprise clutter).
- Tray: `Create checkpoint…` entry that opens the window focused on the create field (§67 tray behaviour).
- Command surface: `create_checkpoint(label, note?)`, `list_checkpoints()`, `rename_checkpoint(id, label)`, `delete_checkpoint(id)`, `get_checkpoint_delta(id)` (counts by importance + the event page), `compare_checkpoint(id)` (snapshot diff vs. now, taking a fresh snapshot).

**Functional — notifications (§53)**
- Default state: notifications **on** but restricted to `critical` only. Settings offers `Off`, `Critical only` (default), `Important and critical`. There is no "everything" option — by design.
- Default-deny rule engine: an event notifies only if all of these hold — importance ≥ threshold, the category is in the notify allowlist, the event is not `synthetic`, the source is not `informational`-classified, no more than 3 notifications in the trailing 10 minutes (rate limit), no identical `(entity_id, event_type)` toast in the trailing hour (dedupe), and the app is not in Do-Not-Disturb (user toggle + automatic during the first 60 s after app start, so a startup backfill can never storm).
- Notify allowlist: `service` (start-type change / stop-without-restart), `network_config` (DNS/gateway/proxy), `driver` (important classes), `startup` (item added), `port` (conflict), `file_config` (change in a watched location the user marked as `watch_closely`), `windows_update` (failed install). Application updates, process start/stop, and container events are **never** notification-eligible.
- Toast content: title = the event summary, body = the "why this matters" line, one action `View change` that focuses the window on the event's details drawer. No action can change system state.
- Delivery via Tauri's notification capability (permission declared narrowly in the capability file); if the OS denies notifications, the app records it once, shows an inline Settings note, and continues (§38).

**Non-functional**
- The notification decision is a pure function `should_notify(event, state, settings) -> Option<Toast>` so the entire policy is unit-testable with zero OS involvement — this is what makes the "Chrome updated never notifies" assertion cheap and permanent.

## Architecture

`domain/notify_policy.rs` (pure), `app/notifications.rs` (delivery + rate-limit state),
`storage/checkpoints.rs`, `commands/checkpoints.rs`, and the UI in
`src/lib/components/checkpoints/*` plus `src/lib/state/checkpoints.svelte.ts`. The tray menu gains its entry in
`app/tray.rs` (Phase 05).

## Related Code Files

**Create:** `src-tauri/src/domain/notify_policy.rs`, `src-tauri/src/app/notifications.rs`,
`src-tauri/src/storage/checkpoints.rs`, `src-tauri/src/commands/checkpoints.rs`,
`src-tauri/migrations/0008_checkpoints.sql`,
`src/lib/state/checkpoints.svelte.ts`,
`src/lib/components/checkpoints/{CheckpointList,CreateCheckpointDialog,CheckpointDelta,SnapshotDiffView,LabelChips}.svelte`,
`src-tauri/tests/{notify-policy.rs,checkpoints.rs}`, `tests/e2e/checkpoints.spec.ts`
**Modify:** `src-tauri/src/app/tray.rs`, `src-tauri/capabilities/default.json` (notification permission),
`src/routes.ts`, `docs/IPC-SURFACE.md`, `docs/COLLECTORS.md` (notify-eligibility column), `STUBS.md`
**Delete:** the Phase 06 checkpoints placeholder body.

## Implementation Steps

1. `storage/checkpoints.rs` + migration; `create_checkpoint` takes the snapshot first, then inserts, in one transaction so a checkpoint can never reference a missing snapshot.
2. `commands/checkpoints.rs` with validation: label 1–64 chars after trim, note ≤512, `delete` refuses the auto `First run` checkpoint only if it is the last remaining one (so the user always has a baseline) — the UI explains that.
3. `get_checkpoint_delta`: counts by importance since `created_at` + the first page of events; the UI composes the exact §24 string from the `important` count.
4. `domain/notify_policy.rs`: the pure rule engine with an explicit `NotifyDecision { Notify(Toast), Suppressed(Reason) }` — suppression reasons are recorded in Diagnostics so "why didn't I get a toast?" is answerable.
5. `app/notifications.rs`: rate-limit ring buffer, dedupe map with hourly expiry, DND window, delivery, permission-denied handling, and the `View change` action wiring (focus window + emit a `focus-event` payload the frontend routes to the drawer).
6. UI: `CreateCheckpointDialog.svelte` with `LabelChips`, required-label validation, and an optimistic list update; `CheckpointDelta.svelte` with the §24 copy and the compare action; `SnapshotDiffView.svelte` rendering grouped added/removed/changed with the same badge language as the timeline.
7. `tests/notify-policy.rs` — the decisive test battery: (a) a Chrome `ApplicationUpdated` at `informational` returns `Suppressed(NotEligibleCategory)`; (b) a Node.js update at `important` with threshold `Critical only` returns `Suppressed(BelowThreshold)`; (c) a DNS change at `important` with threshold `Important and critical` returns `Notify`; (d) the 4th eligible event in 10 minutes is `Suppressed(RateLimited)`; (e) the same DNS change an hour later notifies again but 5 minutes later does not; (f) every event during the first 60 s is `Suppressed(StartupQuietPeriod)`; (g) a synthetic event is never eligible.
8. `tests/checkpoints.rs`: create → delta counts correct after seeding events → rename → compare returns a non-empty diff after a seeded change → delete guard behaviour.
9. `tests/e2e/checkpoints.spec.ts`: create via the dialog with a chip, see it listed, open the delta view, expand the compare, keyboard-only path, and the confirm-on-delete flow.
10. Update `COLLECTORS.md` with a notify-eligible column per collector so the policy is auditable next to the collector it governs.
11. Commit: `feat(storage): checkpoints`, `feat(app): conservative notification policy`, `feat(ui): checkpoint creation, delta and comparison`.

## Todo

- [ ] Checkpoint creation is atomic with its snapshot
- [ ] §25 suggested-label chips present with the exact five labels
- [ ] §24 copy `N important changes since this checkpoint` rendered from real counts
- [ ] Default notification threshold is `Critical only`, with no "everything" option
- [ ] Suppression reasons recorded and surfaced in Diagnostics
- [ ] Startup quiet period prevents a backfill toast storm
- [ ] Notification actions cannot change system state

## Success Criteria

- [ ] `cargo test --test notify-policy --test checkpoints` green, including all seven policy cases
- [ ] The "Chrome updated" case is asserted `Suppressed` — a regression here fails the build
- [ ] Creating a checkpoint from the tray opens the window with the label field focused
- [ ] `compare_checkpoint` on an unchanged system returns an empty diff, not noise
- [ ] `bun run test:e2e -- tests/e2e/checkpoints.spec.ts` green

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test notify-policy --test checkpoints
cd "D:/Project/.WhatChage" && bun run check && bun run test:unit
cd "D:/Project/.WhatChage" && bun run test:e2e -- tests/e2e/checkpoints.spec.ts
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Notifications become annoying anyway | The developer's own dogfooding shows >3 toasts/day at defaults | Tighten the allowlist (remove `startup` first, then `driver`), never loosen; every change to the allowlist requires a new `notify-policy` test case documenting the rationale |
| Tauri notification permission denied by Windows Focus Assist | No toasts appear, no error | Delivery result is checked; a denied/failed send increments a counter and Settings shows "Windows is suppressing notifications" with a link to Windows settings — never a silent dead feature |
| Snapshot-on-create makes checkpoint creation feel slow | Dialog blocks >1 s | The snapshot excludes the 2128 ms driver sweep (it reuses the most recent driver state from entities, ≤1 h old, with `metadata.driver_state_age_ms`); target <300 ms, asserted in the checkpoint test |
| Deleting a checkpoint orphans a snapshot | Snapshots accumulate | Delete removes the checkpoint and releases its snapshot's retention pin; the retention test asserts the snapshot becomes eligible |
| The delta count and the timeline disagree | UI shows 12 but lists 11 | Both come from the same query with the same filter; a test asserts count == length for a seeded set, and the count is never computed in the frontend |

## Security Considerations

Toast bodies are built from already-sealed summaries, so a notification cannot leak a secret onto a lock screen —
worth stating explicitly because toasts render outside the app's trust boundary. The `View change` action carries
only an event id. Checkpoint labels are user input: they are length-bounded, stored as parameters, and rendered as
text (never as HTML), so a label cannot become an injection vector in the UI.

### FILE: phase-17-analysis-engine.md

---
phase: 17
title: "Deterministic Analysis & Ranking Engine"
status: pending
priority: P1
effort: 10h
dependencies: [16]
---

## Overview

The product's actual claim: given "my API returns 500 since this morning", return a ranked, explained list of
changes that plausibly caused it. Every step is deterministic and inspectable — no model, no network, no
randomness — so the same problem and the same history always produce the same ranking, and every rank is
defensible by a printed score breakdown (§28, §29, §59).

## Requirements

**Functional — problem input parsing (§27)**
- Input: free text plus an optional explicit time bound ("since this morning", or a picked timestamp).
- `problem_parser.rs` extracts, without a regex crate (§80) and without an LLM: (a) a time hint from a closed vocabulary (`just now`, `this morning`, `today`, `yesterday`, `last night`, `this week`, `since <date>`, `N minutes/hours/days ago`) resolved against local time, defaulting to 24 h when absent; (b) signal tokens — ports (`:3000`, `port 3000`), URLs/hosts, process/tool names matched against observed entity names, service display names, container/image names, file names, and error-shape keywords (`500`, `timeout`, `refused`, `EADDRINUSE`, `certificate`, `dns`, `permission`, `build`, `slow`); (c) a `failure_category` from a fixed map (network / port-conflict / config / dependency / permission / performance / build / unknown).
- Unmatched free text is not discarded: it is kept for the semantic-category term and shown back to the user as "interpreted as …" so a wrong parse is visible rather than silent.

**Functional — candidate retrieval**
- Window: `[problem_time - lookback, problem_time]`, lookback = max(time hint, 1 h), capped at 30 days.
- Candidates = events in the window, plus events outside the window that touch an entity named by the problem (a config change three days ago is still a candidate if the problem names that file), capped at 2000 candidates ordered by importance then recency.

**Functional — relationship traversal**
- BFS from each problem-named entity over the relationship graph, **max depth 3**, visited set, edge budget 5000. Depth is recorded per candidate (`relationship_distance`), and depth >3 is not "weak evidence" — it is excluded, so the engine cannot manufacture a story out of six hops.

**Functional — scoring (§28, all six terms, no others)**
- `temporal`: proximity of the event to the problem time, decaying over the lookback window (an event 2 minutes before beats one 20 hours before).
- `relationship`: 1 / (1 + distance) with a floor of 0 for unrelated candidates.
- `semantic_category`: affinity between `failure_category` and `EventCategory` from a fixed, readable matrix (port-conflict × port = high; network × network_config = high; config × file_config = high; build × application = medium; any × process = low unless the process is named).
- `severity`: from the event's importance level.
- `rarity`: inverse frequency of this `(entity, event_type)` over the trailing **30 days** — a service that restarts hourly scores near zero; a DNS change that happened once in a month scores high.
- `failure_proximity`: whether the event's entity is *itself* named in the problem, or listens on a named port, or configures a named consumer.
- Weights live in one `const SCORE_WEIGHTS` struct with a comment per weight, and every candidate carries a `ScoreBreakdown { temporal, relationship, semantic_category, severity, rarity, failure_proximity, total }` that the UI renders verbatim (Phase 18's "Why this is ranked #1").

**Functional — confidence tiers (§29)**
- `Confirmed`: the causal link is observed, not inferred (kernel-table port ownership, service dependency list, a config file that names the failing port).
- `Likely`: a strong single-hop relationship plus tight temporal proximity.
- `Correlated`: temporal/statistical only — no relationship path.
- Tier language is enforced: a `Correlated` result may never be phrased as "caused"; the vocabulary per tier lives in `domain/explain_vocab.rs` and is asserted by a forbidden-language test (Phase 18 extends it to the UI).

**Functional — causal chain building (§30)**
- From the top candidates, build chains by walking relationships forward toward the problem entity: e.g. `file:.env (changed) → configures → process:node.exe → listens_on → port:tcp/3000 ← problem names port 3000`.
- A chain is emitted only when every edge exists in the graph; missing edges truncate the chain rather than being bridged by inference. Chains are capped at 5 nodes and deduped by node set.
- Reboot markers (Phase 14) split chains: a chain may not cross a `SystemRebooted` boundary without dropping to `Correlated`.

**Functional — explanation provider (§31, §32)**
- `trait ExplanationProvider { fn explain(&self, req: &ExplanationRequest) -> Result<Explanation>; fn id(&self) -> &'static str; }`.
- `DeterministicExplanationProvider` is the only implementation shipped and the only one wired: it composes explanations from templates + the score breakdown + the chain, entirely offline.
- The trait exists so §32's optional cloud provider is a *later addition behind a default-off setting*, with no network code, no API-key handling, and no provider registry in this MVP beyond the single deterministic entry. Settings shows the cloud option as "Not available in this version" rather than a dead toggle.

**Functional — command surface**
- `analyze_problem(input: ProblemInput) -> AnalysisResult` where `AnalysisResult { interpreted: InterpretedProblem, candidates: Vec<RankedChange>, chains: Vec<CausalChain>, searched_window, candidate_count, truncated }`.
- `explain_change(event_id, problem_context?) -> Explanation` for the single-event "why does this matter" path.
- Analysis runs on `spawn_blocking` under the heavy-work semaphore; target **<500 ms** at 100k events, **<2 s** at 1M.

**Non-functional**
- Fully deterministic: no `HashMap` iteration order in any ranked output (`BTreeMap`/sorted vectors only), no time-of-day dependence beyond the explicit problem time, no floating-point summation order ambiguity (fixed term order, `f64`, ties broken by `(timestamp desc, event_id desc)`).
- Zero network access in the entire module — asserted by an architecture-gate grep for `reqwest`/`http`/`ureq` under `src/analysis`.

## Architecture

```
ProblemInput ─> problem_parser ─> InterpretedProblem
                                      │
                    candidate_retrieval (window + named entities)
                                      │
                    graph_traversal (BFS ≤3) ──> distances
                                      │
                    scoring (6 terms) ──> ScoreBreakdown per candidate
                                      │
                    chain_builder (edges must exist) ──> CausalChain[]
                                      │
                    DeterministicExplanationProvider ──> Explanation
                                      │
                                 AnalysisResult
```

Every arrow is a pure function over data already in the repository; only `candidate_retrieval` touches SQLite.
That split is what makes the whole engine fixture-testable.

## Related Code Files

**Create:** `src-tauri/src/analysis/{mod.rs,problem_parser.rs,candidate_retrieval.rs,graph_traversal.rs,scoring.rs,chain_builder.rs,confidence.rs,explain.rs,vocab.rs,weights.rs}`,
`src-tauri/src/commands/analysis.rs`,
`src-tauri/tests/{analysis-ranking.rs,analysis-parser.rs,analysis-determinism.rs,analysis-perf.rs}`,
`src-tauri/fixtures/scenarios/{port-conflict.json,env-change.json,dns-change.json,driver-update.json,noise-only.json}`,
`docs/ANALYSIS.md`
**Modify:** `src-tauri/src/lib.rs`, `docs/IPC-SURFACE.md`, `ARCHITECTURE.md`, `docs/perf-baseline.md`,
`src-tauri/tests/architecture-gates.rs` (no-network + no-`unwrap` in `analysis/`)
**Delete:** none.

## Implementation Steps

1. `weights.rs` first: the six weights as named constants with a one-line rationale each, plus a `debug_assert` that they sum to 1.0. Weights are a product decision, so they live in one file a human can review.
2. `problem_parser.rs`: closed-vocabulary time resolution, then token extraction against a `KnownNames` snapshot (entity names, service display names, image names) passed in — the parser never queries the DB itself, keeping it pure.
3. `candidate_retrieval.rs`: the two queries (window + named-entity) with the 2000 cap and a `truncated` flag surfaced to the UI (§29 honesty — the user is told when the search was bounded).
4. `graph_traversal.rs`: BFS with depth cap 3 and edge budget; returns `BTreeMap<EntityId, u8>` distances.
5. `scoring.rs`: the six terms as six small functions, each unit-tested in isolation; `rarity` uses a single grouped count query over the trailing 30 days, computed once per analysis and cached in a map.
6. `confidence.rs`: tier assignment from (distance, edge kinds on the path, temporal delta); `vocab.rs`: per-tier allowed verbs (`Confirmed`: "caused", "broke"; `Likely`: "likely caused", "would explain"; `Correlated`: "happened just before", "may be unrelated") and a `forbidden_for(tier)` list.
7. `chain_builder.rs`: forward walk with existence-only edges, 5-node cap, dedupe, reboot-boundary demotion.
8. `explain.rs`: the trait + `DeterministicExplanationProvider` composing `headline`, `reasoning` (from the breakdown's top three terms, named in plain language), `chain_text`, `what_to_check` (a fixed remediation hint per `(failure_category, event_category)` pair — advisory text only, the app never changes anything), and `confidence`.
9. `commands/analysis.rs`: validation (input ≤512 chars, explicit time within the last 366 days), `spawn_blocking`, semaphore.
10. Fixtures + `tests/analysis-ranking.rs` — the decisive §59 test: history contains (a) `.env` change to `PORT=3000` 8 minutes before the problem, (b) `node.exe` started listening on 3000 6 minutes before, (c) a Chrome `ApplicationUpdated` 5 minutes before, (d) 40 routine process events. Problem: `my API returns 500 since this morning, port 3000`. Assertions: rank 1 and 2 are the `.env` change and the port event (in either order), the Chrome update ranks **below both** and below every chain member, a chain containing `file:.env → configures → process:node.exe → listens_on → port:tcp/3000` is present, the Chrome update's tier is `Correlated`, and its explanation text contains no forbidden verb.
11. `tests/analysis-determinism.rs`: run the same fixture 50 times (and once with a shuffled insert order) asserting byte-identical serialized `AnalysisResult`.
12. `tests/analysis-parser.rs`: the time vocabulary table, port/host/name extraction, and the `noise-only.json` case asserting an honest empty result (`"No changes in this window look related."`) instead of a forced top-1.
13. `tests/analysis-perf.rs` (`#[ignore]`d, `WC_PERF_EVENTS`): timing at 100k and 1M into `docs/perf-baseline.md`.
14. `docs/ANALYSIS.md`: the pipeline diagram, the weight table, the tier rules, the affinity matrix, and an explicit "what this engine will not claim" section.
15. Commit: `feat(analysis): problem parsing and candidate retrieval`, `feat(analysis): six-term scoring and confidence tiers`, `feat(analysis): causal chains and deterministic explanations`.

## Todo

- [ ] All six §28 terms implemented, weighted in one reviewable file, and exposed per candidate
- [ ] BFS hard-capped at depth 3; deeper paths excluded, not down-weighted
- [ ] Rarity computed over the trailing 30 days from real counts
- [ ] Chains built only from existing edges; reboot boundaries demote to Correlated
- [ ] `ExplanationProvider` trait present with exactly one deterministic implementation wired
- [ ] Empty/honest result path when nothing correlates
- [ ] `truncated` surfaced when the candidate cap was hit
- [ ] No network, no randomness, no unordered iteration in ranked output

## Success Criteria

- [ ] `cargo test --test analysis-ranking --test analysis-parser --test analysis-determinism` green
- [ ] The §59 fixture assertion holds: the port/config chain outranks the unrelated Chrome update
- [ ] 50 repeated runs produce byte-identical results
- [ ] Measured: <500 ms at 100k events and <2 s at 1M, recorded in `docs/perf-baseline.md`
- [ ] `docs/ANALYSIS.md` documents every weight and tier rule actually implemented

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test analysis-ranking --test analysis-parser --test analysis-determinism
WC_PERF_EVENTS=1000000 cargo test --release --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test analysis-perf -- --ignored --nocapture
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test architecture-gates
cargo clippy --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --all-targets -- -D warnings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Ranking looks plausible on fixtures but wrong in real use | Dogfooding shows an obviously-unrelated event at #1 | The breakdown is printed in the UI, so the offending term is identifiable; fix by adjusting the affinity matrix or the rarity window with a new fixture added to `analysis-ranking.rs` — never by hard-coding a special case for one event type |
| Weight tuning becomes endless | Repeated weight edits with no test change | Rule: a weight may only change together with a fixture that the old weights fail and the new weights pass; enforced socially by the review checklist in `docs/ANALYSIS.md` |
| Rarity query is slow at 1M events | Analysis exceeds the 2 s target | Rarity is one grouped query over a 30-day index range, computed once per analysis; if still slow, maintain an incrementally-updated `entity_event_counts` rollup table (schema already reserved in the migration) and re-measure |
| The parser mis-reads the time hint and searches the wrong window | User sees "interpreted as: yesterday" when they meant this morning | The interpretation is always displayed and directly editable (Phase 18 exposes the resolved window as an editable range), so a mis-parse is a visible, one-click fix rather than a silent wrong answer |
| A `Correlated` result reads as causal | Copy review finds "caused" on a correlated card | `vocab.rs` + the forbidden-language test cover both the generated `Explanation` strings and (Phase 18) the rendered DOM |
| Cap of 2000 candidates hides the real cause | `truncated = true` on a real investigation | Candidates are ordered by importance then recency before the cap, so `critical`/`important` events survive truncation; the UI states the window was bounded and offers a narrower time range |

## Security Considerations

The analysis module reads only already-sealed data, so explanations inherit redaction — an explanation can name
`.env`'s `PASSWORD` key but never its value. The problem text the user types is *input*, not persisted state by
default: it is used for the analysis and stored only if the user saves the investigation (Phase 18), and it passes
through the redactor first because users paste error messages containing tokens. The zero-network gate is what
makes §32's "cloud AI off by default" trivially true in this version: there is no code path to a network at all.

### FILE: phase-18-investigation-experience.md

---
phase: 18
title: "Home & Investigation Experience"
status: pending
priority: P1
effort: 8h
dependencies: [17]
---

## Overview

The engine from Phase 17 becomes the product here: the §43 home screen with its exact §72 voice, and the §45
investigation screen whose tone target is §73/§74. This phase is where the app either earns trust or sounds like a
fortune teller, so the copy is treated as a specification and the confidence language is enforced by a test.

## Requirements

**Functional — home screen (§43, §72 exact voice)**
- Header: product name `What Changed` left, a live status pill right reading `● Active` when recording, `● Paused` when the user paused (§57 control), `● Limited` when any collector is `Unavailable`/`Failed`.
- Hero status line, verbatim from §72: `Everything looks normal.` when there are no `important`/`critical` events in the last 24 h, otherwise `N recent changes may need attention.` (the §72 form, pluralized correctly, `1 recent change may need attention.`).
- Prompt label verbatim: `What stopped working?`
- Input placeholder verbatim from §72: `Example: Discord stopped detecting my microphone this morning`. The §43 mock's `Tell me what happened…` is the losing alternate — `docs/decisions/0003-microcopy.md` records that §72 wins and the alternate stays in `docs/copy-deck.md` only.
- Primary button: `Investigate` (disabled with an explanatory hint until the input has ≥3 non-whitespace characters; `Enter` submits).
- Today block: `Today` heading then the three counters in the §43 order — `37 changes`, `2 important`, `0 critical` — sourced from `get_dashboard_summary`, each clickable to open the timeline pre-filtered.
- Section heading verbatim: `Important changes` (§72), listing up to 5 `important`/`critical` events with the §44 event presentation (time, headline, `581.xx → 582.xx`-style value delta, a relative-context line where one exists, `[ View details ]`).
- Privacy line verbatim, always visible on home: `Your history stays on this PC.`
- Empty first-run state uses the same layout with `Everything looks normal.` and a "Recording started just now — changes will appear here." line, so the screen is never a blank void (§39).

**Functional — investigation screen (§45, tone per §73/§74)**
- Heading `Investigation`, then the user's problem text echoed as typed (redacted for display if it contained a secret shape), then the honest count line: `We found 3 changes that may be related.` — and when the engine found none: `We found no changes that look related.` followed by the searched window and a "widen the window" action (never a forced #1).
- Ranked cards `#1`, `#2`, `#3` … each with: a tier badge (`Strong match` for Confirmed — the §45/§74 wording; `Possible match` for Likely; `Happened around the same time` for Correlated, defined in `docs/copy-deck.md`), the event headline, the time, a `Why this matters:` line, a `Timing:` line in the §45 form (`8 minutes before the reported problem.`), and `[ View evidence ]`.
- `#1` additionally shows the §73-shaped rationale block: `Why it stands out` with the top three score-breakdown terms rendered as sentences, and — for the top card only — the honest disclaimer verbatim from §73: `This does not prove the driver caused the crash, but it is the strongest recent change we found.` is generalized in the copy deck to `This does not prove <headline> caused the problem, but it is the strongest recent change we found.`
- `Why this is ranked #1` expander: the full `ScoreBreakdown` as a labelled bar list (temporal, relationship, category match, severity, rarity, direct mention) with each term's numeric contribution — the engine's reasoning is inspectable, not a black box (§31).
- `What happened` chain view: the §45 vertical arrow ladder (`10:37 Discord updated ↓ 10:42 microphone permission changed ↓ … ↓ ~10:50 issue observed`), with the problem time as the final node prefixed `~`. Built from `CausalChain`, plain DOM (the 3D graph in Phase 20 is an alternate view, never the only one).
- `Show technical details` affordance (§45) revealing raw identifiers, source collector, and the exact timestamps.
- `Possible next step` block (§74) rendered from `what_to_check` — advisory text only, with no action that modifies anything (§74 "Do not automatically modify config in MVP").
- Save/recall: `Save investigation` persists problem text (redacted), resolved window, and the ranked event ids so the user can reopen it; the list lives on the home screen's "Recent investigations" strip when ≥1 exists.
- The resolved interpretation is visible and editable: `Searched 06:00 – 11:20 today · interpreted from "this morning"` with a range picker inline, so a mis-parse is a one-click fix.

**Non-functional**
- Analyze → first paint of results ≤700 ms at 100k events (engine ≤500 ms + render); a spinner appears only after 200 ms to avoid a flash, and the button shows a determinate "Analyzing…" state.
- Every string on both screens comes from `src/lib/copy/deck.ts` (typed, one export per key) — no inline literals, so a copy audit is a single-file review and the forbidden-language test can enumerate keys.

## Architecture

`src/lib/state/investigation.svelte.ts` owns the analysis lifecycle (`submit`, `result`, `status`, `saved`), the
only caller of `analyze_problem`/`explain_change`. `src/lib/copy/deck.ts` is the single copy source, typed as
`Record<CopyKey, string | (args) => string>`, imported by every component. Components:
`HomeView.svelte` (`StatusPill`, `HeroStatus`, `ProblemInput`, `TodayCounters`, `ImportantChangesList`,
`PrivacyLine`, `RecentInvestigations`) and `InvestigationView.svelte` (`InterpretationBar`, `RankedChangeCard`,
`ScoreBreakdownPanel`, `ChainLadder`, `TechnicalDetails`, `NextStep`). Tier→label mapping lives once in
`src/lib/copy/confidence.ts` so no component can invent its own wording.

## Related Code Files

**Create:** `src/lib/copy/{deck.ts,confidence.ts}`, `src/lib/state/investigation.svelte.ts`,
`src/lib/components/home/{HomeView,StatusPill,HeroStatus,ProblemInput,TodayCounters,ImportantChangesList,PrivacyLine,RecentInvestigations}.svelte`,
`src/lib/components/investigation/{InvestigationView,InterpretationBar,RankedChangeCard,ScoreBreakdownPanel,ChainLadder,TechnicalDetails,NextStep}.svelte`,
`src-tauri/src/storage/investigations.rs`, `src-tauri/src/commands/investigations.rs`,
`src-tauri/migrations/0009_investigations.sql`,
`tests/e2e/{home.spec.ts,investigation.spec.ts}`, `src/lib/copy/deck.test.ts`,
`src-tauri/tests/investigation-language.rs`
**Modify:** `src/routes.ts`, `src/lib/state/app.svelte.ts`, `docs/IPC-SURFACE.md`,
`docs/decisions/0003-microcopy.md`, `docs/copy-deck.md`, `STUBS.md`
**Delete:** the Phase 06 home and investigation placeholder bodies.

## Implementation Steps

1. Write `deck.ts` first, transcribing every §72/§43/§44/§45/§73/§74 string exactly, with a `// §NN` comment per key. This is the phase's source of truth; components are written against it.
2. `confidence.ts`: `tierLabel(tier)`, `tierTone(tier)` (semantic color token), and `tierDisclaimer(tier, headline)`; `Correlated` returns the "happened around the same time" label and a mandatory hedge sentence.
3. `HomeView.svelte` in the §43 vertical order, desktop density (max content width 880 px, centered), status pill bound to `get_collector_status` polled every 5 s plus the `collector-status-changed` event.
4. `HeroStatus`: derives from the dashboard summary; the two §72 forms plus correct singular/plural.
5. `ProblemInput`: textarea (1 row growing to 3), placeholder from the deck, `Enter` submits / `Shift+Enter` newlines, ≥3-char guard with the reason rendered next to the disabled button (§39 never a dead control).
6. `TodayCounters` + `ImportantChangesList`: one `get_dashboard_summary` call, counters clickable into the timeline with the filter preset, list items reusing `TimelineRow`'s value-delta and details-drawer wiring (DRY with Phase 09).
7. `investigation.svelte.ts`: `submit(text)` → `analyze_problem`, 200 ms delayed spinner, error path (engine error → "Something went wrong while analyzing. Your history is unaffected." plus a retry), result caching keyed by `(text, window)` so re-render is free.
8. `RankedChangeCard` + `ScoreBreakdownPanel`: the six labelled terms with numeric contributions and a one-sentence plain-language reading per term; `#1` gets `Why it stands out` (top three terms) and the §73 disclaimer.
9. `ChainLadder`: renders `CausalChain` as an accessible ordered list with `↓` separators as decorative `aria-hidden` glyphs; the final node is the problem time prefixed `~`. Truncated chains show `…` with a "chain incomplete — no recorded link" note rather than a bridged guess.
10. `TechnicalDetails` (details/summary, collapsed by default) + `NextStep` (advisory text, no buttons that act).
11. `storage/investigations.rs` + `commands/investigations.rs`: `save_investigation`, `list_investigations(limit)`, `get_investigation(id)`, `delete_investigation(id)`; problem text sealed before insert.
12. `deck.test.ts`: asserts the ten verbatim §72/§43/§45 strings byte-for-byte (a copy regression fails CI), asserts no key is an empty string, and asserts every `CopyKey` is referenced by at least one component (dead-copy check via a source scan).
13. `investigation-language.rs`: for every tier, generate explanations across all five scenario fixtures and assert the rendered text contains no forbidden verb for that tier (`caused`, `because of`, `due to`, `broke` are forbidden for `Correlated`; `proves`, `definitely`, `guaranteed` are forbidden for all tiers).
14. `tests/e2e/home.spec.ts` + `investigation.spec.ts`: exact-string assertions for the §72 lines, the disabled-button hint, submit → ranked cards, breakdown expander, chain ladder order, no-results honest path, interpretation edit re-runs the analysis, keyboard-only completion of the whole flow, and a DOM scan asserting the forbidden verbs are absent from a `Correlated` card.
15. Commit: `feat(ui): copy deck`, `feat(ui): home screen`, `feat(ui): investigation screen with score breakdown`, `feat(storage): saved investigations`.

## Todo

- [ ] Every §72 string present verbatim and asserted by test
- [ ] §43 layout order and the three counters implemented
- [ ] `Your history stays on this PC.` visible on home
- [ ] Tier labels centralized; `Correlated` never phrased causally
- [ ] `Why this is ranked #1` exposes all six score terms numerically
- [ ] `What happened` ladder present as plain accessible DOM
- [ ] Honest empty-result path with a widen-window action
- [ ] Interpretation visible and editable
- [ ] No action anywhere on these screens modifies system state

## Success Criteria

- [ ] `bun run test:unit -- src/lib/copy/deck.test.ts` green with byte-exact §72 strings
- [ ] `cargo test --test investigation-language` green for all tiers × all five fixtures
- [ ] `bun run test:e2e -- tests/e2e/home.spec.ts tests/e2e/investigation.spec.ts` green
- [ ] Typing the §74 problem (`My API returns 502 after I restarted Docker.`) against the seeded scenario yields the port/config chain as `#1` with a visible breakdown
- [ ] Analyze → results paint ≤700 ms at 100k events (measured, recorded)

## Verification Gate

```bash
cd "D:/Project/.WhatChage" && bun run check && bun run test:unit
cd "D:/Project/.WhatChage" && bun run test:e2e -- tests/e2e/home.spec.ts tests/e2e/investigation.spec.ts
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test investigation-language
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Copy drifts from §72 during UI polish | `deck.test.ts` fails | The test is the gate; a deliberate change requires editing `docs/decisions/0003-microcopy.md` with the rationale in the same commit |
| The app sounds certain when it is not | Review or the language test flags a causal verb on a `Correlated` card | `confidence.ts` is the only place tier wording exists, and the hedge sentence is mandatory for `Correlated`; the DOM scan in the e2e spec catches component-level slips |
| Results feel slow on a big history | Spinner visible >1 s regularly | Engine target is already measured; if exceeded, narrow the default lookback from 24 h to 6 h *in the UI default only* (the user can widen) and record the change — never by silently capping candidates lower |
| Score breakdown overwhelms non-technical users | Dogfooding confusion | The panel is collapsed by default behind `Why this is ranked #1`; the always-visible content stays the three plain-language `Why it stands out` sentences |
| Saved investigation text leaks a pasted token | Redaction test on investigations | Problem text is sealed before insert and re-checked by the `redaction-leak` DB scan, which is extended here with an "investigation with a pasted `sk-live-…` error message" specimen |
| Echoing user text into the DOM enables injection | XSS-style payload in the problem field | Svelte escapes text by default and no `{@html}` appears in these components — asserted by an architecture-gate grep for `{@html}` across `src/` |

## Security Considerations

The problem field is the app's only free-text input, so it is the only place a user can hand the app a secret: it
is sealed before persistence, redacted for display echo, and never sent anywhere (Phase 17's zero-network gate).
`{@html}` is banned repo-wide by a gate, and the CSP from Phase 03 blocks inline script, so a pasted payload cannot
execute. `Possible next step` is text, not an action, which is what keeps §74's "do not automatically modify
config" true by construction rather than by discipline.

### FILE: phase-19-three-runtime-ambient-field.md

---
phase: 19
title: "Three.js Runtime & Ambient Field"
status: pending
priority: P2
effort: 6h
dependencies: [18]
---

## Overview

Three.js is in the stack on purpose (§4, §43), so it gets a real runtime instead of a copy-pasted `useEffect`: one
shared renderer lifecycle with a dispose registry, a frame budget, visibility pausing, reduced-motion and
WebGL-absent fallbacks. Getting this right once is what makes Phase 20's causal graph cheap and what keeps the
0–1% idle CPU promise (§56) survivable with 3D on screen.

## Requirements

**Functional — runtime**
- `three-runtime.ts` exposes `createScene(canvas, opts) -> SceneHandle` where `SceneHandle = { start, stop, resize, dispose, stats }`. One `WebGLRenderer` per canvas, `antialias` on only when `devicePixelRatio <= 1.5`, `powerPreference: 'low-power'`, pixel ratio clamped to 2.
- Dispose registry: every geometry, material, texture, and render target created for a scene is registered on creation; `dispose()` walks the registry, calls `renderer.dispose()`, drops the context via `forceContextLoss()`, and nulls the handle. `stats` exposes `renderer.info.memory.geometries/textures` so a test can assert both reach 0 after dispose.
- Frame budget: `requestAnimationFrame` loop with a **30 FPS cap** (accumulator, skip frames rather than shortening deltas), and an adaptive degrade — if the trailing 60-frame average frame time exceeds 24 ms, particle count halves (once, then again, floor 25%) and the fact is recorded in Diagnostics.
- Pause conditions (all of them, checked in one place): `document.visibilityState === 'hidden'`, the window is not focused, the canvas is 0-area or outside the viewport (`IntersectionObserver`), the app is minimized to tray, or the route that owns the canvas is not active. Paused means the RAF loop is cancelled, not throttled.
- `prefers-reduced-motion: reduce` → the scene renders **one** static frame and never animates; no RAF loop is started (§42).
- WebGL absent or context creation failure → `dispose()` and a CSS-only fallback background (a static gradient with the same tokens), logged once, never an error toast. A lost context (`webglcontextlost`) triggers the same fallback plus a single restore attempt on `webglcontextrestored`.
- A global `graphicsMode` setting (`Full` | `Reduced` | `Off`, default `Full`, `Off` forced when reduced-motion is set) is honoured by the runtime, so the user can turn 3D off entirely (§54 performance controls).

**Functional — ambient field (§43)**
- A slow particle field behind the home hero: ~600 points (adaptive), additive-blended `PointsMaterial` with a soft circular sprite generated once on a 32×32 canvas (no texture asset, no network font/texture fetch), drifting on a fixed sinusoidal path — deterministic, seeded, no `Math.random()` per frame.
- Density and hue derive from real data, which is the only reason it earns its place: point count scales with the last 24 h event count (600 base, ±200), and the accent hue shifts toward the warning token when unacknowledged `important`/`critical` events exist. It is decorative but never the *only* signal — the §72 status line always states the same thing in words.
- Readability first (§43): the field is clipped to the hero area, opacity ≤0.35, and sits behind a background layer that guarantees the text contrast ratio computed in Phase 23; a contrast test runs with the field at maximum density.
- Never blocks interaction: `pointer-events: none`, and the canvas is `aria-hidden="true"` with no focusable content.

**Non-functional**
- Home screen with the field running: ≤1 additional % of one CPU core and no measurable GPU fan spin-up on this machine at 30 FPS; the whole field must add <3 MB to the RSS measured in Phase 24.
- Zero new dependencies beyond `three` 0.185.1 — no `three-stdlib`, no post-processing, no `@react-three/*` equivalents (§80).

## Architecture

```
three/
  runtime.ts        createScene / SceneHandle / RAF budget / pause conditions
  dispose.ts        DisposeRegistry (track + walk + assert-empty)
  capability.ts     webgl support probe, reduced-motion, graphicsMode resolution
  sprite.ts         procedural point sprite (canvas-generated, cached module-level)
  scenes/ambient.ts the home particle field (pure builder over AmbientInputs)
```

`scenes/ambient.ts` is a pure builder: `build(registry, inputs) -> { object3D, update(t) }`, so it can be unit
tested headlessly against a stub registry with no WebGL. Svelte integration is one component,
`AmbientField.svelte`, whose `$effect` creates the handle and whose cleanup calls `dispose()` — the only place
Three.js touches component lifecycle.

## Related Code Files

**Create:** `src/lib/three/{runtime.ts,dispose.ts,capability.ts,sprite.ts}`, `src/lib/three/scenes/ambient.ts`,
`src/lib/components/home/AmbientField.svelte`, `src/lib/state/graphics.svelte.ts`,
`src/lib/three/{dispose.test.ts,ambient.test.ts,runtime.test.ts}`, `tests/e2e/graphics.spec.ts`
**Modify:** `src/lib/components/home/HomeView.svelte` (mount the field behind the hero),
`src/lib/components/settings/PerformanceSection.svelte` (the `graphicsMode` control, wired fully in Phase 22),
`src/app.css` (the CSS-only fallback gradient), `docs/perf-baseline.md`, `ARCHITECTURE.md`
**Delete:** none.

## Implementation Steps

1. `dispose.ts`: `DisposeRegistry` with `track<T extends {dispose():void}>(o): T`, `trackObject3D(root)` (walks geometries/materials), `disposeAll()`, and `size()`; `dispose.test.ts` asserts registry emptiness and that double-dispose is a no-op.
2. `capability.ts`: `hasWebGL()` (create a throwaway canvas context, cache the result), `prefersReducedMotion()` (`matchMedia` + a change listener), `resolveGraphicsMode(setting)`.
3. `sprite.ts`: generate the radial-gradient point texture once on an offscreen canvas, register it, and memoize at module scope.
4. `runtime.ts`: renderer construction with the clamped pixel ratio, the 30 FPS accumulator loop, the five pause conditions behind one `shouldRun()` predicate, `ResizeObserver`-driven `resize`, the adaptive degrade hook, context-loss handling, and `dispose()`.
5. `scenes/ambient.ts`: deterministic positions from a seeded LCG (no dependency), the drift `update(t)` with fixed frequencies, `setDensity(n)` and `setAccent(hue)` for data reactivity, all objects registered.
6. `AmbientField.svelte`: `$effect` → `createScene` → `start`; `$effect` on `graphicsMode`/reduced-motion/data inputs → reconfigure or dispose; cleanup → `dispose`. `pointer-events: none`, `aria-hidden`.
7. `graphics.svelte.ts`: the `graphicsMode` store persisted through settings, plus a `frameStats` readable used by Diagnostics.
8. `runtime.test.ts` (jsdom + a stubbed WebGL context): asserts the loop does not start under reduced motion, that `visibilitychange` to hidden cancels the RAF, that a failed context creation yields the fallback path without throwing, and that the FPS cap yields ≤31 ticks in a simulated second.
9. `tests/e2e/graphics.spec.ts` (real Chromium, so real WebGL): mounts home, waits for the canvas, evaluates `window.__WC_SCENE_STATS__()` (a dev-only hook set by the runtime under `import.meta.env.DEV`) to assert geometries/textures > 0 while mounted, then navigates away and asserts both are 0 — a real dispose assertion, not a promise. Also runs with `emulateMedia({ reducedMotion: 'reduce' })` asserting no animation and a single frame.
10. Record on this machine in `docs/perf-baseline.md`: home CPU% with the field `Full` / `Reduced` / `Off`, plus the frame-time average.
11. Commit: `feat(three): scene runtime with dispose registry and frame budget`, `feat(ui): ambient field on home`.

## Todo

- [ ] Dispose registry proves geometries and textures reach 0 in a real browser
- [ ] 30 FPS cap with adaptive degrade implemented
- [ ] All five pause conditions in one predicate
- [ ] Reduced-motion renders one static frame, starts no loop
- [ ] WebGL-absent and context-lost fallbacks are silent and CSS-only
- [ ] `graphicsMode` Off fully removes the canvas
- [ ] Field is `pointer-events: none` and `aria-hidden`, never the only signal
- [ ] No new npm dependency beyond `three`

## Success Criteria

- [ ] `bun run test:unit` green including the three Three.js test files
- [ ] `bun run test:e2e -- tests/e2e/graphics.spec.ts` green including the post-navigation dispose assertion
- [ ] Home CPU with the field running is ≤1% of one core above the field-off baseline (recorded)
- [ ] Text contrast over the field at maximum density meets the Phase 23 threshold
- [ ] Switching `graphicsMode` at runtime never leaks a context (repeat 20× in the e2e spec, `stats` stays bounded)

## Verification Gate

```bash
cd "D:/Project/.WhatChage" && bun run check && bun run test:unit
cd "D:/Project/.WhatChage" && bun run test:e2e -- tests/e2e/graphics.spec.ts
cd "D:/Project/.WhatChage" && bun run build
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| WebGL context leak on route changes | `stats` grows across mounts; eventually "too many contexts" | The 20× mount/unmount e2e loop is the detector; the fix is always the registry, and a leak found here blocks Phase 20 rather than being deferred |
| 3D pushes idle CPU past the §56 promise | Idle sample >1% with home focused | Pause conditions already cover unfocused/hidden; if focused-idle still exceeds budget, drop the cap to 20 FPS then default `graphicsMode` to `Reduced` on first run, recorded in `docs/decisions/0009-graphics-defaults.md` |
| Ambient field hurts readability | Contrast check fails at max density | Opacity and the backing layer are the levers (both already in place); the field loses, text wins — §43 says readability first |
| `three` bundle inflates the app | `bun run build` chunk report shows a large main bundle | `three` is dynamically imported by `AmbientField.svelte`/Phase 20 only, so it lands in its own chunk and the app shell stays small; asserted by a build-output size check in Phase 24 |
| Reduced-motion users still see motion | Manual check or the emulated spec | The reduced-motion branch never starts a loop (structural, not a duration tweak), and the spec asserts it |

## Security Considerations

No external assets: the point sprite is generated procedurally, so the 3D layer adds no network fetch and cannot
weaken the Phase 03 CSP (which stays `script-src 'self'`, `connect-src 'self'`, no `unsafe-eval`). The dev-only
`window.__WC_SCENE_STATS__` hook is compiled out of release builds by `import.meta.env.DEV`, so no diagnostic
surface is exposed to a shipped page. The canvas renders only derived counts — never event text — so nothing
sensitive is rendered into a GPU surface a screenshot could capture.

### FILE: phase-20-causal-graph-system-map.md

---
phase: 20
title: "Causal Graph & System Map"
status: pending
priority: P2
effort: 9h
dependencies: [19]
---

## Overview

The two data-driven 3D views the relationship graph exists to justify: a causal graph for a single investigation
(§30 chains as geometry) and a System Map of the machine's current entity graph. Both are built on the Phase 19
runtime, both are deterministic, and — the non-negotiable part — both ship with a full DOM equivalent so the
feature is not gated on a GPU or a mouse.

## Requirements

**Functional — causal graph (investigation view)**
- Input: the `CausalChain[]` + top `RankedChange[]` from Phase 17. Nodes are entities and events; edges are the
  relationship kinds already recorded — no edge is drawn that does not exist in the graph.
- Deterministic layered layout, hand-written (no layout dependency, §80): layer = relationship distance from the
  problem entity (0 = problem, increasing outward), within-layer order = `(timestamp asc, entity_id asc)`, spacing
  from a fixed constant table. Same data → identical coordinates, asserted by a snapshot test on the computed
  positions.
- Visual language reuses the §35 semantic tokens: node color = importance, node size = score contribution, edge
  style = confidence tier (solid Confirmed, dashed Likely, dotted Correlated). A legend is always visible — an
  unlabeled color code would be a §29 honesty failure.
- Interaction: raycast hover → tooltip with headline + time + tier; click → opens the Phase 09 details drawer
  (one shared drawer, DRY); `OrbitControls` **clamped** — rotation limited to ±35° azimuth / ±25° polar, zoom
  clamped to a 2× range, panning disabled, damping on. The camera can never be lost.
- `Reset view` control, and the graph is a *tab* next to `What happened` (the §45 ladder), never a replacement:
  the ladder stays the default so the primary explanation path is DOM.
- Keyboard: `Tab` enters a node list overlay (the accessible equivalent), `↑/↓` moves selection, `Enter` opens the
  drawer, and selection is mirrored by a highlight in the 3D scene — the DOM ladder is the accessible source of
  truth, the scene is a view of it (§40).

**Functional — System Map (§47)**
- A live view of the current entity graph from the latest snapshot + relationships: processes, ports, services,
  containers, adapters, watched files, grouped into fixed radial clusters by entity kind with a deterministic
  angular ordering (kind order fixed, within-kind alphabetical).
- Node count is bounded to 300 by importance/recency; the bound is stated on screen (`Showing 300 of 1,412`), with
  a per-kind filter row to narrow instead of an unbounded render.
- Hover reveals the entity's last 3 changes; click pivots the timeline to that entity (reusing Phase 09's filter
  pivot). Changed-in-the-last-hour entities pulse once on load (single animation, respects reduced motion).
- `get_system_map(limit, kinds)` command returns `{ nodes, edges, total_nodes, truncated, generated_at }` built
  from the repository — the layout is computed in TypeScript, the data in Rust.
- DOM equivalent: a grouped, sortable table (kind → entity → last change → relationship count) that is the
  fallback when `graphicsMode` is `Off`, WebGL is unavailable, or reduced-motion is set, and is always reachable via
  a `Show as list` toggle.

**Non-functional**
- 300 nodes + ~600 edges at 30 FPS on this machine; edges rendered as a **single** `LineSegments` with a shared
  material and nodes as one `InstancedMesh`, so node count does not multiply draw calls.
- Both views mount and dispose through the Phase 19 registry with the same 0-geometry assertion.
- Layout computation for 300 nodes <16 ms (one frame), measured in a unit test.

## Architecture

```
three/scenes/causal-graph.ts   build(registry, GraphData) -> { object3D, pick(ray), highlight(id) }
three/scenes/system-map.ts     build(registry, MapData)   -> same shape
three/layout/{layered.ts,radial.ts}   pure: data -> Map<NodeId, [x,y,z]>
components/graph/{CausalGraphView,SystemMapView,GraphLegend,NodeList,GraphTooltip,ShowAsList}.svelte
```

`layout/*` is pure and dependency-free, so the determinism test needs no WebGL. Picking uses one `Raycaster`
against the `InstancedMesh` with `instanceId` → node id mapping held in a plain array. Both scenes share
`three/scenes/shared-materials.ts` (one material per tier/importance, registered once) so the two views cannot
double-allocate.

## Related Code Files

**Create:** `src/lib/three/scenes/{causal-graph.ts,system-map.ts,shared-materials.ts}`,
`src/lib/three/layout/{layered.ts,radial.ts}`,
`src/lib/components/graph/{CausalGraphView,SystemMapView,GraphLegend,NodeList,GraphTooltip,ShowAsList,EntityTable}.svelte`,
`src/lib/state/system-map.svelte.ts`, `src-tauri/src/commands/system_map.rs`,
`src-tauri/src/domain/system_map.rs`,
`src/lib/three/layout/{layered.test.ts,radial.test.ts}`, `tests/e2e/graph.spec.ts`,
`src-tauri/tests/system-map.rs`
**Modify:** `src/lib/components/investigation/InvestigationView.svelte` (add the graph tab next to the ladder),
`src/routes.ts` (System Map route), `docs/IPC-SURFACE.md`, `docs/perf-baseline.md`, `STUBS.md`
**Delete:** the Phase 06 system-map placeholder body.

## Implementation Steps

1. `layout/layered.ts`: pure layered layout with the fixed spacing table and the documented tie-break; `layered.test.ts` snapshots positions for a 3-chain fixture and asserts identical output across 20 runs and across shuffled input order.
2. `layout/radial.ts`: fixed kind order, alphabetical within kind, radius by kind index, plus a deterministic jitter derived from a hash of the entity id (never `Math.random()`); tested the same way, plus a 300-node timing assertion (<16 ms).
3. `shared-materials.ts`: the tier/importance material set built once and registered with the Phase 19 registry.
4. `scenes/causal-graph.ts`: `InstancedMesh` nodes + one `LineSegments` for edges with per-vertex colors; dashed/dotted tiers via short segment stitching in the geometry (no `LineDashedMaterial` per-edge material explosion); `pick(ray)` and `highlight(id)`.
5. `scenes/system-map.ts`: the same construction over `radial.ts`, plus the single-pulse animation for recently-changed entities, skipped entirely under reduced motion.
6. `domain/system_map.rs` + `commands/system_map.rs`: bounded query (limit ≤300, kinds validated against the enum), `total_nodes`, `truncated`, `generated_at`; edges restricted to pairs where both endpoints survived the bound (no dangling edges).
7. `CausalGraphView.svelte`: tabbed with the ladder, legend always visible, clamped `OrbitControls` (imported from `three/examples/jsm/controls/OrbitControls.js` — part of `three`, not a new dependency), `Reset view`, and the `NodeList` overlay for keyboard users.
8. `SystemMapView.svelte` + `EntityTable.svelte` + `ShowAsList.svelte`: the 3D view and the table view over the same store, with the toggle persisted; the table is the automatic view when 3D is unavailable.
9. `tests/e2e/graph.spec.ts`: investigation → graph tab renders a canvas; hover a node shows a tooltip; click opens the drawer with the right event; keyboard path reaches every node via `NodeList`; navigating away leaves geometries/textures at 0; with `graphicsMode: Off` the table renders and the canvas is absent; with 300 seeded entities the map reports `Showing 300 of N`.
10. `tests/system-map.rs`: bound enforcement, no dangling edges, deterministic ordering of the returned nodes, and validation rejection of an unknown kind.
11. Record in `docs/perf-baseline.md`: layout time, frame time and CPU% for the 300-node map and for a 12-node causal graph.
12. Commit: `feat(three): deterministic graph layouts`, `feat(ui): causal graph view`, `feat(ui): system map with list fallback`.

## Todo

- [ ] Layouts pure, deterministic, dependency-free, snapshot-tested
- [ ] Nodes as one `InstancedMesh`, edges as one `LineSegments`
- [ ] Legend always visible; edge style encodes confidence tier
- [ ] `OrbitControls` clamped; camera cannot be lost; `Reset view` present
- [ ] DOM ladder remains the default explanation; graph is a tab
- [ ] Full table equivalent for the System Map, automatic when 3D is off/unavailable
- [ ] No edge drawn that does not exist in the relationship graph
- [ ] Node bound stated on screen with per-kind filters
- [ ] Dispose assertions pass for both scenes

## Success Criteria

- [ ] `bun run test:unit` green including both layout test files
- [ ] `bun run test:e2e -- tests/e2e/graph.spec.ts` green, including the `graphicsMode: Off` table path and the dispose assertion
- [ ] `cargo test --test system-map` green
- [ ] 300-node map holds 30 FPS with layout <16 ms (recorded)
- [ ] Every graph node is reachable and openable with the keyboard alone

## Verification Gate

```bash
cd "D:/Project/.WhatChage" && bun run check && bun run test:unit
cd "D:/Project/.WhatChage" && bun run test:e2e -- tests/e2e/graph.spec.ts
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test system-map
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| The 3D graph becomes decoration nobody can read | Dogfooding: the ladder is always used instead | Acceptable and expected — the ladder is the default by design. The graph earns its place only for multi-chain investigations; if it stays unread, it is not expanded further (no scope growth) |
| Dashed/dotted edges via stitched geometry look wrong | Visual review | Fall back to two `LineSegments` objects (solid + stitched) with distinct opacity per tier; still one draw call per style, still no per-edge material |
| Picking is inaccurate on an `InstancedMesh` | Hover highlights the wrong node | `instanceId` mapping is asserted in the e2e spec by hovering a known node position and comparing the tooltip text; a mismatch blocks the phase |
| Layout produces overlapping nodes on dense data | Nodes visually collide at 300 | Within-layer spacing scales with layer population (already in the spacing table) and the per-kind filter row lets the user narrow; the table view is always exact |
| 3D views push memory past the §56 budget | Phase 24 RSS measurement rises | `three` is a dynamic import, both scenes dispose on unmount, and only one scene is ever mounted at a time (asserted in the e2e spec); if RSS still rises, System Map defaults to the table view |
| `OrbitControls` path/name differs in `three` 0.185.1 | Import fails at build | Verify the exact example-module path at implementation time (it is versioned inside the `three` package); if the path differs, use the one the installed package actually exposes — do not guess, and record it in `docs/decisions/0010-three-imports.md` |

## Security Considerations

Both views consume already-sealed, already-bounded data; the map command validates its `limit` and `kinds` inputs
against enums so the frontend cannot request an unbounded scan (a DoS-on-self vector). Tooltips and node labels
render entity names as text through Svelte's escaping, and `{@html}` remains gate-banned. No external asset,
shader source, or font is fetched, so the CSP is unchanged from Phase 03.

### FILE: phase-21-docker-development-mode.md

---
phase: 21
title: "Docker Collector & Development Mode"
status: pending
priority: P1
effort: 12h
dependencies: [20]
---

## Overview

The §20/§21 developer-facing half of the product: containers and the dev-project context that makes an event
legible ("the `.env` you edited belongs to the repo on branch `feature/x`"). Docker is **not installed on the build
machine**, which is stated up front: the absence path is fully verifiable here, the connected path is not, and this
phase is designed so the unverifiable part cannot break the app or the release.

## Requirements

**Functional — Docker collector (§20)**
- Availability probe first, every 60 s while unavailable: does `\\.\pipe\docker_engine` exist? A plain path
  existence check, no connection attempt, no process spawn. Absent → collector state `Unavailable` with reason
  `Docker is not running`, **zero** error logs beyond one line per state transition (§21 explicitly: no error spam).
- Present → connect with `bollard` 0.21.1 over the named pipe. The exact named-pipe feature flag for 0.21.1 is
  **read from the installed crate's manifest/docs at implementation time** and recorded in
  `docs/decisions/0011-docker-transport.md`; it is not guessed here. If the crate cannot connect over a pipe on
  this platform, the fallback is the documented alternative in that decision file, and the collector stays
  `Unavailable` until it works — never a fake success.
- Connected: subscribe to the events stream (`/events`) filtered to `container` and `image` types; on stream error,
  drop to a 10 s `list_containers` diff poll and back off to the availability probe if the connection dies. Both
  paths produce identical `RawEvent`s (one mapping function, DRY).
- Events: `ContainerStarted`, `ContainerStopped`, `ContainerRestarted` (stop+start within 30 s, coalesced by the
  Phase 13 coalescer), `ContainerCreated`, `ContainerRemoved`, `ContainerHealthChanged`, `ImagePulled`,
  `ImageRemoved`, `PortMappingChanged`, `EnvironmentChanged` (key names only, values sealed).
- Summaries follow §20's shape: `Container api restarted`, `Container api port mapping changed: 3000 → 4000`,
  `Image node:22-alpine pulled`.
- Entities `container:<id>` (display: name), `image:<repo:tag>`, `project:<compose project path>`; relationships
  `container → uses_image → image` (Confirmed), `container → publishes → port` (Confirmed, and this is what joins
  Docker to the Phase 11 host port table — the §74 story), `container → belongs_to → project` (Confirmed from the
  compose labels), `file:docker-compose.yml → configures → container` (Correlated).
- Container **env values are never stored**: only key names plus a sealed value; container labels are stored only
  for the compose keys (`com.docker.compose.project`, `.service`, `.working_dir`).
- Cost guard: the events stream is passive; the poll fallback lists containers only (`all: true`), never `inspect`
  per container except once on first sighting.

**Functional — development mode (§21)**
- Opt-in, per project root, added by the user (reuses the Phase 12 `watched_locations` roots plus an explicit
  `is_project` flag) — nothing is scanned unprompted.
- Git state **by reading files only, no process spawn and no git library** (§80: no `gix`): parse `.git/HEAD`
  (ref or detached sha), the pointed-to `.git/refs/heads/<branch>` or `.git/packed-refs`, `.git/MERGE_HEAD`
  presence (merge in progress), and `.git/index` **mtime only**. Events: `BranchChanged`
  (`Switched branch: main → feature/api-port`), `CommitDetected` (HEAD sha changed on the same branch),
  `MergeInProgress`.
- Dirty state honesty: the app does **not** compute a real working-tree diff, so it never claims "clean". It
  reports `dirty (observed)` only when a watched tracked file changed after the last index mtime, and the label
  carries a tooltip explaining it is an observation, not `git status`. `metadata.dirty_source = "observed"`.
- Dependency manifests: watch and structurally diff `package.json`, `bun.lock`/`bun.lockb` (presence + hash only,
  it is binary), `package-lock.json`, `pnpm-lock.yaml`, `yarn.lock`, `Cargo.toml`, `Cargo.lock`,
  `requirements.txt`, `pyproject.toml`, `poetry.lock`, `go.mod`, `go.sum`, `composer.json`, `Gemfile`,
  `Gemfile.lock`, `pom.xml`, `build.gradle(.kts)`, `Dockerfile`, `docker-compose.yml`. Text manifests get the
  Phase 12 structured differ (dependency added/removed/version-changed events with exact old → new); binary or
  huge lockfiles get a hash-change event (`Lockfile changed (bun.lockb)`).
- Runtime/toolchain detection **without spawning processes**: version inferred from observed installed
  applications (Phase 13), from `.nvmrc`/`.node-version`/`rust-toolchain.toml`/`.python-version` file contents, and
  from process image paths already observed by Phase 11. A version that cannot be determined is reported as
  `unknown`, never guessed.
- Project context enrichment: any event whose entity path falls under a project root gains
  `metadata.project` + `metadata.branch`, which is what lets the timeline group by project and the analysis engine
  score `failure_proximity` for "my API" style problems.

**Non-functional**
- Docker-absent overhead: one `Path::exists()` per 60 s. Measurable cost: none.
- Development mode adds no polling beyond the Phase 12 watcher plus a 30 s `.git` metadata check per project
  (≤5 projects), each read bounded to <64 KB.

## Architecture

```
collectors/docker/
  mod.rs          collector: probe -> connect -> stream | poll fallback
  transport.rs    bollard client construction (named pipe), decided in 0011
  map.rs          pure: bollard event | container list diff -> RawEvent[]  (single mapping)
collectors/dev/
  mod.rs          collector over project roots
  git_state.rs    pure: file contents -> GitState   (no process, no git crate)
  manifests.rs    pure: manifest text -> Vec<DependencyChange>  (reuses diff/*)
  toolchain.rs    pure: (files, observed apps, observed processes) -> Vec<Toolchain>
  enrich.rs       pure: (event, project roots) -> metadata.project/branch
```

Every `pure:` module is unit-tested from fixtures, which is how the unverifiable-here Docker path still gets real
test coverage: `map.rs` is tested against **captured JSON fixtures** of Docker event/list payloads, so the mapping
is proven even though the live connection is not.

## Related Code Files

**Create:** `src-tauri/src/collectors/docker/{mod.rs,transport.rs,map.rs}`,
`src-tauri/src/collectors/dev/{mod.rs,git_state.rs,manifests.rs,toolchain.rs,enrich.rs}`,
`src-tauri/tests/{docker-absent.rs,docker-map.rs,dev-git-state.rs,dev-manifests.rs,dev-enrich.rs}`,
`src-tauri/fixtures/docker/{events-stream.json,containers-list.json,container-inspect.json}`,
`src-tauri/fixtures/git/{head-branch,head-detached,packed-refs,merge-head}`,
`src-tauri/fixtures/manifests/` (before/after pairs per ecosystem),
`docs/decisions/0011-docker-transport.md`, `src/lib/components/settings/ProjectRoots.svelte`
**Modify:** `src-tauri/Cargo.toml` (`bollard` 0.21.1 with the verified pipe feature), `src-tauri/src/collectors/mod.rs`,
`src-tauri/src/diff/mod.rs` (manifest dispatch), `docs/COLLECTORS.md`, `docs/IPC-SURFACE.md`,
`src/lib/components/timeline/FilterBar.svelte` (project + container filters), `README.md` (Docker/dev-mode status)
**Delete:** none.

## Implementation Steps

1. `collectors/docker/mod.rs` with the probe-only path **first**, and land `tests/docker-absent.rs` before writing any transport code: assert `Unavailable`, exactly one log line per transition, zero events, and no measurable cost across 10 probe cycles. This is the part that is verifiable on this machine, so it is the part that gates the phase.
2. Read the installed `bollard` 0.21.1 manifest/docs, pick the named-pipe constructor, and write `docs/decisions/0011-docker-transport.md` recording the exact function and feature flag found (plus the fallback if the pipe transport is not offered).
3. `collectors/docker/map.rs`: the single mapping function; capture fixtures from the Docker API schema shape for events, container list and inspect, then test the mapping exhaustively (started/stopped/created/removed/health/pull/remove/port-change/env-change), including a malformed payload that must be skipped rather than panic.
4. `collectors/docker/transport.rs` + the stream/poll paths, both feeding `map.rs`; connection loss → poll → probe, with the backoff owned by the Phase 10 supervisor.
5. `collectors/dev/git_state.rs`: parse the four fixture shapes (branch HEAD, detached HEAD, packed-refs-only branch, merge in progress); malformed `.git` files yield `GitState::Unknown`, never a panic and never a fabricated branch.
6. `collectors/dev/manifests.rs`: map file name → ecosystem → differ; produce `DependencyAdded/Removed/VersionChanged` with exact `old → new`; binary lockfiles produce a hash-change event only.
7. `collectors/dev/toolchain.rs`: the no-spawn inference with an explicit `unknown` variant; a test asserts that an ecosystem with no evidence returns `unknown` rather than a default version.
8. `collectors/dev/enrich.rs` + wiring: longest-prefix match of event paths against project roots, then attach `project` + `branch`; a test asserts a file under two nested roots picks the deepest.
9. `collectors/dev/mod.rs`: 30 s `.git` metadata check per root, branch/commit/merge diffing, `dirty (observed)` rule, and the manifest hookup through the Phase 12 watcher (no second watcher).
10. UI: `ProjectRoots.svelte` (add/remove project roots, shown in Settings), plus project and container filter options in the timeline filter bar and a `project` badge on enriched events.
11. `README.md` + `COLLECTORS.md` honesty rows: Docker absent-path **verified on this machine**, connected path **implemented and fixture-tested, not verified against a live daemon here**; dev-mode git reading is metadata-level and `dirty (observed)` is explained.
12. Commit: `feat(collectors): docker availability probe`, `feat(collectors): docker event mapping`, `feat(collectors): docker transport and streams`, `feat(collectors): development mode git and manifests`, `feat(ui): project roots and filters`.

## Todo

- [ ] Docker-absent path produces zero error spam, verified by test
- [ ] `bollard` pipe transport recorded from the installed crate, not assumed
- [ ] One mapping function shared by stream and poll paths, fixture-tested
- [ ] Container env values never stored; only key names + sealed value
- [ ] `container → publishes → port` written so the §74 host-port join works
- [ ] Git state read from files only — no process spawn, no git crate
- [ ] `dirty (observed)` never claims clean, and says so in the UI
- [ ] Toolchain versions are `unknown` when unproven
- [ ] Live-daemon verification gap stated in `README.md` and `COLLECTORS.md`

## Success Criteria

- [ ] `cargo test --test docker-absent --test docker-map --test dev-git-state --test dev-manifests --test dev-enrich` green
- [ ] With Docker not installed, running the app for 10 minutes yields zero Docker error entries in the log and the collector reads `Unavailable — Docker is not running` in Settings
- [ ] Editing `.env` in a project root produces an event carrying `metadata.project` and `metadata.branch`
- [ ] Switching branches in a project root (`git checkout -b tmp`) produces `Switched branch: <old> → tmp` within 30 s
- [ ] An architecture-gate test asserts no `Command::new`/`std::process` usage in `collectors/dev/`

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test docker-absent --test docker-map --test dev-git-state --test dev-manifests --test dev-enrich
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test architecture-gates
cargo clippy --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --all-targets -- -D warnings
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| The live Docker path is wrong and cannot be tested here | No daemon on this machine | Fixture-test the mapping exhaustively (done), keep the transport thin, and ship with the gap stated in `README.md`/`COLLECTORS.md`. The collector defaults to `Unavailable`, so a wrong transport degrades to "no Docker events", never to a crash or a false event |
| `bollard` 0.21.1 does not expose a named-pipe constructor under the feature name assumed | Compile error or a runtime "unsupported transport" | The feature/flag is read from the installed crate before use, and `docs/decisions/0011-docker-transport.md` records what was actually found plus the fallback (a hand-rolled pipe HTTP client is explicitly **out of scope**; the collector stays `Unavailable` and the limitation is documented) |
| The Docker events stream floods (busy compose project) | Hundreds of events per minute | Phase 08 coalescing + the Phase 13 restart coalescer apply; additionally `informational` container events are excluded from notifications entirely |
| `.git` parsing misreads a repo shape (worktrees, submodules, `gitdir:` files) | Branch shows as `Unknown` | `Unknown` is the designed outcome for unrecognized shapes, surfaced as "branch unavailable" rather than a wrong branch; `.git`-as-a-file (worktree/submodule) is handled by following the `gitdir:` pointer once, with a fixture |
| `dirty (observed)` misleads the user | Support-style confusion in dogfooding | The tooltip and the `dirty_source` metadata exist precisely for this; if it still misleads, the label is dropped entirely rather than made stronger — the app must not appear to know `git status` |
| Manifest diffing produces noise on every `bun install` | Dozens of dependency events per install | Install-burst detection: >10 dependency changes in one debounce window collapse into one `Dependencies changed (N packages)` event with the names in metadata |

## Security Considerations

Container environment variables are the single most secret-dense surface in the whole product: only key names are
persisted in cleartext, values go through `Redactor::seal()`, and the redaction-leak suite gains a Docker specimen
(`POSTGRES_PASSWORD=hunter2` in a fixture container payload) that must not appear in the DB bytes. No process is
ever spawned in this phase — not `git`, not `docker`, not `npm` — which removes command-injection and
PATH-hijack risk entirely and is enforced by an architecture-gate grep. `.git` reads are bounded (<64 KB, specific
file names) and go through the Phase 12 path validator, so a malicious repo cannot point the app at an arbitrary
file via a crafted `gitdir:`.

### FILE: phase-22-onboarding-settings-diagnostics.md

---
phase: 22
title: "Onboarding, Settings & Diagnostics"
status: pending
priority: P1
effort: 8h
dependencies: [21]
---

## Overview

Everything the user needs to consent to, control, and inspect. This phase is where the measured capability matrix
becomes user-visible: each monitoring toggle states what is collected, why, what it costs, and whether admin is
needed — using the numbers actually measured in Phases 11–21, not adjectives.

## Requirements

**Functional — onboarding (§50, five steps, <1 minute, exact copy)**
1. `What Changed — When something suddenly stops working, What Changed helps you see what changed beforehand.`
2. `Everything stays on this computer by default.`
3. `Choose monitoring` — **checked by default:** Apps & updates, Windows settings, Network, Drivers, Services.
   **Unchecked by default:** Development tools, Docker. Each row carries the same what/why/cost/admin detail as
   Settings (one shared component, DRY).
4. `Start with Windows? — Recommended` (a single toggle, default on, writing the Phase 05 per-user Run entry; if
   the write fails it says so instead of silently pretending).
5. `Monitoring started.` plus a short animated example timeline (a canned 6-event sequence rendered with the real
   `TimelineRow` component and `metadata.synthetic = true`, so the animation is honest sample data and cannot be
   confused with the user's history; it is cleared when onboarding closes).
- Reachable again from Settings → About → `Show setup again`; the completion flag lives in settings, so a reinstall
  over an existing profile does not re-onboard (§64 upgrade behaviour).
- Skippable at any step (`Skip setup` keeps defaults); `Esc` does not lose the choices already made; the whole flow
  is keyboard-operable and the step count is announced (`Step 3 of 5`).
- No step blocks on a network call, a permission prompt, or an elevation prompt — the app is non-elevated by design.

**Functional — Settings (§49, all eight sections)**
- `General`: start with Windows, start minimized to tray, close-to-tray vs. exit, language placeholder (English
  only, stated), recording paused/active master switch.
- `Monitoring`: all ten §49 toggles — Apps & updates, Processes, Network, Services, Drivers, Windows updates,
  Startup apps, File/config monitoring, Docker, Development projects — each rendered by
  `MonitoringToggle.svelte` with four columns from a typed table: **what is collected**, **why it helps**,
  **estimated cost** (from `docs/perf-baseline.md`: e.g. Processes `~6 ms per second`, Drivers `~2.1 s hourly`,
  Services `~3 ms per 5 s + ~0.4 s every 15 min`, Network `~0.2 ms per 30 s`, Windows updates `~40 ms per 30 min`),
  and **admin needed** (No for everything shipped; the Security event-log channel row is present, disabled, and
  labelled `Requires administrator`). Sub-panels: watched locations (Phase 12) and project roots (Phase 21).
- `Privacy`: the §33 never-collected list rendered from `privacy/policy.rs` (single source of truth, exposed via a
  command so the doc and the UI cannot drift), redaction on/off is **not offered** (it is structural), cloud AI
  shown as `Not available in this version`, `Delete all history` (typed confirmation), `Delete events older than…`,
  and `Export my data` (redacted JSON).
- `Storage`: database size broken out (events / snapshots / WAL / backups), event count, retention horizons
  (7/30/90/custom for events, snapshots, important events), `Run cleanup now`, `Vacuum now` with the expected
  duration stated, and the estimated growth per day from the last 7 days of real data.
- `Appearance`: theme (system/light/dark), density (comfortable/compact), `graphicsMode` (Full/Reduced/Off from
  Phase 19), reduced-motion override (`Follow system` / `Always reduce`).
- `Advanced`: log level, open log folder, open data folder, `collector_status` table view with enable/disable per
  collector, notification threshold (Phase 16), and the dev-mode flag that reveals §85's simulation panel in a
  release build (default off, explicitly labelled as a developer feature).
- `Integrations`: Docker (state + reason, from the Phase 21 probe), development projects, and a clear
  `Not installed` / `Not running` / `Connected` state — no fake "coming soon" rows.
- `About`: app version, WebView2 version, Tauri version, build date, `Check for updates` (Phase 24's updater, never
  silent), links to `PRIVACY.md`/`SECURITY.md`, and `Show setup again`.
- Every setting writes through one `settings` command surface with typed keys and validation; a change takes effect
  without restart (the collectors' settings watch channel from Phase 10) or states plainly that a restart is needed.

**Functional — Diagnostics (§84)**
- Shows: app version, database size, event count, per-collector health (state, last run, last duration, events last
  run, consecutive errors, last error redacted), last collector error overall, WebView2 version, Rust backend
  health (uptime, ingest queue depth, dropped events, batch stats), monitoring status, frame stats from Phase 19,
  and the notification suppression counters from Phase 16.
- `Export diagnostics` → a `.json` (plus a human-readable `.txt` twin) written via the Tauri save dialog, containing
  the above plus the last 200 log lines — **passed through the redactor before writing**, with paths reduced to
  their last two segments and the machine/user name replaced by `<user>`. A test asserts the export of a seeded
  system containing every secret specimen is specimen-free.

**Non-functional**
- Settings open → painted <150 ms (all values from one `get_settings_bundle` call plus one `get_storage_stats`).
- The monitoring table's cost column is generated from a single typed constant shared with `docs/COLLECTORS.md`, so a
  cadence change updates both or fails a test.

## Architecture

`src-tauri/src/{settings/mod.rs,settings/schema.rs,commands/settings.rs,commands/diagnostics.rs,privacy/export.rs}`;
the settings store is a typed struct persisted as one JSON row (validated on read, defaults filled, unknown keys
preserved) with a `tokio::sync::watch` broadcast so collectors react without a restart.
Frontend: `src/lib/state/settings.svelte.ts`, `src/lib/components/onboarding/*`,
`src/lib/components/settings/{SettingsView,SectionNav,MonitoringToggle,ToggleDetail,StorageSection,PrivacySection,DiagnosticsSection,...}.svelte`.
`src/lib/data/collector-catalog.ts` is generated from the Rust catalog at build time (a small `bun` script reading a
JSON emitted by a Rust test) so the what/why/cost/admin table has exactly one source.

## Related Code Files

**Create:** `src-tauri/src/settings/{mod.rs,schema.rs,defaults.rs}`,
`src-tauri/src/commands/{settings.rs,diagnostics.rs}`, `src-tauri/src/privacy/export.rs`,
`src-tauri/src/domain/collector_catalog.rs`, `src-tauri/migrations/0010_settings.sql`,
`src/lib/state/settings.svelte.ts`, `src/lib/data/collector-catalog.ts` (generated),
`src/lib/components/onboarding/{OnboardingFlow,StepIntro,StepPrivacy,StepMonitoring,StepStartup,StepDone,ExampleTimeline}.svelte`,
`src/lib/components/settings/{SettingsView,SectionNav,MonitoringToggle,ToggleDetail,GeneralSection,MonitoringSection,PrivacySection,StorageSection,AppearanceSection,AdvancedSection,IntegrationsSection,AboutSection,DiagnosticsSection}.svelte`,
`scripts/gen-collector-catalog.ts`, `src-tauri/tests/{settings.rs,diagnostics-export.rs,collector-catalog.rs}`,
`tests/e2e/{onboarding.spec.ts,settings.spec.ts}`
**Modify:** `src/routes.ts`, `src-tauri/src/collectors/context.rs` (watch channel), `docs/COLLECTORS.md`
(generated cost column), `docs/IPC-SURFACE.md`, `PRIVACY.md`, `STUBS.md`, `package.json` (the catalog script in `prebuild`)
**Delete:** the Phase 06 settings/onboarding placeholder bodies.

## Implementation Steps

1. `settings/schema.rs` + `defaults.rs`: the typed settings struct with `#[serde(default)]` per field, a version
   field, and a forward-compatible read (unknown keys retained) so a downgrade cannot destroy a newer profile.
2. `commands/settings.rs`: `get_settings_bundle`, `set_setting(key, value)` (typed enum key, validated value),
   `reset_section(section)`; every write broadcasts on the watch channel.
3. `domain/collector_catalog.rs`: the single catalog — per collector: id, display name, what, why, measured cost
   string, admin required, notify-eligible, default-enabled. `tests/collector-catalog.rs` asserts every registered
   collector has a catalog entry and vice versa (no orphan rows, no undocumented collector).
4. `scripts/gen-collector-catalog.ts` + a Rust test that dumps the catalog to JSON; the script converts it into
   `src/lib/data/collector-catalog.ts` and `docs/COLLECTORS.md`'s cost column. `bun run gate` fails if the generated
   files are stale (regenerate + `git diff --exit-code`).
5. Onboarding components with the exact §50 strings in `deck.ts` (extending Phase 18's deck), the default check
   states from step 3, the startup toggle writing through Phase 05's helper, and `ExampleTimeline.svelte` playing the
   canned synthetic sequence.
6. Settings shell (`SectionNav` + eight sections), `MonitoringToggle` + `ToggleDetail` rendering the four columns
   from the generated catalog.
7. `PrivacySection`: the never-collected list fetched from `get_privacy_policy` (backed by `privacy/policy.rs`),
   `Delete all history` (typed `DELETE` confirmation, runs a transaction + `VACUUM`, then reports the reclaimed
   bytes), `Delete events older than…`, and `Export my data`.
8. `StorageSection`: the broken-out storage stats, retention controls, cleanup/vacuum actions with a progress state
   and the measured expected duration.
9. `commands/diagnostics.rs` + `privacy/export.rs`: the diagnostics payload, the redacted export writer, and the
   log-tail reader (last 200 lines, bounded to 256 KB).
10. `tests/diagnostics-export.rs`: seed every specimen across events, settings, log lines, and a collector error,
    export, then byte-scan the exported files for all 12 specimens **and** for the user name and full home path.
11. `tests/settings.rs`: round-trip, validation rejection of out-of-range values, unknown-key preservation, and that
    disabling a collector via settings actually stops it (asserted against the Phase 10 supervisor).
12. `tests/e2e/onboarding.spec.ts`: the five steps with byte-exact §50 strings, the default check states, skip path,
    keyboard-only completion, and that completing it does not re-appear on the next launch.
13. `tests/e2e/settings.spec.ts`: all eight sections reachable, every monitoring row shows four populated columns,
    the Security-channel row is disabled with the admin label, toggling a collector reflects in Diagnostics, and
    `Delete all history` requires the typed confirmation.
14. Commit: `feat(settings): typed settings with live propagation`, `feat(ui): onboarding flow`,
    `feat(ui): settings sections`, `feat(diagnostics): diagnostics view and redacted export`.

## Todo

- [ ] All five §50 steps with exact copy and the specified default check states
- [ ] Ten §49 monitoring toggles, each with what/why/cost/admin from the generated catalog
- [ ] All eight §49 sections present, no placeholder rows
- [ ] Never-collected list rendered from `privacy/policy.rs`, not duplicated in the UI
- [ ] Diagnostics shows every §84 field
- [ ] Export is redacted and byte-scanned in a test, including user name and home path
- [ ] Settings changes take effect without a restart, or say a restart is needed
- [ ] Catalog generation is checked for staleness in the gate

## Success Criteria

- [ ] `cargo test --test settings --test diagnostics-export --test collector-catalog` green
- [ ] `bun run test:e2e -- tests/e2e/onboarding.spec.ts tests/e2e/settings.spec.ts` green with byte-exact §50 strings
- [ ] Toggling Processes off in Settings stops the process collector within one interval, visible in Diagnostics
- [ ] The exported diagnostics bundle contains no specimen, no user name, and no full home path
- [ ] Settings paints in <150 ms (measured in the e2e spec)
- [ ] `STUBS.md` is empty or contains only items explicitly deferred by the user (§26, §75–§78)

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test settings --test diagnostics-export --test collector-catalog
cd "D:/Project/.WhatChage" && bun run gen:catalog && git -C "D:/Project/.WhatChage" diff --exit-code -- src/lib/data/collector-catalog.ts docs/COLLECTORS.md
cd "D:/Project/.WhatChage" && bun run test:e2e -- tests/e2e/onboarding.spec.ts tests/e2e/settings.spec.ts
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| The cost column drifts from reality after a cadence change | Stale-catalog check fails in the gate | That is the intended failure: regenerate the catalog and re-measure the number before shipping the cadence change; the column may never contain a guessed figure |
| Settings sprawl makes the app feel complicated (§51 "simple by default") | Dogfooding: the Monitoring section is a wall of text | The four columns live behind a per-row `Details` disclosure; the collapsed row shows only name, state, and a one-line "what" — progressive disclosure per §51 |
| A settings write corrupts the profile mid-write | App starts with defaults after a crash | Settings are written atomically (temp file + rename inside the data dir, or a single-row transactional update in SQLite — the SQLite path is chosen since the DB is already transactional) and validated on read with defaults filled |
| `Delete all history` deletes more than intended | User loses checkpoints unexpectedly | The confirmation names exactly what is removed (events, snapshots, checkpoints, investigations) and offers `Delete events only` as a separate action; a test asserts the "events only" path preserves checkpoints |
| The export leaks something the redactor does not model (machine name, SID) | Manual review of the export | The export applies an additional post-pass replacing the resolved user name, home directory, and computer name with placeholders; the test asserts all three, so a redactor gap is caught at the boundary that leaves the machine |
| Onboarding's startup toggle silently fails (per-user Run key blocked by policy) | Startup does not happen | The write result is checked and step 4 reports `Couldn't enable automatic start on this PC.` with a retry — never a silent lie (§87) |

## Security Considerations

The diagnostics export is the only file the app writes outside its own data directory, so it goes through the
redactor plus an identity post-pass and is written only to a user-chosen path via the OS save dialog (no silent
writes, no default location). The privacy policy list is served from the same Rust module the redactor enforces, so
the UI cannot promise something the code does not do. Enabling the §85 dev panel in a release build is an explicit,
default-off, clearly-labelled developer setting, and the dev commands remain compile-time excluded from release
builds — the flag reveals UI, it cannot resurrect absent commands.

### FILE: phase-23-states-accessibility-motion-audit.md

---
phase: 23
title: "Six-State, Accessibility & Motion Audit"
status: pending
priority: P1
effort: 8h
dependencies: [22]
---

## Overview

§70 requires **all six states on every major screen** — loading, empty, error, success, large-data, small-window —
with real microcopy. There are nine major screens, so this phase is a 54-cell matrix that gets filled in and then
locked by tests, plus the §62 accessibility pass, the §71 900×600 pass, the §42 motion pass, and the §63 offline
check. It is deliberately a separate phase: states are what separate a demo from a product, and they never get done
if they are a footnote in nine other phases.

## Requirements

**Functional — the 54-cell matrix**
- Screens (9): Home, Timeline, Investigation, Checkpoints, Checkpoint comparison, System Map, Settings,
  Diagnostics, Onboarding.
- States (6) per screen with a concrete definition each:
  - **loading** — skeletons matching the final layout's geometry (no spinner-only screens, no layout shift on
    resolve); shown only after 200 ms.
  - **empty** — real microcopy naming what will appear and one action (e.g. Timeline: `No changes recorded in this range.` + `Widen to last 7 days`; Checkpoints: `No checkpoints yet.` + `Create your first checkpoint`).
  - **error** — a human sentence, a retry, and a statement of what is unaffected (e.g. `Couldn't load the timeline. Your history is safe.` + `Try again`); never a raw error string, never a dead end.
  - **success** — the normal populated state, with the §72/§45 copy already asserted in Phase 18.
  - **large-data** — 100k events / 300 map nodes / 200 checkpoints / 50 watched locations: virtualized or bounded,
    with the bound stated on screen where one applies.
  - **small-window** — 900 × 600 (§71): no clipped content, no horizontal scroll, nav collapses to icons, drawers
    become full-height sheets, the 3D canvas shrinks rather than pushing content.
- Each cell is reachable deterministically in tests via a dev-only state harness (`?__state=timeline:error`, honored
  only under `import.meta.env.DEV`) that stubs the relevant command — so a state is *proven*, not asserted by eye.
- `docs/ui-states.md` holds the 9 × 6 table with the exact microcopy per cell; it is the review artifact.

**Functional — accessibility (§62)**
- Keyboard: every interactive element reachable in visual order; no keyboard trap; `Esc` closes every overlay;
  focus returns to the invoking element on close; skip-to-content link; all custom controls (drawer, tabs, chips,
  dialogs, section nav) implement the expected roles and key handling (copied by hand from the documented patterns,
  no `bits-ui` dependency per §80/§4).
- Focus indicators: a visible 2 px focus ring using the `border-active` token on every focusable element, never
  `outline: none` without a replacement — asserted by a stylesheet grep test.
- Semantics: one `h1` per screen, headings in order, landmarks (`banner`/`main`/`nav`/`complementary`), lists as
  lists, tables as tables with headers, `aria-live="polite"` on the live-event pill and the analysis status, all
  icon-only buttons labelled, all 3D canvases `aria-hidden` with a DOM equivalent (§62's explicit rule, satisfied by
  Phase 20's ladder and entity table).
- Contrast: every text/background pair ≥4.5:1 (≥3:1 for ≥18.66 px bold), including over the ambient field at
  maximum density and including all four importance stripe colors on all three themes — computed in a unit test from
  the token values, not eyeballed.
- Scalable text: the layout survives 200% browser zoom and a 20 px root font size without clipping.
- No color-only signalling: importance is color + label + icon shape; confidence is style + label.

**Functional — motion (§42)**
- One motion module owns durations/easings from the tokens: panel transitions (spring-like, 180–260 ms), timeline
  insertion, node focus, number interpolation on the Today counters, subtle hover feedback. Nothing animates
  `width`/`height`/`top`/`left`; transforms and opacity only.
- `prefers-reduced-motion: reduce` (or the Settings override) → all transitions ≤1 frame or removed, number
  interpolation becomes an instant set, the ambient field renders one static frame, the map pulse is skipped. A test
  runs the full e2e suite under `reducedMotion: 'reduce'` and asserts no animation API is called (a dev-only
  counter incremented by the motion module).

**Functional — offline (§63)**
- A test asserts the built `dist/` references no absolute `http(s)://` URL for a script, style, font, or image, that
  no `@font-face` points outside the bundle, and that the app renders fully with the network blocked in Playwright
  (`route.abort()` on all external requests).

**Functional — cleanup**
- `STUBS.md` must end this phase either empty or containing only the user's explicit deferrals (§26 health checks,
  §75–§78 future features), each with a link to `ROADMAP.md`. Any other TODO/`unimplemented!()`/placeholder is either
  finished here or converted into a real, linked roadmap item — a stub may never masquerade as done (§87).

**Non-functional**
- The audit adds no production dependency: contrast math, the state harness, and the motion counter are ~150 lines of
  first-party code. Playwright and `axe-core` (dev-only) are the only test-side additions, and `axe-core` is used as a
  *second* opinion — the hand-written assertions are the primary gate, so a version bump cannot silently relax it.

## Architecture

`src/lib/a11y/{contrast.ts,focus-trap.ts,live-region.ts}`, `src/lib/motion/{tokens.ts,transitions.ts,counter.ts}`,
`src/lib/dev/state-harness.ts` (DEV-only command stubbing keyed by the `__state` query param),
`tests/e2e/states.spec.ts` (the 54-cell driver, table-driven from one array so a new screen cannot be forgotten),
`tests/e2e/a11y.spec.ts`, `tests/e2e/offline.spec.ts`, and `src/lib/design/contrast.test.ts`.

## Related Code Files

**Create:** `src/lib/a11y/{contrast.ts,focus-trap.ts,live-region.ts}`,
`src/lib/motion/{tokens.ts,transitions.ts,counter.ts}`, `src/lib/dev/state-harness.ts`,
`src/lib/components/states/{LoadingSkeleton,EmptyState,ErrorState}.svelte`,
`src/lib/design/contrast.test.ts`, `src/lib/motion/counter.test.ts`,
`tests/e2e/{states.spec.ts,a11y.spec.ts,offline.spec.ts,small-window.spec.ts}`, `docs/ui-states.md`
**Modify:** all nine screen components (wire the three state components + the harness), `src/app.css`
(focus-ring rule, motion tokens), `src/lib/components/**` (labels, roles, headings as the audit finds them),
`STUBS.md`, `ROADMAP.md`, `docs/DESIGN.md`, `package.json` (`axe-core` dev dependency, `test:a11y` script)
**Delete:** any leftover placeholder component from Phase 06 that is still referenced.

## Implementation Steps

1. Build the three shared state components (`LoadingSkeleton` with per-screen geometry variants, `EmptyState`,
   `ErrorState`) so all nine screens use one implementation (DRY) and the microcopy lives in `deck.ts`.
2. Write `docs/ui-states.md` with the 9 × 6 table and the exact copy per cell **first** — the table is the spec the
   code and the tests are written against.
3. Wire the state harness: each screen's store accepts an injected command adapter; under DEV the harness swaps in a
   stub that returns pending/empty/error/large fixtures per the `__state` param.
4. `tests/e2e/states.spec.ts`: a table-driven loop over `[screen] × [state]` asserting per cell — no console error, a
   visible heading, the expected microcopy, no horizontal scroll, and (for loading) that the skeleton's bounding box
   is within 10% of the resolved layout's. The array is the source of truth: adding a screen without adding six cells
   fails the test by construction.
5. `contrast.ts` + `contrast.test.ts`: WCAG relative-luminance math over the resolved token values for all three
   themes, all text roles, all four importance colors, plus the ambient-field composite (token color over the field's
   maximum-density blend) — every pair asserted against its threshold.
6. `focus-trap.ts` + `live-region.ts` and the per-component pass: roles, labels, heading order, `Esc` handling, focus
   restoration. Fix as found; each fix gets a spec assertion so it cannot regress.
7. `tests/e2e/a11y.spec.ts`: hand-written keyboard walks per screen (tab order snapshot, no trap, `Esc` closes,
   focus returns), then `axe-core` as the secondary scan with zero serious/critical violations allowed.
8. `motion/*`: centralize durations/easings, replace ad-hoc transitions, add the number-interpolation counter, and
   add the DEV-only animation counter; run the full e2e suite a second time under `reducedMotion: 'reduce'`
   asserting the counter stays at 0.
9. `tests/e2e/small-window.spec.ts`: 900 × 600 viewport across all nine screens — no clipping, no horizontal scroll,
   nav collapsed, drawer as a sheet, canvas shrunk; plus 1200 and 1600 px sanity checks (§71 main target).
10. `tests/e2e/offline.spec.ts`: block all external routes, assert every screen renders; plus a build-output scan for
    external URLs and `@font-face` sources.
11. Cleanup sweep: `rg -n "TODO|FIXME|unimplemented!|todo!" src src-tauri/src` → each hit is finished, or moved to
    `ROADMAP.md` and listed in `STUBS.md` with a link. Update `STUBS.md` to its final state.
12. Commit: `feat(ui): shared loading, empty and error states`, `fix(a11y): keyboard, roles and contrast pass`,
    `feat(ui): centralized motion with reduced-motion support`, `test: 54-cell state matrix and a11y suites`.

## Todo

- [ ] `docs/ui-states.md` 9 × 6 table complete with real microcopy
- [ ] All 54 cells reachable via the harness and asserted in `states.spec.ts`
- [ ] Contrast computed in a test for all themes, roles, importance colors, and over the ambient field
- [ ] Keyboard walk per screen: no trap, `Esc` closes, focus returns
- [ ] Every 3D canvas `aria-hidden` with a DOM equivalent
- [ ] Motion centralized; reduced-motion asserted by a zero-animation counter
- [ ] 900 × 600 clean on all nine screens
- [ ] Offline render proven and the build output free of external URLs
- [ ] `STUBS.md` empty or only user-deferred items, each linked to `ROADMAP.md`

## Success Criteria

- [ ] `bun run test:e2e` green including `states`, `a11y`, `small-window`, and `offline` specs (54 state assertions
      present and passing)
- [ ] `bun run test:unit` green including the contrast and counter tests
- [ ] `axe-core` reports zero serious/critical violations on all nine screens
- [ ] The full e2e suite passes a second time under `reducedMotion: 'reduce'` with the animation counter at 0
- [ ] `rg "TODO|FIXME|unimplemented!|todo!" src src-tauri/src` returns only items listed in `STUBS.md`

## Verification Gate

```bash
cd "D:/Project/.WhatChage" && bun run check && bun run test:unit
cd "D:/Project/.WhatChage" && bun run test:e2e
cd "D:/Project/.WhatChage" && bun run test:a11y
cd "D:/Project/.WhatChage" && bun run gate
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| The 54-cell matrix balloons past the 8 h budget | Half the cells done at the budget | Order the work by screen importance (Home, Timeline, Investigation first) and by state risk (error and small-window are the ones that ship broken); a screen is never marked done with fewer than six cells, so the overflow becomes a visible unfinished screen rather than a silently thin audit |
| Contrast failures force a palette change late | Multiple pairs fail at 4.5:1 | The tokens are CSS variables from Phase 06, so a failing pair is a one-line token change; the test names the exact pair and the achieved ratio, and the ambient field's opacity is the first lever (readability first per §43) |
| `axe-core` version churn breaks the gate | New violations after a bump | `axe-core` is pinned exactly and is the *secondary* check; the hand-written assertions are the contract, so a bump can be triaged without unblocking the build on a real regression |
| The state harness leaks into production | A `__state` param works in a release build | The harness is behind `import.meta.env.DEV` and is asserted absent by a built-bundle grep in Phase 24 |
| Reduced-motion still animates via CSS | The counter is 0 but motion is visible | The audit covers CSS too: a global `@media (prefers-reduced-motion: reduce)` block disables transitions/animations, and the spec takes two screenshots 400 ms apart on each screen and asserts they are identical |
| Fixing a11y issues changes layout and breaks earlier specs | Phase 09/18 specs fail | Expected and welcome — those specs are the regression net; they are updated in the same commit as the fix, never disabled |

## Security Considerations

The state harness is the only new surface here and it is DEV-gated and asserted absent from the release bundle. The
error state's copy rule ("a human sentence, never a raw error string") is also a security property: raw Win32/SQLite
error strings can contain paths and user names, so the UI surfaces a classified message while the redacted detail
goes only to the log and to Diagnostics. The offline test doubles as an exfiltration check — a build that renders
fully with all external routes blocked cannot be phoning home (§63, §82).

### FILE: phase-24-release-performance-security-audit.md

---
phase: 24
title: "Release: Performance, Security & Update Audit"
status: pending
priority: P1
effort: 9h
dependencies: [23]
---

## Overview

The phase that decides whether 0.1.0 ships honestly. Three things are settled here with numbers, not adjectives:
the §37 idle-CPU and RAM budgets (including blocker B2's <80 MB question), the §82 eleven-row threat model verified
row by row, and the §64/§65/§66 update-installer-signing path proven to preserve the user's database.

## Requirements

**Functional — performance verification (§37, §61, blocker B2)**
- Idle CPU: sample the app process **and every `msedgewebview2` child** for 10 minutes with all default collectors
  running, window hidden to tray, via `scripts/measure-idle.ps1` (`Get-Counter` on `% Processor Time` normalized by
  core count, 1 s interval). Target 0–1% of total CPU. Record mean, p95, and max.
- Memory (B2): re-run `scripts/measure-memory.ps1` from Phase 02 against the **full release build** after 30 minutes
  of live recording — sum `WorkingSet64` and `PrivateMemorySize64` across the app process and all WebView2 children,
  and record both numbers plus the per-process split.
- **Pre-decided B2 escalation (this is the honest path, not a fudge):** if the total working set exceeds 80 MB,
  report both figures in `docs/release-audit-0.1.0.md`, note that the WebView2 host is the dominant term and is not
  under the app's control, and raise a three-option decision to the user: (a) accept a revised budget stated as
  "app process <80 MB, total including WebView2 <N MB", (b) spend a follow-up phase on reduction levers (drop the
  3D layer to `Reduced` by default, shrink the timeline page size, trim the WebView2 startup flags), or (c) keep 80 MB
  as a hard target and treat the release as blocked. The budget is never silently redefined and the number is never
  omitted.
- Also re-measure and record: startup to first paint, DB size after 30 minutes, ingest counters, analysis latency at
  the current DB size, and the 3D frame time — all appended to `docs/perf-baseline.md` with the date and the build.
- Bundle checks: the built `dist/` has no external URL, the release binary contains no dev-only symbols
  (`dev_inject_scenario`, `dev_flood_events`, `dev_clear_synthetic`, `__WC_SCENE_STATS__`, `__state` harness), and
  the `three` chunk is separate from the app shell.

**Functional — §82 threat model, eleven rows, each with a verification artifact**
1. *Malicious frontend script* — CSP asserted (`script-src 'self'`, no `unsafe-eval`/`unsafe-inline`), `{@html}` gate,
   capability allowlist reviewed: every permission in `capabilities/*.json` is justified in `SECURITY.md` or removed.
2. *Malicious config file* — the Phase 12 differ fuzzed with 200 hostile inputs (deep nesting, 1 MB single line, NUL
   bytes, invalid UTF-8, YAML bombs) asserting no panic, no unbounded allocation, and the size cap holding.
3. *Symlink / path traversal* — the Phase 12 refusal matrix re-run as part of the release suite.
4. *Secret leakage* — the full 12-specimen × 5-placement suite plus the DB/WAL/log/export byte scans.
5. *Database corruption* — a deliberately truncated and a bit-flipped DB file each produce the §83 recovery path
   (`Repair database` / `Export diagnostics` / `Reset local history`) with **no crash loop**, verified by launching
   twice against the damaged file.
6. *Untrusted Docker metadata* — the Phase 21 fixtures include a container name with control characters, a 1 MB label,
   and a malformed JSON payload; assert skip-not-panic and that no label is rendered as HTML.
7. *Malformed Event Log entries* — Phase 14's malformed-XML fixtures plus an oversized record, asserting skip + count.
8. *Large event floods* — the Phase 08 flood test re-run at release scale (50k) asserting bounded memory, bounded WAL,
   accurate drop counters, and a responsive UI.
9. *IPC abuse* — a test calls **every** registered command with hostile arguments (oversized strings, negative and
   `i64::MAX` numbers, invalid enum values, path-like strings, SQL fragments) asserting a typed error and no panic;
   the test enumerates the command list from the generated bindings so a new command cannot skip it.
10. *Update tampering* — updater signature verification enabled and asserted: an artifact signed with the wrong key is
    rejected; `dialog: true`/no silent install verified (§64).
11. *Local privilege escalation* — assert the app never requests elevation (no `requireAdministrator` manifest, no
    `runas`, no `ShellExecute` elevation), never writes outside its per-user data dir and the user-chosen export path,
    and installs per-user (NSIS `perUser`) so no writable-directory-in-`Program Files` hijack surface exists.

**Functional — update, installer, signing (§64, §65, §66)**
- Build the NSIS per-user installer; verify Start Menu shortcut, optional desktop shortcut, optional launch-at-startup,
  and clean uninstall.
- **Database preservation test (the one that matters):** install 0.1.0, generate ≥500 real events and a checkpoint,
  install a 0.1.1 build over it, then assert event count, checkpoint, and settings survive and the migration ran
  forward. Then downgrade-attempt 0.1.0 over 0.1.1 and assert the app refuses cleanly with `This database was created
  by a newer version.` rather than corrupting anything.
- Uninstall asks whether history should also be removed **where technically feasible** (§65): implement the NSIS
  uninstall prompt if the hook works in the per-user bundle; if it does not, ship the in-app `Delete all history`
  action as the documented alternative and state the limitation in `README.md` — no silent data retention and no
  silent deletion.
- `Check for updates` is manual only, shows the version and release notes, and never installs without an explicit
  click (§64). Auto-download is absent, not stubbed.
- `docs/RELEASING.md` documents production signing end to end with **environment-variable-only** secret handling
  (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` and the Windows certificate variables), an
  explicit "never commit a key, never paste a key into a config file" rule, a `.gitignore` entry for key material, and
  a pre-release checklist. Development signing (unsigned/self-signed local builds) is documented separately from
  production signing. No key material, thumbprint, or password appears anywhere in the repository.

**Functional — architecture gates, final run**
- One `tests/architecture-gates.rs` run must enforce every structural promise made across the plan, each as a named
  assertion: no `unwrap()`/`expect()`/`panic!()` outside tests and `main`; no `unsafe` outside `platform/win/`; the
  repository accepts only `SealedEvent`; no `tauri-plugin-sql`; no raw SQL string reaching a command; no
  `QuickFixEngineering`/`wmic`; no `"Security"` event-log channel literal; no `std::process`/`Command::new` anywhere in
  `src-tauri/src`; no network crate in `analysis/`; no `{@html}` in `src/`; no `Arguments` read in the task-scheduler
  module; dev commands absent from a release build; the collector catalog and `docs/COLLECTORS.md` in sync.
- `docs/release-audit-0.1.0.md` is the deliverable: every performance number measured, every threat row with its
  verification artifact and verdict, every gate with pass/fail, the B2 outcome, the live-Docker verification gap, and
  an explicit list of what was **not** verified on this machine.

**Non-functional**
- The whole release suite must be runnable by one command (`bun run release:audit`) so it is repeatable for 0.1.1.

## Architecture

`scripts/{measure-idle.ps1,measure-memory.ps1,release-audit.ts,check-bundle.ts}` orchestrate the audit;
`src-tauri/tests/{architecture-gates.rs,threat-model.rs,ipc-fuzz.rs,corruption-recovery.rs,upgrade-migration.rs}`
hold the enforceable parts. `scripts/release-audit.ts` runs everything in order, collects outputs, and renders
`docs/release-audit-0.1.0.md` from a template — the audit document is generated from real command output, not typed by
hand, so it cannot claim a passing check that did not run.

## Related Code Files

**Create:** `scripts/{measure-idle.ps1,release-audit.ts,check-bundle.ts}`,
`src-tauri/tests/{threat-model.rs,ipc-fuzz.rs,corruption-recovery.rs,upgrade-migration.rs}`,
`src-tauri/fixtures/hostile/{configs/,docker/,eventlog/}`, `docs/RELEASING.md`,
`docs/release-audit-0.1.0.md` (generated), `SECURITY.md` (threat-model table)
**Modify:** `src-tauri/tests/architecture-gates.rs` (final assertion set), `src-tauri/tauri.conf.json` (updater
config, NSIS options, version 0.1.0), `package.json` (`release:audit`, `check:bundle`), `README.md` (status,
verification gaps), `PRIVACY.md`, `docs/perf-baseline.md`, `ROADMAP.md`
**Delete:** none.

## Implementation Steps

1. Finish `architecture-gates.rs` as the single structural test with one named assertion per promise above; run it and
   fix whatever it catches (this is where accumulated drift surfaces).
2. `scripts/measure-idle.ps1` + the 10-minute idle run; record mean/p95/max.
3. Re-run `scripts/measure-memory.ps1` against the release build after 30 minutes of recording; write both numbers and
   the per-process split; if >80 MB, prepare the three-option escalation text in `docs/release-audit-0.1.0.md` and
   surface it to the user before declaring the phase done.
4. `tests/threat-model.rs`: rows 2, 6, 7 (hostile fixture fuzzing across config, Docker, Event Log) with 200 inputs
   each and an allocation ceiling per input.
5. `tests/ipc-fuzz.rs`: enumerate commands from the generated TS bindings (parse the generated file at test time so it
   cannot drift), invoke each with the hostile argument matrix, assert typed errors and zero panics.
6. `tests/corruption-recovery.rs`: truncated file, bit-flipped page, and a stale `-wal` with a missing DB — each must
   yield the §83 recovery UI path and survive two consecutive launches (no crash loop).
7. `tests/upgrade-migration.rs`: build the 0.1.0 DB, apply the 0.1.1 migration set, assert data preserved and schema
   advanced; then assert a newer-schema DB is refused with the exact user-facing message.
8. Updater: enable signature verification in `tauri.conf.json`, generate a **local development** key pair (never
   committed; path documented in `docs/RELEASING.md`), sign a 0.1.1 artifact, verify `Check for updates` finds it,
   requires a click, and rejects a wrong-key artifact.
9. Installer run-through: build NSIS per-user, install, verify shortcuts/startup/uninstall, execute the
   database-preservation test end to end, and test the uninstall history prompt (implement or document the fallback).
10. `scripts/check-bundle.ts`: external-URL scan, dev-symbol scan of the release binary and bundle, chunk-split check.
11. `docs/RELEASING.md` + `SECURITY.md`: signing (env-var only), the release checklist, and the eleven-row threat table
    with verification artifacts.
12. `scripts/release-audit.ts`: run everything, render `docs/release-audit-0.1.0.md`, exit non-zero on any failure.
13. Commit: `test: architecture gates and threat model suite`, `feat(release): updater and installer configuration`,
    `docs: releasing, security threat model and release audit`.

## Todo

- [ ] Idle CPU measured over 10 minutes including WebView2 children; numbers recorded
- [ ] B2 memory re-measured on the release build; both figures recorded; escalation raised if >80 MB
- [ ] All eleven §82 rows verified with a named artifact each
- [ ] Corruption recovery proven with no crash loop across two launches
- [ ] Upgrade preserves events, checkpoints and settings; downgrade refuses cleanly
- [ ] `Check for updates` is manual, signature-verified, and rejects a wrong-key artifact
- [ ] Uninstall history prompt implemented or its limitation documented
- [ ] Signing documented with env-var-only secrets; no key material in the repo
- [ ] Every architecture gate passing
- [ ] `docs/release-audit-0.1.0.md` generated from real output, including the not-verified list

## Success Criteria

- [ ] `bun run release:audit` exits 0 (or exits non-zero **only** on the B2 budget row, with the escalation text
      present and the user's decision pending — no other failure may be carried)
- [ ] `cargo test --test architecture-gates --test threat-model --test ipc-fuzz --test corruption-recovery --test upgrade-migration` green
- [ ] Idle CPU within 0–1% of total CPU (recorded mean and p95)
- [ ] A signed 0.1.1 update is offered, requires an explicit click, and a wrong-key artifact is rejected
- [ ] After an over-install, the event count and checkpoints are unchanged
- [ ] `docs/release-audit-0.1.0.md` lists the live-Docker gap and every other unverified item explicitly

## Verification Gate

```bash
cargo test --manifest-path "D:/Project/.WhatChage/src-tauri/Cargo.toml" --test architecture-gates --test threat-model --test ipc-fuzz --test corruption-recovery --test upgrade-migration
cd "D:/Project/.WhatChage" && bun run release:audit
cd "D:/Project/.WhatChage" && bun run tauri build
powershell -NoProfile -ExecutionPolicy Bypass -File "D:/Project/.WhatChage/scripts/measure-idle.ps1" -Minutes 10
powershell -NoProfile -ExecutionPolicy Bypass -File "D:/Project/.WhatChage/scripts/measure-memory.ps1"
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Total memory exceeds 80 MB because of WebView2 (blocker B2) | `measure-memory.ps1` reports >80 MB total | Report both numbers, name WebView2 as the dominant term, and escalate the three options above. Never redefine the budget silently, never omit the measurement, never claim the target was met |
| Idle CPU exceeds 1% | `measure-idle.ps1` p95 >1% | Identify the collector by its `last_duration_ms` and cadence, then apply the pre-decided cadence relaxations (process 1 s → 2 s, services 5 s → 10 s), re-measure, and record every change in `COLLECTORS.md` and the audit |
| The updater's signing configuration cannot be completed without a production certificate | Signing step blocked | Development signing proves the mechanism; production signing is **documented** (§66 asks for documentation, not a purchased certificate) and the audit states plainly that production signing is unconfigured pending a certificate |
| The NSIS uninstall prompt hook does not fire in a per-user bundle | Prompt absent on uninstall | Ship the in-app `Delete all history` fallback and document the limitation under §65's "where technically feasible"; the audit records which path was taken |
| An architecture gate fails late and the fix is invasive | A gate red at release | Gates have been running since Phase 03, so a late failure means new drift, not accumulated debt; the fix is scoped to the offending commit, and the release does not proceed with a disabled gate |
| The IPC fuzz test finds a panic in a command | Panic in `ipc-fuzz` | Fix the validation at the command boundary (not in the caller), add the offending input to the matrix permanently, and re-run — a panic reachable from the frontend is a release blocker |

## Security Considerations

This phase is the security deliverable: eleven threat rows each become an executable test or a documented artifact,
so `SECURITY.md` describes behaviour that is actually enforced. Signing secrets exist only as environment variables
and are never written to a config file, a script, or the audit document; the audit generator explicitly filters
environment values out of captured command output so a future run cannot accidentally embed one. The per-user
installer and the absence of any elevation request keep the app outside the privileged surface entirely, which is
what makes the §36 privilege model claim verifiable rather than aspirational.

## Phase decomposition rationale

**Why 24 phases and not 9.** §69's Phase 0–8 order is binding, so it is honoured exactly — but a "user phase" like
U2 (collectors) is 30+ hours of Win32 work with five independent failure modes. Sub-phasing inside each §69 phase
gives the executing agent a commit-sized unit (§89) with its own verification gate, so a failure is localized to
≤12 hours of work instead of poisoning a whole user phase. The mapping column in the plan's Phases table makes the
§69 correspondence auditable at a glance.

**Why the order inside each user phase.** Three rules produced it:
1. *Prove the toolchain before the product.* Phase 02 exists because Rust is absent (B1) and because a linker
   collision (`/usr/bin/link.exe` ahead of MSVC on this PATH) is a known Git-Bash failure. A release-mode Tauri
   smoke build is the cheapest possible proof, and it is also where B2's memory floor is first measured.
2. *Make the sealed path exist before anything can write.* Phase 07 (redaction chokepoint) precedes Phase 08
   (repository) so `insert_batch(&[SealedEvent])` is the *only* insert that ever exists. Redaction is then a type
   error to bypass rather than a review item — the single most load-bearing ordering decision in the plan.
3. *Build the consumer before the producers.* Phase 09 ships the timeline against synthetic data, so all five
   collector phases (10–15) land against a UI that already renders, filters, clusters and drills into events. Each
   collector is therefore verifiable end to end on the day it is written, which is what makes §88's
   "verify after every phase" achievable instead of aspirational.

**Why the collectors are split the way they are.** By *mechanism and measured cost*, not by category: ToolHelp+TCP
table (11), file watching (12), registry+SCM (13), COM+wevtapi (14), SetupAPI+IpHelper+Task Scheduler (15). Each
phase introduces at most two new Win32 subsystems, so a permission or binding surprise is contained. Docker (21)
sits late and alone because it is the one collector whose connected path cannot be verified on this machine.

**Why analysis is one 10-hour phase.** The six-term score, the confidence tiers and the chain builder are one
coherent contract — splitting them would produce a phase whose gate is "half a ranking", which cannot be
meaningfully verified. It is kept honest instead by fixtures: the §59 scenario is the phase's acceptance test.

**Why 3D is two phases.** Phase 19 buys the runtime discipline (dispose registry, frame budget, pause conditions,
fallbacks) that Phase 20 spends. Without that split, the causal graph would carry its own ad-hoc renderer lifecycle
and the §37 idle budget would be unprovable.

**Why states/a11y (23) is its own phase.** §70 requires six states on nine screens. Left as a footnote in nine
phases, the count silently becomes two states on the screens that got attention. As a table-driven 54-cell test it
either passes or names the missing cell.

**Spec citation corrections** (found while cross-checking the digest; the requirements themselves are unaffected):
the System Map is **§48** (some earlier phase text cites §47, which is the Checkpoint screen); the six-state
requirement is **§70**, accessibility is **§62**, design tokens are **§41**, motion is **§42** and the Three.js
DOM-first rule is **§39**. Where an earlier phase file cites a neighbouring number for these, the requirement text is
the authority and the correct § is listed here.

## Risks carried forward

| # | Risk | Owner phase | Current status | Carried mitigation |
|---|---|---|---|---|
| 1 | **B1** Rust toolchain absent on this machine | 02 | Open until Phase 02 runs | Non-elevated `rustup` install, version pinned to the *observed* toolchain, release-mode Tauri smoke build as the gate; PATH entry already present so no PATH mutation is needed |
| 2 | `/usr/bin/link.exe` shadows the MSVC linker on this PATH | 02 | Open, hazard confirmed present | Signal (`link: extra operand`, `LNK1181`) and pre-decided response (re-run cargo via `powershell -NoProfile`, record the resolved linker path in `docs/decisions/0001-rust-toolchain.md`, teach `scripts/gate.ts`) |
| 3 | **B2** <80 MB RAM including WebView2 may be unreachable | 02 → 24 | Unknown until measured twice | Measured on the Phase 02 smoke build and again on the Phase 24 release build, app + all WebView2 children, working set *and* private bytes; three-option escalation pre-written; never silently redefined |
| 4 | **B3** Sub-second processes are invisible to 1 s diff polling | 10, 11, 14 | Accepted, documented | Blind spot stated in `COLLECTORS.md`; crash-exits recovered via Event Log 1000 (probe #16 PASS); ETW only if the Phase 10 spike passes — no claim either way before then |
| 5 | All four Win32 notification APIs may fail non-elevated | 10 | Unknown until the spike battery runs | Every affected collector has a *measured* poll cadence as its fallback (registry ~3 ms/60 s, SCM 3.3 ms/5 s, devices 2128 ms hourly, adapters 0.20 ms/30 s), so the fallback is costed rather than guessed |
| 6 | tauri-specta 1.0 builder shape unverified | 03 | Open | Verify at implementation; fallback `ts-rs` 12.0.1 with a hand-written `invoke` wrapper. No API shape is asserted in advance |
| 7 | WUA typed bindings may not exist in `windows` 0.62.2 | 14 | Open | Check the installed crate first, then late-bound `IDispatch` on `Microsoft.Update.Session`; decision recorded in `docs/decisions/0008-wua-binding.md` |
| 8 | FTS5 may be absent from the bundled SQLite | 08 | Open | Runtime probe `sqlite_compileoption_used('ENABLE_FTS5')`, prefix-index fallback behind one search API, relaxed-but-recorded target |
| 9 | `bollard` 0.21.1 named-pipe transport unverified; no Docker daemon on this machine | 21 | Open, partially unverifiable here | Absent path fully tested (zero error spam); mapping proven by captured fixtures; transport recorded in `docs/decisions/0011-docker-transport.md`; the live gap is stated in `README.md`, `COLLECTORS.md` and the release audit |
| 10 | NSIS uninstall hook may not fire in a per-user bundle | 24 | Open | In-app `Delete all history` as the documented fallback under §65's "where technically feasible"; the taken path is recorded in the release audit |
| 11 | `three` example-module import paths (e.g. `OrbitControls`) differ across versions | 20 | Open | Read the installed package's actual export path; record in `docs/decisions/0010-three-imports.md`; no guessed path |
| 12 | `@tanstack/svelte-virtual` runes integration unproven | 09 | Open | ~60-line hand-rolled windowing fallback specified, dependency dropped if unused (§80) |
| 13 | Ranking quality cannot be validated beyond fixtures before dogfooding | 17, 18 | Accepted | Score breakdown is user-visible, so a bad rank is diagnosable; weights may change only alongside a new failing-then-passing fixture |
| 14 | Idle-CPU budget may be missed once all collectors run together | 24 | Unknown until sampled | Per-collector `last_duration_ms` identifies the offender; pre-decided cadence relaxations with every change recorded in `COLLECTORS.md` and the audit |
| 15 | Production code signing needs a certificate the project may not have | 24 | Out of the plan's control | §66 asks for documentation: development signing is proven, production signing is documented with env-var-only secrets, and the audit states plainly that it is unconfigured pending a certificate |

<!--APPEND-->


































