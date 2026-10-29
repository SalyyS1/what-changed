# Runner safety and delivery contract

Status: proposed opt-in architecture; G1 required. Browser testing can mutate servers.
This is not a claim to sandbox hostile websites or guarantee production safety.

## Recommended implementation

Rust app owns consent, frozen manifest, limits, child lifecycle and persistence.
It starts a pinned packaged Node runtime with a fixed packaged runner entry and
Playwright + matched Chromium resources; never invokes shell/PATH/npx/user JS.
Version/checksum inventory is part of installer. No user Node/Bun install required.
P01 must prove packaging on a clean non-admin machine before committing to this route.
If it fails, replan explicitly; do not silently use developer-installed Playwright.

Tauri supports bundled external binaries, but this does not itself supply a secure
runner or Node packaging. See [Tauri 2 sidecars](https://v2.tauri.app/develop/sidecar/).
Playwright/browser versions must match and be tracked together.
See [Playwright browsers](https://playwright.dev/docs/browsers).

No direct shell plugin exposed to JS. Backend launches only its packaged allowlisted
executable/script, with fixed cwd in private run root and scrubbed minimal environment.
Use framed JSON over child stdin/stdout; stderr fixed/redacted diagnostics.
One active run globally initially; one run has max two separate test-role contexts.
OS process handle / Windows Job Object owns child tree. Closing app, cancel, timeout,
revocation or purge cancels/drains all owned descendants; no daemon or scheduled task.
No TCP control server or remotely accessible CDP port. Keep browser sandbox enabled.

Windows launch invariant: CreateProcessW suspended, assign child to non-breakaway Job
with JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, then resume. Fail/kill before resume if
assignment fails; no post-spawn PID sweep as containment. Non-inherited control handles,
nested-job compatibility and parent-crash tests mandatory. Confirm pinned Windows
API bindings in cook; see [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
and [process creation flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags).

## Explicit permission sequence

1. User creates app and selects environment; no connection happens during typing or URL preview.
2. Explain network/process/storage changes and exact supported test-only actions.
3. User confirms ownership/authorization, test data isolation and app/API/auth origins.
   A hostname containing “preview” is not proof of a test environment.
4. UI shows frozen scenarios, origins/ports, accounts by role, actions with side effects,
   run limits, artifact capture choices and test fixture cleanup steps.
5. User grants a one-run authorization. General opt-in only enables this prompt,
   not permission for every future URL/scenario.
6. Backend validates consent against environment + manifest digest before launch and
   before each action. Any change requires new consent. Revoke remains available.

Default blocked: production targets, unknown environment, real payments/email/SMS,
deleting customer data, arbitrary API write, project scripts, file uploads/downloads,
new external windows, camera/mic/geolocation/clipboard, personal login/profile import,
cross-origin auth popups, CAPTCHA/MFA automation. Missing support is reported, not bypassed.
Test-only payment/mail provider requires explicit later connector contract; not part of
first booking/CRUD support. A demo cannot quietly mock a real integration and call it verified.

## URL and network guard

- Accept normalized HTTPS exact origins; HTTP only explicit loopback host + port chosen
  for a local test server. No wildcard host/port permission, URL userinfo, embedded tokens,
  file/data/javascript schemes, UNC or automatic local-network discovery.
- Reject metadata/link-local/private targets except explicit local loopback policy.
  Check redirects, popups, frame/resource requests, APIRequestContext and WebSocket
  routes separately; disable service workers. Do not assume one page route covers all.
- Enforce exact app/API/CDN origins and resource method/path allowlists. Unknown target
  aborts and becomes blocked. Treat DNS changes/rebinding as a tested threat; do not
  claim hostname checks prove connection-level isolation.
- Remote support additionally requires an owned loopback-only filtering proxy: validate
  CONNECT/origin, resolve/check all destination IPs per connection, pin the connection
  to checked IP, reject private/link-local/metadata addresses, disable DIRECT fallback.
  Chromium and API checks use this route; disable unsupported WebRTC/QUIC/service-worker/
  background traffic mechanisms using verified supported browser policy. Local target
  exception is exact loopback/port only. Proxy control is private per-run, not a public API.
  Native deny tests include DNS rebinding, IPv6/mapped IPv4, CONNECT parsing, UDP/direct
  sockets, inherited proxy settings and browser background traffic. A request interceptor
  alone is insufficient. If normal non-admin confinement cannot meet supported-route
  contract, remote support remains blocked and architecture must be revisited.
- HTTP method is not a side-effect detector: even GET may mutate; approved test target
  and fixture isolation remain mandatory. Browser interception cannot stop the test
  server from forwarding an action to production or sending an email.
- Browser contexts isolate cookies/storage, not OS privileges or backend data.
  See [Playwright isolation](https://playwright.dev/docs/browser-contexts).
- P01/P07 must validate enforceable network boundary with deny-case fixtures and native
  traffic observation. If the browser can escape required restrictions, restrict the
  supported target to the controlled local fixture until redesigned; report full runner
  acceptance as blocked, never silently “done”.

## Typed recipe, not arbitrary code

Supported recipe v1: navigate approved relative path, choose role, click/choose/fill
an approved role/name/test-id locator, assert visible/text/value/count, named read-back
assertion, named denied-access assertion. Bounded arrays, timeouts, strings and URLs.
No eval, arbitrary JS, filesystem, shell, unbounded regex, executable import, recursive
loops or dynamically discovered network actions. Return one result per assertion.

Non-tech authoring uses purpose-built booking/CRUD templates and a guided locator
picker in a visibly separate test browser. Record mode follows the same permissions.
Record only explicit selected locator/actions; password and secret fields are never
recorded. Arbitrary browser clicks are not assumed safe because they were human-driven.
If safe recorder cannot capture a step, ask for a supported template or label unsupported.

## Authentication and artifacts

User signs into isolated non-persistent test-role contexts visibly; no personal profile,
saved password or storageState export. Chromium temporary profiles/cache may still contain
sensitive auth until cleanup: do not claim memory-only credentials. Per-run roots use
same-user ACL and stop/start reconciliation. Never export cookies/headers or log/put them
in command args. Profile deletion does not promise forensic erasure/pagefile protection.
See [Playwright authentication warning](https://playwright.dev/docs/auth).

Default evidence is sanitized structured observations. Screenshots are sensitive raw
artifacts, opt-in per run AND reviewed per capture before retention; limit to selected
test surface, default retention <=7 days, exclude from portable export unless separately
selected/reviewed. Masking cannot guarantee unknown secrets; never describe them as
credential-free. Canary scans test known leak paths after normal/cancel/crash/purge,
not universal absence of arbitrary PII.
Raw trace, HAR, video, console dump and DOM snapshot are off in release. If later
needed, separate consent/export/redaction/retention policy; not automatic failure capture.
All visible text/URLs treated untrusted; strip credential query strings and paths,
render as text, cap lengths. Page text cannot instruct native actions.

Suggested limits: 10 min/run, 30 sec/action, 50 steps/scenario, 10 scenarios/run,
25 MiB approved artifacts/run, single browser tree. Expiry uses monotonic elapsed
time; wall-clock timestamps only for records. Limits have explicit UI and tests.

## Evidence commit and recovery

Parent creates run ID + manifest digest + random one-run IPC nonce. Reject wrong run,
unknown assertion IDs, duplicate completion, oversize/malformed messages and wrong
protocol version. Child cannot set overall pass or owner acceptance; parent derives
result from expected assertions and actual terminal events.
Missing final acknowledgment → interrupted. Partial results remain partial.
Crash/restart reconciles running rows as interrupted; never resumes a mutation silently.
Retry is explicit and names possible duplicate fixture writes; cleanup only tagged
records created by that run, no blanket database reset.

## Release changes and maintenance

- Separate gate: core runtime stays network-free with runner disabled; authorized mode
  measured against declared connections. Existing Rust dependency gate remains useful
  but cannot prove child browser behavior.
- Runner/Node/Chromium bundle, notices, CVE response, update cadence, storage and Windows
  support matrix become release responsibilities. A stale vulnerable browser blocks release
  until reviewed; do not call “pinned” equivalent to “secure”.
- One installer with runtime resources is the proposed first distribution to avoid
  unverified runtime downloads. Measure compressed/installed size in P01; >500 MiB
  installer or >1.5 GiB installed is a re-estimation/distribution decision, not a reason
  to silently swap to users' personal Edge profile.
- No activation or background update network. Existing updater remains unconfigured
  opt-in scaffold until separately supplied release feed/key custody.
