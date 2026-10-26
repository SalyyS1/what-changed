---
phase: 09
title: "Process and network-listener collectors (diff-polled, path-degrading)"
status: in-progress
priority: P1
effort: 6h
dependencies: [08]
---

## Overview

First real data. Process start/stop by 1 s diff-polling (§14) and TCP/UDP listener open/close by 2 s
diff-polling (§15), both non-elevated. Measured costs: process snapshot 6.2 ms warm, listener
enumeration is in the same cheap class — so both are `Cheap`/`Poll`. This phase also establishes the
`process → listens_on → port` relationship rows that the P15 analysis engine walks, and produces the
§15 example output verbatim.

## Requirements

**Functional**
- Process collector: `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)` +
  `Process32FirstW`/`Process32NextW` for pid, ppid, exe name; `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)`
  + `QueryFullProcessImageNameW` for the full path. Diff two consecutive snapshots keyed by
  `(pid, creation_time)` — never pid alone (pid reuse would forge a stop+start pair). Creation time via
  `GetProcessTimes` when the handle opens; when it does not, key on `(pid, exe_name)` and mark the
  event `identity_confidence: Low`.
- Measured reality: ~46% of processes deny `QueryFullProcessImageNameW` non-elevated. Those events are
  emitted with `entity_name` = image name, `path: null`, and `details.path_unavailable = true`; the UI
  shows the name without a path and never renders "unknown". This is normal operation, not an error —
  no log line per occurrence (one aggregate count per hour at DEBUG).
- Events: `process/started`, `process/stopped`. `process/stopped` carries `exit_kind:
  Normal|Unexpected|Unknown`; `Unexpected` is only ever set when the Event Log collector (P13) has a
  corroborating Application Error / 1000 / 1026 entry for the same image within ±5 s — otherwise
  `Unknown`. **B3 is documented in the UI**: the collector descriptor's `what_it_watches` says
  "Programs starting and stopping (checked every second — programs that live less than a second may be
  missed)".
- Noise control (§53): a process image with more than 5 matched start/stop pairs in a 60 s episode
  receives one `process/churn` summary with the final count; raw start/stop evidence stays append-only
  and promptly visible. See the explicit churn clarification below. Browser/helper subprocess trees (parent is a known browser and
  the image matches the parent) are recorded at `importance: informational` and excluded from
  notifications entirely.
- Listener collector: `GetExtendedTcpTable(TCP_TABLE_OWNER_PID_LISTENER)` and
  `GetExtendedUdpTable(UDP_TABLE_OWNER_PID)` for AF_INET and AF_INET6; diff on
  `(protocol, local_addr, local_port)`. Events `port/opened`, `port/closed` with `owning_process`
  resolved from the live process map (not a second enumeration).
- Relationships: on `port/opened`, insert `relationships` rows `process:<id> --listens_on--> port:<id>`
  and the reverse `port:<id> --opened_by--> process:<id>`, both with the event id, so P15's graph walk
  finds them from either direction. §15's output shape — "Port 4000 opened by: api.exe" and "These
  events occurred 300ms apart" — is produced from the relationship row plus the two timestamps.
- Both collectors implement the atomic `snapshot_batch()` boundary over injected providers, so tests
  drive scripted process/port tables without adding a legacy `snapshot()` persistence path.

**Non-functional**
- Process cycle ≤ 15 ms p95 on this machine (measured baseline 6.2 ms warm); listener cycle ≤ 15 ms.
- Steady-state allocation: both provider row buffers and pid/endpoint indexes are pre-sized and
  reused; the session-local process publication uses a two-`HashMap` swap. The serializable ordered
  process snapshot intentionally owns `BTreeMap` nodes/strings each cycle because it can become an
  atomic durable target; that remaining cost is accepted only while the real 50-cycle p95 gate stays
  within 15 ms and is revisited by Phase 20 allocation profiling.
- Zero events when nothing changed (diff-empty is the common case) — verified by a 30 s idle test.

## Architecture

`collectors/process/mod.rs` + `collectors/listeners/mod.rs` hold the diff logic and take
`&dyn ProcessProvider` / `&dyn SocketProvider`. `platform/windows/process.rs` and
`platform/windows/sockets.rs` hold every Win32 call (the already-resolved `windows` 0.61 features
`Win32_System_Diagnostics_ToolHelp`, `Win32_System_Threading`, `Win32_NetworkManagement_IpHelper`,
`Win32_Networking_WinSock`). A shared `ProcessMap` owns an `Arc<RwLock<Published>>`; `Published`
contains active/spare pid→identity hash maps, publication time and publisher cancellation token. The
process collector atomically swaps the complete table and the listener collector reads only live,
fresh identities for owner resolution — one enumeration serves both. Windows startup composition stays under `platform/windows/`: it overlays the canonical
cfg-free registry with process/listener factories and injects the result through
`AppState::open_with_registry`. It does not edit catalog descriptors or add target cfgs under
`collectors/**`.

