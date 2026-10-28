# Checkpoint event-window source freeze — 2026-10-05

Status: source frozen after root-owned rustfmt, full formatting and scoped diff
checks PASS. Focused release/locked/devtools10 cases and full-library535 pass,
zero failures; caller7 and both strict release Clippy configurations pass.
All eight source hashes rechecked exact; exactly one counseled canonical10k
window smoke passes with245 fully decoded IDs and indexed SEARCH. No whole
comparison-plan, current-source p95 or acceptance closure follows.
[Protocol](qa-261005-checkpoint-window-capture-ledger.md).
Root strengthens the one ordering test with out-of-timestamp insertion order;
all source edits are limited to these eight paths. Worker executed no formatter,
Cargo, test or workload. Root owns subsequent gates and failure counsel.

| Path | SHA-256 | Lines |
|---|---|---|
| src-tauri/src/storage/checkpoint_repository.rs | `7eef458df39672eb4c440399e656d50eea517ae6a2b69a9d21812aa244f6f83a` | 551 |
| src-tauri/src/storage/mod.rs | `12f1b652eb9ffc1960b7a4aecb26c2833cb2624a1681068a85540e667ff213f0` | 107 |
| src-tauri/src/storage/page_plan_capture.rs | `a8e12232ef713cc574428a591a270be1b20d1c0b0426d1e944c2494cc5238c95` | 171 |
| src-tauri/src/storage/checkpoint_window_plan_capture.rs | `ac398d95c9bc6c748fe1341edc9551ae7e9c89bbc2397e12209d96ec79caf8d5` | 156 |
| src-tauri/src/storage/checkpoint_window_plan_evidence.rs | `61b71a9d7068381a66747b258965b8c0325df2be24ad2577141c141b9653d463` | 30 |
| src-tauri/src/storage/checkpoint_window_plan_capture_tests.rs | `6456d34d1610e0815c847cb6e1d3d49de061a69ef1a04c27df7fcef14226e87c` | 164 |
| src-tauri/src/storage/checkpoint_window_owner_tests.rs | `d71e20785b0065799d4cad74ae9e65954464cd2ca4e4b6c1e0f8565e97a34c5d` | 147 |
| src-tauri/tests/checkpoint_window_plan_profile.rs | `ac4565460ccdd8a67797b978b3fc76f0889a004f8d1d07c7a6cb8052596affd8` | 52 |

The existing551-line checkpoint repository receives only the narrow owner hook;
core156/evidence30/lifecycle164/owner147/profile52 are cohesive real boundaries.
No arbitrary unrelated modularization. PAGE171 changes only a forwarding wrapper;
parser vocabulary/types/behavior and PerfFixture remain unchanged.

Primitive EXPLAIN SQLite-error proof is separate from actual owner behavior.
The incompatible-schema decode fixture proves real SQLite error preservation,
not a reachable normal migrated-schema corruption path. Actual unsupported-index
fixture proves original result independence without retaining private index names.
Canonical profile is ignored by ordinary suites; root runs it once only after
ordinary gates and execution counsel pass. [Complete output](qa-261005-checkpoint-window-canonical-output.md).
This proves only one compare subquery, never by_id/items/full compare/p95.

Unresolved questions: remaining full-service/branch/scale/native/release gates
and frozen-RC latency refresh are outside this source freeze.
