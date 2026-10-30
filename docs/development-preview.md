# Owner workspace development preview

**Evidence state: hosted build and nonexecuting installer inspection passed; installed acceptance pending.**

This unsigned Windows x64 preview contains the current owner workspace, confirmed
rules, saved requests, reviewed AI briefs, manual acceptance, portable project
memory, and the shared journey/2D/list/optional-3D evidence workbench.

The [verified QA candidate](../plans/reports/qa-261029-0042-runner-installer-artifact.md)
records the successful build, complete installer/source artifact, hashes and
inspection limits. Download the complete artifact with its seven source archives;
its GitHub retention expires on 11 November 2026. It is not a public-v1 release.

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
Ownvero assets and website remain draft source. Existing local history is retained
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
