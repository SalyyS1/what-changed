# Ownvero integration code review

## Scope

- Read-only review of the delegated runner, acceptance, graph/map, export, release-guard and site surfaces.
- Specification: active Ownvero plan, runner safety contract, domain/evidence contract, feature-first sequencing approval.
- Deferred clean-Windows, network-boundary, native performance and real-owner proof were not treated as missing implementation.
- Existing reported checks were reused; no broad tests or builds run because other workers own native/full-suite verification.

## Overall assessment

Two P1 integration defects block calling the preview-runner feature production-ready. One P2 defect forces an owner to make a false freshness acknowledgment. Core evidence derivation correctly keeps automated/manual method, result, lifecycle, freshness and owner decision separate. The reviewed graph projection feeds list, 2D and 3D from the same backend-derived nodes and preserves full totals; no concrete equivalence defect found.

## P1 findings

### 1. Navigation revocation can fail silently after UI ownership is discarded

- Evidence: `src/lib/stores/preview-runner.svelte.ts:55-56` cancels a grant that completes after generation change, but a cancellation error is caught by the outer block and suppressed because the generation is stale. `src/lib/stores/preview-runner.svelte.ts:125-130` fires cancellation during `dispose()`, catches and discards every failure, then clears the only UI session handle. `src/features/verification-runner/RunnerStatus.svelte:13` promises that leaving the workspace revokes the session.
- Impact: a transient invoke/cancel failure can leave the native browser run authorized and active after navigation, while the UI has forgotten its app/run IDs. The run can continue until its own deadline and a later run is merely blocked by the hidden active session. This violates the revocation contract.
- Fix: keep active-run ownership in an app-level/global controller until native status is terminal. Route teardown should confirm cancellation or preserve a visible recoverable revoke control. Late-grant cleanup must not suppress cancellation failure; native authority should be tied to a synchronously revocable lease/generation rather than component lifetime alone. Add failure-path tests for both late grant and disposal cancellation.

### 2. Packaged runner produced by the packaging script cannot pass native inventory loading

- Evidence: `scripts/package-preview-runner.ps1:38` rebuilds `bundle-inventory.json` without `checksumSource`. `scripts/check-runner-bundle.mjs:34` validates versions but does not require that field, so the staged bundle can pass the release check. `src-tauri/src/preview_runner/bundle.rs:12,25-27` requires a non-optional `checksum_source` and rejects anything other than the exact Node checksum URL.
- Impact: the official packaging/check path can succeed, yet every resulting installed preview runner fails at `Bundle::open` before Node/Chromium launch. The runnable feature is therefore unavailable in the packaged candidate despite a green bundle gate.
- Fix: preserve and validate `checksumSource` in the PowerShell-generated inventory and JavaScript checker, then add a contract test that feeds a package-script-shaped inventory through the Rust loader. If checksum provenance is intentionally removed, remove the Rust requirement and the corresponding provenance claim together.

## P2 findings

### 3. Current automated evidence requires a false “unknown build” acknowledgment for acceptance with exceptions

- Evidence: `src/features/acceptance/OwnerDecisionDialog.svelte:14` requires `unknownBuild` for every `accepted_with_exceptions` choice; lines 24 and 29 state freshness is unknown. A completed automated run can be `current` while containing a failed or blocked scenario. The backend correctly requires the unknown-build acknowledgment only when freshness is unknown at `src-tauri/src/owner_workspace/storage/evidence/decision.rs:51-54`.
- Impact: the UI prevents a truthful accept-with-exceptions flow for current automated evidence unless the owner checks a statement contradicted by the evidence record.
- Fix: condition the text, checkbox and `canRecord` requirement on `review.freshness === 'unknown'`. For current evidence, require only the exact unresolved-scenario acknowledgments. Add a UI test for current automated evidence with one failed scenario.

## Edge-case and contract checks

- Freshness is recomputed transactionally and environment revocation, scenario/rule revisions, candidate changes and runner semantics make evidence stale.
- Parent protocol rejects unknown/duplicate assertions and omitted assertion completion; interrupted automated runs cannot project a completed pass.
- Screenshot retention is explicit; retained artifacts are excluded from portable export and included in purge/restore inventory. Expiry code was actively changing during review, so no stale finding is recorded against that area.
- Runnable ZIP contains executable harness/source and fails independently when revision identity is unknown; no concrete fake-export defect found beyond the packaged native inventory mismatch above.
- Release/site guards fail closed on publication approval, artifact hashes, allowlisted output and download-manifest state in the inspected source.
- No broad verification claim: full frontend and clean-Windows/native acceptance remain outside this pass.

## Scoped simplification advice

- Keep one active preview-session owner above the route/component layer. This removes duplicated generation/disposal cleanup and makes revocation state observable without changing behavior.
- Share one typed runner-inventory schema/validator across packaging, JavaScript release checks and Rust loading (or generate fixtures from one schema). The current parallel field lists caused finding 2.

## Recommended order

1. Repair package inventory parity and test package-to-native loading.
2. Make revocation ownership durable across navigation and test cancellation failures.
3. Correct freshness-conditional acceptance copy and gating.
4. Re-run focused runner-store, acceptance-dialog, bundle-check and Rust inventory tests; then resume the already-deferred platform proofs.

## Unresolved questions

- None requiring product direction. The three findings are implementation-contract mismatches.

Status: DONE_WITH_CONCERNS
Summary: Two P1 blockers and one P2 acceptance defect verified from current source; deferred external proof was not misclassified as missing feature code.
Concerns/Blockers: Silent revocation failure and package/native inventory mismatch.

---

## Bounded repair re-review — 2026-10-28

Scope was limited to the three findings above and their focused repair evidence. No Cargo, build, server or test command was run in this pass.

