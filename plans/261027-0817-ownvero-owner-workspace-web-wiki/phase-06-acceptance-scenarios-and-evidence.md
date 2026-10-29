---
phase: 6
title: "Acceptance scenarios, evidence and owner decisions"
status: in-progress
priority: P1
effort: 40h
dependencies: [5]
required_approvals: [G0, G1]
---

# Phase 06 — Acceptance scenarios, evidence and owner decisions

## Goal / contract

Define expected behavior before execution; results never self-approve.
Read [domain/evidence](domain-evidence-contract.md) and [experience](experience-and-site-contract.md).
Dedicated scout must enumerate all confirmed-rule/result selectors before implementation.

## Inventory / owner

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Create | D:/Project/.WhatChage/src-tauri/src/owner_workspace/{scenario,evidence,acceptance,freshness}.rs | ~100–200 each | State-machine invariants |
| Modify / Create | D:/Project/.WhatChage/src-tauri/src/commands/owner_changes.rs (P03 owner); D:/Project/.WhatChage/src-tauri/src/commands/owner_evidence.rs (new) | ~120–180 each | App-scoped commands |
| Create | D:/Project/.WhatChage/src/features/acceptance/{ScenarioEditor,RunReviewScreen,OwnerDecisionDialog}.svelte | ~100–200 each | Approval/unknown states |
| Create | D:/Project/.WhatChage/src/lib/stores/owner-evidence.svelte.ts | ~150–200 lines | Frozen run vs live UI |
| Create | D:/Project/.WhatChage/src-tauri/tests/{owner_acceptance,owner_evidence_freshness}.rs | ~150–200 each | Race/fail-closed |
| Modify | D:/Project/.WhatChage/src-tauri/src/owner_workspace storage and central IPC owners | Bounded additions | Schema/parity |

Evidence owner controls verdict projection. Maps/site later consume, never recalculate
test truth with independent rules.

## Steps

1. Use bounded recipe schema/version frozen in P03; implement capability templates for booking CRUD.
   Expected output is owner language; test method/role/data requirements made explicit.
2. Rule and scenario revision confirmation displays old/new expected behavior and
   provenance. Generated/imported suggestions cannot become confirmed without owner action.
3. Freeze candidate/environment/rule/scenario/fixture/runner manifest for each run.
   Implement lifecycle, per-assertion status, freshness and method as separate axes.
4. Derive summary from expected assertion IDs; missing output blocked/not_run, no pass
   from process exit. Late run belongs to frozen revision, not latest selected app.
5. Add structured manual check with observer/time/steps; preserve method owner_manual.
   It is a valid observation, not equivalent to automated test evidence.
6. Review expected vs observed, scope, build identity, gaps, stale artifacts and failed
   attempts. Provide bounded repair brief using P05 formatter.
7. Implement owner accept/reject/defer; acceptance with exceptions requires reason and
   explicit acknowledgment. Original failures remain visible and exportable.
8. Test content changes, locator edits, candidate changes, environment changes, retention
   and new runner semantics. Recompute freshness on read/accept/export.
9. Failure from missing backend permission stays blocked or UI-only. Persisted record
   verification uses actual defined read-back path; do not use toast or reload heuristic.
10. Before expensive runner work, run an early supervised usability/repeat-value rehearsal
    with 3–5 consenting target owners across two real change requests using manual evidence.
    Owner arranges participants/outreach; no automatic contact or unauthorized binary sharing.
    Record setup friction, whether they understand gaps and voluntarily use the second brief.
    If none can finish without developer interpretation or none sees value in a second
    cycle, stop to review UX/thesis with user before P07; do not cut requested scope silently.
    This is an early risk check, not a replacement for formal pilot or automated runner tests.

## Verification / matrix

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --test owner_acceptance
cargo test --manifest-path src-tauri/Cargo.toml --test owner_evidence_freshness
bun run check:bindings
bun run check
~~~
Create and run scenario-editor.test.ts and run-review.test.ts beside components.

| Priority | Case | Expected |
|---|---|---|
| Critical | Assertion removed/relaxed by import or repair | New unapproved revision, prior verdict cannot transfer |
| Critical | Unknown build / changed build mid-run | Unknown/stale, no current-build acceptance |
| Critical | Owner accepts failed result | Explicit exceptions; failure remains failed |
| High | Result arrives after app/version switch | Stored in correct history only |
| High | Missing role/read-back | Blocked; no backend/persistence claim |
| Medium | Artifact expired/deleted | Metadata retained with missing-artifact label |

- [x] Full status matrix and immutable revisions implemented.
- [x] Automated vs manual vs imported provenance preserved.
- [x] Acceptance/retry/freshness race tests pass.
- [x] No implicit approval, unexplained green or universal safety claim.
- [ ] Early two-change usability evidence reviewed before P07; participant gaps reported honestly.

Source evidence: acceptance 11, freshness 4 and populated portability 8 tests
passed. Scoped UI/helper tests and browser decision checks are recorded in
[browser QA](reports/qa-261028-0215-owner-workspace-browser.md).
The [rehearsal packet](reports/manual-owner-rehearsal-packet.md) is prepared;
no real participant sessions have been performed or inferred from fixtures.

## Risk / rollback

UI and backend both validate but backend is authority. Rolling back UI must not reinterpret
new enum values as passed; unknown versions fail explicitly. Preserve evidence, never rewrite
history to make new assertions appear previously tested.
