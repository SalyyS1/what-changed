---
title: "What Changed — Windows change recorder & causal investigator (MVP)"
description: "Non-elevated Tauri 2 + Svelte 5 tray app that records meaningful Windows change locally, explains evidence deterministically, and ships as a documented public Windows v1."
status: in-progress
priority: P1
effort: 183h
branch: main
tags: [tauri2, rust, svelte5, sqlite, windows, privacy, observability, desktop, public-v1, brand, docs]
created: 2026-08-29
blockedBy: []
blocks: []
---

# What Changed — Windows change recorder & causal investigator (MVP)

## Overview

"What Changed" is a **non-elevated** Windows 10/11 x64 tray app that continuously records
meaningful system change into a local SQLite history and answers **"It worked before. What
changed?"** in seconds, for a non-technical user, with progressive technical depth for an
advanced one. Stack is fixed: Tauri 2 + Rust + Svelte 5 (runes) + TypeScript + Vite + Bun
(dev only) + Tailwind 4 + Three.js + SQLite. No Electron, no Go, no second process, no
account, no cloud, no telemetry, fully offline.

Repo root (`$ROOT`) = `<repository-root>` — the folder name is misspelled upstream and
**nothing in code or config may depend on it**; product = `What Changed`, binary =
`WhatChanged.exe`, package = `what-changed`. Phases 01–03 are committed on `main`: the toolchain
and scaffold, the design system and app shell, and the redaction chokepoint.

Research probed 35 troubleshooting capabilities under a plain `BUILTIN\Users` token: **31 work**
(30 clean, plus `Win32_QuickFixEngineering`, permitted but under-reporting so badly it is
unusable) and **4 fail**. Two failures are elevation-gated — `Win32_ProcessStartTrace` (access
denied) and the `Security` event-log channel (unauthorized). One is permitted but unreliable
(`__InstanceCreationEvent`: subscribed cleanly, 0 events in 3 s). One is an environment gap
(Docker not installed here). §36's non-elevated privilege model survives all four: each has a
measured non-elevated substitute or is out of scope. They are designed around and documented,
never worked around with elevation.

## Goals

| # | Goal | Priority |
|---|------|----------|
| G1 | A normal human describes a symptom and within seconds sees the most likely related change, with plain-language evidence | P1 |
| G2 | 12 collectors record meaningful change continuously, non-elevated, each independently failable | P1 |
| G3 | Deterministic transparent ranking: the port/config causal chain outranks an unrelated Chrome update, with a visible "why" | P1 |
| G4 | Secrets are never persisted and never logged — enforced at one typed chokepoint, proven by leak tests | P1 |
| G5 | Premium calm desktop identity: all six states on every screen, exact §72–§74 microcopy, 900×600 min, a11y | P1 |
| G6 | Idle CPU ≈0–1%; memory measured as both total and private working set at every checkpoint against the 80 MB reference line — user decision D-15: both reported, neither a gate | P2 |
| G7 | Three.js used intentionally — the causal Change Map (P16) and the §48 System Map (P17) — and never required for core use. The §39 ambient field is a deliberate omission: §39 phrases it as a recommendation, and one purposeful visualization beats three decorative ones | P2 |
| G8 | Installs, runs offline, upgrades without ever destroying the event database | P1 |
| G9 | Architecture ready for a future sidecar collector, AI explanation provider, and Impact — none built now | P3 |
| G10 | Public unsigned Windows v1 has one tested brand system, English canonical wiki, a Vietnamese launch story, an OSI-compatible license, and a release go/no-go packet whose claims trace to evidence | P1 |

## Phases

| # | Phase | Status |
|---|-------|--------|
| 01 | [Foundation & Toolchain](./phase-01-foundation-toolchain.md) | completed |
| 02 | [Design System & App Shell](./phase-02-design-system-app-shell.md) | completed |
| 03 | [Redaction & Privacy Chokepoint](./phase-03-redaction-privacy-chokepoint.md) | completed |
| 04 | [Domain Model & Persistence](./phase-04-domain-model-persistence.md) | completed |
| 05 | [Typed IPC, Tray & Lifecycle](./phase-05-typed-ipc-tray-lifecycle.md) | completed |
| 06 | [Event Engine & Queries](./phase-06-event-engine-queries.md) | completed |
| 07 | [Timeline & Search UI](./phase-07-timeline-search-ui.md) | completed |
| 08 | [Collector Runtime](./phase-08-collector-runtime.md) | completed |
| 09 | [Process & Port Collectors](./phase-09-process-port-collectors.md) | in-progress |
| 10 | [File & Config Collector](./phase-10-file-config-collector.md) | completed |
| 11 | [Service & Application Collectors](./phase-11-service-application-collectors.md) | completed |
| 12 | [Update & Driver Collectors](./phase-12-update-driver-collectors.md) | completed |
| 13 | [Network, Startup & Event Log Collectors](./phase-13-network-startup-eventlog-collectors.md) | completed |
| 14 | [Checkpoints](./phase-14-checkpoints.md) | completed |
| 15 | [Analysis Engine & Investigation](./phase-15-analysis-engine-investigation.md) | completed |
| 16 | [Change Map (Three.js causal visualization)](./phase-16-three-js-visualization.md) | completed |
| 17 | [System Map (§48 relationship graph)](./phase-17-system-map.md) | completed |
| 18 | [Docker & Development Mode](./phase-18-docker-development-mode.md) | completed |
| 19 | [Onboarding, Notifications & Diagnostics](./phase-19-onboarding-notifications-diagnostics.md) | in-progress |
| 20 | [Performance & Accessibility Hardening](./phase-20-performance-accessibility-hardening.md) | in-progress |
| 21 | [Installer & Release Audit](./phase-21-installer-updater-release.md) | in-progress |
| 22 | [Brand Identity & Asset System](./phase-22-brand-identity-and-asset-system.md) | in-progress |
| 23 | [Wiki & Documentation Completion](./phase-23-wiki-and-documentation-completion.md) | in-progress |
| 24 | [Bilingual Launch Story & Media Kit](./phase-24-bilingual-launch-story-and-media-kit.md) | in-progress |
| 25 | [Public v1 Readiness & Launch](./phase-25-public-v1-readiness-and-launch.md) | pending |

### Delivery checkpoint — 2026-10-27

The owner approved the exact third-party source notice packet. Normal notices
generation/check and a current-source NSIS build now pass after correcting Tauri
binary discovery of support directories. The 4,525,180-byte installer and its
stable-name copy are byte-identical; source, executable, notice and installer
hashes are recorded in [candidate evidence](../reports/qa-261027-0059-installed-candidate.md).
Native and remaining performance/accessibility/release acceptance continue;
this does not close a phase checkbox or authorize publication.

### Delivery checkpoint — 2026-10-26

Latest continuation, 2026-10-26: **316/369 checked, 53 open**. The user now
authorizes native QA on the current Windows account. An integrity-checked history
backup exists; the original profile remains in place. A corrected standalone build
renders all seven primary screens through the real Tauri bridge without Vite;
bounded search/Privacy/workspace checks and ordinary tray Quit passed. The verified
engineering binary is restored to the local monitoring path. Installer, full
accessibility and four-hour acceptance remain open. Four release process
measurements missed 15 ms; new paired diagnosis justified one original acceptance
run, which passed at 13.7265 ms. Existing repeatability requirements keep P09 open;
no acceptance rows were promoted. See
[continuation evidence](../reports/qa-261026-continuation.md) and
[native QA route](../reports/native-qa-route-261026.md).

### Delivery checkpoint — 2026-09-22

Full sweep: **25 phase files**, **290/369 phase checkboxes complete (78.6%)**, **79 remaining**.
This denominator excludes the separate product-wide success checklist and includes the new Phase 10
privacy acceptance check. Metadata: 18 phases completed, 3 in progress, 4 pending; **all 18 completed
phases have no residual acceptance items**. Phase 05 is **14/14 completed**; Phase 08 is **13/13
completed**; Phase 09 is **12/12 completed**; Phase 11 is **14/14 completed**; Phase 12 is **15/15
completed**; Phase 13 is **19/19 completed**; Phase 14 is **16/16 completed**; Phase 15 is **19/19 completed**;
Phase 16 is **17/17 completed**; Phase 17 is **18/18 completed**.

