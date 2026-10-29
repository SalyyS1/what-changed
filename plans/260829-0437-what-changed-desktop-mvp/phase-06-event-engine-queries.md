---
phase: 06
title: "Event Engine & Queries"
status: completed
priority: P1
effort: 6h
dependencies: [05]
---

## Overview

§69 Phase 1 backend half: the query service (ranges, filters, clustering, counts), search, the
synthetic event generator that unblocks all UI work, the §85 developer simulation panel backend, and
the 10k-scale responsiveness proof. No OS APIs yet — deliberately.

## Requirements

**Functional**
- `EventQuery { range: TimeRange, categories: Vec<Category>, importance_min: Option<Importance>,
  entity_id: Option<String>, cursor: Option<Cursor>, limit: u16 (≤200) }`;
  `TimeRange = Today | Yesterday | Last7Days | SinceLastRestart | Custom{from,to}` — `SinceLastRestart`
  answers a §1 question and resolves from the recorded boot event (falls back to
  `GetTickCount64`-derived boot time once P13 lands; until then, midnight).
- `get_dashboard_summary` → `{ today: {total, notable, important, critical}, status_line, top_important:
  Vec<EventSummary>, monitoring: MonitoringState, last_checkpoint: Option<CheckpointRef> }`;
  `status_line` is computed, not hardcoded: "Everything looks normal." when
  `important + critical == 0`, else "N recent changes may need attention." (§72 voice, exact strings).
- Clustering (§44): after fetching a keyset page of ≤200 rows, group runs sharing
  `(category, coarse entity key, 60 s bucket)` with count > 4 into
  `TimelineItem::Cluster { key, label, count, first_ts, last_ts, highest_importance }`; label from a
  template table ("12 Docker-related changes", "8 file changes in `api/`"). Expanding a cluster
  issues a scoped `get_events` with the cluster key + window.
- `search_events` over the P04 index: application names, event descriptions, file names, ports,
  services, drivers, container names, checkpoint labels (§52). Numeric queries (`25565`) match port
  entities and metadata port fields.
- `get_event_details` → human explanation, timestamp, importance, confidence, before/after, related
  events (by `related_entity_ids` + ±5 min same-entity), affected entities, source collector,
  technical metadata, raw identifiers, and `prior_occurrences` ("This happened 2 times before" — §1).
- Persistence hardening found by the 2026-09-10 production-readiness audit lands before the query layer
  is exposed: `settings_repository::update` applies a multi-field `SettingsUpdate` inside one transaction,
  so a valid early field plus an invalid later field changes **nothing**; migration-ledger verification
  requires exactly one checksum row for every applied version `1..=user_version`, with no gaps or extras.
- Synthetic generator (`src-tauri/src/testing/synthetic.rs`, compiled in release only behind the
  `devtools` cargo feature): deterministic seeded streams producing realistic events across all 15
  categories, with a `--profile {day, week, burst, 10k, 100k, 1m}` selector, used by both the dev
  panel and the perf tests. Clearly labelled `source_collector = "synthetic"` so no synthetic row can
  ever be mistaken for a real one (§87).
- `simulate_event(kind)` (§85) supports: driver update, port change, process crash, DNS change,
  Docker restart, config diff — each producing a correctly shaped normalized event through the real
  write path.
