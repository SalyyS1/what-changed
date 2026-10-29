---
phase: 23
title: "Wiki and documentation completion"
status: in-progress
priority: P1
effort: "10h"
dependencies: [21, 22]
---

# Phase 23: Wiki and documentation completion

## Overview

Ship an English-canonical documentation set that a user, contributor or reviewer can navigate without confusing planned behavior with verified behavior. “Wiki” means versioned repository docs first; hosted projection is optional and never the source of truth.

## Requirements

- Functional: English is canonical; Vietnamese launch content links back to it rather than forking technical truth.
- Functional: reader paths cover install/getting started, first investigation, timeline/search, checkpoints/maps, settings/privacy, diagnostics, troubleshooting, known limits, FAQ, contribution, architecture and release verification.
- Functional: every page labels evidence state: **available**, **verified on this machine**, **known limitation**, **planned/deferred**.
- Functional: use the **MIT License** for public v1 (user decision, 2026-09-10); add the canonical
  `LICENSE` text and confirm third-party notice/dependency obligations separately.
- Non-functional: no duplicated version tables, collector matrices or security guarantees. Link to manifests, tests and owning root docs.
- Non-functional: screenshots include app version/commit and synthetic-data disclosure in adjacent prose or metadata.
- Non-functional: docs work remains useful offline and uses relative repository links.

## Architecture

Keep root authorities (`README.md`, `PRODUCT.md`, `ARCHITECTURE.md`, `COLLECTORS.md`, `ANALYSIS.md`, `PRIVACY.md`, `SECURITY.md`, `ROADMAP.md`) as owning documents. Create task-oriented guides under `docs/`; create `docs/wiki-index.md` as navigation. Any hosted wiki is generated/projected from these files after explicit publication approval.

## Related Code Files

- Create: `docs/wiki-index.md`, `docs/getting-started.md`, `docs/first-investigation.md`, `docs/timeline-and-search.md`, `docs/checkpoints-and-maps.md`, `docs/settings-and-privacy.md`, `docs/diagnostics-and-troubleshooting.md`, `docs/known-limitations.md`, `docs/faq.md`.
- Create/complete: `docs/deployment-guide.md`, `docs/accessibility-audit.md`, `docs/release-audit.md`, `docs/acceptance-walkthrough.md` as owned by P20/P21, not duplicated here.
- Create: `LICENSE`, optional `THIRD_PARTY_NOTICES.md`, `CODE_OF_CONDUCT.md`, `SUPPORT.md` after license/community decisions.
- Modify: root docs only to add navigation and correct final verified status.
- Create: `scripts/check-docs.mjs` for links, headings, status vocabulary and stale planned claims.

## Implementation Steps

1. Build a documentation ownership matrix: concept → canonical file → generated/projection consumers.
2. Add the canonical MIT `LICENSE`; audit dependency obligations and add `THIRD_PARTY_NOTICES.md` only
   where required; align package, installer and README metadata.
3. Write `wiki-index.md` with “Use”, “Understand”, “Trust”, “Contribute”, “Release” paths.
4. Write task guides against the installed app and acceptance fixtures; never describe an unimplemented control.
5. Reconcile root authorities against current source, test evidence and P21 release audit.
6. Capture approved screenshots from a release candidate using synthetic fixtures; add alt text and version provenance.
7. Implement docs check and run link/status/claim audit. Any “works/ships/records” statement needs evidence; otherwise change tense/state.
8. Optionally project to GitHub Wiki/AgentWiki only on a later explicit publish request; repository files remain canonical.

## Success Criteria

- [ ] A new user can install, complete onboarding and run the worked port-3000 investigation from docs alone.
- [x] Every root authority is reachable from README and `docs/wiki-index.md` in at most two links.
- [x] No broken internal link, orphaned guide or duplicate version/dependency table.
- [ ] All screenshots are current, accessible, versioned and clearly synthetic where applicable.
- [ ] Security/privacy/collector limitations exactly match tests and the release audit.
- [ ] Canonical MIT `LICENSE` exists and package/installer/README agree; third-party obligations are audited.
- [x] Hosted wiki, if later created, is identified as a projection rather than authority.

## Risk Assessment

| Risk | Signal | Pre-decided response |
|---|---|---|
| Wiki drifts from product | same fact has two hand-maintained bodies | keep one owner; turn other body into summary + relative link |
| Planned behavior is presented as shipped | guide cannot be completed on release build | block page/release; label planned or remove claim until implemented |
| License metadata drifts | package/installer/README name a different or absent license | `LICENSE` is canonical MIT text; docs check asserts every metadata surface agrees |
| Screenshots leak local data | real username/path/entity appears | regenerate from synthetic corpus; never blur and publish a real history |

## Unresolved Questions

None. MIT was selected by the user on 2026-09-10.

## Automated evidence (2026-09-23)

- `node scripts/check-docs.mjs` passes the repository Markdown route and
  relative-link audit, and now asserts MIT metadata in `package.json`, Cargo,
  the Tauri bundle and the canonical root `LICENSE`.
- `node scripts/check-versions.mjs` passes version agreement at `0.1.0`.
- The docs checker now requires an `Evidence state` label on every user-facing
  Markdown guide (journal entries are historical records) in addition to link
  and license checks.
- These checks do not close installed-app onboarding, screenshot provenance,
  third-party review or hosted-wiki publication criteria.

## Navigation evidence (2026-09-25)

- `bun run check:docs` now checks all eight root authorities within two relative Markdown hops from both `README.md` and `docs/wiki-index.md`, detects orphaned top-level guides, and still validates links, evidence labels and MIT metadata. It passes on 33 Markdown files.
- `node --test scripts/check-docs.test.mjs` passes 3/3 fixture cases, including missing authority, orphan and broken-link failures. The wiki index now links Product, Analysis, brand, performance, release checklist and support policy directly.
- A root/docs source scan found no separate maintained version or dependency table; the one historical installer version reference in `docs/PERFORMANCE.md` is a measurement, not a second version authority.
- This closes navigation reachability only; it does not claim an installed-app walkthrough, current screenshots or reviewed third-party obligations.

## Hosted projection evidence (2026-10-27)

The authorized public wiki identifies itself as a source preview with release
publication still gated and links to canonical repository documents. The local
wiki index and deployment guide explicitly identify the hosted site as a
projection. Read-only verification confirmed live site assets and their source
correspondence; see [hosted verification](../reports/qa-261027-hosted-wiki-verification.md).
This closes only the hosted-projection criterion. Installed documentation use,
screenshot provenance, limitations reconciliation and complete third-party
artifact obligations remain open; public v1 remains NO-GO.
