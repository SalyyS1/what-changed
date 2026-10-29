# Owner entry and runner clarity

Date: 2026-10-29
Branch: `codex/ownvero-owner-workspace`

## Change

- Home now exposes an explicit **Open My apps** entry for people building with AI.
- Home loads the selected recovery workspace and keeps the existing symptom
  investigation scoped to that project when one is selected.
- Preview setup now labels the browser as an **Opt-in runner** instead of
  claiming it is always unavailable. Source/core builds can save configuration
  and manual observations; automated checks require a packaged candidate with
  the reviewed runtime bundle.
- Product and getting-started copy describe the owner workspace alongside the
  legacy troubleshooting workflow.

## Verification

- `bun run check` — passed; 0 Svelte errors and 0 warnings.
- Focused owner/home tests — passed; 3 files, 11 tests.
- Broader owner/change/brief/evidence/runner tests — passed; 5 files, 11 tests.
- Full frontend suite — passed; 90 files, 452 tests.
- `bun run build` — passed.
- `bun run check:bundle` — passed; 213,445 gzip bytes counted for non-Three
  chunks. Three.js remains lazy-loaded (131,093 gzip bytes in its separate
  chunk).

## Boundaries

The packaged Chromium/Node runner remains an optional QA candidate resource;
its roughly 550 MB runtime is not part of the core frontend bundle. Clean
Windows, native install/upgrade, network confinement, soak, participant
acceptance, G2 branding and G3 publication remain open evidence gates.
