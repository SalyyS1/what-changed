import type { OwnerGraphWindow, OwnerLayer } from '../../lib/owner-graph/types';

export interface OwnerPoint3D { readonly x: number; readonly y: number; readonly z: number; }
export interface OwnerLayout3D { readonly positions: ReadonlyMap<string, OwnerPoint3D>; readonly center: OwnerPoint3D; readonly extent: number; }
export function layoutOwnerWorkbench(graph: Pick<OwnerGraphWindow, 'nodes'>, separation = 6): OwnerLayout3D {
  const positions = new Map<string, OwnerPoint3D>();
  const capabilities = [...new Set(graph.nodes.map((node) => node.capabilityId))].sort();
  const rows = new Map<string, number>();
  const layerIndex: Record<OwnerLayer, number> = { intent: 2, scenarios: 1, evidence: 0 };
  let maxZ = 0;
  for (const node of graph.nodes) {
    const key = `${node.capabilityId}:${node.layer}`;
    const row = rows.get(key) ?? 0;
    rows.set(key, row + 1);
    const point = { x: capabilities.indexOf(node.capabilityId) * 9, y: layerIndex[node.layer] * separation, z: row * 3.2 };
    maxZ = Math.max(maxZ, point.z); positions.set(node.id, point);
  }
  const width = Math.max(6, (capabilities.length - 1) * 9 + 6);
  return { positions, center: { x: (width - 6) / 2, y: separation, z: maxZ / 2 }, extent: Math.max(width, maxZ + 8, separation * 3, 18) };
}
