<script lang="ts">
  import Button from '../../components/ui/Button.svelte';
  import Textarea from '../../components/ui/Textarea.svelte';
  import type { InvestigationWindow } from './investigation-types';
  import WindowPicker from './WindowPicker.svelte';

  interface Props {
    problem?: string;
    window?: InvestigationWindow;
    running?: boolean;
    onsubmit: () => void;
  }

  let {
    problem = $bindable(''),
    window = $bindable({ kind: 'today' }),
    running = false,
    onsubmit,
  }: Props = $props();

  const validWindow = $derived.by(() => {
    if (window.kind === 'since_checkpoint') return window.id > 0;
    if (window.kind === 'custom') {
      return Number.isFinite(window.from) && Number.isFinite(window.to) && window.from <= window.to;
    }
    return true;
  });
</script>

<form
  class="flex flex-col gap-3"
  onsubmit={(event) => { event.preventDefault(); onsubmit(); }}
  data-testid="investigation-form"
>
  <Textarea
    bind:value={problem}
    label="What stopped working?"
    placeholder="Example: Discord stopped detecting my microphone this morning"
    rows={3}
    disabled={running}
  />
  <WindowPicker bind:value={window} disabled={running} />
  <div class="flex justify-end">
    <Button variant="primary" type="submit" disabled={running || problem.trim() === '' || !validWindow}>
      {running ? 'Investigating…' : 'Investigate'}
    </Button>
  </div>
</form>
