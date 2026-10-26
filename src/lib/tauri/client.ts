/**
 * The typed client. One exported function per backend command, and the only
 * file in the application that calls `invoke`.
 *
 * Two things happen here that would otherwise happen in thirty places. Every
 * call is narrowed to a known command name, so a typo is a compile error
 * rather than a rejected IPC message at runtime. And every rejection is
 * normalised into `AppErrorDto`: Tauri rejects with whatever the command
 * returned, which for a panic or a deserialization failure is a bare string,
 * and a screen that has to tell those apart cannot.
 *
 * Outside a Tauri window — Vitest's jsdom, a plain browser tab — `invoke` is
 * unavailable. Rather than throwing during a render, the client rejects with a
 * typed error, so the same `AsyncState` error branch a screen already renders
 * covers the case.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  AppErrorDto,
  AppStatusDto,
  CheckpointDiffDto,
  CheckpointDto,
  ChangeMapData,
  ChangeMapRequest,
  ClusterRef,
  CollectorStatusDto,
  CommandName,
  DashboardSummaryDto,
  DiagnosticsDto,
  OnboardingStateDto,
  PrivacySummaryDto,
  PurgeReportDto,
  EventDetailsDto,
  EventQuery,
  EventSummary,
  InvestigationDto,
  InvestigationRequestDto,
  InvestigationWindow,
  MonitoringDto,
  ProjectDto,
  RecoveryAction,
  RecoveryOutcome,
  SearchResultDto,
  Settings,
  SettingsUpdate,
  StorageUsage,
  EntityRef,
  EntityType,
  SystemMapData,
  TimelinePage,
  TimeRange,
  UpdateCheckDto,
  WatchedLocationDto,
} from './bindings';

/** Whether a value has the shape the backend promises for a failure. */
function isAppError(value: unknown): value is AppErrorDto {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const candidate = value as Partial<AppErrorDto>;
  return typeof candidate.code === 'string' && typeof candidate.message === 'string';
}

/**
 * Turn anything a rejection can be into the one shape callers handle.
 *
 * The fallback message is deliberately vague. An unrecognised rejection is by
 * definition a value nobody designed for the user to read, and pasting it into
 * the interface is how an internal detail becomes a screenshot.
 */
export function toAppError(reason: unknown): AppErrorDto {
  if (isAppError(reason)) {
    return reason;
  }
  if (import.meta.env.DEV) {
    console.error('[what-changed] an unrecognised rejection crossed IPC:', reason);
  }
  return {
    code: 'database',
    message: 'Something went wrong. The details are in the log file.',
    retryable: true,
  };
}

/** True inside a Tauri window; false in jsdom and in a plain browser tab. */
export function backendAvailable(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

const NO_BACKEND: AppErrorDto = {
  code: 'not_ready',
  message: 'The application is not running in its own window.',
  retryable: false,
};

/**
 * The single call site. Every wrapper below goes through it, which is what
 * `commands_contract.rs` reads to prove each command has a wrapper.
 */
async function call<T>(
  command: CommandName,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!backendAvailable()) {
    throw NO_BACKEND;
  }
  try {
    return (await invoke(command, args)) as T;
  } catch (reason) {
    throw toAppError(reason);
  }
}

// ---- Meta ----

export const getAppVersion = (): Promise<string> => call('get_app_version');
export const getAppStatus = (): Promise<AppStatusDto> => call('get_app_status');
export const completeOnboarding = (): Promise<void> => call('complete_onboarding');
export const checkForUpdates = (optIn: boolean): Promise<UpdateCheckDto> =>
  call('check_for_updates', { optIn });
export const getOnboardingState = (): Promise<OnboardingStateDto> => call('get_onboarding_state');
export const reopenOnboarding = (): Promise<void> => call('reopen_onboarding');
export const skipOnboarding = (): Promise<void> => call('skip_onboarding');
export const setAutostart = (enabled: boolean): Promise<boolean> => call('set_autostart', { enabled });
export const setTrayForcedColors = (active: boolean): Promise<void> =>
  call('set_tray_forced_colors', { active });

