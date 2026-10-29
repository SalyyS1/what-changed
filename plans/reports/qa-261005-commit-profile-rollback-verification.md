# COMMIT PROFILE rollback verification — 2026-10-05

Status: PASS for restored-source regression scope. The rejected experiment stays rejected; these are regression
checks of restored source, not successful prototype tests or seed acceptance.
[Rejection and byte-exact snapshots](qa-261005-commit-profile-rejection.md).

## Restored source identities

| Path | SHA-256 |
|---|---|
| src-tauri/Cargo.toml | `5d64037750d5a074843ed2d4f46d1ec65ac709ec5d8825210c7a0f214ecda65d` |
| src-tauri/src/storage/mod.rs | `10594a37726ef3e4aeae704f96961a45931d7373b4bac2f1b0131f73ba3869e6` |
| src-tauri/src/storage/event_repository.rs | `7e69a0cd801e047f7a224cbaef5fc56d93081c005bcc63ef2cc4780fc050d3aa` |
| src-tauri/src/bin/seed_history.rs | `7433e762232181411ab9c5258a46bc26050b7f32695958963c0308c92b6dcc08` |

Root rehash confirms all four exact pre-experiment owners. Six new prototype
source files are absent; PAGE and existing insertion recorder remain compiled.
No user APPDATA, installer, SQL, batch size, pragma or acceptance threshold changed.

## Storage gate

Release/locked/devtools library storage scope, one build job and one test thread:
exit0, compilation4m02s; 106 passed, 0 failed, 1 intentionally ignored,
421 filtered, test duration3.30s. 107 matching tests were selected.
The tool receipt was truncated at its output limit; this is an observed terminal
summary, NOT a complete individual-test transcript. No repeat is run solely to
recover stdout. The earlier complete PAGE library transcript stays a separate
historical gate at its own source/artifact checkpoint.

Restored library executable `src-tauri/target/release/deps/what_changed_lib-708106f6a2e3e24a.exe`
now hashes `758fc8aec275e858ee2dde172dcae01bcd4aab4c2456f25ed11d9646277d73a5`.
This differs from the earlier PAGE executable hash; do not silently substitute
its artifact identity. The failed prototype artifact remains separately retained.

## Feature and independent review checks

Independent reviewer and scoped simplifier both finish CLEAN, no edits. Review
rehashes all four restored owners, all nonshared PAGE files and existing
insertion recorder/transaction/fixture identities; all ten archived snapshots
independently decode and match. Failed eight-case evidence and artifact remain.
Exact restoration is the smallest safe rollback; unrelated complexity is out
of scope. Neither delegate executes tests or accesses personal app data.

Normal Windows tree with the original `--no-default-features` command matches
the stored pre-experiment stdout byte-for-byte (829 characters, LF). Both
current default and devtools trees have no trace/hooks; normal HTTP-client
source gate passes. Root initially omits `--no-default-features` in its
comparison, producing907 rather than829 characters due to a root default-feature
label. Root stops at the discrepancy; counsel permits bounded read-only receipt
inspection, which proves command mismatch rather than a dependency change.
The corrected same-command tree check passes; no source fix or workload rerun.

## Seeder and affected callers

Ordinary restored seeder passes23/26 with three deliberate ignored proof/pair
workloads; compilation11m12s and tests49.83s, exit0. [Complete output](qa-261005-commit-profile-rollback-seeder-output.md).
Affected callers pass query9/repository24/purge9/insertion2/PAGEfixture3,
47 passed, zero failed, two deliberately ignored diagnostics; compile11m12s.
[Complete49-record caller output](qa-261005-commit-profile-rollback-callers-output.md).
Together with storage and seeder:176 passed, zero failed, six deliberate
ignores within this rollback scope. This is not a new full-library/release gate.

Root rehashes tested release executables after terminal commands:

| Executable in src-tauri/target/release/deps | SHA-256 |
|---|---|
| what_changed_lib-708106f6a2e3e24a.exe | `758fc8aec275e858ee2dde172dcae01bcd4aab4c2456f25ed11d9646277d73a5` |
| seed_history-b3d08699438bbe77.exe | `6cfdd4d11d435b1b43daf0c050572cf64cad3aa5bb4e46ec283ffeb2b4a1d796` |
| page_plan_capture_profile-231292b0146e9340.exe | `d1a5d6237b5b7ee099f612f5dd86700bd9b13fcd4de38d55da76834a6b01b3d6` |
| query_service-e304c0772d6b058d.exe | `7c34bc3524a8d0ff3022bcb6d0dcd261879eaa6074501e35177c0353c836b73e` |
| repository-e2f16f6a12ab83dc.exe | `9a7a3508d6ba16511a08cc03b054d320891f9627f9c7b0b64866622f925297dd` |
| seed_history_purge-015922782ed19b2f.exe | `f91c6ca3615a7ee96c3a5624a94b93c39fe5b01d1efed08d5d87742813667f13` |
| seed_insertion_profile-4bc2eef69d59356e.exe | `ccd912cec25eb143661ea39da956e2e5716e76e21be284180ee19ef6fee3a14f` |

Earlier PAGE and other run hashes stay historical receipts; their executable
paths may now hold rebuilt bytes. No canonical smoke is rerun or silently
re-attributed to these new artifact hashes. Failed prototype artifact remains
`e65e2f7c2cae6f624d5d1749f8aec8d10b364a2cee25febb4b48148c4e8ce49d`.

## Terminal source gates and scope

Both strict release Clippy configurations pass: all-target/all-feature1m27s;
normal no-default-feature lib/bins19.80s, warnings denied and exit0.
[Complete lint output](qa-261005-commit-profile-rollback-lint-output.md).
Full formatting and scoped diff checks pass after restoration.
Final root rehash confirms four restored owner identities unchanged. No owned
Cargo/rustc/link or selected test/seeder runner remains; inspected perf/insertion
fixture roots and fresh purge-test roots are absent. Existing unowned roots are
not removed or claimed clean. Docs gate passes33 Markdown owners; journal
validation passes; nine COMMIT reports resolve29 local links with zero missing.
No ignored 10k/1M PROFILE or fixed-group workload runs.
No new plan acceptance checkbox closes.
Terminal counsel GO closes only rejected-technique rollback, retaining all
prototype failures and unresolved attribution. It permits bounded checkpoint
event-window capture preparation independently of missing native setup, using
owned synthetic fixtures. Whole compare/loading/items and current rolling
canonical service plan evidence remain open; no PROFILE replacement authorized.
This receipt freezes restoration at its own terminal checkpoint. Subsequent
approved checkpoint-window preparation adds a storage export and PAGE forwarding
wrapper; its [separate source freeze](qa-261005-checkpoint-window-source-freeze.md)
owns those later hashes. Do not compare that changed tree to this restoration
table as if both described one revision, or reuse old executable paths as hashes.

Unresolved questions: seed performance, remaining plan
obligations and separate Windows test-account name/profile/interactive route.
