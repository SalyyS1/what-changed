---
phase: 16
title: "Change Map: purposeful 3D causal/temporal visualization with 2D fallback"
status: completed
priority: P2
effort: 9h
dependencies: [15]
---

## Overview

One visualization, one job: make the *shape* of a cascade visible in a way a list cannot (§35–§39).
The Change Map plots the events of an investigation or a time window in three meaningful axes — X =
time, Y = category lane, Z = chain depth — draws the P15 structural edges as tubes and the temporal
edges as faint lines, and lets the user click a node to open the same details drawer the timeline uses.
Three.js is used because the third axis carries data (chain depth / branching), not because 3D looks
impressive; if the visualization cannot answer "which change started the chain and what followed",
it is cut rather than decorated. A full 2D canvas fallback ships in the same phase for machines without
usable WebGL and for `prefers-reduced-motion` users.

## Requirements

**Functional**
- Entry points: (1) an investigation's "View as map" action showing that investigation's suspects,
  chains, and the symptom marker; (2) the Timeline's "Map" toggle showing the current window/filters.
  Both feed the same component with the same typed payload — no second data path. **Information
  architecture decision (user, 2026-09-10): contextual view.** Change Map is not an eighth sidebar
  destination and has no global number shortcut. It opens from an Investigation's `View as map` action
  or Timeline's current-window `Map` action, preserves the originating query/investigation, and returns
  to that surface with focus restored. Reconcile `PRODUCT.md`, `routes.ts`, the dev harness and route
  accessibility tests so they describe seven top-level destinations plus this contextual surface.
- Layout is deterministic and computed in TypeScript from the Rust payload (`ChangeMapData { nodes, edges,
  symptom_time, lanes }`): X = `(timestamp - window_start) / window_span` scaled to `[-halfW, halfW]`;
  Y = one lane per category in a fixed order (process, port, config, service, application, update,
  driver, network, startup, system) so lane position is stable across sessions; Z = chain depth from
  P15's chains (0 for unchained events). Node size encodes importance (informational 0.6, important 1.0,
  critical 1.4); colour comes from the §41 token palette by category (read from CSS variables at scene
  init so theme switches stay consistent).
- Rendering: one `THREE.InstancedMesh` (sphere, 12×8 segments) for all nodes with per-instance colour
  and matrix; one `LineSegments` with a `BufferGeometry` for temporal edges; structural edges as a second
  `LineSegments` with a brighter material and 2 px `linewidth`-independent visual weight achieved by
  drawing them as short quads only if a spike shows lines are too faint (default: two line materials with
  different opacity — no third-party line library).
- Interaction: orbit-style camera implemented directly with pointer events and a small damping loop
  (~70 lines) instead of adding `OrbitControls` from the examples bundle — keeps the dependency surface at
  the `three` package only, as decided in research §4. Constraints: azimuth free, polar clamped to
  `[0.15π, 0.85π]`, zoom clamped `[0.4, 4]` of the fit distance, pan clamped to the data bounds; a
  `Reset view` button restores the fitted camera.
- Picking: `Raycaster` against the instanced mesh (`instanceId`) on pointer-move throttled to 30 Hz and
  only while the pointer is inside the canvas; hover shows a DOM tooltip (not a WebGL label) with the
  event summary + relative time; click emits `select(eventId)` → the P07 details drawer opens over the
  map; the selected node is highlighted by writing an emphasis colour into the instance colour buffer
  (no extra mesh, no re-layout).
- Chain emphasis: hovering or selecting a node dims all instances not in its chain to 25% opacity via the
  instance colour buffer, and draws the chain's structural edges at full strength. That is the one
  interaction that justifies the third dimension — a cascade becomes a visible ridge.
- Symptom marker: a vertical translucent plane at the symptom time with the label "You noticed the
  problem here" rendered as a DOM overlay pinned by projecting the world point each frame.
- Accessibility and equivalence (§62): the canvas has `role="img"` with an `aria-label` summarizing
  node/edge counts and the top chain; a visually-present `Switch to list` control returns to the ranked
  list (the map is never the only way to reach the information); keyboard `Tab` focuses a hidden
  ordered list of nodes with `Enter` selecting one (same handler as click), so a keyboard user can reach
  every node; `prefers-reduced-motion: reduce` disables camera damping/auto-fit animation and the render
  loop becomes on-demand (render only after an interaction).
