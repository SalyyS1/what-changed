# Whole-plan consistency gate — mechanical audit

Run by the controller against the materialized winner (candidate D), **before** any amendment.
Nothing in the plan has been edited as a result of this gate; findings are proposals, listed in §9,
to be applied together with the red-team findings under one amendment log.

Method: `Grep`/`Read` over `plan.md` and `phase-*.md`. No shell (see §8). Every number below is
counted from the files, not carried from the candidate document.

## 1. File inventory — PASS

- 20 phase files present, `phase-01-foundation-toolchain.md` … `phase-20-installer-updater-release.md`.
- Required headings: `Overview`, `Requirements`, `Architecture`, `Related Code Files`,
  `Implementation Steps`, `Todo`, `Success Criteria`, `Verification Gate`, `Risk Assessment`,
  `Security Considerations` — **200/200** occurrences, exactly 10 per file, no duplicates.
- `plan.md` phases table: 20 rows, IDs `01`–`20`, every relative link resolves to an existing file.
- Placeholder sweep (`@@…@@`, `<!-- CHUNK -->`, `<!-- APPEND -->`, `<!-- PACKET-APPEND-N -->`) over all
  `*.md`: **zero** hits in `plan.md` and all 20 phase files. Two hits remain in `reports/` — the known
  truncation marker in `planner-ultra-candidate-2.md:3252` and the verdict's quotation of it — both are
  historical artifacts of the ultra run, not plan defects.
- One stray file: `phase-01-start.md`, the tombstoned CLI scaffold stub. Still on disk (§8).

## 2. Frontmatter field presence — PASS

All 20 files carry `phase`, `title`, `status`, `priority`, `effort`, `dependencies`.
All 20 are `status: pending`. Priority split: **18 × P1**, **2 × P2** (16 Three.js, 17 Docker/dev mode) —
consistent with `plan.md`, which treats both as the deferrable-under-pressure pair.

## 3. Frontmatter format — FINDING C-0 (LOW, mechanical)

Two ID formats coexist, which matters because YAML parses them as different types
(`01` → string, `8` → integer) and `ak plan reindex` keys phases on that value:

| File | Field | Value | Expected |
|---|---|---|---|
| `phase-08-collector-runtime.md` | `phase` | `8` | `08` |
| `phase-08-collector-runtime.md` | `dependencies` | `[5]` | `[05]` |
| `phase-09-process-port-collectors.md` | `phase` | `9` | `09` |
| `phase-09-process-port-collectors.md` | `dependencies` | `[8]` | `[08]` |
| `phase-10-file-config-collector.md` | `dependencies` | `[8]` | `[08]` |
| `phase-11-service-application-collectors.md` | `dependencies` | `[8]` | `[08]` |
| `phase-12-update-driver-collectors.md` | `dependencies` | `[9, 11]` | `[09, 11]` |
| `phase-17-docker-development-mode.md` | `dependencies` | `[8]` | `[08]` |

Everything else is zero-padded two-digit. `plan.md`'s table is zero-padded, so padding is the
convention to normalize to.

## 4. Dependency graph — PASS on validity, FINDING on completeness

Declared adjacency, read from frontmatter:

```
01 ← []            08 ← [05]          15 ← [13,14]
02 ← [01]          09 ← [08]          16 ← [15]
03 ← [01]          10 ← [08]          17 ← [08]
04 ← [03]          11 ← [08]          18 ← [15]
05 ← [02,04]       12 ← [09,11]       19 ← [16,17,18]
06 ← [05]          13 ← [12]          20 ← [19]
07 ← [06]          14 ← [11]
```

Every edge points to a lower-numbered phase, so the graph is **acyclic** and execution order equals
numeric order. Every referenced phase exists. No orphan references. That part is clean.

The defect is reachability. The transitive closure of the terminal phase 20 is
`{01,02,03,04,05,08,09,11,12,13,14,15,16,17,18,19}` — **16 of 20**. Phases **06, 07 and 10 are not
reachable from phase 20**. An executor that trusts only the dependency graph could reach the release
phase without having built the event engine, the timeline UI, or the file/config collector — the three
things the product is mostly made of. The graph is valid; it is under-declared.

