<script lang="ts">
  import { onMount } from 'svelte';
  import type { OwnerGraph } from '../../lib/owner-graph/types';
  interface Props { graph: OwnerGraph; oninspect?: (id: string, runId: string | null) => void; }
  let { graph, oninspect }: Props = $props();
  let selected = $state('intent');
  const instance = $props.id();
  const reviews = $derived(graph.reviews);
  const preserved = $derived(graph.change?.preserved_rules.length ?? 0);
  const decisions = $derived(reviews.reduce((count, review) => count + review.decisions.length, 0));
  const steps = $derived([
    { id: 'intent', label: 'Intent', count: graph.change ? '1 saved request' : 'No selected request', missing: !graph.change,
      details: graph.change ? `${graph.change.intent} · Request version ${graph.change.version}. Example: ${graph.change.example || 'Not recorded'}. Scope: ${graph.change.scope || 'Not recorded'}.` : 'Select or create a change request to record the intended outcome.' },
    { id: 'preserve', label: 'Preserve', count: `${preserved} linked rule revisions`, missing: !preserved,
      details: preserved ? 'Only the exact preserved rule revisions are highlighted. Select a highlighted rule in the map or list for its confirmed behavior and provenance.' : 'No preserved rule revisions are linked to this request. Review what must stay before handing off the change.' },
    { id: 'brief', label: 'Brief', count: 'Review not recorded', missing: true,
      details: 'Brief previews and clipboard copies are not durable review records. Open this request, review the Markdown and its omissions, then copy or export it to your builder.' },
    { id: 'verify', label: 'Verify', count: `${reviews.length} recorded attempts`, missing: !reviews.length,
      details: reviews.length ? `${reviews.reduce((count, review) => count + review.scenarios.length, 0)} frozen scenario entries across these attempts. Each result belongs to its named candidate, environment and scenario revision. A recorded attempt is not a passed check.` : 'Choose a candidate and confirmed scenarios, then record or authorize an attempt. Missing observations stay visible.' },
    { id: 'decide', label: 'Decide', count: `${decisions} owner decisions`, missing: !decisions,
      details: decisions ? reviews.flatMap((review) => review.decisions.map((decision) => `${decision.decision.replaceAll('_', ' ')}: ${decision.reason} · candidate ${decision.candidate_id} · ${decision.current_freshness}`)).join('\n') : 'Review a finished attempt before recording an explicit owner decision. Results and owner acceptance remain separate.' },
  ]);
  const current = $derived(steps.find((step) => step.id === selected) ?? steps[0]!);
  const artifacts = $derived(graph.nodes.filter((node) => selected === 'preserve' ? node.kind === 'rule' && node.selectedChange : selected === 'verify' ? node.kind === 'evidence' : selected === 'intent' ? node.kind === 'capability' && node.selectedChange : false));
  const storageKey = $derived(`owner-journey:${graph.appId}:${graph.change?.id ?? 'overview'}`);
  function restore(): void { try { const value = localStorage.getItem(storageKey); selected = steps.some((step) => step.id === value) ? value! : 'intent'; } catch { selected = 'intent'; } }
  onMount(restore);
  $effect(() => { storageKey; restore(); });
  function select(id: string): void { selected = id; try { localStorage.setItem(storageKey, id); } catch { /* Navigation remains usable when preference storage is unavailable. */ } }
</script>

<section aria-label="Change journey" class="space-y-3">
  <ol class="grid gap-2 sm:grid-cols-5">
    {#each steps as step, index (step.id)}
      <li class="min-w-0"><button type="button" class="h-full min-h-20 w-full rounded-md border p-3 text-left {step.missing ? 'border-dashed' : ''} {selected === step.id ? 'border-border-active bg-surface-3' : 'border-border-subtle hover:bg-surface-2'}" aria-current={selected === step.id ? 'step' : undefined} aria-controls={`${instance}-journey-details`} onclick={() => select(step.id)}><span class="block text-sm font-medium text-text-primary">{index + 1}. {step.label}</span><span class="mt-1 block text-xs text-text-secondary">{step.count}</span></button></li>
    {/each}
  </ol>
  <div id={`${instance}-journey-details`} class="rounded-md border border-border-subtle p-3" aria-live="polite"><h3 class="text-sm font-medium text-text-primary">{current.label}{current.missing ? ' · prerequisite or record missing' : ''}</h3><p class="mt-1 whitespace-pre-wrap break-words text-sm text-text-secondary">{current.details}</p>{#if artifacts.length}<ul class="mt-2 max-h-40 overflow-y-auto">{#each artifacts as artifact (artifact.id)}<li><button type="button" class="min-h-11 text-left text-xs text-text-secondary underline" onclick={() => oninspect?.(artifact.id, artifact.review?.run.id ?? null)}>{artifact.label}{artifact.revision !== null ? ` · r${artifact.revision}` : ''}{artifact.freshness ? ` · ${artifact.freshness}` : ''}</button></li>{/each}</ul>{/if}</div>
</section>
