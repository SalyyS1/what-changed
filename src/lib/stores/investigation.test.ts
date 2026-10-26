import { describe, expect, it, vi } from 'vitest';
import { InvestigationStore } from './investigation.svelte';

const result = (id = 42) => ({
  id, problem: 'api cannot connect', window: { kind: 'today' },
  resolved_window: { from: 100, to: 200 }, suspects: [],
  total_events_in_window: 3, created_at: 201, dropped_event_count: 0,
  no_cause_reason: 'NothingChanged',
});

function dependencies() {
  return {
    list: vi.fn(async () => [result()]),
    start: vi.fn(async () => result(43)),
    get: vi.fn(async (id: number) => result(id)),
    remove: vi.fn(async () => {}),
    subscribe: vi.fn(async () => () => {}),
  };
}

describe('InvestigationStore', () => {
  it('never persists raw problem text outside the sanitized backend', () => {
    globalThis.localStorage.clear();
    const store = new InvestigationStore(dependencies());
    store.setDraft('token sk-abcdefghijklmnopqrstuvwx');
    expect(store.draft).toContain('sk-');
    expect(globalThis.localStorage.getItem('what-changed:investigation-draft')).toBeNull();
    expect(JSON.stringify(globalThis.localStorage)).not.toContain('sk-');
  });

  it('loads raw DTOs and preserves numeric ids', async () => {
    const store = new InvestigationStore(dependencies());
    expect(await store.load()).toBe(true);
    expect(store.rows[0]?.id).toBe(42);
    expect(store.selected?.id).toBe(42);
  });

  it('deduplicates concurrent starts and selects the result', async () => {
    const deps = dependencies();
    const store = new InvestigationStore(deps);
    store.setDraft('api cannot connect');
    store.setWindow({ kind: 'since_last_restart' });
    const [first, second] = await Promise.all([store.start(), store.start()]);

    expect(deps.start).toHaveBeenCalledTimes(1);
    expect(deps.start).toHaveBeenCalledWith('api cannot connect', { kind: 'since_last_restart' }, null);
    expect(first?.id).toBe(43);
    expect(second?.id).toBe(43);
    expect(store.selected?.id).toBe(43);
  });

  it('does not let a stale history load overwrite a newer investigation', async () => {
    let release: ((rows: unknown[]) => void) | undefined;
    const deps = dependencies();
    deps.list = vi.fn(() => new Promise<unknown[]>((resolve) => { release = resolve; })) as typeof deps.list;
    const store = new InvestigationStore(deps);
    const loading = store.load();
    store.setDraft('api cannot connect');
    const started = await store.start();
    release?.([result(42)]);

    expect(await loading).toBe(false);
    expect(started?.id).toBe(43);
    expect(store.selected?.id).toBe(43);
  });

  it('deletes the selected row and falls back to the next history item', async () => {
    const deps = dependencies();
    deps.list = vi.fn(async () => [result(42), result(41)]);
    const store = new InvestigationStore(deps);
    await store.load();
    expect(await store.remove(42)).toBe(true);
    expect(store.selected?.id).toBe(41);
    expect(store.rows.map((row) => row.id)).toEqual([41]);
  });

  it('does not let a delayed completion notification overwrite a newer selection', async () => {
    let notify: ((id: number) => void) | undefined;
    let release: ((row: unknown) => void) | undefined;
    const deps = dependencies();
    deps.list = vi.fn(async () => [result(42), result(41)]);
    deps.get = vi.fn(() => new Promise<unknown>((resolve) => { release = resolve; })) as typeof deps.get;
    deps.subscribe = vi.fn(async (handler: (id: number) => void) => {
      notify = handler;
      return () => {};
    }) as unknown as typeof deps.subscribe;
    const store = new InvestigationStore(deps);
    await store.load();
    await store.attach();

    notify?.(40);
    expect(await store.select(41)).toBe(true);
    release?.(result(40));
    await Promise.resolve();

    expect(store.selected?.id).toBe(41);
    expect(store.rows.map((row) => row.id)).toEqual([42, 41]);
  });
});
