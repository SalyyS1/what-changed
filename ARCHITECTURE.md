# Architecture

## Shape

One process. A Tauri 2 application whose backend is a Rust binary (`WhatChanged.exe`) and whose
frontend is a Svelte 5 single-page application rendered by the system WebView2 runtime. No service, no
scheduled task, no helper process, no second runtime, no elevation. When the window is closed the same
process keeps running with only a tray icon; quitting the tray is the only exit.

The window is created with native decorations off, so the titlebar is application-drawn. `withGlobalTauri`
is off, so the frontend has no ambient bridge object: every backend call goes through a generated,
typed wrapper. A Content Security Policy is set from the first commit, not added later.

## Data flow

An observation, its proven relationships and its next baseline must describe the same durable fact.
Advancing a baseline without the corresponding history would silently erase the change from the next
diff. The ingestion boundaries have these executable owners:

| Boundary | Authority |
|---|---|
| Raw observations and collector capabilities | [Collector contract](src-tauri/src/collectors/mod.rs) |
| Candidate state and durable acknowledgement | [Snapshot contract](src-tauri/src/collectors/snapshot.rs) |
| Identity normalization and privacy admission | [Normalizer](src-tauri/src/collectors/normalize.rs), [sanitizer](src-tauri/src/privacy/sanitize.rs) |
| Events, entities, evidence and baseline transaction | [Atomic collector commit](src-tauri/src/storage/collector_commit.rs) |
| Queue admission, commit receipts and batch notices | [Writer](src-tauri/src/storage/writer.rs) |
| Durable known-good capture and semantic comparison | [Checkpoint service](src-tauri/src/checkpoint/service.rs), [semantic adapters](src-tauri/src/checkpoint/state_items.rs) |
| Bounded investigation/window visualization payload | [Change Map shaper](src-tauri/src/analysis/change_map.rs), [Change Map command](src-tauri/src/commands/change_map.rs) |
| Bounded relationship traversal and System Map payload | [System Map repository](src-tauri/src/storage/system_map_repository.rs), [System Map command](src-tauri/src/commands/system_map.rs) |
| Database lifecycle and read boundary | [Database owner](src-tauri/src/storage/db.rs) |
| Frontend invalidation and bounded history queries | [Event bus](src-tauri/src/app/event_bus.rs), [query service](src-tauri/src/services/query_service.rs) |

Reads and writes never contend for the same connection. One dedicated writer thread owns the only
read-write connection; every query runs on a read-only connection from a small pool. Nothing else in the
process is allowed to open the database.

## Rust module map

| Module | Owns | Must not |
|---|---|---|
| `app/` | Application state, tray, window lifecycle, event bus, input validation | Contain business rules |
| `commands/` | The typed IPC surface: parse, validate, delegate, map errors | Contain logic; bodies stay short |
| `dto/` | Serializable transfer types with `ts_rs::TS` derives | Leak internal types |
| `domain/` | Events, entities, enums, settings, the tool allowlist | Reference the OS or SQL |
| `storage/` | Migrations, repositories, the writer, retention, purge, recovery | Be bypassed |
| `analysis/` | Scoring, graph walking, explanations, evidence lookup, map payload | Import collectors or platform |
| `privacy/`, `redact/` | The sanitizer, the log-scrubbing writer, the `Sanitized<T>` gate | Have a second constructor |
| `collectors/` | Collector contracts, normalization, runtime and canonical descriptor catalog | Contain `#[cfg(windows)]` |
| `checkpoint/` | Capture coordination, semantic state projection, comparison and automatic checkpoint policy | Enumerate the OS or bypass collector workers |
| `platform/windows/` | Every Win32, COM, registry, named-pipe, and process call | Be called from `domain/` |
| `notify/` | The notification policy as a pure function, plus a thin OS adapter | Decide anything in the adapter |

## Frontend module map

| Directory | Owns |
|---|---|
| `src/app/` | Shell, titlebar, sidebar, hash router, route table |
| `src/components/ui/` | The primitive layer: buttons, dialogs, drawers, chips, inputs, skeletons |
| `src/components/state/` | `StateShell` and the empty/error/loading blocks |
| `src/components/layout/` | `ScreenFrame`: the one header-plus-body frame every screen renders inside |
| `src/components/{timeline,event,search,visualization}/` | Feature-shaped composites |
| `src/features/*/` | One folder per screen, plus its exact copy |
| `src/lib/tauri/` | Generated bindings, typed client wrappers, typed event listeners |
| `src/lib/stores/` | Rune-based state classes; the only consumers of the client |
| `src/lib/formatting/` | Time, bytes, and event copy assembly |
| `src/lib/design/`, `src/styles/` | Tokens, theme, contrast maths |
| `src/three/` | The only place Three.js is imported |

