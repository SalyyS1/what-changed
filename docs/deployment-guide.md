# Deployment guide

**Evidence state: available procedure; partial installed-candidate evidence does not close release verification.**

This guide describes the reproducible local release path for What Changed. The
repository is the source of truth; a hosted download is a projection of the
same version and checksum.

## Build prerequisites

Use Windows 10/11 x64, the pinned stable MSVC Rust toolchain, Bun, MSVC build
tools with a Windows SDK, and WebView2. Run `bun install` once, then use:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/release.ps1
```

For a local rehearsal with uncommitted work, pass `-AllowDirty`.
`CARGO_BUILD_JOBS` defaults to `1` because parallel LLVM builds exhausted this
machine during the Phase 20 measurement. The script stops at the first failed
gate.

`-SkipPerf` and `-SkipInstaller` are rehearsal controls. A run using either
switch is evidence of the checks that did execute, never a complete release
pass. A publishable record must retain the target-resolved no-network graph,
bindings and bundle checks, installer verifier output, release-artifact audit,
upgrade rehearsal and the manual acceptance worksheet for the same candidate.

## Installer and data

The NSIS bundle uses `installMode: "currentUser"`, so installation does not
request elevation. The raw Tauri artifact is named
`What Changed_<version>_x64-setup.exe`; the release script copies it to
`dist-installer/WhatChanged-Setup-x64.exe` and writes SHA-256 values beside it.

The app data directory owner is [`AppPaths::resolve`](../src-tauri/src/app/paths.rs),
which uses `dirs::data_dir()` and normally resolves to `%APPDATA%\\dev.whatchanged.app\\`.
Upgrading leaves that directory
in place; migrations run on the next launch. Uninstall keeps it by default.
The first release has no automatic or background update check. Settings contains an explicit opt-in
scaffold, but the current build has no signed HTTPS feed or public key, so it reports that updates are
not configured and leaves downloading manual.

The privacy screen's full-history action is separate from database recovery: it
requires the visible `delete` phrase and a short-lived one-use challenge,
replaces the database, preserves only the reviewed appearance preferences, and
returns to onboarding. Recovery Repair/Reset is accepted only for a corrupt or
schema-mismatched database and keeps a quarantined artifact for diagnosis.

## Signing and WebView2

The current artifact is unsigned. Windows may show **Windows protected your PC**
and **Unknown publisher**; the documented path is **More info → Run anyway**.
Production signing needs a separately managed EV/OV certificate and `signtool`
or Tauri signing integration. Certificate material must be supplied as a
release secret and never committed.

If WebView2 is missing, the default bootstrapper may access the network during
installation. Runtime collection has no network client in the resolved Windows
dependency graph. A fully offline WebView2 bundle is a later distribution
decision because of its size.

## Verification evidence

Use [release-audit.md](release-audit.md) for requirement evidence,
[acceptance-walkthrough.md](acceptance-walkthrough.md) for installed-app checks,
and [public-v1-go-no-go.md](public-v1-go-no-go.md) for the final publication
packet. Manual Windows checks must include the version, commit, viewport and
whether the fixture data was synthetic.

The [installed-candidate continuation](../plans/reports/qa-261027-0434-mvp-continuation.md)
records the verified NSIS payload, bounded native UI checks, failed workspace
investigation and completed profile/startup restoration. The candidate predates
the subsequent source repair and requires a rebuild and native retest.

## Public source-preview wiki

The user authorized the public source preview at
[What Changed wiki](https://salyys1.github.io/what-changed/). It projects the
canonical source docs and does not authorize binary downloads or a public-v1 launch.

For documentation publication checks, select the public
[`SalyyS1/what-changed`](https://github.com/SalyyS1/what-changed) repository explicitly.
GitHub repository visibility and Pages settings own the route; read-only lookup is
`gh repo view SalyyS1/what-changed --json nameWithOwner,visibility,url` and
`gh api repos/SalyyS1/what-changed/pages`. Pages uses a workflow deployment.
Use an identity with repository/Pages read access; publishing changes requires
the existing source-publication authorization. This route grants no binary
publication permission. Keep private `.qa` data out of the public projection.
Confirm the repository identity and Pages `html_url` before using the route;
if either changes or access fails, consult the repository owner rather than
selecting another destination. The [continuation evidence](../plans/reports/qa-261027-0434-mvp-continuation.md)
records the verified metadata; it is not a live availability monitor.

## Ownvero development candidate

For the hosted runner-enabled development build and its exact notice gate,
see [development preview packaging](development-preview.md). The workflow creates
an Actions review artifact only; publication requires a separate decision for its bytes.

The local static site source is [wiki-site](../wiki-site/README.md). Run
`bun run site:build`, `bun run site:test`, `bun run check:site` and
`bun run check:download`. Preview with `node wiki-site/serve.mjs` at the documented
base path on port 5184. Downloads remain unavailable until an exact G2/G3-bound
manifest exists. The workflow builds a preview artifact and does not deploy.

`bun run runner:package` prepares `runner/bundle` from the previously verified
local spike runtime; it does not download anything. The script validates source
hashes and writes a fresh byte inventory including current runner code.
If packaging is interrupted, preserve its printed `runner/bundle-stage-<id>` directory
and resume with `powershell -File scripts/package-preview-runner.ps1 -ResumeStage bundle-stage-<id>`.
Resume revalidates runtime bytes and refreshes source without another full runtime copy;
unexpected files stop promotion. Earlier output is preserved when promoting a replacement.
`bun run check:runner-bundle` rejects missing/changed/extra/redirected files.
These resources are generated and ignored by Git. `bun run runner:test` checks
source recipes/protocol/policy independently of browser packaging.
`bun run check:runner-notices` requires a fresh exact-bundle license review and
the packaged `release/RUNNER_THIRD_PARTY_NOTICES.txt` payload. Its review record
is currently pending; the older desktop notice approval does not cover this delta.

To build a local preview-enabled candidate, use
`powershell -File scripts/release.ps1 -AllowDirty -IncludePreviewRunner` after
preparing the runtime and reviewing its new license obligations. The Tauri
`tauri.runner.conf.json` overlay includes the resources; default builds omit them.
The core graph gate is also named `bun run core:no-network`; the legacy alias
remains. It proves neither disabled browser behavior nor authorized confinement.

`node scripts/check-owner-release.mjs --candidate release/candidate-evidence.json`
reports exact missing candidate evidence. `--publish` additionally requires all
evidence and G0–G3 approval bound to that packet and the download manifest. Build
checks, native install/upgrade, clean Windows, disabled-mode outbound absence,
authorized egress, crash/cancel, screenshots/deletion, map accessibility, four-hour
soak, license delta and repeated owner use remain separate requirements. Do not
upload a candidate or enable a CTA based only on source tests.
