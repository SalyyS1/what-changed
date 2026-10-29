# Native QA route — 2026-10-26

Status: DONE_WITH_CONCERNS. Read-only scouting; no application launch, history
content read, backup, registry write, account change, or process termination
performed by this scout. The later shutdown diagnosis read only `startup.json`
and file/process/window metadata, as explicitly delegated.
Scope: `--auto --advise`; full native acceptance scope remains.

The latest user authorization allows the current Windows account. It supersedes
the separate-account requirement in `qa-261005-native-account-handoff.md`.
Protect existing history with a verified offline copy and an original-folder
rename; run destructive acceptance only against the newly created disposable
history. Authorization is not a release acceptance pass.

## Actual artifacts and environment

| Path, relative to `D:/Project/.WhatChage` | Observed state |
|---|---|
| `dist-installer/WhatChanged-Setup-x64.exe` | 2026-09-26 05:57 +07; 4,458,019 bytes; SHA-256 `6a5aa3963b5e5bfbe5dd9a8d1e2d8a02e5b97d24654092de432bbc97a71aaa9e`; old rehearsal |
| `src-tauri/target/release/what-changed.exe` | 2026-10-15 17:40 +07; 15,277,056 bytes; SHA-256 `5a378f344df2b624def573ee56caef5b39d25240b4b7b3edadb4d4f7becdbef3`; engineering binary, not frozen current-source RC |
| `src-tauri/target/release/seed_history.exe` | 2026-10-14; 3,816,960 bytes; devtools helper, not a release component |
| `src-tauri/target/release/resource_probe.exe` | Present; 2026-10-15 engineering helper |
| `src-tauri/target/release/bundle/nsis/What Changed_0.1.0_x64-setup.exe` | Absent in this checkout; manifest still names it |
| `src-tauri/target/release/nsis/x64/installer.nsi` | Absent in this checkout |
| `dist-installer/SHA256SUMS.txt` | Old two-entry manifest; current verifier requires three entries including reviewed notices |
| `dist-installer/THIRD_PARTY_NOTICES.txt` | 127-byte `PENDING - NOT FOR RELEASE` placeholder; not acceptable release payload |

`CARGO_TARGET_DIR` was unset. Both package versions remain `0.1.0`; source includes
migrations through `016_project_baseline.sql` and many uncommitted changes. A new
source fingerprint/build is needed before calling any binary current.

Read-only preflight found `what-changed` PID **17316**, running the engineering
binary above, with title `dev.whatchanged.app-siw`. Establish ownership and use
graceful Quit before touching its history. A second launch may just activate this
single-instance process.

The existing `%LOCALAPPDATA%/What Changed` directory, `WhatChanged.exe`, and
`uninstall.exe` retain EFS encryption; both binaries date to 2026-09-24. That
matches the recorded uninstaller-helper failure. Do not decrypt arbitrary
profile files or call the old install a clean baseline. A targeted read of HKCU
uninstall entries with DisplayName exactly `What Changed` returned no row; this
does not establish that no other entry exists.

The current HKCU Run value `What Changed` points to the engineering binary with
`--minimized`. Preserve this exact value/type and the existing Start Menu shortcut
before testing autostart or installing. The Start Menu shortcut exists; no shortcut
was found at the literal Desktop path checked (redirected Desktop was not checked).

## Actual data path and safe backup

Owner: `src-tauri/src/app/paths.rs`, `AppPaths::resolve()` calls `dirs::data_dir()`
and appends `dev.whatchanged.app`. Overriding `APPDATA` alone is insufficient;
normal GUI mode has no supported `--data-dir` override. The current data directory
exists under the OS roaming application-data folder. Its contents were not opened.
`docs/deployment-guide.md` currently says Tauri `app_data_dir`; actual owner is
`dirs::data_dir()` (same intended roaming destination).

No dedicated personal-history backup/restore script exists in `scripts/`.
Use the following reviewed recipe **after genuine Quit and process exit**. Keep
backup/manifest files local under ignored `.qa/`; do not commit them. Copy the
whole directory, including WAL/SHM, backups, logs and window/startup state.

