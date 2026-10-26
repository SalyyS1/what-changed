<script lang="ts">
  import { Switch as Bits } from 'bits-ui';

  interface Props {
    checked?: boolean;
    label: string;
    disabled?: boolean;
    class?: string;
    onchange?: (checked: boolean) => void;
  }

  let {
    checked = $bindable(false),
    label,
    disabled = false,
    class: className = '',
    onchange,
  }: Props = $props();
</script>

<Bits.Root
  bind:checked
  {disabled}
  aria-label={label}
  onCheckedChange={(v) => onchange?.(v)}
  data-testid="switch"
  class="relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border border-transparent motion-fast transition-colors disabled:cursor-not-allowed disabled:opacity-45 data-[state=checked]:bg-accent data-[state=unchecked]:bg-surface-3 {className}"
>
  <Bits.Thumb
    class="pointer-events-none block h-4 w-4 rounded-full bg-text-primary shadow-sm motion-fast transition-transform data-[state=checked]:translate-x-4 data-[state=unchecked]:translate-x-0.5 data-[state=checked]:bg-accent-contrast"
  />
</Bits.Root>
