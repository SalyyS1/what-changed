# What Changed

**Evidence state: available product overview; release and publication rows remain known limitations.**

**It worked before. What changed?**

What Changed is a local-only Windows desktop application that keeps a plain-language history of what
changes on your PC — programs, listening ports, services, drivers, Windows updates, network settings,
startup entries, Docker containers, and the config files inside folders you explicitly choose. When
something stops working you describe the symptom, and it shows the changes around that moment ranked
by evidence, with the relationships it can actually prove.

It records **that** a sensitive value may have changed without storing that value. Non-sensitive
evidence the product must explain — such as ports, versions and service states — may be stored locally.

## Status

Under construction. The accepted plan is
[plans/260829-0437-what-changed-desktop-mvp/plan.md](plans/260829-0437-what-changed-desktop-mvp/plan.md)
and owns phase status, implementation scope and acceptance checks; this page does not duplicate its
checkbox counts. The completed phase rows and their named evidence remain the authority. Work for the
remaining collector, trust, performance, release, brand and launch phases is present in the current
tree, but those phases are not release-complete until their own gates and evidence are closed.

A current-source unsigned installer is available for local QA. Its build inputs, executable, notices
and installer hashes are recorded in the [candidate evidence](plans/reports/qa-261027-0059-installed-candidate.md).
The release audit and the installed-app walkthrough keep manual, machine-specific and publication checks open. The public-v1 packet is
**NO-GO** until those rows have evidence and a human approves one frozen version, artifact hash set
and publication manifest. See [release audit](docs/release-audit.md),
[acceptance walkthrough](docs/acceptance-walkthrough.md), and [public-v1 packet](docs/public-v1-go-no-go.md).
These documents separate measured behavior from accepted intent and known limitations.

The [repaired installed candidate](plans/reports/qa-261027-0619-repaired-installed-candidate.md)
includes the workspace membership fix and has passed bounded installed
checkpoint/compare, diagnostics and category-purge checks. Subsequent source
accessibility fixes require another candidate; full acceptance remains open.

## Owner workspace transition

The [Ownvero implementation plan](plans/261027-0817-ownvero-owner-workspace-web-wiki/plan.md)
adds offline app workspaces, confirmed behavior rules, saved change requests and reviewed
Markdown briefs for repeated AI-assisted changes. This work is in progress; the existing
unsigned installer does not include or prove these features. Legacy troubleshooting remains available.

A [development preview build workflow](docs/development-preview.md) prepares the
current owner features and packaged preview runner on a hosted Windows runner.
Runtime notices are a mandatory pre-bundle gate; no current published installer is claimed.

Windows desktop plus a static landing/wiki is approved. Development source now includes
the explicitly permitted preview runner, semantic 2D/list/3D evidence workbench and
a local [landing/wiki source preview](wiki-site/README.md). The user moved clean-Windows,
network-boundary, performance and participant acceptance after feature implementation;
these checks remain open. Remote runner targets stay unavailable. See the [owner architecture](docs/ownvero/architecture.md) for storage,
privacy and compatibility contracts. Public branding and an exact release still require approval.

## Who it is for

Windows developers and technical users who lose hours to "it worked yesterday": a container that stops
resolving a hostname, a service that fails after an update, a `.env` key that changed, a graphics
driver that rolled forward overnight.

## Non-negotiables

| Promise | How it is held |
|---|---|
| No administrator, ever | Of 35 data sources probed under a plain `BUILTIN\Users` token, 31 work; the four that do not are documented as gaps, not worked around. See [SECURITY.md](SECURITY.md). |
| Offline core | The Windows Rust core has a dependency gate against HTTP clients. Explicitly permitted preview sessions use a separate packaged browser; this gate does not prove browser network confinement. |
| No product account, cloud or telemetry | Owner records stay local. Test-role sign-in happens only inside a separately authorized temporary preview browser. |
| No background service | One Tauri app; an authorized preview owns temporary child processes. No scheduled task or helper daemon. |
| Recorder secret values are redacted | A compile-time chokepoint replaces recognized secret-bearing configuration values. Owner prose and reviewed screenshots have separate limits. See [PRIVACY.md](PRIVACY.md). |
| It never claims a cause | The engine reports correlation and evidence; forbidden causal phrasing is a tested constraint. See [ANALYSIS.md](ANALYSIS.md). |

## Stack

Tauri 2 (Rust) + Svelte 5 with runes + TypeScript + Vite + Tailwind 4 + Three.js, over SQLite in WAL
mode. Bun is the development runtime and package manager only. Optional preview-enabled candidates
add packaged Node/Playwright/Chromium; ordinary core builds omit those resources. Exact versions live in `package.json`, `src-tauri/Cargo.toml`, `runner/package.json`, and
`docs/DECISIONS.md`; they are deliberately not duplicated here.

## Build from source

Prerequisites: Windows 10 or 11 x64, the Rust `stable-x86_64-pc-windows-msvc` toolchain via `rustup`
(per-user, no elevation), MSVC build tools with a current Windows 10/11 SDK, Bun, and the WebView2
runtime (already present on Windows 11). [CONTRIBUTING.md](CONTRIBUTING.md) has the verified toolchain
versions and the MSVC selection rule.

