# Installed-app acceptance walkthrough

**Evidence state: available procedure; `Not run` rows are explicitly open.**

This is the recorded procedure for a release candidate. A blank result is
**not run**; it is never interpreted as a pass.

Record the candidate identity before starting: version, commit, raw NSIS name,
published installer name, SHA-256 manifest, Windows build, viewport and DPI.
Run `scripts/verify-installer.ps1` against the two installer files first. The
script verifies names, version agreement, byte identity and checksums; it does
not install the candidate and cannot replace the GUI rows below.

Latest worksheet: [repaired installed candidate](../plans/reports/qa-261027-0619-repaired-installed-candidate.md).
The unsigned 0.1.0 installer includes the workspace membership repair and has
SHA-256 `0f51687581aed0d74628f662ffc8f6ae494d637ef6e9072480b27406fcda24d9`.
It was silently reinstalled with isolated profile/WebView data; the installer
wizard was not exercised. Later source fixes require a new candidate. Partial
results below do not pass a whole row.

| # | Check | Evidence / result |
|---:|---|---|
| 1 | Install per-user with no UAC prompt | Not run: existing-install verification did not invoke the wizard or establish UAC behavior |
| 2 | Fresh launch shows onboarding and the empty Home state | Partial: isolated-profile onboarding completed; native window appeared after 1,119.734 ms. Empty Home acceptance remains open |
| 3 | Select a watched folder and change a synthetic `.env` key | Observed on the named candidate: native folder picker, watched fixture and subsequent `.env` changes |
| 4 | Timeline shows the redacted change and details drawer | Partial: recorded fixture changes visible; full redaction/details-drawer acceptance remains open |
| 5 | Describe the symptom and run an investigation | Membership repair verified: four workspace events analyzed, with `only_routine_changes` and zero suspects. A separate global port-fixture investigation produced ranked evidence. Whole product-loop acceptance remains open |
| 6 | Evidence ranking explains correlation without claiming causality | Partial: global investigation ranked the named listener and displayed a recorded structural chain; no configuration-to-port causal claim or whole-row acceptance |
| 7 | Create and compare a checkpoint | Observed: Mark working created a linked checkpoint; Compare to now showed additions/removals/changes and disclosed unavailable event-log comparison |
| 8 | Open Change Map and System Map; verify the 2D fallback | Observed: both routes; explicit persisted 2D rendered pixels, and Automatic plus reduced-motion media rendered a 2D canvas. Frame/loop timing remains open |
| 9 | Pause/resume from the tray and restart the app | Not run in this workspace |
| 10 | Delete one category and verify other categories remain | Observed on isolated candidate: configuration purge removed its rows, retained every other category count and checkpoints; Settings pause/resume worked. Disk byte-erasure claims are separately tested |
| 11 | Export diagnostics and grep for fixture names, paths and secrets | Observed: native Save dialog wrote a 1,823-byte report; all five specified fixture/path/account/value absence checks passed. Private evidence: `diagnostics-export-scan.json` named in the candidate report |
| 12 | Restart and confirm retained history survives | Not run in this workspace |
| 13 | Resize to 900×600 and traverse the core flow keyboard-only | Partial: seven routes plus Change Map had no document horizontal overflow at CDP-emulated 900×600/DPR 1.5; 18 Tab samples per primary route. System Map listbox lacked a focus ring; source fix is newer than this candidate. Native resize and complete keyboard flow remain open |
| 14 | Test 150% DPI, 200% zoom, high contrast and Narrator | Not run in this workspace; record each screen in [accessibility-audit.md](accessibility-audit.md) |
| 15 | Observe Resource Monitor for ten minutes and record zero runtime network bytes | Not run in this workspace; installation-time WebView2 download is a separate caveat |

The installed-app rows require a Windows GUI run and cannot be inferred from
unit tests or a successful Tauri build. Keep `Not run` when a row has no named
candidate evidence. Do not publish while any mandatory row is open; attach the
completed worksheet to the same release audit and go/no-go packet.

The earlier controller restored its original profile and monitoring. The repaired
candidate's session restoration has its own completion record in the latest
report. Its native clipboard observer matched the AI pack and restored the six
original clipboard formats. Private journals, clipboard contents and screenshots
of real machine activity are not publication artifacts.
