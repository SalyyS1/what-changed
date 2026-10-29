<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends HTMLInputAttributes {
    value?: string;
    invalid?: boolean;
    /** Accessible name when there is no visible <label>. */
    label?: string;
  }

  let {
    value = $bindable(''),
    invalid = false,
    label,
    class: className = '',
    ...rest
  }: Props = $props();
</script>

<input
  bind:value
  aria-label={label}
  aria-invalid={invalid || undefined}
  data-testid="input"
  class="selectable h-8 w-full rounded-md border bg-surface-1 px-3 text-base text-text-primary placeholder:text-text-muted motion-fast transition-colors focus:border-border-active disabled:opacity-45 {invalid
    ? 'border-danger'
    : 'border-border-subtle'} {className}"
  {...rest}
/>