The router is hash-based and typed. There is no SvelteKit: no SSR, no adapter, no filesystem routing —
this is a desktop application shipped as static assets, and a server framework would be dead weight.

Screens never call the backend directly. A screen consumes a store, a store consumes
`src/lib/tauri/client.ts`, and that file is the only place `invoke` appears. Every store exposes an
`AsyncState<T>` union, and `StateShell` renders exactly one of loading, empty, error, or content from it.
Large-data and small-window are handled by virtualisation and container queries respectively. The Phase 07
Timeline uses TanStack Svelte Virtual with fixed 72px event rows and eight-row overscan; its store retains at
most three 200-row pages and refetches an evicted adjacent page instead of letting a long session grow memory.
Search and event details are mounted once at shell level so their dialog/drawer focus traps never nest.

Storage recovery also fails closed at the shell: until backend status resolves, the app claims neither healthy
history nor active monitoring. A non-ready database replaces ordinary routes with the recovery surface while
leaving Diagnostics reachable.

## Persistence

SQLite through `rusqlite` with the library bundled, so there is no system SQLite dependency and no DLL to
ship. Configured once at open:

| Pragma | Value | Reason |
|---|---|---|
| `journal_mode` | `WAL` | Readers never block the writer |
| `synchronous` | `NORMAL` | Durability appropriate for a local history, not a ledger |
| `busy_timeout` | 5000 | A slow checkpoint waits instead of failing |
| `foreign_keys` | `ON` | Relationship rows cannot orphan |
| `auto_vacuum` | `INCREMENTAL` | Retention pruning can actually return disk |
| `temp_store` | `MEMORY` | Sorting does not hit disk |
| `cache_size` | -8000 (about 8 MB) | Bounded, chosen rather than defaulted |

Sixteen tables. Thirteen are the specified set — events, entities, entity relationships, snapshots,
checkpoints, health states, collector status, settings, watched locations, applications, analysis sessions,
analysis results, and the migration ledger — and three exist because the product would otherwise lie about
what it knows: `collector_baselines` holds each collector's last-known state so a restart diffs against
what the machine looked like when the app was last running, `config_snapshots` holds the redacted per-key
baseline the config diff compares against, and `investigations` holds the denormalized investigation
history the UI pages.

All sixteen are created by the one migration that owns the schema, never by the phase that first writes to
them, and a build gate asserts that **every table name appearing in Rust has a `CREATE TABLE` in the
migrations directory** — a table used but never created is a compile-time-cheap mistake to make and an
expensive one to find at runtime.

Eight indexes exist to serve named queries, and every measured query has a query-plan assertion proving it
uses one — a plan that scans the events table fails the test rather than merely being slow.

Search is FTS5, over an external-content table. That is not a runtime choice: the SQLite library is
vendored into the binary, so full-text availability is fixed when the app is compiled, and it was measured
present before the schema was written. One migration body ships, and a startup assertion fails loudly if a
future dependency bump ever removes the feature.

Timestamps are `i64` epoch **milliseconds UTC**, everywhere, with local time applied only at the display
edge. Pagination is keyset, not offset: `WHERE (timestamp, id) < (?, ?)`, so page 500 costs the same as
page 1. Statements are prepared once and cached.

Entity identifiers follow a documented grammar so the same thing gets the same id across restarts — for
example a process is keyed by a hash of its lowercased executable path rather than by a pid, a port by
protocol plus address plus number, a service by its service name, a container by its name. Identity is
the spine of the analysis graph; if identifiers drift, relationships silently rot.

