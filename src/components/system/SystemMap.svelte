<script lang="ts">
  import { onMount } from 'svelte';
  import type { EntityRef, SystemMapData, SystemMapNode } from '../../lib/tauri/bindings';
  import type { QualityTier } from '../../three/changeMap/quality';

  interface Props {
    data: SystemMapData;
    selectedId: string | null;
    resetKey: number;
    reducedMotion?: boolean;
    quality?: QualityTier;
    onhover: (node: SystemMapNode | null, x: number, y: number) => void;
    onfocus: (focus: EntityRef) => void;
    oncontextlost: () => void;
    onmetrics?: (median: number) => void;
  }
  let { data, selectedId, resetKey, reducedMotion = false, quality = 'high', onhover, onfocus, oncontextlost, onmetrics }: Props = $props();
  let container = $state<HTMLDivElement>();
  let scene = $state<Awaited<ReturnType<typeof import('../../three/systemMap/scene')['createSystemMapScene']>> | null>(null);

  onMount(() => {
    let live = true;
    let observer: ResizeObserver | null = null;
    let themeObserver: MutationObserver | null = null;
    const visibility = () => scene?.setVisible(document.visibilityState !== 'hidden');
    void import('../../three/systemMap/scene').then(({ createSystemMapScene }) => {
      if (!live || !container) return;
      scene = createSystemMapScene({ container, data, selectedEntityId: selectedId, reducedMotion, quality, onHover: onhover, onFocus: onfocus, onContextLost: oncontextlost, onMetrics: onmetrics });
      observer = new ResizeObserver(() => scene?.resize());
      observer.observe(container);
      themeObserver = new MutationObserver(() => scene?.refreshPalette());
      themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'data-theme'] });
      document.addEventListener('visibilitychange', visibility);
      visibility();
    }).catch(oncontextlost);
    return () => {
      live = false;
      observer?.disconnect();
      themeObserver?.disconnect();
      document.removeEventListener('visibilitychange', visibility);
      scene?.destroy();
      scene = null;
    };
  });

  $effect(() => { scene?.update(data); });
  $effect(() => { scene?.setSelected(selectedId); });
  $effect(() => { resetKey; scene?.resetView(); });
</script>

<div bind:this={container} class="absolute inset-0" data-testid="system-map-3d" aria-hidden="true"></div>