## 5. Substantive dependency gaps

**C-1 (HIGH) — phase 15 depends on 06, 07 and 10 in its own text but declares `[13, 14]`.**
- `phase-15…:69` — severity weights "from the P06 classifier".
- `phase-15…:71` — "Uses the P06 `prior_occurrences` query."
- `phase-15…:49` — "P10 `config_of`) — **structural**, the only edge class that can reach `Confirmed`."
  The single strongest confidence class the product can emit rests on a P10 artifact.
- `phase-15…:170-171` — Modify list: "(remove the P07 stub, route to Investigate)",
  `RelatedEvents.svelte` "(wire `get_related_events`)", "ROADMAP.md (clear the P07 stub entry)".
- Proposed: `dependencies: [06, 07, 10, 13, 14]`.

**C-2 (MEDIUM) — phase 17 modifies a P10 artifact but declares `[08]`.**
`phase-17…` Modify list includes `src-tauri/src/collectors/ignore_set.rs`, and Requirements read "for each
watched root (P10) that contains `.git`" and "Requires a narrow exception in the P10 ignore set for
exactly `.git/HEAD`". Proposed: `dependencies: [08, 10]`.

**C-3 (MEDIUM) — phase 14's own gate is unsatisfiable at its declared position.**
`phase-14…:95,119,154` assert the `diff_states` refactor keeps "P09–P13 suites still green" and that a
"P09–P13 test fails" is the observable signal. But 14 declares `[11]`, so phases 12 and 13 have not run
when 14 executes — their suites do not exist to be green. Two honest resolutions:
(a) `dependencies: [13]` — run the refactor after all five collector families exist; keeps 15's `[13,14]`
valid and makes 14's own Verification Gate true; or (b) narrow the claim to "P09–P11 suites" and add an
explicit "adopt `state_set`/`diff_states`" requirement to phases 12 and 13.
**Recommended: (a)** — it is one frontmatter line instead of edits to three phases, and a trait refactor
is cheaper once every implementor exists.

**C-4 / C-5 (LOW, absorbed by C-1) —** phase 18's onboarding step 3 uses the "folder picker (P10
command)" and phase 19 measures the P07 timeline at 100k and extends the P02 contrast test. Both inherit
06/07/10 transitively once C-1 lands; no separate edit needed. Recorded so the inheritance is deliberate
rather than accidental.

With C-1 + C-2 + C-3 applied, the closure of phase 20 becomes all 20 phases, and the graph stays acyclic
and backward-only (`14←13`, `15←06,07,10,13,14`, `17←08,10` are all backward edges).

## 6. Arithmetic and cross-file claims — PASS

- Effort, summed from frontmatter: 5+6+4+7+6 = 28; +6+8+5+6+7 = 60; +5+7+8+5+12 = 97; +9+6+6+7+6 =
  **131h**. `plan.md` frontmatter says `effort: 131h`. Match.
- Longest dependency chain (declared): 01→03→04→05→08→11→12→13→15→16→19→20 = 12 phases. After C-3 it is
  01→03→04→05→08→11→12→13→14→15→16→19→20 = 13. No phase is a single-point bottleneck for more than the
  collector fan-in at 08.
- `plan.md` Phases table row count (20) equals the phase-file count (20) equals the highest phase id (20).

## 6.1 Pinned-dependency contradictions — 3 FINDINGS

`plan.md`'s Dependencies section pins three things that no phase uses, and two of them would actively
break a gate the plan itself defines. Found by cross-checking every pinned crate against a
whole-plan grep of the phase files.

