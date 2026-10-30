# Current-source retention and pending native evidence — 2026-10-04

Outcome: canonical 1M routine retention PASS; eight-query 10k/100k PASS and
current 1M query gate FAIL (7/8 budgets); release
all-target/all-feature Clippy and the 457-test library suite PASS.
Pending native evidence is prepared, not approved for redistribution. Public
v1 remains NO-GO. No user history, reviewed manifest or installer was changed.

## Scope and state

- PowerShell, Windows x64; release profile, locked Cargo inputs, one Cargo job
  at a time. Disposable corpora use the workspace-owned `.perf-temp` parent.
- Fresh anchored 25-phase scan: 312/369 complete, 57 open; 17 completed,
  7 in-progress, 1 pending. No checkbox or phase-status promotion in this run.
- User's capture-test choice is already implemented: an expired absolute
  deadline exercises queued and running timeout states without racing a 25 ms
  worker-scheduling window. Production deadline behavior is unchanged.
- Fresh release all-feature library run: 457 passed, 0 failed, 1 ignored,
  458 total, 23.03 s. The ignored hardware-performance gate was not run here.

## Canonical 1M routine retention

Command, from `src-tauri`, with `CARGO_BUILD_JOBS=1` and workspace TEMP/TMP:

```powershell
$env:WHAT_CHANGED_RETENTION_ROWS='1000000'
cargo test --release --locked --features devtools --test maintenance_reclaim canonical_retention_pass_incrementally_reclaims_pages -- --include-ignored --nocapture
```

- Terminal result: 1 passed, 0 failed, 0 ignored, 4 filtered; 1008.62 s.
- Seed: **955100.992 ms**, unchanged 1M / 90-day canonical corpus: 3000
  entities, 150000 related events and durable edges, 80000 burst rows,
  40 checkpoints, 9800 snapshots and 20 investigations.
- First pass: eligible/deleted **157304**, remaining **842696**, **317**
  batches, **34745 ms**; page count **194187 → 193931**, freelist
  **0 → 18058**. Important rows retain their longer lifetime.
- Follow-up: zero additional event/snapshot/investigation deletes; 3 batches,
  **35 ms**; page count **193931 → 193675**, freelist **18058 → 17802**.
  The bounded reclaim drained exactly **256** free pages, not the whole list.
- Fixture shutdown and exact owned-directory cleanup succeeded.
- **Seed target MISS: 955.101 s / 240 s.** This is not a retention-test failure
  and is not waived. Full compaction and the query matrix are separate gates.

Source/artifact SHA-256 for this run:

| Owner | SHA-256 |
|---|---|
| `src/bin/seed_history.rs` | `46e6a936ba886a4fc6c7daa3febfd59b8ddeec067b3548dd9ec65cb23280e624` |
| `tests/maintenance_reclaim.rs` | `6beff1a2002162e1e3c7bc6fde677a11e67078d1d083029018500e9f19f5f2ac` |
| `tests/support/perf_fixture.rs` before timing forwarding | `e2365bdef95ccb8ce59f1509fa84dfd25cc34563ff7330939843700769feb3f4` |
| `maintenance_reclaim-968b2bc0a817414f.exe` | `81dac03e0b8eebafe3209b549d33a0f5135aae33486ffff37f81ba6b7ed790c1` |
| `seed_history.exe` | `f87cdf29fc68fbd17f2a50ec1f628924ea43220c2668df5efd8e9e6c18948b03` |

Prior 2026-10-02 canonical 100k retention also passed: eligible/deleted 15732,
84268 remaining, 34 batches, 7542 ms; pages 20136 → 19880, freelist 0 → 1399.
Follow-up deleted zero events and drained 256 free pages in 73 ms. These two
runs extend the earlier 10k routine-retention evidence; they do not close the
combined size/sensitivity or installed resource criteria.

## Verification and setup profiling

`cargo clippy --release --locked --all-targets --all-features -- -D warnings`
completed with exit 0 in 4m42s. This precedes two test-only changes: the shared
fixture forwards only the aggregate `seed timings:` line, and the query warmup
now requires no no-cause shortcut and nonempty scored suspects. Neither change
alters production logic, the seeded distribution, sample count or budgets.
After both changes, the same Clippy command passed again with exit 0 in
56.79 s. Rustfmt and scoped whitespace checks passed. The exact capture
deadline test passed 1/1, 457 filtered out, before the full 457-pass library
run. This is release-profile validation, not a clean-target/default `bun verify`
or installed release-candidate verdict.

After those changes, `WHAT_CHANGED_PERF_ROWS=10000 cargo test --release
--locked --features devtools --test perf_queries -- --include-ignored
--nocapture` passed **4/4** (eight-query gate plus three fixture lifecycle
tests), 0 failed/ignored, 8.42 s runtime. Thirty warm samples per cell:

| Query | p95 ms | Budget ms |
|---|---:|---:|
| First page | 1.198 | 10 |
| Page 10 keyset | 1.227 | 10 |
| Dashboard | 0.348 | 15 |
| Two-token search | 1.188 | 40 |
| Details + recurrence | 0.095 | 10 |
| Persisted 5k investigation | 234.212 | 400 |
| 3k checkpoint comparison | 25.604 | 150 |
| Privacy summary | 0.621 | 30 |

