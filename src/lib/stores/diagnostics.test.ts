import { afterEach, describe, expect, it, vi } from 'vitest';
import type { DiagnosticsDto } from '../tauri/bindings';
import { DiagnosticsStore } from './diagnostics.svelte';

const snapshot: DiagnosticsDto = {
  app_version: '0.1.0',
  os_version: '142.0.3595.94',
  db: { state: 'ready', schema_version: 3 },
  database_bytes: 4_096,
  schema_version: 3,
  collectors: [],
  dropped_writes: 0,
  failed_commit_rows: 0,
  cycle_admission_drops: 0,
  failed_cycles: 0,
  write_queue_depth: 0,
  current_working_set_bytes: 0,
  peak_working_set_bytes: 0,
  log_bytes: 0,
};

describe('DiagnosticsStore', () => {
  afterEach(() => vi.useRealTimers());

  it('subscribes before its first authoritative snapshot', async () => {
    const order: string[] = [];
    const store = new DiagnosticsStore({
      getSnapshot: vi.fn(async () => { order.push('refresh'); return snapshot; }),
      subscribe: vi.fn(async () => { order.push('subscribe'); return () => {}; }),
      canReconcile: () => false,
    });

    const detach = await store.attach();

    expect(order).toEqual(['subscribe', 'refresh']);
    await vi.waitFor(() => expect(store.state.status).toBe('ready'));
    expect(store.state).toEqual({ status: 'ready', data: snapshot });
    detach();
  });

  it('coalesces a notice storm into one in-flight and one trailing refresh', async () => {
    let notify = () => {};
    let finishInFlight!: (value: DiagnosticsDto) => void;
    const inFlight = new Promise<DiagnosticsDto>((resolve) => { finishInFlight = resolve; });
    const latest = { ...snapshot, dropped_writes: 7 };
    const getSnapshot = vi.fn()
      .mockResolvedValueOnce(snapshot)
      .mockReturnValueOnce(inFlight)
      .mockResolvedValueOnce(latest);
    const store = new DiagnosticsStore({
      getSnapshot,
      subscribe: vi.fn(async (handler) => { notify = handler; return () => {}; }),
      canReconcile: () => false,
    });
    const detach = await store.attach();
    await vi.waitFor(() => expect(store.state.status).toBe('ready'));

    notify();
    await Promise.resolve();
    expect(getSnapshot).toHaveBeenCalledTimes(2);
    for (let index = 0; index < 25; index += 1) notify();
    const drained = store.refresh();
    expect(getSnapshot).toHaveBeenCalledTimes(2);

    finishInFlight(snapshot);
    await drained;

    expect(getSnapshot).toHaveBeenCalledTimes(3);
    expect(store.state).toEqual({ status: 'ready', data: latest });
    detach();
  });

  it('polls at a bounded rate and cleans up the poll and listener', async () => {
    vi.useFakeTimers();
    const unlisten = vi.fn();
    const getSnapshot = vi.fn().mockResolvedValue(snapshot);
    const store = new DiagnosticsStore({
      getSnapshot,
      subscribe: vi.fn().mockResolvedValue(unlisten),
      canReconcile: () => true,
    });

    const detach = await store.attach();
    expect(getSnapshot).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(getSnapshot).toHaveBeenCalledTimes(2);

    detach();
    detach();
    expect(unlisten).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(20_000);
    expect(getSnapshot).toHaveBeenCalledTimes(2);
  });

  it('detaches the listener and poll before an unresolved initial read settles', async () => {
    vi.useFakeTimers();
    let finishRead!: (value: DiagnosticsDto) => void;
    const pending = new Promise<DiagnosticsDto>((resolve) => { finishRead = resolve; });
    const unlisten = vi.fn();
    let notify = () => {};
    const getSnapshot = vi.fn().mockReturnValue(pending);
    const store = new DiagnosticsStore({
      getSnapshot,
      subscribe: vi.fn(async (handler) => { notify = handler; return unlisten; }),
      canReconcile: () => true,
    });
    const attached = vi.fn();
    void store.attach().then((detach) => { detach(); attached(); });
    await vi.advanceTimersByTimeAsync(0);

    expect(attached).toHaveBeenCalledTimes(1);
    expect(unlisten).toHaveBeenCalledTimes(1);
    expect(store.state.status).toBe('loading');
    notify();
    await vi.advanceTimersByTimeAsync(30_000);
    expect(getSnapshot).toHaveBeenCalledTimes(1);
    finishRead(snapshot);
    await vi.advanceTimersByTimeAsync(0);
    expect(getSnapshot).toHaveBeenCalledTimes(1);
  });

  it('keeps a successful snapshot visible and reports a failed reconciliation', async () => {
    const getSnapshot = vi.fn()
      .mockResolvedValueOnce(snapshot)
      .mockRejectedValueOnce({ code: 'database', message: 'Diagnostics refresh failed.', retryable: true });
    const store = new DiagnosticsStore({
      getSnapshot,
      subscribe: vi.fn(),
      canReconcile: () => false,
    });

    await store.refresh();
    await store.refresh();

    expect(store.state).toEqual({ status: 'ready', data: snapshot });
    expect(store.refreshError).toBe('Diagnostics refresh failed.');
  });

  it('exposes an honest retry state when the initial read fails', async () => {
    const store = new DiagnosticsStore({
      getSnapshot: vi.fn().mockRejectedValue({ code: 'database', message: 'Local history is unavailable.', retryable: true }),
      subscribe: vi.fn(),
      canReconcile: () => false,
    });

    await store.refresh();

    expect(store.state.status).toBe('error');
    if (store.state.status !== 'error') throw new Error('expected error state');
    expect(store.state.message).toBe('Local history is unavailable.');
    expect(store.state.retry).toBeTypeOf('function');
  });
});
