<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Dialog from '../../components/ui/Dialog.svelte';
  import Select from '../../components/ui/Select.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import type { DecisionKind, OwnerEvidenceReviewDto, ScenarioRefDto } from '../../lib/tauri/bindings';
  import type { OwnerDecisionFields } from '../../lib/stores/owner-evidence.svelte';
  interface Props { open?: boolean; review: OwnerEvidenceReviewDto; saving?: boolean; error?: string | null; onRecord: (fields: OwnerDecisionFields) => Promise<boolean>; }
  let { open = $bindable(false), review, saving = false, error = null, onRecord }: Props = $props();
  let decision = $state<DecisionKind>('deferred'); let reason = $state(''); let unknownBuild = $state(false);
  let acknowledged = $state<ScenarioRefDto[]>([]);
  const options = $derived([{ value: 'deferred', label: 'Defer · keep reviewing' }, { value: 'rejected', label: 'Reject this candidate' }, { value: 'accepted_with_exceptions', label: 'Accept with explicit exceptions', disabled: !review.can_accept_with_exceptions }, { value: 'accepted', label: 'Accept verified change', disabled: !review.can_accept }]);
  const exceptions = $derived(decision === 'accepted_with_exceptions');
  const needsUnknownBuild = $derived(review.freshness === 'unknown');
  const canRecord = $derived(reason.trim().length > 0 && (!exceptions || (review.can_accept_with_exceptions && (!needsUnknownBuild || unknownBuild) && review.unresolved_scenarios.every((ref) => acknowledged.some((item) => item.scenario_id === ref.scenario_id && item.revision === ref.revision)))) && (decision !== 'accepted' || review.can_accept));
  function toggle(ref: ScenarioRefDto, checked: boolean): void { acknowledged = acknowledged.filter((item) => item.scenario_id !== ref.scenario_id || item.revision !== ref.revision); if (checked) acknowledged.push({ ...ref }); }
  async function record(): Promise<void> {
    if (!canRecord) return;
    if (await onRecord({ decision, reason: reason.trim(), acknowledged_unresolved: exceptions ? $state.snapshot(acknowledged) : [], acknowledge_unknown_build: exceptions && needsUnknownBuild && unknownBuild })) { open = false; reason = ''; unknownBuild = false; acknowledged = []; }
  }
</script>

<Dialog bind:open title="Record your decision" description="This records the owner's decision. It does not change failed, blocked or missing observations." width="lg">
  <div class="space-y-4">
    <p class="text-sm text-text-secondary">This attempt has {review.freshness} evidence freshness. Your decision preserves the recorded results and their limitations.</p>
    <Select value={decision} onchange={(value) => { if (value === 'accepted' || value === 'accepted_with_exceptions' || value === 'rejected' || value === 'deferred') decision = value; }} {options} label="Owner decision" disabled={saving} />
    <label class="block text-sm text-text-secondary">Reason for this decision<Textarea bind:value={reason} rows={3} maxlength={4000} placeholder="Describe the trade-off and why you are accepting, rejecting or deferring." disabled={saving} /></label>
    {#if exceptions}
      <fieldset class="space-y-3" disabled={saving}><legend class="mb-2 text-sm font-medium text-text-primary">Acknowledge each exception</legend>
        {#if needsUnknownBuild}<label class="flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" bind:checked={unknownBuild} />I understand the tested build is not independently established and freshness is unknown.</label>{/if}
        {#each review.unresolved_scenarios as ref (`${ref.scenario_id}:${ref.revision}`)}{@const scenario = review.scenarios.find((item) => item.scenario.id === ref.scenario_id && item.scenario.revision === ref.revision)}<label class="flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" checked={acknowledged.some((item) => item.scenario_id === ref.scenario_id && item.revision === ref.revision)} onchange={(event) => toggle(ref, event.currentTarget.checked)} /><span>{scenario?.scenario.title ?? 'Unresolved scenario'} · revision {ref.revision} · {scenario?.status.replaceAll('_', ' ') ?? 'unresolved'}. I accept this explicit gap.</span></label>{/each}
      </fieldset>
    {/if}
    {#if review.decision_blockers.length}<ul class="list-inside list-disc text-sm text-warning">{#each review.decision_blockers as blocker (blocker)}<li>{blocker}</li>{/each}</ul>{/if}
    {#if error}<p class="text-sm text-danger" role="alert">{error} Your reason and acknowledgments remain here.</p>{/if}
  </div>
  {#snippet footer()}<Button variant="ghost" onclick={() => (open = false)}>Keep reviewing</Button><Button variant="primary" disabled={!canRecord || saving} onclick={() => void record()}>{saving ? 'Recording…' : 'Record owner decision'}</Button>{/snippet}
</Dialog>
