import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { empty, ready } from '../../lib/types/async-state';
import { deriveGlobalHealth, type HomeSnapshot } from '../../lib/stores/home.svelte';
import { investigation } from '../../lib/stores/investigation.svelte';
import type { CollectorStatusDto } from '../../lib/tauri/bindings';
import HomeScreen from './HomeScreen.svelte';
import { router } from '../../lib/stores/router.svelte';
import ProblemInput from './ProblemInput.svelte';

const snapshot: HomeSnapshot = {
  summary: {
    status_line: '2 recent changes may need attention.',
    today: { total: 8, notable: 2, important: 1, critical: 0 },
    top_important: [],
    monitoring: { paused: false, resume_at: null },
    last_checkpoint: null,
  },
  appStatus: {
    db: { state: 'ready', schema_version: 3 },
    monitoring: { paused: false, resume_at: null },
    tray_available: true,
    safe_mode: false,
    dropped_writes: 0,
  },
  collectors: [],
  health: 'active',
  startedAt: Date.now(),
};

describe('HomeScreen', () => {
  afterEach(() => {
    vi.restoreAllMocks();
    globalThis.location.hash = '#/';
  });

  it('makes the owner workspace discoverable for AI-built apps', async () => {
    const navigate = vi.spyOn(router, 'navigate').mockImplementation(() => {});
    render(HomeScreen, { data: empty<HomeSnapshot>('What Changed started recording at 1:20 AM.') });

    await fireEvent.click(screen.getByRole('button', { name: 'Open My apps' }));

    expect(navigate).toHaveBeenCalledWith(expect.objectContaining({ id: 'owner-apps' }));
  });

  it('renders the recording empty copy without adding generic filler', () => {
    render(HomeScreen, { data: empty<HomeSnapshot>('What Changed started recording at 1:20 AM.') });

    expect(screen.getByText('We just started watching. Come back after something changes.')).toBeTruthy();
    expect(screen.getByText('What Changed started recording at 1:20 AM.')).toBeTruthy();
    expect(screen.queryByText(/recording quietly/i)).toBeNull();
  });

  it('uses backend-authored status copy and keeps a blank investigation disabled', () => {
    render(HomeScreen, { data: ready(snapshot) });

    expect(screen.getByText('Active')).toBeTruthy();
    expect(screen.getByText('2 recent changes may need attention.')).toBeTruthy();
    expect(screen.getByText('Your history stays on this PC.')).toBeTruthy();
    expect((screen.getByRole('button', { name: 'Investigate' }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('hands off a nonblank symptom in memory without storing raw text', async () => {
    globalThis.localStorage.clear();
    globalThis.location.hash = '#/';
    render(ProblemInput);

    await fireEvent.input(screen.getByLabelText('Describe the symptom'), {
      target: { value: 'my api cannot connect' },
    });
    const button = screen.getByRole('button', { name: 'Investigate' }) as HTMLButtonElement;
    expect(button.disabled).toBe(false);
    await fireEvent.click(button);

    expect(investigation.draft).toBe('my api cannot connect');
    expect(globalThis.localStorage.getItem('what-changed:investigation-draft')).toBeNull();
    expect(globalThis.location.hash).toBe('#/investigations');
  });

  it('replaces ordinary history with recovery guidance', () => {
    render(HomeScreen, { data: ready<HomeSnapshot>({ ...snapshot, health: 'recovery' }) });

    expect(screen.getByRole('heading', { name: 'Recovery needed' })).toBeTruthy();
    expect(screen.getByRole('link', { name: 'Open diagnostics' }).getAttribute('href')).toBe('#/diagnostics');
    expect(screen.queryByText(snapshot.summary.status_line)).toBeNull();
  });

  it('treats an enabled degraded collector as globally degraded', () => {
    const collector: CollectorStatusDto = {
      id: 'process', enabled: true, state: 'degraded',
      descriptor: { id: 'process', display_name: 'Programs', what_it_watches: 'Processes.', why_it_matters: 'Explains restarts.', cost_hint: 'cheap', cadence: { kind: 'poll', interval_ms: 5_000 }, requires_admin: false },
      last_run_at: null, last_duration_ms: null, events_emitted: 0, consecutive_failures: 0,
      last_error: null, requires_admin: false, queue_drops: 1, failed_commit_rows: 0,
      details: null,
      cycle_admission_drops: 0, failed_cycles: 0,
    };

    expect(deriveGlobalHealth(snapshot.appStatus, [collector])).toBe('degraded');
    expect(deriveGlobalHealth(snapshot.appStatus, [{ ...collector, enabled: false }])).toBe('active');
  });
});
