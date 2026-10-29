<script lang="ts">
  import { untrack } from 'svelte';
  import Button from '../../components/ui/Button.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import type { ManualAssertionObservationInput, ScenarioRevisionDto } from '../../lib/tauri/bindings';
  import type { OwnerObservationFields } from '../../lib/stores/owner-evidence.svelte';
  import { describeAction, describeAssertion } from './scenario-recipes';
  interface Props { scenario: ScenarioRevisionDto; saving?: boolean; disabled?: boolean; error?: string | null; onRecord: (fields: OwnerObservationFields) => Promise<boolean>; }
  let { scenario, saving = false, disabled = false, error = null, onRecord }: Props = $props();
  let observer = $state('');
  const now = new Date(); now.setMinutes(now.getMinutes() - now.getTimezoneOffset());
  let observedAt = $state(now.toISOString().slice(0, 16));
  let observations = $state<Record<string, string>>({});
  // Each editor belongs to one frozen scenario revision; a different run
  // mounts a new editor instead of carrying these observations forward.
  let assertions = $state<ManualAssertionObservationInput[]>(untrack(() => scenario.recipe.steps.flatMap((step) => step.assertions.map((assertion) => ({ assertion_id: assertion.id, status: 'not_run', observed: '', limitation: null })))));
  let limitation = $state(''); let validation = $state<string | null>(null);
  async function record(): Promise<void> {
    const time = new Date(observedAt).getTime();
    if (!observer.trim() || !Number.isFinite(time)) { validation = 'Record who observed this scenario and when.'; return; }
    if (assertions.some((assertion) => assertion.status !== 'not_run' && !assertion.observed.trim())) { validation = 'Describe what you observed for every check marked passed, failed or blocked.'; return; }
    validation = null;
    await onRecord({ scenario_id: scenario.id, scenario_revision: scenario.revision, observer: observer.trim(), observed_at: time,
      observed_steps: scenario.recipe.steps.filter((step) => observations[step.id]?.trim()).map((step) => ({ step_id: step.id, observed: observations[step.id]!.trim() })),
      assertions: $state.snapshot(assertions), limitation: limitation.trim() || null });
  }
</script>

<fieldset class="mt-3 space-y-4" disabled={disabled || saving}><legend class="sr-only">Record manual observations for {scenario.title}</legend>
  <p class="text-sm text-text-secondary">Record only what you personally observed. Every check starts as Not run. This form does not open the preview or execute any steps.</p>
  <label class="block text-sm text-text-secondary">Observer<Input bind:value={observer} maxlength={120} placeholder="Your name or owner label" /></label>
  <label class="block text-sm text-text-secondary">Observation time (your local time)<Input type="datetime-local" bind:value={observedAt} /></label>
  <div class="space-y-3">{#each scenario.recipe.steps as step, stepIndex (step.id)}
    <section class="space-y-3 rounded-md border border-border-subtle p-3" aria-label={`Observed step ${stepIndex + 1}`}>
      <h4 class="text-sm font-medium text-text-primary">Step {stepIndex + 1}: {describeAction(step.action, scenario.recipe.adapters)}</h4>
      <label class="block text-xs text-text-secondary">What you actually did and saw<Textarea value={observations[step.id] ?? ''} oninput={(event) => (observations[step.id] = event.currentTarget.value)} rows={2} maxlength={2000} /></label>
      {#each step.assertions as expected (expected.id)}
        {@const index = assertions.findIndex((item) => item.assertion_id === expected.id)}
        {@const observation = assertions[index]!}
        <div class="space-y-2 border-t border-border-subtle pt-3"><p class="text-sm text-text-primary">Expected: {expected.expected_behavior}</p><p class="text-xs text-text-secondary">Exact check: {describeAssertion(expected.assertion)}</p>
          <Select value={observation.status} onchange={(value) => { if (value === 'passed' || value === 'failed' || value === 'blocked' || value === 'not_run') observation.status = value; }} label={`Outcome for ${expected.expected_behavior}`} options={[{ value: 'not_run', label: 'Not run' }, { value: 'passed', label: 'Passed · observed exactly as expected' }, { value: 'failed', label: 'Failed · observed a mismatch' }, { value: 'blocked', label: 'Blocked · could not perform the check' }]} disabled={disabled || saving} />
          <label class="block text-xs text-text-secondary">Observed result<Textarea bind:value={observation.observed} rows={2} maxlength={2000} placeholder="Describe the actual result or why this check was not run." /></label>
          <label class="block text-xs text-text-secondary">Limitation for this check (optional)<Input value={observation.limitation ?? ''} oninput={(event) => (observation.limitation = event.currentTarget.value || null)} maxlength={1000} /></label>
        </div>
      {/each}
    </section>
  {/each}</div>
  <label class="block text-sm text-text-secondary">Scenario limitations (optional)<Textarea bind:value={limitation} rows={2} maxlength={2000} placeholder="Missing role, read-back route, cleanup or uncertainty about the tested build." /></label>
  {#if validation || error}<p class="text-sm text-danger" role="alert">{validation ?? error} Your observations remain in this form.</p>{/if}
  <Button size="sm" variant="primary" disabled={disabled || saving} onclick={() => void record()}>{saving ? 'Saving…' : 'Record this observation'}</Button>
</fieldset>
