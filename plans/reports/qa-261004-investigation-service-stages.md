# Actual investigation-service diagnostics — 2026-10-04

Outcome: instrumentation unit tests 2/2 and existing service integration 6/6
PASS. A fresh canonical 10k diagnostic passed one warmup plus five fully scored,
persisted investigations. This is attribution evidence, not an acceptance p95
or an installed release-candidate verdict. The latest profiler-free 1M query
gate remains FAIL at 2403.553/1500 ms.

## Source contract

`investigation_service.rs` wraps its existing executed stages without changing
SQL, query order, transaction/commit boundaries, 5,000 cap, scoring inputs,
sanitization or persistence. Independent read-only review found no actionable
defect. The helper is empty in a normal non-devtools build. Devtools requires
`WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS=1`; the disabled path has no active
clock/record allocation after the cached opt-in probe.

Records contain static stage labels, status and durations only, with one locked
stderr write per request. They contain no problem, path, entity or event data.
Exclusive stage durations sum to inclusive total; input includes reader wait
and read commit. Errors stay assigned to their current stage and are not marked
complete. The dedicated profile requires explicit opt-in, proves persisted id,
exactly 5,000 total/analyzed events, nonempty suspects and no no-cause fallback.
It uses existing owned `PerfFixture` cleanup, never real AppData.

## Commands and first 10k diagnostic

From `src-tauri`, release/locked, `CARGO_BUILD_JOBS=1`, TEMP/TMP under the owned
workspace `.perf-temp` parent:

```powershell
cargo test --release --locked --features devtools --lib services::investigation_service::stage_timings::enabled::tests
cargo test --release --locked --features devtools --test investigation_service
$env:WHAT_CHANGED_PERF_ROWS='10000'
$env:WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS='1'
cargo test --release --locked --features devtools --test investigation_stage_profile five_persisted_investigations_report_actual_stage_timings -- --ignored --exact --nocapture
```

Unit: 2 passed, 458 filtered; integration: 6 passed, 0 failures/ignores, 1.44 s.
Subsequent release all-feature library suite passed 459/460, zero failures,
one hardware-performance test ignored, 14.95 s. Rustfmt check passed.
All-target/all-feature release Clippy passed in 1m19s. The normal-release
no-default-features check then found an existing `AutomaticCheckpointHandle::noop`
compiled without its sole `collectors::testing` caller. Mandatory counsel
confirmed a configuration mismatch; the helper now uses the identical
`any(test, debug_assertions, feature = "devtools")` guard. No warning suppression
or runtime behavior change was applied. The broad no-default-features retry
then failed because release integration tests import the deliberately absent
`testing` namespace (`redaction_leak.rs:32`). Counsel corrected the matrix:
`lib.rs:28–32` explicitly requires devtools for release test helpers and the
canonical lint/test scripts use all features. No test was gated away and no
testing namespace was exposed in production. Production lib/bins without
features and all targets with all features are the complementary checks.
Both corrected release/locked Clippy checks passed: normal lib/bins in 1.56 s,
all-target/all-feature in 1m42s. Rustfmt passed after the guard change.
Diagnostic: 1 passed, 3 filtered, 7.15 s; fixture cleanup succeeded. Canonical
10k: 3,000 entities, 1,500 relationships, 800 burst rows, 40 checkpoints,
9,800 snapshots, 20 investigations, 90 days; seed wall time 5514.251 ms.
Five measured end-to-end calls: total 1101.753 ms, mean 220.351 ms, minimum
183.262 ms, maximum 267.237 ms. One warm call is excluded from that aggregate.

| Exclusive stage | Five-call range, ms |
|---|---:|
| Request setup | 0.012–0.019 |
| Window hydration | 68.415–100.432 |
| Boundary queries | 1.464–2.579 |
| Entity load | 7.511–14.078 |
| Relationships | 5.833–10.454 |
| Recurrence | 25.421–33.193 |
| Input finalization | 0.262–0.508 |
| Analysis scoring | 59.675–104.165 |
| Result sanitization | 0.783–1.289 |
| Persistence | 0.887–1.540 |

Initial aggregate stdout interleaved with the final stderr record in the tool
output. The test aggregate was moved to stderr too; behavior and timing fields
are unchanged. The fresh post-guard run passed below.

Initial source SHA-256:

| Owner | SHA-256 |
|---|---|
| Service | `70496cae3c2b827469e3c482add4db2085219fcc0764caac6c4f275d2485f562` |
| Timing facade | `ee24c90c94867c32ddd167c0a7a3994632617a10506acaead73c154c12bf86fe` |
| Enabled timing | `228d890f0669378a0ee8cb79b128bf8ee866c8d3bb1a25c16ce46428ae536446` |
| Profile before stderr change | `ad5088e15719abe726b59e77a423aeea0c837d59f02733f7a6d1c3986008d960` |

## Fresh stderr-only 10k verification

