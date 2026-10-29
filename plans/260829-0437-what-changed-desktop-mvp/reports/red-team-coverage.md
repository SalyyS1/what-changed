# Red team — requirement coverage §1–§91

Controller-authored after the `rt-coverage` subagent died on a stream-idle timeout. Rather than
re-spawn the same oversized task, the question was attacked mechanically, which turns out to be the
better instrument for it.

## Method, and exactly what it does and does not prove

1. **Mechanical citation extraction.** `§\d+` matched across `plan.md` and all 20 phase files — 254
   citations. This is ground truth for traceability: a requirement with no citation anywhere has no row
   for phase 20's §1–§91 audit to stand on, and P20's own rule is that "any requirement whose evidence
   cannot be named is a **blocking finding**".
2. **Adjudication of every uncited requirement.** For each one, the evidence packet's own `### §N`
   heading was read to establish what it demands, then the plan was searched for the substance under a
   different name. This is where "silently dropped" would surface.

**What this does not prove:** for the 77 requirements that *are* cited, this method confirms a named
phase claims them — not that the claim is deep enough. Depth is the four phase-slice red-team reports'
job (`red-team-p01-05`, `-p06-10`, `-p11-15`, `-p16-20`). Read them together with this. The full
91-row Full/Partial/Missing table the original task specified is therefore **not** delivered here; what
is delivered is the complete answer to "is anything unowned", which was that table's purpose.

## Citation map — which phase claims which requirement

| Phase | § cited |
|---|---|
| 01 Foundation | 4, 55, 68, 69, 86, 90 |
| 02 Design system | 3, 40, 41, 63, 70, 72, 80 |
| 03 Redaction | 4, 13, 34, 60, 82 |
| 04 Persistence | 4, 9, 10, 11, 26, 50, 54, 82, 83 |
| 05 IPC/tray | 7, 11, 13, 24, 35, 47, 48, 50, 56, 57, 58, 64, 69, 81, 82, 83, 84, 85, 87 |
| 06 Event engine | 1, 10, 44, 52, 53, 54, 69, 72, 82, 85, 87 |
| 07 Timeline UI | 5, 43, 44, 46, 51, 52, 57, 69, 70, 72, 87 |
| 08 Collector runtime | 5, 6, 33, 49 |
| 09 Process/port | 11, 15, 53 |
| 10 File/config | 12, 13, 56, 60, 87 |
| 11 Service/app | 5, 14, 17 |
| 12 Update/driver | 5, 16, 19, 53, 54, 73 |
| 13 Network/startup/eventlog | 1, 18, 20, 21, 48, 53, 54 |
| 14 Checkpoints | 7, 23, 24, 25, 48, 50, 51, 58, 60 |
| 15 Analysis engine | 4, 27, 28, 29, 30, 31, 32, 45, 47, 50, 59, 74 |
| 16 Three.js | 4, 35, 39, 41, 62, 74, 81 |
| 17 Docker/dev | 33, 49 |
| 18 Onboarding/notify/diag | 26, 33, 42, 48, 49, 50, 53, 54, 58, 70, 72 |
| 19 Perf/a11y | 61, 62, 63 |
| 20 Installer/release | 26, 75, and the §1–§91 range |
| `plan.md` only | 36, 37 |

## The 14 requirements cited nowhere, adjudicated

`§36` and `§37` are cited in `plan.md` but in no phase file — real coverage, thin traceability. The
remaining 14 are cited nowhere at all. Each was checked against its packet heading:

