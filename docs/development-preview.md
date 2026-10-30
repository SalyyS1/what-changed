# Owner workspace development preview

**Evidence state: hosted build and nonexecuting installer inspection passed; installed acceptance pending.**

This unsigned Windows x64 preview contains the current owner workspace, confirmed
rules, saved requests, reviewed AI briefs, manual acceptance, portable project
memory, and the shared journey/2D/list/optional-3D evidence workbench.

The [current verified QA candidate](../plans/reports/qa-261030-2235-runner-installer-artifact.md)
records successful run `41091508412`, source `75dcb67c`, packaged inventory,
notices, installer hashes and nonexecuting inspection limits. Its
[complete installer/source artifact](https://github.com/SalyyS1/what-changed/actions/runs/41091508412/artifacts/12995563059)
is 2,227,565,645 bytes and retains all seven source archives. GitHub retention
expires on 13 November 2026 (UTC). It is not a public-v1 release.

The installer itself is 175,182,373 bytes; the installed runner inventory is
543,580,088 bytes. The additional source archives are co-delivered for the browser
and are not installed or fetched at app startup. The complete QA ZIP remains the
distribution packet; an extracted installer alone is for local inspection.

## Packaging boundary

The requested hosted candidate includes pinned Node and browser runtime bytes.
The build rehydrates only files in the frozen runtime inventory, verifies hashes,
and requires the new runtime's notice and redistribution review before bundling
or uploading the installer. Native compilation can finish while review is pending;
the pre-bundle gate still blocks the installer. It does not use PATH or download
browsers at app startup.

The [runtime input preflight](../scripts/check-preview-runtime-inputs.mjs) checks
first-party runner source against the frozen inventory before expensive downloads
or builds. A changed source file requires exact delta review; this early check
does not replace runtime notices or publication approval.

The [reviewed notice input preflight](../scripts/check-reviewed-notice-inputs.mjs)
checks desktop license and NOTICE bytes before downloads and at the end of public
source projection. Preserve the reviewed line endings: the legal-file Git
attributes protect exact hashes, and changing a checksum is not a repair.

The browser replacement is pinned ungoogled Chromium with its complete embedded
credits. Matching browser/library sources and patch/build scripts must accompany
the installer in the same downloadable artifact; source preparation is another
mandatory upload gate. See [source delivery and relinking](runner-source-build.md).
Source archive auditing uses Python 3.12.10 in the hosted build, with streaming
tar metadata and no source extraction. This is a build tool, not a bundled runtime.
The Microsoft SDK `d3dcompiler_47.dll` side-copy is excluded; Windows 10/11 x64
must provide `%SystemRoot%\System32\D3DCompiler_47.dll`.

This is a development candidate, not public v1. Clean Windows, installed upgrade,
network confinement, accessibility, performance, four-hour soak and real-owner
repeat-use acceptance remain open. Automatic updates have no configured signed feed.

The native product/data identity stays `What Changed` / `dev.whatchanged.app`.
Ownvero assets remain draft; the [landing/wiki preview](https://salyys1.github.io/what-changed/vi/)
is live with 3D/2D explanation and an unavailable public download. Existing local history is retained
by default on uninstall. Back up data before testing upgrades; no downgrade promise.

## Build provenance

The manual/push-triggered `Build Windows development preview` workflow builds an
isolated public-source revision on GitHub's Windows runner. After runtime review
and successful packaging, its artifact contains the installer, desktop/runtime
notices, these notes, SHA-256 checksums and exact source/run
identity. The workflow has read-only repository permission; it creates no Release,
tag, Pages deployment or automatic update feed. A successful job is build evidence,
not evidence of the unperformed acceptance checks above.

The approved desktop notice terms are unchanged. A guarded input-continuity check
verifies the only Cargo manifest change is the JobObjects feature on the existing
locked Windows crate; new dependency or license changes require another review.
