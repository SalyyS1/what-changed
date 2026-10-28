---
phase: 10
title: "Production brand and coherent app shell"
status: in-progress
priority: P1
effort: 24h
dependencies: [9]
required_approvals: [G0, G1, G2]
---

# Phase 10 — Production brand and coherent app shell

## Goal / gate

Turn raster brand concept into consistent production assets and calm UI.
G2 required before public identity switch. Preserve installer/data identity contract.

## Inventory

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Modify | D:/Project/.WhatChage/assets/brand/icon-source.svg; assets/brand/README.md | Master asset + metadata | Small-size geometry |
| Modify | D:/Project/.WhatChage/scripts/{export-brand-assets.ps1,check-brand-assets.mjs} | Existing pipeline | Hash/palette/parity |
| Modify/generated | D:/Project/.WhatChage/src-tauri/icons/ and installer artwork paths in tauri.conf.json | Generated, enumerate before write | Native installer/tray |
| Modify | D:/Project/.WhatChage/src/app/TitleBar.svelte; src/lib/design/{tokens.ts,theme.svelte.ts} | Brand/theme only | Contrast/native controls |
| Modify | D:/Project/.WhatChage/src/styles/tokens.css | Runtime token authority | TS mirror/contrast/Three palette parity |
| Modify | D:/Project/.WhatChage/docs/brand-guide.md; docs/launch/messaging.md | Owning documentation | Truthful claims |
| Modify | D:/Project/.WhatChage/scripts/check-launch-copy.mjs | Factual AI-builder positioning vs prohibited overclaims | Negative claim fixtures |
| Create | D:/Project/.WhatChage/docs/ownvero/migration-guide.md | <160 lines | Name/data path support |

Brand owner exclusively edits master/generated outputs after inspecting existing manifest.
Use the installed brand/logo/design skills during cook, not ad-hoc raster tracing.

## Steps

1. Verify Ownvero name/domain/store/conflict checks as a limited screening; escalate legal
   clearance where needed, no claim a no-result search proves ownership.
2. Rebuild approved open-O/frame logo as production vector with mark/wordmark/mono/reverse
   variants. Keep charcoal/muted green, clear silhouette at 16/24/32px.
3. Extend existing exporter and hash manifest, generate all icon/tray/installer/social
   outputs from one master. Raster concept is reference, not authoritative SVG.
4. Make shell headings/navigation/empty/error/consent/acceptance copy consistent.
   Keep legacy diagnostics labels meaningful and technical details progressively disclosed.
   Change CSS runtime tokens and TS mirrors together; theme/Three colors read shared
   tokens. Inspect actual rendered contrast, not only exported token objects.
5. Check all brand consumer paths in config/source/docs, but do not blindly replace
   identifiers, app-data paths, crate names, autostart executable or historical reports.
6. Native icon/tray/installer/titlebar acceptance at 100/150/200% display scaling,
   dark/light/high-contrast; keyboard window controls and focus behavior preserved.
7. Update migration guide “What Changed → Ownvero” explaining unchanged data path and
   different Mark working vs owner acceptance. Include honest unsigned/update limits.
8. Migrate launch-copy gate's blanket AI ban to factual AI-builder scope, preserving
   bans on unsupported autonomy/safety. Disposition all six docs/launch consumers:
   launch-article-en.md, launch-article-vi.md, community-posts.md, demo-script.md,
   press-kit.md, release-notes-template.md. Update current Ownvero copy or explicitly
   archive as legacy; don't silently validate obsolete articles. Add gate tests.

## Verification / matrix

~~~powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/export-brand-assets.ps1 -UpdateTauriIcons
bun run check:brand
bun run check:launch
bun run check
bun run test -- src/lib/design/contrast.test.ts src/lib/a11y/contrast.test.ts
~~~
Re-scout exact asset output list before generation; run native checks on fresh candidate.
Plain export:brand defaults to .qa/brand-export and is only preview. Production flag
updates tracked icons/installer BMPs/social projection/generated-assets-manifest;
enumerate and inspect each destination before allowing overwrite.

| Priority | Case | Pass |
|---|---|---|
| Critical | Brand replacement vs install identity | Existing data/autostart/uninstall not stranded |
| High | Small icon/mono/high contrast | Recognizable, legible, not green certification symbol |
| High | Export pipeline | All generated outputs bound to same master hash |
| Medium | Copy/screen state | No promises of universal safety/AI autopilot |

- [ ] G2 recorded and vector/source pipeline complete.
- [ ] App/tray/installer/site-ready assets consistent.
- [ ] Contrast/native scaling checks pass.
- [x] Migration and truthful positioning copy available.

Draft mark/mono/reverse/wordmark assets live under `assets/brand/ownvero/`.
The six legacy launch articles have explicit archive dispositions; factual
owner-workspace messaging and negative claim tests are implemented. See
[site/identity report](../reports/implementation-261028-ownvero-site.md).
G2-bound native/master production asset replacement remains open.

## Risk / rollback

Snapshot/hash owned master assets before export; restore only those on failure.
Brand clearance failure changes display assets/name without forcing data/IPC migration.
No repo/domain rename or purchase in this phase.
