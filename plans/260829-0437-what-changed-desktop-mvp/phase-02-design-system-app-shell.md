---
phase: 02
title: "Design System & App Shell"
status: completed
priority: P1
effort: 6h
dependencies: [01]
---

## Overview

Build the visual identity and the navigation shell so every later screen inherits it: §41 design
tokens as CSS variables, Dark/Light/System, custom window chrome, sidebar, the seven routes as
real (empty-state) screens, the six-state primitive, motion tokens, and the a11y baseline. This is
the phase that decides the app does not look like a generic dashboard (§40/§70).

## Requirements

**Functional**
- Tokens: `background`, `surface-1/2/3`, `border-subtle`, `border-active`,
  `text-primary/secondary/muted`, `accent`, `success`, `warning`, `danger`, `info`,
  `shadow-sm/md/lg`, `radius-sm/md/lg/xl`, spacing scale, motion durations, easing curves — all in
  `src/styles/tokens.css`, re-exported to Tailwind via `@theme` in `src/styles/app.css`.
- Dark default (documented as the stronger MVP surface), Light complete, System follows
  `matchMedia('(prefers-color-scheme: dark)')`; theme applied via `data-theme` on `<html>`.
- Custom chrome: 36px draggable titlebar (`data-tauri-drag-region`), minimize/maximize/close
  buttons, close → hide (wired in P05), traffic-light-free Windows layout, keyboard reachable.
- Sidebar nav with seven destinations: Home, Timeline, Investigations, Checkpoints, System Map,
  Settings, Diagnostics; collapses to icon rail below 1040px; active-route indicator animates.
- Hash-based typed router in `src/lib/stores/router.svelte.ts` (no SvelteKit).
- `StateShell.svelte` renders exactly one of `loading | empty | error | content` from an
  `AsyncState<T>` union; `large-data` and `small-window` are handled by virtualization and
  container queries and are demonstrated in the dev states harness.
- Primitive layer in `src/components/ui/`: `Button, IconButton, Card, Badge, Chip, Input, Textarea,
  Switch, Tabs, Tooltip, Popover, DropdownMenu, Dialog, Drawer, Select, Separator, Skeleton,
  EmptyState, ErrorState, ProgressRing, KeyHint` — styling 100% ours; overlay/menu/select/tooltip/
  switch semantics delegate to `bits-ui` 2.19.0 headless primitives (see Architecture note).
- Dev-only route `#/dev/states` enumerating every screen × six states (§70 checkable).

**Non-functional**
- Zero remote assets: no CDN, no Google Fonts (§63). The system font chain (Segoe UI Variable /
  Cascadia Mono, both present on the Windows 10/11 target) replaces the planned self-hosted subset —
  see D-16.
- Contrast: every token pair used for text ≥ 4.5:1 (3:1 for ≥18.66px bold), asserted by a unit test.
- All motion honours `prefers-reduced-motion: reduce` via a single `--motion-scale` variable.

## Architecture

Token flow: `tokens.css` (raw values per theme) → `@theme` (Tailwind utility names) → components use
utilities only; no component declares a raw hex. Decision D-10 recorded in `docs/DECISIONS.md`:
`bits-ui` is a dependency for the five primitives with hard focus-trap/ARIA semantics rather than
hand-rolling them (§80: one maintained dep replaces risky bespoke focus management); all visuals,
composition and naming are local, satisfying §3's "must not look like a generic shadcn dashboard".

## Related Code Files

