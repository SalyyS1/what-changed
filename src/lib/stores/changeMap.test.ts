import { describe, expect, it, vi } from 'vitest';
import type { ChangeMapData } from '../tauri/bindings';
import { ChangeMapStore } from './changeMap.svelte';

const data = (id: number): ChangeMapData => ({
  nodes: [{ id: `event:${id}`, event_id: id, timestamp_ms: 10, category: 'process', lane: 'process', importance: 'notable', label: 'API started', chain_ids: [], chain_depth: 0, is_suspect: true, snapshot_only: false }],
  edges: [], symptom_time_ms: 20, window_start_ms: 0, window_end_ms: 20,
  lanes: [{ id: 'process', label: 'Process' }], truncated: false,
  total_nodes: 1, omitted_nodes: 0, total_edges: 0, omitted_edges: 0,
});

describe('ChangeMapStore', () => {
  it('drops an older response after a newer contextual request', async () => {
    let release: ((value: ChangeMapData) => void) | undefined;
    const fetch = vi.fn()
      .mockReturnValueOnce(new Promise<ChangeMapData>((resolve) => { release = resolve; }))
      .mockResolvedValueOnce(data(2));
    const store = new ChangeMapStore({ fetch });
    const old = store.load({ kind: 'investigation', investigation_id: 1 });
    expect(await store.load({ kind: 'investigation', investigation_id: 2 })).toBe(true);
    release?.(data(1));
    expect(await old).toBe(false);
    expect(store.state.status === 'ready' && store.state.data.nodes[0]?.event_id).toBe(2);
  });
});
