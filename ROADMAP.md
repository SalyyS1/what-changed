# Roadmap

Source of truth: the accepted plan at
[plans/260829-0437-what-changed-desktop-mvp/plan.md](plans/260829-0437-what-changed-desktop-mvp/plan.md).
Twenty-five phases. The original 21-phase MVP plan was amended on 2026-09-03 after an adversarial review
and a whole-plan consistency gate; phase 17 (System Map) was added and later phases renumbered. On
2026-09-10 a production-readiness review amended owning phases and extended the same accepted plan with
phases 22–25 for brand, English canonical wiki, bilingual launch media and public unsigned-v1 release.
Effort and dependency ownership are in the accepted plan and phase files; if this navigation table
disagrees with them, those files win.

Statuses here mirror the `status:` field in each phase file. If they disagree, the phase file wins.

## Phases

| # | Title | Priority | Effort | Depends on | Status |
|---|---|---|---|---|---|
| 01 | [Foundation & Toolchain](plans/260829-0437-what-changed-desktop-mvp/phase-01-foundation-toolchain.md) | P1 | 5h | — | completed |
| 02 | [Design System & App Shell](plans/260829-0437-what-changed-desktop-mvp/phase-02-design-system-app-shell.md) | P1 | 6h | 01 | completed |
| 03 | [Redaction & Privacy Chokepoint](plans/260829-0437-what-changed-desktop-mvp/phase-03-redaction-privacy-chokepoint.md) | P1 | 4h | 01 | completed |
| 04 | [Domain Model & Persistence](plans/260829-0437-what-changed-desktop-mvp/phase-04-domain-model-persistence.md) | P1 | 8h | 03 | completed |
| 05 | [Typed IPC, Tray & Lifecycle](plans/260829-0437-what-changed-desktop-mvp/phase-05-typed-ipc-tray-lifecycle.md) | P1 | 6h | 02, 04 | completed |
| 06 | [Event Engine & Queries](plans/260829-0437-what-changed-desktop-mvp/phase-06-event-engine-queries.md) | P1 | 6h | 05 | completed |
| 07 | [Timeline & Search UI](plans/260829-0437-what-changed-desktop-mvp/phase-07-timeline-search-ui.md) | P1 | 8h | 06 | completed |
| 08 | [Collector runtime: trait, supervisor, scheduler, failure isolation, settings](plans/260829-0437-what-changed-desktop-mvp/phase-08-collector-runtime.md) | P1 | 5h | 05 | completed |
| 09 | [Process and network-listener collectors (diff-polled, path-degrading)](plans/260829-0437-what-changed-desktop-mvp/phase-09-process-port-collectors.md) | P1 | 6h | 08 | in-progress |
| 10 | [Watched-folder file collector with structured config diffing](plans/260829-0437-what-changed-desktop-mvp/phase-10-file-config-collector.md) | P1 | 8h | 08 | completed |
| 11 | [Windows service collector and installed-application collector](plans/260829-0437-what-changed-desktop-mvp/phase-11-service-application-collectors.md) | P1 | 6h | 08 | completed |
| 12 | [Windows Update collector (WUA COM) and driver collector (notify + rare sweep)](plans/260829-0437-what-changed-desktop-mvp/phase-12-update-driver-collectors.md) | P1 | 7h | 09, 11 | completed |
| 13 | [Network config, startup/scheduled-task, and Windows event-log collectors](plans/260829-0437-what-changed-desktop-mvp/phase-13-network-startup-eventlog-collectors.md) | P1 | 8h | 12 | completed |
| 14 | [Checkpoints: known-good snapshots and diff-since-checkpoint](plans/260829-0437-what-changed-desktop-mvp/phase-14-checkpoints.md) | P1 | 5h | 13 | completed |
| 15 | [Deterministic analysis engine: correlation, causal ranking, investigation flow](plans/260829-0437-what-changed-desktop-mvp/phase-15-analysis-engine-investigation.md) | P1 | 12h | 06, 07, 10, 13, 14 | completed |
| 16 | [Change Map: purposeful 3D causal/temporal visualization with 2D fallback](plans/260829-0437-what-changed-desktop-mvp/phase-16-three-js-visualization.md) | P2 | 9h | 15 | completed |
| 17 | [System Map: §48 relationship graph and dependency inspection](plans/260829-0437-what-changed-desktop-mvp/phase-17-system-map.md) | P2 | 7h | 16 | completed |
| 18 | [Docker collector and developer-environment collector (dev mode)](plans/260829-0437-what-changed-desktop-mvp/phase-18-docker-development-mode.md) | P2 | 6h | 08, 10 | completed |
| 19 | [First-run onboarding, restrained notifications, privacy screen, diagnostics](plans/260829-0437-what-changed-desktop-mvp/phase-19-onboarding-notifications-diagnostics.md) | P1 | 6h | 15, 18 | in-progress |
| 20 | [Performance at 10k/100k/1M events, resource budgets, accessibility conformance](plans/260829-0437-what-changed-desktop-mvp/phase-20-performance-accessibility-hardening.md) | P1 | 8h | 16, 17, 18, 19 | in-progress |
| 21 | [NSIS installer, in-place upgrade path, release audit and requirement traceability](plans/260829-0437-what-changed-desktop-mvp/phase-21-installer-updater-release.md) | P1 | 7h | 20 | in-progress |
| 22 | [Brand identity and asset system](plans/260829-0437-what-changed-desktop-mvp/phase-22-brand-identity-and-asset-system.md) | P2 | 8h | 07, 16, 17, 19, 21 | in-progress |
| 23 | [Wiki and documentation completion](plans/260829-0437-what-changed-desktop-mvp/phase-23-wiki-and-documentation-completion.md) | P1 | 10h | 21, 22 | in-progress |
| 24 | [Bilingual launch story and media kit](plans/260829-0437-what-changed-desktop-mvp/phase-24-bilingual-launch-story-and-media-kit.md) | P2 | 10h | 22, 23 | in-progress |
| 25 | [Public v1 readiness and launch](plans/260829-0437-what-changed-desktop-mvp/phase-25-public-v1-readiness-and-launch.md) | P1 | 12h | 21, 22, 23, 24 | pending |

