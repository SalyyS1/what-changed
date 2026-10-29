# Acceptance editor affordance review

Date: 2026-10-28. Read-only source review against the repaired backend recipe contract.
No code edits, Cargo, browser execution, target requests or personal-data access.

## Outcome

Valid same-origin templates remain authorable. No universal editor dead end found.
One concrete cross-origin configuration blocker and four mismatches can prevent
specific valid recipes or lead owners through a draft that cannot be confirmed/run.

## P2 — Preview settings cannot preserve/configure multiple authorized origins

`src/features/owner-workspace/PreviewSetup.svelte:26` always saves exactly the
preview URL's origin; `:30` does not hydrate existing origins when editing.
Backend `owner_workspace/validation.rs:149` supports 1–16 exact distinct origins,
including the preview origin.

- A frontend on localhost:3000 with a read-back API on localhost:4000 cannot be
  configured correctly through this form. A second environment is not a substitute:
  a run must use origins authorized on its selected candidate's own environment.
- Editing only the label of an existing multi-origin environment silently drops
  the additional API origins. Existing recipes then fail exact-origin validation.

Repair direction: preserve existing origin tuples and allow explicit bounded additions
for the same environment. Do not authorize another environment's origins implicitly.
Test: edit label without origin loss; add/remove a second exact API origin; reject
duplicate/invalid origins; retain the preview origin and the 16-origin cap.

## P2 — Template origin follows list order, not the selected candidate

`OwnerAcceptancePanel.svelte:18` unions latest unrevoked environments. The selected
candidate is available at `:19`, but its environment/revision does not constrain that
list. `ScenarioEditor.svelte:30` chooses `origins[0]` for read-back/access templates.

With candidate B selected and environment A first, the default template describes A.
Confirmation succeeds but freezing the run correctly rejects its foreign origin.
Derive origins from the selected candidate's exact current, unrevoked environment;
missing/stale selection should be explicit. Preserve proposed expectations when a
selection changes; never silently retarget a confirmed or edited recipe.

## P2 — Shared origins create duplicate keyed options

`AdapterEditor.svelte:10` maps every incoming origin to an option keyed by
scheme/host/port. `components/ui/Select.svelte:44` uses `opt.value` as its keyed-each
identity. The panel's cross-environment union does not deduplicate these tuples.
Two distinct environments may legitimately authorize the same origin; storage
configuration validates ownership/revision, not global origin uniqueness.

Opening that route selector produces duplicate keys in the option renderer
(source-traced; no browser reproduction performed here). Deduplicate exact origin
tuples defensively as well as fixing candidate selection. Test two environments
sharing localhost:3000 and a stable selected value after opening the menu.

## P2 — Actions, routes and checks can describe incompatible methods

`RecipeStepEditor.svelte:11` lists all adapters regardless of purpose; `:18–19`
selects the first adapter for either backend action; `:41` exposes every assertion
kind for every action. `AdapterEditor.svelte:24–26` lets roles, fields and route
purpose change independently. Removing a route also leaves existing step references.

Examples: Stored field on Navigate; Denied on a read-only route; deleting a declared
role; removing a selected response field while its assertion remains. The backend
now rejects these, but `ScenarioEditor.svelte:45–48` checks only nonempty headings,
some step and some assertion before presenting confirmation.

Filter route choices by action purpose and offer compatible check kinds. Show clear
per-step prerequisite errors before review, including missing routes/roles/fields.
Do not silently remove or reinterpret an existing assertion when an action changes.
Templates already have matching methods, roles, fields and effects when origin is valid.

## P2 — Check-add controls exceed both supported assertion budgets

`RecipeStepEditor.svelte:48` adds checks without a per-step or aggregate bound.
The backend permits 100 per step and 1,000 across a scenario; the editor review gate
does not count either. Owners can therefore finish a large draft and discover only
at confirmation that it cannot be saved.

Expose both counts/limits and block additions or review with a precise explanation;
never truncate drafted checks. Test 100/101 in one step and 1,000/1,001 across steps.
The existing 50-step and 16-adapter controls already match the backend and need no
scope expansion. Role references remain bounded at 10, fixture references at 50.

## Handoff / verification

Exact findings and expected contract sent to the controller and UI worker. Backend
guards remain authoritative; frontend guidance must not weaken them. UI worker is
handling its authorized acceptance surfaces; PreviewSetup ownership requires controller
coordination. Existing confirmation failure preserves the draft and offers “Keep editing.”

Unresolved questions: none about the verified backend contract. Controller decides
the narrow UI ownership expansion for multi-origin preview settings and records tests.

## Controller verification follow-up

All listed source findings repaired: multi-origin editing preserves configuration,
origins follow the exact selected candidate environment and are deduplicated,
incompatible criteria are retained with review blockers, and assertion budgets are
enforced in authoring. The later focused UI command passed 6 files / 23 tests;
typecheck returned 0 errors/warnings and the production build passed. The original
36-check browser run predates these editor repairs; native and post-repair browser
acceptance remain open. See [browser follow-up](qa-261028-0215-owner-workspace-browser.md)
and [execution](offline-execution.md).
