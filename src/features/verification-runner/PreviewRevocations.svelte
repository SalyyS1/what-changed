<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import { previewRevocations, type PreviewRevocationsStore } from '../../lib/stores/preview-revocations.svelte';
  let { model = previewRevocations }: { model?: PreviewRevocationsStore } = $props();
</script>

{#if model.sessions.length}
  <section aria-label="Preview permission revocation" class="max-h-56 shrink-0 space-y-3 overflow-y-auto border-b border-warning bg-surface-1 px-6 py-4">
    <h2 class="text-sm font-medium text-text-primary" role="status">Preview permission revocation is not confirmed</h2>
    <p class="text-sm text-text-secondary">A test browser may still be running. The app keeps retrying until native status confirms it stopped. These controls stay available when you leave a workspace.</p>
    {#each model.sessions as session (JSON.stringify([session.app_id, session.run_id]))}
      <div class="space-y-2">
        <p class="selectable break-all text-xs text-text-muted">App {session.app_id} · Run {session.run_id}</p>
        {#if session.error}<p class="text-sm text-danger" role="alert">{session.error}</p>{/if}
        <Button size="sm" variant="secondary" disabled={session.busy} onclick={() => void model.retry(session.app_id, session.run_id)}>{session.busy ? 'Revoking and checking status…' : 'Retry revoke and check status'}</Button>
      </div>
    {/each}
  </section>
{/if}
