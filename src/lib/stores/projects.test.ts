import { describe, expect, it, vi } from 'vitest';
import type { Project } from './projects.svelte';
import { PROJECT_LIMIT, ProjectsStore } from './projects.svelte';

function project(id: number, baseline = false): Project {
  return {
    id: `project:${id}`,
    path: `C:\\code\\project-${id}`,
    name: `project-${id}`,
    branch: 'main',
    baseline_checkpoint_id: baseline ? id : null,
    baseline_checkpoint_name: baseline ? 'Working: project' : null,
    baseline_checkpoint_created_at: baseline ? 1_800_000_000_000 : null,
  };
}

describe('ProjectsStore', () => {
  it('uses the backend list after adding a project', async () => {
    const canonical = project(4);
    const list = vi.fn().mockResolvedValueOnce([]).mockResolvedValueOnce([canonical]);
    const add = vi.fn().mockResolvedValue(project(99));
    const store = new ProjectsStore({
      list,
      add,
      remove: vi.fn(),
      markWorking: vi.fn(),
      clearBaseline: vi.fn(),
      chooseDirectory: vi.fn().mockResolvedValue('C:\\code\\requested'),
    });

    await store.load();
    expect(await store.chooseAndAdd()).toBe(true);
    expect(add).toHaveBeenCalledWith('C:\\code\\requested');
    expect(store.rows).toEqual([canonical]);
  });

  it('refreshes the authoritative baseline after Mark working', async () => {
    const list = vi.fn().mockResolvedValueOnce([project(1)]).mockResolvedValueOnce([project(1, true)]);
    const markWorking = vi.fn().mockResolvedValue(project(1, true));
    const store = new ProjectsStore({
      list,
      add: vi.fn(),
      remove: vi.fn(),
      markWorking,
      clearBaseline: vi.fn(),
      chooseDirectory: vi.fn(),
    });

    await store.load();
    expect(await store.markWorking('project:1')).toBe(true);
    expect(markWorking).toHaveBeenCalledWith('project:1');
    expect(store.rows[0]?.baseline_checkpoint_id).toBe(1);
    expect(store.marking.size).toBe(0);
  });

  it('does nothing when project picker is cancelled and enforces the cap', async () => {
    const chooseDirectory = vi.fn().mockResolvedValue(null);
    const store = new ProjectsStore({
      list: vi.fn().mockResolvedValue(Array.from({ length: PROJECT_LIMIT }, (_, index) => project(index + 1))),
      add: vi.fn(),
      remove: vi.fn(),
      markWorking: vi.fn(),
      clearBaseline: vi.fn(),
      chooseDirectory,
    });

    await store.load();
    expect(store.atLimit).toBe(true);
    expect(await store.chooseAndAdd()).toBe(false);
    expect(chooseDirectory).not.toHaveBeenCalled();
  });

  it('notifies active investigation scope when the selected workspace is removed', async () => {
    const selected = project(3);
    const list = vi.fn().mockResolvedValueOnce([selected]).mockResolvedValueOnce([]);
    const store = new ProjectsStore({
      list,
      add: vi.fn(),
      remove: vi.fn().mockResolvedValue(undefined),
      markWorking: vi.fn(),
      clearBaseline: vi.fn(),
      chooseDirectory: vi.fn(),
    });
    const changes: Array<string | null> = [];
    store.onSelectionChange((id) => changes.push(id));
    await store.load();
    store.select(selected.id);
    await store.remove(selected.id);
    expect(changes).toEqual([selected.id, null]);
  });
});
