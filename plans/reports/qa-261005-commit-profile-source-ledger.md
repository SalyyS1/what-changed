# Commit PROFILE source ledger — 2026-10-05

Status: PROFILE technique REJECTED after focused gate FAILS:11 callbacks during
successful insertion COMMIT and no safe pre-execution opaque identity. All8
prototype failures remain preserved; source archived/experiment-only rollback
complete, regression gates PASS176 cases plus complementary lint. No10k/1M PROFILE workload or replacement
diagnostic authorized. No checkpoint
probe, pragma, settings or default-route change authorized.
The [fixed seed pairs](qa-261005-fixed-seed-pair-harness.md) remain diagnostic;
grouping is not promoted and profiler-free seed acceptance still misses240s.

## Verified pinned source boundaries

Read-only scout uses locally pinned rusqlite0.40.2 and libsqlite3-sys0.38.2
bundled SQLite3.53.2. Line references below are those exact registry sources,
not API claims about another release. No fixtures, personal history, native
app, database queries or build processes were accessed by the scout.

| Source | Verified behavior |
|---|---|
| SQLite `sqlite3.c:94430–94437` | Normal successful sqlite3Step calls checkProfileCallback before doWalCallbacks. |
| SQLite `sqlite3.c:189674–189679` | Default WAL hook invokes passive sqlite3_wal_checkpoint at the configured1000-page threshold and returns SQLITE_OK. Replacing WAL hook replaces automatic checkpoint behavior; it is not a passive observer. |
| rusqlite `transaction.rs:187–194` | Consuming Transaction::commit calls commit_, which executes the explicit COMMIT batch. |
| rusqlite `lib.rs:548–566` | execute_batch prepares/steps each statement and drops it before returning. |
| rusqlite `statement.rs:688–691,779–784`; `raw_statement.rs:235–238` | Statement destruction finalizes via sqlite3_finalize. |
| SQLite `sqlite3.c:93656–93672,93685–93698` | Finalize/reset can invoke pending PROFILE before Vdbe reset/deletion. |
| SQLite `sqlite3.c:93635` | Successful normal PROFILE clears startTime, so finalization should not duplicate that callback. |
| rusqlite `transaction.rs:220–228,243–246` | A failed consuming commit can drop its transaction and execute rollback before the caller regains control. Error/cleanup callbacks must not be counted as a successful commit observation. |
| SQLite `sqlite3.c:93625–93626,55063–55068` | Native PROFILE duration uses wall-clock milliseconds scaled to nanoseconds. Do not treat it as a high-resolution monotonic split. |

Existing repository insertion timers cover the whole tx.commit call, not these
internal intervals. Source ordering offers a diagnostic distinction; it does
not establish which interval dominates in this workload.

## Safe API and feature boundary

Project `src-tauri/Cargo.toml` enables bundled/backup/functions; devtools is
empty. Pinned rusqlite defaults/bundled do not enable trace or hooks. The only
other resolved consumer, rusqlite_migration, requests no extra features and
disables default features. Safe trace_v2 is gated by trace in rusqlite
`src/lib.rs:150–151`; it is unavailable in the current configured build.

A future explicit proposal may forward rusqlite/trace through devtools only.
Normal-build resolved features must be verified unchanged before accepting
that delta. No unsafe FFI substitute is authorized. trace_v2 accepts an
Option<fn(TraceEvent)> callback rather than a capturing closure; its PROFILE
branch avoids SQL conversion/allocation and catches unwinding. It replaces the
single trace registration and offers no getter to restore an unknown callback.
Therefore use only an exclusively owned fresh diagnostic writer known to have
no trace callback, reject nesting, and unregister reliably on return/unwind/close.

## Conditional bounded diagnostic protocol

Arm thread-local numeric/monotonic state immediately around explicit COMMIT,
without inspecting SQL, expanded statements, parameters or raw errors.
Require a successful application commit and exactly one scoped PROFILE callback
for a valid observation. Zero/multiple callbacks and every failed commit yield
incomplete/error observations, never fabricated valid splits. A failure can
include rollback profiling while the consuming commit call is still unwinding.

Use monotonic Instant at commit start, callback and caller return. Report
commit-start→PROFILE and PROFILE→commit-return separately. The second interval
includes the WAL callback and other SQLite/callback/finalization/drop completion
overhead; neither interval is exact checkpoint/fsync time. Keep callback
non-panicking/allocation-free, clear state on errors/unwind, and retain only
aggregate integer counts/durations. Instrumentation can perturb timing.

Do not replace WAL hooks, add NOOP/checkpoint probes or change pragmas.
SQLite supports CHECKPOINT_NOOP without checkpoint copying/lock, but it can
change connection error/interrupt state and is excluded from this minimum route.

