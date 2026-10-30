---
title: "Phase 1: Workspace boundary"
status: done
priority: P1
---

# Phase 1: Workspace boundary

## Overview

Make already-registered project roots visible and useful as user-created workspaces without duplicating watched-folder configuration.

## Requirements

- [ ] Load, create and remove workspace roots through existing project commands.
- [ ] Show project name, canonical path, availability and current Git branch when available.
- [ ] Explain that registration does not store .env or secret values.
- [ ] Keep watched-folder consent separate.

## Files

- Modify src/features/settings/SettingsScreen.svelte and add a focused workspace component/store.
- Reuse src/lib/tauri/client.ts project calls and generated ProjectDto.
- Add frontend tests beside the new component/store.
- Do not add a second project table or alter collector semantics.

## Implementation

1. Add a projects store with loading, empty, error, add and remove states matching existing store conventions.
2. Render a Workspace section in Settings with clear empty state and path/branch/availability copy.
3. Keep add/remove focus behavior and test harness injection consistent with WatchedFolders.
4. Add a lightweight Home/Investigation workspace selector only after the Settings surface is stable.

## Success criteria

- Frontend tests cover loading, empty, ready, add, remove, error and unavailable branch states.
- Existing Settings and onboarding tests pass.
- No new runtime dependency or network capability.

## Risks and security

- A workspace path is personal data; show it only where the user explicitly registered it and omit it from diagnostics by default.
- Project registration continues to use existing watched-folder path validation.

## Todo

- [x] Implement workspace store and Settings/Home/Investigations surfaces.
- [x] Add tests and run focused frontend checks.