| Kind | Form | Keyed on |
|---|---|---|
| process | `process:<hash>` | canonical executable path, not the pid |
| port | `port:<tcp\|udp>:<address>:<port>` | protocol, bind address, port number |
| service | `service:<ServiceName>` | the service name, never the display name |
| app | `app:<hive>:<registry key>` | the uninstall key it was enumerated from |
| driver | `driver:<device instance id>` | the device instance the driver is bound to |
| file | `file:<hash>` | canonical path |
| adapter | `adapter:<guid>` | the adapter's GUID |
| container | `container:<12-character id>` | the short id Docker itself shows |
| image | `image:<repo:tag>` | repository and tag |
| project | `project:<hash>` | canonical folder path |
| update | `update:<KB or update id>` | the KB number, or the update id when there is none |
| task | `task:<task path>` | the scheduled-task path |
| system | `system:local` | the machine itself, for boot and shutdown |

`<hash>` is the first sixteen hex characters of the SHA-256 of the canonical form: lowercased, `\\?\`
stripped, forward slashes normalised, trailing separator removed — because Windows paths are
case-insensitive and the same file arrives spelled several ways. Every prefix is written in exactly one
function in `domain/ids.rs`, so a collector cannot invent a variant, and the id's prefix maps back to its
entity type.

Retention defaults to 30 days for events, 7 days for detailed snapshots, and 90 days for events marked
important. Investigation histories use a separate fixed 90-day class. Checkpoint snapshots are exempt: a
marker you named must still be comparable when you need it. Pruning runs in bounded batches with yields
between them, so maintenance never freezes the UI.

### Corruption is a state, not a crash

The database is integrity-checked at open. A corrupt result does not abort startup: the application still
finishes booting and routes the UI to a recovery screen offering repair, diagnostics export, or a history
reset. Three consecutive unclean starts drop into a safe mode with collectors off, so a bad state cannot
become a crash loop that locks the user out of their own data.

The command boundary rejects repair and reset while the database is healthy;
intentional deletion uses the privacy workflow's typed confirmation and
one-use challenge. A fresh install and a post-purge database also keep
collectors disabled until onboarding completes or is skipped, so persisted
preferences cannot start collection before local consent.

Close is terminal for a database handle: recovery must not resurrect it after shutdown. File recovery
needs one lifecycle owner through connection drain, file work and reopening, so a competing recovery
receives a busy error instead of competing for the same files. These decisions and their race tests
belong to [the database owner](src-tauri/src/storage/db.rs).

A writer callback cannot wait for its own writer or coordinate physical shutdown. A callback's close
request makes the handle terminal; an external close or final drop owns physical drain. Keeping that
drain outside the connection-state lock is necessary for queued callbacks to finish. The
[writer and ticket boundary](src-tauri/src/storage/writer.rs) owns reentrant rejection before admission;
the [atomic storage tests](src-tauri/tests/collector_commit.rs) cover the split-ticket path as well.

## IPC surface

The command allowlist is owned by [command registration](src-tauri/src/lib.rs) and
[command metadata](src-tauri/src/commands/mod.rs), with
[contract tests](src-tauri/tests/commands_contract.rs) as the parity authority. Debug and release
registration must remain distinct so synthetic tooling cannot become an ordinary release capability.
There is no dynamic dispatch, no SQL string parameter, no shell parameter, and no
unconstrained path parameter. A command body parses and validates its input, calls a service, and maps the
error — the plan caps bodies at 30 lines precisely to keep logic out of this layer.

Path-taking commands canonicalise, reject reparse points and junctions, cap directory depth, cap the
number of watched roots, and refuse system directories and other users' profiles. The frontend is treated
as untrusted: every command re-validates, including values the UI received from Rust moments earlier.

Rust types are the source of truth for TypeScript. DTOs and the domain types they reuse derive
`ts_rs::TS`, and `dto/bindings.rs` collects those declarations into the single generated
`src/lib/tauri/bindings.ts` — one file rather than one per type, because thirty one-line modules is a
build cost and a reading cost for nothing. `tauri-specta` is not used: its 1.0 release is the Tauri v1
API and its Tauri 2 line is still a release candidate (D-14).

The consequence of that choice is that command *signatures* are hand-written in `client.ts`, since no
derive sees a `#[tauri::command]`. So the parity gate is two-sided. `commands_contract.rs` asserts the
registered handler, the generated `COMMANDS` array and the function names `client.ts` actually invokes
are the same set — a command with no wrapper and a wrapper for no command both fail. `bun run
check:bindings` regenerates the file and fails on any difference, so renaming a Rust field breaks the
build rather than breaking the UI at runtime. Both run inside `bun run verify`.

