# Ownvero local site and draft identity

Status: DONE_WITH_CONCERNS. Authorized local implementation complete; public identity,
release publication, native brand acceptance, and external acceptance gates stay open.
No push, deployment, binary upload, public rename, or external setting change made.

## Outcome

Implemented a real static landing/how-it-works/demo/docs/search/download/changelog/
support site, seven canonical owner guides, Vietnamese landing/quickstart/glossary,
and draft open-frame mark/wordmark variants. The native/master brand assets remain
unchanged. Build output is local, noindex, and explicitly publication-unapproved.

The accepted design brief: Brand register; an app owner reviewing the next AI-made
change on a laptop; quiet editorial direction because the job is deliberate review.
Restrained warm paper, charcoal, muted green; Georgia display and system sans body;
signature is a concrete booking request and evidence workbench. Variance 5, motion 1,
density 4. No invented traction, customers, claims of complete safety, or stock AI art.

## Remote reconciliation

Local wiki-site and workflow owners were absent. No remotes/remote-tracking refs
were configured; git could not read the historical remote object locally. Read-only
GitHub API inspection confirmed main at 9f8c0df94118287d989b1b253f619050aa212543.
The remote tree held wiki-site/index.html, app.js, styles.css and the existing
.github/workflows/deploy-wiki-site.yml. Inspected entry, browser page map, and workflow.
No fetch, checkout, reset, dirty-tree overwrite, or Git mutation used.

The prior site was a client-rendered source preview with seven root hash routes:
overview, first-investigation, timeline, workspace, privacy, build, status. All
remain in the alias manifest, browser redirect, and no-JavaScript landing anchors.
The /what-changed/ base is preserved. Legacy technical links pin that exact public
source revision and disclose their age. New owner docs are statically rendered,
development-versioned, dated, and content-hashed.

The local workflow at the original workflow path is manual preview only. It has
read-only repository permissions, pinned action SHAs, exact tool versions, a main/
repository condition, and uploads only generated output as a review artifact.
It has no Pages deployment or Pages write permission. Workflow source was not run
remotely. A future publish route still needs explicit approval.

## Owners and integration

- Site builder/config/modules/styles/content/tests: wiki-site/.
- Strict download schema: release/release-manifest.schema.json.
- Output gate: scripts/check-site.mjs. It independently checks allowable route/asset
  shapes, links/anchors/base paths, private paths/secret signatures, and desktop IPC.
- Download gate: scripts/check-download-manifest.mjs. validateRelease(value) performs
  structural validation; readApprovedRelease(root) also requires G2/G3 and G3 scope
  binding the exact wiki-site/release-manifest.json. Invalid metadata renders an error
  with no installer link; the separate gate fails. The current unavailable manifest
  intentionally passes its truthful disabled state.
- Build provenance: wiki-site/dist/build-manifest.json records git sourceCommit,
  docsVersion, per-input SHA-256 values, aggregate sourceHash, and explicit output
  allowlist. This is working-tree preview provenance, not release attestation.
- Draft brand assets: assets/brand/ownvero/. No native export invoked.
- User docs: docs/ownvero/{getting-started,owner-workspace,verification,maps,privacy,
  faq,migration-guide}.md. Each carries a development evidence-state label.
- Launch fact sheet and all six legacy launch consumers: docs/launch/. Each legacy
  file has an explicit archive/current-messaging disposition; none is silently
  validated as current Ownvero copy. The checker now permits factual AI-builder
  positioning and rejects unsupported universal safety/testing/autopilot claims.

Controller integration commands (root package owner):

```text
site:build = node wiki-site/build.mjs
site:test = node --test wiki-site/tests/*.test.mjs
check:site = node scripts/check-site.mjs
check:download = node scripts/check-download-manifest.mjs
```

Existing check:launch remains node scripts/check-launch-copy.mjs; its negative
fixtures run with node --test scripts/check-launch-copy.test.mjs.
Root README, wiki index, deployment and owner architecture links remain controller
ownership. No root package or README modified by this worker.

## Verification

