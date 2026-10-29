---
title: Bounded recurrence-window reuse evidence
date: 2026-10-05
summary: Deterministic capture deadline and integer-only opt-in recurrence statistics; acceptance gates remain open.
---

# Bounded recurrence-window reuse evidence

## What happened

The collector capture deadline regression now uses an already-expired request instant, not a race against a 25 ms worker schedule. Starting a flight must not renew the absolute request deadline; queued and running timeout states remain distinct.

Eight bounded integer statistics were added to the explicitly activated devtools recurrence capture. They borrow the existing sorted inputs and retain no signatures, IDs or window bounds. SQL, actual COUNT executions, ranking and duplicate-ID aggregation remain unchanged.

## Evidence and decision

Focused recurrence capture tests passed 15/15; release/all-feature library passed 479, zero failed, two intentionally ignored. Complementary release Clippy passed for all targets/features and normal lib/bins. A single rustfmt wrapping failure was reviewed before formatting-only repair. Independent review and simplifier found no actionable issue.

The authoritative profiler-free 1M investigation result remains 1629.336/1500 ms FAIL and seeding remains over 240 seconds. No acceptance checkbox was promoted. Implementation details and source hashes: [recurrence-window reuse report](../reports/qa-261005-recurrence-window-reuse.md).

## Next steps

Caller integration passed service 6/6, repository 24/24 and fixture cleanup 3/3. Exactly the five-call canonical 10k and one fresh 1M structural profile passed. The 1M 5,000-query input has 3,429 signatures, zero identical-window repeats and 1,571 monotone overlaps; exact-window caching is unsupported as the main repair. Seed still missed 350.420/240 seconds. Both fixtures were removed and all owned runners exited.

Counsel approved one test-only transaction-local rolling COUNT prototype with unchanged production routing and autocommit fallback. Complete-map parity, snapshot/concurrent-commit checks and two rounds of eleven alternating old/new pairs in representative, sparse, dense and tiny-overlap families must precede promotion. No further structural or 30-call tail diagnostic rerun; do not seek a green unchanged-source gate. Seed insertion remains a separate repair.

## Rolling COUNT experiment

The first test-only variant passed parity but slowed every tiny-overlap pair, about 75% aggregate. Its observations remain preserved. Counsel allowed one strict span-cost guard with widened i128 arithmetic, not threshold tuning. The guarded version passed 13 parity cases and the same 88-pair protocol: canonical-like aggregate improved 16–18%, while sparse/tiny results varied in sign. Dense gains alone were not the decision basis. Full library passed 492, zero failed, three ignored.

Clippy then reported a complex tuple type in the test case table. Work stopped for counsel; an alias-only repair preserves all cases and algorithm behavior. Thirteen parity cases, formatting and both complementary release Clippy configurations pass; the benchmark was not rerun. A separately reviewed production candidate is now being integrated; actual canonical acceptance and seed repair remain open. [Evidence and paired observations](../reports/qa-261005-recurrence-incremental-prototype.md).

## Production integration

The guarded helper is now integrated only for large inputs in an existing transaction; autocommit and small-input behavior stay unchanged. Independent old full-count aggregation remains the parity baseline. Capture now reports actual full/delta SQL separately from resolved full/rolled/reused inputs, with parameter-free first-full/first-delta plan evidence. Six capture cases and public-entry-point parity are implemented. Formatting, independent source review, scoped simplifier and complementary release Clippy pass; production tests are running, not yet claimed green. The owning performance page now explains the changed route. [Revision ledger](../reports/qa-261005-recurrence-rolling-production.md).

Focused production tests pass 34 with zero failures, and the full release/all-feature library passes 498 with zero failures and three explicit ignores. The same independent reviewer confirms the documentation correction and no concrete integration failure in a narrow source forecast. Caller binaries are compiling; unchanged profiler-free matrices follow. Unit artifact provenance is recorded in the revision ledger; no installer or canonical acceptance pass is claimed.

Separate frontend preflight passes typecheck with zero errors/warnings and all 61 files/327 tests using one worker. The frontend runner exited while caller compilation continues. These checks are not a complete `bun run verify` or installed release evidence.

Caller verification now passes service 6/6, repository 24/24 and profile fixture cleanup 3/3, with the two explicit diagnostic profiles ignored. All prior owned runners exited and selected disposable perf/WAL fixtures are absent. The unchanged profiler-free 10k matrix is compiling; one 100k and one 1M follow. Counsel confirms that all 30 observations per shape must be used without exclusions, but the established gate need not export individual durations; no raw-sample archival is claimed.

