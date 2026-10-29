# Reviewed preview test recipe

This is an independent executable test artifact. Review every file, exact test
origin, route, fixture value, and action before running. It has no Ownvero consent,
saved session, credentials, owner decision, or production permission.

## Setup and run

Use Node 24.12.x. In this extracted directory run:

```powershell
npm install --ignore-scripts
node node_modules/playwright-core/cli.js install chromium
node src/run.mjs --authorize-isolated-test
```

Setup downloads pinned Playwright 1.62.1 and its matched Chromium 1234. Test runtime
accepts literal loopback HTTP origins with explicit ports only. Review manifest.json
and fixture.json first. Close every test window when finished. Login visibly using
separate, disposable test identities in the two isolated windows, then confirm in
the terminal. Do not use daily accounts, real customer data, payments or mail.

The test prints one sanitized status per named assertion. Exit 0 requires all
expected assertions passed and the same target-reported revision before and after.
That marker is not source attestation. Missing marker, blocked request, missing role,
regression, cancellation or changed revision returns unsuccessful; partial output
is never a completed verification. Results do not change Ownvero history.

The test applies the same bounded route proxy and browser restrictions as the app.
Browser contexts isolate cookies, not OS or server privileges. Native traffic and
crash cleanup acceptance is still open. Temporary profile deletion does not promise
forensic erasure. Use Ctrl+C to cancel; after a crash inspect only the task's own
temporary profile directory before cleanup.

## Synthetic booking fixture

Optional fixture: no external payment/mail integrations are implemented or claimed.
Start in another terminal, with a NEW isolated folder and the app/environment UUIDs
from manifest.json (change.app_id and environment.id):

```powershell
node fixtures/booking-app/server.mjs ./isolated-test-data APP_UUID ENVIRONMENT_UUID
```

It serves http://127.0.0.1:5185 and revision fixture-v1. Prepare that exact environment
and revision in Ownvero before exporting. The synthetic UI provides Staff and Customer
sign-in links. Recipes use Booking note, Test run tag, Save booking, and the
ownvero-fixture-id test id. Tags must begin ownvero-. Grant the fixture's exact
routes: GET /, /signin/staff, /signin/customer, /api/session,
/.well-known/ownvero.json, /api/bookings/{fixture_id}; POST /api/bookings and
/api/bookings/{fixture_id}/cancel. No wildcard route is needed.

Bookings persist in isolated-test-data/bookings.json. Cancellation changes only the
created record after its matching test tag is checked. Inspect remaining tagged
records if a run stopped before cleanup. Do not reset an unrelated database.
