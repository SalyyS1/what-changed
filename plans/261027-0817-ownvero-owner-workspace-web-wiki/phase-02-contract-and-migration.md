---
phase: 2
title: "Architecture, compatibility and migration contract"
status: completed
priority: P1
effort: 24h
dependencies: [1]
required_approvals: [G0, G1]
---

# Phase 02 — Architecture, compatibility and migration contract

## Goal / context

Translate approved G0/G1 into executable contracts without stranding What Changed users.
Read [decisions](decision-contract.md), [domain](domain-evidence-contract.md),
[runner](runner-safety-contract.md). Scout lifecycle, backup/purge and installer owners again.

## Inventory / exclusive owner

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Modify after approval | D:/Project/.WhatChage/README.md; ARCHITECTURE.md; PRIVACY.md; SECURITY.md; docs/DECISIONS.md | Bounded owning sections | Promise/claim parity |
| Read/prepare changes | D:/Project/.WhatChage/src-tauri/src/app/paths.rs; app/state.rs; app/lifecycle.rs | Existing | Multi-store startup/shutdown |
| Read/prepare changes | D:/Project/.WhatChage/src-tauri/src/storage/db.rs; storage/purge.rs; storage/recovery.rs | Existing | Legacy recovery preserved |
| Create | D:/Project/.WhatChage/docs/ownvero/architecture.md | 150–220 lines | New durable owner after gate |
| Create | D:/Project/.WhatChage/plans/261027-0817-ownvero-owner-workspace-web-wiki/reports/compatibility-matrix.md | <160 lines | All contract consumers mapped |

Architecture owner alone edits docs/contracts. Backend/runner implementations wait
for committed interface decisions, not necessarily a git commit.

## Steps / architecture

1. Confirm desktop-primary boundary and exact offline-vs-opt-in behavior. Replace
   absolute promises only after G1; keep meaningful core no-network checks.
   Mark new behavior planned until implemented and verified; contract approval is not
   evidence that the current binary already has the new workflow.
2. Freeze entities, immutable revisions, status axes and result manifests from domain
   contract. Resolve all DTO names, field units/nullability/max sizes in a contract table.
3. Choose separate owner-workspace DB under existing AppPaths as recommended; prove
   it does not change event DB schema/retention. Shared references to legacy project
   are soft, validated links; no false atomic cross-DB transaction claim.
4. Specify AppState ownership, startup integrity errors isolated per store, bounded
   writer queues, cancellation before close, and safe mode leaves runner off.
5. Define global purge journal: close admission/persist purge intent → revoke/cancel/drain → purge each
   owned store/artifact root → mark completed, retain the non-sensitive epoch.
   Interrupted purge resumes idempotently
   before collectors/runner restart; success only after all roots are cleared.
   Include all mutation/artifact producers behind admission barrier + durable epoch.
   Enumerate live/WAL/SHM, backups, recovery/quarantine and failed replacement files;
   inspect legacy recovery.rs/purge.rs, not only current database row counts.
6. Define backup using SQLite-consistent backup/checkpoint method, not raw open-WAL
   copying. Manifest binds both stores/artifact inventory; partial backup is invalid.
   Define staged multi-root restore journal/generation and quiescence. Crash at each
   boundary must recover one whole generation before either store accepts operations.
7. Freeze downgrade policy: old executable can access legacy history but not new store;
   removal/restore of new store is explicit, never silently on failed migration.
8. Map display brand vs install identity. Preserve dev.whatchanged.app, package/library,
   autostart command path and current uninstall registry identity until native proof.
9. Document change authority and effect boundaries. No production safety inferred from
   UI-only tests; no generic executable or remote Tauri bridge.
10. Review integration file list and test matrix before P03. Unresolved cross-store or
    migration behavior is a blocker, not a TODO delegated to a later code writer.

## Test matrix

| Priority | Scenario | Required proof in implementation |
|---|---|---|
| Critical | One store corrupt/newer schema | Other data remains inspectable; runner disabled; no wipe |
| Critical | Purge interrupted after first DB | Recovery marker finishes deletion before accepting new work |
| Critical | Upgrade/downgrade | Legacy IDs/preferences/baseline pointers survive; no silent reset |
| High | Old app running during restore | Operation refused; no simultaneous file replacement |
| High | Soft project link removed/moved | Owner app retained, link marked unavailable; no source deletion |
| Medium | Docs new vs old claims | Every user surface names opt-in behavior consistently |

## Verification / success

This phase owns design tests/specs, not fake passing runtime evidence.
Existing commands to later protect: bun run check:docs; bun run check:no-network;
cargo test --manifest-path src-tauri/Cargo.toml --test migrations;
cargo test --manifest-path src-tauri/Cargo.toml --test privacy_purge.
New owners/test names declared in P03/P07 must be implemented before execution.

- [x] G0/G1 approved changes captured in durable docs.
- [x] Schema/IPC/lifecycle/purge/backup/identity contracts reviewed.
- [x] Legacy rows and new owner records have distinct meanings.
- [x] Cook ownership map complete; P03 may start without architecture guesses.

Design evidence: [architecture](../../docs/ownvero/architecture.md),
[compatibility matrix](reports/compatibility-matrix.md),
[lifecycle review](reports/review-offline-evidence.md) and
[generation implementation review](reports/worker-261028-data-generation-recovery.md).
Runtime/installed acceptance belongs to the implementation phases and remains
separately recorded in [execution](reports/offline-execution.md).

## Risk / rollback

Two DBs add lifecycle complexity; mitigation is an explicit coordinator, not a
general persistence framework. If spike cannot prove idempotent purge/backup,
revisit storage choice before migration code. Roll back only newly introduced
docs/interface changes; never rewrite applied migration checksums.
