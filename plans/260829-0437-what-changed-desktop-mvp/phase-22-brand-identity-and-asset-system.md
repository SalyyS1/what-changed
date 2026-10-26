---
phase: 22
title: "Brand identity and asset system"
status: in-progress
priority: P2
effort: "8h"
dependencies: [07, 16, 17, 19, 21]
---

# Phase 22: Brand identity and asset system

## Overview

Turn the existing before/after glyph and proven UI tokens into a complete, tested Windows identity. Preserve the product's calm evidence-led tone; no generic radar, shield, AI sparkle, gradient, or telemetry-dashboard branding.

## Requirements

- Functional: one master logo system covers app, tray, installer, repository and launch media.
- Functional: preserve the offset **before / change / after** concept in `assets/brand/icon-source.svg`, refining geometry only after small-size tests.
- Functional: provide primary mark, monochrome mark, wordmark, horizontal/stacked lockups, light/dark/high-contrast tray variants, installer header/sidebar images, repository social preview and article cover template.
- Non-functional: every raster is reproducible from versioned vector sources; no manual one-off export is canonical.
- Non-functional: no font redistribution. Wordmarks use the accepted system-font direction unless an explicitly licensed asset is added later.
- Non-functional: all claims/screenshots remain truthful; demo content is visibly synthetic.

## Architecture

Canonical brand sources live under `assets/brand/`. A deterministic export script generates Tauri icons and public variants. Product UI consumes the same accent/neutral tokens already contrast-tested; brand work cannot silently replace D-17 palette values.

Recommended direction: **Delta Bridge** — two offset state bars joined by one narrow change marker. It already maps to “before / after / what changed,” survives monochrome and avoids implying repair, security scanning or AI.

Rejected directions: radar/pulse (monitoring-dashboard promise), shield (security-product promise), magic sparkle (AI/automatic-fix promise).

## Related Code Files

- Modify: `assets/brand/icon-source.svg` — refined master geometry and safe area.
- Create: `assets/brand/logo-mark.svg`, `logo-wordmark.svg`, `logo-lockup-horizontal.svg`, `logo-lockup-stacked.svg`, `tray-light.svg`, `tray-dark.svg`, `tray-high-contrast.svg`, `social-preview-template.svg`, `brand-guidelines.md`.
- Create: `scripts/export-brand-assets.ps1` — deterministic validation/export.
- Modify: `src-tauri/icons/*`, `src-tauri/tauri.conf.json`, `src/app/TitleBar.svelte` only after assets pass review.
- Create: `src-tauri/tests/brand_assets.rs` or equivalent script test for required dimensions/alpha/files.

## Implementation Steps

1. Inventory current icon use and capture actual 16/20/24/32/48/256 px renders.
2. Draw three geometry variations of Delta Bridge; compare silhouette, one-colour and blur/downsample tests; choose one with recorded rationale.
3. Define clear space, minimum size, background rules, monochrome and high-contrast behavior in the brand guide.
4. Build wordmark/lockups without bundling a redistributable font file.
5. Write deterministic exporter; regenerate ICO/PNG/Store tiles and installer/public variants.
6. Inspect in real Windows title bar, taskbar, tray overflow, Start menu, installer, dark/light/high-contrast themes and 150% DPI.
7. Add asset manifest test: dimensions, alpha, source provenance, required variants; rerun app build.
8. Capture approval sheet under `assets/designs/` with source-labelled variants; do not publish.

## Success Criteria

- [x] Mark remains distinguishable and centered at 16, 20, 24 and 32 px.
- [ ] Tray mark is legible on light, dark and Windows high-contrast surfaces.
- [x] ICO/PNG/Store assets regenerate from one command with no untracked manual edits.
- [ ] Installer and application show the same approved mark and product name.
- [x] Brand guide defines meaning, spacing, sizes, colours, misuse and copy voice.
- [ ] A reviewer cannot reasonably read the mark as antivirus, cloud sync or automatic repair.
- [x] Existing contrast tests and production build stay green.

## Risk Assessment

| Risk | Signal | Pre-decided response |
|---|---|---|
| Mark becomes mush at tray size | bridge or state bars merge at 16–20 px | use a separately hinted small-size master with the same silhouette |
| Brand implies unsupported behavior | reviewers describe it as radar/shield/AI | reject that direction; keep Delta Bridge semantics |
| Raster variants drift | generated assets differ across runs | exporter + manifest test; vectors remain canonical |
| Palette work regresses AA | D-17 contrast suite fails | preserve UI tokens; brand-only accents never become text tokens |

## Unresolved Questions

None. Exact geometry is a design-review output, not a product-scope decision.

## Automated evidence (2026-09-23)

- `bun run check:brand` passes all ten canonical vector/guideline sources and
  verifies the approved flat colour tokens remain in the master mark.
- The release pipeline now invokes this check alongside docs and launch-copy
  validation.
- Small-size legibility, Windows theme/tray inspection, installer parity and
  installed-RC parity remain manual/release-candidate criteria.

## Export evidence (2026-09-24)

- `bun run export:brand` generates 20 PNG sizes and a multi-resolution ICO from
  `assets/brand/icon-source.svg` with ImageMagick. `-UpdateTauriIcons` refreshes the tracked
  Tauri PNG/Store/ICO variants. The script now writes SHA-256 hashes and the rasterizer version.
- Two independent output directories produced the same 21 raster/ICO hashes (0 differences).
  `magick identify` confirms 16/20/24/32/48/64/128/256 ICO entries and the 1024px master.
- The generated 16/20/24/32px renders were visually inspected as distinguishable; this does
  not replace the Windows taskbar/tray/installer/DPI/theme inspection still required above.
- `docs/brand-guide.md` was reconciled to the offset-bar master geometry and flat token palette.

## Export and inspection evidence (2026-09-25)

- After expanding the exporter, two independent runs matched all 37 PNG/ICO/BMP raster hashes (0 differences); `bun run check:brand` passes 14 canonical sources and 37 tracked hashes, plus PNG dimension/alpha and ICO/BMP dimension checks. The 1024px master is a reproducible build output, not a required ignored file in a clean clone.
- The 16, 20, 24 and 32 px outputs were inspected at their native sizes. The source geometry retains the centered tile and the offset two-bar silhouette at each size; this closes the raster-size design check, not an installed Windows taskbar or tray check.
- A 16 px light/dark/high-contrast preview sheet is legible on simulated surfaces. The exporter now generates all three 16/20/24/32 px tray variants, NSIS BMPs and draft public rasters. Runtime tray selection and Windows high-contrast inspection remain open until the integration tests and installed-RC review run.
- A source-labelled design sheet under `assets/designs/` previews the colour/monochrome mark, tray, installer and article cover without implying final human approval. The title bar now references the Delta Bridge mark; NSIS artwork is configured for the next final build.
- The 2026-09-26 release rehearsal passed the full frontend test suite (319/319), production build,
  bundle budget and generated NSIS artwork gate after the current brand assets were incorporated.
