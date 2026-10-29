# Process collector budget diagnosis — 2026-09-20

Status: current 15 ms gate **not passed**. No production changes or threshold changes from this investigation.

## Evidence

Historical acceptance in `docs/DECISIONS.md` records process p95 11.8723 ms over 50 cycles. Parent's earlier current-tree runs measured approximately 15–23 ms; its A/B without Phase18 exclusions still measured 15.582 ms. These prior measurements do not prove the current tree passes.

Current session, local time Asia/Saigon:

| Measurement | Result |
|---|---:|
| Existing `collector_performance-0564dd464c1d2219.exe`, sandbox | p95 114.8865 ms; test failed |
| Same executable, outside sandbox | p95 76.1885 ms; test failed |
| Process count before diagnostic | 321 |
| CPU counters at 16:40:44 | 88% total; 52% privileged |

Read-only diagnostic compiled in the unoptimized development profile. It wrapped the production provider to measure provider elapsed time inside the full collector cycle; collector overhead is the paired duration difference. Fifty measured cycles followed one initial baseline. A separate 50-walk diagnostic timed the equivalent Win32 API stages, without storing names or paths.

| Duration | Sandbox p50 / p95 | Outside sandbox p50 / p95 |
|---|---:|---:|
| Full collector cycle | 123.3457 / 283.3291 ms | 169.5411 / 359.0558 ms |
| Provider within collector | 99.439 / 264.4105 ms | 140.4398 / 346.0822 ms |
| Paired collector overhead | 19.8172 / 62.2839 ms | 29.1013 / 82.0637 ms |
| ToolHelp snapshot | 15.2338 / 56.6857 ms | 19.6528 / 32.3677 ms |
| ToolHelp row walk | 9.0491 / 50.1564 ms | 10.4141 / 17.729 ms |
| Open process handles | 5.3221 / 78.5082 ms | 9.9509 / 20.7556 ms |
| Query executable paths | 20.7282 / 61.0666 ms | 30.3377 / 50.9278 ms |
| Query process times + close handles | 1.848 / 4.2284 ms | 2.4176 / 6.6818 ms |
| Processes / readable paths | 329 / 170 | 346 / 185 |

Stage percentiles are separate distributions and must not be summed. Sandbox and outside-sandbox runs were sequential, under changing host load; their difference is not a causal estimate of sandbox overhead.

A nominal two-second PowerShell CPU-delta sample showed `codex` 4.69 CPU-seconds, `ChatGPT` 2.98, two `java` processes 2.42/2.31, plus other application activity. The sampling command itself lasted longer under load, so these are workload indicators, not normalized utilization percentages. The 88% counter above is the utilization measurement. No unrelated processes were terminated.

## Hypotheses tested

1. **Phase18 exclusion overhead alone:** unsupported. Parent's exclusion-free A/B still exceeded 15 ms. Current attribution also shows ToolHelp snapshot alone exceeding the complete-cycle budget.
2. **More processes than the original host:** unsupported. Observed 321–346 processes; the Phase09 source records approximately 370 as its sizing reference. Historical exact count was not retained.
3. **Sandbox alone:** eliminated as a sufficient explanation. Identical existing binary failed outside sandbox at 76.1885 ms. The later outside-sandbox diagnostic was slower still while load changed.
4. **Host contention:** directly observed. High total/kernel CPU and large, variable Windows API durations make these runs unsuitable for attributing a small code regression. This explains why the current measurements cannot establish the earlier 15–23 ms regression's code-level cause; it does not prove that no code inefficiency remains.

Provider evidence chain: `src-tauri/src/platform/windows/process.rs:94` performs the complete snapshot; line 128 creates it; line 181 opens each process; lines 193 and 222 query path and creation time. Completeness and privilege constraints remain intact.

## Phase20 optimization proposal

`src-tauri/src/collectors/process/mod.rs:263` clones the acknowledged map on every cycle, then performs continuation lookups, clones episodes, computes a two-pass diff and finally recognizes unchanged state at line 358.

Add a quiet-cycle fast path **after** successful full enumeration, process-map publication and initial-baseline handling. Admit it only when all are true:

- No pending backlog.
- Current tracked map exactly equals the acknowledged map, including evidence availability.
- No episode has reached `WINDOW_MS`.

Then preserve existing quiet-cycle behavior: clear downtime and return `Noop`. This avoids redundant state clones and diff work without skipping OS observation, confidence changes, PID reuse, pending drains or due churn summaries. Expected improvement is unmeasured; do not claim it fixes the current gate.

Validate with existing process lifecycle/privacy tests plus quiet-cycle, evidence-upgrade/downgrade, pending-drain and due-churn regressions. Measure before/after under controlled host load, using the unchanged full-cycle 50-sample 15 ms gate. Record process count and host utilization beside the result. Temporary diagnostic source was removed after these measurements.

## Unresolved

- Current tree needs a controlled passing performance run; the gate remains open.
- The cause of the earlier narrow 15–23 ms measurements cannot be isolated from this session's much higher host load.
