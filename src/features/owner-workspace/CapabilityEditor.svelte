<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import type { OwnerCapabilityDto } from '../../lib/tauri/bindings';
  import type { OwnerWorkspaceStore } from '../../lib/stores/owner-workspace.svelte';

  interface Props { capabilities: OwnerCapabilityDto[]; model: OwnerWorkspaceStore; active?: boolean; }
  let { capabilities, model, active = true }: Props = $props();
  let label = $state('');
  let description = $state('');
  let editing = $state<string | null>(null);
  let message = $state<string | null>(null);
  const selected = $derived(capabilities.find((item) => item.id === editing));
  function edit(item: OwnerCapabilityDto): void { editing = item.id; label = item.label; description = item.description; message = null; }
  function add(): void { editing = 'new'; label = ''; description = ''; message = null; }
  async function save(): Promise<void> {
    if (!label.trim()) { message = 'Name the capability in the words an owner would use.'; return; }
    const ok = await model.saveCapability({ label: label.trim(), description: description.trim() }, selected);
    if (ok) { editing = null; label = ''; description = ''; message = 'Saved. Confirm the expected behavior before relying on it.'; }
    else message = model.error ?? 'The capability could not be saved.';
  }
</script>

<Card level={1}>
  <div class="flex items-start justify-between gap-3"><div><h2 class="font-medium text-text-primary">What should this app do?</h2><p class="mt-1 text-sm text-text-secondary">Keep capabilities concrete. Suggestions stay drafts until you confirm the expected behavior.</p></div><Button size="sm" variant="secondary" disabled={!active || model.saving} onclick={add}>Add capability</Button></div>
  <div class="mt-4 space-y-2">
    {#if capabilities.length === 0}<p class="rounded-md border border-dashed border-border-subtle px-3 py-4 text-sm text-text-muted">No capabilities recorded yet.</p>{/if}
    {#each capabilities as item (item.id)}<button class="block w-full rounded-md border border-border-subtle px-3 py-3 text-left hover:bg-surface-2" aria-pressed={editing === item.id} onclick={() => edit(item)}><span class="flex items-center justify-between gap-2"><span class="font-medium text-text-primary">{item.label}</span><Badge tone="neutral">v{item.version}</Badge></span><span class="mt-1 block text-sm text-text-secondary">{item.description || 'No example yet.'}</span></button>{/each}
  </div>
  {#if editing !== null || label !== ''}
    <div class="mt-4 border-t border-border-subtle pt-4 space-y-3">
      <label class="block text-sm text-text-secondary">Capability name<Input bind:value={label} maxlength={120} placeholder="Create a booking" /></label>
      <label class="block text-sm text-text-secondary">Example or expected behavior<Textarea bind:value={description} rows={2} maxlength={1000} placeholder="A staff member can see the new booking." /></label>
      {#if message}<p class="text-sm {message.startsWith('Saved') ? 'text-success' : 'text-danger'}" role="status">{message}</p>{/if}
      <div class="flex gap-2"><Button size="sm" variant="primary" disabled={!active || model.saving} onclick={() => void save()}>{model.saving ? 'Saving…' : 'Save capability'}</Button><Button size="sm" variant="ghost" onclick={add}>Clear</Button></div>
    </div>
  {/if}
</Card>
