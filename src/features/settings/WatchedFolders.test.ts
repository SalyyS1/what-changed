import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import {
  WATCHED_LOCATION_LIMIT,
  WatchedLocationsStore,
  type WatchedLocation,
} from '../../lib/stores/watched-locations.svelte';
import WatchedFolders from './WatchedFolders.svelte';

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

function createModel(rows: WatchedLocation[]) {
  return new WatchedLocationsStore({
    list: vi.fn().mockResolvedValue(rows),
    add: vi.fn(),
    remove: vi.fn(),
    chooseDirectory: vi.fn(),
  });
}

describe('WatchedFolders', () => {
  it('renders privacy copy, canonical paths and honest availability badges', async () => {
    const model = createModel([
      location(1),
      location(2, { path: 'E:\\offline-project', available: false }),
    ]);
    await model.load();
    render(WatchedFolders, { active: false, model });

    expect(screen.getByText('We record what changed, never the secret values.')).toBeTruthy();
    expect(screen.getByText('C:\\code\\project-1')).toBeTruthy();
    expect(screen.getByText('E:\\offline-project')).toBeTruthy();
    expect(screen.getByText('Available')).toBeTruthy();
    expect(screen.getByText('Unavailable')).toBeTruthy();
    expect(screen.getAllByRole('button', { name: /Remove watched folder/ })).toHaveLength(2);
  });

  it('removes one row, shows only that action pending and restores useful focus', async () => {
    let finishRemove!: () => void;
    const removePromise = new Promise<void>((resolve) => { finishRemove = resolve; });
    const rows = [location(1), location(2)];
    const list = vi.fn()
      .mockResolvedValueOnce(rows)
      .mockResolvedValueOnce(rows)
      .mockResolvedValueOnce([rows[1]]);
    const remove = vi.fn(() => removePromise);
    const model = new WatchedLocationsStore({
      list,
      add: vi.fn(),
      remove,
      chooseDirectory: vi.fn(),
    });
    await model.load();
    render(WatchedFolders, { active: true, model });
    await waitFor(() => expect(list).toHaveBeenCalledTimes(2));

    const first = screen.getByRole('button', { name: 'Remove watched folder C:\\code\\project-1' });
    const second = screen.getByRole('button', { name: 'Remove watched folder C:\\code\\project-2' });
    first.focus();
    await fireEvent.click(first);

    expect((first as HTMLButtonElement).disabled).toBe(true);
    expect((second as HTMLButtonElement).disabled).toBe(false);
    finishRemove();
    await waitFor(() => expect(screen.queryByText('C:\\code\\project-1')).toBeNull());
    expect(remove).toHaveBeenCalledWith(1);
    await waitFor(() => {
      expect(document.activeElement).toBe(
        screen.getByRole('button', { name: 'Remove watched folder C:\\code\\project-2' }),
      );
    });
  });

  it('keeps cancellation silent and disables add at the root cap', async () => {
    const capped = Array.from(
      { length: WATCHED_LOCATION_LIMIT },
      (_, index) => location(index + 1),
    );
    const model = createModel(capped);
    await model.load();
    render(WatchedFolders, { active: true, model });

    expect((screen.getByRole('button', { name: 'Add folder' }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText('The 20-folder limit has been reached.')).toBeTruthy();
    expect(screen.queryByRole('alert')).toBeNull();
  });
});
