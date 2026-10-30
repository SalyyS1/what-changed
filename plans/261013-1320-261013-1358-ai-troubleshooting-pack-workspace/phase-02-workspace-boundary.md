---
title: "Phase 2: Workspace baseline"
status: done
priority: P1
---

# Phase 2: Workspace baseline

## Overview

Let a workspace own one known-good checkpoint so investigation can start from a concrete working state.

## Requirements

- [ ] Add nullable baseline checkpoint reference to projects through the next additive migration.
- [ ] Expose set and clear baseline operations with project and checkpoint validation.
- [ ] Mark working uses the existing checkpoint capture path and records a user-visible reason.
- [ ] Missing, deleted and partial checkpoints remain explicit; never silently fall back.

## Files

- Add the next migration under src-tauri/migrations/ and register it in src-tauri/src/storage/migrations.rs.
- Modify src-tauri/src/storage/project_repository.rs, src-tauri/src/commands/settings.rs, DTOs and generated bindings.
- Reuse src-tauri/src/checkpoint/service.rs and src-tauri/src/commands/checkpoints.rs; do not duplicate capture logic.
- Add repository, migration and command tests.

## Implementation

1. Back up the development database before local schema testing through the existing migration runner contract.
2. Add baseline metadata and repository methods with transactionally validated project/checkpoint ownership.
3. Add mark_project_working that captures a checkpoint, then associates its committed ID; partial or failed capture never advances baseline.
4. Add clear and replace behavior and surface checkpoint timestamp/name.

## Success criteria

- Fresh and upgraded databases migrate cleanly and record a checksum.
- Backup-before-migration behavior is exercised for the migration path.
- Deleted or invalid baseline returns a typed error and leaves the project unchanged.
- Existing checkpoint and migration suites remain green.

## Risks and security

- Migration is additive and uses the existing backup policy.
- No snapshot state or environment variable value is copied into the project row.

## Todo

- [x] Implement migration and repository/command contract.
- [x] Add tests for fresh, upgrade, replace, clear, partial and missing paths.

