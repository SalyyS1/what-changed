---
phase: 12
title: "Integrated verification, installer, pilot and release"
status: in-progress
priority: P1
effort: 48h
dependencies: [11]
required_approvals: [G0, G1, G2]
publish_approval: G3
---

# Phase 12 — Integrated verification, installer, pilot and release

Source tooling now separates the core graph, runner source, exact resource
inventory, runtime notice delta, site output and frozen candidate evidence.
`release/candidate-evidence.json` remains unfrozen/NO-GO. The
[pilot protocol](../../docs/ownvero/pilot-protocol.md) and
[deferred acceptance ledger](reports/deferred-acceptance.md) name the next checks.
No current-source installer, full verification or participant outcome is claimed.

## Goal / gates

Ship a coherent tested Ownvero candidate; validate repeat-use value with owners.
G3 approves exact artifact/site manifest before any public binary upload.
No claim the former 52 open gates vanished by renaming the product.

## Inventory / release owner

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Modify | D:/Project/.WhatChage/scripts/{release.ps1,verify-installer.ps1,check-no-network.mjs} | Existing pipeline | Core + runner checks |
| Modify | D:/Project/.WhatChage/scripts/third-party-notices-manifest.json; legal/third-party/ | Generated/reviewed deltas | New runtime/browser notices |
| Modify | D:/Project/.WhatChage/docs/{release-audit,acceptance-walkthrough,public-v1-go-no-go,release-checklist}.md | Frozen candidate evidence | Manual/native gates |
| Create | D:/Project/.WhatChage/scripts/check-owner-release.mjs | ~150–200 lines | Artifact/site/runner binding |
| Create | D:/Project/.WhatChage/plans/261027-0817-ownvero-owner-workspace-web-wiki/reports/{release-candidate,pilot-results}.md | Evidence records | Real observations only |
| Modify | D:/Project/.WhatChage/wiki-site/release-manifest.json | Frozen approved values | Download state |
| Create | D:/Project/.WhatChage/docs/ownvero/pilot-protocol.md | ~120–180 lines | Preregistered criteria |

Release owner serializes manifests, version locks, notices, installer and publish.
No QA runs against personal data without fresh scoped backup/restoration preparation.

## Steps

1. Freeze version/source manifest including dirty changes if rehearsal only. Final release
   comes from reviewed clean source identity; record exact runtime/browser/assets/docs versions.
2. Run full verification matrix, secret-canary/adversarial/evidence/gap/2D3D parity tests.
   New tests must run, not zero-match filters or skipped checks.
3. Reconcile every old open row from P01: inherited needed gates must pass on same candidate;
   proposed superseded requirements need explicit approval + rationale. No blanket waiver.
4. Native fresh install, upgrade existing profile, uninstall keep-data/default, autostart,
   tray, no admin, DB recovery, dual-store purge, runner cancel/crash/sleep and WebGL fallback.
   Use controlled data/test identities; restore machine/profile settings after authorized QA.
5. Run four-hour lifecycle soak covering idle/core, repeated owner changes, opt-in runs,
   cancellation and map mounts. Record overall/child CPU, total/private working set,
   disk/network, orphan count; process handle ownership and no unbounded growth required.
6. Verify bundles and license sources for Node/Playwright/Chromium/new frontend deps.
   Generate new packet; human review of changed obligations cannot be forged from old approval.
7. Updater stays unconfigured until signed feed/key provided. Report unsigned status,
   supported OS/runtime and installer size honestly; no “signed” label from checksum.
8. Freeze pilot candidate/hash/notices/site preview. Arrange pilot via supervised sessions
   or separately authorized private distribution; no public upload yet. Invitations and
   payment offers require owner-approved contacts/copy, never automatic outreach.
9. Run formal pre-publication pilot with 8–10 consenting non-tech owners on a real next
   change and another later change. No silent telemetry; collect agreed anonymized timing/
   result notes, redact data. Observe setup, comprehension, repeat use and maintenance burden.
10. Assess preregistered signals: >=6/10 first useful scenario understood; >=5/10 second
    real change unaided; >=3 concrete better decisions/rework avoided; compare effort
    against existing builder workflow. These are hypotheses, not market benchmarks.
    Report continue/improve/reconsider recommendation; do not silently cut user scope.
11. Freeze final installer/hash/manifest/notices/site claims with pilot outcome. Ask G3
    for this exact release scope. Missing approval keeps NO-GO/CTA unavailable. Missing/
    failed pilot evidence also keeps NO-GO unless user explicitly approves a clearly
    labeled unvalidated preview. Guard validates exact scope digest; changed bytes invalidate approval.
12. Only after G3, upload via approved GitHub route and publish site. Verify downloaded
    bytes/hash, external links/base path/docs/release notes/CTA. Record release/Pages
    receipts. G3 gates publication, not the build/freeze/verification needed to request it.

## Verification / matrix

Existing: bun run verify; bun run test:perf; bun run check:versions; bun run check:no-network;
bun run check:docs; bun run check:launch; bun run check:brand; bun run check:bundle;
bun run check:nsis. Read current release.ps1 help before release invocation.
New: runner/site gates from P07/P11; node scripts/check-owner-release.mjs.
Native/manual/soak/pilot are separate evidence, not implied by CI.

| Priority | Case | Pass |
|---|---|---|
| Critical | Missing approval/notices/current candidate proof | NO-GO, no upload/download CTA |
| Critical | Fresh install/upgrade/purge/cancel | No data loss, no elevated helper/orphan |
| Critical | False pass/secret leak/unapproved network | Release blocked, fix cause + regression |
| High | Four-hour soak + maps + runs | Stable bounded resources, no hidden restart/retry |
| High | Approved hosted artifact | Bytes/hash/metadata/docs all same candidate |
| Medium | Pilot repeated value | Observations recorded, threshold outcome not fabricated |

- [ ] Integrated automated and native gates pass for one frozen candidate.
- [ ] Old residual gates reconciled without unauthorized waivers.
- [ ] Notices and exact G3 approval recorded; hosted artifact/site verified if approved.
- [ ] Four-hour soak and pilot evidence captured or honestly marked awaiting participants.
- [ ] No false “all phases complete” while release/pilot obligations remain open.

## Risk / rollback

Roll back website to prior known artifact and withdraw download manifest if corruption.
Do not silently overwrite published installer with different bytes under same version.
App rollback preserves owner store and restores only explicitly approved consistent
backup; warn about newer data. Stop only owned QA/runner processes and restore startup
state. External delays leave phase pending/blocked by named gate, not fake complete.
