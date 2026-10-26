<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import { exportInvestigationContext } from '../../lib/tauri/client';

  interface Props {
    investigationId: number;
  }

  let { investigationId }: Props = $props();
  let copying = $state(false);
  let confirming = $state(false);
  let copied = $state(false);
  let error = $state<string | null>(null);

  async function copyContext(): Promise<void> {
    if (copying) return;
    if (!confirming) {
      confirming = true;
      return;
    }
    confirming = false;
    copying = true;
    copied = false;
    error = null;
    try {
      const markdown = await exportInvestigationContext(investigationId);
      if (!globalThis.navigator.clipboard) throw new Error('Clipboard is unavailable in this window.');
      await globalThis.navigator.clipboard.writeText(markdown);
      copied = true;
    } catch (reason) {
      error = reason instanceof Error ? reason.message : 'The AI context could not be copied.';
    } finally {
      copying = false;
    }
  }
</script>

<div class="flex flex-col items-end gap-1">
  <Button size="sm" variant="secondary" disabled={copying} onclick={() => void copyContext()}>
    {copying ? 'Preparing…' : confirming ? 'Copy redacted pack' : copied ? 'Copied for AI' : 'Copy context for AI'}
  </Button>
  {#if confirming}
    <div class="flex items-center gap-2">
      <p class="max-w-56 text-right text-xs text-text-muted" role="note">
        This copies the redacted pack to your clipboard only. It stays until another app overwrites it; clear it manually when finished.
      </p>
      <Button size="sm" variant="ghost" onclick={() => { confirming = false; }}>Cancel</Button>
    </div>
  {/if}
  {#if error}<p class="max-w-56 text-right text-xs text-danger" role="alert">{error}</p>{/if}
  {#if copied}<p class="text-right text-xs text-text-muted" role="status">The pack stays in your clipboard only; clear it when finished.</p>{/if}
</div>
