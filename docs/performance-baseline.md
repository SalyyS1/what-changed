# Performance evidence

**Evidence state: partial current-source local measurements; not a complete or installed release-candidate pass.**

The [performance phase record](../plans/260829-0437-what-changed-desktop-mvp/phase-20-performance-accessibility-hardening.md)
owns the accepted budgets and chronological gate results. The
[measurement ledger](PERFORMANCE.md) retains the dated query values. This page
routes readers to executable owners and states what that evidence does **not**
prove. Public reports must not include a hostname, username, serial number,
executable path or event payload.

## Recorded scope

The 2026-09-26 local 10k/100k/1M query and database-size/reclaim gates passed
on their recorded synthetic fixtures. The 1M seed-time target was missed; see
the phase record for the exact values and build profile. These runs preceded
[migration 014](../src-tauri/migrations/014_daily_importance_first_day_boundary.sql)
and later repository/capture changes. They are historical measurements, not
proof that the present source or an installed release candidate meets the
same budgets.

## Open performance evidence

- The corrected current-source 10k release-profile corpus passes all eight query
  budgets; Investigation p95 is 144.212 ms against 400 ms. The 10k database
  size is 11,599,872 bytes against 12,000,000. The earlier 7/8 debug result
  used a pre-density, non-clustered fixture and is historical only. The 100k
  final-corpus database-size gate passes at 82,464,768/110,000,000 bytes, but
  investigation initially missed its query budget at 896.470/600 ms. A batched
  indexed recurrence change first reduced that 100k gate reading to
  784.607/600 ms (still 7/8). A later low-load run of the same code/corpus
  passed 8/8 with investigation at 317.194/600 ms; another diagnostic run
  under higher host load had measured 786.441 ms. This variability needs a
  controlled release-candidate rerun. The corrected 1M query matrix then
  passed only 6/8: investigation 11,187.263/1,500 ms and privacy summary
  262.719/200 ms missed. The corrected 1M size/full-compaction cell passed
  before migration 015 (795,377,664/1,100,000,000 bytes). A v15 stage probe found
  `FailureIndex` and relationship loading dominate the investigation; its
  category-count projection read was 0.539 ms p95. A subsequent optimized
  v15 100k release run passed 8/8 with investigation 213.908/600 ms and
  privacy summary 0.627/60 ms. The first optimized v15 1M run passed 7/8;
  investigation remained over budget at 2,028.316/1,500 ms p95, while
  privacy summary passed at 0.614/200 ms. Its seed took 337.172 s against
  the four-minute operational target. A separate five-sample 1M diagnostic
  found 1,189.717 ms p95 in data loading (896.665 ms recurrence counts,
  266.224 ms relationships) and 425.993 ms p95 in analysis; it is not an
  acceptance-gate substitute. After batching the synthetic graph inserts in
  one QA transaction, the v15 1M size/full-compaction rerun passed at
  **795,389,952/1,100,000,000 bytes** (WAL 90,594,712 bytes); tagged purge
  removed 1M events and explicit `VACUUM` reduced pages 194,187→43,825 with
  zero freelist. Seed time improved to **285.559 s**, still above 240 s. A
  subsequent same-corpus diagnostic measured recurrence p95 at **612.954 ms**
  for a whole-index stream, **921.942 ms** for unsorted direct counts, and
  **388.213 ms** for signature-sorted direct counts. The sorted exact path
  and a duplicate-free dense relationship scan then passed the canonical v15
  **1M eight-query release gate 8/8**: investigation **1,151.732/1,500 ms**
  p95 (30 warm samples); the other seven shapes ranged **0.514–44.031 ms**
  and met their respective budgets. Seed time was **270.024 s**, still above
  the 240 s operational target. The subsequent profiler-free 10k and 100k
  release gates passed 8/8; the 1M rerun under higher host load passed 7/8,
  with investigation **1,758.975/1,500 ms p95**. Its seed took **340.589 s**.
  At that historical checkpoint the 1M acceptance gate remained open despite
  the earlier pass; the subsequent guarded-recurrence matrix below supersedes it.
  Rebuilding both FTS indexes during the exact QA purge reduced the current
  v15 1M post-purge/full-`VACUUM` database from the prior **179,507,200**
  byte residual to **741,376 bytes**; pre-purge size remained **795,389,952**
  bytes and the freelist ended at zero. A focused mixed real/synthetic fixture
  confirms surviving real events remain searchable. The
  opt-in assertions run from [`package.json`](../package.json) and live in
  [`perf_queries.rs`](../src-tauri/tests/perf_queries.rs),
  [`perf_query_plans.rs`](../src-tauri/tests/perf_query_plans.rs) and
  [`perf_db_size.rs`](../src-tauri/tests/perf_db_size.rs).
  This exact purge cannot safely identify a QA graph edge after routine
  retention has removed its source event and the foreign key has become NULL;
  it conservatively leaves that unattributable edge rather than risk deleting
  a real edge with the same endpoints. Full-compaction size results use fresh
  fixtures where tagged purge runs before retention.