/**
 * Report a frontend failure. Rate-limited in Rust; a rejection here is
 * swallowed on purpose, because a failure to report a failure has nowhere
 * useful to go and must never replace the original error on screen.
 */
export async function logUiError(message: string, context?: string): Promise<void> {
  try {
    await call('log_ui_error', { message, context: context ?? null });
  } catch {
    // Reporting is best-effort by design.
  }
}

// ---- Events ----

export const getDashboardSummary = (range: TimeRange): Promise<DashboardSummaryDto> =>
  call('get_dashboard_summary', { range });
export const getEvents = (query: EventQuery): Promise<TimelinePage> => call('get_events', { query });
export const expandCluster = (cluster: ClusterRef): Promise<EventSummary[]> =>
  call('expand_cluster', { cluster });
export const getEventDetails = (eventId: number): Promise<EventDetailsDto> =>
  call('get_event_details', { eventId });
export const searchEvents = (query: string, limit = 50): Promise<SearchResultDto[]> =>
  call('search_events', { query, limit });
export const getSystemMap = (
  focus: EntityRef | null,
  depth = 2,
  kinds: EntityType[] = [],
): Promise<SystemMapData> => call('get_system_map', { focus, depth, kinds });

// ---- Checkpoints ----

export const listCheckpoints = (): Promise<CheckpointDto[]> => call('list_checkpoints');
export const createCheckpoint = (name?: string, reason?: string): Promise<CheckpointDto> =>
  call('create_checkpoint', { name: name ?? null, reason: reason ?? null });
export const renameCheckpoint = (checkpointId: number, name: string): Promise<CheckpointDto> =>
  call('rename_checkpoint', { checkpointId, name });
export const deleteCheckpoint = (checkpointId: number): Promise<void> =>
  call('delete_checkpoint', { checkpointId });
export const compareCheckpoint = (
  checkpointId: number,
  against?: number,
): Promise<CheckpointDiffDto> =>
  call('compare_checkpoint', { checkpointId, against: against ?? null });

export type CheckpointProgressState = 'pending' | 'running' | 'complete' | 'failed';
export interface CheckpointProgressNotice {
  collector_id: string;
  collector_name?: string;
  status: CheckpointProgressState;
  completed: number;
  total: number;
}

export const listenCheckpointProgress = (
  handler: (notice: CheckpointProgressNotice) => void,
): Promise<() => void> =>
  listen<CheckpointProgressNotice>('checkpoint-progress', (event) => handler(event.payload));

// ---- Analysis ----

export const startInvestigation = (request: InvestigationRequestDto): Promise<InvestigationDto> =>
  call('start_investigation', { request });
export const listInvestigations = (limit = 50): Promise<InvestigationDto[]> =>
  call('list_investigations', { limit });
export const getInvestigation = (investigationId: number): Promise<InvestigationDto> =>
  call('get_investigation', { investigationId });
export const deleteInvestigation = (investigationId: number): Promise<void> =>
  call('delete_investigation', { investigationId });
export const analyzeWindow = (window: InvestigationWindow): Promise<InvestigationDto> =>
  call('analyze_window', { window });
export const getRelatedEvents = (eventId: number): Promise<EventSummary[]> =>
  call('get_related_events', { eventId });
export const exportInvestigationContext = (investigationId: number): Promise<string> =>
  call('export_investigation_context', { investigationId });
export const getChangeMap = (request: ChangeMapRequest): Promise<ChangeMapData> =>
  call('get_change_map', { request });

// ---- Collectors and monitoring ----

export const getCollectorStatus = (): Promise<CollectorStatusDto[]> => call('get_collector_status');

export const listCollectorStatus = (): Promise<CollectorStatusDto[]> =>
  call('list_collector_status');
