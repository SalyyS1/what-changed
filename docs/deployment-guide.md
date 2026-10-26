# Deployment guide

**Evidence state: available reproducible procedure; installed release verification is a known limitation until the RC walk.**

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

The app data directory is resolved through Tauri's `app_data_dir` and normally
appears as `%APPDATA%\\dev.whatchanged.app\\`. Upgrading leaves that directory
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