Current source is not release-ready. Phase 09's real-OS process p95 gate is open. Phase 20's
final-corpus eight-shape release harness passes current-source 10k and 100k. The v15 1M
matrix passed 8/8 once (investigation 1,151.732/1,500 ms p95), then a profiler-free rerun
under higher host load passed 7/8 (investigation 1,758.975/1,500 ms p95). The current v15 1M
canonical size/full-compaction gate passes at 795.39 MB/1.1 GB, while the separate 10k
collector-payload sensitivity corpus misses the strict 12 MB budget by 42,240 bytes; neither
result closes the current all-scale gate; observed 1M seed time is 270–341/240 seconds.
Routine reclaim has been measured at 10k, and the dev-source 100k Timeline scroll harness passes,
but installed resource, soak, map-frame and accessibility gates remain open. The App-level System
Map route-prominence check is closed. [Migration 014](src-tauri/migrations/014_daily_importance_first_day_boundary.sql)
repairs the first representable UTC-day boundary and [migration 015](src-tauri/migrations/015_event_category_counts.sql)
adds category counts; the preserved NSIS pair predates both and is not a current-source release
candidate. The [public-v1 packet](docs/public-v1-go-no-go.md) remains **NO-GO**. Dated measurements
and remaining criteria live in the linked phase files, not this table.

## How the phases stage

| Stage | Phases | What exists at the end |
|---|---|---|
| Foundation | 01–05 | A real application: it boots, opens a database, hides to tray, and the frontend talks to Rust through typed wrappers only |
| Data and UI on synthetic events | 06–07 | A finished, fast timeline, details drawer, and search — before any collector exists |
| Collection | 08–13, 18 | The twelve collectors, behind a supervisor that isolates their failures |
| Reasoning | 14–15 | Checkpoints and the ranking engine |
| Visualization | 16–17 | The Change Map (causal cascades) and the System Map (what depends on what) |
| Trust and release candidate | 19–21 | Onboarding, notifications, privacy and diagnostics screens, then enforced budgets and the installer |
| Public product system | 22–25 | Tested brand assets, English canonical wiki, bilingual launch package, then evidence-backed public-v1 go/no-go |

Collectors are deliberately built *after* the UI is finished on synthetic data. A collector written against a
half-built screen produces events shaped for the collector's convenience rather than for a person reading them.

## Implemented event-query surface

Phase 06 replaced the four event stubs with bounded Rust services behind the generated TypeScript contract:

