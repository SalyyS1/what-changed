# Preview runner implementation — 2026-10-28

Status: implementation delivered; deferred acceptance remains open.

## Delivered

- Native frozen consent binds exact manifest, candidate/environment/rule/scenario revisions, routes, synthetic values, mode and screenshot choice. One-use five-minute consent; compare the manifest inside the run-creation transaction. No network during preparation.
- Fixed packaged Node/Chromium paths, strict complete inventory, immutable read handles, safe relative paths, exact versions and hashes, no fallback download/PATH/shell. Missing runtime produces interrupted history and actionable status.
- Windows suspended launch with atomic job-list membership, checked job assignment before resume, kill-on-close/non-breakaway ownership and restricted inherited pipe handles. Minimal environment includes derived Windows SystemDrive; cwd/TEMP/profile are app-owned.
- Parent protocol validates run/nonce/digest/sequence, expected roles, step/assertion membership and one terminal receipt. Parent releases each action after current freshness/admission checks. Partial or unacknowledged results cannot project a completed pass; manual IPC cannot finish/append automated evidence. Separate manual/browser operation kinds also bind cached idempotent responses to their owning source.
- Explicit login continuation, cancellation/revocation, close-to-tray cancellation, shutdown join, sleep/clock-change interruption, startup interrupted-run reconciliation and exclusive temporary-profile recovery. No PID/process-name sweeps.
- Local literal-loopback HTTP only. Exact origin/method/path grants; authenticated owned proxy, checked/pinned connection destination, DNS/address deny logic, bounded bodies, blocked service workers/WebSockets/popups/downloads, browser sandbox enabled. Remote targets remain unavailable.
- Browser UI execution and typed read-back/other-role adapter; bounded polling for async UI/fixture IDs. Real missing persistence returns a failed assertion. Missing schema/role/locator remains blocked. Before/after bounded exact revision marker; unknown identity stays observation-only.
- Locator selection revokes new network requests after login, selects from the loaded surface, excludes secret/file fields and emits only reviewed locator suggestions.
- Screenshot permission is part of frozen scope. Explicit requested surface only; no automatic failure captures. Raw captures remain memory-only before individual retention review; PNG payload <=512 KiB, <=50/run. Reviewed files reopen after restart, can be deleted and stay outside portable memory. Expiry worker runs every minute with lifecycle admission and is joined at shutdown; default expiry is seven days minus two minutes for sweep allowance. Startup/access also expire artifacts. Filesystem failure, sleep/power-off and forensic-erasure limitations remain.
- Reviewed runnable multi-file ZIP: pinned Playwright setup, current interpreter/proxy/policy, frozen manifest/routes, synthetic values, fixture source, instructions. Native save revalidates digest after dialog, grants no arbitrary destination IPC, and reuses guarded atomic replacement. No cookies, credentials or session consent exported.

## Verification

Commands run in the existing target directory; no second runtime copy made.

