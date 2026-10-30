---
phase: 5
title: "Portable briefs and project memory"
status: in-progress
priority: P1
effort: 28h
dependencies: [4]
required_approvals: [G0, G1]
---

# Phase 05 — Portable briefs and project memory

## Goal

Useful repeat handoff to existing coding AI; export/import makes data portable.
Read [domain](domain-evidence-contract.md); existing context_pack remains legacy-specific.

## Inventory

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Create | D:/Project/.WhatChage/src-tauri/src/owner_workspace/{brief,portability,import_validation}.rs | ~120–200 each | Redaction/round trip |
| Create | D:/Project/.WhatChage/src-tauri/src/commands/owner_portability.rs | <150 lines | Scoped file output |
| Create | D:/Project/.WhatChage/src/features/project-memory/{ProjectMemoryScreen,ExportReview,ImportReview}.svelte | ~100–180 each | Preview/permission |
| Create | D:/Project/.WhatChage/src-tauri/tests/{owner_brief,owner_portability}.rs | ~150–200 each | Determinism/untrusted input |
| Modify | D:/Project/.WhatChage/src/lib/tauri/client.ts; central IPC/DTO owners from P03 | Bounded additions | Binding parity |

Portability owner; all shared IPC additions via controller. No changes to legacy
export_investigation_context signature or Mark working behavior.

## Steps

1. Deterministic Markdown brief: desired outcome/example, preserved confirmed rules,
   acceptance scenarios, forbidden effects, out-of-scope, unresolved decisions,
   evidence identity/limitations and exact request ID/version.
2. Preview sanitization, omitted sections, byte/token estimate and copy action. Bound output
   at a documented limit (proposal 32 KiB) with visible truncation, never cut a safety
   rule silently. Select sections or export file when it cannot fit.
3. Add project decisions/history with supersession, filter by capability/change and
   clear owner/manual/import provenance. No remote LLM required or enabled.
4. Export versioned bundle with Markdown + JSON + supported recipe definition; readable
   test steps are always included. Generated Playwright tests come only from verified
   recipe compiler P07, not an untested promise or proprietary-only format.
   Use schema frozen in P03. Before scenario authoring exists, export empty scenario
   list explicitly; P06/P07 must rerun portability acceptance with populated real records.
5. Strip auth, local roots/usernames, internal URLs unless selected, raw logs and capture
   files unless separately reviewed. Escape Markdown/HTML injection and malicious links.
6. Import preview validates schema, size, ZIP traversal/links/duplicate names/bombs,
   checksums and unsupported fields; map app IDs/root links explicitly.
   Apply Windows-specific entry policy from domain contract (ADS/device/reparse/
   case/trailing-dot aliases) with streaming extraction limits and handle containment.
7. Transactional import with idempotency and conflict report. Imported evidence historical,
   not current pass; all execution/target consents reset. No execution on import.
8. Round-trip export of workspace twice must preserve business meaning and immutable
   history; canonical order/time handling produces deterministic manifest for same data.

## Verification

~~~powershell
cargo test --manifest-path src-tauri/Cargo.toml --test owner_brief
cargo test --manifest-path src-tauri/Cargo.toml --test owner_portability
bun run check:bindings
bun run check
~~~
Add component tests beside ExportReview/ImportReview and run named files, then verify.

| Priority | Case | Pass condition |
|---|---|---|
| Critical | Canary secret in text/artifact selection | Sanitized/blocked; no secret copied/exported by default |
| Critical | Malicious archive path/symlink/huge expansion | Rejected before filesystem write |
| High | Imported “passed” result/consent | Historical untrusted, consent reset |
| High | Unsupported version / import interruption | No partial overwrite |
| Medium | Brief exceeds limit | Visible selection/omission, invariants not silently dropped |

- [x] Clipboard Markdown first channel works with preview.
- [x] Decisions and evidence remain exportable offline.
- [x] Round-trip, hostile import and secret-canary tests pass.
- [x] Import never grants execution permission or current verification.

Evidence: `owner_portability` 8 tests and `owner_brief` 7 passed; historical
malformed recipes remain archival and cannot be activated. Source UI component
tests passed. Current-source debug Tauri verified the reviewed Unicode Markdown
clipboard copy, native Save/Open dialogs and ZIP import into a separate app.
The synthetic bundle retained two requests and their confirmed rule; the original
clipboard and application profile were restored. See the
[native continuation](../reports/qa-261028-0440-offline-continuation.md).
Packaged-candidate acceptance, full verify and the earlier phase dependencies
remain open, so this phase is not complete.

## Risk / rollback

Clipboard leaves app custody; warn before copying sensitive user text, never read clipboard
automatically. Save only to user-chosen path with overwrite confirmation. Delete only
app-owned temp export/import files; preserve user's exported files.
