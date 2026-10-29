import { afterEach, describe, expect, it, vi } from 'vitest';
import type { AppStatusDto, MonitoringDto } from '../tauri/bindings';
import { MonitoringStore } from './monitoring.svelte';

function appStatus(monitoring: MonitoringDto): AppStatusDto {
  return {
    db: { state: 'ready', schema_version: 3 },
    monitoring,
    tray_available: true,
    safe_mode: false,
    dropped_writes: 0,
  };
}

describe('MonitoringStore subscriptions', () => {
  afterEach(() => vi.useRealTimers());

  it('subscribes first and does not let an older snapshot overwrite an event', async () => {
    const order: string[] = [];
    let handler: (state: MonitoringDto) => void = () => {};
    const unlisten = vi.fn();
    const store = new MonitoringStore({
      subscribe: vi.fn(async (next) => {
        order.push('subscribe');
        handler = next;
        return unlisten;
      }),
      getStatus: vi.fn(async () => {
        order.push('load');
        handler({ paused: true, resume_at: 1_800_000_000_000 });
        return appStatus({ paused: false, resume_at: null });
      }),
      setPaused: vi.fn(),
      backendAvailable: () => false,
    });

    const detach = await store.attach();

    expect(order).toEqual(['subscribe', 'load']);
    expect(store.paused).toBe(true);
    expect(store.resumeAt).toBe(1_800_000_000_000);
    expect(store.status?.monitoring).toEqual({
      paused: true,
      resume_at: 1_800_000_000_000,
    });
    expect(store.resolved).toBe(true);
    detach();
    expect(unlisten).toHaveBeenCalledOnce();
  });

  it('falls back to bounded reconciliation when subscription fails and cleans it up', async () => {
    vi.useFakeTimers();
    const getStatus = vi
      .fn<() => Promise<AppStatusDto>>()
      .mockResolvedValueOnce(appStatus({ paused: false, resume_at: null }))
      .mockResolvedValue(appStatus({ paused: true, resume_at: 1_800_000_000_000 }));
    const store = new MonitoringStore({
      subscribe: vi.fn().mockRejectedValue(new Error('event transport unavailable')),
      getStatus,
      setPaused: vi.fn(),
      backendAvailable: () => true,
    });

    const detach = await store.attach();
    expect(store.paused).toBe(false);
    expect(getStatus).toHaveBeenCalledOnce();

    await vi.advanceTimersByTimeAsync(10_000);
    expect(getStatus).toHaveBeenCalledTimes(2);
    expect(store.paused).toBe(true);
    expect(store.resumeAt).toBe(1_800_000_000_000);

    detach();
    detach();
    await vi.advanceTimersByTimeAsync(30_000);
    expect(getStatus).toHaveBeenCalledTimes(2);
  });
});
