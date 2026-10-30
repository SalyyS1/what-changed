# Repaired installed candidate — 2026-10-27

Status: partial installed verification; release remains **NO-GO**. This follows
the [earlier continuation](qa-261027-0434-mvp-continuation.md). Its artifact
predated the workspace repair; the candidate below includes that repair.

## Artifact identity

Unsigned version 0.1.0, dirty worktree based on commit `13d60df4`.
The frozen build-input inventory is private
`.qa/installer-source-inputs-20261027-workspace-fix-final.json`. Later source
changes require their own build and acceptance; this is not a clean-tree release.

| Artifact | SHA-256 |
|---|---|
| Raw NSIS installer and byte-identical stable-name copy, 4,524,798 bytes | `0f51687581aed0d74628f662ffc8f6ae494d637ef6e9072480b27406fcda24d9` |
| Generic build executable, 15,392,768 bytes | `8cd8332b3611e8b7426edd9ed01790e432c9f9770041387d5a0aec609b29a601` |
| Derived and verified installed NSIS executable | `4fd23b909d1610ee03f4ff050e5c642823302fb8150347d4193d52133e99bb7b` |
| Third-party notices, 453,903 bytes | `aea1d355d6864858959b2a9e4c8ee628acd56718d1e6a8b2620961f344f8c6a5` |

The generic and installed executable hashes differ at the three expected NSIS
bundle-marker bytes. They are different artifacts. `verify-installer.ps1`, notice
copy verification, physical payload checks and source-input freshness checks
passed before the installed app launched. The normal Tauri build completed;
this was not the full, unskipped `release.ps1` gate.

Private provenance: `.qa/installer-artifacts-20261027-ui3.json`,
`.qa/native-ui3-physical-artifact-proof.json`,
`.qa/installer-build-20261027-workspace-fix.log`, and
`.qa/verify-installer-20261027-workspace-fix.log`.

## Installed observations

The controller backed up and isolated the existing profile/WebView data, quit
the original monitor normally, and silently reinstalled the candidate.
Installer exit was zero and installed payload/notices hashes matched.
This does not establish wizard/UAC behavior, a clean-account install,
version-over-version upgrade, optional shortcut behavior or uninstall.

| Check | Observed result and scope |
|---|---|
| Launch/onboarding | Native window appeared after 1,119.734 ms; isolated-profile onboarding completed, including the native folder picker. This is not a cold/warm startup matrix |
| Workspace | Explicit project registration and Mark working saved a linked baseline checkpoint |
| Scoped investigation | Four recorded events were analyzed instead of the previous zero-event defect. The result was `only_routine_changes`, with zero suspects. This proves membership repair, not successful diagnosis or ranking of configuration changes |
| Global investigation | A separate real loopback-listener fixture produced a ranked result with the named port opening first and a recorded four-step structural chain. No configuration-to-port causal claim is established |
| AI pack | Confirmation and copy feedback observed. Native clipboard observer matched the pack, normalizing native line endings, then restored the six original clipboard formats |
| Checkpoint comparison | Compare to now showed additions, removals and changes. The UI disclosed that Windows event log comparison was unavailable |
| Updater opt-in | Check now was disabled before opt-in. After explicit opt-in it reported an unconfigured feed and no request. This does not establish a configured signed updater or ten-minute runtime network observation |
| Diagnostics | Save dialog wrote a 1,823-byte report. All five specified fixture/path/account/value absence checks passed |
| Scoped privacy purge | With monitoring paused through Settings, deleting configuration history removed that category while preserving every other category count and checkpoint count. Monitoring resumed successfully. This is a logical UI check, not an on-disk FTS/WAL/SHM byte-erasure test |
| Navigation/layout | All seven primary routes rendered without page errors or document horizontal overflow at a CDP-emulated 900×600 viewport and DPR 1.5. Change Map also had no document overflow. Native Windows scaling/zoom and every control's usability remain open |
| Change Map fallback | Explicit 2D preference persisted and rendered nontransparent pixels. A separate Automatic preference plus reduced-motion media rendered a positive-size 2D canvas. These do not measure renderer loop/frame timing |

Private evidence is under `.qa/native-installed-ui-20261027-ui3/`:
`investigate.json`, `global-investigation-followup.json`,
`explicit-2d-followup.json`, `reduced-motion-idle-preparation.json`,
`product-followup.json`, `privacy-keyboard-followup.json`,
`diagnostics-export-scan.json`; clipboard outcome is
`.qa/native-clipboard-result-20261027-ui3.json`.
The earlier `map-followup.json` timed out while waiting for an SVG; the actual
2D renderer uses canvas. That failed harness result is retained separately.
None of these screenshots or profile/clipboard data are public launch media.

## Accessibility finding and source follow-up

Tab sampling reached 18 controls per primary route. The System Map node-list
listbox had no outline or box shadow while adjacent controls had solid 2 px
outlines. Source explicitly set `outline-none` on that focusable listbox,
overriding the shared ring. Advisory review approved removing that override.
The initial removal restored a computed ring, but a real browser confirmed
its upper stroke was clipped by the 180 px scroll wrapper. Moving listbox
semantics to that wrapper then exposed an active option hidden after PageDown.
The source follow-up moves focus to the scroll viewport and minimally scrolls
the current option after rendering, only while the listbox is focused.
The fix and browser regression are subsequent source changes; the installer
identified above still contains the finding. Whole keyboard, Narrator,
high-contrast, native DPI and zoom acceptance remain open.