- An earlier current-source canonical 1M BFS query matrix, before the graph-candidate
  revision, completed with **7/8** budgets:
  first page **0.828/45 ms**, page 10 **0.722/45 ms**, summary **3.451/80 ms**,
  search **55.788/250 ms**, details **0.432/30 ms**, checkpoint comparison
  **35.965/300 ms** and privacy summary **1.122/200 ms** passed; persisted
  investigation was **2,403.553/1,500 ms p95 — fail**; three fixture tests passed
  and the single budget test failed. The exact canonical
  fixture used 3,000 entities, 150,000 related events/edges, 80,000 burst rows,
  40 checkpoints, 9,800 snapshots and 20 investigations; the stronger scoring
  warmup passed and cleanup confirmed 3.76 GB free on the disposable D: volume.
  Seed time was **489,448.246 ms** against **240,000 ms**; stages were generation
  11,745.270 ms, insertion **455,443.520 ms** (~93.1%), graph 19,510.494 ms,
  inventory 46.656 ms, checkpoints 109.318 ms, investigations 2,437.063 ms,
  total 489,378.189 ms. This is historical evidence for that earlier revision;
  the latest profiler-free [guarded-recurrence query matrix](../plans/reports/qa-261005-recurrence-rolling-query-matrix.md)
  is the latest recorded query-acceptance checkpoint: all eight shapes passed
  at all three scales on its frozen source/artifacts. The later page capture
  and concrete bind storage changes have correctness checks, not fresh latency
  measurements; current-source budget refresh remains a release prerequisite.
  The independent 1M seed-time target remains missed. Full earlier-run cells are in the [dated QA
  evidence report](../plans/reports/qa-261004-retention-native-evidence.md).
- Current-source routine-retention gates pass at 10k, 100k and 1M via
  [`maintenance_reclaim.rs`](../src-tauri/tests/maintenance_reclaim.rs). The
  10k gate deleted 1,575 eligible rows, reduced `page_count` 2,832→2,691 and
  left `freelist_count=0` in 148 ms. The prior 2026-10-02 100k gate deleted
  15,732 rows, left 84,268, moved pages 20,136→19,880 (freelist 0→1,399),
  and reclaimed 256 pages in 73 ms on its follow-up. The
  2026-10-04 canonical 1M gate deleted 157,304 rows, left 842,696, and used
  317 bounded batches in 34,745 ms; its follow-up deleted no additional rows
  and reclaimed exactly 256 pages in 35 ms on the follow-up: first-pass pages
  194,187→193,931 (freelist 0→18,058), then 193,931→193,675 (freelist
  18,058→17,802). These runs exercise bounded incremental reclaim, not full
  `VACUUM`; the 1M seed took 955.101 s against the 240 s operational target,
  so that target remains missed. Full details are in the dated
  [retention evidence report](../plans/reports/qa-261004-retention-native-evidence.md).
- The canonical 10k size corpus passed at 11,599,872/12,000,000 bytes. A
  separate opt-in 10k collector-shaped payload sensitivity test retained
  application/process/network/service/system field shapes and measured
  **12,042,240/12,000,000 bytes**, 42,240 bytes over the same strict budget
  (852,714 bytes of persisted JSON). This diagnostic does not replace the
  canonical gate or generalize to private production history. Dropping the
  redundant relationship index would trigger a non-additive upgrade backup
  for existing databases; that migration cost was not justified for this
  42 KB sensitivity miss, so no schema or threshold change was made.
