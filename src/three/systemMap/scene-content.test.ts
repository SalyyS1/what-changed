import { afterEach, describe, expect, it, vi } from 'vitest';
import { createSystemMapContent, degreeSize } from './scene-content';
import { layoutSystemMap } from './layout';
import type { SystemMapDataLike, SystemMapRelationshipKind } from './types';

function data(kinds: readonly SystemMapRelationshipKind[] = ['listens_on', 'config_of']): SystemMapDataLike {
  const nodes = Array.from({ length: kinds.length + 1 }, (_, index) => ({
    entity_id: `process:${index}`,
    entity_type: 'process' as const,
    display_name: `Node ${index}`,
    hop: index,
    degree: index + 1,
    last_changed_at: null,
    recent_event_count: 0,
  }));
  return {
    nodes,
    edges: kinds.map((kind, index) => ({
      source: `process:${index}`,
      target: `process:${index + 1}`,
      kind,
      recorded_at: index,
      evidence_event_id: index,
    })),
    focus: null,
    truncated: false,
    total_nodes_available: nodes.length,
  };
}

describe('System Map scene content', () => {
  afterEach(() => vi.restoreAllMocks());

  it('uses exactly three bounded degree-size bins', () => {
    expect(new Set([-5, 0, 1, 2, 4, 5, 100_000].map(degreeSize))).toEqual(new Set([0.72, 1, 1.35]));
  });

  it('renders all five relationship kinds with distinct theme-driven styles', () => {
    const values: Record<string, string> = {
      '--map-relationship-listens-on': '#ff0000',
      '--map-relationship-opened-by': '#00ff00',
      '--map-relationship-config-of': '#0000ff',
      '--map-relationship-provides': '#ffff00',
      '--map-relationship-publishes': '#00ffff',
      '--map-category-process': '#808080',
    };
    vi.stubGlobal('getComputedStyle', () => ({ getPropertyValue: (name: string) => values[name] ?? '' }));
    const input = data(['listens_on', 'opened_by', 'config_of', 'provides', 'publishes']);
    const content = createSystemMapContent(input, layoutSystemMap(input), 'high');
    const colors = content.edges.geometry.getAttribute('color').array as Float32Array;
    const styles = Array.from({ length: 5 }, (_, index) => [...colors.slice(index * 6, index * 6 + 3)]);
    expect(new Set(styles.map((style) => style.join(','))).size).toBe(5);
    content.dispose();
  });

  it('dims non-neighbours and unrelated edges while retaining direct neighbours', () => {
    vi.stubGlobal('getComputedStyle', () => ({ getPropertyValue: () => '#808080' }));
    const input = data();
    const content = createSystemMapContent(input, layoutSystemMap(input), 'high');
    const beforeNodes = [...(content.nodes.instanceColor?.array as Float32Array)];
    const beforeEdges = [...(content.edges.geometry.getAttribute('color').array as Float32Array)];
    content.emphasize('process:0');
    const nodes = content.nodes.instanceColor?.array as Float32Array;
    const edges = content.edges.geometry.getAttribute('color').array as Float32Array;
    expect(nodes[0]).toBeGreaterThan(beforeNodes[0]!);
    expect(nodes[3]).toBeCloseTo(beforeNodes[3]!);
    expect(nodes[6]).toBeCloseTo(beforeNodes[6]! * 0.2);
    expect(edges[0]).toBeCloseTo(beforeEdges[0]!);
    expect(edges[6]).toBeCloseTo(beforeEdges[6]! * 0.12);
    content.dispose();
  });

  it('refreshes instance and edge colors after a theme change', () => {
    let theme = '#204060';
    vi.stubGlobal('getComputedStyle', () => ({ getPropertyValue: () => theme }));
    const input = data(['listens_on']);
    const content = createSystemMapContent(input, layoutSystemMap(input), 'low');
    const beforeNode = (content.nodes.instanceColor?.array as Float32Array)[0];
    const beforeEdge = (content.edges.geometry.getAttribute('color').array as Float32Array)[0];
    theme = '#80a0c0';
    content.refreshPalette();
    expect((content.nodes.instanceColor?.array as Float32Array)[0]).not.toBeCloseTo(beforeNode!);
    expect((content.edges.geometry.getAttribute('color').array as Float32Array)[0]).not.toBeCloseTo(beforeEdge!);
    content.dispose();
  });
});
