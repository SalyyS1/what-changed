---
phase: 12
title: "Windows Update collector (WUA COM) and driver collector (notify + rare sweep)"
status: completed
priority: P1
effort: 7h
dependencies: [09, 11]
---

## Overview

The two collectors that most often answer "what changed?" for a machine that broke overnight: Windows
Update history (§18) and driver installs/updates (§19). Both are measured-PASS non-elevated, but they
have opposite cost profiles — WUA `QueryHistory` is 38 ms and returned 133 entries; a full driver
enumeration is 2128 ms warm, which is far too slow to poll. So updates are polled on a slow timer plus
event-log corroboration, and drivers are notification-driven with a rare sweep.

## Requirements

**Functional**
- Update collector: COM `Microsoft.Update.Session` → `IUpdateSearcher` →
  `GetTotalHistoryCount()` + `QueryHistory(0, n)` → `IUpdateHistoryEntry` fields `Title`, `Date`,
  `ResultCode`, `Operation`, `ClientApplicationID`, `SupportUrl`, `UpdateIdentity`. Poll every 15 min
  and on resume-from-sleep. Diff on `(UpdateIdentity.UpdateID, Date)`. Emit `update/installed`,
  `update/failed` (`ResultCode` 4 = failed, 5 = aborted), `update/uninstalled`
  (`Operation` = `uoUninstallation`).
- **`Win32_QuickFixEngineering` is forbidden** — measured 4 rows vs WUA's 133; a comment in the code
  states why so nobody "simplifies" it back.
- COM lifecycle: the update collector owns a dedicated thread that calls
  `CoInitializeEx(COINIT_APARTMENTTHREADED)` once at start and `CoUninitialize` at stop; the COM
  pointers never cross the thread boundary — only plain `UpdateHistoryRow` structs do. All COM objects
  held in `windows` crate RAII wrappers.
- KB-number extraction from `Title` via `\(KB\d{6,7}\)` for the entity id
  (`update:KB5034123`), falling back to a hash of the `UpdateID` when there is no KB number
  (driver updates delivered through WU often have none).
- §18 phrasing: `Windows Update installed KB5034123` / `Windows Update failed: KB5034123 (error 0x80070002)`.
  `update/failed` is `importance: important` and is one of the §53 notification-worthy categories.
- Event-log corroboration: `Microsoft-Windows-WindowsUpdateClient/Operational` is subscribed in P13;
  when both sources describe the same KB within ±10 min, the WUA event is authoritative and the log
  entry only contributes `details.evidence`.
- Driver collector: `SetupDiGetClassDevs(NULL, NULL, NULL, DIGCF_ALLCLASSES | DIGCF_PRESENT)` +
  `SetupDiEnumDeviceInfo` + `SetupDiGetDeviceRegistryPropertyW(SPDRP_DEVICEDESC, SPDRP_CLASS,
  SPDRP_MFG, SPDRP_DRIVER)` and `SetupDiGetDeviceInstanceIdW`; driver version + date from the
  `SPDRP_DRIVER` key under `HKLM\SYSTEM\CurrentControlSet\Control\Class\{...}\<n>` (`DriverVersion`,
  `DriverDate`, `ProviderName`). Entity id = device instance id (P04 grammar).
- Cadence: `CM_Register_Notification(CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE, ...)` for
  arrival/removal wake, **plus** a sweep every 6 hours and one on resume-from-sleep. If
  `CM_Register_Notification` cannot be used (research §5 flags the callback ABI as unverified), fall
  back to `RegNotifyChangeKeyValue` on the Class key tree as the wake source, and if that also fails,
  pure 6-hour sweep. Three tiers, each independently sufficient; the chosen tier is recorded in
  `CollectorStatus.details` so the diagnostics screen shows it.
- §19 human naming: `NVIDIA graphics driver updated 566.36 → 566.45` — device description + class map
  to a friendly noun (`Display` → "graphics driver", `Net` → "network driver", `AudioEndpoint`/`MEDIA`
  → "audio driver", `USB` → "USB driver", else "<Class> driver"), with the manufacturer's first token as
  the brand. Mapping table is data, not code branches, and is unit-tested against the §73 example.
- Driver events preserve P04's closed 21-value `EventType` storage vocabulary: install = `installed`,
  removal = `uninstalled`, forward version change = `updated`, rollback = `version_changed`. The exact
  driver subtype is always carried by the bounded `details.change_kind` value (`installed`, `removed`,
  `updated`, `rolled_back`). Relevant forward updates and rollbacks are `importance: important` (§54) —
  this is the class of change that silently breaks audio and video.

