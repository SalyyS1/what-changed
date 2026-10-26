import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { diagnostics } from '../../lib/stores/diagnostics.svelte';
import { empty, failed, ready } from '../../lib/types/async-state';
import type { CollectorState, CollectorStatusDto, DiagnosticsDto } from '../../lib/tauri/bindings';
import DiagnosticsScreen from './DiagnosticsScreen.svelte';

function collector(state: CollectorState, index: number): CollectorStatusDto {
  const enabled = state !== 'disabled' && state !== 'unavailable';
  return {
    id: `collector-${index}`,
    enabled,
    state,
    descriptor: {
      id: `collector-${index}`,
      display_name: `${state[0]?.toUpperCase()}${state.slice(1)} collector`,
      what_it_watches: 'A local subsystem.',
      why_it_matters: 'Explains a related change.',
      cost_hint: 'moderate',
      cadence: index % 2 === 0
        ? { kind: 'poll', interval_ms: 5_000 }
        : { kind: 'notify_plus_sweep', interval_ms: 300_000 },
      requires_admin: false,
    },
    last_run_at: enabled ? 1_800_000_000_000 : null,
    last_duration_ms: enabled ? 14 : null,
    events_emitted: index === 4 ? 1_234 : 0,
    consecutive_failures: state === 'failed' ? 3 : 0,
    last_error: state === 'failed' || state === 'degraded' ? 'The latest check did not complete.' : null,
    details: null,
    requires_admin: false,
    queue_drops: state === 'degraded' ? 4 : 0,
    failed_commit_rows: state === 'degraded' ? 2 : 0,
    cycle_admission_drops: state === 'degraded' ? 5 : 0,
    failed_cycles: state === 'failed' ? 6 : 0,
  };
}

const states: CollectorState[] = ['running', 'idle', 'paused', 'failed', 'degraded', 'disabled', 'unavailable'];
const collectors = states.map(collector);
collectors[0] = {
  ...collectors[0]!,
  id: 'drivers',
  descriptor: {
    ...collectors[0]!.descriptor,
    id: 'drivers',
    display_name: 'Drivers',
  },
  details: { wake_tier: 'configuration_manager', docker_state: null, docker_api_version: null },
};
const snapshot: DiagnosticsDto = {
  app_version: '0.1.0',
  os_version: '142.0.3595.94',
  db: { state: 'ready', schema_version: 3 },
  database_bytes: 4_096,
  schema_version: 3,
  collectors,
  dropped_writes: 4,
  failed_commit_rows: 2,
  cycle_admission_drops: 5,
  failed_cycles: 6,
  write_queue_depth: 0,
  current_working_set_bytes: 0,
  peak_working_set_bytes: 0,
  log_bytes: 0,
};

