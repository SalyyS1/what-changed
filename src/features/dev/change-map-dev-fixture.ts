import type { ChangeMapData, ChangeMapLane, ChangeMapRequest } from '../../lib/tauri/bindings';

const laneIds: ChangeMapLane[] = [
  'process', 'port', 'config', 'service', 'application',
  'update', 'driver', 'network', 'startup', 'system',
];

export const changeMapDevRequest: ChangeMapRequest = { kind: 'investigation', investigation_id: 1 };

export function changeMapSample(count: number): ChangeMapData {
  return {
    nodes: Array.from({ length: count }, (_, index) => ({
      id: `event:${index + 1}`, event_id: index + 1, timestamp_ms: 1000 + index,
      category: index % 3 === 0 ? 'configuration' : index % 3 === 1 ? 'service' : 'network',
      lane: laneIds[index % laneIds.length] ?? 'system', importance: index === 0 ? 'critical' : 'notable',
      label: index === 0 ? 'PORT changed to 4000' : `Representative change ${index + 1}`,
      chain_ids: index < 3 ? ['chain:1'] : [], chain_depth: index < 3 ? index : 0,
      is_suspect: index === 0, snapshot_only: false,
    })),
    edges: Array.from({ length: Math.max(0, count - 1) }, (_, index) => ({
      id: `${index < 2 ? 'structural' : 'temporal'}:${index + 1}:${index + 2}`,
      source: `event:${index + 1}`, target: `event:${index + 2}`,
      kind: index < 2 ? 'structural' : 'temporal', chain_ids: index < 2 ? ['chain:1'] : [],
    })),
    symptom_time_ms: 1000 + count, window_start_ms: 1000, window_end_ms: 1000 + Math.max(1, count),
    lanes: laneIds.map((id) => ({ id, label: id[0]?.toUpperCase() + id.slice(1) })),
    truncated: count >= 3000, total_nodes: count >= 3000 ? 4800 : count,
    omitted_nodes: count >= 3000 ? 1800 : 0, total_edges: Math.max(0, count - 1), omitted_edges: 0,
  };
}
