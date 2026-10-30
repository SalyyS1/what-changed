# Actual-bound query-plan capture kickoff ledger — 2026-10-05

Outcome: approved first-slice source is frozen;13 focused cases,525 library
tests,36 caller tests, both strict lint configurations and one actual-bound
canonical10k page1/page10 diagnostic pass. No entire-phase acceptance or
index/SQL/fixture change claimed. The historical canonical
[24-cell query budget matrix](qa-261005-recurrence-rolling-query-matrix.md)
passed at its frozen checkpoint, but copied SQL in the older plan gate does not prove actual execution
and canonical corpus parity. Remaining SQL-plan obligations are not cut.

## Smallest source owner

Start with capture primitive and `event_repository::page` only, covering
actual first-page/page-ten calls without service-entry claim hooks. Details
would immediately add lookup, related, prior and conditional corroboration;
it is a later cohesive slice. Then summary/checkpoints/privacy and investigation
relationship/recurrence variants follow, with required canonical scale checks.
This is not a universal 20-file instrumentation framework.

Read-only source scout identifies the current page builder's values as
integers/strings erased into `Vec<Box<dyn ToSql>>`. Do not invent a universal
replayable `Params: Clone`/arbitrary `ToSql` contract: binding conversion can
have side effects. Proposed page-local concretization uses owned SQLite
`Value` variants with identical SQL, positions and values; EXPLAIN and the
real statement borrow the same vector. Shared `bind_list` remains unchanged.
Existing production refs-vector allocation can remain to minimize unrelated
change; concretization is not extra inactive diagnostic allocation.

## Executing statement ledger

| Page structural variant | Actual parameters | Completion / plan policy |
|---|---|---|
| Base | Two i64 endpoints, final i64 limit+1 | Full cursor exhaustion and all summaries decoded; indexed SEARCH events |
| Keyset | Adds cursor timestamp and ID; timestamp placeholder reused | Preserve exact OR predicate and tie ordering |
| Categories | Enum strings in supplied order; numbered IN arity | Same indexed access; temporary ordering B-tree may be legitimate |
| Minimum importance | Expanded suffix in Importance::ALL order | Preserve arity/threshold; no new cutoff |
| Entity | String after optional category/importance values | Indexed events access, without fixing one planner-selected index |
| Combinations | Four optional branches yield 16 masks; category/importance arities distinguish SQL structure | Static mask plus integer arities, never actual bound values |

## Primitive contract to review before implementation

- Devtools-only page-scoped capture module/tests and gated storage export.
  Explicit synchronous outer scope expects one structural page variant.
  Thread-bound RAII TLS guard clears on normal return/error/panic.
- Owner claims before preparing; structural classification only after an
  active scope is found. Missing, nested, multiple and mismatched invocation
  are evidence errors. Second matching call still executes normally.
- After prepare, explain the exact borrowed SQL string that the owner just
  passed to the real `prepare_cached`, on the SAME connection and concrete
  borrowed values, using a separate uncached statement. Keep the hooks adjacent
  to preparation/execution; finish/drop explain before the original query.
- Mark started after query succeeds, completed only after all cursor reads
  and decodes succeed. Prepare/query/consume/decode failures remain incomplete
  and preserve the original application result/error. Explain failure changes
  evidence only, never suppresses or changes the real operation.
- Retain static variant, integer bind/execution counts, parameter-free EQP
  rows and completion state. No SQL/binds/raw errors, clocks, environment
  switches, extra COUNT, implicit transaction, schema or query changes.
  Check actual EQP detail vocabulary/schema names before a privacy claim.
- Inactive devtools performs only scope check; no conversion/allocation/
  EXPLAIN work. Normal build excludes all capture code.

EQP policy belongs in tests: require indexed `SEARCH events`, prohibit an
unindexed events scan. Do not demand a fabricated index for sorting or pin
one index across combined filters. Explain is diagnostic and can perturb
cache/locks/snapshot timing; it stays outside every p95 acceptance call.

## Proposed ownership and validation

Owned first slice: gated `storage/mod.rs` export; page builder/execution and
existing page explain helper in `event_repository.rs`; new primitive/tests;
page-specific `tests/query_service.rs` additions; separate ignored canonical
page-capture smoke. No production service edits or measured p95 fixture changes.

Primitive tests: missing/nested/multiple/mismatch, disabled no explain or
extra allocation, thread/panic isolation, explain error preserving operation.
Owner tests: 16 masks, multi-category/importance arities, tied keyset progression,
concrete bind order, empty successful page, decoding error leaves incomplete.
Reuse clustered raw-cursor regression and add actual capture alongside the
existing filter/index assertion; helper-only explain is not execution proof.

Canonical kickoff: one fresh unchanged 10k `PerfFixture`, actual first-page
service call captured; eight intermediate calls (pages2–9) run outside scope,
then page ten is captured using page nine's cursor: ten actual calls total,
nine cursor advancements. Assert complete page/cursor result contracts and actual
bound indexed evidence. Larger required scales and other six service shapes
remain explicit follow-up, not silently accepted by a 10k page-only smoke.

No file edits, test/build/seed/perf processes were started by the source scout.
This describes the earlier read-only scouting boundary; implementation is now
authorized by the following counsel checkpoints, without scope expansion.

## Pinned API correction and counsel

Root checks Cargo.lock and local rusqlite 0.40.2 source: public `Statement`
has no `sql()` method; original SQL access is private/raw or Debug-only.
Do not use unsafe handles, Debug parsing, `expanded_sql` exposing bound values,
dependency changes or copied SQL to substitute for the absent API.
Counsel approves borrowing the exact source-owned preparation input and same
concrete values at the adjacent owner hook. `parameter_count` verifies shape,
not SQL identity/value equality; sharing those inputs supplies that assurance.
Keep invocation, EQP result and actual full-decoding outcome distinct; EQP
failure leaves the original operation unchanged. This is diagnostic evidence,
not timing-neutral execution. Implementation waits until grouped QA edits finish.

