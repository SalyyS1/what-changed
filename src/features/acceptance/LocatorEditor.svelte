<script lang="ts">
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import type { LocatorDto } from '../../lib/tauri/bindings';
  interface Props { locator: LocatorDto; label: string; disabled?: boolean; }
  let { locator = $bindable(), label, disabled = false }: Props = $props();
  function kind(value: string): void {
    if (value === 'role') locator = { kind: 'role', role: 'button', name: '' };
    if (value === 'label') locator = { kind: 'label', text: '' };
    if (value === 'test_id') locator = { kind: 'test_id', value: '' };
  }
</script>

<fieldset class="space-y-2" {disabled}><legend class="mb-1 text-xs text-text-muted">{label}</legend>
  <Select value={locator.kind} onchange={kind} label={`${label} lookup method`} options={[{ value: 'label', label: 'Visible field label' }, { value: 'role', label: 'Role and accessible name' }, { value: 'test_id', label: 'Stable test ID' }]} {disabled} />
  {#if locator.kind === 'role'}<label class="block text-xs text-text-secondary">Control role<Input bind:value={locator.role} /></label><label class="block text-xs text-text-secondary">Accessible name<Input bind:value={locator.name} /></label>
  {:else if locator.kind === 'label'}<label class="block text-xs text-text-secondary">Visible label<Input bind:value={locator.text} /></label>
  {:else}<label class="block text-xs text-text-secondary">Stable test ID<Input bind:value={locator.value} /></label>{/if}
</fieldset>