The unchanged profiler-free 10k and 100k matrices pass 8/8 each: Investigation p95 79.222/400 and 147.841/600 ms. The single 1M run is seeding. [All cells and complete aggregate output](../reports/qa-261005-recurrence-rolling-query-matrix.md) retain the observed data without claiming individual-sample archival, cross-run causal speedup or all-scale acceptance.

The single unchanged-source 1M query matrix also passes 8/8, Investigation
1,032.501/1,500 ms p95. Owned fixture cleanup and process exits are verified;
matrix/seeder artifact hashes still match. Counsel GO closes exactly two P20
query-budget checkboxes: 314/369 complete, 55 open; P20 6/23. Seed remains
316.439/240 seconds — MISS, insertion 294.050 seconds. Counsel selects bounded
devtools batch attribution before any insertion rewrite; remaining gates and
public-v1 NO-GO stay unchanged. Historical failures and all observed output
remain preserved; no cross-run causal gain is claimed.

Release-tooling regression tests also pass: all seven Node test files,
101 tests, zero failures/skips in 9.535 seconds. This verifies fail-closed
tooling behavior, not native license approval or installer notice bytes.
The release audit/public-v1 packet now route the frozen query/source evidence
while retaining their open statuses and NO-GO decision.

The explicit devtools insertion recorder is implemented without changing SQL,
interleaving, ownership or transaction boundaries. It reports committed input
rows separately from failed batches and labels the outer residual caller/writer
overhead, not queue waiting. Independent source review and simplifier are clean;
format/diff, seven focused tests, full library 505/zero failed/three ignored and
both complementary release Clippy configurations pass. Caller/fixture tests
are compiling; the one 10k and one 1M attribution runs remain pending.
[Seed attribution record](../reports/qa-261005-seed-insertion-attribution.md).

Caller verification passes CLI 2/2, repository 24/24 and captured writer/cleanup
2/2 after 11m53s compilation. The first 10k diagnostic fails on the copied
`relationships` table name; counsel precedes a one-literal correction to
`entity_relationships`, with the event-ID filter and 15% expected count unchanged.
Failed output and old hash remain preserved. Ordinary target tests then pass
2/2 and corrected 10k smoke passes every numeric/corpus assertion and cleanup.
The single canonical 1M attribution is running. No threshold, production SQL,
schema or algorithm changed; no source-independent causal gain inferred.

The single 1M attribution passes every numeric/basic corpus assertion: 1,000
committed batches/1M input rows, commit 195.394 seconds (~58% repository) and
combined event/entity loop 142.508 seconds; diagnostic total run 368.104 seconds.
Fixture absence, process exits and artifact hashes are verified. This is not
seed acceptance; profiler-free 316.439/240 seconds remains MISS.
Counsel approves one private 1,000-versus-5,000 batch experiment, full parity
before timing, unchanged default/settings. Worker source audit stops before
edits on a potentially batch-dependent graph endpoint/entity timestamp;
root requests proof-first counsel. No phase/checklist/release promotion.
[Bounded protocol](../reports/qa-261005-seed-batch-size-experiment.md).

The canonical batch-boundary negative proof confirms the semantic difference:
all 10,000 events, 1,500 edges and owned sets match; one of 2,578 event/graph
entities differs only in `first_seen`, by 1,710,891,089 ms. The proof passes as
a rejection, not equivalence. Ordinary seeder tests pass 3/3, one proof ignored;
formatting, independent review/simplifier and both selected Clippy routes pass.
All owned proof fixtures/runners are gone; no checkpoint or timing experiment
follows the rejected 5,000-event batch. Default stays 1,000.
Counsel separately approves a scoped grouping prototype preserving each logical
1,000-event event→graph order and grouping five units per outer transaction;
exact helper/timing API audit comes first. No shared writer redesign, SQL or
settings change is authorized by that proposal. [Negative proof and protocol](../reports/qa-261005-seed-batch-size-experiment.md).

The first transaction-grouping stage is source-frozen: the shared insertion
body keeps original SQL/order/error labels and standalone begin/commit timing.
The concrete devtools entry consumes ownership and returns it only on body
success; it is provisional until outer commit, and errors require abandoning
the outer transaction. Distinct body timing has no committed counters.
Fourteen focused legacy/new tests pass, full library 512/zero failed/three
ignored passes in 20.14 seconds, and complementary release Clippy passes
73 seconds / 13.95 seconds. Independent reviewer/simplifier are clean; source
hashes remain identical after review. Caller binaries are compiling, with
grouped seeder edits still gated. [Staged protocol and provenance](../reports/qa-261005-seed-transaction-grouping-prototype.md).

