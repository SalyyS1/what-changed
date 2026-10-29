# Process collector cost scout — 2026-10-26

Status: source-only findings. No live stage measurements, production fix, tests,
builds, or threshold changes performed. Parent's release build was running.
The four current process p95 failures remain failures; no stage dominance or
code-level cause is established. Scope: full MVP, `--auto --advise`; no `--yagni`.

## Fixed measurement contract

[`collector_performance.rs`](../../src-tauri/tests/collector_performance.rs):25–26
sets **50 cycles / 15 ms p95**. Lines 65–77 time the real `snapshot_batch`,
`commit_succeeded` for Atomic results, `black_box`, and `drop(snapshot)` together.
Start, initial baseline and its acknowledgement occur before measurement
(44–59). Durable SQLite commits are not in the timed region. Keep this gate and
its complete region unchanged.

Current evidence: [continuation report](qa-261026-continuation.md), process p95
92.4285, 17.4784, 57.5130, 86.1510 ms. The September
[diagnostic](debug-260920-process-budget-host-contention.md) is historical,
unoptimized, under different host load and older source. It cannot establish
the dominant stage or cause of the October failures.

## Source boundaries

| Boundary | Operations and exact source locations |
|---|---|
| Provider | [`windows/process.rs`](../../src-tauri/src/platform/windows/process.rs):85–95 clears caller rows and guarantees empty output after failure. `enumerate_process_rows` (99–129) creates a ToolHelp snapshot, locks shared path scratch, snapshots probe exclusions, walks every entry, then filters exclusions. `take_snapshot` (133–148) permits up to three attempts. |
| Per-process calls | Same file:160–168 allocates image text and calls `open_details`; 186–194 calls `OpenProcess` for each row, then `QueryFullProcessImageNameW` (198–242) and `GetProcessTimes` (247–253) after successful open. `OwnedHandle::drop` (290–298) closes process and snapshot handles. Thus full OS observation repeats even for unchanged processes; its measured cost is unknown. |
| Provider allocations | Same file:236 and 262 create UTF-16-derived path/image Strings each walk. The caller Vec and 4,096-unit scratch buffer are reused; scratch grows only on insufficient buffer, at most 32,767 units (31–69, 210–240). Path storage is cleared after reading. There is no evidence that those allocations dominate latency. |
| Probe exclusions | [`tool_probe.rs`](../../src-tauri/src/platform/windows/tool_probe.rs):75–92 bypasses the registry lock when the atomic flag is false; otherwise locks/sweeps once per walk and clones its small exclusion view. This is not a per-row registry lock. |
| Quiet path | [`process/mod.rs`](../../src-tauri/src/collectors/process/mod.rs):211–235 always enumerates first. `quiet_rows` (459–479) checks no downtime/pending/due churn, equal row count, instance continuity and current path/creation availability. A proven quiet poll counts denials, clears row strings and refreshes map liveness, then returns Noop. It does not allocate a tracked BTreeMap or run diff/ack. |
| Merge/current view | Same module:236–238 calls [`state.rs`](../../src-tauri/src/collectors/process/state.rs):227–267. It clears/refills a reused PID index and allocates a fresh owned BTreeMap. Continuing instances clone image/identity data (171–185); fresh instances clone path/image and derive identity (147–161). Provider path/image Strings then drop via [`traits.rs`](../../src-tauri/src/platform/traits.rs):118–125. |
| Map publication | `process/mod.rs`:243–254 calls [`process_map.rs`](../../src-tauri/src/collectors/process_map.rs):129–163. Under one write lock, two reusable HashMaps swap, existing unchanged identity strings move across, changed/new identities clone text, leftovers drop, timestamp/token refresh. Quiet `refresh` (169–178) only locks and updates timestamp for the matching publisher. The performance test creates its own map and starts no listener against it, so this test does not demonstrate cross-collector map lock contention. |
| Reconcile/diff/stage | `process/mod.rs`:279–290 can return Noop after map publication. Otherwise 292–334 clones acknowledged/pending/churn state, updates continuing evidence, closes due churn, computes diff. [`diff.rs`](../../src-tauri/src/collectors/process/diff.rs):48–68 performs two ordered-map traversals/lookups and clones changed instances; 104–132 applies drained changes. `process/mod.rs`:344–400 creates observations/pending state and stages. `stage` (154–171) calls [`stateful.rs`](../../src-tauri/src/collectors/stateful.rs):86–109, which serializes the full next state to JSON Value, serializes that Value to bytes to check its size, then drops the bytes. This allocation work occurs on staged cycles, including changes with no event. |
| Acknowledgement | `process/mod.rs`:403–413 calls [`snapshot.rs`](../../src-tauri/src/collectors/snapshot.rs):242–254. After token validation it takes the candidate and assigns it to committed state. That assignment drops the old committed BTreeMap/pending/churn state. It does not perform a durable database commit in this test. |
| Destruction | Collector-local current/target/diff temporaries drop inside `snapshot_batch`; old committed state drops inside acknowledgement. The explicit test `drop(snapshot)` additionally destroys observation strings and the serialized JSON baseline for Atomic snapshots; Noop has no payload. These are distinct costs and all stay in the full timer. |

