/**
 * GENERATED FILE - do not edit.
 *
 * Rust is the source of truth for these shapes. Regenerate with:
 *
 *     bun run gen:bindings
 *
 * `bun run check:bindings` fails the build when this file disagrees with the
 * Rust types, so a renamed field breaks the build rather than the interface.
 */

// ---- Domain vocabularies ----

export type Category = "application" | "process" | "filesystem" | "configuration" | "network" | "service" | "driver" | "update" | "docker" | "git" | "security" | "runtime" | "startup" | "hardware" | "system";
export type EventType = "created" | "deleted" | "modified" | "started" | "stopped" | "restarted" | "installed" | "uninstalled" | "updated" | "enabled" | "disabled" | "connected" | "disconnected" | "opened" | "closed" | "failed" | "recovered" | "configuration_changed" | "version_changed" | "churn" | "renamed";
export type EntityType = "process" | "port" | "service" | "app" | "driver" | "file" | "adapter" | "container" | "image" | "docker_volume" | "docker_network" | "project" | "update" | "task" | "system";
export type Severity = "info" | "warning" | "error";
export type Importance = "informational" | "notable" | "important" | "critical";
export type Confidence = "low" | "medium" | "high";
export type ConfidenceClass = "correlated" | "likely" | "confirmed";
export type InvestigationWindow = { "kind": "last_hour" } | { "kind": "today" } | { "kind": "since_yesterday" } | { "kind": "since_last_restart" } | { "kind": "since_checkpoint", checkpoint_id: number, } | { "kind": "custom", from_ms: number, to_ms: number, };
export type ResolvedInvestigationWindow = { from_ms: number, to_ms: number, };
export type NoCauseReason = "nothing_changed" | "only_routine_changes" | "window_too_narrow" | "not_watching_yet" | "no_recorded_evidence_in_workspace";
export type ScoreBreakdown = { temporal: number, relationship: number, category: number, importance: number, rarity: number, failure_proximity: number, chain_bonus: number, total: number, };
export type ChainStep = { event_ids: Array<number>, timestamp_ms: number, label: string, };
export type CheckpointKind = "manual" | "automatic";
export type RelationshipKind = "listens_on" | "opened_by" | "config_of" | "provides" | "publishes";
export type CollectorCost = "cheap" | "moderate" | "expensive";
export type CollectorCadenceDto = { "kind": "poll", interval_ms: number, } | { "kind": "notify" } | { "kind": "notify_plus_sweep", interval_ms: number, };
export type CollectorDescriptorDto = { id: string, display_name: string, what_it_watches: string, why_it_matters: string, cost_hint: CollectorCost, cadence: CollectorCadenceDto, requires_admin: boolean, };

// ---- JSON ----

/** Arbitrary JSON, as a collector recorded it. Already redacted. */
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };

// ---- Events and entities ----