Caller compilation finishes after 12m01s; seeder 3/3, repository 24/24,
purge 9/9 and insertion profile 2/2 pass, with two intentional proof/diagnostic
ignores. All five frozen source hashes match; owned caller runners/fixtures are
absent. Counsel accepts stage one and approves bin-only fixed-five grouping
and correctness work, not timing or default promotion. Normalization remains a
path-specific source-verified allowlist; evidence times/IDs/scores stay exact.
[Complete extraction verification output](../reports/qa-261005-insertion-body-verification-output.md).

The private fixed-five route passes complete persisted parity, actual SQLite
failure/prefix/progress and conservative purge checks: grouped 13/13 and full
seeder 17/17, including the retained rejected-batch proof. Initial compile
types are repaired only after counsel. Default-route purge 9/9 and insertion
profile 2/2 pass. A 160-byte private-error lint triggers counsel before one
boxed typed payload repair; six failure tests then pass, strict Clippy passes
2.03 s / 0.79 s, and independent amendment review is clean. Source/artifact
identities and absence of owned fixtures/runners are verified.
Counsel permits the separate timing harness and one sequential baseline-first
pair per 10k/1M scale, complete-seed clocks and deterministic corpus digests;
review/disk preflight precede execution. No timing result, default promotion,
seed acceptance or release approval is claimed. [Correctness/next protocol](../reports/qa-261005-fixed-five-seed-grouping.md).

AgentWiki publish skipped.

Independent pending notice work verifies actual SQLite source SHA3 against the
official release and every WebView2 loader candidate against SDK 1.0.3650.58.
The source collection adds 22 complete transcriptions for eleven missing-text
candidates, with exact upstream commit/manifest or npm/tag/integrity provenance
and separately checked local hashes. A raw Cargo metadata mismatch stops status
promotion and triggers counsel before diagnosis; 213 runtime identities and
636 source-manifest hashes match, but the old raw snapshot is unavailable.
The mismatch remains unresolved, draft/manifest unchanged and public release
NO-GO. No native code is executed/installed or personal history accessed.
[Source correspondence](../reports/qa-261005-native-notice-source-correspondence.md),
[missing full-text collection](../reports/qa-261005-missing-license-source-collection.md).

The exact10k/1M fixed-five pairs pass complete persisted/ownership parity and
sequential cleanup under the frozen artifact. Seven harness tests, full seeder
23/23 and both strict Clippy configurations pass. Grouped10k is13.34% slower;
grouped1M332.181s versus baseline369.085s still exceeds240s. Counsel rejects
default promotion and more size/pair tuning. All owner hashes match after runs;
owned fixtures/runners are absent. Default stays1,000, private experiment stays
test-only, profiler-free316.439s acceptance MISS and release NO-GO unchanged.
Next: actual-bound page capture and separate read-only commit-internal source
scout. [Complete pair record/output](../reports/qa-261005-fixed-seed-pair-harness.md).

Source scout verifies PROFILE precedes the automatic WAL callback in pinned
SQLite; safe tracing is absent from current rusqlite features. Counsel permits
a later devtools-only diagnostic conditionally on feature isolation/page freeze,
numeric monotonic timestamps and exact successful callback count. No feature
or callback is added yet. Post-PROFILE tail is not exact checkpoint/fsync time;
no WAL hooks or probes are allowed. [Source/protocol ledger](../reports/qa-261005-commit-profile-source-ledger.md).

User selects a separate Windows standard test account for native/soak checks,
not backup/reuse of current history. Read-only local QA-name-pattern lookup
finds no match; account name/sign-in setup is requested without credentials.
No account creation/elevation/native app launch or current-history access.
[Native handoff](../reports/qa-261005-native-account-handoff.md) retains the
fresh-artifact and interactive-route prerequisites; software checks continue.

Actual-bound page capture is frozen after clean independent review/simplifier,
13 focused and525 library cases,36 callers, both strict Clippy configurations
and formatting. Exactly one counseled canonical10k smoke passes actual first/
tenth page indexed plans, complete lookahead decoding and raw cursor contracts;
fixtures/runners are cleaned. No p95, all-owner/scale or new checkbox closure.
The prior24-cell budget matrix remains verified at its recorded revision, not
silently current after page changes; final frozen-RC latency refresh stays open.
[Page source/gates/output](../reports/qa-261005-page-plan-capture-ledger.md).
Counsel approves next numeric COMMIT PROFILE registration inside the existing
fresh owned writer job; no writer redesign, SQL inspection or WAL/checkpoint
change. Callback count and error/unwind cleanup must be proved before the one
small smoke/one canonical1M diagnostic. Implementation/gates remain pending.

