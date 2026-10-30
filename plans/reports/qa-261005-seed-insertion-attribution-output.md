# Seed insertion aggregate diagnostic output — 2026-10-05

Source, command and provenance: [execution record](qa-261005-seed-insertion-attribution.md).
The [failed preflight](qa-261005-seed-insertion-smoke-failure.md) is retained
separately. These numeric attribution records do not replace seed acceptance.

## Corrected 10k smoke — exit 0

```text
    Finished `release` profile [optimized] target(s) in 0.81s
     Running tests\seed_insertion_profile.rs (target\release\deps\seed_insertion_profile-4bc2eef69d59356e.exe)

running 1 test
test canonical_seed_reports_aggregate_insertion_attribution ... seed insertion timings: batches=10 attempted_rows=10000 committed_batches=10 committed_rows=10000 failed_batches=0 empty_batches=0 completed_phases=40 begin_prepare_ms=0.379 event_entity_loop_ms=793.440 reconciliation_ms=0.118 commit_ms=105.337 repository_total_ms=899.281 caller_insert_ms=904.933 caller_writer_overhead_ms=5.652
seed timings: elapsed_make_batch_ms=72.342 insert_events_ms=904.933 graph_ms=28.407 inventory_ms=41.909 checkpoints_ms=94.344 investigations_ms=247.284 total_run_ms=1429.000
DIAGNOSTIC seed_insertion_corpus rows=10000 entities=3000 relation_percent=15 burst_percent=8 checkpoints=40 snapshots=9800 investigations=20 span_days=90
DIAGNOSTIC seed_insertion_profile rows=10000 elapsed_ms=1480.198 status=attributed_not_acceptance
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 1.57s
```

## Canonical 1M attribution — exit 0

```text
    Finished `release` profile [optimized] target(s) in 0.64s
     Running tests\seed_insertion_profile.rs (target\release\deps\seed_insertion_profile-4bc2eef69d59356e.exe)

running 1 test
test canonical_seed_reports_aggregate_insertion_attribution ... seed insertion timings: batches=1000 attempted_rows=1000000 committed_batches=1000 committed_rows=1000000 failed_batches=0 empty_batches=0 completed_phases=4000 begin_prepare_ms=16.550 event_entity_loop_ms=142507.949 reconciliation_ms=14.239 commit_ms=195393.713 repository_total_ms=337934.303 caller_insert_ms=338668.747 caller_writer_overhead_ms=734.444
seed timings: elapsed_make_batch_ms=8715.957 insert_events_ms=338668.747 graph_ms=14291.192 inventory_ms=129.508 checkpoints_ms=198.448 investigations_ms=6050.253 total_run_ms=368104.063
DIAGNOSTIC seed_insertion_corpus rows=1000000 entities=3000 relation_percent=15 burst_percent=8 checkpoints=40 snapshots=9800 investigations=20 span_days=90
DIAGNOSTIC seed_insertion_profile rows=1000000 elapsed_ms=368150.444 status=attributed_not_acceptance
ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 374.27s
```
