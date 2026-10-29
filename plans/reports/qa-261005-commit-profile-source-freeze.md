# COMMIT PROFILE source freeze — 2026-10-05

Status: archived rejected revision, no longer compiled. Focused8-case command FAILS0/8 after successful
compilation. Successful insertion COMMIT yields11 scoped PROFILE callbacks,
so the exact-one assumption is empirically falsified. No valid split or
workload result claimed. Initial inline review found no concrete source defect
before execution; this new evidence blocks diagnostic eligibility. Independent reviewer/simplifier/advisory
followups unavailable due runtime agent thread limit, disclosed separately;
do not label them CLEAN/completed for this slice.

[Protocol](qa-261005-commit-profile-source-ledger.md),
[feature isolation](qa-261005-commit-profile-feature-isolation.md).

Exactly10 paths, after owned rustfmt/full fmt/scoped diff PASS:

| Path | SHA-256 |
|---|---|
| src-tauri/Cargo.toml | `70dc291c45d62f8fb05d944ad5a0b42d26eeb3468d38bfebd062c7c75d42960a` |
| src-tauri/src/storage/mod.rs | `eef87ebc2e4e831922aa7363bc601f5e1762a83732b59a05d58b8e5811fc562d` |
| src-tauri/src/storage/event_repository.rs | `fd5569f4ae6cd1870c3035092a5d2bd97b74e3e5ee6cffd296c9884f5d86b644` |
| src-tauri/src/storage/commit_profile_capture.rs | `b27ad6165b4f9d19c0bbb24c5a05163019d16edc2fbdb66ab14badffe625c8bb` |
| src-tauri/src/storage/commit_profile_evidence.rs | `5295200a9a3127adc82c31db8a06472db538c40a06539282d5d7c881b5c33d25` |
| src-tauri/src/storage/commit_profile_capture_tests.rs | `f72bfb9975a255e6c46c8183b2748f599053eb63ce198b39c3069e5b6e7addda` |
| src-tauri/src/bin/seed_history.rs | `1c3e38272f6e8f638d7e98d3ccfa7231971317d9ae1bf06c5870e5f138aca82e` |
| src-tauri/src/bin/seed_history_profile/seed_commit_profile.rs | `5bb8999eb44cdb10548bb994cfd630ddcd3c76e757fbf31f1b59d667bcd98c3f` |
| src-tauri/src/bin/seed_history_profile/seed_commit_profile_fixture.rs | `14d467164a9572bb70e5e12c5df1d768d780ad73843f59f209fb390db3785c66` |
| src-tauri/src/bin/seed_history_profile/seed_commit_profile_corpus.rs | `912f22b31eb3c87f2e5e7203c626e72e5360a01c4a003450437fe16ba266152e` |

Core lifecycle/evidence and corpus/fixture/harness already have real separate
boundaries. The coherent198-line lifecycle owner and328-line eight-case test module are
considered for modularization; no arbitrary split obscuring shared TLS/RAII
invariants or unrelated seeder/grouped refactor is introduced.
Live simplifier gate7814 text LOC/146 tracked files/16 binary/max-added634
exceeds default400/8/200; no .ck.json or disabling environment switch.
Independent simplification remains unavailable, not completed. Root inline
review retains the small fixed lifecycle/evidence and separate fixture/corpus
boundaries without behavioral source amendment. A report-only line-count patch
first rejected a mismatched context; one-line retry corrects198/328 counts,
with no code or tested-revision change. Eight pre-existing grouped owner
hashes also remain exact after owned rustfmt.

Root reads all new owners/tests and changed feature/export/insertion/default
seeder branch, plus pinned APIs and original insertion contract. No generic
public callback, unsafe FFI, SQL inspection, WAL-hook replacement, checkpoint/
PRAGMA/settings or batch-size change. Callback entry count for proving actual
unregistration is test-only; runtime retains numeric monotonic fields only.
Inactive owner hook reads no diagnostic clock; normal build excludes capture.
Failed consuming COMMIT and cleanup rollback remain failed evidence, never
valid splits. Synthetic zero arm/return and real two scalar callback lifecycle
cases are not described as actual-owner zero/multiple COMMIT proofs.

All successful/failed/invalid counts are reported independently. Harness
cardinality anomalies yield incomplete split evidence, even if corpus checks
pass; do not fabricate a valid observation or equate corpus PASS with tracing
PASS. Complete numeric corpus validation is not full cross-route persisted
content/digest equality. Existing default1,000 event→graph workflow, CLI and
coarse outputs remain unchanged; private options default false/cfg(test).

Unchanged legacy insertion timer, transaction tests and canonical fixture:
`33414be9f6393a03189bcf856da7153719829d6345e189910acdf0dc15839c24`,
`cc9819cbd9dd95c6ed65214dbfe29297cd8071c7bcecf1e7022a385e1c11b542`,
`c34d037eeae47ad86ac91e7e1907c4d3540c4d101adab592ca018aea49ba1782`.
PAGE smoke source remains5455903c33e72504cc3abe746fe6e3f2151e6af7bb736523b36a11e4c5c5f92e;
shared mod.rs/event_repository whole-file identities legitimately change,
so previous PAGE receipts remain historical terminal evidence.

Focused command `cargo test --release --locked --features devtools --lib storage::commit_profile_capture:: -- --nocapture --test-threads=1`.
One build job/existing release target/workspace-owned TEMP/TMP. Session81397,
Cargo12412/rustc14940 observed; build references pinned trace-enabled rusqlite.
No native app, installer, account modification or personal history access.

Unresolved questions: focused/full/caller/lint gates, current source/artifact
rehash, independent review limitation and exactly one10k/one1M diagnostic.

Terminal first focused command exits1:5m12s compile,0.16s run,8 failed/528
filtered. [Complete failed output](qa-261005-commit-profile-focused-failure-output.md).
Root STOP obtains fresh counsel before diagnosis or edits. Counsel permits
one bounded read-only pinned API/internal-statement feasibility audit only;
never guess first/last/longest callback, subtract callbacks or weaken the
exact identified-commit rule. No further tests/lint/caller/10k/1M runs yet.
The callback recorder's invalid_multiple result remains honest failed evidence.
The read-only audit finds no safe pre-execution COMMIT identity. Counsel GO
rejects the technique and approves only byte-exact archive plus experiment-only
rollback. [Rejection, source map and all10 validated snapshots](qa-261005-commit-profile-rejection.md).
Six new files are retired from compiled source after archive verification;
four shared owners restore their pre-experiment state. This hash table describes
the rejected historical revision, not current paths/source or successful tests.