- The v13→v14 small migration fixture in
  [`migrations.rs`](../src-tauri/tests/migrations.rs) passes. The 1M upgrade's elapsed
  time, temporary/backup free-space cost and installed two-version path have
  not been verified.
- The wider read-path timings, 100k timeline, frame, accessibility
  and installed-resource gates in the phase record remain open. A 10k eight-shape
  query pass does not imply a pass for those other paths or corpus sizes.

## Investigation stage diagnostics

The actual-service attribution route is owned by
[`investigation_service.rs`](../src-tauri/src/services/investigation_service.rs),
the timing facade
[`investigation_stage_timings.rs`](../src-tauri/src/services/investigation_stage_timings.rs),
the seven-stage analysis timing facade
[`analysis/stage_timings.rs`](../src-tauri/src/analysis/stage_timings.rs),
the scoped recurrence-plan capture
[`recurrence_plan_capture.rs`](../src-tauri/src/storage/recurrence_plan_capture.rs)
at the sorted execution in
[`event_repository.rs`](../src-tauri/src/storage/event_repository.rs), and its ignored profile target
[`investigation_stage_profile.rs`](../src-tauri/tests/investigation_stage_profile.rs).
The devtools opt-in `WHAT_CHANGED_INVESTIGATION_STAGE_TIMINGS=1` is cached once
per process. In normal builds the timing facades have no clocks, active state,
environment reads or stderr output; when enabled, they emit only static stage
labels, status and durations. The seven aggregate analysis stages and the
existing service-stage timings wrap execution without changing SQL, query order,
transaction boundaries, the 5,000-event cap, scoring, sanitization, persistence,
providers or batching.

For at least 2,000 inputs inside an existing transaction, the
[rolling-count helper](../src-tauri/src/storage/recurrence_rolling_counts.rs)
reuses exact counts across eligible adjacent windows. Its widened span guard
is a cost heuristic, not a row-density or no-regression guarantee. Small inputs
and autocommit keep their original paths; no implicit transaction is introduced.
SQL, half-open intervals and original 150-input duplicate-ID aggregation remain
unchanged. Recorded guarded-recurrence correctness checks and all three
canonical query matrices pass at their linked revision's source/artifact
checkpoint; that ledger owns their evidence, not a subsequent page-source pass.

The devtools-only recurrence capture explains the first full COUNT and, when
executed, the first delta COUNT at their actual execution points, using the same
SQLite connection, SQL constant and borrowed range bindings. The
[execution hooks](../src-tauri/src/storage/recurrence_execution_capture.rs)
separate successful full/delta SQL executions from resolved full/rolled/reused
inputs: SQL totals need not equal input totals. They issue no extra COUNTs.
Evidence retains no bind values or raw database errors. Capture is synchronous,
thread-bound, nonnested and RAII-cleared on return, error or panic; a second
sorted invocation is rejected rather than misattributed. The ignored
[profile targets](../src-tauri/tests/investigation_stage_profile.rs) retain the
original five-call profile and add a separately selected exact 30-call paired
profile. The warm actual-service call is bound to the capture only and excluded
from the measured calls; capture is then cleared before measurement. Each
synchronous measured service call has one completed engine record and one
completed service record. Static begin/end markers are
outside the outer e2e clock, while stage timing stderr is inside it, so these
profiles are diagnostic attribution rather than acceptance evidence. See the
[paired preflight](../plans/reports/qa-261004-paired-investigation-preflight.md)
and [paired tail report](../plans/reports/qa-261004-paired-investigation-tail.md)
for the earlier recorded contract and observations. Selected full/delta plans
do not represent all queries or establish the eight-shape plan gate.

