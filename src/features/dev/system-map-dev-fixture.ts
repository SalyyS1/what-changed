import type { EntityType, RelationshipKind, SystemMapData } from '../../lib/tauri/bindings';

const kinds: EntityType[] = ['app', 'process', 'service', 'port', 'container', 'adapter', 'file'];
const relationships: RelationshipKind[] = ['provides', 'listens_on', 'opened_by', 'config_of', 'publishes'];

export function systemMapSample(count = 32): SystemMapData {
  const size = Math.max(1, Math.min(400, count));
  const nodes = Array.from({ length: size }, (_, index) => {
    const entity_type = kinds[index % kinds.length] ?? 'process';
    return {
      entity_id: `${entity_type}:dev-${index}`,
      entity_type,
      display_name: index === 0 ? 'Local API' : `${entity_type} ${index + 1}`,
      hop: index === 0 ? 0 : index < 8 ? 1 : index < 64 ? 2 : 3,
      degree: Math.max(1, Math.min(18, size - index)),
      last_changed_at: Date.now() - index * 60_000,
      recent_event_count: index % 7,
    };
  });
  const edges = nodes.slice(1).map((node, index) => ({
    source: nodes[Math.max(0, Math.floor(index / 3))]?.entity_id ?? nodes[0]!.entity_id,
    target: node.entity_id,
    kind: relationships[index % relationships.length] ?? 'provides',
    recorded_at: Date.now() - index * 3_600_000,
    evidence_event_id: index % 9 === 0 ? null : index + 1,
  }));
  return {
    nodes,
    edges,
    focus: { entity_id: nodes[0]!.entity_id, entity_type: nodes[0]!.entity_type },
    truncated: count > 400,
    total_nodes_available: count > 400 ? count : size,
    dropped_edges: 0,
    kind_availability: kinds.map((kind) => ({ kind, count: nodes.filter((node) => node.entity_type === kind).length, available: true, reason: null })),
    focused_recent_events: nodes.slice(0, 5).map((node, index) => ({
      event_id: index + 1,
      timestamp_ms: node.last_changed_at ?? Date.now(),
      summary: `${node.display_name} changed`,
      importance: index === 0 ? 'important' : 'notable',
    })),
  };
}