`EventSummary` is deliberately render-complete but still bounded. Phase 07 added optional `before` and
`after` JSON values to the existing generated type; both have already crossed the redaction chokepoint and
never contain file contents. Timeline rows can therefore show one- or two-sided deltas without an N+1 series
of `get_event_details` calls. Full metadata, related events and occurrence counts remain details-only.

Command bodies whose implementing phase has not landed return a typed `not_implemented` error naming
that phase, and every one is listed in `ROADMAP.md` under "Tracked stubs". A registered command with no
body is otherwise invisible: it type-checks, it appears in the bindings, and it looks finished.

Events flow the other way through the names owned by [the event bus](src-tauri/src/app/event_bus.rs):

| Event | Payload | Rate |
|---|---|---|
| `system-event-added` | Batch notice: count, newest id, categories, highest importance | At most one emit per 500 ms |
| `collector-status-changed` | Collector id; authoritative state comes from the status command | Runtime status changes |
| `analysis-completed` | Investigation id | On completion |
| `monitoring-state-changed` | Paused or active | On change |
| `checkpoint-saved` | Checkpoint id | After a manual, tray or automatic checkpoint commits |

`analysis-completed` has a production emitter in the investigation command and a frontend listener in the
investigation store. The id is only a notice; the get command remains the authoritative state read.

The batch notice deliberately carries no event bodies. The frontend decides whether to fetch, based on
where the user is scrolled — which is what makes a burst of a thousand inserts cost the renderer nothing.
The coalescing merges rather than drops: counts add, the newest id wins, categories union, and the
highest importance survives, so the frontend learns everything that happened, once.

Capabilities are the security surface, so they are enumerated rather than accumulated. The baseline is the
core default set plus window drag plus the folder-picker dialog; the trust layer adds notifications and a
single path-scoped "reveal in folder" permission. A test parses the capability files and fails if any
permission identifier begins with `fs:`, `shell:`, `http:`, `sql:`, `process:` or `updater:`, and a second
test asserts the granted set is *exactly* the documented list — a count alone would not notice one
permission swapped for another.

## Tray and window lifecycle

Closing the window hides it and monitoring continues; quitting from the tray is the only exit, and it is a
clean one — the write queue is drained and the crash-loop counter reset before the process ends. The first
hide shows one balloon saying recording continues, guarded by a stored flag so it is shown once ever.

That behaviour has one dangerous edge, and it is handled rather than assumed away. If the tray icon fails
to build, hiding would leave a process the user cannot reach and cannot quit without Task Manager. So the
hide is conditional on the tray existing: `AppState::tray_available()` is false, `CloseRequested` is
allowed through, `get_app_status` reports it, and the titlebar's close button relabels itself to say the
window will quit. Tray construction failure degrades the app; it never strands it.

The menu is seven items in a fixed order, and Pause and Resume are both always present with the
inapplicable one disabled (D-13) — a menu that changes shape under the cursor moves the row below it, so
the click meant for Settings lands on Quit. Every menu handler calls the same function the equivalent
command calls, so the tray and the interface cannot drift apart.

Checkpoint work is part of clean shutdown. Admission closes first, collector shutdown wakes any capture
waiting on a worker, automation joins, and the app waits for tracked checkpoint callers before flushing and
writing the clean marker. The executable ordering owner is
[lifecycle shutdown](src-tauri/src/app/lifecycle.rs); detached command or tray tasks cannot write after that
marker.

## Collector runtime

Failure belongs to the source that failed, not to the recorder as a whole. The
[supervisor](src-tauri/src/collectors/runtime.rs), [worker](src-tauri/src/collectors/runtime/worker.rs)
and [retry policy](src-tauri/src/collectors/runtime/policy.rs) own lifecycle and isolation. The release
[panic strategy](src-tauri/Cargo.toml) is part of that boundary: aborting the process would defeat
collector-level recovery. The [release executable probe](src-tauri/examples/collector_panic_probe.rs)
is the validation owner; test-harness unwinding alone is not release evidence.

An old notification callback must not regain authority when its collector is recreated. Cancellation
and lifecycle generations belong to the worker, with adversarial coverage in
[isolation tests](src-tauri/tests/collector_isolation.rs) and
[targeted-wake tests](src-tauri/tests/collector_targeted_wake.rs). Disabling a collector means no further
collection, not a requirement to destroy its parked worker thread. The latter would add thread churn
without changing the user's control.

