# Analysis and executed recurrence diagnostics — 2026-10-04

Outcome: nine new diagnostic units PASS; release/all-feature library suite
468 passed, zero failed, one hardware-performance test ignored, 69.11 s with
one test thread. This continues the [service-stage report](qa-261004-investigation-service-stages.md).
It is diagnostic infrastructure, not a repaired performance gate or release candidate.

## Source contract and review

Seven aggregate analysis stages wrap the existing preprocess, structural graph,
chain index, failure index, candidate preparation, candidate scoring and
selection/explanations. The normal-release facade has no clocks, active state,
environment reads or stderr output. Devtools caches the existing explicit
`WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS=1` opt-in once per process.
Records contain only static labels, status and durations. Exclusive stages sum
to inclusive total; incomplete/unwinding requests are not marked complete.
No per-event clock, cap, ranking, scoring, provider or batching change was made.

The devtools-only recurrence capture explains the first sorted query at its
actual execution point, using the same connection, SQL constant and bind helper
as the prepared COUNT. It reports input count separately from successfully
completed executions. TLS is synchronous, nonnested and thread-bound; no borrow
crosses SQLite. Owned evidence has no bind values or raw database error text.
RAII clears state after return, errors or panic. A second sorted invocation
within the scope reports Multiple instead of attaching misleading evidence.

Only the warm actual-service call is captured. Capture is cleared before the
five timed scored/persisted calls. An indexed SEARCH of events is required,
without hardcoding the full SQLite plan text. This represents the first bound
sorted query only: it does not prove all bindings or the eight-shape plan gate.

Independent read-only reviewer and failure forecast found no actionable issue.
The scoped simplifier ran clean (no edits). Source-level callback/reentrancy
reasoning is not presented as an executed callback regression test.

## Verification

From `src-tauri`, `CARGO_BUILD_JOBS=1`, owned workspace `.perf-temp` TEMP/TMP,
existing `src-tauri/target` cache, and diagnostics environment unset:

```powershell
cargo test --release --locked --all-features --lib recurrence_plan_capture::tests -- --test-threads=1
cargo test --release --locked --all-features --lib analysis::stage_timings::enabled::tests -- --test-threads=1
cargo test --release --locked --all-features --lib -- --test-threads=1
```

Capture units: 6/6, 0.01 s; actual indexed execution/counts, inactive path,
nesting rejection, static explain error, multiple invocation and error/panic
cleanup. Analysis units: 3/3, duration sum/static fields, disabled state and
error/unwind status. Full library: 468/469 as above; includes the user-approved
scheduler-independent expired capture deadline and existing analysis parity.
The synthetic unfinished timing unit intentionally emits status=error; it is
not a failed test or an actual failed investigation.

A build was deliberately cancelled before tests when its target selected an
older QA cache and began recompiling dependencies. No gate result is claimed
for that invocation. The subsequent correct-cache focused build completed in
4m04s. No dependency or compilation settings were changed to reuse the cache.

Integration command, same environment:
`cargo test --release --locked --features devtools --test repository --test investigation_service --test investigation_stage_profile -- --test-threads=1`.
Build: 11m24s. Service 6/6 in 1.23 s; repository 24/24 in 5.16 s; profile
cleanup contracts 3/3 in 0.24 s, expensive opt-in diagnostic ignored normally.
Repository includes sorted recurrence versus individual indexed COUNTs across
2,100 inputs, duplicate IDs and i64/range boundaries. No test was weakened.
All-target/all-feature release/locked Clippy passed in 1m02s with `-D warnings`.
Normal release/locked lib/bins without features also passed Clippy in 14.11 s
with `-D warnings`. These complementary checks apply to the hashed attribution
revision below, before the graph-construction experiment. Coverage was not measured.

## Fresh canonical 10k profile

Same release/locked profile target with `WHAT_CHANGED_PERF_ROWS=10000` and
`WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS=1`, exact ignored diagnostic only,
`--nocapture --test-threads=1`: 1 passed, 3 filtered, 2.34 s. Canonical shape
verified: 10k events, 3,000 entities, 1,500 related rows, 800 burst rows,
40 checkpoints, 9,800 snapshots, 20 investigations, 90 days. Seed wall
1677.686 ms; inner generation/insert/graph/inventory/checkpoint/investigation
times 50.729/682.280/21.697/43.121/78.045/203.913 ms, total 1115.253 ms.
Owned fixture cleanup succeeded; no performance directory remained.

Warm execution reported exactly 5,000 input queries and 5,000 completed counts:
`SEARCH events USING COVERING INDEX idx_events_recurrence (entity_id=? AND category=? AND event_type=? AND timestamp>? AND timestamp<?)`.
This text has placeholders, not bind values; inclusive-since/exclusive-before
semantics are owned by the shared SQL and differential tests, not plan spelling.

Five scored/persisted measured calls: total 349.997 ms, mean 69.999 ms,
min/max 67.002/72.343 ms. Warm call excluded. These are not p95 measurements.

| Analysis exclusive stage | Five-call min–max, ms |
|---|---|
| Preprocess | 1.746–1.799 |
| Structural graph | 5.390–6.047 |
| Chain index | 0.860–2.360 |
| Failure index | 0.668–1.392 |
| Candidate preparation | 1.414–1.680 |
| Candidate scoring | 4.587–5.595 |
| Selection/explanations | 8.652–9.858 |