```powershell
$ErrorActionPreference = 'Stop'
$qaRepo = 'D:/Project/.WhatChage'
$qaStamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$qaRoot = Join-Path $qaRepo ".qa/native-$qaStamp"
$roamingRoot = [Environment]::GetFolderPath('ApplicationData')
$dataRoot = [IO.Path]::GetFullPath((Join-Path $roamingRoot 'dev.whatchanged.app'))
$savedRoot = [IO.Path]::GetFullPath((Join-Path $roamingRoot "dev.whatchanged.app-preserved-$qaStamp"))
$backupRoot = [IO.Path]::GetFullPath((Join-Path $qaRoot 'personal-history-backup'))

if (Get-Process -Name 'WhatChanged','what-changed' -ErrorAction SilentlyContinue) {
  throw 'Quit all identified What Changed processes before copying history.'
}
if ((Split-Path -Parent $dataRoot) -ne $roamingRoot -or
    (Split-Path -Parent $savedRoot) -ne $roamingRoot -or
    (Split-Path -Leaf $dataRoot) -ne 'dev.whatchanged.app' -or
    -not $backupRoot.StartsWith([IO.Path]::GetFullPath((Join-Path $qaRepo '.qa')) + [IO.Path]::DirectorySeparatorChar)) {
  throw 'Unexpected backup or rename boundary.'
}
if (Test-Path -LiteralPath $savedRoot) { throw 'Preserved directory already exists.' }
if (-not (Test-Path -LiteralPath $dataRoot -PathType Container)) { throw 'Expected original data folder is absent.' }
$links = @(Get-Item -LiteralPath $dataRoot; Get-ChildItem -LiteralPath $dataRoot -Force -Recurse) |
  Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }
if ($links) { throw 'Review reparse points before recursive backup.' }
New-Item -ItemType Directory -Path $qaRoot | Out-Null

function Get-LocalFileManifest([string]$Base) {
  $prefix = $Base.TrimEnd('\') + '\'
  @(Get-ChildItem -LiteralPath $Base -Force -Recurse -File | ForEach-Object {
    [pscustomobject]@{
      Relative = $_.FullName.Substring($prefix.Length)
      Length = $_.Length
      SHA256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash
    }
  } | Sort-Object Relative)
}
$originalManifest = Get-LocalFileManifest $dataRoot
Copy-Item -LiteralPath $dataRoot -Destination $backupRoot -Recurse
$backupManifest = Get-LocalFileManifest $backupRoot
if (($originalManifest | ConvertTo-Json -Depth 4 -Compress) -ne
    ($backupManifest | ConvertTo-Json -Depth 4 -Compress)) {
  throw 'Backup byte identity failed; original left in place.'
}
$originalManifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $qaRoot 'personal-backup-manifest.json')
[ordered]@{ DataRoot=$dataRoot; SavedRoot=$savedRoot; BackupRoot=$backupRoot } |
  ConvertTo-Json | Set-Content -LiteralPath (Join-Path $qaRoot 'restore-paths.json')
# Same-volume rename keeps the original intact; no deletion or overwrite.
Move-Item -LiteralPath $dataRoot -Destination $savedRoot
```

Stop if the original process reappears. Preserve the Run entry before any sign-out
or reboot, since existing autostart points at the engineering binary. Record the
pre-test installation and shortcuts too; app-data restoration alone does not undo
installer or autostart changes.

Restore after graceful Quit: archive the disposable `dev.whatchanged.app` under a
unique **sibling** `dev.whatchanged.app-qa-result-<stamp>`; assert both resolved paths
have the same expected roaming parent; require destination absent; then move the
preserved original back to the exact data path. Compare its opaque file manifest
with the saved manifest before relaunch. Restore the prior autostart setting and
any test-changed shortcuts. Keep the verified backup until restoration is confirmed.
Never launch the older rehearsal binary against migrated personal history.

## Graceful tray Quit

`src-tauri/src/app/tray.rs`: tooltip `What Changed`, right-click menu item `Quit`
(`tray-quit`) calls `app/lifecycle.rs::quit()`. That calls `shutdown_cleanly()`
(stops collectors/checkpoint work, flushes DB, records clean shutdown), then exits.
`WM_CLOSE`, Alt+F4, and `CloseMainWindow()` normally **hide**, not quit.

No existing script automates tray Quit. Two viable native routes:

1. UI Automation: scope to Explorer taskbar/notification overflow, find exact
   tooltip/name `What Changed`, right-click its verified rectangle, then invoke the
   menu item named `Quit` in the resulting native popup. Verify process exit.
