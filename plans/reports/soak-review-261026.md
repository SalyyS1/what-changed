# Soak Hardening Code Review

## Scope

- Current pending soak diff only: `src-tauri/src/soak.rs`, `src-tauri/src/soak/*.rs`, `src-tauri/src/soak/script-contract.tests.ps1`, `scripts/soak.ps1`, runtime observer/join changes, and `src-tauri/tests/soak_observer.rs`.
- Acceptance focus: isolated QA root, canonical path containment, full production collector readiness, sticky outcome/loss evidence, process/report identity, clean exit and shutdown joins, and backend-only scope.
- No implementation edits. No Cargo command run by reviewer.

## Overall Assessment

GO for the inspected backend soak-hardening source. Both review findings are resolved, and the production readiness, sticky failure/loss, identity, shutdown, and backend-only gates match the requested contract. This is not evidence of a completed native four-hour Tauri/WebView acceptance run.

## Resolved Findings

1. **Resolved: equivalent Windows paths could select one physical file as both trigger and acknowledgement.**
   - Rust now compares the validated direct-child filenames case-insensitively at `src-tauri/src/soak/paths.rs:163-174`, covering separator, case, and current-directory aliases.
   - PowerShell now applies `GetFullPath` to both protocol paths, returns those normalized paths, and normalizes every output-collision comparison.
   - Rust tests cover mixed separators/case and `.` components; the PowerShell contract covers mixed separators/case for protocol and output collisions.

2. **Resolved: atomic observer tests did not reach the acknowledgement hook.**
   - The fixture now commits a real `RawBaseline`, marks hook entry for both hook modes, and checks the baseline is durable while the success hook remains blocked.
   - `HookFailure` explicitly asserts hook entry, preventing the previous empty-commit validation error from masquerading as the intended hook failure.
   - Parent-provided focused result: `soak_observer` 5/5 PASS.

## Verified Contracts

- Exclusive root creation uses `create_dir`; existing roots are rejected.
- Existing ancestors are checked for reparse points and pinned without delete sharing; normal app data is compared through nearest-existing-ancestor canonicalization.
- Readiness uses `production_registry`, enables the full denominator, and rejects unhealthy/unavailable rows. Docker absent maps to `Unavailable`, so it cannot satisfy readiness.
- Outcome observations occur after successful worker outcomes; failure observations and monitoring pause are sticky. Writer loss aggregates dropped writes, failed rows, rejected cycles, and failed cycles.
- Ready/report schema, run ID, PID, requested duration, collector denominator, successful snapshots, zero writer loss, successful process exit, and completed shutdown are validated.
- Observer panics are contained outside the status-sink mutex. The soak evidence sink owns counters only, so it introduces no strong ownership cycle.
- Join timeout or panic sets a sticky `shutdown_incomplete` latch, which prevents a pass.
- Script output explicitly labels the diagnostic `backend-only` and excludes native WebView acceptance.

## Checks

- `src-tauri/src/soak/script-contract.tests.ps1`: PASS, including normalized protocol/output collision cases.
- Parent-provided Rust evidence: soak observer 5/5 PASS; full library 571 PASS / 3 ignored; strict Clippy all targets PASS before the final narrow path comparison adjustment. Final focused soak/Clippy/headless-smoke verification was running at report update time.
- Reviewer `git diff --check` over the final repaired files: PASS.
- Independent failure forecast found no additional lifecycle/report defect after the repaired findings; its sole path-alias concern was the `.`-component case now resolved by filename comparison and regression coverage.
- Live four-hour production soak: not run; no acceptance claim made.

## Informational Gaps

- No empirical Windows 8.3 alias regression test proves normal-profile exclusion, although the canonical nearest-existing-ancestor implementation is aligned with the requirement.
- Existing general `collector_changed`/`monitoring_changed` callbacks are outside this new observer contract and are not panic-contained. The soak sink implementations are non-panicking, so no soak-path defect was established.

## Recommended Actions

1. Record the terminal result of the final focused soak/Clippy/headless-smoke run.
2. Add an empirical 8.3 regression where the volume supports short names.
3. Keep backend diagnostic evidence separate from native Tauri/WebView acceptance.

## Unresolved Questions

- None requiring product input.

Status: DONE  
Summary: Both evidenced findings are repaired and covered; no further source blocker found in the reviewed scope.  
Concerns/Blockers: Native four-hour Tauri/WebView acceptance remains a separate evidence gate, not part of this backend review.

## Standalone Native Build Route Review

### Scope

