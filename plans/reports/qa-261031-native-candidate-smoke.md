# Native candidate smoke — 2026-10-31

Status: **PASS_NATIVE_CANDIDATE_SMOKE; RELEASE_ACCEPTANCE_OPEN**.

This is a bounded smoke of the extracted runner-enabled candidate on the current
Windows account. It is not installer, clean-machine, upgrade, accessibility,
network-boundary, soak, or real-owner acceptance evidence.

## Candidate and session

- Candidate: extracted payload from hosted run `41091508412`, launched at
  `D:/Project/.WhatChage/.qa/qa-runtime-20261030/artifact-41091508412/extracted/WhatChanged.exe`.
- Smoke session: `c8e0a2c4f6b8d0f1a3c5e7a9b1d3f5e7`.
- The already-running legacy app was gracefully quit through its verified tray
  menu after the user explicitly authorized backup, synthetic testing, restore,
  and resume. No installer, registry, shortcut, or release files were changed.

## Observed behavior

1. Candidate first launch reached the normal onboarding/home screen.
2. `My apps` opened successfully.
3. A synthetic app named `Smoke test app` was created with the purpose
   `Record a safe synthetic app for native acceptance.`
4. Candidate exit returned code `0`.
5. Candidate relaunch returned to `#/apps` and retained the synthetic app.
6. The original executable was restarted successfully after the smoke.

## Data safety

- The original `%APPDATA%\\dev.whatchanged.app` profile was restored byte-for-byte
  from the backup: **13 files / 1,085,434,822 bytes**; profile and backup
  manifests matched exactly.
- Original executable SHA-256 remained
  `ff64a0d867c56c19802e321a4e59c7278bb911b55c4da78dc20fcf7c24f26162`.
- Original app resumed at `D:/Project/.WhatChage/src-tauri/target/release/what-changed.exe`.

## Still open

Clean Windows and non-admin proof, installer install/upgrade/uninstall, privacy
purge and retention, disabled/authorized egress behavior, cancellation and
parent-crash lifecycle, accessibility and map fallback, four-hour soak,
consenting owner repeat use, and G2/G3 exact-release approval remain open.
The retained smoke artefacts are private QA evidence and are not a public
download or publication approval.
