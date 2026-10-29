# Experience contract — app, visual models, public website

Proposed UX; evidence labels must use the domain contract, not a second frontend verdict.
Tone: calm, human, concrete. White/warm neutral + charcoal + muted green accent.
No RGB/gradient/neon/glowing AI brain, vanity scores or “100% safe” badge.

## First session and repeat session

1. Choose “My AI-built app”; name app, choose authorized preview and optional local root.
   No account needed. Include “Try example” clearly separated from actual app data.
   Explain supported-preview-contract.md prerequisites; offer one-time builder setup
   brief rather than implying any pasted URL gives backend/current-build verification.
2. Confirm 1–3 capabilities through examples: create booking, staff sees booking, cancel.
   Suggestions remain drafts; do not infer features from ports/packages.
3. Add one intended change. Wizard asks outcome, example, what must stay and unresolved
   business choice. Save locally; resume after closing.
4. Preview readable brief, review redaction/scope, copy Markdown into existing coding AI.
5. After builder finishes, select candidate and approved environment; choose scenarios.
   See execution scope and consent. Nontechnical statuses explain the next action.
6. Compare expected/observed, open step evidence, accept/reject/defer explicitly.
7. Next session shows last owner decision, what changed since it, stale checks and
   “Request next change”; no incident required to get value.

## Screen map

| Screen | Main question | Primary action | Required non-happy states |
|---|---|---|---|
| My apps | App nào đang làm? | Open / create app | empty, loading, unavailable DB, archived |
| App overview | App làm gì, còn gì chưa biết? | Request change / check candidate | no rules, partial coverage, stale, offline |
| Change request | Muốn gì, phải giữ gì? | Review brief | draft conflict, missing decision, failed save |
| Scenario editor | Bằng chứng nào đủ? | Confirm scenario revision | unsupported, ambiguous locator, needs role |
| Run review | Thực tế xảy ra gì? | Accept/reject/defer | failed, blocked, interrupted, expired artifact |
| Project memory | Điều gì đã quyết và vì sao? | Export / handoff | redacted, import conflict, incompatible format |
| Local diagnostics | Máy local thay đổi gì? | Existing investigation | existing errors/gaps preserved |

At 900×600: one task panel, collapsible left nav, details drawer. At larger widths:
context + work area + evidence inspector. Avoid three narrow unreadable columns.
Route IDs stable; deep links carry app/change/run IDs with validation, never secrets.
Keyboard navigation, visible focus, Escape close, dialog focus return, screen reader
labels, 200% zoom, forced colors and system reduced-motion are acceptance criteria.

## 2D models — required, primary

### Change journey

Five labeled steps: Intent → Preserve → Brief → Verify → Decide.
Each shows real state/count and clickable artifacts. Step does not advance simply
because animation completes. Dashed connector for missing prerequisite. “Why blocked?”
opens exact issue, not generic warning. Save state survives route/restart.

### Capability and evidence map

Nodes: capability, confirmed rule, scenario, run/result. Edges explicitly typed:
requires, verifies, observed_by. Layout deterministic, grouped by capability.
No “dependency” edge inferred from visual proximity. Selected change overlays only
explicit linked rules plus “coverage unknown” for unmapped effects.
No automatic claim that a file diff proves which business behavior changed.

### Before/after review

Side-by-side candidate summaries with same scenario version by default. Different
rules/assertions displayed as incomparable with a rule diff. No misleading percentage
improvement from dropping scenarios. Filter failures/gaps and jump to expected/observed.

## 3D model — required, secondary mode

Concept: **Evidence workbench**. An orthographic/isometric three-layer model:

~~~text
Owner intent      capability cards + rule revisions
        │ explicit verifies/requires links
Test scenarios    ordered scenario cards + active step
        │ executed-on links
Observed result   evidence tiles + build/environment labels
~~~

This explains which evidence supports which intention, not a globe/particle cloud.
Same semantic graph as 2D. Click node → same inspector. Select capability → isolate
its rule/scenario/result path. “Explode layers” separates the three planes without
changing semantic distance. Before/after toggle has explicit build labels.

Controls: 2D / 3D / list; reset view; fit selected; zoom; layer visibility; legend;
keyboard-operable equivalent list always available. No auto-rotate, physics layout,
moving background or motion needed to understand state. Camera tween 160–220 ms,
off with reduced-motion; one intentional highlight at a time.

Style: flat matte cards/tiles, low-contrast depth shadows, crisp readable DOM labels,
small edge strokes, consistent shape+label+color. Green reserved for a named current
passed scenario; grey unknown, amber stale/blocked distinction via icons/text, red failed.
Owner accepted is a separate outline/label, never repaint all tests green.

