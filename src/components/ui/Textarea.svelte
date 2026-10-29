<script lang="ts">
  import type { HTMLTextareaAttributes } from 'svelte/elements';

  interface Props extends HTMLTextareaAttributes {
    value?: string;
    label?: string;
    invalid?: boolean;
  }

  let {
    value = $bindable(''),
    label,
    invalid = false,
    rows = 3,
    class: className = '',
    ...rest
  }: Props = $props();
</script>

<textarea
  bind:value
  {rows}
  aria-label={label}
  aria-invalid={invalid || undefined}
  data-testid="textarea"
  class="selectable w-full resize-y rounded-md border bg-surface-1 px-3 py-2 text-base leading-relaxed text-text-primary placeholder:text-text-muted motion-fast transition-colors focus:border-border-active disabled:opacity-45 {invalid
    ? 'border-danger'
    : 'border-border-subtle'} {className}"
  {...rest}
></textarea>
