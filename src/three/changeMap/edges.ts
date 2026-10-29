import { BufferGeometry, Float32BufferAttribute, LineBasicMaterial, LineSegments } from 'three';
import type { ChangeMapDataLike, ChangeMapLayout } from './types';
import { QUALITY_SETTINGS, type QualityTier } from './quality';

export interface ChangeMapEdges {
  readonly temporal: LineSegments<BufferGeometry, LineBasicMaterial>;
  readonly structural: LineSegments<BufferGeometry, LineBasicMaterial>;
  emphasizeChains(chainIds: ReadonlySet<string> | null): void;
  dispose(): void;
}

export function createEdges(data: ChangeMapDataLike, layout: ChangeMapLayout, quality: QualityTier): ChangeMapEdges {
  const stride = QUALITY_SETTINGS[quality].temporalEdgeStride;
  const temporalValues: number[] = [];
  const structuralValues: number[] = [];
  let temporalIndex = 0;
  const structuralChains: string[][] = [];

  for (const edge of data.edges) {
    if (edge.kind === 'temporal' && temporalIndex++ % stride !== 0) continue;
    const source = layout.nodeIndexById.get(edge.source);
    const target = layout.nodeIndexById.get(edge.target);
    if (source === undefined || target === undefined) continue;
    const output = edge.kind === 'structural' ? structuralValues : temporalValues;
    appendPosition(output, layout.positions, source);
    appendPosition(output, layout.positions, target);
    if (edge.kind === 'structural') structuralChains.push([...edge.chain_ids]);
  }

  const temporalGeometry = geometryFrom(temporalValues);
  const structuralGeometry = geometryFrom(structuralValues);
  const structuralColorAttribute = new Float32BufferAttribute(new Float32Array(structuralValues.length).fill(1), 3);
  structuralGeometry.setAttribute('color', structuralColorAttribute);
  const temporalMaterial = new LineBasicMaterial({ color: 0xa5abb6, transparent: true, opacity: 0.18 });
  const structuralMaterial = new LineBasicMaterial({ color: 0x5b8cff, vertexColors: true, transparent: true, opacity: 0.78 });
  const temporal = new LineSegments(temporalGeometry, temporalMaterial);
  const structural = new LineSegments(structuralGeometry, structuralMaterial);

  function emphasizeChains(chainIds: ReadonlySet<string> | null): void {
    if (!chainIds) {
      structuralMaterial.opacity = 0.78;
      temporalMaterial.opacity = 0.18;
      (structuralColorAttribute.array as Float32Array).fill(1);
      structuralColorAttribute.needsUpdate = true;
      return;
    }
    const structuralColors = structuralColorAttribute.array as Float32Array;
    for (let segment = 0; segment < structuralChains.length; segment += 1) {
      const ids = structuralChains[segment] ?? [];
      const factor = ids.some((id) => chainIds.has(id)) ? 1 : 0.18;
      const offset = segment * 6;
      structuralColors.fill(factor, offset, offset + 6);
    }
    structuralColorAttribute.needsUpdate = true;
    structuralMaterial.opacity = 1;
    temporalMaterial.opacity = 0.045;
  }

  function dispose(): void {
    temporalGeometry.dispose();
    structuralGeometry.dispose();
    temporalMaterial.dispose();
    structuralMaterial.dispose();
  }
  return { temporal, structural, emphasizeChains, dispose };
}

function geometryFrom(values: number[]): BufferGeometry {
  const geometry = new BufferGeometry();
  geometry.setAttribute('position', new Float32BufferAttribute(values, 3));
  return geometry;
}

function appendPosition(output: number[], positions: Float32Array, index: number): void {
  const offset = index * 3;
  output.push(positions[offset] ?? 0, positions[offset + 1] ?? 0, positions[offset + 2] ?? 0);
}
