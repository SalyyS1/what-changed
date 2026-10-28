# Fixed-five correctness verification output — 2026-10-05

Protocol: [fixed-five grouping correctness](qa-261005-fixed-five-seed-grouping.md).
The [initial compilation failure](qa-261005-fixed-five-compile-failure.md) and
counseled two type repairs remain recorded. The following two commands exit 0.
All stdout/stderr is retained; the explicitly caught unwind panics are expected
test paths, not failures. This is correctness evidence, not seed-time acceptance.

## Focused grouped correctness

Command: `cargo test --release --locked --features devtools --bin seed_history seed_grouped_events:: -- --test-threads=1`.

```text
   Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 1m 02s
     Running unittests src\bin\seed_history.rs (target\release\deps\seed_history-b3d08699438bbe77.exe)

running 13 tests
test seed_grouped_events::cleanup_tests::disabled_group_attribution_reads_no_clocks_on_success_or_error ... ok
test seed_grouped_events::cleanup_tests::grouped_owner_unwind_rolls_back_and_next_job_has_fresh_state ... ok
test seed_grouped_events::failure_tests::admission_failure_does_not_claim_an_attempted_group_or_publish_state ... ok
test seed_grouped_events::failure_tests::deferred_commit_failure_reports_completed_bodies_but_no_failed_group_commit ... ok
test seed_grouped_events::failure_tests::error_representation_preserves_primary_and_cleanup_causes ... ok
test seed_grouped_events::failure_tests::event_failure_rolls_back_failed_group_and_keeps_committed_prefix ... ok
test seed_grouped_events::failure_tests::graph_failure_rolls_back_successful_event_body_and_aborts_later_units ... ok
test seed_grouped_events::failure_tests::sqlite_raise_rollback_already_ended_transaction_keeps_primary_error ... ok
test seed_grouped_events::parity_tests::canonical_full_seed_preserves_all_persisted_rows_and_cross_boundary_endpoint ... ok
test seed_grouped_events::parity_tests::nondivisible_tail_preserves_full_seed_and_committed_progress ... ok
test seed_grouped_events::parity_tests::real_entity_collision_and_repeated_owned_updates_preserve_complete_parity ... ok
test seed_grouped_events::purge_tests::purge_removes_registered_grouped_artifacts_and_preserves_real_and_possible_entities ... ok
test seed_grouped_events::seed_grouped_fixture::grouped_fixture_cleans_normal_and_unwind_paths_without_touching_siblings ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 40.20s
```

## Complete seeder, explicit negative proof included

Command: `cargo test --release --locked --features devtools --bin seed_history -- --include-ignored --nocapture --test-threads=1`.

```text
    Finished `release` profile [optimized] target(s) in 0.66s
     Running unittests src\bin\seed_history.rs (target\release\deps\seed_history-b3d08699438bbe77.exe)

running 17 tests
test seed_batch_boundary_proof::canonical_cross_boundary_endpoint_rejects_larger_batch_parity ... seeded 10000/10000 synthetic events
seeded 10000/10000 synthetic events
DIAGNOSTIC seed_batch_boundary_proof rows=10000 events_mismatched=0 entity_rows=2578 entities_mismatched=1 relationship_rows=1500 relationships_mismatched=0 ownership_equal=1 witness_edge_index=986 witness_first_subject_index=3186 witness_first_seen_delta_ms=1710891089 status=rejected_semantic_parity
DIAGNOSTIC seed_batch_boundary_entity_field field=first_seen mismatches=1
ok
test seed_batch_boundary_proof::seed_batch_boundary_fixture::private_batch_proof_fixture_cleans_normal_and_unwind_paths_without_touching_siblings ... 
thread 'seed_batch_boundary_proof::seed_batch_boundary_fixture::private_batch_proof_fixture_cleans_normal_and_unwind_paths_without_touching_siblings' (10260) panicked at src\bin\seed_history_proof\seed_batch_boundary_fixture.rs:95:13:
owned batch proof cleanup unwind
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
ok
test seed_grouped_events::cleanup_tests::disabled_group_attribution_reads_no_clocks_on_success_or_error ... seeded 10000/10000 synthetic events
seeded 10000/10000 synthetic events
ok
test seed_grouped_events::cleanup_tests::grouped_owner_unwind_rolls_back_and_next_job_has_fresh_state ... 
thread 'db-writer' (27228) panicked at src\bin\seed_history_grouping\seed_graph_body.rs:23:24:
index out of bounds: the len is 1000 but the index is 1000
ok
test seed_grouped_events::failure_tests::admission_failure_does_not_claim_an_attempted_group_or_publish_state ... ok
test seed_grouped_events::failure_tests::deferred_commit_failure_reports_completed_bodies_but_no_failed_group_commit ... ok
test seed_grouped_events::failure_tests::error_representation_preserves_primary_and_cleanup_causes ... ok
test seed_grouped_events::failure_tests::event_failure_rolls_back_failed_group_and_keeps_committed_prefix ... ok
test seed_grouped_events::failure_tests::graph_failure_rolls_back_successful_event_body_and_aborts_later_units ... ok
test seed_grouped_events::failure_tests::sqlite_raise_rollback_already_ended_transaction_keeps_primary_error ... ok
test seed_grouped_events::parity_tests::canonical_full_seed_preserves_all_persisted_rows_and_cross_boundary_endpoint ... seeded 10000/10000 synthetic events
seeded 10000/10000 synthetic events
ok
test seed_grouped_events::parity_tests::nondivisible_tail_preserves_full_seed_and_committed_progress ... seeded 10000/11001 synthetic events
seeded 11001/11001 synthetic events
seeded 10000/11001 synthetic events
seeded 11001/11001 synthetic events
ok
test seed_grouped_events::parity_tests::real_entity_collision_and_repeated_owned_updates_preserve_complete_parity ... seeded 10000/10000 synthetic events
seeded 10000/10000 synthetic events
ok
test seed_grouped_events::purge_tests::purge_removes_registered_grouped_artifacts_and_preserves_real_and_possible_entities ... seeded 10000/10000 synthetic events
ok
test seed_grouped_events::seed_grouped_fixture::grouped_fixture_cleans_normal_and_unwind_paths_without_touching_siblings ... 
thread 'seed_grouped_events::seed_grouped_fixture::grouped_fixture_cleans_normal_and_unwind_paths_without_touching_siblings' (13652) panicked at src\bin\seed_history_grouping\seed_grouped_fixture.rs:103:13:
owned grouping fixture unwind
ok
test tests::graph_inputs_keep_selected_batch_positions_and_sanitized_fields ... ok
test tests::insertion_attribution_requires_its_own_explicit_cli_flag ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 34.71s
```