## Related Code Files

**Create** — `src-tauri/src/collectors/{process/,listeners/}`, `process_map.rs`, `stateful.rs`;
`src-tauri/src/platform/windows/{process.rs,sockets.rs,socket_rows.rs,composition.rs}` and
`src-tauri/src/platform/{mod.rs,traits.rs}`; `src-tauri/tests/{collector_process.rs,
collector_listeners.rs,collector_performance.rs,relationships_process_port.rs}` plus their harness.
**Modify** — Windows startup composition under `src-tauri/src/platform/windows/` (activate and inject
both factories without changing the canonical registry); `src-tauri/Cargo.toml`; collector
normalization/catalog wiring; domain enums, ids and importance rules; error DTOs; atomic collector
commit and existing entity/relationship repository seams; `ARCHITECTURE.md`, `COLLECTORS.md` and
`docs/DECISIONS.md`.
**Delete** — none.

## Implementation Steps

1. `platform/traits.rs`: `ProcessProvider { fn enumerate(&self) -> Result<Vec<ProcessRow>> }`,
   `SocketProvider { fn listeners(&self) -> Result<Vec<SocketRow>> }`.
2. Implement `platform/windows/process.rs`: ToolHelp enumeration; validate creation time for every
   accessible pid on every snapshot. Caching by pid alone misses reuse between polls. Path enrichment
   may be reused only after the `(pid, creation_time)` identity matches; close every handle in a guard
   type so an early return cannot leak (`struct Handle(HANDLE)` with `Drop`).
3. Implement `platform/windows/sockets.rs`: call `GetExtendedTcpTable` with a null buffer to size,
   allocate, call again; handle `ERROR_INSUFFICIENT_BUFFER` retry loop with a 3-attempt cap; parse the
   `MIB_TCPTABLE_OWNER_PID` rows (ports are big-endian — `u16::from_be`), and the same for UDP/IPv6.
4. `collectors/process.rs`: diff, churn collapse, browser-subtree damping, `ProcessMap` publish.
5. `collectors/listeners.rs`: diff, owner resolution, relationship emission.
6. Verify each Win32 signature against `cargo doc --open -p windows` before writing the call; if a
   name differs from this plan, use the documented one and note the correction in the phase file
   (do not invent a fallback).
7. Tests with scripted providers: start/stop detection, pid-reuse does not forge events, path-denied
   rows degrade rather than error, churn collapse, listener open/close, relationship rows present in
   both directions, 30 s idle emits nothing.
8. Manual: run `bun run tauri dev`, start Notepad and a `python -m http.server 4000`, confirm both
   appear in the timeline within ~2 s with the port linked to the process.
9. Commits: (a) `feat: process provider and win32 enumeration`, (b) `feat: process diff collector`,
   (c) `feat: socket provider and listener collector`, (d) `feat: process-port relationships`,
   (e) `test: process and listener suites`.

## Todo (commit boundaries)

- [x] Provider traits + Win32 process enumeration with handle guards
- [x] Process diff collector with pid-reuse safety and path degradation
- [x] Churn collapse + browser-subtree damping
- [x] Listener enumeration (TCP/UDP, v4/v6) and diff collector
- [x] Relationship rows both directions
- [x] Test suites + manual smoke

## Success Criteria

- [x] Scripted test: pid 1234 exits and a new pid 1234 appears with a different creation time → exactly
      one stop and one start, correctly ordered
- [x] Scripted test: 20 of 40 processes deny their path → 40 events, 20 with `path: null` and
      `details.path_unavailable = true`, zero errors logged
- [x] Scripted test: 8 start/stop pairs in 40 s for one image → 16 timely raw events and, when the
      episode's 60 s window closes, exactly one `process/churn` summary with `count: 8`
- [x] Scripted test: TCP 4000 opens owned by `api.exe` → `port/opened` + 2 relationship rows; the
      timeline row reads "Port 4000 opened by: api.exe"
- [x] 30 s idle run against the real OS emits 0 events for both collectors (allowing for genuine OS churn: assert < 5)
- [ ] `cargo test` p95 cycle assertion: process ≤ 15 ms, listeners ≤ 15 ms (measured over 50 cycles)

## Verification Gate

### Current-candidate recheck — 2026-09-29

