# First supported preview contract

Design proposal, not an existing vendor connector. No claim every AI-built app works.
First family: browser-based booking/simple CRUD app with HTTP JSON backend, stable
role/name/test-id locators, isolated test data and two ordinary test-role sessions.
Any builder may generate it if this contract is met; no undocumented Lovable/Replit API.

## Modes

| Target provides | Permitted claim |
|---|---|
| Approved test preview + supported UI locators only | Timestamped UI observation; backend/persistence/build currency unverified |
| Above + revision handshake + typed read-back adapter | Named persistence scenario on declared tested revision |
| Above + second authorized role + negative API adapter | Named route/record isolation observation, not universal security |

Missing prerequisite results blocked/unsupported; user can still create briefs/portable
memory. Unsupported target does not satisfy P07 real end-to-end acceptance.

## Setup the owner can use

Ownvero produces a one-time “prepare a safe test preview” brief for the user's existing
builder: isolated data/environment, marker, stable labels, role fixtures and the exact
adapter contract below. Owner reviews any infrastructure/business change with builder;
Ownvero does not edit/deploy target source or create accounts automatically.
Onboarding checks prerequisites and gives understandable missing-step guidance.
Pilot measures this setup burden rather than assuming every non-tech owner can do it.

## Version handshake

Proposed target endpoint: GET /.well-known/ownvero.json on approved app origin.
Response schema v1: schemaVersion, appId, environmentId, revision, environmentType.
No credentials, backend URLs, arbitrary code or instructions in response.
revision must change on every relevant deploy/config/data-schema change; capture before
and after run. Reject oversized/unknown schema, missing/mismatched app/environment,
cache-stale response or mid-run revision change. Prefer immutable deploy URL/provider
build metadata where independently available.

Marker is target-reported identity, not proof of authorization, test-data separation,
backend permissions or cryptographic source attestation. Store identity trust/source
explicitly; results say “tested revision reported by this environment”. Git source alone
does not bind a hosted preview. If identity cannot be established, no current-build pass.

## Typed adapter v1

Local configuration is owner-reviewed, not executable and not imported from page instructions.
Exact origins and these named routes are covered by per-run permission:

- createViaUi: approved route/form/fields, test fixture data; capture returned fixture ID
  only from named creation response schema or named fixture locator. No arbitrary script.
- readBack: GET exact path template with validated fixture ID slot, staff role context,
  bounded allowed response fields and expected values tied to created fixture.
- denyOtherRole: GET same named resource with separate customer test context; expected
  denied status (403/404 as configured) and no selected sensitive fields/body disclosure.
- cleanup: optional supported UI cancel/remove action on recorded fixture ID/tag only.
  If no safe cleanup configured, show created fixture and manual cleanup instructions.

No general URL fetch, arbitrary JSONPath/regex/eval, raw auth field, request header dump,
unbounded response body or API mutation command. Routes/defaults verified with controlled
fixture and at least one independent owner-prepared app, not only Ownvero's happy-path demo.
Unexpected redirect/origin/schema/missing fixture ID blocks the scenario.
Adapters include schema version, digest, declared effects and expected role; changing
adapter invalidates consent and evidence freshness.

## Acceptance tests

Missing/stale marker; same revision with explicit configuration mismatch; unknown schema;
UI success but read-back missing; cross-account data exposed; second role unavailable;
cleanup failure; no stable locator; unauthorized origin; fixture ID injection; imported
adapter attempting to broaden network scope; source/preview revision disagreement.

P01 validates feasibility, P03 owns typed config/schema, P04 setup brief, P06 templates,
P07 execution/adapter tests, P11 documents supported vs observation-only modes.
