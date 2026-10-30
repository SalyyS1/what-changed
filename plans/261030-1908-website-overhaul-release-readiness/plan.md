---
title: Website overhaul and desktop release readiness
status: in-progress
created: 2026-10-30
branch: codex/ownvero-owner-workspace
---

# Outcome

Replace the text-heavy landing with an obvious interactive product explanation;
bring the existing Windows candidate closer to a verifiable release. Preserve
data, all documentation routes, synthetic-only website data, and exact-release
approval. No new cloud service, invented customer claims, or release bypass.

## Design brief

Register: Brand/product.
Scene: Nontechnical AI builders on laptop/mobile, deciding whether this tool helps
them understand their next app change; clear, practical, confident.
Direction: Product studio with a physical evidence workbench as the opening visual.
Color: Restrained; neutral white background oklch(0.985 0.003 150), ink
oklch(0.23 0.015 150), green accent oklch(0.43 0.07 150).
Type: Segoe UI Variable/Segoe UI, strong display against quiet body text.
Signature: A volumetric evidence model above the fold, connected to an operable
Intent → Preserve → Brief → Verify → Decide walkthrough.
Dials: variance 6, motion 2, density 3.

## Execution

- [x] Read README, existing site, accepted plan and current release contracts.
- [x] Rebuild landing, shared navigation, wiki and download styling.
- [x] Deliver real lit 3D geometry with static 2D/no-JS fallback, keyboard steps,
  reduced-motion behavior, context-loss recovery and cleanup.
- [x] Inspect native release evidence and repair actionable release workflow gaps.
- [x] Verify source/site, rendered desktop/tablet/mobile and independent review.
- [ ] Commit, publish site preview, check hosted pages and record release status.

## Acceptance

At 1440×900 the actual model is visible in the opening view; on 375px the page
recomposes without overflow. A visitor can follow one concrete booking change
without reading an essay. No-JS and denied-WebGL visitors retain the full explanation.
All old site routes and full demo lazy loading remain valid. Downloads require the
approved exact artifact. Release status cites current commands/artifacts; missing
clean-Windows, human pilot, soak or exact approval is never reported as complete.

## Evidence

- Site build/check: 19 routes; exact-release download remains unavailable.
- Site tests: 12/12 pass, including denied WebGL, no JavaScript, delayed imports,
  persisted-page restoration, actual graphics context loss and retained controls.
- Render inspection: English/Vietnamese landing, download and getting-started
  at 375, 768 and 1440 px; zero checker errors/warnings after repairs.
- [Independent review](../reports/review-261030-1908-website-overhaul.md): all three
  concrete lifecycle/localization findings repaired and re-reviewed.
- [Desktop source checks](../reports/tester-261030-1908-desktop-source.md) and
  [release readiness](../reports/release-261030-1908-readiness.md) distinguish
  passing source gates from outstanding native acceptance.
- Runtime delta explicitly approved by user; exact raw proposal and historical
  bytes preserved. New source preflight tests 7/7 pass before expensive CI work.

## Ownership

Controller owns site templates/styles/tour/browser tests, integration and publishing.
Workbench worker owns only `wiki-site/src/workbench-3d.mjs`.
Release auditor owns inspection and `plans/reports/release-261030-1908-readiness.md`.

## Rollback

Revert the scoped website commit and redeploy the preview branch. Do not alter app
profiles or release approvals. Any further native changes must use a new candidate.
