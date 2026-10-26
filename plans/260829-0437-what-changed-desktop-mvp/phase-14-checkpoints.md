---
phase: 14
title: "Checkpoints: known-good snapshots and diff-since-checkpoint"
status: completed
priority: P1
effort: 5h
dependencies: [13]
---

## Overview

A checkpoint is a named "it worked here" marker plus a full state snapshot, so the user can later ask
"what changed since then?" and get an answer that does not depend on event retention or on the
collectors having been running the whole time (§23–§25). Checkpoints are created manually, from the
tray, and automatically before high-risk changes. This is the feature that converts the app from a log
viewer into a debugging tool, and it is deliberately built before the analysis engine so the engine can
use checkpoint boundaries as evidence.

## Requirements

**Functional**
- `create_checkpoint(name: Option<String>, kind: Manual|Automatic, reason: Option<String>)`: requests
  a forced durable cycle from every enabled collector worker, serializes each collector's compact state
  into checkpoint-owned `snapshots` rows, and writes one `checkpoints` row. The schema and exact columns
  remain owned by the migrations and repository.
- Default name when none given: `Checkpoint <weekday> <h:mm am/pm>` (e.g. `Checkpoint Friday 3:14 PM`).
- All values in `state_json` pass through the P03 sanitizer — a checkpoint is a durable artifact and is
  covered by an extra §60 leak assertion.
- Expected cost: sum of measured snapshots ≈ 6.2 + 3.3 + 50 + 38 + 1440 + 1325 + 2128 ms ≈ 5.0 s. The UI
  therefore shows a determinate progress list ("Programs ✓ / Services ✓ / Drivers …") driven by a
  `CheckpointProgress` event, and the command returns only when finished. A collector that fails or
  times out (8 s per collector) marks the checkpoint `partial: true` and records which collectors are
  missing — a partial checkpoint is still useful and is labelled everywhere it appears.
- `list_checkpoints()` → newest first with name, kind, age, item count, partial flag.
- `delete_checkpoint(id)` (cascades items), `rename_checkpoint(id, name)`.
- `compare_checkpoint(id, to: Now | Checkpoint(id2))` → `CheckpointDiff { added: Vec<DiffItem>,
  removed: Vec<DiffItem>, changed: Vec<DiffItem { entity, field, before, after }>, grouped_by_category,
  event_ids_in_window }`. `changed` uses collector-owned `checkpoint_fields` adapters and shared live
  helpers where identity semantics match. The checkpoint layer owns only the indexed hash-join and
  category grouping; it does not apply a generic recursive JSON diff.
- `event_ids_in_window` links the structural diff to the recorded event stream between the two
  timestamps so the UI can show "27 changes recorded in this window" and jump to the timeline filtered
  to that range.
- Automatic checkpoints: created before a detected Windows Update install (WU/Operational "installation
  started"), before an `application/installed` that is still in progress (best-effort: on the first
  `msiexec`/`*setup*.exe`/`*installer*.exe` process start, debounced to at most one per 10 min), and
  once every 7 days if none exists. Automatic checkpoints are capped at 10 (oldest auto pruned; manual
  ones are never auto-pruned) and are exempt from the P04 retention purge (D-12).
- Tray: "Create checkpoint" is one of the seven §7 entries and calls the same command; a success balloon
  reads "Checkpoint saved. You can compare against this later."
- UI (§24, §25): Checkpoints screen — list with primary action `Compare to now`; comparison view with
  three sections (Added / Removed / Changed), category grouping, a `Show technical details` toggle
  (§51), and the empty state "No checkpoints yet. Create one while things work, so you can compare
  later." Six states covered in the dev harness.

**Non-functional**
- `create_checkpoint` p95 ≤ 8 s on this machine with all collectors enabled; the UI stays responsive
  (progress events at least every 500 ms) and no other query is blocked for more than 20 ms.
- `compare_checkpoint` p95 ≤ 150 ms for a 3000-item checkpoint (two indexed reads + an in-memory
  hash-join, no N+1 queries).
- A checkpoint of a 3000-item machine occupies ≤ 2 MB in SQLite (state_json is per-item, compact keys).

## Architecture

`checkpoint/{service.rs,create.rs,compare.rs,state_items.rs,auto.rs}` owns orchestration and semantic
projection; `storage/checkpoint_repository.rs` owns atomic persistence; `commands/checkpoints.rs` owns
the typed IPC boundary. Creation requests durable cycles from existing collector workers. Comparison
hash-joins compact item sets and delegates field semantics to collector-owned checkpoint adapters;
compare-to-now reads acknowledged durable state without forcing a sweep. Truncated or malformed state
marks its collector missing and is excluded from comparison.

