# Owner semantic workbench implementation

Status: DONE_WITH_CONCERNS

## Delivered

- One shared immutable semantic projection of capability → confirmed rule revision → scenario revision → reviewed result. Links only from stored references. Backend result, freshness, method, counts, decisions and eligibility remain authoritative. No graph-derived pass, score or inferred file-impact edge.
- Intent / Preserve / Brief / Verify / Decide journey: real saved-request, preserved-rule, attempt and decision counts; persistent selected step; exact missing prerequisites; links into recorded artifacts. Brief explicitly says review not recorded because no durable brief-review record exists.
- Deterministic 2D map: bounded pan/zoom, reset, fit selected, explicit selected-request overlay, coverage gaps, capability isolation and state filters.
- Complete virtualized DOM list: shared selection, instance-scoped IDs, arrows/Home/End/Page/typeahead, Escape return, active-row scroll reconciliation and dataset shrink handling. Same exact inspector in all modes.
- Inspector: frozen expected/observed assertions and recipe; rule revision/provenance; scenario roles/fixtures; candidate/trust; environment/revision/origins; request/scope; run/method/lifecycle; manifest/semantics; native execution identity/revision/scope; freshness reasons, limitations and separate owner decisions.
- Before/after: explicitly named attempts/builds, full backend totals, same-scenario comparison; changed revision/rules/assertions/semantics or absent scenario gets incomparable explanation and both exact evidence targets. No percentage improvement from omitted checks.
- Lazy Three workbench: orthographic/isometric intent / scenario / observed-result planes, picking, readable DOM labels, fit/reset/zoom, capability isolation, layer visibility/separation and attempt comparison using the same graph. Maximum 120 cards / 240 links per visual group with explicit grouping; the full matching list and original evidence totals remain available.
- Demand rendering; hidden/offscreen pause; DPR cap and low-quality mode; repeated slow-render fallback; context/import/capability fallback; forced-colors list; immediate cameras including reduced motion; disposal of listeners, observers, controls, geometry/materials and renderer.
- Overview + RunReview integration retains original forms and no-map workflows, including archived read-only contexts. Run review is method-neutral, manual recording/finish/cancel/retry only for manual attempts, owner decision only terminal. Automated evidence includes runner summary and durable capture gallery.

## Verification

| Evidence | Outcome |
|---|---|
| `owner-visuals-tests.log` | 20 passed: projection state parity/ownership, explicit gaps, bounded grouping, comparisons, list keyboard/shrink/two-instance parity, mode selection/fallback, 3D layout and lifecycle |
| `owner-map-method-tests.log` | 7 passed: manual/automated/queued/terminal review boundaries plus scene lifecycle; thread pool used after fork startup timeout |
| `owner-visuals-check.log` | `bun run check`: 0 errors, 0 warnings at that revision; final narrow copy/UI additions require root final integration check |
| `owner-map-regression-tests.log` | Existing system-list/frame, system-scene and change-map disposal: 4 files / 16 tests passed. Separate acceptance test worker did not start before timeout; command exited 1. Not a clean all-file pass |
| `owner-map-browser-success.json` | Real WebGL loaded in headless Edge with reduced motion; fit/zoom/layer separation exercised; evidence selection persisted 2D → 3D → context-loss fallback; fallback preference 2D; forced colors list; 0 runtime errors |
| `owner-map-render/3d-reduced-motion.png`, `2d-dark-inspector.png`, `forced-colors.png` | Successful synthetic browser screenshots; 3D screenshot exposed default black clear color, repaired to theme surface token afterwards |
| `owner-map-render/render-report.json` | Initial rendered checker: 768/1440 no findings; 375 had harness favicon 404 and small targets. Fixed favicon and zoom button width. Checkbox inputs already have 44px clickable labels. Later checker interrupted by concurrent HMR/reload and stopped; final automated recheck not claimed |
| `owner-map-browser-check.log`, `owner-map-browser-results.json` | Last browser rerun timed out during navigation; empty result is preserved as failure. Successful prior session recorded separately above |

Scene tests exercise 20 open/close cycles, idempotent cleanup, shared-geometry disposal once, each material disposal, concurrent mounts, shrinking data, context loss and no idle animation loop. These are mocked-renderer lifecycle checks, not a GPU memory benchmark.

## Design review

Register: Product. Existing app tokens/type scale retained. Calm, precise, evidence-centered workbench; no decorative RGB, motion or acceptance repaint. Shared list and inspector carry readable semantics.

Initial screenshot critique: hierarchy 2, spacing 2, alignment 2, color discipline 2, depth/shape 2, content 2, absence of decorative patterns 2; brief fidelity 1 (current app palette remains under controller branding work), typography 1 (dense inspection), states 1 (full native accessibility acceptance open), responsiveness 1 (initial touch-width issue fixed but final rendered recheck incomplete), craft 1 (native select styling). 19/24; do not treat as acceptance. Weak items repaired: zoom target width, WebGL clear surface, journey artifact navigation. Updated rendered score remains unverified due final navigation timeout.

## Remaining acceptance evidence

- Named-machine cold/warm 3D load, 120-node / 240-edge frame budget, low-tier fallback, private/total working set and long-session GPU memory.
- Current installed-app route/remount, native WebView2 WebGL, display scaling/200% zoom, screen reader, keyboard-only and forced-color walkthrough.
- Real-evidence and user comprehension check: owner can explain which scenario supports which exact rule in both 2D and 3D; repeated real app change.
- Root final check/build/bundle gates, independent scoped review/simplification, and phase-status reconciliation. Existing tests remain an evidence record, not release approval.

## Process ownership

Own Vite command `bun run dev`, workspace `D:/Project/.WhatChage`, child PID 16596, deterministic port 5183. Stopped after QA; normal termination failed, forced termination succeeded. Browser sessions use isolated temporary profiles and close in `finally`. Render-check helper PID 13368 required forced tree cleanup after normal termination failed; all belong to this QA attempt. No user browser/profile/process targeted.

Unresolved questions: none. Unmeasured acceptance checks above remain open.
