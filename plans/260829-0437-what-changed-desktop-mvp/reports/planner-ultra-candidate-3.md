# Candidate 3 — Prove the machine, weld the two chokepoints, then let collectors plug into a pipeline that is already stressed and already testable

## Thesis

Three seams carry this build. (1) **Environment first**: nothing product-shaped is written until `rustc` links through a *pinned* MSVC, a blank Tauri window's RSS is measured (B2 answered on day one, not at release), and the TypeScript-7 gate has a recorded verdict. (2) **Two welded chokepoints before the first byte is persisted**: a `Sanitized<T>` newtype constructible only inside `redact/`, and a line-buffered `RedactingWriter` under every tracing sink — so §34/§60 cannot regress later, and no collector *can* bypass them. (3) **Both ends fake before either end is real**: a `Transport` seam lets the whole Svelte app run in a plain browser against a mock backend (Playwright/Vitest without `tauri-driver`, retiring research assumption 11), and a synthetic collector drives the real pipeline at 1M events before any Win32 call exists — so §69's "polished timeline on fake data first" is architecture, not a detour.

Collector cadence is derived from the measured cost table, not guessed: cheap-diff (process 6 ms, services 3.3 ms, listeners sub-ms from Rust, uninstall hives 50 ms) vs notification-driven-or-rare (drivers 2.1 s warm, scheduled tasks 1.3 s, network config 1.4 s). That single routing decision is what keeps §37's idle-CPU budget reachable.

## Plan metadata

```yaml
---
title: "What Changed — Windows Desktop MVP"
description: "Local-first Windows change recorder that answers 'It worked before. What changed?' — Tauri 2 + Rust + Svelte 5 + SQLite, non-elevated, no cloud."
status: pending
priority: P1
effort: 187h
tags: [tauri2, rust, svelte5, typescript, sqlite, windows, privacy, desktop, observability]
blockedBy: []
blocks: []
created: 2026-08-29
---
```

## PLAN.MD BODY

```markdown
## Overview

"What Changed" continuously records meaningful local system changes on Windows 10/11 and lets a
normal human answer *"It worked before. What changed?"* — a flight recorder for a PC. Local-first:
no account, no cloud, no telemetry, fully offline. Greenfield repo at `D:/Project/.WhatChage`
(folder name is misspelled upstream; product/binary/package names are `What Changed` /
`WhatChanged.exe` / `what-changed` and nothing in code may depend on the folder name).
All commands in this plan run from `$REPO=/d/Project/.WhatChage` in Git Bash, non-elevated.
Rust is **not installed** — Phase 1 installs and pins it before any Rust exists.

## Goals

| # | Goal | Priority |
|---|------|----------|
| 1 | Answer "what changed?" in seconds, in plain language, for both simple and advanced users | P1 |
| 2 | Local-first and secret-free: redaction enforced before persistence *and* before logging | P1 |
| 3 | Non-elevated collectors covering the 34/35 measured capabilities, degrading honestly on the 2 gaps | P1 |
| 4 | Deterministic analysis that ranks the true causal chain above unrelated noise, with visible reasons | P1 |
| 5 | Calm, premium DOM UI: six states per screen, exact microcopy voice, 900×600 floor, a11y | P1 |
| 6 | Durable local store: WAL, migrations, retention, corruption recovery, 1M-event query budget | P1 |
| 7 | Narrow typed IPC only; Rust domain types are the single source of truth for payloads | P1 |
| 8 | Three.js used intentionally (ambient field, causal graph, system map) — never a canvas app | P2 |
| 9 | Optional-by-design modules (Docker, Git projects) that never spam errors when absent | P2 |
| 10 | Shippable per-user installer, updater-ready, docs that stay non-stale | P2 |

## Phases

| # | Phase | Status |
|---|-------|--------|
| 01 | [Toolchain & Environment Gate](./phase-01-toolchain-environment-gate.md) | pending |
| 02 | [Product Docs & Repo Hygiene](./phase-02-product-docs-and-repo-hygiene.md) | pending |
| 03 | [Design System & App Shell](./phase-03-design-system-and-app-shell.md) | pending |
| 04 | [Tauri Runtime, Tray, IPC & SQLite Bootstrap](./phase-04-tauri-runtime-tray-ipc-sqlite-bootstrap.md) | pending |
| 05 | [Redaction & Privacy Chokepoint](./phase-05-redaction-privacy-chokepoint.md) | pending |
| 06 | [Storage Schema, Repository & Retention](./phase-06-storage-schema-repository-retention.md) | pending |
| 07 | [Query Service, Live Events & Performance Harness](./phase-07-query-service-live-events-perf-harness.md) | pending |
| 08 | [Timeline & Event Details UI](./phase-08-timeline-and-event-details-ui.md) | pending |
| 09 | [Collector Runtime & Event Normalizer](./phase-09-collector-runtime-and-normalizer.md) | pending |
| 10 | [Process, Port & Service Collectors](./phase-10-process-port-service-collectors.md) | pending |
| 11 | [File/Config & Installed Application Collectors](./phase-11-file-config-and-application-collectors.md) | pending |
| 12 | [Windows Change Intelligence](./phase-12-windows-change-intelligence.md) | pending |
| 13 | [Checkpoints & Comparison](./phase-13-checkpoints-and-comparison.md) | pending |
| 14 | [Deterministic Analysis Engine](./phase-14-analysis-engine-deterministic.md) | pending |
| 15 | [Investigation Screen](./phase-15-investigation-screen.md) | pending |
| 16 | [Three.js Visualization Layer](./phase-16-threejs-visualization-layer.md) | pending |
| 17 | [Docker & Development Mode](./phase-17-docker-and-development-mode.md) | pending |
| 18 | [Release Hardening & Audit](./phase-18-release-hardening-and-audit.md) | pending |

Execution order **is** the phase-number order, which preserves §69's Phase 0→8 sequence
(P0=01–04, P1=05–08, P2=09–11, P3=12, P4=13, P5=14–15, P6=16, P7=17, P8=18). Each phase's
`dependencies:` field lists *hard technical blockers only* and never authorizes running a
later phase earlier than §69's sequence.

## Architecture Summary

Svelte 5 UI (WebView2) → `Transport` seam → narrow typed Tauri commands → Application Service
→ { Query Service, Analysis Engine, Settings, Checkpoints } → Event Repository → SQLite (WAL).
Collectors → `EventDraft` → Normalizer (entity + relationship upsert) → **`redact::sanitize`** →
`Sanitized<SystemEvent>` → batched writer task → SQLite. The UI never touches an OS API; the
frontend is untrusted relative to Rust.

Two enforced chokepoints: `Sanitized<T>` has a private field and is constructible only inside
`src-tauri/src/redact/` (repository accepts nothing else), and every `tracing` sink writes through
`RedactingWriter`, a line-buffered `io::Write` wrapper. `core/`-style portable logic lives in
`domain/`, `storage/`, `analysis/`, `collectors/*/` (OS-free); every Win32/COM call lives in
`platform/windows/`, with `platform/linux` + `platform/macos` present as `unimplemented` module
stubs so the boundary is real. Collectors implement §5's `Collector` trait, run as supervised
Tokio tasks with `catch_unwind`, per-collector health, backoff restart and a token-bucket rate
limiter; one failing collector can never take the app down. Rust types are the source of truth:
TypeScript bindings are generated and a Rust test fails the build if the committed bindings drift.
Three.js is confined to `src/three/` (ambient field, causal graph, system map) with DOM equivalents
for every fact it displays.

## Success Criteria

- [ ] `WhatChanged.exe` launches non-elevated; window works; tray has the seven §7 entries; close hides to tray; monitoring continues.
- [ ] `bun run check`, `bun run test`, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `bun run tauri build` all pass from a clean clone.
- [ ] Redaction leak suite: none of the 12 §60 secret shapes appears in the DB file bytes, the log file bytes, the diagnostics export, or any serialized event.
- [ ] §59 fixture: the `.env` port → container restart → 3000 closed → 4000 opened → health-fail chain ranks above an unrelated Chrome update, with per-factor reasons shown.
- [ ] 10k / 100k / 1M seeded events: timeline page p95 < 50 ms, search < 250 ms, ranking < 300 ms; timeline DOM nodes < 200 at 100k rows.
- [ ] Migration suite: every §10 table and index exists; every version step preserves seeded rows; a pre-migration backup is written.
- [ ] Controlled actions appear in the timeline: app start/stop, port open/close, watched config edit, app install/version change, service restart, driver/update/network/startup change.
- [ ] Every screen renders all six states with real microcopy; no horizontal overflow at 900×600; contrast script passes; reduced-motion honoured.
- [ ] Idle CPU ≈ 0–1% over 5 min; RAM reported as backend RSS + WebView2 total against the <80 MB target with a recorded verdict.
- [ ] Zero generic IPC: no shell, no arbitrary file read, no SQL string reaches Rust from the frontend; capability file contains only the enumerated permissions.
- [ ] `ROADMAP.md#temporary-stubs` is empty at Phase 18 sign-off and the `stubs_are_tracked` test passes.
- [ ] Every §68 doc exists, matches the code it describes, and `bun run check:docs` passes.

## Key Risks

| Risk | Signal it broke | Pre-decided response |
|---|---|---|
| **B1** Rust absent; 3 MSVC installs | `rustup-init` fails, or `cargo build -v` links via VS18 Insiders / `LNK1181` | Non-elevated `rustup-init.exe -y`; pin resolved version in `rust-toolchain.toml`; select non-prerelease MSVC via `vswhere`; wrap builds in `scripts/dev-shell.cmd` (`vcvars64.bat`) |
| **B2** <80 MB RAM may be unreachable with WebView2 | Blank Tauri release exe already >80 MB total in Phase 01 | Report split metrics (backend RSS vs WebView2 total) in `docs/environment-baseline.md`, apply the listed mitigations, then raise it to the user as a scope question — never silently redefine the metric |
| **B3** Non-admin process events have a blind spot | Short-lived process invisible in diff-poll test | Document in `COLLECTORS.md`; add three secondary signals (port-owner inference, `Application Error` 1000, service-state change) marked lower confidence; time-boxed 2 h ETW spike, pessimistic default kept |
| TypeScript 7.0.2 vs `svelte-check`/Vite 8 | `bunx svelte-check` or `tsc --noEmit` errors on the Phase 01 spike | Pin back to the resolved latest 5.x, record the failure output and the resolved version in `docs/decisions.md` |
| `tauri-specta` 1.0.2 builder API unknown to us | Codegen does not compile within 90 min | Fall back to `specta` derive + manual thin TS wrappers, keeping the `bindings_are_current` drift test as the §81 guarantee |
| Driver version change may have no notification (research §5.5) | Spike shows no event on a version bump | Hourly + on-resume + on-boot sweep only; state the latency openly in `COLLECTORS.md` |
| Registry / SCM change notifications unproven non-admin (§5.3, §5.4) | Notification never fires in the spike | Fall back to the measured 50 ms uninstall scan / 3.3 ms service diff on a slow timer |
| Docker path untestable here (D3) | No `\\.\pipe\docker_engine` on this machine | Verify only the absent path; mark the connected path `untested-on-this-machine` in `COLLECTORS.md`; never require Docker |
| 1M-event budget missed | Perf test exceeds the stated p95 | Add covering indexes, then keyset pagination tuning, then a rollup table for counts — in that order, before touching the budget |

## Dependencies

- **External, must exist before Phase 02:** `rustup` + `stable-x86_64-pc-windows-msvc` (installed in Phase 01), Bun 1.3.11, MSVC C++ build tools, Windows SDK 10.0.26100, WebView2 Evergreen runtime, `git`.
- **Version rule:** every crate/package version in this plan comes from the research pin table and is written as an exact pin. For a dependency *not* in that table, add it with `cargo add` / `bun add` and record the resolved exact version in `docs/decisions.md`; never write a version this plan could not verify.
- **API-shape rule:** before writing a call into any third-party crate, read the vendored source of the pinned version under `~/.cargo/registry/src/`; on any mismatch with this plan, the vendored source wins and the deviation is logged in `docs/decisions.md`.
- **Privilege rule:** a collector may claim only capabilities the measured matrix marked PASS. `Win32_ProcessStartTrace`, the `Security` event-log channel, and `Win32_QuickFixEngineering` as an update source are forbidden.
- **User deferrals honoured (not our trimming):** §26 health checks (tables + scoring hook shipped, monitors deferred), §32 non-deterministic providers (trait + deterministic impl shipped), §75–§78 future features (ROADMAP only).
```

## PHASE FILES

### FILE: phase-01-toolchain-environment-gate.md

```markdown
---
phase: 01
title: "Toolchain & Environment Gate"
status: pending
priority: P1
effort: 5h
dependencies: []
---

## Overview

Blocker B1 is the first executable step of the whole plan: no Rust exists on this machine. This
phase installs and pins the toolchain, forces one MSVC out of three, and answers three questions
with measurements instead of assumptions — B2 (RAM floor of a blank Tauri app), D2 (TypeScript 7),
D-installer (which bundlers the installed CLI actually offers). No product code is written; all
work happens in `spikes/` and the answers land in `docs/environment-baseline.md`.

## Requirements

Functional:
- `rustup`, `cargo`, `rustc` available in Git Bash, non-elevated, host triple `x86_64-pc-windows-msvc`.
- Toolchain pinned in `rust-toolchain.toml` with `rustfmt` + `clippy`.
- One MSVC install chosen deterministically and provable in a build log.
- A blank Tauri 2.11.5 release binary compiles, links, launches, and its memory is measured.
- A recorded verdict on `typescript@7.0.2` + `svelte-check@4.7.6` + `vite@8.2.2` + `@sveltejs/vite-plugin-svelte@7.3.0`.
- A recorded list of bundler targets from the installed `@tauri-apps/cli@2.11.4`.

Non-functional: no elevation, no global npm/pip installs, no bash heredocs (Git Bash constraint —
scripts are `.ps1`/`.cmd`), everything re-runnable.

## Architecture

`spikes/` is a throwaway sandbox excluded from the app build and from `tsconfig`. The measurement
scripts are PowerShell because the agent's Bash tool cannot be trusted with heredocs, and because
`Get-Counter`/`Get-Process` are the only accurate sources for CPU/RSS here. `docs/environment-baseline.md`
becomes the machine-truth document every later phase quotes instead of re-measuring.

## Related Code Files

Create:
- `D:/Project/.WhatChage/.gitignore` (minimal: `target/`, `node_modules/`, `dist/`, `spikes/**/target/`, `*.exe`, `*.pfx`, `*.p12`, `.env*`, `*.db`, `*.db-wal`, `*.db-shm`)
- `D:/Project/.WhatChage/rust-toolchain.toml`
- `D:/Project/.WhatChage/scripts/dev-shell.cmd`
- `D:/Project/.WhatChage/scripts/measure-memory.ps1`
- `D:/Project/.WhatChage/scripts/measure-idle-cpu.ps1`
- `D:/Project/.WhatChage/docs/environment-baseline.md`
- `D:/Project/.WhatChage/docs/decisions.md` (ADR log, seeded with the 9 research stack decisions)
- `D:/Project/.WhatChage/spikes/link-gate/**`, `spikes/tauri-gate/**`, `spikes/ts-gate/**`

Modify: none. Delete: `spikes/rustup-init.exe` after install.
## Implementation Steps

1. `mkdir -p /d/Project/.WhatChage/{spikes,scripts,docs}`; write the minimal `.gitignore` first so no spike artifact is ever stageable.
2. `curl -L --fail -o /d/Project/.WhatChage/spikes/rustup-init.exe https://win.rustup.rs/x86_64`.
3. `/d/Project/.WhatChage/spikes/rustup-init.exe -y --no-modify-path --default-host x86_64-pc-windows-msvc --default-toolchain stable --profile default` (user-scope, no UAC), then `export PATH="$HOME/.cargo/bin:$PATH"`.
4. `rustc -Vv` — assert the output contains `host: x86_64-pc-windows-msvc`; `cargo -V`; `rustup component list --installed` must show `clippy` and `rustfmt`.
5. Write `rust-toolchain.toml` using the *resolved* version from `rustc -V` (do not invent one): `[toolchain] channel = "<x.y.z>"`, `components = ["rustfmt", "clippy"]`, `targets = ["x86_64-pc-windows-msvc"]`, `profile = "default"`.
6. MSVC selection: `"/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe" -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath -format value` (no `-prerelease`, so the VS18 Insiders install is excluded). Record the chosen path; write `scripts/dev-shell.cmd` = `call "<chosen>\VC\Auxiliary\Build\vcvars64.bat" && %*`.
7. Link gate: `cargo new /d/Project/.WhatChage/spikes/link-gate --bin && cargo build --release --manifest-path spikes/link-gate/Cargo.toml -v 2>&1 | tee docs/link-gate.log`; grep the log for the `link.exe` path actually used and record it. If it is the Insiders/`14.50` path or the link fails, re-run through `scripts/dev-shell.cmd cargo build --release ...`.
8. Tauri gate: in `spikes/tauri-gate/`, `bun init -y`, `bun add -d @tauri-apps/cli@2.11.4`, create `dist/index.html` (a single `<h1>gate</h1>`), then `bun run tauri init --frontend-dist ../dist --dev-url http://localhost:1420 --before-dev-command "" --before-build-command ""` (confirm the flag names against `bun run tauri init --help` first), pin `tauri = "2.11.5"` / `tauri-build = "2.6.3"`, then `bun run tauri build --no-bundle`.
9. Launch the produced `spikes/tauri-gate/src-tauri/target/release/*.exe`, wait 30 s, then `powershell -File scripts/measure-memory.ps1 -ProcessName <exe stem>` — the script must report, separately: backend process WorkingSet64, the sum of all `msedgewebview2` children WorkingSet64, and the total. This is the **RAM floor** for B2.
10. `powershell -File scripts/measure-idle-cpu.ps1 -ProcessName <exe stem> -Seconds 300` on the same spike to establish the idle-CPU floor.
11. TS gate: in `spikes/ts-gate/`, `bun add -d typescript@7.0.2 svelte@5.56.10 svelte-check@4.7.6 @sveltejs/vite-plugin-svelte@7.3.0 vite@8.2.2`, add one `.svelte` component with a typed prop and one `.ts` file, then `bunx tsc --noEmit` and `bunx svelte-check --threshold error --fail-on-warnings false`. On failure: capture the output, `bun add -d typescript@^5`, re-run, and record the resolved 5.x version as the pin.
12. `bun run tauri build --help` in `spikes/tauri-gate/` — record the accepted `--bundles` values; assert `nsis` is among them.
13. Write `docs/environment-baseline.md` (rustc/cargo versions, chosen MSVC path + link.exe path, RAM floor triple, idle-CPU floor, TS verdict + pinned version, bundler list, WebView2 runtime version read from `HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}\pv`) and `docs/decisions.md` ADR-001…ADR-009 from research §4 plus ADR-010 (TS pin) and ADR-011 (MSVC pin).
14. `rm /d/Project/.WhatChage/spikes/rustup-init.exe`.

## Todo

- [ ] rustup installed non-elevated, host triple asserted
- [ ] `rust-toolchain.toml` pinned to the resolved version
- [ ] MSVC chosen, `dev-shell.cmd` written, link.exe path recorded
- [ ] blank Tauri release exe builds, launches, memory + idle CPU measured
- [ ] TypeScript verdict recorded with the pinned version
- [ ] bundler target list recorded, `nsis` confirmed
- [ ] `docs/environment-baseline.md` + `docs/decisions.md` written

## Success Criteria

- [ ] `rustc -Vv` prints `host: x86_64-pc-windows-msvc` and matches `rust-toolchain.toml`.
- [ ] `cargo clippy -V` and `cargo fmt -V` both succeed.
- [ ] The Tauri gate exe opens a visible window and `measure-memory.ps1` prints three non-zero numbers.
- [ ] `docs/environment-baseline.md` contains a literal PASS/FAIL for TypeScript 7 and a literal `nsis` line.
- [ ] `docs/decisions.md` contains ≥11 numbered ADRs.

## Verification Gate

```bash
rustc -Vv && cargo -V && rustup show
cargo build --release --manifest-path /d/Project/.WhatChage/spikes/link-gate/Cargo.toml
grep -i "link.exe" /d/Project/.WhatChage/docs/link-gate.log | head -3
powershell -NoProfile -File /d/Project/.WhatChage/scripts/measure-memory.ps1 -ProcessName tauri-gate
grep -E "TypeScript|nsis|WebView2" /d/Project/.WhatChage/docs/environment-baseline.md
```
## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| **B1** rustup download/exec blocked (AV, proxy) | `curl` non-zero, or the exe exits without creating `~/.cargo` | Retry once; then try `winget install --id Rustlang.Rustup --scope user`; if both fail, stop and ask the user to run the installer interactively (`! ` prefix) — do not fake progress |
| Wrong MSVC selected | `link.exe` path in `docs/link-gate.log` is under VS18/Insiders, or `LNK1181`/`LNK1104` | Route every cargo invocation through `scripts/dev-shell.cmd`; record it as the mandatory build entry point in `CONTRIBUTING.md` |
| **B2** blank Tauri already >80 MB total | `measure-memory.ps1` total exceeds 80 MB with an empty page | Record the floor, continue the build (target is a budget not a blocker), and raise the scope question to the user in Phase 01's report — Phase 18 re-measures and reports honestly |
| TypeScript 7 breaks the toolchain | `tsc --noEmit` or `svelte-check` exits non-zero on a trivial component | Pin the resolved latest 5.x, record the exact error text in ADR-010; never carry a broken `bun run check` into Phase 03 |
| `tauri init` flag names differ | `bun run tauri init --help` lacks `--frontend-dist` | Use the flags the installed CLI actually prints; hand-write `tauri.conf.json` from the `$schema` it generates |

## Security Considerations

- `rustup-init.exe` is fetched over HTTPS from the official `win.rustup.rs` host and run **without** elevation; it installs strictly under `~/.cargo` and `~/.rustup`.
- `.gitignore` blocks `*.pfx`, `*.p12`, `.env*`, `*.db*` before any file is created, so no credential or history file can be staged later.
- No probe in this phase reads user documents, and nothing measured here is transmitted anywhere.
- `spikes/` is deleted or left build-excluded; no spike code may be reused in the product without review.
```

### FILE: phase-02-product-docs-and-repo-hygiene.md

```markdown
---
phase: 02
title: "Product Docs & Repo Hygiene"
status: pending
priority: P1
effort: 4h
dependencies: [01]
---

## Overview

§90 requires the product/architecture/security/privacy/roadmap documents *before* implementation
code, and §68 requires them to stay non-stale. This phase writes them against the measured
evidence from Phase 01 (which is why it depends on it), establishes repo hygiene, makes the first
commit, and installs a docs-consistency check that later phases extend so the docs cannot rot.

## Requirements

Functional: `README.md`, `PRODUCT.md`, `ARCHITECTURE.md`, `SECURITY.md`, `PRIVACY.md`, `ROADMAP.md`,
`CONTRIBUTING.md`, `COLLECTORS.md`, `ANALYSIS.md` at repo root; `docs/screens-and-states.md`;
`.gitattributes`; final `.gitignore`; first commit on `main`.
`ROADMAP.md` carries a `## Temporary stubs` section (§87) and a `## Deferred by the user` section
naming §26 health monitors, §32 non-deterministic providers, §75–§78.
`COLLECTORS.md` carries one row per planned collector with: signal source, **admin required
(from the measured matrix)**, cadence + measured cost, implementation status, known gaps.

Non-functional: no lorem ipsum anywhere; every microcopy string quoted in `docs/screens-and-states.md`
is the string that will ship; no secrets; commit identity must not be guessed.

## Architecture

Docs are the contract the later phases are audited against, so they are written in a machine-checkable
shape: fixed headings, a collector table keyed by collector id, an IPC command list keyed by command
name. `scripts/check-docs.ts` (run by Bun) validates presence + headings + internal links now, and is
extended in Phases 09/12/17 to assert every `collectors/<id>/` directory has a `COLLECTORS.md` row and
every `#[tauri::command]` appears in `ARCHITECTURE.md`.
## Related Code Files

Create: `README.md`, `PRODUCT.md`, `ARCHITECTURE.md`, `SECURITY.md`, `PRIVACY.md`, `ROADMAP.md`,
`CONTRIBUTING.md`, `COLLECTORS.md`, `ANALYSIS.md`, `.gitattributes`,
`docs/screens-and-states.md`, `scripts/check-docs.ts`, and a **minimal** `package.json`
(`name: "what-changed"`, `private: true`, `scripts: { "check:docs": "bun run scripts/check-docs.ts" }`
— Phase 03 expands it) — all under `D:/Project/.WhatChage/`.
Modify: `.gitignore` (final form), `docs/decisions.md` (ADR-012 native window decorations).
Delete: none.

## Implementation Steps

1. `.gitattributes`: `* text=auto eol=lf`, `*.ps1 text eol=crlf`, `*.cmd text eol=crlf`, `*.db binary`, `*.woff2 binary`.
2. Final `.gitignore`: add `dist/`, `src-tauri/target/`, `src-tauri/gen/`, `.vite/`, `test-results/`, `playwright-report/`, `*.log`, `diagnostics-*.json`, `backups/`. Keep `plans/` and `research/` tracked.
3. `PRODUCT.md`: §1 vision + the black-box framing, the §1 bad-vs-good UX translation table, §2 dual audience, §72 exact Home copy, §73 + §74 example investigations verbatim as the tone target, §54 calm importance model, §53 notification policy with its explicit must-not-fire list, §70 quality bar, §71 window floor.
4. `ARCHITECTURE.md`: the §8 dataflow, §9 `SystemEvent` field list + category/event-type enumerations, §10 table + index inventory, §55 module map, the §56 command list, the §57 event-channel list, §81 source-of-truth rule and the bindings drift test, §5 `Collector` trait with the note that a future sidecar speaks a documented protocol but is not built now.
5. `SECURITY.md`: §82 threat table (11 threats × mitigation × where it is enforced), §35 capability policy incl. the explicit "no `shell:*`, no `fs:*`, no `sql:*`, no `http:*`" line, §36 privilege model, the two measured hard gaps (`Win32_ProcessStartTrace` denied; `Security` channel unauthorized), §66 signing separation, §64 update-tampering stance.
6. `PRIVACY.md`: exact collected / never-collected lists from §33, the §34 secret classes and the two chokepoints, "history stays on this PC", retention defaults, B3's process blind spot stated plainly, and the diagnostics-export redaction promise.
7. `ROADMAP.md`: §69 Phase 0–8 mapped to plan phases 01–18, `## Temporary stubs` (empty), `## Deferred by the user` (§26 monitors, §32 cloud/local providers, §75–§78 with the note that today's relationship graph is what makes Impact possible later).
8. `COLLECTORS.md`: rows for `process`, `network`, `services`, `applications`, `files`, `updates`, `drivers`, `startup`, `eventlog`, `docker`, `git`, `synthetic` — each with the measured privilege verdict and measured cost from the research tables, status `planned`.
9. `ANALYSIS.md`: §27 eight-step pipeline, the §28 score formula with the initial weight table, the §28 "Why this is ranked #1" reason format, §29 relation kinds (confirmed / likely / temporal-only) and the forbidden-phrase list, §30 relationship catalog, §31 deterministic parser scope.
10. `CONTRIBUTING.md`: the §67 command set, the mandatory `scripts/dev-shell.cmd` build entry point, the gate list, conventional-commit style with no AI references, the version-pin and API-shape rules.
11. `docs/screens-and-states.md`: for Home, Timeline, Investigation, Checkpoints, Search, System Map, Settings, Diagnostics, Onboarding — a 6×N matrix of loading / empty / error / success / large-data / small-window with the exact copy for each cell.
12. `scripts/check-docs.ts`: assert all nine root docs exist, each contains its required headings, all relative links resolve, and no file contains `lorem`, `TBD`, or `TODO` outside `ROADMAP.md#temporary-stubs`.
13. Commit: `git -C /d/Project/.WhatChage config --get user.email` — if empty, **stop and ask the user** for name/email (a required identity, not a routine engineering decision), then `git add -A && git commit -m "docs: product, architecture, security and privacy baseline"`.

## Todo

- [ ] `.gitattributes` + final `.gitignore`
- [ ] nine root docs written from measured evidence
- [ ] `docs/screens-and-states.md` six-state matrix with shipping copy
- [ ] `scripts/check-docs.ts` passing
- [ ] first commit on `main` with a real identity

## Success Criteria

- [ ] `bun run check:docs` exits 0.
- [ ] `COLLECTORS.md` privilege column matches the measured matrix for all 12 rows (no collector claims a capability the probe marked FAIL).
- [ ] `git -C /d/Project/.WhatChage log --oneline` shows exactly one commit and `git status` is clean.
- [ ] `grep -c "lorem" *.md docs/*.md` returns 0.

## Verification Gate

```bash
cd /d/Project/.WhatChage && bun install && bun run check:docs
grep -n "Win32_ProcessStartTrace" SECURITY.md COLLECTORS.md
grep -n "Temporary stubs" ROADMAP.md
git -C /d/Project/.WhatChage status --porcelain && git -C /d/Project/.WhatChage log --oneline
```
## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| No git identity configured | `git config --get user.email` empty; commit fails with "Please tell me who you are" | Ask the user once for name + email and set them repo-local (`git -C … config user.name/user.email`); never invent an identity and never use `-c` with a placeholder |
| Docs drift from code immediately | `check:docs` passes but a later phase adds an untabulated collector/command | Extend `check-docs.ts` in Phases 09/12/17 to cross-check `collectors/*/` dirs and `#[tauri::command]` names; the gate becomes a build failure, not a review note |
| Copy written now diverges from shipped copy | A screen ships a string absent from `docs/screens-and-states.md` | Phase 03 introduces `src/lib/copy/*` as the single source of the strings and a Vitest assertion that the doc's quoted strings exist in the copy modules |
| `plans/` accidentally ignored | `git status` no longer shows plan edits | `.gitignore` is reviewed with `git check-ignore -v plans/plan.md` in the gate |