**Non-functional**
- Driver sweep never runs while a query is in flight (`QueryBusy` gate from P08) and never on the fast
  tick; measured 2128 ms is budgeted as ≤ 3500 ms p95 with a hard 10 s timeout after which the sweep is
  abandoned and retried at the next interval.
- Update poll ≤ 200 ms p95 (baseline 38 ms).
- Neither collector blocks app startup: both run their first (baseline) cycle after the window is shown.

## Architecture

`collectors/updates.rs` + `collectors/drivers.rs` (diff + naming, provider-injected);
`platform/windows/wua.rs` (COM thread, `IUpdateSession`/`IUpdateSearcher`/`IUpdateHistoryEntry` via
  `windows` 0.61.3 features `Win32_System_Com` + `Win32_System_UpdateAgent`; the interfaces are provided
  by the crate directly, so no generated binding module is carried in this repository);
`platform/windows/devices.rs` (SetupAPI via `Win32_Devices_DeviceAndDriverInstallation`,
`Win32_Devices_Properties`); `domain/driver_naming.rs` (the class→noun table).

Data flow is unchanged: provider → collector diff → normalizer → sanitizer → writer. Both collectors
persist their last-seen set in a `collector_baselines` row (P04) so a restart does not re-emit history —
critical for updates, where `QueryHistory` returns 133 entries every single time.

## Related Code Files

