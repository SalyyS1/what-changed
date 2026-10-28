---
phase: 1
title: "Scope lock, source reconciliation and feasibility"
status: pending
priority: P1
effort: 20h
dependencies: []
required_approvals: []
---

# Phase 01 — Scope lock, source reconciliation and feasibility

## Goal / entry gate

Khóa đường triển khai trước khi bỏ công xây pivot. Đây là phase đầu duy nhất có thể
bắt đầu bằng read-only ngay; phần spike/chỉnh contract phải có G0/G1.
Đọc README, toàn bộ plan này, brainstorm và plan MVP cũ trước hành động.
Authority: [decision contract](decision-contract.md), [runner safety](runner-safety-contract.md).

## Inventory và ownership

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Read | D:/Project/.WhatChage/README.md; ARCHITECTURE.md; SECURITY.md; PRIVACY.md | Existing large docs | Extract promises, không sửa ngay |
| Read | D:/Project/.WhatChage/plans/260829-0437-what-changed-desktop-mvp/plan.md | Existing 25-phase index | Carry-forward gate ledger |
| Create | D:/Project/.WhatChage/plans/261027-0817-ownvero-owner-workspace-web-wiki/reports/preflight.md | 100–160 lines | Baseline/decision evidence |
| Create after G1 | D:/Project/.WhatChage/runner/spikes/packaged-runtime-smoke.mjs | <150 lines | Fresh-machine non-admin spike |
| Create after approval | D:/Project/.WhatChage/scripts/check-ownvero-approvals.mjs | <150 lines | Missing/stale/forged ledger denied |
| Reconcile after review | D:/Project/.WhatChage/wiki-site/ and actual remote workflow path | Remote-only at plan time | No overwrite of unknown source |

Primary/controller owns source reconciliation, gates, shared manifests and report.
No parallel working-tree edits. Current branch main @13d60df, 135 pre-plan dirty entries;
a clean worktree alone would omit important uncommitted repairs.

## Steps

1. Read git status/diff names, active plan/todos, exact recent QA report. Record current
   revision + relevant dirty-file digests; do not commit/reset/stash/delete others' work.
2. Resolve root README routes; inspect actual migrations and IPC owners. Re-check source
   map against current checkout. Report stale architecture counts separately.
3. Inventory old 52 open checklist rows: carry into release gate, unchanged legacy obligation,
   or proposed superseded with rationale requiring owner approval. No bulk checkmarks.
4. Read-only verify explicit GitHub repo/Pages metadata with gh; inspect remote tree/diff
   for wiki-site and workflow. Produce merge/reconcile recipe preserving local dirty work.
   If unavailable, record unavailable and make web implementation wait; don't invent source.
5. Record architecture G0 and changed permission/privacy G1 with exact user reply.
   No question on already discoverable paths. If not granted, continue read-only docs only.
   Bind approval evidence to scope digest in approval-ledger.json; implement fail-closed
   guard before phase dispatch. Unknown gates/absent evidence/stale digest reject.
   Test that all-pending ledger cannot authorize P02/P07/public identity/publication.
6. After authority, test pinned Node+Playwright+Chromium resource packaging using a
   controlled synthetic booking fixture, no external user target. Confirm no PATH Node,
   admin rights, generic shell, remote debugging port or personal browser profile needed.
7. Measure packaging size/free disk and process cleanup/cancel. Only 2.21 GB free was
   observed during planning; budget room before builds, no destructive cleanup by default.
8. Capture a vertical acceptance example: desired booking field, preserved persistence,
   candidate build identity, expected assertion, separate owner decision.
   Resolve marker/role/adapter prerequisites against supported-preview-contract.md.
9. Produce handoff record: choices approved/proposed, exact source baseline, owned files,
   unsupported cases, risk estimates and G2/G3 still pending. Re-estimate 432h if spike fails.

## Test matrix / verification

| Priority | Check | Pass condition |
|---|---|---|
| Critical | Dirty source vs new branch/worktree | Required repaired sources retained, no lost edits |
| Critical | G0/G1 absent | No runtime/network policy mutation or target execution |
| Critical | Fresh machine runtime spike | Bundled dependencies only; all child processes terminate |
| High | Remote-only wiki | Exact upstream snapshot/workflow identified or explicit blocked |
| High | Old gates | Every residual row classified with owning evidence, none silently closed |
| Medium | Resource estimates | Installer/runtime sizes and free-disk budget recorded |

Read-only commands: git status --short, git branch --show-current,
gh repo view SalyyS1/what-changed --json nameWithOwner,visibility,url,
gh api repos/SalyyS1/what-changed/pages. Do not run a publish command.
Use installed ak plan parse/validate syntax from --help.

## Todo / success criteria

- [ ] Preflight inventory and source reconciliation recipe recorded.
- [ ] G0/G1 exact approvals or explicit stop recorded; proposed choices not presented as approved.
- [ ] Runtime packaging/network/cleanup spike measured after approval.
- [ ] Old open-gate mapping and one full owner-change example accepted.
- [ ] P02 entry has a stable baseline and no unowned file overlap.

## Risk / security / rollback

Do not inspect personal app DB or .env to plan. Running installed app uses existing
monitoring session; do not kill PID 21820 without fresh ownership/permission check.
Stop only spike processes started here. Revert only owned experimental files after
inspecting diff; restore no user data because preflight must not alter it.
Existing authorization to use current Windows account is preserved. Clean packaging
proof can use an authorized VM/CI machine; no mandatory new human QA account or password.
If packaging or network boundary fails, stop affected implementation and revise
explicitly; do not downgrade browser testing to a fake pass.
