---
phase: 7
title: "Authorized preview execution and packaged runner"
status: in-progress
priority: P1
effort: 64h
dependencies: [6]
required_approvals: [G0, G1]
---

# Phase 07 — Authorized preview execution and packaged runner

Implementation continued under the approved feature-first sequence. Native/Node
source, consent UI, captures and runnable ZIP are present. See
[implementation evidence](../reports/implementation-261028-preview-runner.md)
and [deferred acceptance](reports/deferred-acceptance.md). The unchecked gates
below still require their exact candidate evidence.

Local packaging produced 433 inventoried files / 550836112 bytes, and the actual
Rust loader passed its opt-in parity test against those bytes. The feature UI,
route-independent cancellation recovery and current automated exception decision
have focused tests. These do not replace installed native execution, network
confinement or license-delta approval; see the linked implementation reports.

## Goal / mandatory entry

Execute approved recipes on supported test environments with reproducible observations.
G1 and P01 feasibility evidence required. Read the entire [runner contract](runner-safety-contract.md).
This is major runtime/privacy change, not use of an existing devDependency.
Read [supported preview contract](supported-preview-contract.md); a generic URL does
not imply version-bound backend checks are available.

## Inventory / single runner owner

| Action | Full path / owner | Rough size | Test impact |
|---|---|---|---|
| Create | D:/Project/.WhatChage/runner/package.json; D:/Project/.WhatChage/runner/src/{protocol,policy,egress-proxy,execute-recipe,record-locator,evidence,main}.mjs | Each ~100–200 lines | Unit/protocol/security |
| Create | D:/Project/.WhatChage/runner/tests/{protocol,policy,lifecycle,recipe,privacy}.test.mjs | ~150–200 each | Deny/cleanup fixtures |
| Create | D:/Project/.WhatChage/runner/fixtures/booking-app/ | Minimal real test app/server | Two identities/persistence/regressions |
| Create | D:/Project/.WhatChage/src-tauri/src/preview_runner/{mod,supervisor,protocol,policy}.rs | ~100–200 each | Process/IPC |
| Create | D:/Project/.WhatChage/src-tauri/src/commands/preview_runner.rs | <180 lines | Consent authority |
| Create | D:/Project/.WhatChage/src/features/verification-runner/{RunConsent,RunnerStatus,RunProgress}.svelte | ~100–180 each | Clear action/cancel |
| Create | D:/Project/.WhatChage/scripts/{package-preview-runner.ps1,check-runner-policy.mjs} | ~150–200 each | Reproducible artifact |
| Create/modify | D:/Project/.WhatChage/scripts/{check-runner-bundle.mjs,test-runner-network.ps1,check-no-network.mjs}; D:/Project/.WhatChage/docs/deployment-guide.md | Gate taxonomy/installed proof | No core-only green for whole package |
| Modify | D:/Project/.WhatChage/scripts/release.ps1 | Wire new runtime checks when runner lands | Whole-installer disclosure/policy |
| Modify | D:/Project/.WhatChage/src-tauri/{Cargo.toml,tauri.conf.json}; package.json; lockfiles; app lifecycle/IPC | Central serialized | Permissions/bundle/kill |
| Create | D:/Project/.WhatChage/src-tauri/tests/preview_runner_lifecycle.rs | ~180–200 lines | Shutdown/revoke/purge |

Packaging resources paths finalized from spike; do not place executables at arbitrary
writable user-supplied paths. Only controller edits lockfiles and installer configuration.

## Steps

1. Refresh official Tauri/Playwright/Node support docs; pin compatible supported versions
   and browser revision. Record dependency/license deltas; prior notice approval does not cover them.
2. Build local fixture with actual persistence and two role identities plus known regression
   switches. Test fixture mocks external payments/mail only by excluding these integrations;
   never count absent real integrations as covered.
3. Implement recipe interpreter with schema allowlist, resource/time caps and deterministic
   assertions. Reject JS/eval/shell/imported executable payloads before spawn.
