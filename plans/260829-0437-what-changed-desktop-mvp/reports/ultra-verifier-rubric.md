# Ultra Verifier Rubric — `ak:plan --ultra`, "What Changed"

Controller-authored, fixed before any candidate was judged. The verifier scores each
of the five anonymized candidates on **every criterion, 1–20**, then ranks them and
selects **one winner** — or **rejects all**.

## Hard constraints (checked first, before scoring)

A candidate violating any of these cannot win. Record the violation explicitly.

| # | Constraint |
|---|---|
| H1 | Does not propose Electron, in any form, at any point. |
| H2 | Does not introduce Go or a second binary/sidecar process in the MVP. |
| H3 | Does not swap any mandated stack component (Tauri 2, Rust, Svelte 5, TypeScript, Vite, Bun, Tailwind 4, Three.js, SQLite, serde, Tokio, tracing). |
| H4 | Does not assume Rust is already installed; the toolchain install is an explicit early step. |
| H5 | No invented API, crate, or version. Versions match `research/verified-facts-report.md` §1. |
| H6 | Privilege claims match the measured 35-row matrix in research §2. No collector is claimed to work non-admin when the probe says denied, and none is claimed to need admin when the probe says it passed. |
| H7 | No generic frontend surface: no raw SQL, no arbitrary file read, no shell execution exposed over IPC. |
| H8 | The app is not designed to run permanently elevated. |
| H9 | No requested feature silently dropped or deferred. Only §26 health checks and §75–§78 futures may be parked, because the user parked them. |
| H10 | No unverified research §5 assumption restated as established fact. |

## Scoring criteria (1–20 each)

| # | Criterion | What a 20 looks like | What a 5 looks like |
|---|---|---|---|
| C1 | **Requirement fidelity** | Every one of §1–§91 lands in a named phase. §69's Phase 0–8 dependency order preserved. Traceability is explicit enough to audit. | Whole sections unaddressed; §69 ordering ignored; scope quietly narrowed. |
| C2 | **Technical correctness** | Real APIs and crates at the pinned versions; Windows mechanisms match the probe evidence; the `Win32_QuickFixEngineering` trap and the ~2.1 s driver-sweep cost are both handled. | Plausible-sounding but wrong or unverifiable API/version choices; polls the driver inventory on a short timer. |
| C3 | **Executability** | Exact repo-relative paths, exact commands, per-phase file inventory; every gate runs here (Git Bash, Bun, MSVC, non-elevated, no CI). An agent could start phase 1 with no further questions. | "Implement the collector" with no files, no commands, no ordering. |
| C4 | **Verification rigor** | Objectively checkable per-phase criteria plus all four mandated suites: §60 redaction leak, §59 ranking fixture (port/config chain outranks the Chrome update), §61 10k/100k/1M perf, migration tests. Thresholds are numeric. | Criteria are aspirations ("works well"); mandated suites missing or hand-waved. |
| C5 | **Architecture quality** | `Collector` trait, normalizer, repository, analysis engine cleanly separated; `core/` vs `platform/windows/` enforced; narrow typed IPC; Rust types as source of truth with drift detection; modular files. | Layers blurred; Win32 leaking into core; type duplication between Rust and TS with no guard. |
| C6 | **Security & privacy engineering** | Redaction is a single structurally-enforced chokepoint before persistence *and* logging — not a convention. §82 threats each mitigated. Minimal capabilities, DB-corruption recovery, redacted diagnostics. | Redaction is "remember to call the redactor"; threat model is a list with no mitigations. |
| C7 | **Risk realism** | B1/B2/B3 each handled with an observable signal and a pre-decided response. B2 in particular measures and reports honestly instead of redefining the metric. Unverified assumptions carried as risks with settle-it actions. | Blockers restated as tasks; "<80 MB" asserted as an acceptance criterion with no measurement or fallback. |
| C8 | **Product/UX substance** | Six states on every screen; §72–§74 microcopy voice preserved; progressive disclosure; calm importance model; conservative notifications; a11y; 900×600; §41 tokens enumerated; Three.js intentional per §39. | UI treated as an afterthought; Three.js either absent or turned into the whole interface. |
| C9 | **Sequencing & reviewability** | Commit-sized phases, sane dependency chain, honest effort, no phase that is secretly a month of work. | 4 mega-phases, or 40 phases of trivia; effort numbers with no basis. |
| C10 | **Plan-format compliance** | The required report structure followed exactly; plan.md body under ~120 lines with all mandated sections; phase files carry every required section; phase links use human-readable text. | Structure improvised; sections missing; links are bare filenames. |

## Weighting guidance

No arithmetic weights. When candidates are close, break ties in this order:

1. **C2 + C6** — a plan that is technically wrong or leaks secrets is worse than one that is merely less elegant. These are the two places where being wrong is expensive and hard to discover late.
2. **C7** — honesty about B2 (memory) and B3 (process blind spot) predicts whether the executing agent will report reality or quietly drift.
3. **C3 + C4** — the plan's whole purpose is to be executed and checked.
4. **C1** — completeness.
5. Everything else.

## Reject-all condition

Reject all five if **every** candidate violates a hard constraint, or if none is
executable enough that a coding agent could begin phase 1 without inventing
requirements. On reject-all the controller hard-stops and reports the ranking; it does
not fall back to a default candidate.

## Required verifier output

1. A hard-constraint pass/fail line per candidate (H1–H10), with the specific violation quoted if any.
2. A score table: candidates × C1–C10, plus a total.
3. A ranking, best first, each with 2–4 sentences of **evidence-backed** rationale — quote or cite the candidate's own text, do not characterize it abstractly.
4. **One winner**, named, or an explicit reject-all.
5. The single strongest thing the winner should absorb from a loser, if any, stated as a note for the controller's red-team step (the winner is materialized unchanged; this is only a hint for later review).
6. A confidence note: what the verifier could not check, and how much of each candidate it actually read.
