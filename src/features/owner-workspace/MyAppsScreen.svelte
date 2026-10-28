<script lang="ts">
  import { onMount } from 'svelte';
  import ScreenFrame from '../../components/layout/ScreenFrame.svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Dialog from '../../components/ui/Dialog.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import { ownerWorkspace, type OwnerWorkspaceStore } from '../../lib/stores/owner-workspace.svelte';
  import OwnerWorkspaceFixture from './OwnerWorkspaceFixture.svelte';
  import ImportReview from '../project-memory/ImportReview.svelte';

  interface Props { active?: boolean; model?: OwnerWorkspaceStore; onOpenApp?: (appId: string) => void; }
  let { active = true, model = ownerWorkspace, onOpenApp }: Props = $props();
  let createOpen = $state(false);
  let archived = $state(false);
  let name = $state('');
  let description = $state('');
  let formError = $state<string | null>(null);
  let exampleOpen = $state(false);

  onMount(() => { if (active) void model.loadApps(archived); return () => model.cancel(); });
  function startCreate(): void { name = ''; description = ''; formError = null; createOpen = true; }
  async function create(): Promise<void> {
    if (!name.trim()) { formError = 'Give this app a name so you can recognize it later.'; return; }
    const app = await model.saveApp({ name: name.trim(), description: description.trim(), legacy_project_id: null, archived: false });
    if (app) { createOpen = false; onOpenApp?.(app.id); }
    else formError = model.error ?? 'The app could not be saved. Your details are still here.';
  }
  async function archive(id: string, currentName: string, isArchived: boolean): Promise<void> {
    const row = model.rows.find((app) => app.id === id);
    if (row) await model.saveApp({ name: currentName, description: row.description, legacy_project_id: row.legacy_project_id, archived: !isArchived }, row);
  }
</script>

<ScreenFrame title="My apps" subtitle="Keep each AI-built app's intent and decisions together, offline.">
  {#snippet actions()}
    <Button variant="primary" disabled={!active || model.saving} onclick={startCreate}>Create app</Button>
  {/snippet}
  <div class="mb-4 flex items-center justify-between gap-3">
    <p class="text-sm text-text-secondary">Your app memory stays on this PC. Nothing is contacted while you create it.</p>
    <div class="flex flex-wrap items-center gap-3"><Button size="sm" variant="ghost" disabled={!active} onclick={() => (exampleOpen = true)}>Try example</Button><label class="inline-flex items-center gap-2 text-sm text-text-secondary"><input type="checkbox" disabled={!active} bind:checked={archived} onchange={() => void model.loadApps(archived)} /> Show archived</label></div>
  </div>
  {#if model.error}<p class="mb-3 rounded-md border border-danger/30 bg-danger/8 px-3 py-2 text-sm text-danger" role="alert">{model.error}</p>{/if}
  <StateShell state={model.apps} emptyTitle="No apps yet" emptyDescription="Create an app to record what it should do and what must stay working." emptyIcon="settings" errorTitle="Apps could not be loaded" loadingRows={3}>
    {#snippet children(rows)}
      <div class="grid gap-3 @[760px]/shell:grid-cols-2">
        {#each rows as app (app.id)}
          <Card level={1}>
            <div class="flex items-start justify-between gap-3">
              <div class="min-w-0"><h2 class="truncate text-base font-medium text-text-primary">{app.name}</h2><p class="mt-1 line-clamp-2 text-sm text-text-secondary">{app.description || 'No description yet.'}</p></div>
              <Badge tone="neutral" dot>{app.archived_at === null ? 'Active' : 'Archived'}</Badge>
            </div>
            <div class="mt-4 flex flex-wrap gap-2"><Button size="sm" variant="primary" disabled={!active} onclick={() => onOpenApp?.(app.id)}>Open app</Button><Button size="sm" variant="ghost" disabled={!active || model.saving} onclick={() => void archive(app.id, app.name, app.archived_at !== null)}>{app.archived_at === null ? 'Archive' : 'Restore'}</Button></div>
          </Card>
        {/each}
      </div>
    {/snippet}
  </StateShell>
  {#if active}<div class="mt-4"><ImportReview onImported={() => model.loadApps(archived)} {onOpenApp} /></div>{/if}
</ScreenFrame>

<Dialog bind:open={createOpen} title="Create an app" description="Start with the name and purpose you want to remember.">
  <div class="space-y-4">
    <label class="block text-sm text-text-secondary">App name<Input class="mt-1" bind:value={name} maxlength={120} placeholder="For example, the booking app" /></label>
    <label class="block text-sm text-text-secondary">What is it for?<Textarea bind:value={description} rows={3} maxlength={1000} placeholder="A short description helps future changes stay focused." /></label>
    {#if formError}<p class="text-sm text-danger" role="alert">{formError}</p>{/if}
  </div>
  {#snippet footer()}<Button variant="ghost" onclick={() => (createOpen = false)}>Cancel</Button><Button variant="primary" disabled={model.saving} onclick={() => void create()}>{model.saving ? 'Saving…' : 'Create app'}</Button>{/snippet}
</Dialog>

<Dialog bind:open={exampleOpen} title="Synthetic booking example" description="Read-only example. It is separate from your saved apps and never contacts a preview." width="lg">
  {#if exampleOpen}<div class="flex h-96 min-h-0 flex-col overflow-hidden"><OwnerWorkspaceFixture /></div>{/if}
</Dialog>
