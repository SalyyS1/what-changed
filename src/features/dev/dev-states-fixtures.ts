/**
 * Ready-branch fixtures for the dev states harness. Realistic shapes, not
 * lorem: the point of the harness is to judge how a screen looks with data
 * that resembles what the collectors will actually produce.
 */

import type { RouteId } from '../../app/routes';
import { empty, failed, loading, ready, type AsyncState } from '../../lib/types/async-state';
import { largeCheckpoints, largeTimeline } from './dev-large-fixtures';
import type { CollectorState, CollectorStatusDto, DiagnosticsDto, EventSummary } from '../../lib/tauri/bindings';
import { systemMapSample } from './system-map-dev-fixture';

const now = Date.now();
const event = (id: number, title: string, category: EventSummary['category'] = 'system'): EventSummary => ({
  id, timestamp: now - id * 60_000, category, event_type: 'updated', entity_type: category === 'driver' ? 'driver' : 'system',
  entity_id: `${category}:${id}`, entity_display_name: title.split(' ')[0] ?? 'System', severity: 'info',
  importance: category === 'driver' ? 'important' : 'notable', confidence: 'high', source_collector: 'synthetic',
  before: null, after: null, field: null, user_friendly_summary: title, occurred_at_is_uncertain: false,
});
const collector = (id: string, displayName: string, state: CollectorState, enabled: boolean, cost: 'cheap' | 'moderate' | 'expensive'): CollectorStatusDto => ({
  id,
  enabled,
  state,
  descriptor: {
    id,
    display_name: displayName,
    what_it_watches: id === 'docker' ? 'Containers, images, volumes and rebuilds.' : 'Application and background process starts and stops.',
    why_it_matters: id === 'docker' ? 'Explains when a local service changed because its container changed.' : 'Explains when software appeared, disappeared or restarted.',
    cost_hint: cost,
    cadence: id === 'docker' ? { kind: 'notify_plus_sweep', interval_ms: 300_000 } : { kind: 'poll', interval_ms: 5_000 },
    requires_admin: false,
  },
  last_run_at: state === 'disabled' || state === 'unavailable' ? null : now - 22_000,
  last_duration_ms: state === 'disabled' || state === 'unavailable' ? null : 14,
  events_emitted: enabled ? 1_284 : 0,
  consecutive_failures: state === 'failed' ? 3 : 0,
  last_error: state === 'failed' ? 'The collector could not complete its latest check.' : null,
  details: null,
  requires_admin: false,
  queue_drops: state === 'degraded' ? 4 : 0,
  failed_commit_rows: 0,
  cycle_admission_drops: 0,
  failed_cycles: 0,
});

const diagnosticsFixture: DiagnosticsDto = {
  app_version: '0.1.0',
  // The current backend field is misnamed: this value is the WebView runtime,
  // not Windows. DiagnosticsScreen labels it truthfully until the DTO changes.
  os_version: '142.0.3595.94',
  db: { state: 'ready', schema_version: 3 },
  database_bytes: 43_201_331,
  schema_version: 3,
  dropped_writes: 4,
  failed_commit_rows: 2,
  cycle_admission_drops: 1,
  failed_cycles: 1,
  write_queue_depth: 0,
  current_working_set_bytes: 0,
  peak_working_set_bytes: 0,
  log_bytes: 0,
  collectors: [
    collector('process', 'Programs', 'running', true, 'cheap'),
    collector('listeners', 'Listening ports', 'idle', true, 'cheap'),
    collector('services', 'Services', 'paused', true, 'moderate'),
    {
      ...collector('drivers', 'Drivers', 'degraded', true, 'expensive'),
      queue_drops: 4,
      failed_commit_rows: 2,
      cycle_admission_drops: 1,
      last_error: 'The latest database write did not accept every observation.',
    },
    {
      ...collector('eventlog', 'Windows event log', 'failed', true, 'moderate'),
      failed_cycles: 1,
    },
    collector('startup', 'Startup entries', 'disabled', false, 'expensive'),
    collector('docker', 'Docker', 'unavailable', false, 'moderate'),
  ],
};

/**
 * `large` is the sixth of the six states every screen must survive: the
 * same ready branch with thousands of rows. Screens that have no list fall
 * back to their ordinary ready data, so the column still renders.
 */
export type HarnessState = 'loading' | 'empty' | 'error' | 'ready' | 'large';
export const harnessStates: readonly HarnessState[] = ['loading', 'empty', 'error', 'ready', 'large'];

export type HarnessRoute = Exclude<RouteId, 'dev-states' | 'dev-perf' | 'dev-system-map-perf'>;

