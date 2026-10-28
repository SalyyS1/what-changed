# Verification and evidence

**Evidence state: development-source guidance; no approved owner-workspace installer or completed native acceptance.**

Evidence answers “what was observed under this expectation and context?” It does
not prove every behavior of the app or replace the owner’s acceptance decision.

## Read the state

- **Passed:** the declared assertions completed and matched the expectation.
- **Failed:** an observed result contradicted an assertion. Review expected and observed values.
- **Blocked:** a prerequisite such as permission, test role, locator, or supported adapter is missing. Nothing passed by omission.
- **Stale:** an observation belongs to a different candidate, rule, scenario, environment, or runner semantics.
- **Unknown:** available identity or evidence is insufficient to establish the claimed result.
- **Interrupted:** an attempt ended before completion. Retry as a new attempt.

Historical imported and manual observations retain their evidence strength. A
friendly build label alone is not proof of the current running revision.

## Freeze the expectation

Check the candidate, environment, rule revision, scenario revision, and assertions
before starting. Do not reduce assertions or rewrite a confirmed rule to make a
candidate pass. If the business requirement changed, record that decision and a
new revision, then compare the different expectations openly.

## Roles, data, and permission

Use explicit test accounts with the required roles in a separate preview. A browser
context isolates browser state; it does not isolate a production database, payment
provider, or outgoing email. Permission is scoped to the selected environment and
attempt, not a standing grant for every future preview.

Read-back and denial checks need the supported adapters and declared roles. Missing
those prerequisites blocks the check. Runner execution and clean-machine packaging
remain under acceptance; the website demo never executes these checks.

Leaving the workspace requests cancellation. If termination is not confirmed, the
app keeps a visible session-specific retry control across screens and retries with
a bounded delay. A successful cancellation request alone is not proof of termination.

## Review evidence

Open the step result and compare the expected and observed behavior. Keep the build
and environment labels in view. Screenshots are observations, not proof that every
backend value is correct, and text redaction cannot reliably remove secrets from
an image. Retain only evidence you need. See [Privacy](privacy.md).

## Retry and accept

Retries are separate attempts. An old failed attempt stays failed even if a later
attempt passes. A change of candidate or expectation requires a new review.

Accept, reject, or defer deliberately. If accepting with unresolved results, record
why and acknowledge the gaps. Never turn unavailable checks into a green score.
