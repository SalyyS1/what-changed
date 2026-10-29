# Complete compare capture focused gate

Status: PASS exit0;30 passed/0 failed/0 ignored,524 filtered,1.62s.
Build4m48s. Sixteen new cases plus ten window and four repository regression
cases; predicted26 excluded the four repository cases, terminal30 is authority.
Caught unwind panics are intentional passing RAII/cleanup assertions.
One job, existing release target, no ignored workload. Source identities:
[source freeze](qa-261005-checkpoint-compare-source-freeze.md).

Command: `cargo test --release --locked --features devtools --lib storage::checkpoint_ -- --nocapture --test-threads=1`.

```text
   Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 4m 48s
     Running unittests src\lib.rs (target\release\deps\what_changed_lib-708106f6a2e3e24a.exe)

running 30 tests
test storage::checkpoint_compare_plan_capture::explain::tests::actual_projection_keeps_node_ids_and_eqp_errors_do_not_retain_sql ... ok
test storage::checkpoint_compare_plan_capture::explain::tests::pinned_checkpoint_access_and_temporary_ordering_are_not_conflated ... ok
test storage::checkpoint_compare_plan_capture::explain::tests::private_schema_names_and_unrecognized_vocabulary_are_rejected_whole ... ok
test storage::checkpoint_compare_plan_capture::owner_tests::direct_metadata_zero_one_and_item_empty_three_column_order_are_complete ... ok
test storage::checkpoint_compare_plan_capture::owner_tests::item_raw_and_json_errors_never_claim_full_consumption ... ok
test storage::checkpoint_compare_plan_capture::owner_tests::metadata_completion_waits_for_raw_kind_and_both_manifest_decodes ... ok
test storage::checkpoint_compare_plan_capture::owner_tests::real_prepare_failures_and_unknown_private_indexes_do_not_replace_results ... ok
test storage::checkpoint_compare_plan_capture::service_tests::actual_domain_item_and_sql_failures_keep_partial_prefix_without_window ... ok
test storage::checkpoint_compare_plan_capture::service_tests::actual_missing_checkpoint_short_circuits_keep_original_error_and_prefix ... ok
test storage::checkpoint_compare_plan_capture::service_tests::actual_some_comparison_preserves_full_semantic_return_and_five_read_order ... ok
test storage::checkpoint_compare_plan_capture::test_fixture::owned_comparison_fixture_closes_on_return_and_unwind_without_touching_sibling ... 
thread 'storage::checkpoint_compare_plan_capture::test_fixture::owned_comparison_fixture_closes_on_return_and_unwind_without_touching_sibling' (28988) panicked at src\storage\checkpoint_compare_plan_test_fixture.rs:170:13:
owned comparison fixture unwind
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
ok
test storage::checkpoint_compare_plan_capture::tests::both_scope_nesting_directions_reject_before_closure_and_preserve_outer ... ok
test storage::checkpoint_compare_plan_capture::tests::inactive_and_missing_envelopes_do_not_claim_reads ... ok
test storage::checkpoint_compare_plan_capture::tests::multiple_reads_are_counted_but_retained_evidence_is_bounded ... ok
test storage::checkpoint_compare_plan_capture::tests::primitive_binding_and_explain_failures_preserve_actual_operation_value ... ok
test storage::checkpoint_compare_plan_capture::tests::scopes_are_thread_bound_and_both_guards_clear_after_unwind ... 
thread 'storage::checkpoint_compare_plan_capture::tests::scopes_are_thread_bound_and_both_guards_clear_after_unwind' (30280) panicked at src\storage\checkpoint_compare_plan_capture_tests.rs:170:13:
owned compare scope unwind
ok
test storage::checkpoint_repository::tests::create_is_atomic_and_delete_cascades_items ... ok
test storage::checkpoint_repository::tests::eleventh_automatic_prunes_oldest_and_never_prunes_manual ... ok
test storage::checkpoint_repository::tests::event_window_is_inclusive_ordered_and_bounded_by_contract ... ok
test storage::checkpoint_repository::tests::state_json_is_redacted_before_it_can_reach_storage ... ok
test storage::checkpoint_window_plan_capture::owner_tests::actual_prepare_and_incompatible_schema_decode_errors_remain_incomplete ... ok
test storage::checkpoint_window_plan_capture::owner_tests::actual_unknown_schema_index_changes_only_plan_evidence ... ok
test storage::checkpoint_window_plan_capture::owner_tests::actual_window_cap_preserves_first_ten_thousand_tied_ids ... ok
test storage::checkpoint_window_plan_capture::owner_tests::normalized_inclusive_bounds_preserve_timestamp_id_ties_and_empty_windows ... ok
test storage::checkpoint_window_plan_capture::tests::inactive_and_missing_scopes_do_not_claim_an_invocation ... ok
test storage::checkpoint_window_plan_capture::tests::multiple_actual_calls_preserve_both_original_results ... ok
test storage::checkpoint_window_plan_capture::tests::nested_scope_preserves_outer_and_rejects_before_inner_closure ... ok
test storage::checkpoint_window_plan_capture::tests::primitive_binding_mismatch_does_not_replace_operation_result ... ok
test storage::checkpoint_window_plan_capture::tests::primitive_explain_failure_and_execution_transitions_are_independent ... ok
test storage::checkpoint_window_plan_capture::tests::scopes_are_thread_local_and_clear_after_unwind ... 
thread 'storage::checkpoint_window_plan_capture::tests::scopes_are_thread_local_and_clear_after_unwind' (28144) panicked at src\storage\checkpoint_window_plan_capture_tests.rs:159:13:
owned checkpoint window capture unwind
ok

test result: ok. 30 passed; 0 failed; 0 ignored; 0 measured; 524 filtered out; finished in 1.62s

```

