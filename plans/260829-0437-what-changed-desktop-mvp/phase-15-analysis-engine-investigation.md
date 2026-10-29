---
phase: 15
title: "Deterministic analysis engine: correlation, causal ranking, investigation flow"
status: completed
priority: P1
effort: 12h
dependencies: [06, 07, 10, 13, 14]
---

## Overview

The product thesis lands here. Everything before this phase records; this phase *answers*. The engine
takes a user problem statement plus a time window and returns a ranked, explained list of suspect
changes (§27–§32, §45, §47, §59, §74). Its default provider is fully deterministic — a scoring function
over recorded events and relationships, no ML, no network, no LLM — so the same history always produces
the same ranking, and the ranking is testable with fixtures. A narrow `ExplanationProvider` seam exists
for a future opt-in provider without granting it raw event data or network access. Confidence language
is capped by evidence class so the app never claims causation it cannot support (§32).

## Requirements

**Functional**

*Investigation entry (§45)*
- `start_investigation(problem: String, window: InvestigationWindow)` where
  `InvestigationWindow = LastHour | Today | SinceYesterday | SinceLastRestart | SinceCheckpoint(id) | Custom{from,to}`.
  Default window resolution when the user gives none: if the problem text contains a time hint
  ("this morning", "yesterday", "after the update") map it; else `Today`.
- Problem-text parsing is **keyword extraction, not NLU**: lowercase, strip stopwords, and match
  against (a) known entity names from the `entities` table (fuzzy: case-insensitive substring +
  normalized display name), (b) a small domain lexicon mapping symptom words to categories
  (`microphone|audio|sound|mic → [driver:Audio, application, service]`,
  `internet|network|wifi|dns|vpn|proxy|offline → [network, driver:Net, service]`,
  `port|localhost|connect|refused|3000|8080 → [port, process, config]`,
  `slow|freeze|lag → [driver, update, startup, service]`,
  `crash|closes|quit|error → [application, update, driver]`,
  `build|compile|test → [config, application, update]`,
  `docker|container → [docker, service, application]`,
  `login|sign in|password → [config, network, application]`). The lexicon is a data table with a unit
  test per row; unmatched text simply yields no category boost (never an error, never a wrong guess).
- Result: `Investigation { id, problem, window, resolved_window, suspects: Vec<Suspect>, total_events_in_window, created_at }`
  persisted to the `investigations` table so it appears in history and can be reopened.

*Correlation graph (§28, §29)*
- `analysis/graph.rs` builds an in-memory graph from the window's events (≤ 5000 events; older windows
  are downsampled by dropping `informational` events first, and the drop is disclosed in the UI as
  "Analysed the 5,000 most important changes in this window").
- Edge sources, in decreasing evidence strength:
  1. `relationships` rows recorded by collectors (P09 `listens_on`/`opened_by`, P11 `provides`,
     P10 `config_of`) — **structural**, the only edge class that can reach `Confirmed`.
  2. Same-entity edges: two events on the same `entity_id` — **structural**.
  3. Temporal-proximity edges: events within 5 min, weighted by closeness — **circumstantial**.
  4. Category-affinity edges from a fixed table (`config → service`, `update → driver`,
     `driver → device`, `service → port`, `startup → process`) — **circumstantial**.
- Chain detection: depth-limited DFS (max depth 4, max 200 paths) over structural edges ordered by
  time, producing `Chain { events: Vec<EventId>, span_ms, edge_kinds }`. A chain of ≥ 2 structural
  edges is the §30 "cascade" that the UI renders as a sequence.

