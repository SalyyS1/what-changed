# Getting started with your app

**Evidence state: development-source guidance; no approved owner-workspace installer or completed native acceptance.**

Development documentation. The existing unsigned What Changed installer does not
include or prove this owner workflow. Use the synthetic website example to explore
the model without supplying your app, credentials, or project files.

## Start with one meaningful change

Choose an app you own or have permission to work on. Name what it does in your own
words. Start with a small capability, such as creating a booking. Do not treat a
detected service, port, or package as proof that the capability works.

Confirm one to three rules through examples. For a booking app: only staff can
reschedule; the booking reference stays unchanged; staff can see the revised time.
Suggestions remain drafts until you review and confirm them.

## Write the request

Save the outcome, an example, what must stay true, and choices you have not decided.
“Let staff move a booking to an available time” is easier to check than “improve
booking.” If email or payment behavior is outside the request, say so in the brief.

Review the Markdown before copying it to the coding AI you already use. Remove
customer data and credentials. Saving a request does not execute a test or contact
a preview. See [Your owner workspace](owner-workspace.md).

## Prepare a safe test environment

Use a separate preview with synthetic records and authorized test roles. Keep real
payments, email delivery, and production records outside the test environment.
The first supported target family is a web booking or CRUD preview with explicit
revision identification, deterministic locators, and selected read-only evidence
adapters. A pasted URL alone cannot prove the current backend or build.

Runner implementation and packaging are under development. Clean-Windows execution
and enforceable network limits remain acceptance work. Manual observations retain
their manual label; a configured build name alone cannot establish fresh evidence.

## Review the result and decide

Check the candidate, scenario revision, expected result, and observed result.
Missing roles or permission block the run. A changed candidate or rule can make
older evidence stale. Never interpret either as a pass.

Accept, reject, or defer explicitly. Acceptance records your decision, with any
acknowledged gaps. It does not turn incomplete checks into passing tests.
See [Verification](verification.md) and [Privacy](privacy.md).

## Come back for the next change

Review the last decision and preserved rules before making the next request.
Portable memory is a handoff, not a grant to run tests or trust imported evidence.
Keep the original records available when changing your builder or moving machines.
