import type { OwnerGraphWindow } from './types';

export interface OwnerPosition { readonly x: number; readonly y: number; }
export interface OwnerLayout { readonly positions: ReadonlyMap<string, OwnerPosition>; readonly width: number; readonly height: number; }
export function layoutOwnerGraph(graph: Pick<OwnerGraphWindow, 'nodes'>): OwnerLayout {
  const positions = new Map<string, OwnerPosition>();
  const columns = { capability: 0, rule: 1, scenario: 2, evidence: 3 };
  const heights = [0, 0, 0, 0];
  for (const node of graph.nodes) {
    const column = columns[node.kind];
    const row = heights[column] ?? 0;
    positions.set(node.id, { x: 24 + column * 252, y: 48 + row * 108 });
    heights[column] = row + 1;
  }
  return { positions, width: 1032, height: Math.max(320, Math.max(...heights) * 108 + 56) };
}

export function clampOwnerZoom(value: number): number { return Math.max(0.35, Math.min(2.5, value)); }
export function clampOwnerPan(value: number, extent: number): number { return Math.max(-extent, Math.min(extent, value)); }
