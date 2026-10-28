---
title: Executed recurrence plan and fine analysis attribution
date: 2026-10-04
summary: Source-bound diagnostic evidence identifies graph and recurrence costs without closing acceptance gates
---

# Executed recurrence plan and fine analysis attribution

## What happened

Added opt-in seven-stage analysis timing and thread-bound recurrence plan capture at the actual sorted COUNT execution point. Nine new units, release library 468 passed/1 ignored, service 6/6, repository 24/24 and fixture cleanup 3/3 passed. Both complementary release Clippy configurations passed at the attribution revision. The scheduler-independent capture deadline test remained green.

Fresh canonical 10k/1M diagnostics used the real persisted service. The first sorted recurrence query used its covering index with 5,000 actual completed executions. The 1M five-call mean was 1,724.802 ms; graph construction took 245.282–351.208 ms and recurrence 579.280–1,334.776 ms. These are not acceptance p95. Seed still missed 322.266/240 seconds; no outlier was discarded or host cause inferred.

## Decision

Under cook advisory supervision, choose borrowed, normalized relationship-fact sort/dedup as a bounded graph-construction experiment. Preserve complete graph representation and prove equality to the old builder. Two equality tests and all-feature library 470 passed/2 ignored now pass, with complementary Clippy checks. Two isolated comparisons show 60–67% lower duplicate-heavy builder time and 1–4% higher mostly-unique time; this is not a canonical product gain. Canonical 10k and 1M diagnostics pass; five-call 1M mean is 864.795 ms, not acceptance p95, and seed still misses 330.657/240 s. Profiler-free 10k and 100k each pass all eight budgets over 30 samples; the completed 1M run fails investigation at 1,629.336/1,500 ms. See [graph experiment](../reports/qa-261004-graph-fact-deduplication.md) and [query matrix](../reports/qa-261004-graph-candidate-query-matrix.md).

## Evidence and next steps

The unchanged 1M matrix failed at 1,629.336/1,500 ms p95, with the other seven shapes passing. Seed missed 532.237/240 s. Failed-verification counsel ran before edits: keep graph provisionally and collect one predeclared 30-call paired tail diagnostic before recurrence changes. See [attribution report](../reports/qa-261004-analysis-and-recurrence-attribution.md). Batch insertion attribution follows the query decision; source interleaves rows and entity touches, so keep their timing combined to preserve order and avoid per-event clocks. Keep seed, native/license, installed release, accessibility/resource and public approval gates open. Full plan remains 312/369, 17/25 phases complete. No commit, publication or user AppData access was performed.

## Paired tail follow-up

One exact 30-call 10k preflight and one predeclared 1M profile pass, with strict one-engine/one-service pairing and no discarded observation. The three slowest 1M calls spend 44–46% of their own e2e in recurrence; graph and relationships also rise. Targeted release/devtools Clippy, cleanup and review pass. [Full paired evidence](../reports/qa-261004-paired-investigation-tail.md). These instrumented calls do not close the 1M p95 or seed failure. Counsel approves scoped integer-only reuse statistics before choosing one bounded recurrence experiment; implementation is underway, not verified. AgentWiki publish skipped.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