- Fallback: `ChangeMap2D.svelte` (plain 2D canvas, X = time, Y = lane, chains drawn as polylines, same
  picking/selection API) is used when `WebGLRenderingContext` creation fails, when
  `renderer.getContext().isContextLost()` after init, on `webglcontextlost`, or when the user sets
  `visualization: "2d"` in settings. Switching is transparent to the parent component (both implement the
  same props/events contract).
- Data caps: max 3000 nodes and 6000 edges. Above that, the payload is aggregated server-side by the
  existing P15 downsampler and the map shows a chip "Showing the 3,000 most important changes".

**Non-functional**
- **§38 UI performance is this phase's non-functional section, clause by clause**: 60 FPS target;
  rendering paused when not visible and stopped when minimized; `prefers-reduced-motion` honoured by
  dropping to on-demand rendering; draw calls minimized by one `InstancedMesh` plus two `LineSegments`
  instead of per-node meshes; no post-processing at all; geometries, materials and the renderer disposed
  on unmount with a 20-cycle build gate; and a shipped, tested 2D fallback for machines without usable
  WebGL. §38 has no other owner, so this is the row the §1–§91 audit reads.
- 60 FPS at 3000 nodes on this machine's GPU, measured with a 300-frame timing harness; if the median
  frame exceeds 17.5 ms (one 60 Hz interval plus timer/scheduler tolerance), node geometry drops to 8×6
  segments and temporal edges are thinned by stride —
  both switches are in one `qualityTier` constant.
- No allocation inside the render loop: matrices, colours, and vectors are pre-allocated; the loop does
  `instanceMatrix.needsUpdate` only when something changed.
- Full teardown on unmount: `renderer.dispose()`, `renderer.forceContextLoss()`, every geometry/material
  `.dispose()`, `setAnimationLoop(null)`, pointer/resize listeners removed, and the canvas element
  dropped. A mount/unmount×20 test asserts `renderer.info.memory.geometries === 0` for a fresh renderer
  and that JS heap does not grow monotonically (heap check is advisory; the geometry/texture counters are
  the hard assertion).
- Render loop pauses when the window is hidden/minimized (`document.visibilitychange`) and when the map
  is not the active route — no GPU burn behind the tray.
- The map never blocks the main data path: it is a leaf consumer of an existing command's result.

## Architecture

`src/three/` is the only place `three` is imported (enforced by an import-boundary test):
`three/changeMap/{createScene.ts,layout.ts,nodes.ts,edges.ts,camera.ts,picking.ts,quality.ts,
dispose.ts,palette.ts}` — all plain TS, framework-free, unit-testable with a null renderer.
`src/components/visualization/{ChangeMap.svelte,ChangeMap2D.svelte,ChangeMapFrame.svelte,
MapLegend.svelte,MapTooltip.svelte,MapNodeList.svelte}`. `ChangeMapFrame.svelte` owns capability
detection and chooses 3D or 2D; it is what features import.
Svelte integration per research §4: `$effect` creates the scene on mount and returns a teardown closure;
reactive prop changes call `scene.update(data)` rather than recreating the scene; the store is read once
into a local, so a re-render never rebuilds the WebGL context.
Rust side: `analysis/change_map.rs` shapes `ChangeMapData` from an existing `Investigation` or window
result (`ts-rs`-exported), so the frontend performs no causal logic — it only positions what Rust already
decided (§81, and it keeps the ranking single-sourced).

## Related Code Files