## Fresh canonical 1M profile

Same binary/source as 10k, changing only `WHAT_CHANGED_PERF_ROWS=1000000`:
1 passed, 3 filtered, 343.61 s; exact canonical 1M events, 3,000 entities,
150,000 related rows, 80,000 burst rows, 40 checkpoints, 9,800 snapshots,
20 investigations, 90 days. All six calls were scored and persisted, not
no-cause fallbacks. Owned cleanup succeeded; no performance directory remained.
The first bound sorted recurrence again used the covering index, with exactly
5,000 input queries and completed executions before capture was cleared.

Seed wall **322265.967/240000 ms MISS**. Inner total 322219.940 ms: generation
7301.258, insert 299610.679 (~93%), graph 13054.930, inventory 42.816,
checkpoints 94.524, investigations 2064.745. Seeding behavior was not optimized.
The difference from earlier seeds is not an attributed speedup.

Five measured e2e calls total 8624.008 ms, mean 1724.802 ms, min/max
1349.936/2623.092 ms. No outlier was discarded. Warmup is excluded.

| Call | Service total (inclusive) | Graph (exclusive) | Candidate scoring (exclusive) | Recurrence (exclusive) |
|---|---|---|---|---|
| 1 | 2622.592 | 351.208 | 261.963 | 1334.776 |
| 2 | 1597.854 | 245.282 | 136.908 | 684.556 |
| 3 | 1349.731 | 254.194 | 140.411 | 579.280 |
| 4 | 1495.079 | 283.327 | 143.902 | 684.931 |
| 5 | 1557.291 | 326.451 | 176.468 | 651.554 |

All durations above are ms. Analysis aggregate inclusive totals ranged
520.486–876.185; preparation 60.728–124.890, failure index 32.824–60.292,
selection/explanations 35.985–70.322. Relationship reads 138.261–300.049 and
window hydration 55.298–123.294 remain additional owners. This is attribution,
not the profiler-free 30-sample acceptance result.

Bounded host snapshots: during seed at 22:05:02 +07:00 CPU 37%, available
memory 17,406 MB, D queue 0/disk time 15%; after profile at 22:10:01 CPU 0%,
available memory 17,427 MB, D queue/time 0. These snapshots do not establish
host contention during the individual timed requests or explain the spike.

Counsel approves a bounded graph-construction experiment: normalize borrowed
undirected relationship facts, sort/deduplicate a Vec before owned adjacency
insertion, preserve event-derived edges, nodes, self-loops and kinds. Require
full equality to the old builder and isolated duplicate-heavy/mostly-unique
comparisons before expensive canonical acceptance. No gain is claimed yet;
even a useful graph reduction may leave the larger recurrence gate failing.

## Tested source and artifact SHA-256

Paths below are relative to `src-tauri/`:

| File | SHA-256 |
|---|---|
| src/storage/recurrence_plan_capture.rs | E719EB23CC7ADE59067EDC9E7EFBBAA772007EFBCE6F1684748EBC73C6C8CEB1 |
| src/storage/recurrence_plan_capture_tests.rs | 543B9ACB5F258E06C82B576B75D66BC631BD2F00EB77C419F0F85D9410C8F6EB |
| src/storage/event_repository.rs | A7E58B8C9810D245C572C30AD1BCAAFF15140F070265E832F09711127313AF18 |
| src/analysis/mod.rs | 0AB7026F8AD00919AC012A0951DFE39C82CD682D30E271F61BBE6741D1318AAD |
| src/analysis/graph.rs (baseline before candidate) | CDEB7A58AC78B17BDA46839C6EAB412B1957E05BC57F1CA3214A97351530508E |
| src/analysis/stage_timings.rs | 43492B5069F6AA7E45D2F4AA0E33C0E9A6BFE35AECCF6439A3DBB32D32E87407 |
| src/analysis/stage_timings_enabled.rs | FD340BDD83A3CEB317975BBCA1150567896197C70D6CD308C892EC987B79A546 |
| tests/investigation_stage_profile.rs | F66CE4C8F18C9ADC40EDA1C2CB1AC1D6DCED9E91A742942828D3639436185C91 |
| target/release/deps/what_changed_lib-708106f6a2e3e24a.exe | 5849BF333797186A1E31209DEB76271DD4510C9C55AC7251367DD557DE8CA8EA |
| target/release/deps/investigation_stage_profile-bd8cbe01cef57f37.exe | BF6BC3A9583F571400C87120B734E912583E98D50D54A14E67D500EBB31C7087 |
| target/release/seed_history.exe | 0C1F3A60FDFDB57F47FD1A2FED49C13E184915D842E1C1BD5DBD384CDE291612 |

## Remaining work

The latest profiler-free 1M acceptance remains 2403.553/1500 ms FAIL (7/8
query gates pass); its profiler-free seed wall remains 489448.246/240000 ms
MISS. Fresh five-call diagnostics above are attribution only. Validate the
counsel-selected graph experiment, then rerun unchanged 30-sample acceptance.
The canonical plan remains 312/369, 17/25 phases complete; no checkbox closes
for diagnostic instrumentation alone. Native/license, installed release,
resource/accessibility and human-publication gates remain open.

Unresolved questions: none for this bounded diagnostic implementation.
