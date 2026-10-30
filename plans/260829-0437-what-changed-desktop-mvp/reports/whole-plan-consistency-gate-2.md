# Whole-plan consistency gate — second run, post-amendment

First run: `whole-plan-consistency-gate.md` (2026-09-03, **before** any amendment) —
11 findings, 3 blocking. This run re-audits the amended plan and reports what the
amendments actually closed, what they changed that the first run could not have seen,
and what remains open.

Method is the same instrument, mechanically: read `plan.md` and all 21 phase files, count
from the files rather than carrying numbers forward, and treat any claim the files
contradict as a finding. This run had a working shell, so the two things the first run
could not do — reindex and delete the tombstone stub — are done.

## 1. File inventory — PASS

- **21** phase files, `phase-01-foundation-toolchain.md` … `phase-21-installer-updater-release.md`,
  no gaps, no duplicates. `phase-17-system-map.md` is the new one (Y-6).
- Required headings: `Overview`, `Requirements`, `Architecture`, `Related Code Files`,
  `Implementation Steps`, `Todo`, `Success Criteria`, `Verification Gate`, `Risk Assessment`,
  `Security Considerations` — **exactly one of each in all 21 files** (210/210).
- Placeholder sweep (`@@…@@`, `<!-- CHUNK -->`, `<!-- APPEND -->`, `TODO:`, `TBD`, `FIXME`,
  `XXX`, `PACKET-APPEND`): **zero** hits in `plan.md` and all 21 phase files.
- `phase-01-start.md` (the tombstoned CLI stub the first run could not delete) is **gone**.
- Link integrity: every relative `.md` link in `plan.md` and the phase files resolves, and
  every link in the nine root documents resolves. The three dangling pointers to
  `phase-17-system-map.md` from `ARCHITECTURE.md`, `ROADMAP.md` and `plan.md` are live now
  that the file exists.

## 2. Frontmatter — PASS

All 21 carry `phase`, `title`, `status`, `priority`, `effort`, `dependencies`; all are
`status: pending`; all ids and dependency ids are zero-padded two-digit strings (C-0 closed).
Priority split is **18 × P1, 3 × P2** — 16 Change Map, 17 System Map, 18 Docker/dev mode.

## 3. Dependency graph — PASS, and now complete

```
01 ← []          08 ← [05]        15 ← [06,07,10,13,14]
02 ← [01]        09 ← [08]        16 ← [15]
03 ← [01]        10 ← [08]        17 ← [16]
04 ← [03]        11 ← [08]        18 ← [08,10]
05 ← [02,04]     12 ← [09,11]     19 ← [15,18]
06 ← [05]        13 ← [12]        20 ← [16,17,18,19]
07 ← [06]        14 ← [13]        21 ← [20]
```

Every edge points to a lower-numbered phase, so the graph is acyclic and execution order
equals numeric order. **The transitive closure of the terminal phase 21 is all 21 phases** —
the first run's reachability defect (06, 07 and 10 unreachable) is closed by C-1, and the
`20 → 17` edge that dangled while phase 17 did not exist now resolves.

Longest declared chain: `01→03→04→05→08→11→12→13→14→15→16→17→20→21` = 14 phases.

## 4. Arithmetic — PASS

Effort summed from frontmatter: **143 h** across 21 phases, matching `plan.md`'s frontmatter
and `ROADMAP.md`'s stated total. The 12 h increase over the materialized winner's 131 h is
7 h for phase 17 plus one hour each for phases 04, 10, 11, 20 and 21, which is the work the
findings added rather than a re-estimate.

## 5. Requirement ownership — PASS, 91 of 91

The first run did not cover this (it was `red-team-coverage`'s job, which reported
**1 Missing, 2 Partial, 11 covered-but-uncited**). Recounted mechanically here, excluding
references to the *research report's* own section numbers (`research §4.1`, `research §5`) so
they cannot be miscounted as brief citations:

| | First run | Now |
|---|---:|---:|
| Requirements with a phase owner | 77 | **91** |
| Cited only in `plan.md` | 2 (§36, §37) | 0 |
| Cited nowhere | 12 | **0** |
| Missing an implementing phase | 1 (§48) | **0** |

What closed the gap, in the order it mattered:

- **§48** — phase 17 built (Y-6 CRITICAL).
- **§18, §16, §14, §13, §12, §21, §20** — a chain of **off-by-two collector citations** that
  this run found and the first run could not, because the first run counted citations without
  reading what they cited. Phase 09 cited §11 (retention) for the process collector, which is
  §14; phase 11 cited §14 for the service collector, which is §16; phase 12 cited §16 for the
  Windows Update collector, which is §18; phase 10 cited §12 (collector contract) for the
  file collector, which is §13; phase 13 cited §18 and §20 for network and startup, which are
  §20 and §21. Each wrong number both created a false owner and left the real requirement
  unowned — the same defect class as Y-6, five more times. All five corrected, and §12 is now
  cited where it belongs, in phase 08's collector trait.
