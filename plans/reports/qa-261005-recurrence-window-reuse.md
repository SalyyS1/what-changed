# Executed recurrence window reuse — 2026-10-05

Outcome: bounded warm-capture statistics implemented, reviewed and focused
tests PASS 15/15 and release/all-feature library PASS 479. Caller integration
and complementary lint also pass. Canonical 10k and the one predeclared 1M
structural profiles pass. No production recurrence algorithm
or acceptance gate changed.

## Scope and reason

The [paired tail evidence](qa-261004-paired-investigation-tail.md) establishes
recurrence as 44–46% of the same three slowest actual-service calls. Counsel
requires exact input reuse statistics before selecting one bounded optimization;
these counts establish opportunity, not an expected speedup. No more 30-call
attribution reruns are planned for this structural question.

Only an explicitly active, devtools-only recurrence capture walks the existing
sorted execution order. It borrows input keys and retains eight integer counters;
inactive capture performs no additional traversal, allocation or statistics
work. SQL, binds, connection, input sorting, 2,000-query threshold, actual COUNT
executions, returned maps and 150-query duplicate-ID aggregation are unchanged.

Exact signature means entity/category/event type; window also includes since
and before. Empty/reversed intervals count as keys and as invalid intervals.
Overlap is strict half-open overlap of immediately adjacent nonempty windows
within one signature. Invalid windows interrupt adjacency; monotone-before
overlap additionally requires current.before >= previous.before.

The statistics are returned with the same warm bound plan and printed outside
measured requests. No identity, category, type, bounds, event ID, bind values,
path or raw SQLite error is retained in this statistical evidence.

## Review and checks

Independent review message found no actionable defect and confirmed signature
equality matches the executed SQL key. Simplifier reviewed the frozen scope
and made no edits. Eight deterministic statistics tests cover empty input,
signature boundaries, repeated windows, touching/disjoint intervals, nesting,
invalid adjacency, maximum group size and supplied execution order. Existing
capture tests now verify 2,000 and 5,000 successful counts; inactive invalid
indices prove the scope guard prevents traversal.

The first `cargo fmt --all -- --check` exited 1 for one expression's line
wrapping in `recurrence_window_statistics.rs:40`. Failed-verification counsel
approved formatting only. `cargo fmt --all` changed only that file; the
subsequent format check and scoped diff check pass. Focused release/all-feature
build completed in 5m23s; 15 tests passed, zero failed, 466 filtered in 0.03 s.
Full release/all-feature library then passed 479 tests, zero failed, two
ignored, 481 total in 43.81 s. Complementary release Clippy passed for all
targets/features in 1m13s and normal lib/bins without default features in
20.38 s, both with warnings denied. The ignored tests are the explicitly selected
graph-builder comparison and real Windows device-sweep budget check; neither
was run by this ordinary unit-suite command. Caller integration passed service
6/6 (1.33 s), repository 24/24 (7.71 s), and profile-target fixture cleanup 3/3
(0.15 s; two explicit diagnostics ignored). That release link/build took
13m30s. Coverage percentages were not measured.

## Commands and source provenance

Workdir `src-tauri`; existing release target cache, locked dependencies,
`CARGO_BUILD_JOBS=1`, owned `.perf-temp` TEMP/TMP. One root Cargo runner.

```powershell
cargo test --release --locked --all-features --lib storage::event_repository::recurrence_plan_capture:: -- --test-threads=1
cargo test --release --locked --all-features --lib -- --test-threads=1
cargo clippy --release --locked --all-targets --all-features -- -D warnings
cargo clippy --release --locked --lib --bins --no-default-features -- -D warnings
cargo test --release --locked --features devtools --test investigation_service --test repository --test investigation_stage_profile -- --test-threads=1
```

Paths relative to `src-tauri/`, SHA-256 after approved formatting:

This source/artifact revision is the structural-run freeze, before the later
test-only prototype module registration. That extension changes no production
body and is verified separately; do not treat these hashes as its unit artifact.

| Source | SHA-256 |
|---|---|
| src/storage/recurrence_window_statistics.rs | 1003C7D505B16F8176AE02E8F0C3C3A372FB4495374A20F43729C949CD9DDF3C |
| src/storage/recurrence_window_statistics_tests.rs | 6470B3EB505AC65CC2DDF9644F9B957F47EAEB01C81B13AC0441742BD0681FBA |
| src/storage/recurrence_plan_capture.rs | 80F40A00A6299E55869FED1E9AB42C402F0668526D4FD75884978207AECA5263 |
| src/storage/recurrence_plan_capture_tests.rs | 9F5258D4E2EB97812BF32433DB9803343D179B023AD68F6A5DB9AE8DE8B4A320 |
| src/storage/event_repository.rs | 751BE2379644DFDCE4D256E4856EF5B86B2DA718AD79B805733E0706CFB7C60F |
| tests/investigation_stage_profile.rs | 5DE4A068FB50C4004A7286B4EF0D587D31FD3256497985C2651A4BC991728710 |