```bash
bun install
bun run tauri dev      # debug run with the Vite dev server
bun run verify         # typecheck, frontend tests, rustfmt, clippy, cargo test
bun run tauri build    # release build + NSIS installer
```

For the local Ownvero landing/wiki, run `bun run site:build`, then
`node wiki-site/serve.mjs` and open `http://127.0.0.1:5184/what-changed/`.
This does not publish a website or enable downloads. Preview-runner packaging and
its separate release checks are described in [deployment](docs/deployment-guide.md#ownvero-development-candidate).

Debug builds expose an Event simulator in the developer states harness after `developer_mode` is
enabled. It can seed deterministic `day`, `week`, `burst`, `10k`, `100k` and `1m` corpora (optionally
with entity relationships), or write one of six representative events. Every row is marked
`source_collector = "synthetic"` and still passes through the production sanitizer and writer. The
commands are absent from a normal release handler; a release compiled explicitly with Cargo's
`devtools` feature still requires the runtime setting.

Produces `WhatChanged.exe` and a per-user NSIS installer named
`What Changed_<version>_x64-setup.exe`. A release additionally copies that installer to
`dist-installer/WhatChanged-Setup-x64.exe`, the intended download name. The release script writes
a local checksum manifest with both copies; the preserved rehearsal is not a current-source approval.
Application identifier: `dev.whatchanged.app`. npm
package name: `what-changed`.

## Installing a release build

The NSIS configuration requests a per-user install. A fresh-install UAC and filesystem walkthrough
has not yet been completed for a current-source release candidate.

There is no code-signing certificate for this project. Windows SmartScreen may show an
"Unknown publisher" warning; neither this README nor a matching checksum is a publication approval.
Nothing here is signed and nothing pretends to be; see
[ROADMAP.md](ROADMAP.md) for where signing sits.

The Windows installer may download the WebView2 bootstrapper if the runtime is missing, so
**installation** may touch the network. That is separate from the runtime source gate; an installed
Resource Monitor check is still open. The Settings updater control is an explicit opt-in scaffold; it stays
unconfigured until a signed HTTPS feed and release public key are supplied.

## Where your data lives

`%APPDATA%\dev.whatchanged.app\` — the event database and rolling log files. Development source also
keeps the owner-workspace database and app-owned migration/restore metadata in this folder. The
path is resolved through the OS API for the per-user application data directory, never hardcoded.
Uninstalling keeps that folder unless you explicitly opt in to deleting it. The in-app privacy screen
shows the path and opens the folder so you can check for yourself.

## Documentation

| Document | Owns |
|---|---|
| [PRODUCT.md](PRODUCT.md) | What it does, the screens, the voice, the states |
| [ARCHITECTURE.md](ARCHITECTURE.md) | Process model, module boundaries, data flow, IPC surface |
| [COLLECTORS.md](COLLECTORS.md) | The 12 data sources, their mechanism, cost, and blind spots |
| [ANALYSIS.md](ANALYSIS.md) | How ranking works and why it never asserts a cause |
| [PRIVACY.md](PRIVACY.md) | What is stored, what is redacted, how to delete it |
| [SECURITY.md](SECURITY.md) | Threat model, privilege boundary, capability allowlist |
| [ROADMAP.md](ROADMAP.md) | The 25 phases, their status, and the recorded deferrals |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Toolchain, scripts, code rules, verification gates |
| [docs/wiki-index.md](docs/wiki-index.md) | Canonical offline documentation route and release evidence |
| [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) | Dated license audit gaps; not a completed release notice bundle |

Decisions with live measurements — toolchain selection, dependency choices, memory and query numbers —
live in `docs/DECISIONS.md` and `docs/PERFORMANCE.md`.

## Known limitations

These are real and are not being hidden:

- Programs that start and exit in under a second can be missed. Process detection is a one-second diff
  poll because the event-tracing route requires administrator.
- The Windows `Security` event log channel is not readable without elevation, so it is not subscribed.
- Ordinary Event Log records written while the app is closed are not recovered; only the latest 6005
  boot boundary is queried so “Since last restart” remains truthful.
- About half of running processes do not expose an executable path to a non-elevated reader; those
  events are marked low-confidence rather than guessed.
- The Docker code path has not been exercised against a live daemon on the development machine.
- There is no automatic or background update check. The Settings updater action is explicit opt-in,
  but this build reports that its signed feed is not configured; updates remain a manual download
  until release feed and key custody are established.

Full detail: [COLLECTORS.md](COLLECTORS.md) and [SECURITY.md](SECURITY.md).

## Licensing

MIT selected for public v1. The canonical [`LICENSE`](LICENSE) file is present in this tree. That
license file does not by itself authorize publication of a binary. The
[third-party notice audit](THIRD_PARTY_NOTICES.md) is incomplete; the public-v1 packet still requires
a frozen artifact, checksum, support/security routes and explicit approval.