Independent terminal PAGE review finds no actionable claim/provenance/link or
source-integration defect; later shared-owner changes require separate gates.
User-authorized cleanup removes380 stale deterministic QA test directories,
411,118,506 bytes, preserving7 symlink-related roots and all release artifacts.
Initial process guard matched only its own audit process, so no deletion
occurred until a read-only PID check and self-only exclusion. Fixtures are
regenerable, not backed up. [Cleanup receipt](../reports/qa-261005-stale-qa-fixture-cleanup.md).
Normal resolved SQLite feature tree remains byte-identical after devtools-only
trace forwarding; devtools adds trace without hooks. Implementation/gates
remain pending. Advisor context-only patch-repair attempts hit runtime agent
thread limit; this unavailable checkpoint is disclosed, not called counsel.

The narrow COMMIT PROFILE source is formatted/frozen across10 owned paths;
normal feature tree unchanged, devtools trace-only and normal HTTP-client
source gate pass. Numeric monotonic intervals preserve failed/invalid evidence
separately, with real callback-unregistration tests and private default seeder
diagnostics. Eight focused library cases compile; no observed pass or workload
split yet. Root inline review finds no concrete defect; independent review/
simplifier/advisory remain unavailable at the runtime thread limit, disclosed.
Eight old grouped owner hashes and legacy insertion timer/test/PerfFixture
match; no default, WAL/checkpoint or scope change. [Frozen source](../reports/qa-261005-commit-profile-source-freeze.md).

The first PROFILE focused gate compiles5m12s but fails0/8: a successful actual
insertion COMMIT emits11 callbacks, not1. Recorder honestly marks invalid_multiple;
no valid split, caller/lint or10k/1M workload runs. Root stops before diagnosis/
edits, and this new objective counsel call now succeeds. Counsel approves only
one bounded pinned API/internal FTS statement audit to establish safe pre-execution
COMMIT identity; otherwise reject the technique. No callback guessing/count
subtraction/SQL/unsafe or weakening tests. [Full failed output](../reports/qa-261005-commit-profile-focused-failure-output.md).

Bounded source audit finds no safe pre-execution opaque COMMIT identity in the
pinned API. Internal FTS statements can profile within COMMIT, not proof of
the observed11 callbacks' exact provenance. Counsel rejects PROFILE selection
under unchanged constraints; no first/last/longest or SQL/unsafe workaround.
All10 source snapshots are gzip/base64 archived, decompressed and SHA-verified
before six experiment-owned files are removed. Shared storage/seeder exact
prior hashes and devtools=[] restore; failed8 tests remain failed historical
evidence, not relabeled successful by retirement. Aggregate commit timing
remains valid; post-rollback storage gate compiles. [Rejection/archive](../reports/qa-261005-commit-profile-rejection.md).

Restored-source rollback gates finish176 passes: storage106, seeder23, callers47,
six deliberate ignored proof/diagnostic workloads; both strict release Clippy
configurations and formatting pass. Independent reviewer/simplifier find no
actionable finding, no edits; four restored owner hashes exact and10 archives
independently verified. Root's feature comparison initially omits the original
--no-default-features flag and falsely mismatches; counsel permits one bounded
receipt inspection, then the same-command output matches byte-for-byte. No
dependency fix or workload rerun. Old and rebuilt executable hashes stay
separate; the PAGE canonical capture is not re-attributed to the rebuilt binary.
The25-file phase sweep remains314/369,55 open, not release completion.
[Terminal rollback record](../reports/qa-261005-commit-profile-rollback-verification.md).

The next approved checkpoint-window-only capture freezes eight source paths;
independent reviewer/simplifier are CLEAN. Focused10/full-library535/callers7,
both strict release Clippy configurations and formatting pass. One canonical10k
actual compare(Some) smoke proves complete245-ID window consumption, covering
timestamp-index SEARCH and full inactive/captured return equality. Initial
preflight uses relative .qa from src-tauri and fails before Cargo/workload;
root stops for counsel and corrects only the absolute ancestor, then runs once.
Output/hashes/owned cleanup are retained; no whole-comparison/p95/new checkbox
claim. Separate Windows standard account is user-selected; name/profile and
interactive route remain pending, current-user history untouched. [Checkpoint
ledger](../reports/qa-261005-checkpoint-window-capture-ledger.md).

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
