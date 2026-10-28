---
title: "Build current Ownvero preview and update GitHub"
status: in-progress
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
- [ ] Reclaim reproducible cache where tooling permits.
- [x] Prepare runtime notice source packet and concrete review surface.
- [x] Export an allowlisted source update against current public main; scan secrets
  and private evidence before any push. Preserve the original dirty workspace/history.
- [ ] Build with matching runtime, notice inputs and exact source identity.
- [ ] Inspect the produced artifact and prepare hash-bound publication information.
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
The exact browser is Chrome for Testing, with distinct executable/AVC terms.
Runtime notice packet is prepared. Owner selected verifying the current browser's
redistribution permission first; this does not approve the runtime notice record.
The [official-source check](../reports/research-261028-1638-browser-redistribution.md)
found no distribution grant for CfT and a prohibition in the applicable public
software terms. A browser replacement or separate applicable grant is now pending.
The hosted workflow compiles first, then blocks bundling/upload until that review.

Source is on [draft PR #1](https://github.com/SalyyS1/what-changed/pull/1).
The first hosted run found a lexical/canonical Windows path mismatch in bundle
validation. Canonical-root comparison preserves redirect/hash checks; twelve local
inventory/release tests passed. The [follow-up run](https://github.com/SalyyS1/what-changed/actions/runs/40666073479)
is pending; no new installer is claimed.

Unresolved questions: current browser redistribution permission and exact runtime
human review; artifact-specific publication after successful packaging.
