<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import type { CheckpointDto } from '../../lib/tauri/bindings';
  import { checkpointRow, formatCheckpointAge } from './checkpoint-row';

  interface Props {
    rows: CheckpointDto[];
    pending?: Set<number>;
    oncompare: (id: number) => void;
    onrename: (id: number, name: string) => void;
    ondelete: (id: number) => void;
  }

  let { rows, pending = new Set(), oncompare, onrename, ondelete }: Props = $props();
  let editingId = $state<number | null>(null);
  let editName = $state('');

  function beginRename(id: number, name: string): void { editingId = id; editName = name; }
  function saveRename(): void {
    if (editingId !== null && editName.trim()) onrename(editingId, editName);
    editingId = null;
  }
</script>

<ul class="m-0 flex list-none flex-col gap-2 p-0">
  {#each rows as dto (dto.id)}
    {@const row = checkpointRow(dto)}
    <li class="rounded-md border border-border-subtle bg-surface-1 px-4 py-3">
      <div class="flex flex-col gap-3 @[620px]/shell:flex-row @[620px]/shell:items-center">
        <div class="min-w-0 flex-1">
          {#if editingId === row.id}
            <form class="flex max-w-md gap-2" onsubmit={(event) => { event.preventDefault(); saveRename(); }}>
              <input class="h-8 min-w-0 flex-1 rounded-md border border-border-active bg-surface-2 px-2 text-base text-text-primary" aria-label="Checkpoint name" bind:value={editName} maxlength={120} />
              <Button size="sm" type="submit" disabled={!editName.trim()}>Save</Button>
              <Button size="sm" variant="ghost" onclick={() => (editingId = null)}>Cancel</Button>
            </form>
          {:else}
            <div class="flex flex-wrap items-center gap-2">
              <p class="truncate text-base font-medium text-text-primary">{row.name}</p>
              {#if row.partial}<Badge tone="warning">Partial</Badge>{/if}
              <Badge tone={row.origin === 'manual' ? 'accent' : 'neutral'}>{row.origin === 'manual' ? 'Manual' : 'Automatic'}</Badge>
            </div>
          {/if}
          <p class="mt-1 text-xs text-text-muted font-tabular">{formatCheckpointAge(row.createdAt)} · {row.itemCount.toLocaleString()} items · {(row.durationMs / 1000).toFixed(1)}s</p>
          {#if row.reason}<p class="mt-1 truncate text-sm text-text-secondary">{row.reason}</p>{/if}
          {#if row.partial}
            <p class="mt-1 text-xs text-warning">Partial — {row.missingCollectors.length > 0 ? `${row.missingCollectors.join(', ')} could not be captured` : 'some collectors could not be captured'}</p>
          {/if}
        </div>
        <div class="flex flex-wrap items-center gap-1">
          <Button size="sm" variant="primary" disabled={pending.has(row.id)} onclick={() => oncompare(row.id)}>Compare to now</Button>
          <Button size="sm" variant="ghost" disabled={pending.has(row.id)} onclick={() => beginRename(row.id, row.name)}>Rename</Button>
          <Button size="sm" variant="danger" disabled={pending.has(row.id)} onclick={() => ondelete(row.id)}>{pending.has(row.id) ? 'Deleting…' : 'Delete'}</Button>
        </div>
      </div>
    </li>
  {/each}
</ul>
