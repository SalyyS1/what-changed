<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import type { ChangeRequestStore } from '../../lib/stores/change-request.svelte';
  interface Props { model: ChangeRequestStore; disabled?: boolean; }
  let { model, disabled = false }: Props = $props();
</script>

<section aria-labelledby="preserve-title">
  <h2 id="preserve-title" class="font-medium text-text-primary">What must stay working?</h2>
  <p class="mt-1 text-sm text-text-secondary">New requests reuse the latest confirmed rules by default. Choose explicitly; each request saves the exact revision.</p>
  {#if model.latestRules.length === 0}<p class="mt-4 rounded-md border border-dashed border-border-subtle p-4 text-sm text-text-muted">No confirmed rules yet. You can save the request, then return to the app overview to confirm what matters.</p>{/if}
  <fieldset class="mt-4 space-y-3" {disabled}><legend class="sr-only">Rules to preserve</legend>
    {#each model.latestRules as rule (rule.rule_id)}
      {@const selected = model.preservedRules.find((ref) => ref.rule_id === rule.rule_id)}
      <label class="flex cursor-pointer items-start gap-3 rounded-md border border-border-subtle p-3 hover:bg-surface-2"><input type="checkbox" class="mt-1" checked={selected !== undefined} onchange={(event) => model.toggleRule(rule, event.currentTarget.checked)} /><span class="min-w-0"><span class="block whitespace-pre-wrap break-words text-sm text-text-primary">{rule.expected_behavior}</span><span class="mt-2 flex flex-wrap gap-2"><Badge tone="neutral">Confirmed revision {rule.revision}</Badge><Badge tone="neutral">{rule.provenance.replaceAll('_', ' ')}</Badge>{#if selected && selected.revision !== rule.revision}<Badge tone="warning">Request uses older revision {selected.revision}</Badge>{/if}</span></span></label>
    {/each}
  </fieldset>
  {#if model.outdatedRules.length}<div class="mt-3 rounded-md border border-warning/30 bg-warning/8 p-3"><p class="text-sm text-warning" role="status">A preserved rule has changed since this draft was saved. Review its latest wording before marking the brief ready.</p><Button size="sm" class="mt-2" variant="secondary" {disabled} onclick={() => model.useLatestRules()}>Use latest confirmed revisions</Button></div>{/if}
  {#if model.unavailableRules.length}<p class="mt-3 text-sm text-warning" role="status">An exact preserved revision is unavailable. Reload the app's rules before marking this brief ready.</p>{/if}
</section>