- `node --test runner/tests/*.test.mjs`: final frozen-source rerun **15 passed**, 30.296 s, exit 0. Framing, DNS/address denial, exact grants, authenticated proxy, HTTP CONNECT path/method/destination denial, authority release/EOF, capture consent, recipe denial, marker identity and fixture persistence.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --test preview_runner_lifecycle`: **3 passed**, ~0.16 s after compilation. One-use exact scope, revoked environment, missing bundle cannot pass, automated run manual finish denial, deterministic executable ZIP.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib preview_runner:: -- --test-threads=1`: **5 passed, 1 explicitly ignored**. Protocol forgery/missing/duplicate completion and captures retained/reloaded/expired/deleted. Ignored inventory test requires the actual bundle path; no skipped test counted as packaging proof.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib owner_workspace::storage::evidence::run_source_tests:: -- --test-threads=1`: **2 passed**, 0.09 s after 57.66 s compilation, exit 0. Cached manual start/result/finish cannot satisfy browser operations; cached browser responses cannot satisfy manual operations. Exact retries from the owning source remain idempotent; stored review keeps one unchanged result and its source. An initial test-only compile error used the wrong review DTO field; corrected to `review.scenarios[0].result` before this passing run.
- `cargo check --locked --manifest-path src-tauri/Cargo.toml --lib --features devtools`: passed before final narrow lifecycle additions. Focused native tests compile those additions.
- `$env:OWNVERO_TEST_BUNDLE='D:/Project/.WhatChage/runner/bundle'; cargo test --locked --manifest-path src-tauri/Cargo.toml --lib preview_runner::bundle::tests::packaged_inventory_matches_native_loader -- --ignored --exact`: **1 passed**, 23.25 s. Native production loader hashed/opened the actual packaged inventory: 433 files / 550,836,112 bytes, inventory SHA256 `ec77cefc82ae2b433b188391c36452a4302c049f6778228e53869527b3c66b1b`. This closes the explicitly opt-in loader check for that bundle.
- `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib dto::bindings::tests::generated_declarations_match_the_checked_in_frontend_contract -- --exact`: **1 passed**, 0.01 s after 59.43 s compilation. Current Rust `generate_full()` exactly matches checked-in generated TypeScript. Earlier generation passed; final example regeneration exhausted disk archiving the staticlib twice. No bindings were hand-edited.
- `pwsh -NoProfile -File runner/spikes/run-source-recipe-proof.ps1`: real packaged Node + Chromium against current source synthetic booking fixture. Success `[passed, passed, passed]`; lost persistence `[passed, failed, passed]`; other-role exposure `[passed, passed, failed]`. Stable reported revision `fixture-v1`; owned process descendants drained. Evidence: `.qa/ownvero-source-proof-518ccab071e5485cbebefb5e86acc902/source-recipe-proof.json`. This proves interpreter execution; it does not prove installed native IPC or network confinement.

## Repairs driven by failures

- IPv6 mapped subnet rule in Node BlockList also matched IPv4 lookups: split family filters.
- Real browser proof found APIRequestContext uses CONNECT for HTTP: terminate approved local CONNECT into the filtering HTTP parser, keeping exact route/method enforcement. Added deny regression test.
- Async UI completion exposed immediate visibility/empty fixture-ID races: bounded polling; no fabricated pass.
- Native test setup initially claimed configured environment-reported trust: corrected fixture to self-reported configuration. Production trust guard preserved.
- Shared manual/browser idempotency operation names could replay a cached response before the source-ownership checks ran. Distinct start/result/finish operation kinds now reject cross-source reuse while preserving exact same-source retries. Focused regression coverage exercises both directions through real owner persistence.
- Combined lib/integration compilation exhausted disk archiving staticlib. Focused lib-only verification passed. No denied deletion workaround performed.
- Packaging found Chromium-created font cache paths when SystemDrive missing. Native/proof environments now set the Windows drive explicitly; prior files preserved as evidence by controller.

## Deferred / limitations

- Clean non-admin Windows installer without developer dependencies; native process crash/breakaway races and resource budgets; network traffic/UDP/DIRECT/DNS-rebinding confinement; one independent owner-prepared app and real-owner rehearsal remain open.
- Source proof uses spike owned launcher, not the current native command supervisor. Current native supervisor compiles, protocol/lifecycle persistence tested, full end-to-end native execution still needs validation.
- Node/Playwright/Chromium notice/CVE/release review remains required. Existing notice approval does not cover runner deltas. No public publication approved.
- Remote previews deliberately rejected. Local browser restrictions are implemented; native confinement is unverified and no sandbox guarantee is made.
- Expired screenshots are inaccessible immediately on access and periodically removed while the app can run. Power-off/sleep and filesystem errors prevent a wall-clock forensic deletion guarantee; cleanup errors are surfaced.
- Some authority state-machine modules remain above 200 lines because transition invariants belong together. Consent preparation and session controls split from coordinator; protocol tests separate.

Unresolved questions: none requiring a new product decision. Acceptance evidence listed above remains required.
