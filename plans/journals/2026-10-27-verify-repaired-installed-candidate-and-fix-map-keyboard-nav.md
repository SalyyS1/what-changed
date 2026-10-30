---
title: Verify repaired installed candidate and fix map keyboard navigation
date: 2026-10-27
summary: Native QA restored safely; keyboard defects fixed and canonical query-plan checks pass; release gates remain open.
---

# Verify repaired installed candidate and fix map keyboard navigation

## Work

Continued the accepted MVP rather than changing product direction. Reused the
repaired installer session; verified workspace event membership, clipboard
restoration, checkpoint comparison, diagnostics export, scoped privacy purge
and 2D fallback. Controller and contents journals confirm normal shutdown,
original profile/startup restoration and monitoring restart.

## Findings and corrections

Native keyboard sampling found the System Map listbox suppressed the shared
focus ring. Browser geometry then proved its outline was clipped by the scroll
wrapper and navigation could hide the active option. Moved focus semantics to
the viewport, synchronized focused scrolling after render, rounded required
deltas, added per-instance option identities and repaired frame-local Escape.
Final browser check passed; four affected test files passed 11 tests, typecheck
and frontend build passed. Historical startup timeouts remain recorded.

Canonicalized the query-plan fixture and wired actual page/checkpoint capture
tests into the perf script. Three exact plan targets and strict Clippy passed;
copied SQL still prevents a whole-service parity claim.

## Limits and next work

The idle diagnostic lost samples at serialization because Set-Content lacks
NoClobber. No CPU result was inferred. Fixed only the disposable writer and
verified refusal to overwrite under PowerShell 5.1. Memory readings remain
measurements under D-15. Hosted wiki projection evidence closes one criterion;
317/369 phase checkboxes complete, 52 open.

The installed candidate predates the new accessibility fixes. Rebuild and retest
it before claiming installed acceptance. Native lifecycle/upgrade/uninstall,
runtime network, four-hour soak, full accessibility, media and frozen-manifest
publication approval remain open. See
plans/reports/qa-261027-0619-repaired-installed-candidate.md.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
