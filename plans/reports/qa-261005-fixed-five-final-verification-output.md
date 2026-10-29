# Fixed-five final verification output — 2026-10-05

Protocol: [grouped correctness](qa-261005-fixed-five-seed-grouping.md).
All four commands exit 0. The [initial lint failure](qa-261005-fixed-five-clippy-failure.md)
and counseled failure-only boxed repair remain recorded. Full positive parity
and default caller checks predate the representation-only repair; unchanged
behavior/source owners justify their reuse, with six failure cases rerun.
This is correctness/lint evidence, not a performance or release acceptance pass.

## Affected default-route callers

Command: `cargo test --release --locked --features devtools --test seed_history_purge --test seed_insertion_profile -- --quiet --test-threads=1`.

```text
Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 51.76s
     Running tests\seed_history_purge.rs (target\release\deps\seed_history_purge-015922782ed19b2f.exe)

running 9 tests
.........
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.91s

     Running tests\seed_insertion_profile.rs (target\release\deps\seed_insertion_profile-4bc2eef69d59356e.exe)

running 3 tests
i..
test result: ok. 2 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.41s
```

## Boxed private-error regression

Command: `cargo test --release --locked --features devtools --bin seed_history seed_grouped_events::failure_tests:: -- --test-threads=1`.

```text
Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 50.65s
     Running unittests src\bin\seed_history.rs (target\release\deps\seed_history-b3d08699438bbe77.exe)

running 6 tests
test seed_grouped_events::failure_tests::admission_failure_does_not_claim_an_attempted_group_or_publish_state ... ok
test seed_grouped_events::failure_tests::deferred_commit_failure_reports_completed_bodies_but_no_failed_group_commit ... ok
test seed_grouped_events::failure_tests::error_representation_preserves_primary_and_cleanup_causes ... ok
test seed_grouped_events::failure_tests::event_failure_rolls_back_failed_group_and_keeps_committed_prefix ... ok
test seed_grouped_events::failure_tests::graph_failure_rolls_back_successful_event_body_and_aborts_later_units ... ok
test seed_grouped_events::failure_tests::sqlite_raise_rollback_already_ended_transaction_keeps_primary_error ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 5.59s
```

## All-target/all-feature strict Clippy

Command: `cargo clippy --release --locked --all-targets --all-features -- -D warnings`.

```text
Checking what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 2.03s
```

## Normal-build strict Clippy

Command: `cargo clippy --release --locked --no-default-features --lib --bins -- -D warnings`.

```text
Finished `release` profile [optimized] target(s) in 0.79s
```

## Post-run artifact and cleanup check

Root rehashes all 15 stage-two source owners after the amendment review.
The four extraction/recorder files, both rejected-proof files and canonical
query source remain at their recorded hashes. No Cargo/rustc/seeder/library
runner or owned grouping/batch-proof/history/insertion fixture prefix remains.
No deletion command is run: test-owned RAII cleanup has already removed them.

Current seeder unit SHA-256: `90A0B95EDF918F1230F010E75BAC058639D93B1F0720D4E40DCB670335B2EA68`.
Default-route CLI SHA-256: `6EB5DE1FB4BCD44C4D3CFB42E781E19954778FA3FA9A5A6A06F93B73670D8ECA`.
These identities are not a claim that the separately compiled default CLI
contains the cfg(test) grouping path. Pre-box unit identity is historical.

