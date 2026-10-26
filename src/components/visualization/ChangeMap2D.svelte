<script lang="ts">
  import { onMount } from 'svelte';
  import type { ChangeMapData, ChangeMapNode } from '../../lib/tauri/bindings';
  import { layoutChangeMap, symptomX } from '../../three/changeMap/layout';

  interface Props {
    data: ChangeMapData;
    selectedId: number | null;
    resetKey: number;
    onhover: (node: ChangeMapNode | null, x: number, y: number) => void;
    onselect: (eventId: number, opener: HTMLElement) => void;
    onproject?: (x: number, y: number, visible: boolean) => void;
  }
  let { data, selectedId, resetKey, onhover, onselect, onproject }: Props = $props();
  let canvas: HTMLCanvasElement;
  let width = $state(1);
  let height = $state(1);
  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let drag: { x: number; y: number; panX: number; panY: number; moved: boolean } | null = null;
  let themeRevision = $state(0);
  const layout = $derived(layoutChangeMap(data));

  $effect(() => { void resetKey; zoom = 1; panX = 0; panY = 0; });
  $effect(() => { void selectedId; void data; void width; void height; void zoom; void panX; void panY; void themeRevision; draw(); });

  onMount(() => {
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      const nextWidth = Math.max(1, Math.floor(entry.contentRect.width));
      const nextHeight = Math.max(1, Math.floor(entry.contentRect.height));
      if (nextWidth === width && nextHeight === height) return;
      width = nextWidth;
      height = nextHeight;
      canvas.width = width * Math.min(devicePixelRatio || 1, 2);
      canvas.height = height * Math.min(devicePixelRatio || 1, 2);
      canvas.style.width = `${width}px`;
      canvas.style.height = `${height}px`;
    });
    observer.observe(canvas);
    const themeObserver = new MutationObserver(() => (themeRevision += 1));
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    return () => { observer.disconnect(); themeObserver.disconnect(); };
  });

  function screen(index: number): [number, number] {
    const x = layout.positions[index * 3] ?? 0;
    const y = layout.positions[index * 3 + 1] ?? 0;
    return [width / 2 + (x + panX) * zoom * width / 120, height / 2 - (y + panY) * zoom * height / 100];
  }

  function draw(): void {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const dpr = Math.min(devicePixelRatio || 1, 2);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);
    const style = getComputedStyle(canvas);
    const indexById = layout.nodeIndexById;
    for (const edge of data.edges) {
      const from = indexById.get(edge.source); const to = indexById.get(edge.target);
      if (from === undefined || to === undefined) continue;
      const [x1, y1] = screen(from); const [x2, y2] = screen(to);
      ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2);
      ctx.strokeStyle = edge.kind === 'structural' ? style.getPropertyValue('--accent') : style.getPropertyValue('--text-muted');
      ctx.globalAlpha = edge.kind === 'structural' ? 0.9 : 0.35;
      ctx.lineWidth = edge.kind === 'structural' ? 2 : 1;
      ctx.setLineDash(edge.kind === 'structural' ? [] : [4, 5]); ctx.stroke();
    }
    const selected = data.nodes.find((node) => node.event_id === selectedId);
    for (let index = 0; index < data.nodes.length; index += 1) {
      const node = data.nodes[index]; if (!node) continue;
      const [x, y] = screen(index);
      const related = !selected || selected.chain_ids.some((id) => node.chain_ids.includes(id)) || node.event_id === selected.event_id;
      ctx.globalAlpha = related ? 1 : 0.25;
      ctx.fillStyle = style.getPropertyValue(`--map-category-${node.lane}`);
      ctx.beginPath(); ctx.arc(x, y, Math.max(3, (layout.scales[index] ?? 1) * 5), 0, Math.PI * 2); ctx.fill();
    }
    const markerX = symptomX(data);
    if (markerX !== null) {
      const marker = width / 2 + (markerX + panX) * zoom * width / 120;
      ctx.globalAlpha = 0.7; ctx.strokeStyle = style.getPropertyValue('--warning'); ctx.setLineDash([3, 4]);
      ctx.beginPath(); ctx.moveTo(marker, 0); ctx.lineTo(marker, height); ctx.stroke(); ctx.globalAlpha = 1;
      onproject?.(canvas.getBoundingClientRect().left + marker, canvas.getBoundingClientRect().top + 12, marker >= 0 && marker <= width);
    }
  }

  function hit(event: PointerEvent): ChangeMapNode | null {
    const rect = canvas.getBoundingClientRect();
    let nearestNode: ChangeMapNode | null = null;
    let nearestDistance = Number.POSITIVE_INFINITY;
    for (const [index, node] of data.nodes.entries()) {
      const [x, y] = screen(index); const distance = Math.hypot(event.clientX - rect.left - x, event.clientY - rect.top - y);
      if (distance <= 9 && distance < nearestDistance) { nearestNode = node; nearestDistance = distance; }
    }
    return nearestNode;
  }
</script>

<canvas
  bind:this={canvas}
  class="h-full w-full touch-none bg-background"
  aria-hidden="true"
  onpointermove={(event) => {
    if (drag) {
      if (Math.hypot(event.clientX - drag.x, event.clientY - drag.y) > 4) drag.moved = true;
      panX = drag.panX + (event.clientX - drag.x) / Math.max(1, width) * 120 / zoom;
      panY = drag.panY - (event.clientY - drag.y) / Math.max(1, height) * 100 / zoom;
    }
    const node = hit(event); onhover(node, event.clientX, event.clientY);
  }}
  onpointerdown={(event) => { drag = { x: event.clientX, y: event.clientY, panX, panY, moved: false }; canvas.setPointerCapture(event.pointerId); }}
  onpointerup={(event) => { const node = hit(event); const moved = drag?.moved ?? false; drag = null; if (node && !moved) onselect(node.event_id, canvas); }}
  onpointerleave={() => { drag = null; onhover(null, 0, 0); }}
  onwheel={(event) => { event.preventDefault(); zoom = Math.min(4, Math.max(0.4, zoom * (event.deltaY > 0 ? 0.9 : 1.1))); }}
></canvas>
