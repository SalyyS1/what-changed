<script lang="ts">
  import { untrack } from 'svelte';
  import Button from '../../components/ui/Button.svelte';
  import Input from '../../components/ui/Input.svelte';
  interface Props { from: number; to: number; onapply: (from: number, to: number) => void; }
  let { from, to, onapply }: Props = $props();

  function localInput(timestamp: number): string {
    const value = new Date(timestamp - new Date(timestamp).getTimezoneOffset() * 60_000);
    return value.toISOString().slice(0, 16);
  }

  let start = $state(untrack(() => localInput(from)));
  let end = $state(untrack(() => localInput(to)));
  function validTimestamp(value: number): boolean {
    return Number.isSafeInteger(value) && value >= 0;
  }
  const valid = $derived(validTimestamp(Date.parse(start)) && validTimestamp(Date.parse(end)) && Date.parse(start) < Date.parse(end));
</script>

<div class="flex flex-wrap items-end gap-2 rounded-lg border border-border-subtle bg-surface-1 p-3" data-testid="custom-range">
  <label class="min-w-48 flex-1 text-xs text-text-muted">From<Input type="datetime-local" bind:value={start} invalid={!valid} class="mt-1" /></label>
  <label class="min-w-48 flex-1 text-xs text-text-muted">To<Input type="datetime-local" bind:value={end} invalid={!valid} class="mt-1" /></label>
  <Button size="sm" variant="primary" disabled={!valid} onclick={() => onapply(Date.parse(start), Date.parse(end))}>Apply</Button>
</div>
