<script lang="ts">
  import { Tabs as Bits } from 'bits-ui';
  import type { Snippet } from 'svelte';

  import type { TabItem } from './ui-types';

  /**
   * Segmented tabs. Content is rendered by the caller through `content`
   * keyed on the active value, so the tab strip and the panel stay one
   * component but the panel is free-form.
   */
  interface Props {
    items: readonly TabItem[];
    value?: string;
    label: string;
    class?: string;
    onchange?: (value: string) => void;
    /** Panel per tab. Omit for a bare strip that only drives external state. */
    content?: Snippet<[string]>;
  }

  let { items, value = $bindable(items[0]?.value ?? ''), label, class: className = '', onchange, content }: Props =
    $props();
</script>

<Bits.Root bind:value onValueChange={(next) => onchange?.(next)} class={className}>
  <Bits.List
    aria-label={label}
    class="inline-flex h-8 items-center gap-0.5 rounded-md border border-border-subtle bg-surface-2 p-0.5"
  >
    {#each items as item (item.value)}
      <Bits.Trigger
        value={item.value}
        data-testid="tabs-trigger"
        class="h-7 rounded-sm px-3 text-sm text-text-secondary motion-fast transition-colors hover:text-text-primary data-[state=active]:bg-surface-1 data-[state=active]:text-text-primary data-[state=active]:shadow-sm"
      >
        {item.label}
      </Bits.Trigger>
    {/each}
  </Bits.List>
  {#if content}
    {#each items as item (item.value)}
      <Bits.Content value={item.value} data-testid="tabs-content" class="pt-4">
        {@render content(item.value)}
      </Bits.Content>
    {/each}
  {/if}
</Bits.Root>