## Related Code Files

**Backend authorities** — `src-tauri/src/checkpoint/{service.rs,create.rs,compare.rs,state_items.rs,
auto.rs}`, `src-tauri/src/storage/checkpoint_repository.rs`,
`src-tauri/src/collectors/runtime/capture.rs`, `src-tauri/src/commands/checkpoints.rs`, and
`src-tauri/src/domain/checkpoint.rs`. `src-tauri/src/dto/bindings.rs` owns generated contract inclusion.
Collector modules own their `checkpoint_fields` semantic adapters; live diff helpers are reused where their
identity rules match.

**Frontend authorities** — `src/features/checkpoints/`, `src/lib/stores/checkpoints.svelte.ts`, and the
generated contract in `src/lib/tauri/bindings.ts`.

**Acceptance owners** — `src-tauri/tests/checkpoint_{create,compare,auto,leak,real}.rs` and the checkpoint
frontend tests colocated with the feature/store.

## Implementation Steps

The accepted implementation is routed through the authorities above. Creation uses existing workers and
their durable acknowledgements; comparison projects compact items through collector-owned semantics;
automatic and tray entry points converge on the same application service. The acceptance suites own the
transaction, latency, partial-state, retention, privacy, trigger and lifecycle evidence.

## Todo (commit boundaries)

- [x] Trait-level semantic adapters + collector regressions still green
- [x] Checkpoint persistence and bulk insert
- [x] Creation with progress, timeouts, partial flag
- [x] Comparison with category grouping and event window
- [x] Automatic triggers, cap, retention exemption
- [x] Checkpoints UI with six states
- [x] Checkpoint test suites

## Success Criteria

- [x] Scripted 3000-item checkpoint: created in one transaction, `item_count = 3000`, DB growth ≤ 2 MB
- [x] One collector times out → `partial: true`, missing collector listed, UI shows "Partial — Drivers
      could not be captured"
- [x] Compare with 5 added, 3 removed, 2 field changes → exactly those, grouped by category
- [x] `compare_checkpoint` p95 ≤ 150 ms at 3000 items (asserted in the test)
- [x] Automatic checkpoint fires once for a burst of 5 installer starts within 10 min
- [x] 11th automatic checkpoint prunes the oldest automatic one and leaves all manual ones
- [x] Retention purge with a 1-day cutoff leaves checkpoint rows intact
- [x] `checkpoint_leak.rs`: no secret substring anywhere in persisted checkpoint storage
- [x] Tray "Create checkpoint" produces the same result as the in-app button

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test checkpoint_create --test checkpoint_compare --test checkpoint_auto \
           --test checkpoint_leak --test redaction_leak
cargo test --test collector_process --test collector_services --test collector_applications
cd /d/Project/.WhatChage && bun run check && bun run check:bindings && bun run test
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Checkpoint semantics drift from live collector behaviour | A P09–P13 or semantic-adapter regression fails | Keep field rules beside the owning collector and reuse live helpers where identity rules match |
| 5 s creation feels broken | Users click twice / the button appears dead | Determinate per-collector progress + the button disables during creation + a duplicate call returns the in-flight checkpoint id instead of starting a second one |
| Expensive sweeps during checkpoint creation stall the UI | Timeline queries slow while creating | Sweeps run on collector threads; the command awaits a completion channel; a UI-latency assertion runs inside `checkpoint_create.rs` |
| Checkpoint bloat | DB grows faster than the event stream | `state_json` stores compact semantic fields; the size assertion fails loudly if a collector starts dumping whole objects |
| Automatic checkpoints fire on unrelated `*setup*.exe` | Noise in the checkpoint list | Automatic checkpoints are visually distinct, capped at 10, and prunable; a false positive costs a row, not correctness |
| A partial checkpoint is compared as if complete | Phantom "removed" items for the missing collector | Comparison skips collectors absent from *either* side and states so in the UI ("Drivers not compared — missing from this checkpoint") |

## Security Considerations

- Checkpoints are the longest-lived artifact in the app (exempt from retention), so they get their own
  leak test in addition to the general §60 suite.
- `delete_checkpoint` and delete-all remove owned snapshot rows via `ON DELETE CASCADE`; Phase 19 owns
  exposing checkpoint counts in the privacy screen.
- No checkpoint export in the MVP (nothing to leak by sharing); the §58 diagnostics export includes
  checkpoint *counts* only, never item contents.
