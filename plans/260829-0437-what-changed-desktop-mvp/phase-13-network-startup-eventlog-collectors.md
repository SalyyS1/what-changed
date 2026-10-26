---
phase: 13
title: "Network config, startup/scheduled-task, and Windows event-log collectors"
status: completed
priority: P1
effort: 8h
dependencies: [12]
---

## Overview

The last three collectors, and the three most expensive to enumerate: network configuration (§20,
measured 1440 ms), scheduled tasks and startup entries (§21, measured 1325 ms for 194 tasks of which 28
are logon-triggered), and Windows event logs (§22) which are a *subscription*, not a sweep. All three
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

- Downtime coverage for the two diff-polled collectors: network configuration and startup entries both
  persist their last-seen state to `collector_baselines` (P04) and diff against it on the first cycle
  after a restart, so a DNS server, proxy, or `Run` key changed while the app was closed is reported
  with `occurred_at_is_uncertain = 1`. The event-log collector needs no baseline — it subscribes with
  `EvtSubscribeToFutureEvents`, so log records written during downtime are genuinely gone and
  `COLLECTORS.md` says so under Known limitations rather than implying coverage.

**Non-functional**
- Network sweep ≤ 2500 ms p95 (baseline 1440 ms), task sweep ≤ 2500 ms p95 (baseline 1325 ms); both
  behind `QueryBusy`, staggered so they never overlap, hard 15 s timeout.
- Event-log callback does no parsing or DB work and uses bounded render + non-blocking enqueue. The
  original <1 ms target remains unclaimed because this workstation did not manufacture a machine Event
  Log record solely for timing; queue overflow/render loss is surfaced as collector failure.
- A 500-error log burst produces ≤ 20 stored events (dedup + filters), measured by a fault-injection test.

## Architecture

