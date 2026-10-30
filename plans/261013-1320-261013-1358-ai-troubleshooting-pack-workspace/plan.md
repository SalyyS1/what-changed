---
title: "AI Troubleshooting Pack + Workspace Baseline"
description: "Turn existing projects, checkpoints and investigations into a grounded AI handoff and project-scoped recovery loop."
status: in-progress
priority: P1
effort: 12h
tags: [feature, frontend, backend, database, privacy]
created: 2026-10-13
blockedBy: []
blocks: []
---

# AI Troubleshooting Pack + Workspace Baseline

## Overview

This plan makes the existing What Changed core useful in a real recovery workflow: a user marks a project as working, investigates a scoped failure, and copies a redacted evidence pack to any AI assistant or teammate.

## Outcome

- A named project root becomes a lightweight workspace using the existing projects table.
- A workspace can own one known-good checkpoint without creating a second project configuration system.
- An investigation can be constrained to that workspace and produce a compact Markdown/JSON-compatible AI handoff.
- The app remains offline, non-elevated, deterministic and non-destructive.

## Constraints

- Reuse existing project registration, watched folders, checkpoints, investigations and relationship evidence.
- No embedded LLM, AI API, cloud sync, telemetry, MCP server, automatic rollback or administrator access.
- Never store secret values; sanitize before persistence, clipboard or export.
- Preserve existing investigation confidence language: evidence and correlation, never unsupported causal claims.
- Any schema migration must use the existing backup-before-migration path and have migration tests.

## Non-goals

- Automatic fixes or registry/service edits.
- A global health score or dashboard rewrite.
- Incident Session and Personal Digest; they remain follow-up candidates after usage evidence.
- Public release readiness or phase checkbox closure for unrelated work.

## Phases

| # | Phase | Status |
|---|---|---|
| 1 | Workspace boundary | Done |
| 2 | Workspace baseline | Done |
| 3 | AI context pack | Done |
| 4 | Integration validation | Reopened: installed recovery loop and clipboard verification |

## Success criteria

- From Home or Investigations, a user reaches a useful workspace-scoped result in at most three primary actions.
- The pack includes symptom, time window, ranked suspects, confidence, relationships, evidence gaps and a next verification step.
- The pack contains no recognised secrets and does not claim causation.
- Existing investigation/checkpoint tests remain green; normal builds contain no network path.
- Empty, partial, missing-project and collector-failure states produce useful explicit output.

## Evidence and handoff

- Current project backend exists in src-tauri/src/storage/project_repository.rs, src-tauri/src/commands/settings.rs and src/lib/tauri/client.ts.
- Current investigations are persisted snapshots owned by src-tauri/src/services/investigation_service.rs and src-tauri/src/storage/investigations.rs.
- Current checkpoints are owned by src-tauri/src/checkpoint/service.rs and src-tauri/src/commands/checkpoints.rs.
- Brainstorm record: plans/reports/brainstorm-261013-1358-product-utility.md.

## Validation

## Validation Log

### Installed acceptance follow-up — 2026-10-27

The installed candidate omitted in-root config changes with empty project
associations. Source now resolves recorded file paths against the exact current
workspace registration before the analysis count/cap; persisted investigation
snapshots remain unchanged. Seventeen focused Rust tests, 571 library tests,
strict affected-target Clippy and the ordinary 339-test frontend suite pass.
Installed retest and external clipboard verification remain open. See
[continuation evidence](../reports/qa-261027-0434-mvp-continuation.md).

### Session 1 — product and UX decisions

- Clipboard Markdown is the first AI-pack output; no file export in this slice.
- Workspace selection is explicit and user-created from a registered project root.
- Mark working creates a checkpoint through the existing capture path and attaches its committed ID to the workspace.
- Existing project registration is reused; no second workspace table is planned.
- Implementation proceeds under the accepted offline, non-elevated, redacted and non-destructive constraints.

### Session 2 — implementation and verification

- Workspace boundary is explicit and stored in the existing `projects` registry; Settings, Home and Investigations share the selected project id.
- Mark working captures through the existing checkpoint service and refuses to advance the baseline for partial captures; the global partial checkpoint remains available.
- Investigations persist an optional project id, filter direct and related project event associations before the 5,000-row cap, and keep old snapshots readable.
- AI context is a bounded, deterministic Markdown clipboard handoff derived from the persisted investigation scope. Removed workspaces are labelled unavailable; no path, raw metadata, secret or network request is included.
- Evidence: `bun run check` PASS; `bun run test` PASS (65 files, 339 tests); Rust all-features library PASS (558 tests, 3 ignored); clippy strict PASS; bindings check PASS; migration integration PASS; no-network check PASS; docs-link check PASS; scoped investigation integration PASS (7 tests); focused post-review UI suite PASS (7 files, 32 tests).
- Review fixes: deleted workspaces export as unavailable, removal clears active scope, historical results show their persisted workspace independently, and clipboard copy requires an explicit confirmation step.
- Independent re-review: all four findings resolved; no blocking regression. Minor remaining coverage gap: no direct test crosses the global ProjectsStore listener into InvestigationStore, though source notification and full suites pass.
- `bun test` was intentionally not used as a product gate: Bun's native runner does not provide Vitest/Svelte/jsdom APIs; the repository script is `bun run test`.
- Plan file validation: `ak plan check .../plan.md --json` PASS. The companion `ak plan update` path form could not resolve the planstore id, so frontmatter and phase files were updated directly; no planstore mutation was guessed.

### Verification Results

- Claims checked: project registration, DTO/client commands, checkpoint capture ownership, investigation persistence and migration backup path.
- Verified: 5 | Failed: 0 | Unverified: 0.
- Tier: Standard planning verification.

### Whole-Plan Consistency Sweep

- Phase links and file ownership match the accepted decisions.
- Clipboard-only output is consistent across phases; file export is excluded.
- Explicit project-root workspace and auto-created baseline are consistent across phases.
- No implementation or unrelated phase completion is claimed by this plan.

### Advisory checkpoint

- GO with boundaries: extend existing projects registry; do not create a second workspace table.
- Baseline is one nullable checkpoint pointer per project. A checkpoint remains global and is never deleted when a project is removed.
- Mark working uses the existing CheckpointService capture/coalescing/progress/error path, then associates only a successful committed checkpoint.
- Pack output is typed, deterministic, sanitized persisted data with bounded Markdown; frontend owns clipboard write and there is no file/network side effect.

