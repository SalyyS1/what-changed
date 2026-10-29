<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
  type Size = 'sm' | 'md' | 'lg';

  interface Props extends HTMLButtonAttributes {
    variant?: Variant;
    size?: Size;
    /** Fills the container width. */
    block?: boolean;
    children: Snippet;
  }

  let {
    variant = 'secondary',
    size = 'md',
    block = false,
    type = 'button',
    class: className = '',
    children,
    ...rest
  }: Props = $props();

  const variants: Record<Variant, string> = {
    primary:
      'bg-accent text-accent-contrast shadow-sm hover:brightness-110 active:brightness-95',
    secondary:
      'bg-surface-2 text-text-primary border border-border-subtle shadow-sm hover:bg-surface-3 hover:border-border-active',
    ghost: 'bg-transparent text-text-secondary hover:bg-surface-2 hover:text-text-primary',
    danger: 'bg-danger/12 text-danger border border-danger/30 hover:bg-danger/20',
  };

  const sizes: Record<Size, string> = {
    sm: 'h-7 px-2.5 text-sm gap-1.5',
    md: 'h-8 px-3 text-base gap-2',
    lg: 'h-10 px-4 text-md gap-2',
  };
</script>

<button
  {type}
  data-testid="button"
  class="inline-flex items-center justify-center rounded-md font-medium whitespace-nowrap select-none motion-fast transition-[background-color,border-color,filter] disabled:pointer-events-none disabled:opacity-45 {variants[variant]} {sizes[size]} {block ? 'w-full' : ''} {className}"
  {...rest}
>
  {@render children()}
</button>
