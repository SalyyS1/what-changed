# Contributing

This file is the toolchain and quality contract. Everything in it was run on the
development machine — where a version is named, it is the version that was
actually installed and verified, not a version read from documentation.

## Toolchain

| Tool | Verified version | How it is pinned |
|---|---|---|
| `rustup` | 1.29.1 | per-user install, no elevation |
| `rustc` / `cargo` | 1.98.0, host `x86_64-pc-windows-msvc` | `rust-toolchain.toml` |
| MSVC toolset | 14.50.35717 | selected in `docs/DECISIONS.md` D-01 |
| Windows SDK | 10.0.26100.0 | comes with the MSVC install |
| Bun | 1.3.11 | `bun.lock` pins every package |
| Git | 2.52.0 | — |
| WebView2 | Evergreen runtime, present on Windows 11 | — |

Rust version bumps are deliberate: change `rust-toolchain.toml`, run
`bun run verify`, and record the result in `docs/DECISIONS.md`. Do not bump it as
a side effect of another change.

### MSVC selection

Two MSVC installs exist on the development machine and `vswhere` reports both.
14.50.35717 is used; the 2017-era 14.16 is rejected — see `docs/DECISIONS.md`
D-01 for the reasoning and the failure signature you would see if the wrong one
were picked.

One trap worth knowing before you hit it: `PATH` resolves the bare name `link`
to Git Bash / MSYS2 coreutils, not to MSVC's `link.exe`. This does **not** break
the build, because `rustc` locates the linker through its own MSVC detection
rather than a bare `PATH` lookup — but it is the reason a link failure here would
look impossible from `vswhere` output alone.

- `scripts/verify-toolchain.ps1` prints the resolved `link` and warns when MSVC
  is not first.
- `scripts/dev-shell.cmd` wraps the chosen install's `vcvars64.bat` and is the
  escape hatch if a cargo command ever cannot find the linker.

No absolute linker path is committed to `.cargo/config.toml`: that would
hard-code one machine's directory layout into the repository.

## Commands

```bash
bun install            # frontend deps
bun run dev            # Vite dev server alone (port 5183, strict)
bun run tauri dev      # debug app + dev server
bun run check          # svelte-check + tsc --noEmit
bun run check:bindings # regenerate the TS bindings and fail on any difference
bun run gen:bindings   # regenerate and write src/lib/tauri/bindings.ts
bun run test           # Vitest
bun run rs:fmt         # cargo fmt --check
bun run rs:lint        # cargo clippy -D warnings
bun run rs:test        # cargo test
bun run verify         # all six gates above, in order
bun run build:standalone # internal diagnostic executable with embedded frontend
bun run tauri build    # release build + NSIS installer
bun run mem            # memory measurement against a running release build
bun run toolchain      # report the resolved toolchain and warn on hazards
```

`bun run verify` is the gate. A change that does not pass it is not ready,
and the correct response to a red `verify` is to fix the code — never to remove a
step from the script to make it green.

A red `check:bindings` has one correct fix: `bun run gen:bindings`, then commit
the regenerated file. It means a Rust type changed and the committed
TypeScript no longer matches it, which is the whole point of the gate.

The `rs:*` scripts pass `--manifest-path src-tauri/Cargo.toml` from the repo
root. That flag is accepted by `cargo fmt`, `cargo clippy` and `cargo test` on
1.98.0 (verified), so there is no need to `cd src-tauri` first.

Bun is a development dependency only. Nothing in a shipped artifact requires
Bun, Node, npm, pnpm or yarn — the frontend is compiled at build time and
embedded in the binary.

### Standalone native diagnostics

Use `bun run build:standalone` for native UI and engineering checks without a
Vite server. [The script](scripts/build-standalone.ps1) builds the frontend, then
builds `what-changed` in release mode with `--no-default-features` and
`--features tauri/custom-protocol`, which embeds the frontend. A raw
`cargo build --release` without that feature retains the configured development
URL, `http://localhost:5183`; its UI needs the Vite server.

The script resolves the output directory through Cargo metadata, including
`CARGO_TARGET_DIR`, and selects the executable reported by Cargo's build output.
This also handles target-specific subdirectories from `CARGO_BUILD_TARGET` or
Cargo's `build.target` configuration. It prints that executable's path and
SHA-256; the default host path is `src-tauri/target/release/what-changed.exe`.
Before building, it refuses any running `what-changed.exe` within the configured
target directory, then checks the selected executable again before hashing.
It does not launch or stop applications.

This command produces an internal diagnostic executable. It does not create an
installer, prepare reviewed notices, run release gates, or approve publication.
Use `bun run tauri build` for the NSIS installer; `bun run release` and the
[release audit](docs/release-audit.md) own release validation requirements.

## Repository layout