**X-1 (HIGH) — `bollard 0.21.1` is pinned, unused, and would fail phase 20's own no-network gate.**
- `plan.md:132` pins it among the runtime crates.
- Zero phase files mention `bollard`. Phase 17 builds the Docker transport by hand and says why:
  it "speaks HTTP/1.1 by hand … this avoids adding an HTTP client and a pipe-transport crate for two
  endpoints". `bollard` is precisely the crate that sentence declines.
- Worse, `bollard`'s transport is built on `hyper`, and phase 20 requires
  `scripts/check-no-network.mjs` to grep `Cargo.lock` for "`reqwest`/`hyper`/`ureq`/`isahc` and
  failing if any appears outside a dev-dependency". So a developer who trusts the pinned list and runs
  `cargo add bollard` gets a **failing release gate** with no explanation of why.
- Proposed: move `bollard` from the pinned list to **Rejected on purpose**, with the reason recorded
  (introduces `hyper`, fails the §no-network gate; P17's two-endpoint hand-rolled pipe transport
  replaces it).

**X-2 (MEDIUM) — `gix 0.87.1` is pinned and unused.**
Phase 17 detects branch changes by reading one file: "read `.git/HEAD` on change and emit
`dev/branch_switched`". No phase mentions `gix`. A full git implementation crate for a single
`read_to_string` is dead weight in a plan that pins dependencies as a contract.
Proposed: drop it from the pinned list, or move it to **Rejected on purpose** with that reason.

**X-3 (MEDIUM) — `tauri-plugin-updater 2.10.1` is pinned, but the plan deliberately ships no updater,
and one phase promises the opposite of another.**
- `plan.md:133` pins `updater 2.10.1`.
- Phase 20 records decision **D-09**: ship no automatic update check at all, because "an app whose
  core promise is 'no network calls, your history stays on this PC' cannot silently contact an update
  server". Its risk table repeats: "Do not add a silent update check under any circumstance."
- But `phase-05…:110-112` writes the capability-allowlist test to reject `updater:` **with an
  exemption note** — "`updater:` (updater is added in P20 with a note)". P20 adds no updater. So the
  one mechanical guard against an update-checking capability is documented as temporary, pointing at a
  phase that will never lift it.
- Proposed: remove `updater 2.10.1` from the pinned crate list, and strike the parenthetical in
  `phase-05` so `updater:` is rejected unconditionally. This *tightens* a security gate and removes a
  dependency; it changes no feature.

None of the three is a scope change — X-1 and X-3 delete contradictions between the plan and itself,
and X-3 makes an existing privacy commitment mechanically enforceable instead of conditionally waived.

## 6.2 The headline feasibility number is wrong — FINDING X-4 (HIGH)

`plan.md:30-33` justifies the entire non-elevated architecture with a count:

> "Research measured 34 of 35 troubleshooting capabilities working under a plain `BUILTIN\Users`
> token, so §36's non-admin privilege model is achievable. The **two** measured gaps — …"

Two defects in one sentence. "34 of 35" implies **one** gap, then two are named — the sentence
contradicts itself. And neither number matches the matrix. Counted row by row from
`research/verified-facts-report.md:98-132`:

