<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Dialog from '../../components/ui/Dialog.svelte';
  import Select from '../../components/ui/Select.svelte';
  import type { OwnerCapabilityDto, OwnerChangeDraftDto, OwnerEnvironmentRevisionDto, OwnerRuleRevisionDto, ScenarioRefDto } from '../../lib/tauri/bindings';
  import { OwnerEvidenceStore } from '../../lib/stores/owner-evidence.svelte';
  import ScenarioEditor from './ScenarioEditor.svelte';
  import RunReviewScreen from './RunReviewScreen.svelte';
  import { candidateOrigins } from './scenario-validation';
  import VerificationRunnerPanel from '../verification-runner/VerificationRunnerPanel.svelte';
  interface Props { appId: string; changes: OwnerChangeDraftDto[]; capabilities: OwnerCapabilityDto[]; rules: OwnerRuleRevisionDto[]; environments: OwnerEnvironmentRevisionDto[]; active?: boolean; model?: OwnerEvidenceStore; }
  let { appId, changes, capabilities, rules, environments, active = true, model = new OwnerEvidenceStore() }: Props = $props();
  let selectedChange = $state(''); let selectedScenarios = $state<ScenarioRefDto[]>([]);
  let startOpen = $state(false); let startAcknowledged = $state(false);
  const options = $derived(changes.map((change) => ({ value: change.id, label: `${change.intent || 'Untitled request'} · saved v${change.version}` })));
  const selectedCandidate = $derived(model.context?.candidates.find((candidate) => candidate.id === model.context?.change.app.current_candidate_id));
  const originContext = $derived(candidateOrigins(environments, selectedCandidate));
  const candidateBound = $derived(Boolean(selectedCandidate && model.context?.change.change.candidate_id === selectedCandidate.id));
  $effect(() => {
    const target = { appId, selectedChange, active };
    untrack(() => {
      selectedScenarios = []; startOpen = false;
      if (target.active && target.selectedChange) void model.load(target.appId, target.selectedChange);
      else model.cancel();
    });
  });
  onDestroy(() => model.cancel());
  function toggle(ref: ScenarioRefDto, checked: boolean): void {
    selectedScenarios = selectedScenarios.filter((item) => item.scenario_id !== ref.scenario_id);
    if (checked) selectedScenarios.push(ref);
  }
  async function start(): Promise<void> { if (startAcknowledged && await model.start(selectedScenarios)) startOpen = false; }
</script>

