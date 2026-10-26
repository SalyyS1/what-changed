<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Dialog from '../../components/ui/Dialog.svelte';
  import Input from '../../components/ui/Input.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import type { CheckpointProgressItem } from '../../lib/stores/checkpoints.svelte';
  import CheckpointProgress from './CheckpointProgress.svelte';
  interface Props { open?: boolean; creating?: boolean; error?: string | null; progress?: CheckpointProgressItem[]; completed?: number; total?: number; oncreate: (name: string, reason: string) => void; }
  let { open = $bindable(false), creating = false, error = null, progress = [], completed = 0, total = 0, oncreate }: Props = $props();
  let name = $state(''); let reason = $state('');
  function submit(): void { if (!creating) oncreate(name, reason); }
</script>

<Dialog bind:open sticky={creating} title={creating ? 'Creating checkpoint' : 'Create checkpoint'} description="Capture a known-good view of this PC so you can compare against it later.">
  <form id="create-checkpoint" class="flex flex-col gap-4" onsubmit={(event) => { event.preventDefault(); submit(); }}>
    {#if creating}
      <CheckpointProgress items={progress} {completed} {total} />
    {:else}
      {#if error}<p class="rounded-md border border-danger/30 bg-danger/8 px-3 py-2 text-sm text-danger" role="alert">{error}</p>{/if}
      <label class="flex flex-col gap-1.5 text-sm text-text-secondary">Name <span class="text-text-muted">(optional)</span><Input bind:value={name} maxlength={120} placeholder="Checkpoint Friday 3:14 PM" /></label>
      <label class="flex flex-col gap-1.5 text-sm text-text-secondary">What are you doing right now? <span class="text-text-muted">(optional)</span><Textarea bind:value={reason} maxlength={500} placeholder="About to update the graphics driver" /></label>
    {/if}
  </form>
  {#snippet footer()}
    {#if !creating}
      <Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
      <Button variant="primary" type="submit" form="create-checkpoint" disabled={creating}>Create checkpoint</Button>
    {/if}
  {/snippet}
</Dialog>
