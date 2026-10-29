<script lang="ts">
  import { onMount } from 'svelte';
  import type { EntityRef, EntityType, SystemMapData, SystemMapNode } from '../../lib/tauri/bindings';
  import { layoutSystemMap } from '../../three/systemMap/layout';

  interface Props {
    data: SystemMapData;
    selectedId: string | null;
    resetKey: number;
    onhover: (node: SystemMapNode | null, x: number, y: number) => void;
    onfocus: (focus: EntityRef) => void;
  }
  let { data, selectedId, resetKey, onhover, onfocus }: Props = $props();
  let canvas: HTMLCanvasElement;
  let width = $state(1);
  let height = $state(1);
  let zoom = $state(1);
  let panX = $state(0);
  let panY = $state(0);
  let themeRevision = $state(0);
  let drag: { x: number; y: number; panX: number; panY: number; moved: boolean } | null = null;
  const layout = $derived(layoutSystemMap(data));
  const neighbours = $derived.by(() => {
    if (!selectedId) return null;
    const ids = new Set([selectedId]);
    for (const edge of data.edges) {
      if (edge.source === selectedId) ids.add(edge.target);
      if (edge.target === selectedId) ids.add(edge.source);
    }
    return ids;
  });

  $effect(() => { resetKey; zoom = 1; panX = 0; panY = 0; });
  $effect(() => { selectedId; data; width; height; zoom; panX; panY; themeRevision; draw(); });

  onMount(() => {
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      width = Math.max(1, Math.floor(entry.contentRect.width));
      height = Math.max(1, Math.floor(entry.contentRect.height));
      const dpr = Math.min(devicePixelRatio || 1, 2);
      canvas.width = width * dpr;
      canvas.height = height * dpr;
      canvas.style.width = `${width}px`;
      canvas.style.height = `${height}px`;
    });
    observer.observe(canvas);
    const themeObserver = new MutationObserver(() => (themeRevision += 1));
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'data-theme'] });
    return () => { observer.disconnect(); themeObserver.disconnect(); };
  });

  function scale(): number {
    return Math.min(width, height) / Math.max(2, layout.bounds.width + 8) * zoom;
  }
  function screen(index: number): [number, number] {
    const factor = scale();
    return [width / 2 + ((layout.positions[index * 3] ?? 0) + panX) * factor, height / 2 - ((layout.positions[index * 3 + 1] ?? 0) + panY) * factor];
  }
  function token(kind: EntityType): string {
    const tokens: Partial<Record<EntityType, string>> = { app: 'application', file: 'config', adapter: 'network', process: 'process', port: 'port', service: 'service', driver: 'driver', update: 'update', task: 'startup' };
    return `--map-category-${tokens[kind] ?? 'system'}`;
  }
  function draw(): void {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const dpr = Math.min(devicePixelRatio || 1, 2);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);
    const style = getComputedStyle(canvas);
    for (const edge of data.edges) {
      const from = layout.nodeIndexById.get(edge.source); const to = layout.nodeIndexById.get(edge.target);
      if (from === undefined || to === undefined) continue;
      const [x1, y1] = screen(from); const [x2, y2] = screen(to);
      const emphasized = !selectedId || edge.source === selectedId || edge.target === selectedId;
      ctx.globalAlpha = emphasized ? 0.7 : 0.12;
      ctx.strokeStyle = style.getPropertyValue('--border-active');
      ctx.lineWidth = emphasized ? 1.5 : 1;
      ctx.beginPath(); ctx.moveTo(x1, y1); ctx.lineTo(x2, y2); ctx.stroke();
    }
    for (let index = 0; index < data.nodes.length; index += 1) {
      const node = data.nodes[index]; if (!node) continue;
      const [x, y] = screen(index);
      ctx.globalAlpha = neighbours === null || neighbours.has(node.entity_id) ? 1 : 0.2;
      ctx.fillStyle = style.getPropertyValue(token(node.entity_type)) || style.getPropertyValue('--accent');
      ctx.beginPath(); ctx.arc(x, y, Math.min(10, 3.5 + Math.sqrt(node.degree) * 1.25), 0, Math.PI * 2); ctx.fill();
      if (node.entity_id === selectedId) {
        ctx.globalAlpha = 1; ctx.strokeStyle = style.getPropertyValue('--text-primary'); ctx.lineWidth = 2;
        ctx.beginPath(); ctx.arc(x, y, Math.min(13, 6.5 + Math.sqrt(node.degree) * 1.25), 0, Math.PI * 2); ctx.stroke();
      }
    }
    ctx.globalAlpha = 1;
  }
  function hit(event: PointerEvent): SystemMapNode | null {
    const rect = canvas.getBoundingClientRect();
    let nearest: SystemMapNode | null = null;
    let distance = Number.POSITIVE_INFINITY;
    for (const [index, node] of data.nodes.entries()) {
      const [x, y] = screen(index);
      const candidate = Math.hypot(event.clientX - rect.left - x, event.clientY - rect.top - y);
      if (candidate <= 11 && candidate < distance) { nearest = node; distance = candidate; }
    }
    return nearest;
  }
</script>

<canvas
  bind:this={canvas}
  class="h-full w-full touch-none bg-background"
  data-testid="system-map-2d"
  aria-hidden="true"
  onpointermove={(event) => {
    if (drag) {
      if (Math.hypot(event.clientX - drag.x, event.clientY - drag.y) > 4) drag.moved = true;
      const factor = scale();
      panX = drag.panX + (event.clientX - drag.x) / factor;
      panY = drag.panY - (event.clientY - drag.y) / factor;
    }
    onhover(hit(event), event.clientX, event.clientY);
  }}
  onpointerdown={(event) => { drag = { x: event.clientX, y: event.clientY, panX, panY, moved: false }; canvas.setPointerCapture(event.pointerId); }}
  onpointerup={(event) => { const node = hit(event); const moved = drag?.moved ?? false; drag = null; if (node && !moved) onfocus({ entity_id: node.entity_id, entity_type: node.entity_type }); }}
  onpointercancel={() => { drag = null; onhover(null, 0, 0); }}
  onpointerleave={() => { drag = null; onhover(null, 0, 0); }}
  onwheel={(event) => { event.preventDefault(); zoom = Math.min(4, Math.max(0.4, zoom * (event.deltaY > 0 ? 0.9 : 1.1))); }}
></canvas>
