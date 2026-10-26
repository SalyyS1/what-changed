---
phase: 20
title: "Performance at 10k/100k/1M events, resource budgets, accessibility conformance"
status: in-progress
priority: P1
effort: 8h
dependencies: [16, 17, 18, 19]
---

## Overview

Turn the standing budgets into enforced gates (§61, §62, §63). Three seeded corpora (10k, 100k, 1M
events) drive query, memory, and DB-size assertions; a soak run proves the collectors do not leak over
hours; and a full accessibility pass makes every screen keyboard-operable and screen-reader-sane. This
phase adds no features — it either passes, or it produces a specific, measured optimization commit.

## Requirements

**Functional (as gates)**
- Seeder: `cargo run --bin seed_history --features devtools -- --events 1000000 --days 90 --data-dir <fresh-reserved-QA-dir>` builds a
  realistic corpus (category mix matching the P06 calmness distribution, 8% clustered bursts, 3000
  entities, 15% with relationships, 40 checkpoints, 20 investigations) writing through the real
  `Sanitized` path and tagged `source_collector = "synthetic"` so it can be purged in one statement.
  Deterministic with a seed so numbers are comparable between runs.
- §61 query gates at each size (p95 over 30 runs, warm, asserted in `tests/perf_queries.rs` which is
  `#[ignore]`d by default and run explicitly in this phase's gate):

  | Query | 10k | 100k | 1M |
  |---|---|---|---|
  | `get_events` first page (200 rows) | ≤ 10 ms | ≤ 20 ms | ≤ 45 ms |
  | `get_events` page 10 (keyset) | ≤ 10 ms | ≤ 20 ms | ≤ 45 ms |
  | `get_dashboard_summary` | ≤ 15 ms | ≤ 30 ms | ≤ 80 ms |
  | `search_events` (2-token) | ≤ 40 ms | ≤ 90 ms | ≤ 250 ms |
  | `get_event_details` + prior occurrences | ≤ 10 ms | ≤ 15 ms | ≤ 30 ms |
  | `start_investigation` (5k window) | ≤ 400 ms | ≤ 600 ms | ≤ 1500 ms |
  | `compare_checkpoint` (3k items) | ≤ 150 ms | ≤ 200 ms | ≤ 300 ms |
  | `get_privacy_summary` | ≤ 30 ms | ≤ 60 ms | ≤ 200 ms |

- Every query in the table has an `EXPLAIN QUERY PLAN` assertion proving it uses an index and never
  scans `events` (`tests/perf_query_plans.rs`); a plan containing `SCAN TABLE events` fails the test.
  A failing budget is fixed by an index or a query change, never by relaxing the number without the
  user's decision.
- DB-size gates: ≤ 12 MB at 10k, ≤ 110 MB at 100k, ≤ 1.1 GB at 1M (≈ 1.1 KB/event), plus a
  `page_count`/`freelist_count` check after retention purge proving routine incremental reclaim and the
  explicit full-compaction action reclaim interior pages.
  If 1M exceeds the budget, the response is column-level: drop redundant text from `details`, not a
  schema redesign.
- Resource gates measured with all collectors running:
  - Idle CPU ≤ 1% average over 10 minutes (sampled from `GetProcessTimes` in a `#[ignore]`d test binary
    and cross-checked once manually in Task Manager).
  - **Memory (§37).** Two figures at each of the three B2 checkpoints (window shown, window hidden,
    after 4 hours), never one: **total working set** across the main process and every WebView2 child
    (enumerated by parent pid), and **private working set** — the app-attributable figure, read from
    `Win32_PerfRawData_PerfProc_Process.WorkingSetPrivate`, which excludes pages shared with other
    processes using the same WebView2 runtime. Both are compared to §37's 80 MB target and both are
    reported. Measured on the P01 release scaffold, before any application code existed:
    **361.7 MB total, 77.1 MB private**, with six WebView2 processes accounting for ~92% of the total.
    So the two figures land on opposite sides of the target. **The user decided (D-15): report both at
    every checkpoint, gate on neither.** 80 MB stays as the reference line both are compared against,
    the WebView2 share is broken out, and the phase file records the numbers with no pass/fail verdict
    attached — the honest consequence, stated rather than hidden, is that memory is a measurement in
    this release and not a budget, so the trend across P01 → P05 → P20 is what any later scope
    conversation stands on. The levers that would move total are all WebView2-level (fewer renderer
    processes, single-process flags, disabling GPU acceleration), each trading stability or rendering
    quality; none is applied in this phase without a separate decision.
  - Cold start to visible window ≤ 2.0 s; warm start ≤ 1.2 s; tray-only autostart ≤ 900 ms to tray icon.
  - 4-hour soak (`scripts/soak.ps1` driving `simulate_event` plus real collectors): working set growth
    ≤ 15%, handle count flat ±5%, thread count flat, no `Failed` collectors, zero dropped write batches.

- Frontend gates: 60 FPS scroll on a 100k timeline (measured with a `#/dev/perf` harness recording
  `requestAnimationFrame` deltas over 600 frames; p95 ≤ 20 ms), Change Map median frame ≤ 16.6 ms at
  3000 nodes, **System Map median frame ≤ 16.6 ms at its 400-node cap** (P17, same harness and the same
  shared `qualityTier`), initial JS bundle ≤ 400 KB gzipped excluding `three`, `three` lazily imported
  so the Timeline route never pays for it (`bun run build` + a bundle-size assertion script).
- §48 prominence constraint, asserted rather than trusted: the System Map must not sit on any required
  path for a non-technical user. The test walks the Home, Timeline and Investigations route trees and
  fails if reaching a primary action (investigate, view details, create checkpoint) requires the
  `#/system-map` route. This is what keeps §48's "must not be necessary for normal users" true after
  P17 ships, when the temptation to link to a finished screen is highest.
- §62 accessibility conformance:
  - Keyboard: every interactive element reachable in a logical order; visible focus ring on all of them
    (P02 token); `Esc` closes every overlay; `Ctrl+K` search; arrow-key navigation in the timeline list
    and the search results; no keyboard trap (verified per screen).
  - Screen reader: one `<h1>` per screen, landmark regions (`banner`/`navigation`/`main`), `aria-live="polite"`
    for the live-events pill and toast, `aria-live="assertive"` reserved for the degraded banner only,
    accessible names on all icon-only buttons, table semantics for the before/after and collector-status
    tables. Verified manually with Narrator on Home, Timeline, Details drawer, Investigate, Checkpoints,
    Privacy, Diagnostics, and Settings — findings recorded per screen in the phase file.
  - Contrast: automated WCAG AA check over every token pair used by a component (extends the P02 test to
    walk a manifest of actual pairs, so a new component cannot introduce an untested pair).
  - Motion: `prefers-reduced-motion` removes transitions/animations app-wide (one CSS media block plus
    both maps' on-demand rendering — the shared camera module P17 promotes means one assertion covers
    the Change Map and the System Map) — asserted by a test that computes styles with the media feature
    forced.
  - Zoom/scale: usable at 200% browser zoom and at Windows 150% DPI scaling at 900×600 (no clipped
    controls, no horizontal scroll on the primary column).
- Fixes discovered here land as their own commits in this phase; anything that would change behaviour or
  scope is written up instead of silently applied.

**Non-functional**
- The perf suite must be runnable in one command and must clean up its corpora (`--purge-synthetic`),
  so it can be re-run before release without polluting real history.
- Resource-bound build verification is part of the toolchain contract. On this machine the 2026-09-10
  default-parallel `cargo test --all-features` exhausted rustc/LLVM memory, while the critical suites
  passed with `CARGO_BUILD_JOBS=1`. The canonical verify/release scripts must set a measured safe Cargo
  job count (or equivalent config), prove the full suite from a clean target, and report an OOM as an
  environment failure rather than letting damaged-artifact follow-on errors masquerade as code defects.
- Accessibility release evidence combines automation with an installed-app manual pass: Accessibility
  Insights for Windows FastPass/Live Inspect, keyboard-only primary flows, Narrator name/role/state and
  invocation, Windows high-contrast themes, 150% DPI and 200% text/zoom. Critical failures block release;
  exemptions require owner and expiry.
- Measured numbers are recorded in the phase file next to the budget, with the machine's identity kept
  generic (no serials, no hostnames).

## Architecture

`src-tauri/src/bin/seed_history.rs` (devtools feature), `src-tauri/tests/perf_queries.rs`,
`tests/perf_query_plans.rs`, `tests/perf_db_size.rs`, `src-tauri/src/bin/resource_probe.rs`
(idle CPU / working set / handles sampler), `scripts/soak.ps1`, `scripts/bundle-size.mjs`,
`src/features/dev/PerfHarness.svelte` (`#/dev/perf`: scroll-FPS recorder, map frame timer, route bundle
report). Accessibility work is spread across existing components; the new artifacts are
`src/lib/a11y/contrast-pairs.ts` (the manifest), `src/lib/a11y/contrast.test.ts`,
`src/lib/a11y/reduced-motion.test.ts`, and `docs/accessibility-audit.md` (the per-screen Narrator record).

## Related Code Files

**Create** — `src-tauri/src/bin/{seed_history.rs,resource_probe.rs}`;
`src-tauri/tests/{perf_queries.rs,perf_query_plans.rs,perf_db_size.rs,soak_support.rs}`;
`scripts/{soak.ps1,bundle-size.mjs}`; `src/features/dev/PerfHarness.svelte`;
`src/lib/a11y/{contrast-pairs.ts,contrast.test.ts,reduced-motion.test.ts,focus-order.test.ts}`;
`docs/accessibility-audit.md`; `docs/performance-baseline.md`.
**Modify** — `src-tauri/Cargo.toml` (`devtools` feature, two bins), `src-tauri/migrations/`
(only if an index is added — as `003_perf_indexes.sql` in the one migration directory P04 owns, never
by editing `001_initial.sql`),
`src/styles/motion.css` (reduced-motion block), any component with an a11y finding,
`src/three/shared/renderer.ts` and both scene modules (lazy import boundary for the two map routes),
`src/app/router.ts` (dynamic import for the map route), `package.json` (`test:perf`, `check:bundle`),
`ROADMAP.md`, `docs/system-architecture.md`.
**Delete** — none.

## Implementation Steps

1. `seed_history.rs` with the deterministic generator and `--purge-synthetic`; verify the 10k corpus looks
   like real history in the UI (spot check) before trusting the bigger ones.
2. `perf_query_plans.rs` before `perf_queries.rs` — a plan regression is cheaper to diagnose than a
   timing regression, and it explains any timing failure that follows.
3. Run the 10k gate; fix anything failing; then 100k; then 1M. Record every number in
   `docs/performance-baseline.md` as it is measured, not at the end.
4. Add `003_perf_indexes.sql` only if a plan assertion demands it, with a migration test proving it
   applies to an existing 100k DB in ≤ 5 s.
5. `resource_probe.rs` + the 10-minute idle CPU run + the three working-set readings (main + WebView2
   children) → B2 verdict written into the phase file and `docs/performance-baseline.md`.
6. `scripts/soak.ps1`: 4 hours, samples every 60 s to a CSV, asserts the growth/flatness rules at the end
   and prints a summary. Run it once for real; attach the summary numbers.
7. Frontend: `#/dev/perf` harness, scroll-FPS at 100k, map frames, then `bundle-size.mjs` gate and the
   lazy-import split for `three`.
8. Accessibility sweep, screen by screen: keyboard order and traps, focus rings, landmarks, live regions,
   accessible names, table semantics; then the contrast-pair manifest test and the reduced-motion test;
   then the Narrator pass with findings recorded in `docs/accessibility-audit.md`.
9. Fix commits, each naming the measured before/after.
10. Commits: (a) `feat: deterministic history seeder`, (b) `test: query plan assertions`,
    (c) `test: query performance gates`, (d) `perf: <specific fix>` (as needed),
    (e) `test: db size and vacuum gates`, (f) `feat: resource probe and soak script`,
    (g) `feat: perf harness route`, (h) `perf: lazy load three on the map route`,
    (i) `fix: accessibility findings <screen>` (one per screen with findings),
    (j) `test: contrast manifest and reduced motion`, (k) `docs: performance baseline and a11y audit`.

## Todo (commit boundaries)

- [ ] Deterministic seeder with purge (canonical counts pass at 10k/100k/1M and exact tagged purge/full compaction passes; 3k entity IDs remain unprovable after purge, and the latest canonical 1M seed is 316.439/240 s)
- [ ] Query-plan assertions green (the eight SQL-stage checks and new sorted recurrence covering-index gate pass; final service/corpus parity still needs proof)
- [x] 10k / 100k / 1M query gates green (frozen guarded-recurrence profiler-free matrices pass 8/8 each; 1M investigation 1,032.501/1,500 ms; prior failures retained as historical evidence)
- [ ] DB-size and vacuum gates green (canonical 10k/100k/1M sizes, explicit full compaction and routine incremental retention pass; collector-shaped 10k sensitivity exceeds 12 MB by 42,240 bytes; fresh purge/full-compaction results do not generalize to retention-first fixtures)
- [ ] Idle CPU, working set (B2 verdict), start-time numbers recorded
- [ ] 4-hour soak run completed and summarized
- [ ] Frontend FPS, bundle-size gate, lazy `three`
- [ ] Accessibility sweep + contrast manifest + reduced-motion tests + Narrator record

## Success Criteria

- [x] Every cell of the §61 table passes at its size, or has a recorded, user-approved exception (all 24 cells pass without exceptions; 2026-10-05 guarded-recurrence matrix)
- [ ] `perf_query_plans.rs`: zero `SCAN TABLE events` across all measured queries (eight SQL-stage smoke checks pass, but copied SQL and noncanonical fixture do not yet prove service/corpus parity)
- [ ] DB size within budget at all three sizes on the specified corpus; post-purge `freelist_count` reaches zero after
      routine incremental vacuum plus the explicit full-compaction perf action
- [ ] Idle CPU ≤ 1% over 10 minutes with all collectors enabled
- [ ] B2/§37 (D-15): at each of the three checkpoints **both** the total and the private working set
      are recorded against the 80 MB reference line, along with the WebView2 share. Neither figure is a
      gate. A report that names only one of the two fails this criterion, and so does one that attaches
      a pass/fail verdict the decision did not authorize
- [ ] Cold start ≤ 2.0 s, warm ≤ 1.2 s, tray-only ≤ 900 ms
- [ ] Soak: working set growth ≤ 15%, handles ±5%, threads flat, zero dropped batches, no failed collectors
- [ ] 100k timeline scroll p95 frame ≤ 20 ms; Change Map median ≤ 16.6 ms at 3000 nodes; System Map
      median ≤ 16.6 ms at 400 nodes
- [x] No Home, Timeline or Investigations flow requires the `#/system-map` route (§48 prominence test;
      App-level Home → Investigate, Timeline → Details, Investigations → Create checkpoint tests pass 3/3)
- [x] Initial bundle ≤ 400 KB gzipped excluding `three`; the Timeline route does not load `three`
      (asserted by `bundle-size.mjs`)
- [ ] Keyboard-only traversal of all eight screens with no trap and a visible focus ring throughout
- [x] Contrast manifest test covers every pair used by a component and passes AA
- [x] `prefers-reduced-motion` removes all transitions and stops the map's continuous loop
- [ ] Usable at 900×600 with Windows 150% scaling and at 200% zoom
- [ ] `docs/accessibility-audit.md` records a Narrator result per screen

## Verification Gate

## Verification evidence — 2026-09-22

- Rust gate command used `CARGO_BUILD_JOBS=1` and the ignored Phase 20 targets where available.
- The repeatable release rehearsal now passes its runnable gates: Rust 444 passed / 1 ignored,
  frontend 314 passed across 57 files, typecheck 0 warnings, production build, bundle and docs links.
- 10k: page p95 2.266 ms, summary p95 2.102 ms, search p95 7.286 ms; DB 7,745,536 bytes and freelist 13 → 1,532. Query, plan and size tests passed.
- 100k: page p95 1.796 ms, summary p95 5.416 ms, search p95 74.845 ms; DB 75,767,808 bytes and freelist 181 → 15,647. Query and size tests passed.
- 1M: page p95 4.052 ms, summary p95 347.757 ms, search p95 2,876.329 ms. The asserted 80 ms summary and 250 ms search budgets failed on this machine; no exception was approved. The 1M size gate is therefore still unrun.
- The failed 1M fixture used a workspace-local temporary directory after the system temp volume filled and was removed afterward. No release or user database was touched.
- The seeder and perf tests still cover only the three stable read paths; event details, investigation, checkpoint compare and privacy-summary rows, resource/soak readings, and installed-app accessibility evidence remain open.

## Verification evidence — 2026-10-14 B2 engineering checkpoint

- Current-source release binary, window shown after a 5-second settle: backend working set **48.8 MB**;
  six WebView2 descendants **366.1 MB**; total working set **414.9 MB**; private working set
  **132.7 MB**. The 80 MB reference line is shown for context only under D-15; no memory pass/fail is
  claimed. Hidden-window and four-hour checkpoints, idle CPU, soak and installed-app evidence remain open.
- A hidden-window engineering snapshot in the same session measured backend **50.7 MB**, WebView2
  descendants **359.8 MB**, total **410.5 MB**, private **128.9 MB**. This is the second checkpoint;
  the after-four-hour reading and full resource/soak gate remain open.

## Verification evidence — 2026-09-23 continuation

- The 1M size corpus completed on the workspace volume before the reclaim fix: 749,412,352 database bytes
  against the 1,100,000,000-byte budget; purge deleted 1,000,000 rows but the old 256-page incremental
  vacuum left 155,758 free pages and failed the reclaim assertion.
- At this 2026-09-23 checkpoint `storage::maintenance::reclaim` had been changed to SQLite's
  no-argument incremental vacuum. The 10k size gate then passed (7,745,536 → 7,741,440 bytes;
  freelist 13 → 1,532), but this did not prove routine retention reclaim. That implementation
  was later superseded by the bounded 256-page fix documented below. The 1M post-change size
  rerun remained open then because the earlier run exhausted the system temp volume.
- A valid 1M query run on the D: workspace fixture measured page p95 8.807 ms, summary p95 191.862 ms and
  search p95 1,653.059 ms; page passes, summary/search remain above the 80/250 ms budgets. The temporary
  single-scan conditional aggregation was slower on this schema and has been reverted to the four indexed
  `COUNT(*)` ranges. A fresh 100k harness run with that query measured page p95 4.891 ms, summary p95
  14.354 ms and search p95 232.186 ms: page/summary meet their 20/30 ms budgets, while broad FTS search
  still exceeds the 90 ms budget. The 100k/1M query gate therefore remains open; no exception was approved.

```bash
cd /d/Project/.WhatChage/src-tauri
# Manual seeding requires a fresh, empty what-changed-perf-* or what-changed-seed-*
# directory passed as --data-dir. The automated matrix creates one itself.
cargo clippy --all-targets --all-features -- -D warnings
cd /d/Project/.WhatChage
bun run test:perf  # query plans, eight shapes and DB-size at 10k/100k/1M
bun run check && bun run test && bun run build && bun run check:bundle
# Set $WhatChangedPid and a fail-fast $DriverCommand before the installed-app soak.
powershell -ExecutionPolicy Bypass -File scripts/soak.ps1 -Hours 4 -ProcessId $WhatChangedPid -SimulationCommand $DriverCommand
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| A 1M-event budget is unreachable on this hardware | `perf_queries` fails at 1M only | Diagnose with the query plan first (usually a missing composite index); if the shape is right and it is simply data volume, present the measured number to the user as a scope question. Never relax the assertion silently, and never cap history below the user's chosen retention without their decision |
| Pressure builds to make the memory numbers look better | Total working set far above the reference line while private sits under it — the measured P01 shape — and someone proposes a "memory saver" | Report the split (main process vs WebView2 children) and the three readings. Options presented, not chosen unilaterally: accept a higher target, hide the window aggressively, or defer the UI to a lighter shell. Do not disable collectors to make a number |
| Seeded corpus is unrealistically uniform, so gates pass but real use is slow | Real-machine timeline feels slower than the 100k gate | Seeder mirrors the P06 category distribution and includes burst clusters and entity reuse; a spot-check against this machine's real history (a week of events) is part of step 1 |
| An added index slows writes | Write batch duration rises in the soak | Batch-write timing is part of the soak assertions; an index that costs > 10% write throughput is replaced by a query change instead |
| 1M seeding takes so long it is skipped | The 1M column is never actually run | Seeder writes in batched transactions (target ≤ 4 minutes for 1M); the gate command is in the phase's Verification Gate so skipping it fails the phase |
| Accessibility fixes cause visual regressions | Component tests or the contrast test fail | Each fix is its own commit with the failing/passing test named, so a regression is bisectable |
| Narrator findings are subjective | Disagreement about "sane" output | The record is per-screen and concrete (announced name, role, and state for each control); ambiguous items are listed as follow-ups in `ROADMAP.md` rather than silently passed |

## Security Considerations

- The seeder and probes are behind the `devtools` cargo feature and are not compiled into the release
  binary (asserted by a release-build symbol check in P21).
- `docs/performance-baseline.md` and the soak CSV contain no hostnames, serials, usernames, or paths —
  only counts and timings (the same allowlist discipline as the diagnostics export).
- The perf harness route lives under `#/dev/*`, which is compiled out of release builds along with the
  rest of the dev harness.
- Synthetic events are labelled and purgeable, so a perf run cannot be mistaken for real history in an
  investigation.

## Verification evidence — 2026-09-24 targeted P20 run

- `CARGO_BUILD_JOBS=1 cargo test --locked --test notification_policy --test privacy_purge --test diagnostics_export --test onboarding_state --test capability_allowlist` passed the P19/P20 trust boundary suites; the Phase 20 query-plan and perf fixtures remain opt-in/ignored as documented.
- `bun run check` passed with zero Svelte diagnostics and zero TypeScript errors.
- `bun run test -- src/features/diagnostics/DiagnosticsScreen.test.ts src/lib/a11y/contrast.test.ts src/lib/a11y/reduced-motion.test.ts src/lib/a11y/focus-order.test.ts` passed 12/12 tests. Contrast manifest, reduced-motion CSS/map pause, shared focus ring and landmark invariants are automated green.
- `bun run build && bun run check:bundle` passed. The bundle gate measured 138,236 gzip bytes of non-Three chunks against the 409,600-byte limit; the Three.js chunk remained lazy and excluded from the shell budget.
- The opt-in `perf_query_plans` run passed 1/1 on the current tree; no `SCAN EVENTS` or `SCAN TABLE EVENTS` plan was observed.
- Query-plan and size evidence remains valid for the measured 10k/100k runs. The 1M summary/search budgets and broad 100k search budget remain above target, so no exception is recorded. Idle CPU, B2 total/private working-set readings, cold/warm/tray timings, soak, FPS, installed-app Narrator/DPI/high-contrast pass and route-prominence walk remain open manual or opt-in gates.

## Verification evidence — 2026-09-26 query optimization continuation

- Added the narrow `event_search_order` projection (migration 009) and an importance-token FTS
  projection (migration 010). Broad search now queries importance buckets from critical to
  informational and stops when the bounded result limit is full; the event payload is hydrated only
  for returned ids. Added the daily exact count projection (migrations 012–013), with two indexed edge-day
  scans plus complete interior days, so dashboard counts do not rescan the whole history.
- The migration, lifecycle-trigger, repository search-order and query-service tests are green. The
  query-plan gate reports indexed timeline/entity/port paths, `events_search_fts` plus the narrow
  ordering join, timestamp/importance edge scans, and primary-key daily aggregate lookup; no bare
  `SCAN TABLE events` appears.
- 100k gate (30 warm samples): page p95 **7.513 ms**, summary **10.202 ms**, search **26.749 ms**;
  all are within the 20/30/90 ms budgets.
- 10k gate (30 warm samples): page p95 **2.583 ms**, summary **0.655 ms**, search **2.280 ms**;
  all are within the 10/15/40 ms budgets. These current three-path runs do not close the separate
  event-details, investigation, checkpoint-compare or privacy-summary rows in the full §61 table.
- 1M gate (30 warm samples, workspace fixture with `TEMP`/`TMP` on D:): page p95 **10.069 ms**,
  summary **12.127 ms**, search **219.873 ms**; all are within the 45/80/250 ms budgets. The run
  completed in 905 s on this machine; the seed-time target in the risk table is still not met and
  remains an operational concern. The database-size, resource, soak, detailed-query and manual
  accessibility gates remain open.
- Database-size/reclaim gate now passes at all scales with the new search projections: 10k
  **9,515,008 → 2,703,360 bytes**, 100k **93,519,872 → 23,601,152 bytes**, and 1M
  **930,607,104 → 229,163,008 bytes**, each under the 12/110/1,100 MB budgets; the 1M purge deleted
  1,000,000 rows and reduced freelist **4 → 0**. Maintenance remains incremental for ordinary
  passes; the size gate invokes the explicit full-compaction action separately so routine retention
  never monopolizes the serial writer.
- Follow-up hardening covers renderer full-range sentinels with saturating day arithmetic and
  pre-epoch daily buckets, including the first representable `i64` timestamp. The initial migration
  013 test passed but missed a second-day boundary defect, found on 2026-09-29 and repaired forward
  by migration 014 without editing the applied SQL checksum.

## Verification evidence — 2026-09-29 boundary and route pass

- Migration 014 rebuilds the daily projection with the correct first partial-day endpoint; the
  repository left-edge scan now stops at the true next UTC midnight. Fresh boundary and v13→v14
  repair tests passed (migrations 14/14, repository 20/20). The tiny upgrade fixture does not prove
  1M upgrade duration or free-space behavior.
- The `src/app/route-prominence.test.ts` App-level tests exercised Home → Investigate, Timeline →
  Details and Investigations → Create checkpoint without visiting `#/system-map`. A deliberately
  transient `#/system-map` visit is caught by the negative control. `bun run check` reports zero
  errors/warnings, and the targeted one-worker Vitest rerun passed 4/4.
- `scripts/soak.ps1` now reads actual `WorkingSetPrivate` by PID and records private commit
  separately. It also fails closed on an incomplete process-tree sample, a disappearing/reused PID,
  a failing simulation command, or short/incomplete duration. Short local negative smokes exited 1
  with `status=review`; no app B2 checkpoints or four-hour soak were measured. Successful command
  execution still does not prove actual synthetic writes, zero dropped batches or healthy collectors;
  the complete soak gate stays open. The prior three-path 10k/100k/1M runs lacked five query shapes
  and are not described as the full §61 gate.
- The opt-in `perf_queries.rs` harness now calls all eight named service/command paths at every
  configured corpus size with 30 warm samples and the fixed budgets. Its first 10k run passed six
  shapes; `start_investigation` measured **1,601.975 ms** against 400 ms and
  `compare_checkpoint` **230.312 ms** against 150 ms. The host was 100% CPU after that run, so
  attribution needs a lower-load rerun. More importantly, this fixture still uses the older
  `synthetic::profile` (37 cycling entities, 60-second spacing and no persisted graph edges), not
  the plan's 3,000 entities / 15% relationships / 8% burst seeder. None of the eight shape results
  alone proves the final §61 planned-corpus gate until fixture parity is established; the 100k/1M
  full-matrix runs have not been attempted.
- The devtools seeder now passes a fresh 10k cardinality test: 10,000 events over 90 days, 3,000
  graph entities, 1,500 related-event
  rows, 800 burst rows, 40 checkpoints and 20 investigations. Its devtools-only insert path
  preserves *pre-existing* real entity/edge rows on exact-ID collision. A later review found
  live-DB races; the CLI now requires a fresh empty reserved QA directory, path-bound sentinel and
  exclusive seed/purge lock, while the Tauri executable refuses reserved QA roots before opening
  storage. Focused isolation/semantic tests passed 6/6 and the executable guard test 1/1. Internal
  library callers can still deliberately open a QA DB, so this is a shipped-app containment boundary,
  not a universal multi-process database lock. Purge removes
  tagged events, edges and exactly registered checkpoint/investigation artifacts but deliberately
  leaves entities whose ownership cannot safely be proved; repeat-run cleanup remains open.
- A follow-up found that the first 10k generator emitted 1,500 related-event JSON rows but only
  150 distinct persisted, valid evidence edges. The generator and test now assert **1,500**
  persisted unique edges from **1,500** distinct evidence events with zero uncovered related rows;
  six focused tests and the seeder Clippy gate pass. The corpus changed, so the 10k timings and
  size figures below are pre-density diagnostics, not results for this corrected corpus.
- The query-plan test now checks inspectable SQL stages for all eight §61 paths, including three
  correlated relationship indexes and a 150-row recurrence batch (focused 1/1). Its copied private
  SQL and legacy 10k fixture remain a plan-shape smoke, not full canonical-corpus parity. The
  earlier 10k/100k/1M DB-size runs also use the noncanonical 37-entity profile, so their checkboxes
  remain open; those numbers are diagnostics, not acceptance proof.
- `bun run test:perf` and the default release script now run query plans and then the eight-shape
  query/DB-size tests at 10k, 100k and 1M; mocked fail-fast and safe one-job default checks passed.
  This wiring does not mean the actual three-scale matrix passed; it has not run on the new corpus.
- The first near-canonical 10k fixture measured **11,411,456 / 12,000,000 bytes** before the
  persisted-edge density correction.
  After a safe tagged purge and explicit full vacuum, pages fell **2,786 → 308** and freelist
  reached **0**; 3,000 unprovable synthetic entity orphans remain. At that point the 100k/1M
  canonical size cells were unmeasured. The seeded service-checkpoint state was repaired to use the real
  semantic adapter and a warm assertion now finds changes. On that pre-density 10k fixture, seven of
  eight measured query budgets passed; `start_investigation` is **644.889 / 400 ms** p95 and remains
  open. The other p95 values (ms) were first page **1.849/10**, page 10 **1.869/10**, summary
  **0.251/15**, search **1.501/40**, details **0.279/10**, checkpoint compare **65.463/150**, and
  privacy summary **1.550/30**. At that point no canonical 100k/1M eight-query run had occurred.
- A subsequent review found the initial `burst` metadata was only a label, not a real Timeline
  cluster. The final-corpus seeder now proves **800** burst events form actual five-or-more-row
  same-category/entity/minute clusters, **1,500** distinct persisted evidence edges remain valid,
  and the 90-day endpoints/3,000-entity count hold (focused seeder tests **6/6**). It removed
  synthetic index/slot bookkeeping and duplicate seed values from before/after JSON, which were
  artificial fixture overhead rather than product data. Final-corpus 10k release DB size is
  **11,599,872 / 12,000,000 bytes**. Tagged purge removes all 10,000 events and its owned
  checkpoint/investigation/edge rows; explicit full vacuum reduces pages **2,832 → 309** with
  freelist **0**. The 3,000 entities are retained because their later ownership cannot be proved;
  routine retention reclaim and the larger corpus cells remain open.
- On that same final corpus, the release-profile eight-query gate passes **8/8** over 30 warm
  samples. p95 milliseconds: first page **0.559/10**, page 10 **0.671/10**, summary **0.384/15**,
  two-token search **0.727/40**, details/recurrence **0.064/10**, persisted 5k investigation
  **144.212/400**, 3k checkpoint comparison **15.159/150**, privacy summary **1.006/30**.
  The earlier debug-profile investigation miss was not a release-profile miss; the scripted
  matrix now uses `cargo test --release` for the timed query/size cells and a validated D-backed
  temporary parent for large corpora. Mocked fail-fast/env-restoration checks pass; the 100k
  result follows, while 1M remains unrun.
- These fixture-size numbers are specific to the canonical synthetic payload: it uses a short
  state change and a burst marker instead of each real collector's metadata shape. The planned
  10k UI/payload spot-check against representative collector output remains open, so the 10k/100k
  DB-size passes are not evidence that every real-world history fits the same bytes-per-event ratio.
- The next canonical release-profile **100k** run seeded each fixture in about **23–25 s**.
  Database size passes at **82,464,768 / 110,000,000 bytes**; tagged purge removes all
  100,000 events and explicit full vacuum reduces pages **20,133 → 4,587**, with freelist **0**.
  Seven of eight query budgets pass: first page **0.636/20**, page 10 **0.597/20**, summary
  **0.382/30**, two-token search **4.481/90**, details **0.113/15**, checkpoint compare
  **23.862/200**, privacy summary **9.772/60** ms p95. Persisted 5k investigation is
  **896.470/600 ms p95 — fail**; the matrix stopped before 1M. Read/analysis attribution is
  pending, and no budget exception is recorded.
- A batched, indexed recurrence count preserved boundary/zero/duplicate-ID semantics in
  23/23 focused tests, and the SQL-plan gate found the covering recurrence index with no
  bare `events` scan. The 100k release query gate remains **7/8**: investigation fell to
  **784.607/600 ms p95**, still a failure. A controlled stage run measured recurrence at
  **76.617 ms p95**; load/analysis/persistence attribution continues before another change.
- The new separate routine-retention gate runs production `maintenance::run_pass` on the
  final 10k corpus. It deleted exactly **1,575** eligible events, retained the important
  rows within their longer lifetime, and reduced SQLite pages **2,832 → 2,691** with
  freelist **0 → 0** in **148 ms**. The prior `execute_batch` invocation had stepped the
  incremental-vacuum pragma only once, leaving **140** free pages after that same purge;
  draining a capped `incremental_vacuum(256)` fixes it without an unbounded writer job.
  The >512-free-page cap regression and all 5 maintenance tests pass in release mode;
  7 seeder/purge tests also pass, including `--no-relations` producing neither related IDs
  nor edges. The all-scale DB-size/retention checkbox stays open until 1M is measured.
- A later **unchanged-source** low-load 100k release rerun passed all eight query budgets;
  investigation p95 was **317.194/600 ms** (30 warm samples). A preceding diagnostic on
  the same code/corpus had measured **786.441 ms** under higher host load, so this does
  not establish stable release-candidate performance by itself. The corrected **1M**
  release query matrix then passed **6/8**: first page **0.544/45**, page 10 **0.898/45**,
  summary **2.333/80**, search **41.953/250**, details **0.436/30**, checkpoint compare
  **38.111/300** ms p95 passed; investigation **11,187.263/1,500** and privacy summary
  **262.719/200** ms p95 failed. The 1M seed took **316.041 s**, over its four-minute
  operational target. Query stage profiling and separate size/reclaim measurement follow;
  no budget or fixture was relaxed.
- The separate corrected-corpus **1M size** gate passed before migration 015:
  database **795,377,664/1,100,000,000 bytes**, WAL **88,921,992 bytes**; tagged purge
  deleted all 1M events and explicit full `VACUUM` reduced pages **194,184 → 43,822**,
  ending with freelist zero. Seed time was **362.920 s**. This is not a v15 size
  result and not proof of routine 1M incremental retention reclaim.
- A v15 diagnostic seeded 1M in **325.504 s** and isolated the investigation miss:
  mirrored load **1,961.442 ms p95**, pure analysis **7,840.230 ms p95**;
  `FailureIndex::build` alone **8,509.949 ms p95** in independent samples,
  reflecting two-hop walks from ~2,500 entities over 150k edges. Relationship
  load **1,099.504 ms p95**; equal-row ordered full-scan diagnostic **212.841 ms**.
  The new category-count projection read **0.539 ms p95** vs old `GROUP BY`
  **106.629 ms**, with equal counts. These diagnostic p95s are not additive.
  Migration tests **15/15**, privacy/purge tests **5/5** and updated query-plan
  gate **1/1** pass; full v15 1M budgets and trigger write impact remain open.
- Reverse 2-hop failure propagation and adaptive dense relationship loading passed focused
  differential/repository/analysis/chain tests, SQL plan and Clippy. The first optimized v15
  **100k** release gate passed **8/8**: investigation **213.908/600 ms p95** and privacy
  summary **0.627/60 ms p95**; six other paths also passed. Seed **19.153 s**.
  Investigation read/analysis p95 were **145.947/56.429 ms**; FailureIndex was
  **1.052 ms**. A skewed-graph fallback hardening regression and optimized 1M gate remain
  pending, and the earlier 1M 6/8 failure must not be presented as resolved yet.

- The dev-only 100k scroll harness now renders the production virtualized
  `TimelineList`, not fixed placeholders over a blank spacer. Headless Edge
  measured **16.90/20 ms p95** over 600 frame intervals in three smooth
  start/middle/95%-depth segments, with **0 blank frames** and furthest visible
  virtual row **96,291**. Repositioning time was excluded from the scroll
  metric; separate browser and jsdom checks verified actual event labels at
  middle/end positions. This closes only the dev-source reproduction path;
  installed WebView2 FPS and manual map frame gates stay open.
- The first optimized v15 **1M** release run seeded in **337.172 s** and passed
  **7/8** query budgets (30 warm samples). First page **0.763/45**, page 10
  **0.674/45**, summary **2.542/80**, search **46.108/250**, details
  **0.362/30**, checkpoint compare **17.784/300**, and privacy summary
  **0.614/200** ms p95 passed. Persisted 5k investigation was
  **2,028.316/1,500 ms p95 — fail**. A stage diagnostic is required before
  another optimization; neither the threshold nor corpus has been relaxed.
  The 1M seed also remains above the four-minute operational target.
- The separate 10k collector-shaped payload sensitivity run measured
  **12,042,240/12,000,000 bytes** (payload JSON **852,714 bytes**), missing
  the strict size budget by **42,240 bytes**. This is an opt-in sensitivity
  profile, not a replacement for the canonical corpus; its failure must be
  resolved or reported separately, not hidden by a larger budget.
- The next v15 **1M** stage diagnostic seeded in **389.464 s** under a
  different host-load window and separated persisted-investigation work:
  mirrored load **1,189.717 ms p95**, analysis **425.993 ms p95** (five
  samples). Within load, recurrence counts dominated at **896.665 ms p95**;
  relationship loading was **266.224 ms p95**, window hydration **38.913 ms**.
  Graph build took **163.319 ms** and `FailureIndex` **25.762 ms** in one
  isolated CPU sample. Equal-result relationship full scan was
  **311.908 ms p95** versus current adaptive path **418.856 ms p95** for
  2,483 requested IDs; this diagnostic is not the 30-sample acceptance gate.
- Grouping each QA graph batch's endpoint checks and ~150 edge inserts in one
  SQLite transaction (without changing the sanitized production event-writer
  path) passed the 10k canonical distribution/purge regression. The v15
  canonical **1M size** rerun then seeded in **285.559 s**, still above the
  **240 s** operational target. Database size was **795,389,952 / 1,100,000,000
  bytes** with WAL **90,594,712 bytes**. Tagged purge deleted all 1M events;
  explicit full `VACUUM` reduced pages **194,187 → 43,825**, left freelist
  **0**, and the entire size/reclaim test passed. This establishes the v15
  canonical size/full-compaction cell, not 1M routine incremental retention.
- A whole-index streaming recurrence candidate passed a 2,100-query
  differential regression but did **not** close the 1M gate: investigation
  **2,536.633/1,500 ms p95**, seven other shapes passed. Search also rose
  **46.108 → 105.963 ms** between the two runs, so cross-run host load is a
  confounder. A same-corpus 1M A/B of exact 5k recurrence counts measured
  **612.954 ms p95** for the streaming scan, **921.942 ms** for direct counts
  in input order, and **388.213 ms** for direct counts sorted by signature
  (six alternating samples). The latter is now the candidate under test;
  no acceptance result is claimed yet. The same diagnostic saw relationship
  load **284.925 ms p95** versus an equal-result full scan **200.234 ms**.
- The signature-sorted, exact prepared-count path replaced that streaming
  candidate; the single-scan relationship branch also stopped allocating a
  duplicate set already guaranteed by the table's UNIQUE triple. The 2,100-
  query differential test (including extreme timestamps, equal timestamps,
  empty windows and duplicate IDs), full repository suite **24/24**, Rustfmt
  and all-target/all-feature Clippy passed. The next canonical v15 **1M**
  release gate passed **8/8** over 30 warm samples: first page **0.514/45**,
  page 10 **0.561/45**, summary **2.635/80**, two-token search **44.031/250**,
  details **0.574/30**, persisted investigation **1,151.732/1,500**,
  checkpoint compare **21.985/300**, privacy summary **0.892/200** ms p95.
  Seed time was **270.024 s**, still above the **240 s** operational target.
  A profiler-free gate rerun and the new QA FTS-reclaim assertion remain open.
- The explicit QA purge now rebuilds both external-content FTS indexes from
  surviving events inside its transaction; a focused test proves a real event
  stays searchable while a purged synthetic event disappears. This addresses
  FTS5 tombstone segments that left **179,507,200 bytes** after the previous
  1M full `VACUUM`. The new synthetic-only post-purge bound is ≤12 MB after
  full compaction. At **10k**, current-source size **11,612,160/12,000,000**
  bytes passed and post-purge size fell to **741,376 bytes** (pages
  **2,835 → 181**, freelist zero). The current-source 1M post-purge rerun is
  now complete: **795,389,952/1,100,000,000 bytes** before purge, WAL
  **90,594,712 bytes**, exactly **1,000,000** tagged rows deleted, and
  **741,376 bytes** after explicit full `VACUUM` (pages **194,187 → 181**,
  freelist zero). The seed took **270.223 s**. This verifies full-compaction
  reclaim on the disposable corpus; routine 1M incremental retention and the
  ≤240 s seed target remain open.
- The profiler-free current-source **10k** and **100k** release query reruns
  passed all eight shapes over 30 warm samples. At 10k, investigation was
  **86.965/400 ms p95**; at 100k, investigation was **154.516/600 ms p95**.
  The profiler-free **1M** rerun seeded in **340.589 s**, then passed **7/8**:
  investigation reached **1,758.975/1,500 ms p95** under a more loaded host,
  while the other seven shapes passed. The prior 1M **8/8** result is valid
  for its run but not a stable acceptance verdict; no budget exception is
  approved. A QA-only seed-input reduction is under test and does not change
  the production read path.
- The QA purge can delete graph edges only while their synthetic evidence event
  still exists. Routine retention NULLs the edge's `event_id` when that event
  expires; the remaining edge then has no immutable QA owner. Purge leaves it
  intact to avoid deleting a real edge with matching endpoints. The canonical
  full-compaction size tests purge fresh fixtures before retention, so their
  741,376-byte result must not be extrapolated to a retention-first database.

## Verification evidence — 2026-10-04 current-source sync

- The current BFS-backed 10k release query gate passed 4/4 test cases, covering
  all eight query shapes over 30 warm samples. Persisted investigation p95 was
  **234.212/400 ms**. The stronger warmup requires `no_cause_reason=None`,
  with no no-cause shortcut, and a non-empty ranked-suspect result; the shared fixture forwards the aggregate
  `seed timings:` line. These are test-only harness changes and do not alter
  production logic, corpus distribution, sample count or budgets.
- The current 100k release query gate also passed 4/4, with all eight shapes over
  30 warm samples: first page **1.266/20 ms**, page 10 **1.104/20 ms**, summary
  **0.975/30 ms**, search **5.581/90 ms**, details **0.132/15 ms**, persisted
  investigation **220.788/600 ms**, checkpoint comparison **26.851/200 ms** and
  privacy summary **0.782/60 ms**. The seed took **20,575.794 ms**.
- The current-source canonical 1M query matrix completed with **7/8** budgets in
  **553.50 s** (terminal exit 1): first page **0.828/45 ms**, page 10
  **0.722/45 ms**, summary **3.451/80 ms**, search **55.788/250 ms**, details
  **0.432/30 ms**, checkpoint comparison **35.965/300 ms** and privacy summary
  **1.122/200 ms** passed; persisted investigation was **2,403.553/1,500 ms
  p95 — fail**. Three fixture tests passed and the single budget test failed. The
  exact canonical fixture used 3,000 entities, 150,000 related events/edges,
  80,000 burst rows, 40 checkpoints, 9,800 snapshots and 20
  investigations; the stronger scoring warmup passed and disposable-fixture
  cleanup confirmed 3.76 GB free on D:. Seed time was **489,448.246 ms** against
  **240,000 ms**; stage timings were generation 11,745.270 ms, insertion
  **455,443.520 ms** (~93.1%), graph 19,510.494 ms, inventory 46.656 ms,
  checkpoints 109.318 ms, investigations 2,437.063 ms, total 489,378.189 ms.
  Full cells are in the [dated QA evidence report](../../plans/reports/qa-261004-retention-native-evidence.md).
- The canonical 1M routine-retention gate passed: 157,304 eligible rows deleted,
  842,696 remaining, 317 batches in **34,745 ms**; the follow-up deleted no
  additional rows and reclaimed exactly 256 pages in **35 ms** on the follow-up.
  First-pass pages moved **194,187→193,931** (freelist **0→18,058**); the
  follow-up moved **193,931→193,675** (freelist **18,058→17,802**). The seed took **955.101 s**,
  missing the **240 s** operational target; this does not waive that target or
  close the other Phase 20 gates. The prior 2026-10-02 100k retention result is
  recorded in the [dated evidence report](../../plans/reports/qa-261004-retention-native-evidence.md).
- `cargo clippy --release --locked --all-targets --all-features -- -D warnings`
  passed before and again after the two subsequent test-only changes; the fresh
  rerun exited 0 in **56.79 s**, and rustfmt passed. A fresh all-features release
  library run passed **457**, with 0 failed and 1 ignored (458 total) in **23.03 s**;
  the exact capture test had earlier passed 1/1. These focused checks are not a
  clean default verification result or an installed release-candidate pass.

## Actual-service stage attribution — 2026-10-04

- Opt-in devtools timings now wrap the executed investigation stages without
  changing queries, transaction boundaries, the 5,000 cap, scoring or persistence.
  Normal release has no active timing implementation. Independent review found
  no actionable defect; timing units passed 2/2, service integration 6/6 and
  the release all-feature library suite 459/460 with one hardware gate ignored.
  Complementary Clippy checks pass for normal production lib/bins without
  features and all targets with all features. The test-only noop helper guard
  now matches the test/debug/devtools module boundary; no tests were hidden.
- Fresh 10k and 1M actual-service diagnostics passed one warmup and five fully
  scored/persisted calls each, with exact canonical counts and owned cleanup.
  1M measured e2e mean was **1,446.826 ms**, min **1,000.119 ms**, max
  **2,567.529 ms**. Exclusive recurrence ranged **414.322–874.177 ms**,
  analysis **411.929–1,287.581 ms**, relationships **118.962–294.504 ms**.
  These five-call diagnostics do not replace the 30-sample p95 gate above.
- The fresh 1M diagnostic seed still missed **429.057/240 s**; event insertion
  consumed **400.584 s** (~93.4% of measured setup). No individual trigger,
  statement, allocation or writer optimization is inferred from that aggregate.
- Counsel prioritizes finer actual-analysis stages plus a source-owned bound
  plan for the executed sorted recurrence branch, outside measured requests.
  Keep all samples/budgets and duplicate-ID/time-bound semantics unchanged.
  [Full diagnostic evidence](../reports/qa-261004-investigation-service-stages.md).
  No phase status or acceptance checkbox was promoted.

## Finer analysis and executed recurrence evidence — 2026-10-04

- Seven opt-in analysis stages and a devtools-only scoped bound recurrence
  capture preserve the executed SQL, binds, transactions, counts and ranking.
  Independent reviewer, simplifier and failure forecast found no actionable
  issue. Nine new units passed; release/all-feature library 468 passed/1 ignored,
  service 6/6, repository 24/24 and fixture cleanup 3/3 passed. Complementary
  release Clippy passed for all-feature/all-targets and normal lib/bins.
- Fresh canonical 10k and 1M profiles passed; the executed first sorted COUNT
  uses the covering recurrence index with 5,000 completed counts. Capture runs
  on the warm call only. This is representative plan evidence, not proof of
  every query or the eight-shape plan gate.
- Five 1M calls had e2e mean **1,724.802 ms**, min/max **1,349.936/2,623.092 ms**.
  Exclusive graph construction **245.282–351.208 ms** and recurrence
  **579.280–1,334.776 ms** identify real cost owners. No outlier was discarded;
  host snapshots cannot attribute the request spike. Seed still missed
  **322.266/240 s**, with insertion **299.611 s**; no seed optimization is claimed.
- Counsel selects borrowed, normalized relationship-fact deduplication as a
  bounded graph experiment, with complete old/new graph equality and isolated
  duplicate-heavy/mostly-unique timing before expensive canonical acceptance.
  No candidate gain or acceptance pass is established by this profile.
  [Dated source and artifact evidence](../reports/qa-261004-analysis-and-recurrence-attribution.md).
  The profiler-free 1M p95, seed and remaining acceptance gates stay open.

## Graph candidate and profiler-free matrix — 2026-10-04

- Borrowed normalized relationship facts are deduplicated before owned graph
  insertion. Independent old/new complete equality tests pass; release library
  470 passed, zero failed, two diagnostic benchmarks ignored. Complementary
  normal/all-feature release Clippy, service 6/6 and cleanup suites pass.
  Reviewer, simplifier and failure forecast found no actionable issue.
- Two isolated comparisons show duplicate-heavy builder time 60–67% lower,
  mostly-unique 1–4% higher; estimated extra 5.7 MiB buffer is not a working-set
  measurement. Canonical 1M five-call mean 864.795 ms is diagnostic, not p95.
  [Experiment/source/artifact evidence](../reports/qa-261004-graph-fact-deduplication.md).
- Fresh profiler-free 30-sample matrices pass 10k and 100k 8/8 each:
  investigation 120.964/400 and 361.874/600 ms. Canonical 1M remains 7/8:
  investigation **1,629.336/1,500 ms FAIL**; other seven shapes pass. Seed
  **532.237/240 seconds MISS**, insertion 496.449 seconds. Disposable fixture
  cleanup passed; all owned runner processes exited. No outlier was removed
  or host interference inferred. [All measured cells](../reports/qa-261004-graph-candidate-query-matrix.md).
- Failed-verification counsel retains graph provisionally and requires one
  predeclared 30-call paired actual-stage diagnostic before recurrence changes.
  Measure stages belonging to the same slowest calls, not independent stage
  percentiles. Seed attribution follows; copied query-plan parity stays a
  separate repair. No checkbox, phase or public-v1 verdict is promoted.

## Paired investigation tail diagnostic — 2026-10-04

- A separately ignored 30-call profile preserves the five-call diagnostic,
  canonical recipe and all scoring/persistence assertions. Begin/end markers
  are outside the e2e clock, while existing stage stderr is inside. Source
  review, cleanup 3/3 (two expensive tests ignored), targeted release/devtools
  Clippy and rustfmt pass. Strict output pairing confirms one completed engine
  and service record in every measured interval, with warm excluded.
- Exact 10k preflight passes; one predeclared 1M diagnostic passes with all
  30 observations retained. Mean 1,298.493 ms, min/max 919.125/1,933.324 ms;
  diagnostic p95 1,892.938 ms is not an acceptance replacement. The same three
  slowest calls have recurrence 883.653/859.664/770.817 ms, about 44–46% of
  their own e2e times. Relationships and graph also increase; no single
  universal cause or host interference is inferred. Seed 412.905/240 s misses.
  [All paired observations and provenance](../reports/qa-261004-paired-investigation-tail.md).
- Counsel approves one last structural warm-capture measurement: exact
  signature/window repetition and adjacent overlap, retaining integers only
  after explicit scope activation. Keep the measured SQL and execution count
  unchanged. Then select one bounded recurrence experiment or reject reuse as
  insufficient; do not rerun tail diagnostics seeking certainty or green.
  That statistics step is verified below; all acceptance gates stay
  open; the profiler-free failure above remains authoritative.

## Bounded recurrence-window statistics — 2026-10-05

- The explicitly active devtools warm capture borrows the existing sorted
  inputs and returns eight integer counters, retaining no identities or bounds.
  Inactive capture does no additional walk or allocation. SQL, actual COUNTs,
  the 2,000-input threshold and 150-input duplicate-ID aggregation are unchanged.
- Focused 15/15 and full release/all-feature library 479/0 failed/two ignored
  pass; complementary release Clippy passes with warnings denied. Caller
  integration passes service 6/6, repository 24/24 and fixture cleanup 3/3.
  Review and simplifier found no actionable issue. The format-only wrapping
  failure was counseled before repair. Coverage percentages are not measured.
- Exact five-call canonical 10k profile passes with warm excluded. All 5,000
  actual warm COUNTs use the covering recurrence index; inputs contain 3,217
  signatures, 4,680 windows, 320 exact repeats and 1,783 adjacent monotone
  overlaps. The one predeclared fresh 1M structural run also passes: 3,429
  signatures, 5,000 windows, zero repeats, no invalid windows, maximum group
  18 and 1,571 adjacent monotone overlaps. All 5,000 warm counts use the same
  covering index; owned fixture cleanup passed and all runners exited.
  [Detailed checks, numeric evidence and source/artifact hashes](../reports/qa-261005-recurrence-window-reuse.md).
  These counts establish opportunity only. No recurrence optimization or
  acceptance pass is claimed, and no checkbox or phase status is promoted.
- Zero 1M repeats rejects exact-window caching as the main repair. Counsel
  approves one test-only transaction-local incremental COUNT prototype, with
  unchanged autocommit fallback and production routing. Full-map parity,
  stable-snapshot/concurrent-commit cases and matched alternating representative,
  sparse, dense and tiny-overlap comparisons must precede any promotion.
  The structural 1M seed still misses 350.420/240 seconds; five instrumented
  service calls are not a replacement p95 gate. Seed repair remains separate.

## Transaction-local rolling COUNT experiment — 2026-10-05

- Both test-only variants preserve complete maps and stable read snapshots.
  The first variant regresses tiny-overlap about 75% and is not promotable.
  The one approved i128 span-cost guard passes 13 parity cases, including
  equal/above thresholds, endpoint extremes and clustered-density limitations.
- The unchanged fixed 88-pair guarded benchmark passes. Canonical-like input
  improves 16–18% aggregate in both rounds, all 22 pairs faster; sparse and
  tiny-overlap changes vary in sign. These are synthetic COUNT timings, not
  actual service or canonical acceptance results. All observations retained.
  [Protocol, source hashes and both runs](../reports/qa-261005-recurrence-incremental-prototype.md).
- Full release/all-feature library passes 492/zero failed/three ignored.
  Clippy rejects one test-table tuple type; counsel approves an alias-only
  repair, now passing 13 parity cases, formatting and both complementary
  release Clippy configurations. No lint suppression or benchmark rerun.
  A separately reviewed production candidate is underway, with independent
  old baseline and honest probe capture; integration verification follows.
  All acceptance checkboxes and the profiler-free failure remain unchanged.

## Guarded recurrence production candidate — 2026-10-05

- The frozen production entry point activates the approved helper only for
  ≥2,000 inputs inside an existing transaction. Autocommit/small behavior and
  independent old aggregation remain; no schema, pragmas or seeder change.
- Thirteen parity cases now compare independent old results to the public
  production route. Six new capture cases distinguish actual full/delta SQL
  from resolved/reused inputs and preserve scope/error/panic guards; pinned
  WAL capture survives an external commit. Focused production tests pass
  34/zero failed/one benchmark ignored; full library passes 498/zero failed/
  three explicit ignores in 25.19 seconds for this revision.
- Root formatting/diff checks, independent source review, conditional
  simplifier and complementary release Clippy pass. The stale routing docs
  finding is corrected and the same reviewer finds no concrete integration
  failure in its narrow forecast. Callers pass service 6/6, repository 24/24
  and fixture cleanup 3/3; two diagnostic profiles remain ignored. Frontend
  typecheck and 61 files/327 tests pass. Unchanged profiler-free 10k/100k
  matrices pass 8/8 each, Investigation 79.222/400 and 147.841/600 ms.
  The single unchanged-source 1M matrix also passes 8/8, Investigation
  1,032.501/1,500 ms. [All current cells and output](../reports/qa-261005-recurrence-rolling-query-matrix.md)
  retain aggregate p95, not unexported individual timings. Counsel GO closes
  exactly the two query-budget checkboxes, bringing P20 to 6/23. Seed remains
  316.439/240 seconds — MISS; insertion 294.050 seconds. Next: explicitly
  enabled devtools batch insertion attribution, not a speculative SQL/index
  rewrite. Other acceptance obligations remain open.
  [Hashes and verification ledger](../reports/qa-261005-recurrence-rolling-production.md).

## Seed insertion attribution — 2026-10-05

Counsel selects one explicitly enabled devtools batch recorder after the
316.439/240-second seed miss. It brackets begin/preparation, the unchanged
combined event/entity loop, reconciliation, commit and repository total;
no per-row clocks or identifying data. Inactive capture must read no clocks.
SQL, interleaving, ownership and transaction boundaries remain unchanged.
Focused parity/rollback/isolation checks and one 10k/one 1M diagnostic follow;
neither diagnostic replaces seed acceptance. [Owned execution boundary](../reports/qa-261005-seed-insertion-attribution.md).
Implementation is frozen and verified: seven focused recorder tests, full
library 505 passed/zero failed/three explicit ignores, complementary release
Clippy, CLI 2/2, repository 24/24 and writer/cleanup 2/2 pass. The first 10k
diagnostic fails only its copied table name; counsel-approved test-only fix
preserves its filter/15% count. Both the failed output and corrected 10k pass
are retained. Corrected smoke verifies corpus counts/span and 10/10 committed
batches; repository 899.281 ms, event/entity loop 793.440 ms. One canonical
1M attribution passes all numeric/basic corpus checks and cleanup: 1,000
committed batches/1M inputs, commit 195.394 s (~58% repository), combined loop
142.508 s, diagnostic total run 368.104 s. No 10k extrapolation, cross-run
causal claim or seed checkbox promotion. Counsel approves one private 1,000
versus 5,000 batch experiment with default/settings unchanged and full parity
before timing. Source audit identifies a potential entity-timestamp dependence
on graph/event batching; the actual canonical negative proof confirms one
`first_seen` difference with complete event/edge/ownership equality, rejecting
the 5,000-event batch before timing. Default stays 1,000. Counsel separately
allows design of five unchanged logical event→graph units per outer commit;
exact API audit comes before implementation, with no gate promotion.
[Separate protocol](../reports/qa-261005-seed-batch-size-experiment.md).

## Transaction-body extraction — 2026-10-05

The separately counseled first stage extracts the unchanged event/entity/
reconciliation body behind `&Transaction`, keeping old begin/commit, errors,
prepared scopes and recorder behavior. The devtools entry consumes ownership;
only successful body values return provisional state, and any error requires
abandoning the outer transaction. Body timing has no committed counters.
Focused legacy/new tests pass 14/14; full library 512 passed/zero failed/three
ignored; both release Clippy configurations and independent read-only review/
simplifier pass. Affected callers pass seeder 3/3, repository 24/24, purge 9/9
and insertion profile 2/2, with two intentional diagnostic/proof ignores.
Counsel accepts the extraction and approves private fixed-five grouped seeder
correctness work, preserving each logical 1,000-event event→graph sequence.
This extraction checkpoint did not authorize timing/default promotion.
[Protocol, frozen hashes and evidence](../reports/qa-261005-seed-transaction-grouping-prototype.md).

## Fixed-five grouped correctness — 2026-10-05

The private cfg(test) seeder route preserves five independent logical
1,000-event event→graph units in each transaction. Complete 10k and 11,001
persisted parity, real collisions, deferred commit/event/graph failures,
committed-prefix progress, purge and cleanup pass: 13 grouped cases, full
seeder 17/17 including the rejected 5,000-event negative proof. Default-route
purge 9/9 and insertion profile 2/2 pass (one diagnostic ignored). Two initial
compile types and the large private error lint were counseled before repair;
the boxed-error six-case rerun and both complementary strict Clippy checks pass.
Independent reviewer/simplifier and boxed amendment review are clean; root
checks source/artifact identities and absent owned fixtures/runners.
Counsel permits a private paired timing harness, reviewed before the fixed
10k pair and one canonical 1M pair, sequential fixture/digest parity and disk
preflight. Default stays 1,000; 316.439/240-second seed MISS, all remaining
acceptance checkboxes and release NO-GO stay unchanged.
[Correctness, raw output and next protocol](../reports/qa-261005-fixed-five-seed-grouping.md).

## Fixed seed-grouping pair outcome — 2026-10-05

Exactly one baseline-first10k pair and one fresh canonical1M pair pass complete
normalized all-table/ownership parity, corpus checks and sequential cleanup.
At10k, grouped1,257.987ms versus baseline1,109.966ms is13.34% slower. At1M,
grouped332,180.967ms versus baseline369,085.288ms still misses240,000ms.
Counsel rejects promotion and more size/pair tuning; default1,000 remains and
fixed-five stays private cfg(test). Profiler-free316,438.748ms acceptance MISS
and all remaining gates remain unchanged. Full stdout, frozen source/artifact
and absent owned runner/fixture checks:
[completed pair record](../reports/qa-261005-fixed-seed-pair-harness.md).
Next approved slice is actual-bound page SQL capture; separate source-backed
commit-internal investigation precedes any further seed change.
The read-only [commit PROFILE source ledger](../reports/qa-261005-commit-profile-source-ledger.md)
records a conditional later diagnostic: feature isolation and page freeze first,
no WAL-hook/checkpoint/settings change, and no exact checkpoint/fsync attribution.

## Actual-bound page source checkpoint — 2026-10-05

The approved page-only slice has13 focused cases and525 library tests pass,
zero failures/three intentionally ignored diagnostics; formatting, independent
review and simplifier are clean. Callers pass36, both strict release Clippy
configurations pass, and exactly one canonical10k actual page1/page10 smoke
passes with indexed SEARCH events and complete201-row decoding. The typed capture separates actual operation
completion from EQP/privacy evidence; unknown plan vocabulary does not fail the
query. [Source and verification ledger](../reports/qa-261005-page-plan-capture-ledger.md).
The prior24-cell acceptance matrix remains verified at its recorded source/
artifact checkpoint, retaining exactly its two completed checkbox rows.
Subsequent page bind-storage/inactive-capture changes are correctness-checked,
not current-source latency acceptance. Fresh latency refresh remains explicit
for the eventual RC; no new box or entire-phase status is promoted.

## COMMIT PROFILE diagnostic source checkpoint — 2026-10-05

Counseled numeric capture is source-frozen behind devtools-only trace forwarding;
normal Windows resolved feature tree remains exact. It installs only in a
fresh exclusive QA writer job and arms at actual consuming insertion COMMIT,
preserving original result/error/legacy timings. Invalid callbacks/failures
are separate; post-PROFILE tail is not exact checkpoint/fsync time. Default
1,000 event→graph sequence/CLI/SQL/settings/WAL behavior remains unchanged.
Root fmt/scoped diff and initial inline source review pass; compilation succeeds,
but all8 focused cases fail because successful insertion COMMIT emits11 PROFILE
callbacks, invalidating the exact-one assumption. Diagnostic is stopped; no
10k/1M workload runs. Fresh counsel permits bounded identity/internal-statement
source audit only, not callback guessing or assertion relaxation.
Earlier independent reviewer/simplifier/advisory followups
are unavailable due runtime agent thread limit, not falsely called complete.
[Frozen source/protocol](../reports/qa-261005-commit-profile-source-freeze.md).
Any diagnostic continuation requires resolving safe identity feasibility and
the failed lifecycle/caller/lint gates. No paired run, acceptance checkbox or
seed-time target relaxation. [Full failure](../reports/qa-261005-commit-profile-focused-failure-output.md).
The bounded pinned-source audit finds no safe actual-COMMIT identity;
counsel rejects the technique and approves scoped rollback, not test weakening.
All10 rejected source snapshots decode/SHA-match before6 prototype files are
retired; shared storage/seeder hashes and feature state restore exactly.
Post-rollback storage106/seeder23/callers47 and both strict release Clippy
checks pass, formatting and exact restored-owner hashes verified. Independent
rollback reviewer/simplifier are clean, no edits; all8 rejected test failures
stay historical failures, not masked by these176 passing regression cases.
No new acceptance checkbox closes. No PROFILE workload or
replacement diagnostic is authorized. [Rejection/archive](../reports/qa-261005-commit-profile-rejection.md).

## Actual checkpoint event-window evidence — 2026-10-05

The accepted narrow owner capture keeps the same fixed SQL, connection,
normalized inclusive binds, ordering, hard10k cap and original errors/results.
Eight frozen source paths pass focused10, full-library535/three ignored,
callers7, both strict release Clippy configurations, formatting and clean
independent reviewer/simplifier checks. Exactly one counseled canonical10k
compare(Some) diagnostic passes full inactive/captured return equality and
245 completely decoded event-window IDs; typed EQP is covering timestamp-index
SEARCH. [Source/gates/complete output](../reports/qa-261005-checkpoint-window-capture-ledger.md).
First preflight fails before any workload; counsel approves only an absolute
ancestor-path correction. No source/assertion change or diagnostic rerun.
Owned runners/fixture exit and cleanup pass. This is not by_id/items/full
comparison plan, other branches/scales or fresh latency acceptance. All
remaining boxes and status stay unchanged; P20 6/23/public-v1 NO-GO.

## Local resource run and onboarding re-mount defect — 2026-10-07

One local release-profile build launched against a profile with onboarding unfinished
measured both memory figures, start timing and idle CPU; the full table and method are in
[`docs/PERFORMANCE.md`](../../docs/PERFORMANCE.md). Summary: idle CPU averaged **328 % of one
core** with the window hidden, total/private working set drifted from 668.1/376.5 MB to
917.2/601.0 MB over 10 minutes, and the backend alone stayed near 48/12 MB. The cause was a
front-end loop: `settings.load()` blanked `current`, which unmounted `OnboardingFlow`, whose
mount reloads settings (~63 command round-trips per second). The store now keeps loaded
settings during a reload, with a regression test that fails without the change.

This run is diagnostic, not acceptance: the build contained the defect, the fix is
unit-tested but not yet re-measured on a rebuilt executable, tray-only start was not
measurable externally, and no soak, FPS, accessibility or installed-app row was run. No
checkbox, phase status or release verdict changed. Reports below the 10-minute idle row (B2 #3,
idle CPU, start-time) stay open until the rebuilt run.

### Follow-up after the settings-reload fix — 2026-10-07

A rebuild with the fix measured idle CPU **0.210 %** of one core (p95 interval 0.874 %)
with the window hidden for 602 s, total/private working set 411.5/130.3 MB hidden and
406.2/100.5 MB after 10 minutes, cold/warm start 1,288/124 ms (details in
[`docs/PERFORMANCE.md`](../../docs/PERFORMANCE.md)). The profile had onboarding unfinished,
so every collector was disabled: this clears the front-end loop but is not the
"all collectors enabled" idle-CPU row, so no checkbox changes. The idle-CPU, B2 and
start-time boxes close only after a run on a completed-onboarding profile.

### Attach-mode run with collectors on — 2026-10-07

A 604 s read-only run on a running build after onboarding **Skip** (collectors enabled,
window visible) measured **9.973 %** of one core (0.831 % of the machine, p95 interval
15.802 %), total/private working set 490.9/175.5 MB at start and 492.4/178.4 MB at the end
(peak 539.3/224.3), threads 52 flat; see [`docs/PERFORMANCE.md`](../../docs/PERFORMANCE.md).
The profile's process collector was recording 1,200-2,500 short-lived-process events per
minute from developer tooling during the run, so this is not a quiet-host idle result and no
checkbox changes. Closing the idle-CPU, B2 and soak rows needs a rerun on a quiet host with
the window hidden.

### Bounded history for busy developer machines — 2026-10-07

A real profile on a developer machine with agent tooling held 25,330 events after about 70 hours
(22,393 in one hour; 92% of process events from six helper programs; 25,324 informational), at roughly
930 bytes per stored event. Retention was time-only, so the 30-day default had no size bound. The
maintenance pass now also (1) expires informational process events after 3 days, or after the user's own
Events lifetime if shorter, and (2) when live pages exceed 1 GiB deletes a bounded number of the oldest
informational events, estimated from the overage and the average bytes per event, never touching notable,
important or critical rows. A first measured-loop version removed every informational row in one pass
because SQLite and the full-text index release space lazily; the shipped version fixes the count up front
and lets the next pass measure again. Decision record: D-26 in `docs/DECISIONS.md`.

Verification: rustfmt and strict all-features Clippy clean; library 473 passed (3 ignored),
`privacy_purge` 5/5, `repository` 27/27 including noise expiry, shorter user lifetime, oldest-first
cap pruning and a protected over-cap database. The 1M retention and size gates were not rerun, and the
3-day and 1 GiB values are proposed defaults from one machine. No checkbox changed.

## Verification evidence — 2026-10-14 larger query-shape diagnostics

After the collector-payload sensitivity size miss, the unchanged release-profile query harness
was run independently at 100k and 1M with the existing SQL, 30 warm samples, thresholds and
single-job resource controls. Both runs passed all eight query shapes:

- 100k: first page 0.777 ms, page 10 0.669 ms, summary 0.387 ms, search 4.013 ms, details
  0.105 ms, investigation 144.144 ms, checkpoint 20.950 ms, privacy 0.670 ms.
- 1M: first page 1.657 ms, page 10 1.203 ms, summary 6.904 ms, search 118.718 ms, details
  0.563 ms, investigation 755.032 ms, checkpoint 25.950 ms, privacy 0.474 ms.

The 1M seed took 319.636 s. These are diagnostics, not a promotion of the 10k/100k/1M
database-size, seed-time, resource, soak or accessibility checkboxes. The 10k collector-shaped
size gate remains failed at 12,042,240 bytes against 12,000,000; its paired inspection found
zero free pages and real event-payload/table growth, so no budget was relaxed and no evidence
was discarded. P20 remains in progress.

## Verification evidence — 2026-10-14 headless soak harness

A devtools-only `--soak` branch now requires an explicit reserved
`what-changed-perf-soak-*` directory, pre-seeds onboarding/collector settings,
opens the platform-composed `AppState`, starts the real collector supervisor,
and uses a local trigger/ack pair whose acknowledgement is written only after
the shared sanitized simulation helper commits. It never opens Tauri/WebView or
the normal application data directory. A clean compile/fmt check and release
devtools build passed. A direct trigger smoke committed one synthetic
configuration event and returned `ok|1`; its QA `events.db` had migration 16,
empty logs and `startup.json` with zero unclean starts.

The first attempted four-hour run was discarded as invalid because the initial
runner omitted `spawn_all()`; no soak result was claimed. After adding the
collector start and clean-shutdown path, a fresh livecheck spawned the workers,
ran the writer and exited nonzero with the explicit health result
`enabled collectors are unhealthy: docker`. Its QA WAL reached 1,178,352 bytes,
confirming real activity; the host has no Docker daemon. The runner now verifies
the exact production collector registry, fails on `Degraded` collectors and any
non-zero writer-loss counters
(`dropped_writes`, failed commit rows/cycles, or cycle-admission drops), so a
future run rejects those listed conditions; lifetime coverage needed additional
instrumentation, recorded below. This is an honest
capability-matrix failure, not a relaxed gate or a four-hour pass. The 4-hour
soak checkbox remains open until a host with every enabled collector available
completes the full duration and final growth checks.

## Verification evidence — 2026-10-26 soak diagnostic hardening

Fresh exclusive QA ownership, protocol alias rejection, real all-collector
readiness, sticky snapshot/failure observations, writer-loss checks, shutdown
join evidence and matching report/exit validation are implemented. This remains
a backend diagnostic, not the native/WebView four-hour gate.

Checks: soak unit 8/8, observer integration 5/5, shutdown join 1/1, PowerShell
protocol negatives, full library 571 passed / 3 intentional ignores, final strict
all-target/all-feature Clippy and debug devtools build passed. Review fixes cover
separator/case/dot aliases and a real durable baseline in acknowledgement tests.
The bounded live negative check returned unavailable Docker, exit 1, failed/not
ready report, zero writer loss and incomplete shutdown. No pass or checkbox was
promoted. [Report](../reports/soak-hardening-261026.md),
[review](../reports/soak-review-261026.md).
