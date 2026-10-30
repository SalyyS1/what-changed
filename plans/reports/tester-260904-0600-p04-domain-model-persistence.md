# P04 Domain Model & Persistence: Test Report

Date: 2026-09-04 | Branch: main @ fadaccf (working tree clean) | Tester: p04-tester
Scope: full Rust gates + all 7 Rust suites + frontend gates. Read-only verification; no source, plan, or doc modified.

## Gates

| Gate | Command (from `src-tauri` unless noted) | Exit | Result |
|---|---|---|---|
| fmt | `cargo fmt --all -- --check` | 0 | clean |
| clippy | `cargo clippy --all-targets --all-features -j 2 -- -D warnings` | 0 | 0 warnings, "Finished dev profile" |
| test | `cargo test --all-features -j 2 -- --nocapture` | 0 | 131 passed / 0 failed / 0 ignored |
| svelte-check + tsc | `bun run check` (repo root) | 0 | 798 files, 0 errors, 0 warnings |
| vitest | `bun run test` (repo root) | 0 | 4 files, 89 passed, 74.05 s |

## Rust suites

| Suite | Tests | Pass | Fail | Wall |
|---|---|---|---|---|
| `--lib` (unit) | 86 | 86 | 0 | 1.06 s |
| `--test architecture` | 7 | 7 | 0 | 0.61 s |
| `--test capability_allowlist` | 1 | 1 | 0 | 0.01 s |
| `--test db_recovery` | 7 | 7 | 0 | 0.99 s |
| `--test migrations` | 8 | 8 | 0 | 0.35 s |
| `--test redaction_leak` | 8 | 8 | 0 | 0.44 s |
| `--test repository` | 14 | 14 | 0 | 5.56 s |
| main.rs bin / doc-tests | 0 | n/a | n/a | n/a |
| **Total** | **131** | **131** | **0** | |

## Measurement lines (verbatim, `--nocapture`)

```
throughput: 10000 events in 5.0908089s = 1964 events/s (101.816178ms per batch of 200)
page at 5,000 deep: 200 rows in 4.1382ms
storage: 5509120 bytes database, 4791592 bytes wal, 1030 bytes/event
redaction: 698.044µs per 1060 bytes = 674.337µs per KiB
```

Debug profile, `-j 2`, D: volume near full. Treat as a lower bound, not the release number.

## Criteria coverage

