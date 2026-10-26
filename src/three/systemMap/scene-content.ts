import {
  BufferGeometry,
  Color,
  DynamicDrawUsage,
  Float32BufferAttribute,
  InstancedMesh,
  LineBasicMaterial,
  LineSegments,
  Matrix4,
  MeshLambertMaterial,
  SphereGeometry,
} from 'three';
import { readThreePalette, type PaletteToken } from '../shared/palette';
import type { QualityTier } from '../changeMap/quality';
import type {
  SystemMapDataLike,
  SystemMapEntityType,
  SystemMapLayout,
  SystemMapRelationshipKind,
} from './types';

const ENTITY_PALETTE_TOKENS: Readonly<Record<SystemMapEntityType, PaletteToken>> = {
  process: token('--map-category-process', '--accent'),
  port: token('--map-category-port', '--info'),
  service: token('--map-category-service', '--success'),
  app: token('--map-category-application', '--accent'),
  driver: token('--map-category-driver', '--danger'),
  file: token('--map-category-config', '--accent'),
  adapter: token('--map-category-network', '--info'),
  container: token('--map-category-system', '--text-secondary'),
  image: token('--map-category-system', '--text-secondary'),
  docker_volume: token('--map-category-system', '--text-secondary'),
  docker_network: token('--map-category-network', '--info'),
  project: token('--map-category-config', '--accent'),
  update: token('--map-category-update', '--warning'),
  task: token('--map-category-startup', '--warning'),
  system: token('--map-category-system', '--text-secondary'),
};

const EDGE_PALETTE_TOKENS: Readonly<Record<SystemMapRelationshipKind, PaletteToken>> = {
  listens_on: token('--map-relationship-listens-on', '--info'),
  opened_by: token('--map-relationship-opened-by', '--accent'),
  config_of: token('--map-relationship-config-of', '--warning'),
  provides: token('--map-relationship-provides', '--success'),
  publishes: token('--map-relationship-publishes', '--danger'),
};

export interface SystemMapRenderContent {
  readonly nodes: InstancedMesh<SphereGeometry, MeshLambertMaterial>;
  readonly edges: LineSegments<BufferGeometry, LineBasicMaterial>;
  refreshPalette(): void;
  emphasize(entityId: string | null): void;
  dispose(): void;
}

const matrix = new Matrix4();
const emphasisColor = new Color();

export function degreeSize(degree: number): number {
  if (!Number.isFinite(degree) || degree <= 1) return 0.72;
  return degree <= 4 ? 1 : 1.35;
}