## Security Considerations

- `SECURITY.md`/`PRIVACY.md` are written before any collector exists so the collector phases inherit a written policy instead of retrofitting one.
- The docs state the two measured privilege gaps explicitly; no doc may claim a capability the probe denied.
- No credential, hostname, username, or env value is copied into any doc; the diagnostics-redaction promise is written here and tested in Phases 05 and 18.
```

### FILE: phase-03-design-system-and-app-shell.md

```markdown
---
phase: 03
title: "Design System & App Shell"
status: pending
priority: P1
effort: 10h
dependencies: [02]
---

## Overview

Builds the visual identity and the whole navigable shell in the browser, against a mock backend, so
the UI is developed and tested at Vite speed and Playwright can drive it without `tauri-driver`
(retiring research assumption 11). Delivers the §41 token system, the copied-and-customised
primitives (§4 — concepts from shadcn-svelte/Bits UI, not their code, and deliberately not a generic
shadcn dashboard), the six-state kit (§70), the hand-rolled virtualized list (which retires research
assumption 10 by taking no dependency), and every screen route as a real, styled, empty-state screen.

## Requirements

Functional:
- Bun workspace with the §67 scripts; TypeScript strict; Tailwind 4 via `@tailwindcss/vite`.
- `src/styles/tokens.css` defines every §41 token as a CSS variable, for Dark / Light / System.
- Primitives: Button, Card, Badge, Dialog, Drawer, Input, Select, Switch, Tabs, Tooltip, Skeleton, EmptyState, ErrorState, LoadState, VirtualList, ImportanceDot, KeyValueRow, DiffRow.
- Shell: header with status pill + global search entry, sidebar navigation (Home, Timeline, Investigations, Checkpoints, Search, Advanced → System Map, Diagnostics, Settings), typed router, Ctrl+K palette shell.
- `Transport` seam: `invoke` bound to `@tauri-apps/api/core` in the app build, to `mock-transport.ts` when `VITE_WC_MOCK=1`.
- Six states renderable for every screen from the copy modules; 900×600 with no horizontal overflow.
Non-functional: no `any` (§79); no online font, no CDN (§63) — system stack `Segoe UI Variable Text`
→ `Segoe UI` → `system-ui`, recorded as ADR-013; motion tokens honour `prefers-reduced-motion` (§42);
WCAG-AA contrast for every token pair used on text; keyboard reachable everywhere (§62).

## Architecture

`src/lib/tauri/transport.ts` exports a single `invoke<K extends CommandName>(name, args)` that both
real and mock paths satisfy, so no component ever imports `@tauri-apps/api` directly — this is also
what keeps "the frontend never touches the OS" mechanically true. `mock-transport.ts` returns
fixtures from `src/lib/tauri/fixtures/`. State lives in Svelte 5 runes modules (`*.svelte.ts`) with
derived state instead of duplicated state (§79). `LoadState.svelte` takes a
`ViewState<T> = {kind:'loading'} | {kind:'empty'} | {kind:'error', message, retry} | {kind:'ready', data}`
and renders the matching slot, so a screen cannot silently omit a state; `large-data` is handled by
`VirtualList` plus a "showing first N of M" affordance, and `small-window` by the 900×600 layout rules.

## Related Code Files

Create (all under `D:/Project/.WhatChage/`):
- `package.json` (expand), `bun.lock`, `tsconfig.json`, `vite.config.ts`, `svelte.config.js`, `index.html`, `vitest.config.ts`, `playwright.config.ts`
- `src/main.ts`, `src/app/App.svelte`, `src/app/router.ts`, `src/app/shell/{AppShell,Sidebar,HeaderBar,StatusPill,CommandPalette}.svelte`
- `src/components/ui/{Button,Card,Badge,Dialog,Drawer,Input,Select,Switch,Tabs,Tooltip,Skeleton,EmptyState,ErrorState,LoadState,VirtualList,ImportanceDot,KeyValueRow,DiffRow}.svelte`
- `src/features/{home,timeline,investigation,checkpoints,search,settings,systemmap,diagnostics,onboarding}/<Screen>.svelte` (route shells with real empty-state copy)
- `src/lib/tauri/{transport.ts,mock-transport.ts,fixtures/*.ts}`, `src/lib/stores/{route,theme,app-state}.svelte.ts`
- `src/lib/types/view-state.ts`, `src/lib/formatting/{time,bytes,importance}.ts`, `src/lib/design/motion.ts`
- `src/lib/copy/{home,timeline,investigation,checkpoints,search,settings,onboarding,errors,common}.ts`
- `src/styles/{tokens.css,base.css,app.css}`
- `src/features/**/__tests__/*.test.ts` (six-state matrix), `tests/e2e/{small-window.spec.ts,keyboard.spec.ts}`
- `scripts/check-contrast.ts`

Modify: `package.json` scripts, `.gitignore` (playwright dirs). Delete: none.

## Implementation Steps

1. `bun add -d vite@8.2.2 svelte@5.56.10 @sveltejs/vite-plugin-svelte@7.3.0 tailwindcss@4.3.3 @tailwindcss/vite@4.3.3 vitest@4.1.11 @testing-library/svelte@5.4.2 svelte-check@4.7.6 playwright@1.62.1 typescript@<Phase-01 verdict>`; `bun add @tauri-apps/api@2.11.1`.
2. Scripts (§67): `dev` = `VITE_WC_MOCK=1 vite`, `dev:app` = `vite`, `build` = `vite build`, `check` = `svelte-check --threshold error && tsc --noEmit`, `test` = `vitest run`, `test:e2e` = `playwright test`, `check:contrast` = `bun run scripts/check-contrast.ts`, `check:docs`, `tauri` = `tauri`.
3. `tsconfig.json`: `strict: true`, `noUncheckedIndexedAccess`, `verbatimModuleSyntax`, `moduleResolution: "bundler"`, `types: ["vite/client"]`, exclude `spikes/`, `src-tauri/`.
4. `src/styles/tokens.css`: `:root` with `--wc-background`, `--wc-surface-1/2/3`, `--wc-border-subtle`, `--wc-border-active`, `--wc-text-primary/secondary/muted`, `--wc-accent`, `--wc-success`, `--wc-warning`, `--wc-danger`, `--wc-info`, `--wc-shadow-sm/md/lg`, `--wc-radius-sm/md/lg/xl`, `--wc-space-1..8`, `--wc-dur-fast/base/slow`, `--wc-ease-out/spring`; `[data-theme="light"]` overrides; `@media (prefers-color-scheme: light)` for System. Map into Tailwind with `@import "tailwindcss";` + `@theme { --color-background: var(--wc-background); … }`.
5. Visual identity (§40): dark base `#0B0C0F`-class background, one accent hue, one-pixel `border-subtle` hairlines, restrained 8 %-opacity surface elevation, no gradient blobs, no glow borders, ≤6 chromatic colors total; density tuned for 1200–1600 px with a 900 px collapse rule (sidebar → icon rail).
6. Build the primitives by hand from Bits UI *concepts* (composition + `aria-*` + focus management), each ≤120 lines, no copied dependency; `Dialog`/`Drawer` use `<dialog>`+focus trap; `Tooltip` is hover+focus and `aria-describedby`.
7. `VirtualList.svelte`: fixed-row-height windowing over an items array with `overscan`, `role="list"`, keyboard `Home/End/PageUp/PageDown`, and an `onLoadMore` callback at 80 % scroll — no external virtualizer (ADR-014, retires research assumption 10).
8. `transport.ts` + `mock-transport.ts` + fixtures for `get_settings`, `get_collector_status`, `get_dashboard_summary`, `get_events` (500 fake events across all categories, importance-weighted so most are informational per §54).
9. Router + shell + all nine route screens rendering their empty state with the exact copy (Home shows §72 strings: "Everything looks normal.", "What stopped working?", placeholder "Example: Discord stopped detecting my microphone this morning", "Important changes", "Investigate", "Your history stays on this PC.").
10. Theme store: `system | dark | light` with `document.documentElement.dataset.theme`, persisted through `update_settings` later; browser mock persists to memory.
11. Six-state tests: one shared `screen-state-matrix.test.ts` iterating a `SCREENS` registry × six states, asserting each renders its documented copy and has an accessible landmark; a screen missing a state fails the suite.
12. `scripts/check-contrast.ts`: parse `tokens.css`, compute WCAG contrast for the documented text-on-surface pairs, fail below 4.5:1 (3:1 for ≥18.66 px semibold).
13. Playwright: `small-window.spec.ts` asserts `document.documentElement.scrollWidth <= clientWidth` at 900×600 on every route; `keyboard.spec.ts` tabs through Home and asserts a visible focus ring and that Ctrl+K opens the palette. If browser download fails, set `PLAYWRIGHT_BROWSERS_PATH` to an existing local browser cache and record it in `CONTRIBUTING.md`.
## Todo

- [ ] Bun + Vite 8 + Svelte 5 + Tailwind 4 + TS strict wired, `bun run check` clean
- [ ] §41 token set complete for dark/light/system
- [ ] 18 primitives hand-built, ≤120 lines each
- [ ] `Transport` seam + mock backend + fixtures
- [ ] nine routes with real empty-state copy from `src/lib/copy/*`
- [ ] six-state matrix test green for every screen
- [ ] contrast script green; 900×600 overflow test green; keyboard test green

## Success Criteria

- [ ] `bun run dev` serves the full shell in a browser with zero backend running.
- [ ] `bun run check` reports 0 errors; no `any` outside a documented `// eslint-safe: reason` line.
- [ ] `bun run test` runs the 9-screen × 6-state matrix and passes.
- [ ] `bun run check:contrast` passes for all documented pairs in both themes.
- [ ] `bun run test:e2e` passes `small-window` on all nine routes at 900×600.
- [ ] Switching theme changes only CSS variables (no component re-implementation), verified by a test asserting one `data-theme` attribute drives both themes.

## Verification Gate

```bash
cd /d/Project/.WhatChage && bun install && bun run check && bun run test
bun run check:contrast && bun run check:docs
bun run build && bun run test:e2e
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Vite 8 plugin incompatibility (`@tailwindcss/vite` or `vite-plugin-svelte`) | `bun run dev` fails at config load or Tailwind emits no CSS | Isolate by disabling one plugin at a time; if `@tailwindcss/vite` is incompatible, fall back to Tailwind 4's `@import` + PostCSS path (still Tailwind 4, still CSS-first tokens) and record ADR |
| UI drifts into "generic shadcn dashboard" (§40 violation) | Review finds default shadcn spacing/radii/card stacks, >6 colors, gradient buttons | The token file is the only source of color/radius/spacing; a review checklist in `docs/screens-and-states.md` is run at the end of Phases 03, 08, 15, 16 |
| Mock/real transport divergence | A screen works in mock mode and breaks in the app build | Both paths are typed by the same generated `bindings.ts` from Phase 04; a Vitest test asserts the mock implements every `CommandName` |
| Playwright cannot install a browser | `bunx playwright install chromium` fails | Use `PLAYWRIGHT_BROWSERS_PATH` against an existing local cache; if unavailable, keep the six-state matrix in Vitest and mark the 900×600 check as a documented manual step in `docs/screens-and-states.md` — do not claim E2E coverage that does not run |
| Hand-rolled `VirtualList` mis-measures rows | Blank gaps or jumpy scrolling with 100k fixtures | Fixed row height is enforced by a CSS custom property and asserted in a test; variable-height rows are explicitly out of scope (clusters get their own fixed taller row) |

## Security Considerations

- No component imports `@tauri-apps/api` directly; all IPC flows through `transport.ts`, which is the only file allowed to.
- No remote asset, font, analytics, or CDN reference is introduced (§63); the build must contain zero external URLs — asserted by a grep test over `dist/`.
- Mock fixtures contain no real system data and no secret-shaped strings, so a leaked fixture cannot leak anything.
- `mock-transport.ts` is excluded from the app build by the `VITE_WC_MOCK` guard and dead-code elimination is verified by grepping `dist/` for a mock-only sentinel string.
```

### FILE: phase-04-tauri-runtime-tray-ipc-sqlite-bootstrap.md

