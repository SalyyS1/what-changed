# Rejected COMMIT PROFILE technique — 2026-10-05

Status: REJECTED under the accepted no-SQL/no-unsafe/unchanged-consuming-commit
constraints. Counsel GO permits archival plus experiment-only rollback, then
regression checks. All8 prototype tests FAILED; retirement is not a test pass.
No PROFILE10k/1M workload or replacement diagnostic is authorized.

[Full failed output](qa-261005-commit-profile-focused-failure-output.md),
[frozen revision](qa-261005-commit-profile-source-freeze.md),
[protocol/source history](qa-261005-commit-profile-source-ledger.md).
Failed executable remains src-tauri/target/release/deps/what_changed_lib-47de03a0317e14b9.exe,
SHA-256 `e65e2f7c2cae6f624d5d1749f8aec8d10b364a2cee25febb4b48148c4e8ce49d`.

## Verified feasibility boundary

Read-only scout inspects pinned rusqlite0.40.2/libsqlite3-sys0.38.2 sources.
StmtRef trace.rs98–130 has private pointer/constructor and exactly three
public methods:sql/expanded_sql/get_status. No safe opaque identity/equality/
hash/Deref/Clone; SQL methods violate scope and counters are not unique IDs.
PROFILE dispatch223–228 wraps the native pointer privately. Original consuming
Transaction::commit187–194 enters execute_batch548–555, internally preparing
COMMIT only after entry; no Statement is exposed before execution. Statement
fields14/native accessor759 are crate-private, with unsafe pointer access.
Callback wrapper addresses do not identify the actual statement.

Pinned SQLite can step internal statements during outer COMMIT: vdbeCommit90689
→virtual-table xSync162724–162739→fts5SyncMethod262259–262266→storage sync
263294–263296/265525–265538→save totals264760–264776→index averages258358–258361
→internal sqlite3_step/reset251545–251561. PROFILE eligibility94364–94366 has
no outermost-statement restriction; connection-wide dispatch93632–93633.
This proves possible nested callbacks, NOT the actual provenance of the11
observed callbacks. Do not select first/last/longest or subtract presumed FTS
callbacks. Existing aggregate repository commit timing remains valid; exact
checkpoint/fsync or internal split attribution remains unresolved.

## Byte-exact archived rejected revision

All10 frozen paths are archived before removal as gzip-compressed base64 text.
Decode base64 then gzip to recover original bytes, including line endings.
Root independently reads/decompresses archives in memory and SHA-256 verifies
all10 against frozen identities before code mutation. Archives are evidence,
not compiled modules, current source or release artifacts.

| Original path | Archive | Original SHA-256 |
|---|---|---|
| src-tauri/Cargo.toml | [src-tauri-Cargo-toml.gz.b64](qa-261005-rejected-commit-profile-source/src-tauri-Cargo-toml.gz.b64) | `70dc291c45d62f8fb05d944ad5a0b42d26eeb3468d38bfebd062c7c75d42960a` |
| src-tauri/src/storage/mod.rs | [src-tauri-src-storage-mod-rs.gz.b64](qa-261005-rejected-commit-profile-source/src-tauri-src-storage-mod-rs.gz.b64) | `eef87ebc2e4e831922aa7363bc601f5e1762a83732b59a05d58b8e5811fc562d` |
| src-tauri/src/storage/event_repository.rs | [src-tauri-src-storage-event_repository-rs.gz.b64](qa-261005-rejected-commit-profile-source/src-tauri-src-storage-event_repository-rs.gz.b64) | `fd5569f4ae6cd1870c3035092a5d2bd97b74e3e5ee6cffd296c9884f5d86b644` |
| src-tauri/src/storage/commit_profile_capture.rs | [src-tauri-src-storage-commit_profile_capture-rs.gz.b64](qa-261005-rejected-commit-profile-source/src-tauri-src-storage-commit_profile_capture-rs.gz.b64) | `b27ad6165b4f9d19c0bbb24c5a05163019d16edc2fbdb66ab14badffe625c8bb` |
| src-tauri/src/storage/commit_profile_evidence.rs | [src-tauri-src-storage-commit_profile_evidence-rs.gz.b64](qa-261005-rejected-commit-profile-source/src-tauri-src-storage-commit_profile_evidence-rs.gz.b64) | `5295200a9a3127adc82c31db8a06472db538c40a06539282d5d7c881b5c33d25` |
| src-tauri/src/storage/commit_profile_capture_tests.rs | [src-tauri-src-storage-commit_profile_capture_tests-rs.gz.b64](qa-261005-rejected-commit-profile-source/src-tauri-src-storage-commit_profile_capture_tests-rs.gz.b64) | `f72bfb9975a255e6c46c8183b2748f599053eb63ce198b39c3069e5b6e7addda` |
| src-tauri/src/bin/seed_history.rs | [src-tauri-src-bin-seed_history-rs.gz.b64](qa-261005-rejected-commit-profile-source/src-tauri-src-bin-seed_history-rs.gz.b64) | `1c3e38272f6e8f638d7e98d3ccfa7231971317d9ae1bf06c5870e5f138aca82e` |
| src-tauri/src/bin/seed_history_profile/seed_commit_profile.rs | [src-tauri-src-bin-seed_history_profile-seed_commit_profile-rs.gz.b64](qa-261005-rejected-commit-profile-source/src-tauri-src-bin-seed_history_profile-seed_commit_profile-rs.gz.b64) | `5bb8999eb44cdb10548bb994cfd630ddcd3c76e757fbf31f1b59d667bcd98c3f` |
| src-tauri/src/bin/seed_history_profile/seed_commit_profile_fixture.rs | [src-tauri-src-bin-seed_history_profile-seed_commit_profile_fixture-rs.gz.b64](qa-261005-rejected-commit-profile-source/src-tauri-src-bin-seed_history_profile-seed_commit_profile_fixture-rs.gz.b64) | `14d467164a9572bb70e5e12c5df1d768d780ad73843f59f209fb390db3785c66` |
| src-tauri/src/bin/seed_history_profile/seed_commit_profile_corpus.rs | [src-tauri-src-bin-seed_history_profile-seed_commit_profile_corpus-rs.gz.b64](qa-261005-rejected-commit-profile-source/src-tauri-src-bin-seed_history_profile-seed_commit_profile_corpus-rs.gz.b64) | `912f22b31eb3c87f2e5e7203c626e72e5360a01c4a003450437fe16ba266152e` |

Rollback owns only devtools trace forward, storage export, commit-arm wrapper,
private seeder fields/module/branch and six new experiment-only source files.
Existing PAGE/recurrence/insertion recorder/grouped sources/CLI/batch1000/
SQL/WAL/settings/personal history remain untouched. Archive validation PASS10/10;
no broad Git reset, source checkout, callback heuristic or threshold change.

Post-rollback regression checks pass176 cases with six deliberate ignores;
both strict release Clippy configurations and formatting pass, restored owner
hashes exact and independent review/simplifier clean. Old and rebuilt artifact
identities are kept separate. [Terminal regression ledger](qa-261005-commit-profile-rollback-verification.md).
This does not make the rejected eight-case prototype pass or close performance.

Unresolved questions: seed performance,
commit-internal cost and the remaining plan/native/release obligations.
