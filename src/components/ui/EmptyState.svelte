<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IconName } from './ui-types';

  /**
   * The "nothing here" face of a screen. It is a primitive, not a message:
   * the title says what is true right now, the description says what would
   * make content appear, and the optional action is the one thing worth
   * doing about it. Never a sad-face illustration, never lorem.
   */
  interface Props {
    title: string;
    description?: string | undefined;
    icon?: IconName | undefined;
    action?: Snippet | undefined;
    class?: string;
  }

  let { title, description, icon = 'info', action, class: className = '' }: Props = $props();
</script>

<div
  data-testid="empty-state"
  role="status"
  class="flex min-h-40 flex-col items-center justify-center gap-2 px-6 py-10 text-center wc-rise {className}"
>
  <span
    class="mb-1 inline-flex h-10 w-10 items-center justify-center rounded-full border border-border-subtle bg-surface-2 text-text-muted"
  >
    <Icon name={icon} size={18} />
  </span>
  <p class="text-md font-medium text-text-primary">{title}</p>
  {#if description}
    <p class="max-w-sm text-sm leading-relaxed text-text-secondary">{description}</p>
  {/if}
  {#if action}
    <div class="mt-3">{@render action()}</div>
  {/if}
</div>

<style>
  .wc-rise {
    animation: wc-rise-in var(--t-slow) var(--ease-spring);
  }
</style>
