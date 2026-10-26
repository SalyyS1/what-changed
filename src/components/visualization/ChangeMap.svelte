<script lang="ts">
  import { untrack } from 'svelte';
  import type { ChangeMapData, ChangeMapNode } from '../../lib/tauri/bindings';
  import { createChangeMapScene, type ChangeMapSceneApi } from '../../three/changeMap/createScene';
  import { qualityAfterTiming, type QualityTier } from '../../three/changeMap/quality';

  interface Props {
    data: ChangeMapData;
    selectedId: number | null;
    resetKey: number;
    active?: boolean;
    reducedMotion?: boolean;
    onhover: (node: ChangeMapNode | null, x: number, y: number) => void;
    onselect: (eventId: number, opener: HTMLElement) => void;
    oncontextlost: () => void;
    onproject?: (x: number, y: number, visible: boolean) => void;
    onmetrics?: (medianMs: number) => void;
    quality?: QualityTier;
  }
  let { data, selectedId, resetKey, active = true, reducedMotion = false, onhover, onselect, oncontextlost, onproject, onmetrics, quality = 'high' }: Props = $props();
  let host: HTMLDivElement;
  let scene: ChangeMapSceneApi | null = null;
  let activeQuality = $state<QualityTier>('high');

  $effect(() => { activeQuality = quality; });

  $effect(() => {
    if (!host) return;
    try {
      scene = createChangeMapScene({
        container: host,
        data: untrack(() => data),
        reducedMotion,
        quality: activeQuality,
        onHover: (node, x, y) => onhover(node ? data.nodes.find((item) => item.id === node.id) ?? null : null, x, y),
        onSelect: (id) => onselect(id, scene?.canvas ?? host),
        onContextLost: oncontextlost,
        ...(onproject ? { onProject: onproject } : {}),
        onMetrics: (median) => {
          onmetrics?.(median);
          activeQuality = qualityAfterTiming(activeQuality, median);
        },
      });
    } catch {
      oncontextlost();
      return;
    }
    const observer = new ResizeObserver(([entry]) => {
      if (!entry || !scene) return;
      scene.resize(entry.contentRect.width, entry.contentRect.height);
      project();
    });
    observer.observe(host);
    const themeObserver = new MutationObserver(() => scene?.refreshPalette());
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    return () => { observer.disconnect(); themeObserver.disconnect(); scene?.destroy(); scene = null; };
  });

  $effect(() => { scene?.update(data); project(); });
  $effect(() => scene?.setSelected(selectedId));
  $effect(() => scene?.setVisible(active));
  $effect(() => { void resetKey; scene?.resetView(); project(); });

  function project(): void {
    const point = scene?.projectSymptom();
    if (point) onproject?.(point.x, point.y, point.visible);
  }
</script>

<div
  bind:this={host}
  class="h-full w-full overflow-hidden bg-background"
  role="img"
  aria-label={`Change map with ${data.nodes.length} changes and ${data.edges.length} connections.`}
></div>