**Non-functional**
- No query may return unbounded rows; `limit` is clamped in Rust regardless of the request.
- p95 budgets recorded at 10k: page ≤ 10 ms, summary ≤ 15 ms, search ≤ 40 ms (measured, written to
  `docs/PERFORMANCE.md`; 100k/1M gates are P20's job).
- The generator writes through the real `Sanitized` path — no test-only shortcut into SQLite.

## Architecture

`services/` is introduced here: `query_service.rs` (ranges, filters, clustering, details),
`search_service.rs`, `summary_service.rs`, plus `importance.rs` in `domain/` (the §54 classifier used
by the generator now and by collectors later — one table, one behaviour). Clustering lives in Rust
over a fetched page (deterministic page size, variable item count) rather than in SQL: simpler,
index-friendly, and it keeps §44's "never render 500 tiny events" promise without a second query
planner.

## Related Code Files

**Create** — `src-tauri/src/services/{mod.rs,query_service.rs,search_service.rs,summary_service.rs,
cluster.rs}`; `src-tauri/src/domain/importance.rs`; `src-tauri/src/testing/synthetic.rs`;
`src-tauri/src/dto/{query_dto.rs,summary_dto.rs}`; `src-tauri/tests/query_service.rs`;
`src-tauri/tests/cluster.rs`; `src-tauri/tests/importance_calm.rs`;
`src-tauri/tests/perf_scale.rs` (10k active now, 100k/1M `#[ignore]`d until P20).
**Modify** — `src-tauri/src/commands/{events.rs,dev.rs}` (real bodies), `src-tauri/src/domain/mod.rs`,
`src-tauri/src/storage/event_repository.rs` (cluster-scoped and search queries),
`src-tauri/Cargo.toml` (`[features] devtools = []`), `src/lib/tauri/bindings.ts` (regenerate),
`ROADMAP.md` (remove the stubs this phase implements), `ANALYSIS.md` (importance table).
**Delete** — none.

## Implementation Steps

1. `domain/importance.rs`: `classify(&NormalizedEvent, &ClassifierCtx) -> Importance` from an explicit
   `(category, event_type) → base` table plus escalations: automatic-start service stopped →
   `important`; GPU/audio/network/storage/input driver `updated` → `important`; proxy or DNS change →
   `important`; a port that a checkpoint recorded as open closing → `important`; Windows update
   installed → `notable`, update **failed** → `important`; unexpected shutdown → `critical`;
   application `updated` (Chrome, Spotify, Discord …) → `informational` **always** (§53/§54).
2. `tests/importance_calm.rs`: over a 2,000-event realistic synthetic day, assert ≥ 80% are
   `informational`, ≤ 2% `critical`, and that no `application/updated` row is above `informational`.
3. `query_service.rs`: range resolution (local-day boundaries via the OS timezone, stored ms are UTC),
   filter mapping to the §10 indexes, keyset cursor encode/decode (opaque base64 of `(ts,id)`),
   `EXPLAIN QUERY PLAN` assertions in tests that each filter path uses an index (no `SCAN events`).
4. `cluster.rs` + tests: cluster/no-cluster boundaries, label templates, stable ordering, and
   expansion returning exactly the clustered members.
5. `search_service.rs`: tokenize the query the same way the index tokenizes documents (shared
   function), rank by `importance desc, timestamp desc`, cap 100 results, group by category for the
   palette UI.
6. `summary_service.rs`: today's counts via `events(importance, timestamp)`, status line strings
   exactly as §72, top 3 important events.
7. `synthetic.rs`: seeded `SmallRng`-free deterministic LCG (no new dependency), profiles as
   specified, realistic names (`nvlddmkm`, `Audiosrv`, `Discord.exe`, `api.exe`, `docker-compose.yml`,
   `1.1.1.1 → 192.168.1.1`), and a `--relations` flag that also writes entity relationships so P15 has
   graph data to traverse.
8. Wire `simulate_event` to the generator's single-event constructors; gate the command with
   `#[cfg(any(debug_assertions, feature = "devtools"))]` **and** a `developer_mode` setting check.
9. `perf_scale.rs`: `seed(profile)` then assert the 10k budgets; print measured numbers; write them to
   `docs/PERFORMANCE.md` manually in the same commit.
10. Commits: (a) `feat: importance classifier`, (b) `feat: query service with keyset paging and
    clustering`, (c) `feat: search service`, (d) `feat: synthetic event generator and simulation
    commands`, (e) `test: query, cluster, calmness and 10k performance`.

## Todo (commit boundaries)

- [x] Importance table + calmness test
- [x] Query service + cursor + index-plan assertions
- [x] Clustering + labels + expansion
- [x] Search service + shared tokenizer
- [x] Dashboard summary with exact §72 strings
- [x] Synthetic generator + `simulate_event`
- [x] 10k budgets measured and recorded

## Success Criteria

- [x] 10,000 synthetic events: page/summary/search all inside the recorded budgets, numbers in `docs/PERFORMANCE.md`
- [x] `EXPLAIN QUERY PLAN` shows an index for every filter combination the UI can produce (test)
- [x] Clustering collapses a 40-event Docker burst into one item and expands back to exactly 40
- [x] Search finds `Discord`, `25565`, `NVIDIA`, `nginx`, `DNS` in the synthetic corpus
- [x] `get_event_details` returns related events and a correct `prior_occurrences` count
- [x] Every synthetic row is labelled `source_collector = "synthetic"`; a test asserts no other
      collector id can be produced by the generator
- [x] `simulate_event` is unreachable in a release build without the `devtools` feature *and*
      `developer_mode` enabled (test asserts the double gate)

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run rs:fmt && bun run rs:lint
cargo test --manifest-path src-tauri/Cargo.toml --test query_service --test cluster --test importance_calm
cargo test --manifest-path src-tauri/Cargo.toml --release --test perf_scale -- --nocapture
bun run check:bindings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Rust-side clustering breaks page continuity | Cursor paging skips or repeats items around a cluster edge | Cursor is always the last *row* consumed, never the last item; a dedicated test pages through a clustered corpus and asserts the union equals the raw set |
| Search too slow without FTS5 | Search > 40 ms at 10k | The inverted index is already the fallback; if it is the chosen path and still slow, cap candidate terms and add `search_terms(term)` prefix range scans instead of `LIKE` |
| Synthetic data quietly shipping to users | A `synthetic` row appears in a normal run | Double gate (cargo feature + setting) plus a startup assertion that release builds without `devtools` cannot register the generator; a "Developer mode" badge is visible in the titlebar whenever it is on |
| Local-day boundaries wrong across DST | "Today" count jumps at 23:00 | Resolve day boundaries with the OS local offset at query time (not a cached offset) and add a DST-boundary unit test with fixed offsets |
## Security Considerations

- Cursors are opaque but not trusted: decode failures return a typed error and reset to page one; a
  forged cursor cannot express a different table or predicate.
- `limit` and `range` clamped server-side; a hostile frontend cannot request 1M rows (§82 IPC abuse).
- Search input is parameterized (never string-concatenated into SQL) and length-capped at 128 chars.
- The generator is a development tool: §87 forbids presenting it as real monitoring, so its output is
  labelled at the data layer, not just in the UI.