The active [recurrence-window statistics module](../src-tauri/src/storage/recurrence_window_statistics.rs)
adds eight bounded integer counters while borrowing the existing sorted inputs.
It retains no keys, bounds or event IDs; inactive capture performs no additional
walk or allocation. The warm statistics are printed outside measured requests,
while returned mapping and duplicate-ID aggregation remain unchanged. These
input statistics are separate from actual execution counters. The
[recurrence-window reuse report](../plans/reports/qa-261005-recurrence-window-reuse.md)
records the earlier measurement; the
[production-candidate record](../plans/reports/qa-261005-recurrence-rolling-production.md)
tracks the subsequent integration. Its unchanged canonical query matrices
passed all24 budget cells at that frozen checkpoint; synthetic paired gains
alone were not acceptance. Later page allocation/inactive-capture changes are
not an unchanged implementation or a remeasurement of those latency budgets.
Seed insertion, full SQL-plan branch parity and installed/resource gates remain
independent obligations.

The latest source evidence is recorded after the earlier
[service-stage report](../plans/reports/qa-261004-investigation-service-stages.md)
in the [analysis and recurrence attribution report](../plans/reports/qa-261004-analysis-and-recurrence-attribution.md)
and the production-candidate record above. Diagnostic contracts alone do not
establish acceptance. The separately executed profiler-free canonical matrix
passed at all three scales on its recorded artifacts; current-source latency
refresh, seed, full SQL-plan branch parity and other obligations remain open.

## Actual-bound page diagnostics

The explicit devtools-only
[`page capture`](../src-tauri/src/storage/page_plan_capture.rs) wraps one
synchronous actual [`page`](../src-tauri/src/storage/event_repository.rs) call.
The owner supplies its exact preparation SQL, connection and concrete borrowed
bindings to both diagnostic EQP and the real statement. Invocation, preparation,
EQP outcome, query start and full cursor/lookahead decoding remain distinct.
Evidence failures never replace the operation's result; scope clears on return,
error or panic. Inactive capture performs no EQP or capture allocation; normal
builds exclude capture. Ordinary page parameter allocation remains separate.

The [typed projection](../src-tauri/src/storage/page_plan_explain.rs) retains
node/parent IDs and recognized access/constraint/index/sort/OR structure, never
SQL, bindings, arbitrary schema names or raw errors. Unknown vocabulary yields
incomplete evidence rather than a shortened success. Review the page-specific
allowlist when SQLite changes; it is not a parser for other query families.

The separate ignored [canonical10k target](../src-tauri/tests/page_plan_capture_profile.rs)
captures actual service pages1/10 with eight intermediate calls, outside every
p95 measurement. Active EQP can perturb cache/locks; source and unit evidence
do not establish timing-neutral behavior, all-service plans or larger scales.
[Execution ledger](../plans/reports/qa-261005-page-plan-capture-ledger.md) owns
the verification state. One canonical10k actual page1/page10 smoke passes with
complete201-row decoding and indexed timestamp SEARCH; this covers two page
shapes only. Other filter branches/scales/owners and the full SQL-plan phase
gate remain open. This diagnostic is not fresh p95 acceptance.

## Actual-bound checkpoint event-window diagnostics

The devtools-only [window capture](../src-tauri/src/storage/checkpoint_window_plan_capture.rs)
observes only the actual [event_ids_in_window owner](../src-tauri/src/storage/checkpoint_repository.rs),
using the same connection, fixed SQL and normalized inclusive bind tuple as
the actual statement. Preparation, bind cardinality, query start, full decoding
and typed EQP outcome remain separate. The narrow events-range statement reuses
the pinned PAGE projection; unsupported vocabulary or EXPLAIN failure never
changes the actual result. No SQL, bind values, raw errors or IDs enter evidence.
Scope is thread-local and clears on return/error/unwind; inactive capture does
no EQP/diagnostic allocation, and normal builds exclude it.

The ignored [canonical10k target](../src-tauri/tests/checkpoint_window_plan_profile.rs)
passes once after gates/counsel: full inactive/captured compare(Some) equality,
245 fully decoded window IDs and complete covering timestamp-index SEARCH.
[Execution ledger](../plans/reports/qa-261005-checkpoint-window-capture-ledger.md)
owns hashes/output and proof limits. This is one subquery, not checkpoint
by_id/items reads, the whole comparison shape, other scales or p95 acceptance.

## Seed insertion diagnostics

