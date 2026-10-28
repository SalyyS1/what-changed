# Guarded recurrence production candidate — 2026-10-05

Outcome: frozen source integration passes focused/full/caller verification
and complementary lint checks; all three canonical query matrices pass 8/8.
Only the two query-budget checkboxes close; no phase or release verdict promoted.

## Decision and scope

The [bounded experiment](qa-261005-recurrence-incremental-prototype.md) preserves
both original negative and guarded paired observations. Counsel authorized
separately reviewed integration, not release or acceptance. No new 88-pair
benchmark, structural profile or 30-call tail diagnostic is being run.

Only ≥2,000 inputs inside an existing transaction use the production rolling
helper. Small and autocommit paths retain original behavior and start no
implicit transaction. Exact signature, valid monotone strict overlap and
widened i128 span guard are unchanged; full COUNT is the fallback. Checked
arithmetic and original-order 150-query sum/later-chunk overwrite are retained.
The guard is density-blind and does not guarantee fewer visited rows.

The previous full-count implementation remains the independent baseline with
its original SQL, sort, count loop and aggregation. The thin test wrapper uses
the real helper; parity compares the independent old map to the PUBLIC
repository entry point, not new against itself. The benchmark's old/warm-old
routes explicitly force the original implementation; its protocol is retained
but not executed again for this integration.

## Capture contract

Devtools opt-in capture separates completed full/delta SQL executions and
resolved input coverage. Successful calls satisfy:

- SQL total = completed full + completed delta.
- Resolved inputs = completed full + completed rolling + identical reuses.
- Rolling inputs ≤ delta executions ≤ twice rolling inputs.

SQL counters advance only after successful COUNTs; resolution follows a
completed count/reuse. First-full and first-delta EXPLAIN use actual borrowed
bindings at their execution points, with typed full/delta classification.
No extra COUNT, retained keys/bounds, raw database errors or TLS borrow during
SQLite work. Existing missing/nested/multiple/unwind guards are retained.
Normal builds exclude capture and test probe counters; no history-proportional
materialization, schema, pragmas, dependency or seeder change.
The ignored actual-service profile now checks truthful probe/coverage formulas,
not a false fixed 5,000 executions. Warm capture stays outside measured calls;
selected full/delta plans do not prove every eight-shape query branch.

## Verification ledger

Root formatted only the ten owned files with child traversal disabled. Full
rustfmt verification and scoped diff checks pass. Frozen source-only worker
review reports no design expansion. Independent code review finds no source
defect; its stale-routing documentation finding is corrected in the owning
performance page. Conditional simplifier ran clean without edits: the 243-line
rolling helper and linear parity/capture suites remain cohesive. Neither
review is represented as executed tests. Complementary release Clippy passes
with warnings denied: all-target/all-feature 57.36 s, normal lib/bins without
default features 10.84 s. Production-focused release/all-feature tests pass:
34 passed, zero failed, one benchmark ignored, 466 filtered in 0.83 s;
release compilation took 3m44s. Full production library passes: 498 passed,
zero failed, three explicit ignores (Windows device sweep, graph comparison,
rolling comparison), 501 total in 25.19 s. Caller compilation finishes in
11m51s; service passes 6/6 in 2.09 s, repository 24/24 in 11.37 s and profile
fixture cleanup 3/3 in 0.28 s, two diagnostic profiles explicitly ignored.
All owned runners exited; selected perf/WAL fixture directories are absent.
Unit artifact `target/release/deps/what_changed_lib-708106f6a2e3e24a.exe`
SHA-256: `713487398DC3E3091253933D929C9A119F887886EF0FFF7F0FB331FD84EEA215`.
The same independent reviewer then performed a narrow read-only integration
failure forecast and confirmed the docs correction. No concrete failure found:
deferred transaction pinning/WAL, autocommit visibility, partial capture under
errors, test-probe moves/Drop order and independent aggregation are source-valid.
This is source counsel, not runtime or performance proof.
Separate frontend preflight `bun run check` passes with zero Svelte errors or
warnings and successful TypeScript check. `bun run test --maxWorkers=1` passes
61 files/327 tests in 341.16 s, zero failures. Its runner exited; caller
binaries also finish. No other owned test/build runner remains before matrices.
This is not a complete `bun run verify`, installed-app or release gate.
Live tracked worktree signals: 7,368 added/deleted lines across 145 files,
maximum added 511; thresholds exceed 400/8/200. Simplifier is restricted to
this production integration, not unrelated dirty-tree changes.

