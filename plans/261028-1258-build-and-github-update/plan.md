---
title: "Build current Ownvero preview and update GitHub"
status: completed
created: 2026-10-28
branch: codex/ownvero-owner-workspace
---

# Outcome

Prepare a new installer containing the current features and update the verified
public repository `SalyyS1/what-changed`. Preserve user data, backups and source.
The user explicitly authorized deleting reproducible build/dependency output.
Limit validation remains deferred; do not describe a preview as accepted public v1.

## Execution

- [x] Inspect README, build/notice/publication owners and GitHub target.
- [x] Verify exact debug-cache target, no tracked files/reparse points/running app.
- [x] Record policy-blocked cache cleanup and resolve the build with hosted Windows.
- [x] Prepare exact replacement-runtime packet and receive human review for installer QA.
- [x] Export an allowlisted source update against current public main; scan secrets
  and private evidence before any push. Preserve the original dirty workspace/history.
- [x] Build with matching runtime, notice inputs and exact source identity.
- [x] Inspect the produced artifact and prepare hash-bound publication information.
- [x] Push the authorized source update as a draft PR; installer publication remains open.

## Evidence / constraints

Local D free: 637472768 bytes at start. Verified debug cache: 4244639819 bytes,
6317 files, no tracked file, no reparse point or running project process.
Native PowerShell removal was rejected by automatic approval review with
`blocked by policy`, despite fresh user authorization. No deletion bypass.
Use a hosted Windows build if local space cannot be reclaimed through permitted actions.

The old desktop license approval excludes Node/Playwright/Chromium. G2/G3 and
current candidate evidence remain open; prepare concrete inputs before requesting
any remaining human approval. No claim that the cleanup request reviewed legal terms.

Owner selected a complete runner-enabled candidate; no offline-only release.
The original browser is Chrome for Testing, with distinct executable/AVC terms.
Its runtime notice packet is preserved as rejected-candidate evidence. Owner first
selected verification, then explicitly authorized choosing and validating a
redistributable Chromium replacement. This does not approve a new runtime record.
The [official-source check](../reports/research-261028-1638-browser-redistribution.md)
found no distribution grant for CfT and a prohibition in the applicable public
software terms. Replacement selection and exact notice/source review are complete
for QA; existing CfT bytes must not be distributed. The hosted workflow verifies
sources before native compilation and exact review before bundling/upload.

Source is on [draft PR #1](https://github.com/SalyyS1/what-changed/pull/1).
The first hosted run found a lexical/canonical Windows path mismatch in bundle
validation. Canonical-root comparison preserves redirect/hash checks; twelve local
inventory/release tests passed. The second run then found the same 8.3 alias issue
in native handle validation. Its repair passed managed-lock, lifecycle and redaction
checks locally and on the third hosted run. That run failed later because Git
normalized three approved license texts. All 268 primary/notice records now match
their approved hashes in both worktree and index; byte preservation was pushed in
`8b7284614491d7313c312369be87b1e60d97c25c`.
The [fourth run](https://github.com/SalyyS1/what-changed/actions/runs/40671233391)
passed desktop notice generation and runner-enabled release compilation, then
stopped at the pending human runtime notice gate. No installer was bundled or uploaded.

The owner approved the replacement packet: ungoogled Chromium 156.0.8078.47-1.1,
197 files / 543579634 bytes, inventory `cc3bb9c0…ec5554`. The approval record binds
the exact proposal, notices and matching source-delivery duties. All seven source
archives must pass hosted whole-file/member checks and accompany the installer.
The SDK D3DCompiler side-copy is excluded; actual Windows 11 GPU fallback passed.
Clean Windows 10/11 acceptance remains open.

## Delivered QA candidate

[Build 40730363355](https://github.com/SalyyS1/what-changed/actions/runs/40730363355)
succeeded for `e5906ea6ae2fb06859d7b361df00b36425dd496c`. Its complete artifact
contains the installer and all seven verified source archives with shared retention.
The 175106389-byte installer was freshly extracted without execution; the product
executable, 197-file runtime inventory and both notices passed byte checks.
[Artifact record and hashes](../reports/qa-261029-0042-runner-installer-artifact.md).

Public-v1 publication and installed/clean-Windows/network/performance/soak/real-owner
acceptance remain outside this QA-build result. No merge, Release, tag, download
CTA or website deployment is claimed. Cache reclamation remains policy-blocked.