| § | Requirement | Verdict | Where the substance actually lives |
|---|---|---|---|
| 2 | Audience — simple *and* advanced in one app | **Full** (uncited) | `plan.md` Overview: "for a non-technical user, with progressive technical depth for an advanced one"; P07's §43/§44/§46 details drawer is the progressive-depth mechanism |
| 8 | Core architecture chain | **Full** (uncited) | `plan.md` Architecture Summary reproduces the packet's chain shape exactly, including "The UI never touches an OS API" |
| 22 | Windows Event Log collector | **Full** (mis-cited) | P13 — four channels only, `EvtSubscribeToFutureEvents`, raw ids under technical details, `Security` never subscribed with a grep gate. But P13 attributes it to **§21**. See Y-4 |
| 38 | UI performance | **Full** (uncited) | P16 covers every clause: `dispose()` + `forceContextLoss()` on unmount with a 20-cycle build gate, `visibilitychange` pause asserted by a flat frame counter, `prefers-reduced-motion` stopping the loop, and a shipped tested 2D fallback on WebGL failure or `webglcontextlost` |
| 65 | Installer | **Partial** | P20 owns upgrade-preserves-data, uninstall-asks-first, and P18 owns launch-at-startup. Artifact name contradicts `plan.md` (Y-1); Start Menu and optional desktop shortcuts are unowned (Y-2) |
| 66 | Code signing | **Partial** | P20 documents what production signing needs and refuses to fake a publisher; no signing secret is ever introduced. "Development signing separate from production signing" is unowned (Y-3) |
| 67 | Development commands | **Full** (uncited) | P01:24 defines `dev, build, check, test, tauri, rs:fmt, rs:lint, rs:test, verify, mem`; all seven §67 invocations resolve, and `tauri` covers both `tauri dev` and `tauri build` |
| 71 | Window responsiveness | **Full** (uncited) | P02 desktop-density layout with a 1040px container query; P19 asserts 900×600 at Windows 150% scaling and 200% zoom; `plan.md` carries the 900×600 success criterion |
| 76–78 | Future features | **Deferred (user)** | The packet groups §75–§78 under one "architect for, do **NOT** build" heading; P20 cites §75 as the deferral and `plan.md` G9 carries the architecture-readiness commitment |
| 79 | Coding standard | **Full** (uncited) | `plan.md` success criterion — no `unwrap()`/`expect()` in non-test Rust enforced by clippy, `-D warnings` clean, TS strict with no undocumented `any`; P01 wires rustfmt/clippy and P02 owns CSS tokens |
| 88 | Verify after every phase | **Full** (structural) | All 20 phases carry a `## Verification Gate` with runnable commands. The plan's shape *is* the compliance |
| 89 | Commit-sized work | **Full** (structural) | All 20 phases carry `## Todo (commit boundaries)` plus a lettered commit list as the final implementation step |

**One requirement is Missing: §48, the System Map** — see Y-6. My first pass through this report said
"nothing is Missing", counting §48 as covered because four phases cite it. That was wrong, and wrong in
exactly the way this report warns about: I counted citations without checking what they were citing §48
*for*. Three of the four call it "the privacy screen". §48 is the System Map. Corrected below.

Everything else has an owner. Two (§65, §66) are partially owned, and the gaps in both are small and
nameable.

## Findings

### Y-1 — the installer filename contradicts the plan's own acceptance criterion  [HIGH]
- **Where:** `phase-20-installer-updater-release.md:27` vs `plan.md:107`
- **Requirement:** §65 — "Windows installer artifacts targeting `WhatChanged-Setup-x64.exe` and/or
  appropriate Tauri installer formats."
- **Plan state:** phase 20 specifies "Artifact naming: `What Changed_<version>_x64-setup.exe`" (Tauri's
  NSIS default). `plan.md:107` success criterion says "`WhatChanged-Setup-x64.exe` installs, upgrades
  over an existing install without touching `events.db`, and uninstalls cleanly."