export type NormalizedEvent = {
/**
 * When it happened, epoch milliseconds UTC. When `occurred_at_is_uncertain`
 * is set this is when it was *detected*, which is the best that is known.
 */
timestamp: number,
/**
 * What part of the machine this is about.
 */
category: Category,
/**
 * What happened.
 */
event_type: EventType,
/**
 * What kind of thing it happened to.
 */
entity_type: EntityType,
/**
 * Stable identifier of the thing, per the grammar in `ids`.
 */
entity_id: string,
/**
 * The name a person recognises: a service name, a file name, a port.
 */
entity_display_name: string,
/**
 * How bad it was, on its own terms.
 */
severity: Severity,
/**
 * Whether a person should care.
 */
importance: Importance,
/**
 * The collector that produced this event.
 */
source_collector: string,
/**
 * State before the change, already redacted. Never file contents.
 */
before: JsonValue | null,
/**
 * State after the change, already redacted. Never file contents.
 */
after: JsonValue | null,
/**
 * Structured config key for a provable key-level change.
 */
field: string | null,
/**
 * Anything else worth keeping, already redacted.
 */
metadata: JsonValue | null,
/**
 * Other entities this change touches, by id.
 */
related_entity_ids: Array<string>,
/**
 * One sentence in the product's voice.
 */
user_friendly_summary: string,
/**
 * The same fact with the identifiers a technical reader wants.
 */
technical_summary: string,
/**
 * How sure the collector is of the facts above.
 */
confidence: Confidence,
/**
 * Whether redaction replaced something in this event.
 */
sensitive: boolean,
/**
 * Provenance: the timestamp is a detection time, not an occurrence time.
 */
occurred_at_is_uncertain: boolean, };
export type SystemEvent = {
/**
 * Database id; the second half of a pagination cursor.
 */
id: number,
/**
 * When it happened, epoch milliseconds UTC. When `occurred_at_is_uncertain`
 * is set this is when it was *detected*, which is the best that is known.
 */
timestamp: number,
/**
 * What part of the machine this is about.
 */
category: Category,
/**
 * What happened.
 */
event_type: EventType,
/**
 * What kind of thing it happened to.
 */
entity_type: EntityType,
/**
 * Stable identifier of the thing, per the grammar in `ids`.
 */
entity_id: string,
/**
 * The name a person recognises: a service name, a file name, a port.
 */
entity_display_name: string,
/**
 * How bad it was, on its own terms.
 */
severity: Severity,
/**
 * Whether a person should care.
 */
importance: Importance,
/**
 * The collector that produced this event.
 */
source_collector: string,
/**
 * State before the change, already redacted. Never file contents.
 */
before: JsonValue | null,
/**
 * State after the change, already redacted. Never file contents.
 */
after: JsonValue | null,
/**
 * Structured config key for a provable key-level change.
 */
field: string | null,
/**
 * Anything else worth keeping, already redacted.
 */
metadata: JsonValue | null,
/**
 * Other entities this change touches, by id.
 */
related_entity_ids: Array<string>,
/**
 * One sentence in the product's voice.
 */
user_friendly_summary: string,
/**
 * The same fact with the identifiers a technical reader wants.
 */
technical_summary: string,
/**
 * How sure the collector is of the facts above.
 */
confidence: Confidence,
/**
 * Whether redaction replaced something in this event.
 */
sensitive: boolean,
/**
 * Provenance: the timestamp is a detection time, not an occurrence time.
 */
occurred_at_is_uncertain: boolean, };
export type EventSignature = {
/**
 * What part of the machine.
 */
category: Category,
/**
 * What happened.
 */
event_type: EventType,
/**
 * To what.
 */
entity_id: string, };
export type EventSummary = {
/**
 * Database id.
 */
id: number,
/**
 * Epoch milliseconds UTC.
 */
timestamp: number,
/**
 * What part of the machine.
 */
category: Category,
/**
 * What happened.
 */
event_type: EventType,
/**
 * What kind of thing.
 */
entity_type: EntityType,
/**
 * Stable id of the thing.
 */
entity_id: string,
/**
 * The name a person recognises.
 */
entity_display_name: string,
/**
 * How bad.
 */
severity: Severity,
/**
 * Whether to care.
 */
importance: Importance,
/**
 * How sure.
 */
confidence: Confidence,
/**
 * Which collector.
 */
source_collector: string,
/**
 * State before the change, already redacted. Never file contents.
 */
before: JsonValue | null,
/**
 * State after the change, already redacted. Never file contents.
 */
after: JsonValue | null,
/**
 * Structured config key for a provable key-level change.
 */
field: string | null,
/**
 * One sentence in the product's voice.
 */
user_friendly_summary: string,
/**
 * The timestamp is a detection time, not an occurrence time.
 */
occurred_at_is_uncertain: boolean, };
export type ClusterRef = { key: string, category: Category, coarse_entity: string, bucket: number,
/**
 * Oldest row in the run. The id breaks timestamp ties at a page edge.
 */
first_ts: number, first_id: number,
/**
 * Newest row in the run. The id breaks timestamp ties at a page edge.
 */
last_ts: number, last_id: number, };
export type TimelineItem = { "kind": "event", event: EventSummary, } | { "kind": "cluster", key: string, cluster: ClusterRef, label: string, count: number, first_ts: number, last_ts: number, highest_importance: Importance, category: Category, };
export type TimelinePage = { items: Array<TimelineItem>, next: Cursor | null, };
export type Entity = {
/**
 * Stable id per the grammar in `ids`.
 */
id: string,
/**
 * What kind of thing.
 */
entity_type: EntityType,
/**
 * The name a person recognises. Updated as it changes; the id does not.
 */
display_name: string,
/**
 * First event about it, epoch milliseconds UTC.
 */
first_seen: number,
/**
 * Most recent event about it, epoch milliseconds UTC.
 */
last_seen: number,
/**
 * Anything a collector wants to keep about it, already redacted.
 */
metadata: JsonValue | null, };
export type Relationship = {
/**
 * Where the edge starts.
 */
source_id: string,
/**
 * Where it points.
 */
target_id: string,
/**
 * What the edge means.
 */
kind: RelationshipKind,
/**
 * The event that recorded this edge, so it can be shown as evidence. It
 * becomes `None` if that event has since expired; the edge outlives it.
 */
event_id: number | null,
/**
 * When the edge was recorded, epoch milliseconds UTC.
 */
recorded_at: number, };
export type EventQuery = {
/**
 * Canonical range selected by the caller.
 */
range: TimeRange,
/**
 * Only these categories; empty means all.
 */
categories: Array<Category>,
/**
 * Only rows at least this important.
 */
importance_min: Importance | null,
/**
 * Only rows about this entity.
 */
entity_id: string | null,
/**
 * Continue after this row.
 */
cursor: Cursor | null,
/**
 * Rows wanted. Clamped to `MAX_PAGE_SIZE` by the repository, not trusted.
 */
limit: number, };
export type TimeRange = { "kind": "today" } | { "kind": "yesterday" } | { "kind": "last7_days" } | { "kind": "since_last_restart" } | { "kind": "custom", from: number, to: number, };
export type RetentionPolicy = {
/**
 * Routine events: everything below `important`.
 */
events_days: number,
/**
 * Detailed snapshots not owned by a checkpoint.
 */
snapshots_days: number,
/**
 * Events classified `important` or `critical`.
 */
important_days: number, };

