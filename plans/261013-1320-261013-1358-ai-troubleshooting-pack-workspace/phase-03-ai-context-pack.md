---
title: "Phase 3: AI context pack"
status: done
priority: P1
---

# Phase 3: AI context pack

## Overview

Generate a grounded, redacted handoff that users can paste into ChatGPT, Claude, Copilot, a local model or a support ticket.

## Requirements

- [ ] Build the pack from a persisted investigation snapshot plus optional workspace metadata and baseline comparison.
- [ ] Include symptom, resolved window, ranked suspects, confidence, evidence reasons, chain steps, missing coverage and one safe verification prompt.
- [ ] Explicitly say the pack reports evidence/correlation and does not prove causation.
- [ ] Exclude secrets, raw database errors, unbounded event dumps, unnecessary full paths and unsupported claims.
- [ ] Offer Copy context for AI from Investigation and Details; clipboard only for the first slice.

## Files

- Add a focused Rust service/module for pack assembly and redaction; keep IPC registration in src-tauri/src/commands/.
- Extend DTO/bindings and src/lib/tauri/client.ts.
- Add reusable AiContextPackButton.svelte and integrate it into InvestigationsScreen.svelte and the selected details surface.
- Add Rust privacy/format tests and frontend interaction tests.

## Implementation

1. Define a versioned pack schema and Markdown renderer from typed data, not raw SQL or UI strings.
2. Sanitize the final rendered payload again at the boundary; bound suspect and chain counts and output size.
3. Return an explicit partial-evidence section for no-cause, collector failure or missing baseline.
4. Copy through the WebView clipboard API; show success/failure toast and retain focus.
5. Keep the pack usable without an AI service and never send it over the network.

## Success criteria

- Golden tests prove stable output for complete, empty, partial and missing-baseline cases.
- Secret corpus tests prove credentials and sensitive values do not appear in Markdown or JSON.
- UI tests cover copy success, copy failure, disabled/loading states and keyboard activation.
- Existing investigation DTO semantics and confidence wording remain unchanged.

## Risks and security

- Clipboard content is user-visible and can be read by other local applications; show an explicit confirmation and short expiry/clear hint without claiming clipboard security.
- Do not include raw paths by default; use registered workspace name and branch unless path details are explicitly enabled later.

## Todo

- [x] Implement typed pack contract and renderer.
- [x] Add copy action and tests.