The earlier 11.8723 ms process / 0.3535 ms listener p95 run remains historical evidence, not a
current-candidate pass after the process collector changed. The current all-features 50-cycle test
measured process **103.9315 ms** and listener **3.3945 ms** while the host was saturated. A second
run measured process **64.2719 ms** and listener **2.9661 ms**; host CPU changed from 100% before the
run to 27.6% afterward, then returned to 92.7–100% without a compiler. Both process runs fail the
15 ms assertion, and neither isolates code cost from host contention. A later focused rerun after
host CPU samples of **20% → 11% → 6%** still measured process **56.1406/15 ms p95 — fail** and
listener **1.2743/15 ms p95 — pass** over 50 cycles. This strengthens the case for a process-path
cost, though those CPU samples do not prove every test cycle was uncontended. The acceptance item
stays open; do not change the budget.

A later same-source 50-cycle experiment found no repeatable benefit from
bounded 1/2/4-worker per-PID enrichment; all variants missed 15 ms and the
temporary parallel code and timing probes were removed. On the restored code,
the focused debug gate measured **18.0200/15 ms p95 — fail**. The release-profile
binary measured **23.0354/15 ms — fail** immediately after LTO compilation,
then the *same compiled binary* measured **13.8449/15 ms — pass** when the
pre-run host CPU sample was 5% with 362 processes, and **38.2239/15 ms — fail**
when the pre-run CPU sample was 83% with 363 processes. Pre/post CPU samples
are workload indicators, not per-cycle utilization. A single calm pass shows
the shipped profile can meet the target on this machine, but it does not close
the acceptance gate while repeated runs vary this much under ordinary host
load. Caching executable paths was rejected because a later access-denied
query must still downgrade process/port evidence confidence.

The 2026-10-14 current-source recheck added a conservative quiet-cycle fast path
and identity-text reuse in the session-local publication map; the focused unit
suites pass (21 process tests and 11 process-map tests). The release 50-cycle
gate still measured **19.4261 ms process p95 — fail** and **352.5 µs listener p95 —
pass** (`CPU_BEFORE=0`, `CPU_AFTER=37`, 357–360 live processes). A same-machine
provider-only diagnostic at `CPU_BEFORE=4` measured **16.4154 ms p50,
18.8605 ms p95, 21.266 ms max** across 50 enumerations, confirming that the
Win32 enrichment itself can exceed the budget. No budget relaxation, retained
handles, or executable-path cache was introduced; the acceptance item remains
open pending a correctness-preserving provider strategy or a calm, repeatable
pass.

### Bounded path-buffer experiment — 2026-10-14

