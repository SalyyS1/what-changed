# Research Report — Verified Facts for the "What Changed" Plan

**Author:** controller (`/ak:plan --ultra`), 2026-08-29.
**Method note (important, read first):** the two delegated `researcher` subagents
stalled with no output for ~27 minutes and were stopped. Root cause found: the
`WebSearch` tool is returning `web_search: no available accounts` — web search is
unavailable in this session. Approach was changed rather than retried.

What replaced it is **stronger evidence for this purpose**:

1. **Live registry queries** against crates.io and registry.npmjs.org (authoritative
   version data, fetched 2026-08-29).
2. **Empirical capability probes executed on the actual target machine** under the
   real security context the app will run in — a **non-elevated standard-user token**
   (`SALYYY\Salyyy`, `Elevated: False`, groups = `BUILTIN\Users` only; **not** a
   filtered admin token, and **not** in `Performance Log Users` or `docker-users`).

Probe scripts are kept next to this report and are re-runnable:
`probe-nonadmin-part1.ps1`, `probe-nonadmin-part2.ps1`, `probe-snapshot-cost-part3.ps1`.

**Honest limits of this report:** no external documentation was consulted, so this
report does not cite Microsoft Learn or crate docs. It states what the machine
actually did, plus what registries actually report. Anything not directly measured is
listed in §5 as unverified and must be treated as an assumption the implementation has
to confirm.

---

## 1. Version pins (live registry data, 2026-08-29)

### Rust crates (crates.io, `max_stable_version`)

| Crate | Version | Last updated | Note |
|---|---|---|---|
| `tauri` | **2.11.5** | 2026-07-01 | Tauri 2 is current and actively released. No Tauri 3 as `latest`. |
| `tauri-build` | 2.6.3 | 2026-06-30 | |
| `rusqlite` | **0.40.2** | 2026-08-08 | 98.5M downloads. Use `bundled` feature to avoid a system SQLite dep. |
| `rusqlite_migration` | **2.6.0** | 2026-05-28 | Simple `user_version`-based migrations; pairs with rusqlite. |
| `refinery` | 0.9.2 | 2026-06-10 | Alternative migration runner. |
| `notify` | **8.2.0** | 2026-05-02 | 145M downloads; the standard file-watch crate. |
| `notify-debouncer-full` | **0.7.0** | 2026-05-02 | Required — raw `notify` events on Windows are bursty and emit rename pairs. |
| `windows` | **0.62.2** | 2025-10-06 | Official microsoft/windows-rs. Feature-gated; note the ~10-month-old release date. |
| `windows-sys` | 0.61.2 | 2025-10-06 | Raw bindings, faster to compile, no COM ergonomics. |
| `specta` | **1.0.5** | 2026-05-07 | **Reached 1.0 stable** — the long v2-beta period is over. |
| `tauri-specta` | **1.0.2** | 2026-05-08 | 1.1M downloads. Viable for generated typed command bindings. |
| `ts-rs` | 12.0.1 | 2026-01-31 | Alternative: derive-only TS export, no command-binding generation. |
| `tokio` | 1.53.1 | 2026-07-20 | |
| `tracing-subscriber` | 0.3.23 | 2026-03-13 | |
| `tracing-appender` | 0.2.5 | 2026-04-17 | Rolling file appender. |
| `thiserror` | 2.0.20 | 2026-08-08 | Typed errors per §79. |
| `sysinfo` | 0.39.6 | 2026-07-09 | Cross-platform process enumeration; heavier than direct Win32 but portable. |
| `bollard` | 0.21.1 | 2026-08-16 | Docker API client; needs Windows named-pipe support. |
| `gix` | 0.87.1 | 2026-08-24 | Pure-Rust git; large. For §24 plain file reads of `.git/HEAD` are likely enough. |

Tauri plugins — all v2-compatible, all published:

| Plugin | Version |
|---|---|
| `tauri-plugin-single-instance` | 2.4.3 |
| `tauri-plugin-autostart` | 2.5.1 |
| `tauri-plugin-notification` | 2.3.3 |
| `tauri-plugin-updater` | 2.10.1 |
| `tauri-plugin-log` | 2.9.0 |
| `tauri-plugin-window-state` | 2.4.1 |
| `tauri-plugin-dialog` | 2.7.2 |
| `tauri-plugin-opener` | 2.5.4 |

