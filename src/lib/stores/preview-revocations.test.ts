import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import App from '../../app/App.svelte';
import type { PreviewRunnerStatusDto } from '../tauri/bindings';
import { deferred } from '../../features/owner-workspace/owner-workspace-fixtures';
import { monitoring } from './monitoring.svelte';
import { PreviewRevocationsStore, previewRevocations } from './preview-revocations.svelte';

function session(lifecycle = 'running', app_id = 'app-a', run_id = 'run-a'): PreviewRunnerStatusDto {
  return { app_id, run_id, lifecycle, roles: ['customer'], can_continue: false, mode: 'run', events: [], error: null, captures: [], capture_screenshots: false };
}
const detachments: (() => void)[] = [];
afterEach(() => {
  for (const detach of detachments.splice(0)) detach();
  vi.useRealTimers(); vi.restoreAllMocks();
});

describe('PreviewRevocationsStore native confirmation', () => {
  it('retains a cancellation acknowledgment until native status is terminal', async () => {
    const store = new PreviewRevocationsStore();
    const cancel = vi.fn(async () => {}); const status = vi.fn(async () => session());
    await store.revoke(session(), { cancel, status });
    expect(store.sessions).toMatchObject([{ app_id: 'app-a', run_id: 'run-a', busy: false }]);
    status.mockResolvedValue(session('cancelled'));
    await store.retry('app-a', 'run-a');
    expect(store.sessions).toEqual([]);
  });

  it('accepts native terminal confirmation even when cancellation IPC failed', async () => {
    const store = new PreviewRevocationsStore();
    await store.revoke(session(), { cancel: vi.fn().mockRejectedValue({ code: 'not_ready', message: 'IPC unavailable', retryable: true }), status: async () => session('interrupted') });
    expect(store.sessions).toEqual([]);
  });

  it.each([['app-b', 'run-a'], ['app-a', 'run-b']])('keeps the original handle when terminal status belongs to %s/%s', async (appId, runId) => {
    const store = new PreviewRevocationsStore();
    await store.revoke(session(), { cancel: async () => {}, status: async () => session('cancelled', appId, runId) });
    expect(store.sessions).toMatchObject([{ app_id: 'app-a', run_id: 'run-a' }]);
    expect(store.sessions[0]?.error).toContain('did not match');
  });

  it('removes only the confirmed app and run, preserving another app with the same run ID', async () => {
    const store = new PreviewRevocationsStore(); let stoppedApp: string | null = null;
    const cancel = vi.fn(async () => {});
    const status = vi.fn(async (appId: string, runId: string) => session(appId === stoppedApp ? 'cancelled' : 'running', appId, runId));
    await store.revoke(session(), { cancel, status });
    await store.revoke(session('running', 'app-b'), { cancel, status });
    stoppedApp = 'app-a'; await store.retry('app-a', 'run-a');
    expect(store.sessions).toMatchObject([{ app_id: 'app-b', run_id: 'run-a' }]);
    expect(status).toHaveBeenLastCalledWith('app-a', 'run-a');
  });

  it('deduplicates concurrent revocations and retains the originating native adapter', async () => {
    const store = new PreviewRevocationsStore(); const pending = deferred<void>();
    const cancel = vi.fn(() => pending.promise); const status = vi.fn(async () => session('cancelled'));
    const first = store.revoke(session(), { cancel, status });
    const otherCancel = vi.fn(async () => {});
    expect(store.revoke(session(), { cancel: otherCancel, status })).toBe(first);
    expect(cancel).toHaveBeenCalledTimes(1); expect(otherCancel).not.toHaveBeenCalled();
    pending.resolve(); await first;
    expect(status).toHaveBeenCalledExactlyOnceWith('app-a', 'run-a'); expect(store.sessions).toEqual([]);
  });

  it('uses capped retry delays without overlapping an in-flight cancellation', async () => {
    vi.useFakeTimers(); const store = new PreviewRevocationsStore(); const pending = deferred<void>();
    detachments.push(store.attach());
    const cancel = vi.fn(async () => {}).mockImplementationOnce(async () => {}).mockImplementationOnce(() => pending.promise);
    await store.revoke(session(), { cancel, status: async () => session() });
    await vi.advanceTimersByTimeAsync(1_000); expect(cancel).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(30_000); expect(cancel).toHaveBeenCalledTimes(2);
    pending.resolve(); await store.retry('app-a', 'run-a');
    for (const [delay, calls] of [[2_000, 3], [4_000, 4], [8_000, 5], [10_000, 6], [10_000, 7]] as const) {
      await vi.advanceTimersByTimeAsync(delay - 1); expect(cancel).toHaveBeenCalledTimes(calls - 1);
      await vi.advanceTimersByTimeAsync(1); expect(cancel).toHaveBeenCalledTimes(calls);
      expect(vi.getTimerCount()).toBe(1);
    }
  });

  it('stops timers and follow-up IPC on app detach, then resumes retained recovery on remount', async () => {
    vi.useFakeTimers(); const store = new PreviewRevocationsStore(); const pending = deferred<void>();
    const detach = store.attach(); const cancel = vi.fn(async () => {}).mockImplementationOnce(() => pending.promise);
    const status = vi.fn(async () => session('cancelled'));
    const revoking = store.revoke(session(), { cancel, status });
    detach(); detach(); pending.resolve(); await revoking;
    expect(status).not.toHaveBeenCalled(); expect(vi.getTimerCount()).toBe(0);
    expect(store.sessions).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(30_000); expect(cancel).toHaveBeenCalledTimes(1);
    detachments.push(store.attach()); await store.retry('app-a', 'run-a');
    expect(status).toHaveBeenCalledExactlyOnceWith('app-a', 'run-a'); expect(store.sessions).toEqual([]);
  });

  it('keeps accessible retry controls across app navigation and clears root retry timers on unmount', async () => {
    vi.useFakeTimers(); monitoring.resolved = true;
    vi.spyOn(monitoring, 'attach').mockResolvedValue(() => {});
    Object.defineProperty(window, 'matchMedia', { configurable: true, value: () => ({ matches: false, addEventListener() {}, removeEventListener() {} }) });
    const cancel = vi.fn(async () => {}).mockRejectedValue({ code: 'not_ready', message: 'Cancel IPC unavailable', retryable: true });
    const status = vi.fn(async () => session('running', 'app-root', 'run-root'));
    await previewRevocations.revoke(session('running', 'app-root', 'run-root'), { cancel, status });
    const view = render(App);
    try {
      await tick(); await previewRevocations.retry('app-root', 'run-root'); await tick();
      const banner = view.getByRole('region', { name: 'Preview permission revocation' });
      expect(within(banner).getByRole('button', { name: 'Retry revoke and check status' })).toBeTruthy();
      expect(within(banner).getByRole('alert').textContent).toContain('Cancel IPC unavailable');
      window.location.hash = '#/settings'; window.dispatchEvent(new HashChangeEvent('hashchange')); await tick();
      expect(view.getByRole('region', { name: 'Preview permission revocation' })).toBe(banner);
      view.unmount(); const calls = cancel.mock.calls.length;
      await vi.advanceTimersByTimeAsync(30_000); expect(cancel).toHaveBeenCalledTimes(calls);
    } finally {
      view.unmount(); status.mockResolvedValue(session('cancelled', 'app-root', 'run-root'));
      await previewRevocations.retry('app-root', 'run-root'); window.location.hash = '';
    }
  });
});
