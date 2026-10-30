# Compatibility matrix — owner workspace integration

Source review: 2026-10-28. Scope: P02 contracts and the assigned dual-store lifecycle implementation. This is an execution record, not a shipped-feature claim. G0/G1 and the P02–P06 sequence are approved; G2/G3 and the clean-Windows runner/network gates remain open.

## Contract consumers and owners

| Surface | Existing contract preserved | New boundary / implementation owner | Evidence or remaining proof |
|---|---|---|---|
| Application identity | `dev.whatchanged.app`, roaming data root, `WhatChanged.exe`, `what-changed`, `what_changed_lib` | Display branding independent of install identity; G2 owns rename approval | `app/paths.rs`, `tauri.conf.json`, Cargo/package manifests, NSIS hooks, autostart `--minimized`; no identity changes in this work |
| Legacy SQLite | `events.db`, existing migration SQL/checksums, IDs, retention, checkpoints, project baselines | New write-admission fence in `storage/db.rs`; no owner tables in legacy DB | Existing migration/recovery/privacy suites remain required; unrelated dirty-worktree migrations were not reverted |
| Owner SQLite | No legacy table meanings reused | `owner-workspace.sqlite`; one writer, queue 16, reader pool 2; separate checksummed ledger | `owner_workspace/storage.rs` and `storage/migrations.rs`; owner schema currently 2, with 17 `CREATE TABLE` declarations across migrations 001/002; source is authoritative |
| Owner/legacy references | Troubleshooting project identity remains path-derived | Owner UUIDs independent; optional validated soft legacy project link; not a cross-DB transaction | Owner DTOs, validation/repositories; deleting legacy sources must not delete owner apps |
| Startup | Corrupt/mismatched legacy DB remains inspectable through recovery, not silently reset | Read global journal before either store opens; pending deletion removes owned files before migrations; ordinary owner failure isolated | `DataLifecycle::prepare` + `finish_startup`, `AppState::open_with_registry`; synthetic reciprocal-corruption tests added |
| Shutdown | Flush then mark clean; no hidden process accumulation | Serialize with purge; close global admission, stop maintenance/resume/automatic checkpoints, drain leases/checkpoints, close both stores | `app/lifecycle.rs`; do not mark clean after collector join failure, drain timeout, or flush failure |
| Deferred mutation | Existing synchronous command behavior retained | Owned `MutationLease` moves into the real detached worker; DB epoch tokens reject old writes; validate before promotion | Root owns command coverage; tray checkpoint now moves/validates lease before shared creation helper |
| Full privacy purge | One-use challenge, visible `delete` confirmation; preserve only theme/visualization; reset onboarding | One pending global epoch, both storage gates closed, reversible producer quiescence, exact inventory cleanup, completed epoch retained | `app/data_lifecycle*`, `commands/privacy.rs`; failure retains closed admission and pending intent; no terminal collector `stop_all` on successful purge |
| Scoped deletion | Category and watched-folder purges stay legacy-only | Global mutation leases prevent a scoped operation crossing full replacement | `commands/privacy.rs`; owner records/artifacts are not a side effect of category/folder purge |
| Privacy status | Existing legacy counts/bytes remain unchanged | Separate owner state and owner live/WAL/SHM byte count | `PrivacySummaryDto`, `privacy_summary`; corrupt legacy still reports explicit NotReady rather than fabricated zero counts |
| Settings/onboarding | Reviewed collector preferences and consent fence remain authoritative | Purge disables old collector generations; fresh onboarding can enable surviving workers; checkpoint/maintenance services restart | `app/state.rs`, checkpoint pause/resume API; fresh-onboarding/checkpoint tests added |
| Diagnostics/recovery | Legacy repair/export/reset meanings preserved | Global lease covers recovery/export producers; recovery outputs under app root included in full purge | `commands/storage.rs`, `commands/diagnostics.rs`, `storage/recovery.rs`; no generic external filesystem authority added |
| Backup/restore | Existing per-store migration backups retained until explicit purge | Separate controller/restore-worker implementation: consistent SQLite backups, manifest binding both stores/artifact hashes, staged whole-generation restore | New `app/data_restore.rs`, `storage/data_generation.rs` and managed-file lock modules belong to that owner; this assignment does not claim their runtime proof |
| Downgrade | Old executable may continue using legacy history | Owner corruption/newer schema preserves files; downgrade does not auto-reset owner data | Owner migration integrity/ahead/checksum tests; whole-generation restore remains separate explicit authority |
| Shell/navigation | Legacy diagnostics, Mark working, collector settings and routes remain supported | Owner routes must remain usable when only legacy DB is unavailable; owner Accept change is distinct from Mark working | Root/UI integration owner; `App.svelte`, monitoring/router stores, route/command wiring suites |
| Bridge/security | Main bridged WebView remains bundled/local; capability vocabulary stays closed | Runner/preview must not inherit bridge, profile or generic shell/HTTP/filesystem access | Tauri capabilities/CSP, command allowlist, architecture and no-network gates; P07 not authorized until packaging/network proof |
| Public web/wiki | Existing hash anchors and legacy help remain valid | Static explanation/download surface only; no customer projects, bridge, or user preview execution | Pinned public source snapshot below; G2/G3 and deployment approval still required |

