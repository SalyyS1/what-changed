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
- [x] Commit, publish site preview, check hosted pages and record release status.
- [x] Build and inspect the updated runner-enabled Windows QA artifact.

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
- Website is live and smoke-checked on Pages at `d3efe70`; public QA source
  `75dcb67` carries primary `d71eaf3` after the exact desktop notice-byte repair.
- Combined early source/notice tests 13/13 pass. Actual frontend build and desktop
  notice generation pass locally. Replacement hosted run is `41091508412`.
- [Hosted QA and nonexecuting inspection](../reports/qa-261030-2235-runner-installer-artifact.md)
  pass for source `75dcb67c`: installer, both notices and all 197 packaged runtime
  files match the recorded checksums and approved inventory. Seven source
  archives remain co-delivered in complete artifact `12995563059`.
- [Native candidate smoke](../reports/qa-261031-native-candidate-smoke.md) passed
  launch, workspace creation, relaunch persistence, clean exit and exact profile
  restoration on the current account. It does not close installer, clean-Windows,
  network, accessibility, soak, pilot or G2/G3 gates.

## Remaining release acceptance

The QA build is not public release acceptance. Clean Windows, native install/
upgrade/privacy/egress/accessibility, four-hour soak and consenting owner repeat
use remain open. The complete runner packet needed a public distribution format
decision: the current single-EXE manifest could not represent its full source
packet, which exceeds GitHub's per-asset size limit. The two-asset decision is
now recorded below; it does not create a Release or enable a CTA. See the release
readiness and independent review reports above. Do not drop source delivery.
The [public download proposal](public-download-proposal.md) records the concrete
same-Release installer/source format that was later approved by the user. That
format decision does not approve public publication or bypass the remaining
acceptance gates.

The user subsequently approved the proposed two-asset format. The schema,
validator, candidate assessment and download view now bind an installer and a
corresponding source archive to the same version/tag. `wiki-site/release-manifest.json`
remains the explicit unavailable state until exact G2/G3 approval and a frozen
Release exist.

## Ownership

Controller owns site templates/styles/tour/browser tests, integration and publishing.
Workbench worker owns only `wiki-site/src/workbench-3d.mjs`.
Release auditor owns inspection and `plans/reports/release-261030-1908-readiness.md`.

## Rollback

Revert the scoped website commit and redeploy the preview branch. Do not alter app
profiles or release approvals. Any further native changes must use a new candidate.
