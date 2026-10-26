# Installed-app acceptance walkthrough

**Evidence state: available procedure; `Not run` rows are explicitly open.**

This is the recorded procedure for a release candidate. A blank result is
**not run**; it is never interpreted as a pass.

Record the candidate identity before starting: version, commit, raw NSIS name,
published installer name, SHA-256 manifest, Windows build, viewport and DPI.
Run `scripts/verify-installer.ps1` against the two installer files first. The
script verifies names, version agreement, byte identity and checksums; it does
not install the candidate and cannot replace the GUI rows below.

| # | Check | Evidence / result |
|---:|---|---|
| 1 | Install per-user with no UAC prompt | Not run in this workspace; record the candidate identity and UAC result |
| 2 | Fresh launch shows onboarding and the empty Home state | Not run in this workspace; use a fresh app-data directory |
| 3 | Select a watched folder and change a synthetic `.env` key | Not run in this workspace |
| 4 | Timeline shows the redacted change and details drawer | Not run in this workspace |
| 5 | Describe the symptom and run an investigation | Not run in this workspace |
| 6 | Evidence ranking explains correlation without claiming causality | Not run in this workspace |
| 7 | Create and compare a checkpoint | Not run in this workspace |
| 8 | Open Change Map and System Map; verify the 2D fallback | Not run in this workspace |
| 9 | Pause/resume from the tray and restart the app | Not run in this workspace |
| 10 | Delete one category and verify other categories remain | Not run in this workspace |
| 11 | Export diagnostics and grep for fixture names, paths and secrets | Not run in this workspace; attach the export scan result |
| 12 | Restart and confirm retained history survives | Not run in this workspace |
| 13 | Resize to 900×600 and traverse the core flow keyboard-only | Not run in this workspace |
| 14 | Test 150% DPI, 200% zoom, high contrast and Narrator | Not run in this workspace; record each screen in [accessibility-audit.md](accessibility-audit.md) |
| 15 | Observe Resource Monitor for ten minutes and record zero runtime network bytes | Not run in this workspace; installation-time WebView2 download is a separate caveat |

The installed-app rows require a Windows GUI run and cannot be inferred from
unit tests or a successful Tauri build. Keep `Not run` when a row has no named
candidate evidence. Do not publish while any mandatory row is open; attach the
completed worksheet to the same release audit and go/no-go packet.