const readyData: Record<HarnessRoute, unknown> = {
  home: {
    summary: {
      status_line: '2 recent changes may need attention.',
      today: { total: 37, notable: 4, important: 2, critical: 0 },
      top_important: [event(1, 'Realtek audio driver updated 6.0.9 → 6.0.11', 'driver'), event(2, 'Windows Update KB5031356 installed', 'update')],
      monitoring: { paused: false, resume_at: null }, last_checkpoint: null,
    },
    appStatus: { db: { state: 'ready', schema_version: 3 }, monitoring: { paused: false, resume_at: null }, tray_available: true, safe_mode: false, dropped_writes: 0 },
    collectors: [], health: 'active', startedAt: now - 8 * 60 * 60 * 1000,
  },
  timeline: {
    items: [event(1, 'Realtek audio driver updated', 'driver'), event(2, 'Discord.exe started', 'process'), event(3, 'Port 3000 now listening', 'network'), event(4, 'Docker Desktop Service stopped', 'docker'), event(5, 'Windows Update KB5031356 installed', 'update')].map((item) => ({ kind: 'event', event: item })),
    loadedRows: 5, totalCount: 5, hasMore: false,
  },
  investigations: [
    {
      id: 101, problem: 'Discord stopped detecting my microphone this morning',
      window: { kind: 'today' }, from: now - 8 * 60 * 60_000, to: now,
      totalEvents: 37, createdAt: now - 12 * 60_000, droppedEvents: 0, noCauseReason: null,
      suspects: [{
        event: { id: 1, timestamp: now - 46 * 60_000, title: 'Realtek audio driver updated 6.0.9 → 6.0.11', entity: 'Discord' },
        evidence: 'confirmed',
        score: { total: 8.25, temporal: 0.7, relationship: 1, category: 1, severity: 0.6, rarity: 1, failureProximity: 1, chainBonus: 1.5 },
        explanation: {
          confidenceSentence: 'This change directly affected Discord.',
          relativeTime: '34 minutes before you noticed the problem',
          whyRanked: ['Updated the audio driver used by Discord', 'First time this driver changed in 90 days'],
          remediation: 'Check the selected microphone in Discord and Windows sound settings.',
        },
        chain: [
          { id: 1, title: 'Realtek audio driver updated', timestamp: now - 46 * 60_000 },
          { id: 6, title: 'Discord audio device stopped responding', timestamp: now - 45 * 60_000 },
        ],
      }],
    },
    {
      id: 100, problem: 'The API cannot connect on localhost',
      window: { kind: 'since_yesterday' }, from: now - 24 * 60 * 60_000, to: now,
      totalEvents: 5148, createdAt: now - 3 * 60 * 60_000, droppedEvents: 148,
      noCauseReason: 'only_routine_changes', suspects: [],
    },
  ],
  checkpoints: [
    { id: 1, name: 'Before GPU driver update', kind: 'manual', reason: 'About to update the graphics driver', created_at: now - 60 * 60_000, item_count: 2874, duration_ms: 4921, partial: false, missing_collectors: [] },
    { id: 2, name: 'Checkpoint Monday 2:00 AM', kind: 'automatic', reason: 'Before an installer started', created_at: now - 3 * 24 * 60 * 60_000, item_count: 2711, duration_ms: 8100, partial: true, missing_collectors: ['drivers'] },
  ],
  'system-map': systemMapSample(28),
  settings: [
    collector('process', 'Programs', 'running', true, 'cheap'),
    collector('docker', 'Docker', 'disabled', false, 'moderate'),
    collector('eventlog', 'Windows event log', 'failed', true, 'moderate'),
  ],
  diagnostics: diagnosticsFixture,
};

// Built once: two thousand rows is cheap to generate but not free, and the
// harness renders every screen's large cell on the same page.
const largeData: Partial<Record<HarnessRoute, unknown>> = {
  timeline: largeTimeline(),
  checkpoints: largeCheckpoints(),
  settings: Array.from({ length: 24 }, (_, index) => collector(
    `collector-${index + 1}`,
    `Collector ${index + 1}`,
    index % 7 === 0 ? 'failed' : index % 5 === 0 ? 'disabled' : 'running',
    index % 5 !== 0,
    index % 3 === 0 ? 'expensive' : index % 2 === 0 ? 'moderate' : 'cheap',
  )),
  investigations: Array.from({ length: 60 }, (_, index) => ({
    id: 1_000 - index,
    problem: `Investigation ${index + 1}: local service stopped responding`,
    window: { kind: 'today' }, from: now - 8 * 60 * 60_000, to: now,
    suspects: [], totalEvents: 18 + index, createdAt: now - index * 60_000,
    droppedEvents: index === 0 ? 123 : 0,
    noCauseReason: 'nothing_changed',
  })),
  'system-map': systemMapSample(1832),
};

export function fixtureFor(route: HarnessRoute, state: HarnessState): AsyncState<unknown> {
  switch (state) {
    case 'loading':
      return loading();
    case 'empty':
      return empty();
    case 'error':
      return failed('database is locked (SQLITE_BUSY, code 5)', () => {});
    case 'ready':
      return ready(readyData[route]);
    case 'large':
      return ready(largeData[route] ?? readyData[route]);
  }
}
