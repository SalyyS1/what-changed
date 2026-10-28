<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import type { PreviewCaptureDto } from '../../lib/tauri/bindings';
  import { listPreviewCaptures, reviewPreviewCapture, toAppError } from '../../lib/tauri/client';
  let { appId, runId }: { appId: string; runId: string } = $props();
  let captures = $state<PreviewCaptureDto[]>([]); let error = $state<string | null>(null); let busy = $state(false);
  $effect(() => {
    const currentApp = appId; const currentRun = runId; let live = true;
    void listPreviewCaptures(currentApp, currentRun).then((items) => { if (live) captures = items; }).catch((reason) => { if (live) error = toAppError(reason).message; });
    return () => { live = false; };
  });
  async function remove(id: string): Promise<void> {
    busy = true; error = null; const currentRun = runId;
    try { await reviewPreviewCapture(appId, currentRun, id, false); if (runId === currentRun) captures = await listPreviewCaptures(appId, currentRun); }
    catch (reason) { if (runId === currentRun) error = toAppError(reason).message; }
    finally { if (runId === currentRun) busy = false; }
  }
</script>

<details class="mt-4 rounded-md border border-border-subtle p-3"><summary class="cursor-pointer text-sm font-medium text-text-primary">Reviewed screenshots ({captures.length})</summary><p class="mt-2 text-xs text-text-muted">Images expire after seven days and stay outside portable exports. A screenshot supports only the captured surface; it does not establish backend behavior.</p>{#if error}<p class="mt-2 text-sm text-danger" role="alert">{error}</p>{/if}{#each captures as capture (capture.id)}<article class="mt-3 space-y-2"><p class="text-xs text-text-secondary">{capture.role} · retained after review</p><img src={capture.data_url} alt={`Retained test surface for ${capture.role}`} class="max-h-80 w-full object-contain" /><Button size="sm" variant="ghost" disabled={busy} onclick={() => void remove(capture.id)}>Delete this image</Button></article>{:else}<p class="mt-2 text-sm text-text-secondary">No retained screenshot evidence.</p>{/each}</details>