- **Gap:** the phase produces one filename; the plan's release checkbox names a different one. The
  success criterion is unsatisfiable by the phase that owns it — and it fails on a string comparison, so
  it will be noticed at the last possible moment. Candidates 1, 3 and 5 all solved this the obvious way
  (build with Tauri's default, then copy to the §65 name and record both); the winner dropped the step.
- **Fix:** add to phase 20's Implementation Steps — after `bun run tauri build`, copy
  `src-tauri/target/release/bundle/nsis/*-setup.exe` to `dist-installer/WhatChanged-Setup-x64.exe` and
  record both names in the release audit. Keeps `plan.md` untouched and satisfies §65 literally.

### Y-2 — §65's Start Menu and desktop shortcuts are unowned  [MEDIUM]
- **Where:** `phase-20-installer-updater-release.md:22-26` (the NSIS config list)
- **Requirement:** §65 — "Support Start Menu shortcut, optional desktop shortcut, optional launch at
  startup, clean uninstall."
- **Plan state:** a `shortcut|Start Menu` grep across all 20 phase files returns only `Ctrl+1..7`
  keyboard shortcuts (P02), the Startup-folder autostart fallback (P05), and a Startup-folder test
  fixture (P13). Phase 20's NSIS key list is `installMode`, `displayLanguageSelector`, `installerIcon`,
  `headerImage`/`sidebarImage`, `license`, `compression` — no shortcut keys.
- **Gap:** two of §65's four "support" clauses have no owner. Launch-at-startup is covered (P18) and
  clean uninstall is covered (P20); the shortcuts are not. Candidates 1, 2 and 5 named them explicitly.
- **Fix:** add Start Menu shortcut and optional desktop shortcut to phase 20's NSIS requirements —
  schema-verified like every other key — plus one Success Criteria row asserting both exist after a
  test install.

### Y-3 — §66's dev-vs-production signing separation is unowned  [MEDIUM]
- **Where:** `phase-20-installer-updater-release.md:47-50`
- **Requirement:** §66 — "Development signing separate from production signing. Document how production
  Windows code signing should be configured."
- **Plan state:** phase 20 documents the unsigned reality, the exact SmartScreen dialog and click path,
  and what production signing would require (EV/OV cert, `signtool`/`tauri signer`). It says nothing
  about separating development from production signing.
- **Gap:** with no certificate on this machine the honest coverage is "documented as future work" — but
  it should be written down rather than silently absent, because §66 is one of the requirements P20's own
  traceability table must produce a row for.
- **Fix:** one line in phase 20's `docs/deployment-guide.md` scope: record that no development signing
  identity exists, that production signing would use a separate certificate and credential source, and
  that neither is ever committed (which the plan already guarantees by never introducing a secret).

### Y-4 — §22 is mis-cited as §21, so the Event Log collector has no traceability row  [MEDIUM]
- **Where:** `phase-13-network-startup-eventlog-collectors.md:14`
- **Requirement:** §22 is the Windows Event Log collector. §21 is the Startup collector ("new startup
  application, removed, enabled/disabled, important scheduled startup tasks").
- **Plan state:** phase 13 reads "and Windows event logs (§21) which are a *subscription*, not a sweep."
  §22 appears nowhere in the plan.
- **Gap:** the substance is covered well — but phase 20 walks §1–§91 and will find no evidence named for
  §22, which by its own rule is a blocking finding. A one-character citation error becomes a release
  blocker at the worst time.
- **Fix:** `phase-13:14` → "Windows event logs (§22)", or "(§21–§22)" since the sentence covers both.

### Y-5 — §36 and §37 are cited only in `plan.md`  [LOW]
- **Where:** `plan.md:31` (§36 privilege model), `plan.md:105` (§37 memory budget)
- **Gap:** both are genuinely covered — §36 by the whole non-elevated architecture, §37 by the B2
  measurements in P01/P05/P18/P19 — but no phase file names either number, so the traceability walk has
  to infer the link rather than read it.
- **Fix:** cite §36 in phase 08's collector-runtime privilege statement and §37 in phase 19's resource
  gates. Two citations, no behaviour change. (Note: §36's supporting sentence in `plan.md` also carries
  the miscount corrected by finding X-4 in the consistency gate.)

### Y-6 — §48 System Map has a nav entry and an IPC command but no phase builds it, and a mis-citation hides that  [CRITICAL]
- **Where:** `phase-02-design-system-app-shell.md:28`, `phase-05-typed-ipc-tray-lifecycle.md:26,36`, and
  the absence of any implementing phase. Mis-citations at `phase-13:180`, `phase-14:165`,
  `phase-18:12,69`.
- **Requirement:** §48 — an advanced interactive relationship graph over Applications, Processes,
  Services, Ports, Docker containers, Network adapters and Config files, answering "What depends on
  this?", and the groundwork for the future Impact feature. The packet heading is
  `### §48 System Map (advanced)`.
- **Plan state:** three artifacts point at a screen that is never built.
  - `phase-02:28` gives the sidebar "seven destinations: Home, Timeline, Investigations, Checkpoints,
    **System Map**, …".
  - `phase-05:26` puts `get_system_map` in the typed command surface, and `:36` tags it "§48".
  - `phase-16` is the only Three.js phase and opens by narrowing itself: "One visualization, one job."
    Its two entry points are an investigation's "View as map" and the Timeline's "Map" toggle. It never
    mentions the System Map, and its only `ambient` hit is `ambient + one directional light` — a
    Three.js light, not a screen.
  - A `system.?map|get_system_map` grep across all 20 phase files returns exactly the four lines above.
- **Gap:** a declared IPC command with no implementing phase, a nav destination with no screen behind it,
  and a numbered requirement with no owner. Under HOLD SCOPE this cannot be dropped — §48's "Do not make
  System Map necessary for normal users" is a UX constraint on prominence, not permission to omit it.
- **Why the mis-citation makes this worse:** `phase-13:180`, `phase-14:165` and `phase-18:12,69` all cite
  §48 while describing the **privacy screen** ("the §48 privacy screen lists these categories by name").
  So phase 20's §1–§91 traceability walk *will* find evidence named for §48 — pointing at the privacy
  screen — and mark it covered. The audit built to catch exactly this class of omission is defeated by a
  wrong number. This is the most dangerous single defect found so far, not because the screen is hard to
  build but because the plan's own safety net is wired to miss it.
- **Fix:** two parts, both needed.
  1. Correct the citations: the privacy screen is §33 + §50 territory, so `phase-13:180`,
     `phase-14:165` and `phase-18:12,69` should read §50 (or §33), not §48.
  2. Give §48 an owner. It is a real screen with a real Rust surface, so the honest options are: add a
     phase after 16 that builds it on the P16 renderer and the P04 `entity_relationships` table
     (the graph data already exists — this is a consumer, not new collection); or fold it into phase 16
     as a second entry point with its own success criteria and effort. Either way `get_system_map` gets
     an implementing phase, or it must be removed from phase 05's surface — a command that no phase
     implements is the "faked implementation" §87 forbids.

### Y-7 — goal G7 promises three Three.js surfaces; the plan builds one  [HIGH]
- **Where:** `plan.md:45` (G7) vs `phase-16-three-js-visualization.md:12`
- **Requirement:** §39 lists three recommended Three.js use cases — Home ambient visualization, causal
  graph, investigation transition — and §43 says a "Three.js ambient scene may exist behind/around hero
  content". Both are permissive about the ambient field.
- **Plan state:** but `plan.md`'s own goal G7 is not permissive: "Three.js used intentionally (**ambient
  field, causal graph, system map**) and never required for core use." Phase 16 delivers the causal graph
  only, and says so deliberately: "One visualization, one job."
- **Gap:** two of G7's three named surfaces have no phase. Against §39 the plan is defensible — one
  purposeful visualization beats three decorative ones, and P16 argues that case well. Against its own
  stated goal it is not: G7 as written cannot be satisfied by the 20 phases.
- **Fix:** pick one and make the plan consistent. Either narrow G7 to the causal graph plus the §48
  System Map once Y-6 gives it an owner (and note the ambient field as a §39-permitted omission with the
  reason phase 16 already articulates), or add the ambient field as scope. Narrowing G7 is the honest
  choice **only** because §39 phrases the ambient field as a recommendation rather than a requirement —
  the System Map from Y-6 has no such escape and must be built.


## Tally

| Verdict | Count |
|---|---:|
| Full — cited by a named phase | 74 |
| Full — covered, cited only in `plan.md` | 2 (§36, §37) |
| Full — covered, cited nowhere | 9 (§2, §8, §22, §38, §67, §71, §79, §88, §89) |
| Partial | 2 (§65, §66) |
| Deferred (user's own) | 3 (§76, §77, §78) |
| **Missing** | **1 (§48 System Map)** |
| | **91** |

§26 and §75 are cited as deferrals in phase 20 and counted in the 74 — they are the user's other
sanctioned deferrals, tracked rather than dropped.

Status: DONE_WITH_CONCERNS
Summary: Full: 85 | Partial: 2 | Missing: 1 | Deferred(user): 3. Seven findings; one CRITICAL — §48 System
Map has a sidebar entry and a `get_system_map` command but no implementing phase, and three phases
mis-cite §48 as "the privacy screen", which would make phase 20's §1–§91 audit report it covered.
Concerns: my first pass reported "0 Missing" by counting §48's citations without reading what they cited
it for; corrected above. Depth-of-coverage for the cited requirements is delegated to the four
phase-slice reports — this report proves ownership, not sufficiency.

