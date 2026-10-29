import { BoxGeometry, BufferGeometry, Float32BufferAttribute, Group, LineBasicMaterial, LineSegments, Mesh, MeshBasicMaterial, Color } from 'three';
import type { OwnerGraphWindow } from '../../lib/owner-graph/types';
import type { OwnerLayout3D } from './layout';
import { disposeObjectTree } from '../shared/dispose';

function tokenColor(name: string): Color { return new Color(getComputedStyle(document.documentElement).getPropertyValue(name).trim() || 'gray'); }
export function createOwnerContent(graph: OwnerGraphWindow, layout: OwnerLayout3D) {
  const group = new Group();
  const geometry = new BoxGeometry(5.8, 0.22, 2.2);
  const meshes = graph.nodes.map((node) => {
    const mesh = new Mesh(geometry, new MeshBasicMaterial());
    const point = layout.positions.get(node.id)!;
    mesh.position.set(point.x, point.y, point.z); mesh.userData.ownerNodeId = node.id;
    group.add(mesh); return mesh;
  });
  const edgePositions: number[] = [];
  for (const edge of graph.edges) {
    const source = layout.positions.get(edge.source); const target = layout.positions.get(edge.target);
    if (source && target) edgePositions.push(source.x, source.y, source.z, target.x, target.y, target.z);
  }
  const edges = new LineSegments(new BufferGeometry().setAttribute('position', new Float32BufferAttribute(edgePositions, 3)), new LineBasicMaterial());
  group.add(edges);
  function emphasize(id: string | null): void {
    graph.nodes.forEach((node, index) => {
      const material = meshes[index]!.material;
      const stateToken = node.status === 'failed' ? '--danger' : node.status === 'blocked' || node.freshness === 'stale' ? '--warning' : node.status === 'passed' && node.freshness === 'current' ? '--success' : '--surface-3';
      material.color.copy(tokenColor(id === node.id ? '--text-secondary' : stateToken));
      meshes[index]!.scale.set(1, id === node.id ? 2 : 1, 1);
    });
    edges.material.color.copy(tokenColor('--text-muted'));
  }
  emphasize(null);
  return { group, meshes, emphasize, dispose: () => disposeObjectTree(group) };
}
