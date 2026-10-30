---
phase: 11
title: "Landing website, web wiki, demo and downloads"
status: in-progress
priority: P1
effort: 44h
dependencies: [10]
required_approvals: [G0, G1, G2]
publish_approval: G3
---

# Phase 11 — Landing website, web wiki, demo and downloads

## Goal / entry

A visitor understands repeated-use value, explores docs/demo, and downloads the exact
approved app when available. Full [experience/site contract](experience-and-site-contract.md)
is mandatory scope, not a link to raw Markdown only.

## Inventory / website owner

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Reconcile/modify | D:/Project/.WhatChage/wiki-site/ | Existing remote-only tree, exact entry files frozen P01 | Preserve public routes |
| Create if absent after reconciliation | D:/Project/.WhatChage/wiki-site/{site.config.mjs,build.mjs,release-manifest.json} | ~100–200 each/config | Base path/build/CTA |
| Create if no equivalent | D:/Project/.WhatChage/wiki-site/src/{landing,wiki-shell,download,demo}.ts; wiki-site/styles.css | Split at meaningful boundaries | Responsive/search/demo |
| Create | D:/Project/.WhatChage/release/release-manifest.schema.json | <160 lines | Strict artifact validation |
| Create | D:/Project/.WhatChage/scripts/{check-site,check-download-manifest}.mjs | ~150–200 each | Links/secrets/manifest |
| Create | D:/Project/.WhatChage/docs/ownvero/{getting-started,owner-workspace,verification,maps,privacy,faq}.md | ~100–200 each | Canonical wiki content |
| Modify | D:/Project/.WhatChage/docs/{wiki-index,deployment-guide}.md; README.md | Navigation/truth | Existing doc authority |
| Reconcile/modify | D:/Project/.WhatChage/.github/workflows/ actual Pages workflow | Name must be discovered, not invented | Scoped build/upload |
| Create | D:/Project/.WhatChage/wiki-site/tests/{navigation,search,download,demo}.test.mjs | ~120–200 each | Rendered-site acceptance |

P01 remote reconciliation mandatory. These are proposed additions only if no owning
equivalent exists upstream. Freeze exact inventory before coding; don't create parallel
build systems or overwrite remote wiki blindly. Root desktop dist/public are not site source.

## Steps

1. Safely reconcile upstream wiki/workflow with dirty local repo. Record upstream commit,
   file conflicts and deploy base path. Keep existing URLs and source-preview limitations.
2. Choose existing static renderer/build route; SSR/static HTML for docs, build-time search
   index and minimal client JS. No server/accounts or customer URLs submitted to website.
3. Implement complete route/content map from experience contract: hero, how it works,
   interactive example, limitations/privacy, FAQ/support, docs hub/search, changelog/download.
4. Create synthetic booking story with realistic passing/failing/blocked/stale states.
   Demo and product use shared semantic fixture/schema where portable; no Tauri imports
   in public site bundle. Label demonstration, do not present as live customer's app.
5. 2D walkthrough works without WebGL; optional click-to-load 3D illustrates same layers.
   No auto-playing model or heavy above-fold bundle. Rendered UI screenshots from synthetic
   isolated fixture only; personal .qa screenshots never public.
6. Wiki renders canonical Markdown with sidebar/breadcrumb/TOC, local search, version
   and reviewed revision. Markdown sanitized; no unreviewed raw HTML/scripts.
7. Preserve legacy documentation routes with alias manifest. Implement English canonical
   technical docs and Vietnamese landing/quickstart/glossary; locale availability explicit.
8. Implement strict release manifest and available/unavailable/error/platform states.
   No URL for current unsigned QA artifact until G3. A page with download disabled can
   pass UI tests but does not mean binary publication scope is completed.
9. Add output allowlist and secret/private-path scan; source-only workflow permissions,
   pinned actions, no auto publish from untrusted fork or source build PR.
10. Run local static server only if needed, deterministic port 5184 after owner check;
    track PID/cwd and stop it after validation. Preview only this phase; publication P12.
11. Test mobile/desktop/reduced-motion/no-JS, search/nav/deep-link refresh/base path/404,
    legacy redirects, missing/invalid/approved release, checksum display and notices links.
12. Prepare site preview and publication diff for owner review; no made-up traction,
    testimonials, price or exact benefits unsupported by pilot evidence.

## Verification

New commands to implement in package.json before running:
bun run site:build; bun run site:test; bun run check:site; bun run check:download.
Their owners: wiki-site build/tests and scripts/check-site.mjs/check-download-manifest.mjs.
Also bun run check:docs; bun run check:launch; bun run check:brand.
Test site output, not only Markdown source; no deploy during verification.

| Priority | Case | Pass |
|---|---|---|
| Critical | Missing/unapproved/wrong hash manifest | Download disabled, no fabricated fallback |
| Critical | Build includes private data/IPC/env | Build fails before publication |
| High | Old base-path/deep links | Correct content/redirect; no root-domain asset mistakes |
| High | Search/no JS/keyboard/mobile | Docs accessible and navigable |
| High | Demo stale/failed cases | Honest labels; no live network/test execution |
| Medium | Performance | Budget measured under named mobile lab profile |

- [x] Landing/how-it-works/demo/wiki/search/download/changelog/support implemented locally.
- [x] Versioned docs/legacy links and bilingual entry surfaces pass.
- [x] Bounded rendered-site/layout/privacy/manifest tests pass; full human accessibility acceptance remains open.
- [x] Local preview visually reviewed; no deploy or binary upload.

Evidence: [site report](../reports/implementation-261028-ownvero-site.md).
19 static pages, seven preserved aliases, local search and synthetic 2D/3D.
Download remains unavailable. G2/G3, exact approved artifact matching and public
preview approval remain open; these source checks do not complete publication.

## Risk / rollback

Keep prior Pages artifact/commit for rollback; do not force push. A bad release manifest
fails closed and leaves source/docs accessible. Publish job uploads only explicit site
output, never repository root, .qa or dist-installer directory wholesale.