For stateful sources, [the snapshot contract](src-tauri/src/collectors/snapshot.rs) owns candidate state,
opaque commit identity and collector-bound baseline reads. The normalizer and atomic storage owner above
are the authority for payload limits, sanitized entities, edge evidence and baseline ordering. Their
[storage tests](src-tauri/tests/collector_commit.rs) and
[runtime tests](src-tauri/tests/collector_atomic_runtime.rs) are the acceptance route. Pause and disable
must fence future admission; they cannot revoke a transaction already admitted for durable completion.
That distinction prevents an acknowledged change from being replayed or silently forgotten.

Foreground queries have priority over starting resource-heavy collection. The
[scheduler](src-tauri/src/collectors/schedule.rs) and worker own staggering, the global expensive-work
lease and completion-relative cadence. This is a cooperative query gate, not machine-load detection or
preemption of an OS call already in progress. Automatically changing a polling provider into a
notification provider after overruns is not valid: the required notification source might not exist.

The [registry](src-tauri/src/collectors/registry.rs) owns descriptor copy, cost and cadence;
[Settings](src/features/settings/CollectorList.svelte) consumes the generated contract. Live health and
delivery units belong to the [status DTO](src-tauri/src/dto/status_dto.rs),
[Diagnostics command](src-tauri/src/commands/diagnostics.rs) and
[Diagnostics store](src/lib/stores/diagnostics.svelte.ts). Event-row loss and atomic-cycle loss are
distinct accounting units; a baseline-only cycle must not disappear from failure reporting because it
contains no event rows. Accepted-event counts are not a universal durability claim across legacy and
atomic ingestion. The UI's [collector presentation](src/features/settings/collector-presentation.ts)
and [Diagnostics screen](src/features/diagnostics/DiagnosticsScreen.svelte) own their labels.

The [monitoring owner](src-tauri/src/app/monitoring.rs) and
[tray adapter](src-tauri/src/app/tray.rs) own timed pause. An absolute resume deadline avoids a ticking
tray label and remains meaningful each time the menu is opened.

## Checkpoints

A checkpoint is durable evidence, not a copy of transient collector objects. The
[capture coordinator](src-tauri/src/collectors/runtime/capture.rs) requests cycles from existing enabled
workers and waits for their durable acknowledgement. The [checkpoint service](src-tauri/src/checkpoint/service.rs)
then projects acknowledged baselines and config snapshots into compact items and commits the marker and its
items atomically through the [repository](src-tauri/src/storage/checkpoint_repository.rs). The sanitizer is
the only admission path for item state. Automatic installer, Windows Update and weekly requests enter the
same service through a bounded queue; identical concurrent intents coalesce while distinct automatic intents
wait their turn.

Comparison semantics stay with collector-owned adapters rather than a generic JSON diff. Process instance
reuse, network adapter identity, service state, application and driver versions, listeners, startup entries,
updates and config keys each use their collector's semantic field rules. A truncated baseline, truncated
config snapshot or malformed durable shape makes that collector incomplete; it moves to the side-specific
missing list and is skipped, so partial evidence cannot manufacture removals. The executable owners are
[state projection](src-tauri/src/checkpoint/state_items.rs) and
[comparison](src-tauri/src/checkpoint/compare.rs).

Compare-to-now is deliberately a no-sweep read of the latest acknowledged durable state plus the runtime's
current manifest. A user creates a new checkpoint when a forced fresh capture is wanted; ordinary comparison
therefore remains an indexed-read and in-memory-join operation. The result carries its time window so the UI
can route to the matching custom Timeline range without encoding an unimplemented event-id filter.

## The platform boundary

Windows-specific providers belong behind the platform boundary; collection policy must remain testable
without a particular Windows installation, running process or Docker daemon. Two gates in
[the architecture tests](src-tauri/tests/architecture.rs) own that constraint: no target `cfg` under
`domain/`, `analysis/` or `collectors/`, and no Win32 import outside `src/platform/windows/`.

[The provider traits](src-tauri/src/platform/traits.rs) are the seam. A collector diffs snapshots and a
provider produces them, so every diff rule is unit-testable against a scripted table and the only code
that needs a real machine is the enumeration itself. The trait contract makes a partial read an error
rather than a short list: a collector diffs whatever it is handed, so a truncated process table would
forge a stop for every process past the point where reading failed, and an unreadable address family
would close every socket in it.