<section class="space-y-4" aria-labelledby="acceptance-title">
  <Card><div class="flex flex-wrap items-center gap-2"><h2 id="acceptance-title" class="font-medium text-text-primary">Acceptance and owner decisions</h2><Badge tone="neutral">You decide what to accept</Badge></div><p class="mt-1 text-sm text-text-secondary">Choose a saved request, agree on its scenarios, then observe them manually or check a supported test preview. Review evidence before deciding.</p>
    {#if changes.length}<div class="mt-3"><Select bind:value={selectedChange} {options} label="Saved request to review" placeholder="Choose a saved request…" disabled={!active || model.saving} /></div>{:else}<p class="mt-3 text-sm text-text-muted">Save your first change request before defining its acceptance scenarios.</p>{/if}
  </Card>
  {#if selectedChange && active}
    <StateShell state={model.state} emptyTitle="Request unavailable" errorTitle="Acceptance workspace could not be opened" loadingRows={3}>
      {#snippet children(context)}
        {#if originContext.message}<p class="mb-3 text-sm text-warning" role="status">{originContext.message}</p>{/if}
        <ScenarioEditor {capabilities} {rules} scenarios={context.scenarios} origins={originContext.origins} saving={model.saving} disabled={!active} error={model.error} onConfirm={(fields, previous) => model.confirmScenario(fields, previous)} />
        <Card class="mt-4">
          <h3 class="font-medium text-text-primary">Start a manual observation attempt</h3><p class="mt-1 text-sm text-text-secondary">This freezes the saved request, exact rule and scenario revisions, selected candidate and preview settings. You perform the observations yourself; no target is contacted by this action.</p>
          <p class="mt-3 text-sm text-text-primary">App-selected candidate: {selectedCandidate?.identity ?? 'No candidate selected.'}</p>
          <p class="mt-1 text-xs text-text-muted">Saved request candidate: {context.change.candidate?.identity ?? 'Not bound to a candidate yet.'}</p>
          {#if selectedCandidate && !candidateBound}<div class="mt-3"><p class="mb-2 text-sm text-warning">Save this candidate for the request before starting. Changing it invalidates matching evidence from an older request version.</p><Button size="sm" variant="secondary" disabled={model.saving} onclick={() => void model.bindSelectedCandidate()}>Use selected candidate for this request</Button></div>{/if}
          <fieldset class="mt-4 space-y-3" disabled={!active || model.saving}><legend class="mb-2 text-sm font-medium text-text-primary">Confirmed scenarios for this attempt</legend>{#each model.latestScenarios as scenario (scenario.id)}<label class="flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" checked={selectedScenarios.some((ref) => ref.scenario_id === scenario.id && ref.revision === scenario.revision)} onchange={(event) => toggle({ scenario_id: scenario.id, revision: scenario.revision }, event.currentTarget.checked)} /><span>{scenario.title} · revision {scenario.revision} · {scenario.recipe.steps.reduce((count, step) => count + step.assertions.length, 0)} named checks</span></label>{:else}<p class="text-sm text-text-muted">Confirm at least one scenario before starting an observation attempt.</p>{/each}</fieldset>
          {#if model.error}<p class="mt-3 text-sm text-danger" role="alert">{model.error}</p>{/if}
          <Button class="mt-4" variant="primary" disabled={!active || model.saving || !candidateBound || !selectedScenarios.length} onclick={() => { startAcknowledged = false; startOpen = true; }}>Review and freeze attempt</Button>
        </Card>
        <VerificationRunnerPanel change={context.change} scenarios={model.latestScenarios.filter((scenario) => selectedScenarios.some((ref) => ref.scenario_id === scenario.id && ref.revision === scenario.revision))} disabled={!active || model.saving || !candidateBound} onRunChanged={(runId) => model.reload(runId)} />
        <Card class="mt-4"><h3 class="font-medium text-text-primary">Observation history</h3>{#if context.runs.length}<ul class="mt-3 space-y-2">{#each context.runs as run (run.id)}<li><button class="flex w-full flex-wrap items-center justify-between gap-2 rounded-md border border-border-subtle p-3 text-left text-sm hover:bg-surface-2" disabled={model.saving} onclick={() => void model.selectRun(run.id)}><span class="text-text-primary">{run.manifest.candidate.identity} · saved request v{run.manifest.change.version}</span><span class="text-text-secondary">{run.lifecycle} · {run.method.replaceAll('_', ' ')}{run.retry_of ? ' · retry' : ''}</span></button></li>{/each}</ul>{:else}<p class="mt-2 text-sm text-text-muted">No observation attempts. Confirming rules and saving a brief does not create evidence.</p>{/if}</Card>
        {#if model.runId}<div class="mt-4"><StateShell state={model.review} emptyTitle="No attempt selected" errorTitle="Observation review could not be opened" loadingRows={3}>{#snippet children(review)}{#key review.run.id}<RunReviewScreen {review} {model} disabled={!active} />{/key}{/snippet}</StateShell></div>{/if}
      {/snippet}
    </StateShell>
  {/if}
</section>

<Dialog bind:open={startOpen} title="Freeze this manual observation attempt?" description="You will perform and record the checks yourself. This action does not run a browser or establish the deployed build." width="lg">
  <div class="space-y-3"><p class="text-sm text-text-primary">Candidate: {selectedCandidate?.identity ?? 'Unavailable'} · self-reported or declared identity.</p><ul class="list-inside list-disc text-sm text-text-secondary">{#each selectedScenarios as ref (`${ref.scenario_id}:${ref.revision}`)}<li>{model.latestScenarios.find((item) => item.id === ref.scenario_id)?.title ?? 'Scenario'} · revision {ref.revision}</li>{/each}</ul><label class="flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" bind:checked={startAcknowledged} />I reviewed the test roles, isolated test data, routes and expected checks. I understand this attempt contains manual observations with unknown build freshness.</label>{#if model.error}<p class="text-sm text-danger" role="alert">{model.error}</p>{/if}</div>
  {#snippet footer()}<Button variant="ghost" onclick={() => (startOpen = false)}>Keep reviewing</Button><Button variant="primary" disabled={!startAcknowledged || model.saving} onclick={() => void start()}>{model.saving ? 'Freezing…' : 'Freeze manual attempt'}</Button>{/snippet}
</Dialog>
