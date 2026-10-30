# Runner installer hosted QA — 2026-10-30

Status: **FAIL** for build attempt 41086258362. No current installer or review artifact produced.

## Provenance

- [Hosted build](https://github.com/SalyyS1/what-changed/actions/runs/41086258362): source `f9ac3005e7428b51af0f2ca9907015b90576af26`, projected from primary source `112f95b`.
- Build ran 23:02–23:16 Asia/Saigon. Failed at desktop notice preparation, 16:16:43 UTC.
- Artifacts API confirms **zero artifacts**. Installer/application hashes, size and unsigned PE metadata are unavailable for this attempt.
- [Machine record](qa-261030-2235-runner-installer-artifact.json) contains exact step results and byte diagnostics.

## Passed hosted steps

Frozen runner source preflight, source contracts, focused frontend integration,
seven matching browser-source archives, native authority/generated contract,
exact runtime hydration and native packaged-inventory loading all passed.
Frontend production build also passed immediately before notice generation failed.
The approved runtime inventory remains
`31629890ecec49637621c5a4a321085306ac4392cbd95619598a81c699c47299`;
the current source-delta approval authorizes QA only.

## Failure and verified cause

`generate-third-party-notices.mjs --prepare` rejected the reviewed license hash
for `cargo:generic-array@0.14.7`. Checking all 232 reviewed public entries found
exactly three mismatches:

| Package | Reviewed bytes | Current bytes | Difference |
|---|---:|---:|---|
| generic-array 0.14.7 | 1,107 | 1,087 | 20 CR bytes removed |
| nu-ansi-term 0.50.3 | 1,157 | 1,135 | 22 CR bytes removed |
| quick-xml 0.41.0 | 1,105 | 1,082 | 23 CR bytes removed |

For all three, previous public blobs at commit
`50b32fc88abbcbf80acc4d13965ef151c2e7ed71` match the reviewed manifest hashes
exactly. Removing only CRLF differences from those blobs produces the current
primary/public bytes. This proves a byte-preservation failure; it does not
establish changed license terms. The controller owns the source-projection repair.
No upstream text, review record or human approval was changed during this audit.

## Source repair before replacement build

The controller restored all three exact reviewed blobs without changing approval
or license choices. The new synchronous `checkReviewedNoticeInputs(root)` reuses
the existing `reviewedLicense` validator and checks **232 license texts plus 33
NOTICE files** before any Cargo/Bun/runtime preparation. Actual primary source
passes. Six focused tests pass, including exact CRLF preservation, rejection of
LF normalization, changed/missing NOTICE and incomplete review.

CI invokes `node scripts/check-reviewed-notice-inputs.mjs` in its early frozen
inputs step. The public-source projector can call the exported function on its
destination after all copies; a source projection then fails before committing
or starting expensive CI if reviewed bytes differ. This checks approved input
bytes, not license approval identity, final dependency coverage or a built artifact.
Those existing release checks remain required. Replacement hosted build pending.

## Inspection scope and limits

- D free before inspection: 1,197,551,616 bytes. No artifact download or extraction occurred.
- Existing range transport checks: **5/5 pass**, zero skips. New private helpers were prepared under `.qa/qa-runtime-20261030`; original helpers remain unchanged.
- The seven source archives passed hosted preparation. They were not uploaded after the later failure, so this run supplies no downloadable co-delivery packet.
- Installer compilation/bundling, notice approval check, final artifact verification and upload were skipped after the failed gate.
- No installer/application execution, user-data access or changes to existing QA/backups.

## Unresolved gates

Rebuild after restoring reviewed source bytes. Then verify a complete installer/source
artifact and run native installation/upgrade, privacy restoration, clean Windows,
disabled/authorized network behavior, process lifecycle, accessibility, four-hour
soak and real-owner repeat use. G2/G3 and exact public publication approval remain open.
