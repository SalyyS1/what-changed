# Test Report — P02 Design System & App Shell

Gate: `bun run verify` (svelte-check + tsc, Vitest, cargo fmt/clippy/test) — **EXIT=0** after all review fixes.

## Automated

| Suite | Result |
|---|---|
| svelte-check --tsconfig ./tsconfig.json | 0 errors, 0 warnings (793 files) |
| tsc --noEmit | clean |
| Vitest | **89/89 passed** (4 files: contrast, theme-bootstrap parity, state-shell, App shell) |
| cargo fmt --check | clean |
| cargo clippy -D warnings | clean |
| cargo test | 1/1 passed (`no_capability_grants_a_forbidden_permission_family`) + doc-tests |

New tests this phase: 89 frontend assertions incl. WCAG pair table both themes (44 pairs), palette mutation check, tokens.ts↔tokens.css drift, no-raw-hex walk (incl. arbitrary-value fixture), theme-bootstrap key/mode parity (3), StateShell six-state exclusivity, App shell nav/shortcut/aria/theme/titlebar (7), Rust capability forbidden-prefix test.

## Acceptance criteria evidence

| Criterion | Evidence |
|---|---|
| Theme switch repaints w/o reload, no cold-start flash | App.test.ts theme tests; `public/theme-bootstrap.js` runs pre-paint; parity test pins key+modes |
| Contrast test passes both themes, fails on darkening | 44-pair table at 4.5:1; mutation check proves failure on 40% darken |
| No raw hex outside src/styles | fs-walk test over .svelte/.ts/.css incl. `bg-[#hex]` positions |
| Keyboard reach + visible focus | App.test.ts Ctrl+1..7 + titlebar names; Tabs panel ring restored; global `:focus-visible` outline |
| 900×600 no clip/overlap | captures `p02-min-window-900x600-spacing4.png`, `p02-indicator-row4-rail-900x600.png` (rail mode, indicator on row 4 aligned) |
| Dev harness 7×5 states + 900×600 frame | `harnessStates` now incl. `large` (2,000 rows/400 checkpoints deterministic) |
| reduced-motion removes transitions | `--motion-scale: 0` collapses all `--t-*`; loops frozen by the global reduce block (0.001ms + iteration 1) |

## Visual verification (live window, PrintWindow captures)

- `p02-home-dark-spacing4.png` — full window, dark, 4px spacing base
- `p02-min-window-900x600-spacing4.png` — 900×600, rail collapse correct
- `p02-indicator-row4-rail-900x600.png` — Checkpoints active (row 4): sliding indicator flush with the active row in rail mode, token-driven offset correct

## Environment notes

- Bash tool broken this session (wrapper error line 199 before any command); all commands ran through a Claude Terminal PowerShell tab. Long commands: redirect to log + `DONE_MARKER`, then read the file (tab_wait reports idle prematurely; PS 5 `>` writes UTF-16 error-record noise — harmless).
- Dev app (`bun run tauri dev`) crashed once mid-session (exit 255 → black captures); restarted, verified, then **stopped** (`Stop-Process -Name what-changed` → APP_STOPPED). No orphan processes.

## Unresolved questions

None.
