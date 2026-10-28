# Portable export/save review

Date: 2026-10-28. Initial scope: root export-save repairs, review cache and lifecycle
test; narrow callees traced for snapshot, archive validation and native rename.
Read-only review; no Cargo, native dialog execution, personal data or network access.
Source-backed findings below precede the separately authorized historical-export repair.

## P1 — Active recipe semantics prevent exporting immutable historical memory

`owner_workspace/portability/records.rs:139` originally called active `validation::recipe` for
every stored scenario, including imported historical records. Both
`storage/portability.rs` snapshot construction and `portability/review.rs` bundle
preparation require that validation to succeed.

A previously accepted FieldEquals-on-Navigate definition, or a historical recipe
above the new 1,000-assertion active limit, now blocks the entire workspace export.
The manual review projector correctly retains the raw observation and exposes a
blocked gap, but the owner cannot actually export that history. This contradicts
P05 immutable-history portability and the P06 repair's preserve/export requirement.

Repair direction: separate safe closed-schema archival structure from active recipe
admission. Historical bytes remain bounded, typed and non-executable; active
confirmation/freeze/retry/result checks remain strict. Do not discard old assertions,
relax unsafe routes/unknown versions/fields, or promote imports into active scenarios.
Controller authorized this narrow follow-up. Implemented archival structural
validation separately from unchanged active semantic admission. Two new integration
tests under `tests/owner_portability/historical_recipe.rs` exercise actual DB snapshot
and bundle export/import/re-export for a missing-read-back and 1,100-assertion recipe;
active run/reconfirmation remain rejected. Negative archival fixtures cover unsafe
routes, unknown schema, invalid/duplicate IDs and step/per-step bounds. Rustfmt and
scoped diff check passed; controller owns pending Cargo verification.

## P2 — Cleanup failure silently leaves temporary export bytes outside app custody

`commands/owner_portability/save.rs:23–30` writes the random sibling temporary file,
then discards `remove_file` errors when writing or promotion fails. The reported
AppError names only the selected destination; the exact remaining temporary filename
is lost when the function returns. This folder is user-selected, not a managed app
root, so later privacy cleanup cannot discover it safely.

Condition requires no attacker: a removable destination disappears after writing, or
an ordinary file scanner opens the temporary file without delete sharing after the
writer closes. Rename and cleanup can both fail, leaving partial/complete reviewed
bytes at `.project-memory-<UUID>.tmp` without telling the owner.

Repair direction: retain explicit custody of that exact temporary path through failure.
Attempt exact-file cleanup, but if it fails return a recoverable warning/error that
identifies the residual file, or use a native owned-file cleanup mechanism. Never
scan/delete unrelated destination files. Add an injected cleanup-failure case.

## Verified behavior

- `owner_portability.rs:58–79`: cancelling either the save dialog or replacement
  confirmation returns None before snapshot admission, cached-byte retrieval or save.
- Native selection supplies the local destination; IPC accepts no arbitrary path.
  Existing destination requires an explicit native Ok/Cancel confirmation in addition
  to any OS picker behavior. A file appearing after an initially absent selection is
  not silently replaced: the no-replace flag reaches the final rename.
- `:81–88`: current portable snapshot is read after both dialogs; cached section
  choices are reused and the sanitized bundle digest is recomputed before save.
- `cache.rs:42–92`: app ID, supplied digest, 10-minute expiry and purge epoch are
  checked. Save/cancellation/failure does not consume the preview; expiry/clear or
  changed content requires a new review. Exact cached bytes are returned, not newly
  generated unreviewed bytes.
- Snapshot queries are ordered and use one SQLite transaction. Decision current
  freshness is recomputed inside that transaction. No wall-clock export timestamp
  is injected into prepare_bundle, so unchanged-data revalidation is deterministic.
- The global admission lease remains alive through synchronous file promotion;
  purge must drain that actual worker before deletion. An open dialog owns no lease
  and cannot export its old preview after purge clears it.
- The helper pins the selected parent, creates one new random sibling file, writes
  and syncs complete bytes before promotion, rejects existing links/directories,
  and does not recursively delete or modify unrelated destination files.

## Non-findings / threat boundary

- Concurrent workspace edits after the final snapshot are not proof of a bug:
  export is a snapshot operation, not a promise that a file tracks future edits.
- Hidden/sanitized field changes may leave the reviewed digest unchanged. The
  contract is exact reviewed output, so exporting the same bytes is appropriate.
- An attacker controlling the same OS account could manipulate temporary files or
  inject application code. No stronger sandbox guarantee is claimed; hypothetical
  full same-user compromise is not a reason to reject the ordinary native save flow.
- A rename replaces the selected directory entry, not a symlink's external target.
  Existing file checks are conservative; a hypothetical swap is not automatically
  evidence that unrelated target bytes can be overwritten.
- Non-Windows helpers fail Unsupported; this product's supported native host is Windows.

## Verification limits

The new lifecycle test covers unchanged export bytes, changed app description and
changed epoch. Native helper tests cover create, explicit replacement, unrelated
files, directory rejection and hard-link preservation. This reviewer did not run them.

No fault injection covers a short write, locked promotion, failed cleanup, or an error
after the rename commits. Native rename verifies location and syncs afterward, so a
returned error can occur after destination replacement; do not claim every error
preserves the old destination without a commit-state-aware check. Cancellation source
flow is clear, but real packaged dialog/replace behavior still needs native verification.

Unresolved questions: controller decides cleanup-failure reporting/ownership. Historical
archive validation repair is implemented; fresh test results remain controller-owned.

## Controller verification follow-up

Historical recipe portability passed 8 integration tests. Cleanup failures now
identify the exact residual temporary filename and its reviewed-data custody;
post-promotion errors explicitly report replacement uncertainty. The locked-file
cleanup-failure regression and native create/replace/link-preservation cases passed
in the final library (626 passed, 3 ignored). See [execution](offline-execution.md).
Actual packaged dialogs and file-picker round trips remain open. The source
findings above are repaired; their original diagnosis is retained for traceability.
