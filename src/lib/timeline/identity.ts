import type { ClusterRef, TimelineItem } from '../tauri/bindings';

/**
 * A cluster key describes its semantic lane, but it is deliberately reused
 * for later runs. The exact boundary tuple is therefore part of its UI
 * identity anywhere identity controls state, caching, or DOM reconciliation.
 */
export function clusterIdentity(cluster: ClusterRef): string {
  return JSON.stringify([
    cluster.key,
    cluster.category,
    cluster.coarse_entity,
    cluster.bucket,
    cluster.first_ts,
    cluster.first_id,
    cluster.last_ts,
    cluster.last_id,
  ]);
}

export function timelineItemIdentity(item: TimelineItem): string {
  return item.kind === 'event'
    ? `event:${item.event.id}`
    : `cluster:${clusterIdentity(item.cluster)}`;
}

export function clusterMembersId(cluster: ClusterRef): string {
  return `cluster-members-${encodeURIComponent(clusterIdentity(cluster))}`;
}
