---
phase: 01
title: "Foundation & Toolchain"
status: completed
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
- `$ROOT/package.json` scripts (§67): `dev, build, check, test, tauri, rs:fmt, rs:lint, rs:test,
  verify, mem`. All seven §67 invocations resolve against these — `tauri` covers both `tauri dev` and
  `tauri build` — and the shipped app requires neither Bun nor Node, which is the other half of §67.
- App identity: `productName "What Changed"`, `mainBinaryName "WhatChanged"`,
  `identifier "dev.whatchanged.app"`; window 1280×820, `minWidth 900`, `minHeight 600`,
  `decorations: false` (custom chrome in P02).
- Docs (§55/§68/§90) authored with real content, not placeholders: `README.md`, `PRODUCT.md`,
  `ARCHITECTURE.md`, `SECURITY.md`, `PRIVACY.md`, `ROADMAP.md`, `CONTRIBUTING.md`,
  `COLLECTORS.md`, `ANALYSIS.md`, plus `docs/DECISIONS.md`, `docs/PERFORMANCE.md`.
- One `get_app_version` command + one Svelte call, proving IPC end-to-end (§69 P0 acceptance).

**Non-functional**
- **§79 coding standard, wired mechanically here so no later phase has to argue for it.** Rust:
  `rustfmt` + clippy with `-D warnings`, and `[lints.clippy]` in `src-tauri/Cargo.toml` denying
  `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented` (D-04); `unsafe` only where a Win32
  signature requires it and documented at the call site. TypeScript: `strict`, no undocumented `any`.
  CSS: tokens only, no raw hex outside the token files (P02 adds the test). §79's clauses have no other
  owner, so this is the phase its traceability row points at.
- **§88 verification discipline is the shape of the implementation phases, not a step in one**: every phase
  carries a `## Verification Gate` with runnable commands, and this phase creates the scripts those
  gates call (`verify`, `rs:fmt`, `rs:lint`, `rs:test`, `mem`). §89's commit-sized work is likewise
  structural — every phase ends in a lettered commit list and carries a `## Todo (commit boundaries)`
  section. Both are cited here so the §1–§91 audit has a row to stand on rather than an inference.
- **§86 execution behaviour**: this plan is the concrete task list §86 asks for, decisions are recorded
  in `docs/DECISIONS.md` rather than re-asked, and the only questions escalated to the user are the four
  in `plan.md`'s Unresolved questions — each one either a product decision or a metric the brief leaves
  ambiguous, which is exactly §86's permitted set.
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

1. **Git identity — done.** Set repo-locally (`git config --local user.name/user.email`). Global
   config untouched. Verify with `git config --local user.email` before the first commit.
2. **Install Rust (B1) — done.** `rustup` 1.29.1, `rustc`/`cargo` 1.98.0, host
   `x86_64-pc-windows-msvc`, per-user, no elevation. Re-verify with `rustc -vV`.
3. **Pin it — done.** `rust-toolchain.toml` pins `channel = "1.98.0"`,
   `targets = ["x86_64-pc-windows-msvc"]`, `components = ["rustfmt", "clippy"]` — the installed
   version, not a guessed one.
4. **Pin MSVC — done, recorded as D-01.** Two installs found; 14.50.35717 at `D:\Work\c++` chosen,
   the 2017-era 14.16 rejected. `PATH` resolves bare `link` to Git Bash / MSYS2 coreutils, but
   `rustc` locates `link.exe` through its own MSVC detection, so the hazard is inert — no absolute
   linker path is committed to `.cargo/config.toml`, because that would hard-code one machine's
   layout. `scripts/verify-toolchain.ps1` reports the resolved `link` and warns;
   `scripts/dev-shell.cmd` wraps `vcvars64.bat` as the escape hatch.
5. **Frontend deps.** Hand-write `package.json` with the pinned versions from the plan Dependencies
   list, then `bun install`. Scripts: `check` = `svelte-check --tsconfig ./tsconfig.json && tsc --noEmit -p tsconfig.json`;
   `verify` = `bun run check && bun run test && bun run rs:fmt && bun run rs:lint && bun run rs:test`;
   `rs:*` use `--manifest-path src-tauri/Cargo.toml` — **verified accepted** by `cargo fmt`,
   `cargo clippy` and `cargo test` on 1.98.0, so the `cd src-tauri` fallback is not needed.
