<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import { formatAbsoluteTime } from '../../lib/formatting/time';
  import type { InvestigationView } from './investigation-types';

  interface Props {
    rows: InvestigationView[];
    selectedId: number | null;
    onselect: (id: number) => void;
    ondelete?: ((id: number) => void) | undefined;
  }

  let { rows, selectedId, onselect, ondelete }: Props = $props();
</script>

<section aria-labelledby="investigation-history-heading">
  <h2 id="investigation-history-heading" class="mb-2 text-caps text-text-muted">History</h2>
  <ul class="m-0 flex list-none flex-col gap-1 p-0">
    {#each rows as row (row.id)}
      <li class="flex items-center gap-1 rounded-md {selectedId === row.id ? 'bg-surface-3' : ''}">
        <button
          type="button"
          class="min-w-0 flex-1 rounded-md px-2 py-2 text-left hover:bg-surface-2"
          aria-current={selectedId === row.id ? 'true' : undefined}
          onclick={() => onselect(row.id)}
        >
          <span class="block truncate text-sm text-text-primary">{row.problem}</span>
          <span class="block text-xs text-text-muted">{formatAbsoluteTime(row.createdAt)} · {row.suspects.length} suspects</span>
        </button>
        {#if ondelete}
          <Button variant="ghost" size="sm" aria-label={`Delete investigation: ${row.problem}`} onclick={() => ondelete?.(row.id)}>Delete</Button>
        {/if}
      </li>
    {/each}
  </ul>
</section>
