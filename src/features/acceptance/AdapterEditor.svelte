<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import type { OriginDto, ReadBackAdapterDto } from '../../lib/tauri/bindings';
  import { splitReferences } from './scenario-recipes';
  import { originKey, uniqueOrigins } from '../owner-workspace/owner-origins';
  interface Props { adapters: ReadBackAdapterDto[]; origins: OriginDto[]; disabled?: boolean; }
  let { adapters = $bindable(), origins, disabled = false }: Props = $props();
  const options = $derived(uniqueOrigins(origins).map((origin) => ({ value: originKey(origin), label: originKey(origin) })));
  function add(): void {
    if (!origins[0]) return;
    adapters = [...adapters, { id: crypto.randomUUID(), origin: { ...origins[0] }, path_template: '/api/bookings/{fixture_id}', role_ref: '', allowed_fields: [], effect: 'read_only' }];
  }
</script>

<section class="space-y-3" aria-labelledby="adapter-heading">
  <h3 id="adapter-heading" class="text-sm font-medium text-text-primary">Reviewed read-back routes</h3>
  <p class="text-xs text-text-secondary">Only named routes and selected response fields are described. Saving them does not contact the target. A real read-back observation needs the declared test role and exact test item.</p>
  {#each adapters as adapter, index (adapter.id)}
    <fieldset class="space-y-2 rounded-md border border-border-subtle p-3" {disabled}><legend class="px-1 text-xs text-text-muted">Route {index + 1}</legend>
      <Select value={originKey(adapter.origin)} options={options} label={`Route ${index + 1} exact preview origin`} onchange={(value) => { const selected = origins.find((origin) => originKey(origin) === value); if (selected) adapter.origin = { ...selected }; }} {disabled} />
      <label class="block text-xs text-text-secondary">Relative route with one test-item slot<Input bind:value={adapter.path_template} placeholder={'/api/bookings/{fixture_id}'} /></label>
      <label class="block text-xs text-text-secondary">Declared test-role reference<Input bind:value={adapter.role_ref} placeholder="staff-test" /></label>
      <label class="block text-xs text-text-secondary">Allowed response fields (comma separated)<Input value={adapter.allowed_fields.join(', ')} oninput={(event) => (adapter.allowed_fields = splitReferences(event.currentTarget.value))} /></label>
      <Select value={adapter.effect} onchange={(value) => { if (value === 'read_only' || value === 'read_denied') adapter.effect = value; }} label={`Route ${index + 1} purpose`} options={[{ value: 'read_only', label: 'Read matching stored fields' }, { value: 'read_denied', label: 'Check denied access' }]} {disabled} />
      <Button size="sm" variant="ghost" {disabled} onclick={() => (adapters = adapters.filter((item) => item.id !== adapter.id))}>Remove route {index + 1}</Button>
    </fieldset>
  {/each}
  <Button size="sm" variant="secondary" disabled={disabled || !origins.length || adapters.length >= 16} onclick={add}>Add reviewed route</Button>
  {#if !origins.length}<p class="text-xs text-warning">Select a candidate whose current preview settings include the exact page and API origins before adding read-back or access checks.</p>{/if}
</section>
