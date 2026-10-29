<script lang="ts">
  import { Select as Bits } from 'bits-ui';
  import Icon from './Icon.svelte';

  import type { SelectOption } from './ui-types';

  interface Props {
    options: readonly SelectOption[];
    value?: string;
    label: string;
    placeholder?: string;
    disabled?: boolean;
    class?: string;
    onchange?: (value: string) => void;
  }

  let {
    options,
    value = $bindable(''),
    label,
    placeholder = 'Choose…',
    disabled = false,
    class: className = '',
    onchange,
  }: Props = $props();

  const selectedLabel = $derived(options.find((o) => o.value === value)?.label ?? placeholder);
</script>

<Bits.Root type="single" bind:value {disabled} onValueChange={(v) => onchange?.(v)}>
  <Bits.Trigger
    aria-label={label}
    data-testid="select"
    class="inline-flex h-8 w-full items-center justify-between gap-2 rounded-md border border-border-subtle bg-surface-1 px-3 text-base text-text-primary motion-fast transition-colors hover:border-border-active disabled:opacity-45 data-[placeholder]:text-text-muted {className}"
  >
    <span class="truncate">{selectedLabel}</span>
    <Icon name="chevron-down" size={14} class="text-text-muted" />
  </Bits.Trigger>
  <Bits.Portal>
    <Bits.Content
      sideOffset={6}
      class="z-50 max-h-72 min-w-(--bits-select-anchor-width) overflow-y-auto rounded-lg border border-border-subtle bg-surface-2 p-1 shadow-lg wc-select"
    >
      {#each options as opt (opt.value)}
        <Bits.Item
          value={opt.value}
          label={opt.label}
          disabled={opt.disabled ?? false}
          class="flex h-8 cursor-default items-center justify-between rounded-md px-2 text-base text-text-primary outline-none data-[disabled]:opacity-45 data-[highlighted]:bg-surface-3"
        >
          {#snippet children({ selected })}
            <span>{opt.label}</span>
            {#if selected}<Icon name="check" size={14} class="text-accent" />{/if}
          {/snippet}
        </Bits.Item>
      {/each}
    </Bits.Content>
  </Bits.Portal>
</Bits.Root>

<style>
  :global(.wc-select) {
    animation: wc-rise-in var(--t-base) var(--ease-spring);
  }
</style>
