# Transaction-local rolling COUNT prototype — 2026-10-05

Outcome: bounded test-only experiment completed; the recorded source revisions
below did not activate production. Subsequent integration is a separate
[production-candidate revision](qa-261005-recurrence-rolling-production.md).
Both variants pass parity and their fixed matched benchmarks. The unguarded
variant is rejected; guarded evidence authorizes a separate production candidate.

## Contract and selection

The [canonical structural evidence](qa-261005-recurrence-window-reuse.md) found
zero repeated windows and 1,571 adjacent monotone overlaps in 5,000 actual 1M
inputs. Counsel selected one bounded rolling COUNT experiment, not more
structural measurements or a repeat of the 30-call diagnostic.

The prototype falls back directly to the unchanged repository for autocommit
or fewer than 2,000 queries. It starts no transaction. Within an existing
snapshot, same-signature valid monotone strictly overlapping intervals use
`prior - COUNT[old.since,new.since) + COUNT[old.before,new.before)`; zero-width
deltas are skipped and identical valid windows reuse the prior count. All other
cases execute full COUNT. Original input order and 150-query duplicate-ID
sum/later-block overwrite behavior are preserved with checked arithmetic.

Only previous query/count and existing input-sized order/count buffers are
retained; no history-proportional timestamps are materialized. Production
routing, capture, SQL, schema and pragmas are unchanged. The new module is
registered solely with `cfg(test)`.

## Review and verification

Root inspected the formula, fallback, parity and benchmark boundaries. Scoped
simplifier reviewed the frozen files and made no edits; the cohesive parity
suite remains one file. Formatting changed only five new test-only modules;
rustfmt and scoped diff checks pass. Independent reviewer reports zero
actionable findings, confirming production isolation and benchmark boundaries.

Nine parity cases cover thresholds/empty input, half-open extremes and invalid
windows, repeated timestamps/signature fields, duplicate IDs across 149/150
and 299/300, fixed-seed generated histories/permutations, autocommit fallback,
WAL snapshot across external commit, checked arithmetic failure and typed
missing-table errors. A uniquely created QA directory is resolved and checked
before recursive removal; cleanup errors are observable during unwind.
Focused release/all-feature compilation took 3m30s; nine parity tests passed,
zero failed, one benchmark ignored, 481 filtered in 0.19 s. The exact ignored
benchmark then passed one test, zero failed, 490 filtered in 17.82 s.
Strict parsing validates all 88 unique pairs, declared alternation and actual
full/delta probe sums. No coverage percentage was measured. The owned WAL
fixture is absent and all runner processes exited.

## Fixed benchmark protocol

Each family runs two rounds of eleven matched old/new pairs, alternating first
method within each round and reversing it across rounds. Both use the same
read transaction and warm-up; timing wraps complete calls. Equality, output,
unwrap and map teardown are outside clocks. All 88 pairs and actual full/delta
probe counts must be retained; no outlier removal or acceptance claim.

| Synthetic family | Signatures | Rows/signature | Total rows | Width/shift | Potential overlaps | Max group |
|---|---:|---:|---:|---:|---:|---:|
| Canonical-like input | 3,429 | 128 | 438,912 | 96/1 | 1,571 | 18 |
| Unique sparse | 5,000 | 4 | 20,000 | 96/1 | 0 | 1 |
| Dense overlap | 64 | 2,048 | 131,072 | 1,024/1 | 4,936 | 79 |
| Wide shift/tiny overlap | 1,000 | 1,280 | 1,280,000 | 256/255 | 4,000 | 5 |

All histories are in-memory timestamp grids starting at zero. Windows use
ordinal × shift and that bound + width. Fixed input reversal/rotation 173.
Only the first family's input distribution matches measured structure;
history density and bounds are synthetic, not reconstructed canonical keys.
The tiny-overlap family deliberately exposes increased range/probe costs.

Workdir `src-tauri`, pinned release profile/locked dependencies, single root
Cargo runner, owned `.perf-temp` TEMP/TMP, timing opt-in unset:

```powershell
cargo test --release --locked --all-features --lib storage::event_repository::recurrence_incremental_prototype:: -- --test-threads=1
cargo test --release --locked --all-features --lib storage::event_repository::recurrence_incremental_prototype::benchmark::two_rounds_eleven_alternating_pairs_compare_transaction_local_counts -- --exact --ignored --nocapture --test-threads=1
```

Frozen first-variant SHA-256, paths relative to `src-tauri/`; these identify
the unguarded run, not the later span-guard revision:

| Source | SHA-256 |
|---|---|
| src/storage/event_repository.rs | 00D3E04923A79FE376641D6FC197E24074595E6EAEC308838FB3873360249AA0 |
| src/storage/recurrence_incremental_prototype.rs | A85D556C911BE1D75B642AF08F1F2467C33759FFAC7E6DA0D0FEE9554716CDDA |
| src/storage/recurrence_incremental_prototype_fixture.rs | 2E94CAA49F9802547210F869D50F9AD2790E93091C38618CCEBC05DC1EB03427 |
| src/storage/recurrence_incremental_prototype_parity.rs | 58DE1256BE45D4C5886604851B251E326D701001EBA34882FD841FF370736978 |
| src/storage/recurrence_incremental_prototype_recipes.rs | 8BC812E0D16EB1D9C04A677FFA1F536B44F71F169B668A14D233BD223586A14E |
| src/storage/recurrence_incremental_prototype_benchmark.rs | 89B4C9A807DB712B320B911C5D0600B303D14909DECE97608A37907768C7482F |

