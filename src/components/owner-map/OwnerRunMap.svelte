<script lang="ts">
  import { onDestroy } from 'svelte';
  import type { OwnerEvidenceReviewDto, OwnerCapabilityDto } from '../../lib/tauri/bindings';
  import { listOwnerCapabilities, listOwnerManualRuns, toAppError } from '../../lib/tauri/client';
  import { ownerPages } from '../../lib/stores/owner-workspace.svelte';
  import OwnerMapFrame from './OwnerMapFrame.svelte';
  interface Props { review: OwnerEvidenceReviewDto; readOnly?: boolean; }
  let { review, readOnly = false }: Props = $props();
  let capabilities = $state<OwnerCapabilityDto[]>([]);
  let history = $state<OwnerEvidenceReviewDto[]>([]);
  let error = $state<string | null>(null);
  let generation = 0;
  const key = $derived(`${review.run.app_id}:${review.run.change_id}`);
  const reviews = $derived([review, ...history.filter((row) => row.run.id !== review.run.id)]);
  async function load(appId: string, changeId: string): Promise<void> {
    const request = ++generation; error = null; capabilities = []; history = [];
    try {
      const [savedCapabilities, savedHistory] = await Promise.all([
        listOwnerCapabilities(appId, true),
        ownerPages((page) => listOwnerManualRuns(appId, changeId, page)),
      ]);
      if (generation === request) { capabilities = savedCapabilities; history = savedHistory; }
    } catch (reason) { if (generation === request) error = toAppError(reason).message; }
  }
  $effect(() => { key; void load(review.run.app_id, review.run.change_id); });
  onDestroy(() => { ++generation; });
</script>

{#if error}<p class="mb-2 text-sm text-warning" role="status">Related capabilities or attempt history could not be loaded: {error}. The selected attempt remains readable.</p>{/if}
<OwnerMapFrame input={{ appId: review.run.app_id, capabilities, rules: review.run.manifest.rules, scenarios: review.run.manifest.scenarios, reviews, change: review.run.manifest.change }} {readOnly} preferredRunId={review.run.id} />
