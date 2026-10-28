---
title: "Codebase audit — What Changed production readiness"
date: 2026-09-10
scope: "main @ 75ec2f1 plus uncommitted src-tauri/Cargo.toml"
status: complete
---

# Codebase audit — What Changed production readiness

## Summary

The repository has a strong foundation, not a finished product. Phases 01–05 are implemented: Tauri/Svelte shell, design tokens, privacy chokepoint, SQLite persistence, typed IPC, tray and lifecycle. The product loop does not work yet: the primary event commands are explicit Phase 06 stubs, there is no collector runtime, no collectors, no investigation engine, and the visible screens mostly render design fixtures.

Verdict: **continue the accepted 21-phase plan; do not rewrite the architecture.** Repair the findings below while their owning future phases land, then add a post-MVP public-release track for brand, documentation and launch.

## Verification evidence

| Gate | Result |
|---|---|
| `bun run check` | 808 files, 0 errors, 0 warnings |
| `bun run check:bindings` | passed before the broad Rust failure |
| `bun run test` | 6 files / 111 tests passed |
| `bun run build` | passed; JS 216.81 kB raw / 67.54 kB gzip; CSS 33.47 kB / 7.44 kB gzip |
| `bun run rs:fmt` + `bun run rs:lint` | passed in broad run |
| `cargo test --all-features` broad/default parallelism | failed from rustc LLVM out-of-memory, then emitted damaged-artifact follow-on errors |
| Critical Rust suites, serial (`CARGO_BUILD_JOBS=1`) | 64 tests passed: architecture 7, capabilities 5, command contract 8, recovery 8, migrations 8, redaction 9, repository 14, wiring 5 |

The broad Rust failure is an environmental/tooling failure, not a test assertion. Phase 20/release scripts must serialize or otherwise resource-bound Cargo on this machine. Fresh verification does **not** establish full `bun run verify` green because the default command remains vulnerable to the OOM.

## Implemented / unfinished split

| Area | Evidence | State |
|---|---|---|
| Toolchain + shell + tokens | `package.json`, `src/app/*`, `src/styles/*` | implemented and tested |
| Redaction + log writer | `src-tauri/src/privacy/*`, `observability/*`, leak suite | implemented; critical suites green |
| Domain + SQLite + typed IPC | `domain/*`, `storage/*`, generated bindings and contract tests | implemented; critical suites green |
| Tray/lifecycle/settings | `app/*`, backed settings commands/stores | implemented with bounded follow-up gaps |
| Timeline/query/search product behavior | `commands/events.rs:20-74` | explicit stubs despite lower-level repository/search primitives |
| Collectors | no `collectors/` or `platform/windows/`; status reports unavailable | not implemented |
| Checkpoints/investigations/maps | commands and screen shells only | explicit stubs/placeholders |
| Trust/release surfaces | P19–P21 only | not implemented |

## Verified findings

### High — Settings partial update is not atomic

`settings_repository::update` writes each field independently (`src-tauri/src/storage/settings_repository.rs:133-158`) on the writer connection, without a transaction. Concrete failure: an IPC payload contains a valid `theme` followed by invalid retention. The theme is committed at line 135; validation returns an error at lines 137–143. The caller sees the whole update fail while part of it persisted.

**Owner:** repair before expanding Settings, ideally Phase 06/P19. Wrap all partial writes and the final load in one transaction; add a test with an early valid and later invalid field proving no state changed.

### High — Recovery is callable while the database is healthy

`db_recover` delegates unconditionally (`src-tauri/src/commands/storage.rs:23-27`), and `Db::recover` accepts every state (`src-tauri/src/storage/db.rs:247-270`). `ResetHistory` quarantines a valid database and creates an empty one (`recovery.rs:173-178`). A compromised/mistaken frontend can therefore invoke reset without the corrupt-state recovery flow or a typed destructive confirmation.

This does not violate the current capability allowlist—Tauri commands themselves are the surface. It contradicts `SECURITY.md`'s claim that destructive operations require explicit user action.

**Owner:** P19 privacy/recovery UX. Refuse Repair/Reset unless state is Corrupt/SchemaMismatch; make healthy-history deletion a separate typed command with confirmation nonce/phrase semantics tested at the Rust boundary, not UI-only.

### High — The planned secure purge proof ignores FTS5 forensic remnants

P19 currently plans delete/checkpoint/VACUUM and raw-byte grep (`phase-19-onboarding-notifications-diagnostics.md:145-158,213-215`). SQLite explicitly warns `secure_delete` does not necessarily scrub FTS shadow tables; FTS3/FTS5 can retain forensic traces. The schema indexes user summaries in FTS5 (`migrations/002_search_index.sql:8-28`). A purge may empty logical rows yet fail the stronger privacy claim in `PRIVACY.md:174-187`.

**Owner:** amend P19 before implementation. Test a unique fixture across main DB, WAL/SHM, FTS shadow content after each purge scope and full purge. For full purge, safest simple route is close connections, create a new migrated DB, atomically replace/quarantine then explicitly delete the old artifact according to the documented guarantee. For scoped purge, rebuild FTS + checkpoint + VACUUM, then byte-scan and state the limits honestly.

### Medium — Migration ledger does not prove every applied version has a row

