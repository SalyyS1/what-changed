---
phase: 08
title: "Collector runtime: trait, supervisor, scheduler, failure isolation, settings"
status: completed
priority: P1
effort: 5h
dependencies: [05]
---

## Overview

The chassis every collector plugs into: the `Collector` trait (§5, §12), a supervisor that runs each
collector independently so one failure cannot stop the others (§6, §33), a cost-aware scheduler, the
normalizer that turns raw observations into `NormalizedEvent`s through the P03 sanitizer, and the §49
settings surface driven from collector self-description. No OS-specific code lands here — the phase
ships with two fake collectors so the runtime is fully tested before any Win32 call exists.

## Requirements

**Functional**
- `Collector` trait: `id() -> &'static str`, `descriptor() -> CollectorDescriptor`,
  `start(&mut self, ctx: CollectorContext) -> Result<()>`, `stop(&mut self)`,
  `snapshot(&mut self) -> Result<Vec<RawObservation>>` (§5).
- Stateful sources additionally implement `snapshot_batch(SnapshotContext)` and return
  `CollectorSnapshot::Atomic { token, observations, relationships, baseline }`; `AckedState<T>`
  promotes staged state only on the matching durable acknowledgement. `commit_succeeded` /
  `commit_failed` are panic-isolated hooks. `CollectorContext` exposes monitor, emit, targeted wake,
  collector-bound baseline reads and cancellation, never a generic database/writer handle.
- `CollectorDescriptor { id, display_name, what_it_watches, why_it_matters, cost_hint: Cheap|Moderate|Expensive, cadence: Poll(Duration)|Notify|NotifyPlusSweep, requires_admin: false }` — the single source for §49 toggle copy ("what it watches, why it matters").
- Supervisor: spawns one OS thread per collector (not async — the Win32 calls are blocking), owns a
  `Vec<CollectorHandle>`, and exposes `list_collector_status()` → `CollectorStatusDto` with its
  serializable descriptor, enabled flag, `Running|Paused|Idle|Failed|Degraded|Disabled|Unavailable`,
  last-run timestamp/duration, failure reason/count and separate event-row/cycle loss counters.
  `Unavailable` means no platform factory, not healthy monitoring with no changes.
- Failure isolation: collector factory/start/snapshot/hooks/stop/drop panics are caught,
  recorded, and the collector is restarted with exponential backoff 5 s → 30 s → 5 min, capped; after
  5 consecutive failures the collector enters `Failed` and reports through §33's degraded-mode
  surface. The app keeps running and the other collectors are untouched. Queue-admission drops and
  rows lost after a failed SQLite batch commit are counted separately; either moves the affected
  collector/writer to a degraded state visible in Diagnostics. A non-corruption commit failure is
  never logged and then cleared as if every row committed, and unknown write failures are not blindly
  retried. A poisoned instance is discarded rather than reused. Production uses unwind semantics;
  a real optimized binary must prove isolation because `cargo test` alone cannot prove the shipping
  panic profile. `stop_all()` uses one global 2 s join deadline, including the watchdog; uncooperative
  provider threads may detach, but their stale emit/wake capabilities are fenced.
- Global pause: `pause_monitoring(minutes)` / `resume_monitoring()` update shared `Monitoring`.
  Lifecycle/instance generation fences reject stale callbacks and not-yet-admitted snapshots;
  instance cancellation is set before stop/drop. Workers stay parked while paused. The watchdog
  auto-resumes at the deadline; the tray displays the absolute resume deadline. An atomic cycle
  already admitted before pause/disable may still commit once and receive its acknowledgement.
- Normalizer: `RawObservation { collector_id, kind, entity, before, after, occurred_at, details }` →
  `NormalizedEvent`; assigns `event_type`/`category`/`importance` (P06 classifier), builds the entity id
  by the P04 grammar, and passes every text field through `Sanitizer::sanitize` before the event is
  handed to the P04 batched writer. Events, relationship endpoints and baselines require
  `Sanitized<T>` wrappers. Atomic events/entities/relationships/baseline commit in one transaction;
  failure rolls back all tables and rejects staged state. Baseline-only cycles create no fake event
  or event notice. A failed post-durability hook never resubmits the admitted batch; replacement
  instances hydrate the durable baseline before producing their next candidate.
