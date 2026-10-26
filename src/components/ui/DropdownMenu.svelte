<script lang="ts">
  import { DropdownMenu as Bits } from 'bits-ui';
  import type { Snippet } from 'svelte';

  import type { MenuItem } from './ui-types';

  interface Props {
    items: readonly MenuItem[];
    onselect: (id: string) => void;
    align?: 'start' | 'center' | 'end';
    trigger: Snippet<[Record<string, unknown>]>;
  }

  let { items, onselect, align = 'end', trigger }: Props = $props();
</script>

<Bits.Root>
  <Bits.Trigger>
    {#snippet child({ props })}
      {@render trigger(props)}
    {/snippet}
  </Bits.Trigger>
  <Bits.Portal>
    <Bits.Content
      {align}
      sideOffset={6}
      data-testid="menu"
      class="z-50 min-w-48 rounded-lg border border-border-subtle bg-surface-2 p-1 shadow-lg wc-menu"
    >
      {#each items as item (item.id)}
        {#if item.separatorBefore}
          <Bits.Separator class="my-1 h-px bg-border-subtle" />
        {/if}
        <Bits.Item
          disabled={item.disabled ?? false}
          onSelect={() => onselect(item.id)}
          data-testid="menu-item"
          class="flex h-8 cursor-default items-center justify-between gap-6 rounded-md px-2 text-base outline-none data-[disabled]:opacity-45 data-[highlighted]:bg-surface-3 {item.tone ===
          'danger'
            ? 'text-danger'
            : 'text-text-primary'}"
        >
          <span>{item.label}</span>
          {#if item.shortcut}
            <span class="font-mono text-xs text-text-muted">{item.shortcut}</span>
          {/if}
        </Bits.Item>
      {/each}
    </Bits.Content>
  </Bits.Portal>
</Bits.Root>

<style>
  :global(.wc-menu) {
    animation: wc-rise-in var(--t-base) var(--ease-spring);
  }
</style>
