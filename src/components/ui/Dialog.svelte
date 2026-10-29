<script lang="ts">
  import { Dialog as Bits } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import IconButton from './IconButton.svelte';

  /**
   * Modal dialog. bits-ui owns the focus trap, `aria-modal`, Escape and
   * outside-click dismissal, and focus restoration to the opener; everything
   * visible is ours. `title` is mandatory because a dialog without an
   * accessible name is a dialog a screen reader cannot announce.
   */
  interface Props {
    open?: boolean;
    title: string;
    description?: string;
    /** Prevent outside-click / Escape dismissal for destructive confirmations. */
    sticky?: boolean;
    width?: 'sm' | 'md' | 'lg';
    /** Override the default first-control focus when a dialog has a primary field. */
    onOpenAutoFocus?: (event: Event) => void;
    children: Snippet;
    footer?: Snippet;
  }

  let {
    open = $bindable(false),
    title,
    description,
    sticky = false,
    width = 'md',
    onOpenAutoFocus = () => {},
    children,
    footer,
  }: Props = $props();

  const widths = { sm: 'max-w-sm', md: 'max-w-md', lg: 'max-w-2xl' };
</script>

<Bits.Root bind:open>
  <Bits.Portal>
    <Bits.Overlay class="fixed inset-0 z-40 bg-scrim wc-overlay" />
    <!--
      Centred by a full-screen grid rather than a translate(-50%, -50%) on the
      panel, so the panel's own transform is free for the rise-in animation
      and a panel taller than the window scrolls instead of clipping. The
      wrapper ignores the pointer so a click beside the panel reaches the
      scrim, which is what bits-ui counts as "outside".
    -->
    <div class="pointer-events-none fixed inset-0 z-50 grid place-items-center overflow-y-auto p-4">
      <Bits.Content
        escapeKeydownBehavior={sticky ? 'ignore' : 'close'}
        interactOutsideBehavior={sticky ? 'ignore' : 'close'}
        {onOpenAutoFocus}
        data-testid="dialog"
        class="pointer-events-auto w-full rounded-xl border border-border-subtle bg-surface-1 shadow-lg wc-dialog {widths[width]}"
      >
        <div class="flex items-start justify-between gap-4 px-5 pt-4 pb-2">
          <div class="min-w-0">
            <Bits.Title class="font-display text-lg leading-tight text-text-primary">{title}</Bits.Title>
            {#if description}
              <Bits.Description class="mt-1 text-sm text-text-secondary">{description}</Bits.Description>
            {/if}
          </div>
          {#if !sticky}
            <Bits.Close>
              {#snippet child({ props })}
                <IconButton {...props} icon="close" label="Close" size="sm" class="-mt-1 -mr-1" />
              {/snippet}
            </Bits.Close>
          {/if}
        </div>
        <div class="px-5 py-2 text-base text-text-primary">{@render children()}</div>
        {#if footer}
          <div class="flex justify-end gap-2 px-5 pt-2 pb-4">{@render footer()}</div>
        {/if}
      </Bits.Content>
    </div>
  </Bits.Portal>
</Bits.Root>

<style>
  :global(.wc-overlay) {
    animation: wc-fade-in var(--t-base) var(--ease-standard);
  }
  :global(.wc-dialog) {
    animation: wc-rise-in var(--t-slow) var(--ease-spring);
  }
</style>
