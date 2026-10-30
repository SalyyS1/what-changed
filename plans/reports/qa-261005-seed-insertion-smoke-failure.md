# Seed insertion smoke preflight failure — 2026-10-05

Outcome: failed 10k diagnostic, not accepted attribution or seed performance.
Instrumentation verification passed before this run; the diagnostic's corpus
assertion used `relationships` instead of schema-owned `entity_relationships`.
Counsel was consulted before edits/re-diagnosis and approved exactly the
test-table correction with the event-ID filter and 15% expected count unchanged.
No instrumentation, production SQL, schema, budget or fixture recipe changed.
Owned disposable fixture is absent after unwind; no 1M run preceded the repair.

Command: the exact ignored diagnostic selector from the
[execution record](qa-261005-seed-insertion-attribution.md), with
`WHAT_CHANGED_PERF_ROWS=10000`; one Cargo/one job, release/locked/devtools,
owned `.perf-temp`, investigation-stage timing environment unset.

Failed artifact SHA-256 (before the test-only repair):

- `seed_insertion_profile-4bc2eef69d59356e.exe`:
  `EA1A2C8EE1DB9FF78C912B5184A2E24AFA975557A0EE09230A983726A50959EC`.
- `seed_history.exe`:
  `3B3A2DA8DBB283F67724446E9CD88CED7DB602C0C2FBBAF6E0FE358C86638E73`.

## Complete failed output

```text
    Finished `release` profile [optimized] target(s) in 1.06s
     Running tests\seed_insertion_profile.rs (target\release\deps\seed_insertion_profile-4bc2eef69d59356e.exe)

running 1 test
test canonical_seed_reports_aggregate_insertion_attribution ... seed insertion timings: batches=10 attempted_rows=10000 committed_batches=10 committed_rows=10000 failed_batches=0 empty_batches=0 completed_phases=40 begin_prepare_ms=1.105 event_entity_loop_ms=1701.253 reconciliation_ms=0.180 commit_ms=171.933 repository_total_ms=1874.484 caller_insert_ms=1884.384 caller_writer_overhead_ms=9.900
seed timings: elapsed_make_batch_ms=157.644 insert_events_ms=1884.384 graph_ms=53.179 inventory_ms=48.552 checkpoints_ms=121.667 investigations_ms=302.972 total_run_ms=2654.586

thread 'canonical_seed_reports_aggregate_insertion_attribution' (24184) panicked at tests\seed_insertion_profile.rs:185:79:
called `Result::unwrap()` on an `Err` value: SqliteFailure(Error { code: Unknown, extended_code: 1 }, Some("no such table: relationships"))
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
FAILED

failures:

failures:
    canonical_seed_reports_aggregate_insertion_attribution

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 3.67s

error: test failed, to rerun pass `--test seed_insertion_profile`
```

Printed durations remain observations only: the corpus validation failed.
Do not erase this run, exclude its values as host noise, or call it a passed
diagnostic. Corrected ordinary tests and changed-source 10k preflight must pass
before the single canonical 1M attribution.

Unresolved questions: corrected preflight and validated insertion attribution.