- **§2, §8, §38, §67, §71, §79, §86, §88, §89** — genuinely covered but cited nowhere.
  Now cited where the substance lives: §2 and §71 in phase 07 (progressive disclosure and
  desktop density), §8 in phase 05 (the IPC chain it describes), §38 in phase 16 (which
  implements every clause), §67/§79/§86/§88/§89 in phase 01.
- **§76–§78** — named explicitly in phase 21's deferral row instead of hiding inside a
  "§75 future features" shorthand.
- **§36, §37** — cited in phase 08 and phase 20 respectively; §37 now demands both memory
  figures at each checkpoint.

**One §48-shaped defect found and fixed in the other direction.** Four phases cited **§50
(Onboarding)** while describing the **privacy screen** and the delete-all control, which are
§33 + §49 territory — the identical failure mode as Y-6's "§48 privacy screen", just with a
different wrong number. Phases 13, 14, 15 and 19 corrected. §50 now has exactly three owners
(04 seeding the collector defaults it specifies, 05 for `complete_onboarding`, 19 for the flow
itself), all of which genuinely are onboarding.

## 6. Schema integrity — the first run's three blockers

**X-6 closed.** `collector_baselines`, `config_snapshots` and `investigations` are created in
phase 04's `001_initial.sql` (16 tables, not 13), each with its column list and the reason it
exists. `collector_state` — the table phase 12 reached for and no migration created — is
renamed to `collector_baselines` throughout phase 12, including its test binary name and its
risk row. §10's `analysis_sessions` and `analysis_results` are explicitly *not* left
created-and-unused: phase 04 records that they hold the per-run evidence behind the
denormalized `investigations` read model, which is what makes a past ranking auditable after
a weight change.

The systemic fix matters more than the three tables: phase 04's `tests/architecture.rs` gains
assertion (d) — **every table name appearing after `FROM`, `INSERT INTO`, `UPDATE` or
`DELETE FROM` in any `.rs` file must appear in a `CREATE TABLE` in `src-tauri/migrations/`**.
That is the check that would have caught all three at the phase that introduced them, and its
own success criterion requires it to be proven by a deliberately bad query.

**X-5 closed.** One migration directory (`src-tauri/migrations/`); phase 20's perf index is
`003_perf_indexes.sql` there, not in the `src/storage/migrations/` path that never existed;
phase 21's rehearsal is `004_test_upgrade.sql`; and phase 04 states the allocation rule — next
free integer, allocated by the phase that adds the migration.

**X-7 closed by measurement, not by choice.** The conditional migration `002` is gone. A probe
crate built against the pinned `rusqlite 0.40.2` with `features = ["bundled"]` on this machine
reports SQLite **3.53.2** with `ENABLE_FTS5` present, `CREATE VIRTUAL TABLE … USING fts5(…)`
succeeding, and all three query shapes §52's search needs returning rows against an
`content=''` external-content table: numeric token (`3000`), prefix (`port*`), quoted phrase
(`"0.0.0.0:3000"`). So `002_search_index.sql` ships one body. Recorded as **D-11**, with the
runtime `compile_options` check kept as a startup regression assertion rather than a branch.

Incidental finding worth its line, because it would otherwise be discovered as a bug:
`compile_options` does **not** list `ENABLE_JSON1`, yet `json_extract` and `json_each` both
work. JSON has been compiled in by default since SQLite 3.38, so a gate asserting
`ENABLE_JSON1` would fail on a build that fully supports JSON.

**X-4 closed.** `plan.md`'s Overview reads 31 of 35 with all four failures named and
categorized (two elevation-gated, one permitted-but-unreliable, one environment gap).

**X-1, X-2, X-3 closed.** `bollard`, `gix` and `tauri-plugin-updater` are in *Rejected on
purpose* with their reasons. Phase 05's capability test rejects `updater:` unconditionally —
the "added in P20 with a note" parenthetical is gone, which tightens a security gate rather
than relaxing one.

## 7. New findings from this run

### N-1 — `config_of` was required by P15 and written by nobody  [HIGH, fixed]

Phase 15 defines `Confirmed` — the strongest claim the product can make — as a structural
chain reaching a symptom-matched entity, and names four structural edge kinds:
P09's `listens_on`/`opened_by`, P11's `provides`, and **P10's `config_of`**. A whole-plan grep
for `config_of` returned exactly one hit: phase 15's own definition. Phase 10 emitted no
relationship rows at all.

