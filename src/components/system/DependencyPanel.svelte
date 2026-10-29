<script lang="ts">
  import type { SystemMapData, SystemMapEdge, SystemMapNode } from '../../lib/tauri/bindings';
  import { eventDetails } from '../../lib/stores/eventDetails.svelte';
  import { entityKindName, relationshipLabel } from '../../features/system-map/copy';
  import { formatAbsoluteTime, formatClock } from '../../lib/formatting/time';

  interface Props { data: SystemMapData; onfocus: (node: SystemMapNode) => void; }
  let { data, onfocus }: Props = $props();
  const focusNode = $derived(data.focus ? data.nodes.find((node) => node.entity_id === data.focus?.entity_id) ?? null : null);
  const nodeById = $derived(new Map(data.nodes.map((node) => [node.entity_id, node])));
  const inbound = $derived(focusNode ? data.edges.filter((edge) => edge.target === focusNode.entity_id) : []);
  const outbound = $derived(focusNode ? data.edges.filter((edge) => edge.source === focusNode.entity_id) : []);
  const recentEvents = $derived(data.focused_recent_events.slice(0, 5));

  function other(edge: SystemMapEdge, direction: 'inbound' | 'outbound'): SystemMapNode | undefined {
    return nodeById.get(direction === 'inbound' ? edge.source : edge.target);
  }

  function evidence(edge: SystemMapEdge, opener: HTMLElement): void {
    if (edge.evidence_event_id !== null) void eventDetails.show(edge.evidence_event_id, opener);
  }
</script>

<aside class="flex min-h-0 flex-col border-l border-border-subtle bg-surface-1/80 @[900px]/system-map:w-80" aria-label="Dependency details">
  <div class="border-b border-border-subtle px-4 py-3">
    {#if focusNode}
      <p class="text-caps text-text-muted">{entityKindName(focusNode.entity_type)}</p>
      <h2 class="mt-1 truncate font-display text-lg text-text-primary">{focusNode.display_name}</h2>
      <p class="mt-1 text-xs text-text-muted">{focusNode.degree} links · {focusNode.recent_event_count} recent {focusNode.recent_event_count === 1 ? 'change' : 'changes'}</p>
    {:else}
      <h2 class="font-display text-lg text-text-primary">Whole system</h2>
      <p class="mt-1 text-sm text-text-secondary">Choose a node to inspect its neighbourhood.</p>
    {/if}
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto px-4 py-3">
    {#if focusNode}
      <p class="mb-4 text-xs text-text-muted">These links record past observations, not a live connection or a causal claim.</p>
      {#each [['Recorded incoming links', inbound, 'inbound'], ['Recorded outgoing links', outbound, 'outbound']] as section (section[0])}
        {@const title = section[0] as string}
        {@const edges = section[1] as SystemMapEdge[]}
        {@const direction = section[2] as 'inbound' | 'outbound'}
        <section class="mb-5" aria-label={title}>
          <h3 class="mb-2 text-caps text-text-muted">{title} · {edges.length}</h3>
          {#if edges.length === 0}
            <p class="text-sm text-text-muted">No recorded links in this direction.</p>
          {:else}
            <ul class="space-y-2">
              {#each edges as edge (`${edge.source}:${edge.kind}:${edge.target}`)}
                {@const node = other(edge, direction)}
                {#if node}
                  <li class="rounded-md border border-border-subtle bg-surface-2 p-2.5">
                    <button class="block w-full text-left text-sm text-text-primary hover:text-accent" onclick={() => onfocus(node)}>
                      {relationshipLabel(edge, node, direction)}
                    </button>
                    <div class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-text-muted">
                      <span>{entityKindName(node.entity_type)}</span>
                      {#if Number.isFinite(edge.recorded_at) && !Number.isNaN(new Date(edge.recorded_at).getTime())}
                        <time datetime={new Date(edge.recorded_at).toISOString()} title="When this relationship was observed">
                          Recorded {formatAbsoluteTime(edge.recorded_at)}
                        </time>
                      {:else}
                        <span>Recorded time unavailable</span>
                      {/if}
                      {#if edge.evidence_event_id !== null}
                        <button class="text-accent hover:underline" onclick={(event) => evidence(edge, event.currentTarget)}>View evidence</button>
                      {:else}
                        <span title="The event aged out, but the recorded relationship remains.">Evidence expired</span>
                      {/if}
                    </div>
                  </li>
                {/if}
              {/each}
            </ul>
          {/if}
        </section>
      {/each}
      <section class="mb-2" aria-label="Recent changes">
        <h3 class="mb-2 text-caps text-text-muted">Recent changes · {focusNode.recent_event_count}</h3>
        {#if recentEvents.length > 0}
          <ul class="space-y-1.5">
            {#each recentEvents as recent (recent.event_id)}
              <li>
                <button class="flex w-full items-start justify-between gap-3 rounded px-1 py-1 text-left hover:bg-surface-2" onclick={(event) => void eventDetails.show(recent.event_id, event.currentTarget)}>
                  <span class="line-clamp-2 text-sm text-accent">{recent.summary}</span>
                  <span class="shrink-0 text-xs text-text-muted font-tabular">{formatClock(recent.timestamp_ms)}</span>
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="text-sm text-text-muted">No retained change details for this neighbourhood.</p>
        {/if}
      </section>
    {/if}
  </div>
</aside>
