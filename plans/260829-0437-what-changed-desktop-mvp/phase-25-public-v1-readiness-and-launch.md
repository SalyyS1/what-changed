---
phase: 25
title: "Public v1 readiness and launch"
status: pending
priority: P1
effort: "12h"
dependencies: [21, 22, 23, 24]
---

# Phase 25: Public v1 readiness and launch

## Overview

Final go/no-go for the user-selected target: a public, unsigned Windows v1 with complete product, brand, English canonical wiki and bilingual launch package. This phase verifies and packages; it does not hide failed gates or silently redefine the product.

## Requirements

- Functional: release candidate is built from a clean tree by the documented resource-bounded pipeline, installed per-user, upgraded, uninstalled, and manually walked end-to-end.
- Functional: both installer filenames and SHA-256 hashes exist; source tag/release notes/license/docs/media all identify the same version.
- Functional: public download is unsigned and says “Unknown publisher” plainly; no pretend publisher, signing bypass or automatic update channel.
- Functional: default WebView2 `downloadBootstrapper` behavior is disclosed as install-time network when runtime is missing; runtime remains offline. Offline installer variants remain optional future distribution work due ~127 MB size cost.
- Functional: publication requires one explicit **single launch approval** after the complete go/no-go
  packet lists the exact GitHub release, wiki/site projection and community posts to be published.
  Approval covers only that named version, artifact hashes, URLs/channels and frozen content bundle;
  any changed binary, hash, destination or copy requires a new approval.
- Non-functional: every mandatory P01–P24 acceptance item is complete or explicitly user-deferred; no new technical deferrals can be invented here.
- Non-functional: license and third-party obligations pass; vulnerability reporting/support routes exist.
- Non-functional: rollback keeps the last known-good installer/hash and never rolls database schema
  backward. Before publication, rehearse withdrawal against a private/local release projection: verify
  that the current link can be disabled, the prior artifact/checksum restored, and the incident notice
  path works without deleting or downgrading user data.

## Architecture

`scripts/release.ps1` orchestrates clean verification, target-resolved no-network check, serialized Cargo where measured necessary, build, artifact audit and hashes. `docs/release-audit.md` and `docs/public-v1-go-no-go.md` are the evidence packet. Release hosting is a projection; repository tag, source and checksums remain authoritative.

## Related Code Files

- Modify: `scripts/release.ps1`, `package.json`, `src-tauri/tauri.conf.json` as required by verified release mechanics.
- Create: `docs/public-v1-go-no-go.md`, `docs/release-checklist.md`, `docs/support-policy.md`, `SECURITY.md` disclosure contact section.
- Modify: `README.md`, `ROADMAP.md`, `docs/release-audit.md`, `docs/acceptance-walkthrough.md` with final status/evidence.
- Create: release-manifest/checksum artifact under `dist-installer/` during release; do not commit binaries unless repository policy later says so.

## Implementation Steps

1. Freeze RC commit/version. Require clean working tree and enough disk/commit headroom before builds.
2. Run full resource-bounded verification from a clean target; run frontend checks/tests/build, all Rust targets/features/tests serially or at measured safe jobs, binding parity, architecture/security gates.
3. Run 10k/100k/1M perf gates, 4-hour soak, memory total+private readings per D-15, bundle/frame/startup/idle CPU evidence.
4. Run installed-app accessibility gate: Accessibility Insights FastPass/Live Inspect, keyboard, Narrator, high contrast, 150% DPI, 200% zoom/text and 900×600.
5. Run privacy/destructive-operation gates, including FTS/WAL/SHM byte scans; verify diagnostics export and no runtime network over 10 minutes.
6. Build/install/upgrade/uninstall NSIS artifacts in the cleanest available Windows context; preserve history; record WebView2 behavior and SmartScreen wording.
7. Validate brand assets, docs links/claims, exact license, bilingual content and synthetic media against the same RC.
8. Produce go/no-go packet: gate, command/manual method, result, artifact, owner, exception. Any blocker means NO-GO.
9. Present the frozen publication manifest and get the user-selected **single launch approval**. Only
   after approval: tag, publish source+installer+checksums, project the wiki/site, and post the exact
   approved community content. Stop and re-approve if any artifact/hash/destination/copy changes.
10. After publication, verify download hash/install, links and support/security contact; retain prior artifact and recovery instructions.

## Success Criteria

- [ ] `scripts/release.ps1` passes from clean tree without OOM or skipped steps; every exit code recorded.
- [ ] Full Rust/frontend suites pass; bindings, capabilities, redaction and no-network gates pass.
- [ ] Product loop works on installed RC: observe → timeline → investigate → explain evidence → checkpoint/compare → diagnostics/privacy.
- [ ] Performance/a11y/soak gates pass; D-15 reports both memory figures and gates on neither.
- [ ] Full purge and scoped purge meet documented logical/byte-scan guarantees, including FTS/WAL/SHM.
- [ ] Upgrade preserves 10k seeded events; uninstall keeps history unless explicitly opted into deletion.
- [ ] Canonical MIT license, audited notices, support and security disclosure route exist.
- [ ] English wiki, Vietnamese article, logo/media kit and README all match RC behavior/version.
- [ ] Installer is explicitly documented as unsigned; runtime emits zero network bytes in acceptance session.
- [ ] Human approves publication after reading the complete go/no-go packet.

## Risk Assessment

| Risk | Signal | Pre-decided response |
|---|---|---|
| Cargo verification OOM repeats | rustc LLVM allocation failure | stop parallel run; preserve log; use measured safe job count; do not classify follow-on artifacts as code failures |
| Unsigned installer suppresses adoption | SmartScreen blocks test users | disclose exact flow; do not weaken Windows or fake signing; signing remains later funded work |
| WebView2 missing offline | bootstrapper cannot download | document internet requirement; offer large offlineInstaller artifact only by later explicit distribution decision |
| Release docs claim green with gaps | blank evidence/Not run row | NO-GO; return work to owning phase |
| Post-release regression | install/hash/support smoke fails | withdraw current link, restore prior known-good artifact, fix forward; never roll DB schema back |

## Unresolved Questions

None. Public channels/date are included in the frozen manifest presented for the single launch approval;
changing either invalidates that approval.