6. **D2 gate — already measured, apply the outcome.** `bun run check` with `typescript@7.0.2` fails
   before checking a single file: `svelte-check` 4.7.6's own `bin/ts-version-check.js` throws
   "TypeScript 7 support currently requires both TypeScript 7 and TypeScript 6 installed … and
   requires using the `--tsgo` … flag". Recorded as D-02. So: `bun add -d typescript@5.9.3`, re-run
   `bun run check`, and confirm it exits 0. Do not proceed with a red `check`, and do not remove
   `check` from `verify` to make it green.
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
13. **B2 measurement #1 — done.** Release exe run, `bun run mem` twice (20 s and 12 min after launch).
    Recorded in `docs/PERFORMANCE.md`: total 378.6 → 361.7 MB, private working set 103.3 → 77.1 MB,
    backend RSS 27.6 MB, six WebView2 processes. The private figure lands *under* the 80 MB target at
    idle while total is 4.5× over, so §37 is met or missed depending on which figure it means — the
    brief does not say, so B2 escalates to the user rather than picking one.
14. **Commits:** (a) `chore: repo hygiene and product docs`, (b) `chore: pin rust and node toolchains`,
    (c) `feat: tauri 2 + svelte 5 application scaffold`.
## Todo (commit boundaries)

- [x] Git identity set repo-locally; `.gitignore`/`.gitattributes` committed
- [x] `rustup` installed, MSVC selected, `rust-toolchain.toml` pinned, `docs/DECISIONS.md` D-01
- [x] Eleven §55/§68 docs written with real content (`ANALYSIS.md` and `CONTRIBUTING.md` are the two still missing); `ROADMAP.md` lists all 21 plan phases
- [x] `bun install` clean; `bun run check` green on `typescript@5.9.3` (D-02 applied)
- [x] Tauri scaffold runs; IPC round-trip visible in the window
- [x] `bun run tauri build` produces an NSIS installer
- [x] `docs/PERFORMANCE.md` has the baseline memory rows (total **and** private working set) and the B2 scope question

## Success Criteria

- [x] `rustc -vV` host is `x86_64-pc-windows-msvc`; `cargo build --manifest-path src-tauri/Cargo.toml` succeeds with no elevation
- [x] `bun run check` exits 0; `bun run rs:lint` exits 0 with `-D warnings`
- [x] `bun run tauri dev` opens a 1280×820 window titled "What Changed" that cannot be resized below 900×600
- [x] `src-tauri/target/release/bundle/nsis/` contains an installer; the installed exe is named `WhatChanged.exe`
- [x] Release run shows no console window
- [x] `capabilities/default.json` contains exactly the two permitted permissions
- [x] `git log --oneline` shows the three phase commits; no secret-bearing file is tracked

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
| B1: `rustup-init` download blocked | `Invoke-WebRequest` non-200 / TLS error | Resolved — `rustup` 1.29.1 / `rustc` 1.98.0 installed and verified on this machine. Kept as the recovery note for a clean clone: `winget install --id Rustlang.Rustup -e --accept-source-agreements`; if that also fails, stop and report BLOCKED — no other phase can proceed |
| B1: linker ambiguity between the two MSVC installs, or coreutils `link` winning PATH | `error: linker 'link.exe' not found` or LNK1181 | Re-run the failing cargo command through `scripts/dev-shell.cmd`; record the required env in DECISIONS D-01 and route `rs:*` scripts through it |
| D-02 already measured: TS 7 breaks `svelte-check` before it checks anything | `ts-version-check.js` throws on `bun run check` | Pin `typescript@5.9.3` (step 6); never drop `check` from `verify` to make it green |
| Tauri config key rejected | `unknown field` / schema error from the CLI | Fix against `src-tauri/gen/schemas/desktop-schema.json`; note the corrected key in DECISIONS |
| `bunx tauri icon` rejects the PNG | CLI error about size/alpha | Re-export at exactly 1024×1024 RGBA with ImageMagick `-define png:color-type=6` |
| First `cargo build` takes very long | >10 min with no output | Expected (cold crate compile); do not kill. Record wall time in `docs/PERFORMANCE.md` |

## Security Considerations

- `.gitignore` blocks `node_modules/`, `dist/`, `src-tauri/target/`, `.env*`, `*.pfx`, `*.p12`,
  `*.key`, `*.pem`, `*.db`, `*.db-wal`, `*.db-shm`, `logs/`, `diagnostics-*.json`, `*.local`.
- CSP and `withGlobalTauri: false` are set at scaffold time so no phase inherits a permissive default.
- `capabilities/default.json` starts minimal; every later phase must justify each added permission
  in `SECURITY.md`.
- No signing key material enters the repo (P21 documents production signing via environment only).
