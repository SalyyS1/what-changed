# Plan validation and red-team record

Date: 2026-10-27. Status: planning complete, final mechanical sweep passed;
implementation authorization still pending. No app code, installer or public website changed.

## Independent review

Four lenses used: Security Adversary/Fact Checker; Failure Mode Analyst/Flow Tracer;
Scope & Complexity Critic/Contract Verifier; Assumption Destroyer/Scope Auditor.
Two read-only scouts were reused for related review; two fresh reviewers covered
security and assumptions. Reports returned inline; no reviewer edited files or ran tests.
Controller consolidated 15 findings: 1 Critical, 9 High, 5 Medium. All incorporated
as refinements of this proposed plan, not recorded as user-approved scope/permissions.
Duplicate API-sequencing and approval-publication findings were merged.

| # | Severity | Finding and evidence | Draft disposition |
|---|---|---|---|
| 1 | Critical | Approval only prose + P12/G3 circular prerequisite; original plan row required G3 before artifact freeze | Approval ledger/custom phase metadata, P01 guard, G3 publication substage only; no claim CLI enforces it today |
| 2 | High | Rust no-network graph cannot constrain Chromium (scripts/check-no-network.mjs:44) | Connection-level proxy/no-direct transport contract and deny tests; remote support blocked if infeasible |
| 3 | High | Normal spawn then Job assign leaves descendant race; AppState had no runner owner (app/state.rs:112) | Suspended create → non-breakaway kill-on-close Job → resume; abrupt-parent-death tests |
| 4 | High | Memory-only auth conflicts with temporary profiles; sanitize.rs:91 handles structured text, not screenshots | Sensitive transient profiles; no forensic-erasure claim; screenshots per-capture reviewed, short retention, export excluded by default |
| 5 | Medium | Windows archive aliases not specified; SECURITY.md path boundary already requires canonicalization | ADS/device/case/dot/UNC/link/streaming/handle-containment fixtures |
| 6 | High | Delayed import can commit to reopened DB after purge (storage/db.rs:324,480,550) | Global admission barrier + durable epoch for all writers/producers |
| 7 | High | Recovery keeps full quarantine DB/WAL outside live tables (recovery.rs:100,110; purge.rs:172,223) | Explicit owned-copy inventory; recovery→purge byte-canary; staged consistent restore |
| 8 | High | P04/P05 need real rule/change/capability/environment/candidate APIs before P06; registry currently lacks them (commands/mod.rs:151) | P03 owns complete minimal config/change APIs + recipe schema; P06 extends authoring/evidence; portable recheck after population |
| 9 | Medium | Route union also consumed by typed harness (dev-states-fixtures.ts:90; DevStatesHarness.svelte:24) | P04 owns harness fixtures/parameters; synthetic isolation test |
| 10 | Medium | CSS is runtime authority, TS only mirror (src/lib/design/tokens.ts:4) | P10 owns src/styles/tokens.css and drift/contrast/render checks |
| 11 | Medium | Plain export:brand writes preview, not shipped icons (export-brand-assets.ps1:14,162) | Explicit -UpdateTauriIcons production command + destination review |
| 12 | Medium | Existing launch checker bans all AI mentions (check-launch-copy.mjs:16,42), six copy consumers | P10 owns anti-claim policy migration + all six consumer dispositions |
| 13 | High | Generic preview lacks revision and named backend assertion adapter | New supported-preview contract; UI-observation-only vs bounded backend verification |
| 14 | High | New audience differs from README.md:38, repeat-value test originally after launch | Early P06 two-change rehearsal; formal P12 pilot before G3 |
| 15 | High | Runner added P07 but whole-package runtime taxonomy deferred P12 | P07 introduces core/runner-disabled/authorized-egress/bundle gates; P12 consumes |

Technical tightening added no generic SaaS, AI provider, IDE or production mutation.
2D, 3D, landing, wiki, demo and download scope remains intact. Security mechanisms
are implementation requirements to prove, not evidence that the proposed runner is secure.

