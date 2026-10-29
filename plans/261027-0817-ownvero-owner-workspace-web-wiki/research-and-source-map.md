# Planning evidence and source map

Observed 2026-10-27; source at main, HEAD 13d60df4c98fd50532c3d4a338fd6eb7be6c0db8,
with 135 pre-existing dirty entries. Findings describe current working tree, not only HEAD.
No app tests/build/native QA ran in this planning turn. Two read-only scouts supplied
backend and UI/site maps; controller checked core identity/schema/route constraints.

## Repository instructions and environment

README and supplied AGENTS instructions were read. Local AGENTS.md and referenced
C:/Users/Salyyy/.Codex/rules files were absent. Matching primary/development/orchestration/
documentation rules were found and read at C:/Users/Salyyy/.claude/rules.
No secrets, dotenv or personal app database needed.

Existing monitoring process PID 21820 and its WebView children observed; left untouched.
No dev server/runner created. D: free 2,211,692,544 bytes at planning check, not a build budget.
CLI generated directory uses UTC 0817; local time approximately 15:17 Asia/Saigon.
Do not rename generated folder to match an earlier suggested timestamp.

## Existing owners (verified, not proposed)

| Path relative to root | Evidence / reusable boundary |
|---|---|
| README.md:49 | No-network, one-process promises need explicit update, not assumption |
| src-tauri/tauri.conf.json:3 | Display name/binary/identifier separate; dev.whatchanged.app retained |
| src-tauri/src/storage/migrations.rs:39 | Legacy migrations 001–016, checksummed ledger |
| src-tauri/src/storage/db.rs:324 | Writer boundary; read pool at 338; startup at 636 |
| src-tauri/src/storage/writer.rs:311 | Bounded writer/repository precedent; not new business domain |
| src-tauri/src/storage/project_repository.rs:21 | Explicit root registration + global known-good pointer |
| src-tauri/src/domain/ids.rs:181 | Project ID derives from canonical path, unsuitable portable app ID |
| src-tauri/src/commands/settings.rs:205 | Mark working captures machine state, not business verification |
| src-tauri/src/storage/investigations.rs:121 | Atomic sanitized historical snapshot precedent |
| src-tauri/src/privacy/sanitize.rs:75 | Sanitized admission pattern; not image redaction proof |
| src-tauri/src/storage/purge.rs:74 | Legacy table enumeration; new store/artifacts need explicit integration |
| src-tauri/src/app/state.rs:112 | Process lifetime owner; app/lifecycle.rs:40 owns shutdown sequence |
| src-tauri/src/commands/mod.rs:58 | signatures source list; COMMANDS at 151 |
| src-tauri/src/lib.rs:72 | Debug handler; release handler at 143, both must match |
| src-tauri/src/dto/bindings.rs:84 | Explicit DTO export list in addition to dto/mod.rs |
| src-tauri/examples/export_bindings.rs:21 | Generates committed bindings and command argument table |
| src/lib/tauri/client.ts:103 | Typed IPC wrapper boundary; contract tests enumerate surface |
| src-tauri/tests/commands_contract.rs:129 | Registration/wrapper contract gate |
| src-tauri/capabilities/default.json | No generic shell/http/fs/process/updater permission |
| src-tauri/src/platform/windows/tool_probe.rs:238 | Fixed version probe only, not script runner |
| package.json:46 | Playwright development dependency, no shipped runtime |
| scripts/check-no-network.mjs:44 | Rust Windows graph check; cannot inspect child browser network |
| src/app/routes.ts:10 | Route union; exact hash paths, no parameterized parser |
| src/app/screens.ts:22 | Route-to-screen component owner |
| src/lib/stores/router.svelte.ts:20 | Listener lifecycle, exact path lookup |
| src/app/App.svelte:27 | Theme/router/backend subscription; recovery/onboarding boot gates |
| src/app/AppShell.svelte:24 | Shell plus legacy drawer/search/health integrations |
| src/app/Sidebar.svelte:27 | Responsive rail and accessible route labels |
| src/components/system/SystemMapNodeList.svelte:14 | Scoped IDs, keyboard list, active scroll/shrink reconciliation |
| src/components/system/SystemMapFrame.svelte:35 | Renderer-independent summary/list, Escape/focus/fallback |
| src/three/shared/renderer.ts:10 | Reusable renderer construction and DPR cap |
| src/three/shared/dispose.ts:5 | Geometry/material/texture disposal and renderer teardown |
| src/three/systemMap/scene.ts:110 | Existing loop lifecycle; new idle scene should demand-render |
| src/three/systemMap/types.ts:3 | Machine entity/edge types; do not reuse as product graph |
| src/three/systemMap/scene.lifecycle.test.ts:71 | Lifecycle/reduced-motion/context-loss test precedent |
| docs/deployment-guide.md:76 | Explicit public repo/Pages source-preview route |
| scripts/release.ps1:135 | Installer/canonical copy/hash, no approved URL manifest |
| assets/brand/README.md:3 | Canonical master/exported asset pipeline |
| scripts/check-brand-assets.mjs:15 | Generated/hash/palette checks need planned brand update |

Source-first discrepancy: ARCHITECTURE.md still has outdated table/migration counts.
Do not let a stale description override migration registry. Update owning docs at P02.

## Web source reconciliation

Local wiki-site/ and .github/workflows/ absent; not an empty new project.
Historical [hosted wiki verification](../reports/qa-261027-hosted-wiki-verification.md)
records remote wiki-site/app.js from commit 9f8c0df94118287d989b1b253f619050aa212543.
Known destination: SalyyS1/what-changed and https://salyys1.github.io/what-changed/.
No fresh remote fetch was performed by scouts; P01 must re-check metadata/tree.
Source-publication approval is not binary release approval.

## Official research checked in planning

- [Tauri 2 sidecar distribution](https://v2.tauri.app/develop/sidecar/): external binaries
  need configured names and target-specific artifacts. Does not solve runtime sandboxing.
- [Tauri shell permissions](https://v2.tauri.app/reference/javascript/shell/): executable/
  argument scopes exist; plan deliberately avoids generic frontend shell authority.
- [Playwright authentication](https://playwright.dev/docs/auth): stored auth can impersonate
  accounts; do not ship/commit/export it.
- [Playwright browser contexts](https://playwright.dev/docs/browser-contexts): test-state
  isolation is not backend-data or operating-system isolation.
- [Playwright browser versions](https://playwright.dev/docs/browsers): pinned tooling must
  carry matched browser binaries; dev installation is not product distribution.
- [Playwright network](https://playwright.dev/docs/network): request handling exists but
  requires coverage beyond a single page route; security contract must test boundaries.
- [Three renderer disposal](https://threejs.org/docs/pages/WebGLRenderer): explicit GPU
  cleanup required. Attempt to load manual/en/cleanup.html returned 404; not used as evidence.
- [GitHub Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site):
  static files/custom build workflow supported, not a hosted backend.

Current web docs may describe newer plugin API than locally pinned crate. P01/P07
must verify exact version API/Windows support before coding; no automatic dependency bump.
Competitive research from brainstorm establishes browser tests/rollback already offered,
not Ownvero uniqueness or market demand. No new demand claims introduced by this plan.

## Proposed owners are not existing code

owner_workspace Rust module/database, preview_runner module, owner-graph TS,
owner-map components/3D scene, runner package and owner docs do not exist yet.
Phase tables mark Create versus Modify; remote site paths stay reconcile-first.
Any new test command in a phase is a future acceptance gate, not a command already run.
