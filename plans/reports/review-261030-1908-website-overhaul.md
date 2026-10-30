# Website overhaul code review

## Overall assessment

Pre-Landing Review: 1 open release-contract issue (0 critical security defects,
1 high-priority G3 blocker). All three website findings remain repaired. The
static export, CSP shape, lazy full demo, source-input preflight, lifecycle
handling, and Vietnamese model presentation match the reviewed contracts, but
the current public-download contract cannot represent the approved complete
runner redistribution packet.

**Code verdict: PASS for the website repairs; NO-GO for a runner-enabled public
download until G3 records an approved packaging contract.** Controller owns final
browser/test evidence and release decision; this read-only review does not grant
publication or release approval.

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

## High priority

### 4. Current manifest cannot represent the approved same-packet runner distribution

The explicit source policy requires the installer plus all seven verified source
archives in one complete downloadable packet
(`release/runner-source-delta-approval.json:31-34` and
`docs/runner-source-build.md:20-35`). The current QA artifact is 2,227,488,699
bytes, which is 80,005,051 bytes above the manifest's 2,147,483,648-byte maximum.

The mismatch is structural, not merely the current size:

- `scripts/check-download-manifest.mjs:27,37-40` accepts only one artifact no
  larger than 2 GiB whose URL ends in `.exe`.
- `release/release-manifest.schema.json:7,18-41` rejects additional packet/source
  fields and likewise permits only that `.exe` identity.
- `scripts/check-owner-release.mjs:42-53` binds the public manifest hash and size
  to the installer artifact, not to the complete downloadable packet or its seven
  source archives.

Therefore the current code cannot truthfully enable a runner-equipped public
release while preserving the user's same-packet/source policy. The existing
`status: unavailable` remains correct and fail-closed; this does not invalidate
the QA Actions artifact or reverse its approved delivery policy.

Minimal next action: keep the download unavailable and make a deliberate G3
packaging decision before publication. That decision must identify one public
complete-packet route and update the manifest validator/schema and owner-release
binding together so G3 hashes and sizes the downloadable packet that contains the
installer and seven sources. Do not approve an installer-only manifest or split
source access without a new explicit user decision.

## Edge cases checked

- `await import` invalidation prevents a late scene from being attached after `pagehide`; no late-resource leak found.
- Workbench disposal cancels the pending frame, disconnects `ResizeObserver`, removes listeners/labels, disposes geometries/materials/shadow/renderer, and removes the canvas.
- WebGL denial and loss retain the complete 2D explanation; the full demo retry is re-enabled through `workbenchfallback`.
- Reduced motion disables CSS animation/transitions; the renderer has no autonomous animation loop.
- Full `/demo/` workbench remains user-triggered and dynamically loaded; the landing alone auto-loads the same chunk.
- Export remains fail-closed: `product.css` is the only added named output, hashed JS chunks stay allowlisted, publication remains false, and no installer appears without exact approval.
- Runner public export remains fail-closed, but its current `.exe`-only identity cannot express the required complete same-packet source distribution.
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
- G3 packaging: keep open; current manifest/schema/owner-release binding cannot represent the approved complete runner packet.
- Source/site validation: focused source checks pass; controller owns final browser evidence.
- Commit/publication: not reviewed or authorized here.

## Metrics

- Critical security/auth/data findings: 0
- Open release-contract findings: 1 high-priority G3 blocker
- Type coverage: not applicable to reviewed `.mjs` site surface
- Test coverage: no numeric coverage artifact available
- Lint/type issues: none measured for this isolated surface; `git diff --check` is noisy from the separately owned generated runtime inventory

## Unresolved questions

- Which complete-packet public route will the user approve for G3? Current code supports none; no route/schema design was inferred by this review.

## CI 41086258362 repair re-review

**Narrow code verdict: PASS — no blocker to push the fixed QA workflow.** This
verdict covers reviewed license/NOTICE input integrity only; it does not close
native acceptance, public G3, or the complete-packet decision above.

Evidence:

- `scripts/check-reviewed-notice-inputs.mjs:10-40` requires the existing manifest
  to remain reviewed, bounded, uniquely keyed, and targeted to Windows x64, then
  delegates every one of its 232 package entries to the existing
  `reviewedLicense` authority. That authority checks safe in-tree real paths,
  exact raw-byte hashes, nonempty license/NOTICE files, reviewed obligations,
  license selection, and authoritative HTTPS source routes. The new check does
  not regenerate text, normalize CRLF, or mutate review records.
- The six focused tests cover exact CRLF preservation, LF normalization failure,
  changed and missing NOTICE bytes, incomplete human review, unsupported/empty
  manifests, and duplicate package identities. Local execution passed 6/6; the
  repository preflight passed 232 license and 33 NOTICE hashes.
- `.github/workflows/build-windows-preview.yml:35-43` runs both focused test files,
  the frozen runner-source check, and the reviewed-notice preflight before the
  source-material, runtime hydration, build, bundle, or upload steps. Each command
  propagates a nonzero exit before downloads relevant to redistribution.
- `scripts/prepare-public-source.mjs:38,58-68` preserves the published manifest
  review record in the isolated clone, copies reviewed legal/runtime notice bytes
  byte-for-byte, and then reruns the same checker against the completed projection.
  A projection that changes or omits reviewed bytes fails before it can be used.

No terms, license selections, obligation decisions, source routes, approval
records, or publication state are changed by this repair. No new concrete
regression, trust-boundary defect, or approval bypass was found in the reviewed
delta.
