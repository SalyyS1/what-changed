# Independent owner portability review

Status: DONE_WITH_CONCERNS

## Scope

Reviewed `src-tauri/src/owner_workspace/archive.rs`, `archive/reader.rs`,
`portability/{review,records}.rs`, `storage/portability.rs`, and
`storage/portability/apply.rs` against the P05 plan and owner architecture.
Findings reference the source inspected before controller fixes.

No archive/portability source edited. No Cargo, network, real app-data, or target
access performed. Existing tests inspected; root owns execution and final results.

## Findings

### P1: quoted local paths with spaces leak owner-name suffixes

`portability/review.rs:19` recognizes only unquoted paths. The default recursive
filter turns `Never delete "C:\Users\Private Owner\project"` into
`Never delete "[private detail omitted] Owner[private detail omitted]"`.
The username suffix remains in JSON and generated Markdown. A direct PowerShell
regex reproduction confirmed the source-pattern behavior. The brief renderer's
quoted-path alternatives provide an existing repair pattern.

Recommended fixture: a quoted Windows path with a spaced username, plus quoted
POSIX/UNC forms; assert every canary component is absent from archive and preview
while surrounding rule meaning remains.

### P1: default filtering invalidates valid typed web recipe routes

`portability/review.rs:24` filters every string without typed-field context.
`/users/new` becomes `[private detail omitted]`; `/home/settings`, `/media/new`,
and other route prefixes overlap the local-path matcher too. The subsequent
`workspace.validate()` at line 49 calls recipe validation, whose route invariant
requires a leading `/` (`validation/recipe.rs:6`). A populated supported scenario
using a common users route therefore cannot export with default selection. Import
also reuses default preparation, so selecting targets on the original export does
not avoid this import failure.

Recommended repair: preserve validated relative recipe `path`/`path_template`
fields in their typed context while filtering actual local paths and prose.
Add a populated scenario/adapter round-trip fixture using `/users/new` and
`/users/{fixture_id}`.

### P2: portable decision projection reads stale stored freshness

`storage/portability.rs:32` through line 34 reads raw runs/results/decisions.
Decision `current_freshness` is stored at decision time; live evidence reads
recompute it. After editing a draft, rule, scenario, or environment, a live review
reports stale but the portable decision can still serialize unknown/current
metadata from its original save. Architecture requires recomputation on export.
Imported records remain archival, so this does not grant active verification.

Per controller request, added the owned helper in `storage/evidence.rs`:

```rust
pub(super) fn review_in_snapshot(
    conn: &Connection,
    app_id: &str,
    run_id: &str,
) -> AppResult<OwnerEvidenceReviewDto>
```

It validates IDs and uses the existing review projector in the caller's SQLite
transaction. Portability can consume its freshly projected decisions. Direct
rustfmt passed; root compilation and export freshness fixture remain required.

## Verified boundaries

- Stored ZIP layout has local headers, central directory, EOCD, CRC32, and bounded
  in-memory slices. Entry names never become OS paths. No concrete traversal,
  link, decompression, or extraction failure identified in the inspected code.
- Exact four-name allowlist, metadata/compression rejection, 16 MiB limit,
  contiguous local bodies, CRC checks, and exact central inventory bound hostile
  archive handling. The supported stored-only profile is an explicit decision.
- Import inserts a fresh app and fresh live rule/capability/draft IDs, confirms
  every imported rule, and retains observations/targets/scenarios as history.
  Execution consent and current candidate are not restored.
- Import redoes default privacy preparation before operation-digest persistence;
  untrusted original bytes do not enter the mutation request digest.

## Coverage concern

`PortableRecords::validate` checks top-level app ownership, capability/rule links,
recipe structure, and preserved rule references. It does not fully validate every
candidate/environment/run/result/decision link or duplicate scenario/change ID.
No active-evidence authority escalation was established from this gap; those
records remain archival. Additional graph fixtures would strengthen malformed
import handling, but this was not promoted to a verified security finding.

## Unresolved

Controller fixes and populated P06 portability/freshness Cargo results pending.