### npm packages (registry.npmjs.org, `latest`)

| Package | Version | Note |
|---|---|---|
| `svelte` | **5.56.10** | Svelte 5 is still `latest`. Runes are the current idiom. |
| `@sveltejs/vite-plugin-svelte` | **7.3.0** | |
| `@sveltejs/kit` | 2.70.3 | Only if SvelteKit is chosen; see §4 decision below. |
| `vite` | **8.2.2** | **Vite 8**, not 5/6. Plugin compatibility must be checked against v8. |
| `typescript` | **7.0.2** | **TypeScript 7 is out.** This is a real risk — see §5. |
| `tailwindcss` | **4.3.3** | |
| `@tailwindcss/vite` | **4.3.3** | The Vite-plugin path (no PostCSS chain needed). |
| `three` | **0.185.1** | |
| `@types/three` | 0.185.4 | |
| `vitest` | **4.1.11** | |
| `@testing-library/svelte` | 5.4.2 | |
| `@tauri-apps/cli` | **2.11.4** | Matches `tauri` 2.11.x. |
| `@tauri-apps/api` | **2.11.1** | |
| `playwright` | 1.62.1 | |
| `svelte-check` | 4.7.6 | |
| `@tanstack/svelte-virtual` | 3.13.36 | Actively published — viable for the virtualized timeline. |
| `bits-ui` | 2.19.0 | Reference for copied/customized primitives per §4. |

---

## 2. Non-admin capability matrix — MEASURED on this machine

Legend: **PASS** = worked as a plain `BUILTIN\Users` token. **FAIL** = denied or empty.