After feature/source review, focused disabled/lifecycle/error tests and ordinary
callers, counsel permits one small smoke and exactly one canonical1M diagnostic
on the unchanged default1,000-event route, aggregating all successful commits
and reporting invalid observations separately. Retain existing corpus and
owned-fixture cleanup checks. No grouped pair or acceptance matrix rerun.

A large post-PROFILE tail supports investigating checkpoint-related completion
work, not a causal proof that checkpoints dominate. Root owns feature isolation,
implementation gates and subsequent counsel; the page worker does not edit
Cargo.toml or shared insertion instrumentation for this future slice.

## Accepted existing-writer-job boundary

Root reads pinned trace_v2 signature: it takes `&self`, so the existing
Db.write job's `&Connection` can safely register tracing. No writer constructor
or job contract change is needed. Counsel approves a fixed insertion-only
devtools entry point, not generic trace registration or arbitrary callbacks.
The separate seeder target needs narrow library access; doc(hidden) is not
access control, and fresh exclusive QA/no prior tracer remains an essential
caller precondition. Scope rejection must precede trace replacement.

Implementation owns only the devtools feature forward, gated capture export,
actual consuming insertion COMMIT hook, numeric lifecycle/evidence tests and
cfg(test)-only default seeder diagnostic branch/harness. Default1,000 logical
event→graph sequence and CLI surface stay unchanged. Separate sentinel-aware
owned fixture and numeric corpus validation do not claim full cross-route
persisted digest equality. Exactly one ignored10k and one ignored1M run may
follow review/lifecycle/error/feature-isolation gates; none has run yet.

Root records normal Windows resolved rusqlite features before the change:
backup/bundled/cache/default/ffi-sqlite-wasm-rs/functions/hashlink/modern_sqlite,
no trace/hooks. The normal feature tree must remain unchanged afterward.
Root's before/after check now confirms byte-identical normal tree; devtools
adds trace only, with hooks absent. [Complete feature output](qa-261005-commit-profile-feature-isolation.md).
Compilation/lint/runtime lifecycle evidence is still pending.

Two source/report apply_patch attempts rejected mismatched context before any
implementation patch applied. Worker read-only verification confirms devtools
still empty and existing module still private. Root's advisor followup and new
kongming spawn both fail `agent thread limit reached`; missing counsel is
disclosed, not claimed complete. Only context-only retries within the already
approved design proceed; no failed test/lint/build is hidden or bypassed.

## First focused verification miss and counsel

Compilation succeeds5m12s, then all8 focused cases fail0.16s on the same
clean-success cardinality expectation: attempts/success/valid/callbacks
observed1/1/0/11 versus expected1/1/1/1. The recorder correctly labels
multiple callbacks invalid. Thread join has a downstream child-panic failure;
no source repair or unchanged rerun follows. [Complete failure](qa-261005-commit-profile-focused-failure-output.md).

The new objective failure checkpoint successfully invokes counsel despite the
earlier runtime limit. It permits exactly one source audit: safe public callback
statement identity, SQLite commit-time internal/FTS PROFILE paths and ability
to match actual COMMIT before unchanged Transaction::commit executes. Internal
callbacks are plausible, not proven as the actual cause yet. No first/last/
duration guessing, SQL inspection, unsafe identity or callback subtraction.
If safe identity is absent, reject this technique under current constraints;
changing execution path or SQL inspection requires a new reviewed design.

The bounded audit confirms safe public StmtRef exposes only SQL/expanded_SQL/
status, no opaque identity; unchanged consuming COMMIT internally prepares its
statement after entry. Internal FTS stepping can emit PROFILE during outer
COMMIT, but does not attribute these particular11 callbacks. Counsel rejects
the technique, not the exact-one rule. All10 rejected source snapshots decode
and SHA-match before six experiment-owned files are retired. Shared storage
owners and seeder restore exact prior hashes; devtools=[] has no trace/hooks.
[Source map, snapshots and rollback](qa-261005-commit-profile-rejection.md).
Existing aggregate commit timing is retained; no first/last/longest selection,
count subtraction, SQL inspection, unsafe pointer or execution-path change.

Post-rollback storage106/seeder23/callers47 pass, six deliberate ignored
diagnostics/proofs; both strict release Clippy and formatting pass, source
identities exact and independent rollback review/simplifier CLEAN, no edits.
[Terminal restored-source receipts](qa-261005-commit-profile-rollback-verification.md).

Unresolved questions: seed performance and
commit-internal work. Exact provenance of11 callbacks remains untraced.
