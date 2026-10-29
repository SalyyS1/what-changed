---
title: "Phase 4: Integration validation"
status: done
priority: P1
---

# Phase 4: Integration validation

## Overview

Connect workspace scope, baseline and AI handoff into one calm recovery loop and verify the product contract.

## Requirements

- [ ] Home exposes current workspace and a direct Investigate action.
- [ ] Investigations can use workspace baseline as the default window without hiding the explicit window picker.
- [ ] A user can reach: select workspace, mark working, investigate, copy pack.
- [ ] No-cause, partial collector, missing baseline and unavailable workspace states are honest and actionable.

## Files

- Modify src/features/home/HomeScreen.svelte, investigation stores/components and navigation only where needed.
- Add integration tests using existing fixture injection patterns.
- Update the smallest owning product documentation surface after behavior is verified.

## Validation

- Run focused Rust and frontend tests first, then affected callers, bindings, no-network and full lint/type checks.
- Verify generated bindings are in sync.
- Verify normal build has no updater, network or AI endpoint introduced.
- Review privacy, accessibility, keyboard focus and 900x600 layout states.

## Success criteria

- Three-action recovery flow works with complete and partial evidence.
- Copy payload is deterministic, redacted and useful without an embedded model.
- No existing phase acceptance row is marked complete from this feature alone.

## Risks and rollback

- If baseline migration or pack privacy tests fail, stop before UI polish; revert only feature-owned files and preserve unrelated dirty work.
- Rollback removes the new workspace baseline field and pack command only after backup and migration policy review.

## Todo

- [x] Add integration tests and docs update.
- [x] Run verification gates and record unresolved limitations.

