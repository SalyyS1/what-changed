# Graph fact deduplication experiment — 2026-10-04

Outcome: complete graph equality tests 2/2 PASS. Two isolated synthetic
comparisons show repeatable duplicate-heavy improvement, with a small
mostly-unique slowdown. Release/all-feature library 470 passed, zero failed,
two performance diagnostics ignored, 27.10 s with one test thread. This is a
candidate experiment, not a canonical performance or release acceptance pass.
Baseline actual-service costs are in the [attribution report](qa-261004-analysis-and-recurrence-attribution.md).

## Change and review

Only relationship processing in `StructuralGraph::build` changes. Borrowed
tuples normalize undirected endpoints and mapped StructuralKind, then sort and
deduplicate before owned adjacency insertion. Event nodes and event-derived
SameEntity edges are unchanged. The existing ordered BTreeMap/BTreeSet graph
remains identical, including parallel kinds, reversed facts, self-loops,
relationship-only nodes, precedence and unknown-node behavior. No query,
transaction, scorer, cap, fixture recipe, dependency or pragma was changed.

The independent copied old builder compares complete graph equality on explicit
corner cases and deterministic generated duplicate-heavy/mostly-unique inputs
in both input orders. Review and failure forecast found no actionable issue;
the scoped simplifier ran clean. Known tradeoff: an O(relationships) borrowed
tuple buffer persists through construction; source-derived x64 estimate is
about 5.7 MiB at 150k rows, not an observed app working-set measurement.

## Commands and isolated results

From `src-tauri`, release/locked, `CARGO_BUILD_JOBS=1`, existing target cache,
owned `.perf-temp` TEMP/TMP, stage timing environment unset:

```powershell
cargo test --release --locked --all-features --lib analysis::graph::builder_tests:: -- --test-threads=1
cargo test --release --locked --all-features --lib analysis::graph::builder_tests::benchmark::isolated_synthetic_old_and_new_builder_durations -- --ignored --exact --nocapture --test-threads=1
cargo test --release --locked --all-features --lib -- --test-threads=1
```

Focused build 4m17s; two equality tests pass in 0.03 s, benchmark deliberately
ignored. Explicit benchmark passes in 7.01 s then 6.38 s. Each run compares
identical 5,000 synthetic events/150,000 relationships, warm construction,
11 alternating-order pairs per shape. Full equality and destruction stay
outside clocks. No latency threshold, skipped outlier or canonical claim.

| Run / shape | Original mean ms | Candidate mean ms | Change |
|---|---|---|---|
| 1 duplicate-heavy | 67.253 | 22.222 | about 67% lower |
| 2 duplicate-heavy | 114.579 | 46.184 | about 60% lower |
| 1 mostly-unique | 216.957 | 219.883 | about 1.35% higher |
| 2 mostly-unique | 156.309 | 162.753 | about 4.12% higher |

Totals original/candidate were 739.780/244.439 and 1260.372/508.028 ms for
duplicate-heavy, 2386.531/2418.713 and 1719.400/1790.278 ms for mostly-unique.
Variation is retained, not attributed to host conditions. End-to-end canonical
10k/1M measurements must determine product relevance before acceptance.
All-target/all-feature release Clippy passed with `-D warnings`, 52.43 s.
Normal release lib/bins without features also passed Clippy with `-D warnings`,
13.10 s. Rustfmt passed. Fresh integration build completed in 9m24s:
`cargo test --release --locked --features devtools --test investigation_service --test investigation_stage_profile --test perf_queries -- --test-threads=1`.
Service 6/6 in 1.23 s; profile cleanup 3/3 in 0.12 s and query cleanup 3/3
in 0.13 s. Expensive diagnostic/budget tests were ignored in this normal run.
Coverage percentages were not measured.

## Candidate canonical 10k diagnostic

Exact ignored profile with rows=10000 and stage timing opt-in=1: 1 passed,
3 filtered, 2.65 s. The unchanged canonical 10k/3k-entity/1,500-related-row/
800-burst/40-checkpoint/9,800-snapshot/20-investigation/90-day shape passed;
all six actual-service calls were scored and persisted. Warm bound recurrence
again used its covering index, with 5,000 completed queries, then capture cleared.
Owned fixture cleanup passed.

