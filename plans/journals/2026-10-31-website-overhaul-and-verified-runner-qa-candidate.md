---
title: Website overhaul and verified runner QA candidate
date: 2026-10-31
summary: "Live 3D landing, exact notice-byte repair, successful hosted QA and remaining release gates"
---

# Website overhaul and verified runner QA candidate

## What happened

Rebuilt the English/Vietnamese landing and wiki around the same interactive 3D evidence workbench. Pages is live at d3efe70; 12 browser checks pass. Three lifecycle/localization findings were repaired. Simulated mobile median LCP was 2.016 seconds; this is lab evidence only.

The first hosted desktop run stopped because three reviewed licenses had lost CR bytes. Restored the exact approved blobs and added a shared early notice-input check. Approval records and terms stayed unchanged. Replacement run 41091508412 succeeded at public source 75dcb67c, mapped from primary d71eaf3. Static installer/runtime/notices inspection passed; all seven browser-source archives remain in the complete Actions artifact.

## Decision and limits

User approval covers the exact runner source delta for QA, not public v1. Preserve legal and approval bytes. Keep public downloads unavailable: native/clean-Windows/soak/pilot and G2/G3 remain open. Prepared a concrete two-asset public format proposal because the full QA ZIP exceeds the per-Release-asset size limit; no format change is approved yet.

The existing product process is not proven session-owned. Preserve it while awaiting confirmation to Quit, test on synthetic data with full backup/restore, and resume. Fresh recovery helpers are preparation only; no native execution or profile change is claimed. The controller must verify those helpers and remaining capacity before live use.

## Next steps

Finish the audited delivery records and safe docs projection. Resolve process authority, then perform reversible native candidate acceptance. Obtain clean Windows and consenting pilot owners. Decide the public packet format before changing manifest bindings or requesting exact publication approval.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
