<script lang="ts">
  import { onMount } from 'svelte';
  import { createOwnerScene, type OwnerScene } from '../../three/owner-map/scene';
  import type { OwnerLabel } from '../../three/owner-map/labels';
  import type { OwnerGraphWindow } from '../../lib/owner-graph/types';
  interface Props { graph: OwnerGraphWindow; selectedId: string | null; onselect: (id: string) => void; onunavailable: (reason: string) => void; separation: number; lowQuality: boolean; action?: { id: number; kind: 'reset' | 'fit' | 'in' | 'out' }; }
  let { graph, selectedId, onselect, onunavailable, separation, lowQuality, action }: Props = $props();
  let container: HTMLDivElement;
  let scene = $state<OwnerScene | null>(null);
  let labels = $state<OwnerLabel[]>([]);
  onMount(() => {
    let current: OwnerScene;
    try { current = createOwnerScene({ container, graph, separation, lowQuality, onselect, onunavailable, onlabels: (next) => { labels = next; } }); scene = current; }
    catch { onunavailable('3D is unavailable on this device. The complete 2D and list views remain available.'); return; }
    const resize = new ResizeObserver(() => current.resize()); resize.observe(container);
    const intersection = typeof IntersectionObserver === 'undefined' ? null : new IntersectionObserver((entries) => current.setVisible(entries.some((entry) => entry.isIntersecting)));
    intersection?.observe(container);
    const theme = new MutationObserver(() => current.refreshPalette()); theme.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'class', 'style'] });
    return () => { resize.disconnect(); intersection?.disconnect(); theme.disconnect(); current.destroy(); scene = null; };
  });
  $effect(() => { scene?.update(graph, separation); });
  $effect(() => { scene?.setSelected(selectedId); });
  $effect(() => { const command = action; if (!scene || !command) return; if (command.kind === 'reset') scene.reset(); else if (command.kind === 'fit') scene.fitSelected(); else scene.zoom(command.kind === 'in' ? 1.2 : 1 / 1.2); });
</script>

<p class="mb-2 text-xs text-text-secondary">Owner intent (top), scenarios (middle), observed results (bottom). Solid links are recorded relationships. Drag to orbit; right-drag to pan. Camera changes are immediate, including with reduced motion.</p>
<div class="relative h-100 overflow-hidden rounded-md border border-border-subtle bg-surface-2">
  <div bind:this={container} class="absolute inset-0 [&_canvas]:h-full [&_canvas]:w-full"></div>
  <div class="pointer-events-none absolute inset-0" aria-hidden="true">{#each labels as label (label.id)}<div class="absolute max-w-36 -translate-x-1/2 -translate-y-1/2 rounded border border-border-subtle bg-surface-1 px-2 py-1 text-xs text-text-primary {label.id === selectedId ? 'outline-2 outline-border-active' : ''}" style:left={`${label.x}px`} style:top={`${label.y}px`}><span class="line-clamp-2 break-words">{label.label}</span>{#if label.status}<span class="block text-text-secondary">{label.status.replaceAll('_', ' ')}</span>{/if}</div>{/each}</div>
</div>
<p class="mt-2 text-xs text-text-secondary">{labels.length} nonoverlapping labels shown for {graph.nodes.length} visible cards. Select any card by pointing or using the complete node list. Labels are reduced when the view is dense.</p>