**Create** — `src/styles/{tokens.css,typography.css,motion.css}`;
`src/lib/design/{tokens.ts,contrast.ts,theme.svelte.ts}`;
`src/lib/stores/{router.svelte.ts,ui.svelte.ts}`; `src/lib/tauri/window-controls.ts`;
`src/app/{AppShell.svelte,TitleBar.svelte,Sidebar.svelte,ToastRegion.svelte,routes.ts,screens.ts}`;
`src/components/ui/*.svelte` (22 files: the 21 primitives plus `Icon`) + `src/components/ui/index.ts`;
`src/components/layout/ScreenFrame.svelte`;
`src/components/state/{StateShell.svelte,EmptyState.svelte,ErrorState.svelte,LoadingBlock.svelte}`;
`src/features/{home,timeline,investigations,checkpoints,system-map,settings,diagnostics}/…Screen.svelte`
(the seven sidebar destinations as placeholder screens with real microcopy, no lorem);
`src/features/settings/collector-settings.ts` (the verbatim collector promises, shared with the
harness); `src/features/timeline/timeline-row.ts` + `src/features/checkpoints/checkpoint-row.ts`
(row shapes importable without the components);
`src/features/dev/{DevStatesHarness.svelte,dev-states-fixtures.ts,dev-large-fixtures.ts}`;
`src/lib/types/async-state.ts`; `src/lib/design/motion.ts` (the two JS-side durations);
`public/theme-bootstrap.js` (the flash-free boot script — a separate file because the CSP is
`script-src 'self'`); `src/lib/test/vitest-setup.ts`;
`scripts/{capture-window.ps1,resize-window.ps1,navigate-window.ps1}` (visual verification against
the running window); tests `src/lib/design/contrast.test.ts`,
`src/lib/design/theme-bootstrap.test.ts`, `src/components/state/state-shell.test.ts`,
`src/app/App.test.ts`, `src-tauri/tests/capability_allowlist.rs` (the SECURITY.md
forbidden-prefix claim, made executable here rather than deferred).
**Modify** — `src/styles/app.css` (add `@theme`), `src/app/App.svelte`, `index.html` (`data-theme`
default + bootstrap script), `src-tauri/capabilities/default.json` (window minimize / toggle-maximize /
close for the drawn titlebar, justified in `SECURITY.md`), `vite.config.ts` (test setup file),
`tsconfig.json` (`node` types for the fs-walking test), `package.json` (`@types/node`;
`@tanstack/svelte-virtual` is deferred to P07).
**Not created** — `src/assets/fonts/*`: no font ships (D-16 — the local Segoe/Cascadia builds carry
a no-redistribution licence and fetching an OFL face breaks the offline rule).
**Delete** — none.

## Implementation Steps

1. Author `tokens.css` with the dark palette (`background #0B0C0E`, `surface-1 #121417`,
   `surface-2 #171A1F`, `surface-3 #1E2228`, `border-subtle #22262D`, `border-active #323945`,
   `text-primary #E8EAED`, `text-secondary #A5ABB6`, `text-muted #6E7682`, `accent #5B8CFF`,
   `success #37B37E`, `warning #D9A32B`, `danger #E0575B`, `info #4BA3C7`) and the light palette;
   radii 6/10/14/20px; spacing 2–72 on a 4px base; durations `--dur-instant 90ms`, `--dur-fast
   140ms`, `--dur-base 200ms`, `--dur-slow 320ms`; easings `--ease-spring cubic-bezier(.2,.8,.2,1)`,
   `--ease-standard cubic-bezier(.4,0,.2,1)`; three low-alpha shadows plus a 1px inset top highlight.
2. Mirror tokens into `@theme` (`--color-surface-1: var(--surface-1)` …) so utilities like
   `bg-surface-1` exist; forbid raw hex in components via an ESLint-free grep test in
   `src/lib/design/contrast.test.ts` (scans `src/**/*.svelte` for `#[0-9a-fA-F]{6}`, allowlist = the
   two token files).
3. `theme.svelte.ts`: `$state` theme mode, `matchMedia` listener, `data-theme` writer, and a
   `localStorage` mirror for flash-free boot (authoritative value lives in Rust settings from P05).
4. `TitleBar.svelte`: drag region, window buttons via `@tauri-apps/api/window`
   (`getCurrentWindow().minimize()/toggleMaximize()/close()`), `aria-label`s, focus ring, and a
   status pill on the right ("● Active" / "Paused") bound to a store stubbed until P05.