export const setCollectorEnabled = (
  collectorId: string,
  enabled: boolean,
): Promise<CollectorStatusDto[]> => call('set_collector_enabled', { collectorId, enabled });
export const setMonitoringPaused = (paused: boolean, minutes?: number): Promise<MonitoringDto> =>
  call('set_monitoring_paused', { paused, minutes: minutes ?? null });

export const pauseMonitoring = (minutes?: number): Promise<MonitoringDto> =>
  call('pause_monitoring', { minutes: minutes ?? null });

export const resumeMonitoring = (): Promise<MonitoringDto> => call('resume_monitoring');

export const getMonitorState = (): Promise<MonitoringDto> => call('get_monitor_state');

// ---- Settings, folders and projects ----

export const getSettings = (): Promise<Settings> => call('get_settings');
export const updateSettings = (update: SettingsUpdate): Promise<Settings> =>
  call('update_settings', { update });
export const listWatchedLocations = (): Promise<WatchedLocationDto[]> =>
  call('list_watched_locations');
export const addWatchedLocation = (
  path: string,
  maxDepth?: number,
): Promise<WatchedLocationDto> =>
  call('add_watched_location', { path, maxDepth: maxDepth ?? null });
export const removeWatchedLocation = (locationId: number): Promise<void> =>
  call('remove_watched_location', { locationId });
export const listProjects = (): Promise<ProjectDto[]> => call('list_projects');
export const addProject = (path: string): Promise<ProjectDto> => call('add_project', { path });
export const removeProject = (projectId: string): Promise<void> =>
  call('remove_project', { projectId });
export const markProjectWorking = (projectId: string): Promise<ProjectDto> =>
  call('mark_project_working', { projectId });
export const clearProjectBaseline = (projectId: string): Promise<ProjectDto> =>
  call('clear_project_baseline', { projectId });

// ---- Storage and diagnostics ----

export const getStorageUsage = (): Promise<StorageUsage> => call('get_storage_usage');
export const getDiagnostics = (): Promise<DiagnosticsDto> => call('get_diagnostics');
export const exportDiagnostics = (): Promise<string> => call('export_diagnostics');
export const getPrivacySummary = (): Promise<PrivacySummaryDto> => call('get_privacy_summary');
export const requestPurgeChallenge = (): Promise<string> => call('request_purge_challenge');
export const purgeCategory = (category: string): Promise<PurgeReportDto> =>
  call('purge_category', { category });
export const purgeFolder = (path: string): Promise<PurgeReportDto> =>
  call('purge_folder', { path });
export const purgeAll = (confirmation: string, challenge: string): Promise<PurgeReportDto> =>
  call('purge_all', { confirmation, challenge });
export const setRetention = (retention: Settings['retention']): Promise<Settings> =>
  call('set_retention', { retention });
export const setQuietHours = (
  quietHours: { enabled: boolean; start_hour: number; end_hour: number },
): Promise<{ enabled: boolean; start_hour: number; end_hour: number }> =>
  call('set_quiet_hours', { quietHours });
export const getQuietHours = (): Promise<{ enabled: boolean; start_hour: number; end_hour: number }> =>
  call('get_quiet_hours');
export const dbRecover = (action: RecoveryAction): Promise<RecoveryOutcome> =>
  call('db_recover', { action });

// ---- Development only ----

/**
 * Write one synthetic event. The backend refuses in a shipped build, and the
 * call is behind a build flag here so a release bundle does not carry it.
 */
export const simulateEvent = (kind: string): Promise<number> =>
  import.meta.env.DEV ? call('simulate_event', { kind }) : Promise.reject(NO_BACKEND);
export const seedSynthetic = (profile: string, relations = false): Promise<number> =>
  import.meta.env.DEV
    ? call('seed_synthetic', { profile, relations })
    : Promise.reject(NO_BACKEND);