| # | Capability | Result | Measured detail |
|---|---|---|---|
| 1 | WMI `Win32_ProcessStartTrace` subscription | **FAIL** | `Access denied`. This is the classic "cheap push process events" route and it is **not available non-elevated**. |
| 2 | WMI `__InstanceCreationEvent WITHIN 1` on `Win32_Process` | **FAIL (unreliable)** | Subscription registered without error, but **0 events captured** in a 3 s window while a short-lived `cmd.exe` ran and exited. Permitted, but misses short-lived processes and costs a WMI polling thread. |
| 3 | Process enumeration | **PASS** | 368–375 processes. **Only 197 of 368 exposed a readable executable path** (~54%); the rest are elevated/protected and deny `Path`. |
| 4 | File version + publisher (`FileVersionInfo`) | **PASS** | `explorer.exe ver=10.0.26100.8117, company=Microsoft Corporation`. |
| 5 | Authenticode signature read | **PASS** | `status=Valid` for `explorer.exe`. |
| 6 | TCP listeners + owning PID | **PASS** | 37 listeners with `OwningProcess`. IPv6 included (`:::49683`). |
| 7 | UDP endpoints + owning PID | **PASS** | 67 endpoints. |
| 8 | Service enumeration | **PASS** | 301 services. |
| 9 | Service config (`StartMode`, `PathName`) | **PASS** | 301 rows, **299 with readable `PathName`**. Startup-type change detection is viable non-admin. |
| 10 | Specific service query (`Audiosrv`) | **PASS** | `status=Running, startType=Automatic`. |
| 11 | Event Log: `Application` | **PASS** | readable. |
| 12 | Event Log: `System` | **PASS** | readable. |
| 13 | Event Log: **`Security`** | **FAIL** | `Attempted to perform an unauthorized operation.` **Requires elevation — plan must not depend on it.** |
| 14 | Event Log: `Microsoft-Windows-WindowsUpdateClient/Operational` | **PASS** | readable; newest record id=41. |
| 15 | Event Log: `Setup` | **PASS** | readable. |
| 16 | Event Log filtered query (`Application Error` id 1000) | **PASS** | 3 crash records found — the §22 crash signal works non-admin. |
| 17 | Event Log boot/shutdown ids (6005/6006/6008/41/1074) | **PASS** | 5 records; newest `6005 @ 2026-08-25 00:36`. |
| 18 | **WUA COM `QueryHistory`** | **PASS** | **133 history entries readable non-elevated.** Newest: a Defender definition update, `ResultCode=2` (success), 2026-08-28. |
| 19 | `Win32_QuickFixEngineering` | **PASS but useless** | **4 rows vs WUA's 133.** Confirms QFE massively under-reports. Do not use it as the update source. |
| 20 | Last boot time | **PASS** | `2026-08-25 00:35:54`. |
| 21 | Registry `Uninstall` — HKLM 64-bit | **PASS** | 88 subkeys, 69 with `DisplayName`. |
| 22 | Registry `Uninstall` — HKLM `WOW6432Node` | **PASS** | 148 subkeys, 124 with `DisplayName`. |
| 23 | Registry `Uninstall` — HKCU | **PASS** | 38 subkeys, 38 with `DisplayName`. |
| 24 | Startup: `HKCU\...\Run` | **PASS** | 14 values. |
| 25 | Startup: `HKLM\...\Run` | **PASS** | 3 values. |
| 26 | Startup: `HKCU\...\StartupApproved\Run` | **PASS** | 14 values — enabled/disabled state is readable. |
| 27 | Per-user Startup folder | **PASS** | `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup`, 6 items. |
| 28 | Task Scheduler enumeration | **PASS** | **194 tasks readable, 28 logon-triggered.** §21 "scheduled startup tasks" is feasible non-admin. |
| 29 | Driver inventory (`Win32_PnPSignedDriver`) | **PASS** | 215 rows with version + provider + friendly name. Sample: `Intel(R) Iris(R) Xe Graphics v31.0.101.4502 by Intel Corporation` — exactly the §19 human-friendly shape, no GUID needed. |
| 30 | PnP device enumeration | **PASS** | 214 present devices. |
| 31 | Network adapters | **PASS** | 5 adapters. |
| 32 | DNS servers per interface | **PASS** | 1 interface with DNS; `192.168.0.1`. |
| 33 | Default gateway / route table | **PASS** | 2 default routes, gw `192.168.0.1`. |
| 34 | Windows proxy (`HKCU\...\Internet Settings`) | **PASS** | `ProxyEnable=0`, empty `ProxyServer`/`AutoConfigURL`. |
| 35 | Docker named pipe `\\.\pipe\docker_engine` | **FAIL (absent)** | **Docker is not installed on this machine.** §23's "no error spam when absent" path is the *only* path testable here. |

### What this means (the honest version)

- **31 of the 35 probed capabilities work without Administrator** (30 clean PASS
  plus row 19, which is permitted but under-reports so badly it is unusable).
  **Four fail:** rows 1 and 13 are elevation-gated, row 2 is permitted but
  delivered zero events, row 35 is an environment gap (Docker not installed).
  The privilege model in §36 is achievable — this is the single most important
  de-risking result — and it survives all four failures, because each has either
  a measured non-elevated substitute or is out of scope.
- **Two hard *elevation* gaps** (rows 1 and 13), both worth stating plainly in
  `SECURITY.md`/`PRIVACY.md`:
  1. **Push-based process start/stop events require elevation.** `Win32_ProcessStartTrace`
     is denied; ETW real-time kernel sessions are also elevation-gated (not probed here
     — see §5). The non-admin design must be **diff-based polling of the process list**,
     which is cheap (§3) but has a blind spot: processes that start *and* exit inside one
     poll interval are invisible. This must be documented, not glossed.
  2. **The `Security` event-log channel is unreadable.** Any security-audit-derived
     signal is out of scope for a non-elevated build.
- **The other two failures are not elevation problems.** Row 2
  (`__InstanceCreationEvent WITHIN 1`) is *permitted* non-elevated but delivered
  0 events in 3 s while a short-lived `cmd.exe` ran — so it is rejected on
  reliability, not privilege, and the same diff-poll covers it. Row 35 (Docker
  pipe) is absent because Docker is not installed here; the connected path is
  untestable on this machine and must ship marked as such.
- **~46% of running processes deny their executable path.** Process-collector output
  must degrade gracefully to name+PID rather than showing empty fields.