The consequence is not abstract. The product's §74 worked example — `.env` `PORT` change →
service restart → port change — is a mandatory fixture asserting a `Confirmed` 3-step chain.
Without a `config_of` edge, that fixture returns `Likely` and the plan's own headline scenario
fails at the phase that owns it.

Fixed in phase 10: `config_of` rows are written on config-file events from **three evidence
paths only** — a registered project whose process was seen running from that directory, a
`PORT`-shaped key whose value matches a currently-listening port entity, and a service whose
`ImagePath` lives under the watched root. Each row carries the event id, so the edge is
inspectable in the System Map and citable in an explanation. A file that cannot be tied to
anything gets no row and its events rank on category and time alone — the honest outcome
rather than an invented edge. The success criterion asserts a row **count**, so an over-eager
heuristic fails the suite in the same way a missing one does.

### N-2 — five phases claimed decision ids that measurement had already assigned  [MEDIUM, fixed]

`docs/DECISIONS.md` has D-01 through D-07 recorded from measurements taken this session.
Phase files, written before those measurements, independently claimed **D-03** (bits-ui),
**D-04** (search index), **D-05** (retention exemption, twice), **D-06** (tray Pause/Resume)
and **D-07** (`ts-rs` fallback) — every one of which now means something else. A reader
following phase 02's "Decision D-03" lands on the version-single-source decision.

Fixed by renumbering the phase-file claims to D-10 through D-14 and, more importantly, by
making `docs/DECISIONS.md` the **allocator**: it now carries a reserved block naming
D-08 through D-14 with their owning phase and subject, and states that a phase reserves its
id there before using it. The first run's C-0 was the same class of defect one layer down
(two id formats in frontmatter); this is the id-collision problem for decisions.

### N-3 — the recorder's downtime blindness was recorded in `plan.md` but not wired into the phases  [HIGH, fixed]

`plan.md`'s risk R9 and its amendment log both described the persisted-baseline fix, and phase
04 now creates `collector_baselines`. But a grep for `occurred_at_is_uncertain` and
`collector_baselines` across the phase files returned **zero** hits outside `plan.md` and the
red-team report. The fix existed as a claim in the index and as a table in the schema, with no
phase actually using either.

Wired through, phase by phase:
- **P11** — the old rule ("the first cycle of each collector is a baseline only") is replaced:
  a baseline-only cycle happens in exactly two situations, never having run and a reset
  database, and every other start diffs against the persisted baseline. New criterion asserts
  a service removed and an application version bumped behind the collector's back are both
  reported on restart with the uncertainty flag.
- **P13** — network and startup persist and diff baselines; the event-log collector explicitly
  does **not**, because `EvtSubscribeToFutureEvents` means records written during downtime are
  genuinely gone, and `COLLECTORS.md` says so under Known limitations rather than implying
  coverage.
- **P10** — `config_snapshots` is the baseline, and a file whose mtime is newer than
  `captured_at` is re-parsed on registration.
- **P15** — an uncertain event scores `temporal = 0.0` **and** is exempt from the
  after-the-symptom exclusion, because its stored timestamp is a detection time that can
  neither earn recency credit nor rule the event out by ordering. It can still rank on
  relationship, category, severity and rarity, and it can never reach `Confirmed`, because
  `failure_proximity` is a time-window test with no trustworthy time to window against. Two
  fixtures cover both directions.

Without this, R9 would have been a documented intention with no implementing requirement —
the same shape as Y-6, one level of abstraction up.

### N-4 — §65's shortcuts and §66's signing separation were still unowned  [MEDIUM, fixed]

`red-team-coverage` raised Y-1, Y-2 and Y-3 and none had been applied. Phase 21 still specified
one installer filename (`What Changed_<version>_x64-setup.exe`) while `plan.md`'s release
criterion named a different one (`WhatChanged-Setup-x64.exe`) — a criterion unsatisfiable by
the phase that owns it, failing on a string comparison at the last possible moment. Fixed by
satisfying both: build with Tauri's name, copy to the §65 name, record both plus SHA-256 in
the release audit. Start Menu and optional desktop shortcuts added as schema-verified NSIS
keys with a named fallback mechanism, and §66's dev-vs-production signing separation recorded
in the deployment guide's signing section.

### N-5 — P21's no-network gate was unimplementable as specified  [HIGH, fixed]

Phase 21 required `scripts/check-no-network.mjs` to grep `Cargo.lock` for
`reqwest`/`hyper`/`ureq`/`isahc`. Measured on this machine: `reqwest` and `hyper` **are** in
`Cargo.lock`, because Tauri depends on them for non-Windows targets, and are **absent** from
the resolved Windows graph. The gate as written fails on a clean scaffold forever, which means
the first person to hit it disables it and the real guarantee leaves with it. Fixed: the gate
resolves `cargo tree --target x86_64-pc-windows-msvc --edges normal`.