export function createSystemMapContent(
  data: SystemMapDataLike,
  layout: SystemMapLayout,
  quality: QualityTier,
): SystemMapRenderContent {
  const segments = quality === 'low' ? [8, 6] as const : [12, 8] as const;
  const nodeGeometry = new SphereGeometry(1, segments[0], segments[1]);
  const nodeMaterial = new MeshLambertMaterial({ vertexColors: true });
  const nodes = new InstancedMesh(nodeGeometry, nodeMaterial, data.nodes.length);
  nodes.instanceMatrix.setUsage(DynamicDrawUsage);
  const baseNodeColors = new Float32Array(data.nodes.length * 3);

  for (let index = 0; index < data.nodes.length; index += 1) {
    const node = data.nodes[index];
    if (!node) continue;
    const offset = index * 3;
    const size = degreeSize(node.degree);
    matrix.makeScale(size, size, size);
    matrix.setPosition(
      layout.positions[offset] ?? 0,
      layout.positions[offset + 1] ?? 0,
      layout.positions[offset + 2] ?? 0,
    );
    nodes.setMatrixAt(index, matrix);
  }
  nodes.instanceMatrix.needsUpdate = true;

  const edgePositions: number[] = [];
  const edgeKinds: SystemMapRelationshipKind[] = [];
  const edgeEndpoints: Array<readonly [string, string]> = [];
  for (const edge of data.edges) {
    const source = layout.nodeIndexById.get(edge.source);
    const target = layout.nodeIndexById.get(edge.target);
    if (source === undefined || target === undefined) continue;
    appendPosition(edgePositions, layout.positions, source);
    appendPosition(edgePositions, layout.positions, target);
    edgeKinds.push(edge.kind);
    edgeEndpoints.push([edge.source, edge.target]);
  }
  const edgeGeometry = new BufferGeometry();
  edgeGeometry.setAttribute('position', new Float32BufferAttribute(edgePositions, 3));
  const edgeColorAttribute = new Float32BufferAttribute(new Float32Array(edgePositions.length), 3);
  const edgeColors = edgeColorAttribute.array as Float32Array;
  edgeGeometry.setAttribute('color', edgeColorAttribute);
  const edgeMaterial = new LineBasicMaterial({ vertexColors: true, transparent: true, opacity: 0.72 });
  const edges = new LineSegments(edgeGeometry, edgeMaterial);
  const baseEdgeColors = new Float32Array(edgePositions.length);
  const neighbours = buildNeighbours(edgeEndpoints);

  function refreshPalette(): void {
    const nodePalette = readThreePalette(ENTITY_PALETTE_TOKENS);
    const edgePalette = readThreePalette(EDGE_PALETTE_TOKENS);
    for (let index = 0; index < data.nodes.length; index += 1) {
      const node = data.nodes[index];
      if (!node) continue;
      copyColor(baseNodeColors, index * 3, nodePalette[node.entity_type]);
    }
    for (let index = 0; index < edgeKinds.length; index += 1) {
      const color = edgePalette[edgeKinds[index] ?? 'listens_on'];
      copyColor(baseEdgeColors, index * 6, color);
      copyColor(baseEdgeColors, index * 6 + 3, color);
    }
    emphasize(null);
  }

  function emphasize(entityId: string | null): void {
    const related = entityId === null ? undefined : neighbours.get(entityId);
    for (let index = 0; index < data.nodes.length; index += 1) {
      const node = data.nodes[index];
      if (!node) continue;
      const factor = entityId === null ? 1 : node.entity_id === entityId ? 1.35 : related?.has(node.entity_id) ? 1 : 0.2;
      const offset = index * 3;
      emphasisColor.setRGB(
        Math.min(1, (baseNodeColors[offset] ?? 0) * factor),
        Math.min(1, (baseNodeColors[offset + 1] ?? 0) * factor),
        Math.min(1, (baseNodeColors[offset + 2] ?? 0) * factor),
      );
      nodes.setColorAt(index, emphasisColor);
    }
    if (nodes.instanceColor) nodes.instanceColor.needsUpdate = true;
    for (let index = 0; index < edgeEndpoints.length; index += 1) {
      const endpoints = edgeEndpoints[index];
      const factor = entityId === null || endpoints?.includes(entityId) ? 1 : 0.12;
      const offset = index * 6;
      for (let component = 0; component < 6; component += 1) {
        edgeColors[offset + component] = (baseEdgeColors[offset + component] ?? 0) * factor;
      }
    }
    edgeColorAttribute.needsUpdate = true;
  }

  function dispose(): void {
    nodeGeometry.dispose();
    nodeMaterial.dispose();
    nodes.dispose();
    edgeGeometry.dispose();
    edgeMaterial.dispose();
  }

  refreshPalette();
  return { nodes, edges, refreshPalette, emphasize, dispose };
}

function buildNeighbours(edges: readonly (readonly [string, string])[]): ReadonlyMap<string, ReadonlySet<string>> {
  const result = new Map<string, Set<string>>();
  for (const [source, target] of edges) {
    addNeighbour(result, source, target);
    addNeighbour(result, target, source);
  }
  return result;
}

function addNeighbour(map: Map<string, Set<string>>, source: string, target: string): void {
  const values = map.get(source) ?? new Set<string>();
  values.add(target);
  map.set(source, values);
}

function appendPosition(output: number[], positions: Float32Array, index: number): void {
  const offset = index * 3;
  output.push(positions[offset] ?? 0, positions[offset + 1] ?? 0, positions[offset + 2] ?? 0);
}

function copyColor(output: Float32Array, offset: number, color: Color): void {
  output[offset] = color.r;
  output[offset + 1] = color.g;
  output[offset + 2] = color.b;
}

function token(primary: string, fallback: string): PaletteToken {
  return { token: primary, fallbackToken: fallback };
}
