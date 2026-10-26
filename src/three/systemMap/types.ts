import type { SceneBounds } from '../shared/types';

export const SYSTEM_MAP_ENTITY_TYPES = [
  'process', 'port', 'service', 'app', 'driver', 'file', 'adapter',
  'container', 'image', 'docker_volume', 'docker_network', 'project', 'update', 'task', 'system',
] as const;

export const SYSTEM_MAP_RELATIONSHIP_KINDS = [
  'listens_on', 'opened_by', 'config_of', 'provides', 'publishes',
] as const;

export type SystemMapEntityType = (typeof SYSTEM_MAP_ENTITY_TYPES)[number];
export type SystemMapRelationshipKind = (typeof SYSTEM_MAP_RELATIONSHIP_KINDS)[number];

export interface EntityRefLike {
  readonly entity_id: string;
  readonly entity_type: SystemMapEntityType;
}

export interface SystemMapNodeLike extends EntityRefLike {
  readonly display_name: string;
  readonly hop: number;
  readonly degree: number;
  readonly last_changed_at: number | null;
  readonly recent_event_count: number;
}

export interface SystemMapEdgeLike {
  readonly source: string;
  readonly target: string;
  readonly kind: SystemMapRelationshipKind;
  readonly recorded_at: number;
  readonly evidence_event_id: number | null;
}

export interface SystemMapDataLike {
  readonly nodes: readonly SystemMapNodeLike[];
  readonly edges: readonly SystemMapEdgeLike[];
  readonly focus: EntityRefLike | null;
  readonly truncated: boolean;
  readonly total_nodes_available: number;
}

export interface SystemMapLayout {
  readonly positions: Float32Array;
  readonly ringOf: Uint8Array;
  readonly bounds: SceneBounds;
  readonly nodeIndexById: ReadonlyMap<string, number>;
}
