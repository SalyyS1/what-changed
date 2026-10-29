import { afterEach, describe, expect, it, vi } from 'vitest';
import { render } from '@testing-library/svelte';
import App from './App.svelte';

/**
 * Regression: a profile whose onboarding is unfinished mounted and unmounted
 * every screen in a tight loop (about 63 command round-trips a second, three
 * cores of CPU). `settings.load()` blanked `current`, `App` swapped the
 * onboarding flow for the home screen, and the flow's mount reloaded settings
 * again. This renders the real `App` against a mocked bridge and counts the
 * boot reads: they must stay a handful, with the flow still on screen.
 *
 * Every mocked read resolves on a macrotask so a regression shows up as a high
 * call count at the assertion rather than as a hung microtask loop.
 */

const later = <T>(value: T): Promise<T> =>
  new Promise((resolve) => setTimeout(() => resolve(value), 0));

const client = vi.hoisted(() => ({
  backendAvailable: vi.fn(() => true),
  getSettings: vi.fn(),
  getAppStatus: vi.fn(),
  getCollectorStatus: vi.fn(),
  getDashboardSummary: vi.fn(),
  listWatchedLocations: vi.fn(),
  setTrayForcedColors: vi.fn(),
}));

vi.mock('../lib/tauri/client', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../lib/tauri/client')>()),
  ...client,
}));

vi.mock('../lib/tauri/events', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/tauri/events')>();
  const quiet = Object.fromEntries(
    Object.keys(actual)
      .filter((name) => /^on[A-Z]/.test(name))
      .map((name) => [name, async () => () => {}]),
  );
  return { ...actual, ...quiet };
});

const SETTINGS = {
  theme: 'dark' as const,
  retention: { events_days: 30, snapshots_days: 7, important_days: 90 },
  monitoring_profile: 'balanced' as const,
  collectors: { process: true },
  onboarding_completed: false,
  developer_mode: false,
};

const APP_STATUS = {
  db: { state: 'ready' as const, schema_version: 15 },
  monitoring: { paused: false, resume_at: null },
  tray_available: true,
  safe_mode: false,
  dropped_writes: 0,
};

afterEach(() => {
  window.location.hash = '';
  vi.clearAllMocks();
});

describe('App with onboarding unfinished', () => {
  it('reads settings a bounded number of times and keeps the onboarding flow mounted', async () => {
    client.getSettings.mockImplementation(() => later(SETTINGS));
    client.getAppStatus.mockImplementation(() => later(APP_STATUS));
    client.getCollectorStatus.mockImplementation(() => later([]));
    client.listWatchedLocations.mockImplementation(() => later([]));
    client.setTrayForcedColors.mockImplementation(() => later(undefined));
    client.getDashboardSummary.mockImplementation(() =>
      later({
        today: { total: 0, notable: 0, important: 0, critical: 0 },
        status_line: '',
        top_important: [],
        monitoring: { paused: false, resume_at: null },
        last_checkpoint: null,
      }),
    );

    const { findByTestId } = render(App);
    await findByTestId('onboarding-flow');
    await new Promise((resolve) => setTimeout(resolve, 400));

    // One read at boot and one when the flow mounts. The loop made hundreds.
    expect(client.getSettings.mock.calls.length).toBeLessThanOrEqual(4);
    expect(client.listWatchedLocations.mock.calls.length).toBeLessThanOrEqual(4);
    expect(client.getDashboardSummary).not.toHaveBeenCalled();
    expect(await findByTestId('onboarding-flow')).toBeTruthy();
  }, 20_000);
});
