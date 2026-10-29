import { describe, expect, it } from 'vitest';
import { layoutSystemMap } from './layout';
import type { SystemMapDataLike, SystemMapNodeLike } from './types';

function node(index: number, hop: number, overrides: Partial<SystemMapNodeLike> = {}): SystemMapNodeLike {
  return {
    entity_id: `process:${index}`,
    entity_type: 'process',
    display_name: `Process ${String(index).padStart(3, '0')}`,
    hop,
    degree: 1,
    last_changed_at: null,
    recent_event_count: 0,
    ...overrides,
  };
}

function data(nodes: readonly SystemMapNodeLike[]): SystemMapDataLike {
  return {
    nodes,
    edges: [],
    focus: null,
    truncated: false,
    total_nodes_available: nodes.length,
  };
}

describe('System Map radial layout', () => {
  it('returns byte-identical arrays for identical input', () => {
    const input = data([
      node(0, 0),
      node(1, 1, { entity_type: 'service', display_name: 'Zulu' }),
      node(2, 1, { entity_type: 'app', display_name: 'Alpha' }),
    ]);
    const first = layoutSystemMap(input);
    const second = layoutSystemMap(input);
    expect(new Uint8Array(first.positions.buffer)).toEqual(new Uint8Array(second.positions.buffer));
    expect(new Uint8Array(first.ringOf.buffer)).toEqual(new Uint8Array(second.ringOf.buffer));
  });

  it('assigns ring index from hop distance and orders each ring by kind then display name', () => {
    const input = data([
      node(0, 2, { entity_type: 'service', display_name: 'Zulu' }),
      node(1, 0),
      node(2, 2, { entity_type: 'app', display_name: 'Beta' }),
      node(3, 2, { entity_type: 'app', display_name: 'Alpha' }),
    ]);
    const result = layoutSystemMap(input);
    expect([...result.ringOf]).toEqual([2, 0, 2, 2]);
    const angle = (index: number): number => Math.atan2(result.positions[index * 3 + 1]!, result.positions[index * 3]!);
    expect(angle(3)).toBeCloseTo(-Math.PI / 2);
    expect(angle(2)).toBeGreaterThan(angle(3));
    expect(angle(0)).toBeGreaterThan(angle(2));
  });

  it('does not overlap nodes at the 400-node cap', () => {
    const nodes = Array.from({ length: 400 }, (_, index) => node(index, index === 0 ? 0 : 1 + index % 3));
    const result = layoutSystemMap(data(nodes));
    for (let left = 0; left < nodes.length; left += 1) {
      for (let right = left + 1; right < nodes.length; right += 1) {
        const dx = result.positions[left * 3]! - result.positions[right * 3]!;
        const dy = result.positions[left * 3 + 1]! - result.positions[right * 3 + 1]!;
        expect(Math.hypot(dx, dy)).toBeGreaterThanOrEqual(4.49);
      }
    }
  });
});