- Settings (§49): `get_settings`/`update_settings` persist per-collector `enabled` flags and the
  monitoring profile (`Balanced` default, `Minimal`, `Thorough`) to the `settings` table; toggling a
  collector off cancels/stops its instance after the cooperative in-flight call returns and parks
  its worker; toggling on creates a fresh instance without an app restart. The app-level serialized
  coordinator validates all ids, commits settings transactionally, then applies that exact state.
- Scheduler honours `cost_hint`: `Expensive` collectors never run on the fast tick, are staggered so
  the registered expensive set receives stable distinct offsets in 0–29 s (up to 30 entries), with
  one global expensive-snapshot lease preventing overlap. Their effective sweep interval is at least
  30 s in every profile. `QueryBusy` cooperatively skips new expensive work while a user-visible
  query is active; it does not preempt an already-running provider call or make queries take that lease.

**Non-functional**
- The entire `collectors/**` tree is target-cfg-free and compiles/tests on any host: no `windows`
  crate import or `#[cfg(windows)]` anywhere under it (enforced by `tests/architecture.rs` from P04).
- Idle CPU contribution of the runtime itself (no collectors registered) ≈ 0: threads block on
  `Condvar`/`recv_timeout`, never spin.

## Architecture

`collectors/mod.rs` (trait + descriptor + `RawObservation`), `collectors/runtime.rs` (supervisor,
threads, backoff, pause gate), `collectors/runtime/{worker,policy}.rs` (lifecycle/commit loop and
testable clock/backoff policy), `collectors/schedule.rs` (completion-plus-interval deadlines,
cost staggering, `QueryBusy`), `collectors/normalize.rs` (raw → `NormalizedEvent` → `Sanitized`),
`collectors/snapshot.rs` (atomic candidates, acknowledgements, scoped state reads),
`collectors/registry.rs` (the cfg-free
canonical descriptor catalog plus a factory-only activation overlay). `AppState::open` keeps that
catalog all-unavailable for deterministic cross-platform tests; platform startup composition injects
an activated registry through `AppState::open_with_registry`. Phase 09 owns the first Windows
composition code, outside `collectors/**`; it cannot replace descriptor copy or privilege metadata.
Provider-trait pattern: each future collector takes a `&dyn XProvider` (e.g. `ProcessProvider`) whose
Windows implementation lives in `platform/windows/`; tests inject a scripted fake. That is what makes
collectors 09–13 unit-testable on any machine, and it is established here with
`collectors/testing.rs` (`ScriptedCollector`, `FlakyCollector`, `PanickingCollector` behind
`#[cfg(test)]`).

Legacy flow: `snapshot` → raw observations → normalizer → `Sanitized<NormalizedEvent>` → bounded
writer queue → coalesced transaction → `EventBatchNotice`. Stateful flow: `snapshot_batch` → staged
candidate/token → sanitizer → atomic writer admission → release lifecycle fence → wait for commit
receipt → ack/reject staged state. `events_emitted` counts legacy queue-admitted rows plus atomic
confirmed-committed rows; separate failure counters mean it is not a durable history row total.

## Related Code Files

**Create** — `src-tauri/src/collectors/{mod.rs,runtime.rs,schedule.rs,normalize.rs,registry.rs,snapshot.rs,testing.rs}`;
`src-tauri/src/collectors/runtime/{worker.rs,policy.rs}`;
`src-tauri/src/domain/collector.rs` (descriptor/status types, `ts-rs`-exported);
`src-tauri/src/commands/collectors.rs` (`list_collector_status`, `set_collector_enabled`,
`pause_monitoring`, `resume_monitoring`, `get_monitor_state`);
`src-tauri/tests/collector_runtime.rs`, `src-tauri/tests/collector_isolation.rs`,
`src-tauri/tests/normalize.rs`, `collector_targeted_wake.rs`, `collector_atomic_runtime.rs`,
`collector_commit.rs`, `collector_registry_composition.rs`; `src/features/settings/CollectorList.svelte`,
`src/lib/stores/collectors.svelte.ts`, `src/lib/stores/diagnostics.svelte.ts` and focused UI/store tests.
**Modify** — `src-tauri/src/lib.rs` (build supervisor in `setup`, store in managed state, stop on
exit), `src-tauri/src/commands/mod.rs` + `dto/bindings.rs` (`ts-rs` export + command parity),
`src-tauri/src/domain/settings.rs` (collector flags, profile), `src/features/settings/SettingsScreen.svelte`,
`src/features/diagnostics/DiagnosticsScreen.svelte`, the storage writer/atomic transaction modules,
`ARCHITECTURE.md`, `ROADMAP.md`, `COLLECTORS.md`, `docs/PERFORMANCE.md` (existing canonical docs).
**Delete** — none.

