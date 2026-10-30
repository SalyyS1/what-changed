# Website overhaul code review

## Overall assessment

Pre-Landing Review: no open code issues. All three prior findings are repaired in
the current source. The static export, CSP shape, exact-download fail-closed path,
lazy full demo, source-input preflight, lifecycle handling, and Vietnamese model
presentation now match the reviewed contracts.

**Code verdict: PASS.** Controller owns final browser/test evidence and release
decision; this read-only review does not grant publication or release approval.

## Scope

- Plan: `plans/261030-1908-website-overhaul-release-readiness/plan.md`
- Site: `wiki-site/product.css`, `wiki-site/src/{landing,layout,client,demo,demo-template,workflow-exhibit,workflow-tour,workbench-3d}.mjs`, build/check scripts and site tests
- Runtime gate: `scripts/check-preview-runtime-inputs.mjs`, its tests, and `.github/workflows/build-windows-preview.yml`
- Focus: specification parity, async lifecycle/BFCache, WebGL loss and cleanup, motion, keyboard/mobile, CSP/export boundary, download fail-closed behavior

## Resolved findings

### 1. RESOLVED — pending initial 3D import resumes after BFCache restore

`wiki-site/src/workflow-tour.mjs:10` now records the requested mode independently
as `wants3d`. The generation guard still prevents the invalidated import from
attaching, while persisted `pageshow` restarts the load when `wants3d` remains
true (`:63-67`). Explicitly choosing 2D and context-loss fallback set it false,
so restoration does not override the visitor or recovery state.

`wiki-site/tests/workflow-lifecycle.mjs:17-30` delays the dynamic chunk across
synthetic persisted `pagehide`/`pageshow`, then verifies both auto-3D recovery and
survival of an explicit 2D choice.

### 2. RESOLVED — recreated demo scenes reapply retained controls

Every new scene now receives the current zoom, explode, and layer control values
before selection/result state is applied (`wiki-site/src/demo.mjs:36-41`). This
path is shared by first load, BFCache recreation, and context-loss retry.

`wiki-site/tests/workflow-lifecycle.mjs:44-60` selects a non-default layer,
separation, and zoom; it compares rendered pixels after persisted restore and
after actual `WEBGL_lose_context` recovery, while also asserting two visible
labels for the selected layer.

### 3. RESOLVED — Vietnamese model text and accessible names are localized

`bookingNodeCopy` provides Vietnamese node kind/label presentation without
changing graph identifiers or evidence (`wiki-site/src/demo-data.mjs:28-40`).
The poster and canvas accessible names are localized
(`wiki-site/src/workflow-exhibit.mjs:13-21`), and the renderer applies localized
visible labels, verdicts, ARIA labels, context-loss fallback, and model summary
(`wiki-site/src/workbench-3d.mjs:6-8,76-83,169-182,208-215,266-270`).

Static content checks assert the Vietnamese poster label, while
`wiki-site/tests/workflow-lifecycle.mjs:39-42` verifies the enhanced Vietnamese
button name and absence of the former English node title.

## Edge cases checked

- `await import` invalidation prevents a late scene from being attached after `pagehide`; no late-resource leak found.
- Workbench disposal cancels the pending frame, disconnects `ResizeObserver`, removes listeners/labels, disposes geometries/materials/shadow/renderer, and removes the canvas.
- WebGL denial and loss retain the complete 2D explanation; the full demo retry is re-enabled through `workbenchfallback`.
- Reduced motion disables CSS animation/transitions; the renderer has no autonomous animation loop.
- Full `/demo/` workbench remains user-triggered and dynamically loaded; the landing alone auto-loads the same chunk.
- Export remains fail-closed: `product.css` is the only added named output, hashed JS chunks stay allowlisted, publication remains false, and no installer appears without exact approval.
- Runtime preflight rejects changed/missing/unlisted source, unsafe paths, case-folded duplicates, and redirected directories before runtime hydration.

## Verification

- PASS: `node --test scripts/check-preview-runtime-inputs.test.mjs` (7/7)
- PASS: `node scripts/check-preview-runtime-inputs.mjs` (11 files, 40,681 bytes)
- PASS: `bun run site:build` (19 pages; download unavailable)
- PASS: `node scripts/check-site.mjs` (19 pages, 29 allowlisted files)
- PASS: content and download tests (11/11)
- SOURCE-VERIFIED: lifecycle helper covers delayed import restore, explicit 2D persistence, actual graphics-context loss/retry, localized enhanced labels, and pixel-equivalent retained demo controls.
- CONTROLLER-OWNED: final navigation/lifecycle browser execution on the controller's active server. No duplicate server was started by this reviewer.

## Plan status recommendation

- Rebuild/styling: implemented; rendered-browser evidence still belongs to controller.
- 3D/static fallback/keyboard/reduced motion/context cleanup: implemented; all three review findings resolved in code with targeted lifecycle coverage.
- Release workflow readiness: source-input preflight is present and passing; release approval/evidence remains owned elsewhere and unchanged by this review.
- Source/site validation: focused source checks pass; controller owns final browser evidence.
- Commit/publication: not reviewed or authorized here.

## Metrics

- Critical security/auth/data findings: 0
- Type coverage: not applicable to reviewed `.mjs` site surface
- Test coverage: no numeric coverage artifact available
- Lint/type issues: none measured for this isolated surface; `git diff --check` is noisy from the separately owned generated runtime inventory

## Unresolved questions

- Controller-owned full browser-suite result was not duplicated in this read-only review.
