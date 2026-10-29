# Seed transaction grouping prototype — 2026-10-05

Outcome: separately approved design stage, not implementation/default change.
The [5,000-event batch negative proof](qa-261005-seed-batch-size-experiment.md)
demonstrates one semantic entity timestamp difference; that candidate remains
rejected. This prototype instead preserves each existing logical 1,000-event
event→graph sequence, grouping exactly five such units per outer transaction.
No size sweep, SQL/settings/fixture cut or production collector batch change.
The profiler-free seed remains 316,438.748 / 240,000 ms — MISS.

## Approved boundaries

Counsel finds current commit attribution sufficient to test consolidation;
deeper SQLite instrumentation is not a prerequisite or a proven internal cause.
First source audit the exact API, then extract the unchanged repository body
behind an existing transaction. Original wrapper keeps begin/commit, error
labels, SQL, sanitization, event/entity order and recorder behavior unchanged.
Graph body likewise separates from its original transaction wrapper without
changing the graph algorithm. A concrete devtools transaction-aware helper is
allowed; a general transaction callback framework or writer redesign is not.

Grouped seeding runs in one existing writer job, executing event body then
graph body for each unchanged logical unit. No nested blocking writer call.
Final partial unit/group commits normally. Existing default route remains
selected until separately reviewed promotion. No public experiment CLI option.
Keep the rejected batch's negative proof as a regression.

## State and timing contracts

Ownership, known-entity sets, relationship ordinals and progress are provisional
until the outer commit succeeds. A failed group must not expose reusable state
claiming rolled-back entities exist. Emit progress only for committed groups.
The proposed failure contract intentionally rolls back up to five logical
units together. It is suitable only for disposable QA seeding, not an assertion
that the old committed prefix or production collector contract is unchanged.

Inner insertion success is NOT commit success. Use a distinct body timing type
without committed counters, preserving the legacy recorder. Record actual
outer commit time, attempted/successful groups and committed input rows
truthfully, including partial failures/rollback. Do not silently redefine
the existing insertion record or call a graph-inclusive outer duration the
old repository-only caller residual. No per-row clocks/identities/raw errors.

## Staged implementation and verification

1. Root approves exact helper/timing API after source audit and counsel.
   Extract repository body only; legacy focused recorder, rollback/error,
   ownership and affected repository/writer callers must still pass.
   Add transaction-aware success/error tests proving no inner commit and
   outer rollback. No grouped seeder behavior change in this stage.
2. Extract graph body and implement private test-only fixed-five grouped
   route. Compare complete baseline/grouped persisted events/order/entities,
   relationship evidence and ownership, including the exact crossing witness.
   Then compare checkpoints/snapshots/investigations with only source-justified
   inherent random IDs/markers/wall-clock normalization. No semantic time,
   relationship evidence or score normalization.
3. Cover nondivisible final unit/group, real entity collisions, repeated
   updates, event failure, graph failure and deferred outer-commit failure.
   Prove whole-group rollback, truthful counters/progress, no provisional-state
   reuse, exact synthetic-prefix purge and preservation of real evidence.
   Preserve conservative existing purge behavior for unprovable entity owners.
4. Close/join writers before deleting only exclusive canonical workspace
   children; verify normal/unwind/sibling cleanup. No user history access,
   native launch, schema mutation or manual broad cache deletion.
5. Format, narrow checks, affected caller/library/complementary Clippy, read-only
   independent review and conditional simplifier precede source/artifact freeze.
   No conflicting owner edits or simultaneous Cargo/performance processes.

## Fixed measurement decision

After parity/review, one paired 10k smoke, then one fresh canonical 1M for
baseline → fixed-five grouping, sequential owned fixtures and same release/
settings/seed/end/span/recipe/explicit instrumentation. Preserve every
aggregate output/exit/hash and cleanup observation. Do not compare to an old
unmatched attribution as if it were the experiment baseline.

Measure total time, actual commit attribution and bounded allocation/memory
implications. Neither the commit fraction alone nor a speedup still above
240 seconds closes the seed gate. No outlier exclusion, rerun seeking green,
third group size or settings tuning. Losing/ambiguous evidence stops this
experiment; promotion requires full parity, acceptable changed-failure/memory
contract, material total benefit and fresh counsel. After promotion, one
unchanged profiler-free canonical seed acceptance is required.

Any unexpected failed verification stops for counsel BEFORE further edits or
diagnosis. No phase/checklist/release promotion from this proposal.

## Current evidence

Read-only worker API audit is complete; existing `Db::write`/writer job suffice,
with no writer redesign. Counsel approves STAGE ONE extraction only:
private body requires `&Transaction`; narrow devtools entry consumes the
ownership set and returns `AppResult<(ids, owned)>` only on successful body
completion. Separate optional body timings remain available on errors, with
no committed fields. Successful state is still provisional; every body error
requires abandoning the outer transaction, not committing partial SQL.

Empty input returns ownership unchanged, executes no SQL and reports zero
work. Original wrapper keeps its early empty return, transaction/error labels,
prepared statement lifetimes and six-clock/four-phase timing. Stage-one tests
must prove no inner commit, outer rollback, no ownership returned on failure,
real-entity preservation and disabled attribution clocks before group edits.

