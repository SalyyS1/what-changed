---
title: Prepare public workspace preview and repair Windows packaging validation
date: 2026-10-28
summary: Push reviewed source; repair CI path aliases while runtime redistribution remains under review
---

# Prepare public workspace preview and repair Windows packaging validation

## What happened

Prepared an allowlisted source export on the public repository's existing history and pushed draft PR #1. Original dirty repository, app data and QA backups stayed local. Automatic review rejected debug-cache cleanup, so compilation moved to a hosted Windows runner.

GitHub rejected a synthetic Slack fixture; no bypass was used. Rust concat literals preserve all twelve fixture records byte-for-byte, verified with a standalone Rust comparison. The first CI run then found bundle containment comparing lexical roots with realpath entries. Canonicalizing the audited root fixed the Windows alias mismatch; twelve local inventory/release tests passed, including junction rejection.

## Decision

The owner wants the complete packaged runner. Verification found no applicable
distribution grant for Chrome for Testing, and the owner authorized a suitable
Chromium replacement. Official snapshot 1654400 passed the runner but contained
placeholder credits. Ungoogled Chromium 156 passed the same checks and supplies
497 full credit bodies. Its source/build material is pinned for delivery alongside
the installer; exact notice and source obligations remain a human review boundary.

Hosted native checks exposed an 8.3 alias mismatch in handle validation; expanding
the expected long name preserved handle identity and redirect rejection. A later
notice failure exposed three CRLF license texts normalized by Git. Restoring exact
bytes and marking vendored terms `-text` preserved all 268 reviewed hashes. The
fourth CI run passed notices and release compilation, then stopped at the explicit
pending runtime review gate. No binary release or website deployment was performed.

## Next steps

Finish the new runtime/source packet, obtain its exact human review, and run the
replacement through hosted native loading, source verification and packaging.
Retain artifact-bound release gates. Draft PR: https://github.com/SalyyS1/what-changed/pull/1.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
