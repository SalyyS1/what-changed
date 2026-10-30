# Runner installer hosted QA — 2026-10-30/31

Status: **PASS_BUILD_AND_NONEXECUTING_INSPECTION** for replacement run 41091508412.
Native acceptance and public-v1 approval remain open. Neither installer nor
application was executed.

## Candidate and provenance

- [Successful hosted build](https://github.com/SalyyS1/what-changed/actions/runs/41091508412): public source `75dcb67c3ba9c6e8c6ad97e2166dd1626c44cb11`, projected from primary source `d71eaf36e837257f894255a4898576a8e378699b`.
- Build: 2026-10-30 16:36:59–17:09:51 UTC; 23:36:59–00:09:51 Asia/Saigon, crossing into October 31.
- [Complete review artifact](https://github.com/SalyyS1/what-changed/actions/runs/41091508412/artifacts/12995563059): `what-changed-owner-workspace-preview-10`, ID `12995563059`, **2,227,565,645 bytes**. Expires 2026-11-13 17:09:27 UTC.
- GitHub reports ZIP SHA256 `1cd05d42d352e7bc40fd809dcb5fed01713abfc2ac9aacc03bae7edbf647178d`. The complete ZIP was not downloaded or rehashed locally.
- [Machine report](qa-261030-2235-runner-installer-artifact.json) owns exact hosted steps, approval binding, archive metadata and retained failure evidence.

The frozen source/notices check, source contracts, focused frontend integration,
seven corresponding-source archives, native authority/generated contract,
runtime hydration, native packaged-inventory loading, notice generation and
approval, desktop compilation, installer bundling, final verification and artifact
upload passed in the replacement run.

## Verified local payloads

| Payload | Bytes | SHA256 |
|---|---:|---|
| `What Changed_0.1.0_x64-setup.exe` | 175,182,373 | `071a11aaa5d97af5378070a117eeb3597c73bac96d018046589f22cb555c83e1` |
| `WhatChanged.exe` | 18,817,536 | `31b703c0a9b65e5a10597ca318ef1a45de6a094eaf60325b1c1ce6f1dc55ed67` |
| `THIRD_PARTY_NOTICES.txt` | 453,927 | `78ae0f71406e83125a3e7625e714d393e3719837663f1a9f6410e4babe0686a0` |
| `RUNNER_THIRD_PARTY_NOTICES.txt` | 5,548,380 | `8ad9c7c28d00641f26af587959a1020e60749fd017a3d60945f4efcb53010d9a` |

Both PE files report **NotSigned**, version `0.1.0`, product `What Changed`.
The application executable independently streamed from the checksum-verified
installer matches the extracted executable. Both extracted notice files match
the downloaded copies; the runner notice also matches the approval.

Actual runtime inventory and source parity passed: **197 files / 543,580,088
bytes**, inventory SHA256
`31629890ecec49637621c5a4a321085306ac4392cbd95619598a81c699c47299`.
Runtime versions: Node `v24.12.0`, Playwright `1.62.1`, ungoogled Chromium
`156.0.8078.47-1.1`.

The exact [QA source-delta approval](../../release/runner-source-delta-approval.json)
used here has SHA256
`4aacd57ec5bafe2ac0f5e0dadeceee33a29d3269506411447b64e041b385880d`.
Its scope is the owned runner source delta and aggregate inventory for an unsigned
QA build; it does not close native acceptance, G2, G3 or public-v1 publication.

## Source co-delivery and download scope

Range transport checks: **5/5 passed**, zero skips. Selective download retained
**10 files / 181,209,434 bytes**, including the root checksum manifest. The nine
selected files covered by that manifest matched their listed SHA256 values.

Seven source archive names and sizes in the complete artifact central directory
match the approved descriptor and downloaded source receipt/checksum metadata.
Hosted CI verified their whole-byte hashes and required members. The seven
archive payloads were **not downloaded or independently hashed locally**.
Private receipts remain under `.qa/qa-runtime-20261030/artifact-41091508412/`:
`selective-download-receipt.json` and
`runtime-sources/source-materials-receipt.json`. They are not public repository
links; the complete hosted artifact above contains the source receipt.

Retain the complete installer, seven sources, notices and build instructions
together for any permitted distribution. The selective local installer copy is
not a complete distribution packet.

## Extraction and measured limits

- Full extraction: **208 files / 568,713,142 actual bytes**. No redirected entries or excluded `d3dcompiler_47.dll` were found.
- Listing estimate: **568,718,778 bytes**, **5,636 bytes above** measured extraction.
- 7-Zip NSIS listing may report `BadCmd=13` and omit the final solid member size. That member, `preview-runner\src\session-authority.mjs`, was streamed with a **3,091-byte** cap and matched approved SHA256 `afcb9b879a4ccdb889275e6d368704148caae2e04f4be10bf4957ef933c42cd7`.
- D free before selective download: approximately **1,191,415,808 bytes**; before extraction: **1,009,950,720 bytes**. Extraction preflight required **250 MiB / 262,144,000 bytes** remaining plus **32 MiB / 33,554,432 bytes** reserve.
- Recorded D free after inspection/report preparation: **440,377,344 bytes**. No additional large download, copy or extraction was started.

Private `extraction-preflight.json` and `installer-inspection-receipt.json` in
that same local directory retain the measurements. Extraction does not prove NSIS installer-script
semantics; hosted script/notice checks passed separately. No payload execution,
user-data access, alteration of existing QA/backups or public publication occurred.
No owned background process remains.

The candidate is retained in private
`.qa/qa-runtime-20261030/artifact-41091508412/extracted/` for a later authorized
native check. This local directory is excluded from public source projection.

## Preserved first failure and repair

[First run 41086258362](https://github.com/SalyyS1/what-changed/actions/runs/41086258362)
used public source `f9ac3005e7428b51af0f2ca9907015b90576af26`, projected from
primary `112f95bc295bca62fbbfd619d21be7f31672d2ad`. It failed at 2026-10-30
16:16:43 UTC during desktop notice preparation:
`license text hash does not match reviewed bytes: cargo:generic-array@0.14.7`.
The artifacts API returned **zero artifacts**; no installer belongs to that run.

Checking all 232 reviewed license texts found exactly three mismatches:

| Package | Approved bytes | Failed-source bytes | Difference |
|---|---:|---:|---|
| generic-array 0.14.7 | 1,107 | 1,087 | 20 CR bytes removed |
| nu-ansi-term 0.50.3 | 1,157 | 1,135 | 22 CR bytes removed |
| quick-xml 0.41.0 | 1,105 | 1,082 | 23 CR bytes removed |

Previous public blobs at `50b32fc88abbcbf80acc4d13965ef151c2e7ed71` match the
reviewed hashes. Removing only their CRLF bytes reproduces the failed-source
bytes. The controller restored the exact reviewed blobs, preserving license
choices and approvals, and integrated the early
[`checkReviewedNoticeInputs(root)` guard](../../scripts/check-reviewed-notice-inputs.mjs)
into source projection and CI. It checks 232 license texts plus 33 NOTICE files.
Six new focused tests and the combined early suite **13/13** passed. The
replacement hosted build cleared the former failure and all final artifact gates.

Full first-attempt evidence is preserved in the machine report's
`previousAttempts` and private `.qa/qa-runtime-20261030/` files
`failed-run-41086258362.json` and `notice-byte-diagnostic.json`.

## Remaining gates

Clean Windows; native install/upgrade and privacy purge/restoration; disabled and
authorized network behavior; runner cancellation/parent-crash lifecycle;
accessibility fallback; performance and four-hour soak; real-owner repeat use;
G2/G3; exact public-v1 publication approval. This inspection does not close them.