- **`Win32_QuickFixEngineering` is a trap** (4 vs 133). WUA COM is the real source.

---

## 3. Snapshot cost measurements (poll-interval evidence)

Measured with `System.Diagnostics.Stopwatch`, cold then warm.
**These are UPPER BOUNDS** — PowerShell + CIM marshalling dominates; equivalent direct
Win32 calls from Rust are typically an order of magnitude cheaper. Use them to *rank*
cost, not as absolute Rust budgets.

| Snapshot | Cold | Warm | Verdict for the plan |
|---|---:|---:|---|
| Process list | 46.1 ms | **6.2 ms** | Cheap. Diff-polling at 1–2 s is affordable even with PowerShell overhead; from Rust it is far cheaper. |
| Service list | 15.2 ms | **3.3 ms** | Very cheap. |
| Service config (`Win32_Service`) | 392.6 ms | — | Expensive via CIM. Read startup type only on change or at a long interval. |
| TCP listeners | 636.2 ms | 204.0 ms | Cmdlet overhead; `GetExtendedTcpTable` from Rust is sub-millisecond. Do not conclude polling is costly. |
| UDP endpoints | 104.5 ms | — | Same caveat. |
| Uninstall registry scan (3 hives, 274 subkeys) | 47.6 ms | 53.8 ms | Cheap enough to poll on a change-notification wake, not on a timer. |
| **Driver inventory (`Win32_PnPSignedDriver`)** | **2268.7 ms** | **2128.0 ms** | **Expensive and does not warm up.** Never poll on a short timer. Use device-change notification + a rare full sweep, and prefer SetupAPI from Rust over WMI. |
| Network config (adapters + DNS + 55 routes) | 1440.7 ms | — | Expensive via cmdlets. Use change-notification APIs instead of polling. |
| Scheduled tasks (194) | 1325.7 ms | — | Expensive. Poll rarely (e.g. at startup + on Startup-folder/registry change). |
| WUA history count | **38.2 ms** | — | Cheap. Polling WUA history at a slow cadence is fine. |

**Design consequence:** the collectors split cleanly into *cheap-to-diff*
(process, services, listeners, uninstall keys, WUA count) and
*expensive-to-enumerate* (drivers, scheduled tasks, full network config). The latter
group must be notification-driven or slow-cadence. A plan that polls the driver
inventory every few seconds will blow the §37 CPU budget on its own.

---

## 4. Stack decisions this evidence supports

1. **`rusqlite` 0.40 + `bundled` + `rusqlite_migration` 2.6.** Synchronous SQLite on a
   dedicated writer task avoids a second async runtime and gives direct control of WAL,
   `busy_timeout`, prepared statements, and transactions (§10, §83). `tauri-plugin-sql`
   must be **rejected** — it exposes SQL to the frontend, which directly violates §35/§56.
2. **`windows` 0.62 (not `windows-sys`)** for COM-heavy work (WUA, SetupAPI, Task
   Scheduler, `EvtSubscribe`), with tight feature selection to control compile time.
3. **`specta` 1.0 + `tauri-specta` 1.0** for §81's "Rust types are the source of truth".
   Now that specta is 1.0 stable this is a defensible pin rather than a beta bet.
4. **Plain Svelte 5 + `vite-plugin-svelte` 7, not SvelteKit.** A Tauri SPA needs no SSR,
   no server routes, and no adapter; SvelteKit's static adapter would add a router and
   build layer for nothing. Client-side routing can be a small typed store.
5. **`@tailwindcss/vite` 4.3.3**, tokens declared as CSS variables (§41) — matches
   Tailwind 4's CSS-first configuration and satisfies "CSS variables as design tokens".
6. **`three` 0.185 used directly inside a Svelte `$effect`**, not via a wrapper
   framework — §39 demands a small, intentional, disposable scene, and hand-written
   three.js makes the §38 dispose/pause requirements explicit rather than framework-managed.
