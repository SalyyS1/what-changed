import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CollectorStatusDto } from '../tauri/bindings';

/**
 * The stores' failure branches, which is where they break silently.
 *
 * Every one of these asserts something a user would notice and no other test
 * covers: that a failed save does not blank a screen that still has data, that
 * a paused-until time survives the snake-to-camel crossing, that the close
 * button's label follows the tray's availability, and that a retryable error
 * reads as a warning rather than a failure.
 *
 * `client.ts` is mocked because the point is the store's behaviour around the
 * call, not the call. The stores are module-level singletons, so each test
 * re-imports them with `resetModules` to get a fresh one — otherwise the
 * fourth test in a file inherits the third one's state.
 */

const client = vi.hoisted(() => ({
  getSettings: vi.fn(),
  updateSettings: vi.fn(),
  getCollectorStatus: vi.fn(),
  setCollectorEnabled: vi.fn(),
  getAppStatus: vi.fn(),
  setMonitoringPaused: vi.fn(),
  backendAvailable: vi.fn(() => true),
  toAppError: (reason: unknown) =>
    typeof reason === 'object' && reason !== null && 'code' in reason
      ? reason
      : { code: 'database', message: 'Something went wrong.', retryable: true },
}));

vi.mock('../tauri/client', () => client);
vi.mock('../tauri/events', () => ({
  onMonitoringStateChanged: vi.fn(async () => () => {}),
  onCollectorStatusChanged: vi.fn(async () => () => {}),
}));

const SETTINGS = {
  theme: 'light' as const,
  retention: { events_days: 30, snapshots_days: 7, important_days: 90 },
  monitoring_profile: 'balanced' as const,
  collectors: { process: true, docker: false },
  onboarding_completed: false,
  developer_mode: false,
};

const REFUSED = { code: 'invalid_input' as const, message: 'No.', retryable: false };

beforeEach(() => {
  vi.resetModules();
  for (const fn of Object.values(client)) {
    if (typeof fn === 'function' && 'mockReset' in fn) {
      (fn as ReturnType<typeof vi.fn>).mockReset();
    }
  }
  client.backendAvailable.mockReturnValue(true);
});

afterEach(() => {
  localStorage.clear();
});

describe('the settings store', () => {
  it('loads into the ready face and exposes what it loaded', async () => {
    client.getSettings.mockResolvedValue(SETTINGS);
    const { settings } = await import('./settings.svelte');
    expect(settings.state.status).toBe('loading');
    await settings.load();
    expect(settings.state.status).toBe('ready');
    expect(settings.current?.theme).toBe('light');
  });

  it('keeps the loaded settings available while it reloads them', async () => {
    // Regression: `current` going null mid-reload unmounted the onboarding flow,
    // whose mount reloads settings, which looped forever on an unfinished profile.
    client.getSettings.mockResolvedValue({ ...SETTINGS, onboarding_completed: false });
    const { settings } = await import('./settings.svelte');
    await settings.load();
    expect(settings.current?.onboarding_completed).toBe(false);

    let release: (value: typeof SETTINGS) => void = () => {};
    client.getSettings.mockReturnValue(new Promise((resolve) => { release = resolve; }));
    const reloading = settings.load();
    expect(settings.current).not.toBeNull();
    expect(settings.state.status).toBe('ready');
    release({ ...SETTINGS, onboarding_completed: false });
    await reloading;
    expect(settings.current?.onboarding_completed).toBe(false);
  });

  it('applies the stored theme, so the database is read and not only written', async () => {
    // The defect this covers: Rust stored the theme and nothing read it back,
    // leaving localStorage as the only real source.
    client.getSettings.mockResolvedValue({ ...SETTINGS, theme: 'dark' });
    const { theme } = await import('../design/theme.svelte');
    const { settings } = await import('./settings.svelte');
    theme.set('light');
    await settings.load();
    expect(theme.mode).toBe('dark');
  });

  it('offers a retry only when the failure could pass next time', async () => {
    client.getSettings.mockRejectedValue({ ...REFUSED, retryable: true });
    const { settings } = await import('./settings.svelte');
    await settings.load();
    expect(settings.state.status).toBe('error');
    if (settings.state.status === 'error') {
      expect(settings.state.retry).toBeTypeOf('function');
    }

    client.getSettings.mockRejectedValue(REFUSED);
    await settings.load();
    if (settings.state.status === 'error') {
      expect(settings.state.retry).toBeUndefined();
    }
  });

  it('keeps the loaded settings on screen when a save fails', async () => {
    client.getSettings.mockResolvedValue(SETTINGS);
    client.updateSettings.mockRejectedValue(REFUSED);
    const { settings } = await import('./settings.svelte');
    await settings.load();

    expect(await settings.save({ theme: 'dark' })).toBe(false);
    expect(settings.state.status).toBe('ready');
    expect(settings.current?.theme).toBe('light');
    expect(settings.saving).toBe(false);
  });

  it('shows what the backend stored rather than what was sent', async () => {
    client.getSettings.mockResolvedValue(SETTINGS);
    // The backend clamped the request; the screen must show the clamp.
    client.updateSettings.mockResolvedValue({
      ...SETTINGS,
      retention: { events_days: 1, snapshots_days: 7, important_days: 90 },
    });
    const { settings } = await import('./settings.svelte');
    await settings.load();
    expect(
      await settings.save({ retention: { events_days: 0, snapshots_days: 7, important_days: 90 } }),
    ).toBe(true);
    expect(settings.current?.retention.events_days).toBe(1);
  });

  it('persists the selected monitoring profile and reflects the returned value', async () => {
    client.getSettings.mockResolvedValue(SETTINGS);
    client.updateSettings.mockResolvedValue({ ...SETTINGS, monitoring_profile: 'thorough' });
    const { settings } = await import('./settings.svelte');
    await settings.load();

    expect(await settings.save({ monitoring_profile: 'thorough' })).toBe(true);
    expect(client.updateSettings).toHaveBeenCalledWith({ monitoring_profile: 'thorough' });
    expect(settings.current?.monitoring_profile).toBe('thorough');
  });
});

