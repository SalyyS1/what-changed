<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { projectOwnerGraph } from '../../lib/owner-graph/projection';
  import { filterOwnerNodes, ownerGraphWindow, reconcileSelection, MAX_OWNER_NODES, type OwnerGraphFilter } from '../../lib/owner-graph/selection';
  import type { OwnerGraphInput, OwnerMapMode, OwnerLayer } from '../../lib/owner-graph/types';
  import ChangeJourney from './ChangeJourney.svelte';
  import MapModeControl from './MapModeControl.svelte';
  import CapabilityMap2D from './CapabilityMap2D.svelte';
  import OwnerMapNodeList from './OwnerMapNodeList.svelte';
  import EvidenceInspector from './EvidenceInspector.svelte';
  import BeforeAfterReview from './BeforeAfterReview.svelte';
  interface Props { input: OwnerGraphInput; readOnly?: boolean; showJourney?: boolean; preferredRunId?: string; }
  let { input, readOnly = false, showJourney = true, preferredRunId }: Props = $props();
  type Map3DModule = typeof import('./CapabilityMap3D.svelte');
  let mode = $state<OwnerMapMode>('2d');
  let forcedColors = $state(false);
  let fallbackReason = $state<string | null>(null);
  let lazy = $state<Promise<Map3DModule | null> | null>(null);
  let selectedId = $state<string | null>(null);
  let runId = $state('');
  let capability = $state('');
  let filter = $state<OwnerGraphFilter>('all');
  let page = $state(0);
  let separation = $state(6);
  let lowQuality = $state(false);
  let layers = $state<OwnerLayer[]>(['intent', 'scenarios', 'evidence']);
  let action = $state<{ id: number; kind: 'reset' | 'fit' | 'in' | 'out' }>({ id: 0, kind: 'reset' });
  let resetButton: HTMLButtonElement;
  let inspector: HTMLDivElement;
  const fullGraph = $derived(projectOwnerGraph(input));
  const review = $derived(fullGraph.reviews.find((row) => row.run.id === runId) ?? fullGraph.reviews[0]);
  const graph = $derived(projectOwnerGraph({ ...input, reviews: review ? [review] : [] }));
  const matching = $derived(filterOwnerNodes(graph, filter, capability || null).filter((node) => layers.includes(node.layer)));
  const windowed = $derived(ownerGraphWindow(graph, matching, page));
  const selected = $derived(graph.nodes.find((node) => node.id === selectedId) ?? null);
  const effectiveMode = $derived(forcedColors ? 'list' : mode);
  const storageKey = 'owner-map-mode';
  $effect(() => { if (preferredRunId) runId = preferredRunId; });
  function remember(value: OwnerMapMode): void { try { localStorage.setItem(storageKey, value); } catch { /* Preference storage is optional. */ } }
  function fallback(reason: string): void { fallbackReason = reason; mode = '2d'; remember('2d'); lazy = null; }
  function chooseMode(value: OwnerMapMode): void {
    fallbackReason = null;
    if (value === '3d') {
      try {
        const canvas = document.createElement('canvas'); const context = canvas.getContext('webgl2');
        if (!context) { fallback('WebGL 2 is unavailable. The complete 2D and list views remain available.'); return; }
        context.getExtension('WEBGL_lose_context')?.loseContext();
      } catch { fallback('3D capability could not be checked. The complete 2D and list views remain available.'); return; }
    }
    mode = value; remember(value);
  }
  onMount(() => {
    const media = globalThis.matchMedia?.('(forced-colors: active)');
    const update = () => { forcedColors = media?.matches ?? false; };
    update(); media?.addEventListener('change', update);
    try { const stored = localStorage.getItem(storageKey); if (stored === 'list' || stored === '3d') chooseMode(stored); } catch { /* Use the primary 2D view. */ }
    return () => media?.removeEventListener('change', update);
  });
  $effect(() => { if (effectiveMode === '3d' && !lazy) lazy = import('./CapabilityMap3D.svelte').catch(() => { fallback('The 3D view could not load. The complete 2D and list views remain available.'); return null; }); });
  $effect(() => { selectedId = reconcileSelection(matching, selectedId); page = windowed.page; });
  function select(id: string): void {
    selectedId = id;
    const index = matching.findIndex((node) => node.id === id); if (index >= 0) page = Math.floor(index / MAX_OWNER_NODES);
  }
  function command(kind: typeof action.kind): void { action = { id: action.id + 1, kind }; }
  function toggleLayer(layer: OwnerLayer): void { layers = layers.includes(layer) ? layers.filter((item) => item !== layer) : [...layers, layer]; page = 0; }
  async function inspect(run: string | null, id: string): Promise<void> {
    if (run) runId = run; filter = 'all'; capability = ''; layers = ['intent', 'scenarios', 'evidence'];
    await tick(); select(id); await tick(); inspector?.focus();
  }
