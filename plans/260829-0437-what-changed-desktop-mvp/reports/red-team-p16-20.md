# Red team — phases 16-20

## Summary

Provenance: findings `F-P16-*` were produced by the `rt-p16-20` subagent, which covered phase 16
thoroughly and then died on a stream-idle timeout before reaching 17-20. Findings `F-C*` below were added
by the controller to finish the slice. Nothing from the subagent was edited.

This slice is the strongest-written part of the plan and still carries several defects that would each
cost a day. Phase 16 is unusually disciplined — instanced rendering, a pure layout function, a real 2D
fallback rather than a placeholder, DPR-correct picking, a 20-cycle dispose gate — and yet its dispose
gate is green on a fully leaking implementation (F-P16-C), its lazy-loading story cannot work as designed
(F-P16-B), and the ten lane colours it renders with do not exist in the token set that defines them
(F-P16-E). Phase 18's `purge_all` drops and recreates the schema while collectors and the batched writer
are still running, never invoking the pause/quiesce machinery phase 08 built for exactly that (F-C1).
Phases 17 and 20 are honest about what they cannot verify, which is rarer than it should be.

The most dangerous finding in this slice was reached independently twice: the subagent's **F-P16-A** and
the controller's **Y-6** in `red-team-coverage.md` are the same defect from opposite directions — §48
System Map has a sidebar destination and a `get_system_map` command but no implementing phase. The
coverage report adds the part that makes it lethal: three phases mis-cite §48 as "the privacy screen", so
phase 20's §1–§91 traceability audit will report it covered.

Two checks that did **not** become findings, recorded so nobody re-litigates them: dev-route stripping is
owned (`phase-02:148` — `import.meta.env.DEV` guards **plus** a route-table filter), and phase 15's
downsampler does exist (`analysis/downsample.rs`) — though see F-P16-D for what it cannot do.


## Findings

### F-P16-A — "System Map" sidebar destination has no owning phase  [HIGH]
- **Where:** phase-16-three-js-visualization.md:108-113 ("Related Code Files → **Modify**") vs phase-02-design-system-app-shell.md:28
- **Category:** 6 — missing dependency / 10 — internal contradiction
- **Defect:** P02 ships "Sidebar nav with seven destinations: Home, Timeline, Investigations, Checkpoints, **System Map**, Settings, Diagnostics", and plan.md's frontend layout declares `three/{engine,ambient,causal-graph,system-map}` plus `components/{...,graph,system}`. P16 is the only phase that imports `three`; it creates `src/three/changeMap/*` + `src/components/visualization/*` only, and its Modify list touches `InvestigateScreen.svelte`, `TimelineToolbar.svelte`, `SettingsScreen.svelte`, `commands/mod.rs`, `domain/settings.rs`, `DevStatesHarness.svelte`. Nothing creates a System Map screen or route. A grep of all 20 phase files for `SystemMap|features/system|#/system|system map` returns only P02:28 and P16's own title.
- **Why it bites:** the implementer ships a seventh nav entry that routes to nothing. G5's "all six states on every screen" and P19's route sweep then have to cover a screen that no phase specifies, and plan.md G7's "ambient field, causal graph, system map" is backed by exactly one of the three named surfaces.
- **Fix:** add to P16 Requirements a third entry point — the `#/system` destination renders `ChangeMapFrame` over a default "last 24 h, all categories" window — and add `src/features/system/SystemMapScreen.svelte` + the router case to P16's **Create**/**Modify** lists. If the sidebar item is meant to be this map under another name, say so in P16 and change P02:28's label to match.

