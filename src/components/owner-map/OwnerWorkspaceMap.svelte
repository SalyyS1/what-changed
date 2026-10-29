<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { OwnerWorkspaceDetail } from '../../lib/stores/owner-workspace.svelte';
  import { ownerPages } from '../../lib/stores/owner-workspace.svelte';
  import type { OwnerEvidenceReviewDto, ScenarioRevisionDto } from '../../lib/tauri/bindings';
  import { listOwnerScenarios, listOwnerManualRuns, toAppError } from '../../lib/tauri/client';
  import OwnerMapFrame from './OwnerMapFrame.svelte';
  interface Props { detail: OwnerWorkspaceDetail; }
  let { detail }: Props = $props();
  let changeId = $state('');
  let scenarios = $state<ScenarioRevisionDto[]>([]);
  let reviews = $state<OwnerEvidenceReviewDto[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(false);
  let generation = 0;
  const change = $derived(detail.changes.find((row) => row.id === changeId) ?? detail.changes[0] ?? null);
  async function load(appId: string, requestId: string | null): Promise<void> {
    const request = ++generation; loading = true; error = null; scenarios = []; reviews = [];
    try {
      const [savedScenarios, savedReviews] = await Promise.all([
        ownerPages((page) => listOwnerScenarios(appId, page)),
        requestId ? ownerPages((page) => listOwnerManualRuns(appId, requestId, page)) : Promise.resolve([]),
      ]);
      if (request === generation) { scenarios = savedScenarios; reviews = savedReviews; }
    } catch (reason) { if (request === generation) error = toAppError(reason).message; }
    finally { if (request === generation) loading = false; }
  }
  $effect(() => { const current = detail; void load(current.app.id, change?.id ?? null); });
  onDestroy(() => { ++generation; });
</script>

<div class="space-y-3">
  {#if detail.changes.length}<label class="block text-xs text-text-secondary">Request shown in workbench<select class="mt-1 min-h-11 w-full rounded-md border border-border-subtle bg-surface-1 p-2 text-sm text-text-primary" value={change?.id ?? ''} onchange={(event) => { changeId = event.currentTarget.value; }}>{#each detail.changes as request (request.id)}<option value={request.id}>{request.intent || 'Untitled saved request'} · v{request.version}</option>{/each}</select></label>{/if}
  {#if loading}<p class="text-sm text-text-secondary" role="status">Loading saved scenarios and evidence…</p>{/if}
  {#if error}<p class="text-sm text-danger" role="alert">Evidence could not be loaded: {error}. Coverage shown below is incomplete.</p><button type="button" class="min-h-11 text-sm text-text-secondary underline" onclick={() => void load(detail.app.id, change?.id ?? null)}>Retry evidence loading</button>{/if}
  <OwnerMapFrame input={{ appId: detail.app.id, capabilities: detail.capabilities, rules: detail.rules, scenarios, reviews, change, candidates: detail.candidates }} readOnly={detail.app.archived_at !== null} />
</div>
