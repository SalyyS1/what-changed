<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import type { ChangeRequestStore } from '../../lib/stores/change-request.svelte';
  interface Props { model: ChangeRequestStore; }
  let { model }: Props = $props();
</script>

<section aria-labelledby="brief-title" class="space-y-4">
  <div><h2 id="brief-title" class="font-medium text-text-primary">Review your change brief</h2><p class="mt-1 text-sm text-text-secondary">Check your wording and scope. Saving this review does not verify the change or send anything to a coding AI.</p></div>
  <dl class="space-y-4"><div><dt class="text-xs font-medium text-text-muted">DESIRED OUTCOME</dt><dd class="mt-1 whitespace-pre-wrap break-words text-sm text-text-primary">{model.intent || 'Not described yet.'}</dd></div><div><dt class="text-xs font-medium text-text-muted">EXAMPLE</dt><dd class="mt-1 whitespace-pre-wrap break-words text-sm text-text-primary">{model.example || 'Add an example before marking ready.'}</dd></div><div><dt class="text-xs font-medium text-text-muted">SCOPE AND OPEN CHOICES</dt><dd class="mt-1 whitespace-pre-wrap break-words text-sm text-text-primary">{model.scope || 'No scope limits or unresolved choices recorded.'}</dd></div></dl>
  <div><h3 class="text-sm font-medium text-text-primary">Preserved rules</h3><ul class="mt-2 space-y-2">{#each model.preservedRules as ref (`${ref.rule_id}:${ref.revision}`)}{@const rule = model.context?.rules.find((item) => item.rule_id === ref.rule_id && item.revision === ref.revision)}<li class="rounded-md border border-border-subtle p-3"><p class="whitespace-pre-wrap break-words text-sm text-text-secondary">{rule?.expected_behavior ?? 'This exact rule revision could not be read. Review the app before sharing the brief.'}</p><div class="mt-2 flex flex-wrap gap-2"><Badge tone="neutral">Revision {ref.revision}</Badge>{#if rule}<Badge tone="neutral">{rule.provenance.replaceAll('_', ' ')}</Badge>{/if}</div></li>{:else}<li class="text-sm text-text-muted">No preserved rules selected. This does not imply the app has no existing behavior.</li>{/each}</ul></div>
  <p class="rounded-md bg-surface-2 p-3 text-sm text-text-secondary">Keep credentials, customer data and private URLs out of the brief. The separate export review will show exactly what leaves this app.</p>
</section>