## Minimal next diagnostic patch

Own only test code: add an explicitly invoked ignored diagnostic in
`src-tauri/tests/collector_performance.rs` (or a test support module) alongside
the existing unchanged acceptance test. Reuse its `PERF_LOCK`, one initial
baseline/acknowledgement, 50 cycles, real provider, real collector, token flow,
Atomic acknowledgement, `black_box` and explicit snapshot destruction.

1. Implement test-only `TimedProcessProvider<P>` using the existing
   [`ProcessProvider`](../../src-tauri/src/platform/traits.rs):67–86 seam.
   **Override `enumerate_into`** and directly delegate
   `inner.enumerate_into(output)`, timing that exact call. Delegating only
   `enumerate` would activate the trait's default `enumerate_into`, add another
   whole-table Vec and change the measured allocation behavior. Implement the
   required `enumerate` method as direct delegation too.
2. Store only duration and row count in shared atomics after the delegated call;
   never copy rows or log names, PIDs, paths or identities. Reading/resetting
   diagnostic cells and appending a preallocated numeric sample happens outside
   the full cycle. Explicitly disclose the extra timestamps/atomic stores as
   diagnostic overhead; do not subtract that overhead from acceptance results.
3. Record timestamps immediately before `snapshot_batch`, after its return,
   after conditional acknowledgement, and after `black_box` plus `drop(snapshot)`.
   Preserve the full begin-to-after-drop measurement. Record result kind and
   observation count before destruction; do no formatting, printing or
   per-sample allocation inside the timed region.
4. For each same-cycle tuple report total, provider, snapshot remainder
   (`snapshot_batch elapsed - provider elapsed`), acknowledgement, destruction,
   row count and Atomic/Noop. Keep samples paired; report p50/p95 and the slowest
   full-cycle tuples after all 50 cycles. Do not subtract independent p95s or add
   stage p95s. The snapshot remainder includes instrumentation overhead and all
   merge/map/diff/baseline work; label it honestly.
5. The existing trait can separate provider from collector work, but cannot
   individually time private `quiet_rows`, merge, concrete ProcessMap publication,
   diff and staging. If the paired remainder is material, a later diagnostic-only
   instrumented build must place timestamps at `process/mod.rs`:216, 223,
   237–238, 243–254, 292–334, 344–400 and 162–163. Do not claim the first wrapper
   separately measured these inner operations. Likewise, provider internals need
   later diagnostic hooks at snapshot acquisition, scratch lock, exclusion
   snapshot, ToolHelp walk, open/path/time/close boundaries if provider data
   warrants that next step.

Wait for the parent build to finish before live measurement. Capture host
CPU/process counts and artifact identity alongside one bounded diagnostic run;
retain changed-machine/load limitations. Leave the original acceptance test as
the authority and select a production change only after current stage evidence.

## Unresolved

- Which current stage drives the failing p95, and whether slow cycles are Noop or
  Atomic? Requires the paired live diagnostic.
- Whether any code change can meet 15 ms under the observed host load remains
  unproven. No caching, skipped OS calls, threshold relaxation or candidate fix
  is authorized by these source findings.
