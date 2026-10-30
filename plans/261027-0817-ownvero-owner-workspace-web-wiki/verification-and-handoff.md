# Verification, ownership and cook protocol

## Execution rules

This is a Deep plan. P01 is detailed; P02–P12 are executable contracts that must
be re-scouted immediately before implementation because current tree is dirty and large.
Do not run a giant unreviewed rewrite from these proposed path names.

Each phase:

1. Read plan + phase + referenced contracts; inspect actual owner files/tests.
2. Confirm upstream approvals/dependencies and resolve Create/Modify paths against current tree.
   Read required_approvals/publish_approval + approval-ledger.json, not numeric DAG alone.
   P01 implements explicit fail-closed guard; ak metadata alone is not enforcement.
3. Record baseline failing tests separately from new failures; never hide either.
4. Implement smallest coherent slice preserving full requested scope; write invariant tests first.
5. Focused tests → binding/type/build/lint/integration as affected → independent review.
6. Run real native/browser proof where required; DOM stubs cannot prove native runtime.
7. Update checkboxes only with named evidence, exact source/candidate and command exit/result.
8. Re-read dependent contracts after change; ask before changing user decisions.

Failure protocol: stop affected phase, reproduce narrowly, diagnose cause, repair and
re-run regression. Repeated uncertainty about authority/schema/security → report gate,
do not broaden permission, weaken assertion, fabricate evidence or auto-skip checks.
Implementation request/--auto never authorizes new publication, production actions or secrets.

## Ownership and scheduling

Default dependency DAG is serial 01→02→03→04→05→06→07→08→09→10→11→12.
This avoids shared lifecycle/DTO/router/package/brand collisions. After interfaces
freeze, controller may delegate read-only review or disjoint internal subtasks.
No two writers own migrations, generated bindings, lockfiles, AppState/lib handlers,
shared route registry, release manifest, notices, brand exports or Pages workflow together.

| Owner | Exclusive write boundary | Integration reservation |
|---|---|---|
| Controller | Gates/source reconciliation/phase state | package + lockfiles + shared config |
| Backend | owner_workspace, repositories, privacy coordinator, DTOs | IPC/bindings generation serialized |
| Workflow UI | owner-workspace/change-request/memory/acceptance | routes/screens/shell serialized |
| Runner | runner/, preview_runner, packaged resources | lifecycle/permissions/release serialized |
| 2D | owner-graph + owner-map projection/list | shared contract frozen before 3D |
| 3D | three/owner-map + 3D component | shared renderer modifications reviewed against legacy |
| Brand/site | assets/docs/wiki-site | No simultaneous generated asset/site manifest writes |
| Release | candidate hashes/notices/installer/site release | All other writers stop at freeze |

Do not create branches from clean HEAD and lose dirty repairs. P01 decides how to
preserve baseline first; if a new branch is appropriate use codex/ownvero-owner-workspace.
No commit/push/PR needed to finish planning. No force push/reset/whole-tree cleanup.

## Cross-phase acceptance matrix

| ID | Priority | Scenario | Owning phase(s) | Gate |
|---|---|---|---|---|
| A01 | Critical | User can repeat change with preserved confirmed behavior | 04–07 | Two real iterations |
| A02 | Critical | AI/import/repair cannot relax assertion without owner | 03,06,07 | Negative tests |
| A03 | Critical | Unknown/stale/build swap never current pass | 06,07,08 | Status matrix + race |
| A04 | Critical | Backend isolation not inferred from hidden UI | 06,07 | Named API check or explicit untested |
| A05 | Critical | Persistence not inferred from success toast | 06,07 | Separate read-back role/session |
| A06 | Critical | No production/network/action without permission | 01,02,07 | Consent + deny fixtures/native traffic |
| A07 | Critical | No auth leak/unknown command in runner | 05,07 | Canary/hostile protocol/import |
| A08 | Critical | Cancel/crash/revoke/purge stops child writes | 03,07,12 | Native process + storage proof |
| A09 | Critical | Legacy history/settings/IDs survive upgrade | 02,03,10,12 | Installed upgrade/restore |
| A10 | High | Offline brief/memory/export fully useful | 04,05 | No network with runner disabled |
| A11 | High | Import cannot forge trust/consent/currentness | 05,06 | Atomic import + provenance |
| A12 | High | 2D/3D/list show same evidence/selection | 08,09 | Shared projection fixtures |
| A13 | High | Reduced motion/context loss/no mouse | 08,09 | Native/browser + keyboard |
| A14 | High | GPU/listeners/child resources bounded | 07,09,12 | Repeated mount/run + soak |
| A15 | Critical | Site build excludes private data/IPC/secrets | 11 | Output allowlist + canaries |
| A16 | High | Website docs searchable/versioned, old links work | 11 | Rendered link/base-path/locale suite |
| A17 | Critical | Download matches approved immutable artifact | 11,12 | Manifest→bytes→hash→approval |
| A18 | Critical | New dependencies/notices approved | 07,10,12 | Human-reviewed new packet |
| A19 | High | Nontechnical first-use comprehension | 04,08,12 | Measured pilot sessions |
| A20 | High | Second real change yields repeat value | 06,12 | Early rehearsal + pre-publication pilot, not notifications |

## Commands: observed vs proposed

Observed scripts in package.json: check, test, verify, build, check:bindings,
gen:bindings, rs:fmt, rs:lint, rs:test, check:versions, check:no-network, check:docs,
check:launch, check:brand, check:nsis, check:bundle, release, test:perf, perf:soak.
verify runs frontend check/bindings/tests then Rust fmt/clippy/tests.

Proposed scripts: site:build/site:test/check:site/check:download, check-runner-policy.mjs,
check-owner-release.mjs, package-preview-runner.ps1, check-runner-bundle.mjs,
test-runner-network.ps1, core:no-network alias and check-ownvero-approvals.mjs.
Create and document before running; P07 establishes runtime taxonomy before packaging approval.
Expected test filters must match at least one test. Capture counts and skipped tests;
zero matched tests cannot close a gate. Fixture tests may use controlled failure
injection; final release proof runs installed product and real test app, not mocked product.

## Evidence packet per milestone

Source commit + dirty digest when rehearsal; OS/hardware/viewport; fixture identity and
synthetic label; script/arguments + exit code/counts; artifact hashes; observed result;
known gaps; cleanup and restored machine state. No raw auth/customer/test-secret dumps.
Native screenshot/log evidence local/private unless separately scrubbed and authorized.
Public demo has its own synthetic fixture and artifact allowlist.

## Remaining authorizations

G0/G1 design/runtime policy before changing contracts or running preview. G2 name/assets
before public rename. G3 specific notices/artifact/site approval before publication.
Pilot recruiting/data sharing needs explicit participant consent and owner-approved outreach.
Remote repo source reconciliation and a clean non-admin runtime validation machine
are operational prerequisites, not assumptions the agent can mark complete.

## Handoff

~~~text
/ak:cook D:/Project/.WhatChage/plans/261027-0817-ownvero-owner-workspace-web-wiki/plan.md
~~~

Start P01; validate gates. Do not add --auto from a historical unrelated command.
Optional fresh chat/clear is a convenience, not an instruction to discard approvals or
current dirty-worktree evidence. Plan files own status; live Kanban is a mirror.
