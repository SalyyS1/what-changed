---
phase: 11
title: "Windows service collector and installed-application collector"
status: completed
priority: P1
effort: 6h
dependencies: [08]
---

## Overview

Services (§16) by 5 s diff-poll of `EnumServicesStatusExW` for `SERVICE_WIN32` (measured cheap) with
an optional notification seam that currently declines registration and safely retains polling; installed applications
(§17) by diffing the three uninstall registry hives (measured 88 + 148 + 38 keys, ~50 ms total) woken
by `RegNotifyChangeKeyValue` with a 5-minute fallback poll. Both are read-only, non-elevated, and
measured-PASS in the capability matrix.

## Requirements

**Functional**
- Service collector: `OpenSCManagerW(NULL, NULL, SC_MANAGER_ENUMERATE_SERVICE)` (read-only rights —
  never `SC_MANAGER_ALL_ACCESS`), `EnumServicesStatusExW(SC_ENUM_PROCESS_INFO, SERVICE_WIN32,
  SERVICE_STATE_ALL)` with the size-then-fill buffer pattern and reusable buffer. Driver inventory is
  owned by P12; including 443 driver rows beside 301 Win32 services produced 744 rows on the acceptance
  workstation and made this phase's 12 ms fast-cycle gate impossible. Diff on service name; emit
  `service/started`, `service/stopped`, `service/installed`, `service/removed`,
  `service/start_type_changed`. Start type + display name via `QueryServiceConfigW` on an
  `OpenServiceW(SERVICE_QUERY_CONFIG)` handle, fetched only for services that are new or changed.
- Query configuration immediately for new/status-changed services and run an owned synchronous full
  configuration reconciliation every 15 minutes. This preserves config-only start-type detection
  without doing hundreds of `OpenServiceW` calls on every five-second cycle or detaching a worker.