## DTO/schema authority

Avoid a parallel hand-maintained field schema. Rust DTOs and generated TypeScript bindings own names, optionality, enums and units.

| Contract family | Source authority | Compatibility rule |
|---|---|---|
| Apps/capabilities, rule revisions, changes, environments, candidates, scenarios | `src-tauri/src/dto/owner_workspace_dto.rs`; `owner_workspace/domain.rs` and repository modules | UUID ownership checked transactionally; expected-version conflict explicit; operation IDs idempotent; immutable revisions never edited in place |
| Manual runs, observations, review and acceptance | `src-tauri/src/dto/owner_evidence_dto.rs` | Observation outcome, execution state, freshness and owner decision remain separate; manual/imported evidence never becomes a verified automated pass. Recipe assertions now require their matching route/method/fields and frozen environment; unsupported history retains raw observations but projects a blocked gap |
| Project decisions/readable memory | `src-tauri/src/dto/owner_memory_dto.rs`; owner migration 002 | Immutable decision revisions; app/capability/change ownership checked; no rewrite of confirmed rules |
| Portability | `src-tauri/src/dto/owner_portability_dto.rs` | Bundles never import local consent, credentials, or current-build trust; evolving implementation belongs to the portability owner |
| Limits | `src-tauri/src/owner_workspace/mod.rs` and validation modules | Current primary bounds: text 16 KiB, apps 20, capabilities/app 200, rules/app 500, scenarios/app 100, steps/scenario 50, assertions/scenario 1,000, page size 200; bounded nested payloads need their own validators |
| State/error wire shape | `DbState` in `storage/db.rs`, `OwnerStoreState` in owner storage, `dto/trust_dto.rs` | Legacy `state` and owner `status` are distinct tagged enums; no global healthy flag masks one failed store |

## Purge inventory and durable order

The coordinator validates the resolved `AppPaths::under(data)` layout and rejects symlink/reparse parents, roots and selected entries. A matched file that is a directory fails visibly; only the three fixed artifact roots are recursively removed. Unknown names and external projects/exports remain untouched.

| Owner | Exact inventory |
|---|---|
| Legacy live | `events.db`, `-wal`, `-shm`, `-journal` |
| Legacy old copies | `events.corrupt-<digits>.db`, `events.purge-old-<digits>.db`, `events.purge-fresh-<digits>.db`, `events.repair.db`, plus SQLite companions |
| Legacy migration backups | `backups/events-pre-v<digits>.db`, plus SQLite companions; not arbitrary files in `backups/` |
| Recovery diagnostics | `diagnostics-<digits>.json` under the app data root |
| Owner live/copies | `owner-workspace.sqlite` and companions; `owner-workspace.pre-migration-<UUID>.sqlite` and companions |
| Owned artifact roots | `owner-artifacts/`, `owner-temp-profiles/`, `owner-cached-exports/` |
| Logs | Strict regular-file truncation; errors keep purge pending; per-event queue epochs and file-side flush gate prevent old queued/partial lines from reappearing |
| Durable intent | `data-purge.json` + staged `.tmp`; closed enum preferences, version, epoch, pending flag only; completed tombstone retains epoch |

