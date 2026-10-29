import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { CollectorStatusDto } from '../../lib/tauri/bindings';
import CollectorList from './CollectorList.svelte';

function row(overrides: Partial<CollectorStatusDto> = {}): CollectorStatusDto {
  return {
    id: 'process',
    enabled: true,
    state: 'running',
    descriptor: {
      id: 'process',
      display_name: 'Programs',
      what_it_watches: 'Application and background process starts and stops.',
      why_it_matters: 'Explains when software appeared, disappeared or restarted.',
      cost_hint: 'cheap',
      cadence: { kind: 'poll', interval_ms: 5_000 },
      requires_admin: false,
    },
    last_run_at: Date.now() - 60_000,
    last_duration_ms: 12,
    events_emitted: 1_234,
    consecutive_failures: 2,
    last_error: 'scrubbed failure',
    details: null,
    requires_admin: false,
    queue_drops: 3,
    failed_commit_rows: 4,
    cycle_admission_drops: 5,
    failed_cycles: 6,
    ...overrides,
  };
}

describe('CollectorList', () => {
  it('renders backend-authored copy, state, cost, cadence and loss telemetry', () => {
    render(CollectorList, { rows: [row()], ontoggle: vi.fn() });

    expect(screen.getByRole('heading', { name: 'Programs' })).toBeTruthy();
    expect(screen.getByText('Application and background process starts and stops.')).toBeTruthy();
    expect(screen.getByText('Explains when software appeared, disappeared or restarted.')).toBeTruthy();
    for (const copy of ['Running', 'Low impact', 'Checks every 5 s', 'Events accepted', '1,234', '2 consecutive failures', '3 queue drops', '4 rows lost after a failed database commit', '5 atomic cycles rejected before database admission', '6 admitted atomic cycles failed']) {
      expect(screen.getByText(copy)).toBeTruthy();
    }
  });

  it('shows the backend-scrubbed reason for a failed collector', () => {
    render(CollectorList, { rows: [row({ state: 'failed' })], ontoggle: vi.fn() });
    expect(screen.getByText('Last error: scrubbed failure')).toBeTruthy();
  });

  it('uses an accessible switch and reports the requested next state', async () => {
    const ontoggle = vi.fn();
    render(CollectorList, { rows: [row()], ontoggle });
    const toggle = screen.getByRole('switch', { name: 'Stop watching Programs' });
    await fireEvent.click(toggle);
    expect(ontoggle).toHaveBeenCalledWith('process', false);
  });

  it('disables unavailable and pending collectors with an explanation', () => {
    const unavailable = row({ id: 'docker', enabled: false, state: 'unavailable', descriptor: { ...row().descriptor, id: 'docker', display_name: 'Docker' } });
    const { rerender } = render(CollectorList, { rows: [unavailable], ontoggle: vi.fn() });
    expect((screen.getByRole('switch', { name: 'Watch Docker' }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText('Not available on this PC')).toBeTruthy();

    void rerender({ rows: [row()], pending: new Set(['process']), errors: { process: 'The change was not saved.' }, ontoggle: vi.fn() });
    expect(screen.getByText('Updating…')).toBeTruthy();
    expect(screen.getByRole('alert').textContent).toBe('The change was not saved.');
  });

  it('shows Docker absence truthfully while still allowing an enabled source to be turned off', () => {
    render(CollectorList, {
      rows: [row({
        id: 'docker',
        enabled: true,
        state: 'unavailable',
        descriptor: { ...row().descriptor, id: 'docker', display_name: 'Docker' },
        details: { wake_tier: null, docker_state: 'unavailable', docker_api_version: null },
      })],
      ontoggle: vi.fn(),
    });
    expect(screen.getByText("Docker isn't running. We'll start recording container changes when it does.")).toBeTruthy();
    expect((screen.getByRole('switch', { name: 'Stop watching Docker' }) as HTMLButtonElement).disabled).toBe(false);
  });
});
