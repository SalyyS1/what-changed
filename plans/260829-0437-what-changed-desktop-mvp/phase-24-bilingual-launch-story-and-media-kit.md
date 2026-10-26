---
phase: 24
title: "Bilingual launch story and media kit"
status: in-progress
priority: P2
effort: "10h"
dependencies: [22, 23]
---

# Phase 24: Bilingual launch story and media kit

## Overview

Create a truthful launch package after the product and English wiki are stable: canonical English product copy plus a Vietnamese introduction adapted for Vietnamese developer communities. No publication occurs inside this phase.

## Requirements

- Functional: one messaging hierarchy — problem, promise, proof, limits, call to action — reused without copy-paste drift.
- Functional: produce English README hero/short description, long English launch article, Vietnamese adapted article, release-note template, repository/social snippets and demo narration.
- Functional: media kit includes approved logo files, screenshots, caption/alt text, 60–90 second demo script and a reproducible shot list.
- Functional: worked demo shows config key change → process/listener change → evidence-ranked investigation, plus privacy and checkpoint proof.
- Non-functional: never say “root cause,” “caused,” “fixes your PC,” “AI,” “real-time everything,” “zero memory,” or “fully secure.”
- Non-functional: distinguish What Changed from Process Monitor without claiming replacement: persistent plain-language cross-source history and deterministic evidence ranking vs advanced raw operation tracing.
- Non-functional: no analytics SDK, pixels, tracking links or in-app telemetry. Launch evaluation uses repository/download/support signals available outside the app.

## Architecture

Canonical product facts come from finalized README/root docs and P21/P23 audits. Reusable messages live in `docs/launch/messaging.md`; channel files reference that source. Media sources live under `assets/launch/`; generated outputs never become technical authority.

## Related Code Files

- Create: `docs/launch/messaging.md`, `docs/launch/launch-article-en.md`, `docs/launch/launch-article-vi.md`, `docs/launch/release-notes-template.md`, `docs/launch/demo-script.md`, `docs/launch/community-posts.md`, `docs/launch/press-kit.md`.
- Create: `assets/launch/screenshots/`, `assets/launch/demo/`, `assets/launch/social/` assets from release candidate.
- Modify: `README.md` hero/quick start/status only after product claims gate.
- Create: `scripts/capture-launch-assets.ps1` or extend existing capture scripts for deterministic windows/routes/fixtures.

## Implementation Steps

1. Freeze the fact sheet from P21/P23: shipped capabilities, measured numbers, privacy boundaries, known limitations, install caveats.
2. Write messaging hierarchy and anti-claims list; review against `PRODUCT.md` voice and forbidden causal wording.
3. Draft English long article: pain story, worked example, how evidence ranking works, privacy/non-admin design, screenshots, limitations, install/try/contribute.
4. Adapt—not mechanically translate—the Vietnamese article; keep technical nouns consistent and link to English canonical docs.
5. Build shot list: Home calm/attention, Timeline filter+cluster, details progressive disclosure, investigation evidence, checkpoint diff, maps with 2D fallback, privacy, diagnostics/degraded state, tray.
6. Capture all media from a tagged release candidate with deterministic synthetic fixtures and no personal data. Record viewport/DPI/theme/version.
7. Record demo with captions/transcript; verify no secret/path/notification leakage and no performance cuts that misrepresent response time.
8. Produce short channel variants from messaging, not new claims. Peer-review English and Vietnamese for accuracy/tone.

## Success Criteria

- [ ] Every public claim maps to a passing test, release-audit row or explicitly labelled limitation.
- [x] English and Vietnamese articles tell the same product truth; technical docs remain English canonical.
- [ ] 60–90 second demo completes the worked example without hidden setup or fabricated causal claim.
- [ ] Screenshot set covers core loop, trust surfaces and one honest degraded/empty state.
- [ ] All media contains only deterministic synthetic data and approved brand assets.
- [ ] README status no longer says under construction only after P25 readiness gate passes.
- [x] No telemetry/tracking mechanism is added to measure launch.

## Risk Assessment

| Risk | Signal | Pre-decided response |
|---|---|---|
| Marketing outruns implementation | copy names a control absent from RC | cut or relabel the claim; never stage a fake screen |
| Vietnamese copy changes technical meaning | reviewer reads stronger causality/privacy claim | keep glossary and sentence-level fact map to English source |
| Demo leaks data | username/path/real service name visible | discard take, reset deterministic fixture and recapture |
| Comparison sounds adversarial | “replacement/better than Procmon” language | state different jobs; cite official capability descriptions neutrally |

## Unresolved Questions

None. Exact launch channels are a P25 publication choice and do not change the assets required here.

## Automated evidence (2026-09-23)

- `bun run check:launch` passes for all six public-copy files. Each file has an
  evidence-state label and the scanner rejects the forbidden causal, security,
  telemetry-adjacent and Process Monitor replacement claims.
- `docs/launch/messaging.md` remains the single fact sheet; English and
  Vietnamese articles are separate adapted outputs that link to English
  technical truth.
- This evidence does not close screenshot, recording, installed-RC or human
  publication-approval criteria.


## Draft review (2026-09-25)

- The English and adapted Vietnamese drafts were compared against each other and the English wiki index: both describe local evidence ranking without causal certainty, pre-persistence redaction, the non-admin/offline runtime boundary, unsigned installer, short-lived-process and Docker-live limits. The Vietnamese draft links to the English canonical technical docs.
- `bun run check:launch` passes 6/6 public-copy files. A launch-source scan found no analytics SDK, pixel, UTM/tracking link or in-app telemetry added for launch measurement. This is a draft/source claim, not approval to publish.