*Scoring (§31)*
- `score = w_t·temporal + w_r·relationship + w_c·category + w_s·severity + w_x·rarity + w_p·failure_proximity`,
  computed in `analysis/score.rs` with named constants in one `const` block:
  `W_TEMPORAL = 1.0`, `W_RELATIONSHIP = 3.0`, `W_CATEGORY = 1.5`, `W_SEVERITY = 1.0`,
  `W_RARITY = 1.25`, `W_FAILURE_PROXIMITY = 2.0`.
  - `temporal` = `exp(-minutes_before_symptom / 90.0)` clamped to `[0,1]`, and `0.0` for anything
    *after* the symptom time (a change after the failure cannot have caused it — hard rule, tested).
    An event with `occurred_at_is_uncertain = 1` (P04 — discovered by baseline diff after the app was
    closed, so only its *detection* time is known) scores `temporal = 0.0` and is **excluded from the
    "after the symptom" exclusion too**: its stored timestamp is a detection time, so it can neither
    earn recency credit nor be ruled out by ordering. Such an event can still rank on relationship,
    category, severity and rarity — which is the point, because a change made during downtime is
    exactly the kind a user is asking about — and its explanation says "detected when What Changed
    started" instead of a fabricated relative time. Two fixtures cover it: one where the uncertain
    event is the planted cause and must still reach `Likely` on structural evidence, and one where an
    uncertain event with no structural link must not out-rank a certain one that has it.
  - `relationship` = `1.0` if the event is on an entity structurally linked to a symptom-matched
    entity, `0.6` if two structural hops away, `0.0` otherwise.
  - `category` = `1.0` if the event's category is in the lexicon-derived category set, `0.35` if it is
    in a category-affinity neighbour of one, else `0.0`.
  - `severity` = `{critical: 1.0, important: 0.6, informational: 0.15}` from the P06 classifier.
  - `rarity` = `1.0 - min(1.0, prior_occurrences_90d / 20.0)` (a first-ever driver update scores 1.0;
    a Chrome update that happened 18 times scores 0.1). Uses the P06 `prior_occurrences` query.
  - `failure_proximity` = `1.0` if a `system/error_logged` / `service_failed` / `process/stopped`
    (`exit_kind: Unexpected`) event exists within 10 min *after* this event **and** shares an entity or
    a structural edge with it; `0.5` for the same within 60 min; else `0.0`.
- Chain bonus: an event that is the *head* of a detected structural chain gets
  `score += 0.75 · (chain_len - 1)`, capped at `+2.25`, and the chain's members are attached to the
  suspect instead of listed separately — so the timeline of a cascade is presented as one suspect with
  its steps (§30), not three competing suspects.
- Ranking is total-ordered and stable: sort by `score` desc, then `severity` desc, then `timestamp`
  desc, then `event_id` asc. Two runs over the same data are byte-identical (asserted).

*Confidence classes (§32)*
- `Confirmed` — requires a structural chain reaching a symptom-matched entity **and** a corroborating
  failure signal (`failure_proximity == 1.0`). Copy: "This change directly affected <entity>."
  An event whose occurrence time is uncertain can never be `Confirmed`, because `failure_proximity`
  is a time-window test and there is no trustworthy time to window against — it caps at `Likely`.
- `Likely` — structural edge without a corroborating failure signal, or a chain whose corroboration is
  only within 60 min. Copy: "This change likely affected <entity>."
- `Correlated` — temporal/category evidence only. Copy: "This changed around the same time."
- The class is computed from evidence, not from the score, and the copy template is selected by class in
  `analysis/explain.rs`. A `Correlated` suspect can never render `Confirmed` copy — enforced by the type
  (`Explanation::for_class(class, ...)` is the only constructor) and by the forbidden-phrase test.
- Forbidden phrases anywhere in generated explanation strings for non-`Confirmed` classes: "caused",
  "because of", "the problem is", "this broke", "definitely", "root cause". `tests/no_causal_overclaim.rs`
  generates explanations for 200 fixture suspects across all three classes and greps the output.