## Implementation Steps

1. Define collector/descriptor/raw contracts and the narrow context in `collectors/mod.rs`; implement
   tokenized atomic snapshot/acknowledgement contracts in `collectors/snapshot.rs`.
2. `runtime.rs`: construct the supervisor with registry, database, monitoring, query gate and settings;
   `spawn_all()` owns per-collector workers and watchdog. Fence lifecycle changes, isolate every
   collector callback, recreate poisoned instances with backoff, and share one 2 s shutdown deadline.
3. `schedule.rs`: deadline after completed work plus effective interval, stable collision-free
   expensive offsets, exclusive expensive lease and cooperative `QueryBusy` RAII guard.
4. `normalize.rs`: entity-id construction per P04 grammar, `before`/`after` JSON assembly, category
   mapping, then `Sanitizer::sanitize`. Every field that can contain user data goes through it —
   including `entity_name` and `details`.
5. Wire commands, then the settings UI: one row per collector with name, "what it watches", "why it
   matters", cost badge, live state, last-run time, and a toggle.
6. Tests: `collector_runtime.rs` (start/stop/pause/resume/enable/disable, cadence honoured within
   ±20%), `collector_isolation.rs` (a panicking collector is restarted with backoff, a permanently
   failing one reaches `Failed` after 5 tries, and a healthy neighbour keeps emitting throughout),
   `normalize.rs` (entity grammar + a secret-bearing observation is redacted before reaching the writer).
7. Commits: (a) `feat: collector trait and descriptor`, (b) `feat: collector supervisor with failure isolation`,
   (c) `feat: cost-aware scheduler and pause gate`, (d) `feat: observation normalizer`,
   (e) `feat: collector settings surface`, (f) `test: runtime, isolation and normalizer suites`.

## Todo (commit boundaries)

- [x] Trait, descriptor, context, raw observation and atomic snapshot/acknowledgement seam
- [x] Supervisor with panic isolation and backoff — optimized shipping-profile probe passed
- [x] Scheduler with cost staggering and query gate
- [x] Normalizer and atomic event/entity/baseline paths routed through the sanitizer
- [x] Settings and live Diagnostics surfaces from descriptors
- [x] Runtime, isolation and normalizer suites green; atomic/wake/storage regressions added

## Success Criteria

- [x] Panic/failure fixtures assert exact 5 s → 30 s → 5 min backoff policy, cap at 5 attempts and
      a healthy neighbour continuing; an optimized non-test binary also survives collector panics
- [x] `set_collector_enabled(id, false)` fences stale emissions and stops the instance after its
      cooperative call; `true` recreates it in the parked worker, asserted without app restart
- [x] Pause blocks new admissions/stale callbacks; watchdog auto-resumes at the deadline
- [x] A secret-bearing `RawObservation` is stored redacted (queries the DB in the test)
- [x] `list_collector_status()` returns one row per registered collector with `requires_admin: false`
      for all of them
- [x] `tests/architecture.rs` still passes (no Win32 import outside `platform/`)
- [x] Idle CPU with two fake collectors at 1 s cadence < 0.3% over a 60 s measurement

## Verification Gate

```bash
cargo test --manifest-path src-tauri/Cargo.toml --all-features -j 2 --test collector_runtime --test collector_isolation --test normalize --test architecture --test collector_atomic_runtime --test collector_commit --test collector_targeted_wake --test collector_registry_composition
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features -j 2 -- -D warnings
bun run check:bindings
bun run check
bun run test
bun run build
```