## Typed plan privacy boundary

After both seed pairs finish with parity/cleanup green and no default promotion,
counsel GO permits the page implementation. A further source audit notes that
raw EQP detail can contain arbitrary schema/index names. Counsel approves a
small page-only typed projection tied to pinned SQLite: node/parent IDs,
recognized events SEARCH/SCAN, known migration-index identities, covering
status and relevant sort/OR nodes. Unknown syntax or names produce incomplete
plan evidence, never fabricated classification, silent omission or operation
failure. No raw detail/error text survives projection failure. Actual query
execution/decoding remain independent from parser/EQP outcome.

Acceptance also requires observed parser fixtures plus unknown name/vocabulary
cases, all16 masks/cursors/ties and lookahead decoding. Policy permits intentional
sort/OR operations and does not use a blanket INDEX substring requirement.
A SQLite upgrade requires reviewing this allowlist; it is not a universal
query parser or proof for other service families. Worker owns only the page-local
files above; root runs gates/review/counsel. One owned focused Cargo command
now compiles the frozen library with one build job; no other execution overlaps.

Counsel also approves an explicit failure-test evidence boundary: a real SQLite
EQP failure is induced at the primitive only, separately from a successful scalar
operation. An actual page-owner unsupported-index plan tests incomplete evidence
while the real page succeeds; actual decode failure tests incomplete execution.
Each failure must permit a later clean capture. The primitive does not claim
to force an actual-page SQLite EQP failure, which remains unforced; no global
resource sabotage, dependency feature or fault-injection framework is added.

## Frozen source and initial checks

Three existing owners and ten new page-local modules/tests implement the slice.
Parameter-free data is separated from lifecycle and the pinned parser. Cargo
autodiscovery was caught before execution: a child-only service test moved into
tests/support rather than becoming a standalone target; manifest unchanged.
Root formats the owned files; full cargo fmt/scoped diff checks pass. Live
simplifier gate is7,772 text LOC/146 tracked files/16 binary/max-added619 versus
400/8/200; independent scoped simplifier is CLEAN, no edits. After return only
the unchanged scoped shortstat is logged, not another simplification run.

A read-only Get-FileHash command used the wrong positional syntax and failed
before claiming any hash verification. Counsel approves array-form repair;
the corrected command matches unchanged insertion timing/test and canonical
fixture hashes. It is a tool-syntax correction, not a source/test failure.
Independent code review is CLEAN, no actionable bugs/contract gaps. It confirms
that entity-filter masks of the reused tiny16-mask fixture can be empty; the
proof is actual plan plus captured/inactive parity, not independent expected IDs
for every predicate combination. Separate exact SQL/Value-order and tied cursor/
lookahead cases fulfill their contracts; do not overstate the16-mask evidence.
Focused13 cases pass0.10s after6m33s compilation; full library525/zero failed/
three intentional diagnostics ignored passes21.97s after0.88s preparation.
Affected callers and the separate ignored canonical target compile next;
caller/lint and actual isolated canonical smoke execution remain pending.
[Frozen identities](qa-261005-page-capture-source-freeze.md),
[complete focused output](qa-261005-page-capture-focused-output.md),
[full library output, part1](qa-261005-page-capture-library-output-1.md).

Counsel retains the two query-budget checkbox closures as verified at the prior
matrix's recorded source/artifact checkpoint. Later page bind-storage and
inactive-capture changes mean current-source p95 has not been remeasured; do
not call the implementation unchanged or relabel the old24 cells current.
No immediate three-scale rerun is required for this diagnostic slice; a fresh
latency acceptance refresh remains explicit for the eventual frozen RC, or
sooner if a pending check establishes a performance concern. The10k plan smoke
is plan/execution evidence only, not a budget measurement.

## Terminal caller, lint and actual-service evidence

Affected callers pass9 service/24 repository/3 fixture-lifecycle cases after
10m35s compilation. Both release Clippy configurations pass1m53s/14.60s;
full fmt/scoped diff pass. Rehash13 source owners and canonical fixture matches
the frozen reviewed revision. No post-review source amendment or gate retry.
[Full callers](qa-261005-page-capture-callers-output.md),
[full lint](qa-261005-page-capture-lint-output.md).

Counsel GO precedes exactly one canonical10k ignored smoke. It passes3.02s:
actual first/tenth pages each have one invocation, prepared/bind cardinality
match, complete201-row lookahead decoding and recognized indexed SEARCH
events via idx_events_timestamp. Raw200-item cluster expansion, strict keyset
ordering and cursors pass across exactly10 service calls. Owned fixture is
closed/joined/removed; no owned runner remains. The unrelated pre-existing
seed-size-probe directory is untouched. [Complete output/provenance](qa-261005-page-capture-canonical-output.md).

No full query-plan checkbox, current-source p95 or phase status is promoted.
Counsel permits a separate narrow fresh-writer COMMIT PROFILE diagnostic;
page shared-owner edits are frozen before that work starts.
Independent terminal review reconciles all528 library records,36 caller
passes, both lint outputs, canonical evidence and artifact hashes; links and
the narrow source integration forecast are CLEAN. Subsequent COMMIT work
changes shared mod.rs/event_repository whole-file identities, not the PAGE
receipt: these hashes remain terminal historical freeze records, with later
shared-owner integration separately gated. No current-source relabeling.

Unresolved questions: subsequent actual-bound owners/filter branches/canonical
scales and eventual fresh latency acceptance. Seed work remains independent;
no grouped default promotion or overlapping shared-owner edits.