## Resource observations

Command: `scripts/measure-memory.ps1 -SettleSeconds 0`, scoped to the owned
installed process and its six WebView2 descendants. CDP was enabled for QA.

| State | Backend WS | WebView2 WS | Total WS | Private WS |
|---|---:|---:|---:|---:|
| Diagnostics shown | 92.9 MB | 507.6 MB | 600.5 MB | 250.3 MB |
| Hidden after navigation/maps | 99.2 MB | 553.7 MB | 652.9 MB | 276.7 MB |

Both total and private figures are above the 80 MB reference line. Per D-15,
neither is a pass/fail gate. These are engineering snapshots, not the required
three checkpoints on a four-hour acceptance run. Private logs:
`.qa/native-ui3-memory-shown.log` and `.qa/native-ui3-memory-hidden.log`.

## Completion and verification records

The native controller and WMI launcher both reached `complete`. QA exited
normally with code zero, without forced cleanup. Profile restoration, exact
startup Run metadata and shortcut restoration passed; the original monitoring
process restarted and was intentionally retained. The contents journal reached
`Restored`, with no recovery required, failure or cleanup notes. Private records:
`.qa/native-installed-acceptance-session-20261027-ui3.json`,
`.qa/native-profile-contents-20261027-ui3.json`, and
`.qa/native-installed-acceptance-wmi-launcher-20261027-ui3.json`.

The ten-minute CPU diagnostic sampled until its deadline but failed writing
the result: Windows PowerShell's `Set-Content` has no `-NoClobber` parameter.
No samples were recovered and **no CPU acceptance result is claimed**. The
disposable serializer now uses `Out-File -NoClobber`; a PowerShell 5.1 fixture
verified first write/readback and refusal to overwrite existing bytes. This
does not replace the lost CPU measurement or a four-hour soak.

Focused source verification:

- Four affected System Map test files pass **11/11 tests**, including
  arrow/page/Home/End, Space, shrinking node lists, unique two-list identities
  and Escape returning focus to the correct owning frame's Reset button.
- `bun run check` reports zero errors/warnings; production frontend build passes.
- Canonical 10k query-plan shapes pass with all existing assertions retained.
  The corpus has 3,000 entities, 1,500 related events, 800 bursts, 40 checkpoints,
  9,800 snapshot items and 20 investigations over 90 days.
- Actual first/tenth page plan capture passes. Complete checkpoint comparison
  capture passes all five reads and full-result equality, with 3,000 items per
  side. These are plan/parity checks, not latency-budget acceptance.
- Strict Clippy passes for all three plan targets. `scripts/test-perf.ps1` now
  executes the actual page/complete comparison gates. Other plan shapes still
  copy SQL; whole-service/corpus parity remains open.
- The resolved Windows no-HTTP-client gate and final frontend bundle gate pass
  (142,268 gzip bytes excluding Three.js; limit 409,600 bytes).

Private source logs: `.qa/canonical-query-plan-20261027.log`,
`.qa/canonical-page-plans-20261027.log`,
`.qa/canonical-checkpoint-plans-20261027.log`,
`.qa/canonical-plan-clippy-20261027.log`,
`.qa/system-map-same-active-suite-20261027.log`,
`.qa/source-check-same-active-20261027.log`,
`.qa/bundle-final-focus-20261027.json`, and
`.qa/source-build-delivery-focus-20261027.log`.

Two browser runs timed out before navigation committed. An isolated minimal
server probe then passed blank/data/loopback navigation. A Vite diagnostic
reached the new scenario and exposed an ambiguous two-map selector and
ResizeObserver notifications on resizing the multi-map states page. The new
scenario now unmounts that page before resizing and uses the existing isolated
400-node map route. Historical failures remain recorded; no page-error filter
was added. Further browser checks proved clipping and keyboard scrolling defects;
the correction also rounds positive scroll deltas so fractional CSS geometry
cannot leave the selected row outside an integral scroll offset.

The final exact `node scripts/browser-a11y-check.mjs` run **passed** against
the latest source. At 900×600 it observed the shared solid 2 px ring inside
viewport/ancestor clips, active-option visibility after Home/PageDown/End/
PageUp/ArrowUp, recovery when Home is already active after manual scrolling,
unchanged document/main scroll, and Escape returning to that map's Reset.
It also retained all prior navigation/search/modal/reduced-motion checks and
reported no page errors or off-origin requests. The screenshot was visually
inspected with the ring visible. Private evidence:
`.qa/browser-a11y-same-active-final-20261027.log` and
`.qa/browser-system-map-focus-1793059028980.png`.
These are dev-source browser checks; rebuilt installed acceptance remains open.

Installer lifecycle, runtime
network, four-hour soak, full accessibility, final media, clean release and
frozen-manifest human publication approval remain open.

Unresolved questions: none; remaining items require implementation or evidence.
