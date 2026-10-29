# Graph candidate canonical query matrix — 2026-10-04

Outcome: current-source 10k and 100k matrices PASS 8/8 each. The 1M matrix
is **7/8, investigation FAIL 1629.336/1500 ms**. These are profiler-free 30-sample acceptance runs,
separate from five-call attribution diagnostics. No outlier was removed.

## Reproduction and provenance

Workdir `src-tauri`; existing target cache, `CARGO_BUILD_JOBS=1`, owned
`.perf-temp` for TEMP/TMP. `WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS` unset.
For each canonical scale, set `WHAT_CHANGED_PERF_ROWS` then execute:

```powershell
cargo test --release --locked --features devtools --test perf_queries all_eight_query_shapes_meet_the_selected_budget -- --ignored --exact --nocapture --test-threads=1
```

Same source and artifacts as the [graph experiment](qa-261004-graph-fact-deduplication.md).
Query binary SHA-256: E67127DE0BED28F740CF02EBF98089F80F5FB6A7824FCAA83D02850D35FC23B3.
Fixture recipes, 30 samples, actual service calls, warm scoring/persistence
assertions and budgets are unchanged. This gate does not prove all actual
SQL branch plans, installed-app resources or release readiness.

## Measured cells

Each cell is p95 / budget, milliseconds.

| Actual service shape | 10k | 100k | 1M |
|---|---|---|---|
| First page, 200 events | 0.592 / 10 | 0.658 / 20 | 0.720 / 45 |
| Page 10, keyset cursor | 0.616 / 10 | 0.738 / 20 | 0.727 / 45 |
| Dashboard summary | 0.151 / 15 | 0.349 / 30 | 2.669 / 80 |
| Two-token search | 0.662 / 40 | 3.853 / 90 | 45.964 / 250 |
| Event details and prior | 0.074 / 10 | 0.093 / 15 | 0.400 / 30 |
| Persisted investigation, 5k cap | 120.964 / 400 | 361.874 / 600 | **1629.336 / 1500 FAIL** |
| Compare 3k checkpoint | 26.790 / 150 | 68.605 / 200 | 48.172 / 300 |
| Privacy summary | 0.796 / 30 | 1.246 / 60 | 1.254 / 200 |

10k: 1 test passed, zero failed, 3 filtered, 9.87 s. Seed wall 1,228.039 ms;
inner 1,187.566 ms, including generation 58.324, insertion 720.637,
graph 25.642, inventory 42.831, checkpoints 88.225 and investigations 218.137 ms.
Canonical counts: 10k events / 3k entities / 1,500 related / 800 bursts /
40 checkpoints / 9,800 snapshots / 20 investigations / 90 days.

100k: 1 test passed, zero failed, 3 filtered, 22.72 s. Seed wall 14,169.193 ms;
inner 14,128.567 ms, generation 536.476, insertion 12,567.783,
graph 526.199, inventory 27.755, checkpoints 86.597 and investigations 348.723 ms.
Canonical counts: 100k events / 3k entities / 15k related / 8k bursts /
40 checkpoints / 9,800 snapshots / 20 investigations / 90 days.

## Remaining evidence

1M: exit 1, 0 tests passed / 1 failed / 3 filtered, 581.95 s. Failure:
`start_investigation_5k p95 1629.336 ms > 1500 ms`, at
`tests/perf_queries.rs:88`. Canonical counts: 1M events / 3k entities /
150k related / 80k bursts / 40 checkpoints / 9,800 snapshots /
20 investigations / 90 days. Seed wall **532,237.321/240,000 ms — MISS**;
inner total 532,181.910 ms, generation 11,705.319, insertion 496,448.630,
graph 19,823.163, inventory 75.079, checkpoints 172.412 and investigations
3,901.690 ms. Post-run owned fixture is absent; D: free 2,989,576,192 bytes;
owned Cargo/rustc/seeder/query processes all exited.

Failed-verification advisory checkpoint ran before further edits. Counsel
retains the correctness-verified graph change provisionally, without claiming
acceptance. Next is one predeclared 30-call paired stage-attribution run,
retaining all samples and inspecting stages of the same slowest calls.
Seed attribution follows that query decision. No green-seeking unchanged
acceptance rerun, threshold relaxation or inferred host cause is authorized.

No phase is complete from these two scales. Seed at 1M still exceeded
240 seconds in the preceding diagnostic. Query-plan parity, collector-shaped
database size, resources, soak, installed accessibility and release gates
remain independent obligations.

Unresolved questions: source-owned attribution of the investigation tail;
1M query and seed budgets remain unmet.