7. **Process collector = diff-polling from Rust**, not WMI/ETW. Forced by probe #1.
8. **Driver collector = notification + rare sweep**, never a short timer. Forced by §3.
9. **Update collector = WUA COM `QueryHistory` + `WindowsUpdateClient/Operational`**,
   never `Win32_QuickFixEngineering`. Forced by probes #18/#19.

---

## 5. Unverified / risky assumptions

These were **not** measured and must be confirmed during implementation. Each is a real
risk to the plan, not a formality.

| # | Assumption | Why it is risky | How to settle it |
|---|---|---|---|
| 1 | **TypeScript 7.0.2 works with `svelte-check` 4.7.6, `vite-plugin-svelte` 7.3, and Vite 8** | TS 7 is the native-compiler rewrite; toolchain support historically lags a major TS release. A broken `bun run check` would stall Phase 0. | Phase 0 spike: install and run `svelte-check` + `tsc --noEmit` on the scaffold. Fallback: pin TypeScript to the latest 5.x/6.x line and record why. |
| 2 | ETW real-time kernel-process sessions require elevation (or `Performance Log Users`) | Not probed. If it worked non-admin it would beat diff-polling for process events. | Small Rust spike attempting a real-time session as this token. Current token is **not** in `Performance Log Users`, so the pessimistic assumption is the safe default. |
| 3 | `RegNotifyChangeKeyValue` fires for **subkey creation** under the `Uninstall` hives non-admin | Only the *read* was probed, never the notification. Install/uninstall detection latency depends on it. | Rust spike with `REG_NOTIFY_CHANGE_NAME` + `REG_NOTIFY_THREAD_AGNOSTIC`; fall back to the measured 50 ms scan on a slow timer. |
| 4 | `NotifyServiceStatusChangeW` works non-admin for SCM-level create/delete | Only enumeration and config reads were probed. | Rust spike; fall back to the 3.3 ms service diff-poll. |
| 5 | A device-change notification exists that fires on **driver version change** (not just arrival/removal) | `WM_DEVICECHANGE`/`CM_Register_Notification` are arrival/removal-oriented. If none fires on a version bump, only the 2.1 s sweep detects driver updates. | Spike: update a driver and observe. Fall back to a slow sweep (e.g. hourly + on resume/boot) and accept the latency openly. |
| 6 | **RAM < 80 MB is achievable for a Tauri 2 + WebView2 app** | Not measured. WebView2 spawns multiple processes; a realistic Tauri app is commonly well above 80 MB **counting WebView2 children**. §37's target may be unreachable as literally stated. | Measure in Phase 0 and report honestly: split "Rust backend RSS" from "WebView2 total". If the target is unreachable, surface it to the user as a scope question rather than quietly missing it. |
| 7 | Rust toolchain installs cleanly and selects the right MSVC among three installs | Rust is **absent**; three MSVC installs exist (VS2017 BuildTools, `D:\Work\c++`, VS18 Insiders with MSVC 14.44/14.50). Linker mis-selection is a classic Windows Rust failure. | Phase 0 gate: `rustup default stable-x86_64-pc-windows-msvc`, pin via `rust-toolchain.toml`, then compile a hello-world Tauri app before anything else. |
| 8 | NSIS vs WiX choice, per-user install, and WebView2 bootstrapper behavior | Not researched (no web access). §65's `WhatChanged-Setup-x64.exe` implies NSIS. | Read the installed Tauri CLI's own docs/schema during Phase 0 and decide from the shipped bundler options. |
| 9 | Docker collector cannot be tested here at all | No Docker present, Windows 11 **Home** (WSL2/Hyper-V constraints). | Implement against `bollard` with the absent-path verified, and mark the connected path untested-on-this-machine in `COLLECTORS.md`. |
| 10 | `@tanstack/svelte-virtual` 3.x supports Svelte 5 runes | Version is current but Svelte-5 compatibility was not confirmed. | Phase 1 spike with 100k rows; fall back to a hand-rolled windowed list (the timeline only needs fixed-height rows). |
| 11 | Playwright can drive a Tauri 2 window on Windows | §59 says "Playwright where practical" — practicality unconfirmed. | Try `tauri-driver` + WebDriver first; if neither works, keep E2E at the Vitest/component level and say so instead of claiming E2E coverage. |
