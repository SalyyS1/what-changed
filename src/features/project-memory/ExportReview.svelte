<script lang="ts">
  import { untrack } from 'svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import { OwnerBriefStore } from '../../lib/stores/owner-brief.svelte';
  import type { OwnerBriefSelectionInput } from '../../lib/tauri/bindings';

  interface Props { appId: string; changeId: string; version: number; dirty?: boolean; model?: OwnerBriefStore; onClose?: () => void; }
  let { appId, changeId, version, dirty = false, model = new OwnerBriefStore(), onClose }: Props = $props();
  $effect(() => {
    const binding = { appId, changeId, version, dirty };
    untrack(() => { if (binding.dirty) model.cancel(); else model.bind(binding.appId, binding.changeId, binding.version); });
    return () => model.cancel();
  });
  function select(field: keyof Omit<OwnerBriefSelectionInput, 'scenario_ids'>, checked: boolean): void {
    model.setSelection({ ...model.selection, [field]: checked });
  }
  const current = $derived(!dirty && model.appId === appId && model.changeId === changeId && model.version === version);
</script>

<section aria-labelledby="export-review-title" class="max-w-3xl space-y-4">
  <Card>
    <div class="flex flex-wrap items-start justify-between gap-3"><div><h2 id="export-review-title" class="font-medium text-text-primary">Review what you will share</h2><p class="mt-1 text-sm text-text-secondary">This preview is prepared locally. Copying puts this exact text on your clipboard for you to paste into your existing coding AI.</p></div>{#if onClose}<Button size="sm" variant="ghost" onclick={onClose}>Close review</Button>{/if}</div>
    <p class="mt-3 text-sm text-text-secondary">Desired outcome, example, scope, preserved rule revisions and the evidence boundary are included. Candidate labels and preview origins are omitted unless you choose them.</p>
    <fieldset class="mt-4 grid gap-3 text-sm text-text-secondary" disabled={!current || model.copying}><legend class="mb-2 font-medium text-text-primary">Optional sections</legend>
      <label class="flex items-start gap-2"><input type="checkbox" checked={model.selection.include_scenarios} onchange={(event) => select('include_scenarios', event.currentTarget.checked)} />Include confirmed acceptance scenarios</label>
      <label class="flex items-start gap-2"><input type="checkbox" checked={model.selection.include_candidate_identity} onchange={(event) => select('include_candidate_identity', event.currentTarget.checked)} />Include candidate version label</label>
      <label class="flex items-start gap-2"><input type="checkbox" checked={model.selection.include_environment_details} onchange={(event) => select('include_environment_details', event.currentTarget.checked)} />Include selected preview origins (may reveal a private host)</label>
      <label class="flex items-start gap-2"><input type="checkbox" checked={model.selection.include_limitations} onchange={(event) => select('include_limitations', event.currentTarget.checked)} />Include unresolved tests and limitations</label>
    </fieldset>
    <Button class="mt-4" variant="secondary" disabled={!current || model.loading || model.copying} onclick={() => void model.load()}>{model.loading ? 'Preparing preview…' : model.preview ? 'Refresh preview' : 'Prepare preview'}</Button>
  </Card>
  {#if dirty}<p class="text-sm text-warning" role="status">Save the request before preparing or copying its brief. The previous preview has been cleared.</p>{/if}
  {#if model.error}<p class="rounded-md border border-danger/30 bg-danger/8 p-3 text-sm text-danger" role="alert">{model.error}</p>{/if}
  {#if model.loading}<p class="text-sm text-text-muted" role="status">Preparing the selected sections locally…</p>{/if}
  {#if current && model.preview}
    <Card>
      <div class="flex flex-wrap gap-2"><Badge tone="neutral">Saved request version {version}</Badge><Badge tone="neutral">{model.preview.bytes.toLocaleString()} bytes</Badge><Badge tone="neutral">About {model.preview.estimated_tokens.toLocaleString()} tokens</Badge></div>
      <div class="mt-4 space-y-3 text-sm"><div><h3 class="font-medium text-text-primary">Omitted sections</h3><ul class="mt-1 list-inside list-disc text-text-secondary">{#each model.preview.omitted_sections as item (item)}<li>{item}</li>{:else}<li>No optional sections omitted.</li>{/each}</ul></div><div><h3 class="font-medium text-text-primary">Limits to keep in mind</h3><ul class="mt-1 list-inside list-disc text-text-secondary"><li>A brief is a request; no run or passed check is implied.</li>{#each model.preview.limitations as item (item)}<li>{item}</li>{/each}</ul></div></div>
      <pre aria-label="Exact brief text" class="selectable mt-4 max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border-subtle bg-surface-2 p-3 font-mono text-xs text-text-primary">{model.preview.markdown}</pre>
      <label class="mt-4 flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" bind:checked={model.reviewed} disabled={model.copying} />I reviewed this text and the included and omitted sections. It is ready to copy.</label>
      <div class="mt-3 flex flex-wrap items-center gap-3"><Button variant="primary" disabled={!current || !model.canCopy} onclick={() => void model.copy()}>{model.copying ? 'Copying…' : 'Copy reviewed brief'}</Button>{#if model.copied}<p class="text-sm text-text-secondary" role="status">Copied. Paste it into your coding AI when ready.</p>{/if}</div>
    </Card>
  {:else if !model.loading && !dirty}<p class="text-sm text-text-muted">Choose the sections, then prepare a preview before sharing anything.</p>{/if}
</section>
