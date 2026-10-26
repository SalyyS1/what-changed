<script lang="ts">
  import type { CheckpointProgressItem } from '../../lib/stores/checkpoints.svelte';
  interface Props { items: CheckpointProgressItem[]; completed: number; total: number; }
  let { items, completed, total }: Props = $props();
  const percent = $derived(total > 0 ? Math.min(100, Math.round((completed / total) * 100)) : 0);
</script>

<div class="rounded-md border border-border-subtle bg-surface-2 p-3" aria-live="polite">
  <div class="flex items-center justify-between gap-3 text-sm">
    <span class="font-medium text-text-primary">Capturing this PC</span>
    <span class="font-tabular text-text-muted">{completed}/{total || '…'}</span>
  </div>
  <div class="mt-2 h-1.5 overflow-hidden rounded-full bg-surface-3" role="progressbar" aria-label="Checkpoint capture progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={total > 0 ? percent : undefined}>
    <div class="h-full rounded-full bg-accent motion-base transition-[width]" style:width={`${percent}%`}></div>
  </div>
  {#if items.length > 0}
    <ul class="mt-3 grid list-none grid-cols-1 gap-1 p-0 @[420px]/shell:grid-cols-2">
      {#each items as item (item.collectorId)}
        <li class="flex items-center gap-2 text-xs {item.state === 'failed' ? 'text-warning' : 'text-text-secondary'}">
          <span aria-hidden="true">{item.state === 'complete' ? '✓' : item.state === 'failed' ? '!' : item.state === 'running' ? '…' : '·'}</span>
          <span>{item.label}</span>
        </li>
      {/each}
    </ul>
  {:else}<p class="mt-2 text-xs text-text-muted">Preparing collectors…</p>{/if}
</div>