Worker owns only insertion portions of `event_repository.rs`, distinct body
types/finish in `insertion_timing_capture.rs`, and new private transaction tests.
Seeder/graph/query/recurrence/schema/writer/Db and existing recorder tests stay
untouched. Root owns formatting/execution/review/evidence. Stage-one source is
frozen and root-formatted; full rustfmt/scoped diff checks pass. Seven new
real-transaction cases cover outer rollback/commit, full row/entity preservation,
mid-body/preparation/empty errors, repeated ownership updates, disabled clocks
and caller unwind. The unchanged seven recorder tests remain the legacy check.
Focused combined 14-test release/all-feature run passes: zero failures/ignores,
501 filtered, 0.44 s after 4m22s compilation. Full release/all-feature library
passes 512, zero failed, three explicit ignores, 515 total in 20.14 s;
cached preparation 0.65 s. No coverage percentage measured. Complementary
release Clippy passes with warnings denied: all-target/all-feature 1m13s,
normal lib/bins without defaults 13.95 s. Affected callers also pass: ordinary
seeder 3/3 (negative proof ignored), repository 24/24, purge 9/9 and insertion
profile 2/2 (diagnostic ignored); compilation 12m01s, tests 0.11/11.02/6.88/0.11 s.
independent read-only review and conditional simplifier are clean on the frozen
revision, with no edits or executed-check claim. Live gate signals are 7,605
text LOC / 145 tracked files (16 binary) / maximum added 568. Simplifier ran
clean; root verifies identical source hashes/scoped diff afterward.
Counsel accepts stage one and approves bin-only stage-two graph extraction,
fixed-five grouping and correctness tests after this green caller gate.
Worker implementation is authorized; no timing harness/run or default change.
[Stage-two correctness record](qa-261005-fixed-five-seed-grouping.md) owns later execution.
All five stage-one source hashes remain unchanged after caller execution.
[Complete verification output](qa-261005-insertion-body-verification-output.md).

Library unit executable `what_changed_lib-708106f6a2e3e24a.exe` SHA-256:
`F84A0A51EBFFAF3A5C14B3FA42CDBDEE5DBC3348074925B388844DD5318F8374`.

Stage-one SHA-256, paths relative to `src-tauri/`:

| Source | SHA-256 |
|---|---|
| src/storage/event_repository.rs | 38E9EF8D182AA9CA18218E45484FE5A6D84104925E9FF058B8AAF2E42E62D953 |
| src/storage/insertion_timing_capture.rs | 33414BE9F6393A03189BCF856DA7153719829D6345E189910ACDF0DC15839C24 |
| src/storage/insertion_transaction_tests.rs | CC9819CBD9DD95C6ED65214DBFE29297CD8071C7BCECF1E7022A385E1C11B542 |

Existing recorder tests remain `9358FC4A74DC0057DB8633052F7993D889FD05FEB2E67863692162500A0AA259`;
seeder remains `8CBF90234CD533492AFDED1952A765A68270E54DE093B9DD50A76F8BCE5DE6ED`.

```powershell
cargo test --release --locked --all-features --lib insertion_timing_capture:: -- --test-threads=1
```

Caller command (one Cargo/job, same target/owned TEMP, stage environment unset):

```powershell
cargo test --release --locked --features devtools --bin seed_history --test repository --test seed_insertion_profile --test seed_history_purge -- --quiet --test-threads=1
```

Caller session 51912 exits 0; no owned Cargo/rustc/seeder caller remains at
2026-10-05 01:21 UTC. Matching owned seed/proof/insertion fixtures are absent.
No other performance/native runner started; no manual fixture deletion.
Caller unit executable SHA-256: `A6DDE749783E76B352C0453A48AEA3EFDC6F82534F71184580B9A79CC0E28870`;
CLI executable: `EDACB89C417ECB0FE157E86CD5132B5D926325ABC45EBDBACDA7761FE9D34AA9`.

Stage-two counsel requires path-specific normalization after checking original
marker ownership, null/present structure and audit timestamp ordering. Physical
IDs, recipe/evidence timestamps, scores, chains and relative text remain exact.
Errors stop later units and expose no provisional state. Failed body completion
is not committed input; publish failure timing only after rollback/destruction,
preserving observable rollback errors. Deferred commit tests must reach actual
commit after successful bodies; earlier committed groups must survive. Correctness
and review precede any separate timing proposal. Five-unit memory and changed
rollback granularity remain unproven. Worker owns only the seeder/private modules.

The worker identifies consumed `Transaction::commit` hiding rollback errors.
Counsel rejects `DropBehavior::Ignore`: preserve default drop rollback as the
unwind safeguard. Root verifies pinned rusqlite 0.40.2 transaction source:
`Deref` exposes the connection, commit executes `COMMIT`, default drop rolls
back only when not autocommit, and drop ignores cleanup errors. The private
group executes COMMIT while retaining the transaction, then disposes it before
publishing success. On body/commit error, explicitly roll back only if still
in a transaction; preserve both primary and observable rollback errors in a
bin-private failure type. Failed cleanup aborts; disposal is not proof of
successful rollback. No legacy transaction/error/public AppError change.

Unresolved questions: grouped successful-data parity,
group failure/timing semantics and total benefit; independent remaining gates.
