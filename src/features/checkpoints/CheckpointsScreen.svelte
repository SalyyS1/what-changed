<script lang="ts">
  import { onMount } from 'svelte';
  import ScreenFrame from '../../components/layout/ScreenFrame.svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Dialog from '../../components/ui/Dialog.svelte';
  import { checkpoints, type CheckpointsStore } from '../../lib/stores/checkpoints.svelte';
  import type { CheckpointDto } from '../../lib/tauri/bindings';
  import { backendAvailable } from '../../lib/tauri/client';
  import type { AsyncState } from '../../lib/types/async-state';
  import CheckpointList from './CheckpointList.svelte';
  import { checkpointRow } from './checkpoint-row';
  import ComparisonView from './ComparisonView.svelte';
  import CreateCheckpointDialog from './CreateCheckpointDialog.svelte';

  interface Props { data?: AsyncState<CheckpointDto[]>; model?: CheckpointsStore; }
  let { data, model = checkpoints }: Props = $props();
  let createOpen = $state(false);
  let deleteTarget = $state<CheckpointDto | null>(null);
  let deleteOpen = $state(false);
  const live = $derived(data === undefined && backendAvailable());
  const rows = $derived(data ?? model.state);
  const selected = $derived(model.rows.find((row) => row.id === model.selectedId) ?? null);

  onMount(() => {
    if (!live) return;
    let unlisten = () => {};
    let disposed = false;
    void model.attachProgress().then((stop) => { if (disposed) stop(); else unlisten = stop; });
    void model.load();
    return () => { disposed = true; unlisten(); };
  });

  async function create(name: string, reason: string): Promise<void> {
    if (live && await model.create(name, reason)) createOpen = false;
  }
  async function confirmDelete(): Promise<void> {
    if (deleteTarget !== null && await model.remove(deleteTarget.id)) { deleteTarget = null; deleteOpen = false; }
  }
</script>

<ScreenFrame title="Checkpoints" subtitle="Known-good views of this PC that stay useful beyond event retention.">
  {#snippet actions()}
    <Button variant="primary" disabled={!live || model.creating} onclick={() => (createOpen = true)}>{model.creating ? 'Creating…' : 'Create checkpoint'}</Button>
  {/snippet}

  {#if model.error && live}<p class="mb-3 rounded-md border border-danger/30 bg-danger/8 px-3 py-2 text-sm text-danger" role="alert">{model.error}</p>{/if}

  {#if selected && model.comparison}
    <ComparisonView checkpoint={checkpointRow(selected)} diff={model.comparison} onback={() => model.closeComparison()} />
  {:else if model.comparing && selected}
    <div class="grid min-h-48 place-items-center text-sm text-text-muted" aria-live="polite">Comparing {selected.name} to now…</div>
  {:else}
    <StateShell state={rows} emptyTitle="No checkpoints yet." emptyDescription="Create one while things work, so you can compare later." emptyIcon="checkpoint" loadingRows={4}>
      {#snippet children(list)}
        <CheckpointList
          rows={list}
          pending={model.mutating}
          oncompare={(id) => { if (live) void model.compareToNow(id); }}
          onrename={(id, name) => { if (live) void model.rename(id, name); }}
          ondelete={(id) => { deleteTarget = list.find((row) => row.id === id) ?? null; deleteOpen = deleteTarget !== null; }}
        />
      {/snippet}
    </StateShell>
  {/if}
</ScreenFrame>

<CreateCheckpointDialog bind:open={createOpen} creating={model.creating} error={model.error} progress={model.progress} completed={model.progressCompleted} total={model.progressTotal} oncreate={(name, reason) => void create(name, reason)} />

<Dialog bind:open={deleteOpen} title="Delete checkpoint?" description="This removes the saved snapshot. Recorded timeline events are not deleted.">
  <p class="text-sm text-text-secondary">{deleteTarget?.name}</p>
  {#snippet footer()}
    <Button variant="ghost" onclick={() => { deleteTarget = null; deleteOpen = false; }}>Cancel</Button>
    <Button variant="danger" disabled={deleteTarget ? model.mutating.has(deleteTarget.id) : false} onclick={() => void confirmDelete()}>Delete checkpoint</Button>
  {/snippet}
</Dialog>
