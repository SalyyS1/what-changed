# Manual evidence backend

Status: DONE_WITH_CONCERNS

## Delivered

- New typed contract in `src-tauri/src/dto/owner_evidence_dto.rs`; serde unknown-field rejection and ts-rs DTOs.
- Frozen manual manifest includes full sanitized change/version, selected candidate, exact environment revision, confirmed rules and scenario/assertion/step definitions, fixture digest and semantics version.
- Manual start/result/finish APIs use existing bounded writer, admission epoch, transactional app checks and sanitized operation idempotency.
- Results immutable; retry creates a separate run with original manifest. Missing assertions become not_run. A claimed pass without its observed action step becomes not_run. Failed dominates mixed summaries.
- Observer, observation time, actual named steps and observed assertion summaries retained; method is always owner_manual.
- Read/export/accept projections recompute live freshness for change, candidate, rule, scenario/locator/fixture, environment/revocation, capability and semantics changes.
- Manual deployment identity remains unknown. Ordinary acceptance is unavailable. Completed, nonstale runs permit explicit exception decisions with reason, unknown-build acknowledgment and each failed/blocked/not-run scenario acknowledged. Earlier failures remain unchanged.
- Existing SQLite tables reused; no migrations or lifecycle files edited.

## Public APIs

All methods on `OwnerStore`:

- `start_manual_run(StartOwnerManualRunInput) -> AppResult<OwnerManualRunDto>`
- `record_manual_result(RecordOwnerManualResultInput) -> AppResult<OwnerManualResultDto>`
- `finish_manual_run(FinishOwnerManualRunInput) -> AppResult<OwnerManualRunDto>`
- `get_manual_run(app_id,run_id) -> AppResult<OwnerEvidenceReviewDto>`
- `export_manual_run_review(app_id,run_id) -> AppResult<OwnerEvidenceReviewDto>`
- `list_manual_runs(app_id,change_id,page) -> AppResult<OwnerPageDto<OwnerEvidenceReviewDto>>`
- `record_owner_decision(RecordOwnerEvidenceDecisionInput) -> AppResult<OwnerEvidenceDecisionDto>`
- `list_owner_decisions(app_id,change_id,page) -> AppResult<OwnerPageDto<OwnerEvidenceDecisionDto>>`

Review exposes run/version, frozen scenarios, original result/status, counts, unresolved scenarios, recomputed freshness, acceptance eligibility and blockers. Decision history retains decision-time evidence_freshness and recomputes current_freshness on read.

## Validation

Owned Rust sources/tests formatted directly with rustfmt. No Cargo process started by this worker; root owns shared compilation.

Temporary-fixture suites: `owner_acceptance`, `owner_evidence_freshness`. Coverage: missing assertions/action observations; no implicit or automated pass; unknown build; exact exception acknowledgments; immutable retry/cancellation history; app isolation/idempotency; secret sanitation; every live revision dependency; read/export/accept recomputation; historical decisions remain visible.

## Remaining

Root owns central module/commands/client/generated bindings and Cargo verification. No runner/marker observation API, artifacts, participant outreach, pilot or real target contact added. Early two-change usability evidence remains an explicit P06/P07 gate.

Unresolved questions: focused Cargo outcomes and central integration.