Existing thirteen parity cases now exercise the public production route;
WAL coverage additionally captures the pinned snapshot after an external
commit. Six capture cases cover two delta probes, 5,000 identical-window reuse,
zero-width probes/span rejection, nested/multiple scopes, static delta EXPLAIN
errors, and clearing after real queries plus error/panic. All focused cases
passed through the actual public route for this revision.

Planned root gates, single Cargo runner/one job, pinned release/locked profile:

```powershell
cargo clippy --release --locked --all-targets --all-features -- -D warnings
cargo clippy --release --locked --lib --bins --no-default-features -- -D warnings
cargo test --release --locked --all-features --lib storage::event_repository:: -- --test-threads=1
cargo test --release --locked --all-features --lib -- --quiet --test-threads=1
cargo test --release --locked --all-features --test repository --test investigation_service --test investigation_stage_profile -- --quiet --test-threads=1
```

Focused repository/investigation-service callers and profile compilation pass;
unchanged profiler-free 10k/100k/1M matrices once each follow.
All observations, corpus checks, seed timings and cleanup must be retained.
Counsel confirms unchanged aggregate output is appropriate: all 30 samples
per shape contribute to p95; individual sample durations are not exported.
No raw-sample archival or reconstruction is claimed. Exact 10k and 100k
matrices pass 8/8 each; the single unchanged-source 1M matrix also passes 8/8,
investigation 1032.501/1500 ms p95. No query gate was rerun for green.
[All cells, complete aggregate output and provenance](qa-261005-recurrence-rolling-query-matrix.md).
The 492-pass test-only result does not validate this source revision.

## Frozen source provenance

Paths relative to `src-tauri/`; SHA-256 after root formatting:

| Source | SHA-256 |
|---|---|
| src/storage/event_repository.rs | FF43E63DB9D9F0E51100B85888A42D80D5EF7B46560974CAF1B3705902B65E2F |
| src/storage/recurrence_rolling_counts.rs | 4331B1B85DFBD4B6C0F35AB74F94B11E37AEFC612C8B9F48AA7D554E8EFC245D |
| src/storage/recurrence_incremental_prototype.rs | 96BB6DB1FEAC94E78862D633AF508BBE19C820B7A90B85D232BBAD3F6217A7CA |
| src/storage/recurrence_incremental_prototype_fixture.rs | EEC5B1F54494434FB84CEAE54C0263DBE627C7C3D475AB24C4E354C2591B4BC3 |
| src/storage/recurrence_incremental_prototype_parity.rs | 9DE786920DE5EB27EA31B9F9FFA7A00A1D101500F4F3AF519D0FD2A901056CDF |
| src/storage/recurrence_incremental_prototype_benchmark.rs | 390C126B32991681485AB90A1D3597C4ABB83185A103D938DF0BAB55C0D791B9 |
| src/storage/recurrence_plan_capture.rs | D5E7B48A511627E76D40A67B891ADD27244D87FC71DA4C5F0FF9F1C51B56AE69 |
| src/storage/recurrence_execution_capture.rs | 32C530284972A11E6E225A88A66F5D2876D0BD0B7DF95567238420BF36EA6E72 |
| src/storage/recurrence_plan_capture_tests.rs | 0402D1E352B9364485F8F66603B06CB2DCC804D37C9424114D596CCB87299B71 |
| tests/investigation_stage_profile.rs | 3CF7C08E4EA8F26F7E21523746726DC974B16124C48F4DB17C7D5467DFCB9A5E |

No current-source installer is claimed. This revision supersedes the prior
profiler-free 1M investigation 1629.336/1500 ms FAIL; historical evidence is kept.
The current canonical fixture seed 316.439/240 seconds misses; reading
optimization is not seed repair. Counsel GO closes exactly the two query-budget
checkboxes and selects explicitly enabled batch insertion attribution next.

Unresolved questions: seed, SQL-plan parity,
resources, installed accessibility and release
obligations remain separate and open.
