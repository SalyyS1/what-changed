# Owner domain and brief backend review

Status: DONE_WITH_CONCERNS

## Delivered

- Separate owner SQLite v1 schema and checksum ledger; unavailable corrupt/newer/drift files preserved.
- Bounded writer queue16 and two read-only connections; optimistic versions, app-scoped references, sanitized idempotent operation requests.
- App/capability CRUD; immutable rule/environment/candidate/scenario revisions; change draft storage and review snapshot.
- Durable admission epochs; closed reset, drained handles and exact owned DB/sidecar/pre-migration-backup inventory.
- Deterministic local brief from one read snapshot. Typed scenario criteria preserved; output above32KiB returns an error, never partial rules.
- Preview includes app_id/change_id/change_version, byte/token estimate, omissions and limitations.
- Free text URLs/absolute local paths replaced with visible placeholders; surrounding safety text retained. Only explicitly selected typed environment origin can appear. Backticks/HTML/Markdown image/link markers escaped.

## Validation

Direct rustfmt/check passed for owned sources/tests. Root owns Cargo invocation; root cargo-check diagnostics for SQLite integer conversions, candidate inference, helper visibility and brief Vec<String> were repaired. No independent Cargo command launched.

Fixture suites: owner_workspace_storage; owner_workspace_privacy; owner_brief. Tests use fresh temporary paths only, including real sanitizer canaries, restart, immutable history, cross-app references, retry conflicts, checksum/newer/corruption preservation, limits/pages, epoch replay and exact purge inventory. Brief fixtures cover latest/prior confirmed scenarios, exact typed assertions, Markdown/path/URL injection, explicit environment inclusion, metadata version and32KiB rejection.

## Concrete source review findings

1. **Reviewed non-issue for current IPC:** immutable SQL triggers reject UPDATE, but no DELETE trigger exists. Root retained this invariant because no public domain repository/IPC exposes history deletion; full purge deliberately removes owned files. Low-level backend helpers remain trusted implementation surface.
2. **Fixed:** private remote origin validation normalizes IPv4-mapped IPv6 before loopback/private checks. Negative fixtures supply matching typed origins so rejection cannot pass merely because preview origin differs.
3. **Accepted explicit bound:** rule capacity counts500 distinct rule IDs including historical/archived-capability rules. Root retained the current500-total bound rather than silently introducing archive semantics; this is stricter than the proposed active-rule budget.
4. **Fixed:** brief initially showed assertion labels without their typed exact criteria. Renderer now includes the original TextEquals/FieldEquals/Denied/Visible criterion and named read-back adapter role/path/fields.
5. **Fixed:** brief initially omitted only dedicated environment fields. It now substitutes URL/path fragments inside all free text and emits disclosure without dropping surrounding rules.
6. **Fixed from root fixture evidence:** the generic sanitizer recognizes `preview_url` as a secret-bearing connection-string key and replaced safe configuration with `[REDACTED]`. Typed environment Scrub now invokes the same sanitizer under `preview_address` after credential-free address validation. Shape/pair redaction still runs; global privacy policy is unchanged; structural validation repeats after sanitization.
7. **Fixed from root fixture evidence:** root-relative Windows paths with one leading separator now receive the same omission treatment as drive/UNC paths.

## API to wire

`OwnerStore::render_brief(&self, app_id:&str, change_id:&str, selection:OwnerBriefSelectionInput) -> AppResult<OwnerBriefPreviewDto>`

Central command/AppState/global purge/DTO generation remain root-owned. Full P05 portable archive import/export and project decision memory remain outstanding. P06 result freshness/manual observation/acceptance semantics remain outstanding; no fake automated pass API is exposed by this delivery.

Unresolved questions: focused Cargo outcomes after final repairs; central integration and remaining full P05/P06 scope.
