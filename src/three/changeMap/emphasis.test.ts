import { Color } from 'three';
import { describe, expect, it } from 'vitest';
import { createEdges } from './edges';
import { layoutChangeMap } from './layout';
import { createNodes } from './nodes';
import type { ChangeMapDataLike, ChangeMapLane } from './types';

const data: ChangeMapDataLike = {
  nodes: [
    { id: 'event:1', event_id: 1, timestamp_ms: 1, category: 'process', lane: 'process', importance: 'important', label: 'One', chain_ids: ['a'], chain_depth: 0 },
    { id: 'event:2', event_id: 2, timestamp_ms: 2, category: 'service', lane: 'service', importance: 'important', label: 'Two', chain_ids: ['a'], chain_depth: 1 },
    { id: 'event:3', event_id: 3, timestamp_ms: 3, category: 'network', lane: 'network', importance: 'important', label: 'Three', chain_ids: ['b'], chain_depth: 0 },
  ],
  edges: [
    { id: 'a', source: 'event:1', target: 'event:2', kind: 'structural', chain_ids: ['a'] },
    { id: 'b', source: 'event:2', target: 'event:3', kind: 'structural', chain_ids: ['b'] },
  ],
  symptom_time_ms: 3, window_start_ms: 0, window_end_ms: 4,
};

describe('Change Map chain emphasis', () => {
  it('dims unrelated instances to 25 percent while preserving the active chain', () => {
    const layout = layoutChangeMap(data);
    const palette = Object.fromEntries([
      'process', 'port', 'config', 'service', 'application', 'update', 'driver', 'network', 'startup', 'system',
    ].map((lane) => [lane, new Color('#808080')])) as Record<ChangeMapLane, Color>;
    const nodes = createNodes(data.nodes, layout, palette, 'high');
    nodes.applyEmphasis(new Set(['a']), 0);
    const colors = nodes.mesh.instanceColor?.array as Float32Array;
    expect(colors[6]).toBeCloseTo(nodes.baseColors[6]! * 0.25);
    expect(colors[3]).toBeCloseTo(nodes.baseColors[3]!);
    expect(colors[0]).toBeGreaterThan(nodes.baseColors[0]!);
    nodes.dispose();
  });

  it('keeps active structural edges bright and dims unrelated edges', () => {
    const edges = createEdges(data, layoutChangeMap(data), 'high');
    edges.emphasizeChains(new Set(['a']));
    const colors = edges.structural.geometry.getAttribute('color').array as Float32Array;
    expect(colors[0]).toBe(1);
    expect(colors[6]).toBeCloseTo(0.18);
    expect(edges.structural.material.opacity).toBe(1);
    expect(edges.temporal.material.opacity).toBe(0.045);
    edges.dispose();
  });
});
