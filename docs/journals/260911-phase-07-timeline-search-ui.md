---
date: 2026-09-11
session: phase-07-timeline-search-ui
phase: 07
status: completed
---

# Journal: 2026-09-11 — Phase 07 Timeline & Search UI

## Context

Phase 07 turned the Phase 06 event contracts into the first complete user-facing workflow: Home,
Timeline, event details, live updates, and global search. The UI needed to stay calm and responsive at
desktop density, remain usable at 900×600, and prove that large histories do not become large DOM or
memory payloads.

## What Happened

- A Svelte 5 spike validated `@tanstack/svelte-virtual` 3.13.36, so the production timeline retained
  TanStack virtualization with fixed 72px rows.
- Implemented a bounded three-page window: at most 600 event payloads remain in memory while older
  pages can be refetched in either direction.
- Review found that refreshing page zero could leave an invalid continuation chain. Refresh now
  replaces the cursor chain atomically instead of combining fresh and stale page state.
- Review also found that a semantic cluster key was insufficient for identity across page fragments.
  UI identity now includes the backend's exact first/last `(timestamp, id)` bounds, matching expansion.
- Home and routing now fail closed when recovery is required; the normal event UI does not render
  over an unsafe or unavailable data state.
- Added range/filter controls, local-hour grouping, live-update pill behavior, progressive details,
  Ctrl+K search, keyboard navigation, and six-state development fixtures. Filters collapse below
  1040px, and the 900×600 render passed visual inspection without horizontal overflow.
- Verification completed with 156 frontend tests, 14 focused cursor/responsive tests, 195 Rust
  contract/repository tests, a 767-module production build, clean generated bindings, and a final
  9.6/10 review with no critical findings.

## Reflection

The most valuable review discoveries were state-identity problems rather than visual defects. A
virtual list can look correct while its cursor chain is internally inconsistent, and clusters can
look identical while referring to different bounded event sets. Making snapshot replacement and
cluster identity explicit improved correctness without weakening the 600-row memory ceiling. The
900px pass also confirmed that responsive decisions should follow the real minimum window rather
than an abstract mobile breakpoint.

## Decisions Made

| Decision | Rationale | Impact |
|---|---|---|
| Retain TanStack Svelte Virtual | The Svelte 5 runes spike passed | Avoids a custom virtualizer while keeping fixed-row performance |
| Bound retained history to three pages or 600 events | Large totals must not imply unbounded client memory | The timeline remains predictable at six-figure result counts |
| Replace page-zero state and its cursor chain atomically | Fresh cursors cannot safely continue stale pages | Refresh and filter changes cannot create gaps or duplicate traversal |
| Identify clusters with full backend bounds | Semantic keys can repeat across page fragments | Expansion targets the exact event set, including timestamp ties |
| Fail closed into recovery | Unsafe storage state must not masquerade as an empty timeline | Recovery status and routing remain honest and deterministic |
| Collapse filters below 1040px | The supported minimum is a 900px desktop window | Controls remain usable without horizontal overflow |

## Next Steps

- Begin Phase 08 Collector Runtime against the stable Timeline and live-update surfaces.
- Perform the native Tauri/WebView smoke for real backend search, live events, and focus behavior when
  that environment is available.
- Measure 100k-row scroll FPS in Phase 20; Phase 07 establishes virtualization and the 600-row memory
  bound but deliberately does not claim the later performance gate.