The same diagnostic command, after the matching helper guard and aggregate
stderr change, passed **1/1**, 3 filtered, 0 failed/ignored, **3.54 s**. All six
records (one warm and five measured) are intact and `status=complete`; each
call passed the full persisted/scored assertions. Exact canonical counts are
unchanged. Fixture shutdown/cleanup succeeded, leaving no owned perf directory.
Seed wall time: **2889.662 ms**; setup stages (ms): generation 68.560, insertion
1259.769, graph 84.538, inventory 260.576, checkpoints 173.895,
investigations 200.449, total 2140.082.
Measured e2e: total **414.927 ms**, mean **82.985 ms**, min **80.224 ms**,
max **85.163 ms**. These five-call values are not an acceptance p95.

| Exclusive stage | Fresh five-call range, ms |
|---|---:|
| Window hydration | 31.264–34.240 |
| Boundary queries | 0.751–0.838 |
| Entity load | 3.792–4.078 |
| Relationships | 2.567–2.840 |
| Recurrence | 10.421–11.211 |
| Analysis scoring | 29.901–30.985 |
| Persistence | 0.430–0.722 |

SHA-256: profile source after stderr change
`3c636df56a55cafdb16121479b86158874d4c59e53467e4505fb2736e80a9c2a`;
profile executable `e1c05d3af734f187c2320a5eb796fdda9d00ff0d90b46d249bc4199ab548439f`;
seeder executable `906cd38a13d305a3447f02d7359176649fee3991636ad86b7314c14c2455736c`.

## Fresh 1M actual-service attribution

The same target with `WHAT_CHANGED_PERF_ROWS=1000000` passed **1/1**,
3 filtered, no failures/ignores, **451.59 s**. The exact canonical corpus was
verified: 1M events, 3,000 entities, 150,000 related rows/edges, 80,000 bursts,
40 checkpoints, 9,800 snapshots, 20 investigations and 90 days. All six actual
calls passed persistence/scoring assertions; six static records were complete.
Owned fixture shutdown/deletion succeeded. Executables matched the fresh 10k
hashes above; no source or budget change occurred between the two runs.

Seed wall time **429057.071 ms / 240000 ms — MISS**. Setup stages (ms):
generation 7394.021, insertion **400583.754**, graph 18140.220, inventory
82.854, checkpoints 166.337, investigations 2470.544, total 428964.031.
Insertion accounts for approximately **93.4%** of measured setup, without
attributing that cost to any individual statement, trigger or allocation.

Warm service total: 1090.518 ms. Five measured e2e calls: total **7234.129 ms**,
mean **1446.826 ms**, minimum **1000.119 ms**, maximum **2567.529 ms**.
Input inclusive ranged **586.939–1276.950 ms**. These are diagnostic aggregates,
not a 30-sample p95 gate; the first measured call spiked across input and analysis.

| Exclusive stage | Five-call range, ms |
|---|---:|
| Request setup | 0.007–0.015 |
| Window hydration | 47.327–119.247 |
| Boundary queries | 0.870–2.024 |
| Entity load | 4.594–9.312 |
| Relationships | 118.962–294.504 |
| Recurrence | **414.322–874.177** |
| Input finalization | 0.167–0.350 |
| Analysis scoring | **411.929–1287.581** |
| Result sanitization | 0.453–1.121 |
| Persistence | 0.628–1.468 |

Recurrence and analysis are the largest measured contributors. No production
SQL/writer/scoring optimization, sample removal, waiver or acceptance promotion
was made from these five calls. The latest profiler-free 1M acceptance p95
remains **2403.553/1500 ms — FAIL**, measured before this instrumentation.

## Query-plan scout: criterion remains open

Read-only source tracing found all eight current plan shapes use the legacy
10k seed-42 fixture and handmade identical checkpoints, not the canonical
seeder/AppState fixture. Only page SQL/binds are supplied by the existing
production `explain_page_plan`; other shapes copy SQL, sometimes with reduced
projections. Dashboard's copied right edge uses `<`, while production uses
`<=`; actual summary is a combined query, not three separate explained queries.
Investigation name predicates, selected relationship/recurrence branch, details
related ids, search hydration, checkpoint changed pair and privacy settings
also need actual execution alignment. Predicate similarity is not proof of
covering-index or bound-plan equivalence.

Source-owned SQL/bind construction or execution-point explain hooks, together
with the eight real calls on canonical fixtures, are the strict closure path.
Native SQLite tracing can prove executed SQL but expanded literals may change
planning; it cannot alone prove original parameterized plans. Any trace must
be devtools-only, scoped to the leased reader, reset on success/error/unwind,
metadata-free in output, and absent from timing runs. No trace feature, SQL or
production writer change was made in this scout.

## Remaining

Fresh output verification and complementary feature/nonfeature checks passed.
1M actual-service attribution is recorded; failed 1M query/seed repair and
source-aligned query-plan evidence remain open. Keep all budgets and canonical
counts unchanged.
Mandatory counsel recommends finer aggregate stages in the actual analysis path
and a source-owned parameterized plan for the executed sorted recurrence branch,
captured outside measured requests. Preserve inclusive-since/exclusive-before
and 150-query duplicate-ID aggregation/overwrite semantics. Record contention
alongside the next run; do not discard the slow sample or infer a host cause.
Choose one measured owner to optimize, then rerun the unchanged profiler-free
30-sample 1M gate. Writer substage attribution remains separate work.