- `scripts/build-standalone.ps1`
- `package.json` `build:standalone`
- `CONTRIBUTING.md` standalone diagnostics section
- Read-only review; no Cargo command or process mutation by reviewer.

### Assessment

GO for the standalone build route after the authoritative cached wrapper rerun completes successfully. PowerShell 5.1 parsing, immediate native exit checks, feature scoping, relative `CARGO_TARGET_DIR` resolution through Cargo metadata, target-tree and exact-file running guards, cached-build handling, SHA-256 output, and the non-publication documentation are sound.

The first authoritative build completed Cargo successfully, then the wrapper failed because `Get-FileHash` was unavailable in the Bun-launched nested Windows PowerShell 5.1 session. This was a post-build wrapper failure, not a Cargo or application failure. The script now computes SHA-256 through .NET with explicitly disposed `SHA256` and file-stream objects, avoiding module auto-loading.

The configured-target artifact-selection concern is resolved. The script consumes Cargo `compiler-artifact` JSON, requires exactly one current-package `what-changed` binary executable, rejects relative/outside/missing/duplicate artifacts, and verifies the selected path is within Cargo's reported target directory. It no longer assumes the host `release/` layout.

Delegated Windows PowerShell 5.1 validation: AST PASS and 14 fixture/live assertions PASS. Tests covered a real stale host file plus target-triple artifact, unrelated package/non-binary exclusion, missing/duplicate/outside/relative rejection, directory-boundary/case behavior, the known SHA-256 digest for `abc` with module auto-loading disabled, and non-destructive target-tree/exact running guards.

Runtime acceptance still requires Vite port 5183 absent, a non-error embedded page URL, rendered DOM, and a successful real `get_app_status` bridge call.

The authoritative corrected wrapper rerun exited zero after a 10m14s Cargo release build. It selected `.qa/collector-performance-release-20261015/release/what-changed.exe` and printed SHA-256 `4dab1f31bac288e308d6801d262eeb899da31372345789fe375a422c741308c8`. This identifies that build output only; it is not claimed byte-equivalent to the earlier native-QA executable.

Status: DONE  
Summary: Standalone route is target-aware and PowerShell 5.1-safe by source review, focused fixtures, and an authoritative zero-exit wrapper run.  
Concerns/Blockers: None for the internal standalone build route; installer/publication and native four-hour acceptance remain separate gates.

## Process Collector Paired-Cost Diagnostic Review

### Scope

- `src-tauri/tests/collector_performance.rs`
- `src-tauri/tests/support/process_cost_diagnostic.rs`
- Contract source: `plans/reports/process-cost-scout-261026.md`
- Test-only, read-only review. No Cargo, test, build, or process operation by reviewer.

### Assessment

GO. The diagnostic is explicitly ignored and labels itself observation-only/nonacceptance. The established 50-cycle, 15 ms process and listener acceptance bodies, thresholds, and full timed regions remain unchanged from the worker's entry revision. Existing poison recovery is retained by verified prior decision so a process assertion cannot mask the independent listener result.

`TimedProcessProvider` directly delegates both provider methods and overrides `enumerate_into`, preserving the collector's reused output buffer. Only elapsed nanoseconds and row count cross the seam through same-thread atomics. Every one of 50 samples retains a paired tuple covering full-cycle total through conditional acknowledgement, `black_box`, and destruction; provider time, snapshot remainder, acknowledgement region, destruction, row count, result kind, and observation count stay paired.

The acknowledgement output is correctly named `acknowledgement_region`: it includes result classification and the conditional branch, and explicitly states that Noop/Legacy values are overhead with no acknowledgement hook. Percentiles use nearest-rank positions over the same 50 tuples; stage percentiles are not added or subtracted. Slowest-cycle rows preserve the paired context, and `kind_counts` disclose the population behind pooled stage summaries.

Output contains numeric counts/timings and fixed labels only—no process IDs, names, paths, identities, or row contents. Formatting, sorting, percentile allocation, atomic reads/resets, and sample appends occur outside the full timer. Instrumentation overhead and the inability to separate private collector stages are disclosed.

Checks reported by the worker: Rust formatting PASS and `git diff --check` PASS. No Cargo/test execution was authorized for this source-only review.

Status: DONE  
Summary: Diagnostic matches the paired, privacy-safe, nonacceptance measurement contract with no statistical or instrumentation blocker found.  
Concerns/Blockers: Live results remain diagnostic evidence only and must not replace or relax the original 15 ms acceptance gate.
