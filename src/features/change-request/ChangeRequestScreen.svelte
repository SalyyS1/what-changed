<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import ScreenFrame from '../../components/layout/ScreenFrame.svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Dialog from '../../components/ui/Dialog.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import { changeRequest, type ChangeRequestStore } from '../../lib/stores/change-request.svelte';
  import PreserveRulesStep from './PreserveRulesStep.svelte';
  import ChangeBriefReview from './ChangeBriefReview.svelte';
  import ExportReview from '../project-memory/ExportReview.svelte';

  interface Props {
    active?: boolean; appId?: string | undefined; changeId?: string | undefined; model?: ChangeRequestStore;
    onLeave?: () => void; onDirtyChange?: (dirty: boolean) => void;
    onSaved?: (appId: string, changeId: string) => void;
    onReviewExport?: (appId: string, changeId: string) => void;
  }
  let { active = true, appId, changeId, model = changeRequest, onLeave, onDirtyChange, onSaved, onReviewExport }: Props = $props();
  let step = $state(0);
  let leaveOpen = $state(false);
  let reloadOpen = $state(false);
  let validation = $state<string | null>(null);
  let exportOpen = $state(false);
  const editable = $derived(active && model.context?.app.archived_at === null && !model.saving);
  const steps = ['Intent', 'Preserve', 'Brief', 'Verify', 'Decide'];
  $effect(() => {
    const target = { active, appId, changeId };
    untrack(() => {
      if (!target.active || !target.appId) return;
      // Saving a new draft adds its ID to the URL. The already-loaded saved
      // draft stays at its current wizard step instead of reopening itself.
      if (model.appId === target.appId && model.changeId === target.changeId && model.context) return;
      step = 0; exportOpen = false; void model.load(target.appId, target.changeId);
    });
  });
  onDestroy(() => model.cancel());
  $effect(() => { onDirtyChange?.(model.dirty); });
  $effect(() => () => onDirtyChange?.(false));

  function beforeUnload(event: BeforeUnloadEvent): void { if (model.dirty) { event.preventDefault(); event.returnValue = ''; } }
  function back(): void { if (model.dirty) leaveOpen = true; else onLeave?.(); }
  function advance(): void {
    if (step === 0 && (!model.intent.trim() || !model.example.trim())) { validation = 'Describe the outcome and one concrete example before continuing. You can save an incomplete draft at any time.'; return; }
    validation = null; step = Math.min(step + 1, 2);
  }
  async function save(ready = false, leave = false): Promise<void> {
    if (await model.save(ready ? 'ready_for_review' : 'draft')) {
      if (model.appId && model.changeId) onSaved?.(model.appId, model.changeId);
      if (leave && !model.dirty) { leaveOpen = false; onLeave?.(); }
    }
  }
  function reload(): void { reloadOpen = false; if (appId) void model.load(appId, model.draft?.id ?? changeId); }
</script>

<svelte:window onbeforeunload={beforeUnload} />

