# Domain và evidence contract

Status: designed, chưa có trong source. P03 khóa DTO/schema thực và tái đối chiếu
mọi consumer; tên dưới đây là proposed, không phải symbol đã tồn tại.

## Bounded context và identity

Database mới đề xuất: owner-workspace.sqlite, trong AppPaths được resolve bằng OS.
Không sửa checksum migrations 001–016 của legacy; schema mới có ledger riêng,
được cấp số tại owner workspace storage module. Không mở thêm database kiểu ad-hoc
từ từng command; một lifecycle owner, bounded single writer và transactional repositories.
P02 quyết định reuse cơ chế Db ở mức primitive, không copy nguyên event schema/maintenance.

| Record | Field/contract chính | Lifetime |
|---|---|---|
| ProductApp | UUID, display name, optional legacy project ID, current candidate, archived_at | Workspace |
| TestEnvironment | app ID, exact origins/ports, preview URL, type test/local, version, consent record | Versioned configuration |
| Capability | app ID, owner label/description, order, archived_at | Workspace |
| BehaviorRuleRevision | rule ID + version, expected behavior, provenance, confirmed_by/at, supersedes | Immutable revision |
| ChangeRequest | app ID, intent, example, preserved rule revisions, scope, candidate ID, state | One change |
| ScenarioRevision | rule revisions, typed steps/assertions, fixture/role refs, method, owner approval | Immutable revision |
| CandidateRevision | identity value + source, trust kind, observed_at, environment/version | One tested candidate |
| VerificationRun | candidate/scenario manifest digests, environment revision, runner version, start/end, lifecycle | One execution |
| ScenarioResult | run/scenario revision, assertion outcomes, expected/observed summaries, method, limitations | Immutable committed result |
| EvidenceArtifact | result ID, relative content-addressed path, digest, size, retention/visibility, capture consent | Bounded artifact |
| OwnerDecision | change/candidate/manifest, accepted/rejected/deferred, exceptions, reason, timestamp | Immutable decision |
| ProjectDecision | app ID, text, rationale, provenance, supersedes | Versioned project memory |

Do not use legacy path-hash project ID as portable app identity. Import remaps local
links after explicit mapping; no local path, credential or consent is portable.
Every operation checks app ownership of all referenced IDs inside transaction.
Unique key on operation ID prevents double-click duplicate runs/acceptances.
Optimistic expected-version protects concurrent edits; conflict is visible, not last-write-wins.

## Trạng thái không được trộn

Run lifecycle: queued → running → completed | cancelled | interrupted.
Result: passed | failed | blocked | not_run. Freshness: current | stale | unknown.
Verification method: automated | owner_manual | imported_untrusted.
Owner decision: pending | accepted | accepted_with_exceptions | rejected | deferred.
These are separate axes, not one overloaded green/yellow/red flag.

- Mixed results show counts AND each scenario; failed dominates a summary badge.
- blocked/not_run never become passed; cancelling does not relabel completed observations.
- Owner can record accepted_with_exceptions only after seeing each unresolved scenario and
  writing a reason. Original failures/gaps remain visible; no “all verified” status.
- Manual checks state who observed what; do not count them as automated verification.
- Imported results remain historical/untrusted provenance until run locally. Digests
  detect accidental corruption, not independent truth or cryptographic attestation.
- Changing intent, rule, scenario, assertion, locator, environment configuration,
  fixture schema, candidate identity or runner semantics invalidates the relevant manifest.
- Edit while run active: run uses frozen revisions; late result cannot overwrite the new
  active candidate, re-enable acceptance or enter a different workspace's current view.
- Current is re-evaluated on read/export/accept, not only when a UI event fires.

## Identity and evidence strength

Build identity is not a URL or test timestamp. Supported strongest route:
explicit deploy build ID bound to environment; or app-owned revision marker matched
before AND after the run. Marker changes mid-run → interrupted/stale, no durable acceptance.
Git commit can describe selected source; it does not prove a hosted deployment matches it.
Untracked local changes also prevent “commit-only equals build” inference.
Owner-entered labels are self-reported. Mutable preview with no reliable marker may
still produce a timestamped observation, but cannot claim current-build verification.

Result stores expected vs observed, executed action/assertion IDs, sanitized failure reason,
scope/limitations and only explicitly approved artifacts. An empty successful process exit,
LLM statement, toast screenshot or absence of console errors is not a passed scenario.

## Rule protection

1. Draft can be authored manually, from template or imported from an external AI.
2. UI compares draft to current confirmed expected behavior, not merely text similarity.
3. Owner confirms revision before execution; app freezes scenario/criteria manifest.
4. Runner cannot write rules, scenarios or owner decisions.
5. Locator repair creates a proposal with before/after and invalidates prior association.
   It cannot remove assertions, change expected values, ignore a failure or auto-approve.
6. Retry creates a separate attempt linked to the same manifest. All attempts remain
   visible; no automatic retry loop until green and no hidden best-run selection.

## Proposed command families

All commands typed, validated, bounded and async when storage/process is touched.
Use existing error DTO style; commands delegate to services, no business logic in IPC.

