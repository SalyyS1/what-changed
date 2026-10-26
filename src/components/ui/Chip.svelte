<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  /**
   * A toggleable filter pill. `aria-pressed` carries the state so a screen
   * reader hears "pressed" rather than a visual-only highlight. A chip with
   * nothing behind it renders disabled and says why in its title.
   */
  interface Props extends HTMLButtonAttributes {
    selected?: boolean;
    disabledReason?: string;
    children: Snippet;
  }

  let {
    selected = false,
    disabledReason,
    disabled = false,
    class: className = '',
    children,
    ...rest
  }: Props = $props();
</script>

<button
  type="button"
  aria-pressed={selected}
  disabled={disabled || disabledReason !== undefined}
  title={disabledReason}
  data-testid="chip"
  class="inline-flex h-7 items-center gap-1.5 rounded-full border px-3 text-sm motion-fast transition-colors disabled:cursor-not-allowed disabled:opacity-45 {selected
    ? 'border-accent/40 bg-accent/15 text-accent'
    : 'border-border-subtle bg-surface-2 text-text-secondary hover:border-border-active hover:text-text-primary'} {className}"
  {...rest}
>
  {@render children()}
</button>