**Create** — `src-tauri/src/analysis/change_map.rs`; `src-tauri/src/commands/change_map.rs`
(`get_change_map(investigation_id | window)`); `src-tauri/tests/change_map_payload.rs`;
`src/three/changeMap/{createScene.ts,layout.ts,nodes.ts,edges.ts,camera.ts,picking.ts,quality.ts,
dispose.ts,palette.ts}`; `src/components/visualization/{ChangeMap.svelte,ChangeMap2D.svelte,
ChangeMapFrame.svelte,MapLegend.svelte,MapTooltip.svelte,MapNodeList.svelte}`;
`src/three/changeMap/layout.test.ts`, `src/three/changeMap/picking.test.ts`,
`src/three/changeMap/dispose.test.ts`, `src/components/visualization/change-map-frame.test.ts`;
`src/lib/stores/changeMap.svelte.ts`.
**Modify** — `src/features/investigation/InvestigationsScreen.svelte` (View as map),
`src/features/timeline/TimelineToolbar.svelte` (Map toggle),
`src/features/settings/SettingsScreen.svelte` (`visualization: auto|3d|2d`),
`src-tauri/src/commands/mod.rs` + `dto/bindings.rs` (one command + parity gate),
`src-tauri/src/domain/settings.rs`, `src/features/dev/DevStatesHarness.svelte`,
`package.json` (`three` 0.185.1, `@types/three`), `docs/system-architecture.md`, `PRODUCT.md`.
**Delete** — none.

## Implementation Steps

1. `analysis/change_map.rs` + command + payload test first: nodes/edges/lanes/symptom time derived from
   the P15 result, capped and aggregated. The frontend cannot invent structure it is not given.
2. `layout.ts` as a pure function `(data, viewport) => {positions: Float32Array, colors: Float32Array,
   laneIndex, bounds}` with unit tests (stable lane order, deterministic output for identical input,
   correct scaling at 1-node and 3000-node extremes).
3. `palette.ts`: read the §41 CSS variables via `getComputedStyle(document.documentElement)` once and map
   category → `THREE.Color`; a theme change re-reads and rewrites the instance colour buffer.
4. `createScene.ts`: renderer (`antialias: true`, `powerPreference: 'low-power'`, DPR
   `Math.min(devicePixelRatio, 2)`), orthographic-ish perspective camera fitted to bounds, ambient +
   one directional light, instanced node mesh, two line meshes, symptom plane.
5. `camera.ts`: pointer-drag orbit with damping, wheel zoom, clamps, `reset()`, and a
   `reducedMotion` flag that disables damping and switches to on-demand rendering.
6. `picking.ts`: throttled raycast, `instanceId → eventId` map, hover/select state, chain dimming via the
   colour buffer.
7. `ChangeMap.svelte`: `$effect` lifecycle, resize observer, visibility pause, tooltip/overlay
   positioning, `role="img"` + `aria-label`, hidden `MapNodeList` for keyboard access.
8. `ChangeMap2D.svelte`: same contract on a 2D canvas (lane rows, time X, chain polylines, hit-testing by
   nearest-point within 8 px).
9. `ChangeMapFrame.svelte`: capability probe (create a throwaway context, check for loss), settings
   override, `webglcontextlost` → swap to 2D at runtime with a one-time toast "Switched to the simple map
   because 3D stopped working."
10. `dispose.test.ts`: mount/unmount 20 times in jsdom with a stubbed WebGL context, asserting dispose
    calls and listener removal counts; `quality.ts` frame-timing harness behind `#/dev/perf`.
11. Six states in the dev harness: loading skeleton, empty ("Nothing to map in this window."), error,
    populated (3000 nodes), 2D fallback, and the 900×600 frame.
12. Commits: (a) `feat: change map payload`, (b) `feat: deterministic map layout`,
    (c) `feat: three scene with instanced nodes and edges`, (d) `feat: map camera and picking`,
    (e) `feat: chain emphasis and symptom marker`, (f) `feat: 2d fallback map`,
    (g) `feat: map frame with capability detection`, (h) `test: layout, picking, dispose suites`.

## Todo (commit boundaries)

- [x] Rust `ChangeMapData` payload + test
- [x] Pure layout with unit tests
- [x] Scene with instanced nodes + two edge classes
- [x] Camera, picking, chain emphasis, symptom marker
- [x] 2D fallback with identical contract
- [x] Capability detection + settings override + context-loss recovery
- [x] Dispose/lifecycle tests + six states

## Success Criteria

- [x] The §74 investigation rendered as a map shows a 3-node chain as a connected ridge; selecting any
      member dims the rest and highlights the chain's edges
- [x] Clicking a node opens the same drawer as the timeline row for that event id
- [x] 3000 nodes render at the display's single-frame cadence at `#/dev/perf` on this machine (recorded number in
      the phase file); the quality tier drop is exercised by forcing `qualityTier = 'low'`