Use the shared task-specific `CARGO_TARGET_DIR` for this machine's disk limit. Final closure includes
the optimized panic-isolation binary, release performance gate, 60 s runtime idle measurement and
verification of the final database lifecycle correction; each passed independently below.
Retained probes: `src-tauri/examples/collector_panic_probe.rs` and
`src-tauri/examples/collector_idle_probe.rs`, both devtools-gated. Build/run the panic example with
`cargo run --manifest-path src-tauri/Cargo.toml --release --features devtools --example collector_panic_probe`.
For idle, build the idle example with `--features devtools`, launch its binary, then externally sample
the process after `IDLE_PROBE_READY`; it remains alive long enough for the full measurement.

## Implementation Adjustments — 2026-09-13

| Original wording | Evidence-backed contract / reason / impact |
|---|---|
| `SystemEvent`, four health states, direct serialized runtime descriptor | Existing P04 boundary is `NormalizedEvent`; `CollectorDescriptorDto` encodes durations explicitly. Seven states distinguish unavailable and degraded from healthy idle. Generated bindings and Settings/Diagnostics consume the same rows. |
| Stop/restart the OS thread on toggle | Stop/discard the instance and park the worker. Generation fencing rejects callbacks from old instances; avoids unnecessary thread churn. A non-cooperative provider call cannot be forcibly stopped safely. |
| Every pause implies zero subsequent durable events | New admissions stop immediately; an atomic cycle admitted before the fence still commits once. Cancelling after durable admission would desynchronize baseline and staged state. Tests cover both sides of admission. |
| Hash stagger; no two sweeps in an entire 30 s window | Sorted catalog receives distinct 0–29 s offsets plus one expensive lease. Hashes alone collide, and whole-window exclusion unnecessarily serializes detection by 30 s per source. Keeps no-overlap, minimum 30 s per-source cadence and responsive query admission. |
| Downgrade an overrunning Poll collector to NotifyPlusSweep after three overruns | Schedule next run after completion plus interval, preventing backlog. Do not invent a notification source that a Poll-only provider does not have; expose measured duration and let the owning collector phase choose its real fallback. |
| Supervisor command channel persists settings after runtime mutation | App-level configuration mutex validates all ids, performs the settings transaction first, then applies exact committed state to an immutable registry. Concurrent settings test asserts DB/runtime agreement; invalid writes cannot partly change runtime. |
| Tray shows remaining duration | Tray shows absolute resume deadline, covered by the tray label test. Watchdog remains deadline-driven; no per-second label timer is needed. |
| Test-only panic isolation sufficient | Review found release `panic = "abort"` incompatible with the isolation promise. Changed to unwind; optimized non-test probe passed with exit 0, panic contained and healthy neighbour continuing. |
| Asynchronous event-only diff loop sufficient for future collectors | Stateful collectors require durable event/entity/edge/baseline atomicity before advancing memory. Added opaque commit tokens, `AckedState`, bounded atomic batches and collector-scoped baseline reads; no OS implementation moved into this phase. |
| Closing/recovering a Db handle can race or re-enter its writer | `Db::close` is terminal for that handle; recovery cannot reopen it. A competing recovery returns a busy error. Writer-callback close publishes closed state immediately; physical drain is deferred to external close/drop. Lifecycle fences prevent a stale recovery publishing a new writer after close, and collector submission rejects writer-thread reentrancy before admission. |

## Evidence Snapshot — 2026-09-13

**Completed: 13/13 acceptance/task checkboxes.** Final review **9.6/10**; all four final findings
resolved, no blocking findings. Verification distinguishes the full baseline from focused post-fix
checks; their counts are not added together as distinct tests.

| Gate | Final evidence |
|---|---|
| Full Rust baseline | 315 passed; one release-only benchmark filtered, then executed separately below |
| Changed DB/writer/collector-commit paths | 9 + 4 + 13 = 26 passed |
| Existing recovery/migration/repository regressions | 8 + 10 + 17 = 35 passed |
| Static/type gates | rustfmt, strict all-target/all-feature clippy, bindings generation/check passed |
| Frontend | 191 tests / 22 files passed; check 0 errors / 0 warnings; build 770 modules |
| Shipping-profile panic probe | Optimized non-test binary PASS, exit 0; panicking worker contained, healthy neighbour continued; compile 9 min 12 s |
| Release 10k query benchmark | 1/1 passed, 11 warmed samples; page p50/p95 1.154/1.473 ms; summary 1.044/1.764 ms; search 7.184/8.094 ms |
| Runtime idle probe | Real Db/supervisor, two fake 1 s collectors; 62.324830 s external sample; CPU delta 0 s at Windows counter resolution, 0.000000% of one core and normalized over 12 logical CPUs, below 0.3%; peak working set 8,736,768 B (8.33 MiB); exit 0 |

