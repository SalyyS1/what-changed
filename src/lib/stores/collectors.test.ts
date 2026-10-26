import { afterEach, describe, expect, it, vi } from 'vitest';
import type { CollectorStatusDto } from '../tauri/bindings';
import { CollectorsStore } from './collectors.svelte';

const original: CollectorStatusDto = {
  id: 'process', enabled: false, state: 'disabled',
  descriptor: { id: 'process', display_name: 'Programs', what_it_watches: 'Processes.', why_it_matters: 'Explains restarts.', cost_hint: 'cheap', cadence: { kind: 'poll', interval_ms: 5_000 }, requires_admin: false },
  last_run_at: null, last_duration_ms: null, events_emitted: 0, consecutive_failures: 0,
  last_error: null, requires_admin: false, queue_drops: 0, failed_commit_rows: 0,
  details: null,
  cycle_admission_drops: 0, failed_cycles: 0,
};

describe('CollectorsStore', () => {
  afterEach(() => vi.useRealTimers());

  it('keeps returned backend status authoritative while exposing pending state', async () => {
    let resolve!: (rows: CollectorStatusDto[]) => void;
    const response = new Promise<CollectorStatusDto[]>((done) => { resolve = done; });
    const setEnabled = vi.fn(() => response);
    const store = new CollectorsStore({ getStatus: vi.fn().mockResolvedValue([original]), setEnabled, subscribe: vi.fn() });
    await store.load();

    const pending = store.setEnabled('process', true);
    expect(store.pending.has('process')).toBe(true);
    expect(store.rows[0]?.enabled).toBe(false);
    resolve([{ ...original, enabled: true, state: 'running' }]);
    expect(await pending).toBe(true);
    expect(store.pending.has('process')).toBe(false);
    expect(store.rows[0]?.enabled).toBe(true);
  });

  it('preserves rows and exposes a scrubbed row error when a toggle fails', async () => {
    const store = new CollectorsStore({
      getStatus: vi.fn().mockResolvedValue([original]),
      setEnabled: vi.fn().mockRejectedValue({ code: 'database', message: 'The setting could not be saved.', retryable: true }),
      subscribe: vi.fn(),
    });
    await store.load();

    expect(await store.setEnabled('process', true)).toBe(false);
    expect(store.rows).toEqual([original]);
    expect(store.errors.process).toBe('The setting could not be saved.');
  });

  it('never invokes the backend for an unavailable collector', async () => {
    const setEnabled = vi.fn();
    const store = new CollectorsStore({ getStatus: vi.fn().mockResolvedValue([{ ...original, state: 'unavailable' }]), setEnabled, subscribe: vi.fn() });
    await store.load();
    expect(await store.setEnabled('process', true)).toBe(false);
    expect(setEnabled).not.toHaveBeenCalled();
  });

  it('keeps ready rows visible and ignores an older overlapping refresh', async () => {
    let resolveOld!: (rows: CollectorStatusDto[]) => void;
    const oldRefresh = new Promise<CollectorStatusDto[]>((done) => { resolveOld = done; });
    const newest = { ...original, enabled: true, state: 'running' as const, events_emitted: 9 };
    const getStatus = vi.fn()
      .mockResolvedValueOnce([original])
      .mockReturnValueOnce(oldRefresh)
      .mockResolvedValueOnce([newest]);
    const store = new CollectorsStore({ getStatus, setEnabled: vi.fn(), subscribe: vi.fn() });
    await store.load();

    const older = store.load();
    await Promise.resolve();
    expect(getStatus).toHaveBeenCalledTimes(2);
    expect(store.state.status).toBe('ready');
    const newer = store.load();
    resolveOld([{ ...original, events_emitted: 2 }]);
    await Promise.all([older, newer]);

    expect(store.rows[0]?.events_emitted).toBe(9);
    expect(store.state.status).toBe('ready');
  });

  it('loads authoritatively and polls when event subscription fails', async () => {
    vi.useFakeTimers();
    const getStatus = vi.fn().mockResolvedValue([original]);
    const store = new CollectorsStore({
      getStatus,
      setEnabled: vi.fn(),
      subscribe: vi.fn().mockRejectedValue(new Error('event transport unavailable')),
    });

    const detach = await store.attach();
    expect(store.rows).toEqual([original]);
    expect(getStatus).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(getStatus).toHaveBeenCalledTimes(2);
    detach();
    await vi.advanceTimersByTimeAsync(10_000);
    expect(getStatus).toHaveBeenCalledTimes(2);
  });

  it('subscribes before the initial authoritative load when events are available', async () => {
    const order: string[] = [];
    const store = new CollectorsStore({
      getStatus: vi.fn(async () => { order.push('load'); return [original]; }),
      setEnabled: vi.fn(),
      subscribe: vi.fn(async () => { order.push('subscribe'); return () => {}; }),
    });
    const detach = await store.attach();
    expect(order).toEqual(['subscribe', 'load']);
    detach();
  });

  it('announces meaningful autonomous health changes but not routine poll data', async () => {
    const healthy = { ...original, enabled: true, state: 'running' as const };
    const degraded = { ...healthy, state: 'degraded' as const, queue_drops: 2 };
    const routine = { ...degraded, last_run_at: Date.now(), events_emitted: 10 };
    const getStatus = vi.fn()
      .mockResolvedValueOnce([healthy])
      .mockResolvedValueOnce([degraded])
      .mockResolvedValueOnce([routine]);
    const store = new CollectorsStore({ getStatus, setEnabled: vi.fn(), subscribe: vi.fn() });

    await store.load();
    expect(store.announcement).toBe('');
    await store.load();
    expect(store.announcement).toContain('Programs is now degraded.');
    expect(store.announcement).toContain('2 new queue drops.');
    const announcement = store.announcement;
    await store.load();
    expect(store.announcement).toBe(announcement);
  });

  it('coalesces an event burst into one in-flight and one trailing refresh', async () => {
    vi.useFakeTimers();
    let handler = () => {};
    let resolveFirst!: (rows: CollectorStatusDto[]) => void;
    const firstRefresh = new Promise<CollectorStatusDto[]>((done) => { resolveFirst = done; });
    const healthy = { ...original, enabled: true, state: 'running' as const };
    const degraded = { ...healthy, state: 'degraded' as const, queue_drops: 3 };
    const getStatus = vi.fn()
      .mockResolvedValueOnce([healthy])
      .mockReturnValueOnce(firstRefresh)
      .mockResolvedValueOnce([degraded]);
    const store = new CollectorsStore({
      getStatus,
      setEnabled: vi.fn(),
      subscribe: vi.fn(async (next) => { handler = next; return () => {}; }),
    });
    const detach = await store.attach();

    handler();
    await Promise.resolve();
    expect(getStatus).toHaveBeenCalledTimes(2);
    for (let event = 0; event < 25; event += 1) handler();
    const drained = store.load();
    expect(getStatus).toHaveBeenCalledTimes(2);

    resolveFirst([healthy]);
    await drained;

    expect(getStatus).toHaveBeenCalledTimes(3);
    expect(store.rows).toEqual([degraded]);
    expect(store.announcement).toContain('Programs is now degraded.');
    expect(store.announcement).toContain('3 new queue drops.');
    detach();
  });

  it('does not announce running and idle churn or routine telemetry', async () => {
    const running = { ...original, enabled: true, state: 'running' as const };
    const idle = {
      ...running,
      state: 'idle' as const,
      last_run_at: 1_800_000_000_000,
      last_duration_ms: 8,
      events_emitted: 12,
    };
    const getStatus = vi.fn()
      .mockResolvedValueOnce([running])
      .mockResolvedValueOnce([idle])
      .mockResolvedValueOnce([{ ...running, events_emitted: 15 }]);
    const store = new CollectorsStore({ getStatus, setEnabled: vi.fn(), subscribe: vi.fn() });

    await store.load();
    await store.load();
    await store.load();

    expect(store.announcement).toBe('');
  });
});