2. A source-verified fallback avoids guessing tray coordinates. The installed
   dependency `C:/Users/Salyyy/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tray-icon-0.24.2/src/platform_impl/windows/mod.rs`
   creates a hidden top-level window with class `tray_icon_app`. Use `EnumWindows`
   and `GetWindowThreadProcessId` to require **the intended PID** and that exact
   class. `PostMessageW(hwnd, 6002, 0, 0x205)` delivers its normal tray callback
   (`WM_USER_TRAYICON`, `WM_RBUTTONUP`) and opens the real popup at the cursor.
   Then inspect the popup (`#32768`) owned by that PID and invoke its exact
   `Quit` item via UI Automation or click its verified rectangle. Use PostMessage,
   not SendMessage, because the popup runs a modal menu loop. If no menu appears,
   stop; do not send unrelated window messages or terminate the process.

These routes were proposed from source. The controller subsequently tried the
PostMessage/UIA route in `.qa/quit-native-app.ps1`; it found no app-owned `Quit`
menu, so graceful shutdown remains unverified. Do not describe that route as a
working runtime solution. A separate adviser is investigating it; do not duplicate
the same invocation.

Controller handoff at report completion: an online SQLite backup at
`.qa/native-profile-backup-20261026` completed with exit 0 and `quick_check` OK;
the final manifest records three persistent files after SQLite connections closed.
The original folder remains untouched and has not been renamed. That DB snapshot
is useful recovery evidence; it is not the coherent whole-directory offline
backup/rename prerequisite described above.

## Build, install, upgrade and uninstall

Current full path: `D:/Project/.WhatChage/scripts/release.ps1`. It performs source,
target-network, notice, performance, NSIS, checksum and artifact checks. Current
`scripts/verify-installer.ps1` requires raw+published installers, **reviewed notice**,
generated NSIS script and **three** checksum entries. Existing preserved artifacts
cannot satisfy it. Do not bypass the pending legal-payload gate to claim RC status.

```powershell
Set-Location 'D:/Project/.WhatChage'
# Local rehearsal only while dirty; omit -AllowDirty for a clean release.
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/release.ps1 -AllowDirty
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-installer.ps1
```

After a verified candidate and disposable data setup, launch the installer
interactively (required to observe UAC, choices, artwork, notices and warnings):

```powershell
$installer = 'D:/Project/.WhatChage/dist-installer/WhatChanged-Setup-x64.exe'
$installProcess = Start-Process -FilePath $installer -PassThru
# Record PID; observe the actual wizard. Do not supply -Verb RunAs.
```

Tauri config requests `currentUser`. Expected program filename is
`WhatChanged.exe`; normal local install location is `%LOCALAPPDATA%/What Changed`.
Because that existing location is EFS-encrypted, resolve installer state first;
an explicitly chosen new non-EFS directory inside the current user's profile can
support a fresh-install rehearsal, with the custom path recorded. Do not silently
decrypt the old install or overwrite it without a recoverable installation record.

`src-tauri/nsis/installer-hooks.nsh` makes the desktop shortcut opt-in. Interactive
finish checkbox defaults unchecked. Silent/passive `/DESKTOPSHORTCUT` is explicit
opt-in; default preserves an existing desktop link and still maintains Start Menu
behavior. Silent install cannot prove no UAC or the GUI choice and is unsuitable
for walkthrough #1 alone.

Upgrade acceptance needs **two distinct installed versions and a real migration**,
10k synthetic events before upgrade, retained count and new schema afterward.
The phase's `004_test_upgrade.sql` example is stale: migration 004 already exists,
and current tree reaches 016. Use isolated versioned rehearsal source/artifacts
with the next migration number; never modify the shared tree or run a newer-schema
fixture through the old installer. `upgrade_rehearsal.rs --include-ignored` checks
only identity and absence of an old temporary file; it does not execute an upgrade.

The standalone seeder accepts only an existing **empty** directory named
`what-changed-perf-*` or `what-changed-seed-*`, rejects normal app data, and writes
an ownership sentinel. Build it from the **baseline version being rehearsed**.
Seed outside normal app data, close the seeder, then stage the known synthetic
fixture into the disposable normal path. Keep original seed directory/sentinel
ownership intact; do not point seed/purge at the normal profile.

