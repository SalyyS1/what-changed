<script lang="ts">
  import ScreenFrame from '../../components/layout/ScreenFrame.svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Dialog from '../../components/ui/Dialog.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Select from '../../components/ui/Select.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import { ownerWorkspace, type OwnerWorkspaceStore } from '../../lib/stores/owner-workspace.svelte';
  import type { ProjectDto } from '../../lib/tauri/bindings';
  import CapabilityEditor from './CapabilityEditor.svelte';
  import RuleEditor from './RuleEditor.svelte';
  import PreviewSetup from './PreviewSetup.svelte';
  import CandidateSetup from './CandidateSetup.svelte';
  import OwnerAcceptancePanel from '../acceptance/OwnerAcceptancePanel.svelte';
  import ProjectMemoryScreen from '../project-memory/ProjectMemoryScreen.svelte';
  import OwnerWorkspaceMap from '../../components/owner-map/OwnerWorkspaceMap.svelte';

  interface Props { active?: boolean; appId?: string | undefined; model?: OwnerWorkspaceStore; legacyProjects?: ProjectDto[]; onRequestChange?: (appId: string) => void; onOpenChange?: (appId: string, changeId: string) => void; }
  let { active = true, appId, model = ownerWorkspace, legacyProjects = [], onRequestChange, onOpenChange }: Props = $props();
  let editing = $state(false);
  let name = $state('');
  let description = $state('');
  let legacyProjectId = $state('');
  let formError = $state<string | null>(null);
  const liveApp = $derived(active && model.current?.app.archived_at === null);
  const projectOptions = $derived([{ value: 'none', label: 'No local project linked' }, ...legacyProjects.map((item) => ({ value: item.id, label: item.name }))]);
  $effect(() => { if (active && appId) void model.loadApp(appId); return () => model.cancel(); });

  function startEdit(): void {
    if (!model.current) return;
    name = model.current.app.name; description = model.current.app.description;
    legacyProjectId = model.current.app.legacy_project_id ?? 'none';
    formError = null; editing = true;
  }
  async function save(): Promise<void> {
    if (!name.trim() || !model.current) { formError = 'Give this app a name.'; return; }
    const app = await model.saveApp({ name: name.trim(), description: description.trim(), legacy_project_id: legacyProjectId === 'none' ? null : legacyProjectId, archived: model.current.app.archived_at !== null }, model.current.app);
    if (app) editing = false;
    else formError = model.error ?? 'The app could not be saved. Your details are still here.';
  }
</script>

<ScreenFrame title={model.current?.app.name ?? 'App overview'} subtitle="Record intent now. Verify a named candidate when supported execution is available.">
  {#snippet actions()}<Button variant="primary" disabled={!liveApp || model.saving || !appId} onclick={() => { if (appId) onRequestChange?.(appId); }}>Request change</Button>{/snippet}
  {#if !appId}<Card><p class="text-sm text-text-secondary">Choose an app from My apps to open its workspace.</p></Card>
  {:else}
    {#if model.error}<p class="mb-3 rounded-md border border-danger/30 bg-danger/8 px-3 py-2 text-sm text-danger" role="alert">{model.error} Your input stays available. Reload to review the latest saved revision before retrying.</p>{/if}
    <StateShell state={model.detail} emptyTitle="App not available" errorTitle="This app could not be loaded" loadingRows={4}>
      {#snippet children(detail)}
        <div class="max-w-4xl space-y-4">
          <Card><div class="flex items-start justify-between gap-3"><div class="min-w-0"><div class="flex flex-wrap items-center gap-2"><h2 class="font-medium text-text-primary">App purpose</h2><Badge tone="neutral">{detail.app.archived_at === null ? `Version ${detail.app.version}` : 'Archived · read only'}</Badge></div><p class="mt-2 whitespace-pre-wrap break-words text-sm text-text-secondary">{detail.app.description || 'Add a short purpose to keep future changes focused.'}</p><p class="mt-2 text-xs text-text-muted">{detail.app.legacy_project_id ? 'Linked to a local diagnostics project. Linking does not enable collectors.' : 'No local diagnostics project linked. This is optional.'}</p></div><Button size="sm" variant="ghost" disabled={!active || model.saving} onclick={startEdit}>Edit app</Button></div></Card>
          <Card><h2 class="font-medium text-text-primary">Change requests</h2><p class="mt-1 text-sm text-text-secondary">Every new request can reuse confirmed rules. Previous evidence never becomes a new current pass.</p>{#if detail.changes.length === 0}<p class="mt-3 text-sm text-text-muted">No requests yet. Record your next intended change when ready.</p>{:else}<ul class="mt-3 space-y-2">{#each detail.changes as change (change.id)}<li><button class="flex w-full flex-wrap items-center justify-between gap-3 rounded-md border border-border-subtle p-3 text-left hover:bg-surface-2" onclick={() => onOpenChange?.(detail.app.id, change.id)}><span class="min-w-0 flex-1 truncate text-sm text-text-primary">{change.intent || 'Untitled saved request'}</span><Badge tone="neutral">{change.state.replaceAll('_', ' ')} · v{change.version}</Badge><span class="text-xs text-text-muted">{change.preserved_rules.length} preserved rules</span></button></li>{/each}</ul>{/if}</Card>
          <CapabilityEditor capabilities={detail.capabilities} {model} active={liveApp} />
          {#if active}<OwnerWorkspaceMap {detail} />{/if}
          <RuleEditor capabilities={detail.capabilities} rules={detail.rules} {model} active={liveApp} />
          <PreviewSetup environments={detail.environments} {model} active={liveApp} />
          <CandidateSetup environments={detail.environments} candidates={detail.candidates} {model} active={liveApp} />
          {#if active}<OwnerAcceptancePanel appId={detail.app.id} changes={detail.changes} capabilities={detail.capabilities} rules={detail.rules} environments={detail.environments} active={liveApp} />{/if}
          {#if active}<ProjectMemoryScreen appId={detail.app.id} changes={detail.changes} capabilities={detail.capabilities} readOnly={!liveApp} />{/if}
        </div>
      {/snippet}
    </StateShell>
  {/if}
</ScreenFrame>

<Dialog bind:open={editing} title="Edit app" description="The optional project link only connects local context; it does not enable collectors.">
  <div class="space-y-3"><label class="block text-sm text-text-secondary">App name<Input bind:value={name} maxlength={120} /></label><label class="block text-sm text-text-secondary">Purpose<Textarea bind:value={description} maxlength={1000} rows={3} /></label><Select value={legacyProjectId} onchange={(value) => (legacyProjectId = value)} options={projectOptions} label="Optional local diagnostics project" />{#if formError}<p class="text-sm text-danger" role="alert">{formError}</p>{/if}</div>
  {#snippet footer()}<Button variant="ghost" onclick={() => (editing = false)}>Cancel</Button><Button variant="primary" disabled={model.saving} onclick={() => void save()}>{model.saving ? 'Saving…' : 'Save app'}</Button>{/snippet}
</Dialog>
