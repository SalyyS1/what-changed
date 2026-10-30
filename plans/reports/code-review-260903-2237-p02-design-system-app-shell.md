# Code Review — P02 Design System & App Shell

Plan: `plans/260829-0437-what-changed-desktop-mvp/phase-02-design-system-app-shell.md`
Reviewers: 2 independent review agents (adversarial, findings verified against source), findings applied by controller.
Verdict: **PASS** after remediation. All blockers/should-fixes resolved; every applied fix re-verified by the full gate (see test report).

## Findings and resolutions

| # | Severity | Finding | Resolution |
|---|---|---|---|
| 1 | blocker | `text-muted` rendered at 11–13.5px body size on 20 call sites but held only to 3:1; dark `#6e7682` on surface-3 = 3.48:1 | Palette re-derived: 6 tokens changed (dark text-muted/danger; light text-muted/accent/success/warning/danger/info). All pairs now pinned to `minRatio: 4.5` in `tokens.ts`. Deviation recorded as **D-17** in `docs/DECISIONS.md` |
| 2 | should-fix | Hex-guard regex missed Tailwind arbitrary values (`bg-[#123456]`) | `isColourPosition` now accepts `[`/`]` delimiters; fixture test added |
| 3 | should-fix | SECURITY.md claimed a capability-prefix test that did not exist | `src-tauri/tests/capability_allowlist.rs` created now (forbidden prefixes `fs: shell: http: sql: process:`); P05 step rewritten to extend it |
| 4 | should-fix | Tabs panel stripped its focus ring (`focus-visible:outline-none` on a tabindex=0 tabpanel) | Class removed; global focus ring applies |
| 5 | should-fix | Tailwind spacing ran on 3.375px (1rem=13.5px root) not the documented 4px base | `--spacing: 4px` pinned in `@theme`; 900×600 + full-size visual recheck done |
| 6 | should-fix | Collector promises not verbatim on screen (label/description split) | `collector-settings.ts` holds the two PRODUCT.md sentences as one `promise` string; screen renders one text node; fixtures import the same array |
| 7 | should-fix | Phase codes in code comments (9 files) | All rewritten as behaviour statements |
| 8 | should-fix | Comments claimed an "inline boot script" the CSP forbids | Reworded to name `public/theme-bootstrap.js` |
| 9 | nit | Contrast guard asserted a hard-coded constant | Replaced with a mutation check (40% darken must fail the pair table) |
| 10 | nit | Indicator offset hardcoded `2.25rem + 0.125rem`, coupled to `h-9`/`gap-0.5` | `--nav-row-height`/`--nav-row-gap` tokens consumed by rows, list gap and the indicator calc; rail-mode capture confirms alignment |
| 11 | nit | Literal loop durations / tooltip delay / toast TTL | `--dur-loop-shimmer`/`--dur-loop-spin` tokens (`--t-loop-*` in motion.css, kept out of `--motion-scale` — a 0-duration infinite loop is a strobe; the reduced-motion block freezes them); JS-side `TOOLTIP_DELAY_MS`/`TOAST_TTL_MS` in `src/lib/design/motion.ts` |
| 12 | nit | Missing `data-testid` on Tabs trigger, menu, menu-item, popover, tooltip | Added |
| 13 | nit | Theme storage key duplicated in bootstrap with no guard | `theme-bootstrap.test.ts` pins key + mode literals + `data-theme` write |
| 14 | nit | Harness had 4 states, no large-data fixture | `large` state added: 2,000 deterministic timeline rows + 400 checkpoints (`dev-large-fixtures.ts`, LCG-seeded, no lorem) |
| 15 | nit | Dialog copied the shadcn translate(-50%,-50%) centring recipe | Grid wrapper centres; panel transform freed for `wc-rise-in`; `pointer-events` split keeps outside-click dismissal |
| 16 | nit | Diagnostics printed raw enum values | Label map renders `Running/Idle/Failed/Not available` |
| 17 | nit | Home input accessible name ≠ visible prompt | Verified fixed pre-review-round-2 (label matches heading) |
| 18 | nit | `plan.md` feature list + P05 capability statement stale | Both corrected in plan files |
| 19 | nit | Dead eslint-disable; ProgressRing inline `var()`; typography.css narrative comment | All removed/replaced |

## Rejected / deferred

- **Sidebar `@container` query instead of ResizeObserver** — kept the observer: the measured-parent approach is what lets the dev harness frames drive rail mode without a viewport resize, and jsdom degrades gracefully. Verified working in harness + real window.
- **Exact-set capability assertion** — P05 owns it (the set grows there); P02's forbidden-prefix test already backs the SECURITY.md claim.

## Contract check

- Acceptance criteria: all met (see test report for evidence per criterion).
- Public contracts: none broken; `TimelineRow`/`CheckpointRow` moved to importable modules (`timeline-row.ts`, `checkpoint-row.ts`) with identical shapes; `THEME_MODES` newly exported.
- Patterns: matches P01 conventions (kebab-case modules, token-only colours, LF endings).

## Unresolved questions

None.
