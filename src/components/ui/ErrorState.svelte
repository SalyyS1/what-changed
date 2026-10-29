<script lang="ts">
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';

  /**
   * The failure face of a screen. The message is the real one from the
   * backend, never "Something went wrong"; the retry is offered only when the
   * caller can actually retry. `role="alert"` so assistive tech announces it
   * the moment it appears.
   */
  interface Props {
    title?: string | undefined;
    message: string;
    retry?: (() => void) | undefined;
    retryLabel?: string;
    class?: string;
  }

  let {
    title = 'This could not be loaded',
    message,
    retry,
    retryLabel = 'Try again',
    class: className = '',
  }: Props = $props();
</script>

<div
  data-testid="error-state"
  role="alert"
  class="flex min-h-40 flex-col items-center justify-center gap-2 px-6 py-10 text-center wc-rise {className}"
>
  <span
    class="mb-1 inline-flex h-10 w-10 items-center justify-center rounded-full border border-danger/30 bg-danger/12 text-danger"
  >
    <Icon name="alert" size={18} />
  </span>
  <p class="text-md font-medium text-text-primary">{title}</p>
  <p class="max-w-md font-mono text-sm leading-relaxed break-words text-text-secondary selectable">
    {message}
  </p>
  {#if retry}
    <div class="mt-3">
      <Button variant="secondary" size="sm" onclick={retry}>
        <Icon name="refresh" size={14} />
        {retryLabel}
      </Button>
    </div>
  {/if}
</div>

<style>
  .wc-rise {
    animation: wc-rise-in var(--t-slow) var(--ease-spring);
  }
</style>
