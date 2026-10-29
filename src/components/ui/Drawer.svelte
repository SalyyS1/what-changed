<script lang="ts">
  import { Dialog as Bits } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import IconButton from './IconButton.svelte';

  /**
   * Right-hand panel over the current screen — the event details surface.
   * Same accessibility contract as Dialog (it *is* a dialog, positioned
   * differently), which is why it wraps the same bits-ui primitive rather
   * than a second implementation. Portalled to <body>, so the small-window
   * rule is a viewport query, not a container one: below 1040px it goes
   * full-width so nothing clips at 900×600.
   */
  interface Props {
    open?: boolean;
    title: string;
    children: Snippet;
    header?: Snippet;
  }

  let { open = $bindable(false), title, children, header }: Props = $props();
</script>

<Bits.Root bind:open>
  <Bits.Portal>
    <Bits.Overlay class="fixed inset-0 z-40 bg-scrim wc-overlay" />
    <Bits.Content
      data-testid="drawer"
      class="fixed top-(--titlebar-height) right-0 bottom-0 z-50 flex w-full max-w-130 flex-col border-l border-border-subtle bg-surface-1 shadow-lg wc-drawer max-[1039px]:max-w-none"
    >
      <div class="flex items-start justify-between gap-3 border-b border-border-subtle px-5 py-3">
        <div class="min-w-0 flex-1">
          <Bits.Title class="truncate text-md font-medium text-text-primary">{title}</Bits.Title>
          {#if header}{@render header()}{/if}
        </div>
        <Bits.Close>
          {#snippet child({ props })}
            <IconButton {...props} icon="close" label="Close details" size="sm" />
          {/snippet}
        </Bits.Close>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">{@render children()}</div>
    </Bits.Content>
  </Bits.Portal>
</Bits.Root>

<style>
  :global(.wc-drawer) {
    animation: wc-slide-in-right var(--t-slow) var(--ease-spring);
  }
</style>