export type Cursor = string;
export type Page<T> = {
/**
 * The rows, newest first.
 */
items: Array<T>,
/**
 * Cursor for the next page, or `None` when this was the last one.
 */
next: Cursor | null, };

// ---- Storage ----

export type DbState = { "state": "ready",
/**
 * The schema version in the file.
 */
schema_version: number, } | { "state": "corrupt",
/**
 * What the check said. Names structures, never rows.
 */
reason: string, } | { "state": "schema_mismatch",
/**
 * Which disagreement.
 */
reason: string, } | { "state": "recovering" };
export type StorageUsage = {
/**
 * `page_count * page_size` of the main file.
 */
database_bytes: number,
/**
 * Size of the write-ahead log.
 */
wal_bytes: number,
/**
 * Rows per table, by name.
 */
table_rows: Array<[string, number]>, };
export type MaintenanceReport = {
/**
 * Events deleted.
 */
events_deleted: number,
/**
 * Snapshots deleted.
 */
snapshots_deleted: number,
/**
 * Investigation histories deleted under their fixed 90-day policy.
 */
investigations_deleted: number,
/**
 * Delete statements run.
 */
batches: number,
/**
 * Wall time of the pass, milliseconds.
 */
duration_ms: number, };
export type RecoveryAction = "repair" | "export_diagnostics" | "reset_history";
export type RecoveryOutcome = {
/**
 * The action that ran.
 */
action: RecoveryAction,
/**
 * Whether it achieved what it set out to do.
 */
succeeded: boolean,
/**
 * One sentence for the screen.
 */
message: string,
/**
 * A file the action produced: the quarantined original, or the report.
 */
artifact: string | null,
/**
 * The database state afterwards.
 */
state: DbState, };
export type BatchNotice = {
/**
 * Rows committed.
 */
count: number,
/**
 * Highest id committed.
 */
newest_id: number,
/**
 * Distinct categories in the batch.
 */
categories: Array<Category>,
/**
 * The most important row in the batch.
 */
highest_importance: Importance,
/**
 * The newest bounded set of committed ids. The event bus uses these to
 * evaluate notification policy without putting event bodies on the IPC
 * payload. Older ids are intentionally omitted during large bursts.
 */
event_ids: Array<number>, };

// ---- Transfer types ----

