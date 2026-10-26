export const CHANGE_MAP_LANES = [
  'process', 'port', 'config', 'service', 'application',
  'update', 'driver', 'network', 'startup', 'system',
] as const;

export type ChangeMapLane = (typeof CHANGE_MAP_LANES)[number];
export type ChangeMapImportance = 'informational' | 'notable' | 'important' | 'critical';
export type ChangeMapEdgeKind = 'temporal' | 'structural';

/** Structural mirrors of the generated Rust DTOs. Generated types are assignable to these. */
export interface ChangeMapNodeLike {
  readonly id: string;
  readonly event_id: number;
  readonly timestamp_ms: number;
  readonly category: string;
  readonly lane: ChangeMapLane;
  readonly importance: ChangeMapImportance;
  readonly label: string;
  readonly chain_ids: readonly string[];
  readonly chain_depth: number;
}

export interface ChangeMapEdgeLike {
  readonly id: string;
  readonly source: string;
  readonly target: string;
  readonly kind: ChangeMapEdgeKind;
  readonly chain_ids: readonly string[];
}

export interface ChangeMapDataLike {
  readonly nodes: readonly ChangeMapNodeLike[];
  readonly edges: readonly ChangeMapEdgeLike[];
  readonly symptom_time_ms: number | null;
  readonly window_start_ms: number;
  readonly window_end_ms: number;
}

export interface ChangeMapBounds {
  readonly minX: number;
  readonly maxX: number;
  readonly minY: number;
  readonly maxY: number;
  readonly minZ: number;
  readonly maxZ: number;
  readonly centerX: number;
  readonly centerY: number;
  readonly centerZ: number;
  readonly width: number;
  readonly height: number;
  readonly depth: number;
}

export interface ChangeMapLayout {
  readonly positions: Float32Array;
  readonly scales: Float32Array;
  readonly laneIndices: Uint8Array;
  readonly bounds: ChangeMapBounds;
  readonly nodeIndexById: ReadonlyMap<string, number>;
}

export interface ProjectedPoint {
  readonly x: number;
  readonly y: number;
  readonly visible: boolean;
}
