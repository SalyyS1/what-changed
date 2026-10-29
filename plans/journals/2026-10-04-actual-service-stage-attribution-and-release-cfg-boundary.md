---
title: Actual service stage attribution and release cfg boundary
date: 2026-10-04
summary: Canonical 1M diagnostics identify recurrence and analysis costs; acceptance budgets remain open
---

# Actual service stage attribution and release cfg boundary

## Outcome
Capture deadline regression is scheduler-independent and passes. New actual-service diagnostics preserve SQL, caps, transactions, scoring and persistence. Timing units 2/2, service integration 6/6, release all-feature library 459 passed with 1 hardware gate ignored; complementary Clippy and formatting pass. Native notice tests 96/96 pass, but reviewed production manifest remains pending.

## Measurements
Canonical 1M routine retention passed. Latest profiler-free 1M query gate still fails at investigation p95 2403.553/1500 ms. Fresh actual-service 10k and 1M diagnostics passed full scored/persisted assertions and owned cleanup. Five 1M calls had mean 1446.826 ms, range 1000.119-2567.529 ms, not acceptance p95. Recurrence 414.322-874.177 ms and analysis 411.929-1287.581 ms dominate. Seed 429.057/240 s remains missed; insertion owns about 93.4% of setup.

## Configuration repair
Normal release Clippy found a test-only noop constructor without its matching cfg guard. Counsel verified the sole caller module and recommended identical test/debug/devtools gating. A broad release all-target nonfeatures check then exposed the intentionally absent integration-test helpers; supported coverage is normal lib/bins without features plus all targets with all features. No tests were hidden and no testing namespace was shipped.

## Next
Counsel recommends finer aggregate analysis stages and a source-owned bound plan at the executed sorted recurrence branch, outside measured calls. Preserve time bounds and 150-query duplicate-ID aggregation semantics. Choose one measured owner to optimize before a fresh unchanged 30-sample 1M acceptance run. No budget relaxation, legal approval, public release, commit or personal-history access occurred. Full plan remains active, public v1 NO-GO.

Evidence: plans/reports/qa-261004-retention-native-evidence.md and plans/reports/qa-261004-investigation-service-stages.md.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
