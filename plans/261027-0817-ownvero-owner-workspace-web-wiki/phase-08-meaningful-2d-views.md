---
phase: 8
title: "Semantic 2D workflow and capability evidence views"
status: in-progress
priority: P1
effort: 28h
dependencies: [7]
required_approvals: [G0, G1]
---

# Phase 08 — Semantic 2D workflow and capability evidence views

## Goal

Make the process and evidence understandable without reading technical logs.
2D is primary and fully usable, not a poster for the 3D mode.

## Inventory / ownership

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Create | D:/Project/.WhatChage/src/lib/owner-graph/{types,projection,layout,selection}.ts | ~100–200 each | Domain parity/layout |
| Create | D:/Project/.WhatChage/src/components/owner-map/{ChangeJourney,CapabilityMap2D,OwnerMapFrame,OwnerMapNodeList,EvidenceInspector,BeforeAfterReview}.svelte | ~100–200 each | Keyboard/interaction |
| Create | D:/Project/.WhatChage/src/components/owner-map/owner-map.test.ts; src/lib/owner-graph/projection.test.ts | ~150–200 each | Semantic parity |
| Modify | D:/Project/.WhatChage/src/features/owner-workspace/AppOverviewScreen.svelte; src/features/acceptance/RunReviewScreen.svelte | Integration only | No-map path stays |

Visual projection owner; shared interface freezes before P09. Read existing SystemMapFrame,
SystemMapNodeList and shared scene utilities for patterns; do not reuse machine types/layout.

## Steps

1. Implement immutable semantic projection from authoritative backend evidence DTO.
   Define node/edge IDs, types, labels, provenance and state axes; no graph-inferred verdict.
2. Build five-step journey with actual artifacts/counts and blocked prerequisite inspector.
   Preserve step selection across navigation; no linear animation masquerading as completion.
3. Implement capability→rule→scenario→result 2D map with deterministic ordering,
   pan/zoom/fit and selected-change overlay. Explicitly label unknown impact/coverage.
4. Add before/after same-scenario comparison, incomparable-version explanation, failure/
   unknown/stale filters. No percentage score from missing scenarios.
5. Implement equivalent DOM list and inspector with instance-scoped IDs, active scroll
   reconciliation on dataset shrink, arrows/Home/End/Page/typeahead and Escape return.
6. Define bounded graph expansion: max visible nodes/edges, total counts, truncation reason
   and “show next group”; never silently drop failed/gap nodes from summary.
7. Test all states, keyboard, high contrast, dark theme, zoom and minimum viewport.
   Inspect with real evidence fixture, synthetic state harness labeled separately.

## Verification / matrix

~~~powershell
bun run test -- src/lib/owner-graph/projection.test.ts src/components/owner-map/owner-map.test.ts
bun run test -- src/components/system/system-map-node-list.test.ts src/components/system/system-map-frame.test.ts
bun run check
~~~

| Priority | Case | Pass |
|---|---|---|
| Critical | failed/blocked/stale/manual/exception | Same labels/counts as authoritative review |
| High | Graph shrink + two map instances | Focus preserved, IDs unique, no stale selection |
| High | Unknown impact/unmapped features | Visible gap, not imaginary edge |
| High | No pointer/WebGL | Complete task via list/keyboard |
| Medium | Large graph/theme/resize | Bounded rendering, readable labels |

- [x] Journey, capability map and before/after views implemented.
- [x] Semantic projection and accessible equivalence source tests pass; full native accessibility remains open.
- [x] Evidence inspector exposes exact build/scenario/scope.
- [x] Domain graph is ready for 3D without a second source of truth.

Evidence: [visual workbench report](../reports/implementation-261028-owner-visuals.md).
Native accessibility, complete final rendered recheck and comprehension remain
open; source implementation is not full phase acceptance.

## Risk / rollback

Graph only projects stored facts. An over-dense graph falls back to grouped list with
visible omitted counts; don't increase motion/force layout to conceal complexity.
Can disable visual route while retaining lists and all underlying data.
