import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { monitoring } from '../../lib/stores/monitoring.svelte';
import { settings } from '../../lib/stores/settings.svelte';
import { loading, ready } from '../../lib/types/async-state';
import type { CollectorStatusDto, Settings } from '../../lib/tauri/bindings';
import SettingsScreen from './SettingsScreen.svelte';

const collector: CollectorStatusDto = {
  id: 'process', enabled: true, state: 'running',
  descriptor: { id: 'process', display_name: 'Programs', what_it_watches: 'Process starts and stops.', why_it_matters: 'Explains application restarts.', cost_hint: 'cheap', cadence: { kind: 'poll', interval_ms: 5_000 }, requires_admin: false },
  last_run_at: null, last_duration_ms: null, events_emitted: 0, consecutive_failures: 0,
  last_error: null, requires_admin: false, queue_drops: 0, failed_commit_rows: 0,
  details: null,
  cycle_admission_drops: 0, failed_cycles: 0,
};
const saved: Settings = {
  theme: 'dark', retention: { events_days: 30, snapshots_days: 7, important_days: 90 },
  visualization: 'auto',
  monitoring_profile: 'thorough', collectors: { process: true }, onboarding_completed: true, developer_mode: false,
};

describe('SettingsScreen monitoring controls', () => {
  beforeEach(() => {
    settings.state = loading();
    monitoring.resolved = false;
    monitoring.paused = false;
    monitoring.resumeAt = null;
    monitoring.error = null;
  });

  it('is honest and non-interactive until monitoring and settings resolve', () => {
    render(SettingsScreen, { data: ready([collector]) });

    expect(screen.getByText('Starting')).toBeTruthy();
    expect(screen.getByText('Checking the recorder status…')).toBeTruthy();
    expect((screen.getByRole('button', { name: 'Checking…' }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: 'Monitoring profile' }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText('Loading the saved profile…')).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Watched folders' })).toBeTruthy();
    expect(screen.getByText('We record what changed, never the secret values.')).toBeTruthy();
    expect((screen.getByRole('button', { name: 'Add folder' }) as HTMLButtonElement).disabled).toBe(true);
  });

  it('shows the saved profile and a timed pause without losing the deadline', () => {
    settings.state = ready(saved);
    monitoring.resolved = true;
    monitoring.paused = true;
    monitoring.resumeAt = new Date(2026, 8, 11, 21, 30).getTime();
    render(SettingsScreen, { data: ready([collector]) });

    expect(screen.getByText('Paused')).toBeTruthy();
    expect(screen.getByText(/Recording resumes at/)).toBeTruthy();
    expect(screen.getByText('Thorough')).toBeTruthy();
    expect(screen.getByText('Automatic')).toBeTruthy();
    expect(screen.getByText('Most frequent checks; uses more background resources.')).toBeTruthy();
  });

  it('keeps update checks opt-in and visibly unavailable until configured', () => {
    render(SettingsScreen, { data: ready([collector]) });

    expect(screen.getByRole('heading', { name: 'Updates' })).toBeTruthy();
    expect(screen.getByText('No background or automatic checks.')).toBeTruthy();
    expect(screen.getByText(/no release feed or signing key yet/i)).toBeTruthy();
    expect((screen.getByRole('checkbox', { name: /ask for update checks/i }) as HTMLInputElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: 'Check now' }) as HTMLButtonElement).disabled).toBe(true);
  });
});