Factory activation belongs to [the cfg-free registry](src-tauri/src/collectors/registry.rs), and
application composition belongs to [AppState](src-tauri/src/app/state.rs). The factory-only overlay is
deliberate: platform wiring must not silently change a source's identity, Settings copy, scheduling
metadata or non-elevation promise. Deterministic tests use the unavailable catalog; composed registries
enter through `AppState::open_with_registry`, and the shipped app composes through
[the platform entry point](src-tauri/src/platform/mod.rs). The
[composition tests](src-tauri/tests/collector_registry_composition.rs) own rejection and preservation
checks.

The two collectors share one process enumeration rather than each taking their own. The process collector publishes
a complete, successful snapshot into [a session-local map](src-tauri/src/collectors/process_map.rs) and
the listener collector reads it to name a socket's owner. The map expires and stops answering when its
publisher is cancelled or paused, because attributing a port to a process that was replaced ten minutes
ago is a false relationship and a false relationship is worse than an absent one: an unresolved owner
records the port at low confidence and proves no edge.

Both collectors are stateful, so both go through the atomic snapshot protocol above and neither keeps a
second persistence path. What they stage is deliberately small and structural: a validated process id
and an instance key, never the executable path the id was hashed from. A path the sanitizer rewrote on
the way out of a baseline would otherwise come back as a different program, which reads exactly like a
stop followed by a start.

The service and installed-application collectors use the same rule for cross-source evidence. Services
publish current binary-path evidence, applications publish current install-root evidence, and files read
the service map for `config_of`. The two startup snapshots publish before a bounded, cancel-aware peer
readiness wait so an event cannot permanently miss its `application --provides--> service` edge merely
because worker threads started in a different order. Both maps expire with their publisher; raw paths
are session-only and skipped by durable baselines. Relationship-producing changes are admitted with the
same atomic observation/edge/entity capacity used by storage, so a baseline never advances past a
truncated edge set.

The five-second service path enumerates `SERVICE_WIN32` only. Driver inventory belongs to Phase 12.
Service configuration is queried immediately for new or status-changed services and fully reconciled on
an owned 15-minute cadence; no detached platform worker survives collector shutdown. Registry watchers
keep stop-event ownership in their guard until every watcher thread has joined.

## Update and driver collectors

Windows Update history is read through the `windows` crate's typed WUA interfaces on one dedicated STA
thread. COM pointers remain inside that thread; the collector receives plain rows, diffs the durable
`(UpdateID, Date)` baseline and commits observations with the next baseline atomically. The first read is
therefore a baseline, not a replay of update history. `Win32_QuickFixEngineering` is intentionally not a
fallback because it under-reports the measured history. A shared in-flight coordinator survives runtime
factory recreation: a timed-out synchronous COM call retains ownership until it actually returns, while
replacement collectors report a no-op instead of opening an overlapping WUA query.

Driver inventory stays behind `platform/windows/devices.rs`. SetupAPI handles are RAII-guarded and
properties use size-then-fill reads. The provider owns notification registration and reports one typed
wake tier: Configuration Manager, registry, or timer-only. Sweeps run behind both the expensive-collector
lease and query-busy gate; an atomic in-flight guard prevents overlap, and a ten-second watchdog abandons
a stalled result without advancing the acknowledged baseline. Version rollback remains inside the closed
event vocabulary as `version_changed` plus `details.change_kind = "rolled_back"`.

An owned Windows resume monitor compares wall elapsed time with `QueryUnbiasedInterruptTime`, which
excludes sleep. A gap over sixty seconds coalesces targeted wakes for updates, drivers, and network. Its stop
condition is prompt and joined during application shutdown.

## Network, startup, and Event Log collectors

Network and startup are durable diff collectors. Both commit observations with their next sanitized
baseline, flag first-cycle changes found after downtime as time-uncertain, and share an in-flight query
coordinator across factory recreation so a timed-out Win32/COM call cannot overlap its replacement.
Network wake ownership combines IP Helper interface/route callbacks with the per-user proxy registry
watch; startup combines Run-key registry notifications and private Startup-folder watchers with a rare
Task Scheduler sweep. Both stay behind the global expensive-work lease and foreground query gate.

