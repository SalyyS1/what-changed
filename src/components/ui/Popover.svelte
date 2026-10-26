<script lang="ts">
  import { Popover as Bits } from 'bits-ui';
  import type { Snippet } from 'svelte';

  /**
   * Anchored, dismissable panel with focus management. `trigger` receives the
   * trigger props to spread on whatever element opens it, so the caller keeps
   * control of the control.
   */
  interface Props {
    open?: boolean;
    side?: 'top' | 'bottom' | 'left' | 'right';
    align?: 'start' | 'center' | 'end';
    class?: string;
    trigger: Snippet<[Record<string, unknown>]>;
    children: Snippet;
  }

  let {
    open = $bindable(false),
    side = 'bottom',
    align = 'start',
    class: className = '',
    trigger,
    children,
  }: Props = $props();
</script>

<Bits.Root bind:open>
  <Bits.Trigger>
    {#snippet child({ props })}
      {@render trigger(props)}
    {/snippet}
  </Bits.Trigger>
  <Bits.Portal>
    <Bits.Content
      {side}
      {align}
      sideOffset={6}
      data-testid="popover"
      class="z-50 min-w-48 rounded-lg border border-border-subtle bg-surface-2 p-3 shadow-lg wc-pop {className}"
    >
      {@render children()}
    </Bits.Content>
  </Bits.Portal>
</Bits.Root>

<style>
  :global(.wc-pop) {
    animation: wc-rise-in var(--t-base) var(--ease-spring);
  }
</style>
