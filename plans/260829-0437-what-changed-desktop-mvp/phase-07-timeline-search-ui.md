---
phase: 07
title: "Timeline & Search UI"
status: completed
priority: P1
effort: 8h
dependencies: [06]
---

## Overview

§69 Phase 1 frontend half: the polished Home and Timeline screens plus the event details drawer and
global search — built on synthetic data so the UI is finished and fast before any collector exists.
This is where the product's voice becomes visible (§43, §44, §46, §72).

It is also where §2's dual audience becomes a mechanism rather than an aspiration. The default reading of
every row is one plain sentence a non-technical user can act on; the drawer's collapsed
"Show technical details" section is where an advanced user finds raw paths, PID, service name, registry
path, port, executable, before/after values, related events, timestamps and confidence. One app, one
event, two depths — progressive disclosure is §2's answer, and this phase owns it. §71's window
responsiveness lands here too: desktop density at 1200–1600 px, usable at 900×600 with filters
collapsing into a dropdown and the drawer going full width, never a stretched mobile layout.

## Requirements

**Functional**
- Home (§43/§72): status pill ("● Active"/"Paused"; `Recovery needed` when the database is unusable,
  and `Degraded` once collector health exists), headline "Everything looks normal." or "N recent
  changes may need attention.", prompt "What stopped working?", input placeholder "Example: Discord
  stopped detecting my microphone this morning", primary button "Investigate", today's counts
  ("37 changes · 2 important · 0 critical"), "Important changes" list, privacy line "Your history
  stays on this PC.", plus quick chips: Today / Since last restart / Yesterday. A corrupt or
  schema-mismatched database routes to the recovery surface before ordinary history screens and can
  never display the global status as Active.
- Timeline (§44): range tabs Today / Yesterday / Last 7 days / Custom; filter chips All / Apps /
  Windows / Network / Drivers / Files / Development / Docker / Important only; grouped by local hour
  with sticky headers; each row shows time, human title, `before → after`, an optional relative note
  ("34 minutes before first game crash" when a correlation exists), and "View details".
- Clusters render as one row with a count and "[Expand]" that loads members inline.
- Virtualized list: `@tanstack/svelte-virtual` 3.13.36 with fixed 72 px rows; **fallback** to a
  hand-rolled windowed list (`src/components/timeline/WindowedList.svelte`) if the runes spike fails
  (research §5.10). Never hold more than 3 pages (600 rows) in memory; older pages are dropped and
  refetched.
- Live updates: on `system-event-added`, if the user is at scroll-top on a live range, prepend with a
  140 ms enter animation; otherwise show a "N new changes" pill that scrolls to top on click. Never
  re-render the whole list (§57).
- Details drawer (§46): human explanation, timestamp (absolute + relative), importance chip,
  confidence, before/after table, related events (clickable), affected entities, "Detected by
  <collector>", and a collapsed "Show technical details" section with raw metadata and identifiers
  (§51 progressive disclosure). Keyboard: `Esc` closes, focus returns to the invoking row.
- Global search (§52): `Ctrl+K` palette, debounced 120 ms, grouped results (Applications, Files,
  Ports, Services, Drivers, Docker, Checkpoints, Events), arrow-key navigation, `Enter` opens the
  details drawer or the timeline filtered to that entity.
