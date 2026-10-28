# Checkpoint event-window focused output — 2026-10-05

Status: PASS, exit0, ten cases, no failures,528filtered. Frozen
[source identities](qa-261005-checkpoint-window-source-freeze.md).
Command: `cargo test --release --locked --features devtools --lib storage::checkpoint_window_plan_capture:: -- --nocapture --test-threads=1`.
One build job and existing release target; compile6m04s, tests3.00s.
The caught `owned checkpoint window capture unwind` panic is intentional passing
RAII verification, not a hidden failed case. No canonical or latency outcome.

```text
   Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 6m 04s
     Running unittests src\lib.rs (target\release\deps\what_changed_lib-708106f6a2e3e24a.exe)

running 10 tests
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
thread 'storage::checkpoint_window_plan_capture::tests::scopes_are_thread_local_and_clear_after_unwind' (28728) panicked at src\storage\checkpoint_window_plan_capture_tests.rs:159:13:
owned checkpoint window capture unwind
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 528 filtered out; finished in 3.00s

```

Unresolved questions: caller/lint gates and future single-smoke authorization.