10k seed wall time was 2132.191 ms. Seeder stages (ms): batch generation
97.939, event insertion 1264.038, graph 38.878, inventory 60.699, checkpoints
158.528, investigations 376.082; total run 2062.843. This small profile is
diagnostic; do not extrapolate a 1M acceptance result. The historical pre-BFS
1M investigation failure was 1758.975/1500 ms; the fresh result below supersedes it.

The subsequent identical-command **100k** run passed **4/4**, no failures or
ignores, 29.60 s runtime. All eight 30-sample warm p95 values (ms): first page
1.266/20, page 10 1.104/20, dashboard 0.975/30, search 5.581/90, details
0.132/15, persisted investigation 220.788/600, checkpoint comparison
26.851/200, privacy 0.782/60. Seed wall time: **20575.794 ms**; stage times
(ms): generation 758.553, event insertion **18380.084**, graph 673.678,
inventory 28.835, checkpoints 122.156, investigations 513.424; total run
20522.374. Event insertion accounts for about **89.6%** of this measured
100k setup time. Static writer tracing identifies repeated cached entity
statement checkout and JSON allocations, but does not establish their cost.
Both FTS insert triggers, search ordering, daily/category counts and port JSON
evidence must remain active. No production writer change was made on this basis.

The subsequent serial **1M current-source** gate completed, exit 1: three
fixture lifecycle tests passed and the eight-query budget test failed; 553.50 s
runtime. Exact unchanged canonical counts were verified. Seed stages (ms):
generation 11745.270, event insertion **455443.520**, graph 19510.494,
inventory 46.656, checkpoints 109.318, investigations 2437.063; total run
489378.189, seed wall time **489448.246 ms / 240000 ms target — MISS**.
Insertion accounts for about **93.1%** of measured setup time. This local
measurement is slower than linear scaling from 100k, without proving whether
scale-dependent work or host contention caused the difference.

| 1M query | p95 ms | Budget ms | Result |
|---|---:|---:|---|
| First page | 0.828 | 45 | PASS |
| Page 10 keyset | 0.722 | 45 | PASS |
| Dashboard | 3.451 | 80 | PASS |
| Two-token search | 55.788 | 250 | PASS |
| Details + recurrence | 0.432 | 30 | PASS |
| Persisted 5k investigation | **2403.553** | **1500** | **FAIL** |
| 3k checkpoint comparison | 35.965 | 300 | PASS |
| Privacy summary | 1.122 | 200 | PASS |

The stronger real-scoring warmup passed before measurement. The full fixture
closed and was deleted during failure unwind; workspace free space returned
to approximately 3.76 GB. The earlier pre-BFS 1758.975 ms failure is historical;
**2403.553/1500 ms is the latest current-source result**. Neither the pure 5k
analysis benchmark nor the 10k/100k passes substitute for this failed cell.

The measured query executable SHA-256 was
`1534593978f64beed71de90bd405e2be9b2ea4e56f8be572fd2da92c399569bf`;
the seeder executable still matched the earlier
`f87cdf29fc68fbd17f2a50ec1f628924ea43220c2668df5efd8e9e6c18948b03`.
These results precede any subsequent service-stage instrumentation.

## Pending native evidence

- Actual `bun run prepare:third-party-review` completed: **232 packages / 399
  original source files**, pending, `publishable=false`; 13 entries flagged
  (WebView2 wrapper entry now has both missing full text and native review).
- Draft SHA-256: `3703e5681304948b4c449a7d0b2438c502a2d575930f94a6a741535dd128c95d`.
- WebView2Loader has three exact x64 candidate hashes and two selection-source
  hashes. SDK version/license are null; native obligations remain unreviewed.
  Native binaries are hashed in place, not copied as license texts.
- Root combined Node verification passed **34/34**, no failures/skips;
  independent read-only review passed 28/28 scoped tests, no actionable defect.
  Kongming confirmed the pending-only scope and recommended measured 100k
  seeding stages before another expensive 1M run.
- This proves local packaged candidates, not actual linked bytes, native SDK
  redistribution terms, notice installation or frozen installer coverage.

The subsequently implemented native notice gate passed **96/96** combined
Node tests, with independent read-only review also passing 96/96. SQLite and
WebView2 require their own reviewed native records, contained original license/
NOTICE bytes, exact candidate and selection-source hashes, Cargo manifest SHA
binding and parsed HTTPS provenance/source routes. Wrapper approval alone no
longer passes. A nullable native version requires explicit provenance review.
Malformed HTTPS URLs and stale native bytes fail before replacing output.
`node scripts/generate-third-party-notices.mjs --check` still exits 1 at the
pending-manifest guard, as intended; no legal or publication approval was added.
The optional simplifier delegate could not start due to thread capacity; root
used shared URL validation and scoped module extraction, independently reviewed.
Documentation link checks passed for 33 Markdown files.

## Remaining work / unresolved questions

No new user choice is needed for these measurements. Keep the 240 s seed target
and all query budgets unchanged. Detailed insertion and persisted-investigation
stage attribution, the failed 1M seed/query gates, copied-query-plan parity,
collector-shaped size miss, installed
resource/soak/accessibility/upgrade gates, full native license review and final
artifact-bound publication approval remain open. See the owning phase files;
this report does not replace their acceptance criteria.