- `get_events` accepts `EventQuery`: one of `Today`, `Yesterday`, `Last7Days`, `SinceLastRestart` or a
  validated custom UTC millisecond range, optional category/importance/entity filters, an opaque URL-safe
  base64 keyset cursor, and a server-clamped page size of 1–200. Pages are newest first.
- More than four consecutive events sharing category, coarse entity and a 60-second bucket become one
  `TimelineItem::Cluster`. Its backend-issued `ClusterRef` includes exact timestamp/id boundaries;
  `expand_cluster` validates the reference and returns only those members.
- `search_events` searches indexed event text and checkpoint labels, caps input at 128 characters and results
  at 100, and returns a tagged `event` or `checkpoint` result so callers never infer the kind from nullable
  fields. Numeric terms also match port identifiers and metadata.
- `get_dashboard_summary` returns range counts, the calm/attention status sentence, the three top important
  events, current monitoring state and the newest checkpoint reference. `get_event_details` adds nearby
  related events and the number of prior occurrences.

The debug developer panel can seed six deterministic corpus profiles, optionally with entity relationships,
and simulate driver, port, process, DNS, Docker or config events through the real sanitized writer. Synthetic
commands are registered only in debug builds or builds explicitly carrying `devtools`, and both still refuse
requests unless `developer_mode` is enabled. Normal release handlers contain neither command.

## Implemented event-exploration UI

Phase 07 turns that query surface into the first complete user workflow:

- Home renders the range summary, important changes and exact calm/attention copy through an `AsyncState`.
  A database state other than `ready`, or a failed status request in a real Tauri window, fails closed to
  `Recovery needed`; ordinary history and an `Active` claim are hidden until diagnostics can be opened.
- Timeline combines deep-linkable ranges, category/importance/entity filters, local-hour groups, clusters
  and live notices. TanStack Svelte Virtual renders fixed 72px event rows with eight-row overscan, while the
  store retains at most three 200-row pages and refetches evicted pages in either direction. Refreshing page
  zero replaces its continuation chain atomically, and cluster identity includes the full backend bounds.
- A `system-event-added` notice refreshes the first page only while the user is at the top of a live range;
  otherwise it increments a `N new changes` pill without moving the viewport.
- The details drawer shows the already-redacted before/after delta, timestamp, importance, confidence,
  occurrences, related events and collector, with raw technical fields behind a remembered disclosure.
  `Esc` closes it and focus returns to the invoking row.
- The `Ctrl+K` palette debounces for 120 ms, groups typed event/checkpoint results, supports arrow keys and
  `Enter`, routes checkpoints, opens ordinary event details, and deep-links port hits to an entity-filtered
  Timeline. Home, Timeline and Search have loading, empty, error, content, large-data and 900×600 fixtures.

The Phase 07 gate passed 156 frontend tests and final review at 9.6/10 with no critical findings. A native
Tauri/WebView smoke for real backend search, live events and focus behavior remains an environment check;
measured 100k-row scroll FPS remains owned by Phase 20. Neither deferred item is presented as measured here.

## Collector-runtime acceptance completed

