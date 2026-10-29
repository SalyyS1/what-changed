<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import type { OwnerEvidenceReviewDto } from '../../lib/tauri/bindings';
  import type { OwnerEvidenceStore } from '../../lib/stores/owner-evidence.svelte';
  import { formatAbsoluteTime } from '../../lib/formatting/time';
  import ManualResultEditor from './ManualResultEditor.svelte';
  import OwnerDecisionDialog from './OwnerDecisionDialog.svelte';
  import OwnerRunMap from '../../components/owner-map/OwnerRunMap.svelte';
  import EvidenceCaptureGallery from '../verification-runner/EvidenceCaptureGallery.svelte';
  interface Props { review: OwnerEvidenceReviewDto; model: OwnerEvidenceStore; disabled?: boolean; }
  let { review, model, disabled = false }: Props = $props();
  let decisionOpen = $state(false);
  const terminal = $derived(['completed', 'cancelled', 'interrupted'].includes(review.run.lifecycle));
  const statusTone = (status: string): 'danger' | 'warning' | 'success' | 'neutral' => status === 'failed' ? 'danger' : status === 'blocked' ? 'warning' : 'neutral';
</script>

<Card>
  <h2 class="font-medium text-text-primary">Evidence review</h2>
  <div class="mt-2 flex flex-wrap gap-2"><Badge tone="neutral">{review.run.lifecycle}</Badge><Badge tone="neutral">Method: {review.run.method.replaceAll('_', ' ')}</Badge><Badge tone="warning">Freshness: {review.freshness}</Badge><Badge tone={statusTone(review.status)}>Result: {review.status.replaceAll('_', ' ')}</Badge></div>
  <p class="mt-2 text-sm text-text-secondary">Frozen candidate: {review.run.manifest.candidate.identity} · {review.run.manifest.candidate.trust.replaceAll('_', ' ')}. Environment revision {review.run.manifest.environment.revision}. Request version {review.run.manifest.change.version}.</p>
  <p class="mt-1 text-xs text-text-muted">Started {formatAbsoluteTime(review.run.started_at)}{review.run.retry_of ? ' · separate retry attempt; original observations remain in history' : ''}.</p>
  {#if review.run.execution}<p class="mt-2 break-words text-xs text-text-secondary">Runner-observed identity: {review.run.execution.identity} · revision {review.run.execution.revision ?? 'unknown'} · semantics {review.run.execution.runner_semantics} · scope {review.run.execution.scope_digest}.</p>{/if}
  {#if review.run.method === 'automated'}<EvidenceCaptureGallery appId={review.run.app_id} runId={review.run.id} />{/if}
  <p class="mt-3 text-sm text-text-secondary">{review.counts.passed} passed · {review.counts.failed} failed · {review.counts.blocked} blocked · {review.counts.not_run} not run. Counts apply to this attempt's frozen scenario revisions.</p>
  <ul class="mt-3 list-inside list-disc space-y-1 text-sm text-warning">{#each review.freshness_reasons as reason (reason)}<li>{reason}</li>{/each}{#each review.limitations as limitation (limitation)}<li>{limitation}</li>{/each}</ul>
  <div class="mt-4 space-y-4">{#each review.scenarios as evidence (`${evidence.scenario.id}:${evidence.scenario.revision}`)}
    <section class="rounded-md border border-border-subtle p-3" aria-labelledby={`scenario-${evidence.scenario.id}`}>
      <div class="flex flex-wrap items-center gap-2"><h3 id={`scenario-${evidence.scenario.id}`} class="text-sm font-medium text-text-primary">{evidence.scenario.title}</h3><Badge tone={statusTone(evidence.status)}>{evidence.status.replaceAll('_', ' ')}</Badge><Badge tone="neutral">Frozen revision {evidence.scenario.revision}</Badge></div>
      <p class="mt-2 whitespace-pre-wrap text-sm text-text-secondary">Expected: {evidence.scenario.expected_behavior}</p>
      {#if evidence.result}<div class="mt-3 space-y-3"><p class="text-xs text-text-muted">Observed by {evidence.result.observer} at {formatAbsoluteTime(evidence.result.observed_at)} · {evidence.result.method.replaceAll('_', ' ')}</p><ul class="space-y-2">{#each evidence.result.assertions as assertion (assertion.assertion_id)}<li class="rounded-md bg-surface-2 p-3"><Badge tone={statusTone(assertion.status)}>{assertion.status.replaceAll('_', ' ')}</Badge><p class="mt-2 text-sm text-text-primary">Expected: {assertion.expected}</p><p class="mt-1 whitespace-pre-wrap text-sm text-text-secondary">Observed: {assertion.observed || 'No observation supplied.'}</p>{#if assertion.limitation}<p class="mt-1 text-xs text-warning">Limitation: {assertion.limitation}</p>{/if}</li>{/each}</ul>{#if evidence.result.limitation}<p class="text-sm text-warning">Scenario limitation: {evidence.result.limitation}</p>{/if}<details class="text-xs text-text-secondary"><summary class="cursor-pointer">Recorded steps</summary><ul class="mt-2 space-y-1">{#each evidence.result.observed_steps as step (step.step_id)}<li class="whitespace-pre-wrap">{step.observed}</li>{/each}</ul></details></div>
      {:else if review.run.lifecycle === 'running' && review.run.method === 'owner_manual'}<ManualResultEditor scenario={evidence.scenario} saving={model.saving} {disabled} error={model.error} onRecord={(fields) => model.record(fields)} />
      {:else}<p class="mt-2 text-sm text-warning">No observation recorded. This remains not run; finishing or cancelling the attempt does not make it passed.</p>{/if}
    </section>
  {/each}</div>
  {#if model.error}<p class="mt-3 text-sm text-danger" role="alert">{model.error}</p>{/if}
  <div class="mt-4 flex flex-wrap gap-2">{#if !terminal}{#if review.run.method === 'owner_manual' && review.run.lifecycle === 'running'}<Button variant="secondary" disabled={disabled || model.saving} onclick={() => void model.finish('completed')}>Finish observation attempt</Button><Button variant="ghost" disabled={disabled || model.saving} onclick={() => void model.finish('cancelled')}>Cancel attempt</Button>{:else}<p class="text-sm text-text-secondary">The authorized runner owns this attempt. Its recorded lifecycle and observations update through the runner.</p>{/if}{:else}<Button variant="primary" disabled={disabled || model.saving} onclick={() => (decisionOpen = true)}>Record owner decision</Button>{#if review.run.method === 'owner_manual'}<Button variant="secondary" disabled={disabled || model.saving} onclick={() => void model.start(review.run.manifest.scenarios.map((scenario) => ({ scenario_id: scenario.id, revision: scenario.revision })), review.run.id)}>Start separate retry</Button>{/if}{/if}</div>
  {#if review.decisions.length}<div class="mt-4 border-t border-border-subtle pt-4"><h3 class="text-sm font-medium text-text-primary">Owner decisions for this attempt</h3><ul class="mt-2 space-y-2">{#each review.decisions as decision (decision.id)}<li class="rounded-md border border-border-subtle p-3"><Badge tone="neutral">{decision.decision.replaceAll('_', ' ')}</Badge><p class="mt-2 whitespace-pre-wrap text-sm text-text-secondary">{decision.reason}</p><p class="mt-1 text-xs text-text-muted">{formatAbsoluteTime(decision.recorded_at)} · evidence freshness at decision: {decision.evidence_freshness} · now: {decision.current_freshness}. Original results remain above.</p></li>{/each}</ul></div>{/if}
</Card>
<div class="mt-4"><OwnerRunMap {review} readOnly={disabled} /></div>
<OwnerDecisionDialog bind:open={decisionOpen} {review} saving={model.saving} error={model.error} onRecord={(fields) => model.decide(fields)} />