`verify_ledger` validates only rows that exist (`migrations.rs:145-171`). If an applied version's ledger row is deleted, startup accepts it; `record_applied` only inserts versions greater than `from` (`migrations.rs:174-193`), so the missing evidence never repairs itself. A version-2 DB missing row 1 or 2 therefore defeats the claimed complete drift ledger.

**Owner:** next persistence hardening change. Assert exact versions `1..=user_version`, no gaps/duplicates/extras, before migration; add missing-row tests.

### Medium — Failed event commits are neither retried nor counted as dropped

`writer.rs:191-205` logs a non-corruption commit failure and clears `pending`; `dropped_writes` increments only when queue admission fails (`writer.rs:104-118`). Concrete state: transient SQLite I/O/full-disk failure causes a batch to disappear while Diagnostics still reports zero dropped writes. That makes the recorder's health telemetry false at the failure moment.

**Owner:** Phase 08 supervisor/diagnostics. Count rows lost after commit failure separately (and mark writer/collector degraded); do not blindly retry unknown SQLite failures. Add injected failure-path tests and surface both queue drops and commit drops.

### Medium — App status can present an unusable DB as “Active”

`AppState::open` always initializes monitoring as running (`app/state.rs:135-153`) even when DB state is corrupt; `AppStatusDto` then reports that status (`state.rs:179-188`). Frontend title bar renders `Active` solely from `monitoring.paused` (`src/app/TitleBar.svelte:64-67`). Concrete state: corrupt database boots successfully—as intended—but visible global status says Active while no writes can be accepted.

**Owner:** Phase 06 shell integration/P19 degraded mode. Derive a truthful top-level state (`Active`, `Paused`, `Recovery needed`, later `Degraded`) from DB and collector health; route corrupt state to recovery before ordinary screens.

### Medium — Product navigation is missing the specified Change Map route

The product contract lists Home, Timeline, Investigations, Checkpoints, **Change Map**, System Map, Settings and Diagnostics (`PRODUCT.md:36-50`), but routes provide seven destinations without Change Map (`src/app/routes.ts:34-42`). Existing plan phase 16 builds it yet neither phase 16 nor the current navigation model reconciles shortcut count/copy.

**Owner:** amend P16. Decide the final IA explicitly: eight routes with shortcut policy, or Change Map as investigation-scoped surface and update `PRODUCT.md`. Do not leave the current contradiction.

### Medium — Visible primary controls are inert and Timeline details links are non-actions

Home and Investigations `Investigate` buttons have no handler (`HomeScreen.svelte:42`, `InvestigationsScreen.svelte:59`). Timeline filtering only changes local visual state (`TimelineScreen.svelte:19-38`) and `View details` links back to the same hash without event identity (`TimelineScreen.svelte:72`). This is expected incompleteness, but it is high-risk integration scope because the shell looks interactive before it is connected.

**Owner:** Phase 07/P15. Add stores, query synchronization, keyboard search, details drawer with event id/focus return, loading/cancellation/error paths, and integration tests that prove every primary action changes product state.

## UX / brand assessment

The current visual foundation is coherent: system font decision, AA-tested palette, custom title bar, shared state shell and wrapped accessible primitives are all worth preserving. The icon source is recognizable as “before/after”, but no evidence yet proves 16/20/24px legibility, monochrome tray behavior, Windows high-contrast adaptation, or distinction from the title-bar timeline glyph.

Public-v1 polish should include:

1. Keep the **offset before/after bars** concept; refine spacing/negative space instead of replacing it with a generic radar/AI mark.
2. Build a logo system: master SVG, app icon, monochrome tray light/dark/high-contrast variants, wordmark/lockups, installer assets, social avatar and article/README cover.
3. Test on-device at 16/20/24/32/48/256px, against Windows light/dark/high-contrast and actual taskbar/tray backgrounds; use silhouette and one-color tests.
4. Capture truthful product screenshots only after relevant acceptance fixtures pass. Label synthetic/demo history visibly.

## Plan implications

- Preserve existing phases 06–21 and their dependency graph.
- Insert review findings into owning phases rather than creating a separate architecture rewrite.
- Add phases 22–25 after the release candidate: brand system; wiki/documentation IA; bilingual launch/story assets; public-v1 release audit.
- User decisions recorded 2026-09-10: target public unsigned v1; English canonical wiki/README plus Vietnamese launch article; open-source direction with concrete license selection as a pre-release decision gate.

## External evidence

- Tauri Windows installer docs: default `downloadBootstrapper` needs internet if WebView2 is absent; `offlineInstaller` adds about 127 MB; fixed runtime about 180 MB. Keep install-time network disclosure and test the chosen default: https://v2.tauri.app/distribute/windows-installer/
- SQLite `secure_delete`: ordinary-table erasure has performance/mode caveats and FTS shadow tables may retain traces: https://sqlite.org/pragma.html#pragma_secure_delete
- Microsoft accessibility testing: combine CI automation with keyboard, Narrator, high contrast, DPI and UI Automation inspection; use Accessibility Insights FastPass/Live Inspect: https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessibility-testing
- Process Monitor is an advanced raw real-time file/Registry/process/thread capture and filtering tool. What Changed should position around persistent plain-language cross-source change history and evidence ranking—not “more powerful Procmon”: https://learn.microsoft.com/en-us/sysinternals/downloads/procmon

## Unresolved questions

- Which open-source license: MIT, Apache-2.0, or another? The user chose open source but not the exact license; block public release, not implementation.
- Final Change Map information architecture: eighth top-level route or investigation-scoped view.