`collectors/network.rs`, `collectors/startup.rs`, `collectors/eventlog.rs` with
`NetworkProvider`/`TaskProvider`/`EventLogSource` traits; `platform/windows/network.rs`
(`Win32_NetworkManagement_IpHelper`), `platform/windows/tasks.rs` (`Win32_System_TaskScheduler` COM on
its own initialized thread, same discipline as P12's WUA), `platform/windows/eventlog.rs`
(`Win32_System_EventLog` — `EvtSubscribe`/`EvtRender`; callback is an `extern "system"` fn receiving
an opaque integer key into an `Arc` registry, so failed close/callback rundown cannot dereference freed
Rust memory).
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
`src-tauri/tests/phase13_query_latency.rs`, `docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. Capture the event-log fixtures first: `wevtutil qe Application /c:5 /f:xml` (read-only, non-elevated,
   already proven readable) into `tests/fixtures/eventlog/`, then hand-strip anything machine-identifying.
   The parser is written against real XML, not imagined XML.
2. `platform/windows/eventlog.rs`: subscription per channel, `extern "system"` callback that renders and
   pushes to an `mpsc::SyncSender` with `try_send` (drop + count on full, never block the callback),
   registry-key removal on stop; callbacks already in flight retain their own `Arc`.
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

- [x] Real event-log fixtures captured
- [x] EvtSubscribe + callback + rendering with no callback DB work
- [x] Event-log collector with XPath filters and dedup
- [x] Evidence API wired into P09/P12
- [x] Network provider + per-field diff collector
- [x] Task Scheduler provider + startup collector
- [x] SinceLastRestart switched to boot events
- [x] Test suites incl. storm control

## Success Criteria

- [x] All 10 event-log fixtures parse to the expected `(provider, event_id, level, message)` tuples
- [x] 500 identical Application errors in 60 s → ≤ 20 stored events, one with a count ≥ 480
- [x] `Security` channel is never subscribed (`rg -n '"Security"' src-tauri/src/collectors` → comment only)
- [x] Scripted: proxy `off → 127.0.0.1:8888` → one `network/proxy_changed`, importance `important`,
      before/after present
- [x] Scripted: DNS `192.168.1.1 → 8.8.8.8` → one `network/dns_changed`
- [x] Scripted: 194 tasks with 28 logon-triggered → 28 startup entities, 166 recorded at `informational`
- [x] `exit_kind` test: with a matching 1000 error → `Unexpected`; without → `Unknown` (never guessed)
- [x] `SinceLastRestart` test: a fixture stream with a 6005 boot event returns exactly the post-boot slice
- [x] Downtime detection: persist a network + startup baseline, change the DNS server and add a `Run`
      key behind the collectors' back, restart, and expect one `network/dns_changed` and one
      `startup/run_key_added`, both with `occurred_at_is_uncertain = 1`
- [x] `COLLECTORS.md` states that event-log records written while the app was closed are not recovered
- [x] Real-OS: network sweep 15.2741 ms p95 and `get_events` 3.0725 ms p95 during overlapping
      network/task queries. This host's Task Scheduler root is unavailable, so only its safe-unavailable
      path (5.0435 ms p95) is claimed; populated-task latency remains a host-specific verification gap.

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test collector_network --test collector_startup --test collector_eventlog \
           --test eventlog_storm --test evidence_corroboration --test since_last_restart \
           --test phase13_query_latency
cargo clippy --all-targets -- -D warnings
cargo test --test architecture --test importance_calm
```

## Completion evidence — 2026-09-20

- Scripted suites: network 5/5, startup 10/10, Event Log 7/7, storm 2/2, evidence 9/9,
  SinceLastRestart 4/4; architecture 9/9 and calmness 1/1.
- Cross-drain dedup is owned by the storage writer: 500 identical records persist as one row with
  `count=500`; either process/update versus Event Log arrival order is reconciled durably.
- Subscribe-first startup performs one bounded reverse System query for the latest 6005 boundary; the
  remaining downtime Event Log gap is explicit in `COLLECTORS.md`.
- Real Windows: network p95 15.2741 ms for 46 adapters; task safe-unavailable path p95 5.0435 ms;
  overlapping query p95 3.0725 ms; forced timeout recovered with no overlap; 200 Event Log
  subscribe/unsubscribe cycles passed.
- Environment note: Task Scheduler root is unavailable on this host with `0x80070003` through both COM
  and `schtasks`; the provider preserves prior task state and continues Run/Startup-folder collection.

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `EvtSubscribe` callback ABI/lifetime mistake → crash | Access violation shortly after subscribing | Native code receives an opaque integer key. Registry lookup clones an `Arc`; removal makes late callbacks no-ops even if `EvtClose` fails. A dedicated stress test subscribes/unsubscribes 200 times |
| Event-log volume swamps the DB | `history.db` grows > 100 MB in a day | XPath level filters + dedup + the P04 30-day retention; if growth still exceeds budget, `Application` narrows to Level<=2 only (drop the explicit EventID list) — a one-line change with a test |
| Task Scheduler COM enumeration hangs on a corrupt task | Sweep hits the 15 s timeout repeatedly | Timeout abandons the cycle and the shared coordinator prevents overlap. Returned partial walks merge readable rows while suppressing uncertain removals; a native call that never returns remains busy until app restart rather than spawning more COM work |
| `NotifyIpInterfaceChange` callback fires on its own thread while the collector is stopping | Use-after-free on shutdown | Native code receives an opaque registry id; callbacks clone an `Arc`, failed cancellation is safe, and callback panics are contained at the FFI boundary |
| Proxy read from the wrong place (per-user vs WinHTTP machine proxy) | A proxy change made in Settings is missed | MVP reads the per-user WinINET keys (that is what browsers and most tooling use); the WinHTTP machine proxy is listed in `ROADMAP.md` as a known gap rather than silently claimed |
| VPN detection false positives | Hyper-V/loopback adapters reported as VPN | Detection requires tunnel `IfType` **or** a known-vendor token; anything else is `network/adapter_changed`. Fixture test includes a Hyper-V adapter asserting no VPN event |

## Security Considerations

- Only the four measured-readable channels are subscribed; the unreadable `Security` channel is
  explicitly excluded with a comment, so the app never appears to need elevation.
- Event-log `<Data>` values frequently contain paths and usernames → every rendered field goes through
  the P03 sanitizer before storage, and the leak test corpus gains an event-log case.
- Network data (IPs, DNS servers, SSIDs, proxy hosts) is local-only and never leaves the machine; the
  privacy screen (§33, §49) lists these categories by name.
- Task action paths and run-key command lines are stored **redacted** — run keys are a known place for
  embedded credentials.
