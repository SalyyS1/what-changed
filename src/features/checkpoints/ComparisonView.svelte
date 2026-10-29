<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Switch from '../../components/ui/Switch.svelte';
  import type { CheckpointDiffDto } from '../../lib/tauri/bindings';
  import type { CheckpointRow } from './checkpoint-row';
  import { collectorDisplayName } from './collector-display';
  import DiffSection from './DiffSection.svelte';
  interface Props { checkpoint: CheckpointRow; diff: CheckpointDiffDto; onback: () => void; }
  let { checkpoint, diff, onback }: Props = $props();
  let technical = $state(false);
  const eventHref = $derived(`#/timeline?range=custom&from=${diff.window_from}&to=${diff.window_to}`);
  const leftDiffMissing = $derived(diff.left_missing_collectors.map(collectorDisplayName));
  const rightMissing = $derived([...new Set(diff.right_missing_collectors.map(collectorDisplayName))]);
  const skipped = $derived(diff.skipped_collectors.map(collectorDisplayName));
  const leftMissing = $derived([...new Set([...checkpoint.missingCollectors, ...leftDiffMissing])]);
  const skippedOnly = $derived(
    [...new Set(skipped)].filter(
      (collector) => !leftMissing.includes(collector) && !rightMissing.includes(collector),
    ),
  );
  const partial = $derived(leftMissing.length > 0 || rightMissing.length > 0 || diff.skipped_collectors.length > 0);
</script>

<div class="flex flex-col gap-5">
  <div class="flex flex-col gap-3 @[600px]/shell:flex-row @[600px]/shell:items-start @[600px]/shell:justify-between">
    <div>
      <Button size="sm" variant="ghost" onclick={onback}>← All checkpoints</Button>
      <h1 class="mt-2 font-display text-xl text-text-primary">{checkpoint.name} → now</h1>
      <p class="mt-1 text-sm text-text-secondary">{diff.added.length} added · {diff.removed.length} removed · {diff.changed.length} changed</p>
    </div>
    <label class="flex items-center gap-2 text-sm text-text-secondary">
      <Switch bind:checked={technical} label="Show technical details" />
      Show technical details
    </label>
  </div>

  {#if checkpoint.partial || partial}
    <div class="rounded-md border border-warning/35 bg-warning/8 px-3 py-2 text-sm text-warning">
      <p class="font-medium">{checkpoint.partial ? 'This checkpoint is partial.' : 'This comparison is partial.'}</p>
      {#each leftMissing as collector (collector)}
        <p>{collector} not compared — missing from this checkpoint</p>
      {/each}
      {#each rightMissing as collector (collector)}
        <p>{collector} not compared — missing from the current state</p>
      {/each}
      {#each skippedOnly as collector (collector)}
        <p>{collector} not compared — comparison is unavailable for this collector</p>
      {/each}
    </div>
  {/if}

  {#if diff.event_ids_in_window.length > 0}
    <a class="rounded-md border border-border-subtle bg-surface-2 px-3 py-2 text-sm text-accent hover:border-border-active" href={eventHref}>{diff.event_ids_in_window.length} changes recorded in this window → Open timeline</a>
  {:else}
    <p class="rounded-md border border-border-subtle bg-surface-2 px-3 py-2 text-sm text-text-muted">No recorded timeline changes in this window.</p>
  {/if}

  <div class="grid grid-cols-1 gap-5 @[940px]/shell:grid-cols-3">
    <DiffSection title="Added" kind="added" groups={diff.grouped_by_category} {technical} />
    <DiffSection title="Removed" kind="removed" groups={diff.grouped_by_category} {technical} />
    <DiffSection title="Changed" kind="changed" groups={diff.grouped_by_category} {technical} />
  </div>
</div>