### N-6 — P20 did not know phase 17 existed  [LOW, fixed]

Phase 20 declared `dependencies: [16, 17, 18, 19]` but its gate list, its lazy-import boundary
and its reduced-motion assertion all named only the Change Map. Extended: System Map frame
budget at its 400-node cap, the shared-module import boundary, one reduced-motion assertion
covering both maps, and — the one that matters for §48 — a test asserting **no Home, Timeline
or Investigations flow requires the `#/system-map` route**, which is what keeps §48's
"must not be necessary for normal users" true after the screen exists and becomes tempting to
link to.

## 8. Stale-term sweep

Searched all 22 plan files for terms the amendments should have retired. Every hit is either
in the amendment log or a red-team report describing what was fixed — which is where they
belong — or a phase deliberately naming the rejected option:

| Term | Hits | Verdict |
|---|---|---|
| `collector_state` | 0 outside `plan.md`'s amendment log | correct |
| `phase-17-docker`, `phase-18-onboarding`, `phase-19-perf`, `phase-20-installer` | 0 | correct |
| `phase-01-start` | 0 | correct |
| `src-tauri/src/storage/migrations` | 0 | correct |
| `20 phases` | 1, in the amendment log's "candidate D, 20 phases" | correct — historical fact |
| `34 of 35`, `the two measured gaps` | 1 each, in the amendment log quoting the miscount it fixed | correct |
| `bollard`, `tauri-plugin-updater` | in *Rejected on purpose* + the amendment log | correct |
| `typescript@7` | 1, in phase 01 step 6 explaining the measured failure | correct |
| `minify: 'esbuild'` | 1, in `plan.md`'s settled list explaining why it fails | correct |
| `Cargo.lock` | in the criterion explaining why a lockfile grep is *not* accepted | correct |

No stale term survives as an instruction.

## 9. Index state

`ak plan reindex --apply` run; `ak plan status` reports **0/21 phases done, 0/337 tasks** —
21 phases, matching the files. The first run could not do this (no shell), and its warning
about deleting the tombstone stub *before* reindexing was followed, so no phantom
completed phase 0 was registered.

## 10. What this gate still does not cover

Structure, frontmatter, graph shape, arithmetic, citation-to-subject matching, and stale
terms. It makes no judgement about whether a phase's technical approach is correct, whether
an unmeasured Win32 API behaves as assumed, or whether a success criterion is falsifiable in
practice. Depth-of-coverage for phases 01–05 in particular has **no completed red-team
report** — `red-team-p01-05.md` is a 126-byte stub, and the three follow-up slices
(`p02-03-05`, `p07-10`, `p13-15`) never produced output. Their failure notifications arrived
after this gate was written: all nine delegated agents (the three review slices and six
phase-authoring agents) died at spawn with `API Error: 400 Model "claude-opus-4.8" does not
exist or is disabled`. The API expects `claude-opus-4-8`; the dotted form is not in any agent
definition or settings file under `~/.claude/`, so the runtime supplied it at spawn. They never
ran. Every phase file and finding in this report was therefore
authored by the controller directly, which is why the review depth for 01–05 is the gap it is. That is the largest remaining gap
in the review, and it is a gap in *depth*, not in ownership: every requirement those phases
carry now has a named owner and a citation that matches its subject.

## Verdict

**PASS — 21 phases, 91 of 91 requirements owned, zero unresolved contradictions.**

All 11 findings from the first run are closed, including the three that blocked
implementation (X-4, X-5, X-6) and X-7, which was closed by measuring FTS5 rather than by
choosing between two paths. Six new findings were raised and fixed in this run; five of them
(N-1, N-3 and the citation corrections in §5) are the same defect class as the first
review's CRITICAL — a claim in one file resting on something no other file provides — which
suggests that class is the one to keep checking, not that it is exhausted.

Two blockers were settled empirically this session rather than argued: B1 (Rust installed,
`bun run verify` green, release build and NSIS installer produced) and the FTS5 question
behind D-11. B2 remains **open by design**: 361.7 MB total against 77.1 MB private working
set is a scope question for the user, and the plan's rule is that it is never resolved by
redefining the metric.

Status: DONE
Summary: 21 phases; 210/210 required headings; graph acyclic with all 21 reachable from the
terminal phase; 143 h consistent across `plan.md` and `ROADMAP.md`; 91/91 requirements owned
with every citation checked against what it claims to implement; 11 prior findings closed;
6 new findings raised and fixed.
Concerns: phases 01–05 have no completed depth red-team report (`red-team-p01-05.md` is a
stub), so their coverage is proven at the ownership level only. Four decisions genuinely need
the user: which memory figure §37 means, whether to rename the misspelled repo folder,
whether to install Docker for P18's gate, and confirmation that shipping both installer
filenames is the right resolution rather than editing the criterion.
