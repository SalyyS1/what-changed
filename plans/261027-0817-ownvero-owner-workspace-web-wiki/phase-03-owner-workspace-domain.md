---
phase: 3
title: "Owner domain, storage and typed IPC"
status: in-progress
priority: P1
effort: 40h
dependencies: [2]
required_approvals: [G0, G1]
---

# Phase 03 — Owner domain, storage and typed IPC

## Goal / context

Persist owner-controlled product meaning separately from machine history.
Read [domain contract](domain-evidence-contract.md); scout DB/DTO/writer consumers first.

## Inventory

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Create | D:/Project/.WhatChage/src-tauri/src/owner_workspace/{mod,domain,storage,repository,service}.rs | Each ~100–200 lines; split by domain when needed | Unit/storage lifecycle |
| Create | D:/Project/.WhatChage/src-tauri/src/owner_workspace/migrations/001_owner_workspace.sql | ~150–220 lines | Fresh/upgrade/checksum |
| Create | D:/Project/.WhatChage/src-tauri/src/dto/owner_workspace_dto.rs; D:/Project/.WhatChage/src-tauri/src/commands/{owner_workspace,owner_changes}.rs | ~100–180 each | Validation/IPC |
| Modify | D:/Project/.WhatChage/src-tauri/src/{lib.rs,dto/mod.rs,dto/bindings.rs,commands/mod.rs,app/paths.rs,app/state.rs,app/lifecycle.rs} | Bounded wiring | Both build handlers |
| Modify | D:/Project/.WhatChage/src-tauri/src/commands/{privacy,storage,diagnostics}.rs | Owning integration | Global purge/usage/recovery |
| Modify/generated | D:/Project/.WhatChage/src/lib/tauri/{client.ts,bindings.ts} | Wrappers/generated types | Parity |
| Create | D:/Project/.WhatChage/src-tauri/tests/{owner_workspace_storage,owner_workspace_privacy,owner_workspace_contract}.rs | ~150–200 each | Critical matrix |

Backend owner owns schema sequence and central wiring. Other phases may not edit these
concurrently. Braces above are grouped exact planned filenames, not existing claims.

## Steps

1. Write storage invariants tests first: two apps cannot reference each other's entities;
   immutable revision cannot be rewritten; conflict version rejected; idempotency keys.
2. Implement own stable UUID identity, root link optional, archive/unarchive and bounded
   CRUD. Never reuse installed-program applications table or project path hash as app ID.
   Include real change-draft and rule-revision persistence plus save_change_draft and
   confirm_rule_revision now: P04/P05 depend on them. P06 extends owner_changes with
   scenario confirmation/evidence review; it must not be their first backend implementation.
   Implement capability CRUD/query, environment config, candidate selection and current
   rule/draft read models from command table. Freeze scenario/recipe/adapter schema v1
   and storage now; P06 supplies authoring and evidence semantics, not the first schema.
3. Implement schema ledger/integrity with transactional writes, bounded reads/queues,
   selected app query indexes and pagination. Add no speculative ORM/event bus.
4. Add AppState-owned store lifecycle with startup/recovery and purge coordinator from
   P02. New-store failure must not destroy legacy DB or block export of healthy data.
5. Extend privacy summary/storage usage/diagnostics, global purge and artifact roots.
   Sanitized domain text crosses repository chokepoint; do not claim image sanitization.
   Implement global admission/epoch checks for imports/edits/manual results/export/retention,
   quarantine/backups inventory and staged restore coordinator from P02. Recovery→purge
   byte-canary test includes quarantined DB/WAL/SHM, not just fresh empty tables.
6. Wire typed commands through both lib.rs handlers, COMMANDS and signature source list,
   dto/bindings declarations, export_bindings and client wrappers. Count tests updated
   from actual surface; generated bindings produced by command, never handwritten.
7. Test root removed/relocated, legacy baseline deleted, archived app and restore.
   External project source remains untouched throughout.
8. Measure 20-app/100-scenario fixture queries and writer saturation; record timings
   and bounds, explicit busy errors and backpressure. Commit does not mean display
   refresh may show another app's records.
9. Re-run legacy project/migrations/privacy tests and check full contract generation.

## Verification

Existing: bun run gen:bindings; bun run check:bindings; bun run check; bun run rs:fmt.
Focused after files exist:
~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --test owner_workspace_storage
cargo test --manifest-path src-tauri/Cargo.toml --test owner_workspace_privacy
cargo test --manifest-path src-tauri/Cargo.toml --test owner_workspace_contract
cargo test --manifest-path src-tauri/Cargo.toml --test project_repository
cargo test --manifest-path src-tauri/Cargo.toml --test commands_contract
~~~
Then bun run verify for public IPC/lifecycle changes.

## Test matrix / acceptance

| Priority | Case | Expected |
|---|---|---|
| Critical | App A IDs in app B command | Rejected inside transaction |
| Critical | Crash/full disk during rule save/import | No partial confirmed revision or success |
| Critical | Purge while result pending | Drained; late result cannot resurrect deleted data |
| Critical | Pending import/save/export at purge | Epoch-invalidated commit/promotion cannot recreate data |
| Critical | Recovery then full purge / interrupted restore | All owned copies removed or visible failure; no mixed generation |
| High | Migration checksum mismatch/newer schema | Visible recovery state; no reset |
| High | Current candidate changes during query | Old response discarded by generation/app ID |
| Medium | Bounded query/archives | Stable pagination, no silent truncation |

- [x] Schema/identity/immutability/isolation tests pass.
- [ ] Both store lifecycle and privacy/purge integration verified.
- [ ] IPC generation, both handlers and all consumers match.
- [ ] Legacy tests and full verify pass with recorded command results.

Recorded source evidence: owner storage 7, privacy 4 and capacity/backpressure 3
passed. Final library 626 passed (3 ignored), selected integrations 115 passed
(1 installed-upgrade rehearsal ignored), and bindings/rustfmt/clippy passed.
These include source lifecycle/logger and native Rust recovery tests. See
[capacity measurements](reports/worker-261028-owner-capacity.md) and the exact
command boundaries in [execution log](reports/offline-execution.md).
Unchecked integration items retain installed acceptance and the full-verify
obligation; a selected suite does not satisfy that combined requirement.

## Risk / rollback

Keep old DB read contract untouched. Rollback new code only with new DB preserved
for later recovery; never delete it automatically. Applied new migrations append,
not edit. A partial cross-store operation stays blocked with recoverable marker.