export type ErrorCode = "directory" | "file" | "logging" | "database" | "not_ready" | "queue_full" | "invalid_input" | "not_implemented" | "rate_limited" | "platform";
export type AppErrorDto = {
/**
 * What kind of failure this was.
 */
code: ErrorCode,
/**
 * One sentence for the user. Scrubbed.
 */
message: string,
/**
 * Whether repeating the call could plausibly succeed.
 */
retryable: boolean, };
export type Theme = "dark" | "light" | "system";
export type VisualizationMode = "auto" | "3d" | "2d";
export type MonitoringProfile = "minimal" | "balanced" | "thorough";
export type Settings = {
/**
 * Appearance.
 */
theme: Theme,
/**
 * Automatic capability detection, forced 3D, or forced 2D.
 */
visualization: VisualizationMode,
/**
 * How long history is kept.
 */
retention: RetentionPolicy,
/**
 * How hard the collectors work.
 */
monitoring_profile: MonitoringProfile,
/**
 * Which collectors are enabled, by collector id.
 */
collectors: { [key in string]: boolean },
/**
 * Whether the first-run tour has been completed.
 */
onboarding_completed: boolean,
/**
 * Whether the developer surfaces are shown.
 */
developer_mode: boolean, };
export type SettingsUpdate = {
/**
 * New appearance, if it changed.
 */
theme?: Theme | null,
/**
 * New Change Map renderer preference, if it changed.
 */
visualization?: VisualizationMode | null,
/**
 * New retention lifetimes, if they changed.
 */
retention?: RetentionPolicy | null,
/**
 * New monitoring profile, if it changed.
 */
monitoring_profile?: MonitoringProfile | null,
/**
 * Collector flags to change. Keys not present are left alone; a key that
 * is not a known collector is rejected rather than stored.
 */
collectors?: { [key in string]: boolean } | null,
/**
 * Whether the first-run tour has been completed.
 */
onboarding_completed?: boolean | null,
/**
 * Whether the developer surfaces are shown.
 */
developer_mode?: boolean | null, };
export type MonitoringDto = {
/**
 * True while collection is paused.
 */
paused: boolean,
/**
 * When the pause lifts by itself, epoch milliseconds UTC. `null` means it
 * stays paused until the user resumes.
 */
resume_at: number | null, };
export type AppStatusDto = {
/**
 * Whether the database is usable, and if not, why.
 */
db: DbState,
/**
 * Whether monitoring is running.
 */
monitoring: MonitoringDto,
/**
 * Whether the tray icon was created.
 *
 * `false` means closing the window would strand the user with an
 * invisible process, so the window stays closable and the shell shows a
 * banner saying so. Never silently hide into a tray that is not there.
 */
tray_available: boolean,
/**
 * Whether this start followed three unclean ones, so collectors stay off.
 */
safe_mode: boolean,
/**
 * Events dropped because the write queue was full, since this start.
 */
dropped_writes: number, };
export type CollectorState = "running" | "paused" | "idle" | "failed" | "degraded" | "disabled" | "unavailable";
export type CollectorWakeTier = "configuration_manager" | "registry" | "timer_only";
export type DockerCollectorState = "probing" | "available" | "reconnecting" | "unavailable";
export type CollectorStatusDetails = { wake_tier: CollectorWakeTier | null, docker_state: DockerCollectorState | null, docker_api_version: string | null, };
export type CollectorStatusDto = {
/**
 * Self-described Settings copy and scheduling cost.
 */
descriptor: CollectorDescriptorDto,
/**
 * Collector id, as stored in the `collectors` setting.
 */
id: string,
/**
 * Whether the user has it switched on.
 */
enabled: boolean,
/**
 * What it is doing.
 */
state: CollectorState,
/**
 * Last completed run, epoch milliseconds UTC.
 */
last_run_at: number | null,
/**
 * How long that run took.
 */
last_duration_ms: number | null,
/**
 * Legacy event rows admitted to the queue, plus atomic rows confirmed
 * committed, since registration. Legacy commit failures are counted
 * separately in `failed_commit_rows`; this is not a durable history total.
 */
events_emitted: number,
/**
 * Failures since its last success.
 */
consecutive_failures: number,
/**
 * Why it last failed, already scrubbed. Never a value it was reading.
 */
last_error: string | null,
/**
 * Collector-specific, bounded diagnostics such as the active driver wake
 * tier. Platform providers never place raw observed values here.
 */
details: CollectorStatusDetails | null,
/**
 * Stated so the privilege promise is visible in the product, not only in
 * the documentation. No collector may report `true`.
 */
requires_admin: boolean,
/**
 * Rows rejected before entering the bounded writer queue.
 */
queue_drops: number,
/**
 * Rows discarded only after SQLite rejected a complete batch.
 */
failed_commit_rows: number,
/**
 * Atomic collector cycles rejected before entering the writer queue.
 * Kept separate from `queue_drops`, whose unit is event rows.
 */
cycle_admission_drops: number,
/**
 * Admitted atomic collector cycles whose SQLite transaction failed.
 * Kept separate from `failed_commit_rows`, whose unit is event rows.
 */
failed_cycles: number, };
export type DiagnosticsDto = {
/**
 * Version of the running build.
 */
app_version: string,
/**
 * WebView runtime version. The legacy wire name is retained until the
 * diagnostics export contract is expanded; this is not the Windows version.
 */
os_version: string,
/**
 * Database state.
 */
db: DbState,
/**
 * Database size on disk, bytes.
 */
database_bytes: number,
/**
 * Schema version in the file.
 */
schema_version: number,
/**
 * Per-collector health.
 */
collectors: Array<CollectorStatusDto>,
/**
 * Events dropped because the write queue was full.
 */
dropped_writes: number,
/**
 * Rows lost after SQLite rejected an admitted batch.
 */
failed_commit_rows: number,
/**
 * Atomic collector cycles rejected before entering the writer queue.
 */
cycle_admission_drops: number,
/**
 * Admitted atomic collector cycles whose SQLite transaction failed.
 */
failed_cycles: number,
/**
 * Approximate pending writer work. A zero means the queue is idle or the
 * runtime does not expose a cheap depth counter.
 */
write_queue_depth: number,
/**
 * Current process working set, when the platform provider can read it.
 */
current_working_set_bytes: number,
/**
 * Peak process working set, when the platform provider can read it.
 */
peak_working_set_bytes: number,
/**
 * Total bytes in retained redaction-filtered log files.
 */
log_bytes: number, };
export type UpdateCheckDto = {
/**
 * Whether a signed updater is configured for this build.
 */
supported: boolean,
/**
 * Whether the user explicitly requested an update check for this call.
 * This is echoed for the UI; it is never persisted or acted on remotely.
 */
opted_in: boolean,
/**
 * The version that is running.
 */
current_version: string,
/**
 * What to tell the user instead.
 */
message: string, };
export type DashboardCountsDto = { total: number, notable: number, important: number, critical: number, };
export type CheckpointRef = { id: number, name: string, created_at: number, };
export type DashboardSummaryDto = { today: DashboardCountsDto, status_line: string, top_important: Array<EventSummary>, monitoring: MonitoringDto, last_checkpoint: CheckpointRef | null, };
export type EventDetailsDto = {
/**
 * The event itself.
 */
event: SystemEvent,
/**
 * Events about the same or a related entity, nearby in time.
 */
related: Array<EventSummary>,
/**
 * How many times the same thing happened to the same entity before.
 */
prior_occurrences: number, };
export type SearchResultDto = { "kind": "event", event: EventSummary, category: Category, importance: Importance, } | { "kind": "checkpoint", checkpoint: CheckpointRef, };
export type WatchedLocationDto = {
/**
 * Row id.
 */
id: number,
/**
 * The canonical path, as it will be shown back to the user.
 */
path: string,
/**
 * Whether it is currently watched.
 */
enabled: boolean,
/**
 * Whether the root still resolves to the same safe, ordinary directory.
 */
available: boolean,
/**
 * How deep below the root the collector descends.
 */
max_depth: number,
/**
 * When it was added, epoch milliseconds UTC.
 */
added_at: number, };
export type ProjectDto = {
/**
 * Stable project entity id.
 */
id: string,
/**
 * The canonical path.
 */
path: string,
/**
 * The folder name, which is what a person calls the project.
 */
name: string,
/**
 * The branch checked out when it was last read, if it is a git checkout.
 */
branch: string | null,
/**
 * The known-good checkpoint selected for this project, if any.
 */
baseline_checkpoint_id: number | null, baseline_checkpoint_name: string | null, baseline_checkpoint_created_at: number | null, };
export type CheckpointDto = {
/**
 * Row id.
 */
id: number,
/**
 * What the user called it, or the generated name.
 */
name: string,
/**
 * Manual or bounded automatic capture.
 */
kind: CheckpointKind,
/**
 * Why it was created, when it was automatic.
 */
reason: string | null,
/**
 * When, epoch milliseconds UTC.
 */
created_at: number,
/**
 * How many items it captured.
 */
item_count: number,
/**
 * How long the capture took.
 */
duration_ms: number,
/**
 * Whether a collector failed or timed out during capture.
 */
partial: boolean,
/**
 * Which collectors are missing from it, if it is partial.
 */
missing_collectors: Array<string>, };
export type DiffItemDto = {
/**
 * What part of the machine.
 */
category: Category,
/**
 * The thing that differs.
 */
entity_display_name: string,
/**
 * Which field of it, when the item is a change rather than an
 * appearance or a disappearance.
 */
field: string | null,
/**
 * Value before. Already redacted.
 */
before: string | null,
/**
 * Value after. Already redacted.
 */
after: string | null, };
export type CheckpointCategoryDiffDto = { category: Category, added: Array<DiffItemDto>, removed: Array<DiffItemDto>, changed: Array<DiffItemDto>, };
export type CheckpointDiffDto = {
/**
 * Things that exist now and did not then.
 */
added: Array<DiffItemDto>,
/**
 * Things that existed then and do not now.
 */
removed: Array<DiffItemDto>,
/**
 * Things that exist in both and differ.
 */
changed: Array<DiffItemDto>,
/**
 * The same rows partitioned for category-first rendering.
 */
grouped_by_category: Array<CheckpointCategoryDiffDto>,
/**
 * Events recorded between the two moments, so the structural diff can be
 * opened as a timeline range rather than read as a list of assertions.
 */
event_ids_in_window: Array<number>,
/**
 * Inclusive beginning of the event window, epoch milliseconds UTC.
 */
window_from: number,
/**
 * Inclusive end of the event window, epoch milliseconds UTC.
 */
window_to: number,
/**
 * Collectors unavailable from the historical (left) side.
 */
left_missing_collectors: Array<string>,
/**
 * Collectors unavailable from the comparison (right) side.
 */
right_missing_collectors: Array<string>,
/**
 * Collectors omitted because either side did not capture them.
 */
skipped_collectors: Array<string>, };
export type InvestigationRequestDto = { problem: string, window: InvestigationWindow | null, project_id: string | null, };
export type SuspectDto = { event_id: number, title: string, timestamp_ms: number, score: number, breakdown: ScoreBreakdown, confidence: ConfidenceClass, confidence_text: string, relative_time: string, why_ranked: Array<string>, chain: Array<ChainStep>, what_to_try: string, };
export type InvestigationDto = { id: number, project_id: string | null, problem: string, window: InvestigationWindow, resolved_window: ResolvedInvestigationWindow, suspects: Array<SuspectDto>, no_cause_reason: NoCauseReason | null, no_cause_message: string | null, total_events_in_window: number, analyzed_events: number, dropped_events: number, created_at: number, };
export type ChangeMapRequest = { "kind": "investigation", investigation_id: number, } | { "kind": "window", range: TimeRange, categories: Array<Category>, importance_min: Importance | null, entity_id: string | null, };
export type ChangeMapLane = "process" | "port" | "config" | "service" | "application" | "update" | "driver" | "network" | "startup" | "system";
export type ChangeMapLaneData = { id: ChangeMapLane, label: string, };
export type ChangeMapNode = { id: string, event_id: number, timestamp_ms: number, category: Category, lane: ChangeMapLane, importance: Importance, label: string, chain_ids: Array<string>, chain_depth: number, is_suspect: boolean,
/**
 * True when the immutable investigation snapshot supplied this node
 * because the original event row is no longer retained.
 */
snapshot_only: boolean, };
export type ChangeMapEdgeKind = "structural" | "temporal";
export type ChangeMapEdge = { id: string, source: string, target: string, kind: ChangeMapEdgeKind, chain_ids: Array<string>, };
export type ChangeMapData = { nodes: Array<ChangeMapNode>, edges: Array<ChangeMapEdge>, symptom_time_ms: number | null, window_start_ms: number, window_end_ms: number, lanes: Array<ChangeMapLaneData>, truncated: boolean, total_nodes: number, omitted_nodes: number, total_edges: number, omitted_edges: number, };
export type EntityRef = { entity_id: string, entity_type: EntityType, };
export type SystemMapNode = { entity_id: string, entity_type: EntityType, display_name: string, hop: number, degree: number, last_changed_at: number | null, recent_event_count: number, };
export type SystemMapEdge = { source: string, target: string, kind: RelationshipKind,
/**
 * Observation timestamp retained by the relationship row. The schema
 * keeps the latest proof for each directed triple.
 */
recorded_at: number, evidence_event_id: number | null, };
export type SystemMapRecentEvent = { event_id: number, timestamp_ms: number, summary: string, importance: Importance, };
export type EntityKindAvailability = { kind: EntityType, count: number, available: boolean, reason: string | null, };
export type SystemMapData = { nodes: Array<SystemMapNode>, edges: Array<SystemMapEdge>, focus: EntityRef | null, truncated: boolean, total_nodes_available: number,
/**
 * Relationships whose source or target entity no longer resolves.
 */
dropped_edges: number, kind_availability: Array<EntityKindAvailability>, focused_recent_events: Array<SystemMapRecentEvent>, };
export type OnboardingStateDto = { completed: boolean, can_skip: boolean, };
export type PrivacyCategoryCountDto = { category: string, count: number, };
export type PrivacySummaryDto = { categories: Array<PrivacyCategoryCountDto>, checkpoints: number, checkpoint_items: number, investigations: number, watched_folders: number, config_baselines: number, database_bytes: number, wal_bytes: number, oldest_record_at: number | null, log_bytes: number, data_path: string, database_path: string, retention: RetentionPolicy, };
export type PurgeReportDto = { events_deleted: number, checkpoints_deleted: number, investigations_deleted: number, baselines_deleted: number, watched_folders_deleted: number, };
export type QuietHours = {
/**
 * Whether the setting is enabled.
 */
enabled: boolean,
/**
 * Start hour, inclusive, 0..=23.
 */
start_hour: number,
/**
 * End hour, exclusive, 0..=23. A different value denotes an overnight
 * window when it is below `start_hour`.
 */
end_hour: number, };

