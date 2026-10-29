<script lang="ts">
  import { untrack } from 'svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import { BundleExportModel } from './bundle-export-model.svelte';
  interface Props { appId: string; active?: boolean; model?: BundleExportModel; }
  let { appId, active = true, model = new BundleExportModel() }: Props = $props();
  $effect(() => { const binding = { appId, active }; untrack(() => { if (binding.active) model.bind(binding.appId); else model.cancel(); }); return () => model.cancel(); });
  const current = $derived(active && model.appId === appId);
</script>

<Card>
  <h3 class="font-medium text-text-primary">Export app memory</h3>
  <p class="mt-1 text-sm text-text-secondary">Prepare a local bundle with readable Markdown, supported scenario recipes and immutable history. Review its content before choosing a file.</p>
  <label class="mt-4 flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" checked={model.includeTargetDetails} disabled={!current || model.saving} onchange={(event) => model.setTargetDetails(event.currentTarget.checked)} />Include selected preview target details. These may reveal private hosts; execution permissions and credentials are excluded.</label>
  <Button class="mt-3" variant="secondary" disabled={!current || model.loading || model.saving} onclick={() => void model.prepare()}>{model.loading ? 'Preparing bundle…' : 'Prepare bundle preview'}</Button>
  {#if model.error}<p class="mt-3 text-sm text-danger" role="alert">{model.error}</p>{/if}
  {#if current && model.preview}
    <div class="mt-4 space-y-3 border-t border-border-subtle pt-4">
      <Badge tone="neutral">{model.preview.bytes.toLocaleString()} bytes</Badge>
      <p class="break-all font-mono text-xs text-text-muted">Preview digest: {model.preview.digest}</p>
      <div class="text-sm"><h4 class="font-medium text-text-primary">Omitted from this bundle</h4><ul class="mt-1 list-inside list-disc text-text-secondary">{#each model.preview.omissions as item (item)}<li>{item}</li>{:else}<li>No additional omissions reported.</li>{/each}</ul></div>
      <div class="text-sm"><h4 class="font-medium text-text-primary">Bundle limitations</h4><ul class="mt-1 list-inside list-disc text-text-secondary">{#each model.preview.limitations as item (item)}<li>{item}</li>{/each}</ul></div>
      <details class="text-sm text-text-secondary"><summary class="cursor-pointer">Included rule revisions ({model.preview.rules.length})</summary><ul class="mt-2 space-y-2">{#each model.preview.rules as rule (`${rule.rule_id}:${rule.revision}`)}<li class="whitespace-pre-wrap break-words">Revision {rule.revision} · {rule.expected_behavior}</li>{:else}<li>No confirmed rule revisions.</li>{/each}</ul></details>
      <pre aria-label="Exact bundle Markdown" class="selectable max-h-96 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border-subtle bg-surface-2 p-3 font-mono text-xs text-text-primary">{model.preview.markdown}</pre>
      <label class="flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" bind:checked={model.reviewed} disabled={model.saving} />I reviewed this bundle, its included rules, omissions and limitations.</label>
      <Button variant="primary" disabled={!model.canSave} onclick={() => void model.save()}>{model.saving ? 'Saving bundle…' : 'Save reviewed bundle'}</Button>
      <p class="text-xs text-text-muted">Choose a filename. Replacing an existing file requires a separate confirmation.</p>
      {#if model.savedPath}<p class="break-all text-sm text-text-secondary" role="status">Bundle saved to {model.savedPath}</p>{/if}
    </div>
  {/if}
</Card>
