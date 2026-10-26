<script lang="ts">
  import type { ChangeMapNode } from '../../lib/tauri/bindings';

  interface Props { node: ChangeMapNode | null; x: number; y: number; windowStartMs: number; }
  let { node, x, y, windowStartMs }: Props = $props();

  function relativeTime(timestampMs: number): string {
    const elapsed = Math.max(0, timestampMs - windowStartMs);
    if (elapsed < 1000) return `+${elapsed} ms`;
    if (elapsed < 60_000) return `+${(elapsed / 1000).toFixed(1)} s`;
    return `+${(elapsed / 60_000).toFixed(1)} min`;
  }
</script>

{#if node}
  <div
    class="pointer-events-none absolute z-20 max-w-72 -translate-x-1/2 -translate-y-full rounded-md border border-border-subtle bg-surface-1 px-3 py-2 text-xs shadow-lg"
    style:left={`${Math.max(12, x)}px`}
    style:top={`${Math.max(12, y - 10)}px`}
    role="tooltip"
  >
    <p class="font-medium text-text-primary">{node.label}</p>
    <p class="mt-1 capitalize text-text-muted">{node.lane} · {node.importance} · {relativeTime(node.timestamp_ms)}</p>
  </div>
{/if}