**Create** — `src-tauri/src/collectors/{updates.rs,drivers.rs}`;
`src-tauri/src/platform/windows/{wua.rs,devices.rs}`; `src-tauri/src/domain/driver_naming.rs`;
`src-tauri/tests/collector_updates.rs`, `src-tauri/tests/collector_drivers.rs`,
`src-tauri/tests/driver_naming.rs`, `src-tauri/tests/collector_baseline_restart.rs`.
**Modify** — `src-tauri/src/collectors/registry.rs`, `src-tauri/src/platform/traits.rs`,
`src-tauri/Cargo.toml` (windows features above), `src-tauri/src/storage/baseline_repository.rs`,
`src-tauri/src/domain/event.rs`, `docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. **Verify the WUA binding route first** (`cargo doc -p windows` / search the crate for
   `IUpdateSearcher`). Do not write COM calls before the route is confirmed; if bindgen is required,
   commit the generated module under `src-tauri/src/platform/windows/wua_bindings.rs` with a header
   comment naming the generator and the source metadata.
2. `platform/windows/wua.rs`: dedicated thread, `CoInitializeEx`, session creation, `QueryHistory`
   paging in chunks of 50, mapping to `UpdateHistoryRow`, `CoUninitialize` on stop. `ResultCode` and
   `Operation` mapped through explicit `match` on the documented integer values (2 = succeeded,
   3 = succeeded-with-errors, 4 = failed, 5 = aborted; 1 = installation, 2 = uninstallation).
3. `collectors/updates.rs`: load the persisted baseline, diff, emit, persist state in the same transaction as
   the events (P04 writer supports a state piggyback — add it if absent).
4. `platform/windows/devices.rs`: enumeration with a `Drop` guard on the `HDEVINFO`
   (`SetupDiDestroyDeviceInfoList`), per-device property reads with the size-then-fill pattern, driver
   key read for version/date/provider.
5. `domain/driver_naming.rs`: the class→noun table + brand extraction + the exact §73 assertion.
6. `collectors/drivers.rs`: tiered wake registration (`CM_Register_Notification` → registry notify →
   timer only), sweep with the 10 s timeout, diff, rollback detection via semver-ish numeric compare
   with a lexical fallback for non-numeric versions.
7. Resume-from-sleep hook: compare wall time with `QueryUnbiasedInterruptTime` on an owned watcher
   thread. A >60 s sleep-only gap triggers both collectors through lifecycle-fenced targeted wakes;
   ordinary scheduling delay does not look like sleep time.
8. Tests with scripted providers: WUA history diffing (including "same 133 entries again → zero events"),
   failed-update phrasing with the hex error, uninstall operation, KB-less update ids, driver
   install/update/rollback/removal, naming table, restart-does-not-replay.
9. Manual: run `bun run tauri dev`; confirm the historical update list is *not* dumped into the timeline
   on first run (baseline) and that `driver/…` sweeps appear in the diagnostics screen with the chosen
   wake tier.
10. Commits: (a) `feat: wua binding route`, (b) `feat: update history collector`,
    (c) `feat: setupapi device provider`, (d) `feat: driver collector with tiered wake`,
    (e) `feat: driver friendly naming`, (f) `feat: resume from sleep sweeps`,
    (g) `test: update and driver suites`.

## Todo (commit boundaries)

- [x] WUA binding route verified and committed
- [x] Update history collector + persisted state
- [x] SetupAPI provider with list guard
- [x] Driver collector with three-tier wake and sweep timeout
- [x] Friendly driver naming table
- [x] Resume-from-sleep trigger
- [x] Test suites

## Success Criteria

- [x] Scripted WUA: 133 entries on first run → baseline, 0 events; 134th entry appears → exactly 1
      `update/installed` with summary `Windows Update installed KB5034123`
- [x] Scripted WUA: `ResultCode = 4` → `Windows Update failed: KB5034123 (error 0x80070002)`,
      importance `important`
- [x] Restart test: process restarts, `QueryHistory` returns the same 133 → 0 events
- [x] `driver_naming.rs` test asserts the §73 string `NVIDIA graphics driver updated 566.36 → 566.45`
- [x] Scripted drivers: version decrease → a `version_changed` event with
      `details.change_kind = "rolled_back"`, not a forward-update claim. This preserves the closed P04
      `EventType` vocabulary while retaining the exact driver subtype.
- [x] Real-OS sweep: driver enumeration completes ≤ 3500 ms p95 over 5 sweeps; a forced 10 s stall is
      abandoned cleanly and the collector stays `Running`
- [x] `list_collector_status()` reports which wake tier the driver collector obtained
- [x] `rg -n "QuickFixEngineering" src-tauri/src` returns only the explanatory comment

## Completion Evidence — 2026-09-20

- Direct `windows` 0.61.3 `Win32::System::UpdateAgent` bindings are used on one owned STA COM thread.
- Scripted gates: updates 8/8, drivers 6/6, naming 2/2, restart 1/1, resume/status 1/1;
  WUA lifecycle/recreation unit tests 3/3.
- Real WUA history: 18 rows, p95 2.7221 ms over 20 warm polls (budget ≤200 ms).
- Real SetupAPI: 217 present devices, five-sweep p95 376.8582 ms (budget ≤3500 ms).
- Forced driver stall abandoned after 10.0045373 s; no overlap occurred and a later sweep recovered.
- Timeline queries during a real driver sweep measured p95 1.2294 ms (budget ≤20 ms).
- Core Rust unit suite: 349 passed, 0 failed and one SetupAPI performance gate ignored there, with
  every integration suite green; seven ignored real-OS/long-running gates were exercised explicitly
  for Phase 12 or in their owning earlier
  phase. Strict Clippy, formatting, architecture, binding parity, frontend typecheck and 201 frontend
  tests passed.
- Diagnostics exposes the typed wake tier selected by the driver provider. The only source reference to
  `QuickFixEngineering` is the explanatory prohibition in `wua.rs`.

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test collector_updates --test collector_drivers --test driver_naming \
           --test collector_baseline_restart
cargo clippy --all-targets -- -D warnings
cargo test --test architecture
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| WUA interfaces absent from the resolved `windows` crate | The feature/module search finds no `IUpdateSearcher` | Resolved in `windows` 0.61.3 through the built-in `Win32_System_UpdateAgent` feature; no generated bindings or PowerShell fallback are used |
| COM apartment misuse crashes the app | `RPC_E_*` HRESULT or a hard crash on stop | COM is confined to one owned thread with no pointer escape; the isolation test from P08 covers the panic path, and a failed `CoInitializeEx` disables the collector rather than aborting the process |
| `CM_Register_Notification` callback ABI wrong (research §5 unverified) | Registration returns non-zero, or a callback crash in the fault test | Tier down to registry notify, then to timer. The tier is chosen at start and never mid-flight, so there is no partial-registration state |
| 2.1 s driver sweep stalls the UI | A visible hitch when the sweep runs | Sweep runs on its own thread behind `QueryBusy`; a UI-latency test measures `get_events` p95 while a sweep is deliberately running and asserts ≤ 20 ms |
| Update history replayed after a DB reset | Timeline floods with old updates | `collector_baselines` is reset with the DB, so a reset *does* re-baseline (0 events) rather than re-emit — asserted by the restart test with a fresh DB |
| Driver version strings non-numeric (`31.0.15.5222` vs `1.2.3-hp`) | Rollback misdetected | Numeric-tuple compare when both parse, lexical compare otherwise, and `Unknown` direction → emit `driver/updated` without a rollback claim |

## Security Considerations

- Both collectors are read-only: WUA is used for history only (never `IUpdateInstaller`), SetupAPI for
  property reads only (never `SetupDiCallClassInstaller`).
- Update titles and driver provider strings pass through the P03 sanitizer. WUA `SupportUrl` is read as
  part of the provider contract but deliberately omitted from persisted event metadata: the P03 key
  policy treats URL-shaped fields as sensitive, and no Phase 12 workflow needs the link. The UI has no
  `shell:open` capability (P05 allowlist).
- The `Security` event-log channel is not touched here; corroboration uses only channels measured
  readable non-elevated.