`src/` is the Svelte frontend, `src-tauri/` is the Rust application, `plans/`
holds the implementation plan, `docs/` holds decisions and measurements, and the
capitalized root Markdown files each own one subject (see the table in
`README.md`).

Inside `src-tauri/src/`:

| Directory | Owns |
|---|---|
| `app/` | wiring, shared state, lifecycle |
| `commands/` | the IPC surface, and no logic |
| `domain/` | the event and entity types — the source of truth, exported to TypeScript |
| `storage/` | repositories, migrations, maintenance, recovery |
| `analysis/` | symptom parsing, scoring, chains, explanations |
| `privacy/` + `redact/` | the redaction chokepoint |
| `collectors/` | platform-agnostic diff-to-event logic, written against provider traits |
| `platform/windows/` | the only place Win32 and COM are allowed |

Collectors depend on provider traits rather than on Win32 directly, so each one
is unit-testable against a fake snapshot and the rest of the tree stays free of
`#[cfg(windows)]`.

## Rust rules

- **No `unwrap()` or `expect()` in shipped code.** Enforced mechanically:
  `[lints.clippy]` in `src-tauri/Cargo.toml` denies `unwrap_used`,
  `expect_used`, `panic`, `todo` and `unimplemented`, and `rs:lint` runs under
  `-D warnings`. Test modules opt out with an explicit `#[allow(...)]` so every
  exception is visible in review. The reason is product-level, not stylistic:
  the app sits in the tray all day, so a panic is a silent disappearance — the
  one failure mode this product cannot have.
- Typed errors with `thiserror`. A collector failure is isolated and reported as
  collector health; it never takes down the app.
- `unsafe` only where a Win32 signature requires it, with a comment stating the
  invariant that makes it sound.
- Small modules. Prefer a new file over a growing one.

## TypeScript rules

- `strict` on. No `any` without a comment saying why.
- Business logic lives in Rust. Components render and dispatch.
- Nothing outside `src/lib/tauri/` imports `@tauri-apps/api` directly — the typed
  wrappers are the only door.
- `three` is imported only under `src/three/`, enforced by a boundary test.
- Derived state over duplicated state.

## CSS rules

Design tokens in `src/styles/tokens.css`, re-exported to Tailwind through
`@theme`. No one-off colour literals in components, and no second spacing scale.

## Types cross the boundary once

Rust domain types are the source of truth. TypeScript types are generated from
them into `src/lib/tauri/bindings.ts`, and a parity check fails the build if the
committed copy drifts from a fresh generation. Do not hand-edit the generated
file, and do not define a payload shape twice.

## Adding a dependency

Answer these before you do, and record the answer in `docs/DECISIONS.md` if the
dependency lands:

1. Can the standard library or something already present do this cleanly?
2. Is it actively maintained?
3. How much does it add to the binary?
4. Does it need a background runtime?
5. Does it widen the attack surface?

Two standing prohibitions, both of which a release gate enforces:

- **No HTTP client** may reach the shipped Windows binary. `reqwest` appears in
  `Cargo.lock` because Tauri depends on it for non-Windows targets; it is not in
  the `x86_64-pc-windows-msvc` graph, and the gate resolves that target rather
  than grepping the lockfile.
- **No Electron, no Go, no second process.** Not temporarily, not behind a flag.

## Tests

Rust: unit tests beside the code, integration tests in `src-tauri/tests/`.
Migration tests, redaction leak tests, ranking-fixture tests and collector parser
tests are all required rather than optional.

Frontend: Vitest with Testing Library.

Two suites are load-bearing and must be able to fail:

- **Redaction leak suite** — plants realistic secrets (`sk-` keys, `Bearer`
  tokens, JWTs, AWS-style keys, PEM blocks, `postgres://user:pass@host`,
  `mongodb://`, cookies, session ids) and asserts no raw value reaches the
  database *or* the log files.
- **Ranking fixtures** — assert that a configuration-to-port causal chain
  outranks an unrelated application update, including when the unrelated update
  is moved closer in time to the symptom. A ranking test that passes on recency
  alone proves nothing.

## Commits

Conventional commits, scoped to one coherent change. Do not put plan ids, phase
numbers or audit finding codes in commit messages, code comments, migration names
or test names — describe the behaviour or the invariant directly, because the
code outlives the plan.

Never commit: `.env` files, certificates or keys of any kind, a `history.db`, log
files, or a diagnostics bundle. `.gitignore` covers all of these; check
`git status` before staging rather than relying on it.

## Documentation

Update a document when behaviour, setup, commands, configuration, architecture,
security posture or a durable decision changes — not on every commit. Two rules
make the docs trustworthy:

- `docs/DECISIONS.md` records a decision only once it has been **measured on a
  real machine**, with the evidence and the consequence of reversing it.
- `docs/PERFORMANCE.md` contains measured numbers only. No estimates, and no
  figure carried over from another machine.
