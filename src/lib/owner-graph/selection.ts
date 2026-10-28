import type { OwnerGraph, OwnerGraphNode, OwnerGraphWindow } from './types';

export const MAX_OWNER_NODES = 120;
export const MAX_OWNER_EDGES = 240;
export type OwnerGraphFilter = 'all' | 'failed' | 'gaps' | 'stale';

export function filterOwnerNodes(graph: OwnerGraph, filter: OwnerGraphFilter, capability: string | null): readonly OwnerGraphNode[] {
  return graph.nodes.filter((node) => (!capability || node.capabilityId === capability) &&
    (filter === 'all' || filter === 'failed' && node.status === 'failed' ||
      filter === 'stale' && node.freshness === 'stale' ||
      filter === 'gaps' && (node.gaps.length > 0 || node.status === 'blocked' || node.status === 'not_run' || node.freshness === 'unknown')));
}

/** Paging bounds geometry while the full DOM list and global evidence totals remain available. */
export function ownerGraphWindow(graph: OwnerGraph, nodes: readonly OwnerGraphNode[], requestedPage = 0): OwnerGraphWindow {
  const pages = Math.max(1, Math.ceil(nodes.length / MAX_OWNER_NODES));
  const page = Math.max(0, Math.min(pages - 1, requestedPage));
  const visible = nodes.slice(page * MAX_OWNER_NODES, (page + 1) * MAX_OWNER_NODES);
  const ids = new Set(visible.map((node) => node.id));
  const connected = graph.edges.filter((edge) => ids.has(edge.source) && ids.has(edge.target));
  const edges = connected.slice(0, MAX_OWNER_EDGES);
  return { nodes: visible, edges, totalNodes: nodes.length, totalEdges: connected.length, pages, page,
    reason: pages > 1 || connected.length > edges.length ?
      `Visual group ${page + 1} of ${pages}; at most ${MAX_OWNER_NODES} nodes and ${MAX_OWNER_EDGES} links. The list includes every matching node; evidence totals include every recorded result.` : null };
}

export function reconcileSelection(nodes: readonly OwnerGraphNode[], id: string | null): string | null {
  return nodes.some((node) => node.id === id) ? id : nodes[0]?.id ?? null;
}