### F-P16-B — P19's "dynamic import for the map route" cannot work against P16's entry-point design  [HIGH]
- **Where:** phase-16-three-js-visualization.md:23-25 ("Entry points: (1) an investigation's \"View as map\" action …; (2) the Timeline's \"Map\" toggle") and :88-90 ("`ChangeMapFrame.svelte` owns capability detection … it is what features import") vs phase-19-performance-accessibility-hardening.md:106-107 ("`src/three/changeMap/createScene.ts` (lazy import boundary), `src/app/router.ts` (dynamic import for the map route)")
- **Category:** 10 — internal contradiction
- **Defect:** P16 makes the map a child component of two always-reachable screens and specifies static imports throughout (the only import gate is `rg -n "from 'three'" src --glob '!src/three/**'`, which a static import inside `src/three/**` satisfies). P19 then plans to split `three` out at the *router* level for a "map route" that P16 never creates (see F-P16-A). A router-level dynamic import cannot remove `three` from the initial chunk when the importers are `TimelineToolbar.svelte` and `InvestigateScreen.svelte`.
- **Why it bites:** P19 commit "(h) `perf: lazy load three on the map route`" has nothing to attach to; `three` 0.185.1 stays in the startup chunk, so every launch parses it and the `check:bundle` gate P19 introduces is either red or set to a number that hides it. G7's "never required for core use" stops being true at the bundle level, and startup/RAM budgets absorb the cost.
- **Fix:** state in P16 that `ChangeMap.svelte`/`ChangeMap2D.svelte` are reached only through an `await import(...)` boundary owned by `ChangeMapFrame.svelte` (rendering the already-required loading state while the chunk arrives), that no module under `src/three/` may be statically imported by an always-mounted screen, and extend the import-boundary test to assert exactly that (static `three` import allowed only in files reachable from the dynamic boundary).

### F-P16-C — the dispose gate's "hard assertion" cannot fail  [HIGH]
- **Where:** phase-16-three-js-visualization.md:74-78 ("A mount/unmount×20 test asserts `renderer.info.memory.geometries === 0` for a fresh renderer and that JS heap does not grow monotonically (heap check is advisory; the geometry/texture counters are the hard assertion)") and Success Criteria :167-168
- **Category:** 5 — unfalsifiable success criterion
- **Defect:** `renderer.info.memory` is per-renderer state that only counts objects that renderer has uploaded. A *fresh* renderer reports `geometries === 0` before it renders anything, whether or not the twenty previous scenes disposed anything. The test runs "in jsdom with a stubbed WebGL context" (step 10), where nothing is uploaded at all, so the counter is 0 unconditionally.
- **Why it bites:** the phase's own answer to the "GPU/VRAM leak across route changes" risk is "Dispose test with 20 cycles is a build gate". That gate is green on a fully leaking implementation, and the leak then shows up as growing RSS in P19's memory measurement with no test pointing at the cause.
- **Fix:** make the assertion behavioural: have `dispose.ts` own an explicit disposables registry, spy every geometry/material/texture `createScene` constructs, and assert each `.dispose()` called exactly once, `renderer.dispose()` / `renderer.forceContextLoss()` / `setAnimationLoop(null)` each called once per cycle, and per-target `addEventListener` count === `removeEventListener` count. Demote `renderer.info` to advisory.

