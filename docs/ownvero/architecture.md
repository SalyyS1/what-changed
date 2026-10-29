# Owner workspace architecture

**Evidence state: available development contracts and source; verification and installed acceptance remain in progress.**

Offline implementation is present in the development branch and under verification.
The existing installer predates this workflow. [Active implementation plan](../../plans/261027-0817-ownvero-owner-workspace-web-wiki/plan.md)
owns progress and measured evidence.

## Product and authority

The workspace preserves the owner's intended behavior across repeated AI-built app
changes. A passing check is an observation; accepting a change is a separate owner
decision. Unknown, blocked, stale, manual and imported evidence retain their labels.
No agent may rewrite a confirmed rule or relax an assertion as a way to obtain a pass.

Windows desktop remains the primary application. Static landing/wiki pages explain
the product and publish only approved downloads; they never receive a Tauri bridge,
store customer projects or run user-supplied previews. Offline workspace functions
must remain useful without a runner, account, cloud service or remote model.

See the [domain and evidence contract](../../plans/261027-0817-ownvero-owner-workspace-web-wiki/domain-evidence-contract.md)
for immutable revision semantics and evidence strength, and the
[supported preview contract](../../plans/261027-0817-ownvero-owner-workspace-web-wiki/supported-preview-contract.md)
for the first booking/CRUD target family.

## Storage boundary

Use `owner-workspace.sqlite` under the same resolved application data root. Keep the
legacy event database and migration checksums intact. Owner app UUIDs are independent
of path-derived troubleshooting project IDs; links are optional and soft. Removing a
source root or old checkpoint must not delete the owner workspace.

The owner store has one bounded writer and separate bounded read access, transactional
repositories and its own checksummed schema ledger. Every record reference is checked
against app ownership inside the transaction. Expected-version conflicts are visible;
operation IDs make retries idempotent without changing immutable history.

Store failure preserves its files and disables affected operations. Healthy legacy
history remains available. No startup path silently resets a corrupt or newer store.
Safe mode leaves all execution disabled. Backend commands validate and delegate;
generated Rust DTOs remain the authority for TypeScript bindings.

## Lifecycle, deletion and restoration

AppState owns both stores and their lifecycle. A global admission barrier covers
every owner mutation and artifact producer, including deferred imports/exports.
Operations carry a durable purge epoch and recheck it at final commit/promotion.
Purge closes admission and durably records intent and the new epoch before
cancelling/draining producers. It clears each owned root, then reopens admission
only when deletion is complete.
Startup resumes pending deletion before migrations, collectors or runner admission.
Completion retains a non-sensitive epoch tombstone; removing it could let delayed
work from an older process generation become admissible again. The logging sink has
its own epoch fence so queued and partially formatted old log events cannot appear
after the deletion barrier. Review previews are cleared before admission reopens.

Owned inventory includes live DB/WAL/SHM, pre-migration backups, repair/quarantine
copies, interrupted replacement files, owner artifacts, temporary profiles and cached
exports. Only explicit app-owned patterns/roots can be removed. External projects and
user-chosen exported files are excluded. Deletion errors remain visible and retriable;
logical empty-table counts alone cannot establish complete removal.

The Rust maintenance API implements SQLite-consistent backups of both closed stores
and a SHA-256 artifact inventory. It validates staged restore generations and journals
the switch; startup resolves an interrupted switch before either store opens.
Opened native handles bind hashing and promotion to the same files. The managed
`data-backups`, `data-restores` and `data-restore.json` namespaces are included in full
privacy deletion; pending deletion and its epoch take precedence over restore.
Isolated native restore regressions passed, including interrupted file promotions.
There is no end-user restore screen or completed installed upgrade/restore acceptance
yet. Restoring an older generation
requires explicit confirmation of the newer work that will be lost and a closed app.

## Offline workflow contracts

Implement app/capability CRUD, immutable confirmed rule revisions, change drafts,
environment configuration and candidate selection before adding screens that need
them. Configuration never contacts a preview. Freeze scenario/typed recipe schema
before portability; empty scenario lists stay explicit until authoring is implemented.

