# Transaction-body extraction verification output — 2026-10-05

Source/protocol: [fixed-five grouping prototype](qa-261005-seed-transaction-grouping-prototype.md).
These checks verify the extracted body, not grouped seed parity, performance or release acceptance.
Complete command output is retained below; all five commands exited 0.
One root-owned Cargo/job used the existing release target and owned `.perf-temp` TEMP/TMP.
The diagnostic error line in the library suite belongs to an intentional error-path test.

## Legacy recorder and transaction body

Command: `cargo test --release --locked --all-features --lib insertion_timing_capture:: -- --test-threads=1`.

```text
   Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 4m 22s
     Running unittests src\lib.rs (target\release\deps\what_changed_lib-708106f6a2e3e24a.exe)

running 14 tests
test storage::event_repository::insertion_timing_capture::tests::aggregation_adds_counts_and_durations_without_retaining_history ... ok
test storage::event_repository::insertion_timing_capture::tests::caller_unwinding_and_other_threads_cannot_leave_capture_state ... ok
test storage::event_repository::insertion_timing_capture::tests::captured_insert_keeps_actual_rows_ids_and_entity_ownership ... ok
test storage::event_repository::insertion_timing_capture::tests::deferred_commit_failure_includes_rollback_but_no_committed_rows ... ok
test storage::event_repository::insertion_timing_capture::tests::disabled_insert_paths_read_no_attribution_clocks ... ok
test storage::event_repository::insertion_timing_capture::tests::preparation_error_and_empty_capture_do_not_claim_commits ... ok
test storage::event_repository::insertion_timing_capture::tests::real_mid_batch_failure_rolls_back_and_next_capture_is_fresh ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::body_error_returns_no_ownership_and_requires_outer_abandonment ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::body_success_is_uncommitted_and_outer_drop_rolls_back ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::caller_unwind_rolls_back_completed_body_and_leaves_no_capture_state ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::disabled_body_capture_reads_no_clocks_on_success_error_or_empty ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::outer_commit_matches_legacy_rows_and_preserves_real_entities ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::preparation_error_and_empty_body_keep_transaction_ownership ... ok
test storage::event_repository::insertion_timing_capture::transaction_tests::successive_bodies_refresh_owned_entities_without_committing ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 501 filtered out; finished in 0.44s
```

## Complete library

Command: `cargo test --release --locked --all-features --lib -- --quiet --test-threads=1`.

```text
    Finished `release` profile [optimized] target(s) in 0.65s
     Running unittests src\lib.rs (target\release\deps\what_changed_lib-708106f6a2e3e24a.exe)

running 515 tests
..i.....DIAGNOSTIC analysis_stages status=error total_inclusive_ms=0.051 preprocess_exclusive_ms=0.051 structural_graph_exclusive_ms=0.000 chain_index_exclusive_ms=0.000 failure_index_exclusive_ms=0.000 candidate_preparation_exclusive_ms=0.000 candidate_scoring_exclusive_ms=0.000 selection_explanations_exclusive_ms=0.000
............................................................................... 87/515
....................................................................................... 174/515
....................................................................................... 261/515
........................................i.............................................. 348/515
...............................................................DIAGNOSTIC investigation_stages status=complete total_inclusive_ms=2.011 input_inclusive_ms=1.011 request_setup_exclusive_ms=2.000 window_hydration_exclusive_ms=0.011 boundary_queries_exclusive_ms=0.000 entity_load_exclusive_ms=0.000 relationships_exclusive_ms=0.000 recurrence_exclusive_ms=0.000 input_finalize_exclusive_ms=0.000 analysis_scoring_exclusive_ms=0.000 result_sanitization_exclusive_ms=0.000 persistence_exclusive_ms=0.000
........................ 435/515
...............i................................................................
test result: ok. 512 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 20.14s
```

## All-target/all-feature Clippy

Command: `cargo clippy --release --locked --all-targets --all-features -- -D warnings`.

```text
    Checking what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 1m 13s
```

## Normal lib/bin Clippy

Command: `cargo clippy --release --locked --no-default-features --lib --bins -- -D warnings`.

```text
    Checking what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 13.95s
```

## Affected callers

Command: `cargo test --release --locked --features devtools --bin seed_history --test repository --test seed_insertion_profile --test seed_history_purge -- --quiet --test-threads=1`.

```text
   Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 12m 01s
     Running unittests src\bin\seed_history.rs (target\release\deps\seed_history-b3d08699438bbe77.exe)

running 4 tests
i...
test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.11s

     Running tests\repository.rs (target\release\deps\repository-e2f16f6a12ab83dc.exe)

running 24 tests
........................
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.02s

     Running tests\seed_history_purge.rs (target\release\deps\seed_history_purge-015922782ed19b2f.exe)

running 9 tests
.........
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.88s

     Running tests\seed_insertion_profile.rs (target\release\deps\seed_insertion_profile-4bc2eef69d59356e.exe)

running 3 tests
i..
test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

## Caller artifact/cleanup ledger

Session 51912 exits 0. At 2026-10-05 01:21 UTC, no owned Cargo/rustc/seeder
caller remains and owned `what-changed-seed-history-*`,
`what-changed-seed-insertion-*` and `what-changed-seed-batch-proof-*` fixtures
are absent under `.perf-temp`. Cleanup is fixture-owned; no manual deletion.
Five stage-one source/unchanged-recorder/seeder hashes match the protocol.
The later grouped prototype does not inherit acceptance from this record.

Paths below are relative to `src-tauri/target/release/`:

| Artifact | SHA-256 |
|---|---|
| deps/seed_history-b3d08699438bbe77.exe | A6DDE749783E76B352C0453A48AEA3EFDC6F82534F71184580B9A79CC0E28870 |
| deps/repository-e2f16f6a12ab83dc.exe | 3801C5B7E04640ECE7AA2E91D83A3F78492B37767A7D396969A9C3DFB7130B19 |
| deps/seed_history_purge-015922782ed19b2f.exe | E6E2C567DCF8ED2060E36B6F4F85D061106975755B598E84B803C1FF47CA2B29 |
| deps/seed_insertion_profile-4bc2eef69d59356e.exe | F1298EEAC22BED31C4DC30B95FEBCD2601E585B1D182F73FD366A9AB010D959D |
| seed_history.exe | EDACB89C417ECB0FE157E86CD5132B5D926325ABC45EBDBACDA7761FE9D34AA9 |
