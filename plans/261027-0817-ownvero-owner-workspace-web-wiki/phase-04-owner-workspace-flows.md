---
phase: 4
title: "My app and change request flows"
status: in-progress
priority: P1
effort: 40h
dependencies: [3]
required_approvals: [G0, G1]
---

# Phase 04 — My app and change request flows

## Goal

Owner completes first and second feature requests without using developer terminology.
Follow [experience contract](experience-and-site-contract.md), not incident-first homepage.

## Inventory / owner

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Create | D:/Project/.WhatChage/src/features/owner-workspace/{MyAppsScreen,AppOverviewScreen,CapabilityEditor}.svelte | ~100–200 each | Component flow |
| Create | D:/Project/.WhatChage/src/features/change-request/{ChangeRequestScreen,PreserveRulesStep,ChangeBriefReview}.svelte | ~100–200 each | Wizard/conflict |
| Create | D:/Project/.WhatChage/src/lib/stores/{owner-workspace,change-request}.svelte.ts | ~150–200 each | Race/cancel |
| Modify | D:/Project/.WhatChage/src/app/{routes.ts,screens.ts,App.svelte,AppShell.svelte,Sidebar.svelte} | Bounded integration | Routing/lifecycle |
| Modify | D:/Project/.WhatChage/src/lib/stores/router.svelte.ts | Parameter parsing | URL/back/unknown |
| Modify | D:/Project/.WhatChage/src/features/dev/{dev-states-fixtures.ts,DevStatesHarness.svelte} | New route fixtures/context | Typed completeness, no live data |
| Create | D:/Project/.WhatChage/src/features/owner-workspace/owner-workspace.test.ts; src/features/change-request/change-request.test.ts | ~150–200 each | Repeat-use acceptance |

Frontend flow owner; central route wiring serialized before P10 brand edits.
Do not change legacy stores/projects.svelte.ts meaning.

## Steps

1. Scout shell boot/recovery/onboarding and hash-only router. Add typed app/change/run
   route parameters with explicit validation, no raw parsing scattered in screens.
   Update HarnessRoute/readyData and DevStatesHarness navScreen consumers for new route
   IDs. Parameterized screens receive isolated synthetic app/change/run context; harness
   must never fall through into real workspace data or runner execution.
2. Implement first-session app creation and optional root link. Explain preview is not
   contacted until run consent. Legacy users choose new mode without losing old routes.
3. Implement capabilities/owner rules: name, expected behavior, examples, draft vs
   confirmed revision, unresolved questions. No green inferred features.
   Add safe-preview setup brief and prerequisite check UI from
   supported-preview-contract.md. Missing marker/roles/adapter produces observation-only
   or blocked label, never an unexplained failed onboarding or fake current pass.
4. Build outcome/preserve/business choices/brief review wizard. Only ask choices
   relevant to current change; explicit save/resume, conflict handling, abandon confirm.
5. Let user seed a new change from accepted prior rules/decision; do not clone stale
   evidence as current. One next-action CTA with meaningful empty/blocked/error states.
6. Stores key async requests by app + revision/generation; changing workspace/unmount
   cancels/discards late response. No cross-project state flash.
7. Integrate legacy diagnostics behind clear navigation; keep Home/timeline/checkpoint
   shortcuts discoverable and update explicit route-prominence contract intentionally.
8. Keyboard/screen reader/900×600/zoom/reduced-motion checks. Never make map interaction
   necessary to create request, read evidence or accept change.

## Verification / matrix

~~~powershell
bun run test -- src/features/owner-workspace/owner-workspace.test.ts src/features/change-request/change-request.test.ts
bun run test -- src/app/App.test.ts src/app/App.onboarding-reload.test.ts src/app/route-prominence.test.ts
bun run check
~~~

| Priority | Case | Pass condition |
|---|---|---|
| Critical | App switch during save/load | No wrong-app mutation/display |
| Critical | Unsaved draft / stale confirmed version | Conflict preserved and explained |
| High | Fresh vs existing legacy user | Both journeys reachable, no forced collector opt-in |
| High | Two successive changes | Prior confirmed rules reused; prior evidence freshness recalculated |
| Medium | Keyboard/minimum viewport | Main task complete without drag/hover/3D |

- [ ] First meaningful request saved and reopened.
- [ ] Second request reuses correct confirmed rules.
- [ ] Routing/lifecycle/legacy tests pass.
- [ ] All async/UI states and accessibility checks recorded.

Source/component/store tests and isolated browser flows passed, including saved
wording, reopen and confirmed-rule reuse. A current-source debug Tauri session
also saved two synthetic requests, reused their confirmed rule and completed
native clipboard/export/import. Complete accessibility, packaged acceptance
and meaningful real-owner iterations remain open. See [browser QA](reports/qa-261028-0215-owner-workspace-browser.md),
[native continuation](../reports/qa-261028-0440-offline-continuation.md) and
[execution boundaries](reports/offline-execution.md); synthetic results do not
close the real-owner acceptance boxes above.

## Risk / rollback

Do not migrate old onboarding flag to network consent. Behind new route until integrated
acceptance; disabling new routes leaves stored owner work intact. Never fabricate fixtures
as production app state; built-in example must have separate synthetic namespace.