- Six states on Home, Timeline, and Search (§70): skeleton loading; empty ("Nothing has changed yet.
  What Changed started recording at 1:20 AM." / "No changes match these filters."); error with Retry;
  success; large-data (virtualized + "Showing 200 of 128,431"); small-window at 900×600 (filters
  collapse into a dropdown, drawer becomes full-width).

**Non-functional**
- 60 FPS scroll at 100k rows (measured in P20); no layout thrash (fixed row height, `content-visibility`
  on group headers).
- No business logic in components: formatting lives in `src/lib/formatting/`, data access in stores.
- All interactive rows are real `<button>`/`<a>` elements with `aria-expanded`/`aria-controls`.

## Architecture

Stores: `timeline.svelte.ts` (query state, pages, cursor, live pill), `eventDetails.svelte.ts`,
`search.svelte.ts` — each a rune-based class exposing `AsyncState`. Components:
`components/timeline/{TimelineList,TimelineRow,ClusterRow,GroupHeader,RangeTabs,FilterChips,
LivePill,WindowedList}.svelte`, `components/event/{DetailsDrawer,BeforeAfterTable,RelatedEvents,
TechnicalDetails}.svelte`, `components/search/{SearchPalette,ResultGroup}.svelte`.
Formatting: `formatting/{time.ts,bytes.ts,eventCopy.ts,relative.ts}` — `eventCopy.ts` is the only
place UI strings for event titles are assembled, from the Rust-provided
`user_friendly_summary`/`technical_summary` (never re-derived in TS).
## Related Code Files

**Create** — `src/features/home/{HomeScreen.svelte,StatusHeadline.svelte,TodayCounts.svelte,
ImportantChanges.svelte,ProblemInput.svelte}`;
`src/features/timeline/{TimelineScreen.svelte,TimelineToolbar.svelte,CustomRangePicker.svelte}`;
`src/components/timeline/{TimelineList,TimelineRow,ClusterRow,GroupHeader,RangeTabs,FilterChips,
LivePill,WindowedList}.svelte`; `src/components/event/{DetailsDrawer,BeforeAfterTable,
RelatedEvents,TechnicalDetails,ImportanceChip,ConfidenceBadge}.svelte`;
`src/components/search/{SearchPalette,ResultGroup,ResultRow}.svelte`;
`src/lib/stores/{timeline.svelte.ts,eventDetails.svelte.ts,search.svelte.ts,liveEvents.svelte.ts}`;
`src/lib/formatting/{time.ts,bytes.ts,eventCopy.ts,relative.ts}`; tests
`src/lib/formatting/time.test.ts`, `src/lib/stores/timeline.test.ts`,
`src/components/timeline/timeline-list.test.ts`, `src/components/event/details-drawer.test.ts`,
`src/components/search/search-palette.test.ts`.
**Modify** — `src/features/home/HomeScreen.svelte` and
`src/features/timeline/TimelineScreen.svelte` (replace P02 placeholders), `src/app/AppShell.svelte`
(mount palette + drawer portals), `src/features/dev/DevStatesHarness.svelte` (wire real screens),
`package.json` (`@tanstack/svelte-virtual`), `PRODUCT.md` (screen copy inventory).
**Delete** — none.

## Implementation Steps

1. Spike `@tanstack/svelte-virtual` with 100k synthetic rows in a scratch route; if runes
   incompatibility appears (research §5.10), implement `WindowedList.svelte` (fixed row height,
   `scrollTop`-derived window + 8-row overscan) and record D-08. Do the spike **first** — the rest of
   the timeline depends on the choice.
2. `formatting/time.ts`: absolute (`10:42 AM`), relative (`34 minutes before …`), day headers
   ("Today", "Yesterday", "Friday, Aug 22"), and the §44 duration phrasing; unit-tested with fixed
   timestamps.
3. `timeline.svelte.ts`: query state → `getEvents` → page list; `loadMore` on scroll threshold;
   `applyFilter`/`setRange` reset the cursor; page eviction beyond 3 pages; `liveEvents` integration.
4. Build `TimelineRow` to the §44 shape exactly, `ClusterRow` with "[Expand]", `GroupHeader` sticky
   per hour.
5. `DetailsDrawer`: right-side sheet (480 px, full-width below 1040 px), spring transition
   (`--ease-spring`, `--dur-base`), focus trap via the P02 primitive, `Esc` close, technical section
   collapsed by default with a persisted per-user preference (§51).
6. Home screen assembled from `get_dashboard_summary`; the problem input stores its draft and routes
   to Investigations (P15) — until then the button is disabled with the tooltip "Investigation arrives
   in the next phase" **and** the stub is listed in `ROADMAP.md` (§87).
7. `SearchPalette`: `Ctrl+K` global handler, `role="dialog"` + `aria-modal`, `role="listbox"` results,
   `aria-activedescendant` navigation, empty state "No matches. Try an app name, a port, or a file
   name."
8. Six-state pass: add each screen's four shell states to the dev harness, verify the 900×600 frame,
   and add a large-data state fed by the 100k profile.
9. Vitest + Testing Library tests: filter/range transitions, cluster expand, live-pill behaviour when
   scrolled, drawer focus return, palette keyboard nav, and empty/error rendering.
10. Commits: (a) `feat: virtualization decision and windowed list`, (b) `feat: timeline screen`,
    (c) `feat: event details drawer`, (d) `feat: home screen`, (e) `feat: global search palette`,
    (f) `test: timeline, drawer and palette suites`.

## Todo (commit boundaries)

- [x] Virtualization spike resolved (TanStack Svelte Virtual retained; fallback/D-08 not needed)
- [x] Timeline list, rows, clusters, group headers, toolbar
- [x] Details drawer with progressive disclosure
- [x] Home screen with exact §72 copy
- [x] Search palette
- [x] Six states wired into the dev harness + component tests

## Success Criteria

- [x] A >100k total-count fixture stays virtualized with ≤ 600 event payloads in memory (asserted by store/component tests; measured FPS remains P20)
- [x] Filters and ranges combine correctly and the URL hash reflects state (deep-linkable)
- [x] Cluster expand shows exactly the clustered members and collapses back
- [x] While scrolled down, new events never move the viewport; the pill appears instead
- [x] Drawer shows every §46 field for a synthetic event with `before`/`after`
- [x] `Ctrl+K` → typing `25565` finds the port entity; `Enter` filters the timeline to it
- [x] All six states verifiable at `#/dev/states`, including 900×600
- [x] `bun run test` green; `bun run check` green

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run check
bun run test -- src/lib/formatting src/lib/stores src/components
bun run tauri dev   # manual: simulate_event x20 while scrolled down -> pill only; Ctrl+K; 900x600
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| `@tanstack/svelte-virtual` incompatible with runes | Spike errors or rows fail to recycle | Ship `WindowedList.svelte` (fixed-height rows make this ~120 lines); record D-08; drop the dependency entirely |
| Variable-height rows break virtualization | Rows with long paths wrap and misalign | Row height is fixed at 72 px; long values truncate with a tooltip and full value in the drawer — never wrap in the list |
| Live prepend causes scroll jump | Viewport moves on insert | Prepend only at `scrollTop < 4px`; otherwise pill. Store test covers both branches |
| Drawer + palette both open trap focus incorrectly | Tab escapes to the background | Single portal stack with an ordered focus-trap registry; palette closes the drawer's trap rather than nesting |
| Copy drifts from §72/§44 wording | Manual diff against the packet strings | The exact strings live in `src/lib/formatting/eventCopy.ts` + `src/features/**/copy.ts` and are asserted by a snapshot test, so a wording change is a visible diff |

## Security Considerations

- The drawer renders all values as text nodes; no `{@html}` anywhere in the codebase (asserted by a
  grep test in `src/components/**` — protects against a malicious config/Docker label reaching the DOM).
- Raw identifiers shown in technical details are already scrubbed by P03; the UI never re-fetches raw
  file contents (there is no command that could).
- Deep-link hash state is validated against the typed query schema before use.
