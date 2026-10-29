<script lang="ts">
  import type { OwnerMapMode } from '../../lib/owner-graph/types';
  interface Props { mode: OwnerMapMode; onmode: (mode: OwnerMapMode) => void; threeDisabled?: boolean; }
  let { mode, onmode, threeDisabled = false }: Props = $props();
</script>

<div class="flex flex-wrap gap-1" role="group" aria-label="Evidence view">
  {#each ['2d', '3d', 'list'] as item}
    <button type="button" class="min-h-11 rounded-md border px-3 text-sm {mode === item ? 'border-border-active bg-surface-3 text-text-primary' : 'border-border-subtle text-text-secondary hover:bg-surface-2'}" aria-pressed={mode === item} disabled={item === '3d' && threeDisabled} onclick={() => onmode(item as OwnerMapMode)}>{item === 'list' ? 'List' : item.toUpperCase()}</button>
  {/each}
</div>
