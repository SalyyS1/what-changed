# P04 owner workspace UI/store report — 2026-10-27

Status: DONE_WITH_CONCERNS

Implemented owned files:
- `src/features/owner-workspace/`: `MyAppsScreen`, `AppOverviewScreen`, `CapabilityEditor`, `RuleEditor`, `PreviewSetup`, `CandidateSetup`, synthetic fixtures.
- `src/features/change-request/`: `ChangeRequestScreen`, `PreserveRulesStep`, `ChangeBriefReview`.
- `src/lib/stores/owner-workspace.svelte.ts` and `change-request.svelte.ts` with dependency injection, UUID operation IDs, expected versions, page loading, app/generation stale response guards, and conflict-preserving drafts.
- Focused store tests cover app switching during load/save, create/archive versions, rule provenance/revisions, page cursors, request save/reopen, successive changes, stale rule revisions, conflicts, cross-app responses, edits during save, and load failure isolation.

Additional route integration requested by root:
- `src/app/routes.ts`: `/apps`, `/apps/:appId`, `/apps/:appId/change[/changeId]`.
- `src/lib/stores/router.svelte.ts`: validated app/change params, route-aware hrefs, navigation guard.
- `src/app/screens.ts`, `src/app/App.svelte`, `src/app/Sidebar.svelte`: owner screens and unsaved draft save/keep/discard navigation dialog.
- Owner routes bypass legacy monitoring/recovery/onboarding blockers; their own storage failure remains visible. Optional legacy project linking reads existing project metadata only and never enables collectors.
- Dedicated `OwnerWorkspaceStatesHarness` uses `OwnerWorkspaceFixture`, rejecting API dependencies and inactive screens. My apps exposes the same explicitly synthetic read-only booking example; it does not share production store state.

Expected root client wrappers:
`listOwnerApps`, `saveOwnerApp`, `getOwnerApp`, `saveOwnerCapability`, `listOwnerCapabilities`, `confirmOwnerRule`, `listOwnerRuleRevisions`, `saveOwnerChangeDraft`, `getOwnerChangeDraft`, `listOwnerChangeDrafts`, `getOwnerChangeReview`, `saveOwnerEnvironment`, `listOwnerEnvironments`, `saveOwnerCandidate`, `listOwnerCandidates`, `selectOwnerCandidate`; signatures sent to root in collaboration message.

Validation:
- `bun run check --silent` latest completed run failed while root bindings were pending. An earlier command returned a still-running PTY session; it was initially misreported as passed, then corrected to root. Frontend-only numeric maxlength, optional props and fixture route typing issues were repaired. Root owns the final type gate.
- `bunx vitest run src/lib/stores/owner-workspace.test.ts --maxWorkers=1`: 5 passed.
- `bunx vitest run src/lib/stores/change-request.test.ts --maxWorkers=1`: 7 passed.
- Combined Vitest default worker mode was attempted; Vitest worker startup timed out on this machine (owner file passed; rerunning each file with `--maxWorkers=1` passed). No Cargo run.
- `bunx vitest run src/lib/stores/owner-brief.test.ts src/features/project-memory/export-review.test.ts src/app/App.test.ts src/app/App.onboarding-reload.test.ts src/app/route-prominence.test.ts --maxWorkers=1`: actual exit 0, 5 files / 23 tests passed.
- `bunx vitest run src/lib/stores/router.test.ts src/features/change-request/change-request.test.ts --maxWorkers=1`: actual exit 0, 2 files / 6 tests passed.

Concerns / integration notes:
- Root is generating bindings and client wrappers; rerun `bun run check` and focused tests after those land.
- Owner params are URL-decoded then restricted to bounded identifier characters; unknown or malformed paths resolve Home. Router tests cover explicit and hash-change guards, deep links and unsafe segments.
- Verify root's generated enum spelling matches frozen serde values (`owner`, `test`, `owner_label`, `self_reported`, `draft`, `ready_for_review`).
- `ChangeRequestStore.load` uses `getOwnerChangeReview` for existing drafts and `getOwnerApp` for new drafts; it loads all rule pages for exact revision reuse.
App.svelte now passes onDirtyChange and keeps a route guard flag in addition to the singleton store.

## P05 brief review UI

- New `src/features/project-memory/ExportReview.svelte` + component tests, new `src/lib/stores/owner-brief.svelte.ts` + store tests.
- Inline entry after a saved ready brief in ChangeRequestScreen. Select optional sections, prepare backend-rendered preview, inspect exact plain-text Markdown, bytes/token estimate, omissions and limitations, explicitly acknowledge review, then copy with `navigator.clipboard.writeText`.
- Never reads clipboard or renders raw HTML. Selection, dirty request, app/request/version change invalidates preview. Late preview/copy feedback discarded.
- Preview result app ID, request ID and version must equal open saved request; mismatch clears preview and requests reopening the saved revision. Root added those identity fields to DTO; final regeneration still required at this report update.
- No browser/900×600/zoom/forced-colors run performed in this delegated task; no server or target network call started.
- Synthetic 900×600 state inspection is available in the dev harness, not claimed as a completed browser acceptance run.

