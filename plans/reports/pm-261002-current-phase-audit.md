# Current phase audit — 2026-10-02

## Outcome

The accepted plan remains **in-progress** and the public-v1 decision remains
**NO-GO**. A fresh read of all 25 phase files gives **312/369 checkboxes
complete (84.6%) and 57 open**. Metadata is consistent with that sweep:
**17 completed, 7 in-progress, 1 pending**. No incomplete gate is promoted by
this audit.

## Reconciled plan state

| Phase | Status | Done/open | Current state |
|---|---|---:|---|
| P01–P08 | completed | 110/0 | No residual phase checkboxes. |
| P09 | in-progress | 11/1 | Process/listener p95 gate remains open. |
| P10–P18 | completed | 155/0 | No residual phase checkboxes. |
| P19 | in-progress | 15/4 | Installed/manual notification, autostart and memory evidence remain open. |
| P20 | in-progress | 4/19 | Current performance, resource, soak, frontend and accessibility gates remain open. |
| P21 | in-progress | 9/10 | Current-source release, installed acceptance, upgrade/uninstall and traceability remain open. |
| P22 | in-progress | 4/3 | Windows theme/tray, parity and reviewer inspection remain open. |
| P23 | in-progress | 2/5 | Install-from-docs, screenshots, exact limitations, license notices and projection wording remain open. |
| P24 | in-progress | 2/5 | Claim/media/demo/synthetic-content checks and final README status remain open. |
| P25 | pending | 0/10 | All final readiness and human-approval rows remain open. |

The plan's 2026-09-22, 2026-09-23 and 2026-09-26 counts are historical
checkpoints, not current status. The 2026-09-29 checkpoint is the matching
current baseline. `plan.md`, phase frontmatter and `ROADMAP.md` agree on phase
status; the plan frontmatter correctly remains `status: in-progress`.

## Evidence reconciliation and contradictions

- The latest release-library run is **457 passed / 0 failed / 1 ignored**.
  The current combined Cargo run is serialized as one job and has **no result
  yet**; neither result closes the remaining release or manual gates.
- The 25 ms capture test
  `one_absolute_budget_includes_queue_and_provider_time` passed in the
  458-test run using a deterministic expired deadline. This validates the
  absolute-deadline contract only; it does not close P20 resource/soak gates.
- The corrected 5,000-event benchmark now asserts the exact
  `Duration <= 400 ms` contract, but its rerun is pending. The prior 5,000-row
  result must not be treated as current acceptance evidence.
- The latest profiler-free 1M query matrix is **7/8**, with investigation
  **1,758.975 ms > 1,500 ms**; its seed was **340.589 s > 240 s**. No exception
  is approved. The earlier 8/8 run is historical and does not close the gate.
- Canonical size/full-compaction evidence passes, but routine retention at
  100k/1M remains unmeasured. The collector-shaped 10k sensitivity fixture is
  **12,042,240 > 12,000,000 bytes** and remains a miss, not an approved
  exception.
- Notice preparation succeeded for **232 packages / 399 source files**, but
  remains `publishable=false` with hash prefix `cb86c07f...`; this is inventory
  preparation, not publication approval. The 13 missing-license/native issues
  require review.
- No installed manual accessibility pass, four-hour soak, installed resource
  or startup readings, current-source installer acceptance, or human public
  approval is present.
- User decisions remain binding: updater is an explicit opt-in scaffold with
  no feed/key and no network call; MIT is selected; memory reports both total
  and private working set with neither as a gate; no real private database is
  read for evidence.

## Exact open phase gates

### P09 — 1 open

- `cargo test` 50-cycle p95 assertion: process ≤15 ms and listeners ≤15 ms.
  Listener timings have stayed within budget; the current process result has
  not passed under a sustained low-load rerun.

### P19 — 4 open

- Record B2 measurement #3.
- Manually verify notification click focuses the window and opens the correct
  event drawer.
- Manually verify enabled autostart sets the registry/plugin state and a
  `--minimized` launch shows only the tray.
- Record idle, window-shown/window-hidden and peak B2 numbers in the phase
  file.

### P20 — 19 open

- Complete deterministic seeder/purge acceptance: canonical 10k/100k/1M
  counts, exact tagged purge/full compaction, 3k entity-ID limitation, and 1M
  seed ≤240 s.
- Pass query-plan assertions for all eight SQL-stage checks, sorted recurrence
  covering-index gate and service/corpus parity.
- Pass all eight query shapes at 10k/100k/1M on current source.
- Pass DB-size/vacuum gates, including specified corpus and post-purge
  freelist behavior; routine 100k/1M retention is still pending and the
  collector-shaped 10k sensitivity is over budget.
- Record idle CPU, both working-set figures and startup timings.
- Complete and summarize the four-hour soak.
- Complete frontend FPS, bundle-size and lazy-`three` gates.
- Complete accessibility sweep, contrast manifest, reduced-motion tests and
  per-screen Narrator record.
