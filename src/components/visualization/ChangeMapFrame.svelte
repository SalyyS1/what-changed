<script lang="ts">
  import { onMount } from 'svelte';
  import ScreenFrame from '../layout/ScreenFrame.svelte';
  import StateShell from '../state/StateShell.svelte';
  import Button from '../ui/Button.svelte';
  import Chip from '../ui/Chip.svelte';
  import { eventDetails } from '../../lib/stores/eventDetails.svelte';
  import { changeMap as sharedMap, type ChangeMapStore } from '../../lib/stores/changeMap.svelte';
  import { settings } from '../../lib/stores/settings.svelte';
  import { ui } from '../../lib/stores/ui.svelte';
  import type { ChangeMapNode, ChangeMapRequest } from '../../lib/tauri/bindings';
  import type { ChangeMapData } from '../../lib/tauri/bindings';
  import type { AsyncState } from '../../lib/types/async-state';
  import type { QualityTier } from '../../three/changeMap/quality';
  import MapLegend from './MapLegend.svelte';
  import MapNodeList from './MapNodeList.svelte';
  import MapTooltip from './MapTooltip.svelte';
  import { probeWebGl } from './change-map-capability';
  import { resolveChangeMapRenderer } from './change-map-renderer';

  interface Props {
    request: ChangeMapRequest;
    title?: string;
    subtitle: string;
    onclose: () => void;
    controller?: ChangeMapStore;
    data?: AsyncState<ChangeMapData>;
    mode?: 'auto' | '3d' | '2d';
    quality?: QualityTier;
  }
  let { request, title = 'Change Map', subtitle, onclose, controller = sharedMap, data: fixture, mode, quality = 'high' }: Props = $props();
  let reducedMotion = $state(false);
  let webgl = $state(true);
  let contextLost = $state(false);
  let resetKey = $state(0);
  let hovered = $state<ChangeMapNode | null>(null);
  let tooltipX = $state(0);
  let tooltipY = $state(0);
  let symptomX = $state(0);
  let symptomY = $state(12);
  let symptomVisible = $state(false);
  let frameMedian = $state<number | null>(null);
  let renderer: '3d' | '2d' = $state('2d');
  let mapRegion = $state<HTMLDivElement>();
  const preference = $derived(mode ?? settings.current?.visualization ?? 'auto');
  const screenState = $derived(fixture ?? controller.state);

  $effect(() => {
    renderer = resolveChangeMapRenderer({
      preference,
      reducedMotion,
      webglAvailable: webgl,
      contextLost,
    });
  });

  onMount(() => {
    const media = globalThis.matchMedia?.('(prefers-reduced-motion: reduce)');
    const updateMotion = () => (reducedMotion = media?.matches ?? false);
    updateMotion(); media?.addEventListener('change', updateMotion);
    const probe = document.createElement('canvas');
    webgl = probeWebGl(probe, preference);
    return () => media?.removeEventListener('change', updateMotion);
  });

  $effect(() => {
    const currentRequest = request;
    if (fixture !== undefined) return;
    void controller.load(currentRequest);
    return () => controller.cancel();
  });

  function hover(node: ChangeMapNode | null, x: number, y: number): void {
    hovered = node; controller.hover(node?.event_id ?? null);
    const rect = mapRegion?.getBoundingClientRect();
    tooltipX = rect ? Math.min(rect.width - 12, Math.max(12, x - rect.left)) : x;
    tooltipY = rect ? Math.min(rect.height - 12, Math.max(12, y - rect.top)) : y;
  }

  function select(eventId: number, opener: HTMLElement): void {
    controller.select(eventId);
    const returnTarget = mapRegion?.querySelector<HTMLElement>('[role="listbox"]') ?? opener;
    const node = screenState.status === 'ready'
      ? screenState.data.nodes.find((candidate) => candidate.event_id === eventId)
      : undefined;
    if (node?.snapshot_only) eventDetails.showSnapshot(node, returnTarget);
    else void eventDetails.show(eventId, returnTarget);
  }

  function fallback(): void {
    if (contextLost) return;
    contextLost = true;
    ui.toast('Switched to the simple map because 3D stopped working.', 'warning');
  }

  function projectSymptom(x: number, y: number, visible: boolean): void {
    const rect = mapRegion?.getBoundingClientRect();
    symptomX = rect ? Math.min(rect.width - 72, Math.max(72, x - rect.left)) : x;
    symptomY = rect ? Math.min(rect.height - 24, Math.max(12, y - rect.top)) : y;
    symptomVisible = visible;
  }

  function mapSummary(data: ChangeMapData): string {
    const structural = data.edges.filter((edge) => edge.kind === 'structural').length;
    const chains = new Map<string, number>();
    for (const node of data.nodes) for (const id of node.chain_ids) chains.set(id, (chains.get(id) ?? 0) + 1);
    const top = Math.max(0, ...chains.values());
    return `Change map with ${data.nodes.length} changes across ${data.lanes.length} categories and ${structural} structural links. Top chain has ${top} changes.`;
  }