| # | Criterion | Proving test | Status |
|---|---|---|---|
| 1 | `events.db` under Tauri app data dir, WAL present, `quick_check` ok | `src/storage/db.rs` `a_fresh_open_is_ready_at_the_latest_schema` (db + wal exist); `src/storage/connection.rs` `a_fresh_writer_is_in_wal_mode_with_the_documented_pragmas` (journal_mode=wal, quick_check ok); `src/app/paths.rs` `the_resolved_folder_is_named_by_the_identifier` (name only) | PARTIAL. WAL + quick_check proven in temp dirs; no test opens under the real Tauri data dir |
| 2 | 13 §10 tables + 3 additional + 8 indexes from `sqlite_master` | `tests/migrations.rs` `every_mandated_table_and_index_exists_after_migrating_from_empty` (17 names incl `events_fts`; 8 (table, cols) pairs read back from index SQL) | PASS |
| 3 | `events.occurred_at_is_uncertain` exists, default 0 | `tests/migrations.rs` `the_provenance_column_exists_and_defaults_to_zero` (PRAGMA table_info: INTEGER, dflt "0") | PASS |
| 4 | Table-has-migration assertion fails on deliberate bad query | `tests/architecture.rs` `the_table_assertion_can_fail` (bad `collector_state` reported missing; good `collector_baselines` clean) via shared `missing_tables` helper; gate = `every_table_named_in_rust_is_created_by_a_migration` | PASS |
| 5 | `ENABLE_FTS5` in compile_options at startup; 002 has one body | `tests/migrations.rs` `fts5_is_compiled_in_and_the_search_migration_has_one_body`; `connection.rs` unit test calls `assert_fts5`; `Db::open` calls `assert_fts5` before migrating | PASS |
| 6 | 10,000 events in batches in one measured run; throughput written to `docs/PERFORMANCE.md` | `tests/repository.rs` `ten_thousand_events_insert_in_batches_within_one_measured_run` (count==10000, <60 s) | PARTIAL. Test passes; `docs/PERFORMANCE.md` has NO insert-throughput row (grep insert/throughput/events per/10,000 hits only line 243 idle-CPU prose; "Query and analysis budgets" table empty) |
| 7 | Keyset pages stable, non-overlapping under concurrent inserts | `tests/repository.rs` `keyset_pages_are_stable_and_non_overlapping_under_concurrent_inserts` (1000 seeded + 10x50 concurrent; strict order, no dupes, superset of originals) | PASS |
| 8 | Retention: only expired, important+ 90d, checkpoint snapshot never removed | `tests/repository.rs` `retention_deletes_only_expired_rows_keeps_important_longer_and_spares_checkpoint_snapshots` (3 events deleted, 1 snapshot deleted, exact survivors, owned==1) | PASS |
| 9 | `collector_baselines` survives restart byte-identical | `tests/repository.rs` `a_baseline_survives_a_restart_byte_identical` (store, flush, close, reopen, `assert_eq!(loaded, baseline)`) | PASS. Note: struct / `serde_json::Value` equality, not raw byte compare; JSON key order not asserted |
| 10 | Corrupted DB gives `DbState::Corrupt`; all three recovery actions work | `tests/db_recovery.rs`: Corrupt via `a_damaged_file_opens_into_the_corrupt_state_instead_of_failing` + `a_file_that_is_not_a_database_opens_into_the_corrupt_state`; Reset `reset_history_moves_the_file_aside_and_starts_fresh`; Diagnostics `export_diagnostics_writes_a_report_with_structure_and_no_rows`; Repair `repair_on_an_intact_file_is_a_clean_rebuild_that_keeps_every_row` + `repair_rebuilds_what_can_be_read_or_says_it_could_not` | PASS with caveat. Repair-on-corrupt test passes on either outcome branch (see 3b) |
| 11 | Architecture forbids SQL outside `storage/`, `unwrap()` in prod, `cfg(windows)` in core | `tests/architecture.rs` `sql_lives_only_under_storage`, `no_unwrap_or_expect_outside_tests`, `core_modules_carry_no_windows_cfg`; clippy `unwrap_used` / `expect_used` / `panic` = deny | PASS. Caveat: `core_modules_carry_no_windows_cfg` vacuous for `src/analysis/`, `src/collectors/` (dirs do not exist yet); only `src/domain/` scanned |
| 12 | Raw DB bytes contain no fixture secret after the leak suite | **no test proves this** | FAIL. Not implemented (see 3c) |

Summary: 8 PASS, 3 PARTIAL / caveat (1, 6, 10), 1 FAIL (12).

## Code-reading checks

### 3a. Architecture table gate

- YES, gate runs against the real tree: `rust_files(&crate_root().join("src"))` recurses from `env!("CARGO_MANIFEST_DIR")` (`tests/architecture.rs:15-31,178`); `tables_created_by_migrations` reads `migrations/*.sql` (`tests/architecture.rs:146-166`).
- YES, `the_table_assertion_can_fail` proves failure: feeds a `collector_state` query to the same `missing_tables` helper the gate uses (`tests/architecture.rs:168-197`). It does not mutate the gate itself, but the helper is the entire decision path, so this is sound.
- Caveats: text scan only. Keywords = `FROM `, `INSERT INTO `, `INSERT OR IGNORE INTO `, `UPDATE `, `DELETE FROM `. Misses `JOIN x`, `INSERT OR REPLACE INTO`, `CREATE TRIGGER ... ON x`, `ATTACH`. Grep of `src/` shows none of those shapes today, so no current gap. `production_code` cuts at the first `#[cfg(test)]` literal, so a mid-file `#[cfg(test)]` would hide later production SQL. Scans `src/` only, not `tests/`.