Task Scheduler COM lives on one owned STA thread; interface pointers never cross it. An inaccessible task
is isolated from the rest of the inventory, and an unavailable task store preserves the acknowledged task
subset while Run keys and Startup folders continue to diff. Event subtypes remain inside the closed event
vocabulary and carry bounded `event_name`/`change_kind` metadata.

Event Log callbacks render bounded XML and perform only a non-blocking queue send plus wake request. The
collector parses provider/id/level/time/Data fields away from the callback, skips a malformed record without
discarding its neighbours, and never subscribes to Security. The storage writer owns cross-drain storm
coalescing and order-independent process/update corroboration, keeping one durable event count regardless
of callback batching. Subscriptions target future records; after subscribing, one reverse System query
backfills only the newest 6005 boot boundary so `SinceLastRestart` is truthful without claiming general
downtime replay.

## The analysis boundary

[`analysis/`](src-tauri/src/analysis/mod.rs) is a deterministic pure engine over already-redacted events,
entities and relationships. It imports nothing from `storage/`, `platform/` or `collectors/`, and contains
no ML, network call, model or language runtime. The same input produces the same ranking. Its hard bounds
are 5,000 analysed events, 20 returned suspects, chain depth four and 200 chain paths. Confidence classes
come from evidence shape rather than score magnitude, and the generated-copy gate rejects causal wording
for results without confirmed evidence.

[`InvestigationService`](src-tauri/src/services/investigation_service.rs) owns the impure edge. It resolves
windows and reads events, boundary context, entity names, recorded relationships and 90-day recurrence
counts inside one SQLite read transaction before invoking the engine. Its O(n + relationships)
incident-neighbour construction avoids the full pairwise edge set before deterministic result ordering.
The six typed command entry points live in
[`commands/analysis.rs`](src-tauri/src/commands/analysis.rs).

[`storage/investigations.rs`](src-tauri/src/storage/investigations.rs) owns the durable result: a sanitized,
immutable presentation snapshot plus auditable score rows, written atomically and retained for 90 days.
Reopening does not depend on source events that may already have expired. Before submission, the frontend's
problem draft is memory-only; only the non-sensitive window preference is stored locally. That boundary is
owned by [`investigation.svelte.ts`](src/lib/stores/investigation.svelte.ts).

The Change Map payload is shaped and capped in Rust, so its frontend positions recorded structure rather
than inferring causal structure. [`change_map.rs`](src-tauri/src/analysis/change_map.rs) owns the 3,000-node
and 6,000-edge limits for both Investigation and filtered-Timeline requests.

The System Map consumes the recorded relationship boundary rather than creating a second causal engine.
Its bounded breadth-first walk belongs to
[`system_map_repository.rs`](src-tauri/src/storage/system_map_repository.rs), beside the indexed SQLite
data it traverses; [`analysis/system_map.rs`](src-tauri/src/analysis/system_map.rs) owns only the typed caps
and payload contract. The map reports the latest retained proof timestamp for each directed triple and never
manufactures an executable or process edge absent from the persisted relationship vocabulary.

## Three.js boundary

The contextual Change Map and global System Map share renderer, camera, picking, disposal and palette
primitives under [`src/three/shared/`](src/three/shared/). Their graph meaning remains separate:
[`changeMap/`](src/three/changeMap/) renders recorded temporal/causal evidence, while
[`systemMap/`](src/three/systemMap/) renders the non-temporal dependency neighbourhood supplied by storage.
[`ChangeMapFrame.svelte`](src/components/visualization/ChangeMapFrame.svelte) and
[`SystemMapFrame.svelte`](src/components/system/SystemMapFrame.svelte) own capability selection and accessible
navigation for their surfaces. Three.js imports remain confined to [`src/three/`](src/three/), keeping
feature code and both 2D fallbacks outside the renderer dependency.

Automatic mode uses 2D for reduced motion and unavailable or lost WebGL. A forced 3D preference honours
reduced motion through on-demand rendering. The shared lifecycle and disposal contract is executable in
[`createScene.lifecycle.test.ts`](src/three/changeMap/createScene.lifecycle.test.ts) and
[`scene.lifecycle.test.ts`](src/three/systemMap/scene.lifecycle.test.ts); renderer-choice coverage lives beside
each frame. Hardware-browser measurements for both surfaces are recorded in
[`docs/PERFORMANCE.md`](docs/PERFORMANCE.md).