- [x] `layout.test.ts`: identical input → byte-identical `Float32Array` output; lane order matches the
      documented category order
- [x] `dispose.test.ts`: after 20 mount/unmount cycles, geometries/materials disposed, `setAnimationLoop(null)`
      called, and zero remaining pointer/resize/visibility listeners
- [x] Forcing `WebGLRenderingContext` creation to fail renders `ChangeMap2D` with no console error, and the
      2D map supports hover/select
- [x] `prefers-reduced-motion: reduce` → no continuous render loop (frame counter stays flat while idle)
- [x] Keyboard: `Tab` reaches the node list, `Enter` selects, focus returns to the map control after the
      drawer closes
- [x] Minimizing the window stops rendering (frame counter flat), restoring resumes
- [x] `rg -n "from 'three'" src --glob '!src/three/**'` returns nothing

## Acceptance evidence — 2026-09-20

- `#/dev/perf` rendered the 3,000-node forced-low fixture in headful Chrome on
  `ANGLE (Intel Iris Xe Graphics, Direct3D11)`. End-to-end `requestAnimationFrame` cadence over 300
  intervals measured **16.70 ms p50 / 16.80 ms p95**, the monitor's quantized single-frame cadence
  (about 59.9 FPS); the harness retained 41 DOM options and emitted no page errors.
- The final frontend gate passed 45 files / 274 tests, including the final theme, retained-snapshot,
  history/focus, capability-probe and end-to-end frame-cadence assertions; the
  focused Change Map gate and type-check were rerun after those assertions. The production build, generated
  binding parity, Rust formatting, strict Clippy and full all-target/all-feature Rust suite passed.
- Lifecycle coverage creates and destroys the real scene graph 20 times through a renderer test double,
  checks zero geometry/texture counters, listener symmetry, hidden-window pause/resume, reduced-motion
  on-demand rendering and context-loss notification. Interaction coverage checks 2D hover/select, drag
  suppression, chain/edge emphasis, theme palette refresh, keyboard drawer handoff and import confinement.

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test -- src/three src/components/visualization
cd src-tauri && cargo test --test change_map_payload && cargo clippy --all-targets -- -D warnings
cd /d/Project/.WhatChage && bun run tauri dev   # manual: #/dev/perf frame timing, forced 2D, 900x600
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| 3D adds no insight over the list (decorative) | Reviewer cannot name a question the map answers faster | The map's justification is chain shape + branching; if the chain-emphasis interaction does not read clearly at 3000 nodes, drop the Z axis and ship the 2D map as the only visualization (the fallback already exists, so this is a deletion, not a rewrite) |
| WebGL unavailable or context lost | Blank canvas / `webglcontextlost` | Capability probe at mount + runtime swap to the 2D map with a toast; the 2D path is a shipped, tested component, not a placeholder |
| GPU/VRAM leak across route changes | `renderer.info.memory` counters climb; RAM grows per visit | Dispose test with 20 cycles is a build gate; `forceContextLoss()` on teardown |
| Instanced picking wrong under DPR ≠ 1 | Hover highlights the wrong node on a scaled display | Raycaster uses NDC computed from `getBoundingClientRect`, never `offsetWidth`; `picking.test.ts` covers DPR 1, 1.5, 2 |
| 100k-event windows overwhelm the map | Freeze on open | Hard 3000-node cap enforced in Rust (the frontend cannot request more), with the disclosure chip |
| Theme switch leaves stale colours | Dark-mode map in light theme | Palette re-read on theme change writes the colour buffer; covered by a component test toggling the theme class |
| Reduced-motion users get a moving camera anyway | Motion persists with the OS flag set | `matchMedia` checked at init *and* subscribed for changes; assertion in the frame-counter test |

## Security Considerations

- The map renders only fields already present in the redacted event payload; node labels use
  `user_friendly_summary` (sanitized) and tooltips are DOM text nodes, never `{@html}`.
- No shader receives user-controlled strings; there is no runtime shader compilation from data.
- `three` is pinned at 0.185.1 with no examples/addons imported, keeping the audited dependency surface to
  one package.
- The visualization performs no IPC of its own beyond `get_change_map`, which is read-only and typed.