### 3b. db_recovery

- Corrupt state: two paths (0xAB overwrite of the middle half; file replaced with text + wal/shm removed) both open into `DbState::Corrupt` without `Db::open_in` erroring. The damaged-file test also asserts flush and read are refused.
- ResetHistory: definitive. `succeeded`, state Ready, quarantine `events.corrupt-*.db` exists, count 0 then 1 after insert.
- ExportDiagnostics: definitive. JSON `state.state == "corrupt"`, sqlite_version, database_bytes > 0, no "Synthetic Service" substring, DB still Corrupt afterwards.
- Repair: `repair_on_an_intact_file_is_a_clean_rebuild_that_keeps_every_row` is definitive (succeeded, row count preserved). `repair_rebuilds_what_can_be_read_or_says_it_could_not` (`tests/db_recovery.rs:143-162`) is `if outcome.succeeded {...} else {...}` and passes on both branches. The damage pattern is deterministic (seek size/4, write size/2 of 0xAB) so the branch taken is stable per build, but the test does not pin which one. Repair on genuinely damaged data is therefore not proven either way. Recommend asserting the observed branch, or seeding two fixtures (partially readable vs unreadable), each with a fixed expectation.
- `the_app_state_boots_over_a_corrupt_database`: AppState::open succeeds over a corrupt DB and is not in safe mode. Good.

### 3c. Persisted secret leak assertion

- **Does not exist.** `tests/redaction_leak.rs` has 8 tests; all operate on `Sanitized<T>` JSON, the tracing log path, or the on-disk log file. None calls `Db::open_in`, `insert_events`, or `baseline_repository::store`, and none reads `events.db` bytes. The `SECRETS` fixture is referenced only from `tests/redaction_leak.rs` and `tests/fixtures/secrets.rs`.
- Phase implementation step 13 ("Extend the P03 leak suite with the persisted assertion") and Success Criterion 12 are unfulfilled. Not written, per instructions.
- Nearest partial: `tests/repository.rs` `config_snapshots_key_on_the_canonical_path_and_go_with_their_folder` asserts `DB_PASSWORD == "[REDACTED]"` after a repository round-trip. That goes through the typed read path, not raw bytes, and uses one hand-written value, not the 12-shape fixture corpus.
- Suggested shape for the implementer: open `Db` in a temp dir; build `NormalizedEvent`s whose summary, metadata, and display name carry every `SECRETS` entry; sanitize + `insert_events`; `flush`; `wal_checkpoint(TRUNCATE)`; `close`; `fs::read("events.db")` plus `-wal` if present; assert no `secret.as_bytes()` window match per fixture. Also cover `collector_baselines`, `snapshots`, `config_snapshots` (all persist `Value` blobs). Add a control that writes a secret through a raw `conn.execute` bypass to prove the byte scan itself can fail.

## Flaky / environment-dependent / weak-assert tests