Idle probe start counters were 2/2; terminal counters 69/69 show both collectors kept running through
the sampled interval and its exit margin. Reported zero CPU is measurement resolution, not a proof of
literally zero instructions. The probe measures runtime/Db overhead, not WebView2 or the full desktop
app; it does not close Phase 05's total/private working-set measurement.

Traceability: `collector_runtime.rs` covers lifecycle, averaged cadence ±20%, stagger reapplication,
query gating, seven-state registry and loss counters; `collector_isolation.rs` records production
backoff values while accelerating waits, caps panic/failure workers at five and checks a healthy
neighbour continues. It does not assert every neighbour event or wait ten real minutes.
`collector_atomic_runtime.rs` / `collector_commit.rs` prove acknowledgement order, rollback, token
fencing, baseline replay, stale evidence ordering and row-versus-cycle telemetry. `normalize.rs`
reads the real database to verify redaction; `collector_targeted_wake.rs` covers coalescing, stale
handles and expensive serialization. UI/store suites cover live descriptors and truthful diagnostics.

Phase 08 gates and canonical documentation sync are closed. Phase 09 subsequently completed on
2026-09-19, so Phase 10 is the next implementation; the full plan remains in progress. Phase 05's
NSIS and app-memory carryover also closed on 2026-09-19 with the artifact and both D-15 memory
figures recorded in `docs/PERFORMANCE.md`.

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Release abort bypasses isolation | Optimized binary exits on collector panic | Closed: unwind profile and optimized non-test probe passed. `AssertUnwindSafe` handles the mutable boundary; poisoned instance recreated, not resumed. |
| Thread-per-collector costs too much memory | Measured per-process contribution/idle CPU | Runtime gate closed: 0.000000% measured CPU, 8.33 MiB peak working set with two fake sources. Workers use 256 KB stacks, watchdog 128 KB. Full-app/platform-source cost remains a later-phase measurement. |
| Blocking `snapshot()` overruns its cadence | `last_duration_ms > cadence` | Completion-plus-interval avoids backlog. Preserve actual provider mechanism; no fabricated Poll-to-Notify transition. Future platform calls must check cancellation at bounded page boundaries. |
| Stale callbacks survive pause/disable/shutdown | Old handle admits work after transition | Closed by lifecycle/instance generation fences and cancel-before-stop tests. Already-admitted atomic work may commit once; callbacks cannot re-admit it. |
| Settings write races the supervisor | Persisted settings differ from runtime | Closed by serialized transactional coordinator and concurrent DB/runtime equality test. |
| Writer lifecycle correction invalidates final acceptance baseline | Reentrancy/close/recovery regression fails | Closed: 26 changed-path tests and 35 existing storage regressions passed after correction; final review found no blocking issues. |

## Security Considerations

- The normalizer is the only path into storage, and it takes `RawObservation` and returns
  `Sanitized<NormalizedEvent>` — a collector cannot bypass redaction (no `pub fn` accepts an unsanitized
  event downstream).
- `CollectorContext` gives collectors no filesystem, network, or shell handle; capability is passed as
  narrow provider traits.
- **§36 non-elevated privilege model, enforced here.** No collector requests elevation, and
  `requires_admin` is hardcoded `false` for the MVP set; a future collector that needs it must surface
  the fact in its descriptor before the toggle can be shown. This is the phase §36's traceability row
  points at, because the runtime is where the constraint is mechanical rather than aspirational: the
  supervisor never spawns an elevated helper, `CollectorContext` carries no token, and the four probed
  capabilities that would need elevation (`Win32_ProcessStartTrace`, the `Security` event-log channel)
  or that proved unreliable (`__InstanceCreationEvent`) are absent from the registry by construction,
  each with a measured non-elevated substitute named in `COLLECTORS.md`.
