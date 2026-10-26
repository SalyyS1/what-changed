<script lang="ts">
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import type { InvestigationWindow } from './investigation-types';

  interface Props {
    value?: InvestigationWindow;
    disabled?: boolean;
  }

  let { value = $bindable({ kind: 'today' }), disabled = false }: Props = $props();
  let checkpointId = $state(value.kind === 'since_checkpoint' ? String(value.id) : '');
  let customFrom = $state(toLocalInput(value.kind === 'custom' ? value.from : Date.now() - 60 * 60_000));
  let customTo = $state(toLocalInput(value.kind === 'custom' ? value.to : Date.now()));

  const options = [
    { value: 'last_hour', label: 'Last hour' },
    { value: 'today', label: 'Today' },
    { value: 'since_yesterday', label: 'Since yesterday' },
    { value: 'since_last_restart', label: 'Since last restart' },
    { value: 'since_checkpoint', label: 'Since checkpoint' },
    { value: 'custom', label: 'Custom range' },
  ] as const;

  function toLocalInput(timestamp: number): string {
    const date = new Date(timestamp - new Date(timestamp).getTimezoneOffset() * 60_000);
    return date.toISOString().slice(0, 16);
  }

  function select(kind: string): void {
    if (kind === 'since_checkpoint') {
      value = { kind, id: Number(checkpointId) || 0 };
    } else if (kind === 'custom') {
      value = { kind, from: new Date(customFrom).getTime(), to: new Date(customTo).getTime() };
    } else if (kind === 'last_hour' || kind === 'today' || kind === 'since_yesterday' || kind === 'since_last_restart') {
      value = { kind };
    }
  }

  function updateCheckpoint(): void {
    value = { kind: 'since_checkpoint', id: Number(checkpointId) || 0 };
  }

  function updateCustom(): void {
    value = { kind: 'custom', from: new Date(customFrom).getTime(), to: new Date(customTo).getTime() };
  }
</script>

<div class="grid grid-cols-1 gap-2 @[620px]/shell:grid-cols-2">
  <Select
    label="Time window"
    value={value.kind}
    {options}
    {disabled}
    onchange={select}
  />
  {#if value.kind === 'since_checkpoint'}
    <Input
      bind:value={checkpointId}
      label="Checkpoint ID"
      type="number"
      min="1"
      placeholder="Checkpoint ID"
      {disabled}
      oninput={updateCheckpoint}
    />
  {:else if value.kind === 'custom'}
    <div class="grid grid-cols-2 gap-2">
      <Input bind:value={customFrom} label="From" type="datetime-local" {disabled} oninput={updateCustom} />
      <Input bind:value={customTo} label="To" type="datetime-local" {disabled} oninput={updateCustom} />
    </div>
  {:else}
    <p class="self-center text-xs text-text-muted">The window is resolved against your local event history.</p>
  {/if}
</div>
