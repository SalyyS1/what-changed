# Checkpoint event-window actual-bound capture ledger — 2026-10-05

Status: source frozen; focused10, full-library535 and caller7 PASS, zero failed;
both strict release Clippy configurations and formatting PASS. Exactly one
counseled canonical10k event-window smoke PASS; no whole-service proof,
fresh latency acceptance or acceptance closure claimed.
Full plan stays314/369,55 open/P20 6/23/public-v1 NO-GO.

## Accepted boundary

Capture only `checkpoint_repository::event_ids_in_window` as actually called
through `CheckpointService::compare(left, Some(right))`. Keep original fixed
SQL, normalized inclusive endpoints, timestamp/id ordering, three actual binds
and hard10,000-ID cap; preserve original AppResult and complete collection.
Preparation/query-start/full-decoding evidence is separate from EQP success.
Unknown vocabulary or EXPLAIN errors must not change the actual result.

Reuse the existing pinned parameter-free events EQP projection through one
crate-private forwarding wrapper inside the existing devtools PAGE module.
Counsel approves this narrowly expanded ownership instead of copied parser
logic. No PAGE capture behavior/vocabulary or shared PerfFixture changes.
Three Value objects exist only during active capture from the same tuple used
by actual query_map; parameter_count is not statement-identity proof.
Normal builds compile the observer out. No raw SQL/binds/errors/keys/times
persist in evidence; no tracing hooks, unsafe identity, environment switch,
index/schema/SQL/settings/seeder/threshold or production network changes.

## Ownership and verification sequence

Worker owns checkpoint_repository.rs, storage/mod.rs and only the forwarding
wrapper in page_plan_capture.rs; new checkpoint-window capture/evidence/owner
and lifecycle test modules plus one ignored canonical profile target. Root
owns formatting, frozen-source review, focused tests, relevant callers, strict
lint and further counsel. Worker performs source edits only, no commands that
build/test/run workloads. Existing dirty-tree changes stay untouched.

The proposed canonical10k smoke uses unchanged owned PerfFixture/AppState and
actual compare(Some), numeric/typed evidence, actual returned IDs and cleanup.
It remains ignored until implementation tests/review/lint pass and root obtains
execution counsel. No 100k/1M or p95 run is authorized by this preparation.

## Proof limits

Independent frozen-source reviewer and conditional simplifier both CLEAN,
no edits; all eight hashes verified. Global live signals7883textLOC/147files/
16binary/maxadded664 trigger scoped simplification, not unrelated-tree cleanup.
Root's out-of-timestamp insertion order strengthens the same ordering case.
Tested full-library executable `what_changed_lib-708106f6a2e3e24a.exe` hashes
`ba4e58cb1d4212aa1b917359d68d91022bc31ca6d4cf6dafe2620f58062d78d8`.
PAGE projector/evidence and shared PerfFixture still match their pinned hashes;
old PAGE/rollback executable hashes remain distinct historical checkpoints.
Focused release build6m04s,10 passed3.00s; caught unwind is intentional.
[Complete focused output](qa-261005-checkpoint-window-focused-output.md).
Full release/all-feature library535passed/3deliberateignored,538total records,
23.54s; cached build0.86s. Complete55,075-character stdout retained before
rendering and split into consecutive Markdown presentations:
[part1](qa-261005-checkpoint-window-library-output-1.md),
[part2](qa-261005-checkpoint-window-library-output-2.md),
[part3](qa-261005-checkpoint-window-library-output-3.md),
[part4](qa-261005-checkpoint-window-library-output-4.md).
Independent terminal review finds one precision issue: concatenated fenced
payloads include one final Markdown separator LF (55,076 characters). Counsel
approves provenance-only qualification, not a test rerun. Root verifies exact
retained text equality after removing only that one final LF, with all538
records/3 ignores present. No broad whitespace trim or byte-identity claim.
All other evidence, hashes, links, chronology and proof limits check clean.
The independent tiny follow-up is CLEAN/no concerns; counsel GO closes this
bounded implementation/verification/evidence slice only. Next is a read-only
whole-compare coverage map, not another smoke or automatic checkbox closure.
[Coverage map and preparation boundary](qa-261005-checkpoint-compare-coverage-map.md)
identifies2 metadata/2 item/1 window reads for successful Some(right), with
distinct original error short-circuits and three reader leases. Counsel permits
one cohesive envelope's preparation only; no full-service/scale execution yet.

Caller build12m42s; checkpoint_compare4 and owned-fixture3 pass, one canonical
diagnostic deliberately ignored. [Complete output](qa-261005-checkpoint-window-callers-output.md).
Both strict release Clippy configurations pass: all-target/all-feature2m20s
and normal no-default lib/bins21.30s. [Complete lint output](qa-261005-checkpoint-window-lint-output.md).
Root full formatting/scoped diff pass, eight source hashes rechecked exact;
no owned Cargo/rustc/link/caller runner or what-changed-perf-* fixture remains.
Other unrelated .perf-temp entries remain untouched.
Tested comparison executable SHA-256
`e594fdb1928fe95de24acc867912fd36ddc0ea4c24bc579c6ab52d70cb9f0608`;
compiled checkpoint-window profile executable SHA-256
`111cd9c1416869f1d1e4cfafcafd7681b10785f4033cdc4afc0fd34ef3e90717`.
These are this revision's identities, not historical PAGE canonical artifacts.

Execution counsel GO precedes exactly one canonical10k smoke, terminal PASS
4.61s. Actual compare(Some) returns full inactive/captured equality; one window
invocation prepares3 binds/parameters, starts and completely decodes245 IDs.
Complete typed plan: covering SEARCH idx_events_timestamp, lower/upper timestamp
constraints. [Complete output/provenance](qa-261005-checkpoint-window-canonical-output.md).
The first preflight fails on relative .qa before any workload; root stops for
counsel, substitutes only the absolute ancestor, then identical guards admit
the one diagnostic. [Failure/correction receipt](qa-261005-checkpoint-window-preflight-correction.md).
No source/assertion/threshold change or workload retry. Post-exit runner/owned
performance fixture counts zero; artifact/fixture hashes unchanged.
Documentation check passes33 Markdown files; journal validation PASS. Scoped
Initial11-report link check finds23 existing local targets, and scoped diff whitespace
checks pass. Fresh25-phase sweep remains314 done/55 open/369; all eight source
hashes remain exact after documentation edits. No commit/publish or native
installation/launch/account mutation performed. AgentWiki publish skipped.

This can prove event-window preparation/binds/execution/consumption, not
checkpoint by_id/items reads, compare item processing or the whole comparison
shape. PAGE first/tenth canonical10k receipts are separate historical artifacts.
Current rolling recurrence has actual-bound unit proof; canonical COUNT receipts
from a prior full-count revision do not prove current rolling service capture.
Other full dashboard/search/details/investigation/privacy shapes remain open.
Native checks independently await separate standard-account name/profile and
safe interactive route; current-user AppData remains untouched. Fresh release
artifact/license/notice approvals remain independent prerequisites.

[Closed rejected-technique rollback](qa-261005-commit-profile-rollback-verification.md).

Unresolved questions: remaining full-service/branch/scale plan and frozen-RC
latency/native/release acceptance; separate test-account name/profile.