- `NotifyServiceStatusChangeW(SERVICE_NOTIFY_CREATED|DELETED)` remains an optional future wake. The
  current Windows provider declines registration because the callback lifetime cannot yet be cancelled
  safely, logs at DEBUG, and continues on the pure 5 s poll. Correctness never depends on it.
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
- Both collectors publish to the shared entity store so `service:<Name>` and the stable application
  `(hive, uninstall-subkey)` identity are stable across restarts, and the app collector emits a
  `relationships` row `application:<x> --provides--> service:<y>` when a new service's binary path lives
  under a known `InstallLocation` (this is what lets P15 say "Docker Desktop update → com.docker.service
  restart → port change").

**Non-functional**
- Steady service cycle ≤ 12 ms p95 (baseline 3.3 ms); the separately measured 15-minute full-config
  refresh ≤ 500 ms; registry sweep ≤ 150 ms p95 (baseline ~50 ms) and runs off the fast tick
  (`cost_hint: Moderate`).
- Boot-time storm control **without downtime blindness**: on app start the first cycle diffs against
  the `collector_baselines` row P04 persists, not against nothing. A baseline-only cycle (zero events)
  happens in exactly two situations — the collector has never run, or the database was reset — and
  those are the only two where there is genuinely nothing to compare with. Every other start compares
  the machine now against the machine as it was when the app last ran, so a service installed or an
  application updated while the app was closed **is reported**, with `occurred_at_is_uncertain = 1`
  because only the detection time is known. The old rule (first cycle always a baseline) made the
  recorder blind to precisely the window a user is most likely asking about.
- First-cycle diffs are coalesced by the P08 supervisor into one "changed while you were away" batch
  per collector, so a two-week absence reads as a summary rather than as an incident.

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
`src-tauri/tests/baseline_no_storm.rs`, `src-tauri/tests/downtime_detection.rs`.
**Modify** — `src-tauri/src/collectors/registry.rs`, `src-tauri/src/platform/traits.rs`,
`src-tauri/Cargo.toml` (`Win32_System_Services`, `Win32_System_Registry`),
`src-tauri/src/domain/event.rs` (new event types), `src-tauri/src/storage/entities.rs`,
`docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. `platform/windows/services.rs`: SCM open with minimum rights, reusable size-then-fill enumeration,
   `QueryServiceConfigW` for new/status-changed services plus an owned 15-minute full reconciliation,
   handle guards, and the explicitly optional `try_register_notify() -> bool` fallback seam.
2. `platform/windows/registry.rs`: enumerate the three hives into `Vec<UninstallEntry>`; a notify
   thread per hive that signals an `mpsc` the app collector selects on.
3. `collectors/services.rs`: diff + baseline-first-cycle + start-type change detection.
4. `collectors/applications.rs`: diff + version compare + `InstallLocation` → service relationship.
5. Human naming: trim `DisplayName` for presentation; identity remains `(hive, subkey)` so a vendor
   rename or version suffix cannot merge two registry entries or split one application across restarts.
6. Tests with scripted providers: install/remove/update/version-downgrade; service start/stop/install/
   remove/start-type; `SystemComponent` filtering; baseline-no-storm for both; relationship inference.
7. Manual: `bun run tauri dev`, then `net stop`/`net start` a user-controllable service (e.g. `Spooler`
   requires admin — use a Docker/VMware service if present, otherwise start any user-startable service)
   and confirm one event pair; install a small MSI/portable app and confirm `application/installed`.
8. Commits: (a) `feat: service provider and collector`, (b) `feat: registry uninstall provider`,
   (c) `feat: application collector with version diffing`, (d) `feat: application to service
   relationships`, (e) `test: service and application suites`.

## Todo (commit boundaries)

- [x] Service enumeration with minimum SCM rights + handle guards
- [x] Service diff collector with optional notify wake
- [x] Registry uninstall enumeration + notify threads
- [x] Application diff collector with version phrasing
- [x] Install-location → service relationship inference
- [x] Test suites incl. baseline-no-storm and downtime detection

## Success Criteria

- [x] Scripted: 200 services, one stops → exactly one `service/stopped`, others silent
- [x] Scripted: start type `Automatic → Manual` → one `service/start_type_changed` with before/after
- [x] Scripted: 148 uninstall keys of which 60 are `SystemComponent = 1` → 88 tracked entities
- [x] Scripted: version `4.35.1 → 4.36.0` → summary exactly `Docker Desktop updated 4.35.1 → 4.36.0`,
      importance `informational`
- [x] First cycle with **no persisted baseline** emits zero events for both collectors
      (baseline-no-storm test)
- [x] First cycle **with** a persisted baseline reports downtime change: store a baseline, remove one
      service and bump one application version behind the collector's back, restart, and expect exactly
      one `service/removed` and one `application/version_changed`, both with
      `occurred_at_is_uncertain = 1`
- [x] Real-OS run: service cycle ≤ 12 ms p95, registry sweep ≤ 150 ms p95 over 20 sweeps; owned
      full-config refresh ≤ 500 ms
- [x] Notify-registration failure path leaves the collector `Running` (fault-injected test)

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test collector_services --test collector_applications
cargo test --test collector_windows_performance -- --ignored --nocapture
cargo clippy --all-targets -- -D warnings
```

## Completion evidence — 2026-09-19

- Scripted acceptance: 8 service tests and 10 application tests passed, including baseline-no-storm,
  downtime uncertainty, config-denial recovery, deepest-root ownership, startup readiness, and atomic
  relationship overflow rejection.
- Real Windows gate (20 steady sweeps): service p95 6.519 ms, registry p95 31.9952 ms; the separately
  owned full-config refresh was 173.0581 ms. All are below their budgets.
- Full Rust gate passed after implementation; formatting and strict Clippy passed. Production
  composition activates process, listeners, files, services, and applications with shared expiring
  evidence maps. Registry notification handles remain guard-owned until watcher threads join.

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `NotifyServiceStatusChangeW` unusable non-elevated (research §5, unverified) | `try_register_notify()` returns false | Already the designed fallback: pure 5 s poll. Worst case is 5 s latency, which the §16 requirement tolerates |
| `RegNotifyChangeKeyValue` misses `HKCU` changes | An app install shows up only at the 300 s fallback | Accept: the fallback bounds staleness at 5 minutes, and the requirement is "notices installs", not "notices instantly". Documented in the descriptor's cadence text |
| Registry sweep on a machine with 1000+ entries | Sweep > 500 ms | Sweep is `Moderate`/staggered and gated by `QueryBusy`; if p95 > 500 ms the hives are swept one per tick (3-tick rotation) rather than all at once |
| MSI repair/self-update churn | Repeated `application/updated` for the same version | Diff compares normalized versions; equal versions emit nothing. A same-version key rewrite is ignored |
| Handle leak on the early-return paths | Handle count grows | `Drop` guards and guard-owned watcher stop handles; the 10k whole-provider soak remains a Phase 20 hardening gate |
| Two collectors both claim the same service entity | Duplicate entity rows | `entities` has a unique index on `(kind, external_id)` from P04; inserts are upserts |

## Security Considerations

- Read-only rights everywhere: `SC_MANAGER_ENUMERATE_SERVICE`, `SERVICE_QUERY_CONFIG`, `KEY_READ`. The
  app cannot start, stop, or reconfigure a service even if compromised.
- `HKCU` reading is limited to the current user's own uninstall hive; no other user's hive is opened.
- Service binary paths and install locations pass through the P03 sanitizer (paths embed usernames).