The devtools seeder's separate `--insertion-timings` flag activates an explicitly
owned [batch recorder](../src-tauri/src/storage/insertion_timing_capture.rs)
inside the admitted writer job. Ordinary insertion APIs pass no recorder and
read no attribution clocks; normal builds exclude the module/API. Existing
`--timings` remains the independent coarse stage record and does not activate
insertion attribution. SQL, entity/event interleaving, transaction boundaries,
reconciliation and ownership checks are unchanged.

The aggregate separates begin/preparation, the combined event/entity loop,
reconciliation, commit and repository total. It retains only numeric
durations/counts, including incomplete phase work on returned errors. Failed
batches never claim committed input rows; those counts are not distinct
physical insertions when duplicate events merge. Repository total includes
helper-local destruction/rollback. No ambient state survives success/error/panic.
One fixed record is emitted outside insertion work after DB close. The outer
insertion-minus-repository residual is caller/writer overhead, not isolated
queue waiting. No per-row clocks, event fields, bindings or raw SQL are retained.

The separate devtools transaction-body entry in
[`event_repository`](../src-tauri/src/storage/event_repository.rs) requires an
existing `Transaction` and consumes entity ownership. It returns updated
ownership only on successful body processing; that state and its processed-row
counts remain provisional until the caller commits. Any body error requires
abandoning the entire outer transaction: the helper neither commits nor rolls
back pending writes. Optional `BodyInsertionTimings` has no committed counters
and excludes outer begin/commit/rollback; empty or disabled capture reads no
attribution clocks. Existing standalone insertion and its recorder keep their
original transaction and timing semantics. This helper is not a changed seeder
default or a seed-time acceptance result.

The separate ignored [diagnostic target](../src-tauri/tests/seed_insertion_profile.rs)
owns an exclusive disposable fixture and validates aggregate counts and corpus
span. It is attribution, not a seed-time or eight-query acceptance gate.
[Execution evidence and remaining work](../plans/reports/qa-261005-seed-insertion-attribution.md)
keep diagnostic results separate from the still-missed seed target.

A separate PROFILE experiment is rejected at the pinned rusqlite0.40.2 API
boundary: internal statements can emit callbacks during consuming COMMIT,
and no safe pre-execution opaque identity distinguishes the actual outer
statement under the unchanged transaction API. All8 prototype tests failed;
no callback is selected heuristically or reclassified as valid. The rejected
source/output is [archived](../plans/reports/qa-261005-commit-profile-rejection.md)
and its trace feature/API/harness retired. Existing aggregate commit timing
remains valid, but internal checkpoint/fsync attribution is unresolved. Neither
a PROFILE workload nor a replacement diagnostic or default change is claimed.

## Resource and soak evidence

[`resource_probe`](../src-tauri/src/bin/resource_probe.rs) still leaves its
private-working-set field blank. [`soak.ps1`](../scripts/soak.ps1) now has a
PID-keyed Windows performance-counter path for private working set and a
separate private-commit field. That source change supplies a measurement path,
not an observed B2 result. An installed-app checkpoint must record both total
and private working set and the WebView2 share. A full four-hour collector-active
soak, including dropped/failed write review, remains open.

## Frontend and bundle evidence

The [frontend harness](../src/features/dev/PerfHarness.svelte) and
[`bundle-size.mjs`](../scripts/bundle-size.mjs) own frame and bundle checks.
The dev-only harness now mounts the production `TimelineList` with 100,000
loaded synthetic events rather than a blank spacer. A headless Edge run of
[`measure-timeline-scroll.mjs`](../scripts/measure-timeline-scroll.mjs) on
2026-09-29 measured **16.90 / 20 ms p95** across 600 intervals (three smooth
200-frame segments at the start, middle and 95% depth; repositioning excluded),
**0 blank frames**, and a visible virtual row as deep as **96,291**. The
targeted virtualizer regression also checked real event labels at 50% and
90%. This is dev-source Chromium evidence only; installed WebView2 timeline,
map frames and clean-profile bundle records remain pending.

## Privacy and repeatability

The [synthetic seeder](../src-tauri/src/bin/seed_history.rs) owns fixture and
purge behavior. The opt-in perf tests create an exclusive temporary QA directory
and remove that entire directory after the run; SQL purge alone intentionally
retains entity rows whose ownership cannot be proved. Never point the seeder at
normal app history. Keep reports and CSVs outside the repository and scrub them
before sharing.