## Validation decisions

Questions asked by interactive tool: 0 (no suitable approval interview tool in this mode).
No fabricated user answers. User accepted product direction and requested detailed plan;
desktop-primary vs hosted recommendation and new network/process/privacy promises require
explicit G0/G1 confirmation before implementation. G2/G3 remain later gates.
Latest Windows QA authorization permits current account with safeguards; this plan does
not reinstate a mandatory separate human test account. Fresh-environment packaging proof
may use an authorized clean VM/CI environment; don't silently create a Windows account.

Observed facts separated from proposals:

- 12 phase files scaffolded/read through ak CLI, then populated.
- Main source map records 38 checked existing file/line pairs.
- Create paths/test scripts are explicitly future work, not falsely claimed present.
- Local public-site source/workflow absent; P01 must reconcile remote source.
- No build/tests/native QA/soak/product-performance measurements performed in planning.
- All approval-ledger entries pending, phase implementation checkboxes unchecked.

## Mechanical validation

Initial ak plan validate passed. Initial parse found 12 pending phases and 50
unchecked phase items; early rehearsal adds one further item in the reviewed draft.
Initial custom sweep checked 34 local links and 38 source file/line references;
it correctly reported this not-yet-created validation-review.md link as missing.
Final sweep resolved that missing link and verified effort/DAG/scripts across all phase
files. A first script-name scan falsely split hyphenated names (check:no-network);
the read-only checker was corrected and rerun, no product check was weakened.

Final results:

- ak plan validate: valid, no format errors.
- ak plan parse: 12 phases, 51 phase acceptance items, 0 completed (implementation not started).
- Read-only whole-plan scan: 21 Markdown files; 180 structural checks; 37 local links;
  38 existing source file/line pairs; 20 package-script references classified as existing
  or explicitly proposed; 12 Cargo test targets classified; 4 pending approval entries.
- Effort sum 432h; dependency DAG acyclic; no stub placeholders/trailing whitespace/
  premature completed checkboxes; 0 failures in final scan.
- Proposed command names remain future requirements, not test executions.
- Scoped git diff --check passed; custom file-content scan covers new untracked plan files
  because ordinary git diff alone does not inspect untracked content.
- 12 live Kanban items created in To Do for matching D:/Project/.WhatChage project;
  each links its phase/dependencies/approval gates. Plan files remain authority.
- CLI index rebuilt; current worktree plan pointer set to this plan without changing old
  plan files/status. No other session processes terminated or application code changed.

## Whole-plan consistency sweep

Decision deltas to check across all files: G3 publication-only; approval guard not a
built-in CLI guarantee; P03 APIs/schema before P04/P05; no memory-only auth claim;
no browser-context-as-sandbox claim; global purge epoch/all owned copies; CSS runtime
authority; production brand-export flag; launch AI copy policy; marker/adapter support;
P06 early value rehearsal; P12 pre-publication pilot; P07 runtime gate taxonomy.
No “all phases complete”, binary signing, current-source acceptance or market-fit claim.
All 12 phase files and index were reread; final scan reread every Markdown file after
refinements. The listed decision deltas were reconciled. No unresolved internal plan
contradiction found; G0/G1 authorization and P01 runtime/remote-source feasibility remain
explicit open gates, not claimed solved by this review.

## Handoff limits

Ready for P01 read-only preflight and architecture confirmation, not unrestricted cook.
Current graph/website/runner are still proposed. Unknown remote-source details and
native runtime feasibility are explicit phase-entry checks, not hidden plan omissions.
Old release remains NO-GO; nothing in this plan changes its evidence or approvals.

## Unresolved questions

Confirm G0/G1: downloadable Windows local-first app plus opt-in networked test runner,
with static public landing/wiki/download (not hosted SaaS). G2 public identity and
G3 exact artifact publication will be requested at their respective gates.