| Module (new) | Proposed command names | Main consumers (new unless noted) |
|---|---|---|
| commands/owner_workspace.rs | list_owner_apps, create_owner_app, get_owner_app, update_owner_app, archive_owner_app | owner-workspace store, MyAppsScreen, AppOverviewScreen |
| commands/owner_workspace.rs (split by concern if large) | list_owner_capabilities, save_owner_capability, archive_owner_capability, list_rule_revisions | CapabilityEditor, AppOverviewScreen, PreserveRulesStep |
| commands/owner_workspace.rs (configuration only) | list_test_environments, save_test_environment, remove_test_environment, list_candidate_revisions, select_candidate_revision | AppOverviewScreen, RunConsent, RunReviewScreen |
| commands/owner_changes.rs | save_change_draft, confirm_rule_revision, confirm_scenario_revision, get_change_review | change-request store/screens, evidence-review screen |
| commands/owner_evidence.rs | list_verification_runs, get_verification_run, record_manual_result, record_owner_decision | evidence store, review/map selectors |
| commands/owner_portability.rs | preview_owner_export, export_owner_bundle, validate_owner_import, import_owner_bundle | memory/export/import screens |
| commands/preview_runner.rs | inspect_runner, authorize_test_run, start_test_run, cancel_test_run, revoke_environment_consent | runner store, consent/run dialogs |

Contract wiring: commands/mod.rs sources + COMMANDS, both lib.rs handlers, DTO module
and ts-rs export, examples/export_bindings.rs, generated bindings.ts, client.ts,
commands_contract/wiring/capability tests. Exact count verified in P03, not guessed.
Existing legacy commands remain compatible. No generic “run command”, URL fetch,
file read, SQL, eval or arbitrary script command.

P03 implements capability/environment/candidate configuration and change/rule CRUD/read
models plus scenario/recipe v1 schema and storage. Configuration APIs never connect
to a preview. P06 implements scenario authoring/templates/approval/evidence behavior.
P05 can export no scenarios with an explicit empty state; it cannot fabricate them.
After P06/P07, rerun the same portability gate with real scenario/result/exported recipe.

## Storage, deletion and export

- Metadata suggested bounds: 20 apps, 200 capabilities/app, 500 active rules/app,
  100 scenarios/app, 50 steps/scenario, 16 KiB per text field. Record exact limits in schema
  and friendly errors. These are design budgets to benchmark, not measured capacities.
- Pagination stable by ID/time; newest result index by app/candidate/scenario version.
- Default artifact retention 30 days with 500 MiB workspace cap, configurable within
  documented limit. Preserve metadata and explicit “artifact expired” state.
- Artifacts outside SQL; write temp → sanitize/validate → hash → atomic promote;
  DB transaction links only finalized artifact. Startup sweeps only owned temp/orphan files.
- Corrupt/full disk: fail closed, preserve last committed data; no optimistic success.
- Full privacy purge must include both DBs, artifacts, temp runner profiles and cached
  exports owned by app. Cancel/drain runner before purge so late results cannot recreate data.
- Global lifecycle admission barrier covers every producer, not only runner: edits,
  imports, manual results, decisions, exports and retention. Each operation captures a
  durable purge epoch; final commit/promotion rechecks it. Purge closes admission,
  advances epoch and drains/cancels work before replacing stores; stale commits rejected.
  Resume pending purge before normal DB migrations/maintenance/collectors/run admission.
- Owned-data inventory also includes live DB/WAL/SHM, migration backups, recovery/
  quarantine/repair files (legacy events.corrupt-* included), failed replacement files
  and app-owned bundled-backup manifests. Global purge succeeds only after all known
  owned copies are removed; deletion errors stay visible/retriable. Never sweep broad
  user directories or delete external user-selected exports/source projects.
- Restore uses a staged validated bundle + durable restore journal/generation: quiesce
  both stores/producers, validate hashes/schema/artifact references, preserve current
  generation, switch all managed roots, verify then reopen. Crash resumes/rolls back to
  one consistent generation; never mix old DB with new artifact inventory. P03 proves
  each interruption boundary with fixtures before any real profile restoration.
- Workspace removal confirmation shows retention choices; export first is offered,
  not forced. Never delete source project, test target data or unrelated OS browser cache.
- Portable bundle: versioned JSON + readable Markdown + supported test recipe export,
  file allowlist, relative paths, SHA-256 inventory, no auth/session/consent. ZIP import
  must reject traversal, absolute paths, links, duplicates, oversized entries/expansion.
- Windows import normalizes both separators; rejects drive/UNC/extended prefixes,
  ADS colon streams, device names (CON/NUL/etc.), trailing dots/spaces, case-insensitive
  normalized collisions and every hard-link/reparse/symlink entry. Extract only into
  newly created app-owned staging; cap count/bytes/ratio while streaming, verify final
  handle/path containment before promote. Tests cover each alias and attempted escape.
- Atomic import into new IDs by default; no silent overwrite. Preview conflict/unsupported
  version before write; never execute imported tests during import.

## Evidence examples

Booking persistence: create uniquely tagged booking in approved test fixture, then
read same ID in a separate staff test session through configured route; compare fields.
If staff session/API setup missing → blocked, not passed from customer success toast.

Access isolation: named negative test using second authorized role against named backend
route/record; assert denied/not disclosed. UI absence alone → UI-only observation,
backend isolation remains not tested. Never generalize one route to “secure app”.
