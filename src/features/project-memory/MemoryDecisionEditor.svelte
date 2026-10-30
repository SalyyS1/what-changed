<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import type { OwnerCapabilityDto, OwnerChangeDraftDto, ProjectDecisionRevisionDto, RuleProvenance } from '../../lib/tauri/bindings';
  import { memoryProvenance, type DecisionDraft } from './project-memory-model.svelte';
  interface Props { previous?: ProjectDecisionRevisionDto | null; capabilities: OwnerCapabilityDto[]; changes: OwnerChangeDraftDto[]; disabled?: boolean; saving?: boolean; onSave: (draft: DecisionDraft) => Promise<boolean>; onCancel: () => void; }
  let { previous = null, capabilities, changes, disabled = false, saving = false, onSave, onCancel }: Props = $props();
  let text = $state(''); let rationale = $state(''); let author = $state(''); let capabilityId = $state('none'); let changeId = $state('none'); let provenance = $state<RuleProvenance>('owner'); let reviewed = $state(false);
  $effect(() => { const revision = previous; text = revision?.text ?? ''; rationale = revision?.rationale ?? ''; author = revision?.recorded_by ?? ''; capabilityId = revision?.capability_id ?? 'none'; changeId = revision?.change_id ?? 'none'; provenance = revision?.provenance ?? 'owner'; reviewed = false; });
  const capabilitiesOptions = $derived([{ value: 'none', label: 'Whole app' }, ...capabilities.map((item) => ({ value: item.id, label: item.label }))]);
  const changeOptions = $derived([{ value: 'none', label: 'No particular request' }, ...changes.map((item) => ({ value: item.id, label: item.intent || 'Untitled saved request' }))]);
  const provenanceOptions = Object.entries(memoryProvenance).map(([value, label]) => ({ value, label }));
  const ready = $derived(!disabled && !saving && text.trim() !== '' && rationale.trim() !== '' && author.trim() !== '' && (!previous || reviewed));
  async function save(): Promise<void> {
    if (!ready) return;
    const draft = { capability_id: capabilityId === 'none' ? null : capabilityId, change_id: changeId === 'none' ? null : changeId, text: text.trim(), rationale: rationale.trim(), recorded_by: author.trim(), provenance };
    if (await onSave(draft)) onCancel();
  }
</script>

<div class="space-y-3 border-t border-border-subtle pt-4">
  <h3 class="text-sm font-medium text-text-primary">{previous ? `Draft decision revision ${previous.revision + 1}` : 'New project decision'}</h3>
  {#if previous}<div class="rounded-md border border-border-subtle bg-surface-2 p-3 text-sm"><h4 class="font-medium text-text-primary">Saved revision {previous.revision}</h4><p class="mt-1 whitespace-pre-wrap break-words text-text-secondary">{previous.text}</p><p class="mt-2 whitespace-pre-wrap break-words text-text-secondary">Why: {previous.rationale}</p><p class="mt-1 text-xs text-text-muted">{memoryProvenance[previous.provenance]} · {previous.recorded_by}. This revision will stay in history.</p><p class="mt-1 break-words text-xs text-text-muted">Capability: {capabilities.find((item) => item.id === previous?.capability_id)?.label ?? previous.capability_id ?? 'Whole app'} · Request: {changes.find((item) => item.id === previous?.change_id)?.intent ?? previous.change_id ?? 'No particular request'}</p></div>{/if}
  <fieldset class="space-y-3" disabled={disabled || saving}>
    <label class="block text-sm text-text-secondary">Decision wording<Textarea bind:value={text} rows={3} maxlength={16000} oninput={() => (reviewed = false)} /></label>
    <label class="block text-sm text-text-secondary">Why this decision was made<Textarea bind:value={rationale} rows={2} maxlength={16000} oninput={() => (reviewed = false)} /></label>
    <label class="block text-sm text-text-secondary">Decision author<Input bind:value={author} maxlength={120} oninput={() => (reviewed = false)} /></label>
    <div class="grid gap-3 @[760px]/shell:grid-cols-2"><Select label="Decision capability" value={capabilityId} options={capabilitiesOptions} onchange={(value) => { capabilityId = value; reviewed = false; }} disabled={disabled || saving} /><Select label="Decision change request" value={changeId} options={changeOptions} onchange={(value) => { changeId = value; reviewed = false; }} disabled={disabled || saving} /></div>
    <Select label="Wording source" value={provenance} options={provenanceOptions} onchange={(value) => { provenance = value as RuleProvenance; reviewed = false; }} disabled={disabled || saving} />
    {#if previous}<label class="flex items-start gap-2 text-sm text-text-secondary"><input type="checkbox" bind:checked={reviewed} />I reviewed the saved and proposed wording, rationale, source and links. Save a new revision.</label>{/if}
  </fieldset>
  <div class="flex flex-wrap gap-2"><Button variant="primary" disabled={!ready} onclick={() => void save()}>{saving ? 'Saving decision…' : previous ? 'Save reviewed revision' : 'Save project decision'}</Button><Button variant="ghost" disabled={saving} onclick={onCancel}>Cancel draft</Button></div>
</div>