describe('the monitoring store', () => {
  it('carries a timed pause across the wire without losing its deadline', async () => {
    // `resume_at` to `resumeAt`: a slip here reads as an indefinite pause,
    // which is a plausible-looking wrong answer rather than a visible break.
    const { monitoring } = await import('./monitoring.svelte');
    monitoring.apply({ paused: true, resume_at: 1_700_000_000_000 });
    expect(monitoring.paused).toBe(true);
    expect(monitoring.resumeAt).toBe(1_700_000_000_000);
    expect(monitoring.label).toBe('Paused');

    monitoring.apply({ paused: false, resume_at: null });
    expect(monitoring.resumeAt).toBeNull();
    expect(monitoring.label).toBe('Active');
  });

  it('says the window will quit exactly when there is no tray', async () => {
    const { monitoring } = await import('./monitoring.svelte');
    // Before the status is known, the shell must not claim either.
    expect(monitoring.closeWouldExit).toBe(false);

    client.getAppStatus.mockResolvedValue({
      db: { state: 'ready', schema_version: 2 },
      monitoring: { paused: false, resume_at: null },
      tray_available: false,
      safe_mode: false,
      dropped_writes: 0,
    });
    await monitoring.refresh();
    expect(monitoring.closeWouldExit).toBe(true);

    client.getAppStatus.mockResolvedValue({
      db: { state: 'ready', schema_version: 2 },
      monitoring: { paused: false, resume_at: null },
      tray_available: true,
      safe_mode: false,
      dropped_writes: 0,
    });
    await monitoring.refresh();
    expect(monitoring.closeWouldExit).toBe(false);
  });

  it('fails closed when a real Tauri backend cannot be reached', async () => {
    client.getAppStatus.mockRejectedValue(new Error('no bridge'));
    const { monitoring } = await import('./monitoring.svelte');
    await monitoring.refresh();
    expect(monitoring.status).toBeNull();
    expect(monitoring.recoveryRequired).toBe(true);
    expect(monitoring.label).toBe('Unknown');
  });

  it('keeps the Active preview face when no Tauri bridge exists by design', async () => {
    client.backendAvailable.mockReturnValue(false);
    client.getAppStatus.mockRejectedValue(new Error('no bridge'));
    const { monitoring } = await import('./monitoring.svelte');
    await monitoring.refresh();
    expect(monitoring.recoveryRequired).toBe(false);
    expect(monitoring.label).toBe('Active');
  });

  it('leaves its own state alone when the backend refuses a pause', async () => {
    client.setMonitoringPaused.mockRejectedValue(REFUSED);
    const { monitoring } = await import('./monitoring.svelte');
    await monitoring.setPaused(true);
    expect(monitoring.paused).toBe(false);
  });

  it('exposes pause progress and a scrubbed failure without changing status', async () => {
    let resolve!: (value: { paused: boolean; resume_at: number | null }) => void;
    client.setMonitoringPaused.mockImplementation(() => new Promise((done) => { resolve = done; }));
    const { monitoring } = await import('./monitoring.svelte');
    const pending = monitoring.setPaused(true);
    expect(monitoring.pending).toBe(true);
    resolve({ paused: true, resume_at: null });
    expect(await pending).toBe(true);
    expect(monitoring.pending).toBe(false);
    expect(monitoring.paused).toBe(true);

    client.setMonitoringPaused.mockRejectedValue(REFUSED);
    expect(await monitoring.setPaused(false)).toBe(false);
    expect(monitoring.paused).toBe(true);
    expect(monitoring.error).toBe('No.');
  });
});

