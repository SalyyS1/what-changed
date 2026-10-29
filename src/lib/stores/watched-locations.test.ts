import { describe, expect, it, vi } from 'vitest';
import type { WatchedLocation } from './watched-locations.svelte';
import { WATCHED_LOCATION_LIMIT, WatchedLocationsStore } from './watched-locations.svelte';

function location(id: number, overrides: Partial<WatchedLocation> = {}): WatchedLocation {
  return {
    id,
    path: `C:\\code\\project-${id}`,
    enabled: true,
    available: true,
    max_depth: 6,
    added_at: 1_800_000_000_000 + id,
    ...overrides,
  };
}

describe('WatchedLocationsStore', () => {
  it('keeps the post-add list response authoritative', async () => {
    const canonical = location(4, { path: 'C:\\code\\Canonical' });
    const list = vi.fn()
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([canonical]);
    const add = vi.fn().mockResolvedValue(location(99, { path: 'C:\\code\\requested' }));
    const store = new WatchedLocationsStore({
      list,
      add,
      remove: vi.fn(),
      chooseDirectory: vi.fn().mockResolvedValue('C:\\code\\requested'),
    });

    await store.load();
    expect(await store.chooseAndAdd()).toBe(true);
    expect(add).toHaveBeenCalledWith('C:\\code\\requested');
    expect(store.rows).toEqual([canonical]);
    expect(store.adding).toBe(false);
  });

  it('does nothing and shows no error when the picker is cancelled', async () => {
    const add = vi.fn();
    const store = new WatchedLocationsStore({
      list: vi.fn().mockResolvedValue([]),
      add,
      remove: vi.fn(),
      chooseDirectory: vi.fn().mockResolvedValue(null),
    });

    await store.load();
    expect(await store.chooseAndAdd()).toBe(false);
    expect(add).not.toHaveBeenCalled();
    expect(store.error).toBeNull();
    expect(store.adding).toBe(false);
  });

  it('tracks removal by row and refreshes from the backend afterward', async () => {
    let finishRemove!: () => void;
    const removing = new Promise<void>((resolve) => { finishRemove = resolve; });
    const list = vi.fn()
      .mockResolvedValueOnce([location(1), location(2)])
      .mockResolvedValueOnce([location(2)]);
    const store = new WatchedLocationsStore({
      list,
      add: vi.fn(),
      remove: vi.fn(() => removing),
      chooseDirectory: vi.fn(),
    });
    await store.load();

    const result = store.remove(1);
    expect(store.removing.has(1)).toBe(true);
    expect(store.removing.has(2)).toBe(false);
    finishRemove();

    expect(await result).toBe(true);
    expect(store.rows.map((row) => row.id)).toEqual([2]);
    expect(store.removing.size).toBe(0);
  });

  it('preserves useful rows and exposes only the normalized backend error', async () => {
    const store = new WatchedLocationsStore({
      list: vi.fn().mockResolvedValue([location(1)]),
      add: vi.fn(),
      remove: vi.fn().mockRejectedValue({
        code: 'invalid_input',
        message: 'That folder could not be removed.',
        retryable: false,
      }),
      chooseDirectory: vi.fn(),
    });
    await store.load();

    expect(await store.remove(1)).toBe(false);
    expect(store.rows).toEqual([location(1)]);
    expect(store.error).toBe('That folder could not be removed.');
  });

  it('enforces the twenty-root cap before opening the picker', async () => {
    const chooseDirectory = vi.fn();
    const add = vi.fn();
    const store = new WatchedLocationsStore({
      list: vi.fn().mockResolvedValue(
        Array.from({ length: WATCHED_LOCATION_LIMIT }, (_, index) => location(index + 1)),
      ),
      add,
      remove: vi.fn(),
      chooseDirectory,
    });
    await store.load();

    expect(store.atLimit).toBe(true);
    expect(await store.chooseAndAdd()).toBe(false);
    expect(chooseDirectory).not.toHaveBeenCalled();
    expect(add).not.toHaveBeenCalled();
  });
});
