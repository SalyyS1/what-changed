<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    /** Surface tier. Cards nest: a level-2 card sits on a level-1 surface. */
    level?: 1 | 2 | 3;
    padded?: boolean;
    children: Snippet;
  }

  let { level = 1, padded = true, class: className = '', children, ...rest }: Props = $props();

  const surfaces = { 1: 'bg-surface-1', 2: 'bg-surface-2', 3: 'bg-surface-3' };
</script>

<div
  data-testid="card"
  class="rounded-lg border border-border-subtle shadow-sm {surfaces[level]} {padded ? 'p-4' : ''} {className}"
  {...rest}
>
  {@render children()}
</div>
