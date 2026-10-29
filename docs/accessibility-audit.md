# Accessibility audit

**Evidence state: structural automation and the headless dev-browser smoke pass;
installed-app Narrator and scale checks remain open.**

| Surface | Keyboard order/focus | Name/role/state | Contrast | Manual status |
|---|---|---|---|---|
| Home | automated component tests | automated landmarks | token tests | pending installed RC |
| Timeline and details | automated component tests | table/drawer semantics | token tests | pending installed RC |
| Investigation | automated component tests | form labels/live state | token tests | pending installed RC |
| Checkpoints and maps | automated component tests | buttons/fallback labels | token tests | pending installed RC |
| Settings/privacy | automated component tests | switch/dialog labels | token tests | pending installed RC |
| Diagnostics | automated component tests | table/status labels | token tests | pending installed RC |

## Automated evidence

The repeatable checks live beside the UI rather than in an external report:

- `src/lib/a11y/contrast-pairs.ts` is the token-name manifest; `contrast.test.ts`
  evaluates every declared pair in both dark and light themes against WCAG AA.
- `reduced-motion.test.ts` checks the app-wide CSS media block and both map
  frame boundaries. The map components receive the same `reducedMotion` flag.
  This is a structural safeguard, not a computed-style or renderer measurement.
- `focus-order.test.ts` checks that each primary screen uses `ScreenFrame`, the
  shell remains the sole `main` landmark across primary screens and is
  keyboard-focusable, and the global `:focus-visible` ring is present. It does
  not simulate Tab traversal.
- `node scripts/browser-a11y-check.mjs` starts an isolated loopback Vite dev
  server and a fresh headless Edge/Chromium profile with background networking
  disabled. It requires the checked-in `#/dev/states` synthetic fixture,
  rejects off-origin page requests and a Tauri bridge, then checks computed
  transition/animation durations under forced
  reduced motion, seven-link Tab/Shift+Tab order and focus rings, Ctrl+K search,
  dialog Tab containment, Escape, and focus restoration. It fails if the local
  port is occupied rather than attaching to an unknown running app.

## Headless dev-browser result — 2026-09-29

Command: `node scripts/browser-a11y-check.mjs` (Playwright with installed
headless Edge/Chromium; local Vite on `127.0.0.1:5183`). The first run found a
real keyboard defect: `Ctrl+K` opened the named search dialog, but the query
field did not receive focus within 10 seconds; the active element remained its
`Close` button. The dialog now supplies an explicit input focus target for its
open-autofocus event. The exact browser command was then rerun and exited **0**.

The passing run rendered Home without a Tauri bridge; traversed all seven
sidebar links in order by Tab and back by Shift+Tab, each with a computed solid
focus outline at least 2 px wide; focused the search field on `Ctrl+K`; kept
three subsequent Tab moves within the modal; and restored focus to the opener
on Escape. On `#/dev/states`, the checked-in synthetic ready-result preview
opened and returned focus on Escape. Forced reduced-motion media produced
near-zero computed sidebar transition and dialog animation durations, with the
dialog animation paused. The browser and local Vite process closed after the
run. Focused frontend tests passed **3 files / 12 tests**, and `bun run check`
reported **0 errors / 0 warnings** after the fix.

The browser check is dev-source evidence, not an installed WebView2 run. It
does not measure either map's continuous renderer loop, every control on every
screen, Narrator output, high contrast, 150% DPI or 200% zoom. Those remain
manual installed-release-candidate work; no automated result is a screen-reader
pass.

## Standalone native navigation evidence — 2026-10-26

The internal standalone build rendered all seven primary routes in its actual
WebView2 window, using the real backend and existing profile, with Vite absent.
Ctrl+1 through Ctrl+7 reached the expected headings/current navigation states;
Ctrl+K focused the search input and Escape closed the dialog. Workspace controls
and the Privacy section were reachable. The observed viewport was 1280×678 with
device pixel ratio 1.5. A separate CDP viewport emulation at 900×600 found no
document/main horizontal overflow on Home, Timeline and Settings.

These are bounded engineering checks, not an installed-candidate keyboard audit,
Narrator test, native-window resize or Windows scaling/200% zoom certification.
The worksheet below remains open. Exact artifact identity and local screenshot
locations are in [the continuation report](../plans/reports/qa-261026-continuation.md#standalone-native-ui--completed-continuation).

## Installed-app worksheet (manual)

### Repaired-candidate sampling — 2026-10-27

The [named installed candidate](../plans/reports/qa-261027-0619-repaired-installed-candidate.md)
rendered all seven primary routes plus Change Map at a CDP-emulated 900×600
viewport/DPR 1.5 without document horizontal overflow or recorded page errors.
Eighteen Tab samples per primary route reached visible elements. Body focus at
the document boundary is not a control-ring pass. The System Map node list had
no outline or box shadow; adjacent controls had solid 2 px outlines. This is an
installed accessibility finding, not a passed keyboard sweep.

The source follow-up moves listbox focus onto the bounded scroll viewport,
restores its shared `:focus-visible` ring and keeps the active option visible
after keyboard navigation. Browser regression verification is recorded in
the candidate report; the named installed artifact predates this fix.
Automatic renderer preference plus reduced-motion emulation produced a
positive-size 2D Change Map canvas. That observation does not measure the
continuous loop, native Windows scaling/zoom, high contrast or Narrator.

The final source browser command passed on 2026-10-27 after reproducing and
fixing the clipping, hidden active-option and Escape defects. The isolated
400-node production map showed a solid 2 px ring inside viewport/ancestor clips
at 900×600. Home/End/PageUp/PageDown/ArrowUp kept the active option visible;
Home also recovered it after manual scrolling without changing the active
index. Page scroll remained unchanged and Escape returned focus to that map's
Reset. All existing browser error, navigation, dialog and reduced-motion checks
remained enabled. Four affected component/screen test files pass 11 tests;
typecheck and production frontend build pass. These source results do not
replace the installed manual worksheet below.

Record the Narrator announcement for the first heading, primary action and one
representative status/table control on each surface. Mark `pass`, `finding`, or
`not run`; include an owner and expiry for every exemption.

| Surface | Narrator name/role/state | Keyboard / Esc | High contrast | 150% DPI / 200% zoom / 900x600 | Owner / expiry |
|---|---|---|---|---|---|
| Home | not run | not run | not run | not run | — |
| Timeline + details drawer | not run | not run | not run | not run | — |
| Investigation | not run | not run | not run | not run | — |
| Checkpoints + compare | not run | not run | not run | not run | — |
| System Map / Change Map | not run | not run | not run | not run | — |
| Privacy / Settings | not run | not run | not run | not run | — |
| Diagnostics | not run | not run | not run | not run | — |

Until every row has an installed release-candidate result, Phase 20's
accessibility criterion stays open. The table intentionally does not infer a
manual pass from the automated tests.

The release gate must record Windows Narrator, high contrast, 150% DPI,
200% zoom and 900×600 findings per screen. “Pending” is not a pass claim.
