---
phase: 04
title: "Domain Model & Persistence"
status: completed
priority: P1
effort: 8h
dependencies: [03]
---

## Overview

Define the §9 normalized domain (the single event shape every collector emits), the §10 SQLite
schema with migrations and indexes, the repository that is the *only* SQL site in the codebase, §11
retention/maintenance, and §83 database safety including corruption recovery. Writes accept only
`Sanitized<_>`.

Three tables beyond §10's mandated thirteen are created here, because later phases persist to them and
a table created by the phase that first writes to it is a table some other phase will find missing.
The most important of the three, `collector_baselines`, is what stops the recorder being blind to its
own downtime.

## Requirements

**Functional**
- `SystemEvent` with exactly the §9 fields: `id, timestamp, category, event_type, entity_type,
  entity_id, entity_display_name, severity, importance, source_collector, before, after, metadata,
  related_entity_ids, user_friendly_summary, technical_summary, confidence, sensitive`.
- Enums: `Category` (15 variants exactly as §9), `EventType` (19 variants exactly as §9),
  `Severity` (`info, warning, error`), `Importance` (`informational, notable, important, critical`
  per §54), `Confidence` (`low, medium, high`) — `snake_case` serde, stored as TEXT for query
  readability.
- Tables (§10, migration `001_initial.sql`): `events, entities, entity_relationships, snapshots,
  checkpoints, health_states, collector_status, settings, watched_locations, applications,
  analysis_sessions, analysis_results, schema_migrations`. `health_states` is created now and left
  unused until the §26 Phase-2 feature — noted in `ROADMAP.md` so it is not mistaken for a stub.
- **Three further tables, created here because later phases persist to them** (§10 says "at least",
  so this extends the mandated set rather than departing from it). Each is created by the phase that
  owns the schema, not by the phase that first writes to it, so no phase ships a write to a table
  that does not exist:
  - `collector_baselines(collector_id TEXT, entity_kind TEXT, snapshot_json TEXT, captured_at INTEGER,
    PRIMARY KEY(collector_id, entity_kind))` — the **downtime fix**. A collector's last-known state is
    persisted, so on first cycle after a restart it diffs against what the machine looked like when the
    app was last running instead of treating the current state as a fresh baseline. Without this the
    recorder is blind to every change made while it was closed, which is the window a user is most
    likely to be asking about. Used by P11, P12, P13 and P18.
  - `config_snapshots(path_hash TEXT PRIMARY KEY, path TEXT, keys_json TEXT, captured_at INTEGER,
    format TEXT)` — the per-key baseline P10 diffs against. `keys_json` holds key names and
    *already-scrubbed* values, never raw ones; that is a schema-level restatement of the P03 promise.
  - `investigations(id INTEGER PRIMARY KEY, problem TEXT, window_json TEXT, resolved_from INTEGER,
    resolved_to INTEGER, suspects_json TEXT, total_events_in_window INTEGER, created_at INTEGER)` —
    P15's investigation history. §10's `analysis_sessions` and `analysis_results` remain the normalized
    pair and are **not** left created-and-unused: `analysis_sessions` gets one row per run keyed to the
    `investigations` row, and `analysis_results` one row per ranked suspect with its score terms, which
    is what makes a past ranking auditable after a weight change. `investigations` is the denormalized
    read model the UI pages; the two §10 tables are the evidence behind it.