// ---- Commands ----

/**
 * Every command the backend registers, with the argument names it expects.
 * `client.ts` exposes one typed function per entry; `commands_contract.rs`
 * asserts this table, the Rust handler and the object literals `client.ts`
 * passes all agree — so neither a missing command nor a misspelled argument
 * can reach a running window.
 *
 * Tauri converts `snake_case` parameters to `camelCase` on the wire, so the
 * names here are the camelCase form the frontend actually sends.
 */
export const COMMANDS = {
  get_app_version: [],
  get_app_status: [],
  log_ui_error: ['message', 'context'],
  complete_onboarding: [],
  check_for_updates: ['optIn'],
  get_onboarding_state: [],
  reopen_onboarding: [],
  skip_onboarding: [],
  set_autostart: ['enabled'],
  set_tray_forced_colors: ['active'],
  get_dashboard_summary: ['range'],
  get_events: ['query'],
  expand_cluster: ['cluster'],
  get_event_details: ['eventId'],
  search_events: ['query', 'limit'],
  get_system_map: ['focus', 'depth', 'kinds'],
  list_checkpoints: [],
  create_checkpoint: ['name', 'reason'],
  rename_checkpoint: ['checkpointId', 'name'],
  delete_checkpoint: ['checkpointId'],
  compare_checkpoint: ['checkpointId', 'against'],
  start_investigation: ['request'],
  list_investigations: ['limit'],
  get_investigation: ['investigationId'],
  delete_investigation: ['investigationId'],
  analyze_window: ['window'],
  get_related_events: ['eventId'],
  export_investigation_context: ['investigationId'],
  get_change_map: ['request'],
  get_collector_status: [],
  list_collector_status: [],
  set_collector_enabled: ['collectorId', 'enabled'],
  set_monitoring_paused: ['paused', 'minutes'],
  pause_monitoring: ['minutes'],
  resume_monitoring: [],
  get_monitor_state: [],
  get_settings: [],
  update_settings: ['update'],
  list_watched_locations: [],
  add_watched_location: ['path', 'maxDepth'],
  remove_watched_location: ['locationId'],
  list_projects: [],
  add_project: ['path'],
  remove_project: ['projectId'],
  mark_project_working: ['projectId'],
  clear_project_baseline: ['projectId'],
  get_storage_usage: [],
  get_diagnostics: [],
  export_diagnostics: [],
  db_recover: ['action'],
  request_purge_challenge: [],
  get_privacy_summary: [],
  purge_category: ['category'],
  purge_folder: ['path'],
  purge_all: ['confirmation', 'challenge'],
  set_retention: ['retention'],
  set_quiet_hours: ['quietHours'],
  get_quiet_hours: [],
  simulate_event: ['kind'],
  seed_synthetic: ['profile', 'relations'],
} as const;

export type CommandName = keyof typeof COMMANDS;
