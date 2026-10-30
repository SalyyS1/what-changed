<script lang="ts">
  import { untrack } from 'svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Input from '../../components/ui/Input.svelte';
  import type { OwnerImportResultDto } from '../../lib/tauri/bindings';
  import { BundleImportModel, ruleKey } from './bundle-import-model.svelte';
  import { memoryProvenance } from './project-memory-model.svelte';
  interface Props { active?: boolean; model?: BundleImportModel; onImported?: (result: OwnerImportResultDto) => void | Promise<void>; onOpenApp?: ((appId: string) => void) | undefined; }
  let { active = true, model = new BundleImportModel(), onImported, onOpenApp }: Props = $props();
  $effect(() => { if (!active) untrack(() => model.cancel()); return () => model.cancel(); });
  async function apply(): Promise<void> { const result = await model.apply(); if (result) await onImported?.(result); }
</script>

<Card>
  <h2 class="font-medium text-text-primary">Import app memory</h2>
  <p class="mt-1 text-sm text-text-secondary">Choose a bundle for local validation and review. Import creates a separate app with new IDs. It does not contact a preview or execute any imported scenario.</p>
  <Button class="mt-3" variant="secondary" disabled={!active || model.loading || model.applying} onclick={() => void model.choose()}>{model.loading ? 'Validating bundle…' : 'Choose bundle to review'}</Button>
  {#if model.error}<p class="mt-3 text-sm text-danger" role="alert">{model.error}</p>{/if}
  {#if active && model.preview && !model.result}
    <div class="mt-4 space-y-4 border-t border-border-subtle pt-4">
      <div class="flex flex-wrap gap-2"><Badge tone="neutral">Reviewed as a new app</Badge><Badge tone="neutral">Historical evidence remains untrusted</Badge></div>
      <p class="break-all text-xs text-text-muted">New app ID: {model.preview.proposed_app_id}</p><p class="break-all font-mono text-xs text-text-muted">Source digest: {model.preview.digest}</p>
      <fieldset class="space-y-3" disabled={model.applying}>
        <label class="block text-sm text-text-secondary">Name for the imported app<Input bind:value={model.name} maxlength={120} /></label>
        <label class="block text-sm text-text-secondary">Confirmed by<Input bind:value={model.confirmedBy} maxlength={120} placeholder="Your name or role" /></label>
        <div class="text-sm"><h3 class="font-medium text-text-primary">Conflict report</h3><ul class="mt-1 list-inside list-disc text-text-secondary">{#each model.preview.conflicts as item (item)}<li>{item}</li>{:else}<li>No conflicts reported. Existing apps will be preserved.</li>{/each}</ul></div>
        <div class="text-sm"><h3 class="font-medium text-text-primary">Import limitations</h3><ul class="mt-1 list-inside list-disc text-text-secondary"><li>Imported observations and scenarios are archival, untrusted history. They do not establish a current pass.</li><li>Local project links, active targets and execution permissions are not restored.</li>{#each model.preview.limitations as item (item)}<li>{item}</li>{/each}</ul></div>
        <pre aria-label="Imported bundle Markdown" class="selectable max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-md border border-border-subtle bg-surface-2 p-3 font-mono text-xs text-text-primary">{model.preview.markdown}</pre>
        <div class="space-y-3"><h3 class="text-sm font-medium text-text-primary">Confirm each rule revision, including its history</h3><p class="text-sm text-text-secondary">Each check confirms the imported wording you reviewed. It does not verify the behavior.</p>
          {#each model.preview.rules as rule (`${rule.rule_id}:${rule.revision}`)}
            <label class="flex items-start gap-3 rounded-md border border-border-subtle p-3 text-sm text-text-secondary"><input type="checkbox" checked={model.confirmedRules.includes(ruleKey(rule.rule_id, rule.revision))} onchange={(event) => model.confirmRule(ruleKey(rule.rule_id, rule.revision), event.currentTarget.checked)} /><span class="min-w-0"><span class="block font-medium text-text-primary">Confirm rule revision {rule.revision}</span><span class="mt-1 block whitespace-pre-wrap break-words">{rule.expected_behavior}</span><span class="mt-1 block break-all text-xs text-text-muted">{rule.rule_id} · {memoryProvenance[rule.provenance]} · originally confirmed by {rule.confirmed_by}{rule.supersedes ? ` · replaces revision ${rule.supersedes}` : ''}</span></span></label>
          {:else}<p class="text-sm text-text-muted">This bundle contains no confirmed rule revisions.</p>{/each}
        </div>
        <label class="flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" bind:checked={model.reviewed} />I reviewed the new app identity, conflicts and limitations. Imported history grants no current verification or execution permission.</label>
      </fieldset>
      <div class="flex flex-wrap gap-2"><Button variant="primary" disabled={!active || !model.canApply} onclick={() => void apply()}>{model.applying ? 'Importing reviewed bundle…' : 'Import reviewed bundle'}</Button><Button variant="ghost" disabled={model.applying} onclick={() => model.cancel()}>Discard review</Button></div>
    </div>
  {:else if active && model.result}
    <div class="mt-4 space-y-3"><p class="text-sm text-text-secondary" role="status">Imported {model.result.app.name} as a separate app. Its historical evidence remains untrusted.</p><ul class="list-inside list-disc text-sm text-text-secondary">{#each model.result.limitations as limitation (limitation)}<li>{limitation}</li>{/each}</ul>{#if onOpenApp}<Button variant="primary" onclick={() => { if (model.result) onOpenApp?.(model.result.app.id); }}>Open imported app</Button>{/if}</div>
  {/if}
</Card>