5. `Sidebar.svelte` + `routes.ts`: seven typed routes, icon+label, `aria-current="page"`,
   `Ctrl+1..7` shortcuts, collapse behaviour via `@container` query at 1040px.
6. Build the 21 primitives. Wrap `bits-ui` for Dialog/Drawer/DropdownMenu/Select/Tooltip/Popover/
   Switch; hand-write the rest. Every interactive primitive gets: visible focus ring
   (`outline: 2px solid var(--border-active); outline-offset: 2px`), disabled styling, and a
   `data-testid`.
7. `StateShell.svelte` + `AsyncState<T>` union (`{status:'loading'} | {status:'empty'} |
   {status:'error', message, retry?} | {status:'ready', data}`); every screen must consume it.
8. Seven placeholder screens using the exact §72 voice where applicable ("Everything looks normal.",
   "What stopped working?", "Your history stays on this PC.") — no lorem ipsum anywhere.
9. Dev states harness route iterating `[screen × state]` in a grid at 1200px and again inside a
   900×600 iframe, so the §70 six-state rule is inspectable in one place.
10. Write `contrast.ts` (WCAG relative-luminance ratio) + tests asserting every documented
    text-on-surface pair passes, for both themes.
11. Commits: (a) `feat: design tokens and theming`, (b) `feat: ui primitive layer`,
    (c) `feat: application shell, router and screen placeholders`.
## Todo (commit boundaries)

- [x] Tokens + `@theme` + theme switching + no-flash boot
- [x] 21 primitives with focus rings and ARIA semantics
- [x] Shell: titlebar, sidebar, router, seven screens with real microcopy
- [x] `StateShell` + dev states harness
- [x] Contrast + no-raw-hex tests green

## Success Criteria

- [x] Switching Dark/Light/System repaints without reload and without a flash on cold start
- [x] `src/lib/design/contrast.test.ts` passes for both themes and fails if a token is darkened
- [x] No `.svelte` file outside `src/styles/` contains a raw hex colour (asserted by test)
- [x] Keyboard-only pass reaches every nav item, titlebar button and primitive; focus is always visible
- [x] At 900×600 nothing clips, scrolls horizontally, or overlaps the titlebar
- [x] `#/dev/states` renders 7 screens × 5 shell states (loading/empty/error/ready/large) plus the 900×600 frame
- [x] `prefers-reduced-motion: reduce` (toggled in DevTools emulation) removes all transitions

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test -- src/lib/design src/components/state
bun run tauri dev      # manual: theme cycle, Ctrl+1..7, resize to 900x600, tab-through
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `bits-ui` 2.19.0 incompatible with svelte 5.56 | `bun run check` errors in bits-ui types, or a dialog fails to trap focus | Drop the dep; hand-roll the five primitives using the native `<dialog>` element + `inert` for the rest; record in DECISIONS D-10 and add focus-trap unit tests |
| Custom chrome loses drag or snap behaviour | Window cannot be dragged or Win+Arrow snapping breaks | Keep `decorations: false` but add `data-tauri-drag-region` to the full titlebar background and a double-click maximize handler; if snapping still breaks, re-enable native decorations and record the trade-off |
| Tailwind 4 `@theme` does not emit expected utilities | `bg-surface-1` renders no style | Fall back to plain CSS variable usage (`style="background: var(--surface-1)"` via small `.surface-1` classes in `app.css`); tokens stay the single source |
| Token palette reads as generic dark dashboard | Self-review against §40's avoid-list | Iterate on depth cues (1px inset highlight, single accent, restrained glass on overlays only) before adding any new colour; never add a second accent hue |

## Security Considerations

- Fonts and icons are local files; no network request is possible under the P01 CSP.
- The `localStorage` theme mirror holds a single enum value — never settings, never user data.
- Dev states harness route is compiled out of release builds via
  `import.meta.env.DEV` guards **and** a route-table filter, so it cannot be reached in production.
