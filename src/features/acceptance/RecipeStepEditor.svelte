<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import type { RecipeStepDto, ReadBackAdapterDto, RecipeAssertionDto } from '../../lib/tauri/bindings';
  import LocatorEditor from './LocatorEditor.svelte';
  import { blankLocator, newAssertion, splitReferences } from './scenario-recipes';
  import { actionAdapters, canAddAssertion } from './scenario-validation';
  interface Props { step: RecipeStepDto; index: number; adapters: ReadBackAdapterDto[]; totalAssertions?: number; disabled?: boolean; onRemove?: () => void; }
  let { step = $bindable(), index, adapters, totalAssertions, disabled = false, onRemove }: Props = $props();
  const compatibleAdapters = $derived(actionAdapters(step.action.kind, adapters));
  const adapterOptions = $derived(compatibleAdapters.map((adapter) => ({ value: adapter.id, label: `Route ${adapters.indexOf(adapter) + 1} · ${adapter.role_ref}` })));
  const selectedAdapter = $derived.by(() => { const action = step.action; return 'adapter_id' in action ? compatibleAdapters.find((adapter) => adapter.id === action.adapter_id) : undefined; });
  function action(value: string): void {
    switch (value) {
      case 'navigate': step.action = { kind: 'navigate', path: '/' }; break;
      case 'fill': step.action = { kind: 'fill', locator: blankLocator(), fixture_field: '' }; break;
      case 'click': step.action = { kind: 'click', locator: blankLocator() }; break;
      case 'cleanup': step.action = { kind: 'cleanup', locator: blankLocator() }; break;
      case 'read_back': step.action = { kind: 'read_back', adapter_id: actionAdapters('read_back', adapters)[0]?.id ?? '' }; break;
      case 'deny_other_role': step.action = { kind: 'deny_other_role', adapter_id: actionAdapters('deny_other_role', adapters)[0]?.id ?? '' }; break;
    }
  }
  function assertionKind(value: string): RecipeAssertionDto {
    if (value === 'text_equals') return { kind: 'text_equals', locator: blankLocator(), expected: '' };
    if (value === 'field_equals') return { kind: 'field_equals', field: '', fixture_field: '' };
    if (value === 'denied') return { kind: 'denied', allowed_statuses: [403, 404], absent_fields: [] };
    return { kind: 'visible', locator: blankLocator() };
  }
</script>

<fieldset class="space-y-3 rounded-md border border-border-subtle p-3" {disabled}>
  <legend class="px-1 text-sm font-medium text-text-primary">Step {index + 1}</legend>
  <Select value={step.action.kind} onchange={action} label={`Step ${index + 1} action`} options={[{ value: 'navigate', label: 'Open a page' }, { value: 'fill', label: 'Fill a field with test data' }, { value: 'click', label: 'Activate a control' }, { value: 'read_back', label: 'Read back the recorded item' }, { value: 'deny_other_role', label: 'Check access with another test role' }, { value: 'cleanup', label: 'Clean up the test item' }]} {disabled} />
  {#if step.action.kind === 'navigate'}<label class="block text-xs text-text-secondary">Relative page path<Input bind:value={step.action.path} placeholder="/bookings" /></label>
  {:else if step.action.kind === 'fill'}<LocatorEditor bind:locator={step.action.locator} label="Field to fill" {disabled} /><label class="block text-xs text-text-secondary">Test-data field reference<Input bind:value={step.action.fixture_field} placeholder="customer_name" /></label>
  {:else if step.action.kind === 'click' || step.action.kind === 'cleanup'}<LocatorEditor bind:locator={step.action.locator} label="Control to activate" {disabled} />
  {:else}<Select bind:value={step.action.adapter_id} options={adapterOptions} label="Reviewed read-back route" {disabled} />{#if !selectedAdapter}<p class="text-xs text-warning">Choose a reviewed route with the matching purpose. Existing expected checks remain unchanged.</p>{/if}{/if}
  <div class="space-y-3 border-t border-border-subtle pt-3"><h4 class="text-xs font-medium text-text-primary">Expected checks</h4>
    {#each step.assertions as assertion, assertionIndex (assertion.id)}
      <div class="space-y-2 rounded-md bg-surface-2 p-3">
        <label class="block text-xs text-text-secondary">What must be true?<Textarea bind:value={assertion.expected_behavior} rows={2} /></label>
        <Select value={assertion.assertion.kind} onchange={(value) => (assertion.assertion = assertionKind(value))} label={`Step ${index + 1} check ${assertionIndex + 1} kind`} options={[{ value: 'visible', label: 'Named item is visible' }, { value: 'text_equals', label: 'Text matches exactly' }, { value: 'field_equals', label: 'Stored field matches test data', disabled: step.action.kind !== 'read_back' || !selectedAdapter }, { value: 'denied', label: 'Access denied with no selected fields', disabled: step.action.kind !== 'deny_other_role' || !selectedAdapter }]} {disabled} />
        {#if assertion.assertion.kind === 'visible' || assertion.assertion.kind === 'text_equals'}<LocatorEditor bind:locator={assertion.assertion.locator} label="Item to check" {disabled} />{#if assertion.assertion.kind === 'text_equals'}<label class="block text-xs text-text-secondary">Exact expected text<Input bind:value={assertion.assertion.expected} /></label>{/if}
        {:else if assertion.assertion.kind === 'field_equals'}<p class="text-xs text-text-secondary">Selected route response fields: {selectedAdapter?.allowed_fields.join(', ') || 'Choose a read-only route first.'}</p><label class="block text-xs text-text-secondary">Stored response field<Input bind:value={assertion.assertion.field} /></label><label class="block text-xs text-text-secondary">Matching test-data field<Input bind:value={assertion.assertion.fixture_field} /></label>
        {:else}<p class="text-xs text-text-secondary">Allowed denied statuses: {assertion.assertion.allowed_statuses.join(', ')}.</p><label class="block text-xs text-text-secondary">Fields that must be absent (comma separated)<Input value={assertion.assertion.absent_fields.join(', ')} oninput={(event) => { if (assertion.assertion.kind === 'denied') assertion.assertion.absent_fields = splitReferences(event.currentTarget.value); }} /></label>{/if}
        <Button size="sm" variant="ghost" {disabled} onclick={() => (step.assertions = step.assertions.filter((item) => item.id !== assertion.id))}>Remove check {assertionIndex + 1}</Button>
      </div>
    {/each}
    <div class="flex flex-wrap gap-2"><Button size="sm" variant="secondary" disabled={disabled || !canAddAssertion(step, totalAssertions ?? step.assertions.length)} onclick={() => { if (canAddAssertion(step, totalAssertions ?? step.assertions.length)) step.assertions = [...step.assertions, newAssertion()]; }}>Add expected check</Button><Button size="sm" variant="ghost" {disabled} onclick={onRemove}>Remove step {index + 1}</Button></div>
  </div>
</fieldset>