describe('DiagnosticsScreen', () => {
  afterEach(() => vi.restoreAllMocks());

  it('renders the backend DTO and keeps row telemetry separate from atomic cycles', () => {
    const attach = vi.spyOn(diagnostics, 'attach').mockResolvedValue(() => {});
    render(DiagnosticsScreen, { data: ready(snapshot) });

    expect(attach).not.toHaveBeenCalled();
    expect(screen.getByText('WebView runtime')).toBeTruthy();
    expect(screen.getByText('142.0.3595.94')).toBeTruthy();
    expect(screen.getByText('4.0 KB')).toBeTruthy();
    expect(screen.getByRole('columnheader', { name: 'Wake tier' })).toBeTruthy();
    expect(screen.getByRole('columnheader', { name: 'Docker status' })).toBeTruthy();
    expect(screen.getByRole('columnheader', { name: 'Rows' })).toBeTruthy();
    expect(screen.getByRole('columnheader', { name: 'Atomic cycles' })).toBeTruthy();
    for (const label of ['Running', 'Idle', 'Paused', 'Failed', 'Degraded', 'Disabled', 'Unavailable']) {
      expect(screen.getByText(label)).toBeTruthy();
    }
    const drivers = screen.getByRole('row', { name: /Drivers/ });
    expect(within(drivers).getByText('Configuration Manager')).toBeTruthy();

    const degraded = screen.getByRole('row', { name: /Degraded collector/ });
    for (const value of ['4', '2', '5']) expect(within(degraded).getByText(value)).toBeTruthy();
    const failedRow = screen.getByRole('row', { name: /Failed collector/ });
    expect(within(failedRow).getByText('6')).toBeTruthy();
    expect(screen.getAllByText('The latest check did not complete.')).toHaveLength(2);

    const exportButton = screen.getByRole('button', { name: 'Export Diagnostics in the live app' }) as HTMLButtonElement;
    expect(exportButton.disabled).toBe(true);
  });

  it('shows when Docker is unavailable and explains that no API was negotiated', () => {
    const docker = {
      ...collector('unavailable', 0),
      id: 'docker',
      enabled: true,
      descriptor: {
        ...collector('unavailable', 0).descriptor,
        id: 'docker',
        display_name: 'Docker',
      },
      details: { wake_tier: null, docker_state: 'unavailable' as const, docker_api_version: null },
    };

    render(DiagnosticsScreen, { data: ready({ ...snapshot, collectors: [docker] }) });

    const row = screen.getByRole('rowheader', { name: 'Docker' }).closest('tr');
    expect(row).not.toBeNull();
    const dockerRow = within(row!);
    const details = dockerRow.getByRole('group', { name: 'Docker status details' });
    expect(within(details).getByText('Docker unavailable')).toBeTruthy();
    expect(within(details).getByText('API not negotiated')).toBeTruthy();
    expect(dockerRow.getByText('—')).toBeTruthy();
  });

  it('shows the negotiated Docker API version alongside an available connection', () => {
    const docker = {
      ...collector('running', 0),
      id: 'docker',
      descriptor: {
        ...collector('running', 0).descriptor,
        id: 'docker',
        display_name: 'Docker',
      },
      details: { wake_tier: null, docker_state: 'available' as const, docker_api_version: 'v1.43' },
    };

    render(DiagnosticsScreen, { data: ready({ ...snapshot, collectors: [docker] }) });

    const row = screen.getByRole('rowheader', { name: 'Docker' }).closest('tr');
    expect(row).not.toBeNull();
    const details = within(row!).getByRole('group', { name: 'Docker status details' });
    expect(within(details).getByText('Docker available')).toBeTruthy();
    expect(within(details).getByText('API v1.43')).toBeTruthy();
  });

  it('attaches only for live routing and tears down an attachment that resolves after unmount', async () => {
    diagnostics.state = ready(snapshot);
    let finishAttach!: (detach: () => void) => void;
    const pending = new Promise<() => void>((resolve) => { finishAttach = resolve; });
    const detach = vi.fn();
    const attach = vi.spyOn(diagnostics, 'attach').mockReturnValue(pending);

    const view = render(DiagnosticsScreen);
    await waitFor(() => expect(attach).toHaveBeenCalledTimes(1));
    view.unmount();
    finishAttach(detach);
    await pending;
    await Promise.resolve();

    expect(detach).toHaveBeenCalledTimes(1);
  });

  it('shows explicit empty and retryable error states without starting live IPC', async () => {
    const attach = vi.spyOn(diagnostics, 'attach').mockResolvedValue(() => {});
    const retry = vi.fn();
    const { rerender } = render(DiagnosticsScreen, { data: empty<DiagnosticsDto>() });

    expect(screen.getByText('Diagnostics are not available yet.')).toBeTruthy();
    await rerender({ data: failed<DiagnosticsDto>('The database did not answer.', retry) });
    expect(screen.getByRole('alert')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(retry).toHaveBeenCalledTimes(1);
    expect(attach).not.toHaveBeenCalled();
  });

  it('keeps runtime metadata visible when no collectors are registered', () => {
    render(DiagnosticsScreen, { data: ready({ ...snapshot, collectors: [] }) });
    expect(screen.getByText('No collectors are registered in this runtime.')).toBeTruthy();
    expect(screen.getByText('0.1.0')).toBeTruthy();
  });
});
