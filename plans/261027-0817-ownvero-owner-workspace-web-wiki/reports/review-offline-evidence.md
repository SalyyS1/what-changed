# Offline manual-evidence review

Date: 2026-10-28. Scope: pending P06 backend, DTOs, acceptance UI/store, focused tests.
Initial mode: read-only code/spec review. No target network requests or personal data
access. Findings below record the original source-traced failures. The controller
subsequently authorized the repairs described below; passing verification remains
separate from this initial diagnosis.

## Outcome

Changes needed: one high-priority evidence-semantic gap and two bounded contract bugs.
Unknown-build/manual labeling, immutable history and transactional freshness checks
are preserved. The findings do not claim that ordinary current-build acceptance is possible.

### Authorized repair follow-up

- Implemented compatible action/assertion/route purposes, selected fields and named roles.
- Shared `MAX_ASSERTIONS_PER_SCENARIO=1000` now fences confirmation and recording.
- Shared manifest validation covers freeze, retry, observation admission and read/export
  projection; every adapter origin must belong to the exact frozen environment.
- Unsupported historical recipes project Blocked without changing the raw observation;
  Failed still dominates. Exception decisions must explicitly acknowledge the gap.
- Six focused integration tests added under `tests/owner_acceptance/recipe_contract.rs`
  and `tests/owner_evidence_freshness/recipe_contract.rs`, including valid manual paths,
  1,000/1,001 limits, foreign ports, preserved history and failure dominance.
- Rustfmt and scoped diff check passed. Controller owns the pending focused Cargo run;
  no passing P06 test claim yet. DTO shapes and manual/unknown-build labels unchanged.

## P1 — Bind persistence/access assertions to their required action and route

Primary: `src-tauri/src/owner_workspace/validation/recipe.rs:120` and `:127`.
Related: `src-tauri/src/owner_workspace/evidence/observations.rs:98`,
`src/features/acceptance/RecipeStepEditor.svelte:40`.

Action and assertion validation are independent. A valid Navigate/Click action may
contain FieldEquals or Denied with no adapter or role at all. ReadBack and
DenyOtherRole only require an existing adapter ID; adapter effect/field selection
is not matched to the assertion. The editor exposes every assertion kind for every
action. Result derivation accepts supplied Passed when the containing step has any
nonempty manual observation; it does not preserve a structural missing-route gap.

Source-traced reproduction:

1. Confirm an owner-manual scenario with empty role/adapter lists, Navigate
   `/bookings`, and FieldEquals for `customer_name` versus a named fixture field.
2. Freeze a run; record the navigation step as “Saw the confirmation toast” and
   the field assertion Passed with nonempty observed text.
3. Result and scenario become Passed. `read.rs:131` omits the scenario from unresolved
   acknowledgments, despite the recipe defining no persisted-record read-back.

The same structural hole permits Denied on a UI action without a backend route.
This contradicts phase-06 step 9 and its high-priority missing-role/read-back case,
plus domain-evidence-contract.md:151–157. This is not a request to verify a human's
honesty or to add automation: the frozen recipe itself lacks the required method.

Repair: validate compatible action/assertion/adapter effect and selected fields;
missing backend prerequisites must be rejected or remain explicit blocked/UI-only
evidence. Ensure existing malformed frozen recipes cannot retain a misleading
Passed projection. Keep all historical observations intact.

Regression cases: Navigate+FieldEquals with no adapter; Click+Denied with no role;
read-back on a read-denied adapter; response field outside adapter allowed fields;
valid manual read-back and denied-access recipes remain usable.

## P2 — Match adapter origins to the run's frozen environment

Primary: `src-tauri/src/owner_workspace/storage/evidence/manifest.rs:58`.
Related: `src/features/acceptance/OwnerAcceptancePanel.svelte:18`.

Freeze validates the candidate's current environment revision, then copies selected
scenario recipes without checking their adapter origins against that environment.
The editor supplies the union of origins from all latest unrevoked environments.

Reproduction: configure environment A at localhost:3000 and B at localhost:4000;
select a candidate bound to A, and a scenario whose read-back adapter uses B. The
run freezes A as its tested environment but retains B as the persistence/access
target, without an environment mismatch blocker. Freshness remains Unknown rather
than detecting that internal mismatch, and the scenario may become manual Passed.

Impact is evidence attribution, not automatic unauthorized network access: this
offline flow contacts no target. Repair by validating every exact adapter origin
against the frozen environment's authorized origins before admitting the run, and
surface the mismatch to the owner. Add multi-environment and exact-port tests.

## P2 — Align authoring and result assertion budgets

Primary: `src-tauri/src/owner_workspace/storage/evidence/run.rs:62`.
Related: `src-tauri/src/owner_workspace/validation/recipe.rs:82`,
`src-tauri/src/owner_workspace/evidence/observations.rs:130`,
`src-tauri/src/owner_workspace/validation.rs:34`.

Authoring admits up to 50 steps with 100 assertions each. Eleven steps of 100 small
assertions satisfy recipe collection and 1 MiB limits, but recording 1,100 manual
assertions is rejected by the 1,000-entry input limit. Recording only a subset also
fails: derive_result emits all 1,100 expected outcomes in one array, which payload
validation rejects. Immutable one-result-per-scenario/run storage precludes chunking.

Thus a confirmed and runnable scenario can never receive any result. Use one shared
aggregate assertion budget across authoring, freeze, recording and output validation;
reject over-budget recipes before confirmation. Test exactly 1,000 and 1,001 across
multiple steps, including partial observations and existing frozen oversized recipes.

## Verified strengths / intentional decisions

- Every ordinary manual review has freshness Unknown or Stale and `can_accept=false`.
- Accept-with-exceptions requires completed nonstale input, reason, unknown-build
  acknowledgment, and the exact unresolved scenario set. It does not rewrite failures.
- Freeze binds change/candidate/environment/rule/scenario data and a digest.
  Read, export and decision paths recompute freshness inside their transactions.
- Missing expected IDs become NotRun; failure dominates summaries; observations are
  immutable per scenario/run. Retries retain the exact original manifest and reject
  stale inputs. Late observations remain tied to their original run.
- Frontend async generation tokens and run-keyed review components prevent an old
  selection's response or in-progress form from being displayed as a different run.
- Existing tests cover revision confirmation, failure/exception preservation,
  stale-input races, immutable retry/history and unknown-build presentation. They do
  not cover the three structural cases above.

## Verification boundary

No supervised owner rehearsal, automated runner, live target or release claim made.
Controller owns aggregate Cargo/Vitest evidence and shipping gates. Separately,
the lifecycle test repairs passed 13 database and 5 capture tests on the library
binary compiled with those repairs; this is not P06 regression verification.

Unresolved questions: none needed to reproduce the findings. Product choice remains
whether structurally incomplete recipes are rejected at confirmation or retained
as explicitly blocked/UI-only drafts/results; neither may claim backend persistence.

## Controller verification follow-up

All three findings repaired: strict active admission and exact frozen origins,
shared assertion budgets, and blocked historical projections retaining raw data.
Archival structure validation remains separate so history stays exportable.
Final owner_acceptance 11, owner_evidence_freshness 4 and owner_portability 8 tests
passed. Full library passed 626 with 3 ignored. See [execution](offline-execution.md)
for command/source identity and native/participant limits. No further product
decision is required for these repairs.