### F-P16-D — the 3000-node cap is credited to a downsampler that caps at 5000 and may not drop chained events  [MEDIUM]
- **Where:** phase-16-three-js-visualization.md:65-66 ("max 3000 nodes and 6000 edges. Above that, the payload is aggregated server-side by the existing P15 downsampler and the map shows a chip \"Showing the 3,000 most important changes\"") vs phase-15-analysis-engine-investigation.md:181 ("event fetch with the 5000 cap, `downsample.rs` dropping `informational` first") and :259 ("Downsampling drops `informational` first and *never* drops an event with a structural edge or a `failure_proximity` contribution")
- **Category:** 6 — missing/wrong dependency
- **Defect:** the P15 downsampler targets 5000, not 3000, and is contractually forbidden from dropping structurally-linked or failure-proximate events. It therefore cannot deliver ≤3000 nodes for a window whose survivors exceed 3000, and P16 defines no selection rule of its own — yet its chip asserts these are "the 3,000 most important changes".
- **Why it bites:** `get_change_map` either returns >3000 nodes (breaking the phase's own risk response "Hard 3000-node cap enforced in Rust (the frontend cannot request more)" and the 60 FPS budget) or violates P15's never-drop invariant, which P15's `large-window` fixture asserts.
- **Fix:** give `analysis/change_map.rs` an explicit cap of its own — rank by (structural-edge membership, severity, `|t − symptom_time|`), keep the top 3000, and add a `change_map_payload.rs` case with 5000 chained events asserting `nodes.len() == 3000` with the top chain intact. Change the chip text to "Showing 3,000 of N changes".

### F-P16-E — category colours are neither defined as tokens nor tested, and parse failure is silent  [MEDIUM]
- **Where:** phase-16-three-js-visualization.md:32-33 ("colour comes from the §41 token palette by category (read from CSS variables at scene init so theme switches stay consistent)") and step 3 :123-124 ("read the §41 CSS variables via `getComputedStyle(document.documentElement)` once and map category → `THREE.Color`") vs phase-02-design-system-app-shell.md:20-23 (token list)
- **Category:** 8 — silent-failure design
- **Defect:** P02's token set is semantic — `background`, `surface-1/2/3`, `border-*`, `text-*`, `accent`, `success`, `warning`, `danger`, `info`, shadows, radii, spacing, motion — and contains no per-category colour for the ten lanes P16 needs (process, port, config, service, application, update, driver, network, startup, system). P16 also never states the colour syntax the tokens use, and specifies no behaviour when `getComputedStyle` returns an empty string or a value `THREE.Color` cannot parse. The only colour test in the phase toggles the theme class.
- **Why it bites:** a missing `--category-*` variable or a colour written in a function `THREE.Color` does not accept yields the default instance colour plus at most a console warning; the map renders monochrome, lanes become unreadable, and the theme-toggle test still passes because both themes resolve to the same default.
- **Fix:** name the ten variables in P16 (`--map-category-<name>`), require them in `src/styles/tokens.css` in a syntax `THREE.Color` accepts (hex or `rgb()`), and add `palette.test.ts` asserting all ten resolve to distinct non-default colours and that an absent/unparseable value throws in dev and falls back to a documented constant in release.

### F-P16-F — the keyboard-equivalence criterion passes without delivering usable access  [MEDIUM]
- **Where:** phase-16-three-js-visualization.md:56-57 ("keyboard `Tab` focuses a hidden ordered list of nodes with `Enter` selecting one … so a keyboard user can reach every node") and Success Criteria :172-173 ("Keyboard: `Tab` reaches the node list, `Enter` selects")
- **Category:** 5 — unfalsifiable success criterion
- **Defect:** with a 3000-node cap, "Tab focuses a hidden ordered list of nodes" means up to 3000 sequential tab stops between the map and the next control, and the stated criterion ("Tab reaches the node list, Enter selects") is satisfied by the first item alone — it proves nothing about traversal or escape. "Hidden" is also unqualified: `display:none`/`hidden` removes focusability outright, so the criterion can be met only by a visually-hidden-but-focusable pattern the phase does not name.
- **Why it bites:** §62 equivalence is claimed and gated, but a keyboard user who Tabs into the map cannot get out of it in reasonable time; a reviewer running the stated check sees green.
- **Fix:** specify `MapNodeList` as a single focusable `role="listbox"` with roving `tabindex` (one tab stop, Arrow/Home/End to move, `Enter` to select, `Esc` to leave), built with the project's visually-hidden utility, and restate the criterion as: one Tab stop enters the list, Arrow traverses, `Esc` returns focus to the map control, and the next Tab leaves the map region.

### F-P16-G — reduced-motion behaviour is specified two incompatible ways  [LOW]
- **Where:** phase-16-three-js-visualization.md:18-19 ("A full 2D canvas fallback ships in the same phase for machines without usable WebGL **and for `prefers-reduced-motion` users**") vs :58-59 and :60-64 (Fallback triggers list WebGL creation failure, context loss, and the `visualization: "2d"` setting only; reduced motion instead "disables camera damping/auto-fit animation and the render loop becomes on-demand")
- **Category:** 10 — internal contradiction
- **Defect:** the Overview promises 2D for reduced-motion users; the Requirements keep those users in the 3D renderer with animation disabled. The success criterion (:171) only tests the on-demand loop, so the Overview's promise is verified by nothing.
- **Fix:** delete "and for `prefers-reduced-motion` users" from the Overview, or add reduced-motion to the Fallback trigger list and assert it in `change-map-frame.test.ts`.

### F-P16-H — the verification gate ends in a process that never exits  [LOW]
- **Where:** phase-16-three-js-visualization.md:184 ("`bun run tauri dev   # manual: #/dev/perf frame timing, forced 2D, 900x600`")
- **Category:** 7 — verification gate that cannot pass as written
- **Defect:** the gate block is presented as runnable but its last command is a long-running dev server, so the block has no exit status; three success criteria (median frame ≤ 16.6 ms, forced 2D, 900×600) hang off that manual step with no artifact recording the result.
- **Fix:** split the block into a scripted gate (the first four commands) and a `## Manual checks` list,
  and require the measured median frame time to be written to `docs/PERFORMANCE.md` with the GPU name, as
  the plan already does for memory.

### F-C1 — `purge_all` drops and recreates the schema while collectors are still writing  [HIGH]
- **Where:** phase-18-onboarding-notifications-diagnostics.md — Privacy screen requirements ("a typed
  confirmation (\"delete\") that drops and recreates the schema") and Implementation step 3 ("full purge
  recreates the schema at the current migration version and runs `VACUUM`")
- **Category:** 8 — silent-failure design / 6 — missing dependency
- **Defect:** nothing pauses the collector runtime or drains the batched writer before the schema is
  dropped. Phase 08 built exactly this machinery — "Cancel token + pause flag … the isolation test asserts
  ≤ 1 s to quiesce" — and phase 18 never invokes it. The phase's risk row covers a different problem (a
  stale WAL) with `wal_checkpoint(TRUNCATE)` + `VACUUM`, not the concurrency one.
- **Why it bites:** twelve collectors and a batched writer are live when the user types "delete". A batch
  in flight across the drop either hits a missing table — and by the plan's own no-silent-drops rule that
  failure must surface, so the user sees an error immediately after a successful deletion — or it lands
  after the recreate, and the freshly emptied database is no longer empty while the UI says "Your history
  has been deleted. We're watching again from now." The second outcome is the bad one: the privacy promise
  is the product, and the screen would be lying by a few rows.
- **Fix:** make the sequence explicit in phase 18 — pause monitoring via the P08 pause flag, await the
  ≤ 1 s quiesce, drain the writer queue, then `wal_checkpoint(TRUNCATE)` → drop/recreate → `VACUUM` →
  resume. Add a success criterion asserting zero rows immediately after purge **with collectors enabled**,
  which is the case that currently fails, and note that `purge_category` needs the same treatment.

### F-C2 — two phases each claim to own "the only" execution surface, describing different things  [MEDIUM]
- **Where:** phase-18-onboarding-notifications-diagnostics.md — Privacy screen "Verify for yourself" ("the
  `opener` plugin scoped to exactly the app-data directory — the only shell-adjacent capability in the
  app") vs phase-17-docker-development-mode.md Security Considerations ("Process spawning is the only
  place the app executes anything")
- **Category:** 10 — internal contradiction / 3 — inaccurate security claim
- **Defect:** phase 17 runs first and spawns up to fifteen executables via `std::process::Command` for
  toolchain version probing. Phase 18 then calls a scoped Explorer reveal "the only shell-adjacent
  capability in the app".
- **Why it bites:** phase 18 owns the privacy narrative and feeds `PRIVACY.md` and the privacy screen. Its
  claim is false in the direction that matters — revealing a folder is strictly less powerful than
  spawning binaries resolved from `PATH`. A threat model that understates its own execution surface is
  worse than one that omits it, because a reader stops looking.
- **Fix:** phase 18's wording becomes "the only shell-adjacent capability **exposed to the frontend**",
  which is both accurate and the property that matters for IPC, and cross-references phase 17's
  timer-driven, allowlisted, non-IPC-reachable spawn surface as the other execution path. Both sentences
  then agree and `SECURITY.md` inherits a complete picture.

### F-C3 — the app's own toolchain spawns will appear in its own timeline; the exclusion has no owner  [MEDIUM]
- **Where:** phase-17-docker-development-mode.md Risk Assessment ("Self-events from these spawns are
  tagged and excluded from the timeline") vs phase-17's Related Code Files, and phase-09-process-port-collectors.md
- **Category:** 6 — missing dependency
- **Defect:** the mitigation is asserted in a risk table and implemented nowhere. No tag field, no filter,
  no artifact in phase 17's Create/Modify lists, and no mention in phase 09 — which owns the process
  collector that will observe the spawns. A grep for `self-event|self_event` across all 20 phase files
  returns exactly this one risk row.
- **Why it bites:** every 6 hours, and on every PATH change, the app spawns up to fifteen processes and its
  own process collector records fifteen start/stop pairs. On a calm day that is plausibly the largest
  single contributor to the timeline, generated by the app itself — the precise opposite of §54's "the
  product should feel calm", and it will read as a collector bug rather than a designed omission.
- **Fix:** name the mechanism. The cheapest correct one: `tool_probe.rs` registers each spawned PID in a
  short-lived set that the process collector consults before emitting, with the set owned by the P08
  runtime so both collectors can see it. Add it to phase 17's Create list, add one assertion to
  `collector_dev_env.rs` that a probe cycle produces zero `process/*` events, and reference it from phase
  09 so a future reader of that phase knows the filter exists.

### F-C4 — the Docker event stream is a blocking read with no thread to block  [MEDIUM]
- **Where:** phase-17-docker-development-mode.md — transport trait (`fn stream(&mut self, path: &str) ->
  Result<Box<dyn BufRead>>`) and Non-functional ("Docker event stream costs ~0 when idle (blocking read)")
- **Category:** 6 — missing dependency / 8 — silent-failure design
- **Defect:** `BufRead` is a synchronous interface and the phase states the idle cost is achieved by
  blocking on it. The collector runtime is Tokio-based. Nothing in the phase says the stream runs on a
  dedicated OS thread or under `spawn_blocking`, so as written the collector parks a Tokio worker
  indefinitely.
- **Why it bites:** the runtime has a bounded worker pool. One collector parked forever in a blocking pipe
  read removes a worker permanently; with the pool sized to the core count, a few such parks degrade every
  other collector's scheduling, and the symptom surfaces as unrelated collectors missing their cadence — a
  diagnosis that costs hours because the cause sits in a phase nobody is looking at.
- **Fix:** state it in phase 17: the Docker stream owns a dedicated `std::thread` that pushes decoded
  events into the same `mpsc` the other collectors use, and the async side only awaits that channel. One
  sentence in Architecture, one line in the state-machine step. The `ScriptedTransport` test path is
  unaffected because it is synchronous either way.

### F-C5 — phase 19 never says which build each number is measured on, and two of them cannot share one  [MEDIUM]
- **Where:** phase-19-performance-accessibility-hardening.md — Frontend gates ("60 FPS scroll on a 100k
  timeline (measured with a `#/dev/perf` harness …)", "initial JS bundle ≤ 400 KB gzipped excluding
  `three`") and the Verification Gate
- **Category:** 7 — verification gate that does not test the claim
- **Defect:** the frame-timing numbers come from `#/dev/perf`, which `phase-02:148` correctly guards behind
  `import.meta.env.DEV` **plus** a route-table filter — so the harness exists only in a dev build. The
  bundle-size gate measures `bun run build` output, which is the release bundle. The phase presents both as
  one measurement pass and never states the build provenance of any number.
- **Why it bites:** a dev bundle is unminified, HMR-instrumented and source-mapped, so its scroll frame
  times are systematically worse than the shipped build's. Measuring there is *conservative* for FPS
  (passing dev implies passing release), which is fine — but nobody knows that from the phase, so a
  failing dev number will be argued about rather than re-measured, and the recorded baseline in
  `docs/performance-baseline.md` will be attributed to the wrong artifact. The cold-start and working-set
  budgets in the same phase are release-build properties, so the document ends up mixing three build
  provenances silently.
- **Fix:** add a provenance column to `docs/performance-baseline.md` and one sentence per gate in phase 19
  naming the build: FPS and map frames on a dev build (noted as conservative), bundle size / cold start /
  working set / DB size on the release build. No new work, and it makes every recorded number
  re-measurable.

### F-C6 — the diagnostics export has a size cap and content that can exceed it, with no truncation rule  [MEDIUM]
- **Where:** phase-18-onboarding-notifications-diagnostics.md — Diagnostics ("the last 500 log lines") and
  Non-functional ("`export_diagnostics` ≤ 300 ms and never larger than 512 KB"), asserted by the success
  criterion "size ≤ 512 KB"
- **Category:** 7 — verification gate that cannot pass as written
- **Defect:** 500 log lines plus versions, settings, per-collector status, counts by event type and the
  last 20 error entries are budgeted at 512 KB with no per-line cap and no truncation policy. A tracing
  line carrying a long path or a structured field set runs to several hundred bytes; 500 of them alone can
  approach or pass the cap before anything else is written.
- **Why it bites:** the phase asserts the cap as a test. On a machine that has logged verbose lines the test
  fails for a reason unrelated to the code under test, and the tempting fix — raising the number — silently
  weakens a bound that exists to keep an export pasteable into a bug report.
- **Fix:** state the truncation rule: per-line cap (e.g. 512 bytes, ellipsised) and a total log-section cap,
  with the export writing "N lines omitted to stay within the size limit" when it trims — visible
  truncation rather than a silent cut, consistent with how the rest of the plan treats dropped data. Then
  the 512 KB criterion is enforceable by construction.

### F-C7 — phase 19 attributes the dev-route removal to a mechanism that cannot do it  [LOW]
- **Where:** phase-19-performance-accessibility-hardening.md Security Considerations ("The perf harness
  route lives under `#/dev/*`, which is compiled out of release builds along with the rest of the dev
  harness")
- **Category:** 10 — internal contradiction
- **Defect:** "along with the rest of the dev harness" points at the `devtools` **cargo** feature, named in
  the two bullets above it. A cargo feature cannot remove a Svelte route. The real mechanism is
  `phase-02:148`'s `import.meta.env.DEV` guard plus the route-table filter, which is a different phase and
  a different build system.
- **Why it bites:** low on its own, but phase 20's release audit asserts the built frontend assets contain
  no `#/dev/` route. If that assertion ever fails, this sentence sends the implementer to `Cargo.toml`
  instead of the router. The correct owner exists; only the attribution is wrong.
- **Fix:** one clause — "…which the P02 router removes from production builds via its `import.meta.env.DEV`
  guard and route-table filter, independently of the `devtools` cargo feature."

### F-C8 — `powerPreference: 'low-power'` makes the measured frame budget unrepresentative  [LOW]
- **Where:** phase-16-three-js-visualization.md step 4 (`powerPreference: 'low-power'`) vs the 16.6 ms
  median-frame budget "on this machine"
- **Category:** 2 — measurement that does not generalize
- **Defect:** the budget is measured on this machine, whose only GPU is the Intel Iris Xe recorded in the
  research matrix, so `low-power` selects the same adapter either way and the number is honest here. On a
  hybrid-GPU laptop `low-power` asks for the integrated adapter, so a user with a discrete GPU can be
  *slower* than the reference machine rather than faster.
- **Why it bites:** it inverts the usual assumption that the developer's machine is the floor. Not a defect
  in the choice — `low-power` is right for a tray app that must not spin up a discrete GPU — but the
  recorded budget should not be presented as a worst case.
- **Fix:** record the GPU alongside the measured median in `docs/performance-baseline.md` (F-P16-H already
  asks for this) and add one line noting that `low-power` means integrated-GPU performance is the target
  class, not a lower bound.

## Phase 20 findings recorded elsewhere

Phase 20 was reviewed by the controller before this slice was taken over, so its findings live in the two
reports written first. Listed here so this file is complete for 16-20:

| ID | Sev | Report | Defect |
|---|---|---|---|
| Y-1 | HIGH | `red-team-coverage.md` | Installer artifact name (`What Changed_<version>_x64-setup.exe`) contradicts `plan.md:107`'s own release criterion (`WhatChanged-Setup-x64.exe`), making that checkbox unsatisfiable |
| Y-2 | MED | `red-team-coverage.md` | §65's Start Menu and optional desktop shortcuts have no owner in any phase |
| Y-3 | MED | `red-team-coverage.md` | §66's development-vs-production signing separation is unaddressed |
| X-1 | HIGH | `whole-plan-consistency-gate.md` | `bollard` is pinned, unused, and pulls in `hyper` — which phase 20's own `check-no-network.mjs` gate fails on |
| X-3 | MED | `whole-plan-consistency-gate.md` | `tauri-plugin-updater` is pinned although D-09 ships no updater, and `phase-05` documents the `updater:` capability rejection as temporary "until P20", which never lifts it |

## Slice tally

12 findings in this file (8 subagent, 4 controller) plus 5 cross-referenced for phase 20 = **17** across
phases 16-20. Severity: 4 HIGH in-file (F-P16-A/B/C, F-C1), 6 MEDIUM, 2 LOW, and of the cross-referenced,
2 HIGH and 3 MEDIUM.

Status: DONE_WITH_CONCERNS
Summary: phase 16 is well engineered and its two build gates (dispose, bundle size) are both currently
incapable of failing; phase 18's full purge races the live write path; phases 17 and 20 are honest about
their unverifiable surfaces. The System Map gap (F-P16-A / Y-6) is the one that must be resolved before
implementation starts, because it changes the phase count.
Concerns: phase 19's accessibility requirements were reviewed only for internal consistency, not against
WCAG clause by clause — a Narrator pass is specified in the phase itself and cannot be simulated here.


