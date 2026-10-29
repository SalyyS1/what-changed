import type { EntityType, RelationshipKind, SystemMapEdge, SystemMapNode } from '../../lib/tauri/bindings';

export const systemMapKinds = ['app', 'process', 'service', 'port', 'container', 'adapter', 'file'] as const satisfies readonly EntityType[];

export const entityKindLabel: Record<(typeof systemMapKinds)[number], string> = {
  app: 'Applications',
  process: 'Processes',
  service: 'Services',
  port: 'Ports',
  container: 'Containers',
  adapter: 'Network adapters',
  file: 'Config files',
};

export function relationshipLabel(edge: SystemMapEdge, other: SystemMapNode, direction: 'inbound' | 'outbound'): string {
  const name = other.display_name;
  const phrases: Record<RelationshipKind, Record<'inbound' | 'outbound', string>> = {
    listens_on: { inbound: `${name} was observed listening on this port`, outbound: `was observed listening on ${name}` },
    opened_by: { inbound: `${name} was observed being opened by this process`, outbound: `was observed being opened by ${name}` },
    config_of: { inbound: `${name} was observed configuring this`, outbound: `was observed configuring ${name}` },
    provides: { inbound: `${name} was observed providing this service`, outbound: `was observed providing ${name}` },
    publishes: { inbound: `${name} was observed publishing this port`, outbound: `was observed publishing ${name}` },
  };
  return phrases[edge.kind][direction];
}

export function entityKindName(kind: EntityType): string {
  if (kind in entityKindLabel) return entityKindLabel[kind as keyof typeof entityKindLabel];
  return kind.replaceAll('_', ' ');
}