Seed wall 1995.778 ms; inner total 1390.179 ms, insertion 823.331 ms.
Five measured e2e calls: total 443.896, mean 88.779, min/max 84.621/93.858 ms.
Graph exclusive range 6.612–8.663 ms, recurrence 11.127–11.894 ms.
The earlier attribution revision's mean was 69.999 ms. Other stages also
changed between processes; do not attribute that difference solely to this
builder or infer a performance pass/regression from five diagnostic samples.
The unchanged acceptance matrix is recorded separately below as it completes.

## Candidate canonical 1M diagnostic

Exact ignored profile with rows=1000000 and stage timing opt-in=1: 1 passed,
3 filtered, 346.49 s. Canonical counts were 1M events, 3,000 entities,
150,000 related rows, 80,000 bursts, 40 checkpoints, 9,800 snapshots,
20 investigations and 90 days. All six actual-service calls scored and
persisted; warm capture used the covering recurrence index with 5,000
completed queries. Owned fixture cleanup restored D: to 2,989,727,744 free bytes.

Seed wall **330,657.000/240,000 ms — MISS**; inner total 330,617.349 ms:
generation 7,583.951, insertion 307,262.632, graph 13,481.516, inventory
43.703, checkpoints 93.360 and investigations 2,106.451 ms.
Five measured e2e calls: total **4,323.977**, mean **864.795**, min/max
**843.590/892.092 ms**. Exclusive graph construction 148.170–162.734 ms;
recurrence 368.977–401.428 ms. Inclusive service totals were 843.484,
891.923, 859.374, 877.365 and 851.223 ms (e2e also includes caller overhead).

Graph cost is lower than the preceding attribution run, but other stages
also changed between processes. Do not attribute the whole e2e difference
to graph or substitute these five diagnostic samples for acceptance p95.

## Source and unit artifact SHA-256

Paths relative to `src-tauri/`:

These are artifacts at the graph-experiment revision before the subsequent
test-only 30-call diagnostic extension. The profile executable path is reused
by Cargo; its newer hash is recorded in the paired preflight/tail reports.

| File | SHA-256 |
|---|---|
| src/analysis/graph.rs (baseline) | CDEB7A58AC78B17BDA46839C6EAB412B1957E05BC57F1CA3214A97351530508E |
| src/analysis/graph.rs (candidate) | DAF84DD922DA45E4E4DCF1A2323C2E5AACBF3435F11F50EA590E867BF57D7C98 |
| src/analysis/graph_builder_tests.rs | C3FFEB1E103C6C98BAFCFA83D5C332AC62178FA3960C1D81FD65BBD5491188E7 |
| src/analysis/graph_builder_benchmark.rs | CCD08509BD4CA3846A9616CFACECA085E8FF851382A668FE8081CE76CAAA4DC9 |
| target/release/deps/what_changed_lib-708106f6a2e3e24a.exe | 48C02359A0E96A5AAB374C23AAC29B02368D82908576D6961DA99BC49C453A08 |
| target/release/deps/investigation_stage_profile-bd8cbe01cef57f37.exe | 04F6B4B0B0011D759ACDBC5E1A262169BAADC824248A586F17C303EFBDF79FA6 |
| target/release/deps/perf_queries-5447f552e83eb02d.exe | E67127DE0BED28F740CF02EBF98089F80F5FB6A7824FCAA83D02850D35FC23B3 |
| target/release/seed_history.exe | CFC0A3F215980BBAA335FF1AF3E8F293B09ED6C402248E0363CCD45D1995BB20 |

## Open gates

The [fresh profiler-free matrix](qa-261004-graph-candidate-query-matrix.md)
passes 10k/100k 8/8 each but remains 1M 7/8: investigation
1629.336/1500 ms FAIL. The preceding 2403.553/1500 ms failure is historical,
not a controlled graph-only comparison. Seed remains above 240 seconds.
Recurrence remains a larger measured owner
than graph construction. No phase checkbox, public-v1 verdict or release
artifact approval is promoted by isolated performance or equality alone.

Unresolved questions: 1M query and seed budgets are unmet; paired tail
attribution is next. Mostly-unique slowdown and app working-set impact remain
separate from canonical latency evidence.
