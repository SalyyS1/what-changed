# Fixed-five seed grouping correctness — 2026-10-05

Status: correctness/callers and complementary strict Clippy verified.
No grouped performance result or default promotion.
Lineage: [accepted extraction/design](qa-261005-seed-transaction-grouping-prototype.md)
and [complete stage-one verification](qa-261005-insertion-body-verification-output.md).
The rejected 5,000-event batch remains rejected; default logical batch is 1,000.
The profiler-free 1M seed remains 316,438.748 / 240,000 ms — MISS.

## Authorized ownership

Worker owns only `src-tauri/src/bin/seed_history.rs` and new private bin-local
graph/grouping/timing/parity/fixture modules. Library, Db, writer, schema,
settings, SQL, fixture recipe and existing negative proof remain unchanged.
Root owns formatting, execution, review coordination and evidence. No commits,
native application/history access, timing harness/run or default promotion.

## Frozen design

- Test-only route prepares at most five independent 1,000-event units outside
  the writer, preserving each event-body→graph-body order in one existing
  `Db::write` transaction. The final partial unit/group commits normally.
- Owned/known sets, cloned 3,000 relationship ordinals, inserted count and
  progress publish only after actual commit. Errors abort immediately without
  later units or reusable provisional state. Up to five units roll back:
  this changed failure contract and allocation bound are explicit, not accepted
  production collector or memory/performance behavior.
- Extracted private graph body preserves legacy transaction wrapper labels,
  SQL, order, sanitization, missing-endpoint and real-entity rules.
- Distinct grouped records separate completed body input from committed input,
  actual outer commit from graph/body work and graph-inclusive group total from
  legacy repository-only totals/residual. Admission errors are not attempts.
- Keep default rollback-on-drop. Explicit COMMIT through the live transaction
  retains it for autocommit-aware error rollback; preserve primary and optional
  cleanup errors privately. Publish failure timing after cleanup/disposal.
  Cleanup failure aborts; disposal alone never proves rollback succeeded.

## Correctness gates before timing consideration

Compare complete persisted 10k and 11,001-row baseline/grouped corpora: events,
entities, relationships, checkpoints, snapshots, investigations, analysis
sessions/results, exact physical IDs/order and owned sets. Retain the canonical
cross-boundary witness. Validate original marker ownership, null/present
structure and execution-audit time ordering before path-specific normalization.
Only exact own registered marker references and their settings audit timestamp,
investigation/suspects creation audit time and session execution audit times are
allowed. Evidence/recipe times, scores, weights, chains, relative text and IDs
remain exact; no recursive field stripping or arbitrary string replacement.

Tests must cover final tail, real collisions/repeated owned updates, event and
graph errors, actual deferred commit error after successful body work, earlier
committed-group preservation, accurate counters/progress, no leaked state,
registered purge preserving real evidence and conservative unknown entities.
RAII fixtures close/join writers before deleting only validated exclusive
workspace children, preserving siblings on normal return and unwind.

Any unexpected verification difference/red stops for counsel before diagnosis
or edits. Narrow checks, affected callers, complementary Clippy, independent
review/simplifier and source/artifact freeze precede a separate timing decision.
No phase/checklist/release promotion follows implementation alone.

## Current execution

Root formats the seeder and 14 new private modules; full rustfmt/scoped diff
checks pass. The first focused compile fails on unsupported usize FromSql and
ambiguous error conversion. [Complete failure/counsel record](qa-261005-fixed-five-compile-failure.md)
retains exit 1/no tests. Counsel precedes two exact type repairs: read i64 and
checked usize conversion; explicit DbError→AppError→GroupFailure conversion.
No SQL, assertion, fixture, timing boundary or error handling is weakened.

Repeated focused release/devtools command passes 13/13, zero failures/ignores,
four filtered in 40.20 s after 1m02s compilation. Complete seeder run explicitly
includes its retained negative proof: 17/17, zero failures/ignores in 34.71 s,
cached preparation 0.66 s. The old 5,000 batch still differs only in one
first_seen by 1,710,891,089 ms; it remains rejected, not a successful candidate.
[Complete correctness output](qa-261005-fixed-five-correctness-output.md).

Independent frozen read-only reviewer and conditional simplifier are clean,
with no edits/execution claim. Live gate: 7,629 text LOC / 145 tracked files
(16 binary) / maximum added 584. Root verifies unchanged source hashes/scoped
shortstat after both delegates. New sources are untracked; Git shortstat alone
is not their identity proof. Group/proof fixture prefixes and owned runners are
absent after the terminal run, before caller verification starts.

Seeder unit executable SHA: `D06D88245AD94BA52778399E706A9F887D15FEB4CA1788F5EFB7042FB01FCD28`.
Source seeder SHA: `7433E762232181411AB9C5258A46BC26050B7F32695958963C0308C92B6DCC08`;
corrected parity module: `45C351A043FBEC7C2207407D2A3E797C64E65F771F59068867B29A4D9FB64627`;
corrected transaction: `A4852E337BF232848026B55330AED662AC835BD09A7C0087CE776670C47FD9DD`.
Library extraction/recorder and old proof/fixture hashes remain unchanged.
Root owns caller session 14815, Cargo PID 19392 / rustc PID 20312 at
2026-10-05 01:58 UTC, workspace `D:\Project\.WhatChage`, no port. One Cargo/job.
Caller command selects existing purge/insertion-profile integration suites;
its diagnostic remains ignored. No seed measurement/native process is started.

Caller session 14815 exits 0: purge 9/9 in 8.91 s, insertion profile 2/2
in 0.41 s with one diagnostic ignored; release compilation 51.76 s. The first
all-target/all-feature Clippy fails on the 160-byte private error variant.
[Complete lint failure/counsel](qa-261005-fixed-five-clippy-failure.md) preserves
the failed gate. Counsel approves one boxed typed primary/optional rollback
payload behind the unchanged result wrapper, allocating only on failure.
Root changes only failure representation/accessors and their two test owners;
full formatting passes. Focused six failure cases pass in 5.59 s after 50.65 s
compilation. Strict all-target/all-feature Clippy passes in 2.03 s; normal
no-default-feature lib/bins Clippy passes in 0.79 s. Independent three-file
amendment review is clean. Unchanged full positive parity/default caller reruns
are unnecessary for this representation-only fix. The pre-repair unit artifact
hash above is historical; [final output and current artifact identities](qa-261005-fixed-five-final-verification-output.md)
record the terminal run and cleanup/source checks.

Counsel accepts correctness and approves bounded private timing-harness
implementation, with review before execution. Exactly one 10k pair then one
fresh canonical 1M pair are permitted, baseline first, fixed recipe/settings/
instrumentation and equivalent complete-seed clocks. Each owned fixture is
validated and streamed into deterministic ordered content digests, then closed,
joined and removed before its counterpart begins. Compare counts/digests using
only the reviewed normalization; stop on mismatch. Digest/report/cleanup stay
outside the seed clock. Check conservative single-fixture disk/WAL/temp/build
headroom before each run. No size sweep, rerun, default promotion, statistical
confidence or causal claim follows a single fixed-order pair.

Unresolved questions: memory cost and timing benefit;
remaining independent acceptance gates. No phase/checklist/release promotion.
