# COMMIT PROFILE focused failure — 2026-10-05

Status: FAILED, exit1. Compilation succeeds5m12s; focused8 fail/0 pass in0.16s,
528 filtered. No retries, caller/lint or diagnostic workload runs yet.
[Frozen source](qa-261005-commit-profile-source-freeze.md).

Command: `cargo test --release --locked --features devtools --lib storage::commit_profile_capture:: -- --nocapture --test-threads=1`.
One build job/existing release target/workspace-owned TEMP/TMP.

Successful actual consuming insertion COMMIT records11 scoped PROFILE callbacks,
not the expected1. Recorder correctly yields invalid_multiple/no valid split.
Every focused case fails on the clean-success assertion; thread join also
reports its child panic. No diagnosis, assertion relaxation or source repair
is inferred from this output. Root STOP requests counsel before continuing.

Complete stdout/stderr:

```text
   Compiling rusqlite v0.40.2
   Compiling rusqlite_migration v2.6.0
   Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 5m 12s
     Running unittests src\lib.rs (target\release\deps\what_changed_lib-47de03a0317e14b9.exe)

running 8 tests
test storage::commit_profile_capture::tests::actual_sqlite_commit_preserves_rows_owned_entities_and_legacy_timings ... 
thread 'storage::commit_profile_capture::tests::actual_sqlite_commit_preserves_rows_owned_entities_and_legacy_timings' (26996) panicked at src\storage\commit_profile_capture_tests.rs:49:5:
assertion `left == right` failed
  left: (1, 1, 0, 11)
 right: (1, 1, 1, 1)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
FAILED
test storage::commit_profile_capture::tests::all_valid_observations_accumulate_without_retained_samples ... 
thread 'storage::commit_profile_capture::tests::all_valid_observations_accumulate_without_retained_samples' (26844) panicked at src\storage\commit_profile_capture_tests.rs:49:5:
assertion `left == right` failed
  left: (1, 1, 0, 11)
 right: (1, 1, 1, 1)
FAILED
test storage::commit_profile_capture::tests::failed_consuming_commit_and_drop_rollback_never_produce_valid_splits ... 
thread 'storage::commit_profile_capture::tests::failed_consuming_commit_and_drop_rollback_never_produce_valid_splits' (22184) panicked at src\storage\commit_profile_capture_tests.rs:49:5:
assertion `left == right` failed
  left: (1, 1, 0, 11)
 right: (1, 1, 1, 1)
FAILED
test storage::commit_profile_capture::tests::inactive_real_insertion_and_empty_paths_read_no_profile_clocks ... 
thread 'storage::commit_profile_capture::tests::inactive_real_insertion_and_empty_paths_read_no_profile_clocks' (25924) panicked at src\storage\commit_profile_capture_tests.rs:49:5:
assertion `left == right` failed
  left: (1, 1, 0, 11)
 right: (1, 1, 1, 1)
FAILED
test storage::commit_profile_capture::tests::nested_scope_rejects_before_trace_replacement_or_inner_operation ... 
thread 'storage::commit_profile_capture::tests::nested_scope_rejects_before_trace_replacement_or_inner_operation' (23380) panicked at src\storage\commit_profile_capture_tests.rs:49:5:
assertion `left == right` failed
  left: (1, 1, 0, 11)
 right: (1, 1, 1, 1)
FAILED
test storage::commit_profile_capture::tests::scopes_clear_on_error_empty_and_body_failure_without_fake_commit_attempts ... 
thread 'storage::commit_profile_capture::tests::scopes_clear_on_error_empty_and_body_failure_without_fake_commit_attempts' (25432) panicked at src\storage\commit_profile_capture_tests.rs:49:5:
assertion `left == right` failed
  left: (1, 1, 0, 11)
 right: (1, 1, 1, 1)
FAILED
test storage::commit_profile_capture::tests::thread_isolation_and_unwind_cleanup_allow_subsequent_actual_capture ... 
thread '<unnamed>' (19908) panicked at src\storage\commit_profile_capture_tests.rs:49:5:
assertion `left == right` failed
  left: (1, 1, 0, 11)
 right: (1, 1, 1, 1)

thread 'storage::commit_profile_capture::tests::thread_isolation_and_unwind_cleanup_allow_subsequent_actual_capture' (7308) panicked at src\storage\commit_profile_capture_tests.rs:274:14:
called `Result::unwrap()` on an `Err` value: Any { .. }
FAILED
test storage::commit_profile_capture::tests::zero_and_multiple_callback_lifecycle_observations_are_invalid ... 
thread 'storage::commit_profile_capture::tests::zero_and_multiple_callback_lifecycle_observations_are_invalid' (4720) panicked at src\storage\commit_profile_capture_tests.rs:49:5:
assertion `left == right` failed
  left: (1, 1, 0, 11)
 right: (1, 1, 1, 1)
FAILED

failures:

failures:
    storage::commit_profile_capture::tests::actual_sqlite_commit_preserves_rows_owned_entities_and_legacy_timings
    storage::commit_profile_capture::tests::all_valid_observations_accumulate_without_retained_samples
    storage::commit_profile_capture::tests::failed_consuming_commit_and_drop_rollback_never_produce_valid_splits
    storage::commit_profile_capture::tests::inactive_real_insertion_and_empty_paths_read_no_profile_clocks
    storage::commit_profile_capture::tests::nested_scope_rejects_before_trace_replacement_or_inner_operation
    storage::commit_profile_capture::tests::scopes_clear_on_error_empty_and_body_failure_without_fake_commit_attempts
    storage::commit_profile_capture::tests::thread_isolation_and_unwind_cleanup_allow_subsequent_actual_capture
    storage::commit_profile_capture::tests::zero_and_multiple_callback_lifecycle_observations_are_invalid

test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 528 filtered out; finished in 0.16s

error: test failed, to rerun pass `--lib`
```

Unresolved questions: source-backed callback cardinality/identity boundary and
safe next approach. Exact-one success rule, default batch/SQL/WAL/settings and
all acceptance/release gates stay unchanged; source not verified.

