<script lang="ts">
  import IconButton from '../components/ui/IconButton.svelte';
  import { ui } from '../lib/stores/ui.svelte';

  /**
   * Bottom-right transient messages from `ui.toast()`. A polite live region
   * so screen readers hear them without being interrupted; each toast is
   * dismissable by keyboard because auto-dismiss alone fails anyone who
   * reads slowly.
   */
  const tones = {
    neutral: 'border-border-subtle',
    success: 'border-success/40',
    warning: 'border-warning/40',
    danger: 'border-danger/40',
  } as const;
</script>

<div
  aria-live="polite"
  aria-relevant="additions"
  data-testid="toast-region"
  class="pointer-events-none fixed right-4 bottom-4 z-50 flex w-80 max-w-[calc(100vw-2rem)] flex-col gap-2"
>
  {#each ui.toasts as toast (toast.id)}
    <div
      role="status"
      class="wc-toast pointer-events-auto flex items-start gap-2 rounded-lg border bg-surface-2 py-2 pr-1 pl-3 text-sm text-text-primary shadow-md {tones[
        toast.tone
      ]}"
    >
      <p class="flex-1 py-1 leading-snug">{toast.message}</p>
      <IconButton icon="close" label="Dismiss" size="sm" onclick={() => ui.dismissToast(toast.id)} />
    </div>
  {/each}
</div>

<style>
  .wc-toast {
    animation: wc-rise-in var(--t-slow) var(--ease-spring);
  }
</style>
