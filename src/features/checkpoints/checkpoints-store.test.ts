import { describe, expect, it, vi } from 'vitest';
import type { CheckpointDiffDto, CheckpointDto } from '../../lib/tauri/bindings';
import type { CheckpointProgressNotice } from '../../lib/tauri/client';
import { CheckpointsStore } from '../../lib/stores/checkpoints.svelte';

function checkpoint(id = 1, overrides: Partial<CheckpointDto> = {}): CheckpointDto {
  return {
    id, name: `Checkpoint ${id}`, kind: 'manual', reason: null, created_at: 1_800_000_000_000,
    item_count: 3000, duration_ms: 5010, partial: false, missing_collectors: [], ...overrides,
  };
}

const diff: CheckpointDiffDto = {
  added: [], removed: [], changed: [], grouped_by_category: [], event_ids_in_window: [],
  window_from: 1, window_to: 2, left_missing_collectors: [], right_missing_collectors: [], skipped_collectors: [],
};

function dependencies(overrides: Record<string, unknown> = {}) {
  return {
    list: vi.fn().mockResolvedValue([checkpoint()]),
    create: vi.fn().mockResolvedValue(checkpoint(2)),
    rename: vi.fn().mockResolvedValue(checkpoint()),
    remove: vi.fn().mockResolvedValue(undefined),
    compare: vi.fn().mockResolvedValue(diff),
    subscribe: vi.fn().mockResolvedValue(() => {}),
    subscribeSaved: vi.fn().mockResolvedValue(() => {}),
    ...overrides,
  };
}

describe('CheckpointsStore', () => {
  it('coalesces duplicate create clicks into one collector sweep', async () => {
    let finish!: (value: CheckpointDto) => void;
    const pending = new Promise<CheckpointDto>((resolve) => { finish = resolve; });
    const deps = dependencies({ create: vi.fn(() => pending) });
    const store = new CheckpointsStore(deps);

    const first = store.create('Before driver update', 'Updating GPU');
    const second = store.create('Duplicate', 'Duplicate');
    expect(first).toBe(second);
    expect(store.creating).toBe(true);
    expect(deps.create).toHaveBeenCalledTimes(1);
    finish(checkpoint(2));
    expect(await first).toEqual(checkpoint(2));
    expect(store.creating).toBe(false);
    expect(deps.list).toHaveBeenCalledTimes(1);
  });

  it('renders determinate collector progress from checkpoint-progress events', async () => {
    let progress!: (notice: CheckpointProgressNotice) => void;
    const deps = dependencies({
      subscribe: vi.fn(async (handler: (notice: CheckpointProgressNotice) => void) => { progress = handler; return () => {}; }),
    });
    const store = new CheckpointsStore(deps);
    await store.attachProgress();
    let finish!: (value: CheckpointDto) => void;
    deps.create.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
    const creating = store.create('', '');

    progress({ collector_id: 'process', collector_name: 'Programs', status: 'complete', completed: 1, total: 3 });
    progress({ collector_id: 'services', collector_name: 'Services', status: 'running', completed: 1, total: 3 });
    expect(store.progressCompleted).toBe(1);
    expect(store.progressTotal).toBe(3);
    expect(store.progress.map((item) => [item.label, item.state])).toEqual([['Programs', 'complete'], ['Services', 'running']]);
    finish(checkpoint(2));
    await creating;
  });

  it('loads a comparison and preserves missing-collector metadata on its selected row', async () => {
    const row = checkpoint(4, { partial: true, missing_collectors: ['drivers'] });
    const result: CheckpointDiffDto = {
      added: [{ category: 'application', entity_display_name: 'Toolbox', field: null, before: null, after: null }],
      removed: [], changed: [],
      grouped_by_category: [{ category: 'application', added: [{ category: 'application', entity_display_name: 'Toolbox', field: null, before: null, after: null }], removed: [], changed: [] }],
      event_ids_in_window: [41, 42], skipped_collectors: ['drivers'],
      window_from: 10, window_to: 20, left_missing_collectors: ['drivers'], right_missing_collectors: [],
    };
    const deps = dependencies({ list: vi.fn().mockResolvedValue([row]), compare: vi.fn().mockResolvedValue(result) });
    const store = new CheckpointsStore(deps);
    await store.load();
    expect(await store.compareToNow(4)).toBe(true);
    expect(store.selectedId).toBe(4);
    expect(store.comparison).toEqual(result);
    expect(deps.compare).toHaveBeenCalledWith(4);
  });

  it('refreshes when a checkpoint is saved outside the screen and removes synthetic heartbeat residue', async () => {
    let saved!: () => void;
    let progress!: (notice: CheckpointProgressNotice) => void;
    const deps = dependencies({
      subscribe: vi.fn(async (handler: (notice: CheckpointProgressNotice) => void) => { progress = handler; return () => {}; }),
      subscribeSaved: vi.fn(async (handler: () => void) => { saved = handler; return () => {}; }),
    });
    const store = new CheckpointsStore(deps);
    await store.attachProgress();

    let finish!: (value: CheckpointDto) => void;
    deps.create.mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
    const creating = store.create('', '');
    progress({ collector_id: 'capture', collector_name: 'Capturing system state', status: 'running', completed: 0, total: 3 });
    expect(store.progress.some((item) => item.collectorId === 'capture')).toBe(true);
    finish(checkpoint(2));
    await creating;
    expect(store.progress.some((item) => item.collectorId === 'capture')).toBe(false);

    saved();
    await vi.waitFor(() => expect(deps.list).toHaveBeenCalledTimes(2));
  });

  it('refreshes authoritative rows after rename and delete', async () => {
    const deps = dependencies({
      list: vi.fn().mockResolvedValueOnce([checkpoint(1)]).mockResolvedValueOnce([checkpoint(1, { name: 'Known good' })]).mockResolvedValueOnce([]),
    });
    const store = new CheckpointsStore(deps);
    await store.load();
    expect(await store.rename(1, '  Known good  ')).toBe(true);
    expect(deps.rename).toHaveBeenCalledWith(1, 'Known good');
    expect(store.rows[0]?.name).toBe('Known good');
    expect(await store.remove(1)).toBe(true);
    expect(store.state.status).toBe('empty');
  });
});