- Close every §61 cell, or attach a user-approved exception.
- Prove zero `SCAN TABLE events` across measured queries using the canonical
  fixture/service path.
- Prove DB size at all three sizes and post-purge freelist reaches zero.
- Prove idle CPU ≤1% over 10 minutes with all collectors enabled.
- At all three checkpoints record both total and private working set per D-15.
- Prove cold start ≤2.0 s, warm ≤1.2 s and tray-only ≤900 ms.
- Prove soak growth ≤15%, handles ±5%, flat threads, zero dropped batches and
  zero failed collectors.
- Prove 100k timeline p95 frame ≤20 ms, Change Map median ≤16.6 ms at 3,000
  nodes and System Map frame budget.
- Prove keyboard-only traversal of all eight screens with visible focus and no
  trap.
- Prove usable layout at 900×600 with 150% scaling and at 200% zoom.
- Record a Narrator result per screen in `docs/accessibility-audit.md`.

### P21 — 10 open

- Run an upgrade rehearsal with a real migration, including the current
  1M-v13→v14 elapsed/free-space path where applicable.
- Complete `docs/release-audit.md` §1–§91 traceability.
- Execute and record the manual acceptance walkthrough.
- Verify current installer installs per-user with no UAC and launches visibly
  within 5 s.
- Verify Start Menu shortcut and opt-in desktop shortcut behavior.
- Verify 10k events survive version-over-version install and migration applies.
- Verify uninstall preserves history unless explicitly opted in.
- Ensure the release audit has one named test/manual check for every §1–§91
  requirement.
- Pass all 15 acceptance steps, including zero runtime network bytes in
  Resource Monitor.
- Run `scripts/release.ps1` green from a clean tree without skipped steps.

### P22 — 3 open

- Inspect tray mark on light, dark and Windows high-contrast surfaces.
- Verify installer and application use the same approved mark and name.
- Obtain reviewer confirmation that the mark is not reasonably read as
  antivirus, cloud sync or automatic repair.

### P23 — 5 open

- Verify a new user can install, onboard and run the port-3000 investigation
  from docs alone.
- Replace/verify current, accessible, versioned screenshots labelled synthetic
  where applicable.
- Reconcile security, privacy and collector limitations exactly to tests and
  release audit.
- Verify canonical MIT license and package/installer/README agreement; review
  all third-party obligations, including the 13 notice/license issues.
- If a hosted wiki is created later, label it as a projection, not authority.

### P24 — 5 open

- Map every public claim to a passing test, release-audit row or explicit
  limitation.
- Produce a 60–90 second demo of the worked example with no hidden setup or
  causal overclaim.
- Produce screenshots for the core loop, trust surfaces and one honest
  degraded/empty state.
- Verify all media uses deterministic synthetic data and approved brand assets.
- Keep README's `under construction` status until P25 readiness passes.

### P25 — 10 open

- Clean-tree `scripts/release.ps1` with every exit code recorded and no OOM or
  skipped step.
- Full Rust/frontend suites plus bindings, capabilities, redaction and
  no-network gates.
- Installed-RC product loop: observe → timeline → investigate → explain
  evidence → checkpoint/compare → diagnostics/privacy.
- Performance, accessibility and soak gates, with D-15's two memory figures
  and neither figure as a gate.
- Full/scoped purge logical and byte-scan guarantees, including FTS/WAL/SHM.
- 10k upgrade preservation and uninstall history behavior.
- Canonical MIT license, audited notices, support and security disclosure.
- English wiki, Vietnamese article, logo/media kit and README matching the RC.
- Unsigned-installer disclosure and zero runtime network bytes in acceptance.
- Explicit human approval after review of the complete go/no-go packet.

## Release decision and next actions

The packet at `docs/public-v1-go-no-go.md` correctly remains **NO-GO**. The
preserved installer/checksum pair predates later migrations/source changes and
is not a current release candidate. The next executable sequence is: finish
the P09 low-load timing rerun; finish P19 installed/manual evidence; close P20
current-source performance/resource/soak/accessibility gates; build and install
the current RC for P21–P24 parity checks; then assemble P25's frozen manifest
and obtain the single explicit human approval. Do not mark a row complete from
source-only evidence when its phase requires an installed, manual, current-RC
or publication result.

## Unresolved questions

No new product decisions are required by this audit. The operational questions
are whether the pending serialized Cargo run passes, whether a sustained
low-load P09 process p95 rerun passes, and when a human will execute and approve
the current installed-RC and publication packet.

Status: DONE_WITH_CONCERNS
Summary: Current 25-phase state reconciled at 312/369 complete with 57 open; exact remaining gates and evidence contradictions recorded. Public v1 remains NO-GO.
Concerns/Blockers: Current combined Cargo result, P09 timing, P19 installed/manual checks, P20 performance/resource/soak/a11y, P21 current RC acceptance, P23 license/notice review, and P25 human approval remain unresolved.
