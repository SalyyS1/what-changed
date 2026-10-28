# Seed-pair harness verification output — 2026-10-05

Protocol: [frozen seed-pair harness](qa-261005-fixed-seed-pair-harness.md).
All four commands exit0. No ignored measurement pair is run by these gates.

## Seven ordinary harness checks

Command: `cargo test --release --locked --features devtools --bin seed_history seed_grouped_events::measurement:: -- --test-threads=1`.

```text
Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 1m 11s
     Running unittests src\bin\seed_history.rs (target\release\deps\seed_history-b3d08699438bbe77.exe)

running 9 tests
test seed_grouped_events::measurement::admission::disk_admission_has_fixed_reviewed_single_fixture_allowances ... ok
test seed_grouped_events::measurement::content_digest::tests::corpus_marker_provenance_requires_one_registered_uuid_run ... ok
test seed_grouped_events::measurement::content_digest::tests::framing_distinguishes_types_lengths_and_exact_float_bits ... ok
test seed_grouped_events::measurement::content_digest::tests::normalization_keeps_nested_evidence_and_rejects_unproven_audit_fields ... ok
test seed_grouped_events::measurement::content_digest::tests::normalization_requires_finished_ordered_session_and_exact_registered_manifest ... ok
test seed_grouped_events::measurement::content_digest::tests::ordered_digest_preserves_schema_physical_ids_all_values_and_row_count ... ok
test seed_grouped_events::measurement::content_digest::tests::ownership_digest_is_sorted_but_not_delimiter_ambiguous ... ok
test seed_grouped_events::measurement::fixed_seed_grouping_10k_pair ... ignored, owned fixed-order 10k smoke pair; root executes once after review
test seed_grouped_events::measurement::fixed_seed_grouping_1m_pair ... ignored, owned fixed-order fresh canonical 1M pair; root executes once after reviewed smoke

test result: ok. 7 passed; 0 failed; 2 ignored; 0 measured; 17 filtered out; finished in 0.01s
```

## Full ordinary seeder regression

Command: `cargo test --release --locked --features devtools --bin seed_history -- --test-threads=1`.

```text
Finished `release` profile [optimized] target(s) in 1.24s
     Running unittests src\bin\seed_history.rs (target\release\deps\seed_history-b3d08699438bbe77.exe)

running 26 tests
test seed_batch_boundary_proof::canonical_cross_boundary_endpoint_rejects_larger_batch_parity ... ignored, Explicit proof-only rejection: canonical entity times must not be normalized or promoted
test seed_batch_boundary_proof::seed_batch_boundary_fixture::private_batch_proof_fixture_cleans_normal_and_unwind_paths_without_touching_siblings ... ok
test seed_grouped_events::cleanup_tests::disabled_group_attribution_reads_no_clocks_on_success_or_error ... ok
test seed_grouped_events::cleanup_tests::grouped_owner_unwind_rolls_back_and_next_job_has_fresh_state ... ok
test seed_grouped_events::failure_tests::admission_failure_does_not_claim_an_attempted_group_or_publish_state ... ok
test seed_grouped_events::failure_tests::deferred_commit_failure_reports_completed_bodies_but_no_failed_group_commit ... ok
test seed_grouped_events::failure_tests::error_representation_preserves_primary_and_cleanup_causes ... ok
test seed_grouped_events::failure_tests::event_failure_rolls_back_failed_group_and_keeps_committed_prefix ... ok
test seed_grouped_events::failure_tests::graph_failure_rolls_back_successful_event_body_and_aborts_later_units ... ok
test seed_grouped_events::failure_tests::sqlite_raise_rollback_already_ended_transaction_keeps_primary_error ... ok
test seed_grouped_events::measurement::admission::disk_admission_has_fixed_reviewed_single_fixture_allowances ... ok
test seed_grouped_events::measurement::content_digest::tests::corpus_marker_provenance_requires_one_registered_uuid_run ... ok
test seed_grouped_events::measurement::content_digest::tests::framing_distinguishes_types_lengths_and_exact_float_bits ... ok
test seed_grouped_events::measurement::content_digest::tests::normalization_keeps_nested_evidence_and_rejects_unproven_audit_fields ... ok
test seed_grouped_events::measurement::content_digest::tests::normalization_requires_finished_ordered_session_and_exact_registered_manifest ... ok
test seed_grouped_events::measurement::content_digest::tests::ordered_digest_preserves_schema_physical_ids_all_values_and_row_count ... ok
test seed_grouped_events::measurement::content_digest::tests::ownership_digest_is_sorted_but_not_delimiter_ambiguous ... ok
test seed_grouped_events::measurement::fixed_seed_grouping_10k_pair ... ignored, owned fixed-order 10k smoke pair; root executes once after review
test seed_grouped_events::measurement::fixed_seed_grouping_1m_pair ... ignored, owned fixed-order fresh canonical 1M pair; root executes once after reviewed smoke
test seed_grouped_events::parity_tests::canonical_full_seed_preserves_all_persisted_rows_and_cross_boundary_endpoint ... ok
test seed_grouped_events::parity_tests::nondivisible_tail_preserves_full_seed_and_committed_progress ... ok
test seed_grouped_events::parity_tests::real_entity_collision_and_repeated_owned_updates_preserve_complete_parity ... ok
test seed_grouped_events::purge_tests::purge_removes_registered_grouped_artifacts_and_preserves_real_and_possible_entities ... ok
test seed_grouped_events::seed_grouped_fixture::grouped_fixture_cleans_normal_and_unwind_paths_without_touching_siblings ... ok
test tests::graph_inputs_keep_selected_batch_positions_and_sanitized_fields ... ok
test tests::insertion_attribution_requires_its_own_explicit_cli_flag ... ok

test result: ok. 23 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 23.62s
```

## Strict all-target/all-feature lint

Command: `cargo clippy --release --locked --all-targets --all-features -- -D warnings`.

```text
Checking what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 2.08s
```

## Strict normal-build lint

Command: `cargo clippy --release --locked --no-default-features --lib --bins -- -D warnings`.

```text
Finished `release` profile [optimized] target(s) in 0.77s
```