Briefs carry desired outcome, example, preserved rules, expected evidence, unresolved
choices and limitations. Clipboard Markdown is the first handoff channel. Versioned
bundles preserve readable memory and supported recipes without credentials, local
consent or false current-build trust. Imported evidence is historical/untrusted.
The portable ZIP profile contains four stored regular files and is validated in
bounded memory. Archive entries are never extracted to OS paths; compression, links,
extra fields, unsupported names/versions and inconsistent checksums are rejected.
Import creates separate UUIDs. Each imported rule revision requires explicit review
before activation; original scenarios, target details, decisions and evidence remain
readable archival history. They cannot grant execution or approve a current change.
Saving rechecks the reviewed export against a fresh snapshot with the same section
choices. Changed workspace/evidence requires another review. Replacing a selected
file requires explicit confirmation and a complete staged write before replacement.

Manual observations retain observer/time/steps. Owner decisions bind candidate and
manifest. Exceptions require an explicit reason and acknowledgment of unresolved
results. Freshness is recomputed on read, export and acceptance, including candidate,
scenario, rule, environment and runner-semantic changes.
Manual runs freeze the full request and expected criteria. Missing assertions/steps
cannot pass, retries remain separate attempts and observations committed after an
edit stay attached to the old manifest. Configured build labels alone keep freshness
unknown; ordinary current-build acceptance is unavailable for these observations.
Recipes allow at most 50 steps, 100 assertions per step and 1,000 per scenario.
Field comparisons require a read-back action and a read-only adapter with the
selected response field. Denial checks require the other-role action, a denial
adapter and declared roles. Adapter origins must belong to the exact frozen
environment. Malformed historical observations remain unchanged, while review and
export project the affected scenario as blocked; recorded failures still dominate.

See [offline owner workflow](offline-owner-workflow.md) for the current source UI.

## Runner boundary and scheduling

G0/G1 approved packaged Node/Playwright/Chromium with per-run opt-in network permission.
Development source includes the supervisor, typed protocol, recipe interpreter,
isolated role sessions, reviewed screenshots and runnable recipe exports. The
existing installer does not contain or prove this implementation.
The existing core remains network-free. The runner must never run in the bridged
webview, inherit a personal profile, invoke shell/PATH programs or accept arbitrary JS.
Native Job ownership must be established while the child is suspended.

The [runner safety contract](../../plans/261027-0817-ownvero-owner-workspace-web-wiki/runner-safety-contract.md)
owns exact origin/effect restrictions, sensitive profile/artifact handling and failure
behavior. Request interception alone is not a connection-level network boundary.
The host spike proves local feasibility only. The user subsequently authorized
feature implementation before clean-Windows, network-boundary, performance and
participant acceptance. Those checks remain mandatory acceptance evidence. Remote
targets stay unavailable; local previews require literal loopback origins, exact
method/path rules and a fresh one-session grant. Missing runtime returns an
unavailable/interrupted session, never a download or PATH fallback.

The Rust supervisor freezes and rechecks manifest/scope, owns a suspended-then-assigned
Windows Job process tree, validates framed nonce-bound results and persists each
named assertion. The child cannot accept a change. Raw trace/HAR/video are disabled.
Leaving a workspace transfers revocation to an app-level recovery owner. Failed or
unconfirmed cancellation retains the exact session identity and a retry control
across routes until matching native terminal status is confirmed. Root teardown
stops frontend retry timers; native Job ownership governs child shutdown.
Screenshots require session opt-in, an explicit capture and a separate review before
seven-day retention. Portable memory excludes images and auth. Runnable ZIP exports
contain reviewed source/recipes/synthetic data, with no live consent or credentials;
their independent results do not become authoritative app evidence.

The evidence workbench projects backend facts once for 2D, lazy 3D and keyboard list
modes. Unknown coverage, stale evidence, manual methods and owner exceptions retain
their axes. WebGL failure falls back to the list/2D; no map infers verdicts from edges.

## Installation compatibility

Retain `dev.whatchanged.app`, resolved data path, Rust/package/library identity,
uninstall identity and binary/autostart path in the first transition. Public display
branding is G2-gated. A checksum is not code signing; G3 approves one exact frozen
release/site/notice manifest before upload or enabling a download.

Legacy diagnostics, Mark working/checkpoint behavior, collector preferences and
navigation remain supported. Creating an owner workspace never enables collectors.
Owner Accept change must not be implemented by reusing Mark working.
