<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { layoutOwnerGraph, clampOwnerPan, clampOwnerZoom } from '../../lib/owner-graph/layout';
  import type { OwnerGraphWindow } from '../../lib/owner-graph/types';
  interface Props { graph: OwnerGraphWindow; selectedId: string | null; onselect: (id: string) => void; action?: { id: number; kind: 'reset' | 'fit' | 'in' | 'out' }; }
  let { graph, selectedId, onselect, action }: Props = $props();
  let viewport: HTMLDivElement;
  let width = $state(800);
  let scale = $state(0.75);
  let x = $state(0);
  let y = $state(0);
  let drag: { id: number; x: number; y: number; originX: number; originY: number } | null = null;
  const layout = $derived(layoutOwnerGraph(graph));
  function reset(): void { scale = clampOwnerZoom(Math.min(width / layout.width, 400 / layout.height)); x = (width - layout.width * scale) / 2; y = 8; }
  function fit(): void {
    const position = selectedId ? layout.positions.get(selectedId) : null;
    if (!position) { reset(); return; }
    scale = 1; x = width / 2 - position.x - 108; y = 200 - position.y - 36;
  }
  onMount(() => {
    const observer = new ResizeObserver(() => { width = viewport.clientWidth || 800; reset(); });
    observer.observe(viewport); reset(); return () => observer.disconnect();
  });
  $effect(() => { layout; untrack(reset); });
  $effect(() => {
    const command = action; if (!command) return;
    untrack(() => { if (command.kind === 'fit') fit(); else if (command.kind === 'reset') reset();
      else scale = clampOwnerZoom(scale * (command.kind === 'in' ? 1.2 : 1 / 1.2)); });
  });
  function pan(dx: number, dy: number): void { x = clampOwnerPan(x + dx, layout.width * scale + width); y = clampOwnerPan(y + dy, layout.height * scale + 400); }
  function pointerdown(event: PointerEvent): void {
    if (event.button !== 0 || (event.target as Element).closest('button')) return;
    drag = { id: event.pointerId, x: event.clientX, y: event.clientY, originX: x, originY: y }; viewport.setPointerCapture?.(event.pointerId);
  }
  function pointermove(event: PointerEvent): void { if (drag?.id === event.pointerId) { x = clampOwnerPan(drag.originX + event.clientX - drag.x, layout.width * scale + width); y = clampOwnerPan(drag.originY + event.clientY - drag.y, layout.height * scale + 400); } }
  function pointerup(event: PointerEvent): void { if (drag?.id === event.pointerId) { drag = null; if (viewport.hasPointerCapture?.(event.pointerId)) viewport.releasePointerCapture(event.pointerId); } }
</script>

<p class="mb-2 text-xs text-text-secondary">Capability → requires → confirmed rule → verifies → scenario → observed by → result. Drag the background or use pan controls. Use the list for every record.</p>
<div class="mb-2 flex flex-wrap gap-2" role="group" aria-label="Pan 2D map">{#each [{ label: 'Left', x: 80, y: 0 }, { label: 'Right', x: -80, y: 0 }, { label: 'Up', x: 0, y: 80 }, { label: 'Down', x: 0, y: -80 }] as direction}<button type="button" class="min-h-11 rounded-md border border-border-subtle px-3 text-xs text-text-secondary hover:bg-surface-2" aria-label={`Pan ${direction.label.toLowerCase()}`} onclick={() => pan(direction.x, direction.y)}>{direction.label}</button>{/each}</div>
<div bind:this={viewport} role="group" aria-label="2D capability and evidence map" class="relative h-100 touch-none overflow-hidden rounded-md border border-border-subtle bg-surface-2" onpointerdown={pointerdown} onpointermove={pointermove} onpointerup={pointerup} onpointercancel={pointerup}>
  <div class="absolute origin-top-left" style:width={`${layout.width}px`} style:height={`${layout.height}px`} style:transform={`translate(${x}px, ${y}px) scale(${scale})`}>
    <svg width={layout.width} height={layout.height} class="pointer-events-none absolute inset-0" aria-hidden="true">
      {#each graph.edges as edge (edge.id)}
        {@const source = layout.positions.get(edge.source)}{@const target = layout.positions.get(edge.target)}
        {#if source && target}<path d={`M ${source.x + 216} ${source.y + 36} C ${source.x + 234} ${source.y + 36}, ${target.x - 18} ${target.y + 36}, ${target.x} ${target.y + 36}`} fill="none" stroke="var(--text-muted)" stroke-width={selectedId === edge.source || selectedId === edge.target ? 2 : 1} />{/if}
      {/each}
    </svg>
    {#each ['Capability', 'Confirmed rule', 'Scenario', 'Observed result'] as heading, column}<p class="absolute text-xs font-medium text-text-secondary" style:left={`${24 + column * 252}px`} style:top="16px">{heading}</p>{/each}
    {#each graph.nodes as node (node.id)}
      {@const position = layout.positions.get(node.id)!}
      <button type="button" tabindex="-1" aria-label={`${node.kind}: ${node.label}${node.status ? `; ${node.status}` : ''}${node.freshness ? `; ${node.freshness}` : ''}`} aria-pressed={selectedId === node.id} class="absolute h-20 w-54 overflow-hidden rounded-md border bg-surface-1 p-2 text-left {selectedId === node.id ? 'border-text-primary outline-2 outline-border-active' : node.selectedChange ? 'border-border-active border-dashed' : 'border-border-subtle'}" style:left={`${position.x}px`} style:top={`${position.y}px`} onclick={() => onselect(node.id)}>
        <span class="line-clamp-2 break-words text-sm text-text-primary">{node.label}</span>
        <span class="mt-1 block truncate text-xs {node.status === 'failed' ? 'text-danger' : node.status === 'blocked' || node.freshness === 'stale' ? 'text-warning' : node.status === 'passed' && node.freshness === 'current' ? 'text-success' : 'text-text-secondary'}">{node.status?.replaceAll('_', ' ') ?? node.kind}{node.freshness ? ` · ${node.freshness}` : ''}{node.gaps.length ? ' · gap' : ''}{node.revision !== null ? ` · r${node.revision}` : ''}</span>
      </button>
    {/each}
  </div>
</div>
