<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Icon from '../../components/ui/Icon.svelte';
  import { dbRecover } from '../../lib/tauri/client';

  let pending = $state<string | null>(null);
  let resetPhrase = $state('');
  let message = $state<string | null>(null);

  async function recover(action: 'repair' | 'export_diagnostics' | 'reset_history'): Promise<void> {
    if (action === 'reset_history' && resetPhrase.trim().toLowerCase() !== 'reset') return;
    pending = action;
    message = null;
    try {
      const outcome = await dbRecover(action);
      message = outcome.message;
      if (outcome.succeeded) {
        window.setTimeout(() => window.location.reload(), 250);
      }
    } catch {
      message = 'The recovery action could not be completed.';
    } finally {
      pending = null;
    }
  }
</script>

<div class="flex min-h-0 flex-1 items-center justify-center px-6 py-8" data-testid="recovery-surface">
  <Card level={1} class="max-w-lg text-center">
    <span class="mx-auto inline-flex h-10 w-10 items-center justify-center rounded-full bg-danger/15 text-danger"><Icon name="alert" size={18} /></span>
    <h1 class="mt-3 font-display text-xl text-text-primary">Recovery needed</h1>
    <p class="mt-2 text-sm leading-relaxed text-text-secondary">Your history database needs attention before What Changed can show ordinary history.</p>
    <div class="mt-4 flex flex-wrap justify-center gap-2">
      <Button variant="primary" disabled={pending !== null} onclick={() => void recover('repair')}>
        {pending === 'repair' ? 'Repairing…' : 'Repair database'}
      </Button>
      <Button variant="secondary" disabled={pending !== null} onclick={() => void recover('export_diagnostics')}>
        {pending === 'export_diagnostics' ? 'Exporting…' : 'Export diagnostics'}
      </Button>
      <a href="#/diagnostics" class="inline-flex min-h-11 items-center rounded-md px-3 text-sm text-text-secondary underline">Open diagnostics</a>
    </div>
    <div class="mt-4 text-left">
      <label class="text-xs text-text-muted" for="reset-history-confirm">Type reset to discard damaged history</label>
      <div class="mt-1 flex gap-2">
        <input id="reset-history-confirm" class="min-h-11 min-w-0 flex-1 rounded-md border border-border bg-surface px-3 text-sm" bind:value={resetPhrase} autocomplete="off" />
        <Button variant="danger" disabled={pending !== null || resetPhrase.trim().toLowerCase() !== 'reset'} onclick={() => void recover('reset_history')}>
          {pending === 'reset_history' ? 'Resetting…' : 'Reset history'}
        </Button>
      </div>
    </div>
    {#if message}<p class="mt-3 text-sm text-text-secondary" role="status">{message}</p>{/if}
  </Card>
</div>
