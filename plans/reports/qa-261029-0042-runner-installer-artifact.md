# Runner-enabled installer artifact inspection

Status: DONE for build and nonexecuting artifact inspection; installed acceptance remains open.

## Provenance and complete download

- [Successful Windows build](https://github.com/SalyyS1/what-changed/actions/runs/40730363355), source `e5906ea6ae2fb06859d7b361df00b36425dd496c`.
- [Complete QA artifact](https://github.com/SalyyS1/what-changed/actions/runs/40730363355/artifacts/12838120354): `what-changed-owner-workspace-preview-8`, 2,227,488,699 bytes.
- GitHub-reported ZIP SHA256: `4a9b9f4c462fd851951ae763b27756318ae7f2d2e50435b82bbd80a009be8ad7`. The complete ZIP hash was not recomputed locally.
- Artifact expiry: `2026-11-11T17:31:56Z`. Installer, sources, notices and build/relink instructions share its access and retention. Keep them together.
- [Machine inspection record](qa-261029-0042-runner-installer-artifact.json).

This is an unsigned QA candidate, not an approved public-v1 release. The
[runtime approval](../../release/runner-notice-approval.json) covers the exact
Chromium packet and source duties for QA. Source update: [draft PR #1](https://github.com/SalyyS1/what-changed/pull/1).

## Locally verified bytes

| File | Bytes | SHA256 |
|---|---:|---|
| `What Changed_0.1.0_x64-setup.exe` | 175106389 | `70ced3ac9dd66abd4235d525b7418b0d9f5d147f05e9c652e1b34c619001efbc` |
| `WhatChanged.exe` | 18817024 | `74040788006faab497d9aee93e6985013f7debfa76c6f4bb124084dad0594df9` |
| Desktop notices | 453927 | `78ae0f71406e83125a3e7625e714d393e3719837663f1a9f6410e4babe0686a0` |
| Runner notices | 5548380 | `8ad9c7c28d00641f26af587959a1020e60749fd017a3d60945f4efcb53010d9a` |

Installer and application metadata report version `0.1.0`, product `What Changed`,
and Authenticode status `NotSigned`. Product/data identity is retained.

Ten selected artifact files were hashed against its recursive checksum manifest.
Build evidence matched the expected run and exact source commit. Fresh 7-Zip
extraction produced 208 files / **568,695,583 measured bytes** without executing
the installer or application. The product executable was also streamed directly
from the checksum-verified installer and matched the extracted executable's size/hash.

The complete extracted runtime passed the existing inventory/source-parity guard:
197 files / 543,579,634 bytes, inventory SHA256
`cc3bb9c020ec053b11df341d925f3f25e26a6dc9f0cfbdb5ab1d1cf3d3ec5554`.
Node v24.12.0, Playwright 1.62.1 and ungoogled Chromium 156.0.8078.47-1.1 match
the approved packet. Both extracted notices match the artifact copies; runner
notices additionally match the human-approved payload. No packaged
`d3dcompiler_47.dll` side-copy or redirected extracted entry was found.

## Source delivery verified on CI

The hosted build verified all seven complete source archive hashes and required
members before compiling and uploading the installer. Local inspection compared
the delivered descriptor with approved bytes, checked each ZIP directory entry's
name/size, and matched the hosted receipt and root checksums against the descriptor.
The seven archives total 2,046,353,087 bytes.

The large archives remain in the complete artifact and were not downloaded again
locally. Their whole-file hashes were verified on CI. The selective private QA copy
is not an alternative distribution packet and must not be offered without matching
source materials. See [build/relink instructions](../../docs/runner-source-build.md).

## Inspection repairs and limits

Storage returned HTTP 200 for suffix-range syntax; the body was cancelled
immediately. An explicit one-byte probe returned HTTP 206 with the expected total
size. The local downloader uses explicit offsets, validates every Content-Range
and length, streams with byte limits/timeouts, rejects redirects and cancels ignored
ranges. Five transport checks passed before the real download. No signed storage
URL or credential was recorded in reports.

7-Zip 26.00 reported `NSIS-3 Unicode BadCmd=13` and omitted the final solid member's
size: `preview-runner/src/session-authority.mjs`. Before full extraction, that member
was read with a 3,091-byte cap and matched the exact source commit's SHA256
`afcb9b879a4ccdb889275e6d368704148caae2e04f4be10bf4957ef933c42cd7`.
All member paths passed traversal, alias and duplicate checks. The listing-derived
disk estimate was 568,701,219 bytes, 5,636 above the actual extracted total; these
are not matching measurements. Extraction does not validate installer-script
semantics; hosted generated-script/notice checks are separate evidence.

Automatic review rejected cache deletion. No bypass or deletion was performed;
the hosted route and bounded local download fit available disk. Existing app/data
and QA backups were preserved. D had 699,170,816 free bytes after inspection;
this is a dated observation, not a disk monitor.

## Remaining acceptance and publication work

Clean Windows 10/11, installation/upgrade and data preservation, browser network
confinement, accessibility, performance, four-hour soak and repeated real-owner use
remain unverified for this artifact. System D3DCompiler fallback was previously
measured on the development Windows 11 host, not a clean installation. The opt-in
updater has no signed production feed. Public-v1/branding approval, a GitHub Release,
tag, landing download CTA and deployment are not implied by these checks.