Performance design budgets (measure, not current claims): lazy-load Three only on
3D entry, no frame loop when idle/hidden, 120 nodes/240 edges visible max with explicit
summary/expand, no invisible truncation. 60fps target / >=30fps low-tier at fixed fixture;
fallback to 2D after repeated capability/perf failure with visible explanation.
Destroy listeners, observers, controls, textures, geometry/materials and renderer.
Existing resource lifecycle helps but current continuous animation loop is not copied
uncritically. See [renderer disposal](https://threejs.org/docs/pages/WebGLRenderer).

## Website scope and information architecture

Public static site under existing GitHub Pages base path /what-changed/ until separately
approved. Proposed source owner wiki-site/ after reconciling remote-only source.
Use existing generator if suitable; do not introduce a server or framework migration
just for a landing page. Build-time Markdown rendering, escaped/sanitized content,
static search index; tiny Svelte demo island only if existing tooling supports it cleanly.

| Route under base | Content / action |
|---|---|
| / | Hero, actual value loop, product shots/demo, limits, download + docs CTA |
| /how-it-works/ | One real booking change, preserve → test → decide, explain states |
| /demo/ | Interactive synthetic scenario with intentionally failing/stale/blocked evidence |
| /download/ | Available/unavailable state, OS/version/hash/size/notes/notices |
| /docs/ | Wiki hub, categories, local search, breadcrumbs, on-this-page |
| /docs/getting-started/ | First meaningful change, safe test setup, supported target |
| /docs/owner-workspace/ | Rules, changes, brief, decisions and portable memory |
| /docs/verification/ | Auth/test data, what pass means, gaps/stale/retries |
| /docs/maps/ | 2D/3D/list, accessibility and controls |
| /docs/privacy/ | Offline core vs consented network, artifacts, deletion, export |
| /docs/legacy/ | Existing What Changed docs and migration mapping |
| /changelog/ | Versioned released/planned labels; link matching docs |
| /support/ | FAQ, issue template, private security reporting route |

Hero: “Build with AI. Stay in charge.” Supporting sentence explains preserving
working behavior before accepting the next AI-made change. No fake customer logos,
testimonials, downloads, prices, certification or unsupported competitor claims.

Sections: product outcome → 3-step example → evidence demo → why not just ask builder
to test → privacy/limits → download → docs/support. A visitor should understand
the repeated-use job without reading an architecture essay.
Website 2D walkthrough required; the landing preview opens the same 3D evidence model
after first paint so the process is visible without hunting for a control. The full demo
page keeps 3D opt-in on click. Both paths respect reduced-motion, use the same labeled
synthetic dataset, and never include live customer or local QA data.

Wiki: source docs canonical, version + last-reviewed commit shown; search excludes plans,
private QA, logs and legal-review working files. Retain old deep links via build-time
redirect/alias map, test all documented old routes. No custom domain needed to deliver.
English canonical technical docs + Vietnamese landing, quickstart and glossary;
translation gap is explicit, not a broken locale switch or machine mistranslation.

## Download publication contract

Build-time release manifest proposed:
schemaVersion, product, version, channel, sourceCommit, builtAt, approvedAt,
platform, architecture, assetUrl, bytes, sha256, signingStatus, notesUrl,
noticesUrl, docsVersion, minOS, runnerVersion/browserRevision when included.

Only approved immutable GitHub Release asset URLs on an allowlisted repo. Do not
turn an unreviewed local path, arbitrary URL, “latest” redirect or dev build into CTA.
Checksums detect corruption; they do not substitute code signing or approval.
Bad/missing/unapproved manifest → download unavailable + explanation + source/docs links.
Wrong platform → show support limitation, not a deceptive auto-download.
Pages build publishes only explicit site output. No .qa, app-data, auth, source env,
private license packet, screenshot of personal machine or plans directory.

Static hosting fits Pages; it does not run server code.
See [GitHub Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site).

## Budgets and acceptance

- Mobile 360px, tablet 768px, desktop 1440px; docs readable without JS.
- Proposed baseline: landing initial JS <=150 KiB gzip excluding opt-in 3D;
  LCP <=2.5s and CLS <=0.1 in fixed mobile lab profile; record hardware/network and results.
- WCAG AA contrast, keyboard/skip link, search no-results/loading, heading hierarchy,
  link/focus visibility, reduced motion, zoom/reflow; no critical a11y violations.
- Every download metadata field matches frozen artifact; link-check includes base path,
  refresh/deep link, 404, legacy redirect, locale and missing-release cases.
