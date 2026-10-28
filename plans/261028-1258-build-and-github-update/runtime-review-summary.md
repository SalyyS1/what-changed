# Chromium runner: exact review packet

Status: approved by the owner for installer QA with the mandatory hosted source
gate. This replaces the rejected Chrome for Testing candidate. Exact approval is
recorded in [the user decision](../../release/runner-notice-approval.json).

## Frozen scope

- Node v24.12.0, Playwright 1.62.1, ungoogled Chromium Windows x64 156.0.8078.47-1.1.
- Packaged runtime: **197 files / 543,579,634 bytes**.
- Inventory SHA256: `cc3bb9c020ec053b11df341d925f3f25e26a6dc9f0cfbdb5ab1d1cf3d3ec5554`.
- Full notice payload: 5,548,380 bytes; SHA256
  `8ad9c7c28d00641f26af587959a1020e60749fd017a3d60945f4efcb53010d9a`.
- [Proposed exact record](proposed-runtime-review.json), SHA256
  `e5b6cbd534eb7db8ebd39056a5a8bac72b7e56895cfe46d5ab5a6304741e69fb`.
- [Notice source hashes](runtime-sources.json),
  [complete notices](../../release/RUNNER_THIRD_PARTY_NOTICES.txt),
  [browser/source descriptor](../../release/runner-browser-source.json).

## Terms and delivery duties

Retain Node's full embedded dependency license; Playwright's Apache LICENSE,
NOTICE and all packaged sidecars; Chromium's full 497 credit/license blocks plus
Chromium and both ungoogled projects' root licenses. The 497 cross-platform credit
entries are retained conservatively; that count is not an execution inventory.

Chromium's statically linked FFmpeg includes LGPL-2.1-or-later code. Deliver the
**seven exact source archives (2,046,353,087 bytes)**, platform/core patches and
build/relink instructions in the **same Actions download as the installer**, with
identical access and retention. Whole-archive hashes and required members must
pass before upload. Do not distribute the installer alone or keep it available
without matching source material. This route does not create a three-year written
source offer. Permit library modifications and reverse engineering for debugging
those modifications as the applicable licenses require.

[Source/relink instructions](../../docs/runner-source-build.md) describe the
public toolchain, exact input pins, patch order and tarball rebuild. They do not
promise a byte-identical upstream optimized binary. Build tools may need public
downloads; browser end users do not download a runtime at startup.

The Microsoft SDK `d3dcompiler_47.dll` side-copy is **excluded** from the runtime;
the package does not distribute it. Windows 10/11 x64 must provide its inbox
`%SystemRoot%\System32\D3DCompiler_47.dll`. Chromium's source defines this OS
fallback. Windows 11 checks observed the GPU process loading that system DLL;
clean Windows 10/11 acceptance remains open. No additional SDK EULA is claimed
accepted. The browser's other component terms, including DXC, are retained.

## Evidence and limits

- Actual browser: eight runner actions/nine assertions, role isolation,
  authorized screenshot, close and cancellation passed.
- Browser-without-SDK-side-copy: same checks passed; system GPU DLL path observed.
- Exact local 197-file inventory validated; native loader check passed.
- Artifact acquisition/extraction/source packet tests: 33/33 passed on Windows.
- Inventory/release/continuity tests: 12/12; runner source tests: 15/15.
- Four small supplemental source archives and required members were checked locally.
- The 2 GB Chromium source archive is **not yet downloaded/verified here**. The
  hosted gate will validate all seven source packets before any installer upload.
- [Research and provenance](../reports/research-261028-1703-redistributable-chromium.md),
  [artifact QA](../reports/qa-261028-1703-chromium-artifact.md).

Approval of this packet authorizes recording the owner as reviewer and performing
installer QA with these exact inputs, **conditional on the hosted source gate**.
It does not approve public v1, public branding, a GitHub Release, download CTA,
network confinement, clean Windows acceptance, performance, soak or real-user
repeat-use claims. The source build preview remains unsigned.

Unresolved: hosted source/member receipt;
installer build/inspection; artifact-specific publication and acceptance.