4. Implement frozen consent + normalized exact-origin policy; redirects, all frame/API
   requests, popups, WebSockets, service worker and DNS threats tested independently.
   Implement connection-level filtering proxy/no-DIRECT route and fail-closed transport
   policy from runner contract. Traffic observation alone cannot enforce confinement.
   Guardrails are not a promise of backend/OS sandbox isolation.
5. Implement Node process supervisor with Windows owned process-tree handle, no inherited
   user secrets/proxy/env surprises, fixed paths/args, stdin protocol and bounded output.
   Spawn suspended, assign non-breakaway kill-on-close Job, then resume; assignment failure
   terminates suspended child. Test abrupt parent death and early descendant/breakaway race.
6. Implement visible isolated test login per role, guarded locator authoring and secret
   input suppression. Never inspect/use browser sessions from user's daily profile.
7. Produce per-assertion sanitized events; parent verifies message/run/nonce/version and
   derives outcome. Disable raw trace/HAR/video/DOM capture. Screenshot opt-in separately.
   Treat temporary profiles as sensitive, screenshots as untrusted raw artifacts;
   per-capture review, <=7-day default retention, no portable export unless explicitly selected.
8. Cancel, revoke, shutdown, sleep/resume, crash and timeout: drain/kill owned tree, mark
   interrupted, clean profiles/temp roots safely. No restart of mutations without user.
9. Compile exportable Playwright test recipe for supported steps. Runnable export includes
   version/setup/fixture instructions; credentials unfilled and consent not exported.
10. Integrate permission UI and runner health with offline app. Disabled runner makes zero
    activation/background connections. No generic frontend shell permission.
    At this phase introduce release taxonomy: core:no-network (legacy graph check,
    alias preserved), runner bundle inventory, disabled-mode outbound/child absence,
    authorized egress policy and whole-installer process/runtime disclosure. Wire these
    into release.ps1 now; P12 consumes established gates, not their first definition.
11. Test installed artifact on non-admin fresh profile without Node/Bun or development
    node_modules. Verify packaged binary/browser hashes, space, notices, and network scope.
12. Run one successful change, a known regression, a missing role, stale build and
    malicious page fixture. Record expected vs observed, not only process exit.
    Include revision handshake and adapter deny cases; prove one independent owner-
    prepared test app in addition to controlled fixture. Missing adapter/marker is
    disclosed observation-only, and does not close full P07 acceptance.

## Verification

New commands must be introduced and documented before use:
~~~powershell
node --test runner/tests/protocol.test.mjs runner/tests/policy.test.mjs runner/tests/lifecycle.test.mjs runner/tests/recipe.test.mjs runner/tests/privacy.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml --test preview_runner_lifecycle
node scripts/check-runner-policy.mjs
~~~
Then bun run verify; bun run core:no-network (legacy check:no-network alias preserved)
for Rust core only; bundle, disabled-mode and authorized-egress gates all mandatory.
Installed runner acceptance/native observation are separate from dependency graph proof.

| Priority | Case | Pass |
|---|---|---|
| Critical | Production/metadata/private target/redirect escape | Blocked before action; never silently permitted |
| Critical | Unknown assertion, truncated protocol, forged completion | Interrupted/invalid, not pass |
| Critical | Cancel/app crash/purge | All owned descendants stop; data not resurrected |
| Critical | Malicious page/credential canaries | No native authority, auth/log/export leaks |
| High | Missing runtime/browser/disk | Friendly unavailable; no npx/download fallback |
| High | Known booking persistence/access regression | Named failing assertion found; missing role blocked |
| Medium | Rate/time/size caps | Bounded explicit termination, partial evidence retained |

- [ ] Real recipe execution and separate owner decisions work end-to-end.
- [ ] All critical deny/lifecycle/leak/fixture tests pass.
- [ ] Fresh installed runtime proof, license review delta and size report exist.
- [ ] Core offline mode and consented network mode independently verified.

## Risk / rollback

If secure target restriction cannot be enforced, remote-run support remains blocked.
Do not mark whole phase done using local-only fixture proof. Disable runner via flag
without deleting memory/evidence; stop owned tree before binary replacement.
Browser vulnerability maintenance becomes release obligation. No test touches production.
