# Owner workspace synthetic browser QA — 2026-10-28

Status: DONE_WITH_CONCERNS

## Scope and result

Bounded QA of production Svelte components with injected synthetic models. No App bootstrap,
Tauri bridge, real database, native dialog, preview target, personal browser profile or runner.
Browser checks ran before the later acceptance-editor/preview-origin corrections; those later
corrections have the focused component/helper verification recorded below.

- Broad browser run: exit 0, 32 checks passed, 33 screenshots.
- Focus/fixture follow-up: exit 0, 4 checks passed, 4 screenshots.
- Later focused Vitest: exit 0, 6 files / 23 tests passed.
- Both browser runs: no page/console errors, no blocked requests, only the localhost page origin,
  absent Tauri bridge; browsers closed in `finally`.

## Reproduction and artifacts

QA-only harness and scripts live in
`D:/Project/.WhatChage/.qa/ownvero-preflight/owner-ui/`:

- [harness.html](../../../.qa/ownvero-preflight/owner-ui/harness.html),
  [qa-owner-main.ts](../../../.qa/ownvero-preflight/owner-ui/qa-owner-main.ts),
  [qa-owner-harness.svelte](../../../.qa/ownvero-preflight/owner-ui/qa-owner-harness.svelte).
- [inspect-owner-ui.mjs](../../../.qa/ownvero-preflight/owner-ui/inspect-owner-ui.mjs): broad checks.
- [inspect-owner-focus.mjs](../../../.qa/ownvero-preflight/owner-ui/inspect-owner-focus.mjs): corrected
  focus contrast and saved-request baseline.
- [observations.json](../../../.qa/ownvero-preflight/owner-ui/observations.json) and
  [focus-observations.json](../../../.qa/ownvero-preflight/owner-ui/focus-observations.json): exact
  assertions, dimensions, focus measurements, request origins and timestamps.

Commands, with one owned localhost Vite server running:

```powershell
bun run dev --host 127.0.0.1 --port 1420 --strictPort
node .qa/ownvero-preflight/owner-ui/inspect-owner-ui.mjs
node .qa/ownvero-preflight/owner-ui/inspect-owner-focus.mjs
```

Used installed Playwright 1.62.1 and system Chrome 156 in fresh contexts. Browser background
networking disabled; HTTP and WebSocket routes permit only `http://127.0.0.1:1420`. No browser
download. These are isolated UI observations, not native integration or runner evidence.

## Measured coverage

| Surface | Observed result |
|---|---|
| My apps, app overview, change request | Ready/empty/loading/error at 900×600 and 1440×900: all 24 combinations without horizontal overflow. |
| Scenario creation/revision | Keyboard input and Select; review modal Tab containment, Escape dismissal, focus return; long old/new recipe header/footer reachable by scrolling. |
| Manual exception decision | Reason, unknown-build acknowledgment and each named unresolved gap required; `not_run` remains visible; no decision submitted. |
| Project memory revision | Keyboard-accessible old/new wording review and acknowledgment. |
| Bundle export | Reviewed preview invalidated by selection change; native export never invoked. |
| Bundle import | Every historical rule revision individually required; native apply never invoked. |
| Forced colors/reduced motion | 900×600 overview has no horizontal overflow; reduced-motion media query active. |
| Narrow reflow | 450×300 CSS viewport, deviceScaleFactor 2: no horizontal overflow. This is not native 200% browser-zoom acceptance. Harness controls consume part of this small viewport. |

Representative screenshots, all under the absolute artifact directory above:

- [Overview 900×600](../../../.qa/ownvero-preflight/owner-ui/overview-ready-900.png),
  [overview 1440×900](../../../.qa/ownvero-preflight/owner-ui/overview-ready-1440.png).
- [Scenario review](../../../.qa/ownvero-preflight/owner-ui/scenario-confirm-dialog-900.png),
  [old/new top](../../../.qa/ownvero-preflight/owner-ui/scenario-old-new-dialog-top-900.png),
  [old/new bottom](../../../.qa/ownvero-preflight/owner-ui/scenario-old-new-dialog-bottom-900.png).
- [Manual exception](../../../.qa/ownvero-preflight/owner-ui/manual-exceptions-dialog-900.png),
  [import review](../../../.qa/ownvero-preflight/owner-ui/bundle-import-reviewed-900.png),
  [export review](../../../.qa/ownvero-preflight/owner-ui/bundle-export-reviewed-900.png).
- [Light focus](../../../.qa/ownvero-preflight/owner-ui/focus-light-900.png),
  [dark focus](../../../.qa/ownvero-preflight/owner-ui/focus-dark-900.png),
  [forced-colors focus](../../../.qa/ownvero-preflight/owner-ui/focus-forced-colors-900.png),
  [saved synthetic request](../../../.qa/ownvero-preflight/owner-ui/change-saved-fixture-fixed-900.png).

## Findings repaired

### Focus indicator