- `events` carries one further column beyond §9's field list: `occurred_at_is_uncertain INTEGER NOT
  NULL DEFAULT 0`. A change discovered by baseline diff after downtime has a *detection* time but no
  known occurrence time, and P15 must never reason over a fabricated timestamp — an event with this
  flag set is excluded from the `temporal` score term and renders "detected when What Changed started"
  instead of a false relative time. §9 fixes the field list for the collector-facing event shape; this
  is a storage column about provenance, and it is listed here rather than in `NormalizedEvent`'s §9
  fields so the two stay distinguishable.
- Indexes (§10): `events(timestamp)`, `events(category, timestamp)`, `events(entity_id, timestamp)`,
  `events(event_type, timestamp)`, `entity_relationships(source_id)`,
  `entity_relationships(target_id)`, plus `events(importance, timestamp)` (retention + Home counts)
  and `entities(entity_type, display_name)` (search).
- **Migration numbering is allocated by the phase that adds a migration**: next free integer, four
  digits of prefix, in `src-tauri/migrations/` and nowhere else. P04 owns `001_initial.sql` and
  `002_search_index.sql`; the next phase to need one takes `003`. Stated here because P04 owns the
  schema and because two phases previously both claimed `002`, in two different directories — a
  collision that surfaces as a migration failure inside an unrelated phase.
- Stable entity-id grammar, documented in `ARCHITECTURE.md`: `process:<exe_path_lower_sha256_16>`,
  `port:<tcp|udp>:<addr>:<port>`, `service:<ServiceName>`, `app:<hive>:<registry_key>`,
  `driver:<device_instance_id>`, `file:<canonical_path_lower_sha256_16>`, `adapter:<guid>`,
  `container:<docker_id_12>`, `image:<repo:tag>`, `project:<canonical_path_sha256_16>`,
  `update:<KB or update_id>`, `task:<task_path>`.
- PRAGMAs on open: `journal_mode=WAL`, `synchronous=NORMAL`, `busy_timeout=5000`,
  `foreign_keys=ON`, `auto_vacuum=INCREMENTAL`, `temp_store=MEMORY`, `cache_size=-8000` (8 MB).
- Retention (§11): defaults events 30d, detailed snapshots 7d, important events 90d; options
  7/30/90/custom; **checkpoint-owned snapshots are exempt** from the 7d rule (documented decision
  D-12 — otherwise "Compare with checkpoint" breaks after a week).
- Maintenance task: at startup + every 6h; deletes in ≤500-row batches with a 25 ms yield between
  batches; `PRAGMA incremental_vacuum(256)` after a delete pass; full `VACUUM` only from
  Settings → Storage on explicit user action; never on the UI thread.
- Storage usage: `page_count*page_size` + `-wal` file size + per-table row counts.
- Recovery (§83): `PRAGMA quick_check` on open; on failure enter `DbState::Corrupt` and expose
  `db_recover(action)` with `Repair` (`VACUUM INTO` a new file, swap on success), `ExportDiagnostics`,
  `ResetHistory` (rename to `events.corrupt-<epoch>.db`, create fresh). Crash-loop guard: a
  `startup.json` attempt counter; 3 unclean starts → boot with all collectors off ("Safe mode").
- Pre-migration backup: before any migration that is not purely additive, `VACUUM INTO
  backups/events-pre-v<n>.db`; keep the two most recent.

**Non-functional**
- One dedicated writer thread owning the `Connection` (SQLite single-writer discipline); readers use
  a small pool of read-only connections (`Connection::open_with_flags` READ_ONLY, WAL allows
  concurrent readers). No async SQLite, no second runtime (research §4.1).
- Every statement prepared and cached (`Connection::prepare_cached`); every batch in one transaction.
- No `unwrap()`; all errors flow through `AppError` with a `DbError` variant.

## Architecture

`storage/` layout: `mod.rs` (open/init/pragmas/state), `migrations.rs` (runner + `schema_migrations`
bookkeeping + checksums), `event_repository.rs`, `entity_repository.rs`, `settings_repository.rs`,
`snapshot_repository.rs`, `checkpoint_repository.rs` (P14), `analysis_repository.rs` (P15),
`maintenance.rs`, `recovery.rs`, `writer.rs` (owns the write connection + batching loop),
`search_index.rs`. `rusqlite_migration::Migrations` is built from `include_str!`-ed SQL files so
migrations are reviewable diffs; after `to_latest` the runner inserts a `schema_migrations` row
(version, applied_at, sha256 of the SQL) — §10 asks for the table explicitly even though the crate
tracks `user_version`.
## Related Code Files

**Create** — `src-tauri/migrations/001_initial.sql`, `.../002_search_index.sql`;
`src-tauri/src/domain/{mod.rs,event.rs,entity.rs,ids.rs,enums.rs,importance.rs,page.rs}`;
`src-tauri/src/storage/{mod.rs,migrations.rs,writer.rs,event_repository.rs,entity_repository.rs,
settings_repository.rs,snapshot_repository.rs,baseline_repository.rs,config_snapshot_repository.rs,
maintenance.rs,recovery.rs,search_index.rs}`;
`src-tauri/src/app/{state.rs,paths.rs}`; `src-tauri/tests/migrations.rs`,
`src-tauri/tests/repository.rs`, `src-tauri/tests/architecture.rs`,
`src-tauri/tests/db_recovery.rs`, `src-tauri/src/testing/mod.rs` (temp-DB helper, `cfg(test)` +
`cfg(feature = "devtools")`).
**Modify** — `src-tauri/Cargo.toml` (rusqlite 0.40.2 `features = ["bundled", "backup"]`,
rusqlite_migration 2.6.0), `src-tauri/src/lib.rs`, `src-tauri/src/privacy/sanitize.rs` (add the
`Scrub` impl for `NormalizedEvent`), `ARCHITECTURE.md`, `docs/DECISIONS.md`.
**Delete** — none.

## Implementation Steps

1. `domain/event.rs`: `NormalizedEvent` (what collectors emit — no `id`) and `SystemEvent` (what the
   repository returns — with `id`). `before`/`after`/`metadata` are `Option<serde_json::Value>`;
   `related_entity_ids: Vec<String>` stored as a JSON array column. Timestamps are
   `i64` epoch **milliseconds UTC**; the UI localizes.
2. `domain/enums.rs`: the exact §9 variant lists with `as_str()` / `FromStr` and an exhaustive
   round-trip test (a test asserting the variant count catches accidental additions/removals).
3. `domain/ids.rs`: the entity-id grammar constructors + tests (path canonicalization is
   lowercase + `\\?\` prefix stripped + trailing separator removed before hashing).
4. `001_initial.sql`: the 13 §10 tables + the three additional ones above (16 total) + the 8 indexes +
   `settings` seeded with defaults (theme
   `system`, retention `30/7/90`, every collector default per §50: apps, updates, network, drivers,
   services **on**; processes on; files/config on with an empty watch list; dev tools and docker
   **off**), and `collector_status` rows created lazily.
5. `storage/mod.rs`: `Db::open(app_data_dir)` → PRAGMAs → `quick_check` → migrations →
   `schema_migrations` bookkeeping; returns `Db { writer_tx, reader_pool, state }`.
6. `storage/writer.rs`: dedicated `std::thread` owning the write connection; receives
   `WriteCommand::InsertEvents(Vec<Sanitized<NormalizedEvent>>)` etc. over an `mpsc` with capacity
   4096; coalesces for up to 250 ms or 200 events, then one transaction; on channel-full, increments
   a dropped counter and emits one throttle notice per 10 s (§82 event floods).
7. `event_repository.rs`: `insert_batch(&[Sanitized<NormalizedEvent>]) -> Vec<i64>`,
   `page(EventQuery) -> Page<EventSummary>` (keyset: `WHERE (timestamp, id) < (?, ?) ORDER BY
   timestamp DESC, id DESC LIMIT ?`), `by_id`, `counts_for_range`, `prior_occurrences(signature,
   since)`, `delete_expired(...) -> usize`. Every method uses `prepare_cached`.
8. `baseline_repository.rs`: `load(collector_id, entity_kind) -> Option<Baseline>` and
   `store(collector_id, entity_kind, snapshot)`, both taking `Sanitized<_>` snapshots like every other
   write. This is the API P11–P13 and P18 call on their first cycle; it is created here so the
   downtime fix has one implementation rather than four.
9. `search_index.rs`: **FTS5 is present in the pinned `bundled` amalgamation — measured, not assumed.**
   A probe crate against `rusqlite 0.40.2 features = ["bundled"]` on this machine reports SQLite
   3.53.2 with `ENABLE_FTS5` in `compile_options`, and `CREATE VIRTUAL TABLE … USING fts5(…)`
   succeeds, including numeric-token (`3000`), prefix (`port*`) and quoted-phrase (`"0.0.0.0:3000"`)
   matching against an `content=''` external-content table — the three query shapes §51's search
   actually needs. Because `bundled` vendors the amalgamation, this is a **build-time** property that
   cannot vary between users, so `002_search_index.sql` ships **one** body: the FTS5 table. Record it
   as D-11. A runtime `compile_options` check is kept as a startup assertion that fails loudly if a
   future dependency bump drops the feature, not as a branch selecting between two schemas — a
   conditional migration would give version 2 two valid checksums and two possible shapes, which
   defeats both the drift checksum and P21's "assert the new schema version" upgrade proof.
   `trait SearchIndex` stays, because P06 queries through it and a trait keeps the SQL in one place;
   the `search_terms` inverted-index implementation is **documented as the fallback path and not
   shipped** — under §87 an unexercised second implementation presented as real is the thing to avoid.
   If a bump ever removes FTS5, the startup assertion fires and the fallback is written then, against a
   test that constructs the other schema explicitly.
10. `maintenance.rs` + `recovery.rs` per Requirements; `recovery.rs` gets a test that writes garbage
    over a copy of a valid DB and asserts `DbState::Corrupt` + a successful `ResetHistory`.
11. `tests/architecture.rs`: read every `.rs` under `src-tauri/src/`, assert (a) no `INSERT INTO`
    / `UPDATE ` / `DELETE FROM` string outside `src/storage/`, (b) no `.unwrap()` or `.expect(`
    outside `#[cfg(test)]` blocks and `src/testing/`, (c) no `#[cfg(windows)]` inside `src/domain/`,
    `src/analysis/`, `src/collectors/` (only `src/platform/`), and (d) **every table name appearing
    after `FROM`, `INSERT INTO`, `UPDATE` or `DELETE FROM` in any `.rs` file also appears in a
    `CREATE TABLE` in `src-tauri/migrations/`**. (d) is the check that would have caught all three
    missing tables at the phase that introduced them instead of at implementation time.
12. `tests/migrations.rs`: empty→latest; latest→latest idempotent; v1-with-data→latest preserves row
    counts; `schema_migrations` has one row per applied migration with a matching checksum; a
    corrupted checksum is reported, not silently ignored.
13. Extend the P03 leak suite with the persisted assertion: insert a secret-bearing event through the
    real repository, then read the raw DB file bytes and assert no fixture secret substring appears.
14. Commits: (a) `feat: domain event and entity model`, (b) `feat: sqlite schema and migrations`,
    (c) `feat: event repository and batched writer`, (d) `feat: collector baseline repository`,
    (e) `feat: retention, maintenance and recovery`,
    (f) `test: migration, repository and architecture suites`.

## Todo (commit boundaries)

- [x] Domain types + enum round-trip tests + entity-id grammar
- [x] Migration 001 with 16 tables, 8 indexes, seeded settings
- [x] Writer thread + repository with keyset pagination
- [x] Baseline + config-snapshot repositories (the downtime fix's storage)
- [x] FTS5 search index shipped as one migration body (D-11)
- [x] Retention/maintenance + corruption recovery + safe mode
- [x] Migration, repository, architecture, recovery, persisted-leak suites green

## Success Criteria

- [x] `events.db` is created under the Tauri app data dir with WAL files present and `quick_check` ok
- [x] All 13 §10 tables, the three additional ones, and all 8 indexes exist (asserted from
      `sqlite_master` in a test)
- [x] `events.occurred_at_is_uncertain` exists and defaults to 0
- [x] The architecture test's table-has-a-migration assertion fails when a table name is used in Rust
      without a `CREATE TABLE` — proven by a deliberately introduced bad query in the test itself
- [x] `PRAGMA compile_options` contains `ENABLE_FTS5` at startup, and `002_search_index.sql` has
      exactly one body
- [x] Inserting 10,000 synthetic events in batches completes in one measured run and the recorded
      throughput is written to `docs/PERFORMANCE.md`
- [x] Keyset pagination returns stable, non-overlapping pages under concurrent inserts (test)
- [x] Retention deletes only expired rows, keeps `important`+ for 90d, and never removes a
      checkpoint-owned snapshot (test)
- [x] `collector_baselines` survives a process restart: store a baseline, drop the `Db`, reopen, and
      `load` returns it byte-identical
- [x] A deliberately corrupted DB produces `DbState::Corrupt` and all three recovery actions work
- [x] Architecture test forbids SQL outside `storage/`, `unwrap()` in production paths, and
      `#[cfg(windows)]` in core modules
- [x] Raw DB bytes contain no fixture secret after the leak suite

Evidence, from `plans/reports/tester-260904-0600-p04-domain-model-persistence.md`
and the gate re-run that closed its findings: **133 tests green** — 86 library, 7
architecture, 1 capability, 8 recovery, 8 migration, 9 leak, 14 repository — with
`cargo fmt --check` and `cargo clippy --all-targets --all-features -D warnings`
both clean, and the frontend gates (`bun run check`, 798 files; `bun run test`,
89 tests) untouched by this phase.

Ten thousand events insert in 1.74–2.44 s on an idle machine (4,107–5,737
events/s in batches of 200) and in 5.09 s with a second build running; a page
5,000 rows deep returns its 200 rows in 1.08–4.14 ms; the database costs 1,030
bytes per event including the write-ahead log. The spread and the machine state
are both in `docs/PERFORMANCE.md`, because one number taken on a loaded laptop is
not a figure. Decisions D-12, D-18 and D-19 are recorded in `docs/DECISIONS.md`.

Two of the tester's findings were closed rather than filed. The leak suite had no
persisted assertion — the last criterion above — so it now writes all twelve
fixtures through the real repositories as events, a detailed snapshot, a collector
baseline and a config baseline, closes the database, and searches the bytes of
`events.db`, `-wal` and `-shm`, with one ordinary value kept in the clear that the
test requires to find so it cannot pass against the wrong file. Running it exposed
a real leak owned by the previous phase: the URI-userinfo rule was anchored on a
word boundary before the scheme, and the development console writes its ANSI reset
— which ends in a letter — directly against the value, so
`postgres://user:pass@host` reached the console in the clear while the same value
was redacted in the log file beside it. The anchor is gone and every fixture is
now replayed through a coloured formatter as well as a plain one. Separately, the
repair-on-corrupt test accepted either outcome and so could not fail; it is now a
deterministic case (a file that is not a database must be reported as
unrepairable and left byte-identical for the other two actions) plus an invariant
case for partial damage. The temporary-directory helper sweeps its own leftovers
older than an hour, since one suite run leaves about sixty megabytes in `%TEMP%`
and this machine has run out of disk repeatedly.

One criterion is proven structurally rather than end to end, and this file says so
rather than letting the report be read as more than it is: the database is created
under the *resolved* application data folder only when the app itself runs. The
suites assert the layout — database, log directory, backups, startup marker, all
under one root — against a temporary root, because writing into the real user
profile from a test would be the wrong thing to prove. The first real boot is
Phase 05's gate.

## Verification Gate

Four of the five suites use the temporary-database helper in `src/testing/`, which
is compiled behind the `devtools` feature, so every suite runs with
`--all-features` — without it the integration crate cannot resolve
`what_changed_lib::testing` and does not compile. `-j 2` because this machine runs
out of disk and commit charge linking more in parallel.

```bash
cd /d/Project/.WhatChage
bun run rs:fmt && bun run rs:lint
cargo test --manifest-path src-tauri/Cargo.toml --all-features -j 2 --test migrations
cargo test --manifest-path src-tauri/Cargo.toml --all-features -j 2 --test repository
cargo test --manifest-path src-tauri/Cargo.toml --all-features -j 2 --test architecture
cargo test --manifest-path src-tauri/Cargo.toml --all-features -j 2 --test db_recovery
cargo test --manifest-path src-tauri/Cargo.toml --all-features -j 2 --test redaction_leak
```
## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| A future `rusqlite` bump drops FTS5 from the bundled amalgamation | The startup assertion fires: `compile_options` lacks `ENABLE_FTS5` | Measured present at 0.40.2 / SQLite 3.53.2, so this is a regression guard rather than a live branch. Response is to write the `search_terms` inverted index then, behind the existing `trait SearchIndex`, with its own migration number and its own test — not to ship an unexercised second path now (§87) |
| A later phase writes to a table nobody created | Architecture test (d) fails at that phase's own gate | The assertion is the fix: it names the table and the file. Adding the table means a new migration with the next free number, never an edit to `001_initial.sql` |
| `collector_baselines` grows without bound as entities churn | Table row count climbs past the number of live entities | One row per `(collector_id, entity_kind)` by primary key, holding a whole snapshot — the row count is bounded by collectors × kinds, not by entities. A snapshot larger than 1 MB is logged and truncated with a recorded marker rather than stored |
| A stale baseline makes a restart look like mass change | First cycle after a long downtime emits hundreds of events | P08's supervisor coalesces first-cycle diffs per collector into a single "changed while you were away" batch, and every event carries `occurred_at_is_uncertain = 1`; the §54 classifier keeps routine kinds `informational` so the burst does not read as an incident |
| WAL growth on a long-running writer | `-wal` file > 64 MB in the perf run | `PRAGMA wal_autocheckpoint=1000` plus an explicit `wal_checkpoint(TRUNCATE)` after each maintenance pass |
| Write amplification from retention (§11) | Maintenance pass exceeds 500 ms of CPU or spikes disk | Batch size stays 500 with yields; switch to `incremental_vacuum(64)` per pass; never full-VACUUM automatically |
| `rusqlite_migration` 2.6 API differs from expectation | Compile error on `M::up`/`to_latest` | `cargo doc -p rusqlite_migration --open` and adapt; if the crate cannot express the pre-migration backup hook, run the backup in `migrations.rs` before calling the runner |
| Unclean shutdown leaves a hot journal | App start slow or `quick_check` warns | WAL recovery is automatic; the startup attempt counter routes a third failure into safe mode instead of a crash loop |
| Enum drift between Rust and DB text | Round-trip test fails | Enum text is the storage contract; adding a variant requires a migration note in the same commit |

## Security Considerations

- The repository is the single write path and only accepts `Sanitized<_>` — §34 is enforced by types.
- `foreign_keys=ON` plus `ON DELETE CASCADE` on `entity_relationships`, `snapshots`,
  `analysis_results` and search rows prevents orphan growth used as a disk-fill vector.
- §10's "never store arbitrary full file contents" is enforced by shape: `before`/`after` for files
  carry only `path`, `size`, `mtime`, `hash`, and for recognized config files a per-key diff whose
  values are already scrubbed.
- Recovery actions are explicit user choices; nothing deletes history automatically. `ResetHistory`
  renames rather than deletes so the corrupt file can still be inspected.
- The DB lives in the per-user app data dir (no shared/world-writable location); paths come from
  Tauri's path resolver, never from a frontend-supplied string.