Phase 09 gate: 481 Rust tests passed with all features, 0 failed; 30.05 s real-OS idle run emitted
fewer than 5 genuine events; 50-cycle p95 was 11.8723 ms for processes and 0.3535 ms for listeners.
Frontend: 191 tests / 22 files, check 0 errors/0 warnings, build 770 modules; fmt, strict clippy,
bindings parity and architecture gates passed. Final review: 9.5/10, 0 findings. Manual dev-app smoke
recorded Notepad/Python starts, `Port 4000 opened by: python.exe`, both relationship directions on
event 56, then the matching process stops and port close in events 143–145.
Phase 11 gate: 8 service and 10 application acceptance tests passed. Twenty real-OS steady sweeps
measured service p95 6.519 ms and registry p95 31.9952 ms; the separately owned 15-minute full service
configuration refresh measured 173.0581 ms. Production composition now activates both collectors and
their startup-safe `application --provides--> service` plus `file --config_of--> service` evidence.
Phase 12 gate: scripted updates 8/8, drivers 6/6, naming 2/2, restart 1/1, resume/status 1/1 and
WUA lifecycle/recreation units 3/3.
Real WUA p95 was 2.7221 ms for 18 rows; five real SetupAPI sweeps over 217 devices measured 376.8582 ms
p95; timeline reads during a sweep measured 1.2294 ms p95. A forced stall was abandoned after
10.0045373 s with no overlap and later recovery. Core Rust units 349 passed with one explicitly-run
SetupAPI performance gate ignored in the ordinary suite; all integration suites,
frontend 201/201, typecheck, bindings, formatting, strict Clippy and architecture gates passed.
Phase 14 gate: durable sanitized checkpoints, collector-owned P09–P13 semantic comparison, partial and
truncated-state handling, bounded automatic triggers, an absolute eight-second capture deadline, no-sweep
compare-to-now and lifecycle-safe shutdown are implemented. The final full gate passed 670 Rust tests
(14 ordinary-suite ignores), 211 frontend tests, strict Clippy, formatting, bindings and production build;
the explicit real Windows capture completed non-partial within the eight-second budget, and the persisted
3,000-item comparison passed its asserted 150 ms p95 bound.
Owners/unblock paths and all per-phase counts:
[`reports/pm-260919-1928-phase-09-sync.md`](./reports/pm-260919-1928-phase-09-sync.md).
Phase 05 residual gates closed: release NSIS artifact verified by size/SHA-256; DB-open/tray-active
total and private memory recorded per D-15. Full ledger and evidence:
[`reports/pm-260919-1938-phase-05-gate-close.md`](./reports/pm-260919-1938-phase-05-gate-close.md).
Continue Phase 10 and every unfinished accepted phase; Phase 05/09 completion does not close the
product plan.

### Delivery checkpoint — 2026-09-23 PM sweep

The mandatory whole-plan reconciliation reread all **25 phase files** and counted **302/369 phase
checkboxes complete (81.8%)**, leaving **67 open**. Metadata is **19 completed / 2 in-progress / 4
pending**; every phase marked completed has zero residual acceptance checkboxes. The active tail is
P19 **15/19**, P20 **2/23**, and P21 **8/19**. P22–P25 remain pending with all
acceptance rows open; their automated evidence sections record only source-level checks and do not
close release-candidate, installed-app, media, performance, accessibility or human-approval gates.

The 100k persisted privacy-summary p95 gate is recorded at **28.1072 ms** against the ≤60 ms
criterion (21 samples; isolated test passed), the diagnostics export redaction/size gate passes
through the shared report renderer, fresh-DB onboarding completion is covered by backend/UI tests,
and full-purge byte scanning covers the main DB/WAL/SHM artifacts. P10's persisted
collector-output → §74 `Confirmed` chain now has a 6/6 integration-test gate. P19 still lacks native toast-click, installed onboarding,
autostart-minimized and D-15 evidence; P20 lacks 1M budget, soak, resource,
accessibility and startup/frame measurements; P21 now has a fresh installer pair, checksum and
artifact-audit evidence, but lacks clean-machine install/upgrade/uninstall and manual walkthrough.
The public-v1 packet therefore remains
**NO-GO** and plan status remains `in-progress`.

### Delivery checkpoint — 2026-09-23 privacy benchmark reconciliation

The anchored phase-file sweep now reads **302/369 complete (81.8%)**, with **67 open**. P10's
end-to-end collector-to-investigation chain, P19's diagnostics export redaction/size, fresh-DB
onboarding and `get_privacy_summary` 100k timing rows are now evidence-backed; the exact synchronous
body recorded p95 **28.1072 ms** (21 samples, ≤60 ms).
No manual, installed-app, release, performance, accessibility, media or publication rows were
promoted by this benchmark.

### Delivery checkpoint — 2026-09-26 release rehearsal and trust-boundary updates

The anchored phase-file sweep then read **318/369 phase checkboxes complete (86.2%)**, leaving
**51 open**. Metadata was **18 completed / 4 in-progress / 3 pending**; the active tail was P19
15/19, P20 9/23, P21 9/19, P22 4/7, P23 2/7 and P24 2/7, while P25 remains 0/10.

The release rehearsal `scripts/release.ps1 -AllowDirty -SkipPerf` reached a fresh installer and all
post-build artifact gates. Rust completed 454 passed / 1 ignored in the ordinary suite, frontend
completed 319/319, and release artifact plus upgrade rehearsal passed 1/1 and 2/2. The fresh raw and
published installers are 4,458,019 bytes with SHA-256
`6a5aa3963b5e5bfbe5dd9a8d1e2d8a02e5b97d24654092de432bbc97a71aaa9e`; generated NSIS artwork and
shortcut/data-deletion checks pass. This remains a dirty-tree rehearsal with `-SkipPerf`; no installed
upgrade/uninstall, accessibility/soak or publication approval is claimed. Phase 20's opt-in query
harness now has current 100k and 1M evidence: page/summary/search p95 **7.513/10.202/26.749 ms**
at 100k and **10.069/12.127/219.873 ms** at 1M, within the 20/30/90 and 45/80/250 ms budgets.
Migrations 009–013 add narrow search ordering, importance-token FTS, exact daily dashboard
aggregates; query-plan, migration and lifecycle tests were green. The migration 013 first-day
clamp and its initial test were later found to include part of the next day; migration 014 repairs
the derived projection without changing the applied 013 checksum.
The 1M seed took 905 s on this
machine; the database-size/reclaim gate now passes at 10k/100k/1M (9.5→2.7 MB,
93.5→23.6 MB, and 930.6→229.2 MB respectively, with 1M freelist 4→0). Resource/soak/manual
P20 gates remain open.