<ScreenFrame title={changeId ? 'Change request' : 'Request a change'} subtitle={model.context ? `For ${model.context.app.name} · saved locally, shared only after export review` : 'Describe the next change in your own words.'}>
  {#snippet actions()}<Button variant="ghost" onclick={back}>Back to app</Button>{/snippet}
  {#if !appId}<Card><p class="text-sm text-text-secondary">Choose an app before starting a change request.</p></Card>
  {:else}
    <StateShell state={model.state} emptyTitle="Change unavailable" errorTitle="This request could not be opened" loadingRows={4}>
      {#snippet children(context)}
        <div class="max-w-3xl space-y-4">
          <nav aria-label="Change journey"><ol class="flex flex-wrap gap-2">{#each steps as label, index (label)}<li><button class="rounded-md border px-3 py-2 text-sm {step === index ? 'border-border-active bg-surface-3 text-text-primary' : 'border-border-subtle text-text-muted'}" aria-current={step === index ? 'step' : undefined} disabled={index > 2} onclick={() => { step = index; validation = null; }}>{index + 1}. {label}{index > 2 ? ' · unavailable' : ''}</button></li>{/each}</ol></nav>
          <div class="flex flex-wrap items-center gap-2" aria-live="polite"><Badge tone="neutral">{model.draft ? `Saved revision ${model.draft.version}` : 'New local draft'}</Badge>{#if model.dirty}<Badge tone="warning">Unsaved changes</Badge>{:else if model.saved}<span class="text-sm text-text-secondary">Saved locally.</span>{/if}{#if context.app.archived_at !== null}<Badge tone="neutral">Archived app · read only</Badge>{/if}</div>
          {#if model.error}<div class="rounded-md border border-danger/30 bg-danger/8 p-3" role="alert"><p class="text-sm text-danger">{model.error}</p><p class="mt-1 text-sm text-text-secondary">Your wording is still here.</p>{#if model.conflict}<p class="mt-2 text-sm text-text-secondary">Another saved revision changed. Reload it after discarding these edits, or keep your wording as a separate request.</p><div class="mt-3 flex flex-wrap gap-2"><Button size="sm" variant="secondary" onclick={() => (reloadOpen = true)}>Review latest saved draft</Button><Button size="sm" variant="secondary" onclick={() => model.saveAsNew()}>Keep as new request</Button></div>{/if}</div>{/if}
          <Card level={1}>
            {#if step === 0}
              <section aria-labelledby="intent-title"><h2 id="intent-title" class="font-medium text-text-primary">What do you want to change?</h2><p class="mt-1 text-sm text-text-secondary">Describe the result you want. Implementation choices can stay with your builder.</p><fieldset class="mt-4 space-y-4" disabled={!editable}><legend class="sr-only">Change intent</legend><label class="block text-sm text-text-secondary">Desired outcome<Textarea bind:value={model.intent} rows={3} maxlength={4000} placeholder="Send a reminder before a booking begins." /></label><label class="block text-sm text-text-secondary">A concrete example<Textarea bind:value={model.example} rows={3} maxlength={4000} placeholder="For a booking tomorrow at 10 am, send one reminder today at 10 am." /></label><label class="block text-sm text-text-secondary">Scope limits and unresolved business choices (optional)<Textarea bind:value={model.scope} rows={3} maxlength={4000} placeholder="Keep cancellation behavior. Decide whether reminders use email or SMS before building." /></label></fieldset></section>
            {:else if step === 1}<PreserveRulesStep {model} disabled={!editable} />
            {:else}<ChangeBriefReview {model} />{/if}
          </Card>
          {#if validation}<p class="text-sm text-danger" role="alert">{validation}</p>{/if}
          <div class="flex flex-wrap items-center justify-between gap-3 border-t border-border-subtle pt-3">
            <div class="flex flex-wrap gap-2"><Button variant="secondary" disabled={!editable || model.conflict} onclick={() => void save()}>{model.saving ? 'Saving…' : 'Save draft'}</Button>{#if step > 0}<Button variant="ghost" onclick={() => (step -= 1)}>Previous</Button>{/if}</div>
            {#if step < 2}<Button variant="primary" disabled={!editable} onclick={advance}>{step === 0 ? 'Choose what stays' : 'Review brief'}</Button>
            {:else}<Button variant="primary" disabled={!editable || !model.canReview || model.conflict} onclick={() => void save(true)}>{model.saving ? 'Saving…' : 'Mark brief ready'}</Button>{/if}
          </div>
          {#if step === 2 && model.draft?.state === 'ready_for_review' && !model.dirty}<Button variant="secondary" onclick={() => { exportOpen = true; if (model.appId && model.changeId) onReviewExport?.(model.appId, model.changeId); }}>Review brief export</Button>{/if}
          {#if exportOpen && model.draft && model.appId && model.changeId}<ExportReview appId={model.appId} changeId={model.changeId} version={model.draft.version} dirty={model.dirty} onClose={() => (exportOpen = false)} />{/if}
          <p class="text-xs text-text-muted">Verify and Decide become available only with supported execution and real evidence. A saved or ready brief is owner intent, not a passed check.</p>
        </div>
      {/snippet}
    </StateShell>
  {/if}
</ScreenFrame>

<Dialog bind:open={leaveOpen} title="Leave with unsaved changes?" description="Keep this request by saving it locally, or discard the edits from this session.">
  <p class="text-sm text-text-secondary">Failed saves keep the wording on this screen. You can continue editing.</p>
  {#snippet footer()}<div class="flex flex-wrap justify-end gap-2"><Button variant="ghost" onclick={() => (leaveOpen = false)}>Keep editing</Button><Button variant="danger" onclick={() => { leaveOpen = false; model.cancel(); onLeave?.(); }}>Discard edits</Button><Button variant="primary" disabled={model.saving || model.conflict} onclick={() => void save(false, true)}>Save and leave</Button></div>{/snippet}
</Dialog>
<Dialog bind:open={reloadOpen} title="Discard edits and open the saved revision?" description="Your unsaved wording will be replaced. Choose Keep editing to preserve it or save it as a separate request.">
  <p class="text-sm text-text-secondary">The latest saved rule revisions will be loaded for review.</p>
  {#snippet footer()}<Button variant="ghost" onclick={() => (reloadOpen = false)}>Keep editing</Button><Button variant="danger" onclick={reload}>Discard and reload</Button>{/snippet}
</Dialog>
