---
date: 2026-09-13
status: completed
title: Collector runtime — ordering evidence before completion
---

# Collector runtime — ordering evidence before completion

## Context

Local engineering record for [Phase 08](../../plans/260829-0437-what-changed-desktop-mvp/phase-08-collector-runtime.md).
Scope: runtime isolation, atomic state, database lifecycle and live Diagnostics. Phase 08 is now
completed, 13/13; the full 25-phase plan continues with Phase 09. This is not a public release record.
Phase 05's NSIS/full-app memory carryover remains pending. Durable design rationale belongs to
[Architecture](../../ARCHITECTURE.md#collector-runtime).

## What happened

Repeated green stress runs did not prove the relevant ordering. A later whole-suite run exposed
interleavings the focused loops had missed. The useful evidence came from explicit two-way barriers
and a failing-before/fixed-after comparison, not another unexplained run count.

- Shutdown could reach collector cleanup before the supervisor's cancellation loop reached that
  instance. The [worker](../../src-tauri/src/collectors/runtime/worker.rs) now owns cancellation of the
  exact instance before stop/drop; retained callback handles are part of the isolation boundary.
- Expensive-lease contention incorrectly moved a due deadline forward under a frozen fake clock.
  Keeping the original deadline separated wall-clock retry from scheduled collector time.
- `WorkerObservedClock` announced its first read before capturing time. The test could advance time
  between those operations and unintentionally defer the first sweep again. Capturing first, then
  notifying through a two-way barrier, made the regression deterministic in
  [targeted-wake tests](../../src-tauri/tests/collector_targeted_wake.rs).
- The [atomic protocol](../../src-tauri/src/collectors/snapshot.rs) made candidate-state promotion
  depend on durable acknowledgement. Raw events, entities, relationship evidence and the baseline
  share the sanitizer/transaction boundary. Rollback must not advance state; an acknowledgement-hook
  failure after durability must not resubmit the already-admitted batch.
- Release `panic = "abort"` contradicted collector isolation. The manifest now chooses unwinding;
  the [real executable probe](../../src-tauri/examples/collector_panic_probe.rs) passed separately
  with exit 0 because Cargo's test harness cannot prove that release behavior.
- Database recovery needed exclusive lifecycle admission through writer drain, file work and reopen.
  Close also needed to be terminal. The [database owner](../../src-tauri/src/storage/db.rs) and
  [writer](../../src-tauri/src/storage/writer.rs) reject writer-thread reentrancy; the split ticket
  route rejects it before admitting any write. Close is terminal for that Db handle; competing
  recovery returns busy. Callback close publishes closed state immediately and defers physical drain
  to external close/drop rather than joining its own thread.
- [Diagnostics teardown](../../src/lib/stores/diagnostics.svelte.ts) must be obtainable while the
  initial IPC read is unresolved, so unmount can remove the listener and polling timer promptly.
  The final test-only barrier also needed a narrowly scoped unwrap allowance for strict Clippy.

## Final evidence

The coordinator reported the earlier broad Rust gate as 315 passed, with one exact release benchmark
filtered out; frontend validation as 191 tests across 22 files, zero check errors/warnings and a passing
production build (770 modules). After lifecycle fixes, the focused database/writer/atomic-storage gate
passed 26 tests, and existing recovery/migration/repository checks passed 35. These overlap the broad
baseline and are not summed as unique tests. Formatting, strict Clippy and bindings parity passed.
Final scoped review: 9.6/10, all four findings resolved, no blocking findings.

The optimized panic probe passed: failing worker contained, healthy worker continuing. Release 10k
query gate passed 1/1 with 11 warmed samples: page p50/p95 1.154/1.473 ms, summary 1.044/1.764 ms,
search 7.184/8.094 ms. The debug two-collector idle probe passed over 62.324830 s: CPU delta 0 s at
Windows counter resolution (0.000000%), peak working set 8,736,768 B / 8.33 MiB. Counters progressed
from 2/2 to 69/69 at exit. This measures runtime/Db overhead, not the whole WebView desktop app.

## Reflection

The costly mistake was treating repeated success as evidence of a race being covered. A barrier must
prove that the prerequisite action happened, not merely that the thread reached nearby code. The same
ownership question connected apparently separate failures: who owns this instance, this admitted write,
this database incarnation, or this subscription until teardown actually finishes?

## Decisions

Keep lifecycle authority explicit, make durability acknowledgements distinct from queue admission,
and retain source-linked regression evidence. Do not hide failed baseline-only cycles inside event-row
counts. Phase 08 is marked complete only after its release and runtime resource gates passed;
Phase 05's separate installer/full-app memory checks remain explicitly open. Release unwind rationale
is now [D-21](../DECISIONS.md#d-21--release-uses-unwind-to-preserve-collector-failure-isolation).

## Next

Finish Phase 05's NSIS/full-app total/private memory evidence while Phase 09 begins OS-backed
collector rollout. Continue the remaining accepted phases; a completed runtime is one milestone,
not the whole goal. No external publication was performed; this journal stays local to the repository.