## Invariants enforced by tests

These are build gates, not review conventions. Each one exists because the failure it prevents is
expensive and silent.

| Invariant | What it prevents |
|---|---|
| No SQL outside `storage/` | Queries drifting into commands, where nobody indexes them |
| Every table used in Rust has a `CREATE TABLE` in a migration | A phase persisting to storage that no migration creates — found three times in review |
| No HTTP client in the resolved Windows dependency graph | The offline promise decaying into a promise nobody checks. The graph, not the lockfile: `hyper` is legitimately present for other targets |
| No `unwrap()` or `expect()` in non-test Rust | A panic in a collector thread becoming a lost afternoon |
| No `#[cfg(windows)]` in `domain/`, `analysis/`, `collectors/` | Untestable business logic |
| No Win32 import outside `platform/windows/` | A privilege boundary nobody can audit by reading one directory |
| `analysis/` imports nothing from `platform/` or `collectors/` | Ranking that depends on collection order |
| Only `privacy::sanitize` can construct `Sanitized<T>` | An unredacted write, which is a leak |
| No secret fixture survives in the bytes of the database file | A redaction that holds in the sanitizer's output but not in what SQLite stored |
| Registered commands and generated bindings match exactly | A command reachable without a typed wrapper |
| No capability identifier from the forbidden prefixes | Filesystem or shell access arriving by accident |
| No `@tauri-apps/api/core` import outside `src/lib/tauri/` | Untyped, unvalidated backend calls |
| No `{@html}` anywhere in components | A malicious config value or container label reaching the DOM |
| No raw hex colour outside the token files | A design system that exists on paper only |
| `three` imported only under `src/three/` | The renderer leaking into feature code |

## Budgets

The plan sets numeric budgets per layer — query latency at 10k, 100k, and 1M events, database size per
event, idle CPU, working set, cold-start time, frame times, and bundle size — and treats a missed budget as
a diagnosis task, never as a number to relax. Measured values live in `docs/PERFORMANCE.md`; the budget
tables and the escalation rules live in the phase files, starting with
[phase-20](plans/260829-0437-what-changed-desktop-mvp/phase-20-performance-accessibility-hardening.md).

One budget is deliberately not a budget. The brief's 80 MB memory figure does not say whether it means
the total working set Task Manager shows or the private working set the app actually costs the machine,
and on an empty shell the two land on opposite sides of it (361.7 MB against 77.1 MB — six WebView2
processes are 92% of the total). The user's decision, recorded as D-15, is to measure both at every
checkpoint against 80 MB as a reference line and gate on neither. The consequence is stated rather than
hidden: memory is a measurement in this release, and the trend across checkpoints is what any later scope
decision stands on. What stays enforced is that both figures always appear, the WebView2 share is broken
out, and no collector is ever disabled to move a number.

## Where to read the detail

| Layer | Phase file |
|---|---|
| Toolchain, config, identity | [phase-01](plans/260829-0437-what-changed-desktop-mvp/phase-01-foundation-toolchain.md) |
| Design system and shell | [phase-02](plans/260829-0437-what-changed-desktop-mvp/phase-02-design-system-app-shell.md) |
| Redaction chokepoint | [phase-03](plans/260829-0437-what-changed-desktop-mvp/phase-03-redaction-privacy-chokepoint.md) |
| Domain model and persistence | [phase-04](plans/260829-0437-what-changed-desktop-mvp/phase-04-domain-model-persistence.md) |
| Typed IPC, tray, lifecycle | [phase-05](plans/260829-0437-what-changed-desktop-mvp/phase-05-typed-ipc-tray-lifecycle.md) |
| Query and classification engine | [phase-06](plans/260829-0437-what-changed-desktop-mvp/phase-06-event-engine-queries.md) |
| Collector runtime | [Runtime source](src-tauri/src/collectors/runtime.rs) and [snapshot contract](src-tauri/src/collectors/snapshot.rs) |
| Analysis engine | [phase-15](plans/260829-0437-what-changed-desktop-mvp/phase-15-analysis-engine-investigation.md) |
| Change Map | [phase-16](plans/260829-0437-what-changed-desktop-mvp/phase-16-three-js-visualization.md) |
| System Map | [phase-17](plans/260829-0437-what-changed-desktop-mvp/phase-17-system-map.md) |