```powershell
# Example fixture-only command, after creating a unique empty directory.
cargo run --release --locked --manifest-path src-tauri/Cargo.toml --features devtools --bin seed_history -- --events 10000 --days 30 --seed 42 --data-dir 'D:/Project/.WhatChage/.qa/what-changed-seed-native-upgrade'
```

Uninstall acceptance: Quit; record test-history count/opaque hashes; run the
**candidate's own** `uninstall.exe` interactively; leave `Delete app data` unchecked;
verify program/shortcut removal and disposable history retention; reinstall same
candidate and verify retained events. Test delete opt-in only with disposable data.
The old EFS uninstaller failure is an unresolved environment issue, not a pass.

## Native UI and measurements

All helpers below already exist in `D:/Project/.WhatChage/scripts`. Pass the actual
process base name: installed `WhatChanged`; engineering `what-changed`. Their
defaults differ, and most choose the first matching process rather than a PID.
Ensure exactly one intended process before using them.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/navigate-window.ps1 -ProcessName WhatChanged -Shortcut 2
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/capture-window.ps1 -ProcessName WhatChanged -Out 'D:/Project/.WhatChage/.qa/native-capture.png'
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/resize-window.ps1 -ProcessName WhatChanged -Width 900 -Height 600
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/click-window.ps1 -ProcessName WhatChanged -X 100 -Y 100
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1 -ProcessName WhatChanged -SettleSeconds 5
```

The click coordinates are an invocation example, not a verified control location.
Shortcut routes 1–7: Home, Timeline, Investigations, Checkpoints, System Map,
Settings, Diagnostics. Capture uses PrintWindow flag 2 for WebView2 and warns if
capture may be blank; inspect each PNG. Resize measures **outer window** logical
size, so record actual content viewport too. `capture-launch-assets.ps1` only writes
a capture manifest; it does not take screenshots.

`scripts/browser-a11y-check.mjs` is headless browser/source evidence, not native
WebView2/Narrator evidence. No general native UIA automation harness is checked in.
`scripts/soak.ps1` samples a supplied PID and descendants, with acknowledged
simulation when configured. A headless devtools `--soak --data-dir` route exists,
but it cannot prove WebView2 resource behavior. Record shown/hidden/four-hour total
and private working set; D-15 says **report both, gate on neither**. The memory
helper's printed 80 MB verdict does not override D-15.

## Acceptance still open

- All 15 `docs/acceptance-walkthrough.md` rows need one named installed candidate.
- Fresh onboarding, config fixture/redaction/details/investigation/checkpoint/maps,
  tray pause/resume/restart, scoped purge, diagnostics leak scan, retained history.
- Installed 900×600 keyboard traversal, Narrator, high contrast, 150% DPI and 200%
  zoom per `docs/accessibility-audit.md`; native toast-click behavior.
- Ten-minute Resource Monitor zero-runtime-network record; installation WebView2
  download is separate. Idle CPU/startup timings and four-hour soak remain open.
- New verified installer, actual UAC/shortcut/notice inspection, two-version
  migration, uninstall keep/delete choices; no release publication authorized here.

Unresolved questions at initial inspection: ownership/graceful closure of existing PID; final frozen
candidate and reviewed notices; clean install path given EFS leftovers; selected
baseline/candidate versions for genuine migration rehearsal. These are execution
prerequisites/evidence gaps, not a request for a separate Windows account.

## Focused shutdown diagnosis — continuation

Controller reports a verified real menu-ID Quit dispatch, then one verified
`WM_CANCELMODE`; PID 17316 remained alive. This continuation performs no messages,
input injection, application launch, debugger attachment, termination or file
mutation beyond this report. Read-only native API queries are limited to the
target's window/thread metadata; no memory dump or history content was collected.

Observed:

- `startup.json` contains `unclean_starts: 0` and
  `last_start_at: 1793017741709`; file modified **19:29:01 +07**, matching first
  Quit. `StartupGuard::disarm()` writes this at the end of the clean-shutdown
  routine, after collector/checkpoint stop and DB flush. This supports actual
  Quit execution; it is not merely a failure to identify the menu.
- `GetGUIThreadInfo` now returns flags 0 and menu owner 0 for main thread 17348;
  no `#32768` popup remains. The modal menu itself has been cancelled.
