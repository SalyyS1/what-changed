---
title: Approve redistributable runner and inspect hosted QA installer
date: 2026-10-29
summary: Exact Chromium QA packet approved; hosted source checks and release compilation passed
---

# Approve redistributable runner and inspect hosted QA installer

## What happened

The owner approved the exact Node/Playwright/ungoogled Chromium packet for QA, conditional on corresponding-source verification and co-delivery. The runtime inventory contains 197 files; the Microsoft SDK D3DCompiler side-copy is excluded.

Windows tar rejected a filename in the matching Chromium source archive. A bounded Python archive reader retained whole-file hashes and required-member checks without filename conversion. Forty acquisition/archive/source tests passed. The final hosted run verified all seven sources, native authority/lifecycle, exact native runtime loading, notices, release compilation and NSIS packaging, then uploaded the complete QA artifact.

## Decision

Use the hosted Windows build because automatic review rejected local cache removal. Preserve source, backups, current app data and the rejected deletion boundary. Inspect the exact downloaded installer without executing it. Selective local QA must not become standalone installer redistribution; the full Actions artifact retains installer, all seven source archives, notices and relink instructions together for 14 days.

## Next steps

Run 40730363355 succeeded for source e5906ea6ae2fb06859d7b361df00b36425dd496c. Nonexecuting inspection verified the installer, embedded product executable, complete runtime and notices; seven matching source archives remain in the complete artifact. Azure required explicit HTTP ranges, and one unknown-size NSIS member was measured against the pinned source before extraction. Update draft PR #1 with this evidence. Installed acceptance, clean Windows, network confinement, performance, soak, repeated owner use and public-v1 approval remain open.

> Historical work record; current contracts remain in the development-preview and source-build documentation.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