The provider now starts its reusable UTF-16 path workspace at 4,096 units and
grows/retries only when `QueryFullProcessImageNameW` returns
`ERROR_INSUFFICIENT_BUFFER`, capped at the documented 32,767-unit limit. It
preserves the same limited handle access, creation-time read, access-denied
`None` result, path clearing and no-cache/no-retained-handle rules. Release
functional verification passed `collector_process` 9/9 (one real-machine test
ignored) and `relationships_process_port` 10/10. The release performance gate
measured listener **286.6 µs p95 — pass** and process **16.9625 ms p95 — fail**
against 15 ms. This is a measured improvement over 19.4261 ms, not an acceptance
pass; no further provider tweak is promoted without phase attribution, and the
P09 checkbox remains open.

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test collector_process --test collector_listeners --test relationships_process_port
cargo test --features devtools --test collector_performance -- --nocapture
cargo clippy --all-targets --all-features -- -D warnings
cargo test --test architecture
```

## Risk Assessment

Initial repeat evidence (2026-10-26): a fresh release build measured process
92.4285 ms p95, followed by exactly three same-artifact runs at
17.4784 / 57.5130 / 86.1510 ms. All fail the unchanged 15 ms gate; all four
listener measurements pass. P09 stays 11/12. See
[dated evidence](../reports/qa-261026-continuation.md) for CPU/process counts,
artifact hash, raw-log locations and the stage-attribution next step.

A later ignored paired-stage diagnostic on a newly compiled test artifact measured
50 Noop cycles: total p95 **12.6546 ms**, provider **12.6185 ms**, remaining snapshot
**0.0406 ms**. This identifies the dominant stage for this quiet sample, not the
cause of earlier failures or staged-cycle cost. After advisory review, exactly
one unchanged original acceptance invocation passed: **13.7265 ms process /
0.3222 ms listener**, 2 passed / 1 diagnostic ignored, CPU snapshots 42→34%,
343→342 processes. Targeted strict Clippy passed. No further rerun, production
change or threshold relaxation followed. The existing repeatability requirement
above keeps the checkbox open; this is a recorded run pass, not a reliability fix.
Artifact hash and both raw runs are in the linked continuation report.

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| B3: sub-second processes missed | A known short command never appears | Documented in the descriptor copy and `docs/`; **not** solved by ETW (`Win32_ProcessStartTrace` measured Access-denied non-elevated). Do not add an elevation prompt |
| `GetProcessTimes` denied for the same ~46% | Many `identity_confidence: Low` events | Accept and surface: `Low` events still record, and P15 refuses to build a `Confirmed` link off a `Low`-confidence process identity |
| Handle leak during identity validation | Handle count climbs in Task Manager over an hour | `Drop` guard on every handle + a test that opens/closes 10k times and asserts a stable `GetProcessHandleCount` for our own process |
| Big-endian port parsing wrong | Ports appear as 4000 → 43855 | Unit test with a fixed `MIB_TCPROW_OWNER_PID` byte pattern asserting 4000 |
| IPv6 table sizing differs | `GetExtendedTcpTable` returns error 122 repeatedly | Retry-with-grown-buffer loop already handles it; after 3 attempts the cycle is skipped and logged once, collector stays `Running` |
| Chrome/Teams flood the timeline | Timeline unusable during normal browsing | Damping rules above; the P06 calmness test (≥80% informational) runs against a captured 10-minute real stream, so regressions are caught mechanically |

## Security Considerations

- `PROCESS_QUERY_LIMITED_INFORMATION` only — never `PROCESS_VM_READ`, never command lines. Command
  lines are the highest-risk secret carrier (tokens in argv) and are **out of scope for the MVP**;
  the descriptor says so.
- Process paths pass through the P03 sanitizer (a path can contain a username or a token-named folder).
- No handle is retained across cycles.

## Runtime integration clarification (2026-09-13)

- The Phase08 atomic snapshot protocol owns persistence. Stateful diffs stage their next state with
  `AckedState`, submit observations and relationship evidence together, and promote only on the matching
  acknowledgement. A recreated instance reads its durable baseline before emitting a new candidate.
- Socket snapshots contain owner sets per endpoint. An unchanged endpoint with a changed owner must
  not keep attributing the socket to its previous process. Missing or low-confidence process identity
  cannot create a confirmed graph relationship; lookup uses the shared process snapshot only.
- Process entity ids retain the existing executable-based domain grammar; `(pid, creation_time)` is
  the instance-diff key, not a replacement for the cross-restart entity id.
- The actual event type is `NormalizedEvent`, and atomic relationship persistence is already in
  `storage/collector_commit.rs`; extend those seams, do not add parallel `SystemEvent`/relationship
  repositories. Every Win32 import stays under `platform/windows/`.
- Signature references: [Toolhelp snapshots](https://learn.microsoft.com/en-us/windows/win32/api/tlhelp32/nf-tlhelp32-createtoolhelp32snapshot),
  [process timing](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes),
  [TCP tables](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedtcptable),
  [UDP tables](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedudptable).
  Verify Rust signatures against the resolved local `windows` source before implementation.

### Stateful collection and noise contract

The original wording "collapse into one event" cannot mean replacing already persisted start/stop
facts while also promising immediate delivery and append-only evidence. The implementation retains
those facts and appends one immutable summary. A 60 s episode is anchored at the first completed
start/stop pair, not a rolling window at every possible instant. It closes before its final count is
published. Summary metadata records the observation window; its timestamp does not pretend to be a
single process start. Add `Churn` as the twentieth event type and update vocabulary/binding tests.
Existing page-local, fixed-minute clustering is not proof of matched-pair churn detection; test episode
boundaries and ensure the summary's supporting raw facts remain accessible.

- A new application session establishes its first process/socket snapshot without inventing starts or
  opens for pre-existing objects. A worker recreation in the same session restores acknowledged state
  and any pending target. The path-denial fixture first establishes an empty baseline, then adds its
  40 processes. Gaps across pause/app downtime are rebaselined or marked uncertain, never backdated.
- Drain large snapshots through bounded atomic chunks. Keep PID-reuse stop/start and a port event's
  graph evidence in one indivisible transition. Persist the intermediate baseline and remaining target
  with each acknowledgement, preserving original observation timestamps. More than 200 changes must
  make progress without truncation or endless oversized retries; burst delivery can take extra cycles.
  A single transition or baseline beyond storage bounds fails visibly without advancing state.
- Persist validated structural process ids and instance keys independently of display text. Never
  recompute an id from a scrubbed path when hydrating a baseline. A narrow persisted-process identity
  type must validate the existing `process:<16 hex>` grammar. A secret-bearing path followed by worker
  recreation must not produce false stop/start events or change the stopped event's entity id.
- The shared process map publishes a complete successful OS snapshot, not intermediate persistence
  chunks. It is session-local, timestamped and tied to the publisher's cancellation/pause state.
  An inactive/stale publisher yields unresolved owners and no edges. An ownership-set change on a
  still-bound endpoint is a modification, not a fabricated socket close/open.
- Polling a process map and a socket table is not an atomic kernel join. Document the remaining
  between-poll PID-reuse blind spot; path/timing confidence cannot remove it.