### Finding 1 — resolved: durable revocation ownership

- `src/lib/stores/preview-runner.svelte.ts:61-62,89-95,133-138` now hands late grants, manual cancellation and route disposal to a revocation owner instead of swallowing cancellation and losing the session identity.
- `src/lib/stores/preview-revocations.svelte.ts:48-105` retains the exact app/run pair, serializes attempts, checks native status identity, removes the handle only after a matching terminal state, and retries with bounded backoff while the root is attached.
- `src/app/App.svelte:64-68,114-127` attaches that owner above routed screens and renders persistent recovery controls. `PreviewRevocations.svelte:7-16` keeps exact identities, failure state and manual retry visible across navigation.
- Focused evidence in `plans/reports/fix-261028-preview-revocation.md` records 29 passing tests plus typecheck exit 0. Source tests cover failed cancellation after disposal, late-grant failure across a newer generation, identity mismatch, deduplication, remount and route persistence.
- Verdict: original P1 revocation defect resolved. No remaining concrete defect found in the bounded repair.

### Finding 2 — resolved at source and packaged-artifact contract

- `scripts/package-preview-runner.ps1:33-34,73-75` now validates and preserves the exact `checksumSource` required by the Rust loader.
- `scripts/check-runner-bundle.mjs:7,34-36` now requires the complete closed inventory schema and matches the same checksum source.
- The current `runner/bundle/bundle-inventory.json` has all eight expected fields, 433 files and SHA-256 `ec77cefc82ae2b433b188391c36452a4302c049f6778228e53869527b3c66b1b`, matching `plans/reports/ownvero-runner-package-resumed.log`.
- Verdict: original P1 package/native manifest mismatch resolved. Native parity execution remains owned by the separate active worker; this review does not pre-claim its result or the deferred installed-runtime proof.

### Finding 3 — resolved: freshness-conditional acknowledgment

- `src/features/acceptance/OwnerDecisionDialog.svelte:14-19,25-31` derives the unknown-build requirement from `review.freshness === 'unknown'`, omits the false statement for current evidence and submits `acknowledge_unknown_build: false` for the current-build exception path.
- `src/features/acceptance/run-review.test.ts:35-47` covers current automated failed evidence and verifies exact unresolved acknowledgment without the unknown-build checkbox.
- Verdict: original P2 acceptance defect resolved. No remaining concrete defect found in the bounded repair.

### Re-review outcome

All three original findings are repaired in the inspected source and focused evidence. External clean-Windows, network-boundary, performance and real-owner acceptance remain deliberately deferred; G2/G3 publication approval remains pending and is not a regression.

Status: DONE
Summary: Three original findings resolved; no remaining concrete defect in the bounded repair scope.
Concerns/Blockers: None in this repair scope. Native parity worker result and deferred external acceptance remain separate gates.

---

## Native idempotency follow-up — 2026-10-28

### Source diagnosis

Before this repair, manual and automated lifecycle operations used the same idempotency `kind`. `OwnerStore::mutate_at` returns a cached response before executing the operation body when both `kind` and sanitized request digest match (`src-tauri/src/owner_workspace/storage/repository.rs:226-239`). Because the manual and browser DTO inputs are structurally identical, a cached manual start/result/finish response could therefore satisfy the corresponding automated call and bypass its method-specific checks and output derivation.

### Repair verdict

- `src-tauri/src/owner_workspace/storage/evidence/run.rs:61` now uses `start_browser_run` versus `start_manual_run`.
- `run.rs:116` now uses `record_browser_result` versus `record_manual_result`.
- `run.rs:166-169` now uses `finish_browser_run` versus `finish_manual_run`.
- Cross-source reuse of one operation ID now fails the repository's `prior_kind != kind` guard. Same-source replay keeps the same kind and request digest, so legitimate idempotent retries still return the original cached response.
- Existing stored automated operations from the old shared kinds will fail closed if replayed after upgrade. Browser runs are not resumed after process restart, so this does not create a supported retry regression.

The implementation repair is correct in the inspected source. No further cached manual-to-automated response path was found across start, result or finish.

### Remaining concrete test defect

- `src-tauri/src/owner_workspace/storage/evidence/run_source_tests.rs:86` currently asserts `review.results`, but `OwnerEvidenceReviewDto` has no `results` field (`src-tauri/src/dto/owner_evidence_dto.rs:192-204`). Results are exposed through `review.scenarios[*].result`.
- Impact: the new regression module cannot compile in its current form, so it does not yet prove the otherwise-correct repair.
- Fix: assert the matching scenario projection contains `Some(observation)` (or compare the collected scenario results), then run the focused library regression. The backend worker owns this pending correction/evidence.

Native loader 1/1, source-versus-generated binding 1/1 and Node 15/15 results were reported by the controller; this read-only follow-up did not rerun them. Deferred external limits and G2/G3 remain unchanged.

Status: DONE_WITH_CONCERNS
Summary: Method-specific idempotency kinds correctly prevent cross-source cached responses while preserving same-source retries.
Concerns/Blockers: New regression test currently references a nonexistent review field and needs correction before its result is usable.

### Controller closure of the validation concern

The backend owner corrected the test to the scenario result field and ran
`cargo test --locked --manifest-path src-tauri/Cargo.toml --lib owner_workspace::storage::evidence::run_source_tests:: -- --test-threads=1`.
Result: 2/2 passed, exit 0, 0.09 s after 57.66 s compilation. Both cache-replay
directions reject manual/browser source mismatch; owning-source retries preserve
the original stored observation. Scoped rustfmt/diff checks passed. The final
[runner report](implementation-261028-preview-runner.md) records the evidence.
This closes the test-field concern above; the source-repair verdict is unchanged.
External acceptance and publication gates remain open.
