# Phase 17 progress sync

Scope: Phase 17 System Map completion evidence captured on 2026-09-20. This is a stateful record, not an
evergreen implementation authority.

## Outcome

- Phase 17 status: **completed**; 18/18 phase checkboxes closed.
- Whole-plan phase checklist: **258/369 complete (69.9%)**, 111 remaining.
- Phase metadata: 16 completed, 1 in progress, 8 pending.
- Product-wide §48 System Map criterion closed. The overall plan remains in progress.

## Acceptance evidence

| Gate | Result | Authority |
|---|---|---|
| Evidence vocabulary | All five persisted relationship kinds covered; `recorded_at` is the latest retained proof; no synthetic executable edge | [payload tests](../../src-tauri/tests/system_map_payload.rs) |
| Dense query | Exact connected count 50,001 from 50,000 relationships; final 20-query run 74.254 ms first/max, 67.745 ms median, 73.951 ms p95; source and target indexes asserted | [performance test](../../src-tauri/tests/perf_system_map.rs) |
| Hardware rendering | Intel Iris Xe / Direct3D11, 400 nodes, 16.70 ms p50 / 16.90 ms p95, 41 accessible options, no page errors, panel beside canvas | [performance record](../../docs/PERFORMANCE.md#system-map-query-and-rendering--2026-09-20) |
| Lifecycle | 20 mount/unmount cycles retained no renderer resources or listeners | [scene lifecycle test](../../src/three/systemMap/scene.lifecycle.test.ts) |
| Frontend | 54 files / 304 tests; check and production build passed | [Phase 17 evidence](../260829-0437-what-changed-desktop-mvp/phase-17-system-map.md#acceptance-evidence--2026-09-20) |
| Rust focused gates | command 9, architecture 10, payload 4, performance 1; bindings, rustfmt and strict Clippy clean | [Phase 17 evidence](../260829-0437-what-changed-desktop-mvp/phase-17-system-map.md#acceptance-evidence--2026-09-20) |

The repository-wide Rust run was not green: it reached an unrelated Phase 09 real-machine process timing
assertion. That host-sensitive failure is recorded as out of Phase 17 scope and is not hidden by the focused
green gates.

## Authority sync

- Phase authority updated: [phase-17-system-map.md](../260829-0437-what-changed-desktop-mvp/phase-17-system-map.md).
- Plan index and exact counts updated: [plan.md](../260829-0437-what-changed-desktop-mvp/plan.md).
- Navigation/evidence surfaces updated: [README](../../README.md), [Roadmap](../../ROADMAP.md),
  [Architecture](../../ARCHITECTURE.md), [Performance](../../docs/PERFORMANCE.md), and
  [Decision D-25](../../docs/DECISIONS.md#d-25--system-map-traversal-stays-storage-owned-and-evidence-literal).
- `PRODUCT.md` was intentionally left untouched because its current changes belong to the existing
  public-v1 worktree.

## Remaining plan work

Phase 10 remains in progress. Phases 18–25 remain pending; their dependencies and acceptance criteria are
unchanged by this sync.

## Unresolved questions

None for Phase 17.
