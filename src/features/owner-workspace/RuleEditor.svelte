<script lang="ts">
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Select from '../../components/ui/Select.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import type { OwnerCapabilityDto, OwnerRuleRevisionDto } from '../../lib/tauri/bindings';
  import { latestOwnerRules, type OwnerWorkspaceStore } from '../../lib/stores/owner-workspace.svelte';

  interface Props { capabilities: OwnerCapabilityDto[]; rules: OwnerRuleRevisionDto[]; model: OwnerWorkspaceStore; active?: boolean; }
  let { capabilities, rules, model, active = true }: Props = $props();
  let capabilityId = $state('');
  let behavior = $state('');
  let editingId = $state<string | null>(null);
  let message = $state<string | null>(null);
  const latest = $derived(latestOwnerRules(rules));
  const previous = $derived(latest.find((rule) => rule.rule_id === editingId));
  const options = $derived(capabilities.map((item) => ({ value: item.id, label: item.label })));
  const provenance = { owner: 'Owner wording', template: 'Template', imported_untrusted: 'Imported, untrusted', external_ai: 'External AI suggestion' };

  function edit(rule: OwnerRuleRevisionDto): void { editingId = rule.rule_id; capabilityId = rule.capability_id; behavior = rule.expected_behavior; message = null; }
  async function confirm(): Promise<void> {
    if (!capabilityId || !behavior.trim()) { message = 'Choose a capability and describe the behavior to preserve.'; return; }
    if (await model.confirmRule(capabilityId, behavior.trim(), previous)) { editingId = null; behavior = ''; message = 'Confirmed locally. This records your intent; it is not a test result.'; }
    else message = model.error ?? 'The rule could not be confirmed. Your wording is still here.';
  }
</script>

<Card level={1}>
  <h2 class="font-medium text-text-primary">Confirmed rules</h2>
  <p class="mt-1 text-sm text-text-secondary">Say what must stay working. Each confirmation creates an immutable revision.</p>
  <div class="mt-4 space-y-3">
    {#if latest.length === 0}<p class="text-sm text-text-muted">No confirmed rules. Add a capability, then confirm one expected behavior.</p>{/if}
    {#each latest as rule (rule.rule_id)}
      <article class="rounded-md border border-border-subtle p-3">
        <div class="flex flex-wrap gap-2"><Badge tone="neutral">Confirmed revision {rule.revision}</Badge><Badge tone="neutral">{provenance[rule.provenance]}</Badge></div>
        <p class="mt-2 whitespace-pre-wrap break-words text-sm text-text-primary">{rule.expected_behavior}</p>
        <p class="mt-1 text-xs text-text-muted">Confirmed by {rule.confirmed_by}. {rule.supersedes ? `Replaces revision ${rule.supersedes}.` : 'First revision.'} No current verification evidence.</p>
        <div class="mt-2 flex flex-wrap items-center gap-3"><Button size="sm" variant="ghost" disabled={!active || model.saving} onclick={() => edit(rule)}>Revise rule</Button><details class="text-xs text-text-secondary"><summary class="cursor-pointer">Revision history</summary><ul class="mt-2 space-y-2">{#each rules.filter((item) => item.rule_id === rule.rule_id) as revision (revision.revision)}<li>Revision {revision.revision} · {provenance[revision.provenance]} · {revision.expected_behavior}</li>{/each}</ul></details></div>
      </article>
    {/each}
  </div>
  {#if capabilities.length > 0}
    <div class="mt-4 space-y-3 border-t border-border-subtle pt-4">
      <h3 class="text-sm font-medium text-text-primary">{previous ? `Draft revision ${previous.revision + 1}` : 'Draft rule'}</h3>
      <Select value={capabilityId} onchange={(value) => (capabilityId = value)} label="Capability for this rule" options={options} disabled={!active || previous !== undefined} />
      <label class="block text-sm text-text-secondary">Expected behavior<Textarea bind:value={behavior} rows={3} maxlength={2000} placeholder="When a customer books a time, staff can see that booking." /></label>
      {#if message}<p class="text-sm text-text-secondary" role="status">{message}</p>{/if}
      <div class="flex flex-wrap gap-2"><Button size="sm" variant="primary" disabled={!active || model.saving} onclick={() => void confirm()}>{model.saving ? 'Confirming…' : 'Confirm this rule'}</Button>{#if editingId}<Button size="sm" variant="ghost" onclick={() => { editingId = null; behavior = ''; }}>Cancel revision</Button>{/if}</div>
    </div>
  {/if}
</Card>