- PID remains responsive, CPU unchanged at 161.609375 seconds across snapshots;
  threads fall from 23 to 21. Surviving named threads: `main`, `tracing-appender`,
  `db-writer`, `db-maintenance`, `event-bus`, `windows-resume-monitor`, and Tokio
  workers. No named collector/checkpoint worker remains. This is consistent with
  completed application shutdown work, not proof that every resource is closed.
- Windows remain: `tray_icon_app` HWND 66898, hidden `Tauri Window` HWND 132262,
  single-instance `dev.whatchanged.app-sic` HWND 132356, and
  `Tao Thread Event Target` HWND 132360. All belong to PID 17316/thread 17348.
  Handles are snapshot evidence, never reusable without owner/class validation.
- `.window-state.json` timestamp remains **15:01:49 +07**. The registered
  window-state plugin saves on `RunEvent::Exit`.
- Read-only Windows Wait Chain Traversal for main, writer and maintenance threads
  reports each blocked with one thread node, **no cycle**, no owner chain.
  It does not resolve the blocking call or exclude a Rust synchronization wait.

Source path:

1. Tray Quit calls `state.shutdown_cleanly()` then `app.exit(0)`.
2. `tauri-2.11.5/src/app.rs::AppHandle::exit()` requests runtime exit; it does not
   synchronously force process termination when the request is accepted.
3. `tauri-runtime-wry-2.11.4` queues `Message::RequestExit`, emits
   `ExitRequested`, and sets `ControlFlow::Exit` unless prevented. This app's
   source contains no `prevent_exit()` hook.
4. The single-instance plugin is registered **first**. Its `RunEvent::Exit`
   handler destroys the single-instance HWND, before window-state save and before
   this application's second `shutdown_cleanly()` callback.

Because the single-instance HWND still exists, the strongest current inference
is that `RunEvent::Exit` has **not reached plugin dispatch**. A second-shutdown
deadlock is therefore unsupported by the observed ordering. Current shutdown
helpers are also idempotent at collector stop/automation take; the marker has its
own once guard. Exact source-to-running-binary identity was not re-established,
so this is a source-supported diagnosis, not a captured native stack proof.

Bounded remedy proposed to controller, **not executed by scout**: validate PID,
start time and exact `Tao Thread Event Target` class, then request one
`RedrawWindow(hwnd, null, null, RDW_INTERNALPAINT /* 2 */)` and wait at most 30 s
for process exit. In pinned `tao-0.35.3/src/platform_impl/windows/event_loop.rs`,
the target's `WM_PAINT` handler explicitly completes modal-loop event processing,
clears `handling_events`, and processes control flow. The outer loop exits only
when control flow is Exit and no events remain in progress. This request tests a
stalled modal paint/exit transition without supplying command payloads or killing
the process. If unchanged, stop automated message retries and retain the snapshot;
the original app-data folder must remain in place while the process still lives.

Do **not** forge the registered `Tao::WakeupMsg`: its handler calls blocking
`user_event_receiver.recv()` and requires a matching in-process channel payload.
Sending that message alone could introduce a fresh hang. Do not infer that a
clean marker alone makes moving a still-open database safe.

Unresolved: exact native exit-stage cause; result of the single bounded redraw
probe if controller chooses it; a safe completed process exit before offline
whole-directory backup and disposable-profile swap.

## Controller execution result

The single validated internal-paint request completed PID 17316's exit. Offline
backups were verified; directory isolation subsequently failed before any rename.
The existing-profile fallback then exposed a raw-Cargo executable attempting the
Vite development URL. The corrected embedded standalone executable passed seven
native routes, real backend status, search focus/Escape and workspace/Privacy
visibility with no Vite server. Both subsequent real tray Quit attempts completed
normally; QA PID 4216 exited 0 with no forced termination or internal paint.

The verified standalone binary was copied to the stopped local monitoring path
after preserving the preceding executable. Monitoring was restored as PID 27156
with `--minimized`, without the QA debugging environment. The original profile was
never moved, deleted or restored over newer data. Exact artifact identity, local
evidence paths and acceptance limits are in
[continuation evidence](qa-261026-continuation.md#standalone-native-ui--completed-continuation).
Fresh-profile, installed-candidate and full manual accessibility work remain open.
