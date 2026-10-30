# PM status — full-plan reconciliation

Date: 2026-09-23 03:18 ICT  
Scope: all `phase-XX-*.md` files under this plan and `plan.md`

## Delivery

| Metric | Result |
|---|---:|
| Phase files swept | 25/25 |
| Phase checkboxes | 304/369 complete (82.4%) |
| Remaining checkboxes | 65 |
| Phase metadata | 19 completed / 3 in-progress / 3 pending |
| Completed phases with residual checks | 0 |
| Plan status | `in-progress` |
| Release decision | **NO-GO** |

Counts are from anchored `- [x]` / `- [ ]` rows, not from implementation intent. The separate
product-wide checklist in `plan.md` is excluded, as documented there.

## Phase ledger

| Phase | Status | Checks | Evidence state / residual |
|---:|---|---:|---|
| 01 | completed | 14/14 | toolchain/scaffold gates recorded |
| 02 | completed | 12/12 | shell/design gates recorded |
| 03 | completed | 10/10 | redaction/privacy gates recorded |
| 04 | completed | 19/19 | model/persistence gates recorded |
| 05 | completed | 14/14 | release artifact + D-15 carried gates closed |
| 06 | completed | 14/14 | engine/query gates recorded |
| 07 | completed | 14/14 | timeline/search gates recorded |
| 08 | completed | 13/13 | collector runtime gates recorded |
| 09 | completed | 12/12 | process/port real-OS and test gates recorded |
| 10 | completed | 20/20 | collector-output → persisted `Confirmed` chain gate recorded |
| 11 | completed | 14/14 | service/application gates recorded |
| 12 | completed | 15/15 | update/driver gates recorded |
| 13 | completed | 19/19 | network/startup/event-log gates recorded |
| 14 | completed | 16/16 | checkpoint capture/compare gates recorded |
| 15 | completed | 19/19 | analysis/investigation fixture gates recorded |
| 16 | completed | 17/17 | Change Map gates recorded |
| 17 | completed | 18/18 | System Map gates recorded |
| 18 | completed | 17/17 | Docker/dev-mode gates recorded; live Docker remains a documented limitation |
| 19 | in-progress | 15/19 | manual click/autostart/D-15 open |
| 20 | in-progress | 2/23 | 1M budget, soak, resource, startup/frame and accessibility evidence open |
| 21 | in-progress | 9/19 | fresh artifact/guard/audit/release-script rows closed; clean-machine and manual release gates open |
| 22 | in-progress | 1/7 | deterministic raster/ICO export and hash manifest closed; Windows theme/tray/installer review open |
| 23 | pending | 0/7 | docs/link/license gates automated; installed walkthrough/screenshots/third-party audit open |
| 24 | pending | 0/7 | copy lint automated; media capture/demo/human review open |
| 25 | pending | 0/10 | final clean release, packet and single launch approval open |

## Evidence reconciled this sweep

- `plan.md` remains `status: in-progress`; delivery checkpoint updated to the current 304/369
  denominator and explicitly preserves the open gates.
- P19 evidence records quiet-hours persistence/UI parity, the shared diagnostics report redaction/
  size unit gate, fresh-DB onboarding completion/idempotence and targeted checks. The exact synchronous privacy-summary body now has a recorded
  isolated 100k p95 of 28.1072 ms (21 samples) against the ≤60 ms gate; full-purge byte scanning
  covers the main DB/WAL/SHM artifacts. Native toast click routing, installed onboarding, minimized autostart and D-15 rows
  remain open.
- P10 evidence records the collector-to-investigation §74 chain integration test (6/6); the config
  suspect is `Confirmed` with a three-step chain ending at the live port.
- P20 evidence records 10k/100k measurements and a valid 1M run; 100k broad search and 1M summary/search exceed budgets,
  and post-change 1M size, soak, resources, accessibility and installed measurements remain open.
- P21 now records a newly built x64 NSIS installer pair (4,422,007 bytes each), matching SHA-256
  entries, a release-artifact audit pass and the stable-identity upgrade guard. It still does not
  claim clean-machine install/upgrade/uninstall or manual acceptance.
- P22–P24 automated checks are retained as evidence but do not promote manual/release-candidate
  criteria to complete. P25 remains pending until its dependencies and approval gate are satisfied.

## Blockers and unblock paths

| Blocker | Owner | Definition of done |
|---|---|---|
| P19 installed/manual trust surfaces | main implementation + tester | installed fresh-DB onboarding, minimized autostart, notification click/drawer or documented plugin resolution, export/timing/byte scans, D-15 rows |
| P20 performance/a11y | main implementation + tester | rerun 1M after vacuum change, resolve or user-approve budget exception, complete 4h soak/resource/startup/frame/accessibility/Narrator records |
| P21 release artifact/manual audit | main implementation + tester | clean release script, newly built installer pair + hashes, clean install/upgrade/uninstall, full §1–§91 audit, 15-step walkthrough |
| P22–P24 candidate/media gates | main implementation + reviewer | release-candidate brand inspection, docs/screenshots provenance, deterministic bilingual media/demo and copy review |
| P25 publication | user approval after packet | frozen manifest with zero blocking/unrun gates and one explicit launch approval |

## Scope changes recorded

- Public-v1 extension (brand, English wiki, Vietnamese launch package, MIT license, go/no-go packet)
  remains in accepted scope; it does not relax earlier phase gates.