The broad run exposed a visible but weak shared outline: existing `--border-active` on white,
1.81:1. Root authorized the narrow shared-token correction. `src/styles/tokens.css` now sets
`--focus-ring: 2px solid var(--text-secondary)` using the existing neutral token.

Follow-up keyboard measurements on the Review revision control:

| Mode | Contrast against containing surface | Lowest of background/surface-1/2/3 |
|---|---:|---:|
| Light | 7.840:1 | 6.501:1 |
| Dark | 7.996:1 | 6.922:1 |
| Forced colors | 15.134:1 | 15.134:1 |

All measured outlines were solid, 2px, `:focus-visible`. These measurements do not constitute a
complete accessibility audit of every screen, theme and control.

### Saved synthetic request baseline

The synthetic ready-state fixture assigned a saved request's public fields directly, leaving its
private comparison baseline unset and showing an incorrect Unsaved changes badge. It now loads
immutable synthetic app/draft/rule data through `ChangeRequestStore` dependency injection, with
write dependencies rejecting. Loading/empty/error states remain explicit and cancel stale loads.
Production dirty-state behavior remains intact. Follow-up browser observation and the regression
test verify saved wording, no dirty badge, disabled controls, ready→loading→ready and no real client
read/write.

## Later acceptance/preview corrections

Backend scouting identified route/environment mismatches and origin loss while editing preview
labels. Corrected within the authorized UI ownership:

- Scenario origins use the app-selected candidate's exact environment ID/revision. Missing, stale
  or revoked revisions expose a blocker instead of choosing another environment. Tuple options
  defensively deduplicate before keyed Select rendering.
- Read-back actions offer read-only routes; denied-access actions offer denied-access routes.
  Existing checks survive action/purpose/removal edits. Visible feedback blocks review for route,
  declared-role, allowed-field and assertion compatibility mismatches.
- Add-check controls enforce 100 per step / 1000 per scenario; the reviewer reports excessive
  imported/existing drafts as well. Existing step, route, role, fixture and field bounds retained.
- Preview setup loads additional API origins on edit, preserves them on a label-only revision,
  accepts separate exact origins, canonicalizes/deduplicates and caps at 16 including the preview
  URL origin. Route paths rejected before write. Copy explicitly says configuration is offline.
- New local helpers: `owner-origins.ts`, `scenario-validation.ts`. Backend remains validation
  authority; the UI performs no target request.

Actual completed command, exit 0 (123.66 seconds):

```powershell
bunx vitest run src/features/owner-workspace/owner-workspace.test.ts src/lib/stores/change-request.test.ts src/features/acceptance/scenario-validation.test.ts src/features/acceptance/adapter-editor.test.ts src/features/acceptance/scenario-editor.test.ts src/features/owner-workspace/preview-setup.test.ts --maxWorkers=1 --pool=threads
```

Six files, 23 tests passed. Covers exact candidate environment, missing/stale/revoked handling,
duplicate origins, compatible purpose options, preserved incompatible criteria, role/field/deleted
route feedback, assertion boundaries, API-origin preservation and offline origin authoring.
Root reported final post-freeze `bun run check` exit 0 (0 errors / 0 warnings) and production build
exit 0. Those integration commands ran in the controller session.

## Tool failures and observation corrections

- Installed agent-browser named session timed out with OS 10060 for open/set/snapshot. Offline
  doctor passed; only the owned named session was closed. Used the installed Playwright fallback.
- node_repl failed before execution with missing kernel assets (OS 3); no browser state came from it.
- Initial broad metrics counted an off-screen `sr-only` legend; corrected the QA metric. Escape
  initially asserted before the dialog became hidden; now waits for hidden state.
- QA harness uses explicit `height:100dvh`; ignored `.qa` files were outside Tailwind class scanning.
- Two narrow attempts timed out during cold Vite/module startup. Final run waits for document
  commit and the rendered heading; no fixed sleep. Initial observations and failed attempt JSON
  retained alongside final evidence.

## Cleanup and limits

Owned Vite command: `bun run dev --host 127.0.0.1 --port 1420 --strictPort`.
First PID 13204/parent 18764; follow-up PID 26816/parent 38420. Both stopped via their exec sessions.
Owned agent-browser daemon PID 26464 closed; only inert config/target sidecars remain.
[process-cleanup.json](../../../.qa/ownvero-preflight/owner-ui/process-cleanup.json) and
[focus-process-cleanup.json](../../../.qa/ownvero-preflight/owner-ui/focus-process-cleanup.json)
record zero listeners on 1420 and no owned processes remaining. A final listener check also found
no listener. Test process exited. No Cargo, commits or external network activity in this QA task.

Open acceptance limits: native dialogs/filesystem overwrite and import/export round trip, real
storage/archive round trip, production Tauri shell integration, native 200% zoom, screen reader/full
accessibility audit, full post-correction browser walkthrough and packaged runner remain unverified
by this report. Synthetic passed checks do not close those gates.

Unresolved questions: none for the corrected UI contracts.
