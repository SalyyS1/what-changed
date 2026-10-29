import {
  DynamicDrawUsage,
  InstancedMesh,
  Matrix4,
  MeshLambertMaterial,
  SphereGeometry,
} from 'three';
import type { ThreePalette } from '../shared/palette';
import type { ChangeMapLayout, ChangeMapNodeLike } from './types';
import type { ChangeMapLane } from './types';
import { QUALITY_SETTINGS, type QualityTier } from './quality';

export interface ChangeMapNodes {
  readonly mesh: InstancedMesh<SphereGeometry, MeshLambertMaterial>;
  readonly baseColors: Float32Array;
  applyEmphasis(chainIds: ReadonlySet<string> | null, selectedIndex: number | null): void;
  refreshColors(palette: ThreePalette<ChangeMapLane>): void;
  dispose(): void;
}

const matrix = new Matrix4();

export function createNodes(
  nodes: readonly ChangeMapNodeLike[],
  layout: ChangeMapLayout,
  palette: ThreePalette<ChangeMapLane>,
  quality: QualityTier,
): ChangeMapNodes {
  const settings = QUALITY_SETTINGS[quality];
  const geometry = new SphereGeometry(1, settings.sphereWidthSegments, settings.sphereHeightSegments);
  const material = new MeshLambertMaterial({ vertexColors: true });
  const mesh = new InstancedMesh(geometry, material, nodes.length);
  const baseColors = new Float32Array(nodes.length * 3);
  mesh.instanceMatrix.setUsage(DynamicDrawUsage);

  for (let index = 0; index < nodes.length; index += 1) {
    const offset = index * 3;
    const scale = layout.scales[index] ?? 1;
    matrix.makeScale(scale, scale, scale);
    matrix.setPosition(
      layout.positions[offset] ?? 0,
      layout.positions[offset + 1] ?? 0,
      layout.positions[offset + 2] ?? 0,
    );
    mesh.setMatrixAt(index, matrix);
  }
  mesh.instanceMatrix.needsUpdate = true;

  function refreshColors(nextPalette: ThreePalette<ChangeMapLane>): void {
    for (let index = 0; index < nodes.length; index += 1) {
      const node = nodes[index];
      if (!node) continue;
      const color = nextPalette[node.lane];
      const offset = index * 3;
      baseColors[offset] = color.r;
      baseColors[offset + 1] = color.g;
      baseColors[offset + 2] = color.b;
      mesh.setColorAt(index, color);
    }
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
  }

  function applyEmphasis(chainIds: ReadonlySet<string> | null, selectedIndex: number | null): void {
    const colors = mesh.instanceColor;
    if (!colors) return;
    for (let index = 0; index < nodes.length; index += 1) {
      const node = nodes[index];
      if (!node) continue;
      const inChain = !chainIds || node.chain_ids.some((id) => chainIds.has(id));
      const factor = inChain ? (index === selectedIndex ? 1.35 : 1) : 0.25;
      const offset = index * 3;
      colors.setXYZ(
        index,
        Math.min(1, (baseColors[offset] ?? 0) * factor),
        Math.min(1, (baseColors[offset + 1] ?? 0) * factor),
        Math.min(1, (baseColors[offset + 2] ?? 0) * factor),
      );
    }
    colors.needsUpdate = true;
  }

  function dispose(): void {
    geometry.dispose();
    material.dispose();
    mesh.dispose();
  }

  refreshColors(palette);
  return { mesh, baseColors, applyEmphasis, refreshColors, dispose };
}
