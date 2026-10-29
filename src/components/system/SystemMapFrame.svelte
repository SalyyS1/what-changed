<script lang="ts">
  import { onMount } from 'svelte';
  import type { EntityRef, EntityType, SystemMapData, SystemMapNode } from '../../lib/tauri/bindings';
  import { systemMap as sharedMap, type SystemMapStore } from '../../lib/stores/systemMap.svelte';
  import { settings } from '../../lib/stores/settings.svelte';
  import type { AsyncState } from '../../lib/types/async-state';
  import type { QualityTier } from '../../three/changeMap/quality';
  import { probeWebGl } from '../visualization/change-map-capability';
  import { resolveChangeMapRenderer } from '../visualization/change-map-renderer';
  import StateShell from '../state/StateShell.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import DependencyPanel from './DependencyPanel.svelte';
  import KindFilter from './KindFilter.svelte';
  import SystemMapNodeList from './SystemMapNodeList.svelte';
  import { ui } from '../../lib/stores/ui.svelte';

  interface Props {
    controller?: SystemMapStore;
    data?: AsyncState<SystemMapData>;
    mode?: 'auto' | '3d' | '2d';
    quality?: QualityTier;
    onmetrics?: (median: number) => void;
  }
  let { controller = sharedMap, data: fixture, mode, quality = 'high', onmetrics }: Props = $props();
  let reducedMotion = $state(false);
  let webgl = $state(true);
  let contextLost = $state(false);
  let resetKey = $state(0);
  let hovered = $state<SystemMapNode | null>(null);
  let tooltipX = $state(0);
  let tooltipY = $state(0);
  let mapRegion = $state<HTMLDivElement>();
  let mapFrame = $state<HTMLDivElement>();
  const preference = $derived(mode ?? settings.current?.visualization ?? 'auto');
  const renderer = $derived(resolveChangeMapRenderer({ preference, reducedMotion, webglAvailable: webgl, contextLost }));
  const screenState = $derived(fixture ?? controller.state);

  onMount(() => {
    const media = globalThis.matchMedia?.('(prefers-reduced-motion: reduce)');
    const updateMotion = () => (reducedMotion = media?.matches ?? false);
    updateMotion();
    media?.addEventListener('change', updateMotion);
    webgl = probeWebGl(document.createElement('canvas'), preference);
    return () => media?.removeEventListener('change', updateMotion);
  });

  function hover(node: SystemMapNode | null, x: number, y: number): void {
    hovered = node;
    controller.hover(node?.entity_id ?? null);
    const rect = mapRegion?.getBoundingClientRect();
    tooltipX = rect ? Math.min(rect.width - 12, Math.max(12, x - rect.left)) : x;
    tooltipY = rect ? Math.min(rect.height - 12, Math.max(12, y - rect.top)) : y;
  }

  function reportMetrics(median: number): void { onmetrics?.(median); }

  function focus(ref: EntityRef): void { void controller.setFocus(ref); }
  function focusNode(node: SystemMapNode): void { focus({ entity_id: node.entity_id, entity_type: node.entity_type }); }
  function fallback(): void {
    if (contextLost) return;
    contextLost = true;
    ui.toast('Switched to the simple map because 3D stopped working.', 'warning');
  }
  function toggleKind(kind: EntityType): void { void controller.toggleKind(kind); }
  function summary(data: SystemMapData): string {
    return `System map with ${data.nodes.length} things and ${data.edges.length} recorded relationships${data.focus ? `, focused on ${data.focus.entity_id}` : ''}.`;
  }
</script>

<div bind:this={mapFrame} class="@container/system-map flex min-h-0 flex-1 flex-col" data-testid="system-map-frame">
  <div class="shrink-0 space-y-2 border-y border-border-subtle px-4 py-3">
    <div class="flex flex-wrap items-center justify-between gap-2">
      <KindFilter
        availability={screenState.status === 'ready' ? screenState.data.kind_availability : []}
        selected={controller.kinds}
        ontoggle={toggleKind}
      />
      <div class="flex items-center gap-1.5">
        <span class="text-xs text-text-muted">Depth</span>
        {#each [1, 2, 3] as depth}
          <Chip selected={controller.depth === depth} aria-label={`${depth} ${depth === 1 ? 'hop' : 'hops'}`} onclick={() => void controller.setDepth(depth)}>{depth}</Chip>
        {/each}
        <Chip>{renderer === '3d' ? '3D' : '2D'}</Chip>
        <Button size="sm" data-map-reset onclick={() => (resetKey += 1)}>Reset view</Button>
      </div>
    </div>
    <p class="text-xs text-text-muted">Rings show distance from the focused thing. Select a node to make it the new centre.</p>
  </div>

  <StateShell state={screenState} emptyTitle="Nothing is connected yet." emptyDescription="Relationships appear as collectors record them." emptyIcon="map" loadingRows={6} class="flex min-h-0 flex-1 flex-col">
    {#snippet children(data)}
      <div class="flex min-h-0 flex-1 flex-col @[760px]/system-map:flex-row">
        <div bind:this={mapRegion} class="relative min-h-72 flex-1 overflow-hidden">
          <p class="sr-only">{summary(data)}</p>
          {#if data.truncated}
            <div class="absolute top-3 left-3 z-20"><Chip>Showing {data.nodes.length.toLocaleString()} of {data.total_nodes_available.toLocaleString()} connected things</Chip></div>
          {/if}
          {#if renderer === '3d'}
            {#await import('./SystemMap.svelte') then mod}
              <mod.default {data} selectedId={controller.hoveredId ?? data.focus?.entity_id ?? null} {resetKey} reducedMotion={preference === '3d' && reducedMotion} {quality} onhover={hover} onfocus={focus} oncontextlost={fallback} onmetrics={reportMetrics} />
            {/await}
          {:else}
            {#await import('./SystemMap2D.svelte') then mod}
              <mod.default {data} selectedId={controller.hoveredId ?? data.focus?.entity_id ?? null} {resetKey} onhover={hover} onfocus={focus} />
            {/await}
          {/if}
          {#if hovered}
            <div class="pointer-events-none absolute z-30 max-w-64 -translate-x-1/2 -translate-y-full rounded-md border border-border-subtle bg-surface-1/95 px-3 py-2 shadow-md" style:left={`${tooltipX}px`} style:top={`${tooltipY - 10}px`} role="tooltip">
              <p class="truncate text-sm text-text-primary">{hovered.display_name}</p>
              <p class="text-xs text-text-muted">{hovered.degree} links · {hovered.recent_event_count} recent changes</p>
            </div>
          {/if}
          <SystemMapNodeList nodes={data.nodes} selectedId={controller.hoveredId ?? data.focus?.entity_id ?? null} onhover={(id) => controller.hover(id)} onfocus={focusNode} onescape={() => mapFrame?.querySelector<HTMLElement>('[data-map-reset]')?.focus()} />
        </div>
        <DependencyPanel {data} onfocus={focusNode} />
      </div>
    {/snippet}
  </StateShell>
</div>