The all-features verification path was also exercised with serialized Cargo and D-backed temp
storage: clippy passed, the 454-test library suite passed with one intentional ignore, and the
focused migration/query/repository/update/upgrade integration set passed (13/8/20/2/1, with the
upgrade rehearsal's two-installer case intentionally ignored). One initial full-suite run hit the
pre-existing 60 ms scheduler-sensitive capture assertion; its intermediate 250 ms wall-clock guard
was later replaced with a deterministic expired-deadline test. The production absolute
deadline/status contract remains unchanged.

P15 now has a named `ExplanationProvider` with a deterministic no-I/O implementation and injection
tests. P21 now exposes an explicit opt-in updater scaffold that performs no network request until a
signed HTTPS feed and release public key are supplied; D-09 is updated to prohibit background checks
while allowing that future explicit path. P22–P24 source gates remain green but their installed/media/
publication rows remain open. G7's Home ambient omission remains intentional and is not reopened.

### Delivery checkpoint — 2026-09-29 boundary and candidate-evidence correction

Full 25-phase sweep: **312/369 checkboxes complete (84.6%)**, **57 open**; **17 completed / 7
in-progress / 1 pending**. P10's all-checked metadata is synchronized to completed; P23/P24's
partially complete metadata is synchronized to in-progress. P09 is reopened at 11/12 because its current-candidate 15 ms process
gate has not passed. P20 is 4/23: its prior 10k/100k/1M result covered only three of the eight
§61 query shapes, while the new App-level §48 route test closes the System Map prominence row (3/3).
P19 is 15/19, P21 9/19, P22 4/7, P23 and P24 2/7, P25 0/10.

Migration 014 fixes the first partial UTC-day boundary without editing applied migration 013; the
v13→v14 repair and fresh-boundary suite passed 14/14, repository 20/20, capture and quiet-cycle
targeted tests 1/1 each, plus rustfmt and all-targets/all-features Clippy. The capture deadline test
now simulates an expired request rather than racing worker scheduling. The 50-cycle process gate
measured 103.9315 ms and 64.2719 ms p95 under highly variable, saturated host load; listeners
remained below 15 ms. Neither process result is an acceptance pass or isolates code from host cost.
The unchanged gate needs a sustained low-load rerun.

The 2026-09-26 NSIS hash below is now a **pre-v14 rehearsal artifact**, not a build from current
source. A new current-source build, 1M v13→v14 upgrade/free-space rehearsal, installed RC checks,
four-hour soak, B2 private-working-set readings, full §61 table, media and public approval remain
open. The third-party notice audit identifies missing artifact-scoped texts and does not clear
publication. `scripts/soak.ps1` now requests `WorkingSetPrivate` by PID instead of mislabelling
private commit; the source correction is not a measured B2 pass.

A later §61 harness extension calls all eight paths, but its first 10k run passed only six:
investigation p95 **1,601.975/400 ms** and checkpoint comparison **230.312/150 ms** failed. The
current fixture also differs from the specified 3,000-entity/15%-relationship burst corpus; the
devtools seeder has not yet created the required 40 checkpoints and 20 investigations. Thus
these numbers are diagnostic, not final §61 acceptance evidence. The 100k/1M eight-path runs
remain unrun. P20's query-plan and DB-size checkboxes were also reopened because their fixtures
or assertions do not cover the full specified eight-path corpus.

### Delivery checkpoint — 2026-10-04 current-source evidence sync

The fresh anchored phase-file sweep remains **312/369 complete**, with **57 open**;
metadata remains **17 completed / 7 in-progress / 1 pending**. This checkpoint
records evidence only: no phase status, checkbox, release approval or public-v1
readiness was promoted.

The current BFS-backed 10k release query gate passed all eight shapes over 30 warm
samples, including persisted investigation at **234.212/400 ms p95**; the stronger
warmup requires `no_cause_reason=None`, with no no-cause shortcut, and non-empty
ranked suspects. The current 100k gate also
passed all eight shapes over 30 warm samples: investigation **220.788/600 ms p95**,
with the other seven shapes within their budgets. The current-source canonical 1M
query matrix completed at **7/8**: first page **0.828/45 ms**, page 10 **0.722/45 ms**,
summary **3.451/80 ms**, search **55.788/250 ms**, details **0.432/30 ms**,
checkpoint comparison **35.965/300 ms** and privacy summary **1.122/200 ms** passed;
persisted investigation was **2,403.553/1,500 ms p95 — fail**. Three fixture tests
passed and the single budget test failed. The exact canonical
fixture and stronger scoring warmup passed their checks; disposable-fixture cleanup
confirmed 3.76 GB free on D:. Seed time was **489,448.246 ms** against **240,000 ms**,
with insertion **455,443.520 ms** (~93.1%) of total **489,378.189 ms**; full cells:
[dated QA evidence](../reports/qa-261004-retention-native-evidence.md). The canonical 1M routine-retention gate passed with **157,304** eligible rows
deleted, **842,696** remaining, **317** batches in **34,745 ms**; its zero-delete
follow-up reclaimed exactly 256 pages in **35 ms**: first-pass pages
**194,187→193,931** (freelist **0→18,058**), then **193,931→193,675** (freelist
**18,058→17,802**). The seed took **955.101 s**, missing the 240 s operational
target. Full retention details: [dated QA evidence](../reports/qa-261004-retention-native-evidence.md).

All-target/all-feature release Clippy passed before and again after the two subsequent
test-only changes; the fresh rerun exited 0 in **56.79 s**, and rustfmt passed. A fresh
all-features release library run passed **457**, with 0 failed and 1 ignored (458 total)
in **23.03 s**; the exact capture test had earlier passed 1/1. These focused checks are
not a clean default verification result or an installed release-candidate pass. The
pending review draft preparation contains **232 packages / 399 original source files**
(SPDX among files), with draft SHA-256
`3703e5681304948b4c449a7d0b2438c502a2d575930f94a6a741535dd128c95d`; it is not an
actual approved legal review or redistribution approval. Public v1 remains **NO-GO**.

The subsequent [actual-service diagnostic](../reports/qa-261004-investigation-service-stages.md)
passed at 10k and 1M with five fully scored/persisted calls. At 1M, recurrence
and analysis were the main measured contributors; e2e mean 1,446.826 ms is not
acceptance p95. Seed still missed 429.057/240 s. Fresh timing units 2/2, service
6/6, library 459 passed/1 ignored and complementary Clippy checks passed.
All 57 plan checkboxes remain open; next steps are finer analysis attribution
and an executed, source-owned bound recurrence plan, not budget relaxation.

The [finer diagnostic follow-up](../reports/qa-261004-analysis-and-recurrence-attribution.md)
now records nine new units, library 468 passed/1 ignored, service 6/6, repository
24/24, cleanup 3/3 and both complementary Clippy matrices passing at its hashed
attribution revision. Fresh 10k/1M profiles verify the first executed bound sorted
recurrence query uses the covering index with 5,000 completed counts. At 1M,
graph construction was 245.282–351.208 ms and recurrence 579.280–1,334.776 ms;
five-call e2e mean 1,724.802 ms is still not p95. Seed missed 322.266/240 s.
Counsel selects a graph-fact deduplication experiment, requiring old/new full
equality and isolated duplicate-heavy/mostly-unique evidence before canonical
acceptance. All 57 remaining boxes and public-v1 NO-GO remain unchanged.

The [graph candidate](../reports/qa-261004-graph-fact-deduplication.md) now has
full old/new graph equality, library 470 passed/2 diagnostic tests ignored,
complementary release Clippy and service/cleanup checks. Fresh profiler-free
30-sample [query matrices](../reports/qa-261004-graph-candidate-query-matrix.md)
pass 10k/100k 8/8, but 1M remains **7/8**: investigation
**1,629.336/1,500 ms FAIL**, seed **532.237/240 s MISS**. Five-call diagnostic
mean 864.795 ms is not acceptance p95. Failure counsel retains the candidate
provisionally and prioritizes one paired 30-call tail diagnostic, with all
observations retained, before a recurrence optimization. The query-plan SQL
parity and other 57 open acceptance checks remain independent obligations.

The [paired 30-call diagnostic](../reports/qa-261004-paired-investigation-tail.md)
then completed exactly once: every measured call has one complete engine and
service record, with warm excluded and all observations retained. Recurrence
occupies about 44–46% of the same three slowest calls; relationships and graph
also vary. These instrumented timings do not replace the failed p95 gate.
Counsel approves bounded, scope-only signature/window reuse statistics before
selecting one recurrence experiment. That change is underway, not verified;
phase/checklist counts and public-v1 NO-GO remain unchanged.

### Delivery checkpoint — 2026-10-05 bounded recurrence statistics

The full anchored 25-phase sweep remains **312/369 complete, 57 open**;
17 phases completed, seven in progress, one pending. The opt-in warm capture
now returns integer-only signature/window reuse statistics. Focused tests
pass 15/15; release/all-feature library passes 479 with zero failures and two
explicit ignores. Both complementary release Clippy configurations pass;
caller integration passes service 6/6, repository 24/24 and fixture cleanup
3/3. Independent review and simplifier found no actionable defect.

The exact five-call canonical 10k structural preflight passes. Its warm
5,000 actual counts contain 320 identical-window repeats and 1,783 adjacent
overlaps; this is an opportunity measurement, not an optimization gain.
The single predeclared fresh 1M structural profile also passes: 3,429
signatures, 5,000 distinct windows, zero exact repeats and 1,571 monotone
overlaps. Exact-window caching is therefore unsupported as the main 1M repair.
Counsel approves one test-only transaction-local rolling COUNT prototype;
autocommit and production routing stay unchanged until parity/performance
evidence justifies promotion. Source and artifact provenance:
[recurrence-window reuse evidence](../reports/qa-261005-recurrence-window-reuse.md).
No SQL, algorithm, acceptance budget, checkbox or release verdict changed.
The profiler-free 1M p95 failure and remaining release/installed gates stay open.
The fresh structural fixture seed still misses 350.420/240 seconds; its
instrumented five-call timing is not a replacement acceptance measurement.

The test-only rolling COUNT experiment passes full-map parity. Its unguarded
tiny-overlap case regresses about 75% and is rejected. The one approved span
guard passes 13 parity tests and the same fixed 88-pair protocol: canonical-like
aggregate improves 16–18%; sparse/tiny results remain mixed, not zero overhead.
Full release/all-feature library passes 492, zero failed, three ignored.
Counsel permits separately reviewed production integration, with independent
old baseline and honest actual full/delta/reuse capture. Final lint found a
test-table type-complexity issue; counseled alias-only repair passes 13 parity
cases, formatting and both complementary release Clippy configurations.
Production integration is underway; no canonical acceptance pass is established.
[Prototype and both retained paired runs](../reports/qa-261005-recurrence-incremental-prototype.md).

The subsequent production source is now integrated and frozen: ≥2,000-input
existing transactions use guarded rolling counts; autocommit/small paths and
independent old aggregation remain. Six capture tests and public-entry-point
parity cover honest SQL/input counters and pinned snapshots. Formatting,
independent source review, conditional simplifier and complementary release
Clippy pass. Focused production tests pass 34 with zero failures; full library
passes 498 with zero failures and three explicit ignores. Caller binaries are
now verified: service 6/6, repository 24/24 and profile fixture cleanup 3/3;
two diagnostics stay explicitly ignored. Frontend typecheck and 61 files/
327 tests pass. Unchanged profiler-free 10k and 100k matrices pass 8/8 each,
Investigation p95 79.222/400 and 147.841/600 ms. The single 1M matrix also passes
8/8, Investigation 1,032.501/1,500 ms. Counsel GO closes exactly the two P20
query-budget checkboxes: fresh sweep **314/369 complete, 55 open**, P20 **6/23**;
phase metadata remains 17 completed / seven in progress / one pending.
The canonical 1M seed remains **316.439/240 s — MISS**; next is bounded,
explicitly enabled batch insertion attribution. No entire phase or release
verdict promoted. No individual sample durations are exported;
all 30 observations per shape are used. [Current matrix record](../reports/qa-261005-recurrence-rolling-query-matrix.md).

The separate [seed insertion recorder](../reports/qa-261005-seed-insertion-attribution.md)
passes focused seven tests, library 505/zero failed/three explicit ignores,
both release Clippy configurations and CLI/repository/writer-fixture callers.
Its 10k preflight exposes an incorrect diagnostic table name; counseled
test-only repair preserves the filter/count and the failed output. Corrected
10k and the single canonical 1M attribution pass all numeric/corpus assertions
and cleanup. At 1M, repository commit is 195.394 s (~58%) versus combined
event/entity loop 142.508 s; diagnostic total run 368.104 s is not seed
acceptance. Counsel approves one internal 1,000 versus 5,000 batch experiment,
with unchanged default/settings and full parity before timing. Source audit
flags batch-dependent entity timestamps; the actual canonical negative proof
confirms one `first_seen` difference despite identical events/edges/ownership.
Event batch 5,000 is rejected with no timing experiment or default change.
Counsel separately approves design of five unchanged logical 1,000-event
event→graph units per outer transaction; exact API audit precedes implementation.
[Bounded protocol](../reports/qa-261005-seed-batch-size-experiment.md).
The separately scoped transaction-body extraction passes 14 focused tests,
full library 512/zero failed/three ignored, both release Clippy configurations
and independent review/simplifier. Affected callers pass seeder 3/3,
repository 24/24, purge 9/9 and insertion profile 2/2, with two intentional
proof/diagnostic ignores. Counsel accepts the extraction and approves bin-only
fixed-five grouping and correctness implementation. Timing/default promotion
remains gated on complete parity, failure checks and independent review.
[Staged grouping record](../reports/qa-261005-seed-transaction-grouping-prototype.md).
Private fixed-five correctness passes 13 grouped cases and all 17 seeder cases,
including the retained negative proof. Default-route purge/profile callers,
six post-boxing failure cases and both strict Clippy configurations pass;
independent review/simplifier and amendment review are clean. Counsel permits
the separate fixed 10k/1M paired timing harness, not default promotion or seed
acceptance. [Correctness and next bounded protocol](../reports/qa-261005-fixed-five-seed-grouping.md).
The exact once-per-scale paired harness passes complete persisted/ownership
parity and sequential cleanup. At10k, grouped1,257.987ms versus baseline
1,109.966ms is13.34% slower; at1M, grouped332,180.967ms versus baseline
369,085.288ms remains above240,000ms. Counsel rejects default promotion and
further pair/size tuning; grouping stays private cfg(test). Verified source
hashes/runners/fixture cleanup and full output are in the
[completed pair record](../reports/qa-261005-fixed-seed-pair-harness.md).
Approved actual-bound page capture follows, separate from seed acceptance.
The 240-second seed target, SQL-plan branch parity, resource/native/installed
and release obligations remain open.
[Revision ledger](../reports/qa-261005-recurrence-rolling-production.md).

### Delivery checkpoint — 2026-10-05 actual-bound page evidence

Page-only source is frozen after independent clean review/simplifier. Focused13,
library525, callers36, both strict release Clippy configurations and formatting
pass. Exactly one canonical10k page1/page10 diagnostic proves actual indexed
SEARCH and complete201-row decoding, raw200-item ordering/cluster/cursor
contracts and owned-fixture cleanup. [Source, gates and full output](../reports/qa-261005-page-plan-capture-ledger.md).
This is two-shape plan evidence, not all-service/filter/scale coverage or p95.
The24-cell matrix remains historical at its recorded frozen checkpoint after
page allocation/inactive-hook changes; fresh current-source latency acceptance
remains explicit for the eventual RC. Counts stay314/369,55 open/P20 6/23.
The subsequent narrow COMMIT PROFILE prototype fails all8 focused cases and is
rejected: no safe actual-statement identity exists under accepted constraints.
All10 byte-exact snapshots are verified before experiment-only rollback; four
shared owners restore exactly. Post-rollback storage106/seeder23/callers47 and
both strict release Clippy checks pass; independent rollback review/simplifier
are clean, no edits. No grouped promotion, changed batch sizes,
checkpoint settings or replacement PROFILE diagnostic is authorized.
[Rejection and regression ledger](../reports/qa-261005-commit-profile-rollback-verification.md).
User chooses a separate
Windows standard test account; its name/profile and safe interactive route
remain pending. Existing current-user history is untouched; release NO-GO.

### Delivery checkpoint — 2026-10-05 actual checkpoint event-window evidence

Frozen eight-path source passes focused10, full-library535/three deliberate
ignored diagnostics, callers7, complementary strict release Clippy and
formatting. Independent review/simplifier are CLEAN, no edits. Exactly one
counseled canonical10k smoke proves actual compare(Some) return parity and
245 fully decoded window IDs with covering indexed timestamp SEARCH.
[Source, hashes, complete output and limits](../reports/qa-261005-checkpoint-window-capture-ledger.md).
The initial preflight fails before any workload; counsel approves only the
absolute QA ancestor correction. No source/assertion change or diagnostic retry.
Owned runner/fixture cleanup passes. This is one compare subquery, not whole
service/branch/scale plan coverage or fresh p95; no checkbox/status changes.
Counts remain314/369,55 open/P20 6/23, public-v1 NO-GO. Separate standard
account name/profile and interactive route remain pending; current history untouched.

### Delivery checkpoint — 2026-10-07 settings-reload defect and audit ledger

The anchored sweep reads **316/369 complete (85.6%)**, **53 open**; metadata is unchanged
(17 completed / 7 in-progress / 1 pending). P21 moves to 11/19: the release-audit ledger
has 91 unique rows, exactly §26 and §75–§78 deferred, and all 87 cited files exist.

A first run of a locally built release executable against a profile with onboarding unfinished
found a front-end loop: `settings.load()` blanked `current`, which unmounted `OnboardingFlow`,
whose mount reloads settings, producing about 63 command round-trips per second and 328 % of one
core with the window hidden. The store now keeps loaded settings during a reload, with a
regression test that fails without it. A rebuilt executable measured 0.210 % of one core with
all collectors off (onboarding unfinished); a later collector-on run measured 9.973 % of one
core under heavy short-lived-process event churn, so the idle-CPU, B2 and soak rows stay open
pending a quiet-host rerun. Details: [`docs/PERFORMANCE.md`](../../docs/PERFORMANCE.md) and
P20. The same profile showed 25,330 events in about 70 hours, so retention now also expires informational process events after 3 days and caps live data at 1 GiB (D-26; tests green, 1M gates not rerun). Public-v1 remains **NO-GO**.

### Historical checkpoint — superseded phase sweep

The anchored phase-file sweep then read **304/369 complete (82.4%)**, with **65 open**. P21's
generated NSIS script now has a static behavior gate for the per-user level, shortcut sections and
the opt-in app-data deletion checkbox; the implementation/documentation row is closed. A local
per-user install smoke launched visibly in 2,096 ms, but uninstall remains unverified because this
account's installed files are EFS-encrypted and NSIS cannot launch its extracted helper. P22's
deterministic ImageMagick exporter now generates the 20 PNG sizes, multi-resolution ICO and tracked
Tauri icon set; two independent exports matched all 21 hashes. P22 remains in-progress for the
Windows tray/theme/installer inspection. Metadata is **19 completed / 3 in-progress / 3 pending**;
P19 is **15/19**, P20 **2/23**, P21 **9/19**, P22 **1/7**, and P23–P25 remain pending. The packet
is still **NO-GO** because the manual, performance, accessibility, upgrade/uninstall and approval
gates are not met.

## Architecture Summary

Svelte UI (WebView2) → **narrow typed Tauri commands** → `AppService` → { `QueryService`,
`AnalysisEngine`, `SettingsService`, `CheckpointService` } → `EventRepository` → SQLite (WAL).
Collectors → `Normalizer` → `Sanitizer` → bounded writer queue → repository. Stateful collector
cycles commit events, entities, evidence edges and baseline atomically before acknowledging staged
in-memory state; the legacy event-only path remains coalesced. The UI never
touches an OS API; there is no `execute_shell`, no arbitrary file read, no SQL from the frontend.

Rust module split (`$ROOT/src-tauri/src/`): `app/` (wiring, state, lifecycle), `commands/` (IPC
only, no logic), `domain/` (§9 types — **source of truth**, exported to TS), `storage/`
(repository + migrations + maintenance + recovery), `analysis/` (parser, scoring, chains,
explanations), `privacy/` + `redact/` (chokepoint), `collectors/*` (platform-agnostic diff→event
logic against provider traits), `platform/windows/*` (the only place Win32/COM lives).
Collectors depend on provider traits, so every collector is unit-testable with fake snapshots and
`core/` stays free of `#[cfg(windows)]`.

Frontend (`$ROOT/src/`): `app/` shell+router, `components/{ui,timeline,investigation,visualization,system}`,
`features/{home,timeline,investigations,checkpoints,system-map,settings,diagnostics,dev}`
(search lands as the P07 command palette and onboarding as the P19 modal flow, not as sidebar
destinations),
`lib/{tauri,stores,types,formatting,design}`, `three/` (the only place `three` may be imported,
enforced by an import-boundary test; `changeMap/` for P16, reused by P17's system map),
`styles/`. Rust types generate `src/lib/tauri/bindings.ts`; a parity gate fails the build if
generated types drift from committed ones.

## Success Criteria

- [ ] `bun run verify && bun run tauri build` is green from a clean clone on this machine
- [ ] Close button hides to tray; monitoring continues; tray has exactly the seven §7 entries
- [ ] 12 collectors run non-elevated; `get_collector_status` shows per-collector health; killing one collector never stops the app
- [ ] Redaction leak suite (§60: `sk-`, `Bearer`, JWT, AWS, PEM, `postgres://user:pass@`, `mongodb://`, cookie, session) proves no raw secret in DB **or** log files
- [ ] §59 fixture: the config→restart→port chain ranks #1 and strictly outranks the unrelated Chrome update, including when the Chrome update is moved to 2 minutes before the symptom and the chain is 40 minutes earlier
- [ ] A change made while the app was **not running** (service, application, driver, update, DNS, startup entry) appears on next start, marked as detected-at-startup with an uncertain occurrence time
- [x] §48 System Map renders the relationship graph, answers "what depends on this?", and `get_system_map` has an implementing phase (P17)
- [ ] 10k / 100k / 1M event suites pass their recorded budgets; timeline virtualizes; no query returns unbounded rows
- [ ] Every major screen demonstrably renders loading, empty, error, success, large-data and small-window (900×600) states
- [ ] Migrations apply forward from empty and from v1 with data; a pre-migration backup exists; a deliberately corrupted DB yields the recovery screen, not a crash loop
- [ ] Every table written to by Rust has a `CREATE TABLE` in `src-tauri/migrations/` — asserted, not reviewed
- [ ] Memory measured as both total working set and private (app-attributable) working set at each P20 checkpoint and recorded in `docs/PERFORMANCE.md` against the 80 MB reference line; per D-15 neither figure is a pass/fail gate, and a report that names only one of the two fails this criterion
- [ ] Notification suite: a stream of Chrome/Spotify updates and process start/stop produces **zero** notifications
- [ ] Both installer names exist: the bundler's `What Changed_<version>_x64-setup.exe` and the published `WhatChanged-Setup-x64.exe`; it installs, upgrades over an existing install without touching `events.db`, and uninstalls without deleting history unless the user opts in
- [ ] `scripts/check-no-network.mjs` resolves the Windows target graph and finds no HTTP client — a lockfile grep is not accepted, because `hyper` is legitimately in `Cargo.lock` for other targets
- [ ] No `unwrap()`/`expect()` in non-test Rust (enforced by clippy); `-D warnings` clean; TS strict with no undocumented `any`
- [ ] A full purge's documented erasure check covers the main DB, WAL/SHM and FTS5 shadow content; recovery reset cannot be invoked against a healthy database without the dedicated backend-validated destructive confirmation flow
- [ ] Public-v1 brand assets regenerate from one vector system and are legible in the Windows tray at 16–24 px, in light/dark/high-contrast modes
- [ ] English canonical wiki, bilingual launch story, synthetic screenshot/demo kit, canonical MIT
  `LICENSE`, audited third-party obligations, support/security routes and public-v1 go/no-go packet all
  match the same release candidate
- [ ] Public publication occurs only after explicit human approval of a packet with zero blocking or unrun gates

## Key Risks

| # | Risk | Signal | Pre-decided response |
|---|------|--------|----------------------|
| B1 | MSVC linker ambiguity: `link` on PATH resolves to Git Bash / MSYS2 coreutils, not MSVC | `error: linker 'link.exe' not found` or LNK1181 | `rustc` finds `link.exe` via its own MSVC detection, so the hazard is currently inert; `scripts/verify-toolchain.ps1` reports the resolved `link` and warns, `scripts/dev-shell.cmd` wraps 14.50's `vcvars64.bat` as the escape hatch. Recorded as D-01 |
| B2 | **Measured at P01: 361–378 MB total, 77–103 MB private.** §37's <80 MB is 4.5× off against Task Manager total, and already met against app-attributable (private) memory at idle — the brief does not say which it means | `scripts/measure-memory.ps1` on the P01 release scaffold; re-measured at P05 and P20 | **Decided by the user (D-15): report both, gate on neither.** P20 records both figures at every checkpoint and the WebView2 share; a report naming one figure fails the criterion. The stated cost is that memory is a measurement rather than a budget, and the trend rows are what any future scope conversation stands on. Still never redefine a metric silently, and never disable collectors to move a number |
| B3 | Non-admin process events are diff-polled → blind spot | Short-lived process absent from timeline in the P09 fixture | Document in `COLLECTORS.md` "Known limitations"; classify exits `unexpected` only with Event Log corroboration; 1h ETW spike, then stop |
| D2 | **Measured: TypeScript 7.0.2 fails the gate.** `svelte-check` 4.7.6 refuses to start without TS 6 *and* 7 installed plus `--tsgo` | `bun run check` → `ts-version-check.js` throws before any file is checked | Pin `typescript@5.9.3`; keep `check` inside `verify`. Recorded as D-02 in `docs/DECISIONS.md`; applied by P01 step 6 |
| R5 | Driver/network/task enumeration is 1.3–2.3 s | Idle CPU >1% in P20 sampling | Notification-driven wake + slow sweep only; never a short timer; accept and document detection latency |
| R6 | Notification APIs for registry/service/device change unproven non-admin | Spike returns error or never fires | Fall back to the measured slow diff-poll for that collector and record the latency in `COLLECTORS.md` |
| R7 | Docker connected path untestable here | No `\\.\pipe\docker_engine` | Verify only the absent path; mark connected path "untested on this machine" in `COLLECTORS.md` |
| R8 | Analysis over-claims causality | Any UI string asserts a cause | Language allowlist test over explanation templates; forbidden phrases fail the suite |
| R9 | The recorder is blind to its own downtime — a change made while the app is closed is never reported | P11/P12/P13 fixture: stop the app, change a service/app/driver/DNS entry, start the app, expect events | Persisted `collector_baselines` (P04) diffed on first cycle instead of an in-memory "baseline only" rule; downtime-detected events carry `occurred_at_is_uncertain`, which P15 scores at `temporal = 0` and caps at `Likely`, so the event ranks on structural evidence and never on a fabricated timestamp. Event-log records written during downtime are genuinely unrecoverable (`EvtSubscribeToFutureEvents`) and `COLLECTORS.md` says so rather than implying coverage |
| R10 | Closed in P17: §48 System Map was declared in P02's nav and P05's command surface without an implementation owner | Historical `get_system_map` stub found by red-team Y-6 | P17 now owns §48 through the storage-backed traversal and shared renderer; P21's §1–§91 walk checks each citation against what it claims to implement, not just that a number appears |
| R11 | `Confirmed` — the product's strongest claim — is unreachable because no collector writes the `config_of` edge it requires | P15's §74 fixture returns `Likely` where it asserts `Confirmed` | P10 now owns `config_of` from three evidence paths (project process, matching listening port, service `ImagePath` under the watched root) with a row-count test that fails an over-eager heuristic. A file that cannot be tied to anything gets no edge and ranks on category and time alone |
| R12 | Closed in P06: multi-field settings writes partially persist on a later validation error | Valid theme + invalid retention returns an error but theme changed | Transactional settings update plus atomicity regression passed; P08 additionally serializes persisted collector/profile settings with runtime application |
| R13 | Recovery reset can destroy a healthy history through direct IPC | `db_recover(ResetHistory)` succeeds while `DbState::Ready` | P19 state-gates recovery and gives healthy deletion a dedicated backend-validated one-use confirmation flow |
| R14 | SQLite FTS5 shadow data survives a logical/scoped purge | Fixture is absent from `events` but present in DB/WAL/SHM/FTS bytes | P19 rebuilds FTS for scoped purges and replaces the database for full purge; byte-scan gate covers all artifacts and docs state the forensic limit |
| R15 | Full verification exhausts rustc/LLVM memory under default parallelism on this machine | `rustc-LLVM ERROR: out of memory` followed by damaged-artifact errors | P20/P25 set and verify a measured safe Cargo job count; resource failure is reported separately and never hidden as a test failure |
| R16 | Public launch assets/docs outrun implemented behavior | A screenshot or claim cannot be reproduced on the tagged RC | P23–P25 trace claims to release evidence, capture only deterministic synthetic data, and block publication on any missing/unrun gate |

## Dependencies

- **Toolchain (P01 blocks everything):** `rustup` 1.29.1 + `rustc`/`cargo` 1.98.0
  (`x86_64-pc-windows-msvc`), installed and pinned in `$ROOT/rust-toolchain.toml`;
  MSVC 14.50.35717 selected (D-01 — 14.44 is not present; the second install is a 2017-era
  14.16 and was rejected); Windows SDK 10.0.26100.0; WebView2 Evergreen present; Bun 1.3.11;
  Git 2.52.0 with repo-local identity set.
- **Crates (currently resolved; manifest/lockfile remain authoritative):** tauri 2.11.5,
  tauri-build 2.6.3, rusqlite 0.40.2 (`bundled`, `backup`), rusqlite_migration 2.6.0,
  ts-rs 12.0.1 (`serde-json-impl`), dirs 6.0.0, sha2 0.10.9, regex, serde,
  serde_json, thiserror, tracing, tracing-subscriber and tracing-appender; Tauri plugins
  single-instance 2.4.4, window-state 2.4.1, dialog 2.7.3, autostart 2.5.1 and
  notification 2.4.0. Planned collector-only crates are resolved in their owning phases and then
  recorded in `docs/DECISIONS.md`; do not resurrect the rejected `tauri-specta` path.
- **npm (pinned):** svelte 5.56.10, @sveltejs/vite-plugin-svelte 7.3.0, vite 8.2.2,
  typescript 5.9.3 (D-02: 7.0.2 measured failing — `svelte-check` 4.7.6 needs TS 6+7 and
  `--tsgo`), tailwindcss + @tailwindcss/vite 4.3.3, three 0.185.1 +
  @types/three 0.185.4, vitest 4.1.11, @testing-library/svelte 5.4.2, @tauri-apps/cli 2.11.4,
  @tauri-apps/api 2.11.1, svelte-check 4.7.6, playwright 1.62.1,
  @tanstack/svelte-virtual 3.13.36, bits-ui 2.19.0.
- **Not yet resolved by its owning phase:** collector/runtime crates such as `notify`,
  `notify-debouncer-full`, `windows` and `tokio`. Resolve against the implementation API, record the
  chosen version/evidence in the lockfile and `docs/DECISIONS.md`; do not invent a pin in this index.
- **Rejected on purpose:** `tauri-plugin-sql` (would expose SQL to the frontend, violates
  §35/§56), `tauri-plugin-log` (duplicate logging stack; `tracing` + `tracing-appender` covers
  §4 and is redaction-wrapped), `sysinfo` (two Win32 calls replace it, §80),
  `Win32_QuickFixEngineering` (measured 4 rows vs WUA's 133), Electron/Go (§4/§5),
  `bollard` (its transport pulls in `hyper` and would fail P21's own release gate),
  `gix` (P18 reads `.git/HEAD` for one branch-change signal), and `tauri-plugin-updater`
  (D-09: no automatic runtime network check).

## Public v1 extension (2026-09-10)

The user chose to extend this accepted plan rather than create a competing roadmap. Target: **public,
unsigned Windows v1**; English is canonical for README/wiki/technical docs and the launch kit includes
a Vietnamese adapted introduction; project license for public v1 is **MIT** (user decision,
2026-09-10; the canonical file lands in P23).

The production-readiness audit is
[`plans/reports/code-review-260910-0023-production-readiness.md`](../reports/code-review-260910-0023-production-readiness.md).
Its verified findings amend their owning phases: P06 atomic settings + complete migration ledger, P07
truthful recovery/degraded status and real primary actions, P08 commit-loss telemetry, P16 final Change
Map information architecture, P19 recovery authorization + FTS-aware purge, P20 resource-bounded builds
and installed-app accessibility, P21 license gate. P22–P25 add brand, wiki, bilingual launch media and
public go/no-go without replacing phases 06–21.

Execution tail: P21 → P22 → P23 → P24 → P25. Brand approval uses the actual release-candidate shell and
installer, docs use the approved brand and release audit, and launch media uses both. No publishing happens
during planning or during P22–P24. P25 asks for explicit approval after every gate passes.

## Red Team Review

### Session — 2026-09-10

**Scope:** production-readiness amendment (P06/P07/P08/P16/P19/P20/P21 and new P22–P25).
**Findings:** 6 accepted, 2 rejected after evidence check. **Severity:** 2 Critical, 3 High, 3 Medium.
Parallel reviewer dispatch was attempted once and failed for all four lenses because the runtime's
`WorktreeCreate` hook returned no path. The controller therefore ran the same four evidence roles locally;
this is not represented as independent multi-reviewer evidence.

| # | Finding | Severity | Disposition | Applied To |
|---|---|---|---|---|
| 1 | UI-only purge confirmation can be bypassed through direct IPC | Critical | Accept | P19 backend challenge + recovery state gate |
| 2 | “Keep settings” can preserve watched roots/history-bearing state after Delete everything | Critical | Accept | P19 explicit non-history preference allowlist |
| 3 | Old WAL/SHM/FTS purge risk text still described the superseded delete+VACUUM path | High | Accept | P19 risk/success criteria reconciled to replacement path |
| 4 | Brand could complete before the release installer it is meant to skin | High | Accept | P22 now depends on P21; roadmap and execution tail updated |
| 5 | Rollback promise lacked a rehearsal before public publication | High | Accept | P25 withdrawal/restore rehearsal |
| 6 | Plan index still contained rejected dependency pins and stale `specta` owner language | Medium | Accept | dependency section and touched phase bindings language |
| 7 | Remove brand/wiki/launch as scope creep | Medium | Reject | explicitly requested by the user; requested scope is a constraint |
| 8 | Require signing before public launch | Medium | Reject | reverses explicit public-unsigned-v1 decision without new evidence |

### Whole-Plan Consistency Sweep

- Files reread: `plan.md`; all 25 phase files through the completed plan read/scout pass.
- Decision deltas checked: atomic settings, recovery authorization, FTS-aware purge, dropped-commit telemetry, Change Map IA, bounded Cargo jobs, public-v1 brand/docs/content/release.
- Reconciled stale references: 14 (phase count/effort, dependency pins, `specta` wording, onboarding field, investigation path, purge algorithm, P22 dependency, open questions).
- Unresolved contradictions: 0.
- Validation subsequently resolved the two open decisions: MIT and contextual Change Map; see Validation Log.

## Validation Log

### Session 1 — 2026-09-10

**Trigger:** user selected validation after the production-readiness audit and red-team amendment.
**Questions asked:** 3.

#### Questions & Answers

1. **[License]** Bạn chọn license nguồn mở canonical nào cho public v1?
   - Options: MIT (Recommended) | Apache-2.0 | GPL-3.0
   - **Answer:** MIT (Recommended)
   - **Rationale:** fixes public rights/metadata with the lightest permissive contract; dependency notices remain independently audited.
2. **[Architecture]** Change Map nên xuất hiện trong information architecture theo cách nào?
   - Options: Contextual view (Recommended) | Eighth route | Swap map roles
   - **Answer:** Contextual view (Recommended)
   - **Rationale:** keeps seven primary destinations and ties the temporal cascade to an Investigation/Timeline context; System Map remains the global relationship destination.
3. **[Publication]** Sau khi P25 xanh, quyền phê duyệt publish nên có phạm vi thế nào?
   - Options: Per-channel approval (Recommended) | Single launch approval | Prepare only
   - **Answer:** Single launch approval
   - **Rationale:** one approval covers an explicitly frozen version/hash/channel/content manifest; any delta requires re-approval, so the broader approval cannot silently expand.

#### Confirmed Decisions

- Public-v1 license: MIT.
- Change Map: contextual surface from Investigation/Timeline, no sidebar item/global number shortcut.
- Publication: one explicit launch approval for one frozen manifest; no standing authorization.

#### Impact on Phases

- P16 and `PRODUCT.md`: contextual-map navigation, origin preservation and focus return.
- P21/P23/P25 and `README.md`: MIT canonical file/metadata/obligation checks.
- P25: frozen publication manifest and single-approval boundary.

### Verification Results

- Tier: Full; reused the evidence-backed 2026-09-10 Red Team Review per validation guard.
- Failed: 0; remaining `[UNVERIFIED]` tags: 0.
- `ak plan validate`: valid; source/code references for the three decisions reconciled.

### Whole-Plan Consistency Sweep

- Files reread/searched: `plan.md`, all 25 phase files, README, PRODUCT and ROADMAP.
- Decision deltas checked: MIT, contextual Change Map, frozen-manifest single approval.
- Reconciled stale references: license undecided, Change Map as sidebar destination, per-channel approval language, prior open questions.
- Unresolved contradictions: 0.

## Ultra Selection

Produced by `/ak:plan --ultra`: five independent planners over one immutable evidence packet, then
a single strongest-model verifier scored them against a rubric fixed **before** any candidate was
read, and selected one winner. The winner is materialized here **unchanged**; losing candidates were
not merged in. Full artifacts: [`reports/`](./reports/).

### Candidates

| Label | Thesis | Phases | Total | Report |
|---|---|---:|---:|---|
| A | Toolchain gate first, then docs, then vertical slices; `Sanitized<T>` chokepoint | 18 | 170 | [candidate](./reports/planner-ultra-candidate-3.md) |
| B | Track the normative spec literally; block secret *acquisition*, not just persistence | 17 | 176 | [candidate](./reports/planner-ultra-candidate-5.md) |
| C | Broadest test surface; fixtures for every worked example in the brief | 21 | 177 | [candidate](./reports/planner-ultra-candidate-1.md) |
| **D — winner** | **Prove the toolchain, weld the privacy chokepoint shut, scale the UI on synthetic data, then land collectors in measured-cost order** | **20** | **184** | [candidate](./reports/planner-ultra-candidate-4.md) |
| E | Seal the contracts first as compile-time and grep-time gates; collectors become additive | 24 | 172 | [candidate](./reports/planner-ultra-candidate-2.md) |

Rubric: [`reports/ultra-verifier-rubric.md`](./reports/ultra-verifier-rubric.md) ·
Verdict: [`reports/ultra-verifier-verdict.md`](./reports/ultra-verifier-verdict.md) ·
Evidence packet: [`reports/ultra-evidence-packet.md`](./reports/ultra-evidence-packet.md) ·
Measured research: [`research/verified-facts-report.md`](./research/verified-facts-report.md)

All five passed all ten hard constraints (no Electron, no Go, no stack swap, Rust install treated as
a blocker, no invented APIs, privilege claims matching the measured probe matrix, no generic IPC
surface, no permanent elevation, no silent scope drops, no unverified assumption stated as fact).

### Why D won

- **Its §59 ranking fixture can actually fail.** It requires the config→restart→port chain to
  outrank a Chrome update *even when the Chrome update is moved to 2 minutes before the symptom and
  the chain is 40 minutes earlier* — so the test discriminates instead of passing on recency. Its
  published weights are numerically consistent with that outcome.
- **Ranking scenarios are written before the engine** ("They are the spec; the engine is written to
  satisfy them"), and include the cases others omitted: an event *after* the symptom that must never
  rank, and a cascade whose head is outside the window that must be `Likely`, never `Confirmed`.
- **Honest on the memory blocker.** It names the cheat it forbids: "Never silently redefine the
  target, never disable collectors to make the number."
- **Every unverified research assumption carries a settling phase, a falsification signal, and a
  pre-decided response** — eleven rows, not prose.

Its one real weakness is format (plan body ~137 lines against ~120 guidance). Kept as-is, because
`--ultra` materializes the winner unchanged.

### Rejected alternatives

- **C** scored one point below D's total but was placed 3rd on the rubric's tie-break: it narrows
  the user's <80 MB target to "backend RSS only", where B and D commit to the whole figure and
  escalate to the user. Disclosed in C, but still a narrowing of a user-set metric.
- **E** was strongest on architecture (four invariants each with a gate; two independent redaction
  exits so a stray `{:?}` cannot leak) but scored 9/20 on format: its report lost its
  `## Unresolved questions` section and status line to a connection drop and left an internal
  placeholder in the deliverable.
- **B** had the sharpest single honesty mechanism in the set — chain confidence is its *weakest*
  link, "which prevents one confirmed hop from laundering a coincidence into a claim".
- **A** had the best toolchain and leak-surface handling but substituted its own signal list for
  §28's mandated six-term sum, dropping `rarity_score` and `failure_proximity_score` as named terms.

### Carried into red-team review

The verifier named two ideas D should absorb. Both are follow-ups for `/ak:plan red-team`, not edits
to the materialized winner:

1. **The `/usr/bin/link.exe` MSVC-shadow hazard** (from E, confirmed present on this machine).
   D's `vswhere` + `vcvars64` machinery disambiguates *which* MSVC install but does not stop Git
   Bash coreutils `link` from winning the PATH lookup first. Fix is one explicit `linker =` path
   plus one gate line asserting the resolved `link.exe` is MSVC. Highest value-to-effort: it lands
   in Phase 01, which everything depends on, and the symptom is a link failure that `vswhere` output
   says should be impossible.
2. **B's acquisition-blocking privacy guard** — fail the build if `collectors/` or `platform/`
   reference `.cmd()` / `.environ()` / `GetCommandLine`. Converts D's prose-only commitment (its own
   unresolved question #5) into a mechanical gate, a few lines in the `tests/architecture.rs` it
   already runs per phase.

### Honest limitations of this selection

- **Verifier coverage was 16–25% per candidate**, by its own exact accounting. The analysis-engine
  phase is the only high-weight phase read in full across all five; the planned third comparison
  phase was covered by grep windows only, and the two extra continuous spot-checks per candidate did
  not happen. Nothing was executed — executability is a specificity judgment, not proof.
- **Only D's 7–8 point lead exceeds the verifier's stated measurement noise.** C 177 vs B 176 and
  E 172 vs A 170 are ties; the 2nd–5th ordering above is not a reliable ranking.
- **Requirement fidelity (C1) is the least reliable criterion** — it was sampled via phases tables
  and greps, not a §1–§91 traceability walk. Closing that gap needs a dedicated pass against D's
  phase inventory.
- **Anonymization was partial.** Full anonymization needed copying ~900 KB of reports to neutral
  filenames; the Bash tool was unavailable. Candidates were presented in randomized order as A–E
  with the verifier told the numeric slot indices are arbitrary dispatch order carrying no quality
  signal, and instructed to ignore them. Weaker than true blinding.
- **Web search was unavailable this session** (`no available accounts`), which stalled two
  researcher subagents. No external documentation was consulted. Version pins come from live
  crates.io / npm registry queries; every privilege and cost claim comes from probes executed on
  this machine under a non-elevated `BUILTIN\Users` token. Everything else is carried as an
  unverified assumption in `research/verified-facts-report.md` §5.
- **`ak plan add-phase` could not be used** to scaffold the phase stubs (Bash unresponsive). Phase
  files were written directly. Run `ak plan reindex` on this directory once a shell is available.

## Amendment log

The plan was extended on 2026-09-10 after a production-readiness review. Verified findings were assigned
to P06/P07/P08/P16/P19/P20/P21, and P22–P25 were added for the user-selected public-v1 brand, wiki,
bilingual launch and final go/no-go scope. This is an amendment to the accepted architecture, not a new
competing plan. The original 2026-09-03 amendment record follows.

The materialized winner (candidate D, 20 phases) was amended on 2026-09-03, applying the
findings of `/ak:plan red-team` and the whole-plan consistency gate. Every change is traceable to a
finding id in `reports/`; nothing was rewritten on taste.

| Change | Findings | Effect |
|---|---|---|
| New **phase 17 — System Map**; old 17–20 renumbered to 18–21 | Y-6 CRITICAL | §48 had a sidebar destination and a `get_system_map` command but no implementing phase — the one requirement of 91 owned by nobody. Three phases also mis-cited §48 while describing the privacy screen, so P21's own §1–§91 audit would have reported it covered |
| Persisted `collector_baselines` table + `occurred_at_is_uncertain` marker, wired through P10/P11/P13 and P15's scoring | F-C1 CRITICAL, F-C2 | The recorder was blind to its own downtime: a service, app, driver or update changed while the app was closed was never reported. P12 reached for a `collector_state` table that no migration created, so its headline test could not compile. P15 now scores an uncertain timestamp at `temporal = 0` and caps it at `Likely`, so a downtime event ranks on evidence without a fabricated time |
| Three missing tables created in P04's migration, plus a table-has-a-migration assertion | X-6 | `config_snapshots`, `investigations` and the baseline table were written to by P10/P15/P12 and created by no one. The assertion in `tests/architecture.rs` is what catches the next one at the phase that introduces it |
| `config_of` relationship rows given an owner in P10 | C-1 (substance) | P15 defines `Confirmed` as a structural chain and names `config_of` as one of four structural edge kinds — but no collector wrote it, so the product's own §74 worked example could not reach its top confidence class. P10 now emits it from three evidence paths only, and a row-count test fails an over-eager heuristic |
| Migration renumbering + allocation rule; one migration directory | X-5 | Two phases both claimed `002`, in two different directories |
| FTS5 measured present; migration `002` ships one body (D-11) | X-7 | A runtime-branching migration gave version 2 two valid checksums and two possible shapes, defeating both the drift checksum and P21's upgrade proof. Probed on this machine: SQLite 3.53.2 with `ENABLE_FTS5`, and numeric, prefix and phrase matching all work against an external-content table |
| Decision ids de-collided; `docs/DECISIONS.md` now allocates them | new, found this session | Five phase files claimed D-03 through D-07, all of which measurement had already assigned to something else. Reserved D-08–D-14 with owners so the next collision cannot happen silently |
| Dependency graph completed | C-1, C-2, C-3 | Phases 06, 07 and 10 were unreachable from the terminal phase; P15's `Confirmed` class depends on a P10 artifact it never declared; P14's own gate was unsatisfiable at its declared position. All 21 phases are now reachable from P21 and every edge points backward |
| `bollard`, `gix`, `tauri-plugin-updater` moved to *Rejected on purpose* | X-1, X-2, X-3 | All three were pinned and unused; `bollard` pulls in `hyper` and would fail P21's own release gate, and the updater pin contradicted D-09's no-network-check decision |
| P21's no-network gate resolves the Windows target graph, not `Cargo.lock` | measured this session | `reqwest` and `hyper` are in the lockfile via Tauri's non-Windows targets and absent from the Windows graph, so the gate as specified failed on a clean scaffold and would have been disabled by the first person to hit it |
| §65 installer clauses closed: both artifact names, Start Menu and optional desktop shortcuts; §66 signing separation recorded | Y-1, Y-2, Y-3 | The release success criterion named a file the build never produced. Two of §65's four clauses and one of §66's had no owner at all |
| §36 cited in P08, §37 in P20 with both memory figures | Y-5, B2 | Both requirements were covered but cited only in `plan.md`, so the traceability walk had to infer them. P20 now reports total *and* private working set at every checkpoint, and a report naming one figure fails the criterion |
| Capability-count claim corrected to 31/35 with four named failures | X-4 | "34 of 35 … the two measured gaps" was a miscount that contradicted itself in the same sentence, sitting in the Overview where a reader forms trust in everything after it |
| G7 narrowed to two Three.js surfaces | Y-7 | The goal named three; the plan built one. The ambient field is now an explicit §39-permitted omission with the reason, rather than a silent gap |
| Risks B1, B2, D2 rewritten from measurement; R9 and R10 added | measured this session | See Unresolved questions for what each measurement found |
| §48/§22 citations corrected | Y-4, Y-6 | A wrong section number defeats the traceability audit built to catch exactly this |
| Effort 131h → 143h | consequence of the above | P17 is 7h of new work; P04, P10, P11, P20 and P21 each grew by an hour for the schema, relationship, downtime, dual-memory and installer work the findings added |
| Four open questions answered by the user; D-15 recorded | user, 2026-09-03 | Memory: both figures, neither a gate (G6, B2, P20, `docs/PERFORMANCE.md`, `ARCHITECTURE.md`). Folder name stays. Docker ships untested here. Both installer names (P21, `README.md`). The plan now has no open questions |

Per-phase findings (F1–F8, F-P16-A–H, F-C1–C8, N1–N12) are resolved inside the phase files they
belong to; the reports remain the audit trail.

A second consistency gate then re-audited the amended plan
([`reports/whole-plan-consistency-gate-2.md`](./reports/whole-plan-consistency-gate-2.md)) and found six
further defects, all now fixed. Five are the same shape as the review's original CRITICAL — a claim in
one file resting on something no other file provides:

| Change | Finding | Effect |
|---|---|---|
| `config_of` relationship rows given an owner in P10 | N-1 HIGH | P15 defines `Confirmed` as a structural chain and names `config_of` as one of four structural edge kinds. A whole-plan grep found exactly one hit — P15's own definition. The product's §74 worked example asserts a `Confirmed` 3-step chain, so without the edge the plan's headline scenario failed at the phase that owns it |
| Downtime fix wired into P10/P11/P13/P15 | N-3 HIGH | R9 and `collector_baselines` existed in the index and the schema, and no phase used either. `occurred_at_is_uncertain` appeared in zero phase files |
| Five off-by-two collector citations corrected | N-1 cluster | P09 cited §11 for the process collector (§14), P11 cited §14 for services (§16), P12 cited §16 for Windows Update (§18), P10 cited §12 for the file collector (§13), P13 cited §18/§20 for network/startup (§20/§21). Each wrong number created a false owner *and* left the real requirement unowned — Y-6, five more times |
| Four §50 citations corrected to §33/§49 | N-1 cluster | Phases 13, 14, 15 and 19 cited §50 (Onboarding) while describing the privacy screen and the delete-all control. Identical failure mode to Y-6's "§48 privacy screen", different wrong number |
| Nine uncited-but-covered requirements given citations | coverage | §2 and §71 → P07, §8 → P05, §38 → P16, §67/§79/§86/§88/§89 → P01, §76–§78 named in P21's deferral row |
| Decision ids de-collided; `docs/DECISIONS.md` now allocates them | N-2 | Five phase files claimed D-03 through D-07, all of which measurement had already assigned to something else |
| P20 taught about phase 17 | N-6 | It depended on 17 but its gates named only the Change Map. Now includes the System Map frame budget and the §48 prominence test |

**Requirement ownership is 91 of 91** — up from 77 with a phase owner, 2 cited only in `plan.md`,
12 cited nowhere and 1 (§48) with no implementing phase.

### Current unresolved questions (2026-09-10)

**None.** Validation selected MIT, a contextual Change Map (Investigation/Timeline, not sidebar), and a
single approval covering one frozen publication manifest. Any changed artifact, hash, destination or
copy requires a new approval.

### Previously settled questions

The four questions raised in the 2026-09-03 amendment were answered and remain binding.

**Settled by the user, 2026-09-03**

1. **§37's <80 MB: keep both figures, no single pass/fail target.** Measured on the
   P01 release scaffold — an empty app with no database, no collectors and no
   Three.js: 361.7 MB total working set, 77.1 MB private (app-attributable) working
   set at idle, six WebView2 processes accounting for ~92% of the total. The user's
   decision: P20 records **both** at every checkpoint against the 80 MB reference
   line, and neither is a gate. The plan states the consequence plainly rather than
   hiding it — a budget that cannot fail is a measurement, not a budget — so the
   trend rows in `docs/PERFORMANCE.md` are what a future scope conversation will
   stand on. Recorded as D-15; applied in P20 and G6.
2. **Repo folder stays `.WhatChage`.** Cosmetic; nothing depends on it, by rule.
3. **Docker ships untested on this machine.** The absent-pipe path is verified, the
   protocol parser is fixture-tested, and `COLLECTORS.md` marks the live path
   "untested on this machine". No install.
4. **Both installer names.** P21 builds with Tauri's `What Changed_<version>_x64-setup.exe`
   and copies to `dist-installer/WhatChanged-Setup-x64.exe`, recording both with
   SHA-256 in the release audit. The copy is the published download.

**Not blocking, but worth knowing:** phases 01–05 have **no completed depth red-team
report**. `reports/red-team-p01-05.md` is a 126-byte stub, and the three follow-up
review slices never ran — every one of the nine agents delegated this session died at
spawn with `API Error: 400 Model "claude-opus-4.8" does not exist or is disabled`.
The API expects `claude-opus-4-8`; the dotted form does not appear in any agent
definition or settings file under `~/.claude/`, so it was supplied at spawn time
by the runtime rather than by a file this repo controls. Pass an explicit model
when delegating the review again. Every requirement
those phases carry has a named owner and a subject-checked citation (the second
consistency gate proves that), but nobody has adversarially read them for technical
depth the way phases 06–21 were read. Phase 01 is largely executed on disk and
verified by a green build, which covers it in practice; 02–05 are not yet written.

**Settled by measurement during this session**

- **Rust and the toolchain.** `rustup` 1.29.1 / `rustc` 1.98.0 installed per-user
  and pinned; MSVC 14.50.35717 selected over a 2017-era 14.16 (D-01). Blocker B1 is
  closed: `bun run verify` and `bun run tauri build` both pass, producing a 3.11 MB
  `WhatChanged.exe` and a 1.12 MB NSIS installer.
- **Git identity** is configured repo-locally. No longer a question for P01.
- **TypeScript 7.0.2 fails** — `svelte-check` 4.7.6 refuses to start without TS 6
  *and* 7 plus a `--tsgo` flag. Pinned `5.9.3`; `bun run check` is green (D-02).
- **Vite 8 minifies with Oxc, not esbuild.** `minify: 'esbuild'` fails with
  `Cannot find package 'esbuild'` because Vite 8 made it an optional peer. Switched
  to `'oxc'`.
- **`reqwest`/`hyper` are in `Cargo.lock` but not in the Windows build graph** —
  Tauri depends on them for non-Windows targets. P21's no-network gate now resolves
  `cargo tree --target x86_64-pc-windows-msvc` instead of grepping the lockfile;
  as specified it would have failed on a clean scaffold forever.
- **FTS5 is present in the pinned bundled SQLite.** Probed against
  `rusqlite 0.40.2 features = ["bundled"]`: SQLite 3.53.2, `ENABLE_FTS5` in
  `compile_options`, and numeric (`3000`), prefix (`port*`) and quoted-phrase
  matching all work against an external-content table. So P04's migration `002`
  ships one body instead of branching at runtime (D-11). Incidental: `ENABLE_JSON1`
  is absent from `compile_options` yet `json_extract`/`json_each` work — JSON has
  been compiled in by default since 3.38, so asserting that option would fail on a
  build that fully supports it.

<!-- slug: what-changed-desktop-mvp -->