Phase 08's current implementation and its design adjustments are routed from
[the architecture boundary](ARCHITECTURE.md#collector-runtime). The executable acceptance owners are
the [runtime](src-tauri/tests/collector_runtime.rs),
[isolation](src-tauri/tests/collector_isolation.rs),
[targeted-wake](src-tauri/tests/collector_targeted_wake.rs),
[atomic runtime](src-tauri/tests/collector_atomic_runtime.rs),
[atomic storage](src-tauri/tests/collector_commit.rs) and
[registry composition](src-tauri/tests/collector_registry_composition.rs) suites.
The [release panic probe](src-tauri/examples/collector_panic_probe.rs) passed as an optimized non-test
binary: the panicking worker was contained and its healthy neighbour continued. Final review scored
9.6/10 with all four findings resolved and no blocking findings. Rust baseline: 315 passed; subsequent
corrected-tree checks: 26 changed-path and 35 existing storage regressions passed (overlapping counts,
not extra unique tests). Frontend: 191 tests / 22 files, check 0 errors / 0 warnings, build 770 modules;
fmt, strict Clippy and generated-binding parity passed.

Release 10k query p95: page 1.473 ms, summary 1.764 ms, search 8.094 ms. The
[debug idle probe](src-tauri/examples/collector_idle_probe.rs) passed a 62.324830 s sample with two
fake 1 s collectors: measured CPU 0.000000% at Windows counter resolution, peak working set 8.33 MiB.
This is runtime/Db overhead, not the desktop app's total/private memory. Exact metrics and reproducers:
[Performance](docs/PERFORMANCE.md#phase-08-runtime-overhead--2026-09-13).

The Phase 08 numbers above are a dated runtime measurement, not a whole-app resource pass. Phase 05's
local NSIS build and full-app total/private memory readings are historical evidence; neither memory
figure is a pass/fail gate under D-15, and neither approves the current source for release.
The [startup composition](src-tauri/src/lib.rs) and
[canonical registry](src-tauri/src/collectors/registry.rs) remain the activation authority.

## Process/listener evidence and open gate

Phase 09's executable owners are the [process](src-tauri/tests/collector_process.rs),
[listener](src-tauri/tests/collector_listeners.rs),
[relationship](src-tauri/tests/relationships_process_port.rs) and
[real-cycle performance](src-tauri/tests/collector_performance.rs) suites. The earlier passing
50-cycle timing and development-profile smoke remain dated evidence in the
[phase record](plans/260829-0437-what-changed-desktop-mvp/phase-09-process-port-collectors.md),
not proof for the current source. The unchanged process ≤15 ms p95 gate failed on a saturated
host after collector changes; listener timings remained below budget. The process gate stays open
until a sustained low-load run passes, with no silent budget relaxation.

## Service/application acceptance completed

Phase 11 activates non-elevated Win32 service and three-hive uninstall-registry collectors in the
production registry. Scripted gates cover exact service/application diffs, no-storm baselines,
downtime uncertainty, deepest-root ownership, config-denial recovery, bounded atomic relationships,
and notification fallback. Shared session evidence writes `application --provides--> service` and
`file --config_of--> service` without persisting raw installation paths.

On the acceptance workstation, 20 steady real-OS sweeps measured service p95 6.519 ms and registry
p95 31.9952 ms. The owned 15-minute full service-configuration reconciliation measured 173.0581 ms.
Phase 11 enumerates the 301 Win32 services; Phase 12 owns the 443 driver rows rather than combining all
744 rows into the five-second service path.

## Update/driver acceptance completed

Phase 12 activates an STA-owned WUA history provider and a SetupAPI driver provider. Both persist an
atomic baseline, so the initial history is silent and restarts do not replay it. Driver wakes degrade
from Configuration Manager notification to registry notification to a six-hour timer, and the selected
tier is visible in diagnostics. Resume detection uses wall time minus `QueryUnbiasedInterruptTime` and
wakes only these two sources through the runtime's lifecycle fences.

On the acceptance workstation, WUA returned 18 rows at 2.7221 ms p95, SetupAPI returned 217 devices at
376.8582 ms p95 over five sweeps, and concurrent timeline reads stayed at 1.2294 ms p95. A forced driver
stall was abandoned after 10.0045373 s without overlap and the collector recovered on its next sweep.

## Checkpoint acceptance completed

Phase 14 persists named, sanitized known-good state through the
[checkpoint service](src-tauri/src/checkpoint/service.rs) and
[checkpoint repository](src-tauri/src/storage/checkpoint_repository.rs). Creation asks the existing
collector workers for durable acknowledgements; it does not open a second collection path. Per-collector
semantic adapters own comparison rules, and compare-to-now reads the latest acknowledged state without
forcing another sweep. Incomplete or truncated collector state is excluded from both sides and reported
as missing instead of becoming a false removal.

Manual, tray, weekly, installer and Windows Update entry points converge on the same service. Duplicate
identical requests share one flight; distinct automatic work queues behind active work. Shutdown closes
admission, stops collectors to wake captures, joins automation, waits for checkpoint callers and only then
flushes storage. The executable acceptance owners are the
[creation](src-tauri/tests/checkpoint_create.rs), [comparison](src-tauri/tests/checkpoint_compare.rs),
[automation](src-tauri/tests/checkpoint_auto.rs), [leak](src-tauri/tests/checkpoint_leak.rs) and
[real capture](src-tauri/tests/checkpoint_real.rs) suites. The final combined gate passed 670 Rust tests
with 14 ignored and 211 frontend tests; the 3,000-item comparison budget passed its asserted p95 bound.

## Investigation acceptance completed

Phase 15 replaces the investigation placeholders with six typed commands:
`start_investigation`, `get_investigation`, `list_investigations`, `delete_investigation`,
`analyze_window`, and `get_related_events`. Their executable owners are the
[command boundary](src-tauri/src/commands/analysis.rs),
[service orchestration](src-tauri/src/services/investigation_service.rs),
[pure engine](src-tauri/src/analysis/mod.rs), and
[investigation store](src/lib/stores/investigation.svelte.ts).

The accepted limits are owned by those sources and their focused suites: at most 5,000 events are
analysed, at most 20 suspects are returned, and structural chains are bounded to depth four and 200
paths. Confidence derives from recorded evidence rather than score magnitude, and the generated-copy
suite prevents non-confirmed results from claiming a cause. The 5,000-event ≤400 ms and graph-rich
100,000-row database ≤1.5 s p95 gates passed; the passing output retained no narrower figures.

Investigation input is read under one consistent SQLite transaction. Completed histories are sanitized,
immutable presentation snapshots retained for 90 days, so reopening does not depend on shorter-lived
source events. The frontend keeps the unsent problem draft in memory only. Incident-neighbour lookup uses
the bounded event window and recorded relationships without materializing the full pairwise graph.

## Change Map acceptance completed

Phase 16 adds one contextual view reached from the current Timeline query or an Investigation; it does not
add an eighth navigation destination. The authoritative entry points are the
[payload shaper](src-tauri/src/analysis/change_map.rs),
[typed command](src-tauri/src/commands/change_map.rs),
[feature boundary](src/components/visualization/ChangeMapFrame.svelte), and
[framework-free scene](src/three/changeMap/createScene.ts). The same Rust payload supplies the Three.js
scene and the 2D fallback, with hard backend caps of 3,000 nodes and 6,000 edges.

Automatic mode chooses the 2D surface for reduced-motion users or when WebGL is unavailable or lost.
The accessible node navigator exposes one listbox tab stop and a bounded 41-option DOM window while retaining
the full logical set through position metadata. Scene teardown, hidden-window pausing, on-demand reduced-motion
rendering, renderer fallback and contextual route preservation have executable coverage beside their owners.

The shipped performance harness at `#/dev/perf` rendered the 3,000-node, forced-low-tier fixture for 300
frame intervals in headful Chrome on Intel Iris Xe through Direct3D11 at 16.70 ms p50 / 16.80 ms p95
(about 59.9 FPS, the display's single-frame cadence). The browser exposed 41 node options and reported no
page errors; the measurement context and limits are retained in
[Performance](docs/PERFORMANCE.md#change-map-rendering--2026-09-20).

## System Map acceptance completed

Phase 17 closes §48 without inventing relationships to make a sparse graph look complete. The storage
boundary owns the bounded bidirectional walk and exact disclosure count; the frontend shares Phase 16's
renderer capabilities while preserving a distinct non-temporal dependency view. Entry points are the
[repository](src-tauri/src/storage/system_map_repository.rs),
[payload contract](src-tauri/src/analysis/system_map.rs),
[feature frame](src/components/system/SystemMapFrame.svelte), and
[System Map renderer](src/three/systemMap/).

The dense fixture returned an exact 50,001-node connected count from 50,000 relationship rows across a
20-query gate at 74.254 ms first/max (67.745 ms median, 73.951 ms p95), with both relationship indexes
asserted by query plan. Headful Chrome on Intel Iris Xe /
Direct3D11 rendered the capped 400-node fixture at 16.70 ms p50 / 16.90 ms p95, exposed 41 accessible
options, kept the dependency panel beside the map, and reported no page errors. Scope and the unrelated
Phase 09 full-suite timing failure are retained in
[Performance](docs/PERFORMANCE.md#system-map-query-and-rendering--2026-09-20).

## Current interface boundaries

The watched-location commands are implemented in the
[settings command owner](src-tauri/src/commands/settings.rs), not stubs. The typed command list and
contract checks are owned by [command registration](src-tauri/src/commands/mod.rs) and
[its test](src-tauri/tests/commands_contract.rs).

Collector health, settings and live Diagnostics are no longer placeholders for the runtime. Their
owners are [collector commands](src-tauri/src/commands/collectors.rs),
[settings coordination](src-tauri/src/app/state.rs),
[Diagnostics](src-tauri/src/commands/diagnostics.rs) and the
[generated DTO contract](src-tauri/src/dto/status_dto.rs). An unavailable platform source must not be
presented as idle monitoring. Row-loss counters and atomic-cycle-loss counters have separate units;
neither should be inferred from a successful snapshot count.

The current `os_version` DTO field is populated from WebView's version, and the
[screen](src/features/diagnostics/DiagnosticsScreen.svelte) labels it `WebView runtime`; an actual
Windows-version field remains a diagnostics gap. The [updater command](src-tauri/src/commands/meta.rs)
returns an unsupported, no-I/O answer until a signed feed and public key exist.

`analysis-completed` is now emitted after a persisted investigation commits; the frontend listener
reconciles that id through the authoritative get command.

## Tracked stubs

None currently. Every registered command has a real implementation or is recorded in the deliberate
deferrals below; the wiring test keeps this section synchronized when that changes.

[Runtime event delivery](src-tauri/src/lib.rs), [frontend listeners](src/lib/tauri/events.ts),
[collector reconciliation](src/lib/stores/collectors.svelte.ts) and
[Diagnostics reconciliation](src/lib/stores/diagnostics.svelte.ts) own the live status path. The
[wiring suite](src-tauri/tests/wiring.rs) owns cross-boundary event-name parity.

## Deliberately omitted or deferred

Recorded so the omissions are choices with reasons rather than gaps. A scaffold can exist without
making the capability available; any release input still belongs to its owning phase.

| Deferred | Why now, and what it waits on |
|---|---|
| **Health checks** (§26 — HTTP/TCP/process/service probes, HEALTHY→UNHEALTHY transitions) | The analysis engine already answers "what changed before this moment"; health checks make the moment automatic. The `health_states` table is created in P04 and left unused so the feature does not need a migration later |
| **Ambient event field** (§39's first Three.js suggestion) | §39 phrases it as a recommendation. One purposeful visualization beats three decorative ones — the Change Map earns its third axis with chain depth; a particle field would not. Recorded against goal G7 |
| **Fix Memory** (future §75–§78 bundle — "you have encountered something similar before") | Needs a corpus of resolved investigations to be worth anything, and it must not guess. Investigations are persisted from P15, so the data accumulates from day one |
| **Impact / blast radius** (future §75–§78 bundle — "what breaks if I change this?") | The relationship graph P17 renders is exactly the data this needs; the answer requires confidence in the graph's completeness that only real use will establish |
| **Can I Delete This?** (future §75–§78 bundle) | Same dependency as Impact, plus a much higher cost of being wrong — a false "safe to delete" is unrecoverable |
| **Background update checks** (part of §64) | D-09 forbids silent checks. The explicit opt-in scaffold belongs to P21, but its signed HTTPS feed and public key are unavailable; no network check is claimed in the current build |
| **Rollback / universal undo** | Recording a change is not the same as being able to reverse it safely. This product's contract is observation, and mixing in mutation would change what a user is trusting it with |
| **Cloud, teams, remote machines, mobile companion, browser extension** | All require an account and a network, both of which the product is defined by not having |
| **Linux and macOS** | The platform layer is separated so collectors can be added, but §6 says Windows first and a half-working second platform is worse than an honest one |
| **LLM agent / automatic repairs** | The deterministic engine must be useful alone. An optional explanation provider is architected in P15; letting a model act on the system is a different product |

## Known observation limits

Not deferrals — limits of what a non-elevated Windows app can observe. The measured gaps are
recorded here and in the linked collector/security authorities; no public release is claimed.

- Processes that start *and* exit inside one poll interval are invisible: push-based
  process events (`Win32_ProcessStartTrace`) require Administrator, and the WMI
  polling alternative delivered zero events in a measured 3-second window.
- The `Security` event-log channel is unreadable without elevation, so no
  security-audit signal is collected.
- About 46% of running processes deny their executable path to a non-elevated
  reader; those events degrade to name and PID rather than guessing.
- Docker's connected path is untested on the development machine (Docker is not
  installed); only the absent path is verified.
- Changes made while the app is not running are detected on next start, with an
  uncertain occurrence time — the app says so rather than inventing a timestamp.