*Explanations (§31, §74)*
- Each suspect renders: title (the event's `user_friendly_summary`), timestamp + relative phrasing
  ("34 minutes before you noticed the problem"), the confidence sentence, a `why_ranked` bullet list
  built from the contributing score terms in plain language ("Happened 34 minutes before the problem",
  "Changed a file that this service reads", "First time this driver changed in 90 days"), the chain
  steps when present, and a `What to try` line drawn from a fixed remediation table keyed by event type
  (e.g. `network/proxy_changed` → "Check your proxy setting under Windows Settings → Network → Proxy").
  The remediation table contains no commands the app runs — it is advice text only.
- §74 worked example is a mandatory fixture: a `.env` `PORT 3000 → 4000` change, then a
  `service/stopped`+`service/started` for the API service, then `port/closed 3000` + `port/opened 4000`.
  With the problem "my app can't connect to the api", the top suspect must be the `.env` change,
  `Confirmed`, with a 3-step chain, and the explanation must contain "Port 4000" and the two other
  steps — asserted string-by-string. A linked process/port evidence pair whose recorded timestamps are
  exactly 300 ms apart must render the exact sentence "These events occurred 300ms apart"; this copy
  is asserted here because Phase 09 records the timestamps and relationship but Phase 15 owns the
  explanation renderer.

*Ranking fixture (§59)*
- `tests/fixtures/analysis/scenarios/*.json` — 8 hand-built scenarios with a declared expected top-3.
  The mandated one: the config→restart→port chain must outrank a Chrome update **even when the Chrome
  update is moved to 2 minutes before the symptom and the chain is 40 minutes before**. This is the
  test that proves relationship + rarity dominate raw recency, and it fails loudly if anyone rebalances
  the weights carelessly. Other scenarios: driver update vs unrelated app install; proxy change vs
  service restart; two competing drivers where only one is on the symptom's device; a window with
  nothing but `informational` events (expect "No obvious cause" — see below); a 5000+ event window
  (downsampling still returns the planted cause); a cascade whose head is outside the window (expect
  `Likely`, never `Confirmed`); an event *after* the symptom (must never rank).
- No-answer path (§47): if the top score is below `MIN_REPORTABLE_SCORE = 1.2`, the result is
  `suspects: []` plus `no_cause_reason: NothingChanged | OnlyRoutineChanges | WindowTooNarrow |
  NotWatchingYet`, each with its own copy ("Nothing meaningful changed in this window." /
  "Only routine changes here — updates and background tasks." / "Try widening the time range." /
  "We started watching after this window. Try a more recent range."). Guessing is never preferred over
  saying nothing.

*Commands (already allowlisted in P05)*
- `start_investigation`, `get_investigation`, `list_investigations`, `delete_investigation`,
  `analyze_window(window)` (dashboard "what changed recently" without a problem statement — same engine,
  empty symptom set, category boost disabled), `get_related_events(event_id)` (single-event graph
  neighbourhood for the P07 drawer's "Related events" section, retro-filled here).

**Non-functional**
- `start_investigation` p95 ≤ 400 ms for a 5000-event window on this machine; ≤ 1.5 s at the 100k-event
  DB size (window queries are index-bound, the graph is capped).
- Engine is pure and OS-free: `analysis/` imports nothing from `platform/` or `collectors/`
  (`tests/architecture.rs` extended with this rule). It takes `Vec<SystemEvent>` +
  `Vec<Relationship>` and returns `Investigation` — that is why it is fixture-testable.
- Zero allocation of the whole event table: windows are fetched by the P04 keyset query with a cap.

## Architecture

`analysis/{mod.rs,window.rs,symptom.rs,graph.rs,chain.rs,score.rs,confidence.rs,explain.rs,
remediation.rs,downsample.rs}` plus the existing `analysis/evidence.rs` from P13.
`symptom.rs` owns the lexicon + entity matching and returns `SymptomModel { entities: Vec<EntityId>,
categories: Vec<Category>, symptom_time: DateTime }`. `graph.rs` builds `Graph { nodes: Vec<EventId>,
edges: Vec<Edge { from, to, kind: Structural|Temporal|CategoryAffinity, weight }>`. `score.rs` is a pure
function `fn score(event, ctx: &ScoreContext) -> ScoreBreakdown` where `ScoreBreakdown` keeps every term
separately — that breakdown is what `explain.rs` turns into `why_ranked` bullets, so the explanation can
never disagree with the score (one source of truth, DRY).
`storage/investigations.rs` persists normalized audit rows plus a sanitized immutable presentation
snapshot. Reopened investigations therefore remain explainable after ordinary event retention removes
their source rows; copy changes apply to new analyses rather than rewriting historical evidence.
Frontend: `features/investigation/{InvestigationsScreen.svelte,ProblemForm.svelte,WindowPicker.svelte,
SuspectList.svelte,SuspectCard.svelte,ChainSteps.svelte,NoCauseState.svelte,InvestigationHistory.svelte}`,
store `lib/stores/investigation.svelte.ts`.

## Related Code Files

**Create** — `src-tauri/src/analysis/{window.rs,symptom.rs,graph.rs,chain.rs,score.rs,confidence.rs,
explain.rs,remediation.rs,downsample.rs}`; `src-tauri/src/storage/investigations.rs`;
`src-tauri/src/commands/investigate.rs`; `src-tauri/src/domain/investigation.rs`;
`src-tauri/tests/analysis_scoring.rs`, `src-tauri/tests/analysis_ranking_fixtures.rs`,
`src-tauri/tests/analysis_chains.rs`, `src-tauri/tests/no_causal_overclaim.rs`,
`src-tauri/tests/analysis_determinism.rs`, `src-tauri/tests/analysis_perf.rs`,
`src-tauri/tests/symptom_lexicon.rs`; `src-tauri/tests/fixtures/analysis/scenarios/{port-config-chain,
driver-vs-install,proxy-vs-restart,two-drivers,routine-only,large-window,head-outside-window,
after-symptom}.json`; the frontend files listed above.
**Modify** — `src-tauri/src/analysis/mod.rs`, `src-tauri/src/commands/mod.rs` + `dto/bindings.rs`
(6 commands), `src-tauri/src/domain/event.rs` (`ScoreBreakdown`, `ConfidenceClass` exports),
`src-tauri/tests/architecture.rs` (new purity rule), `src/features/home/ProblemInput.svelte`
(remove the P07 stub, route to Investigate), `src/components/event/RelatedEvents.svelte`
(wire `get_related_events`), `ROADMAP.md` (clear the P07 stub entry), `docs/system-architecture.md`,
`PRODUCT.md`.
**Delete** — none.

## Implementation Steps

1. Write the 8 scenario fixtures **first**, with their expected top-3 declared. They are the spec; the
   engine is written to satisfy them. Each fixture is a JSON list of events + relationships + a problem
   string + a symptom time.
2. `window.rs`: window resolution (incl. `SinceLastRestart` via P13 boot events and `SinceCheckpoint`
   via P14 timestamps), event fetch with the 5000 cap, `downsample.rs` dropping `informational` first
   and reporting the drop count.
3. `symptom.rs`: lexicon table + entity fuzzy match + symptom-time inference; `symptom_lexicon.rs` test
   asserts one row per mapping and that an unmatched problem yields an empty category set.
4. `graph.rs` + `chain.rs`: edge construction from the four sources with `kind` recorded; DFS with the
   depth/path caps; chain span computation.
5. `score.rs`: the const block and the six terms, each with its own unit test at boundary values
   (0 minutes, 90 minutes, after-symptom, 0 vs 20 prior occurrences, all three severities).
6. `confidence.rs` + `explain.rs` + `remediation.rs`: class derivation from evidence only,
   `Explanation::for_class` as the single constructor, bullets generated from `ScoreBreakdown`,
   remediation table keyed by event type with a default of "Compare against a checkpoint from when this
   worked."
7. Wire `start_investigation` end-to-end with persistence, then `analyze_window` and
   `get_related_events` reusing the same graph builder.
8. Frontend: problem form (§45 copy, the P07 placeholder text preserved), window picker, ranked suspect
   cards with confidence badge + `why_ranked` + chain steps + `What to try`, the four no-cause states,
   and investigation history. Six states in the dev harness, including the 900×600 frame.
9. Run the fixture suite; tune only by adjusting a *term definition* with a justification comment, never
   by special-casing a fixture. If a fixture cannot be satisfied without special-casing, stop and record
   it in the phase file as an open design question rather than fudging the test.
10. `analysis_perf.rs`: 5000-event window ≤ 400 ms p95; then seed 100k events and assert ≤ 1.5 s.
11. Commits: (a) `test: analysis ranking fixtures`, (b) `feat: window resolution and downsampling`,
    (c) `feat: symptom model and lexicon`, (d) `feat: correlation graph and chain detection`,
    (e) `feat: suspect scoring`, (f) `feat: confidence classes and explanations`,
    (g) `feat: investigation commands and persistence`, (h) `feat: investigate ui`,
    (i) `test: determinism, overclaim and performance suites`.

## Todo (commit boundaries)

- [x] 8 scenario fixtures with declared expectations
- [x] Window resolution + downsampling with disclosure
- [x] Symptom model (lexicon + entity match)
- [x] Graph + chain detection with caps
- [x] Six-term scorer with named constants
- [x] Confidence classes + explanation generation + remediation table
- [x] Commands, persistence, related-events retro-fill
- [x] Investigate UI with four no-cause states
- [x] Determinism, overclaim, performance suites

## Success Criteria

- [x] §59 fixture passes: with the Chrome update 2 min before the symptom and the config→restart→port
      chain 40 min before, the chain's head is rank 1 and the Chrome update is not in the top 3
- [x] §74 fixture passes: top suspect is the `.env` `PORT` change, class `Confirmed`, 3 chain steps, and
      the explanation contains "Port 4000"
- [x] An event timestamped after the symptom never appears in `suspects` (any fixture, any window)
- [x] `no_causal_overclaim.rs`: zero forbidden phrases across 200 generated `Likely`/`Correlated`
      explanations
- [x] A chain whose head is outside the window yields at most `Likely`, never `Confirmed`
- [x] Routine-only window yields `suspects: []` with `no_cause_reason: OnlyRoutineChanges` and the exact
      §47 copy
- [x] `analysis_determinism.rs`: two runs over the same fixture produce identical suspect ids, scores,
      classes, and explanation strings
- [x] `analysis_perf.rs`: 5000-event window ≤ 400 ms p95; 100k-event DB ≤ 1.5 s
- [x] `tests/architecture.rs`: no `platform/` or `collectors/` import inside `analysis/`
- [x] Every score term has a boundary unit test; `rg -n "if event.id ==" src-tauri/src/analysis` is empty
      (no fixture special-casing)

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test analysis_ranking_fixtures --test analysis_scoring --test analysis_chains \
           --test no_causal_overclaim --test analysis_determinism --test symptom_lexicon \
           --test architecture
cargo test --test analysis_perf -- --include-ignored
cargo clippy --all-targets -- -D warnings
cd /d/Project/.WhatChage && bun run check && bun run check:bindings && bun run test
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Weights tuned until fixtures pass, producing a ranker that only works on fixtures | Fixtures pass but a manual real-machine investigation returns nonsense | Weights may only change with a written justification in `score.rs` and all 8 fixtures re-run; additionally a manual acceptance run on this machine's real 7-day history is part of the phase's Done, and its top-5 must be defensible by inspection |
| Over-claiming causation damages trust (§32) | A `Correlated` suspect reads like a diagnosis | Class-gated constructor + forbidden-phrase test + the `What to try` line is advice, never an assertion. If a reviewer finds one over-claiming string, the phrase joins the forbidden list as a regression test |
| Symptom keyword matching produces a wrong entity match and skews everything | Suspect list is dominated by an unrelated app | Entity matches require a normalized token match of length ≥ 4 (no 2-letter substring hits); unmatched → category-only scoring; the "two-drivers" fixture guards the entity-specificity path |
| Graph blowup on a busy window | Investigation takes seconds, CPU spike | Hard caps: 5000 events, depth 4, 200 paths, and temporal edges only within 5 min. `analysis_perf.rs` fails the build if exceeded |
| Downsampling drops the actual cause | Planted cause missing from a large-window fixture | Downsampling drops `informational` first and *never* drops an event with a structural edge or a `failure_proximity` contribution; the `large-window` fixture asserts the planted cause survives |
| Rarity term punishes legitimately frequent causes (e.g. a nightly job that really does break things) | A recurring true cause ranks low | Rarity is weighted 1.25, not dominant, and `failure_proximity` (2.0) can still lift a frequent event; the `proxy-vs-restart` fixture covers a recurring-but-causal case |
| Persisted presentation copy changes over time | Reopened investigations differ from what the user originally reviewed | Persist a sanitized immutable presentation snapshot beside normalized audit rows; new copy applies to new analyses while old investigations survive source-event retention |

## Security Considerations

- The engine reads only already-redacted events, so explanations cannot leak a secret value; the
  `.env` fixture's explanation is asserted to contain the *key name* and `[REDACTED]`, never a value.
- `remediation.rs` returns advice text only — no command execution, no `shell:open`, no auto-fix. The
  app never modifies the system it observes (§4), and there is no IPC command that could.
- Problem statements are user free text: stored via the P03 sanitizer (a user may paste a connection
  string into "what stopped working"), rendered as a text node, never interpolated into SQL (the
  keyword matcher builds parameterized `LIKE` clauses with escaped `%`/`_`).
- Investigation persistence is subject to the §33/§49 delete-all and the 90-day retention class.
