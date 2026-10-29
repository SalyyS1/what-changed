import { Vector3, type Camera } from 'three';
import type { OwnerGraphWindow } from '../../lib/owner-graph/types';
import type { OwnerLayout3D } from './layout';

export interface OwnerLabel { readonly id: string; readonly label: string; readonly x: number; readonly y: number; readonly status: string | null; }
export function projectOwnerLabels(graph: OwnerGraphWindow, layout: OwnerLayout3D, camera: Camera, width: number, height: number, selectedId: string | null): OwnerLabel[] {
  const result: OwnerLabel[] = [];
  const nodes = [...graph.nodes].sort((a, b) => Number(b.id === selectedId) - Number(a.id === selectedId));
  for (const node of nodes) {
    if (result.length >= 48) break;
    const point = layout.positions.get(node.id); if (!point) continue;
    const projected = new Vector3(point.x, point.y + 0.3, point.z).project(camera);
    const x = (projected.x + 1) * width / 2; const y = (1 - projected.y) * height / 2;
    if (projected.z < -1 || projected.z > 1 || x < 0 || x > width || y < 0 || y > height) continue;
    if (node.id !== selectedId && result.some((label) => Math.abs(label.x - x) < 145 && Math.abs(label.y - y) < 44)) continue;
    result.push({ id: node.id, label: node.label, status: node.status, x, y });
  }
  return result;
}
