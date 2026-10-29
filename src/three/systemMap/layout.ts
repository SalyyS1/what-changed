import type { SystemMapDataLike, SystemMapLayout, SystemMapNodeLike } from './types';

export interface SystemMapLayoutOptions {
  readonly minimumNodeSpacing?: number;
  readonly ringSpacing?: number;
}

const TAU = Math.PI * 2;

/** Deterministic radial layout. Positions remain indexed by payload order for instancing and picking. */
export function layoutSystemMap(
  data: SystemMapDataLike,
  options: SystemMapLayoutOptions = {},
): SystemMapLayout {
  const minimumNodeSpacing = Math.max(3, options.minimumNodeSpacing ?? 4.5);
  const ringSpacing = Math.max(minimumNodeSpacing, options.ringSpacing ?? 8);
  const positions = new Float32Array(data.nodes.length * 3);
  const ringOf = new Uint8Array(data.nodes.length);
  const nodeIndexById = new Map<string, number>();
  const rings = new Map<number, Array<{ node: SystemMapNodeLike; index: number }>>();

  for (let index = 0; index < data.nodes.length; index += 1) {
    const node = data.nodes[index];
    if (!node) continue;
    const hop = Math.min(255, Math.max(0, Math.floor(node.hop)));
    ringOf[index] = hop;
    nodeIndexById.set(node.entity_id, index);
    const ring = rings.get(hop) ?? [];
    ring.push({ node, index });
    rings.set(hop, ring);
  }

  const orderedRings = [...rings.entries()].sort(([left], [right]) => left - right);
  let previousRadius = -ringSpacing;
  let maximumRadius = 0;
  for (const [hop, ring] of orderedRings) {
    ring.sort(compareRingNodes);
    const circumferenceRadius = ring.length <= 1 ? 0 : ring.length * minimumNodeSpacing / TAU;
    const radius = Math.max(circumferenceRadius, previousRadius + ringSpacing, hop * ringSpacing, 0);
    const angleStep = TAU / Math.max(1, ring.length);
    for (let order = 0; order < ring.length; order += 1) {
      const item = ring[order];
      if (!item) continue;
      const angle = -Math.PI / 2 + order * angleStep;
      const offset = item.index * 3;
      positions[offset] = Math.cos(angle) * radius;
      positions[offset + 1] = Math.sin(angle) * radius;
      positions[offset + 2] = 0;
    }
    previousRadius = radius;
    maximumRadius = Math.max(maximumRadius, radius);
  }

  const extent = Math.max(1, maximumRadius + minimumNodeSpacing / 2);
  return {
    positions,
    ringOf,
    nodeIndexById,
    bounds: {
      minX: -extent,
      maxX: extent,
      minY: -extent,
      maxY: extent,
      minZ: 0,
      maxZ: 0,
      centerX: 0,
      centerY: 0,
      centerZ: 0,
      width: extent * 2,
      height: extent * 2,
      depth: 0,
    },
  };
}

function compareRingNodes(
  left: { node: SystemMapNodeLike },
  right: { node: SystemMapNodeLike },
): number {
  return left.node.entity_type.localeCompare(right.node.entity_type)
    || left.node.display_name.localeCompare(right.node.display_name)
    || left.node.entity_id.localeCompare(right.node.entity_id);
}
