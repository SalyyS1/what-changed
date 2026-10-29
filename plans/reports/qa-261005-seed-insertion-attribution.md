# Seed insertion attribution — 2026-10-05

Outcome: implementation frozen; recorder/caller checks, corrected 10k and
single 1M attribution pass. No seed repair or acceptance pass claimed.
The completed [canonical query matrix](qa-261005-recurrence-rolling-query-matrix.md)
passes all 24 query cells, but independent 1M seed time is
316,438.748 / 240,000 ms — MISS. Its coarse insertion stage is 294,049.614 ms.

## Approved boundary

Counsel selects explicitly enabled devtools-only batch attribution before an
insertion change. No SQL, index, pragmas, transaction boundary, entity ownership,
event/entity interleaving, sanitization, fixture recipe or dependency change.
No per-row clock, identifying field, raw SQL/binding/error retention or ambient
capture state. Disabled capture must perform zero clock reads; normal builds
exclude capture. Capture is owned by the specific writer job, after admission.

Measure begin/preparation, the unchanged combined event/entity loop,
reconciliation, commit and total repository duration. Aggregate rows/batches
and bounded durations, emitting once outside insertion work. Preserve existing
coarse `--timings` output. Outer insertion minus repository total is
caller/writer overhead, not proven queue waiting or SQL work attribution.
Attempted/completed/committed counts and error semantics must remain truthful.

## Ownership and verification plan

Owned hooks: insertion portion of `event_repository.rs`, devtools captured API
bridge in `writer.rs`/`db.rs`, seeder explicit diagnostic option/aggregation,
new bounded capture/tests and private `seed_insertion_profile.rs` fixture.
Shared query gate/fixture, recurrence modules, schema and collector code stay
untouched. All existing dirty edits are preserved. Root owns formatting,
execution, review, reports and acceptance; worker starts no process.

Focused checks precede broader validation: actual result/entity parity,
rollback, disabled clock reads, success/error/panic isolation and writer bridge.
Then affected seeder/writer/repository callers, release library and complementary
Clippy. Reuse unaffected frontend/query evidence without rerunning unchanged
matrices. Any unexpected verification failure triggers counsel before repairs.

One corrected 10k diagnostic smoke and one canonical 1M attribution follow
verified instrumentation. The standalone private fixture creates an exclusive
validated child under owned `.perf-temp`, with the unchanged 90-day seed,
3k-entity recipe and explicit diagnostic flag. It closes the DB and removes
only its owned child on return/unwind, surfacing cleanup failure. No user
AppData/history, shared fixture change, 30-call profile or query-matrix rerun.
This diagnostic selects a cause-aligned insertion fix; it is not seed acceptance.

Root command (selected rows 10,000 / 1,000,000, one Cargo/one job, existing
target, owned `.perf-temp` TEMP/TMP, stage timing environment unset):

```powershell
cargo test --release --locked --features devtools --test seed_insertion_profile canonical_seed_reports_aggregate_insertion_attribution -- --ignored --exact --nocapture --test-threads=1
```

## Current evidence

- Coarse 1M seed miss and complete aggregate output are preserved in the linked
  query record; no host-cause or cross-run causal speedup inferred.
- Source recorder and explicit writer/Db API are implemented; seeder activation
  is the separate `--insertion-timings` flag. It is not implied by `--timings`.
  Returned operation errors are separate from aggregate attribution; failed
  batch counts never claim committed input rows. Finish follows helper-local
  statement/transaction destruction, including rollback on errors.
- Root formats only the eight owned files; full rustfmt/scoped diff checks
  pass. Independent read-only reviewer and conditional simplifier find no
  actionable source issue, with no edits or executed-check claim. Live diff
  signals: 7,511 text LOC / 145 tracked files (16 binary) / maximum added 550;
  gate exceeded. Simplifier ran clean with no scoped edits.
- Seven focused release/devtools recorder tests pass, zero failures/ignores,
  501 filtered, 0.12 s after 4m04s compilation. These include actual row/entity
  parity, disabled attribution clocks, real mid-batch and deferred-commit
  rollback, preparation/empty results, unfinished/caller-panic/thread freshness,
  and numeric aggregation. No coverage percentage measured.
- Full release/all-feature library passes 505, zero failures, three explicit
  ignores, 508 total in 19.85 s. Complementary release Clippy passes with warnings
  denied: all-target/all-feature 1m10s, normal lib/bins without defaults 13.79 s.
  Caller compilation finishes in 11m53s; CLI tests pass 2/2, repository 24/24
  in 10.86 s, captured writer bridge and fixture cleanup 2/2 in 0.15 s;
  the explicit diagnostic stays ignored by default.
  No seed/SQL-plan/resource/native/release box closes.
- First 10k diagnostic preflight fails in its corpus assertion: nonexistent
  `relationships` table. [Complete failure and old artifact hashes](qa-261005-seed-insertion-smoke-failure.md)
  are preserved; its printed durations are observations, not validated evidence.
  Counsel approves only `entity_relationships` table-name repair, retaining
  the event-ID filter and 15% assertion. Formatting passes; corrected ordinary
  target tests pass 2/2, one explicit diagnostic ignored, in 0.40 s after
  46.60 s compilation. Corrected 10k smoke passes all numeric/corpus checks;
  the single canonical 1M attribution also passes all numeric/corpus checks.
  Production instrumentation, schema, SQL and thresholds are unchanged.