| Outcome | Rows | Count |
|---|---|---:|
| PASS | 3–12, 14–18, 20–34 | 30 |
| PASS but useless | 19 (`Win32_QuickFixEngineering`, 4 rows vs WUA's 133) | 1 |
| FAIL — elevation-gated | 1 (`Win32_ProcessStartTrace`, access denied), 13 (`Security` channel, unauthorized) | 2 |
| FAIL — permitted but unreliable | 2 (`__InstanceCreationEvent`: subscribed without error, **0 events** in 3 s) | 1 |
| FAIL — absent, not denied | 35 (Docker pipe: Docker is not installed — environment gap, D3) | 1 |
| | **total** | **35** |

So it is **31 of 35 PASS with 4 FAIL**, not 34 of 35 with 2. The same wrong count is the summary line
in the research report itself (`verified-facts-report.md:136`: "34 of 35 troubleshooting capabilities
work without Administrator"), which is where `plan.md` inherited it. The 35 per-row measurements are
correct and untouched by this finding — only the two summary sentences are wrong.

**Why this one matters more than its size suggests.** It is the number the plan cites to prove its
central architectural bet is viable, and it is the number the red-team slices are told to check every
other privilege claim against. The *conclusion* survives — 31/35, with the four failures designed
around rather than elevated past, still supports §36 — but the plan currently overstates its own
evidence, which is the specific thing the standing project rule forbids.

Proposed replacement for `plan.md:30-33` and the research report's summary line:

> 31 of the 35 probed capabilities passed under a plain `BUILTIN\Users` token, so §36's non-admin
> privilege model is achievable. Four did not, and each is designed around rather than elevated past:
> `Win32_ProcessStartTrace` (access denied) and the `Security` event-log channel (unauthorized) are
> elevation-gated; `__InstanceCreationEvent` on `Win32_Process` subscribes without error but captured
> zero events in a 3 s window, so it is permitted-but-unreliable and is treated as blocker B3's
> short-lived-process blind spot; and the Docker named pipe is absent because Docker is not installed
> (decision D3 — an environment gap, not a privilege gap). One further pass is unusable in practice:
> `Win32_QuickFixEngineering` returns 4 rows against WUA `QueryHistory`'s 133.

This is a correction to a factual claim, not a scope change, and it strengthens rather than weakens the
plan's evidence discipline.

## 6.3 Schema and migration integrity — 3 FINDINGS

Added after the red-team slices exposed one missing table; a systematic sweep of every migration filename
and every table name across the 20 phases found two more problems of the same shape.

**X-5 (HIGH) — migration number `002` is claimed by two phases, in two different directories, and the
collision cascades into phase 20's upgrade proof.**

| Phase | Declares | Directory |
|---|---|---|
| 04 (**Create**) | `001_initial.sql`, `002_search_index.sql` | `src-tauri/migrations/` |
| 19 (**Modify**) | `002_perf_indexes.sql` "only if an index is added" | `src-tauri/src/storage/migrations/` |
| 20 | `003_test_upgrade.sql` (rehearsal, created then reverted) | — |

Three defects in one table. (a) Phase 04 already owns `002`; `rusqlite_migration::Migrations` builds an
ordered list from the `include_str!`-ed files, so a second `002` either shadows the first or breaks the
sequence — and phase 19 is where a *perf* index lands, so the failure surfaces as a migration error during
a performance phase. (b) The two phases name different directories; phase 04 owns the schema and the
`include_str!` mechanism, so `src-tauri/migrations/` is authoritative and phase 19 points at a path that
does not exist. (c) If 19 is corrected to `003`, phase 20's rehearsal migration must become `004` —
otherwise the rehearsal collides with the real migration, and that rehearsal is the plan's only proof that
a user's 10k events survive a version-over-version install.

Fix: phase 19 → `003_perf_indexes.sql` in `src-tauri/migrations/`; phase 20 → `004_test_upgrade.sql`; and
state once in phase 04 that migration numbers are allocated by the phase that adds them, next free number,
in `src-tauri/migrations/` only.

**X-6 (HIGH) — three tables are read and written by name but appear in no migration.**

| Table | Used by | In `001_initial.sql`? |
|---|---|---|
| `collector_state` | `phase-12:78,102` (+ `storage/collector_state.rs` listed under **Modify**) | No — phase 04 creates `collector_status`, a different thing |
| `config_snapshots` | `phase-10:39,94,130,160` (+ `storage/config_snapshots.rs` under **Create**) | No |
| `investigations` | `phase-15:41,150` (+ `storage/investigations.rs` under **Create**) | No — §10 mandates `analysis_sessions` and `analysis_results` |

Phase 04's `001_initial.sql` creates exactly §10's 13 tables and nothing else, and no later phase adds a
migration. So three phases persist to storage that does not exist. `collector_state` is the worst because
phase 12's `collector_state_restart.rs` gate cannot compile (see `red-team-p11-15.md` F-C1/F-C2), but
`config_snapshots` breaks phase 10's config diffing — the highest-value signal in the product — and
`investigations` breaks investigation history.

`investigations` also raises a second question: §10 mandates `analysis_sessions` + `analysis_results`, and
phase 04 creates both. If phase 15 uses `investigations` instead, those two mandated tables ship
created-and-unused. Phase 04 explicitly flags `health_states` as intentionally unused pending §26; it does
not flag these two.

The systemic gap is that nothing in the plan requires a table to have a creating migration. Fix: add to
phase 04's `tests/architecture.rs` (which already greps for stray SQL outside `src/storage/`) an assertion
that every table name appearing in a `FROM`/`INSERT INTO`/`UPDATE` also appears in a migration file — a
grep-level check that would have caught all three at the phase that introduced them. Then decide per
table: create `collector_state`/`collector_baselines` and `config_snapshots` in phase 04's initial
migration, and either rename phase 15's usage to `analysis_sessions`/`analysis_results` or flag those two
as intentionally unused the way `health_states` is.

**X-7 (MEDIUM) — migration `002` has two possible bodies, and with `bundled` rusqlite one of them is
unreachable in the shipped build.**

`phase-04:112-117` — "run `PRAGMA compile_options` at open; if `ENABLE_FTS5` is present, migration `002`
creates an FTS5 external-content table …; otherwise `002` creates `search_terms(...)`… Both paths are
implemented behind `trait SearchIndex`." The trait and the dual implementation are good design. The
conditional migration is not, for three reasons:

1. `rusqlite` is pinned with the `bundled` feature (`plan.md` Dependencies), so FTS5 availability is a
   fixed property of the vendored amalgamation, decided at build time. The runtime `compile_options` check
   answers a question that cannot vary between users — which means **one of the two paths is dead code in
   every shipped build**, and the plan does not say which. §87 forbids shipping unexercised
   implementations presented as real.
2. Phase 04's runner stores "sha256 of the SQL" per `schema_migrations` row. Two bodies at version 2 means
   two valid checksums, so the checksum can no longer be used to detect drift — the reason for storing it.
3. Phase 20's upgrade rehearsal asserts "the event count and **the new schema version**". Version 2 no
   longer identifies a schema, so that assertion can pass against a database whose search table is the
   other shape.

Fix: check the bundled amalgamation's `compile_options` once during phase 04, record the answer in D-04,
and ship the one migration that build supports. Keep `trait SearchIndex` and keep the second
implementation only if it is genuinely exercised by a test that constructs the other schema explicitly —
otherwise name it as a documented future path rather than an implemented one.

## 7. What this gate does not cover

This is a mechanical audit only: structure, frontmatter, graph shape, arithmetic, placeholders. It makes
no judgement about whether a phase is technically correct, whether an API exists, whether a success
criterion is falsifiable, or whether the 91 numbered requirements are actually covered. Those are the
four parallel red-team slices (`red-team-p01-05`, `-p06-10`, `-p11-15`, `-p16-20`) and the coverage
auditor (`red-team-coverage`), reported separately.

## 8. Environment blockers carried into this gate

- **Bash is non-functional in this session** — every command, including `echo`, returns empty output.
  Consequences: `ak plan reindex` has not been run, so `plans.db` does not yet reflect the 20 phase
  files; `git` operations are unavailable; and `phase-01-start.md` (the superseded CLI stub) cannot be
  deleted. The stub carries its own tombstone frontmatter (`status: completed`) and the two commands
  needed to clean it up. Files-first means the plan is complete and usable regardless — the index is
  rebuildable, the files are the plan.
  **Order matters:** delete the stub *before* reindexing. Its frontmatter is `phase: 0` /
  `status: completed`, so a reindex run while it is still on disk registers a phantom completed phase 0
  and the plan reports 21 phases with one already done.
- **Rust is not installed on this machine** (blocker B1, from the evidence packet). Phase 01 owns
  installing it. Nothing in this gate depends on it.

## 9. Proposed amendments

Applied only together with the red-team findings, under one amendment log in `plan.md`, so the diff from
the materialized winner stays auditable.

| ID | Sev | File | Change |
|---|---|---|---|
| C-0 | LOW | `phase-08`, `-09` | `phase: 8` → `08`, `phase: 9` → `09` |
| C-0 | LOW | `phase-08`, `-09`, `-10`, `-11`, `-12`, `-17` | zero-pad single-digit dependency ids |
| C-1 | HIGH | `phase-15` | `dependencies: [13, 14]` → `[06, 07, 10, 13, 14]` |
| C-2 | MED | `phase-17` | `dependencies: [08]` → `[08, 10]` |
| C-3 | MED | `phase-14` | `dependencies: [11]` → `[13]` |
| X-1 | HIGH | `plan.md` | move `bollard 0.21.1` from pinned crates → "Rejected on purpose" + reason |
| X-2 | MED | `plan.md` | drop `gix 0.87.1` from pinned crates (P17 reads `.git/HEAD` directly) |
| X-3 | MED | `plan.md` | drop `updater 2.10.1` from pinned crates (D-09 ships no updater) |
| X-3 | MED | `phase-05` | strike "(updater is added in P20 with a note)" → reject `updater:` unconditionally |
| X-4 | HIGH | `plan.md`, `research/verified-facts-report.md` | replace "34 of 35 … the two measured gaps" with the counted 31/35 and all four failures (§6.2) |
| X-5 | HIGH | `phase-19`, `phase-20`, `phase-04` | renumber to `003_perf_indexes.sql` / `004_test_upgrade.sql`; one migration directory (`src-tauri/migrations/`); state the allocation rule |
| X-6 | HIGH | `phase-04`, `phase-10`, `phase-12`, `phase-15` | create the three missing tables (or rename the usages); add a table-has-a-migration assertion to `tests/architecture.rs` |
| X-7 | MED | `phase-04` | resolve FTS5 at build time, ship one body for migration `002`, keep the second path only if a test exercises it |

No content, requirement, success criterion, or scope changes. C-0 through C-3 are frontmatter lines;
X-1 through X-3 delete or relocate dependency entries and one parenthetical; X-4 corrects a miscount in two
summary sentences; X-5 through X-7 fix schema/migration integrity without changing any feature.

## Verdict

**PASS on structure, FAIL on schema integrity — 11 findings, 3 of them blocking implementation.** The
materialized plan is structurally complete: 20 phases, 200 required headings, no leftover placeholders, a
valid acyclic dependency graph, and effort arithmetic that matches the index. What it does not have is a
schema that supports the phases built on top of it.

The findings that must be resolved before phase 04 is implemented, because they change the initial
migration:

- **X-6** — `collector_state`, `config_snapshots` and `investigations` are persisted to by phases 12, 10
  and 15 and created by no migration. Phase 10's is the config diffing that carries the product's
  highest-value signal; phase 12's makes its own verification gate uncompilable.
- **X-5** — two phases both claim migration `002`, in two different directories, and correcting it pushes
  phase 20's rehearsal migration to `004`. Left alone it fails inside a performance phase and then inside
  the only test that proves user data survives an upgrade.
- **X-4** — the plan's headline feasibility claim ("34 of 35 capabilities work non-elevated") is a miscount
  of its own evidence and the sentence contradicts itself by then naming two gaps. Real figure: 31 of 35,
  four failures. The architectural conclusion survives; the overstatement sits in the Overview where a
  reader forms their trust in everything after it.

Also notable, though not blocking: **C-1** (phase 15's `Confirmed` class depends on a P10 artifact it never
declares a dependency on; phases 06, 07 and 10 are unreachable from the terminal phase) and **X-1** (a
pinned crate that no phase uses and that would fail the plan's own release gate by pulling in `hyper`).

Status: DONE_WITH_CONCERNS
Concerns: `ak plan reindex` and the deletion of `phase-01-start.md` are pending a working shell.


