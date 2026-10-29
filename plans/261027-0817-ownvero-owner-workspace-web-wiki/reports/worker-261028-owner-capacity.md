# Owner storage capacity and backpressure

Status: DONE — controller integration target passed all three tests.

## Scope

New integration target `src-tauri/tests/owner_workspace_capacity.rs`, with fixture
and backpressure helpers under `tests/owner_workspace_capacity/`. No core storage
or domain changes. All records pass real public CRUD and recipe validation.
Only UUID-named temporary fixtures are created; stores shut down before cleanup.
No personal app-data, profile, live backup/restore/purge or large blobs used.

## Acceptance exercised

- Twenty apps; each has one capability, one confirmed rule and five confirmed
  scenarios: **100 scenarios total**. The twenty-first app fails and the stored
  app count remains twenty.
- Each selected app is paged at limit two over its five scenarios. Three pages
  must return exact sorted expected IDs, monotonically advancing revision
  cursors, no duplicates and no records belonging to another app. Three rounds
  produce **180 measured page queries**. Test prints actual p50/p95/max in ms;
  no arbitrary performance threshold or fabricated measurement.
- One real writer transaction is held after querying the fixture app count.
  Seventeen unique capability mutations race for the documented sixteen queued
  slots. The first completed request must fail while the head is held. After
  release/join: exactly sixteen succeed and one returns typed `QueueFull`.
  All sixteen rows persist; rejected ID is absent. Retrying the exact rejected
  input succeeds; repeating it is idempotent, leaving seventeen rows.
- Two real pooled readers acknowledge acquisition after a SQL query. A third
  selected-app query returns typed `NotReady` / `every reader is busy` after
  the configured five-second pool wait. Holders release and join before result
  assertions; the same selected-app query then succeeds.

Channels establish acquisition, saturation and release order. No sleeps, fake
timing, ignored tests or production hooks. Thirty-second receive deadlines are
deadlock guards; held callbacks also have bounded deadlines and RAII release/join
on assertion failure. Temporary cleanup verifies the exact generated root under
the OS temporary directory before recursive removal.

## Verification

Direct rustfmt: passed. Initial child-module resolution diagnostic fixed with
an explicit module path; final rustfmt exit zero.

Controller ran the target in a serial Cargo batch to keep disk use bounded:

```powershell
cargo test --offline --manifest-path src-tauri/Cargo.toml --test owner_workspace_capacity -- --nocapture --test-threads=1
```

Evidence: `.qa/ownvero-preflight/offline-portability-capacity.txt`, target
`owner_workspace_capacity`: **3 passed; 0 failed; 0 ignored**, 5.06 s. Same batch:
freshness 4, portability 8, privacy 4 and owner storage 7 passed.

Measured 180 selected-app queries: **p50 0.065 ms, p95 0.102 ms, max 0.269 ms**.
Third-reader saturation returned the expected error in **5007.591 ms**, followed
by successful recovery. Writer evidence: one active transaction, sixteen queued
successes, one typed rejection, idempotent retry and seventeen persisted rows.

These fixtures do not claim installed-app latency, clean-machine acceptance or
100 scenarios/app. Measurements describe this source fixture on this host.

## Unresolved questions

None.
