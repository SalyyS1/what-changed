---
title: The release is test-green but still not shippable
date: 2026-09-23
summary: "Recovery and release hardening landed, but native-platform, soak, and large-corpus evidence still block a truthful public-v1 sign-off."
---

# The release is test-green but still not shippable

**Date**: 2026-09-23 03:25 +07:00
**Severity**: High
**Component**: Recovery, onboarding, diagnostics export, release gates, public-v1 readiness
**Status**: Ongoing

## What Happened

This continuation hardened several dangerous edges instead of pretending the full plan was done. Recovery IPC now rejects `Repair` and `ResetHistory` while the database is healthy. Onboarding completion/reopen now serialize collector configuration and attempt rollback when checkpoint or settings writes fail. Diagnostics export no longer accepts an arbitrary renderer path; it opens the native save dialog in the backend. Release checks now cover docs, brand assets, launch copy, stale installers, and source-map/dev-marker bytes. The preserved unsigned installer remains available for verification.

## The Brutal Truth

The green test suite is comforting but misleading. We have spent days turning implicit assumptions into gates, and the remaining gaps are exactly the ones automated tests cannot fake: notification click routing, clean-machine installation, accessibility, SmartScreen/WebView2, four-hour soak, and large-corpus performance. It is frustrating to be this close while still unable to call the product shippable. Claiming “all phases complete” now would be dishonest.

## Technical Details

`bun run check` and `bun run test` pass (57 files, 314 tests); the Rust library suite passes 446 with 1 ignored, and architecture/recovery/release/upgrade suites are green (release and migration rehearsals retain their explicit ignored/manual rows). Diagnostics now reports live admitted writer-queue depth instead of a hardcoded zero. The 1M-row performance run missed the stated budgets: summary p95 was 191.862 ms against 80 ms and search p95 was 1,653.059 ms against 250 ms. Native notification support emits a delivery event after `show()`, but the installed plugin does not expose a user-click callback, so click-to-drawer behavior is still unverified. Several tests remain source-shape/phantom tests rather than end-to-end proof.

## What We Tried

We added explicit release gates and evidence sections, reverted speculative query optimizations that did not meet budgets, preserved the release artifact while deleting disposable debug caches, and documented unsupported notification semantics. We rejected weakening targets or relabeling manual checks as complete.

## Root Cause Analysis

The root cause is scope growth combined with acceptance evidence that was too indirect. A passing unit test or a generated binding proved compilation, not renderer-origin security, native Windows behavior, or sustained performance. The plan needed executable gates earlier.

## Lessons Learned

Every release claim needs an artifact or runtime observation tied to the exact criterion. Keep native-platform behavior and performance out of “green by default” summaries. A safety fence at the command boundary is more valuable than relying on UI reachability.

## Next Steps

The lead owns the native notification click decision, 100k/1M post-vacuum measurements, four-hour soak, clean-machine installer/accessibility checks, stronger runtime export/onboarding evidence, and final Phase 25 go/no-go review. Do not publish or mark the goal complete until those items have evidence.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