</script>

<ScreenFrame {title} {subtitle} flush>
  <div class="flex shrink-0 items-center justify-between gap-3 border-b border-border-subtle px-6 py-3">
    <div class="flex min-w-0 items-center gap-2">
      <Chip>{renderer === '3d' ? '3D' : '2D'}</Chip>
      <span class="truncate text-xs text-text-muted">Time · category · chain depth</span>
    </div>
    <div class="flex shrink-0 gap-2">
      <Button data-map-reset size="sm" onclick={() => (resetKey += 1)}>Reset view</Button>
      <Button size="sm" variant="primary" onclick={onclose}>Switch to list</Button>
    </div>
  </div>

  <StateShell
    state={screenState}
    emptyTitle="Nothing to map in this window."
    emptyDescription="Try another time range or investigation."
    emptyIcon="map"
    loadingRows={6}
    class="flex min-h-0 flex-1 flex-col"
  >
    {#snippet children(data)}
      <div bind:this={mapRegion} class="relative min-h-0 flex-1 overflow-hidden" data-testid="change-map-frame">
        <p class="sr-only">{mapSummary(data)}</p>
        {#if data.truncated}
          <div class="absolute top-3 left-3 z-20"><Chip>Showing {data.nodes.length.toLocaleString()} of {data.total_nodes.toLocaleString()} changes</Chip></div>
        {/if}
        {#if renderer === '3d'}
          {#await import('./ChangeMap.svelte') then mod}
            <mod.default
              {data}
              selectedId={controller.hoveredId ?? controller.selectedId}
              {resetKey}
              reducedMotion={preference === '3d' && reducedMotion}
              onhover={hover}
              onselect={select}
              oncontextlost={fallback}
              onproject={projectSymptom}
              onmetrics={(median) => (frameMedian = median)}
              {quality}
            />
          {/await}
        {:else}
          {#await import('./ChangeMap2D.svelte') then mod}
            <mod.default {data} selectedId={controller.hoveredId ?? controller.selectedId} {resetKey} onhover={hover} onselect={select} onproject={projectSymptom} />
          {/await}
        {/if}
        {#if data.symptom_time_ms !== null && symptomVisible}
          <div class="pointer-events-none absolute z-10 max-w-48 -translate-x-1/2 rounded bg-warning/12 px-2 py-1 text-center text-xs text-warning" style:left={`${symptomX || mapRegion?.clientWidth / 2 || 0}px`} style:top={`${symptomY}px`}>You noticed the problem here</div>
        {/if}
        <MapTooltip node={hovered} x={tooltipX} y={tooltipY} windowStartMs={data.window_start_ms} />
        <MapNodeList nodes={data.nodes} selectedId={controller.selectedId} onhover={(id) => controller.hover(id)} onselect={select} onescape={() => document.querySelector<HTMLElement>('[data-map-reset]')?.focus()} />
        <MapLegend />
        {#if import.meta.env.DEV && frameMedian !== null}
          <p class="absolute top-3 right-3 z-10 rounded bg-surface-1 px-2 py-1 text-xs text-text-muted" data-testid="map-frame-median">300-frame median: {frameMedian.toFixed(2)} ms</p>
        {/if}
        <p class="sr-only" aria-live="polite">{contextLost ? 'The simple 2D map is now active.' : hovered ? `${hovered.lane}: ${hovered.label}` : ''}</p>
      </div>
    {/snippet}
  </StateShell>
</ScreenFrame>
