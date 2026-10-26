import { describe, expect, it } from 'vitest';
import type { RelationshipKind, SystemMapEdge, SystemMapNode } from '../../lib/tauri/bindings';
import { relationshipLabel } from './copy';

const node: SystemMapNode = { entity_id: 'other:1', entity_type: 'process', display_name: 'Other thing', hop: 1, degree: 1, last_changed_at: null, recent_event_count: 0 };
const edge = (kind: RelationshipKind): SystemMapEdge => ({ source: 'source:1', target: 'target:1', kind, recorded_at: 1, evidence_event_id: 1 });

describe('relationshipLabel', () => {
  it.each([
    ['listens_on', 'Other thing was observed listening on this port', 'was observed listening on Other thing'],
    ['opened_by', 'Other thing was observed being opened by this process', 'was observed being opened by Other thing'],
    ['config_of', 'Other thing was observed configuring this', 'was observed configuring Other thing'],
    ['provides', 'Other thing was observed providing this service', 'was observed providing Other thing'],
    ['publishes', 'Other thing was observed publishing this port', 'was observed publishing Other thing'],
  ] satisfies Array<[RelationshipKind, string, string]>)('%s uses truthful copy in both directions', (kind, inbound, outbound) => {
    expect(relationshipLabel(edge(kind), node, 'inbound')).toBe(inbound);
    expect(relationshipLabel(edge(kind), node, 'outbound')).toBe(outbound);
  });
});
