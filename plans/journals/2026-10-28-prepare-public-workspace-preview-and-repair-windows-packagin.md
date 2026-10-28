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

The owner wants the complete packaged runner and selected verification of the current Chrome for Testing distribution terms. The runtime notice record remains pending; source compilation is permitted, installer bundling/upload remains gated. No binary release or website deployment was performed.

## Next steps

Watch the hosted native tests/compile, finish the official-source browser redistribution report, and retain artifact-bound release gates. Draft PR: https://github.com/SalyyS1/what-changed/pull/1.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
