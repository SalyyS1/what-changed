import type { ChangeMapDataLike, ChangeMapBounds, ChangeMapLayout, ChangeMapImportance } from './types';
import { CHANGE_MAP_LANES } from './types';

export interface LayoutOptions {
  readonly width?: number;
  readonly laneSpacing?: number;
  readonly depthSpacing?: number;
}

const IMPORTANCE_SCALE: Readonly<Record<ChangeMapImportance, number>> = {
  informational: 0.6,
  notable: 0.8,
  important: 1,
  critical: 1.4,
};

export function layoutChangeMap(data: ChangeMapDataLike, options: LayoutOptions = {}): ChangeMapLayout {
  const width = Math.max(1, options.width ?? 100);
  const halfWidth = width / 2;
  const laneSpacing = Math.max(0.01, options.laneSpacing ?? 8);
  const depthSpacing = Math.max(0.01, options.depthSpacing ?? 6);
  const positions = new Float32Array(data.nodes.length * 3);
  const scales = new Float32Array(data.nodes.length);
  const laneIndices = new Uint8Array(data.nodes.length);
  const nodeIndexById = new Map<string, number>();
  const span = Math.max(1, data.window_end_ms - data.window_start_ms);
  const laneCenter = (CHANGE_MAP_LANES.length - 1) / 2;
  let maxDepth = 0;

  for (let index = 0; index < data.nodes.length; index += 1) {
    const node = data.nodes[index];
    if (!node) continue;
    const laneIndex = Math.max(0, CHANGE_MAP_LANES.indexOf(node.lane));
    const normalizedTime = clamp((node.timestamp_ms - data.window_start_ms) / span, 0, 1);
    const offset = index * 3;
    positions[offset] = normalizedTime * width - halfWidth;
    positions[offset + 1] = (laneCenter - laneIndex) * laneSpacing;
    positions[offset + 2] = Math.max(0, node.chain_depth) * depthSpacing;
    scales[index] = IMPORTANCE_SCALE[node.importance] ?? IMPORTANCE_SCALE.informational;
    laneIndices[index] = laneIndex;
    nodeIndexById.set(node.id, index);
    maxDepth = Math.max(maxDepth, Math.max(0, node.chain_depth));
  }

  const halfHeight = laneCenter * laneSpacing;
  const bounds: ChangeMapBounds = {
    minX: -halfWidth,
    maxX: halfWidth,
    minY: -halfHeight,
    maxY: halfHeight,
    minZ: 0,
    maxZ: maxDepth * depthSpacing,
    centerX: 0,
    centerY: 0,
    centerZ: (maxDepth * depthSpacing) / 2,
    width,
    height: halfHeight * 2,
    depth: maxDepth * depthSpacing,
  };
  return { positions, scales, laneIndices, bounds, nodeIndexById };
}

export function symptomX(data: ChangeMapDataLike, width = 100): number | null {
  if (data.symptom_time_ms === null) return null;
  const span = Math.max(1, data.window_end_ms - data.window_start_ms);
  return clamp((data.symptom_time_ms - data.window_start_ms) / span, 0, 1) * width - width / 2;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