</script>

<section class="min-w-0 space-y-4 rounded-md border border-border-subtle bg-surface-1 p-4" aria-label="Owner evidence workbench">
  <div><h2 class="font-medium text-text-primary">Capability and evidence workbench</h2><p class="mt-1 text-sm text-text-secondary">Explore confirmed intent and recorded observations. Placement expresses no dependency or verdict.{readOnly ? ' Archived workspace · read only.' : ''}</p></div>
  {#if showJourney}<ChangeJourney graph={fullGraph} oninspect={(id, run) => void inspect(run, id)} />{/if}
  <div class="flex flex-wrap items-center justify-between gap-3"><MapModeControl mode={effectiveMode} onmode={chooseMode} threeDisabled={forcedColors} /><div class="flex flex-wrap gap-2"><button bind:this={resetButton} type="button" class="min-h-11 min-w-11 rounded-md border border-border-subtle px-3 text-sm text-text-secondary hover:bg-surface-2" onclick={() => command('reset')}>Reset view</button><button type="button" class="min-h-11 min-w-11 rounded-md border border-border-subtle px-3 text-sm text-text-secondary hover:bg-surface-2" onclick={() => command('fit')}>Fit selected</button><button type="button" class="min-h-11 min-w-11 rounded-md border border-border-subtle px-3 text-sm text-text-secondary hover:bg-surface-2" aria-label="Zoom in" onclick={() => command('in')}>+</button><button type="button" class="min-h-11 min-w-11 rounded-md border border-border-subtle px-3 text-sm text-text-secondary hover:bg-surface-2" aria-label="Zoom out" onclick={() => command('out')}>−</button></div></div>
  {#if fallbackReason}<p class="rounded-md border border-border-subtle p-3 text-sm text-warning" role="status">{fallbackReason}</p>{/if}
  {#if forcedColors}<p class="text-sm text-text-secondary">Forced colors is active. The semantic list exposes all evidence and controls.</p>{/if}
  <div class="grid gap-3 sm:grid-cols-3">
    <label class="min-w-0 text-xs text-text-secondary">Evidence attempt<select class="mt-1 min-h-11 w-full rounded-md border border-border-subtle bg-surface-1 p-2 text-sm text-text-primary" value={review?.run.id ?? ''} onchange={(event) => { runId = event.currentTarget.value; page = 0; }}>{#if !fullGraph.reviews.length}<option value="">No reviewed attempt recorded</option>{/if}{#each fullGraph.reviews as row (row.run.id)}<option value={row.run.id}>{row.run.manifest.candidate.identity} · {row.run.id}</option>{/each}</select></label>
    <label class="min-w-0 text-xs text-text-secondary">Isolate capability<select bind:value={capability} onchange={() => { page = 0; }} class="mt-1 min-h-11 w-full rounded-md border border-border-subtle bg-surface-1 p-2 text-sm text-text-primary"><option value="">All capabilities</option>{#each input.capabilities as item (item.id)}<option value={item.id}>{item.label}</option>{/each}</select></label>
    <label class="text-xs text-text-secondary">Evidence filter<select bind:value={filter} onchange={() => { page = 0; }} class="mt-1 min-h-11 w-full rounded-md border border-border-subtle bg-surface-1 p-2 text-sm text-text-primary"><option value="all">All records</option><option value="failed">Failed results</option><option value="gaps">Unknown / blocked / gaps</option><option value="stale">Stale evidence</option></select></label>
  </div>
  {#if review}<p class="rounded-md bg-surface-2 p-3 text-sm text-text-secondary" aria-label="Authoritative evidence totals">This attempt: {review.counts.passed} passed · {review.counts.failed} failed · {review.counts.blocked} blocked · {review.counts.not_run} not run. Backend result: {review.status.replaceAll('_', ' ')}. Freshness: {review.freshness}. Candidate: {review.run.manifest.candidate.identity}. Owner decisions: {review.decisions.length}. These totals stay complete when nodes are filtered or grouped.</p>{:else}<p class="text-sm text-text-secondary">No reviewed attempt recorded. Coverage and observed behavior remain unknown.</p>{/if}
  <fieldset class="flex flex-wrap gap-3"><legend class="text-xs text-text-secondary">Visible layers</legend>{#each ['intent', 'scenarios', 'evidence'] as layer}<label class="flex min-h-11 items-center gap-2 text-xs text-text-secondary"><input type="checkbox" checked={layers.includes(layer as OwnerLayer)} onchange={() => toggleLayer(layer as OwnerLayer)} />{layer === 'intent' ? 'Owner intent' : layer === 'scenarios' ? 'Scenarios' : 'Observed results'}</label>{/each}</fieldset>
  {#if effectiveMode === '3d'}<div class="flex flex-wrap items-center gap-3"><label class="flex min-h-11 items-center gap-2 text-xs text-text-secondary">Layer separation<input type="range" min="2" max="14" step="1" bind:value={separation} /></label><label class="flex min-h-11 items-center gap-2 text-xs text-text-secondary"><input type="checkbox" bind:checked={lowQuality} />Low quality · lower pixel density</label></div>{/if}
  {#if windowed.reason}<p class="text-xs text-warning" role="status">{windowed.reason} Showing {windowed.nodes.length} of {matching.length} matching nodes; {windowed.edges.length} visible links of {graph.edges.length} recorded links.</p>{/if}
  {#if effectiveMode === '2d'}<CapabilityMap2D graph={windowed} {selectedId} onselect={select} {action} />
  {:else if effectiveMode === '3d' && lazy}{#await lazy}<p class="p-4 text-sm text-text-secondary" role="status">Loading the 3D workbench… The complete node list remains available.</p>{:then module}{#if module}{#key lowQuality}<module.default graph={windowed} {selectedId} onselect={select} onunavailable={fallback} {separation} {lowQuality} {action} />{/key}{/if}{/await}{/if}
  {#if effectiveMode !== 'list' && windowed.pages > 1}<div class="flex flex-wrap items-center gap-2"><button type="button" class="min-h-11 min-w-11 rounded-md border border-border-subtle px-3 text-sm text-text-secondary" disabled={page <= 0} onclick={() => { page -= 1; }}>Previous group</button><button type="button" class="min-h-11 min-w-11 rounded-md border border-border-subtle px-3 text-sm text-text-secondary" disabled={page >= windowed.pages - 1} onclick={() => { page += 1; }}>Show next group</button><span class="text-xs text-text-secondary">Group {windowed.page + 1} of {windowed.pages}</span></div>{/if}
  <p class="text-xs text-text-secondary">Legend: text names every state. Green is reserved for a current passed scenario; failure, blocked and stale keep their labels. A dashed card marks explicit selected-request linkage. Owner decisions remain separate.</p>
  <div class="grid min-w-0 gap-4 xl:grid-cols-2"><OwnerMapNodeList nodes={matching} {selectedId} onselect={select} onescape={() => resetButton?.focus()} /><div bind:this={inspector} tabindex="-1"><EvidenceInspector node={selected} /></div></div>
  {#if graph.gaps.length}<details class="text-xs text-text-secondary"><summary class="min-h-11 cursor-pointer py-3">Coverage and relationship gaps ({graph.gaps.length})</summary><ul class="list-inside list-disc space-y-1 text-warning">{#each graph.gaps as gap}<li class="break-words">{gap}</li>{/each}</ul></details>{/if}
  <BeforeAfterReview reviews={fullGraph.reviews} oninspect={(run, id) => void inspect(run, id)} />
</section>
