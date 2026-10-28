# Checkpoint event-window caller output — 2026-10-05

Terminal exit0. Sequential CARGO_BUILD_JOBS=1; no ignored workload activated.

Command (cwd src-tauri):

```text
cargo test --release --locked --features devtools --test checkpoint_compare --test checkpoint_window_plan_profile -- --test-threads=1
```

Complete retained stdout/stderr:

```text
   Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 12m 42s
     Running tests\checkpoint_compare.rs (target\release\deps\checkpoint_compare-df1dcb5ffbf7a1ac.exe)

running 4 tests
test application_install_and_uninstall_are_added_removed_in_application_category ... ok
test comparison_returns_exact_added_removed_and_field_changes ... ok
test p95_uses_nearest_rank_instead_of_the_maximum_sample ... ok
test persisted_3000_item_service_comparison_and_event_window_stay_under_150ms_p95 ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.12s

     Running tests\checkpoint_window_plan_profile.rs (target\release\deps\checkpoint_window_plan_profile-7412c5938e94396d.exe)

running 4 tests
test canonical_10k_actual_checkpoint_compare_event_window_plan ... ignored, one unchanged canonical10k actual checkpoint event-window diagnostic; root executes after gates
test perf_fixture::app_state_fixture_joins_maintenance_and_removes_its_owned_directory ... ok
test perf_fixture::assertion_failure_still_closes_and_removes_the_fixture ... ok
test perf_fixture::dropping_a_fixture_removes_only_its_owned_directory ... ok

test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.16s
```

Four comparison and three owned-fixture tests pass; one canonical diagnostic deliberately ignored. The persisted p95 case is its small caller fixture, not the current canonical three-scale matrix. [Ledger](qa-261005-checkpoint-window-capture-ledger.md).

