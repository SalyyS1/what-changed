---
phase: 9
title: "3D evidence workbench and accessible fallback"
status: in-progress
priority: P1
effort: 32h
dependencies: [8]
required_approvals: [G0, G1]
---

# Phase 09 — 3D evidence workbench and accessible fallback

## Goal

A useful three-layer view links owner intent, scenarios and observed results.
Deliver real interactions and 2D/list parity; not an RGB decorative scene.

## Inventory / owner

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Create | D:/Project/.WhatChage/src/three/owner-map/{layout,scene,scene-content,labels}.ts | ~100–200 each | Lifecycle/layout |
| Create | D:/Project/.WhatChage/src/components/owner-map/{CapabilityMap3D,MapModeControl}.svelte | ~150–200 each | Lazy load/fallback |
| Create | D:/Project/.WhatChage/src/three/owner-map/{scene.lifecycle,layout}.test.ts | ~150–200 each | GPU/resource cleanup |
| Modify | D:/Project/.WhatChage/src/components/owner-map/OwnerMapFrame.svelte | Bounded | Shared state selection |
| Reuse/modify narrowly | D:/Project/.WhatChage/src/three/shared/{renderer,camera,dispose,picking,palette}.ts | Only generic helpers | Both old/new scene regression |

3D owner after P08 interface freeze. No schema/verdict modifications in scene code.

## Steps

1. Implement orthographic/isometric layers from same projection and deterministic layout:
   intent at top, scenarios middle, evidence bottom. Explicit typed connectors only.
2. Add fit selected, reset, zoom, isolate capability, layer separation and comparison.
   DOM labels and inspector carry semantics; no reading tiny WebGL-only text.
3. Add 2D/3D/list modes, remembered preference, capability probing and fallback when
   context unavailable/lost/import fails. Do not persist false “3D working” state.
4. Demand-render while idle; render during user interaction/tween only. Stop hidden/offscreen,
   cap DPR, bounded geometry/labels and low-quality mode. No perpetual rotation/particles.
5. Reduced motion disables camera animation; forced colors uses semantic list/2D.
   An explicit 3D choice may render statically but cannot override accessibility needs.
6. Destroy every listener/observer/control/texture/material/geometry/renderer. Test
   concurrent mounts, route remount, dataset shrink and 20 open/close cycles.
7. Measure cold/warm load, frame budget, UI responsiveness and total/private working set
   on same named machine/fixture. Low-tier <30fps repeated → graceful 2D with reason.
8. Validate a user can explain which scenario supports selected rule in both 2D and 3D;
   record a small comprehension check, not a subjective “looks premium” pass.

## Verification / matrix

~~~powershell
bun run test -- src/three/owner-map/scene.lifecycle.test.ts src/three/owner-map/layout.test.ts
bun run test -- src/three/systemMap/scene.lifecycle.test.ts src/three/changeMap/dispose.test.ts
bun run check
bun run build
bun run check:bundle
~~~
Browser/native visual test required; jsdom cannot prove GPU/render/a11y performance.

| Priority | Case | Pass |
|---|---|---|
| Critical | Mode switch | Same selection/results/gaps, no independent recalculated pass |
| High | Context loss/lazy import failure | Readable 2D/list, task still complete |
| High | Hidden/unmount/reduced motion | No active render loop or dangling resources |
| High | Two maps + 20 mount/unmount cycles | Scoped IDs/listeners, no unbounded growth |
| Medium | 120 nodes/240 edges fixture | >=30fps low-tier or explicit fallback; 60fps target recorded |

- [x] Useful 3D layer model and interaction implemented.
- [x] Source and bounded browser fallback/selection parity pass; full native a11y remains open.
- [ ] Lifecycle/resource/performance evidence recorded.
- [x] No new heavyweight visualization framework or external model asset dependency.

Evidence: [visual workbench report](../reports/implementation-261028-owner-visuals.md).
The final browser rerun timed out; earlier successful real WebGL/fallback results
are identified separately. Machine frame/memory budgets remain unverified.

## Risk / rollback

Three.js already exists; don't add another engine. Keep data separate from render state.
Feature flag can turn off 3D while leaving 2D/list fully functional; this emergency
fallback is not permission to omit requested 3D from final acceptance.