## Frozen source provenance

Paths relative to `src-tauri/`; after root formatting, before root checks:

| Source | SHA-256 |
|---|---|
| src/storage/event_repository.rs | 68C535B305DAD44878743A04BEC038241F67C4C7BFAAB9810AAADABA76267A94 |
| src/storage/writer.rs | 53570252FE37B03B49BD5407553FD0B3306E66A8DDBD1EDC199EAB67E0F69DA5 |
| src/storage/db.rs | D04D605B174BA498299D5D223C913075846109B865ED089381AAC1E0D869B4E9 |
| src/storage/insertion_timing_capture.rs | FA738800BC9A2C4D47319C40C3156A7FD72B883FA68F3816413A9987290C0512 |
| src/storage/insertion_timing_capture_tests.rs | 9358FC4A74DC0057DB8633052F7993D889FD05FEB2E67863692162500A0AA259 |
| src/bin/seed_history.rs | 4A7E972991D8636F46B6802081CD18F12391BDCD6450A3435A8D75805275D24D |
| tests/seed_insertion_profile.rs | 61E4F9E4222CBBBD9156E6B850783D3E0FB36F0976EF88D4C4CE588F41D12132 |
| tests/support/seed_insertion_fixture.rs | A2C73205B82C44781502893DF81006B8E673DB2DB22825A493E99DD6AF8B42B2 |

The table records the original freeze before the test-only preflight repair.
Corrected diagnostic source SHA-256:
`4D22F2F3D54B3DD2116DBD7517403EB520F3DEE6ACCF347B4C32A2A1008825BA`.
Corrected diagnostic executable `seed_insertion_profile-4bc2eef69d59356e.exe`:
`3AA74A92C6F2E27106CD022A033926D023155FA64E9306ABECFB266C48F5507A`.
Seeder executable `seed_history.exe` is unchanged across preflight correction:
`3B3A2DA8DBB283F67724446E9CD88CED7DB602C0C2FBBAF6E0FE358C86638E73`.
Unit executable `what_changed_lib-708106f6a2e3e24a.exe` after recorder integration:
`F3BFDA4CB4431B5BC4036B1ACA36C13769B9B0453DE102E50F3A2EFD47CEACF1`.

## Validated diagnostic observations

10k changed-source smoke: 10/10 batches committed, 10,000 attempted/committed
input rows, zero failed/empty batches, 40 completed phases. Milliseconds:
begin/preparation 0.379, combined event/entity loop 793.440, reconciliation
0.118, commit 105.337, repository total 899.281, caller insert 904.933,
caller/writer residual 5.652. Coarse seed total 1,429.000 ms, child wall
1,480.198 ms; one selected test passes, two filtered, 1.57 s. Expected corpus
counts/categories/span pass; owned diagnostic fixture is absent after cleanup.
This is attribution only, not seed acceptance or evidence for 1M distribution.

Complete aggregate stdout is retained in the [diagnostic output record](qa-261005-seed-insertion-attribution-output.md).
1M uses the same corrected frozen artifacts: 1,000/1,000 batches committed,
1,000,000 attempted/committed inputs, zero failed/empty batches, 4,000 completed
phases. Milliseconds: begin/preparation 16.550, combined event/entity loop
142,507.949, reconciliation 14.239, commit 195,393.713, repository total
337,934.303, caller insert 338,668.747, caller/writer residual 734.444.
Coarse generation 8,715.957, graph 14,291.192, inventory 129.508, checkpoints
198.448, investigations 6,050.253, total run 368,104.063; child wall
368,150.444 ms. One selected test passes, two filtered, 374.27 s.
Expected basic corpus counts/categories/span pass. Root verifies owned fixture
absent, owned runners exited and unchanged diagnostic/seeder hashes after
completion. D: free 2,946,101,248 bytes; C: 3,393,355,776 bytes at that check.
No manual deletion, outlier exclusion, 10k extrapolation or cross-run causal gain.

Commit is about 58% of repository time in this observation, not a proven WAL,
FTS or storage-device cause. Counsel allows one bounded internal 1,000 versus
5,000 batch experiment after source audit, default and settings unchanged.
Full successful-data parity and explicit failure-prefix tradeoffs precede
timing; total time, not commit alone, controls promotion. Its separate
[frozen protocol](qa-261005-seed-batch-size-experiment.md) is not a third
attribution run or a relaxed seed gate. Source audit raises a potential
batch-dependent entity timestamp contract; the actual canonical negative proof
confirms one `first_seen` difference and rejects event batch 5,000. No timing
experiment follows. Counsel separately permits a design for grouping five
unchanged logical 1,000-event event→graph units per outer commit; default and
settings remain unchanged pending separately scoped parity/verification.

Unresolved questions: a parity-safe cause-aligned seed repair; all other
remaining plan obligations remain independent.