| Test | Risk | Note |
|---|---|---|
| `repository::ten_thousand_events_insert_in_batches_within_one_measured_run` | env timing | 5.09 s of a 60 s ceiling on debug / `-j 2`; ample headroom, but a full disk (D: near full) or AV scan could inflate |
| `redaction_leak::redaction_stays_within_its_time_budget` | env timing | 674 µs/KiB vs 2000 ceiling (about 3x headroom) |
| `repository::keyset_pages_are_stable_and_non_overlapping_under_concurrent_inserts` | nondeterministic interleave | assertions are invariant-based (order, dupes, superset), sound under any interleaving; the "writer lands inside paged range" case is not guaranteed to occur each run |
| `db_recovery::repair_rebuilds_what_can_be_read_or_says_it_could_not` | no fixed expectation | either-branch pass (3b) |
| `domain::clock::tests::now_is_after_the_day_this_was_written` | wall clock | fails on a machine with a wrong or past clock |
| `app::paths::tests::the_resolved_folder_is_named_by_the_identifier` | real `dirs::data_dir()` | env-dependent; asserts folder name only |
| `redaction_leak::the_installed_logging_stack_scrubs_the_file_on_disk` | global subscriber | installs a process-wide tracing subscriber; fine today because each test binary is its own process, but a second installer in the same binary would fail |
| `architecture::core_modules_carry_no_windows_cfg` | vacuous | `src/analysis/`, `src/collectors/` do not exist yet; only `src/domain/` scanned |
| `architecture::collectors_and_platform_never_read_command_lines_or_environments` | vacuous | `src/collectors/` absent; `src/platform/` scanned if present |
| `testing::temp_dir` (all integration suites) | resource leak | never cleans up: 130 `what-changed-test-*` dirs, 64 MB in `%TEMP%` after this run alone |

## Other observations (non-blocking)

- `src/privacy/sanitize.rs` `into_inner` doc comment still says the repository "does not exist yet". Stale after P04.
- `src/storage/reader_pool.rs` has `std::process::abort()` on an unreachable path; not covered by any test (by design).
- `src/app/paths.rs` `AppPaths::resolve()` is exercised only by name; `Db::open` under the real data dir is never tested (criterion 1).
- Entity-id grammar: `ARCHITECTURE.md:119-122` describes it in prose (process = hash of lowercased exe path, port = protocol + address + number, service = name, container = name). The literal forms the phase file names (`process:<exe_path_lower_sha256_16>` etc., phase line 72) are not in ARCHITECTURE.md; they live only in `src/domain/ids.rs` (14 constructors, 5 unit tests). Not a success criterion, but the phase checklist says "documented in ARCHITECTURE.md".
- `docs/DECISIONS.md` D-11 and D-12 present; `ROADMAP.md` references `health_states`. Phase file status still `pending`; 6 commit boundaries match the phase plan.
- `--all-features` required and honoured; `devtools` feature gates `src/testing`. No `bun run tauri`, no release builds, no `cargo clean` run.

## Recommendations (priority order)

1. **Add the persisted leak test** (criterion 12 / step 13). Blocking for phase sign-off on the privacy contract.
2. **Record throughput in `docs/PERFORMANCE.md`** (criterion 6): 1964 events/s debug, 1030 bytes/event, page-at-5000 4.1 ms, with profile + machine caveat.
3. **Pin the Repair-on-corrupt expectation** (3b): assert the branch actually taken, or split into two fixtures.
4. Add a `Db::open` test using `AppPaths::resolve()` under an overridden data dir (criterion 1), or reword the criterion to temp-dir.
5. Make `temp_dir` register a cleanup (or use `tempfile::TempDir`). 130 dirs per run will fill the volume the lead flagged as nearly full.
6. Extend `tables_named_in_rust` keywords with `JOIN `, `INSERT OR REPLACE INTO `, `ON ` (triggers) so the gate keeps working when P05+ adds those shapes.
7. Fix the stale `into_inner` comment in `sanitize.rs`.

## Unresolved questions

1. Criterion 12 is unmet and step 13 is unfulfilled. Is P04 considered complete without it, or does the lead want it added before status flips from `pending`?
2. Criterion 6 says throughput "is written to `docs/PERFORMANCE.md`". Is the implementer expected to add that row now, or is it deferred to a later performance phase?
3. Should "byte-identical" in criterion 9 mean raw `snapshot_json` text comparison? The current test compares parsed `Value`, which is key-order-insensitive.
4. Is the prose grammar in `ARCHITECTURE.md:119-122` sufficient for "entity-id grammar documented in ARCHITECTURE.md", or should the literal `process:<...>` / `port:<...>` table be added?
5. `repair_rebuilds_what_can_be_read_or_says_it_could_not`: which branch does the current damage pattern take on this build? Not captured (the test prints nothing). Should it be pinned?
