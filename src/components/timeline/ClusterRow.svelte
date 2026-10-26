<script lang="ts">
  import type { TimelineItem } from '../../lib/tauri/bindings';
  import { clusterIdentity, clusterMembersId } from '../../lib/timeline/identity';
  import Badge from '../ui/Badge.svelte';
  import Button from '../ui/Button.svelte';

  type Cluster = Extract<TimelineItem, { kind: 'cluster' }>;
  interface Props { cluster: Cluster; expanded?: boolean; onexpand: (cluster: Cluster) => void; }
  let { cluster, expanded = false, onexpand }: Props = $props();
  const identity = $derived(clusterIdentity(cluster.cluster));
  const membersId = $derived(clusterMembersId(cluster.cluster));
</script>

<div
  class="grid h-18 grid-cols-[3.5rem_minmax(0,1fr)_auto] items-center gap-3 border-b border-border-subtle bg-surface-1/45 px-6"
  data-testid="cluster-row"
>
  <Badge tone="accent">{cluster.count}</Badge>
  <div class="min-w-0">
    <p class="truncate text-base text-text-primary">{cluster.label}</p>
    <p class="mt-0.5 text-xs text-text-muted">Grouped changes within one minute</p>
  </div>
  <Button
    variant="ghost"
    size="sm"
    aria-expanded={expanded}
    aria-controls={membersId}
    onclick={() => onexpand(cluster)}
  >[{expanded ? 'Collapse' : 'Expand'}]</Button>
  {#if expanded}
    <span id={membersId} data-cluster-identity={identity} class="sr-only">Expanded members follow this row.</span>
  {/if}
</div>