## First-variant result and remedy

All first-variant pairs and round summaries are preserved in the
[unguarded paired observations](qa-261005-recurrence-incremental-unguarded-pairs.md).
Canonical-like aggregate changes are -18.6337/-19.2583%, faster in all 22
pairs. Sparse +2.1402/-0.5728% varies in sign; dense -93.9882/-94.0740%
is faster in all 22 pairs. Tiny-overlap +74.9516/+76.3851% is slower in
all 22 pairs. These are isolated synthetic COUNT timings, not service gains.

Counsel rejects unguarded promotion and authorizes exactly one eligibility
refinement: after the existing guards, removed span + added span must be
strictly less than current full-window span. Convert each endpoint to i128
before subtraction. This is a heuristic, not a row-density or no-regression
guarantee. Preserve parity and add below/equal/above-boundary, i64-extreme and
clustered-edge-history coverage. Tiny-overlap expected probes become 5,000
full/zero delta; the same four recipes and two-by-eleven protocol remain fixed.
One changed-source rerun follows; if material tiny-overlap regression remains,
reject this experiment rather than tune thresholds to these fixtures.

## Guarded revision — measured, not activated

Each endpoint is widened to i128 before subtraction. The strict span guard
retains identical-window reuse and skips zero-width delta probes. Four added
parity cases cover below/equal/above cost, single deltas, i64 extremes and a
clustered-edge counterexample: short deltas can still visit more rows than a
full window. Independent review found zero actionable issues.
Focused release build took 4m19s; 13 passed, zero failed, one benchmark ignored,
481 filtered in 0.27 s. The unchanged fixed 88-pair benchmark passed in 18.03 s,
494 filtered. Full release/all-feature library: 492 passed, zero failed,
three intentionally ignored in 29.40 s. Clippy then rejected a complex tuple
type in the parity table; failed-verification counsel preceded the alias-only
repair. All cases/assertions/order and algorithm stay unchanged. Rustfmt passes;
13 parity tests pass again in 0.45 s (release build 5m18s). Complementary
release Clippy passes with warnings denied: all-target/all-feature 29.41 s,
normal lib/bins/no-default-features 22.72 s. All owned Cargo runners exited.
Repaired parity source SHA-256:
`DD12BA669DF76F98433494893DB184A3FBF2E40F324C628DD18FD73B2C2373F5`.
The paired-run source hashes below remain the measured pre-alias revision;
no second benchmark was run for a type-spelling repair.

[All guarded pairs](qa-261005-recurrence-incremental-guarded-pairs.md) retain every
observation. Canonical-like aggregate -16.5175/-18.2603%, all 22 pairs faster;
dense -94.4977/-94.0765%, all 22 faster. Sparse -0.4761/+3.0459% and tiny
+4.4636/-0.5643% vary in sign, not proof of zero overhead. Tiny now executes
5,000 full/zero delta COUNTs. Synthetic results are not actual service gains.

Guarded SHA-256 for changed sources, paths relative to `src-tauri/`; parent
registration and fixture retain their first-variant hashes above:

| Source | SHA-256 |
|---|---|
| src/storage/recurrence_incremental_prototype.rs | 1F6E5F7AB3BE4FAB5D4FB35C0ECC3901471C166113982ABD9EEC153BF70EE3BA |
| src/storage/recurrence_incremental_prototype_parity.rs | B5D5DB7CF4E64C92622D9F8A00A330B298D546454A99E450F0949586E412B1C3 |
| src/storage/recurrence_incremental_prototype_benchmark.rs | 86DAEDBCF243BEA7645EADF92E10FF79D7914B82AFDE5000149F854F131EE1DE |
| src/storage/recurrence_incremental_prototype_recipes.rs | 02D3EF178D7A099F67F9608BBF716CCE90FC6EEFE7AB64EE7B995ED93411F6FB |

Counsel permits separately reviewed production integration, not acceptance or
release. Preserve an independent old test baseline, existing-transaction-only
activation and honest full/delta/reuse capture at actual execution points.
Then validate the production entry point and run unchanged profiler-free
10k/100k/1M matrices once per changed revision. Seed remains separate.

## Acceptance remains open

Production promotion requires complete-map parity and repeatable benefit in
representative overlap cases, without material repeatable sparse regression.
An artificial dense-only win is insufficient; marginal benefit rejects the
candidate. Counsel reviews observations before any production change.
The existing profiler-free 1M 1629.336/1500 ms failure, seed target, other
57 open checks and public-v1 NO-GO remain unchanged.

Unresolved questions: production integration and actual canonical acceptance
remain pending; span is a density-blind heuristic and seed still misses budget.