```markdown
---
phase: 04
title: "Tauri Runtime, Tray, IPC & SQLite Bootstrap"
status: pending
priority: P1
effort: 12h
dependencies: [02, 03]
---

## Overview

Completes §69 Phase 0: the real `src-tauri` crate, window lifecycle (close → tray), the seven-entry
tray menu, structured tracing to rolling files, settings persisted in SQLite behind a migration
runner, the first narrow typed commands with generated TypeScript bindings and a drift test, and a
release installer that builds. This is where the Svelte shell from Phase 03 stops being a browser
app and becomes `WhatChanged.exe`.

## Requirements

Functional:
- `src-tauri` crate with `tauri 2.11.5` / `tauri-build 2.6.3`; `productName: "WhatChanged"`, identifier `com.whatchanged.app`, window 1360×860, `minWidth: 900`, `minHeight: 600`.
- Plugins: `tauri-plugin-single-instance 2.4.3` (focus existing window), `tauri-plugin-autostart 2.5.1`, `tauri-plugin-notification 2.3.3`, `tauri-plugin-window-state 2.4.1`, `tauri-plugin-dialog 2.7.2`, `tauri-plugin-opener 2.5.4`, `tauri-plugin-updater 2.10.1` (configured, manual-only).
- Tray with exactly: Open What Changed / What changed today? / Create checkpoint / Pause monitoring / Resume monitoring / Settings / Quit (§7).
- `CloseRequested` → `api.prevent_close()` + `window.hide()`; monitoring state survives hiding.
- SQLite opened at `%APPDATA%/com.whatchanged.app/history.db` (`history-dev.db` under the `dev-tools` feature) with WAL, `busy_timeout`, `auto_vacuum=INCREMENTAL` set **before** the first table exists; migration `0001_core.sql` creates `schema_migrations`, `settings`, `collector_status`.
- Commands shipped: `get_app_info`, `get_settings`, `update_settings`, `get_collector_status`, `set_monitoring_paused`, `set_autostart`. Generated bindings + `bindings_are_current` test.
- `bun run tauri build` produces an NSIS per-user installer.
Non-functional: no `unwrap()` in production paths (§79) — `thiserror 2.0.20` typed errors mapped to a
serializable `IpcError { code, message }`; capability file contains only enumerated permissions and no
`shell`/`fs`/`sql`/`http` permission; `panic = "unwind"` kept in release so Phase 09's `catch_unwind`
collector isolation works (ADR-015); tracing has dev-console + rolling-file sinks with configurable
verbosity and **never** logs a secret (the writer is replaced by `RedactingWriter` in Phase 05).

## Architecture

`lib.rs` builds the Tauri app; `app/setup.rs` performs ordered startup: resolve paths → init tracing →
open DB → run migrations → load settings → build tray → register commands/events → show window unless
`start_hidden`. `app/state.rs` holds `AppState { settings: RwLock<Settings>, db: Db, monitoring:
AtomicMonitoringState, stubs: &'static [&'static str] }` in Tauri's managed state. `storage/pool.rs`
opens one writer connection pinned to a dedicated thread plus a small reader pool (2 connections,
`Mutex<VecDeque<Connection>>`, no r2d2 dependency per §80); commands read via
`tauri::async_runtime::spawn_blocking`. `commands/errors.rs` converts every `thiserror` variant into
`IpcError` so no panic or raw string crosses IPC.

## Related Code Files

Create:
- `src-tauri/{Cargo.toml,build.rs,tauri.conf.json}`, `src-tauri/capabilities/default.json`, `src-tauri/icons/*`
- `src-tauri/migrations/0001_core.sql`
- `src-tauri/src/{main.rs,lib.rs}`
- `src-tauri/src/app/{mod.rs,setup.rs,paths.rs,logging.rs,settings.rs,tray.rs,window.rs,state.rs,stubs.rs,events_bus.rs}`
- `src-tauri/src/commands/{mod.rs,app_info.rs,settings.rs,collectors.rs,errors.rs}`
- `src-tauri/src/storage/{mod.rs,pool.rs,migrate.rs,repository/{mod.rs,settings.rs,collector_status.rs}}`
- `src-tauri/src/domain/{mod.rs,ids.rs,time.rs,settings.rs}`
- `src-tauri/src/platform/{mod.rs,windows/mod.rs,linux/mod.rs,macos/mod.rs}`
- `src-tauri/tests/{migrations.rs,bindings_current.rs,stubs_tracked.rs,settings_roundtrip.rs}`
- `src/lib/tauri/{bindings.ts (generated),commands.ts,events.ts}`

Modify: `package.json` (tauri scripts), `src/lib/tauri/transport.ts` (real path), `src/app/shell/StatusPill.svelte` (live monitoring state), `ARCHITECTURE.md` (command list), `ROADMAP.md` (stub registry), `.gitignore` (`src-tauri/target`, `src-tauri/gen`).

## Implementation Steps

1. From `$REPO`: `bun run tauri init` with the flags verified in Phase 01 (`--frontend-dist ../dist`, `--dev-url http://localhost:1420`, `--before-dev-command "bun run dev:app"`, `--before-build-command "bun run build"`); then hand-edit `tauri.conf.json` against the `$schema` it wrote.
2. `Cargo.toml` exact pins: `tauri = { version = "2.11.5", features = ["tray-icon", "image-png"] }` (feature names verified against the vendored crate's `Cargo.toml` before use), `tauri-build = "2.6.3"`, `rusqlite = { version = "0.40.2", features = ["bundled", "backup"] }`, `rusqlite_migration = "2.6.0"`, `tokio = { version = "1.53.1", features = ["rt-multi-thread","sync","time","macros"] }`, `tracing = "0.1"`, `tracing-subscriber = { version = "0.3.23", features = ["env-filter","fmt"] }`, `tracing-appender = "0.2.5"`, `thiserror = "2.0.20"`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `specta = "1.0.5"`, `tauri-specta = "1.0.2"`, plus the seven pinned plugins. `[features] dev-tools = []`. `[profile.release] lto = "thin"`, `codegen-units = 1`, `strip = "symbols"`, **no** `panic = "abort"`.
3. `tauri.conf.json`: `app.windows[0]` = title "What Changed", 1360×860, min 900×600, `resizable`, native decorations (ADR-012: native chrome for Win11 snap + a11y, revisit trigger recorded); `app.withGlobalTauri: false`; `app.security.csp` = a strict `default-src 'self'` policy with no remote origin (the resulting header is read from devtools once and pasted into `SECURITY.md`); `bundle.targets: ["nsis"]`, `bundle.windows.nsis.installMode: "currentUser"`, publisher, license, `resources`/`icons`. Any key the CLI schema rejects is replaced with the schema's real key, and the deviation is logged in `docs/decisions.md`.
4. `capabilities/default.json`: enumerate permissions from `src-tauri/gen/schemas/*.json` after the first build and include **only** window show/hide/start-dragging-free set actually used, event listen/emit, notification, dialog save, opener open-url, updater check. Assert by test that the file contains none of `shell:`, `fs:`, `sql:`, `http:`, and no `*:default` catch-all unless the schema proves it is minimal.
5. `app/paths.rs`: `%APPDATA%/com.whatchanged.app/{history.db,logs/,backups/}` via Tauri's app-data resolver; create dirs with `create_dir_all`; never derive a path from the repo folder name (D1).
6. `app/logging.rs`: `tracing_subscriber::registry()` + `EnvFilter` from settings (`error|warn|info|debug|trace`, default `info`) + a `fmt` layer over `tracing_appender::rolling::Builder` (daily rotation, `max_log_files(7)`) and a dev-console layer under `cfg!(debug_assertions)`. Writers are wrapped by a `NopRedactWriter` placeholder whose type is swapped for `RedactingWriter` in Phase 05 — recorded in `ROADMAP.md#temporary-stubs` as `logging.redaction` so §87 tracking is real.
7. `storage/pool.rs` + `migrate.rs`: open with `PRAGMA journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`, `foreign_keys=ON`, `temp_store=MEMORY`, and `auto_vacuum=INCREMENTAL` executed on a brand-new file before DDL; run `rusqlite_migration` (`user_version`) and additionally insert one row per applied migration into `schema_migrations` (§10 requires the table literally).
8. `0001_core.sql`: `schema_migrations(version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at INTEGER NOT NULL)`, `settings(key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at INTEGER NOT NULL)`, `collector_status(collector_id TEXT PRIMARY KEY, enabled INTEGER NOT NULL, health TEXT NOT NULL, last_error TEXT, last_ok_at INTEGER, updated_at INTEGER NOT NULL)`.
9. `domain/settings.rs`: one `Settings` struct with serde defaults covering §49's sections (general, monitoring toggles, privacy, storage/retention, appearance, advanced incl. `developer_mode`, integrations); persisted as key→JSON rows; `update_settings` takes a partial patch and validates ranges in Rust (§35).
10. `app/tray.rs`: `TrayIconBuilder` with a `Menu` of the seven §7 items in that order; `Pause monitoring`/`Resume monitoring` enabled-state follows the monitoring flag; `What changed today?` shows the window and routes to Timeline/today; `Create checkpoint` is wired in Phase 13 and until then is disabled with the tooltip "Available after checkpoints are enabled" and registered in the stub registry.
11. `app/window.rs`: `on_window_event` → `WindowEvent::CloseRequested { api, .. }` → `api.prevent_close()`, `window.hide()`, emit `monitoring-state-changed`; `Quit` performs an ordered shutdown (stop collectors → flush writer → `wal_checkpoint(TRUNCATE)` → remove the clean-exit marker).
12. Typed IPC + codegen: read the vendored `tauri-specta 1.0.2` source, follow its own builder example to collect commands + events and export to `src/lib/tauri/bindings.ts`. `tests/bindings_current.rs` regenerates into a temp buffer and asserts byte-equality with the committed file (§81 enforcement). If the builder cannot be wired within 90 min, fall back to `specta` derives + hand-written wrappers in `commands.ts`, keeping the same drift test over the exported type set.
13. `tests/stubs_tracked.rs`: assert `app::stubs::STUBS` and the ids listed under `ROADMAP.md#temporary-stubs` are the same set.
14. `bun run tauri build` → verify the NSIS artifact; copy it to `dist-installer/WhatChanged-Setup-x64.exe` with `scripts/rename-installer.ps1` (§65 naming) and record both names.
## Todo

- [ ] `src-tauri` crate compiles and links through the pinned MSVC
- [ ] window + tray + close-to-tray + single-instance + autostart
- [ ] tracing to rolling files with configurable verbosity
- [ ] SQLite opened with the full PRAGMA set; `0001_core.sql` applied; `schema_migrations` populated
- [ ] six commands + generated bindings + drift test
- [ ] minimal capability file with the forbidden-permission test
- [ ] NSIS per-user installer builds and installs into a per-user location
- [ ] stub registry wired and tested

## Success Criteria

- [ ] `bun run tauri dev` opens the Phase 03 shell inside WebView2 with live settings from SQLite.
- [ ] Closing the window hides it; the tray menu's seven entries are present in order; `Quit` exits cleanly and leaves no `history.db-wal` growth.
- [ ] `%APPDATA%/com.whatchanged.app/history.db` exists with `user_version = 1` and `PRAGMA journal_mode` = `wal`.
- [ ] Theme + verbosity changes persist across a restart.
- [ ] `cargo test` passes `migrations`, `bindings_current`, `stubs_tracked`, `settings_roundtrip`.
- [ ] `bun run tauri build` succeeds and the installer creates a Start-Menu shortcut without elevation.
- [ ] `grep -E '"(shell|fs|sql|http):' src-tauri/capabilities/default.json` returns nothing.

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
bun run check && bun run test
bun run tauri build
powershell -NoProfile -File scripts/measure-memory.ps1 -ProcessName WhatChanged
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `tauri-specta 1.0.2` builder API differs from expectation | Codegen does not compile after 90 min | Switch to `specta` derive + hand-written typed wrappers; keep the drift test; log ADR — §81 is satisfied either way |
| Tauri config/permission key invented | CLI rejects `tauri.conf.json` or capability identifier | Use the CLI-generated `$schema` and `gen/schemas/*.json` as the only key source; never keep a key the schema rejects |
| WAL file grows unbounded during dev | `history.db-wal` > 64 MB | `wal_checkpoint(TRUNCATE)` on clean exit and whenever WAL > 16 MB (Phase 06 maintenance) |
| **B2** app RSS already over budget with the real shell | `measure-memory.ps1` total > 80 MB | Record; apply the mitigation list (single window, no dev-tools in release, three.js disposed, virtualized lists, page size 100); re-measure in Phase 18 and escalate as a scope question with numbers |
| Installer writes user data into Program Files | Uninstall removes history, or install prompts for UAC | `installMode: "currentUser"`; DB lives in `%APPDATA%`; asserted by installing, creating data, upgrading, and checking the DB survives (Phase 18 repeats this) |
| Tray "Create checkpoint" ships as a dead entry | Clicking it does nothing | It is disabled with real microcopy and registered in the stub registry until Phase 13 — a disabled, labelled control is not a fake implementation |

## Security Considerations

- Capability file is minimal and machine-checked for forbidden permission prefixes; `withGlobalTauri: false` keeps the API off `window`.
- Strict CSP with no remote origin; `dangerousDisableAssetCspModification` stays false; devtools are not enabled in release builds.
- Every command validates its input in Rust (enum-typed filters, bounded integers); no command accepts a path, SQL fragment, or command string.
- Errors crossing IPC are typed `IpcError` values — no `unwrap()`, no panic payload, no internal path leakage in messages.
- Logs are written under `%APPDATA%`, rotated daily and capped at 7 files; the redaction writer lands in Phase 05 and is tracked as a stub until then.
```

### FILE: phase-05-redaction-privacy-chokepoint.md

```markdown
---
phase: 05
title: "Redaction & Privacy Chokepoint"
status: pending
priority: P1
effort: 7h
dependencies: [04]
---

## Overview

Builds §34's central redaction engine **before** any event can be persisted or any collector exists,
and welds it into two places that cannot be bypassed: the type that the repository accepts, and the
writer under every tracing sink. §60's leak tests are written here as an adversarial suite that
scans the raw database file, the raw log file and the diagnostics bundle for the secret needles.

## Requirements

Functional:
- Key-concept redaction for `password, passwd, secret, token, apikey, api_key, authorization, cookie, session, private_key, client_secret` (case/separator-insensitive, substring match on the key).
- Value-pattern redaction for: `sk-…` style API keys, `Bearer …`, JWT (`eyJ…\.…\.…`), AWS-style access key + secret, PEM private-key blocks, DB URLs with inline credentials (`scheme://user:pass@host`, incl. `postgres://`, `mongodb://`, `mysql://`, `redis://`, `amqp://`), OAuth client secrets, session-cookie values, and a high-entropy fallback for unknown keys in config diffs (length ≥ 20 and Shannon entropy ≥ 3.5 bits/char).
- `Sanitized<T>` newtype with a private field, constructible only inside `redact/`; `EventRepository::insert_events` accepts `&[Sanitized<SystemEvent>]` and nothing else.
- `RedactingWriter<W: Write>`: line-buffered, scrubs each complete line before delegating; installed under both the rolling-file and dev-console sinks, replacing the Phase 04 placeholder.
- Secret change detection without storing secrets: compare per-process-random-salted hashes **in memory only**, persist a `secret_changed: bool` and the literal `[REDACTED]` before/after values.
Non-functional: one regex set compiled once (`regex = "1"`, `OnceLock`); redaction of a 4 KiB config
value under 1 ms; no allocation in the log writer's fast path when a line contains no candidate; false
positives are acceptable, false negatives are not (documented policy).

## Architecture

`redact/patterns.rs` owns the compiled `RegexSet` + per-pattern replacements. `redact/engine.rs`
exposes `scrub_text(&str) -> Cow<str>`, `scrub_json(&mut serde_json::Value)` (key-aware walk) and
`sanitize_event(EventDraft) -> Sanitized<SystemEvent>`, which also sets the event's `sensitive` flag
(§9) when anything was redacted. `redact/sanitized.rs` is the whole security boundary:

```rust
pub struct Sanitized<T>(T);                       // field private to this module
impl<T> Sanitized<T> { pub fn get(&self) -> &T { &self.0 } }
pub(super) fn wrap<T>(v: T) -> Sanitized<T> { Sanitized(v) }   // only redact/ can call this
```

There is no `From`, no `DerefMut`, and no public constructor, so the only path into the repository is
through `sanitize_event`. `tests/chokepoint.rs` additionally scans `src/**/*.rs` and fails if
`Sanitized(` or `Sanitized {` appears outside `src/redact/`.

## Related Code Files

Create:
- `src-tauri/src/redact/{mod.rs,patterns.rs,engine.rs,sanitized.rs,config_values.rs,log_writer.rs,entropy.rs}`
- `src-tauri/src/privacy/{mod.rs,policy.rs}`
- `src-tauri/tests/{redaction_leak.rs,redaction_unit.rs,chokepoint.rs}`
- `src-tauri/tests/fixtures/secrets/{env-with-secrets.env,config-with-secrets.json,pem-key.txt}`

Modify: `src-tauri/src/app/logging.rs` (swap placeholder writer), `ROADMAP.md` (remove the
`logging.redaction` stub), `PRIVACY.md` (final pattern list), `SECURITY.md` (chokepoint description),
`src-tauri/Cargo.toml` (`regex`).

## Implementation Steps

1. `patterns.rs`: build a `RegexSet` for the value patterns listed in Requirements plus individual `Regex` objects for replacement; every pattern gets a name used in tests and in `PRIVACY.md`.
2. `entropy.rs`: Shannon entropy over the byte histogram; used only for config-diff values whose key is not recognised, gated by length ≥ 20 and an alphabet check that excludes plain sentences (whitespace ratio < 5 %).
3. `engine.rs::scrub_text`: single pass with `RegexSet::matches` to decide whether any replacement is needed; on no match return `Cow::Borrowed`. Replacement is always the literal `[REDACTED]` (keeps the §13 display shape).
4. `engine.rs::scrub_json`: recursive walk; when a key matches a §13/§34 concept, replace the whole value regardless of shape; otherwise scrub the string value; arrays walked element-wise; depth capped at 32 and node count at 50 000 (flood defence, §82).
5. `config_values.rs`: `redact_kv_diff(before: &[(String,String)], after: &[(String,String)]) -> Vec<ValueChange>` producing `ValueChange { key, before_display, after_display, secret: bool, secret_changed: bool }`; secret values never leave the function as plaintext; the salt is a `OnceLock<[u8;32]>` filled from `getrandom` at process start and never persisted (ADR-016).
6. `sanitized.rs`: as shown above, plus `sanitize_event` setting `sensitive = true` and stripping any field the policy forbids (full command lines, file contents, raw HTTP headers) before wrapping.
7. `privacy/policy.rs`: a const list of forbidden field kinds (`FileContents`, `CommandLine`, `ClipboardData`, `Keystrokes`, `BrowserHistory`, `ScreenCapture`) with a compile-time-checked mapping test asserting no domain struct carries such a field — implemented as a source scan for the forbidden identifiers in `domain/`.
8. `log_writer.rs`: `RedactingWriter<W>` implementing `io::Write`; buffer until `\n`, scrub the completed line, delegate; `flush` scrubs and emits any partial buffer. Wire it as the `MakeWriter` for both sinks in `app/logging.rs` and delete the placeholder + its stub-registry entry.
9. `tests/redaction_unit.rs`: table-driven, one case per pattern name, asserting both replacement and non-replacement of a benign lookalike (e.g. `sk` alone, `Bearer` with no token, a `postgres://host/db` URL without credentials).
10. `tests/redaction_leak.rs` (§60, adversarial): a corpus of 12 needles (`sk-`-style key, `Bearer` token, JWT, `postgres://user:password@host`, `mongodb://user:pw@host`, AWS access key + secret, PEM block, session cookie, `Authorization` header line, `client_secret`, a raw password, a high-entropy blob). For each, push it through: (a) `sanitize_event` metadata, (b) `redact_kv_diff`, (c) `tracing::error!` into a temp log dir, (d) a diagnostics-export call. Then `wal_checkpoint(TRUNCATE)`, read `history.db`, the log file and the export as **raw bytes** and assert the needle appears in none of them, and that `[REDACTED]` does appear.
11. `tests/chokepoint.rs`: the source scan for `Sanitized(`/`Sanitized {` outside `src/redact/`, plus a scan asserting no `tracing::` macro call site interpolates a `ValueChange::raw`-like accessor (there is none — the accessor does not exist).
12. Update `PRIVACY.md` with the final pattern names and the in-memory-salt design; update `SECURITY.md` with the two chokepoints and the tests that enforce them.

## Todo

- [ ] pattern set + entropy fallback implemented and unit-tested per pattern
- [ ] `Sanitized<T>` boundary + repository signature change
- [ ] `RedactingWriter` installed under both sinks; placeholder stub removed
- [ ] secret-change detection with no persisted hash
- [ ] §60 leak suite scanning DB bytes, log bytes, diagnostics bytes
- [ ] chokepoint source-scan test
- [ ] `PRIVACY.md` / `SECURITY.md` updated

## Success Criteria

- [ ] `cargo test redaction` passes, including 12/12 needles absent from all four sinks.
- [ ] `cargo test chokepoint` proves `Sanitized` is constructed only inside `redact/`.
- [ ] Attempting to call `insert_events` with a bare `SystemEvent` is a compile error (demonstrated by a commented example in `sanitized.rs` and asserted in review).
- [ ] A log line containing a JWT written via `tracing::info!` appears in the file as `[REDACTED]`.
- [ ] `redact_kv_diff` on the §13 sample yields `API_PORT 3000 → 4000` and `OPENAI_API_KEY [REDACTED] → [REDACTED]` with `secret_changed = true`.
## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml redaction -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml chokepoint
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
grep -rn "Sanitized(" src-tauri/src --include=*.rs | grep -v "src-tauri/src/redact/"
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| A secret shape ships unmatched (false negative) | A new needle added to the corpus fails the leak test | Policy is "false positives acceptable, false negatives not": add the pattern, widen the entropy fallback, and add the needle permanently to the corpus |
| Log line split across two `write` calls leaks a token | Leak test finds half a needle in the log | Writer is line-buffered by construction; the test writes a token in two `write` calls to prove the buffering |
| Entropy heuristic redacts a legitimate value (e.g. a hash a user wants to see) | User-visible `[REDACTED]` where a benign long value was expected | Restricted to unknown keys in config diffs only; the event keeps `sensitive = true` and the UI shows "Value hidden because it looked like a secret" — never silently blank |
| Salt reuse across restarts would enable offline correlation | Any code path writing the salt to disk | Salt lives in a `OnceLock` filled at start; a test asserts no `settings`/`events` row contains the salt bytes |
| Redaction cost on a large config file | Diff of a 1 MiB file takes > 50 ms | 1 MiB parse cap + 5 000 line cap (§82 malicious config); over the cap the event records "file too large to diff" instead of contents |

## Security Considerations

- This phase *is* the §34 mitigation and the anchor for §82's "secret leakage" row; both chokepoints are enforced by tests that fail the build.
- Nothing here stores a hash, fingerprint or truncated prefix of a secret — the only persisted derivative is a boolean.
- Diagnostics export (built in Phase 18) is included in the leak corpus now so the export can never regress ahead of its own phase.
- The forbidden-field policy scan makes "no spyware behavior" (§33) a mechanical check rather than an intention.
```

### FILE: phase-06-storage-schema-repository-retention.md

```markdown
---
phase: 06
title: "Storage Schema, Repository & Retention"
status: pending
priority: P1
effort: 12h
dependencies: [04, 05]
---

## Overview

The §9 domain model and the full §10 SQLite schema, behind a repository that only accepts sanitized
events, with §11 retention/maintenance, batched non-blocking writes (§37), and §83 database safety
including pre-migration backups and a non-crash-looping corruption recovery path with its own screen.

## Requirements

Functional:
- `SystemEvent` with every §9 field; `EventCategory` (15 variants) and `EventType` (19 variants) as Rust enums serialized as stable snake_case strings.
- Migration `0002_events.sql` creating `events, entities, entity_relationships, snapshots, checkpoints, health_states, watched_locations, applications, analysis_sessions, analysis_results` and every §10 index.
- Repository modules for events, entities, relationships, snapshots, checkpoints, applications, analysis; prepared statements cached; all writes in transactions.
- Writer task: bounded `mpsc` (10 000), batches of 250 or 500 ms, one transaction per batch, `try_send` overflow accounted and surfaced as collector health `Degraded`.
- Retention: defaults 30 d events / 7 d snapshots / 90 d important-or-above; options 7/30/90/custom; batched deletes (500 rows), `PRAGMA incremental_vacuum`, WAL truncate above 16 MB; runs 60 s after start then every 6 h; never on the UI thread.
- Storage estimate: `page_count * page_size` + per-table row counts for §49.
- Corruption: unclean-shutdown marker → `PRAGMA quick_check` at open; on `SQLITE_CORRUPT`/`SQLITE_NOTADB` at any time enter recovery mode (no crash loop) offering Repair / Export diagnostics / Reset local history.
- Pre-migration backup to `backups/pre-migration-v<N>-<ts>.db` (keep 3) using `rusqlite::backup`.

Non-functional: never store file contents or arbitrary documents (§10); large metadata as JSON columns;
no `unwrap()`; every public repository fn returns a typed `StorageError`.

## Architecture

`domain/` is OS-free and is the source of truth (§81); `storage/repository/*` maps rows ↔ domain
structs with hand-written SQL (no ORM, §80). Entity ids are **deterministic strings** so upserts are
idempotent and relationships need no lookup table pass: `process:<pid>:<start_ts>`,
`executable:<lowercased path>`, `port:tcp:<addr>:<port>`, `service:<name>`, `application:<publisher>|<name>`,
`driver:<device instance id>`, `adapter:<guid>`, `container:<id>`, `config_file:<lowercased path>`,
`project:<lowercased path>`. This removes a uuid dependency and makes `entity_relationships` writes
`INSERT OR IGNORE`. Reads happen on `spawn_blocking` against the reader pool; writes only ever happen
on the single writer thread, which is also the only holder of a write transaction — so `busy_timeout`
is a safety net, not a design element.
## Related Code Files

Create:
- `src-tauri/migrations/0002_events.sql`
- `src-tauri/src/domain/{event.rs,category.rs,event_type.rs,importance.rs,entity.rs,relationship.rs,snapshot.rs,checkpoint.rs,health.rs}`
- `src-tauri/src/storage/{writer.rs,retention.rs,integrity.rs,backup.rs,estimate.rs}`
- `src-tauri/src/storage/repository/{events.rs,entities.rs,relationships.rs,snapshots.rs,checkpoints.rs,applications.rs,analysis.rs,health_states.rs}`
- `src-tauri/src/commands/{database.rs}` (`get_storage_estimate`, `repair_database`, `reset_local_history`, `export_diagnostics` stub-free minimal version)
- `src-tauri/tests/{migrations.rs (extend),repository.rs,retention.rs,integrity_recovery.rs,writer_batching.rs}`
- `src/features/diagnostics/RecoveryScreen.svelte`, `src/lib/copy/recovery.ts`

Modify: `src-tauri/src/storage/{mod.rs,migrate.rs,pool.rs}`, `src-tauri/src/app/setup.rs` (marker file,
recovery branch), `ARCHITECTURE.md` (schema), `PRIVACY.md` (retention defaults), `src/app/router.ts`
(recovery route).

## Implementation Steps

1. `domain/event.rs`: `SystemEvent { id: i64, timestamp: i64 (unix ms UTC), category, event_type, entity_type, entity_id, entity_display_name, severity, importance, source_collector, before: Option<Json>, after: Option<Json>, metadata: Json, related_entity_ids: Vec<String>, user_friendly_summary: String, technical_summary: String, confidence: Confidence, sensitive: bool }`, plus `EventDraft` (same minus `id`) as the only thing collectors construct. `chrono` is added with `cargo add chrono --no-default-features --features clock,serde` and the resolved version recorded (version rule).
2. `category.rs` / `event_type.rs`: the exact 15 and 19 variants from §9, with `as_str`/`FromStr` and a round-trip test; stored as TEXT for readable SQL and forward-compatible migrations.
3. `importance.rs`: `Importance { Informational, Notable, Important, Critical }` + `Severity`, plus the mapping table `(category, event_type, entity class) → importance` implementing §54's "most events are informational"; a test asserts that for a representative 40-event corpus ≥ 80 % land on `Informational` and that no app-update event is `Critical`.
4. `0002_events.sql`: `events` (all §9 columns; `metadata TEXT NOT NULL DEFAULT '{}'`, `synthetic INTEGER NOT NULL DEFAULT 0`, `search_text TEXT NOT NULL`), `entities(id TEXT PRIMARY KEY, entity_type, display_name, first_seen_at, last_seen_at, attributes TEXT)`, `entity_relationships(source_id, target_id, kind, confidence, first_seen_at, last_seen_at, PRIMARY KEY(source_id,target_id,kind))`, `snapshots`, `checkpoints`, `health_states`, `watched_locations`, `applications`, `analysis_sessions`, `analysis_results`. Indexes exactly per §10 plus `events(importance, timestamp)`, `events(source_collector, timestamp)`, `entities(entity_type)`.
5. Extend `tests/migrations.rs`: (a) fresh DB → `user_version = 2`, all 13 tables present, every §10 index present by name; (b) build a v1 DB with seeded settings rows, migrate to v2, assert rows survive and a backup file exists; (c) assert `schema_migrations` has one row per version.
6. `storage/backup.rs`: before any migration that changes existing tables, `wal_checkpoint(TRUNCATE)` then `rusqlite::backup::Backup` to `backups/pre-migration-v<N>-<ts>.db`; prune to the 3 newest.
7. `storage/writer.rs`: writer thread owning the write connection; `WriteOp::{Events(Vec<Sanitized<SystemEvent>>), UpsertEntities(..), UpsertRelationships(..), Snapshot(..), CollectorStatus(..)}`; batching by count/time; one transaction per flush; on `SQLITE_BUSY` retry twice then report `StorageError::Busy`; emits `(inserted, dropped)` counters for diagnostics.
8. `repository/events.rs`: `insert_events(&[Sanitized<SystemEvent>])`, `get_by_id`, `list_page(EventQuery)` with keyset pagination on `(timestamp DESC, id DESC)`, `count_by_importance(range)`, `delete_older_than(...)`. All statements prepared and cached (`Connection::prepare_cached`).
9. `repository/entities.rs` + `relationships.rs`: `upsert_entity` (`INSERT … ON CONFLICT(id) DO UPDATE SET last_seen_at=…`), `upsert_relationship` with `kind` from the §30 catalog, `neighbors(entity_id, depth)` returning a bounded BFS frontier (depth ≤ 3, ≤ 500 nodes) for Phase 14.
10. `storage/retention.rs`: `RetentionPolicy { events_days, snapshot_days, important_days, custom }`; delete loop in 500-row batches ordered by `timestamp`; keep `importance >= Important` for `important_days`; `PRAGMA incremental_vacuum(256)` after a pass; skip entirely while an investigation is running.
11. `storage/integrity.rs`: clean-exit marker `open.lock`; if present at open → `PRAGMA quick_check` (log duration); on failure or on a runtime `SQLITE_CORRUPT`, rename to `history.corrupt-<ts>.db`, open a fresh DB, set `AppState.recovery = Some(reason)`, and emit `monitoring-state-changed`. `repair_database` opens the corrupt file read-only and copies salvageable rows table-by-table, counting failures; `reset_local_history` deletes the corrupt file after an explicit confirmation.
12. `RecoveryScreen.svelte`: shown when `recovery` is set — headline "We could not read your history file.", body "Your settings are intact. You can try to repair the history, export diagnostics, or start a fresh history.", three buttons, all six states covered (loading = repairing with a progress count).
13. `storage/estimate.rs` + `get_storage_estimate`: DB bytes, WAL bytes, backups bytes, event count, oldest event date, projected 30-day size from the last 7 days' rate.
14. `tests/writer_batching.rs`: 5 000 drafts pushed in a burst → asserts ≤ 25 transactions, no drops, and that a `Sanitized` wrapper was required to compile. `tests/retention.rs`: seed 50 000 events across 120 days → assert post-pass counts match the policy and important events survive.

## Todo

- [ ] §9 domain types + enum round-trip tests
- [ ] `0002_events.sql` with every §10 table and index
- [ ] migration + backup + `schema_migrations` tests
- [ ] batched writer with bounded channel and drop accounting
- [ ] repositories with prepared statements and keyset pagination
- [ ] retention + incremental vacuum + WAL truncate
- [ ] corruption detection, repair, reset, and the Recovery screen
- [ ] storage estimate command

## Success Criteria

- [ ] `cargo test` passes migrations (fresh + upgrade), repository, retention, writer batching, integrity recovery.
- [ ] `PRAGMA user_version` = 2 and every §10 index name is present (asserted, not eyeballed).
- [ ] A deliberately truncated `history.db` produces the Recovery screen, not a crash loop (asserted by test + manual run).
- [ ] Retention pass over 50 000 events completes < 2 s and leaves `importance >= Important` events younger than 90 d intact.
- [ ] Inserting 5 000 events uses ≤ 25 transactions and the UI thread never blocks (measured by a timing assertion in the test).
- [ ] `insert_events` cannot be called without `Sanitized` (compile-time).
## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml migrations repository retention writer integrity
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
bun run tauri dev   # observe: DB at %APPDATA%/com.whatchanged.app, user_version=2, no WAL growth at idle
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `auto_vacuum=INCREMENTAL` set too late (no effect) | `PRAGMA auto_vacuum` returns 0 on a fresh DB | It is executed on the empty file before DDL; a migration test asserts the pragma value on a fresh DB, and if a legacy DB reports 0 the maintenance pass uses a full `VACUUM` at most monthly instead |
| Write amplification from frequent small batches | WAL churn visible as constant disk writes at idle | Batch floor of 500 ms and idle suppression (no transaction when the batch is empty); measured in the Phase 18 idle test |
| Keyset pagination breaks on identical timestamps | Duplicate or skipped rows across pages | Cursor is `(timestamp, id)` and the index is `events(timestamp)` + PK; a test seeds 1 000 events sharing one timestamp and walks all pages asserting exact coverage |
| Repair copies corrupt rows and re-corrupts the new DB | `quick_check` fails on the fresh DB after repair | Repair copies row-by-row through domain deserialization (invalid rows are skipped and counted) and runs `quick_check` before switching over; on failure it falls back to Reset with the count reported |
| Retention deletes data the user still expects | User reports missing history | Defaults are exactly §11's; the Settings copy states the effective cutoff dates; a retention pass logs the counts it removed per policy bucket |

## Security Considerations

- The repository's only insert path takes `Sanitized<SystemEvent>` (Phase 05), so the schema cannot be filled with unredacted data.
- No column stores file contents; `before`/`after` hold JSON diffs of keys and redacted values only, with size caps enforced at the collector boundary.
- Corrupt-file handling never executes anything from the corrupt DB and never deletes the user's data without explicit confirmation (§83).
- Backups live under `%APPDATA%` alongside the DB, are excluded from git, and are pruned to 3 to bound disk use.
- `entity_id` strings may contain user paths; the diagnostics export replaces the user profile prefix with `%USERPROFILE%` (Phase 18) and the leak suite covers it.
```

### FILE: phase-07-query-service-live-events-perf-harness.md

```markdown
---
phase: 07
title: "Query Service, Live Events & Performance Harness"
status: pending
priority: P1
effort: 10h
dependencies: [06]
---

## Overview

Turns the store into a queryable, live-updating service the UI can trust: typed filters and paging,
indexed global search (§52), throttled live channels (§57), the §85 developer simulation panel and
synthetic event generator (the mechanism §69 Phase 1 asks for so the timeline can be built on fake
data), and the §61 performance harness at 10k / 100k / 1M events.

## Requirements

Functional:
- `get_events(EventQuery)` where `EventQuery { range: TimeRange (Today|Yesterday|Last7Days|Custom{from,to}), categories: Vec<EventCategory>, importance_min: Importance, collectors: Vec<String>, entity_id: Option<String>, cursor: Option<Cursor>, limit: u16 (≤200) }` — every filter is an enum or bounded scalar, never a SQL fragment.
- `get_event_details(id)` returning the event, its entity, its related events (via `related_entity_ids` + relationship neighbours), and its raw metadata (§46).
- `get_dashboard_summary()` returning today's counts by importance, the §72 status line, and the top recent important events.
- `search_events(query, filters, limit)` over application names, event descriptions, file names, ports, services, drivers, container names, checkpoint labels (§52).
- Live channels: `system-event-added` (batched payload), `collector-status-changed`, `monitoring-state-changed`, `analysis-completed` — emitted at most 10×/s with coalescing.
- Dev-only (`dev-tools` feature + `developer_mode` setting): `dev_inject_event(kind)` for driver update / port change / process crash / DNS change / Docker restart / config diff, `dev_seed_events(count, spread_days)`, and a `SimulationPanel.svelte`.
- Perf harness: seeded 10k / 100k / 1M event benchmarks with stated budgets.

Non-functional: no query loads more than `limit` rows into memory; the frontend never receives an
unbounded array; all synthetic rows carry `synthetic = 1` and are excluded from production queries by
default; the release build without `dev-tools` contains no injection command at all.
## Architecture

`storage/query.rs` builds SQL from the typed `EventQuery` with a fixed clause set (each optional
filter appends a parameterized fragment; there is no string interpolation of user input anywhere).
`storage/search.rs` decides its index strategy once at startup: run
`SELECT sqlite_compileoption_used('ENABLE_FTS5')` — if 1, create `events_fts` as an external-content
FTS5 table over `events.search_text` maintained by triggers; if 0, fall back to
`event_search_tokens(token, event_id)` with a `token` index and prefix matching. Both paths satisfy
§52 and both are measured by the same 1M-row test; the chosen path is logged and shown in Diagnostics.
`app/events_bus.rs` coalesces emissions: a Tokio task drains a broadcast queue every 100 ms and emits
one `system-event-added` payload containing up to 200 events plus an `omitted` count, so the UI never
re-renders per event (§57).

## Related Code Files

Create:
- `src-tauri/src/storage/{query.rs,search.rs}`
- `src-tauri/src/commands/{events.rs,search.rs,dashboard.rs,dev.rs}`
- `src-tauri/src/collectors/synthetic/{mod.rs,generator.rs,scenarios.rs}` (behind `dev-tools`)
- `src-tauri/tests/{query_paging.rs,search.rs,perf_events.rs,live_events_throttle.rs}`
- `src/lib/stores/events.svelte.ts`, `src/lib/tauri/events.ts` (typed listeners)
- `src/features/settings/sections/DeveloperSection.svelte`, `src/components/system/SimulationPanel.svelte`
- `scripts/seed-perf-db.ps1`

Modify: `src-tauri/src/app/events_bus.rs`, `src-tauri/src/lib.rs` (register commands), `src/lib/tauri/mock-transport.ts` (mirror the new commands), `ARCHITECTURE.md`, `COLLECTORS.md` (synthetic row).

## Implementation Steps

1. `query.rs`: `EventQuery` → `(sql, params)`; clauses for range, `category IN (…)`, `importance >= ?`, `source_collector IN (…)`, `entity_id = ?`, `synthetic = 0 OR ?`, keyset `(timestamp, id) < (?, ?)`, `ORDER BY timestamp DESC, id DESC LIMIT ?`. A unit test asserts the generated SQL for 8 filter combinations is stable (snapshot test) and that no parameter is ever inlined.
2. `search.rs`: startup probe + both strategies; `search_text` is built at insert time as lowercased `display_name || ' ' || user_friendly_summary || ' ' || technical_summary || ' ' || entity_id`; query terms are split on whitespace, capped at 6 terms of ≤ 64 chars, and ANDed.
3. `commands/events.rs`, `search.rs`, `dashboard.rs`: thin, validating, `spawn_blocking` wrappers; `get_dashboard_summary` also returns the §72 status string chosen by a rule (`0 important → "Everything looks normal." ; n>0 → "{n} recent changes may need attention."`).
4. `events_bus.rs`: `tokio::sync::broadcast` + a 100 ms drain task; payload `{ events: [...], omitted: n }`; a test asserts that 10 000 events pushed in 1 s produce ≤ 10 emissions and that `omitted` accounts for the remainder exactly.
5. `src/lib/stores/events.svelte.ts`: a rune-based store holding at most 500 recent events for the live head of the timeline, appending batches and dropping the tail; the rest is fetched by page. A test asserts the store length never exceeds 500 after 100 batches.
6. `collectors/synthetic/scenarios.rs`: the six §85 injections, each producing a realistic normalized draft chain (driver update = `driver/version_changed` with before/after versions; port change = `network/closed` + `network/opened` 300 ms apart; process crash = `process/stopped` + an `eventlog` Application Error; DNS change = `network/configuration_changed` with `1.1.1.1 → 192.168.1.1`; Docker restart = `docker/restarted`; config diff = `filesystem/modified` with a redacted `.env` diff).
7. `generator.rs`: `seed(count, spread_days)` producing an importance distribution matching §54 (≈85 % informational, 12 % notable, 3 % important, <0.5 % critical), plausible entity ids reused across events so relationship traversal has something to walk, and `synthetic = 1`.
8. `commands/dev.rs`: `#[cfg(feature = "dev-tools")]` on the module *and* a runtime `developer_mode` check; every injected event's `source_collector = "dev.simulator"`; the UI badges them "Simulated". `package.json` `dev:app` and the Tauri `beforeDevCommand` chain pass `--features dev-tools` in dev only.
9. `SimulationPanel.svelte` inside Settings → Advanced → Developer, hidden unless `developer_mode`; buttons for the six scenarios plus a seed control (1k/10k/100k).
10. `tests/perf_events.rs` (`#[ignore]`, run with `--release`): `seed_10k`, `seed_100k`, `seed_1m` each asserting — first page (limit 100, category filter, range Last7Days) p95 < 50 ms over 20 runs; `search_events("nvidia")` < 250 ms; `count_by_importance(today)` < 30 ms; retention pass < 2 s; DB size logged. 1M seeding uses one transaction per 10 000 rows and is expected to take minutes — it is `#[ignore]` and run explicitly.
11. `scripts/seed-perf-db.ps1`: wraps the ignored tests so the developer can run `powershell -File scripts/seed-perf-db.ps1 -Count 1000000` and get a report file under `docs/perf/`.
12. Frontend perf assertion: a Playwright test loads the mock transport with a 100 000-row fixture and asserts `document.querySelectorAll('[data-event-row]').length < 200` while scrolling (virtualization proof, §61).

## Todo

- [ ] typed `EventQuery` + keyset paging + snapshot SQL test
- [ ] search strategy probe + both implementations + 1M measurement
- [ ] dashboard summary with the §72 status rule
- [ ] coalesced live channels + bounded frontend store
- [ ] synthetic generator + six §85 scenarios behind `dev-tools`
- [ ] perf harness at 10k/100k/1M with stated budgets
- [ ] virtualization proof in the browser

## Success Criteria

- [ ] `cargo test --release -- --ignored perf` prints all budgets met at 10k and 100k; the 1M run's numbers are recorded in `docs/perf/`.
- [ ] `search_events("25565")`, `("NVIDIA")`, `("Discord")`, `("DNS")`, `("nginx")` each return matching seeded rows.
- [ ] 10 000 events emitted in 1 s produce ≤ 10 IPC emissions with an exact `omitted` count.
- [ ] A release build (`--no-default-features`, no `dev-tools`) contains no `dev_inject_event` symbol — asserted by `grep` over the binary's exported command list in the generated bindings.
- [ ] Timeline DOM stays under 200 rows with 100 000 items.

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml query search live_events
cargo test --manifest-path src-tauri/Cargo.toml --release -- --ignored perf
cargo test --manifest-path src-tauri/Cargo.toml --features dev-tools synthetic
bun run test && bun run test:e2e
```
## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| FTS5 absent in the `rusqlite` bundled amalgamation | Low×Med | startup probe returns 0 | token-table path is already implemented and measured; log the choice, show it in Diagnostics, no scope change |
| 1M-row budgets missed | Med×High | perf report over budget | add the covering index `(category, timestamp DESC, id DESC)`, then narrow the default range to Last7Days; if still over, report the measured numbers to the user as a scope question — do not relax the printed budget silently |
| Live-event flood starves the UI | Med×Med | > 10 emissions/s in the throttle test | the coalescing drain is the fix by construction; if a collector still floods, cap per-collector emissions per interval and count the rest |
| Synthetic rows leak into real analysis | Low×High | an analysis result cites `synthetic = 1` | every read path defaults to `synthetic = 0`; a test asserts `get_events` with default query returns zero synthetic rows after seeding 1 000 |

## Security Considerations

The frontend can express only what `EventQuery` allows: no SQL, no table names, no `LIKE` patterns of
its own, no unbounded `limit`. Search terms are length- and count-capped before reaching the index.
`dev_inject_event`/`dev_seed_events` are compiled out of release builds and additionally gated at
runtime, so a compromised WebView in a shipped build cannot fabricate history. Because seeded rows are
marked `synthetic`, no simulated data can ever be presented as evidence about the real machine.
```

### FILE: phase-08-timeline-and-event-details-ui.md

```markdown
---
phase: 08
title: "Timeline, Dashboard & Event Details UI"
status: pending
priority: P1
effort: 12h
dependencies: [03, 07]
---

## Overview

Build the two screens the product is judged on: the Dashboard (§70 — "what changed recently" at a
glance) and the Timeline (§44/§46/§51 — reverse-chronological, virtualized, filterable, searchable),
plus the Event Details panel (§52) that shows before/after and related events. Everything renders from
the typed commands of Phase 07 through the `Transport` seam, so this phase is fully testable in a
browser with `VITE_WC_MOCK=1` and needs no working collector.

## Requirements

Functional:
- Dashboard: a status line from `get_dashboard_summary` (§72 tone), counts by importance for today, the
  five most recent notable-or-higher events, a "System is being monitored / paused" indicator, and a
  primary action into the Timeline. Empty state (§68): a plain explanation that monitoring just started
  and what will appear, never a bare "No data".
- Timeline: virtualized list grouped by day with sticky day headers; each row shows time, category
  icon, importance accent, `display_name`, and the one-line `user_friendly_summary` (§59 plain
  language). Rows are keyboard-focusable and open Details on Enter.
- Filters (§51): category multi-select, importance floor, time range (Today/Yesterday/Last 7 days/
  Custom), and collector source. Filters are reflected in the URL query so a state is shareable and
  survives reload. A single "Clear filters" resets to defaults.
- Search (§52): debounced 250 ms, calls `search_events`, shows a result count, and highlights matched
  terms by wrapping text nodes only (no `innerHTML`).
- Live head: new events arriving on `system-event-added` prepend into the current view only when the
  list is scrolled to the top and no filter excludes them; otherwise a "N new changes" pill appears and
  scrolls to top on click. This is the only place live data mutates the visible list.
- Details panel: `display_name`, importance, exact timestamp with relative age, source collector,
  before/after values rendered as a two-column diff for scalars and a line diff for text, the technical
  summary behind a "Technical details" disclosure, related events (§52) as a navigable list, and a
  "Copy details" action that copies the redacted text form.
- All 6 §54 importance levels have a distinct non-color-only treatment (icon + label), per §64.

Non-functional:
- Timeline first paint with 100 000 rows in the fixture: interaction stays responsive; DOM rows < 200.
- No component reaches into `invoke` directly; only `src/lib/tauri/*` may (§56 boundary).
- Every interactive element is reachable by keyboard in a logical order, and the details panel traps
  focus while open and restores it on close.

## Architecture

Routing stays the shell's hash-free SvelteKit-free router from Phase 03 (a `$state` route object driven
by `history.pushState`), so filters live in `location.search` and are parsed into a single
`timelineFilters` rune object. `TimelineView.svelte` owns one `PagedEvents` controller
(`src/features/timeline/paged-events.svelte.ts`) that holds `items`, `cursor`, `loading`, `exhausted`
and calls `transport.getEvents()` when `VirtualList` reports the last rendered index is within 20 of
the tail. `VirtualList.svelte` (hand-rolled, ADR-014) takes `items`, `estimatedRowHeight`, and a
snippet; it measures rendered rows to correct offsets so variable-height rows do not drift. Day grouping
is computed from the same array by a derived function that emits `{ type: 'header' } | { type: 'event' }`
entries, so headers are ordinary virtualized rows and no nested scroller exists.

## Related Code Files

Create:
- `src/features/dashboard/{DashboardView.svelte,StatusHeadline.svelte,ImportanceCounts.svelte,RecentChanges.svelte}`
- `src/features/timeline/{TimelineView.svelte,TimelineRow.svelte,DayHeader.svelte,FilterBar.svelte,SearchField.svelte,NewEventsPill.svelte,paged-events.svelte.ts,group-by-day.ts}`
- `src/features/details/{EventDetailsPanel.svelte,BeforeAfterDiff.svelte,TextDiff.svelte,RelatedEvents.svelte,TechnicalDisclosure.svelte}`
- `src/components/ui/VirtualList.svelte`
- `src/lib/format/{relative-time.ts,importance.ts,category.ts,highlight.ts}`
- `tests/unit/{group-by-day.test.ts,relative-time.test.ts,highlight.test.ts,paged-events.test.ts}`
- `tests/e2e/{dashboard.spec.ts,timeline.spec.ts,details.spec.ts}`

Modify: `src/routes.ts`, `src/lib/tauri/mock-transport.ts` (fixtures: empty, 12 events, 100 000 events), `DESIGN.md` (row anatomy + importance treatments).

## Implementation Steps

1. `group-by-day.ts`: pure function `(events) => Row[]` inserting a header row when the local calendar day changes; day label is `Today`/`Yesterday`/`EEEE, d MMM` computed with `Intl.DateTimeFormat` (no date library). Unit-tested across a DST boundary and a year boundary.
2. `relative-time.ts`: `Intl.RelativeTimeFormat`, thresholds seconds/minutes/hours/days; unit-tested at 0 s, 59 s, 61 s, 90 min, 25 h, 8 d.
3. `VirtualList.svelte`: window of `start..end` computed from `scrollTop`, a measured-height cache keyed by index, top/bottom spacer divs, `overscan = 6`, `role="list"` with each row `role="listitem"`, and `aria-setsize`/`aria-posinset`. Emits `onRangeChange(start, end)`.
4. `paged-events.svelte.ts`: `load(reset)` guarded against overlapping calls; keyset cursor from the last item; `exhausted` when a page returns fewer than `limit`. A unit test with a fake transport asserts no duplicate ids across pages and that a filter change resets the cursor.
5. `FilterBar.svelte`: native `<select multiple>` is rejected for usability — use a disclosure with checkbox groups, labelled `<fieldset><legend>`; each change writes `location.search` via `replaceState` and triggers `load(true)`.
6. `SearchField.svelte`: 250 ms debounce, `type="search"`, `aria-describedby` result count, and an explicit clear button. Search replaces the paged source with a search result source; the pill and live prepending are disabled while searching.
7. `highlight.ts`: splits a string on matched terms and returns segments; `TimelineRow` renders segments with `<mark>`. A unit test asserts a term containing `<script>` is rendered as text.
8. `TimelineRow.svelte`: fixed left rail with the category icon, importance chip (icon + text), monospaced time, name, then summary truncated with `text-ellipsis`; `tabindex="0"`, `aria-label` composed as `"{importance} {category}: {display_name}, {relative time}"`.
9. `NewEventsPill.svelte` + store wiring: subscribe to the coalesced channel; if `scrollTop > 8` or a filter is active, increment a counter instead of inserting.
10. `EventDetailsPanel.svelte`: a `<dialog>`-less custom panel (Tauri WebView2 supports `<dialog>`, but focus-trap behavior is asserted by test either way) with `role="dialog" aria-modal="true"`, first-focus on the heading, `Esc` to close, focus returned to the invoking row.
11. `BeforeAfterDiff.svelte`: for scalars, two labelled columns with a changed-marker; for multi-line text, a line diff computed by a small LCS in `src/lib/format/line-diff.ts` (no dependency), capped at 400 lines with a "diff truncated" note.
12. `RelatedEvents.svelte`: renders `get_event_details().related`, each item navigating the panel to that event and pushing a back-stack entry so `Esc`/back returns to the first event.
13. Fixtures + e2e: `dashboard.spec.ts` asserts the empty state text and the populated status line; `timeline.spec.ts` asserts filter → URL → reload round-trip, DOM row count under load, and pill behavior; `details.spec.ts` asserts focus trap, restore, disclosure, and copy content being the redacted form.

## Todo

- [ ] `VirtualList` with measured heights + a11y list semantics
- [ ] day grouping, relative time, importance/category formatting
- [ ] paged controller with keyset cursor and filter reset
- [ ] filters serialized to the URL and restored on load
- [ ] debounced search with safe highlighting
- [ ] live-head pill with filter-aware suppression
- [ ] details panel: diff, disclosure, related events, focus management
- [ ] e2e specs for dashboard, timeline, details

## Success Criteria

- [ ] With the 100 000-row fixture, scrolling from top to 50 000 keeps `[data-event-row]` count < 200 and no duplicate ids appear.
- [ ] Applying `category=network&importance=notable`, reloading the page, and reading the filter bar shows the same selection.
- [ ] Typing `nvidia` shows a result count and `<mark>` around matches; typing `<script>alert(1)</script>` renders literally with no execution (asserted by a page error listener).
- [ ] Opening details from a row and pressing `Esc` returns focus to that exact row.
- [ ] Empty database renders the §68 explanatory empty state, not "No data".
- [ ] `bun run check` (svelte-check, strict) reports 0 errors and 0 warnings.

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test -- tests/unit
VITE_WC_MOCK=1 bun run test:e2e -- tests/e2e/dashboard.spec.ts tests/e2e/timeline.spec.ts tests/e2e/details.spec.ts
```

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| Hand-rolled virtualization drifts with variable row heights | Med×Med | scroll position jumps in the e2e scroll test | the measured-height cache is the designed fix; if drift persists, pin rows to a single height (56 px) and clamp the summary to one line — a visual change the user's §46 requirement still allows |
| Live prepending fights the user's scroll | Med×Med | e2e pill test fails or rows shift while reading | suppression rule (top-only, no active filter) is normative here; never insert while `scrollTop > 8` |
| Text diff on a huge file blocks the UI | Low×Med | frame stall on a large `.env`/config diff | 400-line cap plus a note; the full text is never sent — the backend truncates before the IPC boundary |
| Filters in the URL leak sensitive strings into history | Low×Low | a search term appears in `location.search` | search terms are deliberately *not* URL-persisted; only enum filters are |

## Security Considerations

No component calls `invoke`; only `src/lib/tauri/*` does, keeping the §56 boundary auditable by a single
grep asserted in Phase 18. Highlighting never uses `innerHTML`, so an event name harvested from a
filesystem path or process name cannot inject markup. "Copy details" copies the same redacted strings
the backend produced — the frontend has no access to unredacted values, and the details payload contains
no absolute path beyond what redaction already allowed.
```

### FILE: phase-09-collector-runtime-and-normalizer.md

```markdown
---
phase: 09
title: "Collector Runtime, Supervisor & Event Normalizer"
status: pending
priority: P1
effort: 9h
dependencies: [05, 06]
---

## Overview

Build the machinery every collector plugs into (§5): the `Collector` trait, the supervisor that owns
scheduling, isolation and backoff, the snapshot/diff helper that turns two entity sets into add/remove/
change drafts, and the normalizer (§30) that turns a draft into the single canonical event row —
importance-scored (§54), plain-language-summarized (§59), redacted (§49) and deduplicated (§58). No real
collector ships here; a `mock` collector proves the whole path end to end, which is why this phase can
be verified without any Win32 work.

## Requirements

Functional:
- `Collector` trait: `id() -> &'static str`, `display_name()`, `cadence() -> Cadence`,
  `requires() -> Requirements` (admin/none, cost class), and `collect(&mut self, ctx) -> Result<Vec<EventDraft>, CollectorError>`.
  `Cadence` is `Interval(Duration) | Notification | OnDemand`, matching the measured cost split.
- Supervisor: owns a registry of boxed collectors, runs interval collectors on a Tokio interval inside
  `spawn_blocking`, wraps each `collect` in `catch_unwind` so one collector cannot take down the app,
  records per-collector `last_run`, `last_duration`, `consecutive_failures`, `state`
  (`Running|Backoff{until}|Disabled{reason}`), and applies exponential backoff 1→2→5→15 min capped at
  15 min after 3 consecutive failures, resetting on success.
- A collector whose `requires()` is unmet (no admin, absent dependency) is registered but reported
  `Disabled{reason}` and never scheduled — the reason string is user-facing and shown in Settings and
  Diagnostics (§26-adjacent honesty requirement, without adding health-check scope).
- Snapshot/diff helper: generic over an `Entity` with a stable `id()` and a `fingerprint()`; produces
  `Added | Removed | Changed { field, before, after }` lists. First run after install seeds baselines
  and emits *no* events (§12) — proven by test, so a fresh install does not show 400 "new" processes.
- Normalizer: `EventDraft -> Event` applying, in order — category/type validation against a closed enum,
  redaction of every string field through `Sanitized<String>`, importance scoring by a table of
  `(category, event_type, signals) -> Importance`, summary generation (plain first, technical second),
  entity id derivation, and dedup key computation.
- Dedup (§58): key = `(source_collector, event_type, entity_id, changed_field, before, after)`; if an
  identical key was written within the collector's dedup window (default 60 s, per-collector override),
  the existing row's `occurrence_count` is incremented and `last_seen` updated instead of inserting.
- Collector state is persisted (`collector_state` table) so cadence, baselines and backoff survive
  restart; a corrupt baseline is discarded and re-seeded silently with no events emitted.

Non-functional:
- The normalizer is pure and synchronous; every rule is table-driven so adding a collector adds rows,
  not branches (DRY).
- A panicking collector logs one `error!` with its id, increments failures, and the app keeps running —
  asserted by a test that registers a deliberately panicking collector.
- Supervisor overhead with 10 registered interval collectors at their real cadences stays under 1 % CPU
  average, measured over 5 minutes.

## Architecture

```rust
pub enum Cadence { Interval(Duration), Notification, OnDemand }

pub trait Collector: Send {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn cadence(&self) -> Cadence;
    fn requires(&self) -> Requirements { Requirements::default() }
    fn collect(&mut self, ctx: &CollectorCtx) -> Result<Vec<EventDraft>, CollectorError>;
}
```

`CollectorCtx` hands out only what a collector may touch: the baseline store, a clock, the cancellation
token, and a `tracing` span — never the DB connection, so a collector cannot write rows directly and
every event must pass the normalizer. Notification-driven collectors do not implement a polling loop:
they receive a `Notifier` handle in construction and push into an mpsc queue the supervisor drains, so
the Win32 callback threads of Phases 10–12 never touch SQLite. `panic = "unwind"` is retained in
`Cargo.toml` release profile precisely so `catch_unwind` is meaningful; a comment states that invariant
next to the profile.

## Related Code Files

Create:
- `src-tauri/src/collectors/{mod.rs,traits.rs,supervisor.rs,registry.rs,baseline.rs,diff.rs,error.rs}`
- `src-tauri/src/normalize/{mod.rs,importance.rs,summaries.rs,entity_id.rs,dedup.rs,tables.rs}`
- `src-tauri/src/collectors/mock.rs` (cfg(test) + `dev-tools`)
- `src-tauri/src/commands/collectors.rs` (`get_collector_status`, `set_collector_enabled`)
- `src-tauri/tests/{supervisor.rs,diff_seed.rs,normalizer.rs,dedup.rs,panic_isolation.rs}`
- `src/features/settings/sections/CollectorsSection.svelte`

Modify: `src-tauri/src/storage/migrations/` (add `collector_state`, `entity_baseline`), `src-tauri/Cargo.toml` (profile comment), `ARCHITECTURE.md`, `COLLECTORS.md` (the table every later phase appends to).

## Implementation Steps

1. `traits.rs` + `error.rs`: the trait above; `CollectorError` via `thiserror` with variants `Unavailable{reason}`, `PermissionDenied`, `Transient(String)`, `Fatal(String)` — only `Fatal` counts toward disabling, `Transient` toward backoff.
2. `baseline.rs`: `BaselineStore` keyed `(collector_id, entity_kind)` storing a `HashMap<String, u64>` of id→fingerprint, serialized as a length-prefixed binary blob in `collector_state`; a version byte allows discarding an unreadable baseline. `load_or_seed` returns `Seeded` on first run so the caller suppresses events.
3. `diff.rs`: `diff(prev: &Baseline, next: &[E]) -> DiffResult` returning added/removed ids and changed fingerprints; a second pass asks the collector for field-level detail only for changed ids (avoids formatting cost for the 99 % unchanged rows — this is what keeps the measured 6.2 ms process snapshot cheap).
4. `supervisor.rs`: `Supervisor::new(registry, sender)`; one Tokio task per interval collector using `tokio::time::interval` with `MissedTickBehavior::Delay`; each tick runs `spawn_blocking(|| catch_unwind(AssertUnwindSafe(|| c.collect(ctx))))`; results flow to the normalizer then the storage writer, then the coalescing bus. Cancellation on app exit awaits in-flight ticks with a 2 s timeout.
5. `tables.rs`: `IMPORTANCE_RULES: &[(Category, &str, ImportanceRule)]` and `SUMMARY_TEMPLATES` with named placeholders; both are plain data, unit-tested for total coverage — a test iterates the closed `(category, event_type)` enum product used by the registry and fails if a pair has no rule or no template, so a new collector cannot ship un-summarized.
6. `importance.rs`: rule evaluation with signal inputs (`is_system_path`, `is_signed`, `is_security_relevant`, `entity_is_watched`, `version_downgrade`); returns one of the 6 §54 levels. Test fixture: 24 representative drafts with expected levels.
7. `summaries.rs`: builds `user_friendly_summary` (§59 — no jargon, no hex, no registry path in the plain line) and `technical_summary` (full detail). A test asserts the plain line for 24 fixtures contains none of the forbidden tokens (`HKEY_`, `0x`, `\\Device\\`, `GUID`, `errno`).
8. `entity_id.rs`: deterministic ids — `process:{lowercased exe name}`, `service:{service name}`, `port:{proto}/{port}`, `driver:{device instance id}`, `app:{publisher}/{name}`, `file:{normalized path}`; a test asserts stability across two runs and case/separator normalization.
9. `dedup.rs`: computes the key, then a storage call `upsert_or_increment(key, window)`; test writes the same draft 5× within the window (1 row, `occurrence_count = 5`) and once after it (2 rows).
10. `mock.rs`: a collector emitting a deterministic sequence including one panic on the 3rd call, used by `panic_isolation.rs` to assert the app survives, backoff engages, and a later success resets it.
11. `commands/collectors.rs` + `CollectorsSection.svelte`: list id, display name, state, last run, last duration, and the disabled reason; a toggle per collector persisted in settings. Disabled-by-requirement collectors show the reason and a disabled toggle.

## Todo

- [ ] `Collector` trait + `CollectorError` severity semantics
- [ ] baseline store with seed-without-events on first run
- [ ] generic diff with lazy field-level detail
- [ ] supervisor: intervals, `catch_unwind`, backoff, cancellation
- [ ] table-driven importance + summaries with total-coverage test
- [ ] deterministic entity ids
- [ ] dedup with occurrence counting
- [ ] mock collector + panic isolation test
- [ ] collector status command + Settings section

## Success Criteria

- [ ] First supervisor run against a seeded baseline emits 0 events; the second run with one injected change emits exactly 1.
- [ ] A collector that panics does not abort the process; its state becomes `Backoff`, and a subsequent success returns it to `Running`.
- [ ] `cargo test normalizer` proves every `(category, event_type)` pair has an importance rule and both summaries.
- [ ] No plain-language summary in the 24-fixture set contains `HKEY_`, `0x`, `\\Device\\`, `GUID`, or `errno`.
- [ ] The same draft repeated 5× inside the dedup window yields one row with `occurrence_count = 5`.
- [ ] `grep -rn "Connection" src-tauri/src/collectors/` returns no match — collectors cannot reach SQLite.

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml supervisor diff_seed normalizer dedup panic_isolation
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
grep -rn "rusqlite" src-tauri/src/collectors/ ; test $? -eq 1
```

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| `catch_unwind` silently useless because a future release profile sets `panic = "abort"` | Low×High | `panic_isolation` test fails | the test *is* the guard; the profile comment explains why; never change the profile to satisfy binary size |
| Baseline blob format changes break upgrades | Med×Med | version byte mismatch after update | discard + re-seed with no events; this is the designed behavior, documented so a "quiet first hour after update" is expected, not a bug |
| Importance table becomes a dumping ground of special cases | Med×Low | rule count grows past ~40 with overlapping conditions | rules stay data; if signals multiply, add a signal rather than a branch, and keep the fixture set growing with it |
| Backoff hides a permanently broken collector | Med×Med | `consecutive_failures` keeps climbing | after 3 backoffs at the 15 min cap, transition to `Disabled{reason}` and surface it in Settings — never fail silently |
| Supervisor CPU cost exceeds budget with all collectors | Med×High | 5-minute CPU measurement over 1 % | move the expensive collectors (drivers 2 128 ms, tasks 1 326 ms, network 1 441 ms) to `Notification`/`OnDemand` — the measured costs already dictate that split; escalate with numbers before changing the budget |

## Security Considerations

`CollectorCtx` is the isolation boundary: no DB handle, no arbitrary filesystem access, no command
execution. Because every draft passes the normalizer, and the normalizer redacts before storage, a
collector that accidentally captures a token in a path or a command line cannot persist it. `CollectorError`
messages are treated as untrusted strings and are redacted before being written to `collector_state` or
shown in Settings, so a Win32 error text containing a user name is sanitized like any other value.
```

### FILE: phase-10-process-port-service-collectors.md

```markdown
---
phase: 10
title: "Process, Listening-Port & Service Collectors"
status: pending
priority: P1
effort: 13h
dependencies: [09]
---

## Overview

The three collectors that answer "what is running that wasn't" (§14), "what is listening that wasn't"
(§15) and "which services changed" (§16). All three are cheap by measurement (process snapshot 6.2 ms
warm, services 3.3 ms), so all three are `Interval` collectors, with the service collector additionally
subscribing to `NotifyServiceStatusChangeW` so state flips are caught between ticks. This phase also
implements the honest handling of blocker **B3**: without admin there is no reliable process *event*
stream, so a process that starts and exits inside one interval is invisible, and the app says so.

## Requirements

Functional:
- Process collector: enumerate with `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` +
  `Process32FirstW`/`Process32NextW`; for each pid open with `PROCESS_QUERY_LIMITED_INFORMATION` and read
  the image path via `QueryFullProcessImageNameW`. Emit `process/started` and `process/stopped` on diff.
  Entity id is `process:{exe name lowercased}` so restarts of the same program group into one entity.
- Enrichment, best-effort and cached by path: file version info
  (`GetFileVersionInfoSizeW`/`GetFileVersionInfoW`/`VerQueryValueW` for `CompanyName`, `ProductName`,
  `FileVersion`) and signature status via `WinVerifyTrust` with `WINTRUST_ACTION_GENERIC_VERIFY_V2`.
  Results are cached keyed by `(path, size, mtime)` because `WinVerifyTrust` is the expensive call.
- The measured fact that **~46 % of processes deny the image path** to a non-admin caller is designed
  for, not worked around: such entries fall back to `process:{Process32 exe name}`, are marked
  `path_available = false`, get `signature = Unknown`, and their importance is capped at `notable` since
  we cannot verify them. `COLLECTORS.md` documents this as a known limitation with the measured share.
- **B3 mitigation** (no admin process events): three secondary signals raise a `low_confidence` short-lived
  process indication — (a) a listening port whose owning pid is gone at the next tick, (b) an Application
  Error / event id 1000 record naming an image we never saw start, (c) a service state transition without a
  matching process observation. Each produces an event with `confidence = "inferred"` rendered as
  "likely" in the UI, never as fact.
- Port collector: `GetExtendedTcpTable(TCP_TABLE_OWNER_PID_LISTENER)` and
  `GetExtendedUdpTable(UDP_TABLE_OWNER_PID)` for both `AF_INET` and `AF_INET6`; emit `network/opened` and
  `network/closed` keyed `port:{proto}/{port}`; resolve the owning process through the process
  collector's cache (shared read-only snapshot, not a second enumeration) and include it in the summary
  ("Port 25565 is now open, used by javaw.exe").
- Service collector: `OpenSCManagerW(SC_MANAGER_ENUMERATE_SERVICE|CONNECT)` +
  `EnumServicesStatusExW(SC_ENUM_PROCESS_INFO)` for state/pid, and `QueryServiceConfigW` per service for
  start type and binary path (read on first sight and on change only, since config rarely changes).
  Emit `service/started`, `service/stopped`, `service/start_type_changed`, `service/installed`,
  `service/removed`. Additionally register `NotifyServiceStatusChangeW` for `SERVICE_NOTIFY_CREATED|DELETED`
  on the SCM handle so installs/removals surface promptly.
- Importance rules: a new *service* set to `Automatic` outranks a new process; an unsigned binary in a
  user-writable directory outranks a signed one in `System32`; a listening port on `0.0.0.0` outranks
  one on `127.0.0.1`.

Non-functional:
- Process + port tick together on a 5 s interval (well above the measured 6.2 ms cost) and share one
  `spawn_blocking` call, so we pay one snapshot per tick, not two.
- Services tick at 10 s; `QueryServiceConfigW` is not called for unchanged services.
- No collector holds a handle across ticks except the SCM handle used for notifications; every opened
  process/snapshot handle is closed on every path including error paths (RAII wrappers).

## Architecture

All Win32 calls live in `src-tauri/src/win32/` behind safe wrappers that return owned Rust types, so the
collectors themselves contain no `unsafe`. Each wrapper family is one file (`process.rs`, `version_info.rs`,
`trust.rs`, `tcpip.rs`, `services.rs`) with an RAII handle guard (`OwnedHandle`-style newtype whose `Drop`
calls `CloseHandle`/`CloseServiceHandle`/`SetupDiDestroy…`). Buffer-sizing follows one shared pattern:
call with a null buffer to get the required size, allocate, call again, and retry up to 3 times on
`ERROR_INSUFFICIENT_BUFFER` because the set can grow between calls. The enrichment cache is an
`LruCache`-shaped `HashMap` + `VecDeque` (hand-rolled, ~30 lines, no dependency) capped at 512 entries.
The shared process snapshot is published as an `Arc<ProcessSnapshot>` the port collector reads, so
ownership resolution never re-enumerates.

## Related Code Files

Create:
- `src-tauri/src/win32/{mod.rs,handle.rs,process.rs,version_info.rs,trust.rs,tcpip.rs,services.rs,widestring.rs}`
- `src-tauri/src/collectors/process.rs`, `ports.rs`, `services.rs`
- `src-tauri/src/collectors/inference.rs` (B3 secondary signals)
- `src-tauri/src/cache/enrichment.rs`
- `src-tauri/tests/{win32_process.rs,win32_tcpip.rs,win32_services.rs,collector_process.rs,collector_ports.rs,collector_services.rs,inference.rs}`
- `spikes/verify-privileges.ps1` (re-runs the three FAIL rows to confirm the matrix on the dev machine)

Modify: `src-tauri/Cargo.toml` (`windows` features: `Win32_System_Diagnostics_ToolHelp`, `Win32_System_Threading`, `Win32_Storage_FileSystem`, `Win32_Security_WinTrust`, `Win32_NetworkManagement_IpHelper`, `Win32_System_Services`), `collectors/registry.rs`, `normalize/tables.rs` (new rules + templates), `COLLECTORS.md`.

## Implementation Steps

1. `win32/widestring.rs`: `to_wide(&str) -> Vec<u16>` and `from_wide_nul(&[u16]) -> String` (stops at the first NUL, lossy-decodes invalid UTF-16). Unit-tested with a path containing non-ASCII and an unterminated buffer.
2. `win32/handle.rs`: `SafeHandle`, `ScHandle`, `SnapshotHandle` guards; a test asserts `Drop` runs by observing handle-count stability across 10 000 open/close cycles (`GetProcessHandleCount` on our own process).
3. `win32/process.rs`: `snapshot() -> Vec<ProcessRow { pid, parent_pid, exe_name, image_path: Option<String> }>`. Confirm the `windows` 0.62.2 signatures against the vendored source before writing calls (API-shape rule). Time the call in a test and assert < 50 ms cold to catch a pathological regression against the measured 6.2 ms warm.
4. `win32/version_info.rs` + `trust.rs`: `file_version_info(path)` and `signature_status(path) -> Signed{subject} | Unsigned | Unknown`. `WinVerifyTrust` uses `WTD_UI_NONE`, `WTD_REVOKE_NONE`, `WTD_CHOICE_FILE`, `WTD_STATEACTION_VERIFY` followed by a `WTD_STATEACTION_CLOSE` call in all paths (a leak here is a real handle leak, so it is a `defer`-style guard, not a happy-path call).
5. `collectors/process.rs`: uses the diff helper with `fingerprint = hash(pid_set_membership, image_path)`; enrichment only for added entities; caps importance when `path_available = false`.
6. `win32/tcpip.rs`: both table calls for both families, returning `Vec<ListenerRow { proto, local_addr, port, pid }>`; a test asserts our own process appears when the test binds a temporary listener on `127.0.0.1:0`.
7. `collectors/ports.rs`: diff on `(proto, port, local_addr)`; summary uses the shared process snapshot; `0.0.0.0`/`::` bindings get the higher importance rule.
8. `win32/services.rs`: enumeration + config read + a `ServiceNotifier` that owns the SCM handle and a dedicated thread running `NotifyServiceStatusChangeW` with a callback pushing into an mpsc sender. The callback does nothing but send — no allocation-heavy work, no SQLite (enforced by the `CollectorCtx` design).
9. `collectors/services.rs`: diff on `(name, state, start_type, binary_path)`; `installed`/`removed` from set membership; config read lazily.
10. `collectors/inference.rs`: the three B3 signals, each with a `confidence = "inferred"` draft and a summary phrased as a likelihood ("A program was likely started and stopped between checks"); a test feeds a synthetic sequence (port present with pid 1234 → pid absent) and asserts exactly one inferred event with the expected wording.
11. `spikes/verify-privileges.ps1`: read-only re-run of the matrix's three failing rows (`Win32_ProcessStartTrace` subscribe, `Security` log read, Docker pipe existence) printing PASS/FAIL, so a machine with different privileges is detected rather than assumed. Its output is pasted into `COLLECTORS.md`, not into code.
12. Append the three collectors to `COLLECTORS.md` with: what it detects, cadence, measured cost, admin requirement, known blind spots (including the 46 % path-denial share and the B3 gap).

## Todo

- [ ] `win32` wrapper layer with RAII handles and wide-string helpers
- [ ] process snapshot + enrichment cache + path-denial handling
- [ ] listening-port tables (TCP/UDP, v4/v6) with owner resolution
- [ ] service enumeration + lazy config + SCM change notifications
- [ ] B3 inference signals marked `inferred`
- [ ] privilege re-verification spike script
- [ ] `COLLECTORS.md` rows with measured costs and blind spots

## Success Criteria

- [ ] Launching Notepad produces exactly one `process/started` within 5 s; closing it produces one `process/stopped`.
- [ ] `python -m http.server 8000` produces one `network/opened` naming the owning process; stopping it produces one `network/closed`.
- [ ] `sc.exe config <a benign service> start= demand` (run manually by the developer) produces one `service/start_type_changed` with correct before/after.
- [ ] Processes whose path is denied are recorded with `path_available = false`, `signature = Unknown`, importance ≤ notable, and no error is logged for the denial (it is expected, not exceptional).
- [ ] Handle count of our process is stable (±2) after 5 minutes of ticking — no handle leak.
- [ ] A process that starts and exits between ticks produces either nothing or one `inferred` event, never a confident claim.

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml win32_ collector_process collector_ports collector_services inference
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
powershell -NoProfile -File spikes/verify-privileges.ps1
```
Manual: run the app, perform the four actions above, confirm the timeline rows and their wording.

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| `windows` 0.62.2 signatures differ from what this plan assumes (e.g. `PWSTR` vs `&mut [u16]`, `BOOL` vs `Result<()>`) | High×Low | compile error | expected; the API-shape rule applies — read the vendored crate source, follow it, note any deviation in `docs/decisions.md`. No fallback needed, only discipline |
| `WinVerifyTrust` cost dominates the tick | Med×Med | tick duration > 100 ms | already cached by `(path, size, mtime)`; if still hot, move signature verification to a lazy on-demand path invoked when an event is opened in Details |
| Handle or memory leak in the unsafe layer | Med×High | handle count climbs in the 5-minute test | the RAII guards are the design; the leak test is a gate, not an afterthought — do not ship with it failing |
| B3 inference produces false "likely" events | Med×Med | inferred events outnumber real ones in a quiet hour | require two of the three signals to agree before emitting, and keep confidence wording; if still noisy, demote inferred events to informational so they never drive analysis |
| Some service names/paths contain user data | Low×Med | a service binary path under `C:\Users\<name>` | redaction already normalizes user profile paths; verified by the Phase 05 chokepoint test |

## Security Considerations

Every `unsafe` block is confined to `src-tauri/src/win32/`, with a module-level comment stating the
invariant each call relies on (buffer sized from the API's own required-size call, handle valid for the
call's duration). No privileged operation is attempted: we request `PROCESS_QUERY_LIMITED_INFORMATION`
rather than `PROCESS_QUERY_INFORMATION`, and `SC_MANAGER_ENUMERATE_SERVICE|CONNECT` rather than a
write-capable SCM handle, so the app cannot alter services even if compromised. Command lines are
deliberately **not** collected — they are the most common carrier of secrets, and §14 does not ask for
them. Image paths and service binary paths pass through redaction before storage.
```

### FILE: phase-11-file-config-and-application-collectors.md

```markdown
---
phase: 11
title: "Watched-File / Config Collector & Application Inventory"
status: pending
priority: P1
effort: 11h
dependencies: [05, 09]
---

## Overview

Two collectors that produce the highest-signal rows in the product: watched configuration files with a
real before/after diff (§13), and the installed-application inventory with install/uninstall/update
detection (§17). The file collector is the one place the app stores file *content*, so it is the phase
where redaction and size limits are load-bearing rather than precautionary. The application inventory is
a cheap registry scan (measured ~50 ms) made event-driven by `RegNotifyChangeKeyValue`.

## Requirements

Functional — watched files (§13):
- The user picks files and folders to watch (Settings → Watched Files, plus drag-and-drop onto the
  window). Defaults are **empty**: the app watches nothing until asked, and the onboarding step offers
  three optional presets (a chosen project folder, `%APPDATA%` config subset, and the hosts file).
- Watching uses `notify` 8.2.0 with `notify-debouncer-full` 0.7.0 at a 500 ms debounce, recursive for
  folders, with an include/exclude filter: excluded by default are `node_modules`, `.git`, `target`,
  `dist`, `build`, `__pycache__`, `*.log`, `*.tmp`, `*.lock`, and any path over the size cap.
- Events: `filesystem/created`, `filesystem/modified`, `filesystem/deleted`, `filesystem/renamed`.
  For a text file under **256 KB**, store a unified-diff-style before/after using the previous stored
  snapshot; over that cap, store only size and mtime change and say so in the summary.
- Content snapshots are stored in a separate `file_snapshot` table with the same retention as events,
  and the **whole content passes through redaction** — the `.env`-style key/value redactor replaces
  values of keys matching `(?i)(pass|pwd|secret|token|key|cred|auth|conn)` while keeping key names, so a
  diff still shows *that* a secret changed without showing it.
- A watched path that disappears (drive unplugged, folder deleted) does not spam: one event, then the
  watch is marked `unavailable` and retried every 60 s, silently, until it returns.
- Binary files are detected by a NUL byte in the first 8 KB and never diffed.

Functional — applications (§17):
- Scan all three uninstall locations: `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`,
  `HKLM\SOFTWARE\WOW6432Node\...\Uninstall`, and `HKCU\SOFTWARE\...\Uninstall`, reading
  `DisplayName`, `DisplayVersion`, `Publisher`, `InstallDate`, `InstallLocation`, `SystemComponent`,
  `ReleaseType`, `ParentKeyName`.
- Filter the noise honestly: entries with `SystemComponent = 1`, an empty `DisplayName`, or a
  `ParentKeyName` (update entries) are inventoried but not surfaced as user-facing installs; MSI patch
  rows (`ReleaseType = Security Update|Update Rollup`) are grouped under their parent product.
- Events: `application/installed`, `application/uninstalled`, `application/updated` (version changed),
  keyed `app:{publisher}/{display_name}`. `application/updated` carries before/after versions and is
  the row that makes "it worked before" answerable.
- Made event-driven with `RegNotifyChangeKeyValue` on each of the three keys using
  `REG_NOTIFY_CHANGE_NAME | REG_NOTIFY_CHANGE_LAST_SET | REG_NOTIFY_THREAD_AGNOSTIC`, plus a 30-minute
  safety-net rescan (notifications on registry keys are per-key and can be missed under load).

Non-functional:
- The file collector never reads a file larger than the cap and never holds a file open across ticks.
- A folder watch on a 50 000-file tree must not stall startup: the initial hash pass is chunked and
  yields, and its progress is reported in Settings.
- The application scan stays within the measured ~50 ms; the safety-net rescan is `spawn_blocking`.

## Architecture

The file collector is `Cadence::Notification`: the debouncer owns its own thread and pushes
`DebouncedEvent` batches into the supervisor's mpsc queue; the collector's `collect` drains that queue,
so no polling loop exists. Content handling is a three-step pipeline in `collectors/files/content.rs`:
`classify(path) -> Text | Binary | TooLarge`, `read_capped(path, 256 KiB)`, then
`redact_content(&str) -> Sanitized<String>` — the same `redact` module as everything else, extended with
the key/value rule. Diffs are computed by the Phase 08 LCS moved to shared code
(`src-tauri/src/diffing/line_diff.rs` for the backend, TS copy for the UI is deleted in favor of the
backend producing the diff — the UI only renders it). The registry side lives in `win32/registry.rs` with
one `RegistryNotifier` thread per watched key, each owning its `HKEY` guard and an event handle.

## Related Code Files

Create:
- `src-tauri/src/collectors/files/{mod.rs,watcher.rs,content.rs,filters.rs,snapshot_store.rs}`
- `src-tauri/src/collectors/applications.rs`
- `src-tauri/src/win32/registry.rs`
- `src-tauri/src/diffing/line_diff.rs`
- `src-tauri/src/redact/content.rs` (key/value rule; still inside the chokepoint module)
- `src-tauri/tests/{files_watch.rs,files_content_redaction.rs,files_large_tree.rs,applications.rs,registry_notify.rs,line_diff.rs}`
- `src/features/settings/sections/WatchedFilesSection.svelte`, `src/features/settings/WatchPathPicker.svelte`

Modify: `src-tauri/Cargo.toml` (`notify`, `notify-debouncer-full`, `windows` features `Win32_System_Registry`), `storage/migrations/` (`file_snapshot`, `watched_path`), `collectors/registry.rs`, `normalize/tables.rs`, `COLLECTORS.md`, `PRIVACY.md` (what file content is stored, where, and how it is redacted).

## Implementation Steps

1. `redact/content.rs`: the key/value redactor plus reuse of the Phase 05 patterns (user profile path, IPv4/IPv6 in file content is *not* redacted — it is often the answer; tokens, connection strings, and `Bearer` values are). Test corpus: a 40-line `.env`, an `appsettings.json` with a connection string, an nginx conf, a hosts file, and a YAML with an inline password.
2. `collectors/files/filters.rs`: include/exclude matching with `globset`-free simple segment matching (exclude by directory name and by extension — no new dependency needed for this ruleset); unit-tested against 20 paths.
3. `watcher.rs`: build the debouncer against the vendored `notify-debouncer-full` 0.7.0 source (API-shape rule; the entry point name and whether a `Config` is required must be read, not guessed). Per-path watch handles stored in a map so removing a watched path drops exactly one watcher.
4. `content.rs` + `snapshot_store.rs`: classification, capped read, redaction, then `store_snapshot(path, sanitized, hash)`; the previous snapshot is fetched by path for diffing, and only the newest snapshot per path plus the ones referenced by retained events are kept (retention join, exercised by the Phase 06 retention test extended here).
5. `line_diff.rs`: LCS-based unified diff with 3 lines of context, hard cap 400 changed lines, returning `{ added, removed, hunks }`. Property-tested: applying the diff to `before` reproduces `after` for 200 random pairs.
6. `collectors/files/mod.rs`: the `Collector` impl — drain, filter, classify, diff, draft. Rename events pair the two debounced paths into one `filesystem/renamed` with before/after names.
7. Unavailable-path handling: one event, then `unavailable` state with a silent 60 s retry; a test unmounts by watching a path inside a temp dir that the test deletes, and asserts exactly one event and no repeats over 3 retries.
8. `win32/registry.rs`: `open_key_read`, `enum_subkeys`, `read_string_value`, `read_dword_value`, and `RegistryNotifier` (per-key thread, `CreateEventW` + `RegNotifyChangeKeyValue` + `WaitForSingleObject`, re-arming after each signal because the notification is one-shot).
9. `collectors/applications.rs`: three-location scan into `AppRow`, filtering rules above, diff on `(display_name, publisher, version)`; `application/updated` when only version differs; a test uses a fixture HKCU subtree the test itself creates under `HKCU\Software\WhatChangedTest` (user-writable, no admin) and deletes afterward, proving install/update/uninstall detection against a real registry rather than a mock.
10. `WatchedFilesSection.svelte` + `WatchPathPicker.svelte`: add via the Tauri dialog plugin **or** drag-and-drop; list with size/state/last change; remove; per-path "diff content" toggle so a user can watch a file's existence without storing its content. The picker is the only place the frontend names a filesystem path, and it passes an opaque handle-free absolute path to a narrow `add_watched_path(path)` command that validates existence, canonicalizes, rejects UNC and reparse-point traversal outside the chosen root, and rejects the app's own data directory.
11. Append both collectors to `COLLECTORS.md`; add the content-storage disclosure to `PRIVACY.md`.

## Todo

- [ ] content redactor + corpus test (env/json/conf/hosts/yaml)
- [ ] include/exclude filters and size/binary classification
- [ ] debounced recursive watcher with per-path handles
- [ ] snapshot store + retention join
- [ ] backend line diff with property test
- [ ] unavailable-path silent retry
- [ ] registry read + one-shot notification re-arm
- [ ] application inventory with honest noise filtering
- [ ] Watched Files settings UI + validated `add_watched_path`

## Success Criteria

- [ ] Editing a watched `.env` produces one `filesystem/modified` with a diff showing changed key names and `[redacted]` values — the real value appears nowhere in the DB (asserted by scanning the DB file for the test secret string).
- [ ] A 1 MB file change produces a size/mtime event and no stored content.
- [ ] A folder with 50 000 files can be added without blocking the UI; progress is visible and the initial pass emits no events.
- [ ] Deleting a watched folder yields exactly one event, then silence.
- [ ] Installing and uninstalling a small MSI/exe (developer-run, e.g. a portable tool with an installer) produces exactly one `application/installed` and one `application/uninstalled`.
- [ ] Changing `DisplayVersion` in the test HKCU fixture produces one `application/updated` with correct before/after.
- [ ] `RegNotifyChangeKeyValue` re-arms: two consecutive fixture changes produce two detections.

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml files_ applications registry_notify line_diff
cargo test --manifest-path src-tauri/Cargo.toml redaction_corpus
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
bun run check && VITE_WC_MOCK=1 bun run test:e2e -- tests/e2e/watched-files.spec.ts
```

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| Stored file content leaks a secret | Med×**High** | the DB-scan test finds the plaintext | this is the phase's hardest gate: the scan test must pass before merge. If a class of secret is missed, add the pattern to `redact/content.rs` and to the corpus; never store unredacted content "temporarily" |
| `notify-debouncer-full` 0.7 API differs from the assumed shape | High×Low | compile error | API-shape rule: read the vendored source. Fallback if the debouncer proves unusable: `notify` raw events + our own 500 ms coalescing in `watcher.rs` (≈40 lines), which we already need for the queue |
| Recursive watch on a huge tree exhausts handles or floods | Med×Med | event rate > 200/s or watcher error | exclusion list applies before hashing; on flood, auto-narrow the watch to the top two levels and tell the user in Settings why |
| Registry notification missed under load | Med×Med | an install seen only by the rescan | the 30-minute safety-net rescan is exactly this mitigation; timestamp uses the registry `InstallDate` when present so the timeline stays truthful |
| Diff of a large config blocks the writer thread | Low×Med | writer queue depth grows | diffing happens in the collector's `spawn_blocking`, never on the writer thread; the 400-line cap bounds it |

## Security Considerations

`add_watched_path` is the only command that accepts a path, and it is the app's widest attack surface, so
it validates: canonicalize, reject non-existent, reject UNC (`\\?\UNC`, `\\server\share`), reject paths
resolving inside the app's own data directory or `%WINDIR%\System32`, and cap the watched-path count at
64. Reads are opened read-only with sharing allowed and are never executed. The frontend can only
*enumerate* watched paths and their diffs — it cannot request arbitrary file content, because there is no
command that reads a path on demand. `PRIVACY.md` states plainly that watched-file content is stored
locally in the SQLite database after redaction, and that removing a watched path deletes its snapshots.
```

### FILE: phase-12-windows-change-intelligence.md

```markdown
---
phase: 12
title: "Windows Change Intelligence: Updates, Drivers, Tasks, Network, Event Log"
status: pending
priority: P1
effort: 15h
dependencies: [09, 11]
---

## Overview

The five system-level collectors that explain the changes users cannot see: Windows Update history (§18),
device drivers (§19), scheduled tasks (§20), network configuration (§21), and the Windows event log as a
corroborating source (§22). Their measured costs decide their design: WUA history is cheap (38.2 ms) and
polls; drivers (2 128 ms), scheduled tasks (1 326 ms) and network configuration (1 441 ms) are far too
expensive to poll often, so each is notification-driven with a rare safety-net rescan. This phase also
implements the before/after value capture that makes each of these rows actionable.

## Requirements

Functional — Windows Update (§18):
- Use WUA COM: `IUpdateSession` → `CreateUpdateSearcher` → `GetTotalHistoryCount` → `QueryHistory(0, n)`,
  reading `Title`, `Date`, `ResultCode`, `Operation`, `SupportUrl`, and the KB id parsed from the title.
- Emit `windows_update/installed`, `/failed`, `/uninstalled` keyed `kb:{id}` (or a title hash when no KB).
  Importance: a failed update is `important`; a successful one is `notable`.
- `Win32_QuickFixEngineering` is **not** used. The research measured 4 rows from WMI against 133 from
  WUA on the same machine; a note in `COLLECTORS.md` records that measurement as the reason, so nobody
  "optimizes" back to the WMI class later.
- Cadence: 15 min poll (cheap), plus an immediate poll when the event log reports a servicing event.

Functional — drivers (§19):
- Enumerate with SetupAPI: `SetupDiGetClassDevsW(NULL, NULL, NULL, DIGCF_ALLCLASSES|DIGCF_PRESENT)`,
  `SetupDiEnumDeviceInfo`, device instance id + `SPDRP_DEVICEDESC`/`SPDRP_CLASS`/`SPDRP_MFG`, and the
  driver version/date from `SetupDiOpenDevRegKey(..., DIREG_DRV)` values `DriverVersion`, `DriverDate`,
  `ProviderName`, `InfPath`.
- Emit `driver/installed`, `driver/removed`, `driver/version_changed` (before/after versions),
  `driver/device_arrived`, `driver/device_removed`, keyed `driver:{device instance id}`.
- Because a full enumeration costs 2 128 ms, cadence is `Notification`: a message-only window plus
  `CM_Register_Notification` for `CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE` device arrival/removal triggers a
  scan; a 6-hour safety-net rescan catches driver *version* changes that arrive without a device event
  (the common Windows Update driver path), and a servicing event from the event log triggers one too.
- A GPU driver version change is the flagship row and must read correctly on the dev machine.

Functional — scheduled tasks (§20):
- Task Scheduler COM: `ITaskService::Connect` (local, no credentials) → `GetFolder("\\")` → recursive
  `GetTasks`/`GetFolders`, reading `Name`, `Path`, `Enabled`, `State`, `LastRunTime`, `LastTaskResult`,
  `NextRunTime`, and the actions' `Path`/`Arguments` (arguments pass through redaction like any string).
- Emit `scheduled_task/created`, `/removed`, `/enabled`, `/disabled`, `/definition_changed`,
  keyed `task:{full path}`. `LastRunTime` alone never produces an event (it changes constantly).
- Cadence: `Interval(6 h)` given the 1 326 ms cost, plus an on-demand scan when a checkpoint is created.

Functional — network configuration (§21):
- `GetAdaptersAddresses(AF_UNSPEC, GAA_FLAG_INCLUDE_GATEWAYS|GAA_FLAG_INCLUDE_ALL_INTERFACES)` for
  adapter name/description/status, unicast addresses, gateways, and DNS servers; plus the default-route
  read used for "which adapter is the way out".
- Emit `network/configuration_changed` with a field-level before/after (`dns_servers`, `ipv4_address`,
  `default_gateway`, `adapter_status`, `mtu`), keyed `adapter:{luid}`. The DNS change from `1.1.1.1` to a
  router address is the flagship row.
- Cadence: `Notification` via `NotifyIpInterfaceChange`, `NotifyUnicastIpAddressChange` and
  `NotifyRouteChange2`, each callback coalesced through a 2 s timer before the 1 441 ms full read; plus a
  12-hour safety-net rescan.
- Local IP addresses are stored, not redacted (they are the answer, not a secret); the public IP is never
  fetched because the app makes no network requests (§ privacy stance).

Functional — event log (§22):
- `EvtSubscribe` (pull mode) on `Application` and `System` only. The measured matrix shows the
  `Security` channel is **unauthorized** without admin, so it is not subscribed; `COLLECTORS.md` records
  that as a limitation with the measurement.
- A closed allow-list of records is turned into events: Application Error (1000), Application Hang (1002),
  .NET Runtime (1026), Service Control Manager 7000/7001/7031/7034 (service start failures/crashes),
  Kernel-Power 41 (unexpected shutdown), Kernel-General 12/13 (boot/shutdown),
  Kernel-Boot 27, `Microsoft-Windows-WindowsUpdateClient` 19/20 (install success/failure),
  Disk 7/11/51, and `Ntfs` 55 (filesystem corruption).
- Rendering uses `EvtCreateRenderContext(EvtRenderContextValues)` with an explicit value path list plus
  `EvtRender`, and `EvtFormatMessage` for the human message — no XML parsing crate.
- Emit `system/crash`, `system/boot`, `system/shutdown_unexpected`, `system/disk_error`,
  `service/failed`, `windows_update/servicing`, each keyed by a stable entity where one exists.
- Backfill on first run: read the last 24 h of the allow-listed records so the timeline is not empty at
  install, marked `backfilled = 1` and excluded from live notifications.

Non-functional:
- No collector in this phase runs on the UI thread or the writer thread; COM is initialized per worker
  thread with `CoInitializeEx(COINIT_APARTMENTTHREADED)` and uninitialized on exit.
- Each expensive scan records its measured duration into `collector_state` so Diagnostics can show real
  costs on the user's machine rather than the dev machine's numbers.
- Total steady-state CPU with all five registered stays under the app's 1 % average budget, verified over
  30 minutes.

## Architecture

Two new infrastructure pieces carry this phase. `win32/com.rs` provides a `ComApartment` guard
(`CoInitializeEx` on construction, `CoUninitialize` on `Drop`) and a `com_worker(f)` helper that runs a
closure on a dedicated thread inside an apartment — every COM collector (WUA, Task Scheduler) runs through
it, so apartment rules are enforced in one place. `win32/message_window.rs` creates a hidden message-only
window (`HWND_MESSAGE` parent) with its own thread and pump, used by the device-notification registration;
its handler does nothing but send on an mpsc channel. Each collector is a thin `Collector` impl over a
pure `parse`/`diff` core that takes already-materialized rows, so the whole normalization path is testable
from fixtures without Win32 (`tests/fixtures/{wua_history.json,drivers.json,tasks.json,adapters.json,evtx_records.json}`).

## Related Code Files

Create:
- `src-tauri/src/win32/{com.rs,message_window.rs,wua.rs,setupapi.rs,taskscheduler.rs,ipHelper_config.rs,eventlog.rs}`
- `src-tauri/src/collectors/{windows_update.rs,drivers.rs,scheduled_tasks.rs,network_config.rs,event_log.rs}`
- `src-tauri/src/collectors/eventlog_allowlist.rs`
- `src-tauri/tests/{wua.rs,drivers.rs,tasks.rs,network_config.rs,eventlog.rs,com_apartment.rs}`
- `src-tauri/tests/fixtures/*.json`
- `spikes/measure-collector-costs.ps1`

Modify: `src-tauri/Cargo.toml` (`windows` features `Win32_System_Com`, `Win32_System_UpdateAgent`, `Win32_Devices_DeviceAndDriverInstallation`, `Win32_System_TaskScheduler`, `Win32_NetworkManagement_IpHelper`, `Win32_System_EventLog`, `Win32_UI_WindowsAndMessaging`), `collectors/registry.rs`, `normalize/tables.rs`, `COLLECTORS.md`.

## Implementation Steps

1. `win32/com.rs` + `com_apartment.rs` test: assert nested initialization is refused, that `Drop` uninitializes, and that a panic inside `com_worker` still uninitializes (the guard is on the stack, unwinding runs it — this is the second reason `panic = "unwind"` matters).
2. `win32/wua.rs`: the `IUpdateSession` chain. The exact `windows` 0.62.2 module path and interface names for WUA must be read from the vendored crate before writing the call (API-shape rule). Fallback if the feature is unavailable in the pinned crate: create the session via `CoCreateInstance` with the documented CLSID/IID through `IDispatch`-free `windows::core::Interface` casting, still inside `win32/wua.rs`, and record the deviation in `docs/decisions.md`. `QueryHistory` is paged 25 at a time.
3. `collectors/windows_update.rs`: parse KB from the title with `regex` `(?i)\bKB(\d{6,8})\b`; diff by `(kb_or_hash, operation, result_code)`; the timeline timestamp is WUA's own `Date`, not the scan time, so history lands where it happened.
4. `win32/setupapi.rs`: enumeration + `SetupDiOpenDevRegKey` reads, with an RAII `DevInfoSet` guard calling `SetupDiDestroyDeviceInfoList`. Time it in a test and assert < 5 s (measured 2 128 ms) so a pathological machine is detected rather than hanging a tick.
5. `win32/message_window.rs` + `CM_Register_Notification` registration; unregister on shutdown. A test asserts the thread exits and the window is destroyed within 2 s of cancellation.
6. `collectors/drivers.rs`: notification-driven scan + 6 h safety net; diff on `(instance_id, driver_version, driver_date, present)`; `version_changed` before/after is the flagship field.
7. `win32/taskscheduler.rs`: `ITaskService` recursive walk inside `com_worker`; action arguments are captured then redacted by the normalizer. `collectors/scheduled_tasks.rs` diffs on a definition fingerprint that deliberately **excludes** `LastRunTime`/`NextRunTime`, with a unit test proving a run-time-only change produces no event.
8. `win32/ipHelper_config.rs`: `GetAdaptersAddresses` with the grow-buffer pattern; DNS servers read from the adapter's `FirstDnsServerAddress` chain. `collectors/network_config.rs` registers the three notification APIs, coalesces with a 2 s debounce, then does the full read; diff is field-level so the event says exactly which field changed.
9. `collectors/event_log.rs` + `eventlog_allowlist.rs`: the allow-list as data (`provider`, `event_id`, `category`, `event_type`, `importance`, `summary_template`); `EvtSubscribe` in `EvtSubscribeToFutureEvents` mode plus a one-time 24 h backfill query with `EvtQuery`; render values by an explicit path list; `EvtFormatMessage` for the message with a graceful fallback to the template when the provider's message DLL is missing (common and expected).
10. Wire the cross-triggers: a `windows_update/servicing` event triggers an immediate WUA poll and a driver rescan (this is how a Windows-Update-delivered driver change is caught promptly without polling SetupAPI).
11. `spikes/measure-collector-costs.ps1`: re-measures the four expensive operations on the developer's machine and writes `docs/perf/collector-costs.md`, so the cadence choices are re-validated rather than inherited.
12. Append all five collectors to `COLLECTORS.md` with cadence, measured cost, admin requirement, and the two recorded limitations (no `Security` channel; WMI QFE rejected with the 4-vs-133 measurement).

## Todo

- [ ] COM apartment guard + worker helper (+ panic-safe uninit test)
- [ ] WUA history collector keyed by KB, timestamped by WUA's own date
- [ ] SetupAPI driver enumeration + device notifications + 6 h safety net
- [ ] Task Scheduler walk with run-time-excluded fingerprint
- [ ] adapter/DNS/route notifications with 2 s coalescing and field-level diff
- [ ] event-log allow-list, subscription, 24 h backfill, message fallback
- [ ] servicing-event cross-triggers
- [ ] cost re-measurement script + `COLLECTORS.md` rows

## Success Criteria

- [ ] WUA history returns the same order of magnitude as the research measurement (≈133 rows on the dev machine) and produces `windows_update/installed` rows dated by their real install dates.
- [ ] Changing the machine's DNS server produces exactly one `network/configuration_changed` naming `dns_servers` with correct before/after, within 10 s.
- [ ] Plugging and unplugging a USB device produces `driver/device_arrived` and `driver/device_removed` within 10 s, with no full-enumeration poll in between (verified by the recorded scan count).
- [ ] Creating and deleting a scheduled task (`schtasks /create`, user-level, developer-run) is detected by the next scan or an on-demand scan; a task merely *running* produces no event.
- [ ] Killing a process to force an Application Error 1000 record produces one `system/crash` row with a readable message.
- [ ] First run backfills the last 24 h of allow-listed records, marked `backfilled`, with no tray notifications fired.
- [ ] 30-minute idle run: average CPU < 1 %, and Diagnostics shows real measured durations per collector.

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml com_apartment wua drivers tasks network_config eventlog
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
powershell -NoProfile -File spikes/measure-collector-costs.ps1
```
Manual: DNS change, USB plug/unplug, `schtasks` create/delete, forced crash — confirm the six rows above.

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| WUA COM bindings absent/renamed in `windows` 0.62.2 | Med×Med | no such module/interface | documented fallback: `CoCreateInstance` with the CLSID/IID inside the same wrapper file; if that also fails, WUA becomes the one collector shipped behind a "not available on this build" reason string — never faked from `Win32_QuickFixEngineering`, whose 4-vs-133 gap makes it worse than nothing |
| `CM_Register_Notification` callback contract violated (blocking work in callback) | Med×High | deadlock or missed notifications | the callback only sends on a channel; enforced by review and by the callback file containing no other call |
| Driver rescan (2.1 s) triggered in a storm (dock/undock) | Med×Med | > 3 rescans/minute | coalesce device notifications through the same 2 s debounce as network, and rate-limit rescans to one per 60 s |
| `EvtFormatMessage` yields empty text for some providers | High×Low | blank message in tests | expected; the allow-list template is the fallback and is already required, so the row is always readable |
| Task Scheduler arguments contain credentials | Med×High | a task action with `-p <password>` | arguments pass through the same redactor as file content; the Phase 05 chokepoint plus the corpus test cover it |
| Cadence choices wrong on a slower machine | Med×Med | re-measurement shows > 2× the research numbers | cadences live in one table (`collectors/registry.rs`); adjust the interval, keep the notification design, and record the machine's numbers in `docs/perf/` |

## Security Considerations

Nothing in this phase requests write access: SetupAPI is used read-only, the Task Scheduler connection is
local and unauthenticated, WUA is only queried for history (never `IUpdateInstaller`), and the event log is
subscribed read-only to two non-privileged channels. The deliberate omission of the `Security` channel is
recorded as a measured limitation rather than worked around with elevation, keeping the app's non-admin
promise intact. Every string crossing from Win32 into the app — task arguments, driver INF paths, event
messages — is untrusted input and passes the redaction chokepoint before storage or display; event-log
messages in particular are provider-controlled text and are never rendered as markup.
```

### FILE: phase-13-checkpoints-and-comparison.md

```markdown
---
phase: 13
title: "Checkpoints & Two-Point Comparison"
status: pending
priority: P1
effort: 8h
dependencies: [06, 07]
---

## Overview

Checkpoints (§25) are the user's "it worked here" marker, and comparison (§47) is the feature that turns
the marker into an answer: everything that changed between a checkpoint and now, or between two
checkpoints. This is the smallest phase that delivers the product's core promise, and it depends only on
storage and query — so it can be built and demonstrated before any collector is complete, using the
Phase 07 synthetic seeder.

## Requirements

Functional:
- Create a checkpoint from the tray menu, the dashboard, a keyboard shortcut, and automatically before a
  detected high-impact operation is not attempted (no hooking) — automatic checkpoints are instead created
  daily at first launch of the day, and immediately after a `windows_update/installed` or
  `driver/version_changed` event, because those are the moments users later want to compare against.
- A checkpoint stores: `id`, `created_at`, `label` (user text, default "Working state — {date time}"),
  `note` (optional), `kind` (`manual|automatic`), `auto_reason`, and a **snapshot manifest**: the current
  entity baselines for every registered collector at that moment (processes, listeners, services, drivers,
  applications, scheduled tasks, network configuration, watched-file hashes).
- Manifests are stored as one row per entity in `checkpoint_entity(checkpoint_id, kind, entity_id,
  fingerprint, display_name, detail_json)` so comparison is a SQL set difference rather than a blob diff,
  and retention can delete a checkpoint's rows in one statement.
- Creating a checkpoint triggers an on-demand scan of the expensive collectors (drivers, tasks, network)
  so the manifest is current — the UI shows progress ("Capturing system state… 3 of 8") and the operation
  is cancellable. Measured worst case is ≈5 s (2 128 + 1 326 + 1 441 ms), which the UI must state.
- Comparison (`compare(from, to)` where each side is a checkpoint id or `Now`): returns
  `appeared`, `disappeared`, `changed{field, before, after}` grouped by entity kind, plus the count and
  the list of events recorded between the two timestamps, and an "unmonitored gap" flag if the app was not
  running for part of the interval (derived from the Phase 06 session table).
- Comparison view UI: two pickers (checkpoint / Now), a grouped result list with counts per group, a
  filter for "only important", each row expandable into before/after, and a link from any row into the
  timeline filtered to that entity. Empty result renders as "Nothing changed in the areas being
  monitored" plus the unmonitored-gap warning when applicable — never a bare "No differences".
- Checkpoints are listed in a `Checkpoints` screen with label, kind, time, entity count, and actions
  (compare with now, compare with another, rename, delete). Deleting is confirmed and irreversible.
- Retention: manual checkpoints are never auto-deleted; automatic checkpoints keep the newest 30 and are
  pruned by the Phase 06 retention pass.

Non-functional:
- A manifest for a typical machine (≈400 processes, ≈300 services, ≈900 drivers, ≈200 applications,
  ≈120 tasks, ≈8 adapters) is ≈2 000 rows; capture must complete in one transaction and the whole
  checkpoint must add < 1 MB to the database.
- `compare` for two such checkpoints returns in < 200 ms (measured in the perf harness with the 100 k
  fixture plus two real manifests).
- Comparison is read-only and never mutates events.

## Architecture

Comparison is expressed as one SQL statement per entity kind using a `FULL OUTER JOIN` emulation
(SQLite has no `FULL OUTER JOIN` before 3.39; the bundled amalgamation is newer, but the query is written
as two `LEFT JOIN`s unioned so it works either way — verified by a test that prints
`sqlite_version()`). `Now` is materialized by writing a transient checkpoint with `kind = 'transient'`
that is deleted at the end of the comparison transaction, so exactly one code path exists for both cases
(DRY). The on-demand scan reuses the supervisor: `Supervisor::scan_now(kinds) -> ScanReport` with a
progress channel the UI consumes through a `checkpoint-progress` event.

## Related Code Files

Create:
- `src-tauri/src/storage/{checkpoints.rs,compare.rs}`
- `src-tauri/src/commands/checkpoints.rs` (`create_checkpoint`, `list_checkpoints`, `rename_checkpoint`, `delete_checkpoint`, `compare_checkpoints`)
- `src-tauri/src/collectors/supervisor_scan.rs` (`scan_now` + progress)
- `src-tauri/tests/{checkpoint_capture.rs,compare.rs,checkpoint_retention.rs}`
- `src/features/checkpoints/{CheckpointsView.svelte,CheckpointCard.svelte,CreateCheckpointDialog.svelte,CaptureProgress.svelte}`
- `src/features/compare/{CompareView.svelte,CheckpointPicker.svelte,DiffGroup.svelte,DiffRow.svelte,UnmonitoredGapNotice.svelte}`
- `tests/e2e/{checkpoints.spec.ts,compare.spec.ts}`

Modify: `storage/migrations/` (`checkpoint`, `checkpoint_entity`), `storage/retention.rs`, `src/routes.ts`, tray menu (Phase 04), `src/lib/tauri/mock-transport.ts`, `ARCHITECTURE.md`.

## Implementation Steps

1. Migration: `checkpoint(id TEXT PK, created_at INTEGER, label TEXT, note TEXT, kind TEXT, auto_reason TEXT)` and `checkpoint_entity(checkpoint_id TEXT, kind TEXT, entity_id TEXT, fingerprint TEXT, display_name TEXT, detail_json TEXT, PRIMARY KEY(checkpoint_id, kind, entity_id))` with an index on `(checkpoint_id, kind)`.
2. `supervisor_scan.rs`: `scan_now(kinds, progress_tx, cancel)` running the requested collectors' snapshot half only (no drafts, no events) and returning materialized entity rows; cancellation is checked between collectors.
3. `checkpoints.rs`: `create(label, note, kind, auto_reason)` — start transaction, insert the checkpoint, insert entity rows in batches of 500, commit; on cancellation roll back so a partial manifest never exists.
4. `compare.rs`: per-kind `appeared/disappeared/changed` queries plus `events_between(from_ts, to_ts)` (count + first 200 by importance then time) plus `unmonitored_gap(from_ts, to_ts)` from the session table. Returns `ComparisonResult { groups: Vec<DiffGroup>, event_count, events: Vec<EventSummary>, gap: Option<GapInfo> }`.
5. `commands/checkpoints.rs`: validation — label ≤ 120 chars, note ≤ 2 000 chars, both redacted before storage (a user may paste a connection string into a note); `compare_checkpoints(from: Side, to: Side)` where `Side = Checkpoint(String) | Now`.
6. Automatic checkpoints: a daily-first-launch rule in the app lifecycle, and a normalizer post-hook that requests one after `windows_update/installed` or `driver/version_changed` (debounced to one per hour so an update storm produces one checkpoint, not twelve).
7. `CheckpointsView.svelte` + `CheckpointCard.svelte`: list, actions, and a destructive-delete confirmation naming the label.
8. `CreateCheckpointDialog.svelte` + `CaptureProgress.svelte`: label prefilled, note optional, live progress from `checkpoint-progress`, a stated "this takes a few seconds" line, and a cancel button that actually cancels.
9. `CompareView.svelte`: two pickers with `Now` as a first-class option, grouped results with per-group counts, "only important" toggle, expandable rows via `DiffRow.svelte`, deep-link into the timeline by entity id, and `UnmonitoredGapNotice.svelte` when the app was off.
10. Perf + retention tests: `compare.rs` builds two 2 000-row manifests and asserts < 200 ms and correct set arithmetic (including an entity that changed *and* one that appeared with the same display name); `checkpoint_retention.rs` asserts 31 automatic checkpoints prune to 30 and that manual ones survive.
11. E2E with the mock transport: create → compare with Now → expand a row → jump to the timeline; and the empty-result wording.

## Todo

- [ ] checkpoint + manifest schema and batched capture transaction
- [ ] `scan_now` with progress and real cancellation
- [ ] comparison queries incl. unmonitored-gap detection
- [ ] automatic checkpoint rules (daily, post-update, debounced)
- [ ] Checkpoints screen with rename/delete confirmation
- [ ] Compare screen with grouped diff and deep links
- [ ] perf (<200 ms) and retention tests

## Success Criteria

- [ ] Creating a checkpoint on the dev machine captures all eight entity kinds, completes in < 8 s, shows progress, and adds < 1 MB to the DB.
- [ ] Cancelling mid-capture leaves zero rows for that checkpoint.
- [ ] After installing a small application, comparing the pre-install checkpoint with `Now` lists that application under "Appeared" and nothing spurious under "Changed".
- [ ] `compare` of two 2 000-entity manifests returns in < 200 ms.
- [ ] A comparison spanning a period when the app was closed shows the unmonitored-gap notice with the gap's start and end.
- [ ] 31 automatic checkpoints prune to the newest 30; manual checkpoints are untouched.

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml checkpoint_capture compare checkpoint_retention
cargo test --manifest-path src-tauri/Cargo.toml --release -- --ignored perf_compare
VITE_WC_MOCK=1 bun run test:e2e -- tests/e2e/checkpoints.spec.ts tests/e2e/compare.spec.ts
```

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| Capture takes long enough to feel broken | Med×Med | > 8 s on the dev machine | progress + stated expectation + cancel are the mitigation; if a single collector dominates, capture it last so partial progress is visible, and never block the UI thread |
| Manifest growth blows up the DB with many checkpoints | Med×Med | DB grows > 50 MB from checkpoints | 30-automatic cap plus `detail_json` limited to the fields comparison needs; if still large, store `detail_json` only for changed entities and keep fingerprints for the rest |
| Comparison shows noise (pids, run times) as "changed" | High×Med | a compare with Now right after a manual capture shows dozens of changes | fingerprints deliberately exclude volatile fields (pid, `LastRunTime`, uptime); a test creates two checkpoints 5 s apart on an idle machine and asserts the comparison is empty — this is the phase's sharpest correctness gate |
| Unmonitored gap misreported | Med×Med | gap notice appears for a continuously-running app | session rows are written on start/stop and heartbeated every 60 s; a crash leaves a stale open session, so a session with no heartbeat for > 5 min is closed at the last heartbeat, and the gap starts there |

## Security Considerations

Checkpoint labels and notes are user-supplied free text and are redacted before storage like any other
string, then rendered as text (never markup). `compare_checkpoints` accepts only ids and the `Now` marker,
so the frontend cannot express an arbitrary query. Deleting a checkpoint deletes its entity rows in the
same transaction, so no orphaned system inventory survives a user's decision to remove it — which is the
privacy-relevant part: a manifest is a detailed picture of the machine and must disappear when asked.
```

### FILE: phase-14-analysis-engine-deterministic.md

```markdown
---
phase: 14
title: "Deterministic Analysis Engine: Correlation, Ranking, Explanations"
status: pending
priority: P1
effort: 12h
dependencies: [12, 13]
---

## Overview

The engine that answers the product's question. Given a symptom ("it broke around 3pm", "this app stopped
working", or a checkpoint), it selects candidate changes, scores them, ranks them, groups causally related
ones, and produces a plain-language explanation with an honest confidence statement (§27–§29, §31–§32,
§59). It is **entirely deterministic and local** — rule-based scoring over stored events, no model, no
network — so the same inputs always produce the same ranking, which is what makes it testable and
trustworthy.

## Requirements

Functional:
- Entry points: `analyze(AnalysisRequest)` where the request is one of
  `TimeWindow { center, radius }`, `SinceCheckpoint { checkpoint_id }`, or
  `AboutEntity { entity_id, since }`. All three converge on one scoring pipeline.
- Candidate selection: events in the window, plus events touching entities related to the subject
  (one hop through the relationship table), plus any `important`/`critical` event within the window even if
  unrelated (users need to see those regardless).
- Scoring is a sum of documented, individually-testable signals with fixed weights, each contributing a
  named reason string:
  - temporal proximity to the symptom (decaying: ≤5 min 30, ≤30 min 22, ≤2 h 14, ≤12 h 7, ≤7 d 3)
  - category prior (driver 18, windows_update 16, network_config 15, service 12, application 12,
    file/config 12, docker 10, process 6)
  - severity of the change (version downgrade +10, unsigned binary +8, security-relevant service +8,
    listener on all interfaces +6)
  - entity relevance (same entity as the subject +25, one hop +12, shared port/path +10)
  - breadth (an event affecting many entities +6; a routine daily occurrence −8)
  - reversibility hint (a change with a known revert path +0 to score, but it is recorded for the advice)
  - noise suppression (occurrence_count > 20 in 7 days −12; `synthetic` excluded entirely; `inferred`
    confidence ×0.6)
- Grouping: events within 120 s that share an entity, or that are causally linked by a fixed rule set
  (`windows_update/installed` → `driver/version_changed` → `system/crash`; `application/updated` →
  `service/start_type_changed`; `network/configuration_changed` → `network/closed`) collapse into one
  `Suspect` with a primary event and supporting events, so the user sees one cause, not five rows.
- Output `AnalysisResult`: ordered `Vec<Suspect>` (max 10), each with `score`, `confidence`
  (`high ≥ 70`, `medium 40–69`, `low < 40`), `reasons: Vec<Reason>` (the named signals that fired, with
  their contributions), `explanation` (plain language, §59), `what_to_check` (concrete verification step),
  and `how_to_revert` (only when a real revert path exists for that event type), plus a top-level
  `overall_confidence` and, when nothing scores above `low`, an explicit "no likely cause found" result
  that says what was and was not monitored in that window.
- Honesty rules (§31/§32), enforced by tests:
  - the engine never claims causation — wording is "most likely related", "changed just before", "worth
    checking"; a forbidden-phrase test fails the build if any generated string contains
    `caused`, `definitely`, `guaranteed`, `will fix`, `100%`, `must be`, `always the reason`.
  - every suspect's reasons are shown; a score with no reason string is a bug (asserted).
  - `how_to_revert` is present only for event types with a documented revert path (driver rollback,
    application version reinstall, DNS restore, service start-type restore, watched-file diff restore);
    otherwise the field is absent rather than filled with a guess.
  - if the window overlaps an unmonitored gap, the result states it and caps `overall_confidence` at
    `medium`.

Non-functional:
- `analyze` over a 7-day window in a 100 000-event database returns in < 500 ms (perf harness).
- The scoring table lives in one file as data; adding a signal means adding a row and a fixture case.
- Deterministic: the same DB and request produce byte-identical results, asserted by running `analyze`
  twice and comparing serialized output.

## Architecture

Pipeline as five pure functions over materialized rows, so every stage is unit-testable without a DB:
`select_candidates(db, request) -> Vec<Event>` (the only stage that touches storage) →
`score(&[Event], &Subject) -> Vec<Scored>` → `group(Vec<Scored>) -> Vec<Suspect>` →
`explain(&mut [Suspect])` → `finalize(Vec<Suspect>, Context) -> AnalysisResult`. Weights and causal rules
are `const` tables in `analysis/weights.rs` and `analysis/causal_rules.rs`. Explanations come from the same
template mechanism as summaries (`normalize/summaries.rs`), extended with an `advice` template set, so
plain-language rules are enforced in one place rather than two.

## Related Code Files

Create:
- `src-tauri/src/analysis/{mod.rs,request.rs,candidates.rs,score.rs,weights.rs,group.rs,causal_rules.rs,explain.rs,advice.rs,result.rs}`
- `src-tauri/src/commands/analysis.rs` (`analyze`, `analyze_since_checkpoint`, `analyze_entity`)
- `src-tauri/tests/{analysis_ranking.rs,analysis_wording.rs,analysis_determinism.rs,analysis_perf.rs}`
- `src-tauri/tests/fixtures/ranking/*.json` (12 labelled scenarios with expected top suspect)
- `docs/ANALYSIS.md` (every signal, its weight, and its rationale — the user-facing explanation of how ranking works)

Modify: `normalize/summaries.rs` (advice templates), `storage/query.rs` (candidate query + one-hop relationship query), `src/lib/tauri/mock-transport.ts`, `ARCHITECTURE.md`.

## Implementation Steps

1. `request.rs` + `result.rs`: the typed request/response with `specta` derives so bindings regenerate; `Reason { signal: &'static str, contribution: i32, detail: String }`.
2. `candidates.rs`: window query + one-hop relationship expansion + the unconditional important/critical inclusion, capped at 2 000 candidates (highest importance first) so a pathological window cannot blow up scoring.
3. `weights.rs`: all weights as named consts with a doc comment each stating why; `docs/ANALYSIS.md` is generated-by-hand from this file and a test asserts every const name appears in the doc (keeps documentation honest without codegen).
4. `score.rs`: one function per signal, each returning `Option<Reason>`; the total is their sum. Every signal has at least two unit tests (fires / does not fire).
5. `causal_rules.rs` + `group.rs`: the fixed rule list plus the 120 s/shared-entity rule; grouping picks the primary event by (highest importance, then earliest timestamp) so the user sees the *initiating* change, not the consequence.
6. `explain.rs` + `advice.rs`: template selection by `(category, event_type, confidence)`; `what_to_check` is concrete ("Open Device Manager → Display adapters → Roll Back Driver", "Run `ipconfig /all` and confirm the DNS server"); `how_to_revert` only from the documented set.
7. `tests/fixtures/ranking/*.json`: 12 scenarios, each a small event set plus a request and the expected top suspect and expected confidence band — including the flagship four: GPU driver update before a crash; DNS change before "internet stopped working"; application update before a service failing to start; a watched `.env` edit before an app misbehaving. Plus two negative scenarios where the correct answer is "no likely cause found", and one where an unmonitored gap must cap confidence.
8. `analysis_ranking.rs`: runs all 12 fixtures; a failure prints the full scored list with reasons so a weight change is debuggable. This test is the engine's contract — weights may be tuned, but the 12 expectations must hold.
9. `analysis_wording.rs`: renders every template with every fixture and asserts (a) no forbidden phrase, (b) no jargon token (`HKEY_`, `0x`, `GUID`, `errno`, `\\Device\\`) in the plain explanation, (c) every suspect has ≥ 1 reason, (d) `how_to_revert` appears only for allowed event types.
10. `analysis_determinism.rs`: two runs, identical serialized output; also asserts ordering stability for equal scores (tie-break by timestamp then id).
11. `analysis_perf.rs` (`#[ignore]`, release): 7-day window over the 100 000-event fixture < 500 ms; also 1M-row DB with a 1-day window < 500 ms.
12. `commands/analysis.rs`: the three narrow commands, `spawn_blocking`, with request validation (radius ≤ 7 days, entity id length ≤ 256).

## Todo

- [ ] typed request/result + specta bindings
- [ ] candidate selection with one-hop expansion and cap
- [ ] documented weight table + per-signal tests
- [ ] causal rules + grouping with primary-event selection
- [ ] explanation/advice templates incl. revert allow-list
- [ ] 12 labelled ranking fixtures as the engine contract
- [ ] wording, determinism, and perf gates
- [ ] `docs/ANALYSIS.md` kept in sync by test

## Success Criteria

- [ ] All 12 ranking fixtures pass with the expected top suspect and confidence band.
- [ ] The GPU-driver-before-crash scenario ranks the driver change first with `high` confidence and lists temporal proximity, category prior, and entity relevance as reasons.
- [ ] Both negative scenarios return "no likely cause found" with a statement of what was monitored.
- [ ] No generated string in any fixture contains a forbidden phrase or a jargon token.
- [ ] Two identical `analyze` calls produce byte-identical serialized results.
- [ ] 7-day analysis over 100 000 events completes in < 500 ms (release).
- [ ] Every weight const is documented in `docs/ANALYSIS.md` (asserted).

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml analysis_
cargo test --manifest-path src-tauri/Cargo.toml --release -- --ignored analysis_perf
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
bun run check
```

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| Ranking looks plausible in fixtures but wrong on the real machine | Med×High | a real incident where the obvious cause is not first | the 12 fixtures are the floor, not the ceiling: every real miss becomes a 13th fixture before any weight is touched, so tuning cannot regress earlier cases |
| Weight tuning turns into endless fiddling | High×Low | repeated weight edits without fixture changes | rule: a weight may change only together with a fixture that motivates it; documented in `ANALYSIS.md` |
| Confidence inflation misleads the user | Med×**High** | `high` confidence on a thin single-signal match | `high` requires ≥ 70 *and* at least two independent signals; asserted by a test that a lone temporal-proximity hit can never exceed `medium` |
| Grouping hides the real cause inside a group | Med×Med | expected top suspect appears as a supporting event | primary-event selection is by importance then earliest time, and supporting events are always listed and expandable; a fixture covers the update→driver→crash chain explicitly |
| Advice suggests a revert that damages the system | Low×**High** | a revert string for an unsupported event type | `how_to_revert` is an allow-list, tested; advice never instructs a destructive action, only navigation to the OS's own rollback UI |

## Security Considerations

The engine reads only already-redacted stored data, performs no network calls, and executes nothing — its
advice strings point the user at Windows' own tooling rather than running commands, so a bad rule cannot
cause damage. Requests are bounded (window ≤ 7 days, capped candidate count) so a compromised WebView
cannot use `analyze` as a CPU-exhaustion primitive. Because the pipeline is deterministic and local, no
event data ever leaves the machine for scoring — which is the property that lets `PRIVACY.md` state that
analysis happens entirely offline.
```

### FILE: phase-15-investigation-screen.md

```markdown
---
phase: 15
title: "Investigation Screen: Ask, Rank, Explain"
status: pending
priority: P1
effort: 8h
dependencies: [08, 14]
---

## Overview

The screen where a user in trouble types nothing more than "when did it break" and gets a ranked list of
likely-related changes with plain explanations (§45, §73, §74). It is the product's front door: the
Dashboard's primary action, the tray's "Something broke — what changed?" item, and the empty-state CTA all
land here. It consumes the Phase 14 engine and adds no analysis logic of its own.

## Requirements

Functional:
- Three ways to start, presented as one question, not a form:
  1. "It broke around…" — a time picker with quick options (Just now, 1 hour ago, This morning,
     Yesterday, Pick a time) mapping to `TimeWindow { center, radius }` with radius defaulting by choice
     (Just now → 2 h, 1 hour ago → 4 h, This morning → 12 h, Yesterday → 36 h, Custom → user-chosen).
  2. "It worked at this checkpoint" — a checkpoint picker mapping to `SinceCheckpoint`.
  3. "This program stopped working" — an entity picker (searchable list of known applications, services and
     processes) mapping to `AboutEntity`.
- Results: an ordered suspect list, each card showing importance/category, the primary change with its
  time and relative age, the plain explanation, a confidence badge with a tooltip explaining what the band
  means, an expandable "Why this is listed" showing the reasons with their contributions, an expandable
  "Supporting changes" list, "What to check" as a step, and "How to undo this" only when present.
- Overall confidence is shown once, above the list, with the §32 honesty framing: what the app monitored
  during that window, what it did not, and an unmonitored-gap warning if applicable.
- "No likely cause found" is a first-class result with its own layout: what was checked, the window, the
  monitored categories, and two next actions (widen the window; create a checkpoint now so the next
  investigation has a reference point).
- Every suspect links into the timeline filtered to its entity, and into the comparison view when the
  request came from a checkpoint.
- The investigation is shareable within the app: "Copy summary" produces a plain-text report (title,
  window, top three suspects with reasons, confidence, and the explicit statement that this is
  correlation, not proven cause). This is the same text the Phase 18 diagnostics export embeds.
- Re-running the same investigation is one click and shows if the result changed since the last run.

Non-functional:
- The screen renders a skeleton within 100 ms and the result as soon as `analyze` returns; a spinner over
  500 ms shows what it is doing ("Looking at 1 240 changes from the last 4 hours").
- Fully keyboard operable: the question flow, the pickers, expanding a card, and copying.
- Confidence and importance are never communicated by color alone (§64).

## Architecture

`InvestigationView.svelte` holds a small state machine in `investigation.svelte.ts`:
`Asking → Running → Result | Empty | Failed`, with the request object as the single source of truth so
re-running is `request` → `transport.analyze(request)` again. The request is serialized into the URL so an
investigation can be reloaded or linked from the tray. Suspect cards are presentational and receive the
typed `Suspect` from the backend — no scoring, wording, or ordering happens in the frontend, which is what
keeps the §31/§32 wording guarantees enforceable by the backend tests alone.

## Related Code Files

Create:
- `src/features/investigation/{InvestigationView.svelte,QuestionFlow.svelte,TimeAnchorPicker.svelte,CheckpointAnchorPicker.svelte,EntityAnchorPicker.svelte,investigation.svelte.ts}`
- `src/features/investigation/{SuspectCard.svelte,ReasonList.svelte,SupportingChanges.svelte,ConfidenceBadge.svelte,WhatToCheck.svelte,HowToUndo.svelte,NoCauseFound.svelte,MonitoringScopeNotice.svelte}`
- `src/lib/format/report-text.ts` (the "Copy summary" plain-text builder)
- `tests/unit/{investigation-state.test.ts,report-text.test.ts}`
- `tests/e2e/investigation.spec.ts`

Modify: `src/routes.ts`, `src/features/dashboard/DashboardView.svelte` (primary action), tray menu wiring (Phase 04), `src/lib/tauri/mock-transport.ts` (three canned analysis results: strong cause, weak cause, none), `DESIGN.md` (suspect card anatomy).

## Implementation Steps

1. `investigation.svelte.ts`: the state machine, request serialization to/from `location.search`, `run()` guarded against double submission, `rerun()` keeping the previous result for a "changed since last run" comparison (compare by suspect ids and order).
2. `QuestionFlow.svelte`: one question with three tabs (`role="tablist"`, arrow-key navigation); each tab renders its picker; a single primary "Show me what changed" button.
3. `TimeAnchorPicker.svelte`: quick options as radio group + a custom `datetime-local` input; radius derived per option and shown explicitly ("looking at changes from 1:00 PM to 5:00 PM") so the user is never guessing what was searched.
4. `EntityAnchorPicker.svelte`: searchable combobox over `list_known_entities()` (a new narrow command returning id + display name + kind, capped at 500 with a search filter); `role="combobox"` with `aria-activedescendant`.
5. `SuspectCard.svelte`: header (category icon, importance chip, relative time, exact time on hover/focus), explanation paragraph, `ConfidenceBadge` (icon + word + tooltip), then three disclosures: Why this is listed (`ReasonList`), Supporting changes (`SupportingChanges`), Technical details.
6. `ReasonList.svelte`: each reason as "signal — detail" plus its contribution rendered as a small bar with a numeric label (never color-only).
7. `NoCauseFound.svelte` + `MonitoringScopeNotice.svelte`: the honest-empty layout and the monitored/not-monitored scope statement, driven by the backend's returned scope so the UI cannot overstate coverage.
8. `report-text.ts`: builds the plain-text report deterministically; a unit test asserts it contains the correlation disclaimer and no forbidden phrase (the same forbidden list as the backend, imported from a single shared JSON so the two lists cannot drift — `src-tauri/../shared/forbidden-phrases.json` read by both the Rust test and the TS test).
9. Loading state: skeleton cards; after 500 ms, the "Looking at N changes from …" line using the candidate count returned by a lightweight `count_candidates` call issued in parallel.
10. E2E: all three entry paths against the three canned results; assert the strong-cause path shows a `high` badge with ≥ 2 reasons, the weak path shows `low` with the scope notice, the none path shows `NoCauseFound` with both next actions, and that "Copy summary" writes the disclaimer to the clipboard.

## Todo

- [ ] investigation state machine + URL round-trip
- [ ] three anchor pickers with explicit searched-window display
- [ ] suspect card with reasons, supporting changes, advice
- [ ] confidence badge with non-color encoding and explanation
- [ ] honest no-cause layout + monitoring scope notice
- [ ] shared forbidden-phrase list used by both TS and Rust tests
- [ ] e2e for all three paths and clipboard content

## Success Criteria

- [ ] Each of the three entry paths produces a result screen; the searched window is stated on screen in every case.
- [ ] The strong-cause fixture renders the driver change first with a `high` badge, ≥ 2 reasons visible when expanded, and a "How to undo this" step.
- [ ] The no-cause fixture renders the explanatory layout with both next actions and no empty-list placeholder.
- [ ] "Copy summary" clipboard text contains the correlation disclaimer and passes the shared forbidden-phrase check.
- [ ] Reloading the page with an investigation URL reproduces the same request and result.
- [ ] Full keyboard walkthrough of the flow is possible with a visible focus indicator at every stop.
- [ ] `bun run check` clean; axe scan of the results screen reports no violations.

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test -- tests/unit/investigation-state.test.ts tests/unit/report-text.test.ts
VITE_WC_MOCK=1 bun run test:e2e -- tests/e2e/investigation.spec.ts
cargo test --manifest-path src-tauri/Cargo.toml forbidden_phrases_shared
```

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| The flow feels like a form and users bounce | Med×High | e2e/manual walkthrough needs more than two decisions to get a result | keep the "Just now" default one click from a result; if a picker adds friction, make it optional rather than removing the entry point |
| UI restates confidence more strongly than the backend | Med×High | wording test finds a forbidden phrase in a component | all wording comes from backend strings; the frontend adds only fixed labels, which the shared list checks |
| Copy summary leaks unredacted detail | Low×High | clipboard text contains a raw path or secret | the report is built from the same already-redacted fields the cards display; there is no second data source |
| Long analyses look hung | Med×Med | > 2 s without feedback | skeleton + candidate count + a cancel that abandons the result; the backend call is bounded by the Phase 14 caps |

## Security Considerations

The screen introduces exactly one new command, `list_known_entities`, which returns ids and display names
already stored and redacted, capped and searchable server-side — it is not a filesystem or process browser.
Clipboard writes use the Tauri clipboard permission scoped to write-only. No component performs scoring or
rewrites explanations, so the honesty guarantees proven by the Phase 14 tests remain true of what the user
actually sees.
```

### FILE: phase-16-threejs-visualization-layer.md

```markdown
---
phase: 16
title: "Three.js Visualization Layer with DOM Equivalents"
status: pending
priority: P2
effort: 12h
dependencies: [08, 14, 15]
---

## Overview

The visual layer (§38, §39, §43, §48): a 3D timeline river that makes change density and clustering
legible at a glance, and a relationship graph that shows how a suspect connects to the things around it.
Both are built on three 0.185.1 inside a single reusable canvas host, and **both ship with a DOM
equivalent that is not a degraded fallback but a first-class view** — required by §43/§64 (accessibility,
reduced motion, no-WebGL machines) and by the app's own performance budget.

## Requirements

Functional:
- Timeline river (§38): time on one axis, categories as lanes, each event an instanced quad whose height
  encodes importance and whose color encodes category; density visible as clustering. Interactions: drag to
  pan time, wheel to zoom (1 hour → 30 days), hover to highlight with a DOM tooltip (not in-canvas text),
  click to open the Phase 08 details panel. A "now" marker and day separators are drawn as lines.
- Relationship graph (§39): the subject entity at the center, related entities as nodes, edges from the
  relationship table, laid out by a deterministic seeded force simulation run for a fixed 300 iterations
  then frozen (no perpetual animation — determinism and idle CPU both matter). Node size encodes event
  count; click focuses a node and re-centers.
- Both views render **at most 5 000 instances**; above that the backend returns pre-bucketed aggregates
  (`get_timeline_buckets(range, bucket)` returning per-(bucket, category, importance) counts), so the
  visualization scales to 1M events by drawing buckets rather than rows.
- DOM equivalents, always available and switchable by an explicit toggle that persists:
  - timeline → a density heat-strip built from the same bucket data using CSS grid, with per-cell
    `aria-label` ("Tuesday 14:00, 12 changes, 2 important") and keyboard navigation across cells;
  - graph → a nested list of related entities grouped by relationship kind, each item a link.
- Automatic selection: if WebGL is unavailable, `prefers-reduced-motion: reduce` is set, or the app is on
  battery-saver-like conditions we can detect (`navigator.hardwareConcurrency <= 2`), the DOM view is the
  default — with a visible, dismissible note saying why and how to switch.
- The canvas is paused when not visible: `IntersectionObserver` plus window-blur/tray-minimize stops the
  render loop entirely (no `requestAnimationFrame` while hidden), which is the app's largest idle-CPU
  lever.
- Reduced motion honored: no camera easing, no entry animations, instant transitions.

Non-functional:
- 60 fps while panning with 5 000 instances on the dev machine; measured and recorded.
- Three.js is dynamically imported so the initial bundle stays small; the DOM views require no 3D code at
  all (proven by a bundle-analysis assertion that the entry chunk does not include three).
- WebGL context loss is handled: `webglcontextlost` switches to the DOM view with a note rather than
  showing a dead canvas.
- Every visual encoding has a legend, and every color has a non-color partner (height, shape, or label).

## Architecture

One host component, `SceneHost.svelte`, owns the WebGL renderer, camera, resize observer, visibility
gating and the render loop; scenes are plain TS classes (`TimelineRiverScene`, `RelationshipGraphScene`)
implementing `{ mount(ctx), update(data), dispose() }`, so no Three.js object lives in Svelte state and
there is exactly one renderer for the whole app. Data comes from the same typed commands as the DOM views
(`get_timeline_buckets`, `get_entity_graph`), which is what makes the two renderings provably equivalent:
`tests/unit/view-equivalence.test.ts` asserts the DOM heat-strip cell totals equal the instance counts the
scene would draw from the identical payload. Colors and importance heights read from the Phase 03 CSS
variables via `getComputedStyle` at mount, so theming stays single-sourced.

## Related Code Files

Create:
- `src/lib/three/{SceneHost.svelte,renderer.ts,visibility.ts,scene.ts}`
- `src/lib/three/scenes/{timeline-river.ts,relationship-graph.ts,instancing.ts,force-layout.ts,legend.ts}`
- `src/features/timeline/{TimelineRiver.svelte,DensityStrip.svelte,ViewToggle.svelte}`
- `src/features/investigation/{RelationshipGraph.svelte,RelatedEntityList.svelte}`
- `src-tauri/src/commands/visualization.rs` (`get_timeline_buckets`, `get_entity_graph`)
- `src-tauri/src/storage/buckets.rs`
- `tests/unit/{force-layout.test.ts,buckets.test.ts,view-equivalence.test.ts}`
- `tests/e2e/{visualization.spec.ts,visualization-fallback.spec.ts}`
- `scripts/check-bundle.ts` (asserts three is not in the entry chunk)

Modify: `vite.config.ts` (manual chunk for three), `src/features/timeline/TimelineView.svelte`, `src/features/investigation/InvestigationView.svelte`, `src/lib/stores/settings.svelte.ts` (view preference), `DESIGN.md` (legend + encodings), `ACCESSIBILITY.md`.

## Implementation Steps

1. `storage/buckets.rs` + `commands/visualization.rs`: `get_timeline_buckets(range, bucket_size)` returning `Vec<Bucket { start, category, importance, count }>` with bucket sizes constrained to a fixed set (5 min, 1 h, 6 h, 1 day) chosen by the requested range; `get_entity_graph(entity_id, depth ≤ 2)` returning nodes and edges capped at 200/400. Unit-tested for boundary alignment (a bucket starts on the local hour) and for the cap behavior.
2. `renderer.ts`: single `WebGLRenderer` with `antialias: false`, `powerPreference: 'low-power'`, DPR capped at 1.5; `createRenderer()` returns `null` when WebGL is unavailable so callers fall back deterministically.
3. `visibility.ts`: combines `IntersectionObserver`, `document.visibilitychange`, and Tauri window focus events into one `isRenderable` rune; the loop starts and stops with it. A test asserts the loop's frame callback stops being called after a simulated blur.
4. `instancing.ts`: `InstancedMesh` builder for quads with per-instance color and scale, plus a picking path using an off-screen id buffer read on click only (no per-frame raycast).
5. `timeline-river.ts`: lanes from the category list, x from timestamp with a zoom transform, y from lane, height from importance; day separators and the now marker; pan/zoom handlers that only recompute the transform matrix (data is uploaded once per payload).
6. `force-layout.ts`: deterministic seeded layout (fixed PRNG, 300 iterations, then frozen); unit test asserts identical positions across two runs with the same input.
7. `relationship-graph.ts`: nodes as instanced circles with DOM labels positioned by projecting world→screen each frame for at most 30 visible labels; edges as line segments.
8. `SceneHost.svelte`: dynamic `import('three')` on mount, error boundary that switches to the DOM view on failure or `webglcontextlost`, `dispose()` on unmount asserted by a test that mounts/unmounts 20 times and checks `renderer.info.memory` returns to baseline.
9. `DensityStrip.svelte` + `RelatedEntityList.svelte`: the DOM equivalents with full keyboard navigation and `aria-label`s from the same payload; `ViewToggle.svelte` persists the choice and states the reason when the DOM view was auto-selected.
10. `check-bundle.ts`: parses the Vite manifest and fails if the entry chunk includes the three chunk; wired into `bun run build`.
11. E2E: `visualization.spec.ts` (WebGL available in headless Chromium) asserts canvas presence, that panning changes the rendered range label, and that clicking an instance opens details; `visualization-fallback.spec.ts` launches with WebGL disabled and `prefers-reduced-motion: reduce` and asserts the DOM strip is default, keyboard-navigable, and that no canvas is created.

## Todo

- [ ] bucket + graph commands with fixed bucket sizes and caps
- [ ] single renderer host with visibility gating and disposal
- [ ] instanced timeline river with pan/zoom and click picking
- [ ] deterministic frozen force layout + graph scene
- [ ] DOM heat-strip and related-entity list as first-class views
- [ ] auto-selection rules (no WebGL, reduced motion, low cores) with a stated reason
- [ ] equivalence test + bundle assertion + fps measurement

## Success Criteria

- [ ] Timeline river renders 5 000 instances and sustains 60 fps while panning on the dev machine (recorded in `docs/perf/visualization.md`).
- [ ] With 1M events the view draws buckets, not rows, and the payload stays under 5 000 items (asserted).
- [ ] Hiding the window stops the render loop: no `requestAnimationFrame` callbacks fire while hidden (asserted), and idle CPU with the app minimized is indistinguishable from the no-visualization build.
- [ ] With WebGL disabled and reduced motion on, the DOM strip is the default, is keyboard-navigable cell by cell, announces counts, and no canvas element exists.
- [ ] DOM strip totals equal the 3D instance counts for the same payload (equivalence test).
- [ ] Mount/unmount 20× returns Three.js memory counters to baseline (no leak).
- [ ] Entry chunk contains no three code (`bun run build` gate).

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test -- tests/unit/force-layout.test.ts tests/unit/buckets.test.ts tests/unit/view-equivalence.test.ts
cargo test --manifest-path src-tauri/Cargo.toml buckets entity_graph
VITE_WC_MOCK=1 bun run test:e2e -- tests/e2e/visualization.spec.ts tests/e2e/visualization-fallback.spec.ts
bun run build && bun run scripts/check-bundle.ts
```

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| WebGL unavailable or unstable in WebView2 on some machines | Med×Med | context creation returns null / context lost in testing | the DOM views are first-class and always present, so the feature degrades to a different rendering of the same data rather than to nothing |
| 3D layer pushes memory past the app budget | **High**×High | memory measurement with the canvas open exceeds the target | visualization is lazily imported and disposable; if it is the deciding factor for the B2 memory question, report the measured delta with and without it and let the user decide — do not silently drop the feature or silently redefine the budget |
| Render loop keeps running when hidden (silent battery drain) | Med×High | the frame-callback assertion fails | visibility gating is a gate test, not a nicety |
| Determinism lost in the graph layout (jitter between runs) | Med×Low | layout test diff | seeded PRNG + fixed iteration count + freeze; never `Math.random()` |
| In-canvas text unreadable / inaccessible | Med×Med | labels illegible at DPR 1 or unreadable by AT | all text is DOM, positioned by projection — the canvas draws no glyphs at all |

## Security Considerations

The visualization consumes only aggregate counts and entity display names that are already redacted, so no
new data crosses the IPC boundary — bucket payloads cannot leak a path or a secret because they contain
integers and enum labels. Shaders are the library's built-ins plus one instanced material; no shader source
is constructed from data. The dynamic `import('three')` resolves to a bundled local chunk, never a CDN, so
the app makes no network request to render (consistent with the offline-only privacy claim).
```

### FILE: phase-17-docker-and-development-mode.md

```markdown
---
phase: 17
title: "Docker Monitoring & Development Mode"
status: pending
priority: P2
effort: 11h
dependencies: [09, 11]
---

## Overview

Two collectors for the developer audience: Docker container changes (§23) and development mode (§24), which
watches the project folders a developer chooses and reports branch switches, dependency-manifest changes,
and lockfile changes — the "what changed" answers that live in a repository rather than in Windows.
Docker is the one collector that **cannot be verified on the development machine**: the measured matrix
records the Docker named pipe as absent (decision D3). That fact shapes the whole design: the integration
is written against a mockable transport, its live path is behind an availability check, and it ships
labelled as unverified rather than claimed as working.

## Requirements

Functional — Docker (§23):
- Availability check first: probe `\\.\pipe\docker_engine` existence; if absent, the collector registers as
  `Disabled { reason: "Docker Desktop is not running or not installed" }` and the UI says exactly that. No
  retries beyond a 60 s re-probe, no error spam, no log noise.
- When available, connect with `bollard` 0.21.1 over the named pipe and poll containers (list with all
  states) plus images, at 10 s. Emit `docker/container_started`, `/stopped`, `/created`, `/removed`,
  `/restarted` (restart detected by a changed `StartedAt` on the same container id), `/health_changed`,
  `/image_changed` (same container name, different image id), and `/port_mapping_changed` — keyed
  `container:{name}` so a recreated container is the same entity, with the id in the details.
- Include the fields a developer needs in before/after: image tag and digest-short, published ports,
  restart count, exit code on stop, and health status. Environment variables are **not** collected
  (they are the single most likely place for secrets, and §23 does not ask for them).
- Event-stream upgrade is explicitly deferred to polling for the MVP: `bollard`'s event stream would be
  lower-latency, but it cannot be validated here, and a 10 s poll matches the rest of the app's cadence
  design. `COLLECTORS.md` records this as a deliberate, revisitable choice with the reason.

Functional — development mode (§24):
- The user adds project folders (reusing the Phase 11 watched-path validation, but a separate list with its
  own semantics). For each project the collector detects the toolchain by manifest presence and watches:
  `.git/HEAD` (branch switch), `.git/MERGE_HEAD`/`REBASE_HEAD` (in-progress operations), `package.json`,
  `bun.lock`/`bun.lockb`/`package-lock.json`/`pnpm-lock.yaml`/`yarn.lock`, `Cargo.toml`/`Cargo.lock`,
  `requirements.txt`/`pyproject.toml`/`poetry.lock`, `go.mod`/`go.sum`, `composer.json`/`composer.lock`,
  `Gemfile`/`Gemfile.lock`, `*.csproj`/`packages.lock.json`, and `.env`/`.env.local` (existence and
  redacted diff only).
- Events: `dev/branch_switched` (before/after branch names), `dev/dependencies_changed` (with a parsed
  added/removed/version-changed list for `package.json`, `Cargo.toml`, `requirements.txt`, `go.mod` —
  the four manifests worth parsing; others report "manifest changed" honestly), `dev/lockfile_changed`
  (with the lockfile's own change summary: entries added/removed count only, never the full diff),
  `dev/env_changed` (redacted), `dev/git_operation` (rebase/merge started or finished).
- Branch and status come from one fixed-argv git invocation:
  `git --no-optional-locks status --porcelain=v2 --branch` executed with the project path as cwd, no shell,
  a 5 s timeout, and output parsed by field. `gix` was rejected (§80 dependency discipline) because one
  fixed command with `--no-optional-locks` is both smaller and safer than embedding a git implementation.
  If git is not on `PATH`, `.git/HEAD` is read directly as the fallback (it contains the ref name).
- Development mode is off by default and has its own Settings section listing projects with detected
  toolchains, so a user can see exactly what is being watched.

Non-functional:
- The git command is never constructed from user input beyond the validated project path passed as cwd, and
  is executed with `Command::new("git")` and separate `.arg()` calls — no string concatenation, no shell.
- Docker polling must not block the supervisor: the Docker collector runs in its own Tokio task with a
  5 s per-call timeout.
- Manifest parsing failures are non-fatal: a malformed `package.json` mid-edit yields "manifest changed"
  rather than an error event.

## Architecture

Docker access sits behind a trait so the untestable part is exactly one thin implementation:
```rust
pub trait DockerSource: Send {
    fn probe(&self) -> Availability;
    fn containers(&self) -> Result<Vec<ContainerRow>, CollectorError>;
    fn images(&self) -> Result<Vec<ImageRow>, CollectorError>;
}
```
`BollardSource` is the live implementation (the only code that cannot be exercised on this machine);
`FixtureSource` replays recorded JSON so the diff, normalization, importance and UI paths are fully tested.
The collector holds a `Box<dyn DockerSource>` chosen at construction. Development mode reuses the Phase 11
watcher for file signals and adds `collectors/dev/manifests/` with one small parser per format, each a pure
`fn parse(&str) -> Result<BTreeMap<String, String>>` so `dependencies_changed` is a map diff.

## Related Code Files

Create:
- `src-tauri/src/collectors/docker/{mod.rs,source.rs,bollard_source.rs,fixture_source.rs,diff.rs}`
- `src-tauri/src/collectors/dev/{mod.rs,git.rs,projects.rs,manifests/{mod.rs,package_json.rs,cargo_toml.rs,requirements_txt.rs,go_mod.rs}}`
- `src-tauri/src/commands/{docker.rs,dev_projects.rs}`
- `src-tauri/tests/{docker_diff.rs,docker_unavailable.rs,dev_git.rs,dev_manifests.rs,dev_branch_switch.rs}`
- `src-tauri/tests/fixtures/docker/{containers_before.json,containers_after.json,images.json}`
- `src/features/settings/sections/{DockerSection.svelte,DevelopmentSection.svelte}`
- `src/features/settings/ProjectList.svelte`

Modify: `src-tauri/Cargo.toml` (`bollard` 0.21.1 with the named-pipe feature — read the crate's feature names from the vendored source before writing them), `collectors/registry.rs`, `normalize/tables.rs`, `COLLECTORS.md`, `PRIVACY.md` (project paths and `.env` handling).

## Implementation Steps

1. `docker/source.rs`: the trait plus `Availability { Available, Unavailable(String) }`; `probe()` for the fixture source always returns `Available`.
2. `bollard_source.rs`: named-pipe connection. The exact constructor (`Docker::connect_with_named_pipe…` family) and its feature flag must be read from the vendored `bollard` 0.21.1 source before writing the call (API-shape rule) — this is the single highest fabrication risk in the phase and is handled by reading, not guessing. Every call is wrapped in a 5 s `tokio::time::timeout`.
3. `fixture_source.rs` + `tests/fixtures/docker/*.json`: two container snapshots differing by one started container, one stopped with exit code 137, one recreated with a new image, and one with changed port mappings — recorded shapes mirroring the Docker API's documented fields, marked in the file header as fixtures authored from documentation, not captured from a live daemon (honesty about their provenance matters for later validation).
4. `docker/diff.rs`: diff on `(name, state, started_at, image_id, ports, health)`; restart detection; `container:{name}` entity ids; exit code carried into the stop event's details.
5. `docker/mod.rs`: `Collector` impl with the probe gate, own Tokio task, 10 s cadence, and 60 s re-probe when unavailable. `docker_unavailable.rs` asserts: registered, `Disabled` with the exact user-facing reason, zero events, zero error logs over 5 simulated minutes.
6. `dev/projects.rs`: project list persistence, toolchain detection by manifest presence, and per-project watch registration through the Phase 11 watcher.
7. `dev/git.rs`: fixed-argv invocation, 5 s timeout, `porcelain=v2 --branch` parsing (`# branch.head`, `# branch.oid`, `# branch.upstream`, then entry lines for dirty state), plus the `.git/HEAD` fallback parser. Tested against a repo the test creates with `git init` in a temp dir (skipped with a clear message if git is absent).
8. `dev/manifests/*`: four parsers; `package_json.rs` merges `dependencies` + `devDependencies` + `optionalDependencies`; `cargo_toml.rs` reads `[dependencies]`/`[dev-dependencies]` with a minimal TOML subset parser or the already-present TOML capability if one is in the dependency graph (checked, not assumed); `requirements_txt.rs` handles `==`, `>=`, extras, and comments; `go_mod.rs` reads the `require` block. Each parser has a malformed-input test asserting a graceful "manifest changed" outcome.
9. `dev/mod.rs`: the `Collector` impl mapping file signals to the six event types, with a 2 s debounce so a branch switch (which rewrites many files) produces one `dev/branch_switched` and not a storm of `filesystem/modified` rows — a test performs a real branch switch in the temp repo and asserts exactly one branch event plus at most one dependency event.
10. `DockerSection.svelte` / `DevelopmentSection.svelte` / `ProjectList.svelte`: availability state with the exact reason text, project add/remove, detected toolchain chips, and a note that `.env` files are diffed with values redacted.
11. `COLLECTORS.md`: both collectors, with Docker's row explicitly stating "live path unverified on the development machine (Docker named pipe absent at planning time); fixture-tested" and the polling-vs-event-stream decision.

## Todo

- [ ] `DockerSource` trait + fixture source + fully tested diff path
- [ ] `BollardSource` written against the vendored crate source
- [ ] unavailable-state behavior with exact user-facing reason and no noise
- [ ] project list + toolchain detection + watch wiring
- [ ] fixed-argv git status parsing with `.git/HEAD` fallback
- [ ] four manifest parsers with malformed-input tolerance
- [ ] branch-switch debounce proven on a real temp repo
- [ ] Settings sections + honest documentation rows

## Success Criteria

- [ ] With no Docker present (the current machine), the collector reports `Disabled` with the stated reason, emits nothing, logs no errors, and the Settings section shows the reason — verified on this machine.
- [ ] The fixture pair produces exactly four Docker events (started, stopped with exit 137, image changed, port mapping changed) with correct before/after values.
- [ ] Switching branches in a test repository produces exactly one `dev/branch_switched` with correct before/after names.
- [ ] Adding a dependency to `package.json` produces one `dev/dependencies_changed` listing that package, and the accompanying lockfile change produces one `dev/lockfile_changed` with counts only.
- [ ] Editing `.env` in a project produces a redacted diff — the DB scan for the test secret finds nothing.
- [ ] No environment variable value from any container or project appears anywhere in the database (scan test).
- [ ] `bun run check` and clippy clean.

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml docker_ dev_
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
VITE_WC_MOCK=1 bun run test:e2e -- tests/e2e/settings-docker-dev.spec.ts
```
Manual (only if a Docker daemon becomes available): start/stop a container and confirm two events; record the result in `COLLECTORS.md` and flip the row from unverified to verified.

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| `bollard` named-pipe API shape or feature name differs from assumption | **High**×Med | compile error or connection failure | API-shape rule: read the vendored source. The trait boundary means only `bollard_source.rs` changes; nothing else in the phase is affected |
| Docker live path never validated before release | **High**×Med | no manual confirmation possible on this machine | ship it labelled unverified in `COLLECTORS.md` and in the Settings section's help text; never claim verified behavior. This is the honest resolution of D3, and it is a documentation obligation, not a code compromise |
| Fixture shapes diverge from the real Docker API | Med×Med | live run produces parse errors | fixtures are marked as documentation-derived; the first live run becomes a recorded fixture that replaces them |
| Git invocation blocks or hangs on a network drive | Med×Med | 5 s timeout fires | timeout + `--no-optional-locks` (avoids index lock contention) + `.git/HEAD` fallback |
| A branch switch floods the timeline | **High**×Med | dozens of `filesystem/modified` rows per switch | the 2 s debounce and the branch-event collapse are the design; the temp-repo test is the gate |
| Project `.env` content stored unredacted | Low×**High** | DB scan finds a secret | same chokepoint and scan test as Phase 11; `.env` is diffed by key, never by value |

## Security Considerations

Docker environment variables and container command lines are deliberately not collected, so the app cannot
become a secret store for compose files. The Docker connection is read-only in intent — only list/inspect
calls are used, and no code path performs a container action, so a compromised WebView cannot start, stop,
or exec into containers (there is no command to do so). The git integration executes a fixed argv with no
shell and no user-controlled arguments; the only variable is the cwd, which is a path already validated by
the Phase 11 rules. Project paths themselves are user data and pass through redaction before storage.
```

### FILE: phase-18-release-hardening-and-audit.md

```markdown
---
phase: 18
title: "Onboarding, Notifications, Diagnostics, Accessibility, Installer & Final Audit"
status: pending
priority: P1
effort: 16h
dependencies: [16, 17]
---

## Overview

The phase that makes the app shippable: first-run onboarding and the privacy disclosure (§50, §53), the
tray notification policy (§62), the diagnostics export with proven redaction (§65), the accessibility and
performance audits (§64, §66), the installer/updater/signing story (§84, §88), and the final cross-cutting
audit that re-proves every invariant the earlier phases established. Nothing here is optional polish —
each item is a gate the previous seventeen phases assumed would exist.

## Requirements

Functional — onboarding & privacy (§50, §53):
- First run shows a four-step onboarding: (1) what the app does, in one screen, in plain language;
  (2) the privacy statement — everything stays on this machine, nothing is uploaded, no account, no
  telemetry, with a link to `PRIVACY.md` rendered in-app; (3) what will be monitored, as a checklist the
  user can adjust immediately (each collector with its plain description and a toggle); (4) optional
  watched files / projects, explicitly skippable.
- Onboarding ends by creating the first checkpoint and explaining why ("so we have a 'working' reference").
- A "Start monitoring" choice is required — the app does not silently begin collecting before the user has
  seen step 2. Declining leaves the app running, monitoring paused, with a clear resume affordance.
- Autostart is offered, off by default, implemented per-user (registry `Run` key via the Tauri autostart
  plugin or a direct `HKCU` write — whichever the pinned plugin set supports; verified, not assumed) and
  removable from Settings.
- Pause/resume monitoring is available from the tray and Settings, persists across restarts, and is
  visible in the UI at all times when paused (a persistent, non-dismissible banner).

Functional — notifications (§62):
- Tray notifications fire only for `important` and `critical` events, at most one per 60 s, coalescing
  extras into "and N more changes"; never during the first 60 s after start (avoids a burst from the
  event-log backfill), never for `backfilled` or `synthetic` rows, and never when monitoring is paused.
- A Settings control offers: all important changes / critical only / none. Default is important+critical.
- Clicking a notification opens the app to the timeline filtered to that event.

Functional — diagnostics export (§65):
- "Export diagnostics" produces a single `.zip` in a user-chosen location containing: app version, OS
  build, WebView2 version, collector states with measured durations, last 200 log lines, DB size and row
  counts, the settings (with watched paths reduced to a count, not the paths), and the last investigation
  report — every string passing the redaction chokepoint on the way in.
- A test asserts the export contains no user name, no absolute user-profile path, no watched-file content,
  and none of the corpus secrets — by generating an export from a DB seeded with the redaction corpus.

Functional — installer, updater, signing (§84, §88):
- NSIS per-user installer (no elevation), install to `%LOCALAPPDATA%\Programs\What Changed`, Start-menu
  entry, optional desktop shortcut, and an uninstaller that asks whether to delete the database and
  honors the answer (a monitoring app that leaves its history behind after uninstall is a privacy defect).
- Product name "What Changed", binary `WhatChanged.exe`, package identifier `what-changed`, publisher and
  copyright set, version from a single source (`Cargo.toml`), asserted equal to `package.json` by a test.
  The repository folder's misspelling (`.WhatChage`) never appears in any shipped artifact — asserted by
  grepping the built installer's metadata and the binary's resources.
- Updater: configured but pointed at a documented, not-yet-existing endpoint; `docs/RELEASE.md` states
  exactly what must exist before enabling it (a signing key pair, a static JSON endpoint, and a public key
  in the config) and the config ships with the updater **disabled** rather than pointing at a placeholder
  URL that would fail silently.
- Code signing: `docs/RELEASE.md` documents the unsigned reality (SmartScreen warning expected), the exact
  steps to sign once a certificate exists, and the fact that no key material is stored in the repository.

Non-functional — audits:
- Accessibility (§64): axe scan on every screen with zero violations; a full keyboard walkthrough script;
  contrast verified for both themes against the tokens; screen-reader pass on the timeline, investigation,
  and comparison screens documented in `ACCESSIBILITY.md` with what works and what is imperfect.
- Performance (§66): the final numbers table in `docs/perf/release.md` — idle CPU over 30 min, memory
  (backend + WebView2 children, split out), startup to interactive, timeline paint at 100 k, analysis at
  100 k, DB size after a simulated 30 days. Every number is measured, dated, and machine-identified.
- The **B2 memory question is closed here with data**: the measured total is compared to the < 80 MB target,
  and if it is not met the mitigations already applied are listed with their individual savings, and the
  remaining gap is presented to the user as an explicit decision with numbers — never quietly restated.

## Architecture

The audit is code, not a checklist: `src-tauri/tests/invariants.rs` re-proves the cross-cutting rules in one
place — the redaction chokepoint scan, the `no rusqlite in collectors/` rule, the `invoke` boundary
(`grep` over `src/` excluding `src/lib/tauri/`), the `bindings_are_current` drift check, version equality
between `Cargo.toml` and `package.json`, and the absence of `dev-tools` symbols in a default build. Each was
introduced by the phase that needed it; this file is the single command that runs them all, so a release
cannot ship with one quietly disabled. Onboarding is a route with its own store so it can be replayed from
Settings ("Show the introduction again") rather than being a one-shot flag.

## Related Code Files

Create:
- `src/features/onboarding/{OnboardingView.svelte,StepWhatItDoes.svelte,StepPrivacy.svelte,StepMonitoring.svelte,StepWatchTargets.svelte,onboarding.svelte.ts}`
- `src/features/settings/sections/{NotificationsSection.svelte,StartupSection.svelte,DiagnosticsSection.svelte,AboutSection.svelte}`
- `src/components/system/PausedBanner.svelte`
- `src-tauri/src/notify/{mod.rs,policy.rs}`
- `src-tauri/src/diagnostics/{mod.rs,export.rs}`
- `src-tauri/src/commands/{diagnostics.rs,monitoring.rs,startup.rs}`
- `src-tauri/tests/{notification_policy.rs,diagnostics_export.rs,invariants.rs,version_sync.rs}`
- `tests/e2e/{onboarding.spec.ts,a11y.spec.ts,keyboard.spec.ts,paused.spec.ts}`
- `scripts/{measure-release.ps1,audit-a11y.ts,verify-installer.ps1}`
- `docs/RELEASE.md`, `docs/ACCESSIBILITY.md`, `docs/perf/release.md`, `docs/UNINSTALL.md`

Modify: `src-tauri/tauri.conf.json` (NSIS per-user, product/binary names, updater disabled with documented reason), `src-tauri/src/app/tray.rs`, `src/routes.ts`, `README.md`, `PRIVACY.md`, `COLLECTORS.md` (final state), `CHANGELOG.md`.

## Implementation Steps

1. `onboarding.svelte.ts` + four step components; completion writes `onboarding_version = 1` so a future material change can re-show it. Step 3 renders the live collector list from `get_collector_status`, including disabled-by-requirement ones with their reasons — the user's first impression is an honest one.
2. `commands/monitoring.rs`: `pause_monitoring`, `resume_monitoring`, `get_monitoring_state`; pausing stops interval ticks and unregisters notification callbacks (not merely dropping events), so a paused app is genuinely idle — asserted by a CPU measurement while paused.
3. `PausedBanner.svelte`: persistent, non-dismissible, with a resume button; asserted present by `paused.spec.ts`.
4. `notify/policy.rs`: the policy as a pure function `decide(event, state, now) -> Notify | Suppress(reason)`; `notification_policy.rs` tests all eight rules including the startup quiet period, coalescing counts, pause suppression, and the backfilled/synthetic exclusions.
5. `commands/startup.rs`: autostart enable/disable. The mechanism must be read from the pinned plugin set (or implemented as a direct `HKCU\...\Run` write through `win32/registry.rs`, which already exists) — verified by reading the value back, not assumed.
6. `diagnostics/export.rs`: assemble the payload, pass every string through redaction, write a `.zip` with a manifest and a `README.txt` explaining what the file contains and what was removed. `diagnostics_export.rs` seeds the redaction corpus, exports, unzips in the test, and asserts none of the corpus secrets, no user name, and no absolute profile path appear in any entry.
7. `tauri.conf.json`: NSIS `installMode: perUser`, install path, shortcuts, product name "What Changed", `mainBinaryName: "WhatChanged"`, identifier, publisher, copyright; updater block present but `active: false` with a comment pointing to `docs/RELEASE.md`. All key names verified against the Tauri 2.11.5 schema in the vendored CLI before writing.
8. `verify-installer.ps1`: builds the bundle, then asserts — installer exists, its metadata product name is "What Changed", the binary is `WhatChanged.exe`, no artifact string contains `WhatChage`, install is per-user (no elevation prompt in a silent install test), and the uninstaller exists.
9. `docs/UNINSTALL.md` + uninstaller data prompt: an NSIS custom page (or, if the pinned Tauri NSIS template does not support one, a first-run-after-uninstall-impossible fallback: a Settings action "Delete all collected data" plus documentation — the honest option chosen only if the template genuinely cannot host the prompt, and recorded in `docs/decisions.md`).
10. `audit-a11y.ts`: Playwright + `@axe-core/playwright` over every route in both themes and both view modes; zero violations required. `keyboard.spec.ts`: a scripted walkthrough (onboarding → dashboard → timeline → details → investigation → compare → settings) asserting focus visibility at each stop and no keyboard trap.
11. `measure-release.ps1`: the final numbers table (idle CPU 30 min, memory split, startup to interactive, timeline paint at 100 k, analysis at 100 k, DB size after a 30-day simulation), written to `docs/perf/release.md` with date and machine identification. Closes B2 with data.
12. `tests/invariants.rs` + `version_sync.rs`: the six cross-cutting assertions listed in Architecture, each with a failure message naming the rule and the phase that introduced it.
13. Documentation finalization: `README.md` (what it is, what it needs, what it cannot see without admin, how to build), `PRIVACY.md` (final, including diagnostics export and uninstall behavior), `COLLECTORS.md` (every collector with cadence, measured cost, admin requirement, blind spots, and Docker's unverified label), `CHANGELOG.md` (0.1.0), and `docs/RELEASE.md` (build, bundle, sign-later steps, updater prerequisites).

## Todo

- [ ] four-step onboarding with consent before collection
- [ ] pause/resume that truly idles the collectors + persistent banner
- [ ] notification policy as a tested pure function
- [ ] autostart verified by reading the value back
- [ ] diagnostics export with a secret-scan test
- [ ] NSIS per-user config with verified key names + installer assertions
- [ ] uninstall data decision honored and documented
- [ ] axe + keyboard audits at zero violations
- [ ] final measured performance table (closes B2)
- [ ] `invariants.rs` running every cross-cutting rule
- [ ] all docs finalized, versions in sync

## Success Criteria

- [ ] A clean install shows onboarding, collects nothing until "Start monitoring", and ends with a first checkpoint.
- [ ] Paused monitoring measurably idles the app (CPU indistinguishable from a stopped process over 10 min) and the banner is always visible.
- [ ] Notification policy tests pass all eight rules; a 50-event burst produces one notification with "and 49 more".
- [ ] Diagnostics export contains none of the corpus secrets, no user name, and no absolute profile path.
- [ ] `verify-installer.ps1` passes every assertion, including that no artifact contains the string `WhatChage`.
- [ ] Uninstalling with "delete my data" chosen leaves no database, log, or settings file behind (verified by path check).
- [ ] axe reports zero violations on every route in both themes; the keyboard walkthrough completes with visible focus throughout.
- [ ] `docs/perf/release.md` contains all six measured numbers with date and machine, and the memory result is stated plainly against the target.
- [ ] `cargo test invariants` passes: chokepoint, no-SQL-in-collectors, invoke boundary, bindings current, version sync, no dev-tools in release.
- [ ] `README.md`, `PRIVACY.md`, `COLLECTORS.md`, `ACCESSIBILITY.md`, `RELEASE.md`, `CHANGELOG.md` all present and accurate.

## Verification Gate

```bash
cd /d/Project/.WhatChage
cargo test --manifest-path src-tauri/Cargo.toml notification_policy diagnostics_export invariants version_sync
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
bun run check && bun run test
VITE_WC_MOCK=1 bun run test:e2e
bun run scripts/audit-a11y.ts
bun run tauri build
powershell -NoProfile -File scripts/verify-installer.ps1
powershell -NoProfile -File scripts/measure-release.ps1
```

## Risk Assessment

| Risk | L×I | Signal | Pre-decided response |
|---|---|---|---|
| Memory target unmet (B2) | **High**×Med | `measure-release.ps1` total exceeds 80 MB | present the measured split (backend vs WebView2), the mitigations already applied with their savings, and the remaining gap as an explicit user decision. Never redefine the metric, never exclude WebView2 children to make the number look good |
| Tauri config key names differ from the assumed schema | Med×Med | `tauri build` config error | read the schema shipped with the pinned CLI; the installer assertions catch a silently wrong product name |
| Unsigned binary triggers SmartScreen and looks broken | **High**×Med | first-run warning on a clean machine | documented in `README.md` and `RELEASE.md` with the exact user-visible text and the signing path; no attempt to hide it |
| Uninstaller cannot host the data prompt in the pinned NSIS template | Med×Med | template lacks a custom page hook | fall back to the documented Settings action plus explicit `UNINSTALL.md` instructions, and record the deviation — do not silently leave data behind without telling the user |
| A11y violations found late and expensive | Med×High | axe failures across many components | the audit runs on every screen and the components were built with semantics from Phase 03 onward; fix in the owning component, never by suppressing a rule |
| Updater misconfigured and silently failing | Med×Med | updater points at a non-existent endpoint | shipped disabled with prerequisites documented — a disabled updater is honest, a broken one is not |
| Diagnostics export becomes a data-leak vector | Low×**High** | the secret-scan test fails | the export is built only from redacted fields and the test is a release gate; if a new field is added later, the scan test fails first |

## Security Considerations

This phase is where the app's promises become auditable claims. Consent precedes collection; the privacy
statement is shown before monitoring starts, not buried in Settings. The diagnostics export is the only
feature that packages data for movement off the machine, so it passes the same chokepoint as storage and is
covered by a dedicated secret-scan test. The installer requires no elevation and writes only under the
user's profile, matching the non-admin design measured in the research; the uninstaller deletes collected
data when asked, so a monitoring database cannot outlive the user's consent. No signing key, updater private
key, token, or endpoint credential is stored in the repository — `RELEASE.md` documents where they must come
from at release time instead.
```

## Phase decomposition rationale

The decomposition follows one rule: **a phase ends at a boundary that can be proven by a command, not by
inspection.** That is why the sequence looks unusual in two places, and both are deliberate.

First, the *entire vertical slice through the data path is built before any collector exists* (04 → 05 →
06 → 07 → 08). Storage, redaction, query and UI are exercised with the synthetic seeder from Phase 07, so
by the end of Phase 08 the app is demonstrable end to end while `collectors/` is still empty. This inverts
the tempting order (collect first, display later) for three reasons: the redaction chokepoint must exist
*before* the first real system string is captured, or the invariant is retrofitted onto data already in the
database; the performance question at 1M rows is answerable on synthetic data and would otherwise surface
after every collector is written; and the UI phases become independent of Win32 risk, which is where the
schedule risk actually lives.

Second, **redaction is its own phase (05) placed before storage (06)**, not a cross-cutting concern applied
later. A `Sanitized<T>` whose constructor is reachable from anywhere is not a boundary, so the phase that
creates the type also creates the test that scans the tree for constructions outside `redact/`. Every
later phase inherits an invariant that a build failure enforces.

The collector work is then split by *cost class and API family*, not by feature list: Phase 10 groups the
three cheap Win32 poll-and-diff collectors that share `win32/handle.rs`, wide-string helpers and the
enrichment cache; Phase 11 groups the two that share the `notify` watcher and the registry reader; Phase 12
groups the five expensive ones that share the COM apartment guard and the message-only window. Grouping this
way means each phase writes its shared `win32/` primitive once (DRY) and each phase's verification gate can
run the same handle-leak and cost-measurement assertions across its whole family.

Analysis is decomposed into engine (14) and screen (15) because the honesty guarantees of §31/§32 are
testable only in the backend: all wording, ordering and confidence come from Rust, and the frontend adds
only fixed labels checked against a shared forbidden-phrase list. Splitting them lets the wording contract
be a `cargo test`, not a UI review.

Visualization (16) is deliberately late and depends on 08/14/15 because it is the only feature that can be
cut down without removing capability — the DOM equivalents it ships alongside are the accessibility
requirement *and* the fallback, so if the memory budget forces a decision, the data is already there to
make it with numbers.

Dependency graph (each phase lists the same in its frontmatter):
`01 → 02 → 03 → 04 → {05 → 06 → 07 → 08}`; `09 ← {05,06}`; `10 ← 09`; `11 ← {05,09}`;
`12 ← {09,11}`; `13 ← {06,07}`; `14 ← {12,13}`; `15 ← {08,14}`; `16 ← {08,14,15}`;
`17 ← {09,11}`; `18 ← {16,17}`.

Safe parallelism, with disjoint file ownership: **08 ‖ 09** (frontend `src/features/**` vs backend
`collectors/**`), **10 ‖ 11** only if `win32/handle.rs` and `win32/widestring.rs` are landed by 10 first —
otherwise sequential, because both would create those files; **13 ‖ 12** (storage/UI vs Win32);
**17 ‖ 12** (both extend `collectors/registry.rs` and `normalize/tables.rs`, so the two table files must be
owned by one branch at a time — the recommended split is 12 first, then 17). Phases 05, 06, 14 and 18 are
never parallelized: each establishes an invariant the others depend on.

The §69 ordering requirement is satisfied by construction: privacy before persistence (05 before 06),
persistence before collection (06 before 09), collection before analysis (09–12 before 14), analysis before
presentation of conclusions (14 before 15), and consent before collection (18's onboarding gates the
monitoring start, with the app shipping paused until the user chooses).

## Risks carried forward

| # | Risk | Where it lands | Carried state |
|---|---|---|---|
| B1 | Rust is not installed; three MSVC installations are present, one of them a prerelease | Phase 01 | Mitigated by a scripted non-elevated `rustup-init` install, a pinned `rust-toolchain.toml` written from the *resolved* version, `vswhere` selection without `-prerelease`, and a `cargo build -v` linker-path proof. Residual risk: the correct MSVC set may be incomplete (missing Windows SDK), in which case Phase 01 stops and reports rather than proceeding — the whole plan is blocked until it passes, which is why it is Phase 01 and 5h |
| B2 | The < 80 MB memory target may be unreachable with WebView2 | Measured in Phase 01, closed in Phase 18 | Open by design. Day one produces the floor from a blank Tauri release build; Phase 18 produces the final split (backend vs summed `msedgewebview2` children). If unmet, the plan's response is to report the split, the mitigations and their individual savings, and the remaining gap as a user decision. No phase is permitted to redefine the metric or exclude WebView2 children |
| B3 | Without admin there is no reliable process-event stream, so a process that starts and exits inside one interval is invisible | Phase 10 | Mitigated, not solved: three secondary signals emit `inferred` events phrased as likelihoods, a 2h ETW spike is time-boxed with the pessimistic default retained, and `COLLECTORS.md` states the blind spot. Residual risk: inferred events could be noisy, in which case the pre-decided response is to require two agreeing signals and demote them to informational |
| R4 | Several third-party API shapes could not be verified in planning (no network): `tauri-specta` builder, `notify-debouncer-full` 0.7 entry point, `bollard` named-pipe constructor and feature name, the `windows` 0.62 WUA module path, Tauri 2.11.5 config/permission key names | Phases 04, 11, 12, 17, 18 | Handled by the global API-shape rule (read the vendored pinned source under `~/.cargo/registry/src/` before writing the call; the vendored source wins; deviations recorded in `docs/decisions.md`) plus a named fallback in each affected phase. No call in this plan should be treated as a verified signature |
| R5 | Docker's live path cannot be validated on this machine (named pipe absent, decision D3) | Phase 17 | Carried as a documentation obligation: the `DockerSource` trait confines the unverifiable code to one file, fixtures prove everything else, and `COLLECTORS.md` plus the Settings help text label the live path unverified until a real daemon confirms it |
| R6 | Ranking quality may be plausible in fixtures and wrong in reality | Phase 14 | Carried with a process: the 12 labelled fixtures are the floor, every real-world miss becomes a new fixture before any weight changes, and a weight may only change alongside a fixture that motivates it |
| R7 | Collector cadences derive from one machine's measurements (drivers 2 128 ms, tasks 1 326 ms, network 1 441 ms, WUA 38.2 ms) | Phases 10, 12 | `spikes/measure-collector-costs.ps1` re-measures on the target machine and writes `docs/perf/collector-costs.md`; cadences live in one table so a slower machine changes data, not code |
| R8 | Unsigned binary will trigger SmartScreen | Phase 18 | Accepted and documented in `README.md` and `docs/RELEASE.md` with the exact user-visible text and the signing path; the updater ships disabled rather than pointed at a placeholder |
| R9 | The repository folder is misspelled `.WhatChage` (decision D1) | Phases 02, 18 | Contained: product name "What Changed", binary `WhatChanged.exe`, package `what-changed`, and an installer assertion that no shipped artifact contains the string `WhatChage` |

## Unresolved questions

1. **Memory target (B2).** If the measured total exceeds 80 MB after the listed mitigations, is the
   preference to (a) keep the target and cut the Three.js layer to its DOM equivalents, (b) keep all
   features and raise the target to the measured number, or (c) keep both and make the 3D layer opt-in at
   first run? The plan measures and reports; it does not choose.
2. **Admin-mode escalation.** The plan is strictly non-admin, so `Security`-channel events and reliable
   process start/stop are permanently out of reach. Should a future opt-in elevated mode be planned (a
   separate scope), or is the documented blind spot acceptable as the product's permanent boundary?
3. **Retention defaults.** Phase 06 implements configurable retention; the default chosen (30 days of
   events, 30 automatic checkpoints, newest watched-file snapshot plus those referenced by retained events)
   is a judgment call. Confirm it matches the intended disk footprint.
4. **Docker validation.** Will a Docker Desktop installation be available before release? If not, the
   feature ships labelled unverified — confirm that is acceptable rather than deferring the collector.
5. **Notification default.** Default is important+critical with a 60 s coalescing window. For a tool aimed
   at a developer's own machine, is "critical only" the better default?
6. **Update channel.** `docs/RELEASE.md` documents the updater prerequisites, but the decision of whether
   an update endpoint will exist at all (and where) is outside this plan's evidence.

Status: DONE_WITH_CONCERNS
Summary: Complete 18-phase, 187h execution-ready plan for What Changed delivered at the specified path; concerns are the three carried blockers (Rust toolchain gate, unproven <80 MB memory target, non-admin process blind spot) plus several third-party API shapes that must be read from the vendored pinned sources rather than trusted from this plan.
