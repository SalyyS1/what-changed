---
phase: 17
title: "System Map: §48 relationship graph and dependency inspection"
status: completed
priority: P2
effort: 7h
dependencies: [16]
---

## Overview

§48 asks a different question from §35, and that is why it is a different screen. The Change Map (P16)
asks *what happened, in what order*; the System Map asks *what depends on what*. The data already
exists — `entity_relationships` rows written by P09 (`listens_on` / `opened_by`), P10 (`config_of`),
P11 (`provides`) and P18 (`publishes`) — so this phase is a **consumer of the graph, not new
collection**. It answers "What depends on this?" for an application, process, service, port, container,
network adapter or config file, and it is the data the future §77 Impact feature will need.

§48's constraint "must **not** be necessary for normal users" is honoured literally: no flow on Home,
Timeline or Investigations requires visiting this screen, and P20 carries the assertion that proves it.
This phase exists because red-team finding Y-6 found §48 to be the one requirement of 91 with a sidebar
destination (P02), an allowlisted IPC command (P05) and no implementing phase — which under §87 makes
`get_system_map` a faked implementation until it is built here.

## Requirements

**Functional**
- `get_system_map(focus: Option<EntityRef>, depth: u8, kinds: Vec<EntityType>) -> SystemMapData`.
  `focus` absent returns the highest-degree component roots (the graph's natural entry points) rather
  than an arbitrary slice. `depth` is clamped to `1..=3` in Rust; the UI defaults to 2.
- Payload, shaped in Rust so the frontend positions what it is given (§81):
  `SystemMapData { nodes, edges, focus, truncated, total_nodes_available, dropped_edges,
  kind_availability, focused_recent_events }` where
  `MapNode { entity_id, entity_type, display_name, hop, degree, last_changed_at: Option<i64>,
  recent_event_count: u32 }` and
  `MapEdge { source, target, kind: RelationshipKind, recorded_at: i64, evidence_event_id: Option<i64> }`.
  `recorded_at` is the latest retained proof timestamp for the directed triple; the schema does not
  fabricate an unavailable first-seen history. The focused recent-event list is newest-first and capped at five.
  `evidence_event_id` is what makes an edge inspectable instead of asserted.
- Layout is **deterministic and non-temporal**: concentric rings by hop distance from the focus, nodes
  ordered within a ring by `(entity_type, display_name)` so the same graph always draws the same way.
  No force simulation — a layout that settles differently on each open cannot be reasoned about, and
  §48's value is recognising a shape you have seen before.
- "What depends on this?" panel beside the canvas, split by edge direction: *depends on this* (inbound)
  and *this depends on* (outbound), each row naming the relationship kind in plain language ("listens on
  port 3000", "reads this config file") with a link to the evidence event.
- Entity-kind filter chips. A kind with zero recorded rows renders **disabled with the reason** —
  `No Docker containers recorded — the Docker collector is off` — never as an empty graph with no
  explanation.
- `Ctrl+K` search results gain a "Show in System Map" action that focuses that entity; clicking a node
  changes focus and re-queries rather than opening the drawer. A per-node `Recent changes` list opens the
  P07 details drawer for an individual event, so the two screens share one drawer.
- Caps: 400 nodes and 1200 edges at depth 3, enforced in Rust with `truncated: true` and the disclosure
  chip `Showing 400 of 1,832 connected things`.
- Full 2D canvas fallback (`SystemMap2D.svelte`) on the same props/events contract, used on WebGL
  failure, context loss, or the `visualization: "2d"` setting — identical policy to P16.
- Empty state: `Nothing is connected yet. Relationships appear as collectors record them.`

**Non-functional**
- 60 FPS at the 400-node cap, measured with P16's frame-timing harness; the same `qualityTier` constant
  governs both maps.
- `get_system_map(depth = 3)` returns in ≤ 120 ms against a 100k-event / 50k-relationship database, with
  a query-plan assertion proving the walk uses `entity_relationships(source_id)` and `(target_id)` —
  a plan that scans the table fails the test rather than merely being slow.
- Teardown is the same hard gate as P16: renderer disposed, context force-lost, geometries and materials
  disposed, animation loop cleared, listeners removed, asserted over 20 mount/unmount cycles.
- Render loop pauses when the window is hidden and when this route is not active.
- No allocation inside the render loop.

## Architecture

`analysis/system_map.rs` builds the payload by bounded breadth-first expansion over
`entity_relationships`, using both indexes P04 creates, with an explicit visited set and no recursion
(depth is user input; a recursive walk would make it a stack-depth question). It imports nothing from
`platform/` or `collectors/`, like the rest of `analysis/`.

The renderer is **shared, not duplicated**. This phase promotes P16's framework-free modules from
`src/three/changeMap/` to `src/three/shared/{renderer.ts,camera.ts,picking.ts,dispose.ts,palette.ts}`
and re-points the Change Map's imports at them — a mechanical move whose gate is P16's own test suites
passing unchanged. That promotion is the reason this phase depends on 16 rather than running in
parallel with it, and it is why ARCHITECTURE.md can claim the two screens share capability detection,
disposal discipline and the 2D machinery instead of each shipping their own.

`src/three/systemMap/{layout.ts,scene.ts}` hold the only System-Map-specific 3D code.
`src/components/system/{SystemMap.svelte,SystemMap2D.svelte,SystemMapFrame.svelte,
DependencyPanel.svelte,KindFilter.svelte}` compose it; `SystemMapFrame.svelte` owns capability
detection and is what the feature imports. The import-boundary test is unchanged: `three` is still only
reachable under `src/three/`.

## Related Code Files

**Create** — `src-tauri/src/analysis/system_map.rs`; `src-tauri/src/commands/system_map.rs`;
`src-tauri/tests/system_map_payload.rs`; `src-tauri/tests/perf_system_map.rs`;
`src/three/shared/{renderer.ts,camera.ts,picking.ts,dispose.ts,palette.ts}` (moved from `changeMap/`);
`src/three/systemMap/{layout.ts,scene.ts}`; `src/three/systemMap/layout.test.ts`;
`src/components/system/{SystemMap.svelte,SystemMap2D.svelte,SystemMapFrame.svelte,
DependencyPanel.svelte,KindFilter.svelte}`;
`src/components/system/system-map-frame.test.ts`;
`src/features/system-map/{SystemMapScreen.svelte,copy.ts}`;
`src/lib/stores/systemMap.svelte.ts`.
**Modify** — `src/three/changeMap/*` (import from `../shared/`, no behaviour change);
`src/app/routes.ts` (register `#/system-map` behind the sidebar entry P02 already renders);
`src-tauri/src/commands/mod.rs` + `dto/bindings.rs` (one command, already allowlisted in P05);
`src/features/search/SearchPalette.svelte` ("Show in System Map" action);
`src/features/dev/DevStatesHarness.svelte` (six states);
`docs/system-architecture.md`, `PRODUCT.md`, `ARCHITECTURE.md`.
**Delete** — none. The `changeMap/` modules are moved, not removed, and git records the rename.

## Implementation Steps

1. **Promote the shared modules first, as its own commit.** Move
   `renderer/camera/picking/dispose/palette` to `src/three/shared/`, re-point `changeMap` imports, and
   run P16's full suite plus the import-boundary test. This is the riskiest step in the phase and the
   cheapest to revert, so nothing else starts until it is green.
2. `analysis/system_map.rs`: bounded BFS, degree computation, caps, `truncated`. Payload test uses a
   hand-built relationship fixture covering all five edge kinds — `process → listens_on → port`,
   `port → opened_by → process`,
   `application → provides → service`, `file → config_of → application`,
   `container → publishes → port` — and asserts ring membership, cap behaviour and determinism.
3. `commands/system_map.rs`: validate the entity id against the P04 id grammar, clamp depth, map the
   error; ≤ 30 lines, no logic.
4. `layout.ts` as a pure function `(data, viewport) => { positions, colors, ringOf, bounds }` with unit
   tests: identical input yields byte-identical `Float32Array`s, ring assignment matches hop distance,
   and no two nodes overlap at the 400-node cap.
5. `scene.ts` on the shared renderer: instanced nodes sized by `degree`, edges coloured by
   `RelationshipKind`, non-neighbour dimming on focus through the instance colour buffer.
6. `DependencyPanel.svelte`: the two directions, plain-language edge phrasing, evidence links.
7. `SystemMap2D.svelte` + `SystemMapFrame.svelte`: capability probe, settings override, runtime
   context-loss swap — the same contract and the same toast copy as P16.
8. `KindFilter.svelte`: disabled-with-reason chips driven by the payload's per-kind counts, plus all six
   states in the dev harness including the 900×600 frame.
9. `perf_system_map.rs`: seed 50k relationship rows over the P20 generator, assert the latency budget and
   the query plan.
10. Commits: (a) `refactor: promote shared three modules`, (b) `feat: system map payload`,
    (c) `feat: deterministic radial layout`, (d) `feat: system map scene`,
    (e) `feat: dependency panel with evidence links`, (f) `feat: 2d fallback and capability frame`,
    (g) `test: payload, layout and perf suites`.

## Todo (commit boundaries)

- [x] Shared three modules promoted; P16 suites green unchanged
- [x] Rust `SystemMapData` payload + BFS + caps + test
- [x] Pure radial layout with determinism tests
- [x] Scene with degree-sized nodes and kind-coloured edges
- [x] Dependency panel with both directions and evidence links
- [x] 2D fallback + capability frame + kind filter
- [x] Perf suite with query-plan assertion

## Success Criteria

- [x] §48's question is answerable end to end: focusing `port:tcp:0.0.0.0:3000` shows the process that
      opened or listens on it, an application/service or container/port neighbourhood when those
      relationships were recorded, and any config-to-application edge in range, with every displayed
      edge linking to its recording event when that retained evidence is available. The map does not
      synthesize process-to-executable or service-to-process edges absent from the persisted vocabulary.
- [x] Every entity type §48 names has a node kind that renders: application, process, service, port,
      Docker container, network adapter, config file
- [x] `get_system_map(depth = 3)` on a 50k-relationship database returns in ≤ 120 ms and its query plan
      uses `entity_relationships(source_id)` and `(target_id)`
- [x] `layout.test.ts`: identical input → byte-identical output; ring index equals hop distance
- [x] P16's Change Map suites pass unchanged after the shared-module promotion
- [x] Forcing WebGL creation to fail renders `SystemMap2D` with no console error, and focus/hover work
- [x] 20 mount/unmount cycles leave zero geometries, zero materials and zero listeners
- [x] A kind with no recorded rows renders a disabled chip naming the reason
- [x] Over the 400-node cap, the disclosure chip states both numbers
- [x] No Home, Timeline or Investigations flow requires the System Map (the assertion lives in P20)
- [x] `rg -n "from 'three'" src --glob '!src/three/**'` still returns nothing

## Acceptance Evidence — 2026-09-20

- The shipped payload exposes the latest retained `recorded_at` proof for each directed triple; it does
  not invent first-seen history. The fixture covers all five persisted relationship kinds (`listens_on`,
  `opened_by`, `provides`, `config_of`, `publishes`) and rejects the synthetic process-to-executable edge
  that the storage vocabulary cannot prove.
- The storage-owned breadth-first walk returned an exact `total_nodes_available` of **50,001** from the
  dense 50,000-relationship fixture across 20 queries with **74.254 ms first/max, 67.745 ms median and
  73.951 ms p95**, inside the 120 ms gate. Query-plan assertions cover
  both the source and target relationship indexes.
- The final frontend gate passed **54 files / 304 tests**; typecheck and production build passed. Focused Rust
  gates passed: command tests **9**, architecture **10**, payload **4**, performance **1**. Generated
  bindings, rustfmt and strict Clippy were clean.
- The headful hardware-browser gate rendered 400 nodes on Intel Iris Xe / Direct3D11 at **16.70 ms p50 /
  16.90 ms p95**, with 41 accessible options, the dependency panel beside the map, and no page errors.
  Twenty scene mount/unmount cycles retained no renderer resources or listeners.
- The repository-wide Rust run reached an unrelated Phase 09 real-machine process timing assertion; that
  host-sensitive failure is retained as out-of-scope evidence rather than represented as a Phase 17 pass.

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test -- src/three src/components/system src/components/visualization
cd src-tauri && cargo test --test system_map_payload --test perf_system_map \
  && cargo clippy --all-targets -- -D warnings
cd /d/Project/.WhatChage && bun run tauri dev   # manual: focus an entity, forced 2D, 900x600
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Promoting P16's modules to `shared/` breaks the Change Map | P16 suites red after step 1 | Step 1 is its own commit whose gate is P16's full suite; revert is one `git revert` and the phase continues with the modules duplicated, recorded as debt in `ROADMAP.md` rather than carried silently |
| The graph is an unreadable hairball at depth 3 | A reviewer cannot trace a single dependency path on screen | Depth defaults to 2, ring layout is fixed, non-neighbour dimming on focus; if depth 3 is still unreadable, cap the UI at 2 and keep depth 3 reachable only through the dependency panel's list, which does not need to be legible as a picture |
| Relationship data is too sparse for the screen to be useful | Fewer than ~20 edges after a day of real recording | Report it as a **collector** finding, not a UI one: the panel names which collectors contribute edges and which are disabled. Never synthesize an edge to make the screen look populated (§87) |
| §48's "not necessary for normal users" erodes over time | A Home or Investigations flow starts linking here as a required step | P20 carries the assertion; the route stays reachable but never on a required path |
| Two live WebGL contexts if both maps mount | Context-loss warnings; GPU memory climbs across route changes | The shared module owns renderer creation and the dispose test asserts a single live context; route change disposes before the next mount |
| Entity ids drift so edges point at nothing | Nodes with a degree the payload cannot resolve | The payload drops unresolvable edges and reports the count; a non-zero count fails `system_map_payload.rs`, which makes P04's id-grammar stability a build gate rather than an assumption |

## Security Considerations

- The map renders only entity display names and relationship kinds, both already sanitized at write time
  by the P03 chokepoint. No file contents, no config values, no command lines, no environment values
  reach this payload — there is nothing here that the timeline does not already show.
- Tooltips, the dependency panel and the kind filter render DOM text nodes only; no `{@html}`, so a
  hostile container label or service display name cannot become markup.
- `get_system_map` takes one entity id and two bounded scalars. No path, no SQL string, no shell
  argument; the entity id is validated against the P04 grammar before it reaches a query, and an
  unparseable id is a validation error rather than a lookup.
- The payload is capped in Rust, so a compromised frontend cannot request an unbounded graph as a
  memory-exhaustion path.
