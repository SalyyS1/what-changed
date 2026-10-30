# Page capture focused verification — 2026-10-05

[Frozen source](qa-261005-page-capture-source-freeze.md) and
[approved boundary](qa-261005-page-plan-capture-ledger.md).
Command: `cargo test --release --locked --features devtools --lib page_ -- --nocapture --test-threads=1`.
Terminal exit0:13 passed/zero failed/zero ignored,515 filtered; test0.10s after
6m33s compilation. The deliberate unwind panic is caught by its passing
lifecycle test; it is not a failed test. Complete stdout/stderr follows.
Unit artifact SHA-256:
`bcf5409167c127d5cfe5a5231d863f335e3dd6760ebb8a3d2f3adfac206ef5f3`.
No caller/canonical/lint/full-library acceptance claimed by this receipt.

```text
   Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 6m 33s
     Running unittests src\lib.rs (target\release\deps\what_changed_lib-708106f6a2e3e24a.exe)

running 13 tests
test storage::event_repository::page_capture_owner_tests::concrete_page_bindings_preserve_exact_sql_order_and_variable_arities ... ok
test storage::event_repository::page_capture_owner_tests::prepare_and_lookahead_decode_errors_remain_incomplete_and_clear_scope ... ok
test storage::event_repository::page_capture_owner_tests::tied_keyset_pages_decode_lookahead_and_complete_empty_pages ... ok
test storage::event_repository::page_capture_owner_tests::unknown_actual_schema_index_fails_only_plan_evidence_not_page_execution ... ok
test storage::page_plan_capture::page_plan_explain::tests::actual_sqlite_projection_preserves_all_node_ids_parents_and_order ... ok
test storage::page_plan_capture::page_plan_explain::tests::pinned_access_sort_and_or_vocabulary_preserves_structural_fields ... ok
test storage::page_plan_capture::page_plan_explain::tests::unknown_names_literals_and_vocabulary_are_not_retained_or_classified ... ok
test storage::page_plan_capture::tests::capture_is_thread_local_and_clears_after_unwind ... 
thread 'storage::page_plan_capture::tests::capture_is_thread_local_and_clears_after_unwind' (25560) panicked at src\storage\page_plan_capture_tests.rs:136:13:
page capture unwind
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
ok
test storage::page_plan_capture::tests::inactive_and_missing_scopes_do_not_claim_an_invocation ... ok
test storage::page_plan_capture::tests::multiple_actual_calls_and_variant_mismatch_preserve_operation_results ... ok
test storage::page_plan_capture::tests::nested_scope_preserves_outer_capture_and_does_not_run_inner_operation ... ok
test storage::page_plan_capture::tests::primitive_binding_mismatch_does_not_override_real_operation ... ok
test storage::page_plan_capture::tests::primitive_explain_sqlite_failure_is_independent_of_successful_real_operation ... ok

test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 515 filtered out; finished in 0.10s
```
