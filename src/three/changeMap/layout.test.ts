import { describe, expect, it } from 'vitest';
import { layoutChangeMap } from './layout';
import { CHANGE_MAP_LANES, type ChangeMapDataLike, type ChangeMapNodeLike } from './types';

function node(index: number, overrides: Partial<ChangeMapNodeLike> = {}): ChangeMapNodeLike {
  return {
    id: `node:${index}`,
    event_id: index,
    timestamp_ms: index,
    category: 'system',
    lane: 'system',
    importance: 'informational',
    label: `Node ${index}`,
    chain_ids: [],
    chain_depth: 0,
    ...overrides,
  };
}

function data(nodes: readonly ChangeMapNodeLike[]): ChangeMapDataLike {
  return { nodes, edges: [], symptom_time_ms: 500, window_start_ms: 0, window_end_ms: 1000 };
}

describe('layoutChangeMap', () => {
  it('is byte-identical for identical input', () => {
    const input = data([node(1), node(2, { lane: 'process', chain_depth: 2, importance: 'critical' })]);
    const first = layoutChangeMap(input);
    const second = layoutChangeMap(input);
    expect([...new Uint8Array(first.positions.buffer)]).toEqual([...new Uint8Array(second.positions.buffer)]);
    expect([...new Uint8Array(first.scales.buffer)]).toEqual([...new Uint8Array(second.scales.buffer)]);
  });

  it('uses the documented fixed lane order and importance scales', () => {
    const nodes = CHANGE_MAP_LANES.map((lane, index) => node(index, { lane }));
    const result = layoutChangeMap(data(nodes));
    expect([...result.laneIndices]).toEqual(CHANGE_MAP_LANES.map((_, index) => index));
    for (let index = 1; index < CHANGE_MAP_LANES.length; index += 1) {
      expect(result.positions[index * 3 + 1]).toBeLessThan(result.positions[(index - 1) * 3 + 1] ?? 0);
    }
    expect(layoutChangeMap(data([
      node(0),
      node(1, { importance: 'notable' }),
      node(2, { importance: 'important' }),
      node(3, { importance: 'critical' }),
    ])).scales).toEqual(new Float32Array([0.6, 0.8, 1, 1.4]));
  });

  it.each([1, 3000])('produces finite fixed-size arrays for %i nodes', (count) => {
    const nodes = Array.from({ length: count }, (_, index) => node(index, {
      timestamp_ms: index * 10,
      lane: CHANGE_MAP_LANES[index % CHANGE_MAP_LANES.length] ?? 'system',
      chain_depth: index % 20,
    }));
    const result = layoutChangeMap({ ...data(nodes), window_end_ms: count * 10 });
    expect(result.positions).toHaveLength(count * 3);
    expect(result.scales).toHaveLength(count);
    expect([...result.positions].every(Number.isFinite)).toBe(true);
    expect(result.bounds.maxZ).toBeGreaterThanOrEqual(0);
  });
});
