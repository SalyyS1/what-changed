<script lang="ts">
  import type { OwnerGraphNode } from '../../lib/owner-graph/types';
  import type { OwnerCapabilityDto, OwnerRuleRevisionDto, ScenarioRevisionDto, OwnerScenarioEvidenceDto } from '../../lib/tauri/bindings';
  interface Props { node: OwnerGraphNode | null; }
  let { node }: Props = $props();
</script>

<section aria-label="Selected evidence details" class="min-w-0 rounded-md border border-border-subtle p-4">
  {#if node}
    <h3 class="break-words text-sm font-medium text-text-primary">{node.label}</h3>
    <p class="mt-1 text-xs text-text-secondary">{node.kind}{node.revision !== null ? ` · revision ${node.revision}` : ''}{node.selectedChange ? ' · explicitly linked to selected request' : ''}</p>
    {#if node.kind === 'capability'}
      {@const capability = node.source as OwnerCapabilityDto}
      <p class="mt-3 whitespace-pre-wrap text-sm text-text-secondary">{capability.description || 'No description recorded.'}</p>
      <p class="mt-2 text-xs text-text-secondary">{capability.archived_at === null ? 'Active capability' : 'Archived capability · historical record'}</p>
    {:else if node.kind === 'rule'}
      {@const rule = node.source as OwnerRuleRevisionDto}
      <p class="mt-3 whitespace-pre-wrap text-sm text-text-primary">Expected: {rule.expected_behavior}</p>
      <p class="mt-2 break-words text-xs text-text-secondary">Confirmed by {rule.confirmed_by} · provenance: {rule.provenance} · recorded timestamp {rule.confirmed_at}. Supersedes: {rule.supersedes ?? 'none'}.</p>
    {:else}
      {@const evidence = node.kind === 'evidence' ? node.source as OwnerScenarioEvidenceDto : null}
      {@const scenario = evidence?.scenario ?? node.source as ScenarioRevisionDto}
      <p class="mt-3 whitespace-pre-wrap text-sm text-text-primary">Expected: {scenario.expected_behavior}</p>
      <p class="mt-2 text-xs text-text-secondary">Method: {scenario.method.replaceAll('_', ' ')} · {scenario.recipe.steps.length} ordered steps · {scenario.recipe.steps.reduce((count, step) => count + step.assertions.length, 0)} assertions.</p>
      <p class="mt-1 break-words text-xs text-text-secondary">Rules: {scenario.rules.map((ref) => `${ref.rule_id} r${ref.revision}`).join(', ') || 'none'}. Test roles: {scenario.recipe.role_refs.join(', ') || 'none'}. Fixtures: {scenario.recipe.fixture_refs.join(', ') || 'none'}.</p>
      {#if evidence && node.review}
        {@const review = node.review}
        <dl class="mt-3 space-y-2 text-xs text-text-secondary">
          <div><dt class="font-medium text-text-primary">Backend result / freshness</dt><dd>{evidence.status.replaceAll('_', ' ')} / {review.freshness}</dd></div>
          <div><dt class="font-medium text-text-primary">Candidate / trust</dt><dd class="break-words">{review.run.manifest.candidate.identity} / {review.run.manifest.candidate.trust.replaceAll('_', ' ')} · {review.run.manifest.candidate.id}</dd></div>
          <div><dt class="font-medium text-text-primary">Frozen environment</dt><dd class="break-words">{review.run.manifest.environment.label} · revision {review.run.manifest.environment.revision} · {review.run.manifest.environment.preview_url}</dd></div>
          <div><dt class="font-medium text-text-primary">Request / attempt</dt><dd class="break-words">{review.run.change_id} v{review.run.manifest.change.version} · {review.run.id} v{review.run.version} · {review.run.lifecycle} · {review.run.method.replaceAll('_', ' ')}</dd></div>
          <div><dt class="font-medium text-text-primary">Frozen change scope / allowed origins</dt><dd class="break-words">{review.run.manifest.change.scope || 'No scope recorded'} · {review.run.manifest.environment.origins.map((origin) => `${origin.scheme}://${origin.host}:${origin.port}`).join(', ') || 'No origins recorded'}</dd></div>
          <div><dt class="font-medium text-text-primary">Manifest / verification semantics</dt><dd class="break-all">{review.run.manifest_digest} · {review.run.manifest.semantics_version}</dd></div>
          {#if review.run.execution}<div><dt class="font-medium text-text-primary">Native runner observation</dt><dd class="break-all">{review.run.execution.identity} · revision {review.run.execution.revision ?? 'unknown'} · {review.run.execution.runner_semantics} · scope {review.run.execution.scope_digest}</dd></div>{/if}
        </dl>
        {#if evidence.result}
          <p class="mt-3 text-xs text-text-secondary">Observed by {evidence.result.observer} at timestamp {evidence.result.observed_at}.</p>
          <ul class="mt-2 space-y-2">{#each evidence.result.assertions as assertion (assertion.assertion_id)}<li class="rounded-md bg-surface-2 p-2 text-xs"><p class="font-medium text-text-primary">{assertion.status.replaceAll('_', ' ')} · {assertion.assertion_id}</p><p class="mt-1 whitespace-pre-wrap text-text-secondary">Expected: {assertion.expected}</p><p class="mt-1 whitespace-pre-wrap text-text-secondary">Observed: {assertion.observed || 'No observation supplied.'}</p>{#if assertion.limitation}<p class="mt-1 text-warning">{assertion.limitation}</p>{/if}</li>{/each}</ul>
          {#if evidence.result.limitation}<p class="mt-2 text-xs text-warning">{evidence.result.limitation}</p>{/if}
        {:else}<p class="mt-3 text-sm text-warning">No observation recorded.</p>{/if}
        {#if review.decisions.length}<h4 class="mt-3 text-xs font-medium text-text-primary">Separate owner decisions</h4><ul class="mt-1 space-y-1 text-xs text-text-secondary">{#each review.decisions as decision (decision.id)}<li class="whitespace-pre-wrap">{decision.decision.replaceAll('_', ' ')} · {decision.reason} · freshness now: {decision.current_freshness}</li>{/each}</ul>{/if}
        {#if review.decision_blockers.length}<h4 class="mt-3 text-xs font-medium text-text-primary">Decision prerequisites</h4><ul class="mt-1 list-inside list-disc text-xs text-warning">{#each review.decision_blockers as blocker}<li>{blocker}</li>{/each}</ul>{/if}
        <ul class="mt-2 list-inside list-disc text-xs text-warning">{#each review.limitations as limitation}<li>{limitation}</li>{/each}</ul>
      {/if}
      <details class="mt-3 text-xs text-text-secondary"><summary class="min-h-11 cursor-pointer py-3">Exact scenario recipe and recorded evidence</summary><pre class="max-h-80 overflow-auto whitespace-pre-wrap break-all rounded-md bg-surface-2 p-3">{JSON.stringify(node.source, null, 2)}</pre></details>
    {/if}
    {#if node.gaps.length}<ul class="mt-3 list-inside list-disc space-y-1 text-xs text-warning">{#each node.gaps as gap}<li class="break-words">{gap}</li>{/each}</ul>{/if}
    <p class="mt-3 break-all text-xs text-text-muted">Record: {node.id}</p>
  {:else}<p class="text-sm text-text-secondary">Select a capability, rule, scenario or result to inspect its exact recorded evidence.</p>{/if}
</section>