- node wiki-site/build.mjs: passed, 19 generated HTML pages, download unavailable.
- node --test wiki-site/tests/*.test.mjs scripts/check-launch-copy.test.mjs:
  14 tests passed, zero failures; final run 41.47s.
- node scripts/check-site.mjs: passed, 19 pages / 28 allowlisted files.
- node scripts/check-download-manifest.mjs: passed truthful unavailable state.
- node scripts/check-launch-copy.mjs: passed one active fact sheet, six explicit archives.
- node scripts/check-docs.mjs: passed 42 docs and relative links after controller nav work.
- Scoped git diff --check: passed; Git reported line-ending normalization notices only.

Rendered Edge browser acceptance covered desktop/tablet/mobile, deep-link refresh,
legacy hash navigation, 404, Vietnamese entry routes, search results/no-results,
platform limitations, no installer fallback, all four demo evidence states, before/
candidate labels, real lazy-loaded WebGL, resource release, reduced motion, and
no-JavaScript reading. No runtime errors or outbound requests recorded. Synthetic
demo nodes and explicit edges are identical for 2D/list/3D. No target tests run.

The frontend skill render-check ran on landing and verification docs at 375x812,
768x1024, 1440x900. Final results both zero errors / zero warnings. Fixed mobile
outline availability, small touch links, and initially crowded 3D labels. Direct
rectangle measurement after the camera/layout fix found zero default 3D label
overlaps at 360 and 1440px. Screenshots visually reviewed.

Design rubric: 23/24. Brief fidelity, hierarchy, spacing, alignment, typography,
color, depth/shape, states, responsiveness, content, and absence of generic AI
decoration score 2 each. Craft detail scores 1 because the wordmark remains a system
font draft, and native small-size/scaling acceptance remains deliberately open.

One cold mobile lab run: Edge headless, 360x800 DPR1, local Windows host, 4x CPU
throttling, 150ms latency, 1.6Mbps downstream/750kbps upstream. LCP 952ms, CLS 0.
Initial client plus shared demo-data JavaScript is about 3KiB gzip; Three.js is
separate and loaded only after a click. This is a bounded local lab result, not
field performance or a clean-device result. Exact final asset sizes are in the lab
record. No continuous render loop exists; renderer listeners/observer/resources
are disposed on close and page exit.

Private local artifacts (never copied into public output):
[browser results](../../.qa/ownvero-site/browser-results.json),
[mobile lab](../../.qa/ownvero-site/mobile-lab.json),
[landing layout report](../../.qa/ownvero-site/render-check-final/render-report.json),
[docs layout report](../../.qa/ownvero-site/render-docs-final/render-report.json),
[3D layout measurement](../../.qa/ownvero-site/3d-label-layout.json),
[desktop landing](../../.qa/ownvero-site/landing-1440.png),
[final 3D desktop](../../.qa/ownvero-site/demo-3d-final-1440.png).

Process custody: browser test owns and stops its port-5184 server. A separate local
preview server was tracked through execution session 49376 and stopped after render
review. Final check showed no listener on 5184. No dev server intentionally remains.

## Concerns and remaining acceptance

- G2/G3 pending: no public Ownvero identity switch or download enablement claimed.
- The public private-vulnerability-reporting API returned enabled:false and the
  canonical security policy has no private contact. Support names this limitation;
  it does not fabricate a usable private security channel. Establish the route
  before publication. No setting changed.
- Native icon/export/scaling/installer acceptance, name clearance, clean-Windows
  runner packaging, network enforcement, real owner repeat-use pilot, and actual
  approved-artifact metadata matching remain their separate gates.
- No full WCAG audit, screen-reader session, or native 200% scaling acceptance was
  inferred from the bounded layout/browser checks. Keyboard/native reading controls
  are implemented; broader human acceptance remains open.
- Independent review/simplifier requested from controller because all four active
  agent slots were occupied. Source self-review and tests completed; do not label
  that as an independent security or accessibility review.

Unresolved questions: none for local implementation; approval/contact decisions
remain with the owner at their publication gates.