## P06 manual acceptance and evidence UI — 2026-10-28

- Added `src/features/acceptance/` scenario templates/editors, readable exact recipe summaries, revision comparison/confirmation, manual observation editor, run review and owner decision dialog.
- Added `src/lib/stores/owner-evidence.svelte.ts` with app/load/run generation guards, pagination, expected versions and immutable retry/history display. Backend owns status/counts/freshness, gaps and decision eligibility.
- Explicit candidate binding saves the selected candidate to the change before starting a frozen manual manifest. Every assertion starts `not_run`; no bulk-pass action. Observer, observation time, step observations and exact expected/observed checks are visible.
- Manual evidence remains unknown freshness. Ordinary acceptance stays disabled. Explicit exceptions require a reason, unknown-build acknowledgment and acknowledgment of every unresolved scenario. Failures and gaps remain unchanged. Decision-time/current freshness are separate.
- App overview mounts the acceptance panel only in active production mode. Synthetic evidence states use rejecting/inactive dependencies; they do not contact targets.
- Corrected component test interaction to match installed bits-ui: Select trigger is a button, opens via keyboard, item selection uses pointer-up.

## P05 project memory and bundle UI — 2026-10-28

- Added `ProjectMemoryScreen.svelte`, `MemoryDecisionEditor.svelte`, `BundleExportReview.svelte`, `ImportReview.svelte` and feature-local reactive models under `src/features/project-memory/`.
- Project decisions load all pages, retain immutable revisions, filter latest entries by capability/request/source, show rationale/author/provenance/time and preserve drafts on conflicts. Supersession compares saved/proposed wording and uses exact expected revision.
- Export target details opt in. Selection/app changes invalidate previews and review. Owner reviews literal Markdown, digest, bytes, rules, omissions and limitations, then native export receives the exact reviewed digest. Existing filename errors request another filename; cancellation is not reported as success.
- Import validates through the native picker before any write. Review shows new app ID/name, digest, conflicts and limitations. Author, general review and individual confirmation of every historical rule revision are required. No bulk confirmation. Apply uses exact review ID/digest/rule references and verifies returned app identity.
- Imported evidence/scenarios remain untrusted archive. Collapsed archive view loads only on explicit request and renders literal JSON; it does not turn imported passed history into current verification or restore targets/consent.
- Integration: App overview mounts project memory after acceptance only when active; archived apps are read-only. My apps mounts import only when active and reloads app rows after successful import, with explicit open-imported-app action.
- Existing change brief `ExportReview.svelte` retained. All Rust DTOs imported from generated bindings; root owns Rust, client wrappers and binding generation.

## Final verification for P05/P06 UI

Actual completed commands, exit 0:

- `bun run check`: Svelte 0 errors / 0 warnings, TypeScript passed. Supersedes earlier pending-generation notes above.
- `bunx vitest run src/features/project-memory/project-memory-model.test.ts src/features/project-memory/bundle-review.test.ts src/features/project-memory/project-memory.test.ts src/lib/stores/owner-evidence.test.ts src/features/acceptance/scenario-editor.test.ts src/features/acceptance/run-review.test.ts --maxWorkers=1`: 6 files / 21 tests passed.
- `bunx vitest run src/app/App.test.ts --maxWorkers=1`: 1 file / 11 tests passed.

No Cargo, network, browser, dev server or subagent started in this delegated work. Test/type processes exited. No files committed.

Remaining manual evidence: native import/export dialog walkthrough, real archive round trip with populated evidence (backend gate owned by root), visual/900×600/200% zoom/forced-colors/reduced-motion/keyboard acceptance. These are unverified, not claimed complete. Public rename/publication gates remain outside this UI delivery.

Unresolved questions: none for implemented UI contracts.

## Browser QA and follow-up corrections — 2026-10-28

See [synthetic browser QA report](qa-261028-0215-owner-workspace-browser.md): 32 broad browser
checks plus 4 focus/fixture follow-up checks passed; screenshots and exact JSON evidence retained.
This supersedes earlier browser-unverified notes only for the report's measured synthetic scope.
Native dialogs/storage, native 200% zoom and full accessibility acceptance remain open.

Follow-up UI corrections: contrast-preserving shared focus token, saved synthetic request load
baseline, exact selected-candidate environment origins, deduplicated origin options, route-purpose
and expected-check compatibility feedback, assertion limits, and preservation/authoring of API
origins in PreviewSetup. Actual focused Vitest exit 0: six files / 23 tests. Root subsequently
reported final check/type exit 0 (0 errors / 0 warnings) and production build exit 0.
