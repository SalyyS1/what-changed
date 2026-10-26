---
title: "Release rehearsal and trust-boundary fixes"
date: 2026-09-22
type: technical-journal
---

# Release rehearsal and trust-boundary fixes

## Context

The Phase 19–21 sweep was still carrying real release blockers: a directory was validated as a
file, diagnostics export lacked the save capability, onboarding captured an empty first checkpoint,
folder purge missed descendant baselines, and quiet-hours tests assumed UTC epoch hours.

## What happened

The fixes were kept at their owning boundaries. The release script gained a directory resolver;
capabilities and the exact-set test gained only `dialog:allow-save`; onboarding applies persisted
collector choices before the first checkpoint and rolls workers back on any failure; the UI hydrates
those choices from settings. Full purge removes app-owned migration backups and restores runtime
collector state if replacement fails. Folder purge uses bounded descendant predicates for both event
metadata and config snapshots. Quiet-hours policy now uses the local clock, with a DST-safe test.

The repeatable release rehearsal then passed version/network guards, formatting, strict Clippy, 444
Rust tests (one explicit ignored real-OS gate), generated bindings, 314 frontend tests in 57 files,
typecheck, production build, bundle budget (137,890 gzip bytes), docs links and upgrade guard.

## Reflection

The first green-looking run was not enough: the release test exposed an environment-sensitive quiet
hours assumption, and the next run exposed an accessibility-label assertion that had drifted from the
live/runtime distinction. Both failures were useful because they changed the evidence rather than
being hidden behind a skipped gate. The remaining installer, 1M performance, soak, and installed
accessibility checks are still explicitly open.

## Decisions

- Keep native notification click behavior marked not-verifiable until the installed desktop plugin
  exposes a trustworthy click callback; do not auto-open the drawer on delivery.
- Keep installer and 1M performance outside the routine local rehearsal because they require large
  artifacts/manual Windows evidence; report the skips explicitly.
- Keep the plan NO-GO for public release until the remaining manual and artifact gates have named
  evidence.

## Next

Run the real Tauri installer build and audit on a machine with sufficient disk, then execute the
remaining Phase 20 resource/soak/accessibility measurements and complete the Phase 23–25 launch
traceability/manual packet.