describe('the collectors store', () => {
  it('keeps the rows on screen when a toggle fails', async () => {
    const rows: CollectorStatusDto[] = [
      {
        id: 'docker',
        enabled: false,
        state: 'disabled' as const,
        descriptor: {
          id: 'docker',
          display_name: 'Docker',
          what_it_watches: 'Containers, images, volumes and rebuilds.',
          why_it_matters: 'Explains when a local service changed because its container changed.',
          cost_hint: 'moderate',
          cadence: { kind: 'notify_plus_sweep', interval_ms: 300_000 },
          requires_admin: false,
        },
        last_run_at: null,
        last_duration_ms: null,
        events_emitted: 0,
        consecutive_failures: 0,
        last_error: null,
        details: null,
        requires_admin: false,
        queue_drops: 0,
        failed_commit_rows: 0,
        cycle_admission_drops: 0,
        failed_cycles: 0,
      },
    ];
    client.getCollectorStatus.mockResolvedValue(rows);
    client.setCollectorEnabled.mockRejectedValue(REFUSED);
    const { collectors } = await import('./collectors.svelte');
    await collectors.load();
    expect(collectors.rows).toHaveLength(1);

    expect(await collectors.setEnabled('docker', true)).toBe(false);
    expect(collectors.state.status).toBe('ready');
    expect(collectors.rows).toHaveLength(1);
  });

  it('reports an empty list rather than throwing before it has loaded', async () => {
    const { collectors } = await import('./collectors.svelte');
    expect(collectors.rows).toEqual([]);
  });
});

describe('the toast store', () => {
  it('reads a retryable failure as a warning and a final one as a danger', async () => {
    const { toasts } = await import('./toast.svelte');
    toasts.clear();
    toasts.fromError({ message: 'Try again.', retryable: true });
    toasts.fromError({ message: 'No.', retryable: false });
    expect(toasts.items.map((t) => t.tone)).toEqual(['warning', 'danger']);
  });

  it('keeps the newest four so the stack cannot outgrow the window', async () => {
    const { toasts } = await import('./toast.svelte');
    toasts.clear();
    for (let i = 1; i <= 6; i++) {
      toasts.push(`message ${i}`, 'neutral', 0);
    }
    expect(toasts.items).toHaveLength(4);
    expect(toasts.items[0]?.message).toBe('message 3');
    expect(toasts.items[3]?.message).toBe('message 6');
  });

  it('dismisses by id and tolerates an id that is already gone', async () => {
    const { toasts } = await import('./toast.svelte');
    toasts.clear();
    const id = toasts.push('one', 'neutral', 0);
    toasts.dismiss(id);
    expect(toasts.items).toHaveLength(0);
    toasts.dismiss(id);
    expect(toasts.items).toHaveLength(0);
  });

  it('dismisses itself after its lifetime', async () => {
    vi.useFakeTimers();
    const { toasts } = await import('./toast.svelte');
    toasts.clear();
    toasts.push('temporary', 'neutral', 100);
    expect(toasts.items).toHaveLength(1);
    vi.advanceTimersByTime(150);
    expect(toasts.items).toHaveLength(0);
    vi.useRealTimers();
  });
});
