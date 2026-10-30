# Your owner workspace

**Evidence state: development-source guidance; no approved owner-workspace installer or completed native acceptance.**

The workspace is a local record of what your app should do and why you accepted a
change. The owner makes business decisions; a builder supplies changes; evidence
records what was observed. Those roles stay distinct.

## Confirmed rules

A capability names useful behavior, such as rescheduling a booking. A confirmed
rule names a requirement that should survive the next edit. Examples make rules
clearer: a customer cannot change another customer’s booking; staff must see the
new time after saving.

Draft suggestions are not confirmed rules. Editing a confirmed rule creates a new
revision rather than rewriting the expectation attached to older evidence.

## Change requests and briefs

Keep one desired outcome, a concrete example, preserved rules, unresolved business
choices, and expected evidence together. Review the brief before sharing it with
your coding AI. A brief is readable Markdown, not an autonomous instruction to
edit your app, deploy it, or run arbitrary code.

## Candidates, runs, and decisions

A candidate identifies the version being considered. A scenario describes how to
check behavior. A run records an attempt against a frozen expectation and context.
The owner decision accepts, rejects, or defers that candidate and evidence set.

A passing observation covers its checked behavior. An accepted change may still
have explicitly acknowledged gaps. Acceptance does not recolor failed, stale,
unknown, or blocked results as passed. See [Verification](verification.md).

## Project memory and portability

Keep the last decision and its reasoning available for the next request. Review
the selected sections and redaction before export. Business descriptions can
contain private information even when they are not passwords.

Imports are historical and untrusted. Imported rules need review before activation;
evidence does not become fresh, and imported settings do not grant execution
permission. Keep readable Markdown with your portable bundle so you can leave
without losing the reasoning behind your app.

## Legacy diagnostics

An optional project-root link connects the workspace to local troubleshooting.
It does not infer verified capabilities from machine events. “Mark working” still
creates a diagnostic checkpoint; “Accept change” records a business decision.
See [Migration](migration-guide.md).
