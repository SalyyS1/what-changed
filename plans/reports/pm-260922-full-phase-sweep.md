---
title: "Full phase sweep — 2026-09-22"
date: 2026-09-22
type: project-management-report
---

# Full phase sweep — 2026-09-22

## Summary

The current plan has 290/369 phase checklist items complete (78.6%), 79 open. Eighteen phases are
complete, Phase 10/19/20/21 are in progress, and Phases 22–25 remain pending. Public-v1 status is
NO-GO: the implementation and repeatable local gates are strong, but artifact/manual evidence is not
complete.

## Evidence

| Area | Current evidence | Verdict |
|---|---|---|
| Rust | 444 passed, 1 ignored in full release rehearsal; strict Clippy and fmt pass | PASS |
| Frontend | 57 files / 314 tests, typecheck clean, production build pass | PASS |
| Release rehearsal | version, target no-network, bindings, bundle (137,890 gzip), docs and upgrade guard pass | PASS with installer/perf skips |
| Privacy/onboarding | purge lifecycle, backup cleanup, folder descendants, rollback, first checkpoint transition fence | PASS by targeted tests; legacy/manual byte scan open |
| 1M performance | prior run: summary 347.757 ms, search 2,876.329 ms; budgets 80/250 ms | FAIL, no exception |
| Installer | existing release artifact preserved, but no fresh Tauri build/audit in this sweep | NOT-VERIFIABLE |
| Notifications | policy/rate-limit pass; native click callback unavailable in installed plugin | NOT-VERIFIABLE |
| Soak/resource/a11y | support tests and docs exist; 4h soak, B2 readings, Narrator/manual pass not run | OPEN |
| Brand/docs/launch | brand checker and docs checker pass; publication/media/manual acceptance remain open | OPEN |

### 2026-09-23 continuation

- The 1M size corpus completed on D: at 749,412,352 bytes against the 1.1 GB budget. The old
  256-page reclaim limit failed the freelist assertion after deleting 1,000,000 rows, so maintenance
  now uses no-argument incremental vacuum; the post-change 10k size gate passes.
- A D:-backed 1M query run measured page 8.807 ms, summary 191.862 ms and search 1,653.059 ms p95.
  Page passes; summary/search remain open. The one-scan range-count experiment was slower on this
  schema and was reverted to the four indexed `COUNT(*)` ranges. A fresh 100k harness run measured
  page 4.891 ms, summary 14.354 ms and search 232.186 ms p95; broad FTS search still exceeds its
  90 ms budget, so no 100k/1M query gate is marked green.
- Release rehearsal after the storage changes remains a local rehearsal with `-SkipInstaller -SkipPerf`.
  The Rust suite itself is 444 passed / 1 ignored, but the live `perf_system_map` budget can fluctuate
  under the full suite (isolated D:-temp run passed at p95 69.704 ms; full-suite attempts reached
  123–159 ms). The dev-environment restart test also passed isolated after one timestamp-ordering flake.

## Actions completed

- Fixed release directory validation and added exact save-dialog capability coverage.
- Added quiet-hours settings controls and local-time policy test.
- Corrected onboarding choice hydration and checkpoint ordering with failure rollback.
- Hardened full/folder purge lifecycle and migration-backup cleanup.
- Updated phase metadata and checkpoint counts; added journal entry under `docs/journals/`.

## Unresolved questions

- Can the Windows release workstation provide enough free space for a fresh NSIS build plus 1M
  corpus, byte scans and installer audit without deleting the preserved release artifact?
- Which native notification API/version will provide a verifiable toast-click callback on the target
  Windows deployment?
- Who will perform and sign off the installed-app Narrator, 150%/200% scaling, 10-minute no-network,
  four-hour soak and clean-machine acceptance checks?