Runtime order: close global command admission and persist pending epoch → close owner/legacy write admission → stop background producers → drain actual command workers → reset collectors/checkpoints → validate inventory → replace both stores → remove old copies/artifacts/log bytes → persist completed epoch → reopen storage → restart services → reopen global admission last.

The initialized file logger participates before inventory validation/deletion. Formatting captures an epoch without waiting for disk; a bounded 1,024-line queue carries that epoch. The file sink rejects stale or paused frames and flushes accepted writes while fenced. The purge guard pauses across truncation, then resumes a fresh epoch on success or failure. Existing redaction and level controls remain in use; failed file flush prevents purge success. Pinned tracing-appender 0.2.5 source was checked to confirm one complete queue frame is passed per underlying `write_all` call.

Startup order: validate/recover journal → if pending, remove both generations and owned copies before SQLite open → open/migrate fresh stores → restore typed appearance preferences → synchronize store epochs → complete durable intent → admit producers. Invalid/partial/backwards journal state fails closed, not as “no journal.”

Completed intent differs deliberately from the earlier plan's “remove marker” wording: removing the only epoch record would let the coordinator reset to zero on a normal restart. Architecture/phase wording should say “mark completed; retain the non-sensitive epoch.”

## Promise and release consumers

- Product/privacy/security authority: root README, ARCHITECTURE, PRIVACY, SECURITY, PRODUCT, `docs/DECISIONS.md`, deployment guide, settings/privacy, FAQ, limitations, release-audit, acceptance walkthrough, public go/no-go, wiki index and launch docs.
- User-visible consumers: PrivacyScreen, OnboardingFlow/copy, SettingsScreen, DiagnosticsScreen, owner workspace/acceptance surfaces. Approved design is not a claim that the current installed binary ships it.
- Regression gates: no-network script + tests; docs/link checks; launch-copy checks; version checks; release/installer/NSIS verification; command/capability/architecture tests; third-party notices manifest/generator.
- Static site restore source: `.qa/ownvero-preflight/upstream-site/`, manifest `.qa/ownvero-preflight/upstream-site-manifest.json`, pinned commit `9f8c0df94118287d989b1b253f619050aa212543`. Original files: `wiki-site/index.html`, `styles.css`, `app.js`, `.github/workflows/deploy-wiki-site.yml`. Retain overview/first-investigation/timeline/workspace/privacy/build/status anchors. No deployment performed.

## Verification status / unresolved gates

- Added focused synthetic tests for pre-open interrupted deletion, retained epochs, stale owner/legacy tokens, detached leases, bounded lease-drain timeout, inventory failures and retry, reciprocal corrupt-store isolation, temporary journal recovery, exact copies/companions, unrelated-file preservation, Windows locked artifacts, reversible checkpoint admission and service restart. Six logger tests cover delayed/partial/paused frames, real nonblocking file truncation, inner buffering, an active writer, flush failure/recovery and frame-header exclusion.
- This agent ran `rustfmt` and scoped `git diff --check`; both passed. Authorized focused checks passed 13 database and 5 capture tests on the library binary compiled with their fixture/synchronization repairs. Controller owns the latest integrated lifecycle/logging and P06 regression runs because of disk/process constraints; do not convert test source into passing evidence.
- Full purge with corrupt legacy data remains explicitly unavailable: existing report fields cannot distinguish unknown deletion counts from zero. No silent zero-count fallback or DTO expansion was authorized.
- The initially identified logging queue gap has an implementation and targeted test coverage in `observability/log_epoch.rs` and `log_epoch_tests.rs`; passing controller-run/native evidence is still required before treating the gate as verified.
- Whole-generation backup/restore and exclusive closed-app ownership were assigned to the separate restore worker. Controller reports pre-open recovery and purge inventory wiring landed; this assignment has not reviewed or verified that new implementation. Single-instance UI plugin setup alone is not a pre-open restoration lock.
- Clean-Windows packaged runtime/network enforcement, native upgrade/downgrade/restore proof, G2 public naming and G3 frozen publication manifest remain open. Existing user monitoring PID 21820 was not touched; no personal appdata, DB, secrets, network or publishing actions were used by this agent.

Unresolved questions: controller to record integrated and native verification; no additional product decision required for the implemented offline lifecycle boundary.
