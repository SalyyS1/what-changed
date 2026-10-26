<script lang="ts">
  import { Tooltip as Bits } from 'bits-ui';
  import type { Snippet } from 'svelte';

  import { TOOLTIP_DELAY_MS } from '../../lib/design/motion';

  /**
   * Hover/focus hint. Text only — a tooltip that needs markup is a popover.
   * `Tooltip.Provider` lives here so callers never have to remember it; the
   * delay is shared across the app so tips feel like one system.
   */
  interface Props {
    text: string;
    side?: 'top' | 'bottom' | 'left' | 'right';
    children: Snippet;
  }

  let { text, side = 'top', children }: Props = $props();
</script>

<Bits.Provider delayDuration={TOOLTIP_DELAY_MS}>
  <Bits.Root>
    <Bits.Trigger>
      {#snippet child({ props })}
        <span {...props} class="inline-flex">{@render children()}</span>
      {/snippet}
    </Bits.Trigger>
    <Bits.Portal>
      <Bits.Content
        {side}
        sideOffset={6}
        data-testid="tooltip"
        class="z-50 max-w-64 rounded-md border border-border-subtle bg-surface-3 px-2.5 py-1.5 text-sm text-text-primary shadow-md wc-tip"
      >
        {text}
      </Bits.Content>
    </Bits.Portal>
  </Bits.Root>
</Bits.Provider>

<style>
  :global(.wc-tip) {
    animation: wc-fade-in var(--t-fast) var(--ease-standard);
  }
</style>