## Canonical structural measurements

Both selected runs use stage opt-in, unchanged canonical recipe and the exact
five-call diagnostic target; warm capture is excluded from the five calls.
Set `WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS=1` and
`WHAT_CHANGED_PERF_ROWS` to `10000` or `1000000`, respectively:

```powershell
cargo test --release --locked --features devtools --test investigation_stage_profile five_persisted_investigations_report_actual_stage_timings -- --ignored --exact --nocapture --test-threads=1
```

The 10k run passed one selected test, four filtered, in 2.39 s. Its actual warm
capture completed all 5,000 counts through the covering recurrence index,
without an events scan. All measured service calls persisted scored results.

| Warm input metric | Canonical 10k | Canonical 1M |
|---|---:|---:|
| Input queries | 5,000 | 5,000 |
| Distinct signatures | 3,217 | 3,429 |
| Distinct windows | 4,680 | 5,000 |
| Identical window repeats | 320 | 0 |
| Empty/reversed windows | 0 | 0 |
| Maximum queries/signature | 22 | 18 |
| Adjacent overlapping windows | 1,783 | 1,571 |
| Monotone-before overlapping windows | 1,783 | 1,571 |

The 10k seed wall time was 1,876.956 ms. Five diagnostic e2e observations were
68.455, 67.347, 68.192, 71.630 and 69.532 ms (mean 69.031 ms). Timing includes
stage stderr and establishes no acceptance gain. Cleanup left no owned
`what-changed-perf-*` fixture; D: free space was 3,037,057,024 bytes.

Frozen executable SHA-256 for both selected structural runs:

| Artifact | SHA-256 |
|---|---|
| target/release/deps/investigation_stage_profile-bd8cbe01cef57f37.exe | 5088FEDD7577ECE45ACD3AC504BCD8061F77D4450D36D4069F623278EB8F55A9 |
| target/release/seed_history.exe | C59AEA0FD4787BA79A34FDD3CECCAD64BA114E9A959EA7184C3287BF4DB2EF72 |

No source or build changes occur between 10k and the one fresh 1M run.

The 1M selected test passed in 368.38 s, four filtered. Canonical metadata
remained 3,000 entities, 150,000 related rows/edges, 80,000 burst rows, 40
checkpoints, 9,800 snapshots, 20 investigations and 90 days. Warm capture
completed all 5,000 actual counts through the same covering index. Strict
pairing validates one engine and one service record in each warm/measured
interval for both scales, with all five observations retained.

The 1M seed wall time was 350,420.397 ms (inner 350,378.704 ms; generation
7,590.104, insertion 327,040.851, graph 13,573.016, inventory 42.831,
checkpoints 89.599, investigations 1,996.726 ms). Five diagnostic e2e calls
were 851.361, 836.039, 818.710, 824.311 and 1,644.692 ms; mean 995.022 ms.
The fifth call's recurrence/graph were 919.142/275.340 ms; no outlier was
discarded or host cause inferred. These five timings are not acceptance p95.
The owned fixture is absent, all runner/seed/profile processes exited, and
D: free space is 3,035,942,912 bytes. Both executable hashes remain unchanged.

Exact-window caching is unsupported as the main 1M repair: there are zero
repeats. Counsel approves one test-only transaction-local monotone-overlap
COUNT prototype, preserving production routing and capture. Its two delta
probes can raise executions from 5,000 to 6,571; overlap does not predict a
31.4% speedup. Autocommit must use the unchanged path without an implicit
transaction. Promotion requires complete-map parity and alternating matched
representative/dense/sparse/wide-shift comparisons; marginal benefit rejects
the candidate. No further structural or 30-call diagnostic rerun is planned.

## Acceptance remains open

The [profiler-free matrix](qa-261004-graph-candidate-query-matrix.md) records
10k/100k 8/8 but 1M investigation 1629.336/1500 ms FAIL, seed
532.237/240 s MISS. Statistics do not close either failure. All 57 open plan
checks, source-query-plan parity, resources, installed release and public-v1
approval remain independent obligations. No user AppData access or publication.

Unresolved questions: parity and performance of the single test-only rolling
COUNT candidate remain unverified; seed insertion needs a separate repair.