- Recovery authorization, FTS-aware purge, bounded release verification, brand/docs/launch validators
  are implemented as hardening inside P19–P24; no new checkbox was marked complete without evidence.
- Notification native click callback remains a plugin capability limitation, not silently reclassified
  as a successful click-routing implementation.

## Risk register

| Risk | State | Mitigation / owner |
|---|---|---|
| 1M query budgets fail on current machine | open | main implementation: optimize or obtain explicit exception; do not call 1M gate green |
| Installer/clean-machine behavior unverified | open | tester: build and run installed RC; preserve fixture as evidence only |
| Native notification click callback unavailable | open | main implementation: resolve plugin/runtime support or retain explicit limitation |
| Accessibility/soak/resource evidence absent | open | tester: execute installed/manual gates and record artifacts |
| Publication claims outrun RC | controlled | PM: keep P22–P25 NO-GO until packet rows are named and green |
| Disk headroom for release/perf builds | mitigated | disposable debug/cache targets reclaimed; preserved release fixture retained |

## Next actions

1. Main implementation + tester: execute P19/P20 manual and performance rows; done = every measured
   row has artifact, method and result (or explicit user-approved exception).
2. Main implementation + tester: run P21 release candidate from a clean tree; done = installer,
   hashes, install/upgrade/uninstall, full audit and walkthrough recorded.
3. Main implementation + reviewer: complete P22–P24 candidate/media review; done = deterministic
   assets/docs/copy tied to the same RC.
4. PM/user: assemble P25 packet; done = no blocking/unrun rows and explicit single launch approval.

## Unresolved questions

- Whether the installed notification plugin can expose a reliable native toast-click callback; otherwise
  the click/drawer row remains an accepted limitation and blocks the corresponding criterion.
- Whether the measured 1M summary/search timings can meet the stated budgets without a scope change or
  a user-approved exception.
- When a human can perform the clean-machine, accessibility, soak and publication-approval steps.

## Continuation after sweep — 2026-09-23

- Recovery boundary now uses `Db::recover_for_command`: Repair/ResetHistory rejection for a healthy
  database is checked while holding the lifecycle fence; the regression test covers both actions.
- Onboarding rolls back enabled collectors when the checkpoint-list read fails, in addition to the
  existing checkpoint/settings-write rollback paths.
- Diagnostics reports the live admitted writer-queue depth; it is no longer a constant zero.
- Verification: frontend 57 files/314 tests; Rust lib 446 passed/1 ignored; architecture 10/10;
  database recovery 8/8; release audit with the preserved fixture + current `dist/` 1/1; upgrade
  contract 1/1 with the two explicit ignored/manual rows; docs/brand/launch/version/bindings gates pass.
- One P19 checkbox was promoted by the isolated 100k privacy-summary benchmark (28.1072 ms p95,
  21 samples).
  Installed/manual notification click, clean-machine release, 1M performance, soak, accessibility,
  media provenance and human launch approval remain genuine NO-GO rows.

## Continuation after benchmark reconciliation — 2026-09-23

- Anchored sweep: **302/369 phase checkboxes complete (81.8%)**, **67 open**; metadata is now
  19 completed / 2 in-progress / 4 pending after closing P10.
- P10 is now **20/20** after the persisted collector-to-investigation §74 chain integration test.
- P19 is **15/19**. The diagnostics export redaction/size, fresh-DB onboarding and
  `get_privacy_summary` 100k criteria are closed by backend/UI tests; the recorded exact
  synchronous-body benchmark at p95 **28.1072 ms** (21 samples; threshold ≤60 ms).
- P21 now closes eight artifact/guard/release-script rows after a fresh 0.1.0 x64 NSIS build, checksum pair,
  release-artifact audit and guard reruns; clean-machine install/upgrade/uninstall, shortcut,
  SmartScreen/WebView2 and manual acceptance remain open. P19 installed trust surfaces, P20
  1M/soak/resource/a11y rows and P22–P25 candidate/publication gates stay open.

## Unresolved questions (still open)

- Can the installed notification plugin expose a reliable native toast-click callback?
- Can the measured 1M summary/search timings meet the stated budgets without a scope change or
  user-approved exception?
- When can a human perform clean-machine, accessibility, soak and publication-approval steps?

## Continuation after release/brand hardening — 2026-09-24

- The current anchored sweep is **304/369 complete (82.4%)**, **65 open**; metadata is
  **19 completed / 3 in-progress / 3 pending**. P21 is **9/19** after closing the
  implementation/documentation decision for uninstall data; P22 is **1/7** and now in-progress.
- `scripts/check-nsis-script.mjs` passes against the generated NSIS source and verifies the
  per-user request level, shortcut creation sections and the app-data deletion checkbox guard.
  The local installed candidate launched visibly in 2,096 ms, but NSIS uninstall remains open:
  the account's installed files are EFS-encrypted and the extracted uninstaller helper reports
  `Error launching installer`; history was not deleted.
- `bun run export:brand` plus `-UpdateTauriIcons` now produces the canonical 20 PNG sizes and
  multi-resolution ICO. Two output manifests matched all 21 hashes, and the tracked Tauri icon
  set is covered by `assets/brand/generated-assets-manifest.json` and `bun run check:brand`.
- This hardening does not close P19 native notification/autostart/D-15, P20 performance/soak/a11y,
  P21 clean-machine/upgrade/uninstall/manual acceptance, or P23–P25 publication gates. The release
  decision remains **NO-GO**.
