# Backend soak diagnostic hardening

Status: verified backend diagnostic hardening; native acceptance remains open.

Scope: devtools backend diagnostic only. No installed-app, WebView, four-hour, or all-capabilities acceptance claim. Docker remains in the enabled production denominator; unavailable Docker fails.

## Confirmed causes

- Old runner created QA root and trigger parents before validating them, reused existing roots/settings, and could follow leaf/ancestor aliases.
- Ten-second warmup checked transient collector states, allowing never-successful or subsequently recovered failures to disappear.
- Script could report pass without matching terminal backend evidence or an observed zero exit.
- Runtime shutdown detached components after its existing two-second deadline without exposing that loss to diagnostics.

## Changes

- `src-tauri/src/soak.rs` now delegates path ownership, health evidence, and run lifecycle to `soak/paths.rs`, `health.rs`, and `runner.rs`.
- QA root must be fresh, absolute, reserved, and below an existing plain local parent. Validate all paths before exclusive root creation. Canonical nearest-existing paths exclude normal app data even when missing or addressed by Windows short names. Windows directory handles reject reparse points and deny ancestor deletion while owned.
- Trigger and ack must be distinct direct-child ordinary `.txt` files. Protocol writes use exclusive temporary creation and atomic non-overwriting hard-link publication. Oversized, unreadable, malformed, or aliased triggers fail.
- Observe real completed snapshot outcomes and sticky failure notifications before workers start. Successful snapshots include Noop and legacy drains; they are not mislabelled as exact durable-commit counts. Failure observations can include startup and asynchronous delivery; they are not exact failed-cycle counts.
- Readiness requires exact full registry, every collector enabled and healthy, at least one successful snapshot each, and a successful writer flush. Readiness timeout is 120 seconds. `--seconds` begins after readiness.
- Final success additionally requires committed simulation input, a new complete requested capture from every worker, no lifetime writer losses, no observed failure/pause, and no incomplete runtime shutdown. `CollectorSupervisor::shutdown_incomplete()` latches any missed or panicked worker/watchdog/dispatcher join; repeated shutdown cannot clear it. Existing shutdown policy remains unchanged.
- `soak-ready.json` and `soak-result.json` bind schema, backend-only mode, UUID, PID, requested duration, and exact collector IDs. Result includes successful snapshots, failure observations, writer loss, trigger count, and actual shutdown completeness.
- PowerShell waits for readiness; retains the process handle; requires matching result plus zero exit; rejects reused output/protocol files, unsafe paths, stale identity, missing/failed reports, incomplete duration/coverage, and lifetime loss. Output states native/WebView acceptance is false.

## Verification

Completed locally:

- `rustfmt --edition 2021` on owned Rust files: success.
- `powershell.exe -NoProfile -NonInteractive -File src-tauri/src/soak/script-contract.tests.ps1`: PASS. Includes missing report/readiness, nonzero exit, stale PID/run ID, incomplete shutdown/duration, zero collector successes, loss, missing denominator, reused ack, traversal, and junction rejection.

First PowerShell check found only a test harness path one directory too high. Kongming confirmed the exact correction before repair; rerun passed.

Parent-executed checks:

```text
cargo test --manifest-path src-tauri/Cargo.toml --features devtools --lib soak::
cargo test --manifest-path src-tauri/Cargo.toml --features devtools --lib collectors::runtime::shutdown_tests
cargo test --manifest-path src-tauri/Cargo.toml --features devtools --test soak_observer
```

Rust negatives cover prevalidation/no directory creation, root reuse preserving existing data, non-overwriting outputs, normal-app exclusion, oversized/non-UTF8 triggers, junction ancestors, missing/duplicate/unhealthy registry rows, no readiness without successful snapshots, and recovered failures. Real supervisor tests cover exact Noop/legacy success observations, observer panic containment, provider recovery, acknowledgement-hook failure, success after durable acknowledgement, direct asynchronous legacy rejection, and sticky blocked-stop shutdown evidence.

Kongming reviewed callback semantics, lock/cycle avoidance, shutdown detach evidence, and canonical path comparisons. No normal application data touched; no Docker installation; no background process started by this worker.

## Open verification

- Soak units 8/8, observer integration 5/5, shutdown join 1/1, full library 571
  passed / 3 intentional ignores, final strict all-target/all-feature Clippy and
  devtools debug build passed. Final alias-focused unit run also passed 8/8.
- Independent review repairs: compare validated direct-child filenames to reject
  separator/case/dot aliases; normalize every PowerShell output collision path.
  Protocol negatives pass. Atomic fixtures now commit a real baseline and assert
  hook entry plus persistence, correcting one failed test and one phantom pass.
  A temporary raw-SQL assertion failed compilation due to the repository's error
  conversion contract; the existing baseline repository reader replaced it.
- Live negative smoke `.qa/what-changed-perf-soak-final-20261026` rejected Docker
  as unhealthy: backend exit 1, failed/not-ready backend-only report, Docker
  retained, zero writer loss, `shutdown_complete=false`. The test of rejection
  passed; this is not a healthy soak or complete-shutdown result.
- Four-hour native/installed/WebView acceptance remains open.

Unresolved questions: none; remaining work is verification.
