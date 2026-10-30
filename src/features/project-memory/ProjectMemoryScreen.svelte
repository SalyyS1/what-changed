<script lang="ts">
  import { untrack } from 'svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Select from '../../components/ui/Select.svelte';
  import type { OwnerCapabilityDto, OwnerChangeDraftDto, ProjectDecisionRevisionDto } from '../../lib/tauri/bindings';
  import { ProjectMemoryModel, latestDecisions, memoryProvenance } from './project-memory-model.svelte';
  import MemoryDecisionEditor from './MemoryDecisionEditor.svelte';
  import BundleExportReview from './BundleExportReview.svelte';
  interface Props { appId: string; capabilities: OwnerCapabilityDto[]; changes: OwnerChangeDraftDto[]; active?: boolean; readOnly?: boolean; model?: ProjectMemoryModel; }
  let { appId, capabilities, changes, active = true, readOnly = false, model = new ProjectMemoryModel() }: Props = $props();
  let editing = $state(false); let previous = $state<ProjectDecisionRevisionDto | null>(null); let capabilityFilter = $state('all'); let changeFilter = $state('all'); let sourceFilter = $state('all');
  $effect(() => { const binding = { appId, active }; untrack(() => { editing = false; previous = null; capabilityFilter = changeFilter = sourceFilter = 'all'; if (binding.active) model.bind(binding.appId); else model.cancel(); }); return () => model.cancel(); });
  const current = $derived(active && model.appId === appId);
  const latest = $derived(latestDecisions(model.revisions));
  const visible = $derived(latest.filter((item) => (capabilityFilter === 'all' || item.capability_id === capabilityFilter) && (changeFilter === 'all' || item.change_id === changeFilter) && (sourceFilter === 'all' || item.provenance === sourceFilter)));
  const capabilityOptions = $derived([{ value: 'all', label: 'All capabilities' }, ...capabilities.map((item) => ({ value: item.id, label: item.label }))]);
  const changeOptions = $derived([{ value: 'all', label: 'All change requests' }, ...changes.map((item) => ({ value: item.id, label: item.intent || 'Untitled saved request' }))]);
  const sourceOptions = [{ value: 'all', label: 'All wording sources' }, ...Object.entries(memoryProvenance).map(([value, label]) => ({ value, label }))];
  function closeEditor(): void { editing = false; previous = null; }
</script>

<section aria-labelledby="project-memory-heading" class="space-y-4">
  <Card>
    <div class="flex flex-wrap items-start justify-between gap-3"><div><h2 id="project-memory-heading" class="font-medium text-text-primary">Project memory</h2><p class="mt-1 text-sm text-text-secondary">Record what was decided and why. Decisions keep their authors and sources; they do not change acceptance or test evidence.</p></div><div class="flex gap-2"><Button size="sm" variant="ghost" disabled={!current || model.loading || model.saving} onclick={() => void model.load()}>Reload history</Button><Button size="sm" variant="secondary" disabled={!current || readOnly || model.loading || model.saving} onclick={() => { previous = null; editing = true; }}>Add decision</Button></div></div>
    {#if model.error}<p class="mt-3 text-sm text-danger" role="alert">{model.error}</p>{/if}
    {#if model.loading}<p class="mt-3 text-sm text-text-muted" role="status">Loading every saved decision revision…</p>{/if}
    <div class="mt-4 grid gap-3 @[760px]/shell:grid-cols-3"><Select label="Filter decisions by capability" value={capabilityFilter} options={capabilityOptions} onchange={(value) => (capabilityFilter = value)} /><Select label="Filter decisions by request" value={changeFilter} options={changeOptions} onchange={(value) => (changeFilter = value)} /><Select label="Filter decisions by source" value={sourceFilter} options={sourceOptions} onchange={(value) => (sourceFilter = value)} /></div>
    <div class="mt-4 space-y-3">
      {#each visible as decision (decision.id)}
        <article class="space-y-2 rounded-md border border-border-subtle p-3"><div class="flex flex-wrap gap-2"><Badge tone="neutral">Revision {decision.revision}</Badge><Badge tone="neutral">{memoryProvenance[decision.provenance]}</Badge></div><p class="whitespace-pre-wrap break-words text-sm text-text-primary">{decision.text}</p><p class="whitespace-pre-wrap break-words text-sm text-text-secondary">Why: {decision.rationale}</p><p class="text-xs text-text-muted">Recorded by {decision.recorded_by} · {new Date(decision.recorded_at).toLocaleString()}{decision.supersedes ? ` · replaces revision ${decision.supersedes}` : ''}</p><p class="break-words text-xs text-text-muted">Capability: {capabilities.find((item) => item.id === decision.capability_id)?.label ?? (decision.capability_id ? decision.capability_id : 'Whole app')} · Request: {changes.find((item) => item.id === decision.change_id)?.intent ?? (decision.change_id ? decision.change_id : 'No particular request')}</p>
          <Button size="sm" variant="ghost" disabled={!current || readOnly || model.loading || model.saving} onclick={() => { previous = decision; editing = true; }}>Revise decision</Button>
          <details class="text-sm text-text-secondary"><summary class="cursor-pointer">Immutable decision history</summary><ol class="mt-2 space-y-3">{#each model.revisions.filter((item) => item.id === decision.id) as revision (revision.revision)}<li class="whitespace-pre-wrap break-words"><span class="font-medium">Revision {revision.revision} · {memoryProvenance[revision.provenance]} · {revision.recorded_by}</span><p>{revision.text}</p><p>Why: {revision.rationale}</p><p class="text-xs text-text-muted">{new Date(revision.recorded_at).toLocaleString()}{revision.supersedes ? ` · replaces revision ${revision.supersedes}` : ''}</p></li>{/each}</ol></details>
        </article>
      {:else}{#if !model.loading}<p class="text-sm text-text-muted">{latest.length ? 'No decisions match these filters.' : 'No saved project decisions yet.'}</p>{/if}{/each}
    </div>
    {#if editing && current}{#key `${appId}:${previous?.id ?? 'new'}:${previous?.revision ?? 0}`}<MemoryDecisionEditor {previous} {capabilities} {changes} disabled={readOnly} saving={model.saving} onSave={(draft) => model.save(draft, previous ?? undefined)} onCancel={closeEditor} />{/key}{/if}
  </Card>
  <BundleExportReview {appId} active={current} />
  <Card><details class="text-sm text-text-secondary"><summary class="cursor-pointer font-medium text-text-primary">Imported archival history</summary><p class="mt-3">Imported observations and scenarios are untrusted historical records. This view never presents them as a current pass or restores execution permissions.</p><Button class="mt-3" size="sm" variant="secondary" disabled={!current || model.historyLoading} onclick={() => void model.loadHistory()}>{model.historyLoading ? 'Loading archive…' : model.historyLoaded ? 'Reload imported archive' : 'Load imported archive'}</Button>{#if model.historyError}<p class="mt-3 text-danger" role="alert">{model.historyError}</p>{/if}{#if current && model.historyLoaded}{#if model.history === null}<p class="mt-3 text-text-muted">No imported archive for this app.</p>{:else}<pre aria-label="Untrusted imported archive" class="selectable mt-3 max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border-subtle bg-surface-2 p-3 font-mono text-xs">{JSON.stringify(model.history, null, 2)}</pre>{/if}{/if}</details></Card>
</section>
